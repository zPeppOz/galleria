# Fase 1 — Demone Linux `galleriad` + CLI minima: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** un demone Linux che accende una connessione VPN alla volta (WireGuard nel kernel, OpenVPN via
management interface) comandato via JSON-RPC su socket Unix, più una CLI minima, provati in un laboratorio Docker e in CI.

**Architettura:** workspace Rust di dieci crate. Un solo task (`Manager`) possiede la sessione e una macchina a stati
pura; i backend stanno dietro un trait comune e parlano con il demone per canali; DNS, rotte e blocco IPv6 stanno in
un modulo di rete comune con giornale per la pulizia dopo un crash. Il demone non legge mai file dell'utente.

**Stack:** Rust 1.96.0 (edizione 2024), tokio, serde/serde_json, toml, clap, thiserror/anyhow, ipnet, uuid, zeroize,
`rustix` (feature `process`, per `PR_SET_DUMPABLE`), `libc` (solo in `galleriad/src/peer_groups.rs`), `defguard_wireguard_rs` 0.12, rstest e proptest per i test; Docker per il laboratorio; GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-10-fase1-demone-design.md` (da leggere per intero prima del T1; i rimandi
"spec §n" puntano lì). Decisioni: `docs/01-analisi.md` (D1–D18: non si riaprono).

## Vincoli globali

- Toolchain `1.96.0` in `rust-toolchain.toml`; `resolver = "3"`; edizione 2024; ogni crate `license = "GPL-3.0-or-later"`, `publish = false`.
- Un task = un commit sul branch di lavoro. Messaggi in inglese, minuscolo, imperativo, senza prefisso (stile dello
  storico: `add …`, `build …`, `record …`). Nessun push, nessuna PR.
- TDD: prima il test, lo si vede fallire, poi il codice. Ogni task finisce con `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` verdi.
- Non si toccano: `spikes/`, `docs/00`–`05`, `design/`, `.github/workflows/spike-windows-build.yml`. Dal codice degli
  spike si copia e si adatta, non lo si importa.
- `unsafe`: `#![forbid(unsafe_code)]` in ogni crate **tranne `galleriad`**. `forbid` non si può allentare con un
  `allow` più interno, quindi `galleriad` usa `#![deny(unsafe_code)]` a livello di crate e un solo
  `#[allow(unsafe_code)]`, sul modulo `galleriad/src/peer_groups.rs`, con un commento che dice perché: legge
  `SO_PEERGROUPS` con `libc::getsockopt`, perché nessun crate lo espone in modo sicuro (verificato il 2026-11-10 su
  `rustix` 1.1.6 e `nix` 0.31.3; `SO_PEERCRED` invece si legge con `UnixStream::peer_cred()` di tokio). Il modulo
  contiene quella sola funzione, ogni blocco `unsafe` ha il suo `// SAFETY:`, e nessun altro `allow(unsafe_code)`
  è ammesso nel workspace. Se alla data del T4 un crate già in `Cargo.lock` espone `SO_PEERGROUPS` in modo sicuro,
  si usa quello e `galleriad` torna a `forbid`.
- Nessun segreto negli argomenti di un processo, mai: né di `openvpn`, né della CLI, né nei comandi dei test
  (un `sh -c "echo segreto | …"` mette il segreto in argv: i test scrivono sullo stdin del processo figlio).
- Niente `unwrap()`/`expect()` fuori dai test. `thiserror` nelle librerie, `anyhow` nei binari.
- Dipendenze nuove solo con licenza ammessa da `deny.toml` (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC,
  Unicode-3.0, Zlib, MPL-2.0, LGPL-2.1-or-later, GPL-3.0-or-later).
- Costanti fissate dalla spec, da non cambiare: `PROTOCOL_VERSION = 1`; `MAX_MESSAGE_BYTES = 1_048_576`;
  `DEFAULT_SOCKET_PATH = "/run/galleria/galleriad.sock"`; interfacce `glr-wg0` e `glr-ovpn0`; fwmark e tabella
  `51820`; 16 client; 5 s per `hello`; tentativi 4 (prima connessione) e 8 (dopo una caduta); attese
  1, 2, 4, 8, 16, 30 s; 120 s per `needs_input`; 2000 righe di log; 15 s per lo smontaggio.
- Il demone parla inglese e restituisce codici stabili (spec §4.6); nessun segreto in log, errori o risposte.
- `galleriad` non dipende da `galleria-profile` e non ha parametri IPC che siano percorsi di file.
- I test end-to-end sono `#[ignore]` nel crate `galleria-e2e` e girano solo dentro il laboratorio
  (`lab/run.sh <26|27|79> [filtro]`). Il laboratorio non tocca rotte, DNS o pacchetti dell'host; a fine lavoro `lab/down.sh`.
- Se un punto **[S]** della spec §12 risulta falso: si applica la colonna "Se è falsa". Se tocca una decisione D
  (per esempio S4), ci si ferma e si riporta; non si corregge l'analisi.
- API di librerie esterne (`defguard_wireguard_rs`, `tokio`, `clap`): verificare firme e campi sulla documentazione
  della versione fissata in `Cargo.lock` prima di scrivere il codice; i nomi di questo piano valgono per il **nostro** codice.

## Punti di attenzione per la revisione

Ingressi che la spec implica e che è facile dimenticare. Ognuno ha il suo test nel task indicato.

1. **Client muto o lento**: si collega e non manda nulla, o mezza riga. Il demone lo chiude dopo 5 s e nel
   frattempo serve gli altri → T4 `silent_client_is_dropped_without_blocking_others`.
2. **Socket già presente all'avvio**: file rimasto da un crash (va sostituito) oppure demone già in esecuzione (la
   seconda istanza deve rifiutarsi, non rubare il socket) → T4 `stale_socket_file_is_replaced`, `second_instance_refuses_to_start`.
3. **Endpoint WireGuard che non si risolve, o solo IPv6** → T8 `unresolvable_endpoint_exits_transport`, `ipv6_only_endpoint_is_accepted`.
4. **`.ovpn` scritto su Windows**: fine riga CRLF, BOM UTF-8, tabulazioni, blocco inline non chiuso → T10
   `parses_crlf_and_bom`, `unterminated_block_is_an_error`.
5. **Nome di profilo ambiguo o con caratteri da percorso** (`../x`, due profili con lo stesso nome) nella CLI → T15
   `find_reports_ambiguous_names`, `name_with_path_characters_never_reaches_the_filesystem`.

## Mappa dei file

```
Cargo.toml  rust-toolchain.toml  deny.toml  scripts/check-boundaries.sh        T1
.github/workflows/ci.yml                                                       T1, T7, T12
crates/galleria-ipc/src/{lib,wire,types,methods,error,codec,secret}.rs         T1, T2    client.rs  T4
crates/galleria-backend/src/{lib,net,fake}.rs                                  T3
crates/galleriad/src/{lib,machine,manager,session_dir}.rs                      T3
crates/galleriad/src/{main,config,auth,peer_groups,server,dispatch}.rs         T4
crates/galleriad/src/{logbuf,redact}.rs                                        T5
crates/galleria-net/src/{lib,plan,runner,journal,manager}.rs                   T6
packaging/linux/{galleriad.service,galleria.sysusers,90-galleria.conf,install.sh}   T7
lab/{images/*,server/*,wg/*,gen-pki.sh,build.sh,up.sh,down.sh,run.sh,README.md}     T7
crates/galleria-e2e/{src/lib.rs,src/bin/ipc-probe.rs,tests/t07_daemon.rs}      T7   tests/t08…t16  T8–T16
crates/galleria-backend-wireguard/src/{lib,validate,device,session}.rs         T8
crates/galleriad/src/factory.rs                                                T8, T12
crates/galleria-ovpn/src/{lib,parse,classify,filter}.rs                        T10
crates/galleria-backend-openvpn/src/{lib,mgmt,queue,config,push,hookenv,version}.rs   T11
crates/galleria-backend-openvpn/src/{session,plat,hook}.rs                     T12
crates/galleria-profile/src/{lib,model,store,secrets,import_wg,import_ovpn}.rs T15
crates/galleria-cli/src/{main,cli,commands,messages,prompt}.rs                 T16
docs/06-esito-fase1.md                                                         T16
```

Ordine: T1–T7 danno un demone vero sotto systemd con un backend finto; T8–T9 WireGuard end-to-end; T10–T14 OpenVPN;
T15–T16 profili e CLI. Stima totale: circa 84 ore di agente (1–2 settimane part-time, come `01-analisi.md` §10).

---

### Task 1: workspace e CI

**Stima:** 2 h. **File:** crea `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `scripts/check-boundaries.sh`,
`.github/workflows/ci.yml`, `crates/galleria-ipc/Cargo.toml`, `crates/galleria-ipc/src/lib.rs`.

**Interfacce — produce:** `galleria_ipc::{PROTOCOL_VERSION, PROTOCOL_MIN, PROTOCOL_MAX, MAX_MESSAGE_BYTES, DEFAULT_SOCKET_PATH}`.
`Cargo.toml` con `members = ["crates/*"]`, `[workspace.package]` (versione `0.1.0`, edizione, licenza),
`[workspace.dependencies]` e `[workspace.lints.clippy]` (`unwrap_used = "deny"`, `expect_used = "deny"`, esclusi i test).

- [ ] **1. Test che fallisce** in `crates/galleria-ipc/src/lib.rs`:

```rust
#[test]
fn protocol_constants_match_the_spec() {
    assert_eq!((PROTOCOL_MIN, PROTOCOL_VERSION, PROTOCOL_MAX), (1, 1, 1));
    assert_eq!(MAX_MESSAGE_BYTES, 1_048_576);
    assert_eq!(DEFAULT_SOCKET_PATH, "/run/galleria/galleriad.sock");
}
```

- [ ] **2.** `cargo test -p galleria-ipc` → fallisce (costanti non definite).
- [ ] **3.** Definisci le costanti. Scrivi `deny.toml` (licenze dei vincoli globali, `[advisories]` attivo, `[bans] multiple-versions = "warn"`).
- [ ] **4.** `scripts/check-boundaries.sh`: esce 1 se `cargo tree -p galleriad -e normal` contiene `galleria-profile`;
  esce 0 con un messaggio se il crate `galleriad` non esiste ancora.
- [ ] **5.** `ci.yml`: trigger `push` e `pull_request`, `permissions: contents: read`. Ogni job **installa gli
  strumenti che usa**: il runner `ubuntu-latest` ha `rustup` e `cargo`, ma non `cargo-deny` né `cargo-llvm-cov`.
  Job `check` su `ubuntu-latest`: toolchain da `rust-toolchain.toml` (componenti `rustfmt`, `clippy`,
  `llvm-tools-preview`), cache cargo, un passo `taiki-e/install-action` con `tool: cargo-deny`, poi nell'ordine
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cargo deny check`, `scripts/check-boundaries.sh`. Job `coverage` (`continue-on-error: true`): stessa toolchain,
  `taiki-e/install-action` con `tool: cargo-llvm-cov`, poi `cargo llvm-cov --workspace --summary-only`.
  Le azioni di terze parti si fissano a un commit SHA (con la versione in un commento), non a un tag mobile.
- [ ] **6. Verifica:** i cinque comandi del job `check` in locale, tutti con uscita 0 (`cargo install cargo-deny --locked`
  se manca), e `cargo llvm-cov --workspace --summary-only` (`cargo install cargo-llvm-cov --locked` se manca). Poi
  rileggi `ci.yml` un comando alla volta: per ogni `cargo <sottocomando>` che non fa parte di cargo deve esserci,
  **prima** nello stesso job, il passo che lo installa.
- [ ] **7. Commit:** `add rust workspace and ci`

**Fatto quando:** i comandi dei job `check` e `coverage` passano in locale, nessun job usa uno strumento che non ha
installato, e `ci.yml` supera `actionlint` (o, se non è installato, una lettura contro la sintassi dei workflow).
Il job gira su GitHub al primo push autorizzato e deve poter essere verde già con questo solo commit.

---

### Task 2: tipi e codec dell'IPC

**Stima:** 4 h. **File:** crea `crates/galleria-ipc/src/{wire,types,methods,error,codec,secret}.rs`,
`crates/galleria-ipc/tests/golden.rs`; modifica `lib.rs`.

**Interfacce — produce** (tutti i tipi di richiesta con `#[serde(deny_unknown_fields)]`; quelli di risposta senza):

```rust
// secret.rs — Debug stampa "Secret(..)", zeroize al drop, Serialize/Deserialize come stringa
pub struct Secret(/* privato */);
impl Secret { pub fn new(s: impl Into<String>) -> Self; pub fn expose(&self) -> &str; }
// wire.rs
pub enum RequestId { Num(i64), Str(String) }
pub struct Request { pub id: RequestId, pub method: String, pub params: serde_json::Value }
pub struct RpcError { pub code: i32, pub message: String, pub data: Option<serde_json::Value> }
// error.rs — codici della spec §4.6
pub enum ErrorCode { HelloRequired, IncompatibleProtocol, MessageTooLarge, TooManyClients, AlreadyActive, NotActive,
                     InvalidConfig, ProfileRejected, BackendUnavailable, InputNotExpected, Internal }
impl ErrorCode { pub fn as_str(self) -> &'static str; pub fn to_rpc(self, message: &str) -> RpcError; }
pub enum SessionErrorCode { AuthFailed, InputTimeout, ServerUnreachable, TunnelSetupFailed, NetSetupFailed,
                            DnsUnavailable, UnsupportedChallenge, BackendExited }
pub struct SessionError { pub code: SessionErrorCode, pub message: String }
// types.rs — forma JSON identica alla spec §4.2, §4.4, §4.5
pub enum Protocol { Wireguard, Openvpn }
pub enum State { Disconnected, Connecting, NeedsInput, Connected, Reconnecting, Disconnecting, Error }
pub struct Status { /* campi della spec §4.4 */ }
pub struct NetInfo; pub enum Ipv6BlockInfo { None, Unreachable }
pub struct Stats { pub rx_bytes: u64, pub tx_bytes: u64, pub last_handshake: Option<String> }
pub struct InputRequest { pub request: u64, pub kind: InputKind, pub prompt: String, pub echo: bool, pub expires_in_secs: u64 }
pub enum InputKind { Credentials, Otp, KeyPassphrase }
pub struct InputValues { pub username: Option<String>, pub password: Option<Secret>, pub otp: Option<Secret>, pub key_passphrase: Option<Secret> }
pub struct LogEntry { pub seq: u64, pub ts: String, pub level: LogLevel, pub source: LogSource, pub message: String }
pub enum BackendParams { Wireguard(WireGuardParams), Openvpn(OpenVpnParams) }   // tag "protocol"
pub struct WireGuardParams; pub struct WireGuardPeer; pub struct OpenVpnParams;
pub struct ConnectOptions { pub ipv6_block: bool, pub ignore_pushed_routes: bool, pub reconnect: ReconnectOptions }
pub struct ProfileRef { pub id: String, pub name: String }
pub struct HelloParams; pub struct ConnectParams; pub struct ProvideInputParams; pub struct SubscribeParams; pub struct LogsParams;
pub struct HelloResult; pub struct BackendInfo; pub struct Capabilities;
pub struct ConnectResult { pub session: u64, pub status: Status }
pub struct LogsResult { pub entries: Vec<LogEntry>, pub last: u64 }
pub enum LogLevel { Debug, Info, Warn, Error }   pub enum LogSource { Daemon, Openvpn, Wireguard, Net }
pub enum Event { State(Status), Stats(Stats), Log(LogEntry) }                   // event.state / event.stats / event.log
// methods.rs
pub enum Call { Hello(HelloParams), Status, Connect(ConnectParams), Disconnect, ProvideInput(ProvideInputParams),
                Subscribe(SubscribeParams), Logs(LogsParams) }
pub fn parse_call(method: &str, params: serde_json::Value) -> Result<Call, RpcError>;
pub fn negotiate(client_protocol: u32) -> Result<u32, RpcError>;
// codec.rs
pub fn decode_line(line: &[u8]) -> Result<Request, RpcError>;
pub fn encode_line<T: serde::Serialize>(msg: &T) -> Vec<u8>;                    // con '\n' finale
pub fn response_ok(id: &RequestId, result: impl serde::Serialize) -> Vec<u8>;
pub fn response_err(id: Option<&RequestId>, err: &RpcError) -> Vec<u8>;
pub fn notification(event: &Event) -> Vec<u8>;
pub enum FrameError { TooLarge, Io(std::io::Error) }
pub async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R, max: usize) -> Result<Option<Vec<u8>>, FrameError>;
```

- [ ] **1. Test che falliscono** (i JSON d'oro sono quelli della spec §4.2, §4.4, §4.5, copiati in `tests/golden.rs`):

| Test | Asserzione |
|---|---|
| `hello_request_matches_golden_json` | il JSON della spec §4.2 → `Call::Hello` con `protocol == 1`, `client.name == "galleria-cli"` |
| `status_golden_json_deserializes` | il JSON della spec §4.4 → `state == Connected`, `net.ipv6_block == Unreachable`, `stats.rx_bytes == 48_213_000` |
| `backend_params_are_tagged_by_protocol` | i due JSON della spec §4.5 → `Wireguard(..)` e `Openvpn(..)` |
| `connect_rejects_unknown_field` | `parse_call("connect", …+{"evil":1})` → `Err`, `code == -32602` |
| `unknown_method_is_method_not_found` | `parse_call("reboot", {})` → `code == -32601` |
| `decode_rejects_batch_array` | `decode_line(b"[]")` → `code == -32600` |
| `decode_rejects_request_without_id` | → `code == -32600` |
| `decode_rejects_invalid_json` | `decode_line(b"{")` → `code == -32700` |
| `read_frame_accepts_line_at_limit` | riga di 1 048 576 byte compreso `\n` → `Ok(Some(_))` |
| `read_frame_rejects_line_over_limit` | un byte in più → `Err(FrameError::TooLarge)` senza allocare oltre il limite |
| `negotiate_accepts_1_and_rejects_others` | `negotiate(1) == Ok(1)`; `negotiate(2)` → `data.code == "incompatible_protocol"`, `data.daemon_min == 1`, `data.daemon_max == 1` |
| `secret_debug_is_redacted` | `format!("{:?}", Secret::new("hunter2"))` non contiene `hunter2` |
| `client_side_types_ignore_unknown_fields` | `HelloResult` con un campo in più si legge |
| `error_codes_are_stable_strings` | `ErrorCode::AlreadyActive.as_str() == "already_active"` e così per tutti |

- [ ] **2.** `cargo test -p galleria-ipc` → fallisce. **3.** Implementa. **4.** `cargo test -p galleria-ipc` → verde.
- [ ] **5. Commit:** `add ipc wire types and line codec`

**Fatto quando:** i 14 test passano e ogni esempio JSON della spec §4 è coperto da un test d'oro.

---

### Task 3: interfaccia dei backend, macchina a stati, sessione unica

**Stima:** 7 h. **File:** crea `crates/galleria-backend/{Cargo.toml,src/lib.rs,src/net.rs,src/fake.rs}`,
`crates/galleriad/{Cargo.toml,src/lib.rs,src/machine.rs,src/manager.rs,src/session_dir.rs}`.

**Interfacce — usa:** tipi del T2. **Produce:**

```rust
// galleria-backend/lib.rs — trait e tipi della spec §6.1, più:
pub const WG_INTERFACE: &str = "glr-wg0";
pub const OVPN_INTERFACE: &str = "glr-ovpn0";
pub const WG_FWMARK: u32 = 51820;
pub struct TunnelInfo { pub local_addresses: Vec<std::net::IpAddr>, pub remote: Option<std::net::SocketAddr> }
pub struct LogLine { pub level: LogLevel, pub message: String }
// galleria-backend/net.rs — NetConfig, Ipv6Block, is_full_tunnel_v4, carries_ipv6_default (spec §6.1)
// galleria-backend/fake.rs (feature "fake") — backend pilotato da uno script
pub enum FakeStep { Emit(FakeEvent), WaitCommand, Sleep(std::time::Duration) }
pub struct FakeBackend; impl FakeBackend { pub fn new(script: Vec<FakeStep>, live: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self; }
// galleriad/machine.rs
pub enum Phase { Disconnected, Connecting { attempt: u32 }, NeedsInput { attempt: u32, reconnecting: bool }, Connected,
                 Reconnecting { attempt: u32 }, Disconnecting { then_connect: bool }, Error(SessionError) }
pub enum ExitKind { Stopped, Auth, Transport, Fatal(SessionErrorCode) }
pub enum Input { Connect { replace: bool }, Disconnect, InputRequired, InputProvided, InputTimeout, Up,
                 Restarting, NetFailed(SessionErrorCode), Exited(ExitKind), RetryTimer }
pub enum Effect { SpawnBackend, StopBackend, TeardownNet, ForwardInput, StartRetryTimer(std::time::Duration),
                  StartInputTimer(std::time::Duration), Reject(ErrorCode), Publish }
pub struct RetryPolicy { pub initial_attempts: u32, pub reconnect_attempts: u32, pub base: std::time::Duration,
                         pub cap: std::time::Duration, pub reconnect_enabled: bool }
impl Default for RetryPolicy { /* 4, 8, 1 s, 30 s, true */ }
pub fn backoff(attempt: u32, policy: &RetryPolicy) -> std::time::Duration;
pub fn step(phase: &Phase, input: Input, policy: &RetryPolicy) -> (Phase, Vec<Effect>);
// galleriad/manager.rs
pub type BackendFactory = std::sync::Arc<dyn Fn(&BackendParams, &ConnectOptions) -> Result<Box<dyn Backend>, RpcError> + Send + Sync>;
pub trait NetControl: Send + Sync + 'static {
    fn recover(&self) -> Result<(), SessionError>;
    fn apply(&self, cfg: &NetConfig) -> Result<(), SessionError>;
    fn teardown(&self) -> Result<(), SessionError>;
}
#[derive(Clone)] pub struct ManagerHandle;
impl ManagerHandle {
    pub async fn connect(&self, params: ConnectParams, uid: u32) -> Result<ConnectResult, RpcError>;
    pub async fn disconnect(&self, uid: u32) -> Status;
    pub async fn status(&self) -> Status;
    pub async fn provide_input(&self, params: ProvideInputParams) -> Result<(), RpcError>;
    pub fn events(&self) -> tokio::sync::broadcast::Receiver<Event>;
}
pub fn spawn_manager(factory: BackendFactory, net: std::sync::Arc<dyn NetControl>, policy: RetryPolicy,
                     runtime_dir: std::path::PathBuf) -> ManagerHandle;
// galleriad/session_dir.rs — spec §7.4
/// Crea `<runtime_dir>/s/<uuid v4>/` con modo 0700. Il nome è casuale e mai riusato: se esiste già è un errore.
pub fn create_session_dir(runtime_dir: &std::path::Path) -> std::io::Result<std::path::PathBuf>;
/// Svuota `<runtime_dir>/s/` senza seguire i collegamenti simbolici e la ricrea 0700. Da chiamare all'avvio.
pub fn purge_session_dirs(runtime_dir: &std::path::Path) -> std::io::Result<()>;
```

Il `Manager` chiama `create_session_dir` prima di ogni `spawn` (una cartella per tentativo, passata al backend in
`SessionContext.session_dir`) e la rimuove quando il backend è uscito. Eventi del backend (spec §6.1): `NetUp` →
se c'è già una rete applicata `teardown`, poi `apply`, poi risposta immediata; `NetDown` → `teardown` e risposta
immediata, in qualunque fase (anche in `disconnecting`, mentre si aspetta l'uscita del backend); `Restarting` →
`Input::Restarting`; `Up` → `Input::Up` solo se l'ultimo `NetUp` è stato accettato.

- [ ] **1. Test che falliscono.** `machine.rs`: un caso `rstest` **per ogni riga** della tabella della spec §5.2
  (`step_follows_the_spec_table`), più:

```rust
#[test]
fn backoff_sequence_is_1_2_4_8_16_30_30() {
    let p = RetryPolicy::default();
    let secs: Vec<u64> = (1..=7).map(|n| backoff(n, &p).as_secs()).collect();
    assert_eq!(secs, [1, 2, 4, 8, 16, 30, 30]);
}
#[test]
fn auth_failure_never_retries_from_any_phase() { /* per ogni Phase attiva: Exited(Auth) → Error(auth_failed), nessun StartRetryTimer */ }
#[test]
fn drop_with_reconnect_disabled_goes_to_backend_exited() { /* Connected + Exited(Transport), reconnect_enabled=false */ }
#[test]
fn internal_restart_goes_to_reconnecting_without_respawn() {
    /* Connected + Restarting → Reconnecting { attempt: 1 }; effetti: TeardownNet e Publish, nessuno SpawnBackend,
       StopBackend o StartRetryTimer. Poi Reconnecting + Up → Connected. In ogni altra fase Restarting non cambia fase. */
}
```

  `net.rs`: `full_tunnel_detects_default_and_two_halves`, `ipv6_default_detects_every_covering_form` (`::/0`;
  `::/1` + `8000::/1`; `2000::/3`; `2000::/4` + `3000::/4`), `ipv6_default_is_false_for_partial_cover` (solo
  `2000::/4`; solo `fd00::/8`; solo `2001:db8::/32`), `split_routes_are_not_full_tunnel`. `session_dir.rs`:
  `session_dir_names_are_unique_and_0700` (1000 creazioni, nessun nome ripetuto, modo `0700`),
  `existing_name_is_never_reused`, `purge_removes_orphans_and_does_not_follow_symlinks` (un collegamento verso
  una cartella esterna: sparisce il collegamento, la cartella esterna resta intatta). `manager.rs` (tokio con tempo in pausa, `FakeBackend`, `NetControl` che registra):

| Test | Asserzione |
|---|---|
| `never_two_live_backends` (proptest) | per sequenze casuali di connect / connect-replace / disconnect / uscite del fake, il massimo del contatore `live` è ≤ 1 |
| `replace_waits_for_teardown_before_spawning` | nel registro: `stop(1)`, `teardown`, poi `spawn(2)` |
| `net_teardown_precedes_backend_stop` | ordine registrato alla `disconnect` |
| `connect_while_active_is_already_active` | `data.code == "already_active"`, `data.profile.name` valorizzato |
| `disconnect_is_idempotent` | due `disconnect` di fila → `state == Disconnected`, nessun panico |
| `netup_refusal_sets_net_setup_failed` | `NetControl::apply` → `Err` ⇒ `Status.error.code == NetSetupFailed`, backend fermato |
| `connected_only_after_net_applied` | nessun `event.state` con `connected` prima della risposta a `NetUp` |
| `needs_input_times_out_after_120s` | avanzando il tempo di 120 s ⇒ `error.code == InputTimeout` |
| `transport_exits_retry_four_times_then_give_up` | 4 `spawn`, attese 1, 2, 4 s, poi `ServerUnreachable` |
| `secrets_are_dropped_at_session_end` | dopo `disconnect` il manager non tiene più i `ConnectParams` |
| `netup_is_answered_without_waiting_for_up` | il fake manda `NetUp` e poi aspetta la risposta **prima** di mandare `Up`: la risposta arriva (nessuno stallo) e lo stato è ancora `connecting` |
| `up_before_accepted_netup_does_not_connect` | `Up` senza `NetUp` accettato → mai `connected` |
| `internal_restart_reapplies_network` | script: `NetUp(A)`, `Up`, `Restarting`, `NetDown`, `NetUp(B)`, `Up` → stati `connected`, `reconnecting`, `connected`; registro: `apply(A)`, `teardown`, `apply(B)`; un solo `spawn`; dopo, `Status.net` è quello di B |
| `second_netup_without_netdown_replaces_the_first` | `NetUp(A)`, `Up`, `Restarting`, `NetUp(B)` → mai due configurazioni applicate insieme: `teardown` prima di `apply(B)` |
| `netdown_is_answered_during_disconnect` | `disconnect`, e il fake manda `NetDown` prima di uscire → risposta ricevuta, `disconnected` entro il limite |
| `each_spawn_gets_a_fresh_session_dir` | due tentativi → due `session_dir` diverse; ognuna rimossa dopo l'uscita del suo backend |

- [ ] **2.** `cargo test -p galleria-backend -p galleriad` → fallisce. **3.** Implementa: `step` è una `match` esaustiva
  su `(phase, input)`, senza ramo `_` per le fasi; il `Manager` è un ciclo `select!` su richieste, eventi del backend e timer.
- [ ] **4.** Test verdi. **5. Commit:** `add backend interface, state machine and single-session manager`

**Fatto quando:** tutte le righe della tabella §5.2 hanno un caso (riavvio interno compreso) e la proprietà "mai due backend" passa con 256 sequenze.

---

### Task 4: server IPC, autorizzazione, client

**Stima:** 6 h. **File:** crea `crates/galleriad/src/{main,config,auth,peer_groups,server,dispatch}.rs`,
`crates/galleriad/tests/ipc_server.rs`, `crates/galleria-ipc/src/client.rs` (feature `client`).

**Interfacce — usa:** `ManagerHandle`, codec. **Produce:**

```rust
// auth.rs
pub struct PeerCreds { pub uid: u32, pub gid: u32, pub groups: Vec<u32>, pub pid: Option<i32> }
pub struct AccessPolicy { pub allowed_gid: Option<u32> }
pub fn authorize(creds: &PeerCreds, policy: &AccessPolicy) -> bool;
pub fn peer_creds(stream: &tokio::net::UnixStream) -> std::io::Result<PeerCreds>;      // peer_cred() di tokio + peer_groups
// peer_groups.rs — l'UNICO modulo con `#[allow(unsafe_code)]` (vincoli globali): una sola funzione
pub fn peer_groups(fd: std::os::fd::BorrowedFd<'_>) -> std::io::Result<Vec<u32>>;       // SO_PEERGROUPS; riprova con un buffer più grande su ERANGE
// config.rs
pub struct DaemonConfig { pub socket: PathBuf, pub group: String, pub runtime_dir: PathBuf,
                          pub hello_timeout: Duration, pub max_clients: usize }
// server.rs
pub struct DaemonInfo { pub version: String, pub backends: Vec<BackendInfo> }
pub async fn serve(cfg: DaemonConfig, manager: ManagerHandle, info: DaemonInfo,
                   shutdown: tokio_util::sync::CancellationToken) -> anyhow::Result<()>;
// galleria-ipc client.rs
pub enum ClientError { Io(std::io::Error), Rpc(RpcError), Closed, PermissionDenied }
impl Client {
    pub async fn connect(path: &std::path::Path) -> Result<Client, ClientError>;
    pub async fn hello(&mut self, name: &str, version: &str) -> Result<HelloResult, ClientError>;
    pub async fn call<P: Serialize, R: DeserializeOwned>(&mut self, method: &str, params: &P) -> Result<R, ClientError>;
    pub async fn next_event(&mut self) -> Option<Event>;
}
```

`main.rs`: `galleriad run [--socket P] [--group G] [--runtime-dir D]`; per ora una fabbrica che risponde
`backend_unavailable` e un `NetControl` vuoto (i veri arrivano in T6 e T8). `SIGTERM`/`SIGINT` → `disconnect`, uscita 0.
All'avvio, prima di `serve`: `galleriad::harden_process()`, cioè
`rustix::process::set_dumpable_behavior(DumpableBehavior::NotDumpable)` (spec §9; se fallisce, avviso nel log e si prosegue) e `purge_session_dirs(runtime_dir)` (spec §7.4; se fallisce il demone esce
con errore). In `lib.rs`: `#![deny(unsafe_code)]` e `#[allow(unsafe_code)] mod peer_groups;` con il commento del perché.

- [ ] **1. Test che falliscono.** `auth.rs` (puri):

```rust
#[rstest]
#[case::root(0, 0, vec![], Some(1500), true)]
#[case::primary_group(1000, 1500, vec![], Some(1500), true)]
#[case::supplementary_group(1000, 1000, vec![27, 1500], Some(1500), true)]
#[case::outsider(1001, 1001, vec![27], Some(1500), false)]
#[case::no_group_configured(1000, 1500, vec![1500], None, false)]
fn authorize_follows_group_membership(#[case] uid: u32, #[case] gid: u32, #[case] groups: Vec<u32>,
                                      #[case] allowed: Option<u32>, #[case] expected: bool) {
    let creds = PeerCreds { uid, gid, groups, pid: None };
    assert_eq!(authorize(&creds, &AccessPolicy { allowed_gid: allowed }), expected);
}
```

  `tests/ipc_server.rs` (socket in `tempdir`, gruppo = gruppo primario di chi esegue il test, `FakeBackend`):

| Test | Asserzione |
|---|---|
| `hello_then_status_is_disconnected` | `hello.protocol == 1`, `status.state == Disconnected` |
| `method_before_hello_is_hello_required` | `data.code == "hello_required"` |
| `incompatible_protocol_closes_the_connection` | errore, poi la lettura successiva dà EOF |
| `oversized_line_is_message_too_large_then_close` | 1 048 577 byte → errore `message_too_large`, EOF |
| `silent_client_is_dropped_without_blocking_others` | con `hello_timeout = 200 ms`: il client muto riceve EOF; un secondo client intanto completa `hello` |
| `half_line_then_silence_is_dropped` | stessa attesa, nessuna risposta parziale |
| `seventeenth_client_is_too_many_clients` | con `max_clients = 16` |
| `connect_twice_is_already_active_then_replace_succeeds` | secondo `connect` → `already_active`; con `replace: true` → nuova `session` |
| `state_events_reach_every_client` | due client: entrambi ricevono `event.state` `connecting` → `connected` |
| `client_dropping_mid_session_leaves_it_running` | chiuso il client che ha fatto `connect`, un altro vede ancora `connected` |
| `provide_input_with_wrong_request_is_input_not_expected` | |
| `socket_mode_is_0660` | `metadata.mode() & 0o777 == 0o660` |
| `stale_socket_file_is_replaced` | file socket senza processo in ascolto → `serve` parte |
| `second_instance_refuses_to_start` | con un `serve` vivo sullo stesso percorso → `Err`, il primo continua a rispondere |
| `sigterm_disconnects_before_exit` | token di arresto ⇒ `teardown` registrato, socket rimosso |
| `peer_groups_match_the_process_groups` (in `peer_groups.rs`) | su una `socketpair`, `peer_groups` restituisce gli stessi gid della riga `Groups:` di `/proc/self/status` |
| `orphan_session_dirs_are_purged_before_listening` | con `<runtime_dir>/s/vecchia/mgmt.sock` già presente: quando il socket del demone esiste, `s/` è vuota |
| `harden_process_makes_it_not_dumpable` (in `tests/hardening.rs`, un eseguibile di test a parte: il flag vale per tutto il processo) | dopo `galleriad::harden_process()`, la funzione che `main` chiama all'avvio, `rustix::process::dumpable_behavior()` è `NotDumpable` |

- [ ] **2.** Fallisce. **3.** Implementa: un task per connessione; `peer_creds` + `authorize` subito dopo `accept`
  (chi non passa: chiusura senza scrivere nulla); `hello` obbligatorio per primo; notifiche da `ManagerHandle::events()`.
- [ ] **4.** `cargo test -p galleriad -p galleria-ipc --all-features` → verde.
- [ ] **5. Commit:** `add ipc server with peer authorization and async client`

**Fatto quando:** i test passano senza privilegi, `scripts/check-boundaries.sh` esce 0 e
`grep -rn 'allow(unsafe_code)' crates/` trova una sola riga, quella di `peer_groups`.

---

### Task 5: log, filtro dei segreti, sottoscrizioni

**Stima:** 3 h. **File:** crea `crates/galleriad/src/{logbuf,redact}.rs`; modifica `manager.rs`, `dispatch.rs`,
`tests/ipc_server.rs`.

**Interfacce — produce:**

```rust
pub const REDACTED: &str = "[redatto]";
pub struct Redactor; impl Redactor {
    pub fn new() -> Self;
    pub fn add_secret(&mut self, value: &str);          // nessuna lunghezza minima; ignora solo la stringa vuota; multilinea: una voce per riga non vuota
    pub fn clear(&mut self);
    pub fn redact<'a>(&self, line: &'a str) -> std::borrow::Cow<'a, str>;
}
pub struct LogBuffer; impl LogBuffer {
    pub fn with_capacity(lines: usize) -> Self;         // 2000 in produzione
    pub fn push(&mut self, level: LogLevel, source: LogSource, message: String) -> LogEntry;
    pub fn after(&self, seq: Option<u64>, limit: usize) -> (Vec<LogEntry>, u64);
    pub fn clear(&mut self);
}
```

- [ ] **1. Test che falliscono:**

| Test | Asserzione |
|---|---|
| `redacts_registered_secret_anywhere` | `"pw=alice-secret ok"` → `"pw=[redatto] ok"` |
| `redacts_every_line_of_a_multiline_secret` | ogni riga di un PEM registrato sparisce |
| `redacts_secrets_of_any_length` (rstest: 1, 2, 3, 4, 64 caratteri) | registrato `"7"`: `"otp=7 ok"` → `"otp=[redatto] ok"`; registrato `"ab"`: `"pw=ab"` → `"pw=[redatto]"` (e `"cabina"` → `"c[redatto]ina"`: effetto collaterale accettato, spec §5.4) |
| `empty_secret_is_ignored` | registrare `""` non altera nessuna riga e non blocca `redact` |
| `longer_secrets_are_replaced_first` | registrati `"ab"` e `"abcd"`: `"x=abcd"` → `"x=[redatto]"`, senza resti del valore lungo |
| `redacts_pem_bodies_structurally` | tra `-----BEGIN … PRIVATE KEY-----` e `-----END` anche se non registrato |
| `redacts_management_credentials` | `password "Auth" "x1y2z3"` → `password "Auth" [redatto]`; idem `username` |
| `buffer_drops_oldest_beyond_capacity` | capacità 3, 5 inserimenti → `seq` 3, 4, 5 |
| `after_returns_only_newer_entries` | `after(Some(3), 10)` → 4, 5 e `last == 5` |
| `logs_limit_is_capped_at_1000` | richiesta 5000 → al più 1000 |
| `stats_and_logs_only_reach_subscribers` | client senza `subscribe` riceve solo `event.state` |
| `secrets_from_connect_never_reach_any_client` | il `FakeBackend` scrive nel log la propria password; nessun frame ricevuto da nessun client la contiene |
| `buffer_is_cleared_on_new_session` | dopo una nuova `connect`, `logs` non ha righe della sessione precedente |
| `audit_line_records_uid_and_method` | dopo `connect` da uid N il buffer contiene `uid N` e `connect` |

- [ ] **2.** Fallisce. **3.** Implementa: all'accettazione di `connect` il manager registra nel `Redactor` ogni
  `Secret` dei parametri; ogni `BackendEvent::Log` e ogni riga del demone passa da `redact` prima di buffer, journald ed eventi.
- [ ] **4.** Verde. **5. Commit:** `add log buffer, secret redaction and event subscriptions`

**Fatto quando:** i 14 test passano; M7 ha una prova unitaria, password corte comprese.

---

### Task 6: modulo di rete comune

**Stima:** 5 h. **File:** crea `crates/galleria-net/{Cargo.toml,src/lib.rs,src/plan.rs,src/runner.rs,src/journal.rs,src/manager.rs}`;
modifica `crates/galleriad/src/main.rs` (usa il modulo vero, `recover()` prima di `serve`).

**Interfacce — usa:** `NetConfig`, `Ipv6Block`, costanti del T3. **Produce:**

```rust
pub enum Program { Ip, Resolvectl, Sysctl }
pub struct Cmd { pub program: Program, pub args: Vec<String> }
pub struct Step { pub apply: Cmd, pub undo: Option<Cmd> }
pub fn plan(cfg: &NetConfig) -> Vec<Step>;                 // puro
pub fn fixed_cleanup() -> Vec<Cmd>;                        // puro: link e regole con nomi fissi
pub trait CommandRunner: Send + Sync { fn run(&self, cmd: &Cmd) -> Result<(), NetError>; }
pub struct SystemRunner; impl SystemRunner { pub fn locate() -> Result<Self, NetError>; }   // cerca in /usr/sbin, /usr/bin, /sbin, /bin
pub enum NetError { DnsUnavailable(String), Command { cmd: String, stderr: String }, Journal(std::io::Error), ToolMissing(&'static str) }
pub struct NetManager<R: CommandRunner>; impl<R: CommandRunner> NetManager<R> {
    pub fn new(runner: R, journal: std::path::PathBuf) -> Self;
    pub fn apply(&self, cfg: &NetConfig) -> Result<(), NetError>;
    pub fn teardown(&self) -> Result<(), NetError>;
    pub fn recover(&self) -> Result<(), NetError>;
}
```

In `galleriad`: `impl NetControl for NetManager<SystemRunner>` (`DnsUnavailable` → `SessionErrorCode::DnsUnavailable`,
il resto → `NetSetupFailed`), chiamato dentro `spawn_blocking`.

- [ ] **1. Test che falliscono.** Il piano è deterministico; l'ordine è rotte → blocco IPv6 → DNS:

```rust
#[test]
fn plan_for_wireguard_full_tunnel_with_dns() {
    let cfg = NetConfig { interface: "glr-wg0".into(), dns_servers: vec!["10.99.0.1".parse().unwrap()],
        search_domains: vec!["corp.test".into()], routes: vec!["0.0.0.0/0".parse().unwrap()],
        install_routes: Some(51820), ipv6_block: Ipv6Block::Unreachable };
    let applied: Vec<String> = plan(&cfg).iter().map(|s| s.apply.to_string()).collect();
    assert_eq!(applied, [
        "ip -4 route add 0.0.0.0/0 dev glr-wg0 table 51820",
        "ip -4 rule add not fwmark 51820 table 51820",
        "ip -4 rule add table main suppress_prefixlen 0",
        "sysctl -q -w net.ipv4.conf.all.src_valid_mark=1",
        "ip -6 route replace unreachable ::/1",
        "ip -6 route replace unreachable 8000::/1",
        "resolvectl dns glr-wg0 10.99.0.1",
        "resolvectl domain glr-wg0 corp.test ~.",
        "resolvectl default-route glr-wg0 yes",
    ]);
}
```

| Test | Asserzione |
|---|---|
| `undo_commands_mirror_the_plan` | per il caso sopra: `ip -4 rule del not fwmark 51820 table 51820`, `ip -4 rule del table main suppress_prefixlen 0`, `ip -6 route del unreachable ::/1`, `… 8000::/1`, `resolvectl revert glr-wg0`; nessun undo per `sysctl` e per la rotta nella tabella (muore con il link) |
| `split_tunnel_adds_specific_routes_only` | `routes = [192.168.1.0/24]` → `ip -4 route add 192.168.1.0/24 dev glr-wg0`; nessuna regola, nessun `~.` |
| `ipv6_routes_use_dash_six` | `fd00::/8` → `ip -6 route add fd00::/8 dev glr-wg0` |
| `backend_installed_routes_touch_only_dns` | `install_routes: None` → solo comandi `resolvectl` (+ `unreachable` se richiesto) |
| `no_dns_means_no_resolvectl` | |
| `no_block_adds_no_ipv6_routes` | `Ipv6Block::None` → nessun `ip -6 route … unreachable` |
| `openvpn_block_is_unreachable_routes_without_other_routes` | `install_routes: None` + `Ipv6Block::Unreachable` → le due `ip -6 route replace unreachable` e i loro undo, nessun'altra rotta |
| `apply_after_teardown_works_again` | `apply(A)`, `teardown`, `apply(B)` (riavvio interno, spec §6.3): giornale di B, nessun resto di A |
| `apply_writes_journal_before_first_command` | il runner finto, alla prima chiamata, trova il file del giornale già scritto |
| `apply_failure_rolls_back_what_ran` | terzo comando fallisce → eseguiti gli undo dei primi due, giornale rimosso, `Err` |
| `resolvectl_failure_is_dns_unavailable` | |
| `teardown_runs_undos_in_reverse_and_removes_journal` | |
| `teardown_twice_is_a_noop` | |
| `teardown_tolerates_missing_objects` | undo che falliscono non interrompono gli altri |
| `recover_replays_journal_then_fixed_cleanup` | include `ip link del glr-wg0`, `ip link del glr-ovpn0`, le quattro `rule del` (v4 e v6) |
| `recover_without_journal_still_runs_fixed_cleanup` | |
| `daemon_recovers_before_listening` (in `galleriad`) | `recover` registrato prima che il socket esista |

- [ ] **2.** Fallisce. **3.** Implementa (giornale: JSON con `undo: Vec<Cmd>`, scrittura con file temporaneo + `rename`).
- [ ] **4.** `cargo test -p galleria-net -p galleriad` → verde. **5. Commit:** `add common network module with journal and crash recovery`

**Fatto quando:** i test passano e nessuna funzione di `galleria-net` accetta stringhe libere come argomenti di comando.

---

### Task 7: laboratorio Docker, unit systemd, job `lab`

**Stima:** 6 h. **File:** crea `packaging/linux/*`, `lab/*` (spec §9 e §10.2), `crates/galleria-e2e/{Cargo.toml,src/lib.rs,
src/bin/ipc-probe.rs,tests/t07_daemon.rs}`; modifica `.github/workflows/ci.yml`, `.gitignore` (`lab/pki/`, `lab/out/`).

**Interfacce — produce:**

- `lab/run.sh <26|27|79> [filtro]`: `build.sh` (compila workspace e test e2e; in un container `ubuntu:24.04` con
  rustup salvo `LAB_BUILD=host`), `gen-pki.sh`, `up.sh` (rete `galleria-fase1`, `glab-target`, i quattro server
  OpenVPN, `glab-wg`, il client scelto), `packaging/linux/install.sh` dentro il client, poi ogni eseguibile di test
  con `--ignored --test-threads=1 [filtro]`. Uscita = esito dei test. `down.sh` sempre alla fine salvo `KEEP=1`.
- Server già pronti per i task successivi: tabella della spec §10.2. `glab-wg` scrive in `lab/out/` `wg-split.conf`
  (`AllowedIPs = 10.99.0.0/24, 198.51.100.1/32`, `DNS = 10.99.0.1`) e `wg-full.conf` (`AllowedIPs = 0.0.0.0/0`).
  I server OpenVPN usano `keepalive 2 8` (serve al T13). Profili `.ovpn` in `lab/out/`: `base`, `otp`, `enckey`,
  `unreachable`, `dns-option`, `split`, `v6`, `malicious`. Tranne `malicious`, i profili sono già "puliti" (senza
  `dev` e `verb`, che per il demone sono direttive sue): così i test li mandano al demone prima che esista l'import del T15.
- Client: utenti `alice` (nel gruppo `galleria`) e `mallory` (fuori); unit `systemd-binfmt.service` e
  `proc-sys-fs-binfmt_misc.automount` mascherate; `tcpdump`, `curl`, `python3`, `oathtool`, `iproute2`.
- `galleria_e2e`: `pub fn sh(cmd: &str) -> Output`, `pub async fn client() -> Client` (già dopo `hello`),
  `pub async fn wait_state(c: &mut Client, s: State, timeout: Duration) -> Status`, `pub fn residue() -> String`
  (`ip -br link; ip route; ip -6 route; ip rule; ip -6 rule; resolvectl dns`), `pub fn lab_file(name: &str) -> String`.
- `ipc-probe hello`: uscita 0 se `hello` riesce, 13 se permesso negato, 1 altrimenti.

- [ ] **1. Test che falliscono** (`tests/t07_daemon.rs`, tutti `#[ignore]`):

| Test | Asserzione |
|---|---|
| `daemon_is_active_under_systemd` | `systemctl is-active galleriad` → `active` |
| `socket_is_root_galleria_0660` | `stat -c '%U:%G %a'` → `root:galleria 660` |
| `root_gets_hello` | `protocol == 1` |
| `group_member_gets_hello` | `runuser -u alice -- ipc-probe hello` → 0 |
| `non_member_is_denied` (M1) | `runuser -u mallory -- ipc-probe hello` → 13 |
| `unit_hardening_is_active` | `systemctl show galleriad -p ProtectHome -p NoNewPrivileges` → `yes` entrambi |
| `core_dumps_are_disabled` | `systemctl show galleriad -p LimitCORE` → `LimitCORE=0`; in `/proc/<pid di galleriad>/limits` la riga `Max core file size` ha limite 0 |
| `restart_recovers_and_listens_again` | `systemctl restart galleriad`; `hello` risponde entro 5 s |
| `binfmt_units_are_masked` | `systemctl is-enabled systemd-binfmt.service` → `masked` |

- [ ] **2.** `lab/run.sh 26 t07` → fallisce (laboratorio assente). **3.** Scrivi immagini, script, unit, helper.
- [ ] **4.** `lab/run.sh 26 t07` → 9 superati; `docker ps -a --filter name=glab-` vuoto dopo l'uscita.
- [ ] **5.** Job `lab` in `ci.yml`: matrice `client: [26, 27, 79]`, passi `sudo modprobe wireguard`, `LAB_BUILD=host lab/run.sh ${{ matrix.client }}`.
- [ ] **6. Commit:** `add docker lab, systemd unit and end-to-end harness`

**Fatto quando:** `lab/run.sh` è verde in locale su almeno il client 26 e lascia l'host pulito. Chiude S2 (parte
systemd) e S6 (demone a vuoto): l'esito va annotato per il T16.

---

### Task 8: backend WireGuard

**Stima:** 6 h. **File:** crea `crates/galleria-backend-wireguard/{Cargo.toml,src/lib.rs,src/validate.rs,src/device.rs,
src/session.rs}`, `crates/galleriad/src/factory.rs`, `crates/galleria-e2e/tests/t08_wireguard.rs`; modifica `main.rs`.

**Interfacce — usa:** `Backend`, `NetConfig`, `WG_INTERFACE`, `WG_FWMARK`. **Produce:**

```rust
pub struct ValidWireGuard { /* chiavi di 32 byte, IpNet, peer con endpoint host:porta */ }
pub fn validate(p: &WireGuardParams) -> Result<ValidWireGuard, RpcError>;      // invalid_config + data.field
pub fn net_config(cfg: &ValidWireGuard, ipv6_block_enabled: bool) -> NetConfig;
pub struct DeviceConfig; pub struct DeviceStats { pub rx_bytes: u64, pub tx_bytes: u64, pub last_handshake: Option<std::time::SystemTime> }
pub trait WgDevice: Send + 'static {
    fn create(&mut self) -> Result<(), WgError>;
    fn configure(&mut self, cfg: &DeviceConfig) -> Result<(), WgError>;
    fn read(&self) -> Result<DeviceStats, WgError>;
    fn remove(&mut self) -> Result<(), WgError>;
}
pub struct KernelDevice;                                                        // unico file che importa defguard_wireguard_rs
pub struct WireGuardBackend; impl WireGuardBackend { pub fn new(cfg: ValidWireGuard, options: &ConnectOptions, device: Box<dyn WgDevice>) -> Self; }
// galleriad/factory.rs
pub fn real_factory(daemon_exe: PathBuf) -> BackendFactory;
pub fn backend_infos() -> Vec<BackendInfo>;
```

`KernelDevice` usa di `defguard_wireguard_rs` solo creazione, configurazione, lettura e rimozione; **mai**
`configure_peer_routing` né `set_dns` (spec §6.2).

- [ ] **1. Test unitari che falliscono:**

| Test | Asserzione |
|---|---|
| `validate_rejects_*` (rstest) | chiave di 31 byte → `field == "private_key"`; `10.0.0.1/33` → `addresses`; nessun peer → `peers`; MTU 100 → `mtu`; endpoint senza porta → `peers.0.endpoint` |
| `net_config_split_tunnel` | `routes == [10.99.0.0/24, 198.51.100.1/32]`, `install_routes == Some(51820)`, `ipv6_block == None` |
| `net_config_full_tunnel_requests_unreachable_block` | `allowed_ips = [0.0.0.0/0]` → `Unreachable` |
| `net_config_full_tunnel_with_ipv6_default_requests_no_block` | `[0.0.0.0/0, ::/0]` → `None` |
| `ipv6_block_option_disables_block` | `ipv6_block_enabled = false` → `None` |
| `session_calls_device_in_order` | dispositivo finto: `create`, `configure`, (dopo l'ok a `NetUp`) `read`…, e a `Stop` `remove` |
| `net_refusal_removes_device_and_exits_fatal` | risposta `Err` a `NetUp` → `remove`, `Exited(Fatal(net_setup_failed))` |
| `unresolvable_endpoint_exits_transport` | endpoint `nonexistent.invalid:51820` → `Exited(Transport)`, `create` mai chiamato |
| `ipv6_only_endpoint_is_accepted` | `[2001:db8::1]:51820` → `configure` riceve quell'indirizzo |
| `fwmark_is_set_only_for_full_tunnel` | `DeviceConfig.fwmark == Some(51820)` solo con `0.0.0.0/0` |
| `warns_once_after_10s_without_handshake_and_without_keepalive` | tempo in pausa, peer senza `persistent_keepalive`: dopo 10 s da `Up` un solo `Log` di livello `warn` con testo `no handshake yet`; nessun `Exited`, nessun altro evento di stato (spec §6.2, Q7) |
| `no_handshake_warning_when_keepalive_is_set` | `persistent_keepalive = 25` e nessun handshake → nessun avviso |
| `no_handshake_warning_when_handshake_happened` | handshake entro 10 s → nessun avviso |
| `up_does_not_wait_for_handshake` | `Up` subito dopo l'ok a `NetUp`, con `last_handshake == None` |
| `stats_are_emitted_every_second` | |

- [ ] **2.** Fallisce. **3.** Implementa backend e fabbrica. **4.** `cargo test -p galleria-backend-wireguard` → verde.
- [ ] **5. Test end-to-end che falliscono** (`t08_wireguard.rs`, profilo `wg-split.conf` letto e tradotto in `ConnectParams` dal test):

| Test | Asserzione |
|---|---|
| `hello_reports_wireguard_available` | `backends[wireguard].available` |
| `wg_split_connects_and_pings_server` | stato `connected`; `ping -c 3 10.99.0.1` → 0 |
| `wg_split_routes_target_through_tunnel` | `ip route get 198.51.100.1` contiene `dev glr-wg0`; `curl -4 http://198.51.100.1/` → 200 |
| `wg_stats_grow_and_handshake_is_recent` | due `event.stats` con `rx_bytes` crescente durante il ping; `last_handshake` entro 30 s |
| `wg_dns_is_applied_before_connected` | al primo `event.state` `connected`, `resolvectl dns glr-wg0` mostra `10.99.0.1` |
| `wg_disconnect_leaves_no_residue` | `residue()` prima di `connect` == dopo `disconnect` |
| `wg_replace_switches_session` | seconda `connect` → `already_active`; con `replace` nuova `session`, una sola `glr-wg0` |
| `invalid_key_is_invalid_config` | |

- [ ] **6.** `lab/run.sh 26 t08` → verde. **7. Commit:** `add wireguard backend on the linux kernel`

**Fatto quando:** i test unitari e gli 8 end-to-end passano sul client 26. Chiude S1, S2 (modulo), S6 per WireGuard.
Se S1 è falso: ripiego su `netlink-packet-wireguard` dentro `device.rs`, stesso trait, annotato per il T16.

---

### Task 9: WireGuard "tutto nel tunnel", blocco IPv6, pulizia dopo un crash

**Stima:** 5 h. **File:** crea `crates/galleria-e2e/tests/t09_wireguard_full.rs`; modifica `crates/galleria-e2e/src/lib.rs`;
correzioni in `galleria-net` / `galleria-backend-wireguard` se le prove le chiedono (ognuna con il suo test unitario).

**Interfacce — produce** in `galleria_e2e`:

```rust
/// Pacchetti IPv6 non link-local e non multicast visti su eth0 in `secs` secondi, mentre gira `curl -6` in ciclo.
pub fn count_global_ipv6_on_eth0(secs: u64) -> u64;
/// Tempo impiegato da un client che prova gli indirizzi in sequenza (socket.create_connection, timeout 10 s) e famiglia usata.
pub fn sequential_client(host: &str, port: u16) -> (std::time::Duration, AddressFamily);
```

- [ ] **1. Test end-to-end che falliscono** (profilo `wg-full.conf`):

| Test | Asserzione |
|---|---|
| `control_without_block_leaks_ipv6` | `options.ipv6_block = false` → `count_global_ipv6_on_eth0(15) > 0` (la prova di controllo) |
| `full_tunnel_routes_all_ipv4` | `ip route get 198.51.100.1` → `dev glr-wg0`; `ip rule` contiene `not from all fwmark 0xca6c lookup 51820`; ping a `10.99.0.1` ok |
| `full_tunnel_blocks_ipv6_with_zero_packets` | `count_global_ipv6_on_eth0(15) == 0` |
| `ipv6_connect_fails_immediately` | `connect()` a `[2001:db8:99::1]:80` fallisce in meno di 200 ms |
| `sequential_client_falls_back_to_ipv4_under_2s` | `sequential_client("dual.galleria.test", 80)` → IPv4, < 2 s |
| `lan_ipv6_still_works_during_block` | `ping -6 -c 2 2001:db8:31::20` → 0 |
| `status_reports_full_tunnel_and_unreachable_block` | `net.full_tunnel`, `net.ipv6_block == Unreachable` |
| `disconnect_removes_rules_and_unreachable_routes` | `residue()` uguale a prima |
| `kill9_then_restart_cleans_everything` | `systemctl kill -s KILL galleriad`; dopo il riavvio `residue()` uguale a prima e stato `disconnected` |

- [ ] **2.** `lab/run.sh 26 t09` → fallisce o rivela difetti. **3.** Correggi finché passa. **4.** `lab/run.sh 26 t08 && lab/run.sh 26 t09` verdi.
- [ ] **5. Commit:** `verify wireguard full tunnel, ipv6 block and crash cleanup`

**Fatto quando:** i 9 test passano. Chiude S3, S4, S5 (WireGuard). S4 falso → ci si ferma e si riporta (tocca D15).

---

### Task 10: lettura `.ovpn` e lista delle direttive ammesse

**Stima:** 4 h. **File:** crea `crates/galleria-ovpn/{Cargo.toml,src/lib.rs,src/parse.rs,src/classify.rs,src/filter.rs,
tests/fixtures/{base,malicious,crlf}.ovpn}` (nelle fixture i blocchi PEM sono testo segnaposto, non chiavi).

**Interfacce — produce:**

```rust
pub enum Item { Directive { line: usize, name: String, args: Vec<String> }, Block { line: usize, tag: String, body: String },
                ConnectionOpen { line: usize }, ConnectionClose { line: usize } }
pub fn parse(text: &str) -> Result<Vec<Item>, ParseError>;
pub fn render(items: &[Item]) -> String;
pub enum Class { Allowed, DaemonOwned, FileRef, Dangerous, Unknown }
pub fn classify(name: &str, args: &[String]) -> Class;
pub const SECRET_BLOCKS: &[&str] = &["key", "tls-auth", "tls-crypt", "tls-crypt-v2", "pkcs12"];
pub struct Rejected { pub line: usize, pub name: String, pub class: Class }
pub fn check_strict(items: &[Item]) -> Result<(), Vec<Rejected>>;               // demone: tutto ciò che non è Allowed
pub struct Filtered { pub items: Vec<Item>, pub warnings: Vec<Rejected> }
pub fn import_filter(items: Vec<Item>) -> Filtered;                             // import: toglie e avvisa (DaemonOwned senza avviso)
pub fn has_directive(items: &[Item], name: &str) -> bool;
```

Lista completa (i nomi si confrontano così come sono scritti, senza `--` iniziale; maiuscole diverse = sconosciuta):

- **Allowed:** `client`, `tls-client`, `pull`, `dev-type`, `proto`, `remote`, `remote-random`, `remote-random-hostname`,
  `resolv-retry`, `nobind`, `bind`, `lport`, `rport`, `port`, `float`, `cipher`, `data-ciphers`, `data-ciphers-fallback`,
  `ncp-ciphers`, `auth`, `auth-user-pass` (zero argomenti), `auth-nocache`, `static-challenge`, `remote-cert-tls`,
  `verify-x509-name`, `tls-version-min`, `tls-version-max`, `tls-cipher`, `tls-ciphersuites`, `tls-groups`,
  `tls-cert-profile`, `key-direction`, `comp-lzo`, `compress`, `allow-compression`, `tun-mtu`, `mssfix`, `fragment`,
  `mtu-disc`, `sndbuf`, `rcvbuf`, `keepalive`, `ping`, `ping-restart`, `ping-exit`, `ping-timer-rem`, `inactive`,
  `reneg-sec`, `reneg-bytes`, `reneg-pkts`, `hand-window`, `tran-window`, `explicit-exit-notify`, `redirect-gateway`,
  `redirect-private`, `route`, `route-ipv6`, `route-nopull`, `route-metric`, `route-gateway`, `pull-filter`,
  `dhcp-option`, `dns`, `topology`, `block-ipv6`, `mute-replay-warnings`, `replay-window`, `http-proxy-option`,
  `http-proxy` (2 argomenti, o 3 con il terzo tra `none`, `basic`, `ntlm`, `auto`, `auto-nct`), `socks-proxy` (1 o 2
  argomenti), e `ca`, `cert`, `key`, `tls-auth`, `tls-crypt`, `tls-crypt-v2`, `pkcs12`, `extra-certs`, `crl-verify`
  solo come blocco o con argomento `[inline]`.
- **DaemonOwned:** `dev`, `verb`, `mute`, `auth-retry`, `connect-retry`, `connect-retry-max`, `connect-timeout`,
  `server-poll-timeout`, `persist-tun`, `persist-key`, `script-security`, `block-outside-dns`, `ignore-unknown-option`,
  e tutto ciò che inizia per `management`.
- **FileRef:** le nove direttive di file qui sopra con un argomento che non è `[inline]`.
- **Dangerous:** `up`, `down`, `route-up`, `route-pre-down`, `ipchange`, `tls-verify`, `client-connect`,
  `client-disconnect`, `learn-address`, `auth-user-pass-verify`, `plugin`, `setenv`, `setenv-safe`, `config`, `cd`,
  `chroot`, `user`, `group`, `log`, `log-append`, `syslog`, `status`, `writepid`, `daemon`, `engine`, `providers`,
  `dns-updown`, `iproute`, `tmp-dir`, `tls-export-cert`, `up-restart`, `up-delay`, `echo`, `http-proxy-user-pass`,
  `auth-user-pass` con un argomento, `askpass` con un argomento, `http-proxy`/`socks-proxy` con un file di credenziali.
- **Unknown:** tutto il resto.

- [ ] **1. Test che falliscono:**

| Test | Asserzione |
|---|---|
| `parses_quoted_arguments` | `static-challenge "Enter OTP" 1` → `args == ["Enter OTP", "1"]` |
| `skips_comments_and_blank_lines` | righe con `#` e `;` ignorate |
| `strips_leading_double_dash` | `--nobind` → `name == "nobind"` |
| `parses_crlf_and_bom` | `crlf.ovpn` dà gli stessi `Item` di `base.ovpn` |
| `block_body_is_preserved_byte_for_byte` | corpo di `<ca>` identico all'ingresso |
| `unterminated_block_is_an_error` | `<key>` senza chiusura → `ParseError::UnterminatedBlock { line }` |
| `connection_blocks_are_classified_inside` | `up` dentro `<connection>` è `Dangerous` |
| `classify_matches_the_list` (rstest) | **un caso per ogni nome** dell'elenco qui sopra |
| `uppercase_name_is_unknown` | `UP` → `Unknown` |
| `malicious_fixture_is_rejected_by_strict` | nomi rifiutati == {`script-security`, `up`, `down`, `route-up`, `ipchange`, `plugin`, `management`, `status`, `writepid`, `setenv`, `verb`, `dev`} (il profilo malevolo dello spike 2, criterio 2.1, che ha anche `dev tun` e `verb 3`) |
| `import_filter_then_strict_always_passes` (proptest) | per ogni profilo generato: `check_strict(import_filter(parse(x)).items).is_ok()` |
| `import_filter_warns_on_dangerous_not_on_daemon_owned` | avviso per `up`, nessuno per `verb` |
| `render_parse_roundtrip` | `parse(render(items)) == items` |

- [ ] **2.** Fallisce. **3.** Implementa. **4.** `cargo test -p galleria-ovpn` → verde.
- [ ] **5.** Verifica di completezza (S7): passa a `classify` ogni direttiva dei profili di `lab/out/*.ovpn`
  (tranne `malicious`) e delle sezioni "Client Options" e "Link Options" del manuale di OpenVPN 2.6; le direttive
  sensate rimaste `Unknown` si aggiungono con il loro caso di test, le altre si elencano per il T16.
- [ ] **6. Commit:** `add ovpn parser and directive allowlist`

**Fatto quando:** un caso per ogni direttiva elencata; la proprietà import → strict passa.

---

### Task 11: backend OpenVPN, parti pure

**Stima:** 5 h. **File:** crea `crates/galleria-backend-openvpn/{Cargo.toml,src/lib.rs,src/mgmt.rs,src/queue.rs,
src/config.rs,src/push.rs,src/hookenv.rs,src/version.rs}`.

**Interfacce — usa:** `galleria_ovpn`, `NetConfig`, `OVPN_INTERFACE`. **Produce:**

```rust
// mgmt.rs
pub struct StaticChallenge { pub echo: bool, pub concat: bool, pub text: String }
pub enum AuthKind { Auth, PrivateKey }
pub enum MgmtEvent { Info(String), Hold, State { name: String, detail: String, local_ip: Option<IpAddr>, remote: Option<SocketAddr> },
    ByteCount { rx: u64, tx: u64 }, NeedAuth { static_challenge: Option<StaticChallenge> }, NeedDynamicChallenge,
    NeedKeyPassphrase, VerificationFailed(AuthKind), Log { flags: String, text: String }, Fatal(String),
    Success(String), Error(String), Other(String) }
pub fn parse_line(line: &str) -> MgmtEvent;
pub fn mgmt_quote(s: &str) -> String;
pub fn auth_response(password: &str, otp: Option<&str>, sc: Option<&StaticChallenge>) -> String;
// queue.rs — un comando alla volta; il successivo parte dopo la risposta o dopo 400 ms
pub struct CommandQueue; impl CommandQueue { pub fn push(&mut self, cmd: String); pub fn next_to_send(&mut self, now: Instant) -> Option<String>; pub fn on_reply(&mut self); }
// version.rs
pub struct OpenVpnVersion { pub major: u32, pub minor: u32 }
pub fn parse_version(first_line: &str) -> Option<OpenVpnVersion>;
pub const MIN_VERSION: OpenVpnVersion = OpenVpnVersion { major: 2, minor: 6 };
// config.rs
pub struct ValidOpenVpn { /* Item già passati da check_strict + blocchi segreti */ }
pub fn validate(p: &OpenVpnParams) -> Result<ValidOpenVpn, RpcError>;          // profile_rejected + data.directives
pub struct Redirect { pub ipv4: bool, pub ipv6: bool }                         // `redirect-gateway` e il suo flag `ipv6`
pub fn profile_redirect(p: &ValidOpenVpn) -> Redirect;
pub fn profile_ipv6_routes(p: &ValidOpenVpn) -> Vec<IpNet>;                    // dalle `route-ipv6` del profilo
pub struct GenInput<'a> { pub profile: &'a ValidOpenVpn, pub version: OpenVpnVersion, pub mgmt_socket: &'a Path,
                          pub hook_socket: &'a Path, pub daemon_exe: &'a Path, pub options: &'a ConnectOptions }
pub fn generate_config(input: &GenInput) -> String;                            // mai direttive IPv6 del demone (spec §7.3)
// push.rs
pub struct Pushed { pub redirect: Redirect, pub routes: Vec<IpNet>,            // `routes`: `route` e `route-ipv6`
                    pub ifconfig_ipv6: bool, pub dns_servers: Vec<IpAddr> }
pub fn parse_push_reply(text: &str) -> Option<Pushed>;
// hookenv.rs
pub const HOOK_ENV_PREFIXES: &[&str] = &["dev", "script_type", "foreign_option_", "ifconfig_", "route_", "redirect_gateway", "dns_", "tun_mtu"];
pub struct HookEnv { pub script_type: String, pub dev: String, pub dns_servers: Vec<IpAddr>, pub search_domains: Vec<String>,
                     pub routes: Vec<IpNet>,                    // `route_network_N` e `route_ipv6_network_N`
                     pub has_ipv6_address: bool }               // solo informativo: NON decide il blocco
pub fn parse_hook_env(env: &BTreeMap<String, String>) -> Result<HookEnv, String>;
pub struct ProfileFacts { pub redirect: Redirect, pub ipv6_routes: Vec<IpNet> }
pub fn net_config(env: &HookEnv, pushed: Option<&Pushed>, profile: &ProfileFacts, ipv6_block_enabled: bool) -> NetConfig;
```

Regola di `net_config` per il blocco IPv6 (spec §7.3): `Unreachable` se `ipv6_block_enabled`, c'è un
`redirect-gateway` IPv4 (profilo o spinto) e il tunnel **non** porta la rotta predefinita IPv6, cioè nessun
`redirect-gateway` ha il flag `ipv6` e `carries_ipv6_default` è falso sull'unione delle rotte IPv6 di profilo,
`PUSH_REPLY` e ambiente; altrimenti `None`. `has_ipv6_address` e `ifconfig_ipv6` non entrano nella decisione.

- [ ] **1. Test che falliscono.** Le righe sono quelle vere delle prove dello spike 2 (`spikes/openvpn-mgmt/evidence/ovpn26/`):

```rust
#[test]
fn parses_connected_state() {
    let MgmtEvent::State { name, detail, local_ip, remote } =
        parse_line(">STATE:1794305544,CONNECTED,SUCCESS,10.8.0.2,172.30.0.10,1194,,") else { panic!() };
    assert_eq!((name.as_str(), detail.as_str()), ("CONNECTED", "SUCCESS"));
    assert_eq!(local_ip, Some("10.8.0.2".parse().unwrap()));
    assert_eq!(remote, Some("172.30.0.10:1194".parse().unwrap()));
}
#[test]
fn builds_scrv1_response() {
    let sc = StaticChallenge { echo: true, concat: false, text: "Enter OTP".into() };
    assert_eq!(auth_response("pw", Some("123456"), Some(&sc)), "SCRV1:cHc=:MTIzNDU2");
    assert_eq!(auth_response("pw", Some("123456"), Some(&StaticChallenge { concat: true, ..sc })), "pw123456");
    assert_eq!(auth_response("pw", None, None), "pw");
}
#[test]
fn reads_push_reply_from_real_log_line() {
    let line = "PUSH: Received control message: 'PUSH_REPLY,dhcp-option DNS 10.8.0.1,dhcp-option DOMAIN galleria.test,\
route 192.0.2.0 255.255.255.0,redirect-gateway def1,route-gateway 10.8.0.1,topology subnet,ping 10,ping-restart 60,\
ifconfig 10.8.0.2 255.255.255.0,peer-id 1,cipher AES-256-GCM,protocol-flags cc-exit tls-ekm dyn-tls-crypt,tun-mtu 1500'";
    let p = parse_push_reply(line).unwrap();
    assert!(p.redirect.ipv4 && !p.redirect.ipv6 && !p.ifconfig_ipv6);
    assert_eq!(p.routes, ["192.0.2.0/24".parse().unwrap()]);
}
```

| Test | Asserzione |
|---|---|
| `parses_progress_and_exit_states` | `>STATE:1794305544,ASSIGN_IP,,10.8.0.2,,,,` → `local_ip` presente; `>STATE:1794305552,EXITING,SIGTERM,,,,,` → `detail == "SIGTERM"` |
| `parses_password_prompts` | `>PASSWORD:Need 'Auth' username/password SC:1,Enter OTP` → `NeedAuth` con `echo`, non `concat`, testo `Enter OTP`; senza `SC:` → `None`; `SC:3,…` → `concat`; `Need 'Private Key' password` → `NeedKeyPassphrase`; `Verification Failed: 'Auth'` → `VerificationFailed(Auth)`; riga con `CRV1:` → `NeedDynamicChallenge` |
| `parses_bytecount_hold_info_log` | `>BYTECOUNT:2432,2408`; `>HOLD:Waiting for hold release:0`; `>INFO:OpenVPN Management Interface Version 5 -- type 'help' for more info`; `>LOG:1794305544,I,testo` |
| `parses_success_and_error_replies` | `SUCCESS: hold release succeeded` → `Success`; `ERROR: …` → `Error` |
| `garbage_line_is_other_never_panics` (proptest) | qualunque stringa → nessun panico |
| `mgmt_quote_escapes_quotes_and_backslashes` | `a"b\c` → `"a\"b\\c"` |
| `queue_holds_next_command_until_reply` | dopo il primo `next_to_send`, il secondo è `None` finché non arriva `on_reply` o passano 400 ms |
| `parses_openvpn_version_line` | `OpenVPN 2.6.19 x86_64-pc-linux-gnu [SSL (OpenSSL)] [LZO] [LZ4] [EPOLL] [PKCS11] [MH/PKTINFO] [AEAD] [DCO]` → 2.6 |
| `validate_rejects_non_allowed_directives` | profilo con `up x` → `profile_rejected`, `data.directives[0].name == "up"` |
| `validate_rejects_unknown_secret_block` | `secret_blocks: {"ca": …}` → `invalid_config` |
| `generated_config_has_daemon_directives_last` | contiene, dopo le righe del profilo, tutte le righe della spec §6.3 con `mgmt.sock`, `dev glr-ovpn0`, `up`/`down` verso `ovpn-hook` |
| `dns_updown_disable_only_from_2_7` | assente con 2.6, presente con 2.7 |
| `generated_config_never_adds_ipv6_directives` (rstest) | profilo con o senza `redirect-gateway def1`, `ipv6_block` acceso o spento: tra le righe aggiunte dal demone nessuna `redirect-gateway`, `block-ipv6`, `ifconfig-ipv6` |
| `push_reply_reads_ipv6_redirect_and_routes` | `PUSH_REPLY,redirect-gateway def1 ipv6,route-ipv6 2001:db8:77::/48,ifconfig-ipv6 fd00:8::2/64 fd00:8::1,…` → `redirect.ipv6`, `routes` contiene `2001:db8:77::/48`, `ifconfig_ipv6` |
| `profile_redirect_reads_the_ipv6_flag` | `redirect-gateway def1` → `{ipv4: true, ipv6: false}`; `redirect-gateway def1 ipv6` → entrambi; `redirect-gateway ipv6 !ipv4` → `{ipv4: false, ipv6: true}` |
| `ignore_pushed_routes_adds_pull_filters` | `pull-filter ignore "route "` e `pull-filter ignore "redirect-gateway"` |
| `secret_blocks_are_rendered_as_inline_blocks` | `<key>…</key>` in fondo |
| `hook_env_reads_foreign_options` | ambiente vero dello spike (`foreign_option_1=dhcp-option DNS 10.8.0.1`, `foreign_option_2=dhcp-option DOMAIN galleria.test`, `route_network_1=192.0.2.0`, `route_netmask_1=255.255.255.0`, `dev=glr-ovpn0`, `script_type=up`) → DNS `10.8.0.1`, dominio `galleria.test`, rotta `192.0.2.0/24`, `has_ipv6_address == false` |
| `hook_env_reads_ipv6_address_and_routes` | con `ifconfig_ipv6_local` → `has_ipv6_address`; con `route_ipv6_network_1=2000::/4` e `route_ipv6_network_2=3000::/4` → entrambe in `routes` |
| `net_config_decides_ipv6_block` (rstest) | redirect spinto, niente IPv6 → `Unreachable`; redirect nel profilo, niente IPv6 → `Unreachable`; **redirect + solo indirizzo IPv6 nel tunnel (`ifconfig_ipv6_local`, nessuna rotta predefinita IPv6) → `Unreachable`**; **redirect + indirizzo IPv6 + rotta IPv6 parziale (`2001:db8:77::/48`) → `Unreachable`**; redirect con flag `ipv6` spinto → `None`; redirect con flag `ipv6` nel profilo → `None`; redirect + `route_ipv6_network_N` che coprono `2000::/3` → `None`; redirect + `route-ipv6 ::/0` nel profilo → `None`; nessun redirect → `None`; opzione spenta → `None`. Sempre `install_routes == None` |

- [ ] **2.** Fallisce. **3.** Implementa. **4.** `cargo test -p galleria-backend-openvpn` → verde.
- [ ] **5. Commit:** `add openvpn management parser and config generation`

**Fatto quando:** tutti i test passano; ogni riga della tabella "Riga → Effetto" della spec §6.3 ha un test.

---

### Task 12: backend OpenVPN, processo e helper

**Stima:** 9 h. **File:** crea `crates/galleria-backend-openvpn/src/{session,plat,hook}.rs`,
`crates/galleria-e2e/tests/t12_openvpn.rs`; modifica `crates/galleriad/src/{main,factory}.rs`, `.github/workflows/ci.yml`.

**Interfacce — usa:** tutto il T11. **Produce:**

```rust
pub struct OpenVpnBinary { pub path: PathBuf, pub version: OpenVpnVersion }
pub enum Unavailable { NotFound, TooOld(OpenVpnVersion) }
pub fn probe() -> Result<OpenVpnBinary, Unavailable>;                    // /usr/sbin/openvpn, poi /usr/bin/openvpn
pub struct OpenVpnBackend; impl OpenVpnBackend { pub fn new(cfg: ValidOpenVpn, bin: OpenVpnBinary, options: ConnectOptions, daemon_exe: PathBuf) -> Self; }
pub fn hook_main(socket: &Path) -> i32;                                   // `galleriad ovpn-hook <socket>`
/// Ciclo della sessione, generico sul canale della management: è ciò che si prova senza processo.
pub async fn drive<S: AsyncRead + AsyncWrite + Unpin>(mgmt: S, creds: Credentials, ctx: &SessionContext,
    commands: mpsc::Receiver<BackendCommand>, hook: mpsc::Receiver<HookCall>) -> ExitReason;
```

Cartella di sessione: quella ricevuta in `SessionContext.session_dir` (nome casuale, creata e rimossa dal demone,
spec §7.4; il backend non costruisce percorsi dal numero di sessione), con dentro `mgmt.sock` e `hook.sock`.
Arresto: `signal SIGTERM` sulla management, 10 s, poi `kill`. `stdout`/`stderr` di OpenVPN → `Log`.

**Ordine tra `--up` e `CONNECTED` (spec §6.3).** OpenVPN chiama `--up` e ne aspetta l'uscita **prima** di mandare
`>STATE:…,CONNECTED` (`spikes/openvpn-mgmt/evidence/ovpn26/2.7-dhcpopt.log`, righe 57–72: `ASSIGN_IP`, chiamata
del programma `--up`, rotte, `CONNECTED`). `drive` quindi non aspetta mai `CONNECTED` per servire l'helper:

1. chiamata dell'helper con `script_type=up` → `NetUp` subito, con ciò che si sa in quel momento (ambiente +
   `PUSH_REPLY` ricordato) → alla risposta del demone, verdetto all'helper;
2. `>STATE:…,CONNECTED` → `Up`, solo se il `NetUp` di questa connessione è stato accettato;
3. `>STATE:…,RECONNECTING` dopo un `Up` → `Restarting`, e il `PUSH_REPLY` ricordato si scarta;
4. chiamata dell'helper con `script_type=down` → `NetDown`; alla risposta l'helper esce 0. Poi si torna al punto 1.

`drive` continua a leggere la management mentre aspetta la risposta a `NetUp`/`NetDown` (un `select!`, non un'attesa in fila).

- [ ] **1. Test unitari che falliscono** (management simulata con `tokio::io::duplex`):

| Test | Asserzione |
|---|---|
| `releases_hold_after_setup_commands` | scritti nell'ordine, uno per risposta: `version 3`, `state on`, `bytecount 1`, `log on`, `hold release` |
| `hook_up_is_answered_before_connected` | la management simulata manda `ASSIGN_IP` e **nient'altro** finché l'helper non ha il verdetto: chiamata `up` → `NetUp` → risposta `Ok` → verdetto positivo all'helper, senza nessun `CONNECTED` (nessuno stallo); in questo momento nessun `Up` è stato emesso |
| `up_is_sent_at_connected_after_netup_is_accepted` | dopo il caso sopra, `>STATE:…,CONNECTED,SUCCESS,…` → `Up` con indirizzo e remoto |
| `connected_without_accepted_netup_sends_no_up` | `CONNECTED` senza chiamata dell'helper → nessun `Up` |
| `netup_refusal_makes_hook_fail` | risposta `Err` → verdetto negativo all'helper |
| `internal_restart_emits_restarting_netdown_and_new_netup` | dopo `Up`: `>STATE:…,RECONNECTING,ping-restart,…` → `Restarting("ping-restart")`; helper `down` → `NetDown`, verdetto 0; `WAIT`/`AUTH` → `Progress`; nuovo `PUSH_REPLY` con un altro DNS + helper `up` → `NetUp` con il DNS **nuovo**; `CONNECTED` → `Up` |
| `restart_forgets_the_previous_push_reply` | primo `PUSH_REPLY` con `redirect-gateway def1`, dopo il riavvio un `PUSH_REPLY` senza → il secondo `NetUp` ha `ipv6_block == None` |
| `restart_events_are_tolerated_in_any_order` (rstest) | `down` prima di `RECONNECTING`, `RECONNECTING` prima di `down`, `RECONNECTING` senza `down`: sempre un `Restarting`, mai un panico, il nuovo `up` porta sempre un `NetUp` |
| `hook_down_is_answered_while_stopping` | `Stop` → `signal SIGTERM`; helper `down` → `NetDown`, verdetto 0; poi `Exited(Stopped)` |
| `answers_auth_with_credentials` | scrive `username "Auth" "alice"` e `password "Auth" "…"` |
| `missing_otp_asks_for_input` | `SC:1,Enter OTP` senza OTP → `InputRequired { kind: Otp, prompt: "Enter OTP" }`; dopo `ProvideInput` risponde `SCRV1:…` |
| `verification_failed_exits_auth_failed` | |
| `dynamic_challenge_exits_unsupported` | `Fatal(unsupported_challenge)` |
| `socket_close_exits_transport` | |
| `stop_sends_sigterm_signal` | scrive `signal SIGTERM`, `Exited(Stopped)` |
| `push_reply_in_log_is_remembered_for_netup` | `>LOG` con `PUSH_REPLY,redirect-gateway def1` prima dell'helper → `NetUp.config.ipv6_block == Unreachable` |
| `hook_forwards_only_allowed_env` | ambiente con `password=x` e `foreign_option_1=…` → al demone arriva solo la seconda |
| `hook_exit_code_follows_verdict` | `up`: `ok` → 0; rifiuto → 1; demone irraggiungibile → 1. `down`: sempre 0 |

- [ ] **2.** Fallisce. **3.** Implementa sessione, `plat` (socket Unix, `--config /dev/stdin`), helper, `probe`, voce
  `openvpn` in `backend_infos()`. **4.** `cargo test -p galleria-backend-openvpn` → verde.
- [ ] **5. Test end-to-end che falliscono** (`t12_openvpn.rs`; `alice` / `alice-secret`). Aggiungi a `galleria_e2e`
  `pub fn ovpn_params(profile: &str) -> OpenVpnParams`, che legge `lab/out/<profile>.ovpn` con `galleria_ovpn::parse`
  e sposta i blocchi di `SECRET_BLOCKS` in `secret_blocks`:

| Test | Asserzione |
|---|---|
| `hello_reports_openvpn_version` | `available`, versione che inizia con quella del client (2.6 o 2.7) |
| `ovpn_connects_with_password` | `connected`, `local_addresses[0]` in `10.8.0.0/24`, `ping -c 3 10.8.0.1` ok |
| `ovpn_reports_progress_before_connected` | tra gli `event.state` in `connecting` compare un `detail` tra `AUTH`, `GET_CONFIG`, `ASSIGN_IP` |
| `ovpn_stats_grow_during_ping` | `event.stats.rx_bytes` crescente |
| `ovpn_pushed_dns_is_applied_before_connected` | al primo `connected`: `resolvectl dns glr-ovpn0` = `10.8.0.1`; `resolvectl query intranet.galleria.test` risponde via `glr-ovpn0` |
| `ovpn_dns_option_server_works` | stesso esito con `dns-option.ovpn` |
| `status_shows_pushed_routes_and_dns` (M3) | `net.routes` contiene `192.0.2.0/24`; `net.full_tunnel` |
| `ignore_pushed_routes_drops_server_routes` | con l'opzione: `ip route` senza `192.0.2.0/24`, `net.full_tunnel == false` |
| `ovpn_cmdline_and_disk_have_no_secrets` | `/proc/<pid>/cmdline` == `openvpn --config /dev/stdin`; una riga della chiave privata cercata in `/run /tmp /var /etc` (escluso `lab/out` montato) → 0 file |
| `mgmt_socket_is_denied_to_normal_user` (M5) | `runuser -u alice` che apre `mgmt.sock` → permesso negato |
| `malicious_profile_is_rejected_and_nothing_runs` (M2) | `profile_rejected`; nessun `/tmp/PWNED-*`; nessun processo `openvpn` |
| `ovpn_disconnect_leaves_no_residue` | `residue()` uguale a prima; cartella di sessione rimossa (`/run/galleria/s/` vuota) |
| `session_dir_name_is_random_and_private` | durante la connessione `/run/galleria/s/` ha una sola voce, `0700 root`, con un nome che non è il numero di sessione; due connessioni di fila usano due nomi diversi |
| `kill9_daemon_kills_openvpn_and_cleans` | dopo `KILL` e riavvio: nessun `openvpn`, `residue()` uguale, `/run/galleria/s/` vuota già alla prima risposta a `hello` (spec §7.4) |
| `replace_wireguard_with_openvpn` (D5) | da `wg-split` connesso, `connect` OpenVPN con `replace` → `glr-wg0` assente, `glr-ovpn0` presente |
| `ovpn_dco_state_and_bytecount` | solo se `GALLERIA_E2E_DCO=1`: nel log `DCO` attivo, stato e statistiche come sopra |

- [ ] **6.** `lab/run.sh 26 t12 && lab/run.sh 27 t12 && lab/run.sh 79 t12` → verdi (il test DCO si salta da solo).
- [ ] **7.** Job `lab-dco` in `ci.yml` (`continue-on-error: true`): installa sul runner il modulo DCO disponibile
  (`openvpn-dco-dkms`), `GALLERIA_E2E_DCO=1 LAB_BUILD=host lab/run.sh 26 dco`.
- [ ] **8. Commit:** `add openvpn backend driven through the management interface`

**Fatto quando:** i test passano sui tre client. Chiude S5 (OpenVPN), S6, S8; S14 solo se il job `lab-dco` riesce.

---

### Task 13: OpenVPN — credenziali, errori, riconnessione

**Stima:** 6 h. **File:** crea `crates/galleria-e2e/tests/t13_openvpn_auth.rs`; correzioni in `session.rs` / `manager.rs`
con i loro test unitari. Per provocare un riavvio interno senza far cadere il server: `kill -USR1 <pid di openvpn>`.

- [ ] **1. Test end-to-end che falliscono:**

| Test | Asserzione |
|---|---|
| `missing_password_asks_then_connects` | `connect` senza password → `needs_input` con `kind == Credentials`; `provide_input` → `connected` |
| `otp_static_challenge_prompts_and_connects` | profilo `otp`, utente `bob`: `needs_input` con `kind == Otp`, `prompt == "Enter OTP"`; codice da `oathtool` → `connected` |
| `wrong_password_is_auth_failed_without_retry` | `error.code == AuthFailed` entro 10 s; `attempt` mai > 1 |
| `wrong_otp_is_auth_failed` | |
| `encrypted_key_asks_for_passphrase` | profilo `enckey`: `kind == KeyPassphrase`; con la passphrase → `connected` |
| `unreachable_server_gives_up_after_four_attempts` | profilo `unreachable`: `attempt` 1→4, poi `error.code == ServerUnreachable`, in 20–45 s |
| `internal_restart_returns_to_reconnecting_then_connected` (S13) | connesso con `base.ovpn`; `kill -USR1` a `openvpn` → `event.state` `reconnecting` con la **stessa** `session` e lo stesso pid di `openvpn`; poi `connected` entro 30 s; nella sequenza degli stati non c'è mai un `connected` tra il segnale e la nuova `CONNECTED` |
| `internal_restart_removes_and_reapplies_dns_and_ipv6_block` (M4) | stesso scenario: mentre lo stato è `reconnecting`, `resolvectl dns glr-ovpn0` non mostra più `10.8.0.1` (o l'interfaccia non esiste) e `ip -6 route` non ha `unreachable`; al primo `connected` successivo ci sono di nuovo entrambi; dopo `disconnect` `residue()` è uguale a prima |
| `blackholed_server_triggers_reconnect_and_recovery` (S13) | connesso; `ip route add blackhole 172.31.0.10/32` → dopo `ping-restart` stato `reconnecting`, DNS e rotte `unreachable` tolti; il riavvio interno fallisce, il processo esce e il demone ne avvia uno nuovo (`attempt` cresce); tolta la rotta → `connected` entro 60 s con DNS riapplicati |
| `reconnect_of_otp_profile_asks_for_a_new_otp` | stesso scenario con `bob` → `needs_input` (`Otp`), non riuso del vecchio codice |
| `auth_nocache_survives_renegotiation` (S12) | profilo con `auth-nocache` e `reneg-sec 10`: ancora `connected` dopo 25 s, senza `needs_input` |
| `disconnect_during_needs_input_cleans_up` | `disconnect` in `needs_input` → `disconnected`, nessun `openvpn` |
| `known_secrets_never_appear_in_logs_or_journal` (M7) | `alice-secret`, l'OTP usato e una riga della chiave privata assenti da `logs` (IPC) e da `journalctl -u galleriad` |

- [ ] **2.** `lab/run.sh 26 t13` → fallisce o rivela difetti. **3.** Correggi (test unitario per ogni correzione).
- [ ] **4.** Verde su 26, 27, 79. **5. Commit:** `verify openvpn credentials, failures and reconnection`

**Fatto quando:** i 13 test passano sui tre client. Chiude S12 e S13; S16 resta **[S]** (solo il test unitario del T11).

---

### Task 14: OpenVPN — blocco IPv6

**Stima:** 5 h. **File:** crea `crates/galleria-e2e/tests/t14_openvpn_ipv6.rs`; correzioni in `galleria-backend-openvpn`
con i loro test unitari.

- [ ] **1. Test end-to-end che falliscono:**

| Test | Asserzione |
|---|---|
| `control_without_block_leaks_ipv6` | `base.ovpn`, `ipv6_block: false` → `count_global_ipv6_on_eth0(15) > 0` |
| `pushed_redirect_gets_unreachable_block` (S9) | `base.ovpn`: `net.ipv6_block == Unreachable`; 0 pacchetti; `sequential_client` → IPv4 in < 2 s |
| `profile_redirect_gets_unreachable_block` | `split.ovpn` + `redirect-gateway def1` nel profilo: `net.ipv6_block == Unreachable`; 0 pacchetti; IPv4 in < 2 s; nella configurazione passata a OpenVPN (dal log `verb 3`) nessuna `block-ipv6` né `ifconfig-ipv6` |
| `split_tunnel_leaves_ipv6_untouched` (S10) | `split.ovpn`: `net.ipv6_block == None`, `net.full_tunnel == false`; `curl -6 http://[2001:db8:99::1]/` → 200 dalla rete fisica |
| `server_with_ipv6_default_keeps_ipv6_inside_the_tunnel` (S11) | `v6.ovpn` (il server spinge `redirect-gateway def1 ipv6`): `net.ipv6_block == None`; `ip -6 route` senza `unreachable`; `ip -6 route get 2001:db8:99::1` → `dev glr-ovpn0`; `curl -6` → 200; 0 pacchetti IPv6 globali su `eth0` |
| `ipv6_address_without_ipv6_default_keeps_the_block` | `v6.ovpn` + nel profilo `pull-filter ignore "redirect-gateway"` e `redirect-gateway def1`: il tunnel ha un indirizzo IPv6 (`ip -6 addr show glr-ovpn0` ne mostra uno globale o ULA) ma non la rotta predefinita IPv6 → `net.ipv6_block == Unreachable`; 0 pacchetti IPv6 globali su `eth0`; `sequential_client` → IPv4 in < 2 s; il ping all'indirizzo IPv6 del server **dentro** il tunnel funziona ancora |
| `control_ipv6_address_without_block_leaks` | stesso profilo con `ipv6_block: false` → `count_global_ipv6_on_eth0(15) > 0`: è il caso che il blocco tolto troppo presto lasciava scoperto |
| `unreachable_routes_are_gone_after_disconnect` | `ip -6 route` senza `unreachable` |
| `unreachable_routes_are_gone_after_kill9` | idem dopo `KILL` e riavvio |

- [ ] **2.** `lab/run.sh 26 t14` → fallisce o rivela difetti. **3.** Correggi. Se il `PUSH_REPLY` non arriva dalla
  management (S9 falso) leggi la stessa riga da `stdout` di OpenVPN, dove lo spike 2 l'ha già vista; se S10 è falso
  (blocco IPv6 su un profilo che non è "tutto nel tunnel") è un difetto da correggere. Per S11 annota, per ogni
  versione, quale fonte dice che il tunnel porta la rotta predefinita IPv6 (flag `ipv6` nel `PUSH_REPLY`,
  `route_ipv6_network_N` nell'ambiente di `--up`, o entrambe): se una delle due manca su una versione si usa
  l'altra; se mancano entrambe il blocco resta (fallisce chiuso) e lo si scrive nell'esito.
- [ ] **4.** Verde su 26, 27, 79. **5. Commit:** `verify openvpn ipv6 block on pushed, profile and split routes`

**Fatto quando:** i 9 test passano sui tre client. Chiude S9, S10, S11 (spec §7.3, Q3 decisa: un solo meccanismo,
rotte `unreachable`, nessuna direttiva nativa).

---

### Task 15: profili TOML, import, archivio dei segreti

**Stima:** 5 h. **File:** crea `crates/galleria-profile/{Cargo.toml,src/lib.rs,src/model.rs,src/store.rs,src/secrets.rs,
src/import_wg.rs,src/import_ovpn.rs}`.

**Interfacce — usa:** `galleria_ipc::{ConnectParams, Secret}`, `galleria_ovpn`. **Produce:**

```rust
pub const SCHEMA_VERSION: u32 = 1;
pub struct Profile { /* campi della spec §8.2 */ }  pub struct Settings;  pub enum ProtocolConfig { Openvpn(OpenVpnProfile), Wireguard(WireGuardProfile) }
pub enum StoreError { NotFound, Ambiguous(Vec<(uuid::Uuid, String)>), NewerSchema { found: u32 }, Invalid(String), Io(std::io::Error),
                      InsecurePermissions { path: PathBuf, mode: u32 }, WrongOwner { path: PathBuf } }
pub struct ProfileStore; impl ProfileStore {
    pub fn default_dir() -> PathBuf;                                    // $XDG_CONFIG_HOME/galleria o ~/.config/galleria
    pub fn open(dir: &Path) -> Result<Self, StoreError>;                // crea profiles/ e galleria.toml se mancano
    pub fn list(&self) -> Result<Vec<Profile>, StoreError>;
    pub fn find(&self, name_or_id: &str) -> Result<Profile, StoreError>;
    pub fn save(&self, profile: &Profile) -> Result<(), StoreError>;    // atomica, 0600
    pub fn delete(&self, id: &uuid::Uuid) -> Result<(), StoreError>;
}
pub trait SecretStore { fn get(&self, id: &uuid::Uuid, key: &str) -> Result<Option<Secret>, StoreError>;
                        fn set(&self, id: &uuid::Uuid, key: &str, value: &Secret) -> Result<(), StoreError>;
                        fn delete_all(&self, id: &uuid::Uuid) -> Result<(), StoreError>; }
/// File `secrets/<id>.toml` 0600 in una cartella 0700 (spec §8.3, Q6 decisa). I permessi si verificano a OGNI lettura.
pub struct FileSecretStore; impl FileSecretStore { pub fn open(dir: &Path) -> Result<Self, StoreError>; }
pub struct ImportWarning { pub line: usize, pub directive: String, pub kind: WarningKind }   // Dangerous | Unsupported | NoTlsClient
pub struct Imported { pub profile: Profile, pub secrets: Vec<(String, Secret)>, pub warnings: Vec<ImportWarning> }
pub fn import_wireguard(text: &str, name: &str) -> Result<Imported, ImportError>;
pub fn import_openvpn(text: &str, name: &str, read_file: &dyn Fn(&str) -> std::io::Result<String>) -> Result<Imported, ImportError>;
pub fn to_connect_params(profile: &Profile, secrets: &dyn SecretStore, replace: bool) -> Result<ConnectParams, StoreError>;
```

- [ ] **1. Test che falliscono:**

| Test | Asserzione |
|---|---|
| `profile_toml_matches_the_spec_example` | il TOML della spec §8.2 si legge e si riscrive uguale (a meno di spazi) |
| `unknown_field_is_an_error` | |
| `newer_schema_version_is_reported` | `schema_version = 2` → `StoreError::NewerSchema { found: 2 }` |
| `save_is_atomic_and_private` | dopo `save`: nessun file temporaneo; file `0600`; cartella `0700` |
| `concurrent_saves_leave_a_valid_file` | due thread, 100 `save` ciascuno → il file si legge sempre |
| `find_by_name_and_by_id` | |
| `find_reports_ambiguous_names` | due profili "Ufficio" → `Ambiguous` con i due id |
| `name_with_path_characters_never_reaches_the_filesystem` | nome `../../etc/x` → file creato solo in `profiles/<uuid>.toml` |
| `import_wireguard_moves_private_key_to_secrets` | il TOML salvato non contiene la chiave; `secrets` ha `wireguard.private_key` |
| `import_wireguard_handles_comma_lists` | `Address = 10.0.0.2/24, fd00::2/64` → due indirizzi; idem `AllowedIPs`, `DNS` |
| `import_wireguard_drops_scripts_with_warning` | `PostUp = …` → avviso `Dangerous`, assente dal profilo; `Table` → `Unsupported` |
| `import_openvpn_inlines_referenced_files` | `ca ca.crt` + `read_file` → blocco `<ca>` nel `config` |
| `import_openvpn_reports_missing_file` | `read_file` che fallisce → `ImportError::MissingFile("ca.crt")` |
| `import_openvpn_moves_secret_blocks_out` | `<key>` e `<tls-auth>` fuori dal `config`, in `secrets` come `openvpn.block.key`, `openvpn.block.tls-auth` |
| `import_openvpn_drops_dangerous_with_warning` | `up /x` → avviso, assente |
| `import_openvpn_maps_dev_tap_to_dev_type` | `dev tap0` → `dev-type tap`, nessun `dev`; `dev tun` → tolto senza avviso |
| `import_openvpn_warns_without_tls_client` | profilo senza `client` né `tls-client` → `NoTlsClient` |
| `imported_openvpn_always_passes_strict` | `galleria_ovpn::check_strict` sul `config` importato → `Ok` |
| `to_connect_params_reassembles_secrets` | `secret_blocks["key"]` valorizzato; opzioni da `[settings]` |
| `secrets_file_is_0600_and_removed_with_profile` | |
| `secrets_read_checks_permissions_every_time` (rstest: `0640`, `0604`, `0660`, `0644`) | dopo un `chmod` del file tra due letture, la seconda `get` → `InsecurePermissions { path, mode }` e nessun segreto restituito; riportato a `0600` → di nuovo `Ok`. Nessuna correzione automatica dei permessi |
| `secrets_dir_with_group_or_other_bits_is_refused` | cartella `secrets/` a `0750` → `InsecurePermissions` sulla cartella |
| `secrets_symlink_is_refused` | `secrets/<id>.toml` sostituito da un collegamento simbolico → errore, il bersaglio non viene letto |

- [ ] **2.** Fallisce. **3.** Implementa. **4.** `cargo test -p galleria-profile` → verde; `scripts/check-boundaries.sh` → 0.
- [ ] **5. Commit:** `add toml profile store, importers and file secret store`

**Fatto quando:** i 23 test passano.

---

### Task 16: CLI `galleria`, prova completa, esito della fase

**Stima:** 6 h. **File:** crea `crates/galleria-cli/{Cargo.toml,src/main.rs,src/cli.rs,src/commands.rs,src/messages.rs,
src/prompt.rs}`, `crates/galleria-e2e/tests/t16_cli.rs`, `docs/06-esito-fase1.md`; modifica `packaging/linux/install.sh`.

**Interfacce — usa:** `galleria_ipc::Client`, `galleria_profile`. **Produce** (pacchetto `galleria-cli`, binario `galleria`):

| Comando | Effetto |
|---|---|
| `galleria import <file> [--name N]` | `.conf` o `.ovpn` (dall'estensione); stampa gli avvisi; salva profilo e segreti |
| `galleria list` · `galleria delete <nome\|id>` | |
| `galleria connect <nome\|id> [--replace] [--username U] [--password-stdin] [--otp-stdin] [--no-wait]` | manda `connect`; segue `event.state` fino a `connected` o `error`; in `needs_input` chiede al terminale senza eco |
| `galleria disconnect` · `galleria status [--json]` · `galleria logs [-f]` · `galleria version` | `version` mostra anche versione e backend del demone |

**Nessun segreto in argv** (spec §8.3). Password, OTP e passphrase della chiave arrivano solo da prompt senza eco
o da stdin; non esiste nessuna opzione che prenda un segreto come valore (`--otp CODICE`, `--password X` non
esistono e clap li rifiuta). Con `--password-stdin` e `--otp-stdin` la CLI legge da stdin una riga per ciascuno,
nell'ordine password poi OTP (una sola riga se è presente una sola delle due opzioni), togliendo il fine riga; con
stdin non terminale e senza l'opzione che serve, un `needs_input` fa uscire con 3. `--username` resta un
argomento: non è un segreto. Esempi:

```sh
galleria connect ufficio                                   # chiede al terminale, senza eco, ciò che manca
galleria connect ufficio --username alice --password-stdin < /run/user/1000/pw     # password da un file 0600
printf '%s\n%s\n' "$PW" "$OTP" | galleria connect ufficio --username bob --password-stdin --otp-stdin
```

(`printf` è un comando interno della shell: il segreto non compare negli argomenti di nessun processo. Mai
`sh -c "echo $PW | …"` né `--otp 123456`.)

Codici di uscita: 0 riuscito · 1 errore generico (anche `already_active`) · 2 `auth_failed` · 3 serve un dato e non
c'è un terminale · 4 `server_unreachable` · 5 demone non raggiungibile, permesso negato o protocollo incompatibile.
`messages.rs`: `pub fn text(code: &str, lang: Lang) -> &'static str` per ogni codice della spec §4.6, in inglese e
italiano; `Lang` da `LC_ALL`/`LANG` (`it*` → italiano, altrimenti inglese).

- [ ] **1. Test unitari che falliscono:**

| Test | Asserzione |
|---|---|
| `parses_every_subcommand` | clap accetta ogni riga della tabella |
| `exit_code_follows_the_table` (rstest) | un caso per riga dei codici di uscita |
| `every_error_code_has_both_languages` | per ogni `ErrorCode` e `SessionErrorCode`: testo inglese e italiano non vuoti e diversi |
| `permission_denied_message_names_the_group` | il testo per `PermissionDenied` contiene `galleria` e `usermod -aG` |
| `incompatible_protocol_message_is_actionable` | contiene le versioni del demone e del client |
| `non_tty_needs_input_exits_3` | con stdin non terminale e senza `--password-stdin`; idem per l'OTP senza `--otp-stdin` |
| `secret_valued_options_do_not_exist` (rstest) | `connect x --otp 123456`, `connect x --password pw`, `connect x --key-passphrase pw` → errore di clap (uscita 2 di clap rimappata a 1), nessuna connessione tentata |
| `stdin_secrets_are_read_in_order` | stdin `"pw\n123456\n"` con le due opzioni → password `pw`, OTP `123456`; con il solo `--otp-stdin` la prima riga è l'OTP; il fine riga (anche `\r\n`) è tolto |
| `prompt_never_echoes` | il prompt usa la lettura senza eco per `Credentials` (password), `Otp` con `echo: false` e `KeyPassphrase` |
| `status_json_is_the_ipc_status` | l'uscita di `status --json` si rilegge come `galleria_ipc::Status` |

- [ ] **2.** Fallisce. **3.** Implementa. **4.** `cargo test -p galleria-cli` → verde.
- [ ] **5. Test end-to-end che falliscono** (`t16_cli.rs`, eseguiti come `alice` con `runuser`):

| Test | Asserzione |
|---|---|
| `cli_wireguard_roundtrip` | `import wg-split.conf` → `list` lo mostra → `connect` esce 0 → `status` dice `connected` → `ping` ok → `disconnect` → `residue()` pulito |
| `cli_openvpn_with_password_stdin` | `import base.ovpn`; `galleria connect base --username alice --password-stdin` con `alice-secret\n` scritto dal test sullo stdin del processo (niente shell, niente `echo`) → 0 |
| `cli_otp_from_stdin` | profilo `otp`, utente `bob`: `--password-stdin --otp-stdin` con password e codice di `oathtool` su due righe di stdin → 0, stato `connected` |
| `cli_secrets_never_appear_in_argv` (M7) | durante i due test qui sopra, `/proc/<pid di galleria>/cmdline` e `ps -eo args` non contengono né la password né l'OTP |
| `cli_wrong_password_exits_2` | |
| `cli_unreachable_exits_4` | |
| `cli_connect_while_active_exits_1_and_replace_works` | |
| `cli_non_member_exits_5` | come `mallory`: uscita 5, messaggio con il nome del gruppo |
| `cli_logs_follow_prints_new_lines` | `logs -f` in sottofondo riceve righe durante una `connect` |
| `cli_ambiguous_name_lists_candidates` | due profili con lo stesso nome → uscita 1, i due id nell'errore |
| `cli_profile_files_are_private` | `~alice/.config/galleria/profiles/*.toml` e `secrets/*.toml` → `600`; il TOML non contiene la chiave privata |
| `daemon_never_opened_user_files` | `ProtectHome` attivo e nessun percorso di `/home` in `ls -l /proc/<pid di galleriad>/fd` |

- [ ] **6.** `lab/run.sh 26 && lab/run.sh 27 && lab/run.sh 79` (tutti i test, senza filtro) → verdi.
- [ ] **7.** Scrivi `docs/06-esito-fase1.md`, sul modello di `04` e `05`: criteri E1–E7 della spec con la prova;
  tabella S1–S16 chiusi/aperti; restrizioni systemd tolte; direttive aggiunte alla lista o rimaste sconosciute (T10);
  per S11, quale fonte ha detto "rotta predefinita IPv6 nel tunnel" su ogni versione (T14); **lista di controllo manuale** per S15 (VM Ubuntu e Fedora desktop con NetworkManager:
  installazione con `install.sh`, `nmcli device` mostra `glr-*` come non gestite, DNS e blocco IPv6 come in
  laboratorio) e per S14 se `lab-dco` non è riuscito; cosa serve da Peppe (questioni della spec ancora aperte: Q1, Q2, Q4, Q5, Q8–Q11; Q3, Q6 e Q7 sono risolte) e
  il promemoria che la parte Linux di D15 nell'analisi va aggiornata con la frase della spec §13, se non è già stato fatto.
- [ ] **8. Commit:** `add galleria cli and record phase 1 outcome`

**Fatto quando:** l'intero laboratorio è verde sui tre client, il job `check` passa, `docs/06-esito-fase1.md` esiste e
non dichiara chiuso nessun **[S]** senza una prova indicata.

---

## Copertura della spec

| Sezione della spec | Task |
|---|---|
| §3 workspace e confini | T1 (scheletro, controllo dei confini), poi un crate per task |
| §4 protocollo IPC | T2 (tipi, codec, versioni), T4 (server, `hello`, limiti), T5 (`subscribe`, `logs`) |
| §5.1 autorizzazione | T4 (unitaria), T7 (end-to-end) |
| §5.2 sessione unica e macchina a stati | T3 |
| §5.3 avvio e arresto | T4 (`PR_SET_DUMPABLE`, pulizia delle cartelle di sessione), T6 (`recover`) |
| §5.4 log e segreti | T5, T13, T16 |
| §6.1 interfaccia dei backend | T3 (compresi `NetDown`, `Restarting` e l'ordine `NetUp` → `Up`) |
| §6.2 WireGuard | T8, T9 |
| §6.3 OpenVPN | T10, T11, T12 (ordine `--up`/`CONNECTED`, riavvio interno), T13 |
| §7 modulo di rete | T6, T9, T14; §7.4 cartelle di sessione: T3, T4, T12 |
| §8 profili e segreti | T15, T16 |
| §9 unit systemd | T7 (`LimitCORE=0`), T4 (`PR_SET_DUMPABLE`) |
| §10 test e CI | T1, T7, T12 |
| §12 punti [S] | colonna "Task" della spec; riepilogo in T16 |
| §13 decisioni Q3, Q6, Q7 | Q3: T2, T6, T11, T12, T14 · Q6: T15 · Q7: T8 |
