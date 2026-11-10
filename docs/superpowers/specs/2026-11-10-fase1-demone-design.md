# Fase 1 — Demone Linux `galleriad` + CLI minima: design

Versione 0.2 — 2026-11-10 — stato: proposta, da rivedere con Peppe prima di eseguire il piano
(`docs/superpowers/plans/2026-11-10-fase1-demone.md`). Rispetto alla 0.1: corretti i rilievi della review e
applicate le decisioni su Q3, Q6 e Q7 (§13).

Fonti: `docs/01-analisi.md` v0.6 (decisioni D1–D18, qui non riaperte; fa eccezione la parte Linux di D15 per
OpenVPN, cambiata dalla decisione su Q3: il testo dell'analisi va aggiornato, §13), `docs/02-modello-minacce.md` (M1–M7),
`docs/04-esito-spike-2-linux.md`, `docs/05-esito-spike-windows.md`, codice in `spikes/` (solo come riferimento:
non si modifica e non si importa come dipendenza).

Legenda come negli altri documenti: **[V]** verificato (spike o fonte letta) · **[S]** supposto, con il task del
piano che lo verifica (§12). Le sigle T1…T16 sono i task del piano.

## 1. Obiettivo e criteri di uscita

La fase 1 (`01-analisi.md` §10: "Demone Linux + WireGuard + OpenVPN, CLI minima") consegna:

1. `galleriad`, demone di sistema su Linux, unico componente privilegiato (§5 dell'analisi), che accende **una
   connessione alla volta** (D5) con due backend reali dietro la stessa interfaccia (D10): WireGuard nel kernel e
   OpenVPN via management interface.
2. Il protocollo IPC definitivo della v0.1 (D12), in un crate condiviso che la GUI della fase 2 userà senza modifiche.
3. `galleria`, CLI minima: importa un profilo, lo salva in TOML (D13), connette, disconnette, mostra stato e log.
4. Un laboratorio Docker e una CI che provano tutto questo a ogni push.

Criteri di uscita (tutti con prova automatica, salvo dove indicato in §12):

- E1. `galleria connect` alza un tunnel WireGuard e uno OpenVPN verso i server del laboratorio; il traffico passa.
- E2. Con una connessione attiva, una seconda `connect` è rifiutata o la sostituisce (`replace`): mai due backend vivi (D5).
- E3. OpenVPN: stesso codice su 2.6.19, 2.7.0 e 2.7.9; credenziali, OTP `static-challenge` e passphrase senza file;
  configurazione e segreti mai su disco (M1, criteri 2.5 e 2.6 dello spike).
- E4. DNS del profilo o del server applicati con systemd-resolved **prima** dello stato `connected` (M4).
- E5. Blocco IPv6 (D15, parte Linux) sui profili "tutto nel tunnel" senza IPv6: zero pacchetti IPv6 globali
  sull'interfaccia fisica e ripiego su IPv4 in meno di 2 s anche per un client senza Happy Eyeballs.
- E6. Dopo `disconnect`, e dopo `kill -9` del demone seguito dal riavvio, non restano interfacce, rotte, regole
  o DNS del demone.
- E7. Un utente fuori dal gruppo `galleria` non ottiene nulla dal socket (M1); un profilo con direttive pericolose
  è rifiutato dal demone (M2); nessun segreto noto compare nei log (M7).

## 2. Fuori scope

- **GUI** (fase 2), tray, import grafico, drag & drop, QR, associazione delle estensioni.
- **Windows** (fase 3): servizio, named pipe e ACL (D18), WireGuardNT, filtro WFP (D15 parte Windows), DNS dal
  `PUSH_REPLY` su Windows (D17), ricerca di `openvpn.exe` (D14). Il codice della fase 1 tiene i punti di
  innesto (§3.3) ma non compila né prova nulla per Windows.
- **Kill switch**, protezione DNS leak, split tunneling per app, connessione automatica, avvio con il sistema
  (elenco "Fase 2" del §4 dell'analisi). La finestra senza protezione durante la riconnessione resta (M4).
- **Packaging di rilascio** (`.deb`, `.rpm`, firma, repository): qui solo lo stretto necessario ai test (§9).
- **Aggiornamenti** (D16): il demone non contatta nessun servizio; le sole connessioni che apre sono quelle della VPN
  e la risoluzione del nome dell'endpoint.
- Portachiavi di sistema (Secret Service): entra con la GUI in fase 2 (§8.3, Q6 risolta).
- OpenVPN: `--management-external-key`, challenge dinamico `CRV1`, proxy con autenticazione da file, modo a chiave
  statica, TAP provato in laboratorio (ammesso dalla lista ma non provato).
- OpenConnect e gli altri protocolli del §4.1 dell'analisi. L'interfaccia dei backend non contiene nulla pensato per
  loro (D10: "niente astrazioni pensate per protocolli che non abbiamo ancora").
- resolvconf e sistemi senza systemd-resolved ("best effort" nella matrice A12): il demone lo dichiara e fallisce
  con un errore chiaro se il profilo ha DNS da applicare (§7.2).

## 3. Workspace Rust

### 3.1 Struttura

Workspace Cargo nella radice del repository, `resolver = "3"`, edizione 2024, licenza `GPL-3.0-or-later` (D8) in ogni
crate, `publish = false`. Toolchain fissata in `rust-toolchain.toml` a **1.96.0** (`defguard_wireguard_rs` chiede
almeno la 1.95 **[V]**). `Cargo.lock` committato. Runtime asincrono: **tokio** (un solo runtime nel demone; le
chiamate bloccanti a netlink e ai comandi di rete passano da `spawn_blocking`).

```
Cargo.toml  rust-toolchain.toml  deny.toml
crates/
  galleria-ipc/                tipi del protocollo, codec a righe, client asincrono (feature `client`)
  galleria-backend/            interfaccia comune dei backend (D10) e tipi che attraversano il confine; nessun I/O
  galleria-net/                modulo di rete comune: DNS, rotte, blocco IPv6, giornale e pulizia
  galleria-ovpn/               lettura dei file .ovpn e lista delle direttive ammesse; puro, nessun I/O
  galleria-backend-wireguard/  backend WireGuard (kernel, netlink)
  galleria-backend-openvpn/    backend OpenVPN (processo figlio + management interface)
  galleriad/                   demone: server IPC, autorizzazione, macchina a stati, sessione unica, log
  galleria-profile/            lato utente: profili TOML (D13), import, archivio dei segreti
  galleria-cli/                binario `galleria`
  galleria-e2e/                prove end-to-end da eseguire nel laboratorio (mai in `cargo test` normale)
lab/                           laboratorio Docker della fase 1 (§10.2)
packaging/linux/               unit systemd e file di sistema minimi (§9)
```

### 3.2 Confini (chi può dipendere da chi)

| Crate | Dipende da | Non deve |
|---|---|---|
| `galleria-ipc` | serde, serde_json, tokio (solo I/O; il client sta dietro la feature `client`) | conoscere backend, rete, file dei profili |
| `galleria-backend` | `galleria-ipc` (riusa `Stats`, `InputRequest`, `SessionError`, `Secret`), tokio (canali), ipnet | fare I/O |
| `galleria-net` | `galleria-backend` (solo `NetConfig`) | conoscere protocolli VPN o IPC |
| `galleria-ovpn` | nessun crate del workspace | fare I/O, conoscere il demone |
| `galleria-backend-wireguard` | `galleria-backend`, `defguard_wireguard_rs` | toccare DNS e rotte (li chiede a `galleria-net` tramite il demone) |
| `galleria-backend-openvpn` | `galleria-backend`, `galleria-ovpn` | applicare DNS da solo |
| `galleriad` | tutti i precedenti | dipendere da `galleria-profile`; leggere file indicati dal chiamante |
| `galleria-profile` | `galleria-ipc`, `galleria-ovpn`, toml | girare con privilegi |
| `galleria-cli` | `galleria-ipc` (`client`), `galleria-profile` | parlare con i backend |

Il vincolo "il demone non legge i file dei profili" (D13, M1) è reso verificabile: la CI fallisce se
`cargo tree -p galleriad` contiene `galleria-profile`, e la unit systemd ha `ProtectHome=yes` (§9).

### 3.3 Punti di innesto per la fase 3 (Windows)

Solo tre, senza codice Windows: il trasporto del client IPC è dietro una funzione `connect(endpoint)` (socket Unix
oggi, named pipe poi); `galleria-net` ha un trait `CommandRunner` e un piano di passi indipendente dalla piattaforma;
il backend OpenVPN isola in un modulo `plat` canale della management e passaggio della configurazione, che sono le
differenze elencate in `05-esito-spike-windows.md` §6.

## 4. Protocollo IPC (D12)

### 4.1 Trasporto e formato

- Socket Unix `SOCK_STREAM` in `/run/galleria/galleriad.sock` (costante `DEFAULT_SOCKET_PATH`).
- JSON-RPC 2.0, **un oggetto JSON per riga** (UTF-8, terminatore `\n`). Niente batch (un array è `invalid_request`).
- Dimensione massima di una riga: **1 048 576 byte** (`MAX_MESSAGE_BYTES`). Oltre: errore `message_too_large` e
  chiusura della connessione, senza leggere il resto.
- Il demone **rifiuta i campi sconosciuti** (`deny_unknown_fields` su ogni tipo di richiesta, D12/M1). Il client
  invece **ignora** i campi e le notifiche che non conosce: è quello che permette al demone di aggiungere dati
  alle risposte senza cambiare versione.
- `id`: numero intero o stringa. Il client non manda notifiche; una richiesta senza `id` è `invalid_request`.
- Limiti: 16 connessioni contemporanee; 5 s per mandare `hello` dopo la connessione, poi il demone chiude.
- I tipi stanno in `galleria-ipc`, condivisi da demone, CLI e (fase 2) GUI. Nomi dei campi in `snake_case`.

### 4.2 `hello` e versioni

Il primo messaggio di ogni connessione deve essere `hello`; ogni altro metodo prima di `hello` riceve
`hello_required`.

```json
→ {"jsonrpc":"2.0","id":1,"method":"hello","params":{"protocol":1,"client":{"name":"galleria-cli","version":"0.1.0"}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocol":1,"daemon_version":"0.1.0",
     "backends":[{"protocol":"wireguard","available":true,"version":null,"unavailable_reason":null,
                  "capabilities":{"handshake":true,"interactive_auth":false,"server_pushed_config":false}},
                 {"protocol":"openvpn","available":true,"version":"2.6.19","unavailable_reason":null,
                  "capabilities":{"handshake":false,"interactive_auth":true,"server_pushed_config":true}}],
     "status":{ ... §4.4 ... }}}
```

- La versione del protocollo è un intero (`PROTOCOL_VERSION = 1`). Il demone dichiara l'intervallo che parla
  (`PROTOCOL_MIN..=PROTOCOL_MAX`, oggi 1..=1). Se `params.protocol` è fuori intervallo risponde con l'errore
  `incompatible_protocol` (`data`: `daemon_min`, `daemon_max`, `daemon_version`) e chiude: il client mostra il
  messaggio chiaro chiesto da D12 e dal §8 dell'analisi ("aggiorna Galleria / riavvia il demone").
- Si alza la versione quando cambia il significato di un campo o se ne aggiunge uno a una **richiesta** (il demone
  rifiuta i campi sconosciuti); aggiungere campi a risposte e notifiche non la cambia.
- `capabilities` è l'elenco di capacità di D10. Un backend non disponibile (per esempio `openvpn` assente o più
  vecchio della 2.6) resta in elenco con `available: false` e `unavailable_reason` (`openvpn_not_found`,
  `openvpn_too_old`), così la GUI può spiegare cosa installare.

### 4.3 Metodi

| Metodo | Parametri | Risultato | Note |
|---|---|---|---|
| `hello` | `protocol`, `client{name,version}` | vedi sopra | obbligatorio per primo |
| `status` | `{}` | `Status` | fotografia dello stato |
| `connect` | `profile{id,name}`, `replace` (bool), `backend` (§4.5), `options` (§4.5) | `{"session": n, "status": Status}` | risponde appena la richiesta è accettata (stato `connecting`); l'esito arriva con `event.state` |
| `disconnect` | `{}` | `{"status": Status}` | risponde a smontaggio finito (al più 15 s, poi il backend è ucciso); idempotente |
| `provide_input` | `session`, `request`, `values{username?,password?,otp?,key_passphrase?}` | `{}` | risposta a uno stato `needs_input` |
| `subscribe` | `stats` (bool), `logs` (bool) | `{}` | `event.state` arriva sempre; statistiche e log solo a chi li chiede |
| `logs` | `after` (seq o null), `limit` (≤ 1000) | `{"entries":[LogEntry], "last": seq}` | dal buffer circolare (§5.4) |

Notifiche del demone (stessa connessione, senza `id`): `event.state` (`{"status": Status}` a ogni cambio di stato o
di `net`), `event.stats` (`Stats`, una al secondo mentre lo stato è `connected`), `event.log` (`LogEntry`).

### 4.4 Stato

```json
{"state":"connected","session":7,"profile":{"id":"3f0c…","name":"Ufficio Napoli"},"protocol":"openvpn",
 "since":"2026-11-10T10:42:03Z","attempt":1,"detail":null,
 "local_addresses":["10.8.0.2"],"remote":"172.31.0.10:1194",
 "net":{"interface":"glr-ovpn0","dns_servers":["10.8.0.1"],"search_domains":["galleria.test"],
        "routes":["0.0.0.0/1","128.0.0.0/1","192.0.2.0/24"],"full_tunnel":true,"ipv6_block":"unreachable"},
 "stats":{"rx_bytes":48213000,"tx_bytes":6120000,"last_handshake":null},
 "input_request":null,"error":null}
```

- `state`: `disconnected` · `connecting` · `needs_input` · `connected` · `reconnecting` · `disconnecting` · `error`.
  `error` è uno stato fermo senza backend vivo: resta finché arriva una `connect` o una `disconnect`, così chi si
  collega dopo vede perché la connessione è caduta (§7 flusso 3 dell'analisi).
- `detail`: testo libero di avanzamento durante `connecting` e `reconnecting` (per OpenVPN lo stato della
  management: `WAIT`, `AUTH`, `GET_CONFIG`, `ASSIGN_IP`, `ADD_ROUTES`, `RECONNECTING`).
- `net` è ciò che M3 chiede di mostrare all'utente: DNS e rotte effettivamente in uso, anche se spinti dal server.
  `ipv6_block`: `none` · `unreachable` (§7.3).
- `input_request` (solo in `needs_input`): `{"request": n, "kind": "credentials" | "otp" | "key_passphrase",
  "prompt": "Enter OTP", "echo": false, "expires_in_secs": 120}`.
- `error`: `{"code": "...", "message": "..."}` con i codici di sessione del §4.6.
- Nessuna risposta e nessuna notifica contiene mai configurazione o segreti (M1: un altro utente del gruppo non
  può leggere i segreti di chi ha connesso).

### 4.5 Parametri di `connect`

`backend` è un'unione etichettata dal campo `protocol`. I segreti sono stringhe nel JSON e nel codice sono del tipo
`Secret` (niente `Debug`, azzerati al rilascio).

WireGuard:

```json
{"protocol":"wireguard","private_key":"<base64>","addresses":["10.99.0.2/24"],
 "dns":["10.99.0.1"],"search_domains":[],"mtu":null,
 "peers":[{"public_key":"<base64>","preshared_key":null,"endpoint":"vpn.example.org:51820",
           "allowed_ips":["0.0.0.0/0"],"persistent_keepalive":25}]}
```

OpenVPN:

```json
{"protocol":"openvpn","config":"client\nremote 172.31.0.10 1194\n…<ca>…</ca>\n",
 "secret_blocks":{"key":"-----BEGIN PRIVATE KEY-----…","tls-auth":"…"},
 "username":"alice","password":"…","otp":null,"key_passphrase":null}
```

- `config` contiene solo direttive della lista ammessa (§6.3) e blocchi inline non segreti; i blocchi segreti
  viaggiano in `secret_blocks`, così il demone li registra nel filtro dei log e non li tratta mai come testo.
- Tutto ciò che manca tra `username`/`password`/`otp`/`key_passphrase` e che OpenVPN chiede porta a `needs_input`.

`options`: `{"ipv6_block": true, "ignore_pushed_routes": false, "reconnect": {"enabled": true}}`.
`ipv6_block: false` è il "disattivabile per profilo" di D15; `ignore_pushed_routes` è l'opzione di M3 (solo OpenVPN:
genera `pull-filter ignore "route "` e `pull-filter ignore "redirect-gateway"`).

Regola D5 su `connect`: se esiste una sessione in qualunque stato diverso da `disconnected` ed `error`, il demone
risponde `already_active` (con `data.profile`) salvo `replace: true`; con `replace` chiude la sessione corrente,
**aspetta la fine dello smontaggio** e poi avvia la nuova. Il vincolo sta nel demone; la GUI sceglie solo se
chiedere conferma prima di mandare `replace` (flusso 2 del §7 dell'analisi).

### 4.6 Errori

Errori JSON-RPC standard: `-32700` (JSON non valido), `-32600` (`invalid_request`), `-32601` (metodo sconosciuto),
`-32602` (`invalid_params`, compresi i campi sconosciuti). Errori applicativi con codice numerico `-32000` e codice
stabile a stringa in `data.code`; `message` è in inglese, la traduzione è del client:

| `data.code` | Quando |
|---|---|
| `hello_required`, `incompatible_protocol`, `message_too_large`, `too_many_clients` | handshake e trasporto |
| `already_active`, `not_active` | D5; `provide_input` senza sessione |
| `invalid_config` | parametri del backend non validi (chiave, CIDR, endpoint); `data.field` |
| `profile_rejected` | OpenVPN: direttive fuori lista; `data.directives: [{line, name, class}]` |
| `backend_unavailable` | `openvpn` assente o troppo vecchio; modulo `wireguard` non caricabile |
| `input_not_expected` | `provide_input` con `session`/`request` sbagliati o scaduti |
| `internal` | difetto del demone; il dettaglio va nel log, non al client |

Codici di sessione (in `Status.error.code`): `auth_failed` (mai nuovo tentativo), `input_timeout`,
`server_unreachable` (tentativi esauriti), `tunnel_setup_failed`, `net_setup_failed`, `dns_unavailable`,
`unsupported_challenge`, `backend_exited`.

## 5. Demone

### 5.1 Autorizzazione del chiamante (M1; analogo Linux di D18)

Due livelli, entrambi obbligatori:

1. **Permessi del socket**: cartella `/run/galleria` `0755 root:root`, socket `0660 root:galleria`. Il kernel
   rifiuta la `connect()` di chi non è root né nel gruppo. Le cartelle di sessione (`/run/galleria/s/<id>/`, socket
   della management e dell'helper; nome casuale, §7.4) sono `0700 root`.
2. **Credenziali del peer**: a ogni connessione il demone legge `SO_PEERCRED` (uid, gid, pid) e `SO_PEERGROUPS`
   (gruppi supplementari) e applica `authorize(creds, policy)`: passa uid 0 oppure chi ha il gid di `galleria` tra
   i propri gruppi. Chi non passa riceve la chiusura immediata, senza risposta. L'uid finisce nel log di audit
   ("uid 1000 ha chiesto connect/disconnect"). `SO_PEERCRED` si legge con `UnixStream::peer_cred()` di tokio;
   per `SO_PEERGROUPS` non c'è un'API sicura (verificato il 2026-11-10: né `rustix` 1.1.6 né `nix` 0.31.3 la
   espongono), quindi serve una `libc::getsockopt` in un solo modulo con `unsafe` ammesso (vincoli del piano).

Il gruppo si risolve una volta all'avvio; se non esiste, il socket resta `0600 root` e il demone lo scrive nel log.
Percorso del socket e nome del gruppo si possono cambiare da riga di comando (`--socket`, `--group`,
`--runtime-dir`): serve ai test, che girano senza privilegi su un socket temporaneo. Non c'è file di configurazione.

Tutti i membri del gruppo comandano la stessa, unica connessione della macchina e ne vedono stato e log (Q10).

### 5.2 Sessione unica (D5) e macchina a stati

Un solo task (`Manager`) possiede lo stato: le richieste IPC e gli eventi del backend gli arrivano da un canale e
vengono trattati uno alla volta. Il backend vivo è un `Option<SessionHandle>`: "una connessione attiva" è vero per
costruzione, non per controllo.

La logica è una funzione pura, provata con tabelle di casi (TDD, §8 dell'analisi):

```rust
pub fn step(phase: &Phase, input: Input, policy: &RetryPolicy) -> (Phase, Vec<Effect>);
```

| Stato | Evento | Nuovo stato | Effetti |
|---|---|---|---|
| `disconnected`/`error` | `connect` | `connecting{1}` | avvia backend |
| attivo (qualsiasi) | `connect` senza `replace` | invariato | risposta `already_active` |
| attivo (qualsiasi) | `connect` con `replace` | `disconnecting{poi connect}` | ferma backend, smonta rete |
| `connecting`/`reconnecting` | backend chiede un dato mancante | `needs_input` | timer 120 s |
| `needs_input` | `provide_input` | `connecting` (stesso tentativo) | inoltra al backend |
| `needs_input` | timer scaduto | `error{input_timeout}` | ferma backend, smonta rete |
| `connecting`/`reconnecting` | tunnel su e rete applicata | `connected` | avvia statistiche |
| `connecting`/`reconnecting` | rete non applicabile | `error{net_setup_failed}` o `{dns_unavailable}` | ferma backend, smonta rete |
| `connecting{n}` | backend uscito: trasporto, n < 4 | `connecting{n+1}` dopo l'attesa | smonta rete, timer |
| `connecting{4}` | backend uscito: trasporto | `error{server_unreachable}` | smonta rete |
| `connected` | backend uscito senza richiesta | `reconnecting{1}` | smonta rete, timer |
| `connected` | il backend segnala un riavvio interno (OpenVPN: `RECONNECTING`, §6.3) | `reconnecting{1}` | smonta rete; **nessun** backend nuovo, nessun timer: il processo è lo stesso |
| `reconnecting{n}` | backend uscito: trasporto, n < 8 | `reconnecting{n+1}` | smonta rete, timer |
| `reconnecting{8}` | backend uscito: trasporto | `error{server_unreachable}` | smonta rete |
| qualsiasi | backend uscito: autenticazione | `error{auth_failed}` | smonta rete, **nessun** nuovo tentativo |
| attivo (qualsiasi) | `disconnect` | `disconnecting` → `disconnected` | ferma backend, smonta rete |

Attese tra i tentativi: 1, 2, 4, 8, 16, 30, 30… secondi (`RetryPolicy`: 4 tentativi alla prima connessione come
nello spike 2 criterio 2.9, 8 dopo una caduta, tetto 30 s). Con `options.reconnect.enabled = false` una caduta porta
subito a `error{backend_exited}`. Ogni tentativo è un backend nuovo (per OpenVPN un processo nuovo, come nello spike).
Il demone tiene in memoria parametri e segreti della sessione per poter riprovare; li azzera alla fine della sessione.
Un OTP non si riusa: alla riconnessione il backend lo richiede e lo stato torna `needs_input`.

Un riavvio interno del backend (§6.3) non è un tentativo nuovo: lo stato lascia `connected` e passa a
`reconnecting{1}` con lo stesso backend; da lì valgono le righe di `reconnecting` (tunnel su e rete applicata →
`connected`; richiesta di un dato → `needs_input`; uscita del processo → `reconnecting{2}` con un backend nuovo).

Ordine di smontaggio, sempre lo stesso: prima `galleria-net` toglie DNS, regole e rotte (finché l'interfaccia esiste),
poi il backend chiude il tunnel. Mentre aspetta l'uscita del backend il `Manager` continua a servire i suoi eventi
(`NetDown` compreso, §6.1): l'helper `down` di OpenVPN riceve risposta anche durante l'arresto.

### 5.3 Processo, avvio, arresto

- Binario unico `galleriad` con due modi: `galleriad run` (predefinito) e `galleriad ovpn-hook <socket>` (§6.3).
- All'avvio, **prima** di aprire il socket: `prctl(PR_SET_DUMPABLE, 0)` (§9), `NetManager::recover()` e pulizia
  delle cartelle di sessione orfane (§7.4), poi ricerca di `openvpn` (`/usr/sbin/openvpn`, poi `/usr/bin/openvpn`;
  mai dal chiamante né dal profilo) e lettura della versione.
- `SIGTERM`/`SIGINT`: stessa sequenza di `disconnect`, poi uscita 0.
- Mai `SIGHUP` a OpenVPN (rilegge una configurazione che non c'è più, `04-esito-spike-2-linux.md` criterio 2.6).

### 5.4 Log e filtro dei segreti (M7)

- Buffer circolare in memoria di 2000 righe (`LogEntry`: `seq`, `ts`, `level`, `source` = `daemon` | `openvpn` |
  `wireguard` | `net`, `message`), svuotato all'inizio di ogni sessione. È ciò che servono `logs` ed `event.log`.
- Tutto passa da un `Redactor` **prima** di entrare nel buffer e in journald: sostituisce con `[redatto]` ogni
  valore segreto della sessione (chiavi, password, OTP, passphrase, blocchi segreti riga per riga) e, per
  struttura, i corpi dei blocchi PEM e gli argomenti di `password`/`username` nei comandi della management.
  **Nessuna lunghezza minima**: una password di uno o due caratteri si oscura come le altre (si salta solo la
  stringa vuota). Il prezzo è qualche riga di log con pezzi oscurati per coincidenza, accettato: un log meno
  leggibile è meglio di una password in chiaro. I valori più lunghi si sostituiscono per primi.
- Su Linux il log persistente è **journald** (stdout del servizio): nessun file di log proprio in fase 1 (Q8).

## 6. Backend

### 6.1 Interfaccia comune (D10)

In `galleria-backend`. Le operazioni di D10 (connetti, disconnetti, stato, statistiche, log, capacità) sono un
trait più due canali: comandi verso il backend, eventi verso il demone.

```rust
pub trait Backend: Send + 'static {
    fn protocol(&self) -> Protocol;
    fn capabilities(&self) -> Capabilities;
    /// Un tentativo di connessione. Finisce sempre con `BackendEvent::Exited`.
    fn spawn(self: Box<Self>, ctx: SessionContext) -> SessionHandle;
}
/// `session_dir`: cartella `0700 root` di questo tentativo, creata e poi rimossa dal demone (§7.4).
pub struct SessionContext { pub session: u64, pub session_dir: PathBuf, pub events: mpsc::Sender<BackendEvent> }
pub struct SessionHandle { pub commands: mpsc::Sender<BackendCommand>, pub task: JoinHandle<()> }

pub enum BackendCommand { Stop, ProvideInput(InputValues) }
pub enum BackendEvent {
    Progress(String),
    InputRequired(InputRequest),
    /// L'interfaccia esiste e il backend sa cosa applicare: il modulo comune applica la rete e risponde **subito**.
    /// Il demone non aspetta nient'altro dal backend prima di rispondere. `Up` non parte prima di questa risposta.
    NetUp { config: NetConfig, reply: oneshot::Sender<Result<(), String>> },
    /// L'interfaccia sta per sparire (OpenVPN: helper `down`): il modulo comune smonta la rete e risponde subito.
    NetDown { reply: oneshot::Sender<()> },
    /// Il tunnel porta traffico. Il backend lo manda solo dopo un `NetUp` accettato.
    Up(TunnelInfo),
    /// Il tunnel è caduto e il backend sta riprovando da solo, nello stesso processo (§6.3).
    Restarting(String),
    Stats(Stats),
    Log(LogLine),
    Exited(ExitReason),
}
pub enum ExitReason { Stopped, AuthFailed(String), Transport(String), Fatal(SessionError) }
```

`NetConfig` è "quello che il backend ha ricevuto dal server" (D10), indipendente dal protocollo:

```rust
pub struct NetConfig {
    pub interface: String,
    pub dns_servers: Vec<IpAddr>,
    pub search_domains: Vec<String>,
    pub routes: Vec<IpNet>,            // destinazioni che vanno nel tunnel
    pub install_routes: Option<u32>,   // Some(fwmark): le installa il modulo comune; None: già installate dal backend
    pub ipv6_block: Ipv6Block,         // None | Unreachable
}
pub fn is_full_tunnel_v4(routes: &[IpNet]) -> bool;   // 0.0.0.0/0 oppure 0.0.0.0/1 + 128.0.0.0/1
/// Vero se l'unione delle rotte copre tutto `2000::/3` (gli indirizzi globali): `::/0`, `::/1` + `8000::/1`,
/// `2000::/3`, oppure `2000::/4` + `3000::/4` (ciò che installa `redirect-gateway ipv6` di OpenVPN).
pub fn carries_ipv6_default(routes: &[IpNet]) -> bool;
```

**Ordine degli eventi e riavvii.** `connected` richiede due cose, in quest'ordine: un `NetUp` accettato e poi
`Up`. `NetUp` può arrivare più volte nella stessa sessione: se c'è già una rete applicata il demone la smonta e
applica quella nuova (mai due configurazioni sovrapposte). Alla ricezione di `Restarting` il demone smonta la
rete (lo smontaggio è idempotente: non importa se arriva prima `Restarting` o `NetDown`) e lo stato passa a
`reconnecting` (§5.2); il `NetUp` e l'`Up` successivi lo riportano a `connected` con DNS e rotte della sessione nuova.

Il demone non sa quale backend ha davanti: riceve una fabbrica `fn(&BackendParams) -> Result<Box<dyn Backend>,
RpcError>` che valida i parametri ("parse, don't validate": dopo la fabbrica esistono solo tipi già controllati).
Per i test c'è un `FakeBackend` pilotato da uno script di eventi: con quello macchina a stati, D5 e server IPC si
provano senza rete e senza privilegi.

### 6.2 WireGuard su Linux

**Crate scelto: `defguard_wireguard_rs` 0.12** (candidato del §5 dell'analisi), verificato il 2026-11-10 su
crates.io, sul `Cargo.toml` e sul sorgente del ramo `main`:

| Crate | Licenza | Ultima release | Giudizio |
|---|---|---|---|
| **`defguard_wireguard_rs`** | Apache-2.0 (compatibile con GPL-3.0-or-later **[V]**) | 0.12.5, 2026-11-04; 8 release negli ultimi 4 mesi; repository attivo (push 2026-11-04, 4 issue aperte) | **scelto**: API ad alto livello sopra netlink, kernel Linux **[V sul sorgente]** |
| `netlink-packet-wireguard` | MIT | 0.5.1, 2026-11-04 | è ciò che `defguard` usa sotto; usarlo direttamente vuol dire scrivere noi socket netlink, creazione del link e indirizzi. **Ripiego** se il crate scelto delude |
| `wireguard-control` | LGPL-2.1-or-later | 2.0.0, 2026-07-02 | nato dentro `innernet`, rilasci radi; nessun vantaggio |
| `wireguard-uapi` | MIT | 3.0.1, 2025-05-10 | fermo da 18 mesi |
| comando `wg` (wireguard-tools) | — | — | dipendenza di sistema in più e output di testo da leggere |

Cose da sapere sul crate scelto **[V sul `Cargo.toml` e su `src/wgapi_linux.rs`]**: su Unix porta sempre con sé
`defguard_boringtun` (BSD-3-Clause), anche se usiamo solo il kernel; è alla versione 0.x e rilascia ogni due
settimane circa; `remove_interface` chiama anche una sua pulizia dei DNS; ha funzioni proprie per rotte
(`configure_peer_routing`) e DNS (`set_dns`) che **non usiamo**, perché rotte e DNS sono del modulo comune (D10).
L'API GitHub non riconosce il file LICENSE (`NOASSERTION`): fanno fede `Cargo.toml` e crates.io, e `cargo deny` in CI.

Per contenere il rischio il crate è chiuso dietro un trait nostro di quattro operazioni (`WgDevice`: `create`,
`configure`, `read`, `remove`), in un solo file: cambiare libreria non tocca il resto.

Comportamento:

1. Validazione: chiavi base64 di 32 byte, indirizzi e `allowed_ips` come CIDR, almeno un peer, MTU 576–9000,
   `persistent_keepalive` 0–65535. Errore → `invalid_config` con il campo.
2. Risoluzione del nome dell'endpoint (una volta per tentativo, 5 s; fallimento → `Transport`).
3. Interfaccia fissa **`glr-wg0`** (un solo tunnel per D5; il nome fisso serve alla pulizia e a NetworkManager, §9).
   `create` → `configure` (chiave, indirizzi, MTU, peer, `fwmark 51820` se "tutto nel tunnel") → link su.
4. `NetUp` con `routes` = unione degli `allowed_ips`, `install_routes: Some(51820)`, DNS del profilo, e
   `ipv6_block: Unreachable` se `options.ipv6_block` e `is_full_tunnel_v4 && !carries_ipv6_default` (D15).
5. `Up` appena la rete è applicata. Lo stato `connected` di WireGuard significa "interfaccia configurata", come in
   `wg-quick` e nei client ufficiali; l'handshake è un dato a parte (`stats.last_handshake`, capacità `handshake`)
   (Q7, decisa). Se dopo **10 s** da `Up` non c'è ancora stato un handshake e nessun peer del profilo ha
   `persistent_keepalive` (assente o 0), il backend emette una volta un avviso (`Log` di livello `warn`, testo
   `no handshake yet`) e **non cambia stato**. Perché 10 s: WireGuard ritenta l'handshake ogni 5 s, quindi 10 s
   coprono il primo tentativo e una ripetizione; e senza keepalive né traffico l'handshake non parte da solo,
   quindi l'avviso spiega un silenzio che altrimenti sembra un guasto. Con il keepalive l'avviso non serve:
   l'handshake parte subito e il suo esito si legge in `stats.last_handshake`.
6. Statistiche ogni secondo da `read` (byte e ultimo handshake del primo peer).
7. `Stop`: `remove` (il demone ha già smontato la rete).

Nessuno spike ha provato WireGuard su Linux (lo spike 1 era Windows): è il **[S]** più grosso della fase e per
questo WireGuard è il primo backend del piano (T8, T9).

### 6.3 OpenVPN via management interface

Schema dello spike 2, che resta il riferimento (`spikes/openvpn-mgmt/src/`), con le correzioni emerse dagli spike.

**Configurazione generata.** Il demone riceve `config` + `secret_blocks`, li legge con `galleria-ovpn` e applica la
**lista delle direttive ammesse** (non più la lista delle vietate: `04-esito-spike-2-linux.md` criterio 2.1). Ogni
direttiva cade in una classe:

| Classe | Esempi | Import (CLI/GUI) | Demone |
|---|---|---|---|
| ammessa | `client`, `remote`, `proto`, `nobind`, `cipher`, `data-ciphers`, `auth`, `remote-cert-tls`, `verify-x509-name`, `tls-version-min`, `key-direction`, `auth-user-pass` (senza argomento), `auth-nocache`, `static-challenge`, `redirect-gateway`, `route`, `route-ipv6`, `pull-filter`, `dhcp-option`, `dns`, `tun-mtu`, `mssfix`, `keepalive`, `ping`, `ping-restart`, `reneg-sec`, `explicit-exit-notify`, `compress`, `comp-lzo`, `allow-compression`, `http-proxy` (senza file), `socks-proxy` (senza file), `dev-type`, `<connection>` | tenuta | tenuta |
| del demone | `dev`, `verb`, `mute`, `auth-retry`, `connect-retry`, `connect-retry-max`, `server-poll-timeout`, `connect-timeout`, `persist-tun`, `persist-key`, `script-security`, `management*` | tolta in silenzio | **rifiuto** |
| riferimento a file | `ca`, `cert`, `key`, `tls-auth`, `tls-crypt`, `tls-crypt-v2`, `pkcs12`, `extra-certs`, `crl-verify` con un percorso | file letto dall'utente normale e messo inline (M2) | ammessa solo inline, altrimenti **rifiuto** |
| pericolosa (M2) | `up`, `down`, `route-up`, `route-pre-down`, `ipchange`, `tls-verify`, `plugin`, `setenv`, `config`, `cd`, `chroot`, `user`, `group`, `log`, `log-append`, `status`, `writepid`, `daemon`, `engine`, `providers`, `dns-updown`, `iproute`, `tmp-dir`, `auth-user-pass <file>`, `askpass <file>` | tolta con avviso | **rifiuto** |
| sconosciuta | tutto il resto | tolta con avviso "non supportata" | **rifiuto** |

L'import ripulisce e avvisa; il demone non ripulisce mai, **rifiuta** (`profile_rejected`): un client corretto non
manda direttive fuori lista, e un client malevolo non ottiene una configurazione "aggiustata". In fase 1 non esiste
nessun percorso per eseguire uno script di un profilo, nemmeno con conferma (Q5). L'elenco completo sta in
`galleria-ovpn` con un test per riga; la sua completezza rispetto ai profili reali è **[S]** (T10).

Alla configurazione ammessa il demone aggiunge, in coda (l'ultima occorrenza vince):
`management <dir>/mgmt.sock unix`, `management-client-user root`, `management-hold`, `management-query-passwords`,
`auth-retry none`, `connect-retry-max 1`, `server-poll-timeout 5`, `verb 3`, `dev glr-ovpn0` (+ `dev-type tun`
se il profilo non ne ha uno), `script-security 2`, `up`/`down` = `"<percorso di galleriad> ovpn-hook <dir>/hook.sock"`,
`dns-updown disable` se la versione è ≥ 2.7 e le righe `pull-filter` di `ignore_pushed_routes`. Il demone non
genera **nessuna** direttiva IPv6 (`redirect-gateway ipv6`, `block-ipv6`, `ifconfig-ipv6`): il blocco IPv6 è del
modulo comune (§7.3, Q3 decisa).

**Processo.** `openvpn --config /dev/stdin` con la configurazione scritta sulla pipe: riga di comando senza segreti,
niente su disco **[V su Linux]**. Versione minima 2.6; sotto, `backend_unavailable`.

**Management interface.** Socket Unix nella cartella di sessione `0700 root` (M5) **[V]**. Un comando alla volta: il
successivo parte solo dopo `SUCCESS:`/`ERROR:` del precedente (`05-esito-spike-windows.md` §6, vale su tutte le
piattaforme). Sequenza: `>INFO` → `version 3`, `state on`, `bytecount 1`, `log on` → al `>HOLD` → `hold release`.
Lettura delle righe in un parser puro (`parse_line`), provato con righe vere prese dalle prove dello spike:

| Riga | Effetto |
|---|---|
| `>STATE:<t>,CONNECTED,SUCCESS,<ip>,<remoto>,<porta>,,` | `Up` con indirizzi e remoto (il `NetUp` dell'helper `up` è già stato accettato: vedi "Ordine tra `--up` e `CONNECTED`") |
| `>STATE:…,WAIT/AUTH/GET_CONFIG/ASSIGN_IP/ADD_ROUTES` | `Progress` |
| `>STATE:…,RECONNECTING,<motivo>,…` dopo un `Up` | `Restarting(<motivo>)`; gli stati `WAIT`/`AUTH`/… che seguono sono `Progress` |
| `>STATE:…,EXITING,auth-failure` · `>PASSWORD:Verification Failed: 'Auth'` | `Exited(AuthFailed)` |
| `>PASSWORD:Need 'Auth' username/password` | risponde con `username`/`password`; se mancano → `InputRequired(credentials)` |
| `>PASSWORD:Need 'Auth' username/password SC:<flag>,<testo>` | OTP: `SCRV1:b64(password):b64(otp)`; se il bit 2 del flag è acceso (formato `concat` della 2.7) `password+otp` **[S, non provato]**; senza OTP → `InputRequired(otp)` con `<testo>` come prompt |
| `>PASSWORD:Need 'Private Key' password` | `key_passphrase` o `InputRequired(key_passphrase)` |
| `>PASSWORD:Need 'Auth' … CRV1…` (challenge dinamico) | `Exited(Fatal(unsupported_challenge))` |
| `>BYTECOUNT:<in>,<out>` | `Stats` (al più una al secondo) |
| `>LOG:<t>,<flag>,<testo>` | `Log`; se il testo contiene `PUSH_REPLY`, lettura delle opzioni spinte (§7.3) |
| `>FATAL:` · chiusura del socket | `Exited(Transport)` |

**Ordine tra `--up` e `CONNECTED`.** OpenVPN esegue `--up` **prima** di dichiararsi `CONNECTED`: apre il tun
(`ASSIGN_IP`), chiama `--up` e ne aspetta l'uscita, poi installa le rotte (`ADD_ROUTES`) e solo alla fine manda
`>STATE:…,CONNECTED` **[V: `spikes/openvpn-mgmt/evidence/ovpn26/2.7-dhcpopt.log`, righe 57–72: `ASSIGN_IP`,
chiamata del programma `--up` con i suoi `resolvectl`, `net_route_v4_add`, `ADD_ROUTES`, `CONNECTED`; stesso
ordine nelle prove di `ovpn27` e `ovpn79`]**. Quindi il demone **non deve aspettare `CONNECTED` per rispondere
all'helper**: resterebbero fermi tutti e due. Il flusso è:

1. l'helper `up` consegna l'ambiente → il backend manda `NetUp` → il modulo comune applica DNS e blocco IPv6 →
   il demone risponde subito all'helper, che esce 0 (o diverso da 0 se la rete non si applica, e OpenVPN chiude);
2. OpenVPN prosegue: rotte, poi `>STATE:…,CONNECTED` → il backend manda `Up` → lo stato diventa `connected`.

Tra 1 e 2 lo stato resta `connecting` (o `reconnecting`): M4 è rispettato, i DNS sono applicati prima di `connected`.

**Riavvii interni** (`ping-restart`, `SIGUSR1`): OpenVPN resta lo stesso processo ma, non avendo `persist-tun`
(direttiva "del demone", mai aggiunta), chiude il tun, chiama `--down`, passa per `RECONNECTING` → `WAIT` → `AUTH`
→ … e alla fine richiama `--up` con le opzioni della sessione nuova, che possono essere diverse (altri DNS, altre
rotte). Il backend manda `Restarting` alla riga `RECONNECTING` e `NetDown` alla chiamata dell'helper `down`: il
demone toglie DNS e blocco IPv6 della sessione vecchia e lo stato passa da `connected` a `reconnecting` (§5.2, §6.1); il nuovo
`--up` porta un nuovo `NetUp`, applicato da capo, e il nuovo `CONNECTED` riporta `connected`. Le opzioni spinte
ricordate dal `PUSH_REPLY` precedente si scartano a ogni `Restarting`. Con `connect-retry-max 1` un riavvio
fallito fa uscire il processo e la riconnessione torna al demone (§5.2) **[S: S13, T13]**.

**Helper `ovpn-hook`.** È il "nostro programma fisso" di D10: OpenVPN lo esegue come `--up` e `--down`; lui manda al
demone, su un socket `0700 root`, le sole variabili d'ambiente utili (`dev`, `script_type`, `foreign_option_*`,
`ifconfig_*`, `route_*` comprese le `route_ipv6_*`, `redirect_gateway`, `route_redirect_gateway_*`, `dns_*`,
`tun_mtu`) e aspetta il verdetto. Con `script_type=up` esce 0 solo se il modulo comune ha applicato la rete,
altrimenti OpenVPN chiude; con `script_type=down` il demone smonta la rete e l'helper esce sempre 0. Il demone
risponde all'helper appena il modulo comune ha finito, senza aspettare altri eventi di OpenVPN. Il demone ricava
`NetConfig` da `foreign_option_N=dhcp-option DNS|DNS6|DOMAIN|DOMAIN-SEARCH …` (identico su 2.6 e 2.7 **[V]**), con
`install_routes: None`: le rotte le installa OpenVPN e muoiono con il tun (raffinamento di D10); il blocco IPv6 lo
decide qui il demone, con le regole del §7.3.

## 7. Modulo di rete comune (`galleria-net`)

### 7.1 Forma

`plan(&NetConfig) -> Vec<Step>` è puro: ogni passo ha il comando da eseguire e il comando che lo annulla.
`NetManager::apply` scrive prima il giornale (§7.4), poi esegue; `teardown` esegue gli annullamenti al contrario ed
è idempotente. I comandi passano da `CommandRunner` (in produzione `ip` e `resolvectl` cercati in un elenco fisso di
percorsi, mai da `PATH` del chiamante); gli argomenti sono solo tipi già validati (`IpAddr`, `IpNet`, nome fisso
dell'interfaccia), mai stringhe arrivate dall'IPC. Nei test unitari un `CommandRunner` finto registra i comandi.

### 7.2 DNS

Unico backend DNS della fase 1: **systemd-resolved** via `resolvectl`, come provato nello spike 2 (criterio 2.7):
`resolvectl dns <if> <server…>`, `resolvectl domain <if> <domini…> [~.]`, `resolvectl default-route <if> yes`;
annullamento `resolvectl revert <if>` (se l'interfaccia è già sparita resolved ha già dimenticato tutto). Il dominio
di instradamento `~.` si aggiunge solo quando il profilo è "tutto nel tunnel".

Se ci sono DNS da applicare e resolved non risponde, `apply` fallisce con `dns_unavailable` e la connessione non
sale: meglio un errore chiaro che un tunnel con il DNS della rete locale (M4). Senza DNS nel profilo non si tocca nulla.

### 7.3 Rotte e blocco IPv6 (D15, parte Linux)

**Rotte** (solo se `install_routes: Some(fwmark)`, cioè WireGuard): destinazioni specifiche con
`ip route add <cidr> dev glr-wg0` (muoiono con l'interfaccia). "Tutto nel tunnel" con lo schema di `wg-quick`
**[S: non provato negli spike, T9]**: rotta predefinita nella tabella `51820`, `ip rule add not fwmark 51820 table
51820`, `ip rule add table main suppress_prefixlen 0`, `net.ipv4.conf.all.src_valid_mark = 1`. Così il traffico
cifrato di WireGuard esce dalla rete fisica anche quando questa cambia (Wi-Fi → cavo), senza una rotta verso
l'endpoint da tenere aggiornata. Le regole non muoiono con l'interfaccia: stanno nel giornale. Per IPv6 nel tunnel
valgono le stesse regole con `-6`.

**Blocco IPv6** (Q3 decisa il 2026-11-10: un solo meccanismo su Linux, per tutti e due i backend). Il modulo
comune installa `ip -6 route replace unreachable ::/1` e `8000::/1`: errore immediato, non scarto silenzioso
**[V con OpenVPN su 2.6.19, 2.7.0 e 2.7.9: prova "reject" dello spike 2, criterio 2.8, 0 pacchetti e ripiego su
IPv4 in 1 ms; S con WireGuard: T9]**. La regola è la stessa per i due backend:

> il blocco si applica se `options.ipv6_block` è acceso, il tunnel è "tutto nel tunnel" per IPv4 e il tunnel
> **non** porta la rotta predefinita IPv6 (`carries_ipv6_default`, §6.1).

| Backend | Quando si decide | "Tutto nel tunnel" IPv4 | Rotta predefinita IPv6 nel tunnel |
|---|---|---|---|
| WireGuard | prima di `NetUp` | `is_full_tunnel_v4(allowed_ips)` | `carries_ipv6_default(allowed_ips)` |
| OpenVPN | alla chiamata di `--up`, quando si sa tutto | `redirect-gateway` nel profilo **oppure** nel `PUSH_REPLY` letto dal log della management **[S: T11, T14]** | `redirect-gateway` con il flag `ipv6` (profilo o `PUSH_REPLY`), oppure `carries_ipv6_default` sulle rotte IPv6 del profilo, del `PUSH_REPLY` (`route-ipv6`) e dell'ambiente di `--up` (`route_ipv6_network_N`) **[S: T11, T14]** |

Due conseguenze volute:

- **Un indirizzo IPv6 assegnato al tunnel non basta a togliere il blocco.** Un server può dare al client un
  indirizzo IPv6 (`ifconfig_ipv6_local`) e instradare solo la propria rete: l'IPv6 verso Internet uscirebbe dalla
  rete fisica. Il blocco resta finché `::/0` (o il suo equivalente) non è instradata nel tunnel; la rete IPv6
  interna del tunnel continua a funzionare perché la sua rotta è più specifica di `::/1` e `8000::/1`.
- **Il demone non genera direttive IPv6 native** (`redirect-gateway ipv6`, `block-ipv6`, `ifconfig-ipv6` locale):
  andrebbero scritte prima del lancio, quando non si sa ancora né se il server spinge `redirect-gateway` né se dà
  IPv6. Se il profilo dell'utente contiene da sé `redirect-gateway … ipv6` o `block-ipv6` (direttive ammesse),
  le applica OpenVPN e la regola sopra vede la rotta predefinita IPv6 nel tunnel: il demone non aggiunge il blocco.

Su un profilo **non** "tutto nel tunnel" il demone non tocca l'IPv6 **[S: S10, T14 con un server che non spinge
`redirect-gateway`]**. Le rotte `unreachable` non sono legate al tun: stanno nel giornale e il demone le toglie
alla disconnessione, a ogni riavvio interno di OpenVPN (§6.3) e dopo un crash (D15).

Questa è una modifica alla parte Linux di D15 (per OpenVPN l'analisi prevede la via nativa): la frase da
sostituire nell'analisi è in §13, Q3.

### 7.4 Giornale e pulizia dopo un crash

- `/run/galleria/net-journal.json` (tmpfs: un riavvio della macchina azzera insieme giornale e stato del kernel).
  Contiene l'elenco dei comandi di annullamento della sessione corrente e i nomi delle interfacce. Si scrive
  **prima** di applicare (file temporaneo + rename) e si cancella a smontaggio finito.
- `NetManager::recover()`, all'avvio del demone prima di accettare client: esegue gli annullamenti del giornale
  (ognuno tollera "non esiste"), poi cancella comunque `glr-wg0` e `glr-ovpn0` se esistono e le regole con
  `fwmark 51820`. Senza giornale fa solo questa seconda parte.
- I processi `openvpn` orfani li uccide systemd (`KillMode=control-group`), quindi il tun sparisce con loro; il
  DNS di resolved è legato all'interfaccia e sparisce con lei.
- **Cartelle di sessione.** Ogni tentativo di connessione ha la sua cartella `/run/galleria/s/<id>/`, dove `<id>`
  è casuale (UUID v4, 122 bit dal generatore del sistema) e **mai riusato**: il demone la crea con `mkdir` e modo
  `0700` (se esiste già è un errore, non la riusa), la passa al backend in `SessionContext.session_dir` e la
  rimuove quando il backend è uscito. Il numero `session` dell'IPC resta un contatore e non compare in nessun
  percorso: dopo un riavvio riparte da 1 senza poter incontrare file vecchi.
- **Pulizia delle cartelle orfane.** Dopo un crash restano in `/run/galleria/s/` cartelle con socket della
  management e dell'helper che nessuno ascolta più (`RuntimeDirectoryPreserve=yes` le conserva insieme al
  giornale). All'avvio, dopo `recover()` e **prima di aprire il socket e accettare richieste**, il demone
  svuota `/run/galleria/s/`: rimuove ogni voce senza seguire i collegamenti simbolici e ricrea la cartella `0700
  root`. Se la pulizia fallisce il demone non parte (errore nel log): meglio fermo che in ascolto sopra resti
  di un'altra sessione.

## 8. Profili e segreti

### 8.1 Chi tiene cosa

| Dato | Dove sta | Chi lo legge |
|---|---|---|
| Profilo senza segreti | `$XDG_CONFIG_HOME/galleria/profiles/<id>.toml` (D13) | CLI/GUI, da utente normale |
| Gruppi, ordine, impostazioni | `$XDG_CONFIG_HOME/galleria/galleria.toml` (D13) | CLI/GUI |
| Segreti (chiave privata WireGuard, preshared key, blocchi `key`/`tls-auth`/`tls-crypt*`/`pkcs12`, password salvate) | archivio dei segreti dell'utente (§8.3) | CLI/GUI |
| File referenziati da un `.ovpn` (`ca ca.crt`) | copiati **inline** all'import, da utente normale (M2) | CLI/GUI |
| Configurazione + segreti della sessione | memoria del demone, solo per la durata della sessione | demone |
| Stato della connessione, log | memoria del demone (`ConnectionState` del §6 dell'analisi), journald | demone |

Il demone non riceve mai un percorso: nessun metodo dell'IPC ha un parametro che sia un nome di file (M1). Non
scrive nulla fuori da `/run/galleria`.

### 8.2 Formato TOML (D13)

```toml
schema_version = 1
id = "3f0c2a9e-6f0b-4c0e-9a57-1d2e8a4b7c11"   # UUID v4, anche nome del file
name = "Ufficio Napoli"
protocol = "openvpn"                           # "openvpn" | "wireguard"
group = "Lavoro"
tags = []
favorite = false
notes = ""
created = "2026-11-10T10:00:00Z"
modified = "2026-11-10T10:00:00Z"
secret_refs = ["openvpn.block.key", "openvpn.block.tls-auth"]

[settings]
ipv6_block = true
auto_reconnect = true
ignore_pushed_routes = false

[openvpn]
config = """
client
remote vpn.esempio-ufficio.it 1194
…
"""
```

Per WireGuard la tabella è `[wireguard]` (`addresses`, `dns`, `search_domains`, `mtu`) con `[[wireguard.peers]]`
(`public_key`, `endpoint`, `allowed_ips`, `persistent_keepalive`); `secret_refs` contiene `wireguard.private_key` e,
se c'è, `wireguard.peer.0.preshared_key`. Campi sconosciuti in lettura: errore (uno `schema_version` più alto di
quello conosciuto dà "profilo creato da una versione più recente"). Scrittura atomica: file temporaneo nella stessa
cartella + `rename`, permessi `0600`, cartella `0700`. `ultimo_uso` del modello dati dell'analisi è rinviato alla
fase 2 (obbligherebbe a riscrivere il profilo a ogni connessione).

Import (flusso 1 del §7 dell'analisi), tutto lato utente in `galleria-profile`:
- `.conf` WireGuard: `[Interface]` (`PrivateKey`, `Address`, `DNS`, `MTU`) e `[Peer]`. `PreUp`/`PostUp`/`PreDown`/
  `PostDown` tolte con avviso (equivalente di M2); `Table`, `FwMark`, `ListenPort`, `SaveConfig` tolte con avviso
  "non supportata".
- `.ovpn`: classi del §6.3; `dev tapN` diventa `dev-type tap` (il nome dell'interfaccia lo decide il demone);
  i file referenziati si leggono dalla cartella del `.ovpn` e diventano blocchi inline;
  i blocchi segreti escono dal testo ed entrano nell'archivio dei segreti. Un profilo senza `client`/`tls-client`
  dà l'avviso previsto da `04-esito-spike-2-linux.md` §5.

### 8.3 Archivio dei segreti in fase 1

Deciso (Q6, 2026-11-10): in fase 1 i segreti della CLI stanno in un **file `0600` provvisorio**; il portachiavi
del sistema (Secret Service) previsto dall'analisi (§5) entra con la GUI in fase 2.

Trait `SecretStore` (`get`, `set`, `delete` per `(profile_id, chiave)`), con **una** implementazione in fase 1:
`FileSecretStore`, un file `secrets/<id>.toml` nella cartella di configurazione dell'utente
(`$XDG_CONFIG_HOME/galleria/secrets/`, cartella `0700`, file `0600`, scrittura atomica come i profili). I permessi
si **verificano a ogni lettura**: se il file o la cartella non sono dell'utente che esegue la CLI, o hanno un
qualsiasi bit di gruppo o "altri", la lettura fallisce con un errore che dice quale percorso correggere e come
(`chmod 600`); la CLI non li aggiusta da sola e non legge il segreto. Il motivo della scelta: la CLI minima deve
funzionare in laboratorio, in CI e via SSH, dove un Secret Service di solito non c'è; è lo stesso livello di
protezione di `/etc/wireguard/*.conf`. Il portachiavi sarà una seconda implementazione dello stesso trait.

Password e OTP non si salvano in fase 1 e **non passano mai dalla riga di comando** (gli argomenti di un
processo si leggono da `/proc/<pid>/cmdline` e restano nella cronologia della shell): la CLI li chiede al
terminale senza eco quando lo stato è `needs_input`, oppure li legge da stdin (`--password-stdin`,
`--otp-stdin`) per l'uso da script. Non esistono opzioni che prendano un segreto come valore.

## 9. Unit systemd e installazione minima per i test

In `packaging/linux/`, installati da `packaging/linux/install.sh` (usato dalle immagini del laboratorio e per le
prove manuali; **non** è il packaging di rilascio):

- `galleriad.service`: `ExecStart=/usr/local/libexec/galleria/galleriad run`, `Type=exec`, `Restart=on-failure`,
  `RuntimeDirectory=galleria`, `RuntimeDirectoryPreserve=yes` (il giornale deve sopravvivere al crash),
  `KillMode=control-group`, `After=network.target systemd-resolved.service`.
  Riduzione dei privilegi **[S: T7 con il demone a vuoto, poi T8 e T12 con i backend]**: `NoNewPrivileges=yes`,
  `ProtectHome=yes`, `ProtectSystem=strict`, `PrivateTmp=yes`, `CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW
  CAP_CHOWN CAP_DAC_OVERRIDE CAP_SETUID CAP_SETGID`, `RestrictAddressFamilies=AF_UNIX AF_NETLINK AF_INET AF_INET6`.
  Se una restrizione rompe un backend si toglie quella, si annota nel documento di esito e si prosegue.
  **Core dump**: `LimitCORE=0`, non negoziabile (non è tra le restrizioni che si possono togliere): la memoria del
  demone e di `openvpn` contiene chiavi e password, e un core dump le scriverebbe su disco (M7). Il limite si
  eredita, quindi vale anche per il processo `openvpn`. In più il demone chiama `prctl(PR_SET_DUMPABLE, 0)`
  all'avvio (§5.3), con `rustix::process::set_dumpable_behavior`, che è una funzione sicura **[V su rustix
  1.1.6]**. Serve perché `LimitCORE` da solo non copre due casi: il demone avviato fuori da systemd (test, prove
  a mano) e i sistemi dove `core_pattern` è una pipe verso un gestore (`systemd-coredump`, `apport`), dove il
  rispetto del limite è lasciato al gestore; con il processo non "dumpable" il kernel non produce il dump affatto.
  Limite noto: il flag si azzera a ogni `execve`, quindi `openvpn` è coperto solo da `LimitCORE`.
- `galleria.sysusers`: `g galleria -` (crea il gruppo).
- `90-galleria.conf` per NetworkManager: `unmanaged-devices=interface-name:glr-*` (R2) **[S: i container non hanno
  NetworkManager; prova manuale, §12]**.
- Binari: `galleriad` in `/usr/local/libexec/galleria/`, `galleria` in `/usr/local/bin/`.

Dipendenze di sistema: systemd con systemd-resolved, `iproute2`, modulo del kernel `wireguard`, e `openvpn` ≥ 2.6
dal pacchetto della distribuzione (§5 dell'analisi) solo per usare profili OpenVPN.

## 10. Test

### 10.1 Unitari (senza privilegi, `cargo test --workspace`)

| Crate | Cosa si prova |
|---|---|
| `galleria-ipc` | andata e ritorno di ogni tipo; campi sconosciuti rifiutati; riga oltre 1 MiB; batch; `hello` fuori intervallo; `Secret` non stampabile |
| `galleriad` | tabella della macchina a stati (§5.2) e attese; `authorize`; proprietà "mai due backend" con sequenze casuali di richieste sul `FakeBackend`; server IPC su socket temporaneo; `Redactor` (M7); buffer circolare |
| `galleria-net` | `plan` per ogni forma di `NetConfig`; ordine di apply/teardown; giornale scritto prima; `recover` idempotente |
| `galleria-ovpn` | lettura (virgolette, commenti, blocchi inline, `<connection>`); una riga di test per ogni direttiva classificata; profilo malevolo dello spike (criterio 2.1) |
| `galleria-backend-wireguard` | validazione; `NetConfig` generata; sequenza di chiamate su un `WgDevice` finto |
| `galleria-backend-openvpn` | `parse_line` su righe vere dello spike; coda dei comandi; risposta `SCRV1`; configurazione generata; lettura di `PUSH_REPLY` e dell'ambiente dell'helper; risposta all'helper `up` senza aspettare `CONNECTED`; riavvio interno (`RECONNECTING`, `down`, nuovo `up`) |
| `galleria-profile` | andata e ritorno TOML; scrittura atomica; import `.conf` e `.ovpn` con separazione dei segreti; permessi |
| `galleria-cli` | argomenti, codici di uscita, messaggi in inglese e italiano |

Obiettivo di copertura: 80 % delle righe sui crate puri (`ipc`, `ovpn`, `net`, macchina a stati), misurata con
`cargo llvm-cov` in CI come dato, senza soglia bloccante in fase 1.

### 10.2 Laboratorio Docker (`lab/`)

Nuovo, derivato da `spikes/openvpn-mgmt/lab` e `spikes/wireguard-nt/lab` (che restano com'erano). Rete
`galleria-fase1`, dual-stack `172.31.0.0/24` + `2001:db8:31::/64`, nomi `glab-*` per non scontrarsi con il
laboratorio degli spike.

| Container | Indirizzo | Ruolo |
|---|---|---|
| `glab-target` | `.20` | "sito" dual-stack fuori dalla LAN (`198.51.100.1`, `2001:db8:99::1`) e router predefinito |
| `glab-ovpn` | `.10` | OpenVPN 2.6.19: spinge `redirect-gateway def1`, `dhcp-option DNS/DOMAIN`, una rotta; utenti `alice` (password) e `bob` (password + TOTP) |
| `glab-ovpn-dns` | `.11` | come sopra con l'opzione `dns` |
| `glab-ovpn-split` | `.13` | **nuovo**: nessun `redirect-gateway`, solo `route 192.0.2.0/24` |
| `glab-ovpn-v6` | `.14` | **nuovo**: IPv6 dentro il tunnel (`server-ipv6`, `redirect-gateway def1 ipv6`) |
| `glab-wg` | `.12` | server WireGuard `10.99.0.1/24` |
| `glab-client<v>` | `.36` / `.37` / `.89` | client con systemd e systemd-resolved: Ubuntu 24.04 (OpenVPN 2.6.19), Ubuntu 26.04 (2.7.0), Fedora corrente (2.7.9) |

Le prove sono i test `#[ignore]` di `galleria-e2e`, compilati fuori ed eseguiti come root dentro il client contro il
demone vero avviato da systemd. Misure IPv6 come nello spike: cattura `tcpdump` su `eth0` dei pacchetti IPv6 non
link-local e non multicast, `curl` e client sequenziale verso `dual.galleria.test`; ogni prova di blocco ha il suo
controllo senza protezione, che **deve** vedere pacchetti (altrimenti la prova non dimostra nulla).

Insidie già pagate, da non ripetere (`05-esito-spike-windows.md` §7): i client con systemd privilegiato azzerano
`binfmt_misc` dell'host e su WSL rompono l'avvio degli `.exe` → le immagini mascherano `systemd-binfmt.service` e
`proc-sys-fs-binfmt_misc.automount`; `CLIENTS=""` deve voler dire "nessuno"; `lab/down.sh` toglie sempre tutto.
PKI e chiavi si generano all'avvio in cartelle ignorate da git; le password del laboratorio sono dati di prova.

### 10.3 CI (GitHub Actions, `ubuntu-latest`)

| Job | Contenuto | Blocca |
|---|---|---|
| `check` | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` (licenze compatibili con GPL-3.0-or-later, avvisi di sicurezza), controllo "`galleriad` non dipende da `galleria-profile`" | sì |
| `coverage` | `cargo llvm-cov --workspace` con riepilogo nel log | no |
| `lab` | matrice client 26 / 27 / 79: `modprobe wireguard`, build, `lab/run.sh <v>` | sì, da T7 |
| `lab-dco` | come `lab` su un client, con il modulo DCO installato sul runner | no (`continue-on-error`) |

**Non gira in CI**: NetworkManager e un desktop vero (prova manuale su VM Ubuntu e Fedora, lista in §12); DCO se il
job `lab-dco` non riesce a caricare il modulo (stessa VM); tutto ciò che è Windows; le prestazioni. Il workflow dello
spike (`.github/workflows/spike-windows-build.yml`) non si tocca.

Ogni job installa da sé gli strumenti che usa e che il runner non ha: `cargo-deny` nel job `check` e
`cargo-llvm-cov` nel job `coverage`, con `taiki-e/install-action` (binari già compilati; azione fissata a un
commit); `llvm-tools-preview` sta tra i componenti di `rust-toolchain.toml`. La prima CI, quella del T1, deve
poter essere verde.

## 11. Sicurezza: riepilogo rispetto al modello delle minacce

| Minaccia | Dove è trattata | Prova |
|---|---|---|
| M1 | §5.1 (gruppo + credenziali del peer), §4.1 (schema rigido, dimensione massima), §8.1 (nessun percorso, nessun segreto restituito), `ProtectHome` | T4 unitaria, T7 end-to-end con un utente fuori dal gruppo |
| M2 | §6.3 (lista ammessa, rifiuto nel demone, file letti solo all'import) | T10, T12 (profilo malevolo: nessun file `PWNED-*`) |
| M3 | `Status.net` mostra DNS e rotte ricevuti; `ignore_pushed_routes` | T12 |
| M4 | DNS prima di `connected` (§6.1 `NetUp`) e riapplicati a ogni riavvio interno (§6.3), blocco IPv6 (§7.3); kill switch fuori scope | T9, T12, T13, T14 |
| M5 | socket della management in cartella `0700`, `management-client-user root` | T12 (utente normale: accesso negato) |
| M6 | fuori scope (nessun installer in fase 1); `cargo deny` in CI | T1 |
| M7 | `Redactor` centrale senza lunghezza minima (§5.4); nessun segreto in argv (§8.3); niente core dump (§9); file dei segreti `0600` verificato a ogni lettura (§8.3) | T5 unitaria, T7 (`LimitCORE`), T13 e T16: ricerca dei segreti noti in `logs`, in journald e negli argomenti dei processi |

## 12. Punti [S] e primo task che li verifica

| # | Supposizione | Task | Se è falsa |
|---|---|---|---|
| S1 | `defguard_wireguard_rs` crea, configura, legge e rimuove un'interfaccia WireGuard del kernel dal nostro demone; chi alza il link; la sua pulizia DNS in `remove_interface` non disturba la nostra | T8 | ripiego su `netlink-packet-wireguard` dietro lo stesso `WgDevice` |
| S2 | Sul runner GitHub si carica il modulo `wireguard` e girano container privilegiati con systemd | T7 (systemd), T8 (modulo) | job `lab` solo in locale, annotato; CI resta sul job `check` |
| S3 | Schema `wg-quick` (fwmark, tabella 51820, `suppress_prefixlen`) installato e tolto dal modulo comune | T9 | rotte `/1` + rotta verso l'endpoint, annotando il limite sul cambio di rete |
| S4 | Rotte `unreachable` con WireGuard: zero pacchetti IPv6 globali ed errore immediato (provato solo con il tun di OpenVPN) | T9 | fermarsi e riportare: tocca D15 |
| S5 | Pulizia dopo `kill -9` del demone (giornale + nomi fissi + cartelle di sessione orfane) | T9 (WireGuard), T12 (OpenVPN) | — |
| S6 | Le restrizioni della unit systemd sono compatibili con i due backend | T7, T8, T12 | togliere la singola restrizione e annotarla |
| S7 | La lista delle direttive ammesse copre i profili reali | T10 (profili del laboratorio e manuale di OpenVPN) | aggiungere direttive, una per test |
| S8 | Le prove Linux dello spike 2, non rifatte dopo il porting a Windows (`05` criterio 2.10), reggono ancora | T12 (le rifà il prodotto) | — |
| S9 | Il `PUSH_REPLY` si legge dal log in tempo reale della management (`log on`, `verb 3`) su 2.6 e 2.7: è il meccanismo di D17, qui provato solo sul lato Linux | T11 (formato), T14 (dal vivo) | ripiego: la stessa riga letta da `stdout` di OpenVPN, dove i log dello spike 2 la mostrano già **[V]** (`evidence/ovpn26/2.4.log`) |
| S10 | Senza `redirect-gateway` (né nel profilo né spinto) il demone non applica il blocco e l'IPv6 dell'utente resta intatto | T14 | — |
| S11 | Al momento di `--up` il demone sa se il tunnel porta la rotta predefinita IPv6: flag `ipv6` di `redirect-gateway` nel `PUSH_REPLY` e `route_ipv6_network_N` nell'ambiente, su 2.6 e 2.7; con un server che la porta, l'IPv6 passa nel tunnel senza blocco | T11 (formato), T14 con `glab-ovpn-v6` | usare solo il `PUSH_REPLY` (o solo l'ambiente), quello dei due che risulta affidabile; se nessuno lo è, il blocco resta sempre e si annota: fallisce chiuso |
| S12 | `auth-nocache`: OpenVPN richiede le credenziali a ogni rinegoziazione e il demone risponde (`04` §6) | T13 | — |
| S13 | Riavvio interno di OpenVPN (`ping-restart`, `SIGUSR1`): `--down`, `RECONNECTING`, nuovo `--up` in quest'ordine o in un altro (il demone li tollera in qualsiasi ordine); ritorno al demone con `connect-retry-max 1` se il riavvio fallisce | T12 (unitario), T13 | — |
| S14 | Stato e `bytecount` con DCO su Linux vero (`04` criterio 2.4; su Windows è **[V]**) | T12, job `lab-dco`; altrimenti prova manuale su VM | resta aperto, annotato nell'esito |
| S15 | systemd-resolved insieme a NetworkManager su Ubuntu e Fedora desktop; `unmanaged-devices` (R2) | prova manuale su VM, lista di controllo scritta in T16 | — |
| S16 | Formato `concat` dello static challenge (2.7) | solo test unitario in T11; dal vivo non provato | resta **[S]** |

Restano **[S]** e passano alla fase 3 perché sono Windows: lettura del `PUSH_REPLY` su Windows e applicazione del
DNS (D17), filtro WFP con OpenVPN e come sessione dinamica (D15), ACL della pipe per gli utenti INTERACTIVE (D18),
controllo del proprietario della porta TCP della management, ACL del file temporaneo di ripiego,
`defguard_wireguard_rs` contro `wireguard-nt` su Windows. Fuori dalla v0.1: `--management-external-key` con RSA, `CRV1`.

## 13. Questioni per Peppe

Nessuna blocca l'avvio del piano: per ognuna è indicata la scelta provvisoria adottata, conservativa e reversibile.
L'analisi non è stata modificata. **Q3, Q6 e Q7 sono RISOLTE** (decisioni di Aitano su delega di Peppe,
2026-11-10) e già applicate nel testo e nei task; le altre restano aperte.

- **Q1 — "CLI" in due fasi diverse.** `01-analisi.md` §4 mette "CLI `galleria` che parla con lo stesso demone"
  tra le funzionalità di "Fase 2", mentre il §10 mette "CLI minima" nella fase 1 del piano. Lettura adottata: la
  fase 1 fa la CLI che serve a usare e provare il demone (import, connect, disconnect, status, logs); il resto
  (completamento, output per script, gestione di gruppi) resta dopo. In più "fase 2" indica due cose diverse nei
  due paragrafi (funzionalità dopo l'MVP e GUI): conviene rinominarne una.
- **Q2 — "Primo test/criterio della fase 1" per cose Windows.** D15 ("con OpenVPN [S], primo test della fase 1") e
  D17 ("è il primo criterio della fase 1") parlano di prove su Windows, ma la fase 1 del §10 è Linux e Windows è
  la fase 3. Lettura adottata: restano alla fase 3; in fase 1 si prova su Linux solo il pezzo comune di D17, cioè
  la lettura del `PUSH_REPLY` dalla management (S9).
- **Q3 — RISOLTA (Aitano su delega di Peppe, 2026-11-10): proposta accettata.** Su Linux, per OpenVPN, il blocco
  IPv6 usa **sempre** le rotte `unreachable` del modulo comune, decise al momento di `--up`, quando si sa già se
  c'è `redirect-gateway` (nel profilo o spinto) e se il server porta l'IPv6 nel tunnel; il demone non genera
  nessuna direttiva nativa (`redirect-gateway ipv6`, `block-ipv6`, `ifconfig-ipv6`). Motivo: la via nativa va
  scritta nella configurazione prima di avviare OpenVPN, quando quei due fatti non sono ancora noti; le rotte
  `unreachable` sono un solo percorso, uguale a quello di WireGuard e già misurato nello spike 2 con OpenVPN
  (0 pacchetti, ripiego in 1 ms su 2.6.19, 2.7.0 e 2.7.9). Applicata in §4.4, §6.1, §6.3, §7.3, §12 (S10, S11) e
  nei task T2, T6, T11, T12, T14 del piano. **La parte Linux di D15 in `docs/01-analisi.md` va aggiornata (la
  applica Aitano; questa spec non tocca l'analisi).** Nella riga D15, sostituire il testo che va da
  `Due vie, secondo il backend. **OpenVPN**: via nativa` fino a `Su Windows il meccanismo è da provare nello spike 1
  (criterio 1.5) **[S]**.` (compresi) con:

  > Su Linux un solo meccanismo, per tutti i backend, nel modulo di rete comune (D10): rotte `unreachable` per `::/1` e `8000::/1`, più specifiche della rotta predefinita della rete locale, che danno un **errore immediato** e **non** lo scarto silenzioso nel tunnel (con lo scarto le app senza Happy Eyeballs aspettano tutto il proprio timeout: 10 s nella prova, minuti con TCP) **[V su Linux con OpenVPN 2.6.19, 2.7.0 e 2.7.9, spike 2 criterio 2.8; S con WireGuard, fase 1]**. Il demone le applica quando il profilo è "tutto nel tunnel" per IPv4 e il tunnel non porta la rotta predefinita IPv6 (`::/0` o equivalente): un indirizzo IPv6 assegnato al tunnel non basta a togliere il blocco. **Per OpenVPN** la decisione si prende al momento di `--up`, quando si sa già se `redirect-gateway` è nel profilo o spinto dal server e se il server porta l'IPv6 nel tunnel; il demone **non genera** le direttive native `redirect-gateway ipv6`, `block-ipv6` e `ifconfig-ipv6`, che andrebbero scritte prima del lancio, quando questi fatti non sono ancora noti (decisione di Aitano su delega di Peppe, 2026-11-10, questione Q3 della spec di fase 1; la via nativa resta provata **[V]** ma non è usata). Le rotte `unreachable` non sono legate al tun: il demone le toglie da solo, alla disconnessione e anche dopo un crash.

  Per coerenza, nello stesso file parlano ancora della via nativa su Linux e vanno allineati: il raffinamento di
  D10 ("rotte e blocco IPv6 (D15) restano a OpenVPN": il blocco IPv6 passa al modulo comune, le rotte restano a
  OpenVPN), il §5 ("Il blocco IPv6 dei profili "tutto nel tunnel" lo fa OpenVPN stesso…") e il §8 alla voce
  **IPv6** ("per OpenVPN con la via nativa più un `ifconfig-ipv6` locale").
- **Q4 — Rimando in D13.** D13 dice "Il demone non legge questi file (M2)"; la mitigazione è scritta in M1 ("il
  demone non accetta percorsi di file arbitrari"), M2 parla dei file referenziati dal profilo. Solo il rimando.
- **Q5 — "Bloccate o confermate esplicitamente".** Il §8 dell'analisi e il mockup ("gli script non vengono eseguiti
  senza conferma esplicita") lasciano aperta l'esecuzione di `up`/`down` del profilo dopo una conferma. Il demone
  della fase 1 non ha nessun percorso per eseguirli, nemmeno confermati, e M1 dice che non accetta comandi. Se la
  conferma deve esistere serve un design a parte (chi esegue, con quali privilegi); altrimenti il testo del mockup
  va cambiato in "non vengono eseguiti".
- **Q6 — RISOLTA (Aitano su delega di Peppe, 2026-11-10): file `0600` provvisorio.** In fase 1 i segreti della
  CLI stanno in un file `0600` nella cartella di configurazione dell'utente, con i permessi verificati a ogni
  lettura; il portachiavi (Secret Service) entra con la GUI in fase 2 come seconda implementazione dello stesso
  trait. Applicata in §2, §8.3 e nel task T15.
- **Q7 — RISOLTA (Aitano su delega di Peppe, 2026-11-10): `connected` a interfaccia configurata**, come
  `wg-quick`, con l'ultimo handshake esposto a parte (`stats.last_handshake`). Se dopo **10 s** non c'è stato un
  handshake e il profilo non ha `PersistentKeepalive`, il demone emette l'avviso `no handshake yet` e non cambia
  stato (N = 10 s: due tentativi di handshake di WireGuard, uno ogni 5 s). Applicata in §6.2 e nel task T8.
- **Q8 — "File di log ruotato"** (§6 dell'analisi) e "Apri cartella log" (mockup): su Linux la fase 1 usa journald
  e il buffer in memoria, nessun file proprio. Se la GUI deve aprire una cartella, il file va aggiunto in fase 2.
- **Q9 — Management "con password"** (§5 dell'analisi): su Linux il socket Unix è protetto dai permessi e da
  `management-client-user`, senza password, come nello spike 2 e in M5. La password resta solo su Windows. Solo una
  precisazione del testo.
- **Q10 — Più utenti sulla stessa macchina.** Ogni membro del gruppo `galleria` può chiudere la connessione aperta
  da un altro e leggerne stato e log (non i segreti). È il comportamento di una connessione di sistema; se non va
  bene serve un concetto di "proprietario della sessione".
- **Q11 — Messaggi.** Il demone restituisce codici stabili e testo inglese; la traduzione (inglese e italiano, §8
  dell'analisi) è della CLI e della GUI. I messaggi italiani dello spike non sono riportati nel demone.
