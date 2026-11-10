# Spike 2 — management interface OpenVPN (parte Linux)

Codice usa e getta. Esito e conclusioni in [`docs/04-esito-spike-2-linux.md`](../../docs/04-esito-spike-2-linux.md).

## Cosa c'è

- `src/` — `galleriad-spike`, il demone ridotto all'osso:
  - `connect`: legge da stdin la richiesta che la GUI manderebbe via IPC (profilo + segreti, JSON), ripulisce il
    profilo (M2), genera la configurazione e lancia `openvpn --config /dev/stdin` (la config passa da una pipe,
    mai da disco); pilota OpenVPN solo dalla management interface (socket Unix in `/run/galleria/s<pid>/`, 0700 root,
    `--management-client-user root`); risponde a `>PASSWORD:` (utente/password, OTP `static-challenge`,
    passphrase della chiave), firma `>PK_SIGN:` con `key_mode=external`; stampa gli eventi come righe JSON.
  - `netcfg <socket>`: il programma fisso usato come `--up`/`--down`; passa l'ambiente di OpenVPN
    (`foreign_option_*`, `dns_*`, `route_*`) al demone, che applica DNS (systemd-resolved) e blocco IPv6.
- `lab/` — laboratorio Docker: server OpenVPN 2.6 (`gl-server` con `dhcp-option`, `gl-server-dns` con l'opzione
  `dns`), un "sito" dual-stack fuori dalla LAN (`gl-target`, fa anche da router), tre client con systemd-resolved:
  `gl-client26` (Ubuntu 24.04, OpenVPN 2.6.19), `gl-client27` (Ubuntu 26.04, 2.7.0), `gl-client79` (Fedora 45, 2.7.9).
- `evidence/ovpn26|ovpn27|ovpn79/` — le prove: `<criterio>.txt` (comandi e output), `<criterio>-*.log` (log del
  demone: eventi JSON + output di OpenVPN), `2.8-*.pcap` (catture su `eth0`).

## Rifare le prove

```
cargo build --release
docker build -t galleria-lab-server -f lab/server.Dockerfile lab
docker build -t galleria-lab-client26 --build-arg BASE=ubuntu:24.04 -f lab/client.Dockerfile lab
docker build -t galleria-lab-client27 --build-arg BASE=ubuntu:26.04 -f lab/client.Dockerfile lab
docker build -t galleria-lab-clientfedora -f lab/client-fedora.Dockerfile lab
bash lab/gen-pki.sh && bash lab/make-profiles.sh && bash lab/up.sh
bash lab/run-criteria.sh 26        # oppure 27, 79; si può limitare: run-criteria.sh 27 2.6 2.8
bash lab/probe.sh 27 nome profiles/base.ovpn username=alice password=alice-secret ipv6=native6
```

Tutto gira nei container (i client sono `--privileged` per systemd); l'host non viene toccato.
Pulizia: `docker rm -f gl-server gl-server-dns gl-target gl-client26 gl-client27 gl-client79; docker network rm galleria-lab`.
