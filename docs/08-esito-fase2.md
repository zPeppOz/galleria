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
| S6 | Il fumo WebDriver gira nel container e sul runner | **Vero nel container; sul runner il primo tentativo è rosso, la correzione è da provare.** `scripts/gui.sh smoke` (build release dell'app + `fake-daemon` + `tauri-driver` 2.1.0 installato con `cargo install --locked` sotto `target/gui-tools` + `WebKitWebDriver` di `webkit2gtk-driver` + Xvfb + `dbus-run-session`) è passato 3 volte di fila, 5 prove su 5 ogni volta (circa 6 s). Servono `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` (il sandbox bubblewrap non parte in un container senza privilegi) e `WEBKIT_DISABLE_DMABUF_RENDERER=1`, impostate solo dallo script del fumo. **Sul runner, run 45638906906 (commit `a37eed7`):** il job `gui-smoke` è rosso, tutti gli altri verdi. `ubuntu-latest` è Ubuntu 26.04 e `apt-get install` esce con 100: «E: Package 'webkit2gtk-driver' has no installation candidate» (nella 26.04 il pacchetto si chiama `webkitgtk-webdriver`); il job installava i pacchetti sul runner invece di usare l'immagine. **Correzione:** il job lancia `scripts/gui.sh smoke`, cioè la stessa immagine del locale (`ubuntu:24.04`, dove `webkit2gtk-driver` esiste), con il toolchain Rust del runner montato in sola lettura; il `Dockerfile` ha un commento che dice che i nomi dei pacchetti sono quelli della 24.04. **Prova del ramo non rootless di `run_in_image`:** `scripts/gui.sh smoke` da una copia del repository con `docker info` finto (senza «rootless»), quindi `--user 1000:1000` come sul runner, in un utente senza voce in `/etc/passwd`, con `~/.rustup` e `~/.cargo/bin` montati in sola lettura, `cargo install tauri-driver` e l'intero fumo: 5 prove su 5, uscita 0. Il demone Docker era comunque rootless (gli uid del container sono mappati in sub-uid), la copia aveva i permessi aperti a tutti al posto del proprietario uguale all'utente del container: non è un runner. **Secondo tentativo, run 45645079856:** il job gira nell'immagine ma è rosso: dbus esce con «Could not get password database information for UID of current process: User "???" unknown» (codice 127). Il runner lancia il container con `--user 1001:1001` e l'uid 1001 non è nel `/etc/passwd` di `ubuntu:24.04`; la prova qui sopra non lo vedeva perché il demone locale è rootless e il container gira come root. **Correzione:** `run_in_image` dà al container un `passwd` e un `group` con una voce `gui` per l'utente con cui parte (home `/work/target/gui-home`), montati in sola lettura; l'utente resta senza privilegi. `GUI_UID` e `GUI_GID` forzano l'utente per provarlo in locale: con `GUI_UID=1001` `dbus-run-session` falliva con lo stesso errore, ora il fumo è verde (5 prove su 5), e lo è anche con l'utente di casa. `continue-on-error: true` è tolto in un commit separato; resta da vedere un run verde sul runner | T6 | libreria di prova: `webdriverio` da solo con Vitest (config in `ui/e2e/`), non il test runner `@wdio/cli`; due browser driver che `webdriverio` si porta dietro (`edgedriver`, `geckodriver`) hanno gli script di installazione negati in `pnpm-workspace.yaml` (`allowBuilds: false`): non si scarica nulla |
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

## B2 · Design system e guscio

Piano: `docs/superpowers/plans/2026-11-25-fase2-b2-design-system.md`. Q22–Q26 approvate da Aitano il 2026-11-25 su delega di
Peppe, tutte come raccomandate (segnate nella spec, §18). Ambiente: quello di B1, più nell'immagine `galleria-gui-dev` Orca
46.1, AT-SPI 2.52 e `python3-gi` (`lab/images/gui.Dockerfile`). `scripts/gui.sh a11y` lancia la verifica con lo screen reader
(locale, non in CI: Q22) e scrive `target/gui-a11y/` (`transcript.txt`, `orca-speech.log`, `orca-debug.log`).

### Punti [S] di B2

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S14 | Orca nell'immagine riceve il contenuto e gli eventi della webview | **Vero.** Con `dbus-run-session`, `xvfb-run`, `gsettings set org.gnome.desktop.interface toolkit-accessibility true`, `at-spi-bus-launcher` e Orca, `orca -l` elenca `galleria-gui`; l'albero AT-SPI della finestra contiene il documento web con landmark, titoli, pulsanti, menu e finestra modale; Orca pronuncia lo spostamento del fuoco dentro la pagina («Menu combo box. opens popup», «System selected radio menu item.») anche senza gestore di finestre. Per leggere il log mentre Orca gira serve un lanciatore (`lab/tests/orca-line-buffered.py`): Orca apre il file di debug con buffer a blocchi e le righe `SPEECH OUTPUT` arrivano in ritardo | T1 | nessun gestore di finestre (`openbox`): non è servito. Il lanciatore rende il file di debug a righe |
| S15 | Finestra modale e menu della libreria funzionano con la nostra CSP | **Vero.** Nessun evento `securitypolicyviolation` nella finestra vera mentre si usano menu e finestra modale (riga X2), con la CSP invariata (`default-src 'self'`, mai `unsafe-inline`: `tests/config.rs::csp_has_no_unsafe_inline`). Il `csp-provider` della libreria non è stato necessario; non ho cercato quali stili la libreria imposti, ho solo osservato che la finestra vera non ne vede violazioni | T1 | nessuno |
| S16 | `pnpm licenses list --prod --json` dà nome e licenza di ogni pacchetto di produzione | **Vero.** Oggetto con chiave la licenza e per valore un array di pacchetti con `name`, `versions`, `license` (pnpm 12.15.0). `scripts/check-js-licenses.mjs` lo legge, confronta con l'elenco `allow` di `deny.toml` (anche le espressioni `OR`/`AND`) e passa su 13 pacchetti di produzione; `scripts/tests/check-js-licenses.sh` lo prova con una licenza `GPL-2.0-only`, con una `AND`, e con una licenza assente | T1 | nessuno |
| S8 | `oklch()` e `color-mix()` disegnati giusti da WebKitGTK | **Vero** su WebKitGTK 2.52.6 (Ubuntu 24.04, l'immagine). `tokens_paint_the_expected_srgb` nella finestra vera, per i due temi: 15 colori dei token (riquadro con quello sfondo, fotografia della pagina, pixel in mezzo) e 5 tinte al 14 % (`color-mix(in oklch, var(--ok) var(--tint), transparent)` sopra `--panel`) coincidono con l'esadecimale atteso, **differenza massima 1,3 per canale** (tolleranza della prova: 3). WebKit calcola `oklch()` come `lab()` nello stile calcolato e lo disegna giusto. **Aperto** per Ubuntu 26.04 e Fedora: provato solo sull'immagine; resta aperto fino alla prima passata manuale su quelle distribuzioni (B6, con G10) | T2 | nessuno: i token restano `oklch()` |
| S17 | `system-ui` in WebKitGTK è il carattere del desktop | **Falso.** Nell'immagine, con `gtk-font-name=DejaVu Serif 11` in `~/.config/gtk-3.0/settings.ini` (GTK lo legge: `Gtk.Settings` restituisce «DejaVu Serif 11»), la larghezza di una frase a 40 px in `system-ui` resta quella del sans predefinito (817 px, uguale a «DejaVu Sans» e a `sans-serif`) e non diventa quella del serif (832 px); `fc-match system-ui` dà «DejaVu Sans». Misurato con una prova usa e getta, non committata | T2 | F3 applicato: `system-ui` tolto da `--font`, resta l'elenco del mockup (Segoe UI Variable Text, Segoe UI, Cantarell, Ubuntu, Noto Sans, `ui-sans-serif`, `sans-serif`). Su un desktop vero il carattere che esce dall'elenco non è stato guardato |

### Q12 · Libreria «headless»: tabella di accettazione (T1)

Libreria scelta: **`@base-ui/react` 1.9.0** (MIT). Non è scattata né la condizione di stop sulla libreria né quella sul canale
di verifica: tutte le righe sono verdi con Base UI, quindi **non si è passati a Radix**. Nessun file fuori da `ui/src/design/`
importa la libreria (`design/boundary.test.ts`).

(a) = Vitest in jsdom (`Dialog.test.tsx`, `Menu.test.tsx`); (b) = albero AT-SPI e (c) = trascrizione di Orca nella finestra vera
(`ui/e2e/overlays.a11y.ts`, `scripts/gui.sh a11y`). Parole di Orca copiate da `orca-speech.log`.

| # | Comportamento | (a) | (b) AT-SPI | (c) Orca |
|---|---|---|---|---|
| D1 | Il fuoco entra nella finestra, ruolo `dialog` con il titolo come nome | verde | `dialog «About Galleria»`, il pulsante «Close» ha `focused` | «About Galleria dialog A VPN client for WireGuard and OpenVPN.», «Close push button.» |
| D2 | `Tab`/`Shift+Tab` non escono; dietro non è raggiungibile | verde | 6 pressioni: il fuoco resta nel `dialog`; i pulsanti «Copy» e «Try again» della pagina dietro non sono nell'albero | — |
| D3 | La descrizione è collegata | verde | `description=«A VPN client for WireGuard and OpenVPN.»`, relazione `described-by` | pronunciata insieme al titolo (D1) |
| D4 | `Esc` chiude; il fuoco torna al pulsante «Menu» | verde (Esc e pulsante di chiusura) | `dialog` sparito, «Menu» ha `focused` | «Menu combo box.» «opens popup» |
| M1 | Il pulsante dichiara il menu | verde (`aria-haspopup`, `aria-expanded`) | ruolo **`combo box`**, stato `has-popup`; **`expanded` non viene esportato** (vedi sotto); relazione `controller-for` quando è aperto | «Menu combo box.» «opens popup» |
| M2 | `Invio` apre; il fuoco va sulla prima voce; ruolo `menu` con nome | verde (`Invio`, `Spazio`, `↓`) | `menu «Menu»` con relazione `controlled-by`, «System» ha `focused`; `aria-expanded="true"` letto dal DOM con WebDriver | «Menu menu», «Language panel.», «System selected radio menu item.» |
| M3 | `↓` `↑` `Home` `End` e la lettera muovono il fuoco | verde | dopo ogni tasto la voce attesa ha `focused` (English, About Galleria, System, About Galleria con `a`) | una frase per voce («English not selected radio menu item.», «About Galleria.») |
| M4 | La scelta singola espone il suo stato e cambia con `Invio` | verde | `radio menu item` ×3, `checked` su una sola voce; dopo `Invio` su «English» il segno passa a «English» | «System selected radio menu item.» / «English not selected radio menu item.» |
| M5 | `Esc` chiude e riporta il fuoco; una voce scelta chiude | verde | menu sparito, «Menu» ha `focused` | «Menu combo box.» |
| X1 | axe senza violazioni con finestra e menu aperti | verde (regola dei landmark spenta: sono componenti in un portale, il guscio la tiene accesa) | — | — |
| X2 | Nessuna violazione della CSP | — | `no_csp_violation_while_using_dialog_and_menu` (ascolta `securitypolicyviolation`) verde | — |

**Divergenza dal piano, M1/M2.** Il piano si aspettava ruolo `push button` e stato `expanded` sul pulsante. WebKitGTK 2.52.6
espone un `<button aria-haspopup="menu">` come **`combo box`** (stato `has-popup`) e non esporta `expanded`. Non è Base UI:
la prova di controllo `control_how_webkitgtk_exposes_a_plain_popup_button` inserisce nella pagina un `<button
aria-haspopup="menu" aria-expanded="true">` nudo e ottiene la stessa cosa (`combo box`, `has-popup`, nessun `expanded`).
Per questo la riga M1/M2 verifica `aria-expanded` nel DOM (dove lo imposta la libreria) e, nell'albero, `has-popup` e le
relazioni `controller-for`/`controlled-by` tra pulsante e menu; Orca dice «opens popup» e non «collapsed/expanded». Non è
una riga bocciata: il difetto sta nell'esportazione di WebKitGTK, la stessa che avrebbe Radix.

Altre osservazioni: (1) il primo `Tab` sul menu e sul resto dell'app funziona anche senza gestore di finestre; (2) un
gruppo del menu è esposto come `panel` con nome («Language panel.») e non come `group`: così lo esporta WebKitGTK per un
`role="group"` con etichetta; (3) il gruppo a scelta singola in jsdom: `Menu.RadioItem` di Base UI **non chiude** il menu al
clic, di predefinito; `closeOnClick` (proprietà documentata) lo fa chiudere come le altre voci, come chiede M5.

### Verifiche del T1

- Rosso prima del verde: `scripts/tests/check-js-licenses.sh` senza lo script («No such file or directory»); `Dialog` e
  `Menu` e le loro prove sono stati scritti insieme (il primo giro di Vitest: 4 prove su 18 rosse, tutte per difetti della
  prova: `Tab` che passa un istante dalla guardia di fuoco della libreria, un tasto premuto prima che il menu fosse
  aperto, la regola axe dei landmark su un componente solo, `closeOnClick` della voce a scelta singola).
- `scripts/gui.sh check`: 65 prove Vitest, 7 prove di `tests/config.rs`, controllo delle licenze; `scripts/gui.sh smoke`: le
  cinque prove di B1 (il fumo ora usa `ui/e2e/support.ts` e `lab/tests/gui-lib.sh`, condiviso con `gui-a11y.sh`);
  `scripts/gui.sh a11y`: 11 prove, verdi.

### Token e temi (T2)

Q23 applicata per intero: `ui/src/design/tokens.css` ha i due temi con le correzioni C1–C7, la scala dei raggi (R1), degli spazi
(P1), dei caratteri (F1, F2, F4) e `--font` senza `system-ui` (F3, vedi S17). Il tema è sempre scritto sulla radice come
`data-theme` (`theme.ts`), prima del montaggio di React (`main.tsx`); in B2 non c'è salvataggio (Q25).

Nota sul tema del sistema: in questa immagine WebKitGTK dichiara `prefers-color-scheme: light` (nessun desktop che chieda lo
scuro), quindi con la scelta «Sistema» l'app parte chiara. Come lo riporti un desktop vero resta da vedere (non verificato).

Scoperta sulla finestra vera: `takeScreenshot` di un elemento, con WebKitWebDriver, restituisce l'**intera pagina** (1180×740 per
un riquadro di 60×60). La prova S8 fotografa la finestra e legge il pixel in mezzo a un riquadro messo in alto a sinistra.

### Verifiche del T2

- Rosso prima del verde, con i valori del mockup così come sono in `tokens.css`: 13 prove su 42 rosse. Tema chiaro: `--faint`
  2,47; `--ok`, `--warn`, `--err`, `--wg`, `--ovpn` come testo 1,61–2,68 (la prova ne cita uno per volta: `--ok` 1,73, `--warn`
  1,61, `--err` 2,68, `--wg` 1,88, `--ovpn` 1,98 su `--bg`); `--accent-ink` bianco su `--accent` 3,83; sigle del protocollo sulla
  loro tinta 1,88 e 1,98; tema scuro: `--faint` 3,51 su `--bg`; più le due prove di `css-rules` sui valori scritti a mano di
  `styles.css`. Con i valori della proposta: 107 prove Vitest verdi.
- I dodici valori `oklch()` della tabella del piano coincidono con la conversione scritta in `color.ts` a ±1 per canale.
- `scripts/gui.sh smoke`: 7 prove su 7 (le cinque di B1 e le due nuove), `S8: 20 colori … differenza massima 1,3`.

### Cataloghi e lingua (T3)

I testi stanno solo in `ui/src/i18n/{en,it}.json` (74 chiavi ciascuno); `messages.ts` e `i18n.tsx` non esistono più. `i18n/index.tsx`
dà `useT`, `detectLang`, `hasKey` e `errorText` (D22: il testo del catalogo per il codice, il messaggio inglese del demone per un
codice che il catalogo non conosce). `prefs.tsx` tiene lingua e tema **solo in memoria** (Q25), scrive `lang` e `data-theme`
sulla radice e rende il fornitore della lingua: cambiando lingua dal menu si ridisegna tutto senza smontare `App`. `AppMenu` ha
il gruppo «Tema» (Sistema · Chiaro · Scuro) accanto a «Lingua».

- G8: `crates/galleria-gui/tests/catalogs.rs` legge i due cataloghi e chiede `error.<codice>` per le 11 varianti di `ErrorCode`,
  le 8 di `SessionErrorCode` e le voci di `GUI_CODES` (oggi vuoto), `state.<s>` per i 7 stati e `bar.<l>` per i 5 stati del
  collegamento. Ogni enum ha un elenco e, generato dalla stessa macro, un `match` senza ramo di ripiego: una variante nuova non
  compila finché non è nell'elenco, e nell'elenco senza testo la prova diventa rossa. Un'altra prova cerca nei sorgenti di
  `galleria-gui/src` i letterali `gui_…` e `import_…` e pretende che stiano in `GUI_CODES`; una terza scarta le chiavi `error.*`
  senza codice. Le frasi partono dalla tabella della CLI (`galleria-cli/src/messages.rs`): «demone» → «servizio» come nel resto
  dell'interfaccia, e `already_active` senza `--replace` (la GUI non ha quel parametro).
- Provato a mano: tolta `error.auth_failed` da `it.json`, `every_session_error_code_has_text_in_both_catalogues` diventa rossa
  («missing in it»); rimessa, torna verde.
- Fumo: `language_switches_without_a_restart` (dal menu «Italiano» compare «Avvia il servizio», «English» → «Start the
  service», stessa sessione WebDriver). `scripts/gui.sh smoke`: 8 prove su 8.
- `overlays.a11y.ts` (T1) aggiornata: il menu ha due gruppi a scelta singola, i radio sono sei e i segnati due. `a11y`: 11 su 11.

### Controlli di base (T4)

`ui/src/design/{icons,Button,Field,Switch,Segmented,Tabs}.tsx`, ciascuno con il suo `.css` (solo token) e la sua `.test.tsx`;
nove icone nostre (`more`, `close`, `check`, `warning`, `error`, `info`, `star`, `chevron`, `copy`), nascoste alla tecnologia
assistiva. Nessun componente contiene un testo visibile: arrivano tutti come proprietà.

- Dietro: HTML nativo per `Button` (`<button type="button">`; senza testo il tipo pretende `aria-label`, e la prova lo verifica
  con `@ts-expect-error`), `Field` (`<label for>`, `aria-describedby`, `aria-invalid`; `select` nativo, in sola lettura mostra
  solo la voce scelta), `Switch` (`<button role="switch">` dentro una `<label>`: Spazio, Invio e clic sull'etichetta) e
  `Segmented` (radio nativi: frecce e una sola sosta del Tab gratis, `aria-readonly` + cambio ignorato in sola lettura);
  `@base-ui/react/tabs` per `Tabs` (frecce, Home, End e selezione al fuoco; solo il pannello scelto è disegnato).
- Per `Tabs` Base UI rende il pannello stesso raggiungibile con Tab (`tabindex="0"`), come nel modello WAI-ARIA: dalla lista il
  Tab va al pannello e poi al suo contenuto; la prova `tablist_is_one_tab_stop` lo dice così.
- L'indicatore della scheda (`Tabs.Indicator`) imposta le sue misure con `style` da script, non con un `<style>` né con lo
  script di pre-idratazione (`renderBeforeHydration` resta spento): non dovrebbe violare la CSP. **Non verificato nella finestra
  vera**: il componente non è ancora usato dall'app; lo si prova al T7 con il guscio (`no_csp_violation…` esteso).
- Bersagli: ogni controllo ha almeno 24 × 24 px (WCAG 2.5.8); l'interruttore è 38 × 22 e la sua `<label>` ne copre l'area (24 di
  altezza minima).
- Colore: il pulsante primario usa `--accent` solo come riempimento (`--accent-ink` per il testo), il pericolo `--err` come testo,
  il bordo dei controlli `--line-strong`: `css-rules.test.ts` verde sui file nuovi.
- Verifiche: 155 prove Vitest (T3: 121), tutte verdi; `scripts/gui.sh check` verde. Axe passa su ogni componente in jsdom, il
  contrasto non lo vede (jsdom): resta alla prova sui token (T2). `smoke` e `a11y` non rilanciati dopo T4: i componenti non
  sono ancora nel bundle dell'app.

### Avvisi, conferma e schermate di B1 (T5)

`design/{Toast,InlineAlert,ConfirmDialog}.tsx` con i loro `.css` e prove. Le schermate `FirstRun` e `StatusView` passano ai
componenti e hanno un `.css` loro accanto (`screens/*.css`): nessun file di `screens/` usa più una classe di `styles.css`, che
tiene ora solo la cornice di B1 (titolo, contenuto, barra) fino al T6.

- `ToastProvider` tiene **due regioni vive sempre presenti e vuote** (`status` per l'informazione, `alert` per l'errore): un
  lettore di schermo annuncia solo ciò che si aggiunge a una regione che già conosce. L'avviso non prende mai il fuoco; resta
  sotto il puntatore o con il fuoco dentro (5 s, 8 s per l'errore, a tempo finto nella prova). Scritto da noi, senza il
  `Toast` di Base UI: il comportamento che serve è piccolo e si prova per intero in jsdom.
- `InlineAlert` ha una proprietà in più rispetto al piano, `toneLabel` (la parola del tono: «Errore», «Attenzione»…): è il
  nome accessibile dell'icona, cioè ciò che impedisce al tono di stare nel solo colore. Testi nuovi nei cataloghi:
  `alert.info|ok|warn|error`, `toast.close`. Solo l'errore è `alert`.
- `ConfirmDialog` sta su `AlertDialog` di Base UI (ruolo `alertdialog`, nome dal titolo, descrizione dal testo): con
  `danger` il fuoco parte da «Annulla» (`initialFocus`), altrimenti dall'azione; `Esc` annulla. I due pulsanti sono `<button>`
  con le classi di `Button`, perché il fuoco si assegna per riferimento.
- Stile di finestra e menu: erano già scritti con i soli token (`--r-lg`, `--shadow-lg`, `--line-strong`, `--scrim`; menu
  `--r-lg` e `--shadow-md`); in T5 resta confermato dall'`a11y` verde e da `css-rules.test.ts`. `ConfirmDialog` usa lo stesso.
- `FirstRun`: il comando in un blocco a carattere fisso, «Copia» con l'icona, «Riprova» primario; il testo di «protocollo
  incompatibile» e il promemoria di «permesso negato» stanno in `InlineAlert`. `StatusView`: errore in `InlineAlert` (`alert`),
  backend non disponibili in un `InlineAlert` di avviso con la stessa lista (la prova di B1 conta ancora un `listitem`).
  Testi, ruoli e prove di B1 invariati.
- Verifiche: 185 prove Vitest verdi; mutazione provata (tempo di permanenza e pausa tolti → `toast_leaves_by_itself…` rossa);
  `scripts/gui.sh smoke` 8/8 (S8: 1,30 per canale), `a11y` 11/11, gate dell'host verdi.

### Guscio con lista e dettaglio (T6)

`ui/src/shell/`: `Shell` (cornice: barra in alto, lista a sinistra di 300 px, contenuto, barra di stato; sotto 860 px una
colonna con la lista sopra), `TopBar`, `ProfileList`, `ProfileDetail`, `ServiceArea`, `StatusBar`, `ProtocolTag`, `demo-data.ts`,
ciascuno con il suo `.css`. `ui/src/styles.css` non esiste più e `css-rules.test.ts` non ha più eccezioni. `App.tsx` ora è
soltanto il collegamento al demone (`useDaemon`) dentro `Shell`; `Shell` tiene il `ToastProvider`.

- **Dati finti**: cinque profili del mockup su domini `example` (`DemoProfile[]`, nessun segreto), gruppi con titolo dal catalogo
  (`group.work|personal|clients`) per cambiare lingua a caldo. Il segno «Dati di esempio» sta nella barra in alto, `--ink` su
  tinta di `--warn` con bordo `--warn`. `shell/boundary.test.ts`: nessun `invoke(`, nessun import di `@tauri-apps` o di
  `daemon/store` nei sorgenti del guscio.
- **Lista**: `listbox` con un `group` (nome = titolo) per gruppo e righe `option` con **tabindex a rotazione**: un solo punto di
  sosta (la riga scelta), frecce, Home, End e la prima lettera spostano fuoco e scelta insieme. Nome della riga = nome del
  profilo; server, protocollo («WireGuard»/«OpenVPN» in testo nascosto accanto alla sigla) e «Preferito» stanno nella
  descrizione (`aria-describedby`). La selezione si vede con un riempimento e una barra `--accent` (mai testo).
- **Dettaglio**: nome come unico `h1` del documento; «Altre azioni» (Duplica · Esporta · Elimina) con testo visibile; striscia di
  quattro celle «—» (con «non disponibile» per lo screen reader); cinque schede; «Generale» con `Field`, `Field as="select"`,
  `Segmented` di sola lettura (solo OpenVPN) e `Switch`. «Elimina» apre `ConfirmDialog` (fuoco su «Annulla»), alla conferma
  l'avviso `demo.no_change`; «Duplica» ed «Esporta» mostrano lo stesso avviso. I profili restano cinque.
- **ServiceArea (Q26)**: senza collegamento il pannello di `FirstRun`; prima della prima risposta un pannello «Collegamento al
  servizio»; con collegamento gli avvisi dei backend (`BackendWarnings`, estratto da `StatusView`) e, con una sessione non
  `disconnected`, `StatusView` (che porta già gli avvisi: mostrati una volta sola). La lista resta sempre usabile.
- **StatusBar**: a sinistra lo stato vero (`state.<s>`, o «Nessuno stato della connessione» senza sessione), a destra il collegamento
  (`bar.*`) e la versione dell'app; `aria-live="polite"`. Il fumo di B1 cerca ancora «Disconnected» e «Connected to the service».
- **Ordine del `Tab`** (DOM): «Menu» → lista → pannello di primo avvio («Copia», «Riprova») → «Altre azioni» → schede → primo
  controllo della scheda. Il T7 lo fissa con `keyboard.test.tsx`.
- Il piano non aveva il campo di ricerca (B3) e non lo ho messo. `daemon/protocol.ts` raccoglie i nomi dei protocolli (nomi propri,
  uguali in ogni lingua, fuori dai cataloghi).
- Verifiche: 238 prove Vitest (T5: 185) e 7 di `tests/catalogs.rs` verdi; `scripts/gui.sh check`, `smoke` 8/8 (S8: 1,30),
  `a11y` 11/11 e i gate dell'host verdi. **Guardato in una finestra vera** (fotografie di WebKitGTK sotto Xvfb, tema chiaro e scuro,
  primo avvio, menu aperto): lista, dettaglio, striscia e schede si disegnano senza sovrapposizioni; la scheda «Trasporto»
  non aveva etichetta visibile (aggiunta). Le fotografie non sono nel repo. Non guardato: sotto 860 px, la scheda «Log» e la
  finestra di conferma nella finestra vera.
