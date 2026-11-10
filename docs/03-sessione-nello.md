# 03 — Scaletta sessione con Nello (CTO)

Obiettivo: chiudere le decisioni tecniche che bloccano la fase 0 e assegnare gli spike. Durata: 60 minuti.
Prima di arrivare: leggere `01-analisi.md` §4.1, §5, §8, §11 e `02-modello-minacce.md`.

Le raccomandazioni sono di Aitano (CEO): la decisione tecnica è di Nello, quella finale di Peppe.

## 1. Contesto (5')

- Scope v0.1 invariato: OpenVPN + WireGuard, Linux + Windows, tetto ~8 settimane.
- Novità D9: roadmap protocolli (OpenConnect in v0.2, AmneziaWG in v0.3). Impatto sulla v0.1: solo l'architettura (A9).

## 2. Decisioni (40')

| ID | Domanda | Opzioni | Raccomandazione | Perché |
|---|---|---|---|---|
| A9 | Backend intercambiabili | interfaccia comune subito · solo quando serve | **subito** | costa poco ora, evita di riscrivere demone e IPC in v0.2 |
| A9b | OpenConnect | `libopenconnect` · processo figlio | **processo figlio** per partire | stesso schema di OpenVPN; meno FFI da mantenere |
| A3 | IPC | JSON-RPC su socket/pipe · gRPC (tonic) | **JSON-RPC** | un solo client (la GUI + CLI), facile da ispezionare e testare; gRPC aggiunge build e dipendenze |
| A5 | Archivio profili | TOML · SQLite | **TOML** | leggibile, versionabile, backup banale; volumi piccoli |
| A4 | OpenVPN su Windows | incluso · prerequisito | **prerequisito in v0.1**, inclusione dopo | elimina R4 (licenze driver, aggiornamenti di sicurezza) dal percorso critico; costo: un passo in più per l'utente |
| A10 | Blocco IPv6 in v0.1 | sì · rinviare al kill switch | **sì** | è la fuga più comune e la correzione è piccola |
| A13 | Aggiornamenti | vedi §8 | Windows: updater Tauri; Linux: solo avviso in v0.1 | evita di gestire un repository di pacchetti adesso |
| A12 | Matrice di test | vedi §8 | Ubuntu LTS, Fedora, Windows 11 obbligatori; il resto best effort | dimensione realistica per una persona più gli agenti |


### Esito (Nello, 2026-11-09) — proposte, da ratificare da Peppe

Dettagli e motivi in `01-analisi.md` §2.

- A9 → D10: interfaccia comune subito, con DNS/rotte/IPv6 in un modulo comune del demone. D'accordo.
- A9b → D11: OpenConnect processo figlio, rete configurata da un nostro programma fisso via `--script`. D'accordo.
- A3 → D12: JSON-RPC 2.0, un messaggio per riga, `hello` con versione. D'accordo.
- A5 → D13: TOML, un file per profilo, scrittura atomica. D'accordo.
- A4 → D14: OpenVPN prerequisito su Windows. D'accordo sul prerequisito; **non** sull'"inclusione dopo" come impegno: le licenze la permettono, ma ogni release di OpenVPN (circa una al mese) diventerebbe una nostra release. Si rivaluta solo se gli utenti lo chiedono.
- A10 → D15: sì, blocco IPv6 in v0.1. D'accordo sul sì; la correzione è piccola nel codice ma va provata su ogni piattaforma, quindi entra nei criteri degli spike. Aggiornamento 2026-11-10: D15 corretta dopo lo spike 2 e **approvata da Peppe** (OpenVPN con la via nativa più un `ifconfig-ipv6` locale; modulo comune con errore immediato, non scarto silenzioso).
- A13 → D16: d'accordo, più tre aggiunte: controllo versioni dalla GUI e disattivabile (A11), avviso se OpenVPN è troppo vecchio, ripiego "solo avviso" anche su Windows se lo spike fallisce.
- A12: proposta in `01-analisi.md` §8. Differenza da Aitano: tutte e due le Ubuntu LTS obbligatorie (26.04 ha OpenVPN 2.7, 24.04 la 2.6).

Due correzioni alle premesse della scaletta:
- L'alternativa `wireguard-go` + wintun **non** risolve R3: `wintun.dll` precompilata ha la stessa licenza di `wireguard.dll` **[V]**. Serve solo se WireGuardNT non funziona tecnicamente.
- La versione corrente di OpenVPN è la 2.7 (2.7.9, novembre 2026) **[V]**, non la 2.6: lo spike va fatto su tutte e due.

## 3. Fase 0: spike e verifiche (10')

Durata della fase 0: 1 settimana, cioè spike 1 (3 giorni) + spike 2 (2 giorni). Gli spike producono codice da buttare
e una nota di esito in `docs/`. Un criterio è superato solo con una prova allegata (output di comando, cattura di
pacchetti, screenshot). Server WireGuard e OpenVPN di prova in container; client su VM (per Windows vedi "Ambiente Windows").

| Spike | Obiettivo | Durata massima | Se fallisce | Chi |
|---|---|---|---|---|
| 1. WireGuardNT su Windows | Il servizio Rust alza un tunnel WireGuard con WireGuardNT; l'MSI di Tauri installa e aggiorna il servizio | 3 giorni | vedi sotto | da assegnare |
| 2. Management interface OpenVPN | Il demone pilota `openvpn` 2.6 e 2.7 solo via management interface, senza segreti su disco, su Linux e Windows | 2 giorni | vedi sotto | da assegnare |
| ~~Licenza `defguard_wireguard_rs`~~ | FATTO 2026-11-09: Apache-2.0, compatibile con GPL-3.0-or-later → sì all'uso. Su Windows chiede la toolchain `windows-gnu`: da provare nello spike 1 | — | — | — |
| ~~`management ... unix` e `pull-filter` in OpenVPN 2.6~~ | FATTO 2026-11-09: entrambi presenti; il socket Unix va ristretto con `--management-client-user/group` | — | — | — |
| ~~Socket Unix della management interface su Windows (M5)~~ | FATTO 2026-11-09 sul sorgente: non esiste, su Windows solo TCP locale con password | — | — | — |
| ~~Updater Tauri con MSI + servizio Windows~~ | FATTO 2026-11-09 sulla doc: updater riusa l'MSI, servizio via frammento WiX; prova pratica nello spike 1 | — | — | — |

### Ambiente Windows (aggiornato 2026-11-10)

Gli spike Windows che installano programmi, driver o servizi, cambiano rotte o DNS, o disinstallano **non si fanno
sull'host di Peppe**: lì girano OpenVPN 2.7.4 (la VPN di lavoro), il client WireGuard ufficiale, Tailscale, ProtonVPN e
OpenVPN Connect, e uno spike che lascia un adattatore, un driver o una rotta sbagliati gli ferma il lavoro. Sull'host
solo verifiche in sola lettura (come le ACL di D14).

- Ambiente proposto: **Windows Sandbox**, da abilitare sull'host (Windows 11 Pro for Workstations). Parte pulita a ogni
  avvio e si butta alla chiusura: adatta a installazioni e disinstallazioni ripetute, ma le prove vanno copiate fuori
  prima di chiuderla.
- Ripiego: **VirtualBox**, già installato sull'host, con una VM Windows 11 x64 e uno snapshot prima di ogni prova.
- Da verificare prima di partire: IPv6 nella Sandbox **[V: non c'è, solo link-local; un IPv6 finto sulla scheda basta per contare i pacchetti, `05-esito-spike-windows.md` §2]** (serve una rete dual-stack per 1.5 e 2.8; se manca, si
  passa a VirtualBox); che nella Sandbox si installino driver e servizi (OpenVPN, WireGuardNT, MSI di Tauri) **[V per OpenVPN e WireGuardNT; MSI di Tauri non provato]**; come
  raggiunge i server di prova (`04-esito-spike-2-linux.md` §7 punto 2).

### Spike 1 — WireGuardNT su Windows

Ambiente: Windows 11 x64 in Windows Sandbox o in VM VirtualBox (vedi "Ambiente Windows"), rete dual-stack (IPv4 + IPv6).
Prima di iniziare: mail a SignPath con la domanda su `wireguard.dll` e `wintun.dll` (R3). La risposta non blocca lo
spike, ma blocca la scelta di chi firma.

| # | Criterio | Come si verifica |
|---|---|---|
| 1.1 | Il servizio `galleriad` (LocalSystem), scritto in Rust, carica `wireguard.dll` tramite l'API di `wireguard.h`, crea l'adattatore e alza il tunnel da un `.conf` di prova | ping a un indirizzo del server dentro il tunnel |
| 1.2 | Il servizio legge ultimo handshake e byte in/out ogni secondo | i valori crescono durante un ping continuo |
| 1.3 | Disconnessione pulita: adattatore rimosso, nessuna rotta residua | `Get-NetAdapter` e `route print` uguali a prima della connessione |
| 1.4 | Build: scelta tra `defguard_wireguard_rs` (toolchain `windows-gnu` **[V]**) e un binding diretto a `wireguard.h`; la scelta compila insieme all'app Tauri | job GitHub Actions verde su runner Windows |
| 1.5 | IPv6 (D15, meccanismo comune): profilo "tutto nel tunnel" solo IPv4, nessun pacchetto IPv6 globale esce dall'interfaccia fisica; l'IPv6 riceve un errore immediato (rotte unreachable o ICMPv6), non uno scarto silenzioso, e le app ripiegano su IPv4. Su Windows capire come ottenere l'errore immediato **[V: blocco WFP in uscita, errore in 12 ms; le rotte verso il tunnel non bastano, `05-esito-spike-windows.md` §5]** | cattura `pktmon` o Wireshark per 60 s; verso un sito dual-stack sia `curl` (Happy Eyeballs) sia un client che prova gli indirizzi uno dopo l'altro rispondono via IPv4 in meno di 2 s |
| 1.6 | MSI di Tauri con frammento WiX (ServiceInstall/ServiceControl): dopo l'installazione il servizio è avviato e automatico; la disinstallazione toglie servizio e adattatori | `sc qc galleriad`, `Get-Service galleriad`, `Get-NetAdapter` |
| 1.7 | Updater: da 0.0.1 a 0.0.2 con `latest.json` firmato, il servizio viene fermato, sostituito e riavviato; GUI e servizio hanno la stessa versione; un aggiornamento con firma sbagliata viene rifiutato | versione letta con `hello` (D12); log dell'updater |
| 1.8 | Named pipe minima con ACL: un utente locale non autorizzato riceve accesso negato (M1) | client di prova lanciato con `runas` da un secondo account |

Se fallisce:
- 1.1–1.3 (WireGuardNT da Rust non funziona): `wireguard-go` + wintun come processo figlio. Attenzione: non cambia R3, stessa licenza.
- 1.6 (servizio nell'MSI): installer NSIS di Tauri con hook di installazione **[S]**, oppure servizio registrato da un piccolo programma lanciato dall'installer.
- 1.7 (updater + servizio): D16 punto 3, in v0.1 anche Windows ha solo l'avviso "nuova versione".
- SignPath dice no: decide Peppe tra certificato a pagamento, v0.1 non firmata (avviso SmartScreen) o WireGuard su Windows rinviato. Lo spike non sceglie.

### Spike 2 — Management interface OpenVPN

Ambiente: Ubuntu 24.04 (OpenVPN 2.6.19 **[V]**), Ubuntu 26.04 o Fedora corrente (2.7.x **[V]**), Windows 11 x64 con
l'installer ufficiale 2.7 (D14), in Windows Sandbox o VM VirtualBox, mai sull'host di Peppe (vedi "Ambiente Windows").

| # | Criterio | Come si verifica |
|---|---|---|
| 2.1 | Il demone lancia `openvpn` come processo figlio con una configurazione generata da lui: `management`, `plugin`, `up`, `down`, `script-security` del profilo sono sempre tolti (M2) | profilo malevolo con un `up` che crea un file: il file non compare |
| 2.2 | Linux: socket Unix in una cartella 0700 di root, con `--management-client-user root`; un utente normale non si collega (M5) | `socat` da utente normale → errore |
| 2.3 | Windows (M5): porta TCP casuale su 127.0.0.1, password casuale per sessione con `pw-file stdin`; senza password l'accesso è negato; la password non compare nella riga di comando né su disco; un secondo client mentre il demone è collegato viene rifiutato **[V su Windows: resta senza risposta o riceve connessione rifiutata]**. Valutare anche `--management-client` (OpenVPN si collega al demone invece di ascoltare) **[V: l'opzione esiste]** | riga di comando con `Get-CimInstance Win32_Process`; ricerca della password nei file e in `%TEMP%`; connessione di prova da un altro processo |
| 2.4 | Stato: con `state on` arriva la sequenza fino a CONNECTED con l'IP assegnato; con `bytecount 1` i byte in/out ogni secondo, anche con DCO attivo | log degli eventi del demone |
| 2.5 | Credenziali senza file: utente/password su `>PASSWORD:Need 'Auth'`, OTP con `static-challenge`, passphrase della chiave privata | login riuscito sul server di prova con OTP |
| 2.6 | Chiavi inline (`<key>`, `<tls-auth>`) senza file in chiaro su disco. Provare, in ordine: configurazione passata via pipe **[V: stdin su Linux, named pipe su Windows]**, `--management-external-key` **[V: l'opzione esiste]**, ripiego di §5 (file temporaneo 0600, su Windows ACL solo SYSTEM, cancellato subito dopo l'avvio) | annotare quale funziona; chiude il [S] di `01-analisi.md` §5 |
| 2.7 | DNS e rotte spinte dal server arrivano al modulo di rete comune (D10). Provare il canale (per esempio un nostro programma fisso come `--up`, con le variabili `foreign_option_*` **[V su Linux; su Windows non arrivano]**); su Windows capire cosa applica OpenVPN da solo **[V: rotte e DNS `dhcp-option` sì; DNS dell'opzione `dns` no, senza il suo servizio interattivo: `05-esito-spike-windows.md` §5]** | DNS del server attivi: `resolvectl status` / `Get-DnsClientServerAddress` |
| 2.8 | IPv6 (D15, via nativa): stesso criterio di 1.5 con `redirect-gateway ipv6` + `block-ipv6` + un `ifconfig-ipv6` locale generato dal demone quando il server non spinge IPv6. Su Linux superato su 2.6.19, 2.7.0 e 2.7.9 (`04-esito-spike-2-linux.md`); resta Windows | cattura dei pacchetti, `curl` dual-stack e client sequenziale |
| 2.9 | Errori: password sbagliata → evento AUTH_FAILED → messaggio leggibile; server irraggiungibile → il demone riprova con backoff e si ferma dopo N tentativi | log del demone |
| 2.10 | Stesso codice su 2.6 e 2.7; le differenze sono annotate | esito dei punti sopra su tutte e tre le macchine |

Se fallisce:
- `pw-file stdin` non funziona quando OpenVPN è figlio del servizio Windows: file con la password, ACL solo SYSTEM, cancellato appena il demone si è collegato.
- Nessuna via per le chiavi inline senza disco: ripiego di §5, scritto nel modello delle minacce.
- 2.6 troppo diversa dalla 2.7: su Windows minimo 2.7 (lo controlliamo noi, D14); su Linux 2.6 con le differenze chiuse nel backend.
- La management interface non regge (eventi mancanti, byte assenti con DCO): stop e revisione con Peppe prima della fase 1, perché è il percorso critico di OpenVPN.

## 4. Chiusura (5')

- Decisioni scritte in `01-analisi.md` (§2 come D10–D16, §12 stato "PROPOSTA NELLO" finché Peppe non ratifica).
- Data di inizio della fase 0 e assegnazione dei due spike.
