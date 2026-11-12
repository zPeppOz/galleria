#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
clients=${CLIENTS-26 27 79}
for client in $clients; do case "$client" in 26|27|79) ;; *) exit 2;; esac; done
./down.sh
docker network create --ipv6 --subnet 172.31.0.0/24 --subnet 2001:db8:31::/64 galleria-fase1 >/dev/null
docker run -d --name glab-target --network galleria-fase1 --ip 172.31.0.20 --ip6 2001:db8:31::20 \
  --cap-add NET_ADMIN --sysctl net.ipv6.conf.all.disable_ipv6=0 --sysctl net.ipv4.ip_forward=1 --sysctl net.ipv6.conf.all.forwarding=1 \
  glab-server bash -euc 'ip addr add 198.51.100.1/32 dev lo; ip -6 addr add 2001:db8:99::1/128 dev lo; exec python3 /lab/server/target.py' >/dev/null
for entry in 'ovpn 10 server.conf' 'ovpn-dns 11 server-dns.conf' 'ovpn-split 13 server-split.conf' 'ovpn-v6 14 server-v6.conf'; do
  read -r name octet conf <<< "$entry"
  docker run -d --name "glab-$name" --network galleria-fase1 --ip "172.31.0.$octet" --ip6 "2001:db8:31::$octet" \
    --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 --sysctl net.ipv6.conf.all.forwarding=1 \
    -v "$PWD/pki:/pki:ro" glab-server /lab/server/entrypoint.sh "$conf" >/dev/null
done
docker run -d --name glab-wg --network galleria-fase1 --ip 172.31.0.12 --ip6 2001:db8:31::12 \
  --cap-add NET_ADMIN --sysctl net.ipv4.ip_forward=1 -v "$PWD/out:/out" glab-server /lab/wg/entrypoint.sh >/dev/null
for client in $clients; do
  octet=$((client+10))
  docker run -d --name "glab-client$client" --hostname "client$client" --network galleria-fase1 \
    --ip "172.31.0.$octet" --ip6 "2001:db8:31::$octet" \
    --cap-add NET_ADMIN --cap-add NET_RAW --cap-add SYS_ADMIN --cgroupns=private \
    --security-opt seccomp=unconfined --security-opt apparmor=unconfined \
    --tmpfs /run --tmpfs /run/lock --tmpfs /tmp --device /dev/net/tun \
    --sysctl net.ipv6.conf.all.disable_ipv6=0 \
    -v "$PWD/..:/lab:ro" "glab-client$client" \
    bash -euc 'umount /sys/fs/cgroup; mount -t cgroup2 none /sys/fs/cgroup; exec /sbin/init' >/dev/null
  ready=0
  for ((n=0;n<150;n++)); do
    if docker exec "glab-client$client" systemctl is-active systemd-resolved >/dev/null 2>&1; then ready=1; break; fi
    [[ $(docker inspect -f '{{.State.Running}}' "glab-client$client") == true ]] || break
    sleep .2
  done
  if ((ready==0)); then docker logs "glab-client$client"; exit 1; fi
  docker exec "glab-client$client" bash -euc '
    ip route replace default via 172.31.0.20
    ip -6 route replace default via 2001:db8:31::20
    printf "198.51.100.1 dual.galleria.test\n2001:db8:99::1 dual.galleria.test\n" >> /etc/hosts
    ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf 2>/dev/null || true
    install -m755 /lab/lab/out/bin/ipc-probe /usr/local/bin/ipc-probe
    /lab/packaging/linux/install.sh /lab/lab/out/bin'
done
for name in glab-target glab-wg glab-ovpn glab-ovpn-dns glab-ovpn-split glab-ovpn-v6; do
  [[ $(docker inspect -f '{{.State.Running}}' "$name") == true ]] || { docker logs "$name"; exit 1; }
done
