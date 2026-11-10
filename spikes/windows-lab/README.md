# Laboratorio Windows degli spike (Windows Sandbox pilotata da WSL)

Codice usa e getta. Esito e conclusioni in [`docs/05-esito-spike-windows.md`](../../docs/05-esito-spike-windows.md).

Regola: **niente sull'host Windows**. Tutto ciò che installa programmi, driver o servizi o cambia rotte e DNS gira
dentro la Sandbox, che si butta alla chiusura. Sull'host si avvia e si chiude la Sandbox, e si scrivono file in
`C:\Users\hackp\galleria-sbx` (le due cartelle mappate: `in` in sola lettura per l'ospite, `out` scrivibile).

## Cosa c'è

- `lab.sh` — `start` (avvia la Sandbox con l'agent), `job <file.ps1>` (esegue uno script nell'ospite e ne stampa
  l'output), `relays` (riavvia i relay sui due lati), `stop` (`wsb stop`).
- `sandbox/agent.ps1`, `agent.wsb` — l'agent gira al logon come `WDAGUtilityAccount` (amministratore) ed esegue in
  ordine gli script che trova in `in\jobs`; output in `out\jobs`. `install-toolchain.ps1` e `app-warm.ps1` mettono
  nell'ospite Rust (host `windows-gnu`), w64devkit e `cargo-tauri` per la build dell'app Tauri.
- `filerelay/` — tra Sandbox e WSL non c'è una via di rete (provato nei due sensi, `evidence/relay-latency.txt`) e
  aprirne una vorrebbe dire toccare la rete dell'host. I datagrammi UDP di OpenVPN e WireGuard viaggiano quindi come
  record in due file della cartella scrivibile: nell'ospite il client VPN parla con `127.0.0.1:<porta>`, in WSL il
  relay consegna al server vero nel laboratorio Docker. Misurato: 100 datagrammi, 0 persi, ~18 ms andata e ritorno.
- `xbuild/` — immagine Docker con `mingw-w64` per compilare i binari Windows (`x86_64-pc-windows-gnu`) da WSL.

## Insidie trovate (tutte costate tempo)

- La cartella mappata in sola lettura mostra all'ospite i cambiamenti fatti dall'host con secondi di ritardo
  (handle aperto: fino a 4 s; elenco della cartella: decine di secondi). Sulla cartella scrivibile, riaprendo il
  file a ogni lettura, il ritardo è di pochi millisecondi. Il relay usa solo quella.
- `Start-Process -Wait` aspetta anche i processi figli: uno script che lascia un processo in background blocca
  l'agent. L'agent usa `WaitForExit()` sul solo processo.
- In PowerShell i nomi non distinguono maiuscole e minuscole (`$R` e `$r` sono la stessa variabile) e gli alias
  vincono sulle funzioni (`cli` è `Clear-Item`).
- Con `remote 127.0.0.1` e `redirect-gateway def1` una prova ha lasciato l'ospite senza TCP su loopback (mancava la
  rotta `127.0.0.1/32`); la causa non è stata isolata. I profili di prova hanno `redirect-gateway local`.
- **I client Linux del laboratorio dello spike 2 (`gl-client*`, privilegiati, con systemd) azzerano `binfmt_misc`
  di WSL**: sparisce `WSLInterop` e da WSL non parte più nessun `.exe` di Windows (nemmeno `wsb.exe`), per tutta
  la distribuzione. Per le prove Windows il laboratorio va avviato senza client: `CLIENTS= bash lab/up.sh`.
  Ripristino usato: da un container privilegiato con `/init` montato,
  `echo ':WSLInterop:M::MZ::/init:PF' > /proc/sys/fs/binfmt_misc/register` (oppure `wsl --shutdown`).
- Un server OpenVPN prende per rinegoziazione un nuovo login che arriva dalla stessa porta sorgente: il relay tiene
  un socket per ogni porta sorgente del client.

## Rifare le prove

```
bash spikes/windows-lab/xbuild/xbuild.sh spikes/windows-lab/filerelay && (cd spikes/windows-lab/filerelay && cargo build --release)
spikes/windows-lab/lab.sh start
CLIENTS= bash spikes/openvpn-mgmt/lab/up.sh && bash spikes/wireguard-nt/lab/up.sh
spikes/windows-lab/lab.sh relays
spikes/openvpn-mgmt/lab/win/run-win.sh setup && spikes/openvpn-mgmt/lab/win/run-win.sh 2.3      # 2.1 ... 2.9
bash spikes/wireguard-nt/lab/win/run-wg.sh 1.1                                                  # 1.5, 1.8
spikes/windows-lab/lab.sh stop
```

Pulizia: `docker rm -f gl-server gl-server-dns gl-target gl-wg gl-udpecho gl-binfmt-fix`.
