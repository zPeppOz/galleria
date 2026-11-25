# Fase 2 — GUI Linux di Galleria: design

Versione 0.1 — 2026-11-25 — stato: proposta di Nello, da rivedere prima di eseguire i piani. Il piano del primo
blocco è `docs/superpowers/plans/2026-11-25-fase2-b1-fondamenta.md`; gli altri blocchi hanno un piano ciascuno,
scritto quando tocca a loro (§15).

Fonti: `docs/07-proposte-fase2.md` (D19–D32, approvate da Aitano su delega di Peppe il 2026-11-25: qui non si
riaprono), `docs/01-analisi.md` v0.7 (D1–D18), `docs/02-modello-minacce.md` (M1–M7), la spec della fase 1
(`2026-11-10-fase1-demone-design.md`, citata come «spec F1 §n»), `docs/06-esito-fase1.md`, il codice in `crates/`
a `e4a2d71` e il mockup `design/mockup/Rubrica.html`.

Legenda: **[V]** verificato oggi su una fonte letta o sul codice, con la fonte tra parentesi · **[S]** supposto,
con il task che lo verifica (§17) · *dedotto*: ragionamento mio su fatti verificati.

Restano fermi: D5–D18, Q3, Q6, Q7, A6, A8, A11 e **l'IPC della fase 1, senza modifiche** (nessun metodo, campo o
codice nuovo: `PROTOCOL_VERSION` resta 1). La validazione dei concorrenti (A7) è tolta.

## 1. Obiettivo e criteri di uscita

La fase 2 consegna la rubrica grafica su Linux: un'app Tauri 2 + React che gira da utente normale sopra il demone
della fase 1 e sostituisce la CLI nell'uso quotidiano di Peppe e dei colleghi (import, rubrica, connetti e
disconnetti, stato, log, tray).

Criteri di uscita. Ognuno ha una prova automatica, salvo dove è scritto «manuale». L'ultima colonna dice quale
blocco lo chiude (§15).

| # | Criterio | Come si verifica | Blocco |
|---|---|---|---|
| G1 | La GUI parte da utente normale, si collega al demone e ne mostra lo stato; se il demone riparte si ricollega da sola | prove dei comandi contro il demone con `FakeBackend`; fumo: riavvio di `galleriad` a GUI aperta | B1 |
| G2 | I quattro stati del primo avvio (D27) hanno ognuno il suo messaggio: demone fermo, utente fuori dal gruppo, protocollo incompatibile, OpenVPN assente | prove dei comandi con socket assente, socket non accessibile, `hello` fuori intervallo, backend `available: false` | B1 |
| G3 | Import da file, trascinamento e testo di un `.ovpn` e di un `.conf` del laboratorio: anteprima con gli avvisi, salvataggio solo dopo «Importa»; un profilo con `up` mostra il testo di D19 e lo script non viene mai eseguito | fumo nel laboratorio con il profilo malevolo della fase 1: nessun file `PWNED-*`; prove dei componenti sull'anteprima | B4 |
| G4 | Connetti, disconnetti e sostituisci dalla GUI con tutti e due i backend sui server del laboratorio; mai due backend vivi (D5); IP, byte e ultimo handshake si aggiornano | fumo nel laboratorio sui tre client; controllo dei processi come in E2 | B5 |
| G5 | Credenziali: password, OTP e passphrase chiesti solo quando il demone li chiede; «Ricorda» salva nel portachiavi e la connessione successiva non chiede la password; l'OTP non si salva mai | fumo con gli utenti `alice` e `bob` del laboratorio; prove dell'archivio con un Secret Service di prova | B5 |
| G6 | Un profilo importato dalla CLI si connette dalla GUI e viceversa (D25) | prove di `galleria-profile` sui due archivi; fumo incrociato | B5 |
| G7 | Nessun segreto noto nella scheda Log, nel file esportato, nei log della GUI e negli argomenti dei processi (M7); nessun segreto salvato passa dalla webview | ricerca dei segreti del laboratorio, come T13 e T16; prova che i comandi Tauri di lettura non restituiscono mai campi segreti | B5 |
| G8 | Ogni codice d'errore del demone ha testo inglese e italiano; cambiando lingua l'interfaccia cambia senza riavvio | prova Rust sui cataloghi (D22); prove dei componenti | B2 |
| G9 | Sessione estranea: con una connessione avviata da un altro utente del gruppo la GUI la mostra e chiede conferma prima di chiuderla (D21) | prove dei comandi con un profilo che non è in rubrica; fumo con due utenti | B5 |
| G10 | Tray: stato, preferiti, connetti e disconnetti dal solo menu; chiudere la finestra non chiude la connessione | **manuale** su Ubuntu 24.04 e 26.04 (GNOME con AppIndicator) e Fedora, lista di controllo nell'esito | B6 |
| G11 | Avviso di nuova versione: una sola richiesta, senza identificativi, niente richiesta se disattivato (D16, A11) | prova con un server finto che conta le richieste e ne controlla le intestazioni | B6 |
| G12 | Tutta la rubrica si usa da tastiera; finestre e schede hanno i ruoli corretti; **gli esiti e i cambi di stato arrivano allo screen reader come dice Q27** (§12.1): annuncio ATK dal lato Rust per quelli non bloccanti, fuoco spostato per conferme ed errori che chiedono una risposta | prove dei componenti con un controllo automatico di accessibilità; nell'immagine, Orca e AT-SPI sulla finestra vera (Q22), con la prova che Orca pronuncia l'annuncio emesso da `galleria-gui`; passata manuale con lo screen reader, annotata (B6) | B2, poi ogni blocco per i suoi componenti; annunci da B3 |
| G13 | Avvio a freddo della GUI sotto 1 s sulla macchina di Peppe (obiettivo [S] dell'analisi §8) | **manuale**, tre misure annotate nell'esito; non blocca la chiusura della fase, ma il numero va scritto | B6 |

## 2. Fuori scope

Windows (fase 3); pacchetti di rilascio, firma e repository (fase 4); kill switch, protezione DNS leak, avvio con
il sistema e connessione automatica (dopo la v0.1); QR WireGuard e associazione dei `.conf` (D31); esecuzione di
script dei profili, anche con conferma (D19); file di log propri (D20); proprietario della sessione nel demone
(D21); campi modificabili per OpenVPN oltre ai tre interruttori (D26); richieste di privilegi dalla GUI (D27);
OpenConnect. Nessuna modifica al demone, all'IPC o al formato dei profili.

## 3. Architettura (D29)

### 3.1 Struttura

```
crates/galleria-gui/        lato Rust di Tauri: collegamento al demone, comandi, eventi, archivi dell'utente
  Cargo.toml  build.rs  tauri.conf.json  capabilities/main.json
  src/main.rs               solo avvio
  src/lib.rs                costruzione dell'app Tauri
  src/link.rs               stato del collegamento e classificazione degli errori (D27)        — senza `tauri`
  src/supervisor.rs         connessione degli eventi, ricollegamento                           — senza `tauri`
  src/calls.rs              chiamate al demone, una connessione breve per chiamata             — senza `tauri`
  src/commands/             comandi Tauri: involucri sottili sui moduli sopra
  src/events.rs             inoltro degli eventi del demone alla webview
  tests/                    prove di livello 2 (§14)
ui/                         React + TypeScript + Vite
  src/  e2e/                componenti e prove di livello 1; fumo di livello 3
package.json  pnpm-workspace.yaml   radice: solo la CLI di Tauri e lo spazio di lavoro con `ui/`
```

`members = ["crates/*"]` prende già il crate nuovo **[V: `Cargo.toml`]**. I moduli marcati «senza `tauri`» non
importano nulla di Tauri: sono Rust e tokio, e sono quelli che le prove di livello 2 esercitano.

### 3.2 Confini

| Componente | Può | Non deve |
|---|---|---|
| `galleria-gui` | dipendere da `galleria-ipc` (feature `client`) e `galleria-profile`; in prova da `galleriad` e `galleria-backend` (feature `fake`) | avere `galleriad` o un backend tra le dipendenze normali; girare con privilegi; chiedere privilegi (D27) |
| `galleriad` | restare com'è | dipendere da `galleria-gui` o da `galleria-profile` (regola nuova in `scripts/check-boundaries.sh`) |
| Webview (`ui/`) | invocare i comandi del §4 e ascoltare gli eventi del §4.3 | aprire socket, file, processi o connessioni di rete; ricevere segreti salvati; passare percorsi di file ai comandi |

Regole che estendono M1 e M7 alla GUI:

1. **Solo i comandi Tauri parlano con il socket** (D12, D29).
2. **Nessun comando restituisce un segreto.** Import, lettura dal portachiavi e composizione dei parametri di
   `connect` avvengono in Rust. La webview manda solo ciò che l'utente digita nella finestra delle credenziali.
3. **Nessun comando accetta un percorso di file dalla webview.** Le finestre «Apri» e «Salva» le apre Rust; il
   trascinamento lo riceve Rust (`WindowEvent::DragDrop` esiste nel runtime **[V: `crates/tauri/src/app.rs` sul
   ramo `dev`]**; che i percorsi arrivino a Rust senza passare dalla webview è **[S]**, B4).
4. **Nessuna rete dalla webview.** CSP senza host remoti; anche la richiesta delle versioni (D16) parte da Rust.
5. Nessun segreto negli argomenti di un processo o in una variabile d'ambiente (come in fase 1).

### 3.3 Collegamento al demone

Il client della fase 1 è un oggetto con `&mut self` che legge e scrive sulla stessa connessione, e durante una
`call` accoda in memoria le notifiche che arrivano **[V: `crates/galleria-ipc/src/client.rs`]**. Per non toccarlo,
la GUI usa due tipi di connessione *(dedotto; alternativa in Q16)*:

- **Connessione degli eventi**, una, lunga: `hello`, poi `subscribe {stats: true, logs: true}`, poi solo
  `next_event()`. La tiene un task (`supervisor`). Se cade, lo stato del collegamento cambia e il task riprova:
  attese 0,5 · 1 · 2 · 5 · 5… secondi, più un tentativo immediato su richiesta dell'utente («Riprova»).
- **Connessioni dei comandi**, brevi: una per chiamata (`connect`, `disconnect`, `provide_input`, `logs`):
  apertura, `hello`, chiamata, chiusura. È ciò che fa la CLI. Il limite del demone è 16 client **[V: spec F1 §4.1]**.

La verità sullo stato è il flusso degli eventi: la `Status` restituita da una chiamata serve solo a segnalare
l'errore di quella chiamata. Lo stato iniziale è la `status` dentro la risposta a `hello` **[V: spec F1 §4.2]**.

Percorso del socket: `DEFAULT_SOCKET_PATH`, sostituibile con la variabile d'ambiente `GALLERIA_SOCKET` (serve alle
prove e al laboratorio; non è un segreto).

### 3.4 Configurazione di Tauri

- Tauri **2.12.7**, `tauri-build` 2.7.6; chiedono Rust ≥ 1.90, il workspace è a 1.96 **[V: crates.io, 2026-11-25]**.
  Licenza `Apache-2.0 OR MIT`, già ammessa da `deny.toml` **[V]**.
- `tauri.conf.json` in `crates/galleria-gui/`, `frontendDist: "../../ui/dist"`. La CLI di Tauri cerca il file
  scendendo dalla cartella corrente fino a una profondità limitata (`TAURI_CLI_CONFIG_DEPTH`) **[V:
  `crates/tauri-cli/src/helpers/app_paths.rs`, ramo `dev`]**: per questo la CLI si lancia dalla radice del repo.
  Che la profondità predefinita basti per `crates/galleria-gui/` è **[S]**, B1-T1.
- **Permessi**: un solo file `capabilities/main.json` per la finestra `main`, con i soli permessi che servono. In
  B1: ascolto degli eventi e i comandi del §4.1. I comandi dell'app sono permessi a tutte le finestre se non si
  dichiarano, e la pagina indica `AppManifest::commands` in `build.rs` per cambiarlo **[V: Tauri,
  «Capabilities»]**: si fa così. Il nome dei permessi che ne escono lo controlla B1-T4 sulla versione fissata. Nessun plugin con accesso a file, shell o rete esposto alla webview.
- **CSP** impostata (senza, la protezione non è attiva **[V: Tauri, «Content Security Policy»]**):
  `default-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost`. È il valore di partenza: la
  parte `connect-src` la ricordo dall'esempio ufficiale `api` e oggi non l'ho riletta, la conferma B1-T1.
  `withGlobalTauri: false`, `freezePrototype: true`.
- **Identificatore**: `io.github.zpeppoz.galleria` (Q18).
- Chiusura della finestra: fino a B6 chiude l'app. La connessione VPN è del demone e resta attiva.

## 4. Comandi Tauri sopra l'IPC della fase 1

Ogni comando restituisce `Result<T, CmdError>`. `CmdError` è `{ code, message, data? }`: `code` è un codice
stabile, `message` il testo inglese di ripiego (D22: la webview traduce per codice). I codici del demone passano
uguali (`already_active`, `profile_rejected`…); la GUI ne aggiunge di propri con il prefisso `gui_`.

### 4.1 Blocco B1

| Comando | Restituisce | Verso il demone |
|---|---|---|
| `daemon_view` | `DaemonView` (sotto) | nessuna chiamata: fotografia tenuta dal `supervisor` |
| `daemon_retry` | `()` | fa ripartire subito il tentativo di collegamento |
| `app_info` | `{ version, socket_path, user, group }` | nessuna: servono ai comandi da copiare di D27 |

```json
{"link":"connected","daemon_version":"0.1.0","incompatible":null,
 "backends":[{"protocol":"openvpn","available":false,"version":null,"unavailable_reason":"openvpn_not_found","capabilities":{}}],
 "status":{"state":"disconnected"}}
```

`link`: `connecting` · `connected` · `daemon_not_running` · `permission_denied` · `incompatible_protocol`.
`backends` e `status` sono i tipi di `galleria-ipc`, senza trasformazioni (nell'esempio sono abbreviati).

### 4.2 Blocchi successivi

Elenco di riferimento; nomi e campi definitivi nel piano di ogni blocco.

| Blocco | Comandi | Metodi IPC usati |
|---|---|---|
| B3 | `profiles_list`, `profile_get`, `profile_update`, `profile_validate_raw`, `profile_duplicate`, `profile_export`, `profile_delete`, `groups_get`, `groups_set`, `ui_state_get`, `ui_state_set`, `announce` (Q27, §12.1) | nessuno: file dell'utente (`galleria-profile`, `state.toml`) |
| B4 | `import_pick_file`, `import_from_text`, `import_commit`, `import_discard` | nessuno |
| B5 | `connect_profile(id, replace)`, `disconnect`, `provide_input(session, request, values, remember)`, `forget_credentials(id)`, `logs_get(after, limit)`, `logs_export`, `secret_store_status` | `connect`, `disconnect`, `provide_input`, `logs` |
| B6 | `settings_get`, `settings_set`, `update_check`, `app_quit` | nessuno |

- `profile_get` restituisce il profilo senza segreti e l'elenco di `secret_refs` (nomi, mai valori).
- `import_pick_file` apre la finestra di scelta in Rust e restituisce un'**anteprima**: identificativo, nome
  proposto, protocollo, avvisi come dati (`riga`, `direttiva`, `tipo`). Profilo e segreti restano in memoria in
  Rust fino a `import_commit` o `import_discard`; l'anteprima scade dopo 10 minuti. Gli errori dell'import escono
  con un codice per variante: `import_syntax`, `import_missing_file`, `import_outside_folder`, `import_invalid`
  (D22, regola 1), senza toccare `galleria-profile` **[V: `ImportError` ha queste quattro varianti]**.
- `connect_profile` legge profilo e segreti in Rust e chiama `to_connect_params` di `galleria-profile` **[V: esiste]**.
- `logs_export` apre «Salva» in Rust e scrive ciò che `logs` restituisce: testo già filtrato dal demone (D20).

### 4.3 Eventi verso la webview

| Evento | Carico | Origine |
|---|---|---|
| `galleria:link` | `DaemonView` | cambio di stato del collegamento |
| `galleria:state` | `Status` | `event.state` |
| `galleria:stats` | `Stats` | `event.stats` (una al secondo da `connected`) |
| `galleria:log` | `LogEntry` | `event.log` |
| `galleria:open-file` (B4) | anteprima di import | file aperto dal file manager o trascinato |

## 5. Primo avvio e demone assente (D27)

Classificazione del tentativo di collegamento, in `link.rs`, pura e provata con una tabella:

| Esito | `link` | Cosa mostra la GUI |
|---|---|---|
| `connect()` fallisce con «file assente» o «connessione rifiutata» | `daemon_not_running` | «Avvia il servizio»: `sudo systemctl enable --now galleriad`, da copiare |
| `connect()` fallisce con «permesso negato» (`ClientError::PermissionDenied`), oppure il demone chiude prima di rispondere a `hello` | `permission_denied` | `sudo usermod -aG galleria <utente>`, da copiare, e l'avviso che serve rientrare nella sessione |
| `hello` risponde `incompatible_protocol` | `incompatible_protocol`, con `daemon_min`, `daemon_max`, `daemon_version` | «Aggiorna Galleria e riavvia il demone», con le due versioni |
| `hello` riesce | `connected` | la rubrica; se un backend ha `available: false`, un avviso sui soli profili di quel protocollo con il motivo (`openvpn_not_found`, `openvpn_too_old`) |
| altro errore di I/O, `too_many_clients` | `daemon_not_running`, con il dettaglio nel log della GUI | come la prima riga; si riprova |

«Il demone chiude prima di rispondere» è il rifiuto dell'autorizzazione al secondo livello **[V: spec F1 §5.1]**.
La GUI non esegue mai `sudo`, `pkexec` o `systemctl`: mostra il comando e basta. La rubrica resta consultabile e
modificabile senza demone (da B3).

## 6. Rubrica, profili e stato dell'interfaccia (D26, D32)

- Archivio: quello della fase 1 (`ProfileStore`, `galleria.toml`), stesso formato e stessi permessi. GUI e CLI
  scrivono con rename atomico; la GUI rilegge la cartella quando la finestra torna in primo piano e dopo ogni
  propria scrittura *(dedotto: niente osservatore di file in v0.1)*.
- Schede e campi modificabili: tabella di D26 in `docs/07` §3.2, senza variazioni. WireGuard a campi veri; OpenVPN
  in sola lettura più `auto_reconnect`, `ignore_pushed_routes`, `ipv6_block`; il resto dall'editor raw, validato
  con la classificazione delle direttive della fase 1.
- Stato nella testata: i sette stati di `State` più `detail` mappato su etichette tradotte, con il valore grezzo
  come ripiego (D22).
- `state.toml` in `$XDG_STATE_HOME/galleria/` (D32): ultimo uso per profilo, dimensione e «massimizzata» della finestra (Q34: la posizione non si salva),
  ordinamento scelto. Scritto dalla GUI, `0600`; se manca o non si legge si riparte dai valori predefiniti senza errore.

## 7. Import (D19, D31)

- Tre ingressi (file, trascinamento, testo incollato), un solo flusso: anteprima con gli avvisi, salvataggio solo
  dopo «Importa» (M2). Anche dal doppio click sul file.
- Testo dell'avviso per una direttiva che lancia script: «Direttiva `<direttiva>` **rimossa**: Galleria non
  esegue gli script dei profili.» Segue «I DNS del server li applica Galleria.» solo se lo script è
  `update-resolv-conf` o `update-systemd-resolved`; altrimenti «Se il profilo ne dipende, potrebbe non funzionare
  del tutto.» Dopo l'import: «N direttive rimosse».
- **Associazione di `.ovpn`**: `bundle.fileAssociations` di Tauri, che su Linux diventa `MimeType=` nel file
  `.desktop` **[V: riferimento di configurazione di Tauri 2]**. I `.conf` non si associano (D31).
- **Apertura del file su Linux**: il percorso arriva come argomento della riga di comando di un processo nuovo.
  `RunEvent::Opened` esiste solo su macOS, iOS e Android **[V: `crates/tauri/src/app.rs`, attributo `cfg`; esempio
  ufficiale `examples/file-associations`, che su Linux e Windows legge `std::env::args()`]**.
- **Istanza unica**: plugin ufficiale `tauri-plugin-single-instance` 2.5.5. Il secondo processo avvisa il primo e
  termina; il primo riceve `args` e `cwd` del secondo nella funzione passata a `init`. Va registrato per primo. Su
  Linux usa D-Bus e pubblica il servizio `org.<id>.SingleInstance`, dove `<id>` è l'identificatore con punti e
  trattini sostituiti da `_` **[V: Tauri, «Single Instance»]**. Conseguenze *(dedotte)*: serve un bus di sessione,
  quindi nel laboratorio il fumo gira dentro `dbus-run-session`; cosa succede senza bus è **[S]**, B4.

## 8. Connessione (D5, D21, D28)

- Connetti: `connect_profile(id, replace=false)`. Se risponde `already_active`: la sessione attiva è della propria
  rubrica (il suo `profile.id` è tra i profili) → la GUI richiama con `replace=true` senza chiedere, e mostra
  «Disconnesso da X» a scomparsa; è **estranea** → finestra di conferma, poi `replace=true` (D21, D28).
- Sessione estranea: riga fissa in cima alla lista e variante della barra di stato («Connessione attiva: *nome* —
  avviata fuori da questa rubrica»), «Disconnetti» con conferma, scheda Log in sola lettura.
- Credenziali: la GUI manda ciò che ha nell'archivio; su `needs_input` mostra **solo** il campo chiesto
  (`credentials`, `otp`, `key_passphrase`), con il `prompt` del server non tradotto e il conto alla rovescia di
  `expires_in_secs`. «Ricorda» vale per utente e password, mai per l'OTP. «Annulla» durante la connessione è una
  `disconnect`.
- Errori: `Status.error.code` e `data.code` tradotti dal catalogo; per un codice sconosciuto si mostra il `message`
  inglese del demone (D22).

## 9. Archivio dei segreti (D25)

Fatti: `SecretStore` ha `get`, `set`, `delete` per `(profile_id, chiave)` e una sola implementazione,
`FileSecretStore` **[V: `crates/galleria-profile/src/secrets.rs`]**.

Design, tutto in `galleria-profile` perché lo usano GUI e CLI:

| Pezzo | Comportamento |
|---|---|
| `KeyringSecretStore` | Secret Service sul bus di sessione. Una voce per segreto, con gli attributi `service=galleria`, `profile=<uuid>`, `key=<secret_ref>` ed etichetta «Galleria: <nome profilo> (<chiave>)» |
| `LayeredSecretStore` | lettura: portachiavi, poi file. Scrittura della GUI: portachiavi; se il servizio **non esiste**, file `0600` con un avviso visibile e persistente nella GUI. Cancellazione: da tutti e due |
| CLI | legge da tutti e due con lo stesso ordine; continua a scrivere nel file, come in fase 1 |

Regole *(dedotte da D25; i casi limite sono in Q20)*:

1. Se la stessa chiave è in tutti e due, vale il portachiavi. Dopo una scrittura riuscita nel portachiavi la GUI
   cancella la copia nel file, così non restano due valori diversi.
2. Portachiavi presente ma bloccato, o sblocco rifiutato dall'utente: errore `gui_keyring_locked`, **nessun**
   ripiego silenzioso sul file.
3. Nessuna migrazione automatica dal file al portachiavi: i segreti importati dalla CLI restano nel file finché
   il profilo non viene salvato di nuovo dalla GUI.
4. Chiavi salvate: quelle della fase 1 (`wireguard.private_key`, `openvpn.block.*`…) più `openvpn.username` e
   `openvpn.password` per «Ricorda». L'OTP e la passphrase della chiave non si salvano.
5. `secret_store_status` dice alla webview solo quale archivio è in uso (`keyring` · `file`), per l'avviso.

Quale crate: **[S]**, primo task di B5. Candidati con licenza già ammessa da `deny.toml` **[V: crates.io,
2026-11-25]**: `keyring` 4.2.0 e `secret-service` 6.0.0 (`MIT OR Apache-2.0`), `oo7` 0.7.3 (MIT). Si sceglie
provandoli con GNOME Keyring e KWallet; se il portachiavi dà problemi il ripiego sul file è già nel design e B5
non si ferma.

## 10. Log (D20)

La scheda Log mostra il buffer del demone: `logs_get` all'apertura, poi `galleria:log`. Titolo «Sessione
corrente» se lo stato non è `disconnected`, altrimenti «Ultima sessione». «Copia» e «Esporta log…»; nessuna
cartella di log. Le righe non si traducono. La GUI non scrive file di log propri: i suoi messaggi diagnostici
vanno su `stderr`, e non contengono mai parametri di `connect`, valori digitati o contenuti di profili (G7).

## 11. Traduzioni (D22)

Cataloghi `en` e `it` in `ui/src/i18n/`, chiave = codice. Una prova Rust in `galleria-gui` legge i due file e
fallisce se manca un codice di `ErrorCode` o di `SessionErrorCode` (11 e 8 varianti oggi **[V:
`crates/galleria-ipc/src/error.rs`]**) o un codice `gui_*`. Lingua del sistema, scelta manuale nelle impostazioni,
inglese come ripiego; il cambio non chiede riavvio (G8). In B1 i pochi testi stanno in `ui/src/messages.ts` con
le due lingue; B2 li sposta nei cataloghi.

## 12. Design system (D30)

- **Token**: dal CSS del mockup, che li ha già come variabili **[V: `Rubrica.html`, righe 8–19]**: superfici
  (`--bg`, `--panel`, `--panel-2`, `--line`), testo (`--ink`, `--muted`, `--faint`), accento (`--accent`,
  `--accent-ink`), stati (`--ok`, `--warn`, `--err`), protocolli (`--wg`, `--ovpn`), raggio (`--r`), caratteri
  (`--font`, `--mono`); tema chiaro con `:root[data-theme=light]`. Finiscono in `ui/src/design/tokens.css`, tema
  scuro predefinito, tema del sistema rispettato, scelta manuale salvata in `state.toml`.
- I colori usano `oklch()`: che WebKitGTK delle tre distribuzioni obbligatorie lo disegni giusto è **[S]**, B2
  (ripiego: valori esadecimali equivalenti).
- **Componenti di base** in `ui/src/design/`: pulsante, campo, interruttore, selettore a segmenti, schede,
  finestra modale, menu, avviso a scomparsa, avviso in linea. Stile tutto nostro, CSS semplice con i token; nessun
  framework CSS.
- **Comportamento accessibile**: libreria «headless» `@base-ui/react` 1.9.0, licenza MIT **[V: registro npm,
  2026-11-25]**, compatibile con GPL-3.0-or-later; ha `dialog`, `alert-dialog`, `menu`, `tabs`, `select`, `switch`,
  `toast`, `tooltip` **[V: campo `exports` del pacchetto]**. La scelta è Q12.
- Licenze JS: uno script in CI elenca le licenze delle dipendenze di produzione di `ui/` e fallisce fuori
  dall'elenco di `deny.toml` (B2).
- Niente emoji nei pulsanti e nelle icone: SVG nostri (nel fumo senza schermo le emoji non si disegnano).

### 12.1 Annunci per lo screen reader (Q27)

Fatti, dallo spike `t_b81c3909` (`docs/08-esito-fase2.md`, «Spike Orca»): Orca non pronuncia **nessuna** regione viva di
WebKitGTK, né nell'immagine (Orca 46.1) né su GNOME vero (Orca 50.3 su Ubuntu 26.04, Orca 51.1 su Fedora 45), `role="alert"`
compreso **[V]**; pronuncia invece il segnale ATK `announcement` o `notification` emesso dal processo `galleria-gui`
sull'oggetto accessibile della finestra, nelle tre versioni **[V]**. Il nome dei campi di testo, muto nell'immagine, su
GNOME vero si sente **[V]**: non è un limite del prodotto.

Regole (Q27, approvata):

1. **Esiti non bloccanti e cambi di stato** (salvato, duplicato, eliminato; da B5 gli stati della connessione) passano
   dal segnale ATK, emesso dal lato Rust: *polite* di norma, *assertive* per gli errori. Il testo è quello dei
   cataloghi, già tradotto. Gli esiti che nascono nella webview usano il comando `announce`; i cambi di stato che
   nascono in Rust (B5) li emette lo stesso codice che manda `galleria:state`, senza passare dalla webview.
2. **Conferme ed errori che chiedono una risposta** spostano il fuoco (finestra di conferma, finestra modale): è il
   fuoco a farli leggere, come oggi.
3. **Tutto il resto è un limite dichiarato**, scritto nell'esito e nel README: altri lettori di schermo, sistemi con un
   ATK senza il segnale (`announcement` esiste da ATK 2.46, `notification` da 2.50 **[V: feature `v2_46` e `v2_50` del
   crate `atk` 0.18.2]**), testo che cambia nella pagina senza un esito (per esempio la barra di stato).
4. **Niente `aria-live` come unico canale** e **niente notifiche del desktop**. Le regioni vive restano nel markup, che è
   giusto e serve agli altri lettori, ma nessun esito dipende solo da loro.

L'avviso a scomparsa è l'unico punto della webview che annuncia: chi mostra un avviso ottiene anche l'annuncio, e
nessun componente chiama `announce` per conto suo. Le prove di controllo sulle regioni vive restano: il giorno in cui
Orca le legge diventano rosse e l'annuncio si toglie, per non sentire due volte la stessa frase.

## 13. Tray e avviso di versione (D16)

- Tray: stato, preferiti, connetti e disconnetti, «Apri Galleria», «Esci». Su Linux l'evento di click
  sull'icona non viene emesso, funziona solo il menu **[V: Tauri, «System Tray»: «Linux: Unsupported»]**: tutte le
  azioni stanno nel menu. Da B6 chiudere la finestra la nasconde; l'uscita è esplicita.
- Versione: una richiesta `GET` all'endpoint delle release di GitHub, da Rust, senza identificativi né
  intestazioni oltre a quelle minime, al più una volta al giorno, disattivabile; scritta nel README (D16, A11).

## 14. Prove su tre livelli

| Livello | Cosa prova | Con cosa | Dove gira |
|---|---|---|---|
| 1 · Frontend | componenti, stati, tastiera, ruoli ARIA, traduzioni | Vitest 5 + Testing Library; comandi simulati con `mockIPC` di `@tauri-apps/api/mocks` **[V: Tauri, «Mock Tauri APIs»]**; controllo automatico con axe | job `gui`, senza schermo e senza demone |
| 2 · Comandi Rust | collegamento, classificazione, ricollegamento, comandi, archivi | `cargo test -p galleria-gui` contro un demone vero con `FakeBackend` su socket temporaneo, senza privilegi; server finti a copione per i casi che il demone non produce (`incompatible_protocol`) | job `gui` |
| 3 · Fumo end-to-end | l'app vera, costruita come per il rilascio, pilotata via WebDriver | WebdriverIO + `tauri-driver` + `WebKitWebDriver`, sotto `xvfb-run` e `dbus-run-session` | immagine `lab/images/gui.Dockerfile`; in CI nel job `gui-smoke` |

Verifiche sulla documentazione ufficiale, per il livello 3:

- Su Linux `tauri-driver` usa `WebKitWebDriver`, che su Debian e derivate sta nel pacchetto `webkit2gtk-driver`
  **[V: Tauri, «WebDriver · Manual setup»]**. `tauri-driver` 2.1.0 chiede Rust ≥ 1.95 **[V: crates.io]**.
- In CI le prove girano «creando uno schermo finto»: l'esempio ufficiale installa `webkit2gtk-driver` e `xvfb` e
  lancia `xvfb-run yarn test` su `ubuntu-latest` **[V: Tauri, «WebDriver · Continuous Integration»]**.
- La pagina ufficiale oggi raccomanda WebdriverIO con `@wdio/tauri-service`, che **per impostazione predefinita
  avvia un server WebDriver dentro l'app** (plugin `tauri-plugin-wdio-webdriver`) e in alternativa pilota il
  driver nativo con `tauri-driver` **[V: Tauri, «WebDriver»]**. È un fatto nuovo rispetto a `docs/07`: la scelta è Q13.

Che il fumo giri nel container e sul runner resta **[S]**: lo prova B1-T6. Se non regge, il fumo resta manuale
con una lista di controllo scritta, e lo si scrive nell'esito.

Il livello 2 usa il demone vero ma non i backend veri: G3–G6 e G9 si chiudono nel laboratorio della fase 1, con
la GUI installata sui client `glab-client*` (pacchetti di WebKit da aggiungere a quelle immagini: lo decide il
piano di B4).

**CI.** `galleria-gui` porta nel workspace le librerie di sistema di WebKitGTK, e la macro
`tauri::generate_context!` legge `frontendDist` in compilazione (**[S]**: che fallisca se `ui/dist` manca lo
verifica B1-T1). Oggi `check`, `coverage` e `lab/build.sh` usano `--workspace` **[V: `.github/workflows/ci.yml`,
`lab/build.sh`]**: aggiungere il crate li romperebbe. Quindi (Q14): quei tre comandi escludono `galleria-gui`, e
due job nuovi lo coprono.

| Job | Contenuto | Blocca |
|---|---|---|
| `check`, `coverage`, `lab` | come oggi, con `--workspace --exclude galleria-gui` | come oggi |
| `gui` | librerie di sistema di Tauri, Node e pnpm, `pnpm --dir ui build`, prove di livello 1, `cargo clippy` e `cargo test -p galleria-gui`, `cargo deny check` | sì |
| `gui-smoke` | build dell'immagine `gui`, fumo di livello 3 | no (`continue-on-error`) finché B1-T6 non lo dichiara stabile |

Librerie di sistema per compilare su Debian e Ubuntu: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`,
`file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` **[V: Tauri, «Prerequisites»]**.
Sulla macchina di sviluppo (WSL) oggi `webkit2gtk-4.1` non c'è **[V: `pkg-config --modversion webkit2gtk-4.1`,
2026-11-25: non trovato]**: vedi Q15.

## 15. Blocchi

Ogni blocco è un piano a sé (un task = un commit, TDD, review indipendente) e finisce con qualcosa che si avvia.
Il numero dei task di B2–B6 resta la stima di `docs/07` §5; quello definitivo esce dal piano del blocco.

| Blocco | Task | Contenuto | Dipende da | Criteri | Dove ci si può fermare |
|---|---|---|---|---|---|
| B1 · Fondamenta | 6 (piano scritto) | crate e `ui/`, CI, collegamento con ricollegamento, eventi, permessi minimi, primo avvio (D27), prova del fumo WebDriver | fase 1 in `main` | G1, G2 | finestra che mostra lo stato vero del demone, sola lettura |
| B2 · Design system e guscio | 6 | token, componenti di base, disposizione lista + dettaglio, temi, cataloghi con la prova dei codici (D22), tastiera | B1 | G8, G12 | guscio navigabile con dati finti, nelle due lingue |
| B3 · Rubrica e profili | 9 (piano scritto, in quattro gruppi) | lista, gruppi, ricerca, preferiti, ordinamento; schede secondo D26; editor raw; duplica, esporta, elimina; `state.toml` (D32) con lingua e tema (Q25); comando `announce` e primo uso di Q27 | B2 | G12 per la parte degli annunci | rubrica completa sui profili importati dalla CLI |
| B4 · Import | 7 (piano scritto, in tre gruppi) | file, trascinamento, testo; anteprima con il testo di D19; associazione di `.ovpn` e istanza unica (D31) | B3 | G3 | si importa dalla GUI e si connette dalla CLI |
| B5 · Connessione | 8 | archivio dei segreti (D25); connetti, disconnetti, sostituisci (D28); stati e statistiche; credenziali su `needs_input`; errori tradotti; Log con «Esporta log…» (D20); sessione estranea (D21) | B3 (B4 per il giro completo) | G4, G5, G6, G7, G9 | **uso quotidiano possibile** |
| B6 · Tray, versione, chiusura | 6 | tray, chiusura nella tray, avviso di versione (D16), fumo completo, prove manuali, `docs/08-esito-fase2.md` | B5 | G10, G11, G13 | fase 2 chiusa |

«Fase 1 in `main`»: oggi `main` e `origin/main` sono a `e4a2d71` e contengono i crate della fase 1 **[V: `git log
-1 main`, 2026-11-25]**. Lo step «Integra la fase 1 in `main`» della roadmap dice ancora `8964358`: va riallineato
da chi lo ha in carico, non in questa spec.

## 16. Differenze volute dal mockup

Il mockup resta il riferimento visivo. Dove dice altro, vale questa spec:

1. Via kill switch, «Connetti all'avvio», «Protezione DNS leak» (D26; il mockup li mostra ancora).
2. OpenVPN: server e trasporto in sola lettura; «Metodo» è un'etichetta, non un selettore; DNS e rotte effettivi
   in sola lettura da `Status.net`; «Tutto il traffico nel tunnel» è un'indicazione (D26).
3. Editor raw senza segnaposto `<portachiavi:…>`; una riga sotto dice quali segreti sono nell'archivio (D26).
4. Sette stati, non quattro; finestra delle credenziali con i soli campi chiesti (D26, D28).
5. Barra in basso: lo stato vero del collegamento al posto di «Demone galleriad · connesso» fisso (D27).
6. Riga fissa e barra per la sessione estranea (D21).
7. Via «scansiona un QR WireGuard» dalla finestra di import (D31).
8. Già allineati nel mockup: testo dell'avviso di import, «1 direttiva rimossa», «Esporta log…», righe di log
   in inglese con `glr-wg0` e `glr-ovpn0` (D19, D20).

## 17. Punti [S] e task che li verifica

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S1 | La CLI di Tauri trova `crates/galleria-gui/tauri.conf.json` dalla radice | B1-T1 | `TAURI_CLI_CONFIG_DEPTH`, oppure la CLI si lancia dalla cartella del crate |
| S2 | Senza `ui/dist` il crate non compila; con `--exclude galleria-gui` i job della fase 1 restano verdi | B1-T1 | se compila comunque, l'esclusione resta per le librerie di sistema |
| S3 | Un build fatto con `cargo build` senza la CLI incorpora i file di `ui/dist` solo con la feature `custom-protocol` di `tauri` | B1-T1 | si costruisce sempre con la CLI (`tauri build --no-bundle`) |
| S4 | `tauri::test` (runtime finto, modulo dichiarato instabile **[V: `crates/tauri/src/test/mod.rs`]**) basta a provare i comandi senza schermo | B1-T4 | i comandi restano involucri di una riga e si provano solo i moduli senza `tauri` |
| S5 | `galleriad run --socket … --group … --runtime-dir …` parte nel container del fumo senza toccare la rete | B1-T6 | il fumo usa un demone a copione costruito dalle prove |
| S6 | Il fumo WebDriver gira nel container e sul runner GitHub | B1-T6 | fumo manuale con lista di controllo scritta |
| S7 | `navigator.clipboard.writeText` funziona nella webview per «Copia» | B1-T5 | comando Rust `copy_text`, con il plugin degli appunti in sola scrittura |
| S8 | `oklch()` è disegnato giusto da WebKitGTK su Ubuntu 24.04, 26.04 e Fedora | B2 | colori esadecimali equivalenti nei token |
| S9 | I percorsi del trascinamento arrivano a Rust senza passare dalla webview | B4 | un comando accetta i percorsi e li tratta come input non fidato: stessa anteprima, stesse regole di `galleria-profile` |
| S10 | `.ovpn` ha un tipo MIME registrato sulle tre distribuzioni | B4 | il pacchetto installa una definizione MIME propria |
| S11 | Comportamento del plugin di istanza unica senza bus di sessione | B4 | l'app parte comunque e lo scrive nel log |
| S12 | Crate per Secret Service e comportamento con GNOME Keyring e KWallet | B5, primo task | ripiego sul file (D25) |
| S13 | `cargo deny check` passa con l'albero di Tauri: mi aspetto avvisi «non più mantenuto» sulle librerie GTK3 per Rust e forse licenze non ancora in elenco (previsione, non eseguito) | B1-T1 | avvisi ignorati uno per uno con il motivo scritto; licenze aggiunte solo se compatibili con GPL-3.0-or-later; una incompatibile ferma il blocco |

## 18. Questioni aperte nuove

Numerazione che continua Q1–Q11 della spec della fase 1. Ognuna ha la mia raccomandazione, che vale come
**predefinito per chi implementa** se nessuno decide diversamente prima del blocco indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q12 | Libreria «headless» del design system (D30 la lascia alla spec) | `@base-ui/react` 1.9.0 (MIT) · `radix-ui` 1.8.1 (MIT) · `react-aria-components` 1.22.1 (Apache-2.0); tutte compatibili con la nostra licenza **[V: registro npm]** | **`@base-ui/react`**: un solo pacchetto, senza stile, con tutti i componenti che servono. Non ho confrontato le tre sul comportamento con lo screen reader: lo fa il primo task di B2 su finestra modale e menu, e se Base UI delude si passa a Radix senza toccare i token | B2 |
| Q13 | Come pilotare l'app nel fumo | (a) `tauri-driver` + `WebKitWebDriver`, esterni all'app · (b) server WebDriver dentro l'app con `tauri-plugin-wdio-webdriver`, che è il predefinito della documentazione di oggi | **(a)**: il binario provato è identico a quello rilasciato e un client VPN non porta dentro un server di telecomando. (b) solo come ripiego se (a) non gira nel container, dietro una feature `e2e` che la CI verifica assente dal build di rilascio | B1-T6 |
| Q14 | `galleria-gui` e i comandi `--workspace` della fase 1 | (a) escluderlo da `check`, `coverage` e `lab/build.sh`, con job `gui` a parte · (b) installare WebKit e Node ovunque | **(a)**: i job e il laboratorio della fase 1 restano veloci e non dipendono dal frontend. Tocca tre righe di file della fase 1 (`ci.yml`, `lab/build.sh`), non il codice | B1-T1 |
| Q15 | Dove si compila e si prova la GUI in locale: su WSL mancano le librerie di WebKitGTK | (a) immagine Docker `lab/images/gui.Dockerfile` per build, prove e fumo · (b) `sudo apt install` sull'host | **(a) per tutto ciò che è automatico**: nessun `sudo` sulla macchina di Peppe e stesso ambiente della CI. (b) solo se Peppe vuole vedere la finestra con `tauri dev` su WSLg: è un'installazione di sistema, la decide lui | B1-T1 |
| Q16 | Collegamento al demone | (a) una connessione lunga per gli eventi più una breve per ogni comando, senza toccare `galleria-ipc` · (b) dividere il client della fase 1 in lettura e scrittura | **(a)**: nessuna modifica al codice della fase 1; il costo è un `hello` in più per ogni azione dell'utente, su un socket locale | B1-T2 |
| Q17 | Demone finto per le prove di livello 2: l'attrezzatura sta in `crates/galleriad/tests/support`, non esportata | (a) copia ridotta in `crates/galleria-gui/tests/support`, con `galleriad` e `galleria-backend[fake]` come dipendenze di sola prova · (b) crate `galleria-testkit` condiviso | **(a)**: un centinaio di righe duplicate, ma la feature `fake` non entra mai in un build normale. Con (b) `cargo build --workspace` la attiverebbe anche per `galleriad` | B1-T2 |
| Q18 | Identificatore dell'app (nome D-Bus dell'istanza unica, file `.desktop`) | `io.github.zpeppoz.galleria` · un dominio proprio dopo la verifica A1 | **`io.github.zpeppoz.galleria`** ora. Cambiarlo dopo costa poco: profili e stato stanno in `$XDG_CONFIG_HOME/galleria` e `$XDG_STATE_HOME/galleria`, non nelle cartelle derivate dall'identificatore | B1-T1 |
| Q19 | Gestore dei pacchetti JS e versione di Node | pnpm · npm | **pnpm** con `pnpm-lock.yaml` committato e Node fissato in `.nvmrc` alla versione della macchina di Peppe (oggi Node 26.12.1 e pnpm 12.15.0 **[V]**) | B1-T1 |
| Q20 | Casi limite di D25 | portachiavi bloccato: errore oppure ripiego sul file · stessa chiave nei due archivi: quale vale · migrazione automatica dal file | **Errore senza ripiego** se il portachiavi c'è ma è bloccato; **vale il portachiavi** e la copia nel file si cancella dopo una scrittura riuscita; **nessuna migrazione automatica** (§9) | B5 |
| Q21 | Testo residuo dell'analisi fuori dall'elenco approvato: «AUR in fase 3» nella matrice di test (§8) usa ancora il vecchio nome; §3, §10 e R6 parlano di mercato | correggere · lasciare | **Correggere «AUR in fase 3» in «AUR solo con trazione»** in un commit di soli documenti: è lo stesso refuso di Q1. §3, §10 e R6 li riscrive Peppe quando vuole: descrivono un obiettivo che ha cambiato lui | quando capita |
| Q22 | Chi fa la verifica con lo screen reader, e dove | (a) trascrizione di Orca e albero AT-SPI nell'immagine · (b) una persona con Orca su un desktop vero · (c) solo jsdom e axe | **Approvata (a)** da Aitano il 2026-11-25 su delega di Peppe (piano B2, Q22): prova locale, non in CI; la passata a orecchio su un desktop vero si fa in B6 | B2-T1 |
| Q23 | Correzioni ai token del mockup (piano B2: C1–C7, R1, P1, F1–F4) | approvarle tutte · tenere i valori del mockup dove non passano AA · accento più scuro con testo bianco | **Approvata**, tutte come raccomandate, da Aitano il 2026-11-25 su delega di Peppe | B2-T2 |
| Q24 | Barra del titolo | (a) decorazioni del sistema più una barra nostra sotto · (b) barra disegnata da noi | **Approvata (a)** da Aitano il 2026-11-25 su delega di Peppe: nessun permesso nuovo per la finestra | B2-T6 |
| Q25 | Dove si salvano lingua e tema scelti a mano | (a) in B2 solo in memoria, in B3 in `state.toml` · (b) `localStorage` · (c) un comando nuovo in B2 | **Approvata (a)** da Aitano il 2026-11-25 su delega di Peppe | B2-T3 |
| Q26 | Dove finisce nel guscio la vista di B1 (stato vero del demone) | (a) fascia in cima al dettaglio · (b) riga fissa in cima alla lista · (c) toglierla fino a B5 | **Approvata (a)** da Aitano il 2026-11-25 su delega di Peppe | B2-T6 |
| Q27 | Come arrivano a Orca gli esiti e i cambi di stato, visto che le regioni vive di WebKitGTK restano mute (spike `t_b81c3909`) | (1) segnale ATK `announcement`/`notification` dal lato Rust · (2) notifiche del desktop · (3) spostare il fuoco · (4) limite dichiarato · (5) altro `aria-live` | **Approvata** da Aitano il 2026-11-25 su delega di Peppe, come raccomanda lo spike: **(1)** per esiti non bloccanti e cambi di stato, *polite* di norma e *assertive* per gli errori; **(3)** per conferme ed errori che chiedono una risposta; **(4)** per tutto il resto; niente `aria-live` come unico canale, niente notifiche del desktop (§12.1) | B3-T2 |

Le questioni da Q28 in poi stanno nel piano del blocco che le apre (B3: `docs/superpowers/plans/2026-11-25-fase2-b3-rubrica.md`).

| # | Questione | Decisione | Serve entro |
|---|---|---|---|
| Q28 | Orca 46 su Ubuntu 24.04 desktop vero per il nome dei campi di testo | **Approvata (b)** da Aitano il 2026-11-25 su delega di Peppe: nella passata di B6, non una card ora | B6 |
| Q29 | Dove sta il codice nuovo che legge e scrive i file della fase 1 | **Approvata (a)**: solo aggiunte in `galleria-profile`, nessuna firma esistente cambia | B3-T3 |
| Q30 | Formato dei gruppi in `galleria.toml` | **Approvata (a)**: `groups = [...]`, appartenenza in `Profile.group` | B3-T3 |
| Q31 | Contenuto del file di «Esporta» | **Approvata (a)**: file nativo con le chiavi, dopo una conferma, `0600`, scritto da Rust | B3-T8 |
| Q32 | Severità dell'editor raw | **Approvata (a)**: OpenVPN si salva solo senza problemi, WireGuard in sola lettura | B3-T7 |
| Q33 | Quando si scrive sul disco | **Approvata (a)**: «Salva/Annulla» per i campi di testo, subito per interruttori e stella | B3-T5 |
| Q34 | Posizione della finestra in `state.toml` (§6) | **Approvata (a)**: si salvano solo dimensione e «massimizzata»; riduzione rispetto al §6 | B3-T1 |
| Q35 | Chiusura della finestra con modifiche non salvate (aperta dall'esito di B3, T5) | **Decisa: va in B6.** Riportata qui dalla card `t_73168fca`: il testo della decisione non era ancora nel repo | B6 |

Q36–Q41 (istanza unica, laboratorio di G3, percorsi del trascinamento, riconoscimento del formato, più file insieme,
associazione dei `.ovpn`) sono **aperte**, con la raccomandazione, nel piano di B4:
`docs/superpowers/plans/2026-11-25-fase2-b4-import.md`.

## 19. Cosa non ho verificato

- Nessuna prova pratica: né Tauri, né WebDriver, né Secret Service sono stati eseguiti. Su questa macchina la GUI
  oggi non si può nemmeno compilare (mancano le librerie di WebKitGTK).
- Le API di Tauri citate (`AppManifest::commands`, `tauri::test`, `WindowEvent::DragDrop`, `fileAssociations`) le
  ho lette nella documentazione e nei sorgenti del ramo `dev`, non nella versione 2.12.7 fissata: ogni piano le
  ricontrolla sulla versione in `Cargo.lock` prima di scrivere codice.
- Le versioni dei pacchetti sono quelle dei registri al 2026-11-25; i piani fissano quelle del momento.
- Non ho confrontato le tre librerie «headless» sull'accessibilità reale (Q12).
- Le pagine ufficiali di Tauri le ho lette dai sorgenti del sito (`tauri-apps/tauri-docs`, ramo `v2`), perché
  l'estrattore tagliava le pagine pubblicate: il testo è lo stesso, la pubblicazione potrebbe essere indietro.

## Fonti esterne lette il 2026-11-25

- Tauri 2, istanza unica: https://v2.tauri.app/plugin/single-instance/ (sorgente: `tauri-apps/tauri-docs`, `src/content/docs/plugin/single-instance.mdx`)
- Tauri 2, apertura dei file: `RunEvent::Opened` in https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/src/app.rs ; esempio ufficiale https://github.com/tauri-apps/tauri/blob/dev/examples/file-associations/src-tauri/src/main.rs
- Tauri 2, argomenti su Linux e Windows («deep links are delivered as a command line argument to a new app process»): https://v2.tauri.app/plugin/deep-linking/
- Tauri 2, WebDriver: https://v2.tauri.app/develop/tests/webdriver/ ; https://v2.tauri.app/develop/tests/webdriver/manual-setup/ ; https://v2.tauri.app/develop/tests/webdriver/ci/
- WebdriverIO, Tauri: https://webdriver.io/docs/desktop-testing/tauri/platform-support
- Tauri 2, permessi e CSP: https://v2.tauri.app/security/capabilities/ ; https://v2.tauri.app/security/csp/
- Tauri 2, prerequisiti Linux: https://v2.tauri.app/start/prerequisites/
- Tauri 2, simulazione dei comandi: https://v2.tauri.app/develop/tests/mocking/ ; modulo `tauri::test`: https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/src/test/mod.rs
- Tauri 2, tray: https://v2.tauri.app/learn/system-tray/
- CLI di Tauri, ricerca della configurazione: https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/helpers/app_paths.rs
- Versioni e licenze: https://crates.io (`tauri`, `tauri-build`, `tauri-plugin-single-instance`, `tauri-driver`, `keyring`, `secret-service`, `oo7`) ; https://registry.npmjs.org (`@base-ui/react`, `radix-ui`, `react-aria-components`, `@tauri-apps/api`, `@wdio/tauri-service`)
