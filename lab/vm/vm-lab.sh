#!/bin/bash
# Run INSIDE a manual test VM (Ubuntu or Fedora desktop), as root. Disposable VMs only.
# It rebuilds, around the VM's real NetworkManager, what lab/up.sh gives the Docker client:
#   - the lab servers (glab-*) on the Docker network galleria-fase1 (172.31.0.0/24, 2001:db8:31::/64);
#   - an `eth0` veth, plugged into that bridge, as the "physical" lab link owned by NetworkManager;
#   - /.dockerenv, which is the only thing the e2e guard (galleria_e2e::assert_lab) looks at;
#   - alice/mallory, hosts entry, sysctl, the installed product (packaging/linux/install.sh).
# The VM keeps its user-mode NIC (enp0s1) for SSH; like the Docker client, eth0 carries the default
# routes (via the lab target, metric 50), so while the lab is up the VM has no internet: to install
# something run `nmcli connection down glab-eth0`, then `vm-lab.sh up` again.
# Usage: vm-lab.sh prepare <octet>   once (27 -> 172.31.0.37 on Ubuntu, 79 -> 172.31.0.89 on Fedora)
#        vm-lab.sh up <octet>        after every boot: servers + eth0
#        vm-lab.sh run <filter>      run the lab tests from /lab/lab/out/tests (like lab/run.sh)
set -euo pipefail
lab=${LAB_ROOT:-/lab}
cmd=${1:?prepare|up|run}
case $cmd in
prepare)
  octet=${2:?octet}
  [[ $EUID == 0 ]] || { echo 'run as root' >&2; exit 1; }
  touch /.dockerenv
  # Cloud images leave systemd-networkd (dracut default DHCP) running next to NetworkManager: a real
  # desktop has only NetworkManager, so stop networkd to avoid a second default route.
  if systemctl is-enabled systemd-networkd >/dev/null 2>&1; then
    systemctl disable --now systemd-networkd.service systemd-networkd.socket systemd-networkd-wait-online.service 2>/dev/null || true
    rm -f /run/systemd/network/zzzz-dracut-default.network
  fi
  printf 'net.ipv4.conf.all.src_valid_mark=1\n' > /etc/sysctl.d/90-galleria-lab.conf
  sysctl -q -p /etc/sysctl.d/90-galleria-lab.conf
  grep -q dual.galleria.test /etc/hosts || printf '198.51.100.1 dual.galleria.test\n2001:db8:99::1 dual.galleria.test\n' >> /etc/hosts
  systemctl enable --now docker
  if ! docker image inspect glab-server >/dev/null 2>&1; then gunzip -c /var/tmp/glab-server.tar.gz | docker load; fi
  install -m755 "$lab/lab/out/bin/ipc-probe" /usr/local/bin/ipc-probe
  "$lab/packaging/linux/install.sh" "$lab/lab/out/bin"
  id alice >/dev/null 2>&1 || useradd -m -G galleria alice
  id mallory >/dev/null 2>&1 || useradd -m mallory
  ;;
up)
  octet=${2:?octet}
  c=$((octet+10))
  # SELinux (Fedora): containers may only write bind mounts labelled container_file_t.
  if command -v selinuxenabled >/dev/null && selinuxenabled; then
    chcon -R -t container_file_t "$lab/lab/pki" "$lab/lab/out"
  fi
  ( cd "$lab/lab" && CLIENTS="" ./up.sh )
  br=br-$(docker network inspect galleria-fase1 -f '{{.Id}}' | cut -c1-12)
  # The bridge's own addresses would win over eth0's connected routes (metric 0): the VM must reach the
  # lab only through eth0, the way the Docker client does. Containers never need the bridge gateway.
  ip addr flush dev "$br"
  ip link del eth0 2>/dev/null || true
  ip link add eth0 type veth peer name glab-p0
  ip link set glab-p0 master "$br" up
  # NetworkManager adds 20000 to the route metric of a link that fails its connectivity check, and
  # the lab link (no internet) fails it: push the user-mode uplink's own metric above that, so the
  # lab default (via the target) wins as it does in the Docker client.
  uplink=$(ip -4 route show 10.0.2.0/24 | awk '{print $3; exit}')
  if [[ -n $uplink ]]; then
    upcon=$(nmcli -g GENERAL.CONNECTION device show "$uplink")
    nmcli connection modify "$upcon" ipv4.route-metric 30000 ipv6.route-metric 30000
    nmcli device reapply "$uplink" >/dev/null
  fi
  if nmcli -t -f NAME connection show | grep -qx glab-eth0; then nmcli connection delete glab-eth0 >/dev/null; fi
  # Like the Docker client: the lab target is the default gateway on eth0 (metric 50 beats the
  # user-mode NIC's 100), so a full tunnel overrides it with 0/1+128/1 exactly as in the lab.
  nmcli connection add type ethernet ifname eth0 con-name glab-eth0 \
    ipv4.method manual ipv4.addresses "172.31.0.$c/24" ipv4.gateway 172.31.0.20 ipv4.route-metric 50 \
    ipv6.method manual ipv6.addresses "2001:db8:31::$c/64" ipv6.gateway 2001:db8:31::20 ipv6.route-metric 50 >/dev/null
  nmcli device set eth0 managed yes || true
  nmcli connection up glab-eth0 >/dev/null || true
  ip link set eth0 up
  for i in $(seq 20); do ip route get 198.51.100.1 | grep -q 'dev eth0' && break; sleep 1; done
  for i in $(seq 10); do ip -6 addr show dev eth0 tentative | grep -q inet6 || break; sleep 1; done
  ping -c1 -W2 172.31.0.20 >/dev/null && echo "lab link ok (eth0 172.31.0.$c)"
  ;;
run)
  filter=${2:-}
  export PATH="$lab/lab/vm/shims:$PATH"   # pgrep/pkill openvpn must not see the lab servers' containers
  matched=0 failed=0
  for executable in "$lab"/lab/out/tests/*; do
    name=$(basename "$executable")
    args=()
    if [[ -n $filter && $name != *"$filter"* ]]; then args+=("$filter"); fi
    # binfmt_units_are_masked checks the WSL2 workaround baked into the lab container images: a real VM
    # keeps the distro's binfmt units, so it is skipped here by design.
    args+=(--skip binfmt_units_are_masked)
    listed=$(GALLERIA_LAB=1 "$executable" --ignored --list "${args[@]}")
    [[ $listed == *': test'* ]] || continue
    matched=$((matched+1))
    GALLERIA_LAB=1 GALLERIA_E2E_DCO=${GALLERIA_E2E_DCO:-0} "$executable" --ignored --test-threads=1 --nocapture "${args[@]}" || failed=$((failed+1))
  done
  ((matched>0)) || { echo 'no end-to-end tests matched' >&2; exit 1; }
  ((failed==0)) || { echo "$failed test binaries failed" >&2; exit 1; }
  ;;
esac
