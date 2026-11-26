#!/bin/bash
# What the client sees while the window works on its daemon (plan B5, T9; started in the client by `scripts/gui.sh lab-connect-window`
# with `docker exec -d`). Everything goes to /tmp/gui-lab-watch.log, which `crates/galleria-e2e/tests/t18_gui_connect.rs` reads:
#
#   residue-begin / residue-end   the state of the network when the watch starts (the same lines as `galleria_e2e::residue`)
#   sample <ms> openvpn=<n> wg=<0|1>   every 200 ms: how many `openvpn` processes there are and whether `glr-wg0` is there
#   argv <ms> <pid> <arguments>   the arguments of each `openvpn` process of a sample
#
# Once a second, with a tunnel up, one ping through it so the bytes of the window move (10.8.0.1 for OpenVPN as in t16, 10.99.0.1
# for WireGuard as in t08). The watch never writes a value it did not read from the processes or the interfaces.
set -u
log=/tmp/gui-lab-watch.log
: > "$log"
{
  echo residue-begin
  bash -c "set -o pipefail; { ip -br link; ip route; ip -6 route; ip rule; ip -6 rule; resolvectl dns; } | sed -E 's/ expires [0-9]+sec//'"
  echo residue-end
} >> "$log"
tick=0
while :; do
  ms=$(date +%s%3N)
  processes=$(pgrep -a -x openvpn || true)
  count=0
  [[ -z $processes ]] || count=$(printf '%s\n' "$processes" | grep -c .)
  wg=0
  ip -br link show glr-wg0 > /dev/null 2>&1 && wg=1
  {
    echo "sample $ms openvpn=$count wg=$wg"
    [[ -z $processes ]] || printf '%s\n' "$processes" | sed "s/^/argv $ms /"
  } >> "$log"
  if (( tick % 5 == 0 )); then
    if (( wg == 1 )); then (ping -c 1 -W 1 10.99.0.1 > /dev/null 2>&1 &); fi
    if (( count > 0 )); then (ping -c 1 -W 1 10.8.0.1 > /dev/null 2>&1 &); fi
  fi
  tick=$((tick + 1))
  sleep 0.2
done
