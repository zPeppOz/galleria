# Esito della fase 2

Registro dei punti [S] verificati e dei criteri chiusi, aggiornato a ogni task. Spec:
`docs/superpowers/specs/2026-11-25-fase2-gui-design.md`. Un esito scritto qui vale solo per ciò che è stato eseguito:
quello che non lo è stato sta in «Non verificato».

## B1 · Fondamenta

Piano: `docs/superpowers/plans/2026-11-25-fase2-b1-fondamenta.md`. Ambiente di build e prove: immagine
`galleria-gui-dev` (`lab/images/gui.Dockerfile`, Ubuntu 24.04, WebKitGTK 2.52.6), Docker rootless, build come l'utente
che invoca; Node 26.12.1 e pnpm 12.15.0 come in `.nvmrc` e `package.json`.

### Punti [S]

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S1 | La CLI di Tauri trova `crates/galleria-gui/tauri.conf.json` dalla radice | **Vero.** `pnpm tauri info` dalla radice mostra CSP e `frontendDist` letti da quel file; `pnpm tauri build --no-bundle` costruisce il binario. Nessuna variabile `TAURI_CLI_CONFIG_DEPTH` | T1 | nessuno |
| S2 | Senza `ui/dist` il crate non compila | **Vero.** `cargo check -p galleria-gui` senza `ui/dist` fallisce nella macro `generate_context!`: «The `frontendDist` configuration is set to `"../../ui/dist"` but this path doesn't exist» (proc macro panicked). Con `--exclude galleria-gui` i comandi della fase 1 restano verdi (vedi sotto) | T1 | nessuno |
| S3 | Un build con `cargo build` incorpora `ui/dist` solo con la feature `custom-protocol` | **Falso per la nostra configurazione.** Costruiti tre binari release: `cargo build --release`, `cargo build --release --features custom-protocol` e `pnpm tauri build --no-bundle`. In tutti e tre `strings` trova il percorso `assets/index-…js` e `/index.html`; nessuno contiene `localhost:1420`. Il motivo si legge nel sorgente di `tauri-codegen` 2.7.6 (`context.rs`): gli asset si incorporano sempre, tranne quando la feature manca **e** `build.devUrl` è impostato, e noi non impostiamo `devUrl`. La feature cambia solo il cfg `dev` del runtime | T1 | Il comando di build del resto del piano è `scripts/gui.sh build` = `pnpm --dir ui build` + `pnpm tauri build --no-bundle` (la CLI imposta la feature): resta corretto anche se un giorno si aggiunge `devUrl`. Binario: `target/gui-image/release/galleria-gui`. Non ho lanciato i tre binari: l'incorporamento è dedotto da `strings` e dal sorgente, non da una finestra aperta |
| S4 | `tauri::test` basta a provare i comandi | **Vero, con due avvertenze.** Con Tauri 2.12.7 il runtime finto (`mock_builder`, `mock_context`, `get_ipc_response`) invoca i comandi come fa la webview e un `Listener` Rust sente gli eventi emessi da `TauriSink`: 8 prove in `tests/commands.rs`. (1) `Builder::build` **non** esegue l'hook `setup` sul runtime finto: serve una chiamata a `App::run_iteration` (deprecata, in sola prova con `#[allow(deprecated)]`). (2) Il contesto finto non ha ACL: le prove non dicono se i permessi di `capabilities/main.json` ammettono davvero i comandi. Quella corrispondenza la controllano `tauri-build` in compilazione (un permesso inesistente rompe la build) e `tests/config.rs`; l'ammissione nell'app vera l'ha mostrata il fumo di T6: la finestra costruita da `scripts/gui.sh build` ottiene `daemon_view` e `app_info` dall'ACL reale (`shows_the_real_daemon_state` verde). Avvio a mano dell'app costruita da `scripts/gui.sh build` sotto `dbus-run-session` + `xvfb-run` per 8 s senza demone: nessun panico, il supervisore scrive «daemon link DaemonNotRunning» su stderr e l'uscita è 124 (timeout) | T4 | nessuno |
| S5 | `galleriad run` parte nel container senza toccare la rete | **Falso.** Nel container (senza `NET_ADMIN`) `galleriad run --socket … --group … --runtime-dir …` esce subito: «Network command failed: ip -4 rule del not fwmark 51820 table 51820: RTNETLINK answers: Operation not permitted» e «Network setup failed». All'avvio il demone ripulisce lo stato di rete (regole e rotte), quindi non si può lanciare in un fumo che non deve toccare la rete: con i privilegi lo farebbe nella rete in cui gira (su un runner, quella del runner) | T6 | `crates/galleria-gui/examples/fake-daemon.rs`: il vero `galleriad::server::serve` con il backend finto di `tests/support`, sul socket `<dir>/galleriad.sock`. È un esempio del pacchetto (`cargo build --example fake-daemon`), non un binario consegnato; `galleriad` non è stato modificato. Quello che il fumo prova è l'IPC vero, non la partenza di `galleriad run` |
| S6 | Il fumo WebDriver gira nel container e sul runner | **Vero nel container; sul runner il primo tentativo è rosso, la correzione è da provare.** `scripts/gui.sh smoke` (build release dell'app + `fake-daemon` + `tauri-driver` 2.1.0 installato con `cargo install --locked` sotto `target/gui-tools` + `WebKitWebDriver` di `webkit2gtk-driver` + Xvfb + `dbus-run-session`) è passato 3 volte di fila, 5 prove su 5 ogni volta (circa 6 s). Servono `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` (il sandbox bubblewrap non parte in un container senza privilegi) e `WEBKIT_DISABLE_DMABUF_RENDERER=1`, impostate solo dallo script del fumo. **Sul runner, run 45638906906 (commit `a37eed7`):** il job `gui-smoke` è rosso, tutti gli altri verdi. `ubuntu-latest` è Ubuntu 26.04 e `apt-get install` esce con 100: «E: Package 'webkit2gtk-driver' has no installation candidate» (nella 26.04 il pacchetto si chiama `webkitgtk-webdriver`); il job installava i pacchetti sul runner invece di usare l'immagine. **Correzione:** il job lancia `scripts/gui.sh smoke`, cioè la stessa immagine del locale (`ubuntu:24.04`, dove `webkit2gtk-driver` esiste), con il toolchain Rust del runner montato in sola lettura; il `Dockerfile` ha un commento che dice che i nomi dei pacchetti sono quelli della 24.04. **Prova del ramo non rootless di `run_in_image`:** `scripts/gui.sh smoke` da una copia del repository con `docker info` finto (senza «rootless»), quindi `--user 1000:1000` come sul runner, in un utente senza voce in `/etc/passwd`, con `~/.rustup` e `~/.cargo/bin` montati in sola lettura, `cargo install tauri-driver` e l'intero fumo: 5 prove su 5, uscita 0. Il demone Docker era comunque rootless (gli uid del container sono mappati in sub-uid), la copia aveva i permessi aperti a tutti al posto del proprietario uguale all'utente del container: non è un runner. Il job resta con `continue-on-error: true` finché non è verde su un runner; la riga va tolta dopo il primo run verde | T6 | libreria di prova: `webdriverio` da solo con Vitest (config in `ui/e2e/`), non il test runner `@wdio/cli`; due browser driver che `webdriverio` si porta dietro (`edgedriver`, `geckodriver`) hanno gli script di installazione negati in `pnpm-workspace.yaml` (`allowBuilds: false`): non si scarica nulla |
| S7 | `navigator.clipboard.writeText` funziona nella webview per «Copia» | **Vero.** Nella webview WebKitGTK vera, sotto Xvfb, un clic WebDriver su «Copia»/«Copy» riempie gli appunti di sistema: `xclip -o -selection clipboard` restituisce esattamente `sudo systemctl enable --now galleriad` (prova `copy_fills_the_system_clipboard`, verde nelle 3 esecuzioni). Il comando Rust `copy_text` non serve | T5, T6 | non applicato |
| S13 | `cargo deny check` passa con l'albero di Tauri | **Quasi.** Con cargo-deny 0.20.2 e il database degli avvisi di oggi: 2 errori. (1) licenza `Apache-2.0 WITH LLVM-exception` di `target-lexicon` 0.12.16 (dipendenza di `system-deps`): aggiunta all'elenco, è Apache-2.0 con un'eccezione che la allarga, compatibile con GPL-3.0-or-later. (2) `RUSTSEC-2024-0370`, `proc-macro-error` 1.0.4 non più mantenuto, dipendenza di compilazione di `glib-macros` e `gtk3-macros`: ignorato per identificativo, con il motivo in `deny.toml`. Gli avvisi «non più mantenuto» sui crate GTK3 che mi aspettavo **non sono comparsi** con questa versione e questo database; potrebbero comparire in CI con una versione più nuova, e andrebbero trattati come questo (per identificativo, con il motivo). 51 avvisi `duplicate` di `bans`, come atteso: `multiple-versions = "warn"` | T1 | licenza aggiunta, un avviso ignorato per identificativo |

### Criteri di uscita

| # | Prova | Esito |
|---|---|---|
| G1 | Collegamento e ricollegamento senza intervento dell'utente | Rust: `daemon_restart_relinks_without_user_action` (`tests/supervisor.rs`, 20 esecuzioni di fila verdi) e `slow_sink_does_not_stall_the_event_connection`; webview vera: `shows_the_real_daemon_state` e `relinks_after_a_daemon_restart` (il demone si ferma, compare il comando `systemctl`; riparte sullo stesso socket e lo stato torna entro 10 s senza toccare la finestra) | **Verde**: `scripts/gui.sh check` e `scripts/gui.sh smoke` (3 volte). Su un runner: da provare |
| G2 | Quattro stati del primo avvio, senza privilegi | `classify` in `tests/link.rs` e sette prove pure di `link.rs` (demone assente, permesso negato, protocollo incompatibile, collegato); il rifiuto del secondo livello, che il demone vero fa chiudendo subito dopo `accept` senza leggere `hello` (il client vede `Closed`, `BrokenPipe` o `ConnectionReset`, a seconda di chi arriva prima), in `daemon_closing_right_after_accept_is_permission_denied` (200 tentativi contro un server che chiude senza leggere) e nelle prove di `classify_hello` (un `BrokenPipe` o `ConnectionReset` durante `hello` è permesso negato, uno della `connect()` no); schermate `FirstRun` e `StatusView` in Vitest, con la tastiera, axe e `nothing_is_executed`; webview vera: `first_run_without_daemon`. Il permesso negato e il protocollo incompatibile si provano solo a livello 1 e 2 (il fumo non ha un demone che li produca) | **Verde** a livello 1 e 2 per i quattro stati; a livello 3 per «demone assente» e «collegato». Nessuna schermata guardata a occhio, con un'immagine, in una finestra |

### Verifiche del T1

- Rosso prima del verde: `scripts/tests/check-boundaries.sh` falliva («boundary check accepted galleria-profile», lo
  script ignorava `BOUNDARY_TREE`); i cinque test di `tests/config.rs` fallivano per i file mancanti; `App.test.tsx`
  falliva perché `./App` non si risolveva.
- Verdi: `scripts/tests/check-boundaries.sh`; `tests/config.rs` (5 prove); `App.test.tsx` (1 prova);
  `scripts/gui.sh check`; sull'host `cargo fmt --all --check`, `cargo clippy --workspace --exclude galleria-gui
  --all-targets -- -D warnings`, `cargo test --workspace --exclude galleria-gui` (529 passate, 81 ignorate, 0 fallite:
  nessun file della fase 1 è cambiato, quindi il numero è quello di prima), `cargo deny check`,
  `scripts/check-boundaries.sh`.

### Verifiche del T6

- Rosso prima del verde: `pnpm --dir ui test:e2e` nell'immagine senza l'ambiente del fumo: 5 prove su 5 fallite
  («GALLERIA_SMOKE_BIN is not set»). Primo giro di `scripts/gui.sh smoke` con `galleriad run`: 2 fallite su 5 (le prove
  che avviano il demone, S5), 3 verdi. Con `fake-daemon`: 5 su 5, poi altre due volte di fila (3 su 3).
- Numero di prove per livello (spec §14): livello 1, 43 Vitest; livello 2, 40 Rust di `galleria-gui` (7 in `src/`,
  8 `commands`, 6 `config`, 9 `link`, 10 `supervisor`); livello 3, 5 WebDriver (`shows_the_real_daemon_state`,
  `relinks_after_a_daemon_restart`, `first_run_without_daemon`, `window_has_no_remote_content`,
  `copy_fills_the_system_clipboard`).
- `cargo deny check`: con cargo-deny 0.18.9 (la versione che `taiki-e/install-action` ha installato nel run 45633910830),
  0.20.2 e 0.22.1 passa. La 0.18.9 segnala come errore `RUSTSEC-2024-0429` (`glib` 0.18 sotto Tauri, `VariantStrIter`),
  le versioni più nuove no (guardano solo i crate del workspace); ignorato per identificativo con il motivo in
  `deny.toml`. Le versioni nuove avvisano che `RUSTSEC-2024-0370` «non è stato incontrato»: è un avviso, non un errore.
- `ui/`: `pnpm typecheck` ora controlla anche `ui/e2e/` e fa parte di `check-here`.

### Correzioni dopo la review di B1 (`t_5b760fb5`)

- **Permesso negato al secondo livello (G2).** Rosso prima del verde: `daemon_closing_right_after_accept_is_permission_denied`
  falliva al primo tentativo («Connection reset by peer», `DaemonNotRunning`); con `classify_hello` passa, e passa
  anche 5 volte di fila. Il demone vero chiude senza leggere `hello`: il client vede `BrokenPipe` o `ConnectionReset`
  (la review ne ha contati 191 e 9 su 200, nessun `Closed`).
- **Nessuna attesa dopo `subscribe`.** Rosso prima del verde: `daemon_closing_right_after_subscribe_is_not_hammered`
  contava 3731 tentativi in circa un secondo con la politica predefinita; con la correzione sono al più 5 (il primo
  ritentativo resta immediato, poi attese di 0,5 s, 1 s, 2 s). `tests/supervisor.rs` verde 10 volte di fila. Una
  connessione che dura quanto la prima attesa della politica o più azzera il conteggio. La vista resta «collegato»
  durante l'attesa che segue una connessione caduta subito dopo `subscribe`: l'evento `galleria:link` esce una volta
  per ciclo, non a ogni tentativo.
- **Risposta di `daemon_view` contro un evento arrivato nel frattempo (osservazione di Nello).** Riprodotta: due prove
  in `ui/src/daemon/store.test.ts` con una risposta lenta e un evento in mezzo, entrambe rosse («connected» al posto
  di «daemon_not_running»; stato «disconnected» al posto di «connecting»). `useDaemon` ora scarta lo snapshot se è
  arrivato un `galleria:link` mentre aspettava, e ne sovrascrive lo stato se è arrivato un `galleria:state`.

### Non verificato

- Il job `gui` e il job `gui-smoke` di `.github/workflows/ci.yml` non sono mai girati verdi su un runner: il `gui`
  è girato una volta (run 45633910830) ed era rosso solo per `cargo deny`, corretto dopo; il `gui-smoke` è girato una
  volta (run 45638906906, su `a37eed7`) ed era rosso per il nome del pacchetto `webkit2gtk-driver` (S6), quindi non è
  arrivato a `tauri-driver` né a WebKitWebDriver. Il job ora lancia `scripts/gui.sh smoke` nell'immagine: su un runner
  non è ancora girato, e vanno visti il build dell'immagine, l'utente del runner con il toolchain montato da
  `~/.rustup` e `~/.cargo/bin`, `cargo install tauri-driver` e WebKitWebDriver sotto Xvfb (nell'immagine funzionano
  con il sandbox WebKit spento).
- Il fumo gira contro `fake-daemon` (il vero `serve` con il backend finto), non contro `galleriad run` (S5):
  l'avvio reale del demone e la sua pulizia di rete non sono coperti.
- Permesso negato e protocollo incompatibile non hanno una prova a livello 3.
- Nessuna schermata è stata guardata in una finestra vera (le prove leggono il testo, non i pixel); axe senza la
  regola del contrasto (jsdom non ha colori calcolati) e nessuna prova di contrasto nella finestra.
- Clone pulito (`scripts/gui.sh check` da un clone) non provato.
- Le versioni dei pacchetti npm nuove (`webdriverio` 10.2.0, `@types/node` 26.6.10) sono sotto `minimumReleaseAge` di
  pnpm 12: `@types/node` è in `minimumReleaseAgeExclude`. Il comportamento di pnpm 12 su un runner pulito è da vedere.
