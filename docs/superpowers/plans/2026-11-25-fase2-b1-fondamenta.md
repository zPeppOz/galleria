# Fase 2 · Blocco B1 — Fondamenta della GUI: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** una finestra Tauri che gira da utente normale, si collega al demone della fase 1, ne mostra lo
stato vero in sola lettura, si ricollega da sola se il demone riparte e guida l'utente nei quattro casi del primo
avvio. Chiude i criteri **G1** e **G2** della spec e scioglie i punti **S1–S7** e **S13**.

**Architettura:** crate `galleria-gui` nel workspace (lato Rust di Tauri) più `ui/` (React, TypeScript, Vite). Un
task Rust (`supervisor`) tiene una connessione lunga al demone per gli eventi e li inoltra alla webview; la
webview vede solo tre comandi e quattro eventi. Nessuna modifica a demone, IPC, profili.

**Stack:** Rust 1.96.0, Tauri 2 (2.12.7 al 2026-11-25), tokio; React 19, TypeScript, Vite, Vitest, Testing
Library, axe; WebdriverIO, `tauri-driver`, `WebKitWebDriver`, Xvfb per il fumo; Docker per l'immagine di build.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md` (da leggere per intero prima del T1; «spec §n»
punta lì, «spec F1 §n» alla spec della fase 1). Decisioni: D1–D32, non si riaprono.

**Prima di partire:** le questioni Q13–Q19 della spec §18 devono avere una risposta. Se nessuno ha risposto, vale
la raccomandazione scritta lì: questo piano è scritto su quelle. Ramo di lavoro in un worktree dedicato, a
partire da `main` (che oggi contiene la fase 1 e deve contenere anche questo piano).

## Vincoli globali

- Un task = un commit. Messaggi in inglese, minuscolo, imperativo, senza prefisso. Nessun push, nessuna PR.
- TDD: prima la prova, la si vede fallire per il motivo giusto, poi il codice.
- **Non si toccano**: `crates/galleriad`, `crates/galleria-ipc`, `crates/galleria-backend*`, `crates/galleria-net`,
  `crates/galleria-ovpn`, `crates/galleria-profile`, `crates/galleria-cli`, `spikes/`, `design/`, `docs/00`–`07`.
  Eccezioni, solo quelle elencate nei task: `Cargo.lock`, `deny.toml`, `scripts/check-boundaries.sh`,
  `.github/workflows/ci.yml`, `lab/build.sh`, `ROADMAP.md`. Se un task sembra chiedere una modifica al codice
  della fase 1, ci si ferma e si riporta.
- `PROTOCOL_VERSION` resta 1. La GUI non manda al demone nulla che la CLI non mandi già.
- `#![forbid(unsafe_code)]` in `galleria-gui`. Niente `unwrap()`/`expect()` fuori dalle prove (le regole di
  clippy del workspace lo negano); `thiserror` nei moduli, `anyhow` solo in `main.rs`.
- La GUI non esegue mai `sudo`, `pkexec`, `systemctl` né altri processi. Nessun segreto in argomenti, variabili
  d'ambiente, log o valori restituiti dai comandi.
- La webview non ha plugin con accesso a file, shell o rete. Ogni permesso aggiunto a
  `capabilities/main.json` deve comparire nella prova `tests/config.rs`, che elenca quelli ammessi.
- **Dove girano i comandi.** Sulla macchina di sviluppo mancano le librerie di WebKitGTK: tutto ciò che compila
  `galleria-gui` o usa Node passa da `scripts/gui.sh` (immagine Docker, T1). I controlli della fase 1 girano
  sull'host con `--exclude galleria-gui`. Ogni task finisce con tutti e due i gruppi verdi:

  ```bash
  # host: la fase 1 non deve accorgersi della GUI
  cargo fmt --all --check
  cargo clippy --workspace --exclude galleria-gui --all-targets -- -D warnings
  cargo test --workspace --exclude galleria-gui
  cargo deny check
  scripts/check-boundaries.sh
  # immagine: la GUI
  scripts/gui.sh check
  ```

- Dipendenze nuove: Rust solo con licenza ammessa da `deny.toml`; JS di produzione solo MIT, Apache-2.0, BSD, ISC.
  Una licenza nuova si aggiunge a `deny.toml` solo se è compatibile con GPL-3.0-or-later, e si scrive nel
  messaggio del commit. Una licenza incompatibile ferma il task.
- API di Tauri, WebdriverIO e Vitest: verificare nomi e firme sulla documentazione **della versione fissata nei
  lockfile** prima di scrivere il codice. I nomi di questo piano valgono per il **nostro** codice.
- Testi dell'interfaccia: italiano e inglese in `ui/src/messages.ts`, mai scritti dentro i componenti. Niente emoji.
- Se un punto **[S]** risulta falso si applica la colonna «Se è falsa» della spec §17 e lo si annota in
  `docs/08-esito-fase2.md`. Se il ripiego tocca una decisione D, ci si ferma e si riporta.

## Punti di attenzione per la revisione

1. **`cargo deny` e l'albero di Tauri.** Su Linux Tauri porta le librerie GTK3 per Rust, che hanno avvisi di
   «non più mantenuto»; possono comparire licenze non ancora in elenco. Gli avvisi si ignorano **per
   identificativo**, ognuno con il motivo in `deny.toml` e nell'esito; mai disattivando la categoria. È una mia
   previsione, non l'ho eseguito: il T1 la verifica.
2. **Il fumo non deve dipendere dal tempo.** Le attese sono «finché compare X, al più N secondi», mai `sleep` fissi.
3. **Il demone chiude senza rispondere** a chi non passa l'autorizzazione (spec F1 §5.1): è `permission_denied`,
   non `daemon_not_running` → T2 `daemon_closing_before_hello_reply_is_permission_denied`.
4. **Socket rimasto da un demone morto**: il file c'è ma nessuno ascolta → `daemon_not_running` → T2
   `stale_socket_file_is_daemon_not_running`.
5. **Prove come root.** Nell'immagine Docker si è root e i permessi dei file non fermano root: le prove che
   contano su «permesso negato» si saltano con un messaggio se `euid == 0`, e girano nel job `gui` (utente normale).
6. **Raffica di eventi.** `event.stats` arriva una volta al secondo e `event.log` a raffiche: il `supervisor` non
   deve mai bloccarsi sulla webview → T3 `slow_sink_does_not_stall_the_event_connection`.
7. **Ordine.** Il primo `galleria:link` con `connected` deve portare la `status` di `hello`: la webview non deve
   vedere uno stato vuoto tra il collegamento e il primo evento → T3 `connected_view_carries_hello_status`.

## Mappa dei file

```
package.json  pnpm-workspace.yaml  pnpm-lock.yaml  .nvmrc            T1
crates/galleria-gui/
  Cargo.toml  build.rs  tauri.conf.json  icons/                      T1 (build.rs anche T4)
  capabilities/main.json                                             T1, T4
  src/main.rs  src/lib.rs                                            T1, T4
  src/link.rs                                                        T2
  src/supervisor.rs                                                  T3
  src/error.rs  src/events.rs  src/commands/{mod,daemon,app}.rs      T4
  tests/config.rs                                                    T1, T4
  tests/support/mod.rs  tests/link.rs                                T2
  tests/supervisor.rs                                                T3
  tests/commands.rs                                                  T4
ui/
  package.json  tsconfig.json  vite.config.ts  index.html            T1
  src/main.tsx  src/App.tsx  src/App.test.tsx                        T1, T5
  src/messages.ts  src/styles.css                                    T5
  src/daemon/{types,store}.ts  src/daemon/store.test.ts              T5
  src/screens/{FirstRun,StatusView}.tsx  + .test.tsx                 T5
  e2e/wdio.conf.ts  e2e/smoke.e2e.ts                                 T6
lab/images/gui.Dockerfile                                            T1, T6
scripts/gui.sh  scripts/tests/check-boundaries.sh                    T1 (gui.sh anche T6)
lab/tests/gui-smoke.sh                                               T6
docs/08-esito-fase2.md                                               T1 (creato), aggiornato a ogni task
```

Modificati: `scripts/check-boundaries.sh`, `.github/workflows/ci.yml`, `lab/build.sh`, `deny.toml` (se serve),
`Cargo.lock`, `ROADMAP.md` (T6).

---

### Task 1: crate, `ui/`, immagine di build, confini, CI

**Stima:** 6 h. **Scioglie:** S1, S2, S3; applica Q14, Q15, Q18, Q19.

**File — crea:** `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, `.nvmrc`, `crates/galleria-gui/{Cargo.toml,
build.rs,tauri.conf.json,capabilities/main.json,icons/,src/main.rs,src/lib.rs,tests/config.rs}`,
`ui/{package.json,tsconfig.json,vite.config.ts,index.html,src/main.tsx,src/App.tsx,src/App.test.tsx}`,
`lab/images/gui.Dockerfile`, `scripts/gui.sh`, `scripts/tests/check-boundaries.sh`, `docs/08-esito-fase2.md`.
**Modifica:** `scripts/check-boundaries.sh`, `.github/workflows/ci.yml`, `lab/build.sh`, `deny.toml` (solo se serve).

**Produce:**

- `scripts/gui.sh <image|run CMD…|check>`: `image` costruisce `galleria-gui-dev` da `lab/images/gui.Dockerfile`
  (Ubuntu 24.04 più i pacchetti dei prerequisiti di Tauri, spec §14, più Node della versione di `.nvmrc` e pnpm);
  `run` esegue un comando nel repo montato, con lo stesso schema di `lab/build.sh` (toolchain dell'host in sola
  lettura, utente che invoca, Docker rootless compreso) e `CARGO_TARGET_DIR=target/gui-image`; `check` esegue in
  ordine: `pnpm install --frozen-lockfile`, `pnpm --dir ui test`, `pnpm --dir ui build`,
  `cargo clippy -p galleria-gui --all-targets -- -D warnings`, `cargo test -p galleria-gui`.
- `tauri.conf.json`: `identifier: "io.github.zpeppoz.galleria"`, `productName: "Galleria"`, una finestra `main`
  1180×740, `frontendDist: "../../ui/dist"`, CSP della spec §3.4, `withGlobalTauri: false`, `freezePrototype: true`.
- `capabilities/main.json`: finestra `main`, permessi `core:event:allow-listen` e `core:event:allow-unlisten`.
- `App.tsx`: per ora solo l'intestazione «Galleria».
- `scripts/check-boundaries.sh`: oltre alla regola esistente, fallisce se l'albero di `galleriad` contiene
  `galleria-gui`. Per poterlo provare legge l'albero da `BOUNDARY_TREE` quando la variabile è impostata.

- [ ] **1. Prove che falliscono.**

  `scripts/tests/check-boundaries.sh`:

  ```bash
  #!/usr/bin/env bash
  set -euo pipefail
  cd -- "$(dirname -- "$0")/../.."
  BOUNDARY_TREE=$'galleriad v0.1.0\ngalleria-ipc v0.1.0' scripts/check-boundaries.sh >/dev/null
  for forbidden in galleria-profile galleria-gui; do
    if BOUNDARY_TREE=$'galleriad v0.1.0\n'"$forbidden v0.1.0" scripts/check-boundaries.sh >/dev/null 2>&1; then
      echo "boundary check accepted $forbidden" >&2; exit 1
    fi
  done
  echo "boundary check rejects forbidden dependencies"
  ```

  `crates/galleria-gui/tests/config.rs` (legge i due file JSON con `serde_json`, senza avviare Tauri):

  | Prova | Asserzione |
  |---|---|
  | `csp_is_set_and_has_no_remote_host` | `app.security.csp` è una stringa non vuota; l'unico `http` ammesso è `http://ipc.localhost`; nessun `https:`, nessun `*`, nessun `unsafe-eval` |
  | `global_tauri_is_off_and_prototype_is_frozen` | `withGlobalTauri == false`, `freezePrototype == true` |
  | `identifier_is_the_agreed_one` | `io.github.zpeppoz.galleria` |
  | `main_capability_grants_only_listed_permissions` | `windows == ["main"]`; `permissions` è **uguale** all'elenco `ALLOWED` scritto nella prova |
  | `no_plugin_is_configured` | `plugins` assente o vuoto |

  `ui/src/App.test.tsx`: `renders_the_app_title` (c'è un'intestazione di livello 1 «Galleria»).

- [ ] **2.** Le prove falliscono: la prima perché lo script non conosce `galleria-gui`, le altre perché i file mancano.
- [ ] **3. Implementa**, in quest'ordine, annotando ogni esito in `docs/08-esito-fase2.md` (sezione «B1», tabella
  dei punti [S]):
  1. Immagine e `scripts/gui.sh`.
  2. Crate e `ui/` minimi. **S2**: prima di creare `ui/dist`, `scripts/gui.sh run cargo check -p galleria-gui`:
     annota se fallisce e con quale messaggio.
  3. **S1**: `scripts/gui.sh run pnpm tauri info` e `pnpm tauri build --no-bundle` dalla radice: annota se la CLI
     trova `crates/galleria-gui/tauri.conf.json`.
  4. **S3**: confronta il binario fatto con `cargo build -p galleria-gui --release` e quello fatto dalla CLI: quale
     dei due contiene i file di `ui/dist` (`strings` sul binario, cercando il titolo di `index.html`). Scrivi
     nell'esito il comando di build che il resto del piano deve usare e mettilo in `scripts/gui.sh build`.
  5. `lab/build.sh` e i job `check` e `coverage` di `ci.yml`: `--workspace` → `--workspace --exclude galleria-gui`
     (due righe in `lab/build.sh`, tre in `ci.yml`). Job nuovo `gui` su `ubuntu-latest`: pacchetti dei
     prerequisiti, toolchain come negli altri job, Node da `.nvmrc`, pnpm, poi gli stessi passi di
     `scripts/gui.sh check` più `scripts/tests/check-boundaries.sh`. Azioni fissate a un commit, come le altre.
  6. `cargo deny check`: vedi il punto di attenzione 1.
- [ ] **4.** Verde: i due gruppi di comandi dei vincoli globali, `scripts/tests/check-boundaries.sh`, e
  `LAB_BUILD=host lab/build.sh 26` arriva in fondo senza compilare `galleria-gui`.
- [ ] **5. Commit:** `add gui crate, ui package, build image and ci job`

**Fatto quando:** `scripts/gui.sh check` esce 0 partendo da un clone pulito; `cargo test --workspace --exclude
galleria-gui` dà lo stesso numero di prove di prima del task; S1, S2 e S3 hanno un esito scritto.

---

### Task 2: stato del collegamento e classificazione degli esiti (D27, G2)

**Stima:** 5 h. **Applica:** Q16, Q17. **File — crea:** `crates/galleria-gui/src/link.rs`,
`crates/galleria-gui/tests/support/mod.rs`, `crates/galleria-gui/tests/link.rs`. **Modifica:** `Cargo.toml` del
crate (dipendenze: `galleria-ipc` con `client`, `tokio`, `serde`, `thiserror`; di sola prova: `galleriad`,
`galleria-backend` con `fake`, `tempfile`, `rstest`, `nix`, `tokio-util`, `serde_json`).

**Interfacce — usa:** `galleria_ipc::client::{Client, ClientError}`, `HelloResult`, `BackendInfo`, `Status`,
`RpcError`. **Produce:**

```rust
// link.rs — nessun import di `tauri`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState { Connecting, Connected, DaemonNotRunning, PermissionDenied, IncompatibleProtocol }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Incompatible { pub daemon_min: u32, pub daemon_max: u32, pub daemon_version: String }

#[derive(Debug, Clone, Serialize)]
pub struct DaemonView {
    pub link: LinkState,
    pub daemon_version: Option<String>,
    pub incompatible: Option<Incompatible>,
    pub backends: Vec<BackendInfo>,
    pub status: Option<Status>,
}
impl DaemonView { pub fn connecting() -> Self; pub fn connected(hello: &HelloResult) -> Self; pub fn failed(f: &LinkFailure) -> Self; }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFailure { pub state: LinkState, pub incompatible: Option<Incompatible>, pub detail: String }

pub fn classify(err: &ClientError) -> LinkFailure;                         // pura: tabella della spec §5
pub async fn attempt(socket: &Path, client_version: &str) -> Result<(Client, HelloResult), LinkFailure>;
pub fn socket_path(env_override: Option<OsString>) -> PathBuf;             // GALLERIA_SOCKET, poi DEFAULT_SOCKET_PATH
```

`attempt` si presenta al demone come `galleria-gui`. `detail` è il testo dell'errore per il log della GUI: non
esce dai comandi. `tests/support/mod.rs` è una copia ridotta di `crates/galleriad/tests/support/mod.rs` più
l'avvio di `serve` di `crates/galleriad/tests/ipc_server.rs`: `FakeDaemon::start(scripts, info) -> FakeDaemon
{ socket, harness, stop() }`, più `Scripted::listen(dir, |stream| …)` per i server a copione.

- [ ] **1. Prove che falliscono.** In `link.rs`, pure:

```rust
#[rstest]
#[case::no_socket(io(ErrorKind::NotFound), LinkState::DaemonNotRunning)]
#[case::nobody_listening(io(ErrorKind::ConnectionRefused), LinkState::DaemonNotRunning)]
#[case::kernel_refused(ClientError::PermissionDenied, LinkState::PermissionDenied)]
#[case::closed_before_reply(ClientError::Closed, LinkState::PermissionDenied)]
#[case::too_many_clients(rpc("too_many_clients"), LinkState::DaemonNotRunning)]
#[case::other_io(io(ErrorKind::InvalidData), LinkState::DaemonNotRunning)]
fn classify_follows_the_spec_table(#[case] err: ClientError, #[case] expected: LinkState) {
    assert_eq!(classify(&err).state, expected);
}

#[test]
fn incompatible_protocol_carries_the_daemon_range() {
    let err = ClientError::Rpc(RpcError { code: -32000, message: "x".into(),
        data: Some(json!({"code":"incompatible_protocol","daemon_min":2,"daemon_max":3,"daemon_version":"0.2.0"})) });
    let failure = classify(&err);
    assert_eq!(failure.state, LinkState::IncompatibleProtocol);
    assert_eq!(failure.incompatible, Some(Incompatible { daemon_min: 2, daemon_max: 3, daemon_version: "0.2.0".into() }));
}
```

  `tests/link.rs`:

  | Prova | Preparazione | Asserzione |
  |---|---|---|
  | `fake_daemon_gives_connected_with_disconnected_status` | `FakeDaemon` | `Ok`; `hello.status.state == Disconnected` |
  | `missing_socket_is_daemon_not_running` | cartella vuota | `DaemonNotRunning` |
  | `stale_socket_file_is_daemon_not_running` | `UnixListener` creato e chiuso, file lasciato | `DaemonNotRunning` |
  | `unreadable_socket_is_permission_denied` | socket con modo `000`; saltata se `euid == 0` | `PermissionDenied` |
  | `daemon_closing_before_hello_reply_is_permission_denied` | `Scripted` che accetta e chiude | `PermissionDenied` |
  | `hello_out_of_range_is_incompatible_protocol` | `Scripted` che risponde con l'errore `incompatible_protocol` e chiude | `IncompatibleProtocol` con i tre campi |
  | `unavailable_backend_is_kept_in_the_view` | `FakeDaemon` con `DaemonInfo.backends = [openvpn, available: false, openvpn_not_found]` | `DaemonView::connected(&hello).backends[0].unavailable_reason == Some("openvpn_not_found")` |
  | `view_serializes_with_snake_case_link` | — | JSON di `DaemonView::connecting()`: `{"link":"connecting","daemon_version":null,"incompatible":null,"backends":[],"status":null}` |
  | `socket_path_prefers_the_env_override` | — | con `Some("/tmp/x.sock")` quello; con `None` `DEFAULT_SOCKET_PATH` |

- [ ] **2.** Falliscono (`link` non esiste). **3.** Implementa `link.rs`; nessuna logica nelle prove di supporto
  oltre a quella copiata.
- [ ] **4.** `scripts/gui.sh run cargo test -p galleria-gui link` → verde; poi i due gruppi dei vincoli globali.
- [ ] **5. Commit:** `add daemon link state and connection outcome classification`

**Fatto quando:** le prove passano senza privilegi; `grep -rn "tauri" crates/galleria-gui/src/link.rs` non trova
nulla; `cargo tree -p galleria-gui -e normal` non contiene `galleriad` né `galleria-backend`.

---

### Task 3: connessione degli eventi e ricollegamento (G1)

**Stima:** 6 h. **File — crea:** `crates/galleria-gui/src/supervisor.rs`, `crates/galleria-gui/tests/supervisor.rs`.

**Interfacce — usa:** `link::{attempt, DaemonView, LinkFailure}`, `Client::{call, next_event}`,
`SubscribeParams`, `Event`. **Produce:**

```rust
// supervisor.rs — nessun import di `tauri`
pub trait EventSink: Send + Sync + 'static {
    fn link(&self, view: &DaemonView);
    fn state(&self, status: &Status);
    fn stats(&self, stats: &Stats);
    fn log(&self, entry: &LogEntry);
}
pub struct RelinkPolicy { pub delays: Vec<Duration> }      // predefinito 500 ms, 1 s, 2 s, 5 s; l'ultimo si ripete
pub struct SupervisorHandle { /* … */ }
impl SupervisorHandle {
    pub fn view(&self) -> DaemonView;                       // ultima fotografia (tokio::sync::watch)
    pub fn retry_now(&self);                                // salta l'attesa in corso
    pub async fn shutdown(self);                            // chiude la connessione e aspetta il task
}
pub fn spawn(socket: PathBuf, client_version: String, sink: Arc<dyn EventSink>, policy: RelinkPolicy) -> SupervisorHandle;
```

Ciclo del task: `sink.link(connecting)` → `attempt` → se riesce: `subscribe {stats: true, logs: true}`,
`sink.link(connected con la status di hello)`, poi `next_event()` in ciclo (`State` aggiorna anche la `status`
della fotografia); a `None` torna a tentare. Se `attempt` fallisce: `sink.link(failed)` **solo se è cambiato
qualcosa** rispetto all'ultima fotografia, poi attesa della politica o `retry_now`. Le chiamate al `sink` non
devono bloccare il ciclo: il `sink` di Tauri (T4) è già non bloccante; quello di prova lento serve a dimostrarlo.

- [ ] **1. Prove che falliscono.** `tests/supervisor.rs`, con un `Recorder` che implementa `EventSink` e accoda
  tutto in un canale; attese sempre «finché arriva X, al più 5 s»; `RelinkPolicy` di prova con 20 ms.

  | Prova | Asserzione |
  |---|---|
  | `first_views_are_connecting_then_connected` | i primi due `link` sono `connecting`, `connected` |
  | `connected_view_carries_hello_status` | il `link` `connected` ha `status.state == Disconnected` e `daemon_version` |
  | `state_events_are_forwarded_in_order` | un secondo client fa `connect` sul `FakeDaemon`: il `Recorder` riceve `connecting` poi `connected`, e `view().status.state == Connected` |
  | `stats_and_logs_arrive_after_subscribe` | copione con `FakeEvent::Stats` e `FakeEvent::Log`: arrivano entrambi |
  | `daemon_restart_relinks_without_user_action` | `stop()` del demone → `link` `daemon_not_running`; nuovo `FakeDaemon` sullo stesso percorso → `link` `connected` |
  | `repeated_failures_emit_one_link_event` | senza demone, dopo 10 tentativi il `Recorder` ha un solo `daemon_not_running` |
  | `retry_now_skips_the_wait` | politica a 60 s, senza demone; parte il demone, `retry_now()` → `connected` entro 2 s |
  | `incompatible_daemon_is_reported_and_retried` | `Scripted` incompatibile → `link` `incompatible_protocol` con le versioni; sostituito da un `FakeDaemon` → `connected` |
  | `slow_sink_does_not_stall_the_event_connection` | `Recorder` che dorme 200 ms per chiamata, 50 righe di log dal copione: arrivano tutte e 50, in ordine di `seq`, e il demone non chiude la connessione |
  | `shutdown_closes_the_connection` | dopo `shutdown()`, lo `Scripted` che ha accettato la connessione legge EOF |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** `scripts/gui.sh run cargo test -p galleria-gui supervisor`, ripetuto
  20 volte di fila per stanare le prove instabili (`for i in $(seq 20); do …; done`); poi i due gruppi.
- [ ] **5. Commit:** `add daemon event supervisor with automatic relink`

**Fatto quando:** 20 esecuzioni consecutive verdi; nessun `sleep` fisso nelle prove oltre al `Recorder` lento.

---

### Task 4: comandi ed eventi Tauri, permessi (D29)

**Stima:** 5 h. **Scioglie:** S4. **File — crea:** `crates/galleria-gui/src/{error,events}.rs`,
`crates/galleria-gui/src/commands/{mod,daemon,app}.rs`, `crates/galleria-gui/tests/commands.rs`. **Modifica:**
`src/lib.rs`, `src/main.rs`, `build.rs`, `capabilities/main.json`, `tests/config.rs`, `Cargo.toml` (feature `test`
di `tauri` tra le dipendenze di sola prova).

**Produce:**

```rust
// error.rs
#[derive(Debug, Clone, Serialize)]
pub struct CmdError { pub code: String, pub message: String, #[serde(skip_serializing_if = "Option::is_none")] pub data: Option<serde_json::Value> }
// commands/daemon.rs
#[tauri::command] pub fn daemon_view(state: tauri::State<'_, SupervisorHandle>) -> DaemonView;
#[tauri::command] pub fn daemon_retry(state: tauri::State<'_, SupervisorHandle>);
// commands/app.rs
#[derive(Serialize)] pub struct AppInfo { pub version: String, pub socket_path: String, pub user: String, pub group: String }
#[tauri::command] pub fn app_info(state: tauri::State<'_, AppInfo>) -> AppInfo;
// events.rs
pub const EV_LINK: &str = "galleria:link";   // e EV_STATE, EV_STATS, EV_LOG
pub struct TauriSink<R: tauri::Runtime>(pub tauri::AppHandle<R>);          // impl EventSink: una `emit` per chiamata
// lib.rs
pub fn configure<R: tauri::Runtime>(builder: tauri::Builder<R>, socket: PathBuf) -> tauri::Builder<R>;
```

`configure` registra i tre comandi e, in `setup`, avvia il `supervisor` con `TauriSink` e lo mette nello stato
dell'app. `AppInfo.user` è il nome dell'utente che esegue (dal sistema, non da `$USER`); `group` è `galleria`.
`build.rs` dichiara i tre comandi con `AppManifest::commands`; `capabilities/main.json` aggiunge i tre permessi
corrispondenti (nome esatto dalla documentazione della versione fissata) e la lista `ALLOWED` di
`tests/config.rs` cresce degli stessi tre. `main.rs` gestisce l'errore di avvio senza `expect`.

- [ ] **1. Prove che falliscono.** `tests/commands.rs`, con il runtime finto di `tauri::test` e il `FakeDaemon`:

  | Prova | Asserzione |
  |---|---|
  | `daemon_view_is_connected_against_the_fake_daemon` | la risposta al comando, come JSON, ha `link == "connected"` (attesa: al più 5 s) |
  | `daemon_view_without_daemon_is_daemon_not_running` | `link == "daemon_not_running"` |
  | `daemon_retry_relinks_immediately` | politica a 60 s; parte il demone; dopo il comando `daemon_view` dà `connected` entro 2 s |
  | `app_info_reports_user_group_and_socket` | `user` uguale al nome dell'utente che esegue la prova, `group == "galleria"`, `socket_path` quello passato a `configure` |
  | `state_event_reaches_a_listener` | un ascoltatore su `galleria:state` riceve `connecting` quando un altro client fa `connect` |
  | `link_event_reaches_a_listener` | fermato il demone, un ascoltatore su `galleria:link` riceve `daemon_not_running` |
  | `command_outputs_never_contain_secret_fields` | nel JSON di `daemon_view` e `app_info`, a ogni profondità, nessuna chiave tra `private_key`, `preshared_key`, `password`, `otp`, `key_passphrase`, `secret_blocks`, `config` |
  | `cmd_error_serializes_code_and_message` | `{"code":"gui_x","message":"y"}`, senza `data` quando è `None` |

  In `tests/config.rs`: `every_registered_command_has_a_permission` (i comandi dichiarati in `build.rs` e quelli
  in `ALLOWED` coincidono: la prova legge `build.rs` come testo).

- [ ] **2.** Falliscono. **3.** Implementa. Se `tauri::test` non permette di invocare i comandi o di ascoltare gli
  eventi (S4 falsa): le prove chiamano le funzioni dei comandi direttamente, con uno stato costruito a mano, e
  l'inoltro degli eventi si prova nel fumo (T6); lo si scrive nell'esito.
- [ ] **4.** `scripts/gui.sh run cargo test -p galleria-gui`; `scripts/gui.sh build` e avvio a mano nell'immagine
  sotto `xvfb-run` per 5 s senza panico (`timeout 5 xvfb-run … ; test $? -eq 124`); poi i due gruppi.
- [ ] **5. Commit:** `expose daemon view commands and events to the webview`

**Fatto quando:** `grep -rn "tauri" crates/galleria-gui/src/{link,supervisor}.rs` non trova nulla; i permessi in
`capabilities/main.json` sono cinque (due di ascolto, tre comandi) e la prova li elenca tutti.

---

### Task 5: frontend — stato del demone e primo avvio (G2)

**Stima:** 6 h. **Scioglie:** S7. **File — crea:** `ui/src/messages.ts`, `ui/src/styles.css`,
`ui/src/daemon/{types,store}.ts`, `ui/src/daemon/store.test.ts`, `ui/src/screens/{FirstRun,StatusView}.tsx` con le
loro `.test.tsx`. **Modifica:** `ui/src/App.tsx`, `ui/src/App.test.tsx`, `ui/package.json` (Testing Library, axe).

**Produce:**

- `types.ts`: i tipi TypeScript di `DaemonView`, `Status`, `BackendInfo`, `AppInfo`, scritti a mano e uguali ai
  nomi dei campi Rust (`snake_case`).
- `store.ts`: `useDaemon()` → `{ view, info, retry() }`. All'avvio invoca `daemon_view` e `app_info` e ascolta
  `galleria:link` e `galleria:state`; alla chiusura toglie gli ascoltatori.
- `App.tsx`: `link !== "connected"` → `FirstRun`; altrimenti `StatusView`. Barra in basso con lo stato del
  collegamento (spec §16, punto 5).
- `FirstRun`: un pannello per `daemon_not_running`, `permission_denied`, `incompatible_protocol`, più l'attesa per
  `connecting`. Comando in un blocco a carattere fisso con «Copia»; «Riprova» chiama `retry()`.
- `StatusView`, sola lettura: stato (sette etichette), nome del profilo se c'è una sessione, protocollo,
  indirizzi, `net` (interfaccia, DNS, rotte), errore con codice; un avviso per ogni backend con
  `available: false`. Nessun pulsante di connessione: arriva in B5.
- `styles.css`: i token del mockup copiati così come sono (spec §12); B2 li sposta nel design system.
- `messages.ts`: `{ en: {...}, it: {...} }` con le stesse chiavi; lingua da `navigator.language`, inglese come ripiego.

- [ ] **1. Prove che falliscono** (Vitest, Testing Library, `mockIPC` di `@tauri-apps/api/mocks`):

  | File | Prova | Asserzione |
  |---|---|---|
  | `store.test.ts` | `loads_the_view_and_app_info_on_start` | dopo il montaggio `view.link` e `info.user` vengono dai comandi simulati |
  | | `follows_link_and_state_events` | un evento `galleria:link` simulato cambia `view.link`; un `galleria:state` cambia `view.status.state` |
  | | `retry_invokes_daemon_retry` | il comando simulato è chiamato una volta |
  | `FirstRun.test.tsx` | `daemon_not_running_shows_the_systemctl_command` | il testo `sudo systemctl enable --now galleriad` è nel documento |
  | | `permission_denied_shows_usermod_with_the_user_name` | `sudo usermod -aG galleria mrossi` con `info.user = "mrossi"`, e l'avviso di rientrare nella sessione |
  | | `incompatible_protocol_shows_both_versions` | versione della GUI e `daemon_version` nel testo |
  | | `copy_puts_the_command_on_the_clipboard` | `navigator.clipboard.writeText` chiamata con il comando esatto |
  | | `nothing_is_executed` | nessun comando simulato oltre a `daemon_view`, `app_info`, `daemon_retry` viene mai invocato |
  | `StatusView.test.tsx` | `each_of_the_seven_states_has_a_label` | parametrica sui sette valori di `state`, in `it` e in `en` |
  | | `unavailable_openvpn_shows_a_warning_with_the_reason` | testo per `openvpn_not_found` e per `openvpn_too_old`; WireGuard senza avviso |
  | | `session_error_shows_code_and_daemon_message` | codice e `message` inglese come ripiego |
  | | `there_is_no_connect_control` | nessun pulsante con nome «Connetti» o «Disconnetti» |
  | tutte le schermate | `has_no_accessibility_violations` | axe senza violazioni |
  | | `is_usable_from_the_keyboard` | con il solo `Tab` si raggiungono «Copia» e «Riprova»; `Enter` li attiva |
  | `messages.ts` (prova accanto) | `every_key_exists_in_both_languages` | stesso insieme di chiavi in `en` e `it` |

- [ ] **2.** Falliscono. **3.** Implementa. **S7**: nell'immagine, sotto `xvfb-run`, avvia l'app senza demone e
  verifica a mano o con uno script WebDriver provvisorio che «Copia» riempia gli appunti; annota l'esito. Se non
  funziona, applica il ripiego della spec §17 **in questo task** (comando `copy_text`, permesso e prova compresi).
- [ ] **4.** `scripts/gui.sh check`; poi il gruppo dell'host.
- [ ] **5. Commit:** `add first run screens and read-only daemon status view`

**Fatto quando:** nessuna stringa visibile è scritta dentro un componente (`grep` di lettere accentate e di parole
italiane in `ui/src/screens/` non trova testo fuori dalle chiavi); axe verde su tutte le schermate.

---

### Task 6: fumo WebDriver nell'immagine e in CI, esito del blocco

**Stima:** 6 h, con un tetto: se dopo 4 h il fumo non gira con `tauri-driver`, si prova per al più 2 h il ripiego
di Q13 (server dentro l'app, dietro la feature `e2e`); se non gira nemmeno quello, lista di controllo manuale.
**Scioglie:** S5, S6. **File — crea:** `ui/e2e/wdio.conf.ts`, `ui/e2e/smoke.e2e.ts`, `lab/tests/gui-smoke.sh`.
**Modifica:** `lab/images/gui.Dockerfile` (`webkit2gtk-driver`, `xvfb`, `dbus`, `tauri-driver` installato con
`cargo install tauri-driver --locked`), `scripts/gui.sh` (`smoke`), `ui/package.json`, `.github/workflows/ci.yml`
(job `gui-smoke`, `continue-on-error: true`), `docs/08-esito-fase2.md`, `ROADMAP.md`, `.claude/handoffs/`.

**Produce:** `scripts/gui.sh smoke`, che nell'immagine: costruisce l'app con il comando fissato nel T1 e
`galleriad` con `cargo build -p galleriad`; poi lancia `lab/tests/gui-smoke.sh`. Lo script crea una cartella
temporanea, avvia `galleriad run --socket <tmp>/galleriad.sock --group <gruppo di chi esegue> --runtime-dir <tmp>`,
esporta `GALLERIA_SOCKET`, e lancia WebdriverIO dentro `dbus-run-session -- xvfb-run`. Alla fine, sempre: ferma
demone e driver e cancella la cartella. Non tocca rotte, DNS o interfacce: nessuna prova chiede una connessione.

- [ ] **1. Prove che falliscono.** `ui/e2e/smoke.e2e.ts`:

  | Prova | Passi | Asserzione |
  |---|---|---|
  | `shows_the_real_daemon_state` | avvio dell'app con il demone attivo | entro 10 s: stato «Disconnected» o «Disconnesso» e barra con il collegamento attivo |
  | `relinks_after_a_daemon_restart` | termina `galleriad`; poi lo riavvia sullo stesso socket | entro 10 s compare il comando `systemctl`; entro 10 s dal riavvio torna lo stato, senza toccare la finestra (G1) |
  | `first_run_without_daemon` | avvio dell'app senza demone | compare il pannello «Avvia il servizio» con il comando |
  | `window_has_no_remote_content` | — | `document.querySelectorAll('script[src^="http"],link[href^="http"],img[src^="http"]').length === 0` |

  Per fermare e riavviare il demone dalle prove, `gui-smoke.sh` espone due piccoli script nella cartella
  temporanea (`daemon-stop`, `daemon-start`) e ne passa il percorso in una variabile d'ambiente: la prova li
  esegue da Node, non dalla webview.

- [ ] **2.** Falliscono (l'immagine non ha ancora driver né Xvfb). **3.** Implementa. **S5**: se `galleriad run` non
  parte nel container senza toccare la rete, annota l'errore e usa al suo posto un eseguibile di prova del crate
  (`FakeDaemon` in un `[[bin]]` solo di prova) — senza modificare `galleriad`.
- [ ] **4.** `scripts/gui.sh smoke` verde **tre volte di fila** in locale; job `gui-smoke` verde sul runner (se in
  questa sessione non si può fare push, lo si scrive nell'esito come «da verificare al primo push» e non lo si
  dichiara verde).
- [ ] **5. Esito e roadmap.** In `docs/08-esito-fase2.md`, sezione «B1»: tabella G1 e G2 con la prova che li
  copre e il suo risultato; tabella S1–S7 con esito e ripiego applicato; numero di prove per livello; cosa **non**
  è stato verificato. In `ROADMAP.md`: caselle di «B1 · Fondamenta» secondo i fatti (una casella resta vuota se
  la sua prova non è verde). Handoff in `.claude/handoffs/`.
- [ ] **6. Commit:** `add webdriver smoke test for the gui in the lab image`

**Fatto quando:** il fumo è verde tre volte di fila, oppure l'esito dice per iscritto perché non gira e contiene
la lista di controllo manuale di G1 e G2 eseguita e annotata; in entrambi i casi S5 e S6 hanno un esito.

---

## Copertura della spec

| Spec | Dove |
|---|---|
| G1 collegamento e ricollegamento | T3 (`daemon_restart_relinks_without_user_action`), T4, T6 (`relinks_after_a_daemon_restart`) |
| G2 quattro stati del primo avvio | T2 (classificazione, sette prove), T5 (schermate), T6 (`first_run_without_daemon`) |
| §3.2 confini e regole 1–5 | T1 (`check-boundaries`, `config.rs`), T4 (`command_outputs_never_contain_secret_fields`), T5 (`nothing_is_executed`) |
| §3.3 due tipi di connessione | T3 (eventi); le connessioni dei comandi arrivano in B5, in B1 nessun comando chiama il demone |
| §3.4 configurazione, permessi, CSP | T1, T4 |
| §4.1 comandi e §4.3 eventi di B1 | T4 |
| §5 tabella degli esiti | T2 |
| §14 tre livelli e CI | T1 (job `gui`), T5 (livello 1), T2–T4 (livello 2), T6 (livello 3, job `gui-smoke`) |
| S1–S7 | T1 (S1–S3), T4 (S4), T5 (S7), T6 (S5, S6) |
| Q13–Q19 | T1 (Q14, Q15, Q18, Q19), T2 (Q16, Q17), T6 (Q13) |

Fuori da B1, per scelta: design system e cataloghi (B2), profili (B3), import e istanza unica (B4), comandi che
chiamano il demone e archivio dei segreti (B5), tray e versione (B6).
