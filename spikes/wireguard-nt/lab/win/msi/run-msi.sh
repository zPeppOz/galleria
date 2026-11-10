#!/bin/bash
# Spike 1 su Windows, criteri 1.6 e 1.7 (MSI di Tauri con il servizio, updater). Tutto dentro la Sandbox.
# Uso: run-msi.sh build | 1.6 | 1.7
# Serve: Sandbox con l'agent (spikes/windows-lab/lab.sh start) e i job sandbox/install-toolchain.ps1 e app-warm.ps1.
# Non serve il laboratorio Docker: il server di latest.json gira nella Sandbox su 127.0.0.1:8765.
set -uo pipefail
cd "$(dirname "$0")"
SP="$(cd ../../.. && pwd)"; LAB="$SP/../windows-lab"
SBX=/mnt/c/Users/hackp/galleria-sbx
EVD="$SP/evidence/windows"; mkdir -p "$EVD" "$SBX/in/wgwin" "$SBX/in/msi" "$SBX/in/src"
job() { # job <script.ps1> [timeout]
  local n="m-$(basename "$1" .ps1)-$(date +%s)"
  cp lib.ps1 "$SBX/in/wgwin/msi-lib.ps1"; cp httpd.ps1 light-sval.rs "$SBX/in/wgwin/"; cp "$1" "$SBX/in/jobs/$n.ps1"
  for _ in $(seq 1 "${2:-300}"); do [ -f "$SBX/out/jobs/$n.done" ] && break; sleep 1; done
  tr -d '\r' < "$SBX/out/jobs/$n.log"; [ -s "$SBX/out/jobs/$n.err" ] && { echo "--- stderr"; tr -d '\r' < "$SBX/out/jobs/$n.err"; }
  [ -f "$SBX/out/jobs/$n.done" ] || echo "[JOB TIMEOUT]"
}
case "$1" in
  build)
    # servizio 0.0.1 dal crate, 0.0.2 da una copia con la sola versione cambiata; wireguard.dll dallo zip ufficiale
    bash "$LAB/xbuild/xbuild.sh" "$SP/galleriad" && cp "$SP/galleriad/target/x86_64-pc-windows-gnu/release/galleriad.exe" "$SBX/in/msi/galleriad-0.0.1.exe"
    T="$(mktemp -d)"; rsync -a "$SP/galleriad/" "$T/"; sed -i 's/^version = "0.0.1"/version = "0.0.2"/' "$T/Cargo.toml"; sed -i '/name = "galleriad"/{n;s/0.0.1/0.0.2/}' "$T/Cargo.lock"
    bash "$LAB/xbuild/xbuild.sh" "$T" && cp "$T/target/x86_64-pc-windows-gnu/release/galleriad.exe" "$SBX/in/msi/galleriad-0.0.2.exe"
    (cd "$SBX/in/msi" && unzip -o -j -q ../dl/wireguard-nt-0.10.1.zip 'wireguard-nt/bin/amd64/wireguard.dll')
    rsync -a --delete --exclude target --exclude gen "$SP/app" "$SBX/in/src/"
    job build.ps1 2400 > "$EVD/1.6-1.7-build.txt"
    [ -f "$SBX/out/msi/build-0.0.1-ice.log" ] && tr -d '\r' < "$SBX/out/msi/build-0.0.1-ice.log" > "$EVD/1.6-1.7-build-ice-failure.txt" ;;
  1.6) job c-1.6.ps1 400 > "$EVD/1.6.txt"   # estratti .txt: i *.log sono ignorati da git
       for f in install uninstall; do iconv -f UTF-16 -t UTF-8 "$SBX/out/msi/1.6-$f.log" 2>/dev/null | tr -d '\r' | grep -i 'galleriad\|InstallServices\|StartServices\|StopServices\|DeleteServices\|Installation success\|Removal success\|return value 3' > "$EVD/1.6-msi-$f.txt"; done ;;
  1.7) job c-1.7.ps1 900 > "$EVD/1.7.txt" ;;
  *) echo "uso: run-msi.sh build | 1.6 | 1.7"; exit 64 ;;
esac
echo "$1 done"
