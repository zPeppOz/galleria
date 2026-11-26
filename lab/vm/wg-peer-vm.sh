#!/usr/bin/env bash
# Dentro la VM, da `lab` (usa sudo): un server WireGuard vero per il giro quotidiano (B6, gruppo D). Il server vive in un
# namespace di rete `glabpeer` collegato con un veth, così il tunnel del demone attraversa una rete vera (UDP sul veth) e
# non c'è bisogno di internet né dell'host. Tunnel diviso: solo 10.99.0.0/24, la SSH della VM non si tocca.
#
#   wg-peer-vm.sh up        crea namespace, veth, server wg0 (10.99.0.1) e scrive ~/spike/daily-office.conf (client 10.99.0.2)
#   wg-peer-vm.sh status    handshake visto dal server e ping del server attraverso il tunnel (exit 0 solo se entrambi)
#   wg-peer-vm.sh down      toglie il namespace e il veth
#
# Ricreato a ogni `up`: le chiavi cambiano, il file .conf va reimportato.
set -euo pipefail
ns=glabpeer
conf=${DAILY_CONF:-$HOME/spike/daily-office.conf}
case ${1:-} in
up)
  command -v wg >/dev/null || { echo 'manca wg (wireguard-tools)' >&2; exit 1; }
  sudo ip netns del "$ns" 2>/dev/null || true
  sudo ip link del glab-v0 2>/dev/null || true
  sudo ip netns add "$ns"
  sudo ip link add glab-v0 type veth peer name glab-v1 netns "$ns"
  sudo ip addr add 10.77.0.1/24 dev glab-v0; sudo ip link set glab-v0 up
  sudo ip -n "$ns" addr add 10.77.0.2/24 dev glab-v1; sudo ip -n "$ns" link set glab-v1 up; sudo ip -n "$ns" link set lo up
  umask 077
  skey=$(wg genkey); ckey=$(wg genkey); spub=$(printf '%s\n' "$skey" | wg pubkey); cpub=$(printf '%s\n' "$ckey" | wg pubkey)
  sudo ip -n "$ns" link add wg0 type wireguard
  sudo ip -n "$ns" addr add 10.99.0.1/24 dev wg0
  printf '[Interface]\nPrivateKey = %s\nListenPort = 51820\n[Peer]\nPublicKey = %s\nAllowedIPs = 10.99.0.2/32\n' "$skey" "$cpub" |
    sudo ip netns exec "$ns" wg setconf wg0 /dev/stdin
  sudo ip -n "$ns" link set wg0 up
  mkdir -p "$(dirname "$conf")"
  cat > "$conf" <<PROFILE
[Interface]
PrivateKey = $ckey
Address = 10.99.0.2/24

[Peer]
PublicKey = $spub
Endpoint = 10.77.0.2:51820
AllowedIPs = 10.99.0.0/24
PersistentKeepalive = 5
PROFILE
  unset skey ckey
  echo "server up in $ns (10.77.0.2:51820), client config $conf"
  ;;
status)
  hs=$(sudo ip netns exec "$ns" wg show wg0 latest-handshakes | awk '{print $2}')
  echo "server latest handshake (epoch): ${hs:-none}"
  echo "client interfaces: $(ip -br link show type wireguard | awk '{print $1, $2}' | tr '\n' ' ')"
  [[ -n ${hs:-} && ${hs:-0} != 0 ]] || { echo 'no handshake'; exit 1; }
  ping -c2 -W2 10.99.0.1
  ;;
down)
  sudo ip netns del "$ns" 2>/dev/null || true
  sudo ip link del glab-v0 2>/dev/null || true
  echo down
  ;;
*) sed -n '2,12p' "$0"; exit 2 ;;
esac
