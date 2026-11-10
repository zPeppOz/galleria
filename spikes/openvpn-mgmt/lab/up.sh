#!/bin/bash
# Avvia il laboratorio: rete dual-stack, server OpenVPN, "internet" dual-stack (target), due client.
# Tutto dentro Docker: nessuna modifica a rotte/DNS dell'host.
set -euo pipefail
cd "$(dirname "$0")"
SPIKE="$(cd .. && pwd)"
# binario del demone (cargo build --release) montato in sola lettura nei client
mkdir -p bin profiles "$SPIKE/evidence"
[ -x "$SPIKE/target/release/galleriad-spike" ] || { echo "build first: cargo build --release" >&2; exit 1; }
cp "$SPIKE/target/release/galleriad-spike" bin/
docker network inspect galleria-lab >/dev/null 2>&1 || \
  docker network create --ipv6 --subnet 172.30.0.0/24 --subnet 2001:db8:30::/64 galleria-lab >/dev/null

# target: simula un sito dual-stack fuori dalla LAN (198.51.100.1 e 2001:db8:99::1 su lo, raggiunti via router)
docker rm -f gl-target >/dev/null 2>&1 || true
docker run -d --name gl-target --hostname target --network galleria-lab --ip 172.30.0.20 --ip6 2001:db8:30::20 \
  --cap-add NET_ADMIN --sysctl net.ipv6.conf.all.disable_ipv6=0 --sysctl net.ipv4.ip_forward=1 \
  galleria-lab-server bash -c '
    ip addr add 198.51.100.1/32 dev lo; ip -6 addr add 2001:db8:99::1/128 dev lo
    ip route add 10.8.0.0/24 via 172.30.0.10
    cd /tmp && echo "hello from dual-stack target" > index.html
    exec python3 -c "
import http.server, socket
class S(http.server.ThreadingHTTPServer):
    address_family = socket.AF_INET6
    def server_bind(self):
        self.socket.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 0); super().server_bind()
class H(http.server.SimpleHTTPRequestHandler):
    def log_message(self, f, *a): print(self.client_address[0], f % a, flush=True)
S((\"::\", 80), H).serve_forever()"' >/dev/null

docker rm -f gl-server >/dev/null 2>&1 || true
docker run -d --name gl-server --hostname vpnserver --network galleria-lab --ip 172.30.0.10 --ip6 2001:db8:30::10 \
  --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 \
  -v "$PWD/pki:/pki:ro" galleria-lab-server >/dev/null

docker rm -f gl-server-dns >/dev/null 2>&1 || true
docker run -d --name gl-server-dns --hostname vpnserver-dns --network galleria-lab --ip 172.30.0.11 --ip6 2001:db8:30::11 \
  --cap-add NET_ADMIN --device /dev/net/tun --sysctl net.ipv4.ip_forward=1 \
  -v "$PWD/pki:/pki:ro" galleria-lab-server /etc/openvpn/lab/entrypoint.sh server-dns.conf >/dev/null

# 26 = Ubuntu 24.04 (OpenVPN 2.6), 27 = Ubuntu 26.04 (2.7.0), 79 = Fedora corrente (2.7.9)
CLIENTS="${CLIENTS:-26 27 79}"
for v in $CLIENTS; do
  img=galleria-lab-client$v; [ $v = 79 ] && img=galleria-lab-clientfedora
  ip4=172.30.0.$((v+10)); ip6=2001:db8:30::$((v+10))
  docker rm -f gl-client$v >/dev/null 2>&1 || true
  # --privileged serve a systemd (PID 1) e a systemd-resolved dentro il container
  docker run -d --name gl-client$v --hostname client$v --network galleria-lab --ip $ip4 --ip6 $ip6 \
    --privileged --tmpfs /run --tmpfs /run/lock --sysctl net.ipv6.conf.all.disable_ipv6=0 \
    -v "$PWD/bin:/usr/local/lib/galleria:ro" \
    -v "$PWD/pki:/pki:ro" -v "$PWD/profiles:/profiles:ro" -v "$SPIKE/evidence:/evidence" \
    $img >/dev/null
done
sleep 4
for v in $CLIENTS; do
  # il "router" della LAN di prova e' il target: default v4/v6 verso di lui, come una LAN domestica
  docker exec gl-client$v bash -c '
    ip route replace default via 172.30.0.20
    ip -6 route replace default via 2001:db8:30::20
    grep -q dual.galleria.test /etc/hosts || printf "198.51.100.1 dual.galleria.test\n2001:db8:99::1 dual.galleria.test\n" >> /etc/hosts'
done
docker ps --filter name=gl- --format '{{.Names}}\t{{.Status}}'
