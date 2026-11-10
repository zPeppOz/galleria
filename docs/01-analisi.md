# 01 — Analisi tecnico-funzionale

Versione 0.4 — 2026-11-09 — stato: decisioni tecniche D10–D16 proposte da Nello (CTO), da ratificare da Peppe
Nome: **Galleria** (D7).

Legenda: **[V]** verificato su fonte letta (link in fondo) · **[S]** supposto, da verificare.

## 1. Obiettivo

Client desktop open source per gestire una rubrica di connessioni VPN, partendo da **OpenVPN** e **WireGuard**
e arrivando alle principali VPN aziendali (D9, §4.1):
importare profili e certificati, organizzarli, configurarli e accenderne **una alla volta**,
con un'interfaccia in stile VirtualBox, veloce e curata.

Non obiettivi (MVP): server VPN, provider VPN integrati, macOS, mobile, sincronizzazione cloud,
monetizzazione.

## 2. Decisioni prese

| ID | Decisione | Fonte |
|---|---|---|
| D1 | Progetto **open source**, nessun obiettivo di ricavo diretto | Peppe, 2026-11-06 |
| D2 | Piattaforme MVP: **Linux e Windows**. macOS rinviato a quando il progetto ha trazione | Peppe, 2026-11-06 |
| D3 | Niente Apple Developer Program per ora (99$/anno) | Peppe, 2026-11-06 |
| D4 | Protocolli: OpenVPN e WireGuard | Peppe |
| D5 | **Una sola connessione attiva** alla volta, imposta dal demone | Peppe |
| D6 | UX stile VirtualBox: lista/rubrica a sinistra, dettaglio a schede a destra | Peppe |
| D7 | Nome: **Galleria** (scelto da Peppe il 2026-11-06). Scartati in precedenza: Connectio (tool OSS omonimo, ott. 2026), Connect.io (VPN aziendale), Varco (varco.io, prodotto di onboarding dispositivi), Tunnelbox (prodotto omonimo) | ricerca 2026-11-06 |
| D8 | Licenza **GPL-3.0-or-later**; repo pubblico `github.com/zPeppOz/galleria` | Peppe, 2026-11-06 |
| D9 | Obiettivo "compatibile con qualsiasi rete" nei due sensi: (a) più protocolli, partendo dalle VPN aziendali via OpenConnect; (b) funzionare anche su reti filtrate. Roadmap e esclusioni in §4.1 | Peppe, 2026-11-09 |
| D10 | (A9) **Interfaccia comune dei backend già dalla v0.1.** Ogni protocollo implementa le stesse operazioni nel demone: connetti, disconnetti, stato (eventi), statistiche, log, più un elenco di capacità (es. "ha l'handshake", "chiede l'OTP"). DNS, rotte e blocco IPv6 stanno in un modulo comune del demone: il backend gli passa quello che ha ricevuto dal server, dove il protocollo lo permette (per OpenVPN il canale è da provare nello spike). Due backend reali in v0.1 bastano a validare l'interfaccia; niente astrazioni pensate per protocolli che non abbiamo ancora | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D11 | (A9b) **OpenConnect come processo figlio**, non come libreria. Motivi: stesso schema di OpenVPN; niente FFI con `libopenconnect` da mantenere; i segreti passano da stdin (`--passwd-on-stdin`, `--cookie-on-stdin` **[V]**); la rete si configura con `--script` **[V]**, che punterà a un nostro programma fisso (mai uno script del profilo) che consegna rotte e DNS al modulo comune (D10). Su Windows `--script` gira con `cscript.exe` **[V]**: da valutare nello spike della v0.2, insieme al login SSO (`--external-browser` **[V]** o webview) | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D12 | (A3) **IPC con JSON-RPC 2.0**, un messaggio JSON per riga, su socket Unix (Linux) e named pipe (Windows). Gli eventi di stato sono notifiche JSON-RPC sulla stessa connessione. Il primo messaggio è `hello` con la versione del protocollo: se non è compatibile, la GUI mostra un errore chiaro (§8). Tipi delle richieste in un crate Rust condiviso da demone, GUI e CLI; il demone rifiuta campi sconosciuti e messaggi oltre una dimensione massima (M1). La webview non parla mai con il socket: passa dai comandi Tauri, in Rust. gRPC scartato: aggiunge `protoc` e codice generato alla build e non ci serve nulla che JSON-RPC non dia; JSON si ispeziona a mano durante il debug | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D13 | (A5) **Archivio profili in TOML**, un file per profilo (`profiles/<id>.toml`) più un `galleria.toml` per gruppi, ordine e impostazioni, nella cartella di configurazione dell'utente. Ogni file ha `schema_version` per le migrazioni. Scrittura atomica (file temporaneo + rename), così GUI e CLI non si corrompono i file a vicenda; esportare un profilo significa copiare un file. Il demone non legge questi file (M2): riceve la configurazione dalla GUI via IPC. SQLite solo se un giorno servono ricerche su migliaia di profili o il demone deve connettere senza GUI (fase 2, connessione automatica) | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D14 | (A4) **Windows: OpenVPN è un prerequisito in v0.1**, non incluso. L'utente installa l'MSI ufficiale (anche `winget install OpenVPNTechnologies.OpenVPN` **[V]**); il demone trova `openvpn.exe` dal valore `exe_path` in `HKLM\SOFTWARE\OpenVPN` **[V]** (chiave e cartella scrivibili solo dagli amministratori **[S: da controllare nello spike]**), richiede almeno la 2.6 e, se manca, la GUI spiega come installarlo. Motivi: le licenze permetterebbero di includerlo (§5, **[V]**), ma OpenVPN esce con una release circa ogni mese (2.7.5 → 2.7.9 tra luglio e novembre 2026 **[V]**) e ognuna diventerebbe una nostra release; SignPath firma solo binari costruiti da noi **[V]**; i driver li firma già OpenVPN. Costo: un passo in più per l'utente, e l'MSI ufficiale propone anche OpenVPN GUI, che va lasciato fuori **[S: selezione predefinita da controllare nello spike]**. Inclusione: da rivalutare dopo il lancio solo se gli utenti lo chiedono, non come impegno | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D15 | (A10) **Blocco IPv6 già in v0.1** per i profili "tutto nel tunnel" (WireGuard: `AllowedIPs` con `0.0.0.0/0`; OpenVPN: `redirect-gateway` nel profilo o ricevuto dal server) che non portano IPv6 nel tunnel. Un solo meccanismo nel modulo di rete comune (D10): tutto l'IPv6 globale va verso l'interfaccia del tunnel (`::/1` e `8000::/1`, più specifiche della rotta predefinita della rete locale) e lì viene scartato. Che WireGuard scarti i pacchetti fuori da `AllowedIPs` è **[S]**, da provare; per OpenVPN c'è anche la via nativa `redirect-gateway ipv6` + `block-ipv6`, che risponde con ICMPv6 "no route" **[V]**. Disattivabile per profilo. Criteri: nessun pacchetto IPv6 globale sull'interfaccia fisica durante la connessione, e le app ripiegano su IPv4 senza attese lunghe | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |
| D16 | (A13) **Aggiornamenti**: Windows con l'updater di Tauri, che installa l'MSI completo (GUI + servizio) firmato con la nostra chiave; Linux in v0.1 solo avviso "nuova versione" con link ai pacchetti, repository APT/DNF in fase 2. In più: (1) il controllo lo fa la GUI, mai il demone: una sola richiesta all'endpoint delle release su GitHub, senza identificativi, disattivabile nelle impostazioni e scritta nel README, perché è l'unico contatto di rete di Galleria fuori dalla VPN (coerente con A11); (2) OpenVPN su Windows non lo aggiorniamo noi (D14): la GUI avvisa se la versione installata è sotto la minima; (3) se lo spike Windows mostra che updater + servizio non funzionano, in v0.1 anche Windows ha solo l'avviso | Nello, 2026-11-09 — **proposta, da ratificare da Peppe** |

## 3. Differenziazione (perché esiste)

Rispetto a Pritunl/Eddie/Defguard (dettagli in `00-mercato-e-concorrenti.md`):
1. **Nessun legame con un server o provider**: importa `.ovpn` e `.conf` WireGuard qualsiasi.
2. **UX moderna e veloce**: app nativa leggera (Tauri, non Electron), cambio connessione in un click,
   tray con connessione rapida.
3. **Configurabilità per profilo** esposta in GUI (DNS, rotte, kill switch, riconnessione) invece che
   solo nel file di testo, con editor raw sempre disponibile.
4. **Un solo client per le VPN di tutti i clienti** (da v0.2, D9): chi lavora per più aziende oggi installa
   un client diverso per ciascuna (AnyConnect, GlobalProtect, Fortinet...). Galleria li mette nella stessa rubrica.
5. Pritunl, il concorrente più vicino: UI spartana (giudizio di Peppe) e WireGuard utilizzabile solo con server Pritunl (verificato). Le funzioni 1 e 2 sono quindi il cuore del prodotto: se la UX non è nettamente migliore, il progetto non ha motivo di esistere.

## 4. Funzionalità

### MVP (v0.1)
- **Import**: file `.ovpn` (anche con `<ca>`, `<cert>`, `<key>`, `<tls-auth>` inline o file separati),
  `.conf` WireGuard, drag & drop, apertura da file manager (associazione estensioni), incolla da testo / QR WireGuard **[S: QR da valutare]**.
- **Rubrica**: lista con gruppi/cartelle, tag, ricerca, preferiti, ordinamento, icona di stato.
- **Dettaglio a schede**: Generale (nome, protocollo, server, gruppo) · Autenticazione (utente/password,
  certificati, chiavi) · Rete (DNS, rotte, MTU) · Avanzate (editor raw con validazione) · Log.
- **Connessione**: connetti/disconnetti; se un'altra è attiva viene chiusa prima (D5); stato in tempo reale
  (connessione in corso, connesso, errore), IP assegnato, byte in/out, ultimo handshake (WireGuard).
- **Credenziali**: richiesta al connect con opzione "ricorda" nel portachiavi del sistema; supporto OTP
  per OpenVPN (`static-challenge`/`auth-user-pass`) **[V su Linux con `static-challenge`, spike 2 criterio 2.5; challenge dinamico `CRV1` non provato]**.
- **Tray**: stato, lista rapida, connetti/disconnetti. Su Linux Tauri non emette gli eventi di click sull'icona,
  funziona solo il menu **[V]**: tutte le azioni della tray stanno nel menu.
- **Riconnessione automatica** su caduta di rete.
- Duplica/esporta/elimina profilo.

### Fase 2
- Kill switch e protezione DNS leak (per profilo).
- Split tunneling per rotte/subnet (per app è molto più complesso, fuori).
- Avvio con il sistema, connessione automatica su reti non fidate.
- CLI `galleria` che parla con lo stesso demone (utile su SSH/server).
- Backup/ripristino della rubrica.

### Fase 3 (solo con trazione)
- macOS (demone launchd + `utun`, poi eventuale Network Extension a pagamento).
- Flatpak, AUR, winget.
- Protocolli aggiuntivi secondo la roadmap del §4.1.

### 4.1 Protocolli: roadmap (D9)

| Versione | Protocolli | Come | Note |
|---|---|---|---|
| v0.1 | OpenVPN, WireGuard | come §5 | invariato |
| v0.2 | Cisco AnyConnect, Palo Alto GlobalProtect, Fortinet FortiGate, Pulse Connect Secure, Juniper SSL VPN, F5 BIG-IP, Array Networks | **OpenConnect** (LGPL-2.1 **[V]**), un solo backend per sette protocolli aziendali **[V]**; gira anche su Windows **[V]** | controlli di conformità del PC: vedi R8. Login SSO/SAML nella webview di Tauri **[S: flusso da verificare in spike]** |
| v0.3 | AmneziaWG; OpenVPN su TCP 443 come opzione guidata | `amneziawg-go` (MIT **[V]**), fork di wireguard-go con offuscamento contro il DPI **[V]** | serve per reti filtrate (D9b) |
| solo su richiesta | IPsec/IKEv2 (anche L2TP/IPsec), SSTP | strongSwan su Linux (GPL-2.0-or-later, salvo i plugin des/md4/md5 **[V]**; compatibile con la nostra GPL-3.0-or-later), client nativi Windows **[S]** | DNS/rotte molto diversi per OS (R1) |
| esclusi | Tailscale/ZeroTier/NetBird/Nebula (mesh con server di controllo proprio), Shadowsocks/V2Ray/Trojan (proxy, altro prodotto), Zscaler/WARP/Check Point (proprietari), PPTP (insicuro) | — | — |

## 5. Architettura proposta

```
┌────────────────────────┐   IPC locale autenticato   ┌──────────────────────────────┐
│ GUI (Tauri 2)          │ ─────────────────────────▶ │ Demone galleriad (Rust)        │
│ React + TS, utente     │ ◀───────── eventi ──────── │ root / LocalSystem           │
│ normale, niente privil.│                            │ - stato e vincolo "1 attiva" │
└────────────────────────┘                            │ - backend WireGuard          │
         │ portachiavi utente                         │ - backend OpenVPN            │
         ▼                                            │ - DNS / rotte / kill switch  │
  Secret Service / Windows                            └──────────────────────────────┘
  Credential Manager
```

- **GUI**: Tauri 2 + React + TypeScript (stack di Peppe). Gira come utente normale.
- **Demone `galleriad`** in Rust, unico componente privilegiato:
  - Linux: unit **systemd**; IPC su socket Unix con controllo delle credenziali del chiamante
    (gruppo `galleria`).
  - Windows: **servizio Windows**; IPC su named pipe con ACL per gli utenti autorizzati.
  - Protocollo IPC: JSON-RPC 2.0, un messaggio per riga, con versione negoziata (D12, proposta).
- **Backend WireGuard**:
  - Linux: modulo del kernel (in mainline) via netlink. Candidato: il crate `defguard_wireguard_rs`,
    licenza **Apache-2.0 [V]** (file LICENSE e crates.io, versione 0.12.5 del 2026-11-04), compatibile con la
    nostra GPL-3.0-or-later **[V]** (dichiarazione Apache). Supporta anche WireGuardNT su Windows **[V]**, ma
    per Windows il README chiede la toolchain `windows-gnu` e MSYS2 **[V]**: da provare nello spike Windows.
    Fallback userspace (`wireguard-go`, MIT **[V]**, o `boringtun` **[S]**).
  - Windows: **WireGuardNT** (`wireguard.dll`, API in-kernel) **[V]**. La DLL precompilata ha una licenza
    propria che consente la ridistribuzione solo se usata tramite l'API di `wireguard.h` **[V]**
    (vedi rischio R3).
- **Backend intercambiabili** (D10, proposta): ogni protocollo implementa la stessa interfaccia nel demone
  (connetti, disconnetti, stato, statistiche, log), così aggiungere OpenConnect o AmneziaWG non tocca GUI,
  IPC e vincolo "una attiva". OpenConnect sarà un processo figlio come OpenVPN (D11, proposta).
- **Backend OpenVPN**: binario `openvpn` (2.6 o successivo; la versione corrente è la 2.7 **[V]**) lanciato come processo figlio e pilotato tramite la
  **management interface** (socket locale con password). Su Linux si dipende dal pacchetto della distro;
  su Windows dall'installer ufficiale, come prerequisito (D14, proposta). Licenze dei driver **[V]**:
  ovpn-dco-win (driver predefinito da OpenVPN 2.6) è MIT; tap-windows6 è GPLv2; `wintun.dll` precompilata ha la
  stessa licenza restrittiva di `wireguard.dll` (ridistribuibile solo se usata tramite l'API di `wintun.h`).
  L'MSI ufficiale di OpenVPN 2.7 installa solo ovpn-dco e tap-windows6, quello della 2.6 anche wintun **[V]**.
- **Archivio profili**: metadati e configurazioni non segrete in file TOML dell'utente, uno per profilo (D13, proposta);
  le parti segrete (chiavi private, password, chiavi inline) estratte all'import e salvate nel
  **portachiavi dell'utente**. Al connect la GUI le passa al demone, che non le scrive su disco
  (per OpenVPN: configurazione passata via pipe, `--config /dev/stdin`, **[V su Linux, spike 2 criterio 2.6]**; segreti
  via management interface **[V su Linux]**; su Windows **[S: parte Windows dello spike 2]**; esito in `04-esito-spike-2-linux.md`).

## 6. Modello dati (bozza)

- `Profile`: id, nome, protocollo (`openvpn` | `wireguard`, estendibile secondo §4.1), gruppo, tag[], preferito, note,
  config_ref (config non segreta), secret_refs[] (chiavi nel portachiavi), impostazioni
  (dns, rotte, mtu, kill_switch, auto_reconnect), creato/modificato, ultimo_uso.
- `Group`: id, nome, ordine, padre (annidamento opzionale).
- `ConnectionState` (solo nel demone, in memoria): profile_id, stato, ip, byte, handshake, errore, avviata_alle.
- `LogEntry`: profile_id, timestamp, livello, messaggio (buffer circolare + file di log ruotato).

## 7. Flussi chiave

1. **Import**: file → parser (ovpn/wg) → validazione → anteprima con avvisi (es. direttive non supportate,
   file esterni mancanti) → separazione dei segreti → salvataggio.
2. **Connetti**: click → se c'è una connessione attiva: disconnetti e attendi → recupera i segreti
   (prompt se mancano) → demone avvia il backend → eventi di stato in streaming → connesso / errore
   con messaggio leggibile e link al log.
3. **Errore/caduta**: il demone rileva la caduta → riconnessione con backoff → dopo N tentativi
   stato "errore", notifica.

## 8. Requisiti trasversali

- **Sicurezza**: GUI senza privilegi; il demone valida ogni richiesta e accetta solo chiamanti
  autorizzati; nessun segreto nei log; l'import non esegue mai direttive che lanciano script
  (`up`, `down`, `script-security` in OpenVPN vanno bloccate o confermate esplicitamente).
- **Prestazioni**: avvio GUI < 1 s, installer di pochi MB **[S: obiettivo, da misurare]**.
- **Accessibilità e i18n**: inglese e italiano dal primo giorno.
- **Qualità**: TDD sui parser e sulla macchina a stati del demone; test end-to-end su VM Linux e Windows
  con server OpenVPN e WireGuard di prova in container.
- **Distribuzione**: Linux `.deb` e `.rpm` (installano la unit systemd; AppImage non può installare un
  demone di sistema); Windows MSI che installa il servizio. Release automatiche con GitHub Actions.
- **Firma Windows**: SignPath Foundation (gratuita per OSS) **[V]**, se il progetto rispetta le condizioni (vedi R3).
- **Aggiornamenti** (A13, D16 proposta): la GUI usa il plugin updater di Tauri, che richiede sempre la firma degli
  aggiornamenti con una chiave nostra **[V]**; se la chiave privata si perde, chi ha l'app installata non riceve
  più aggiornamenti **[V]** → chiave nei secret di GitHub Actions + copia offline. GUI e demone vanno aggiornati
  insieme: il protocollo IPC ha un numero di versione e la GUI rifiuta un demone incompatibile con un messaggio chiaro (D12).
  Windows aggiorna l'MSI completo (GUI + servizio): l'updater Tauri riusa l'MSI standard **[V]** e l'MSI si
  estende con frammenti WiX **[V]**, quindi il servizio va dichiarato in un frammento (ServiceInstall/ServiceControl
  **[S: da provare nello spike Windows]**); Linux nessun auto-aggiornamento in v0.1 (avviso "nuova versione" + pacchetti), repository
  APT/DNF in fase 2. Il controllo delle versioni si può disattivare.
- **IPv6** (A10, D15 proposta): un profilo che manda tutto il traffico IPv4 nel tunnel ma non ha rotte IPv6 lascerebbe uscire
  il traffico IPv6 dalla rete locale. Già in v0.1, per i profili "tutto nel tunnel", l'IPv6 globale va verso il tunnel
  e lì viene scartato (dettagli in D15). È più semplice del kill switch completo, che resta in fase 2.
- **Telemetria** (A11): proposta **nessuna**. Niente analytics né crash report automatici; per le segnalazioni
  l'utente esporta il log dalla GUI (con i segreti già filtrati). Va scritto nel README: per un client VPN è un argomento.
- **Modello delle minacce**: vedi [`02-modello-minacce.md`](02-modello-minacce.md).
- **Matrice di test** (A12): proposta di Nello sotto, da decidere con Peppe. "Obbligatoria" vuol dire: test end-to-end
  a ogni release, e una release non esce se fallisce. "Best effort": si correggono i bug segnalati, nessun test fisso.
  Il punto che cambia di più tra le distro è la versione di OpenVPN (2.6 o 2.7), perché la management interface è il
  nostro percorso critico: per questo entrambe le Ubuntu LTS sono obbligatorie (Aitano ne indicava una).
  Backend DNS (R1): systemd-resolved con NetworkManager obbligatorio (è il caso di Ubuntu e Fedora desktop **[S]**);
  resolvconf best effort, su Debian.

| Piattaforma | Versioni | Livello | OpenVPN nei repository | Note |
|---|---|---|---|---|
| Ubuntu | 26.04 LTS e 24.04 LTS | obbligatoria | 2.7.0 (26.04), 2.6.19 (24.04) **[V]** | GNOME mostra la tray solo con l'estensione AppIndicator, inclusa in `ubuntu-desktop-minimal` (24.04) **[V]** |
| Fedora | corrente | obbligatoria | 2.7.9 (F44) **[V]** | GNOME, systemd-resolved; estensione AppIndicator non preinstallata **[S]** → la tray è un accessorio, l'app deve essere completa senza |
| Windows | 11 x64 | obbligatoria | installer ufficiale, ultima 2.7 (D14) | anche l'ultima 2.6 ma best effort |
| Debian | 13 (stabile) | best effort | 2.6.14 **[V]** | utile per resolvconf |
| KDE Plasma | su una delle distro sopra | best effort | — | tray nativa |
| Arch | rolling | best effort | — | solo community (AUR in fase 3), nessuna garanzia |
| Windows 10, ARM64 | — | best effort | — | Windows 10 fuori supporto Microsoft dal 2025-10-14 **[V]** |

## 9. Licenza

- Il codice del progetto può avere una licenza a scelta: OpenVPN (GPLv2 **[V]**) viene eseguito come
  **processo separato**, non linkato, quindi non impone la sua licenza **[S: lettura comune
  dell'aggregazione GPL, non un parere legale]**. Correzione rispetto alla chat: GPLv3 non è *obbligata*
  da OpenVPN.
- Scelta: **GPL-3.0-or-later** (D8), come Eddie: impedisce fork chiusi.

## 10. Piano a fasi e tempi

Stime part-time con gli agenti di coding, da rivedere dopo la fase 0.

| Fase | Contenuto | Stima |
|---|---|---|
| 0 | Validazione con Pritunl/Eddie, nome, licenza, scelta IPC; spike WireGuardNT + management interface OpenVPN (criteri in `03-sessione-nello.md` §3) | 1 settimana |
| 1 | Demone Linux + WireGuard + OpenVPN, CLI minima | 1–2 settimane |
| 2 | GUI rubrica + import + connetti/disconnetti + tray | 2 settimane |
| 3 | Porting Windows (servizio, named pipe, WireGuardNT, rilevamento di OpenVPN installato come prerequisito, D14), MSI | 1–2 settimane |
| 4 | Rifinitura, documentazione, release v0.1 pubblica | 1 settimana |

**Criterio di stop**: tetto di ~8 settimane per arrivare alla v0.1. Dopo il lancio pubblico
(r/selfhosted, r/WireGuard, Hacker News) si valuta se continuare in base a uso reale e segnali
(stelle, issue, contributori) — soglie da fissare (A6).

## 11. Rischi

| ID | Rischio | Impatto | Mitigazione |
|---|---|---|---|
| R1 | DNS, rotte e kill switch si comportano in modo diverso su ogni OS/distro (systemd-resolved, resolvconf, NetworkManager) | Alto: è dove nascono la maggior parte dei bug | Backend DNS per piattaforma con test su più distro; kill switch solo in fase 2 |
| R2 | Su Linux il demone può entrare in conflitto con NetworkManager | Medio | Interfacce marcate come non gestite da NM; documentare |
| R3 | `wireguard.dll` precompilata ha una licenza non OSI; SignPath chiede che nessun componente sia proprietario | Medio: potrebbe bloccare la firma gratuita | Chiedere per iscritto a SignPath. Attenzione: `wireguard-go` + wintun **non** risolve, perché `wintun.dll` precompilata ha la stessa licenza di `wireguard.dll` **[V]**; compilare i driver da sorgente richiede la firma driver di Microsoft, fuori portata. Se SignPath dice no, decide Peppe (vedi spike in `03-sessione-nello.md`) |
| R4 | Ridistribuzione di OpenVPN e driver su Windows (firme, licenze, aggiornamenti di sicurezza) | Medio | Licenze verificate **[V]** (§5): ridistribuire si può, il costo vero è seguire ogni release di sicurezza. SignPath firma solo binari costruiti da noi **[V]**. Mitigazione: installer ufficiale OpenVPN come prerequisito |
| R5 | Il demone privilegiato è una superficie di attacco | Alto | IPC autenticato, input validato, nessuno script da profili non confermati, audit prima della v1 |
| R6 | Mercato: Pritunl copre già molto, la differenza sta "solo" nella UX | Medio | Validazione fase 0, lancio presto, tetto di tempo |
| R7 | Tempo sottratto ai progetti che fatturano | Medio | Criterio di stop, lavoro delegato agli agenti |
| R8 | VPN aziendali con controllo di conformità del PC (Cisco CSD/HostScan, GlobalProtect HIP): OpenConnect li simula con script, ma un server può accettare il report e negare comunque l'accesso; CSD non è supportato su Windows **[V]** | Medio: alcune aziende non funzioneranno | Dichiararlo nel README, niente promesse di compatibilità totale; elenco "testato con" alimentato dagli utenti |
| R9 | Allargamento dello scope (D9) che sfora il tetto di 8 settimane | Alto | v0.1 resta OpenVPN + WireGuard; gli altri protocolli solo dopo il lancio |

## 12. Punti aperti

| ID | Domanda | Chi decide | Stato |
|---|---|---|---|
| A1 | Nome definitivo | Peppe | RISOLTA: Galleria (D7). Verifica marchio/dominio/nome repo ancora da fare |
| A2 | Licenza | Peppe | RISOLTA: GPL-3.0-or-later (D8) |
| A3 | IPC: JSON-RPC su socket/pipe o gRPC (tonic) | Nello | PROPOSTA NELLO: JSON-RPC 2.0 (D12), da ratificare da Peppe |
| A4 | Windows: includere OpenVPN o richiederlo come prerequisito | Nello | PROPOSTA NELLO: prerequisito in v0.1 (D14), da ratificare da Peppe |
| A5 | Storage profili: TOML (leggibile, versionabile) o SQLite | Nello | PROPOSTA NELLO: TOML, un file per profilo (D13), da ratificare da Peppe |
| A6 | Soglie del criterio di stop dopo il lancio | Peppe | DA DECIDERE |
| A7 | Lista dei difetti di Pritunl/Eddie (validazione) | Peppe | PARZIALE: Pritunl = UI spartana + WireGuard solo con server Pritunl (verificato su forum); Eddie e prova pratica non fatti |
| A8 | Design system: riuso di xr1 design-system (verificarne la licenza per un progetto OSS) | Peppe | DA DECIDERE |
| A9 | Interfaccia comune dei backend nel demone già dalla v0.1, e OpenConnect come libreria (`libopenconnect`) o come processo figlio | Nello | PROPOSTA NELLO: interfaccia comune subito (D10), OpenConnect processo figlio (D11), da ratificare da Peppe |
| A10 | Blocco IPv6 per i profili "tutto nel tunnel" già in v0.1 (§8) | Nello | PROPOSTA NELLO: sì (D15), da ratificare da Peppe |
| A11 | Nessuna telemetria (§8) | Peppe | DA DECIDERE |
| A12 | Matrice di test (§8) | Nello + Peppe | PROPOSTA NELLO (§8): obbligatorie Ubuntu 26.04 e 24.04, Fedora corrente, Windows 11 x64; il resto best effort. Da decidere con Peppe |
| A13 | Canale e meccanismo degli aggiornamenti (§8) | Nello | PROPOSTA NELLO: updater Tauri + MSI su Windows, solo avviso su Linux (D16), da ratificare da Peppe |

## Fonti

- WireGuardNT e licenza binari: https://git.zx2c4.com/wireguard-nt/plain/prebuilt-binaries-license.txt , https://github.com/WireGuard/wireguard-nt
- wireguard-windows (MIT, embeddable DLL service): https://github.com/WireGuard/wireguard-windows
- wireguard-go (MIT): https://github.com/WireGuard/wireguard-go
- OpenVPN COPYING (GPLv2 + eccezioni): https://github.com/OpenVPN/openvpn/blob/HEAD/COPYING
- SignPath Foundation, condizioni: https://signpath.org/terms.html
- Tauri updater (firma obbligatoria, perdita della chiave): https://v2.tauri.app/plugin/updater/
- OpenVPN 2.6 `--pull-filter` e `--management`: https://github.com/OpenVPN/openvpn/blob/release/2.6/doc/man-sections/client-options.rst , https://github.com/OpenVPN/openvpn/blob/release/2.6/doc/man-sections/management-options.rst
- Tauri: installer Windows e frammenti WiX https://v2.tauri.app/distribute/windows-installer/ ; tray https://v2.tauri.app/learn/system-tray/
- Ubuntu 24.04 `ubuntu-desktop-minimal` (include `gnome-shell-extension-appindicator`): https://packages.ubuntu.com/noble/ubuntu-desktop-minimal
- Windows 10 fine supporto: https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro
- OpenConnect (protocolli, LGPL-2.1): https://www.infradead.org/openconnect/ ; piattaforme e CSD su Windows: https://www.infradead.org/openconnect/platforms.html ; CSD: https://www.infradead.org/openconnect/csd.html ; HIP: https://www.infradead.org/openconnect/hip.html
- AmneziaWG (MIT, fork di wireguard-go): https://github.com/amnezia-vpn/amneziawg-go
- strongSwan: https://docs.strongswan.org/docs/latest/howtos/introduction.html , https://github.com/strongswan/strongswan , licenza: https://github.com/strongswan/strongswan/blob/master/LICENSE
- OpenConnect, opzioni da riga di comando (`--passwd-on-stdin`, `--cookie-on-stdin`, `--script`, `--external-browser`): https://www.infradead.org/openconnect/manual.html
- `defguard_wireguard_rs` (Apache-2.0, requisiti Windows): https://github.com/DefGuard/wireguard-rs/blob/main/LICENSE , https://crates.io/crates/defguard_wireguard_rs , https://github.com/DefGuard/wireguard-rs ; compatibilità Apache-2.0 con GPLv3: https://www.apache.org/licenses/GPL-compatibility.html
- ovpn-dco-win (MIT): https://github.com/OpenVPN/ovpn-dco-win/blob/multipeer/COPYRIGHT.MIT ; tap-windows6 (GPLv2): https://github.com/OpenVPN/tap-windows6/blob/master/COPYING ; wintun (sorgente GPL-2.0, DLL con licenza propria): https://www.wintun.net/ , https://git.zx2c4.com/wintun/plain/prebuilt-binaries-license.txt
- MSI ufficiale OpenVPN (driver inclusi, chiave di registro `exe_path`): https://github.com/OpenVPN/openvpn-build/blob/master/windows-msi/msi.wxs , https://github.com/OpenVPN/openvpn-build/blob/release/2.6/windows-msi/msi.wxs , https://github.com/OpenVPN/openvpn-gui/blob/master/README.rst
- Release OpenVPN (cadenza, versione corrente): https://github.com/OpenVPN/openvpn/releases ; pacchetto winget: https://github.com/microsoft/winget-pkgs/tree/master/manifests/o/OpenVPNTechnologies/OpenVPN
- OpenVPN 2.6 `--block-ipv6` e `--redirect-gateway ipv6`: https://github.com/OpenVPN/openvpn/blob/release/2.6/doc/man-sections/vpn-network-options.rst
- Versioni di OpenVPN nelle distro: Ubuntu https://launchpad.net/ubuntu/+source/openvpn ; Debian https://sources.debian.org/src/openvpn/ ; Fedora https://mdapi.fedoraproject.org/f44/pkg/openvpn
- Homebrew 5.0 e cask non firmati (per la fase macOS): https://brew.sh/2025/11/12/homebrew-5.0.0/ , https://workbrew.com/blog/homebrew-5-0-0
