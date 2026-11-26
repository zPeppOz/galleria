#!/bin/bash
# What the client sees while the window works on its daemon (plan B5, T9; started in the client by `scripts/gui.sh lab-connect-window`
# with `docker exec -d`). Everything goes to /tmp/gui-lab-watch.log, which `crates/galleria-e2e/tests/t18_gui_connect.rs` reads:
#
#   residue-begin / residue-end   the state of the network when the watch starts (the same lines as `galleria_e2e::residue`)
#   sample <ms> openvpn=<n> wg=<0|1> ovpn_if=<0|1>   every 200 ms: how many `openvpn` processes there are, whether `glr-wg0` is there
#                                 (1 only if it was there before and after the processes) and whether `glr-ovpn0`, the interface of
#                                 an OpenVPN tunnel, is there (1 if it was there before or after: when in doubt, it is there)
#   argv <ms> <pid> <arguments>   the arguments of each `openvpn` process of a sample
#   mark <ms> <name>              what the window said it was doing (plan B6, T6, G10g): the window appends `<ms> <name>` to
#                                 target/gui-lab/marks, which the client sees as /lab/target/gui-lab/marks; the watch copies each
#                                 line as it appears. t18 reads the samples between two marks (`gui_closed_window_kept_the_tunnel`)
#
# Once a second, with a tunnel up, one ping through it so the bytes of the window move (10.8.0.1 for OpenVPN as in t16, 10.99.0.1
# for WireGuard as in t08). The watch never writes a value it did not read from the processes or the interfaces.
set -u
log=/tmp/gui-lab-watch.log
marks=${GALLERIA_LAB_MARKS:-/lab/target/gui-lab/marks}
marks_seen=0
: > "$log"
{
  echo residue-begin
  bash -c "set -o pipefail; { ip -br link; ip route; ip -6 route; ip rule; ip -6 rule; resolvectl dns; } | sed -E 's/ expires [0-9]+sec//'"
  echo residue-end
} >> "$log"
tick=0
while :; do
  ms=$(date +%s%3N)
  # The two readings are not one instant: `glr-wg0` is read before and after the processes, and counts only when it was there both
  # times. A tunnel that was torn down while `openvpn` was being read is then not taken for two backends at once; one that was there
  # the whole time and an `openvpn` that was seen in between is two backends alive together (D5).
  # `glr-ovpn0` is read the other way round: it counts as there when it was there in either reading (Q48: an `[openvpn]` is left out
  # of the backends only when the tunnel interface is gone in the same sample, so the doubt goes to the interface being there).
  wg_before=0
  ovpn_before=0
  ip -br link show glr-wg0 > /dev/null 2>&1 && wg_before=1
  ip -br link show glr-ovpn0 > /dev/null 2>&1 && ovpn_before=1
  processes=$(pgrep -a -x openvpn || true)
  count=0
  [[ -z $processes ]] || count=$(printf '%s\n' "$processes" | grep -c .)
  wg=0
  if (( wg_before == 1 )) && ip -br link show glr-wg0 > /dev/null 2>&1; then wg=1; fi
  ovpn_if=$ovpn_before
  if (( ovpn_if == 0 )) && ip -br link show glr-ovpn0 > /dev/null 2>&1; then ovpn_if=1; fi
  {
    echo "sample $ms openvpn=$count wg=$wg ovpn_if=$ovpn_if"
    [[ -z $processes ]] || printf '%s\n' "$processes" | sed "s/^/argv $ms /"
  } >> "$log"
  if (( tick % 5 == 0 )); then
    if (( wg == 1 )); then (ping -c 1 -W 1 10.99.0.1 > /dev/null 2>&1 &); fi
    if (( count > 0 )); then (ping -c 1 -W 1 10.8.0.1 > /dev/null 2>&1 &); fi
  fi
  # the marks of the window, a whole line at a time: the window writes each with one append, and `wc -l` counts only finished lines
  if [[ -r $marks ]]; then
    marks_total=$(wc -l < "$marks")
    if (( marks_total > marks_seen )); then
      sed -n "$((marks_seen + 1)),${marks_total}p" "$marks" | sed 's/^/mark /' >> "$log"
      marks_seen=$marks_total
    fi
  fi
  tick=$((tick + 1))
  sleep 0.2
done
