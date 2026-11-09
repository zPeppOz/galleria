# 01 — Analisi tecnico-funzionale

Versione 0.3 — 2026-11-09 — stato: bozza da discutere con Nello (CTO)
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
  per OpenVPN (`static-challenge`/`auth-user-pass`) **[S: copertura da verificare]**.
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
  - Protocollo IPC: JSON-RPC o gRPC (tonic) — **da decidere con Nello**.
- **Backend WireGuard**:
  - Linux: modulo del kernel (in mainline) via netlink **[S: valutare il crate `defguard_wireguard_rs`
    e la sua licenza]**; fallback userspace (`wireguard-go`, MIT **[V]**, o `boringtun` **[S]**).
  - Windows: **WireGuardNT** (`wireguard.dll`, API in-kernel) **[V]**. La DLL precompilata ha una licenza
    propria che consente la ridistribuzione solo se usata tramite l'API di `wireguard.h` **[V]**
    (vedi rischio R3).
- **Backend intercambiabili** (proposta, A9): ogni protocollo implementa la stessa interfaccia nel demone
  (connetti, disconnetti, stato, statistiche, log), così aggiungere OpenConnect o AmneziaWG non tocca GUI,
  IPC e vincolo "una attiva".
- **Backend OpenVPN**: binario `openvpn` 2.6 lanciato come processo figlio e pilotato tramite la
  **management interface** (socket locale con password). Su Linux si dipende dal pacchetto della distro;
  su Windows va incluso con il suo driver (ovpn-dco-win / wintun) **[S: modalità e licenze dei driver da verificare]**.
- **Archivio profili**: metadati e configurazioni non segrete in un file per utente (TOML o SQLite);
  le parti segrete (chiavi private, password, chiavi inline) estratte all'import e salvate nel
  **portachiavi dell'utente**. Al connect la GUI le passa al demone, che non le scrive su disco
  (per OpenVPN: file temporaneo con permessi 0600 o passaggio via management interface **[S]**).

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
- **Aggiornamenti** (A13): la GUI usa il plugin updater di Tauri, che richiede sempre la firma degli
  aggiornamenti con una chiave nostra **[V]**; se la chiave privata si perde, chi ha l'app installata non riceve
  più aggiornamenti **[V]** → chiave nei secret di GitHub Actions + copia offline. GUI e demone vanno aggiornati
  insieme: il protocollo IPC ha un numero di versione e la GUI rifiuta un demone incompatibile con un messaggio chiaro.
  Proposta: Windows aggiorna MSI completo (GUI + servizio): l'updater Tauri riusa l'MSI standard **[V]** e l'MSI si
  estende con frammenti WiX **[V]**, quindi il servizio va dichiarato in un frammento (ServiceInstall/ServiceControl
  **[S: da provare nello spike Windows]**); Linux nessun auto-aggiornamento in v0.1 (avviso "nuova versione" + pacchetti), repository
  APT/DNF in fase 2.
- **IPv6** (A10): un profilo che manda tutto il traffico IPv4 nel tunnel ma non ha rotte IPv6 lascerebbe uscire
  il traffico IPv6 dalla rete locale. Proposta: già in v0.1, per i profili "tutto nel tunnel", bloccare l'IPv6 non
  instradato nel tunnel (rotta `::/0` verso il tunnel). È più semplice del kill switch completo, che resta in fase 2.
- **Telemetria** (A11): proposta **nessuna**. Niente analytics né crash report automatici; per le segnalazioni
  l'utente esporta il log dalla GUI (con i segreti già filtrati). Va scritto nel README: per un client VPN è un argomento.
- **Modello delle minacce**: vedi [`02-modello-minacce.md`](02-modello-minacce.md).
- **Matrice di test** (A12), proposta (sotto). Backend DNS da coprire su Linux: systemd-resolved, NetworkManager, resolvconf (R1).

| Piattaforma | Versioni | Note |
|---|---|---|
| Ubuntu | LTS corrente e precedente | GNOME mostra la tray solo con l'estensione AppIndicator, inclusa in `ubuntu-desktop-minimal` (24.04) **[V]** |
| Fedora | corrente | GNOME, systemd-resolved; estensione AppIndicator non preinstallata **[S]** → la tray è un accessorio, l'app deve essere completa senza |
| Debian | stabile corrente | |
| Arch | rolling | solo community (AUR in fase 3), nessuna garanzia |
| KDE Plasma | su una delle distro sopra | tray nativa |
| Windows | 11 | Windows 10 fuori supporto Microsoft dal 2025-10-14 **[V]**: best effort |

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
| 0 | Validazione con Pritunl/Eddie, nome, licenza, scelta IPC; spike WireGuardNT + management interface OpenVPN | 1 settimana |
| 1 | Demone Linux + WireGuard + OpenVPN, CLI minima | 1–2 settimane |
| 2 | GUI rubrica + import + connetti/disconnetti + tray | 2 settimane |
| 3 | Porting Windows (servizio, named pipe, WireGuardNT, OpenVPN incluso), MSI | 1–2 settimane |
| 4 | Rifinitura, documentazione, release v0.1 pubblica | 1 settimana |

**Criterio di stop**: tetto di ~8 settimane per arrivare alla v0.1. Dopo il lancio pubblico
(r/selfhosted, r/WireGuard, Hacker News) si valuta se continuare in base a uso reale e segnali
(stelle, issue, contributori) — soglie da fissare (A6).

## 11. Rischi

| ID | Rischio | Impatto | Mitigazione |
|---|---|---|---|
| R1 | DNS, rotte e kill switch si comportano in modo diverso su ogni OS/distro (systemd-resolved, resolvconf, NetworkManager) | Alto: è dove nascono la maggior parte dei bug | Backend DNS per piattaforma con test su più distro; kill switch solo in fase 2 |
| R2 | Su Linux il demone può entrare in conflitto con NetworkManager | Medio | Interfacce marcate come non gestite da NM; documentare |
| R3 | `wireguard.dll` precompilata ha una licenza non OSI; SignPath chiede che nessun componente sia proprietario | Medio: potrebbe bloccare la firma gratuita | Verificare con SignPath; alternativa: compilare WireGuardNT da sorgente o usare `wireguard-go` + wintun |
| R4 | Ridistribuzione di OpenVPN e driver su Windows (firme, licenze, aggiornamenti di sicurezza) | Medio | Valutare l'uso dell'installer ufficiale OpenVPN come prerequisito |
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
| A3 | IPC: JSON-RPC su socket/pipe o gRPC (tonic) | Nello | DA DECIDERE |
| A4 | Windows: includere OpenVPN o richiederlo come prerequisito | Nello | DA DECIDERE |
| A5 | Storage profili: TOML (leggibile, versionabile) o SQLite | Nello | DA DECIDERE |
| A6 | Soglie del criterio di stop dopo il lancio | Peppe | DA DECIDERE |
| A7 | Lista dei difetti di Pritunl/Eddie (validazione) | Peppe | PARZIALE: Pritunl = UI spartana + WireGuard solo con server Pritunl (verificato su forum); Eddie e prova pratica non fatti |
| A8 | Design system: riuso di xr1 design-system (verificarne la licenza per un progetto OSS) | Peppe | DA DECIDERE |
| A9 | Interfaccia comune dei backend nel demone già dalla v0.1, e OpenConnect come libreria (`libopenconnect`) o come processo figlio | Nello | DA DECIDERE |
| A10 | Blocco IPv6 per i profili "tutto nel tunnel" già in v0.1 (§8) | Nello | DA DECIDERE |
| A11 | Nessuna telemetria (§8) | Peppe | DA DECIDERE |
| A12 | Matrice di test (§8) | Nello + Peppe | DA DECIDERE |
| A13 | Canale e meccanismo degli aggiornamenti (§8) | Nello | DA DECIDERE |

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
- Homebrew 5.0 e cask non firmati (per la fase macOS): https://brew.sh/2025/11/12/homebrew-5.0.0/ , https://workbrew.com/blog/homebrew-5-0-0
