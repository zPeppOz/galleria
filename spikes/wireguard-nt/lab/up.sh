#!/bin/bash
# Spike 1: server WireGuard di prova in Docker + relay verso la Sandbox + file per la Sandbox.
# Serve il laboratorio dello spike 2 gia' su (rete galleria-lab e container gl-target: spikes/openvpn-mgmt/lab/up.sh).
set -euo pipefail
cd "$(dirname "$0")"
SBX=/mnt/c/Users/hackp/galleria-sbx
RELAY="$(cd ../../windows-lab/filerelay && pwd)/target/release/filerelay"
mkdir -p out "$SBX/in/wg" "$SBX/out/relay"
docker build -q -t galleria-lab-wg -f wg.Dockerfile . >/dev/null
if ! docker ps --format '{{.Names}}' | grep -qx gl-wg; then
  docker run -d --name gl-wg --hostname wgserver --network galleria-lab --ip 172.30.0.12 \
    --cap-add NET_ADMIN --sysctl net.ipv4.ip_forward=1 -v "$PWD/out:/out" galleria-lab-wg >/dev/null
  sleep 2
fi
docker logs gl-wg 2>&1 | tail -6
# il relay verso 172.30.0.12:51820 lo avvia spikes/windows-lab/lab.sh relays
# file per la Sandbox: profilo, servizio, wireguard.dll ufficiale (dallo zip di download.wireguard.com), client di prova
cp out/wgtest.conf "$SBX/in/wg/"
cp ../galleriad/target/x86_64-pc-windows-gnu/release/galleriad.exe "$SBX/in/wg/"
cp ../../openvpn-mgmt/target/x86_64-pc-windows-gnu/release/galleriad-spike.exe "$SBX/in/wg/probe.exe"
[ -f "$SBX/in/wg/wireguard.dll" ] || { T=$(mktemp -d) && python3 -m zipfile -e "$SBX/in/dl/wireguard-nt-0.10.1.zip" "$T" && cp "$T/wireguard-nt/bin/amd64/wireguard.dll" "$SBX/in/wg/"; }
sha256sum "$SBX/in/wg/wireguard.dll" "$SBX/in/dl/wireguard-nt-0.10.1.zip"
ls -la "$SBX/in/wg/"
