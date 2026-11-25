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

### Tastiera, fumo, screen reader (T7)

`ui/src/shell/keyboard.test.tsx` (9 prove, mai il puntatore), `ui/e2e/smoke.e2e.ts` (5 prove nuove nel blocco «the shell in the
real window») e `ui/e2e/shell.a11y.ts` (11 prove con Orca e AT-SPI: 8 del T7, `S4b` e due prove di controllo, aggiunte dopo la
review di B2). Due difetti trovati dalle prove e corretti nei componenti:

- **Il fuoco finiva sul `body` chiudendo un avviso dalla tastiera.** Il pulsante «Chiudi l'avviso» ha il fuoco, l'avviso sparisce e il
  fuoco cade. Ora `Toast` ricorda l'elemento che aveva il fuoco prima di entrare nell'avviso e glielo restituisce (se non c'è più,
  il primo punto di sosta della pagina fuori dagli avvisi). Prove: `a_notice_closed_from_the_keyboard_does_not_drop_the_focus_on_the_body`
  (guscio) e `toast_can_be_dismissed_from_the_keyboard` (esteso).
- **Le prove di tastiera sul menu erano instabili**: un tasto premuto appena il menu compare va al pulsante, perché il fuoco entra nel
  menu un istante dopo. Le prove aspettano il fuoco dentro il menu (`menuOpen`). Non è un difetto del componente (un utente non preme
  due tasti nello stesso millisecondo), ma è bene saperlo per le prove che verranno.

Le regioni vive dei toast portano ora anche `aria-live` e `aria-atomic` scritti (WebKitGTK esporta così la cortesia della regione,
`container-live:polite`); le prove di T5 non cambiano.

#### Criteri di uscita di B2

| # | Criterio | Prova | Esito |
|---|---|---|---|
| U1 | Guscio navigabile con dati finti, nelle due lingue | fumo nella finestra vera: `shell_is_navigable_with_sample_data` (cinque profili, «Sample data», `↓`/`End`/`Home` nella lista cambiano il titolo, frecce fino a «Log»), `shell_works_in_both_languages` («Dati di esempio», «Generale», «Altre azioni», ritorno a «English»), `both_themes_render`; in jsdom `Shell.test.tsx` | **verde** (13/13 al fumo, tre volte di fila) |
| G8 | Ogni codice d'errore del demone ha testo `en` e `it`; la lingua cambia senza riavvio | `tests/catalogs.rs` (7 prove: 11 `ErrorCode`, 8 `SessionErrorCode`, `GUI_CODES`, stati e barra); `prefs.test.tsx::switching_language_changes_every_visible_text`; fumo `language_switches_without_a_restart` e `shell_works_in_both_languages` (stessa sessione WebDriver) | **verde** |
| G12 (B2) | Guscio e componenti da tastiera; ruoli giusti per finestra modale, menu, schede, lista e interruttore | axe e tastiera per ogni componente (T1, T4–T6) e `keyboard.test.tsx` sul guscio; ruoli e parole nella finestra vera con Orca e AT-SPI (`overlays.a11y.ts`, `shell.a11y.ts`) | **verde, con due buchi misurati nell'immagine**: Orca 46.1 non pronuncia il nome di un campo di testo e non pronuncia nessuna regione viva, quindi nemmeno l'avviso a scomparsa (sotto). Tutto il resto è verde; il giudizio è della piattaforma (Orca con WebKitGTK), il nostro albero AT-SPI è giusto. **Correzione dopo lo spike (`t_b81c3909`, sezione «Spike Orca»)**: su GNOME vero (Orca 50.3 e 51.1) il nome dei campi **si sente**, quindi il buco 1 esiste solo nell'immagine; il buco 2 è confermato e ha una strada decisa (Q27) |
| Q12 | Libreria «headless» regge finestra modale e menu con lo screen reader | tabella del T1 | **verde con Base UI 1.9.0** (rimasto invariato) |
| S8 | `oklch()` e `color-mix()` giusti da WebKitGTK | `tokens_paint_the_expected_srgb`; in più `both_themes_render` (pixel dello sfondo del guscio contro `--bg` nei due temi) | **vero** su WebKitGTK 2.52.6 (differenza massima 1,30 per canale); aperto su Ubuntu 26.04 e Fedora |
| CSP | Nessuna violazione con il guscio | `no_csp_violation_during_the_tour` (lista, schede, interruttore, menu, finestra, conferma, avviso, lingua e tema) e `X3` nel giro con Orca; la prova di controllo `control_the_csp_listener_sees_a_violation` inserisce un `<style>` e il listener lo sente | **verde**: anche l'indicatore delle schede (`Tabs.Indicator`, misure con `style` da script) non viola la CSP |

#### Cosa dice Orca sul guscio (`SPEECH OUTPUT`, così come sono)

Ambiente: Orca 46.1, AT-SPI 2.52, WebKitGTK 2.52.6, nell'immagine `galleria-gui-dev`, senza gestore di finestre; lo scarico è
`target/gui-a11y/transcript.txt` (ignorato da git), identico in due giri di fila.

| Passo | Parole di Orca |
|---|---|
| `Tab`, `Tab` dal pulsante «Menu» alla lista | «landmark Profile list» · «Connections list box» · «Ufficio Napoli.» · «Favourite vpn.ufficio.example:1194 OpenVPN.» |
| `↓` nella lista | «Datacenter Milano.» · «gw.dc-mi.example:443 OpenVPN.» |
| `Tab` `Tab` alle schede | «General page tab.» |
| `→` sulle schede | «Authentication page tab.» (`selected`, `focused`; cambia anche il pannello) |
| `Tab` fino all'interruttore | «General scroll pane.» · «Datacenter Milano» «selected» (il campo «Nome»: **il nome non c'è**, solo il valore, buco 1 sotto: solo nell'immagine) · «Group combo box.» «Work.» · «gw.dc-mi.example:443» «selected» (il campo «Server», di nuovo senza nome) · «Transport panel.» «TCP.» «selected radio button» «leaving panel.» · «Reconnect automatically toggle button pressed.» · «Try again by itself when the connection drops.» |
| `Spazio` sull'interruttore (spento, poi acceso) | «not pressed» · «pressed» (Orca dice solo il nuovo stato) |
| «Elimina» dal menu «Altre azioni» | «alert Delete Datacenter Milano?» · «These are sample data: nothing is deleted, whatever you choose.» · «Cancel push button.» |
| `Esc` sulla conferma | «More actions combo box.» «opens popup» (il fuoco è tornato sul pulsante) |
| «Duplica» (avviso «Sample data: nothing was changed») | **niente**: Orca dice solo «More actions combo box.» «opens popup» (il ritorno del fuoco); buco 2 sotto |

Albero AT-SPI dei punti, con la stessa prova (righe `ruolo «nome» [stati]`):

```
landmark «Profile list»
  list box «Connections»
    panel «Work»
      list item «Ufficio Napoli» [focusable,focused,selected,…] description=«Favourite vpn.ufficio.example:1194 OpenVPN»
      list item «Datacenter Milano» [focusable,…]      description=«gw.dc-mi.example:443 OpenVPN»
    panel «Personal»  (Homelab, VPS Hetzner)      panel «Clients»  (Cliente Rossi)
page tab list «Profile sections»
  page tab «General» [focused,selectable,selected,…] rel=label-for->General,controller-for->General
  page tab «Authentication» · «Network» · «Configuration» · «Log»
toggle button «Reconnect automatically» [checkable,checked,focused,…] description=«Try again by itself when the connection drops.»
alert «Delete Datacenter Milano?» [focusable,…] description=«These are sample data: nothing is deleted, whatever you choose.»
  heading «Delete Datacenter Milano?»   paragraph «»
  push button «Cancel» [focused]   push button «Delete»
status bar «»   notification «»      (le due regioni vive, vuote)
```

Cose da sapere, tutte di WebKitGTK o di Orca e non del nostro codice:

- **L'interruttore (`role="switch"`) è un `toggle button`**: Orca dice «pressed/not pressed», non «on/off». Il nome e il testo di aiuto
  si sentono. Non l'ho cambiato: un `<button role="switch">` è la forma giusta e l'esportazione è della piattaforma.
- **I gruppi della lista sono `panel`** con il nome del gruppo (come il gruppo del menu in T1), non `group`.
- **Buco 1 · Il nome di un campo di testo NON è pronunciato, ma solo nell'immagine (Orca 46.1).** Su GNOME vero, Ubuntu 26.04
  con Orca 50.3 e Fedora 45 con Orca 51.1, Orca dice «Name» «entry» e poi il valore, con `Tab` e con `Shift+Tab` (sezione
  «Spike Orca», punto 2): non è un difetto del prodotto né un limite da dichiarare per quelle due distribuzioni. Resta
  non provato Orca 46 su un desktop vero (Ubuntu 24.04, Q28 del piano di B3). Quello che segue è la misura nell'immagine,
  e lì resta vera. Con `Tab` su «Name» Orca dice «General scroll pane.» «Datacenter
  Milano» «selected»: il valore e basta; con `Shift+Tab` nulla. L'albero AT-SPI è giusto (`entry «Name»`, `labelled-by`, `S4b` lo
  asserisce) e non dipende dal nostro markup: la prova di controllo `control_orca_does_not_say_the_name_of_a_bare_text_field`
  mette nella pagina campi nudi, senza niente di nostro, e li raggiunge con `Tab` da un pulsante. Con un valore dentro Orca dice solo
  il valore e «selected» («Some value» «selected»; la password «10 • characters» «selected»; la `textarea` nulla) per otto forme:
  `aria-label`, `<label for>`, etichetta che avvolge, `aria-labelledby`, `textarea`, `readonly`, `password`, `aria-describedby`.
  Il nome si è sentito solo per un campo **vuoto** con `aria-label` al primo arrivo («Probe alpha empty entry.» nella review,
  «Probe alpha entry.» in un giro di questa correzione, niente in un altro: la prova lo registra e non lo asserisce). Nel log di
  Orca l'evento `text-caret-moved` arriva prima di `state-changed:focused` e il fuoco non viene più presentato. `S4b_text_field_name_is_spoken_by_orca`
  asserisce il fuoco e l'albero come al solito e rovescia solo l'attesa del parlato (`rejects.toThrow`, «buco noto»): il giorno
  in cui Orca dice «Name» la prova diventa rossa. Oltre al nome, la trascrizione di sopra mostrava già il buco («Datacenter
  Milano» «selected») senza segnarlo.
- **Buco 2 · Avviso a scomparsa: NESSUNA regione viva è pronunciata.** L'avviso compare nell'albero (`status bar` con il pulsante
  «Dismiss the notice», il fuoco resta su «More actions»: la metà «senza spostare il fuoco» è verde), Orca riceve
  `object:children-changed:add` sulla regione (la cortesia è esportata, `container-live:polite`, `container-atomic:true`) e lo
  script `WebKitGtk` di Orca 46 non lo presenta. Non dipende dal nostro markup: la prova di controllo
  `control_orca_does_not_say_any_form_of_live_region` inserisce nella pagina dieci forme e Orca le tace tutte, **`role="alert"`
  compreso**: `role=alert` già presente e poi `textContent`; `role=alert` nuovo con testo; `role=alert` nuovo con testo e
  pulsante; `role=alert` vuoto e testo dopo 300 ms; `aria-live=assertive` e `textContent`; `role=status` e `<p>`; `role=status`
  con `aria-live=polite` e testo; `aria-live=polite` e modifica del testo di uno `span` già presente; `role=log` e `<p>`;
  `role=status` con `aria-atomic` e un `div` con testo e pulsante. Quindi vale per **ogni `aria-live`** (compresa la barra di
  stato di B1), non solo per l'avviso. La prova finisce con un pulsante nudo che prende il fuoco e che Orca pronuncia, perché il
  silenzio non sia quello di un Orca morto. `S6b_notice_is_spoken_by_orca` asserisce l'apertura del menu e la comparsa dell'avviso
  come al solito e rovescia solo l'attesa del parlato: il giorno in cui Orca parla diventa rossa.
  **La frase della versione precedente («in B5 la strada da provare è il ruolo `alert`: Orca pronuncia gli `alert` che
  compaiono, lo fa per la conferma») era sbagliata**: la conferma si sente perché il fuoco ci entra, non perché sia un `alert`.
  **Conseguenza**: oggi l'unico avviso è «Dati di esempio: nessuna modifica» (informazione, non errore), ma da B3 in poi gli esiti
  di salvataggio ed eliminazione, e da B5 i cambi di stato della connessione, passano da regioni vive. Con questa piattaforma nessuna
  correzione di markup basta: come far arrivare a Orca un esito è una decisione da prendere prima del piano di B3 (non qui), da
  confermare su un desktop vero (sotto, «Non verificato»). **Deciso dopo lo spike: Q27** (sezione «Spike Orca»): il buco è
  confermato su GNOME vero e gli esiti passano dal segnale ATK emesso dal lato Rust, a partire da B3.

#### Numeri per livello

- Livello 1 (Vitest, jsdom): **249 prove in 28 file** (T6: 238; +9 di `keyboard.test.tsx`, più un'asserzione in `Toast.test.tsx`; +2 di `css-rules.test.ts` dopo la review: gli spazi e la prova della regola).
  Rust: `scripts/gui.sh check` verde (7 di `tests/catalogs.rs`, `config.rs`, clippy e test di `galleria-gui`).
- Livello 3, fumo: **13 prove** (5 di B1, `language_switches_without_a_restart`, 2 dei token, 5 nuove del guscio: tre prove,
  la prova di controllo del listener della CSP e il giro con la CSP). Verde tre volte di fila; S8: differenza massima 1,30.
- Livello 3, screen reader (`scripts/gui.sh a11y`): **22 prove** (11 di T1 con la prova di controllo del menu, 11 di `shell.a11y.ts`:
  8 del T7, `S4b` e le due prove di controllo dei buchi), tutte verdi; nessun `it.fails`: i due buchi sono prove che aspettano il
  silenzio di Orca e diventano rosse quando parla. Verde due volte di fila (i numeri sono di questa correzione, fatti dopo la review).

#### Non verificato

- **S8 su Ubuntu 26.04 e Fedora**: solo sull'immagine (Ubuntu 24.04, WebKitGTK 2.52.6). Resta a B6 (G10).
- **La passata a orecchio (Q22)**: Orca ha girato senza scheda audio, con il testo che pronuncia scritto nel file di debug; nessuno
  ha ascoltato la voce né provato con un lettore reale in uso quotidiano.
- **Il tema del sistema su un desktop vero**: nell'immagine WebKitGTK dichiara `prefers-color-scheme: light` e basta.
- **L'aspetto a occhio in una finestra** al di là delle fotografie del T6: sotto 860 px, la scheda «Log», la finestra di conferma
  non guardate; il carattere che esce dall'elenco su un desktop vero (S17).
- **I due buchi di Orca su un desktop vero** *(chiuso dallo spike `t_b81c3909`, sezione «Spike Orca»: le regioni vive sono mute
  anche su GNOME vero, il nome dei campi invece si sente; la strada degli annunci è Q27. Il testo che segue è quello di
  prima dello spike)*: misurati solo nell'immagine, senza gestore di finestre e senza audio (Orca 46.1,
  WebKitGTK 2.52.6). Non so se una sessione GNOME vera (Ubuntu 26.04, Fedora 45) li riproduca; la decisione se anticipare la
  passata di Q22 sulle VM di S14/S15 è di Aitano e Peppe. Non so nemmeno se dipendano dallo script `WebKitGtk` di Orca o da
  WebKitGTK. Come far arrivare gli esiti a Orca (per esempio il segnale AT-SPI `announcement` dal lato Rust, le notifiche del
  desktop, o un limite dichiarato) non è stato provato.
- **Il nome di un campo vuoto con `aria-label`**: si sente in alcuni giri e in altri no; la prova lo registra senza asserirlo.
- Altri lettori (NVDA e Narrator sono Windows, fase 3).
- Un clone pulito (`scripts/gui.sh check` da un clone) non provato, come in B1.

## Spike Orca · regioni vive e campi di testo in una sessione GNOME vera

Card `t_b81c3909`, richiesta dalla review di B2 (`t_5e8d80a7`): i due buchi di Orca con WebKitGTK erano misurati solo
nell'immagine Docker (Xvfb, nessun gestore di finestre, nessun audio). Qui sono ripetuti su due desktop veri, con l'app di
B2 a `93c309a` (nessun codice di prodotto toccato; la prova dentro il processo è un patch in `lab/vm/`, non applicato).
Anticipa una parte della passata di Q22 (prevista in B6): è la sola parte fatta, **senza audio** (Orca scrive il testo
che pronuncia nel file di debug, come nell'immagine; nessuno ha ascoltato la voce).

### Ambiente

| | Immagine Docker (riferimento, dalla review) | VM Ubuntu 26.04.1 | VM Fedora 45 |
| --- | --- | --- | --- |
| Sessione | Xvfb, nessun gestore di finestre | **GNOME Shell 50.1, Wayland**, login automatico GDM | **GNOME Shell 51.1** (mutter 51.1), **Wayland**, login automatico GDM |
| Orca | 46.1 | **50.3** | **51.1** |
| WebKitGTK | 2.52.6 | 2.52.6 (`libwebkit2gtk-4.1-0 2.52.6-0ubuntu0.26.04.2`) | **2.54.2** (`webkit2gtk4.1`) |
| at-spi2-core | 2.52.0 | 2.60.4 | 2.62.1 |
| GTK | 3.24 | 3.24.52 | 3.24.52 |
| Kernel | – | 7.0.0-43 | 7.2.11 |

Stesso binario in tutte e due le VM: `galleria-gui` (release) e `fake-daemon` compilati nell'immagine `galleria-gui-dev`
da un `git archive 93c309a` in `/mnt/b/vm/build/src` (`scripts/gui.sh build`, poi `cargo build -p galleria-gui --example
fake-daemon`); `ldd` non trova librerie mancanti né su Ubuntu né su Fedora. Il ramo `t_43cd01da` non era chiuso.

### Come è stato fatto

Tutto in `/mnt/b/vm` (dischi, build, log, fotografie in `/mnt/b/vm/spike/`). Script nuovi in `lab/vm/`:

```
DISPLAY_OPT=vnc=127.0.0.1:1 lab/vm/vmctl.sh start ubuntu     # :2 per fedora; il monitor qemu è /mnt/b/vm/<vm>.mon
~/spike/orca-spike-vm.sh orca        # in VM: Orca con --debug-file (a righe), via systemd-run --user nella sessione
~/spike/orca-spike-vm.sh app         # in VM: demone finto + finestra di Galleria nella sessione
python3 lab/vm/orca-spike-run.py <vm> fields|toast|statusbar|all
python3 lab/vm/vncshot.py 5901 foto.png                       # fotografia dal VNC (5902 per fedora)
```

- **I tasti sono veri**: `sendkey` del monitor qemu, quindi tastiera virtuale → Mutter → Orca, non WebDriver. Dopo ogni tasto
  0,6 s, poi 2,5 s di attesa del parlato (come `S4b` di Nello).
- **Il parlato** sono le righe `SPEECH OUTPUT` del file di debug di Orca (`lab/vm/orca-line-buffered.py`, copia di quello
  di `lab/tests/`). Orca fa l'eco dei tasti («tab», «left shift», «return»): è rumore, non parlato dell'app.
- **Il fuoco** è letto da AT-SPI a ogni passo (`atspi-dump.py`) e scritto accanto al parlato.
- `qemu screendump` restituisce il vecchio schermo di avvio con `virtio-vga` + GNOME: le fotografie vengono dal VNC.
- Trappole incontrate, tutte nostre: GNOME blocca lo schermo dopo 5 minuti (su Ubuntu serve `gsettings set
  org.gnome.desktop.session idle-delay 0`, `…screensaver lock-enabled false` e `loginctl unlock-session`); su Fedora, al
  primo accesso, la panoramica (Overview) resta aperta, l'app nasce senza il fuoco e Orca dice «Main stage»: `Esc` e poi
  riavviare l'app. Il primo giro Fedora fatto così non vale e non è riportato.

### Esito

| # | Problema | Ubuntu 26.04 (Orca 50.3) | Fedora 45 (Orca 51.1) |
| --- | --- | --- | --- |
| 1 | Regioni vive mute (avviso a scomparsa, barra di stato) | **SI CONFERMA** | **SI CONFERMA** |
| 2 | Il nome dei campi di testo non si sente con `Tab` e `Shift+Tab` | **NON SI CONFERMA** | **NON SI CONFERMA** |

**1 · Regioni vive: confermato su entrambe.** L'avviso c'è (fotografia in `/mnt/b/vm/spike/*-toast.png`: «Sample data:
nothing was changed», in basso, con il pulsante di chiusura) e Orca non dice niente:

```
Enter su «Duplicate» (fuoco su «More actions», avviso a schermo, attesa 6 s)       Ubuntu                     Fedora
  SPEECH OUTPUT: 'return'                                                      'return'                   'return'
  SPEECH OUTPUT: 'More actions' 'combo box.' 'opens menu'  (il fuoco che torna)   sì                         sì
  …l'avviso: niente
Demone fermato → la pagina diventa «Start the service», la barra di stato «No connection state» / «The service is not running»
  parlato: (niente)                                                            (niente)                   (niente)
```

Nel file di debug di Orca 50.3 l'avviso è un solo evento: `object:children-changed:add` sulla regione (`status bar`,
`live:polite, atomic:true, xml-roles:status, container-live:polite`) con il pulsante «Dismiss the notice» come dato; lo
tratta lo script dell'applicazione (`toolkits.gtk`, «script handles events when inactive») e non pronuncia nulla. Vale
per l'avviso, per la barra di stato e per la sostituzione della pagina quando il demone si ferma. L'albero AT-SPI è
giusto (`status bar`, `landmark «Start the service»`). **La sola correzione rispetto alla review è l'ambiente: è così anche
su Orca 50/51, WebKitGTK 2.52/2.54 e Wayland, quindi non è un effetto di Xvfb o dell'assenza del gestore di finestre.**

**2 · Campi di testo: non si conferma.** Con `Tab` e con `Shift+Tab` Orca dice il nome del campo, il ruolo, il valore e
«selected» (parlato, eco dei tasti tolta; Ubuntu e Fedora uguali salvo i punti finali):

```
Tab sul campo «Name»        Ubuntu: 'Name' 'entry' 'Datacenter Milano' 'selected.' 'Focus mode'
                            Fedora: 'Name' 'entry' 'Datacenter Milano' 'selected'  'Focus mode'
Tab sul campo «Server»      'Server' 'entry' 'gw.dc-mi.example:443' 'selected(.)'
Tab su «Group»              'Group' 'combo box' 'Work(.)'
Shift+Tab su «Name»         'Name' 'entry' 'Datacenter Milano' 'selected(.)'      (nessun «niente»: il nome si sente)
Shift+Tab su «Group»        'Group' 'combo box' 'Work(.)'
```

Riprodotto due volte su Ubuntu e due su Fedora, stesso risultato. Una sola differenza minore: `Shift+Tab` che torna sul
pannello della scheda (`scroll pane «General»`) è muto su Ubuntu e su Fedora dice «General scroll pane» «Browse mode».
**Cosa non so**: l'immagine ha Orca 46.1, quella di Ubuntu 24.04 LTS; i buchi del campo di testo potrebbero essere dello
script `WebKitGtk` di Orca 46 e non di Xvfb. Una VM Ubuntu 24.04 desktop lo dice (non era nel perimetro dello spike).

### Strada provata: l'evento AT-SPI `announcement` (tempo impiegato: circa 30 minuti, sotto l'ora)

Orca ha un gestore di `object:announcement` (`scripts/default.py::_on_announcement`) che pronuncia l'`any_data` se è una
stringa; la priorità dipende da `detail1` (assertive → più alta, polite → alta). Il segnale D-Bus è
`org.a11y.atspi.Event.Object.Announcement` (`siiva{sv}`). Nella pila provata (ATK 2.52–2.62) lo emette l'atk-bridge quando un oggetto
accessibile emette il segnale GObject `announcement(testo)` o `notification(testo, urgenza)`. I quattro modi provati
(`lab/vm/orca-spike-announce.sh`, `atspi-announce.py`, `gtk-announce.py`), uguali su Ubuntu e Fedora:

| | Chi emette | Orca lo dice? | Perché |
| --- | --- | --- | --- |
| A | programma fuori dall'app, connessione nuova sul bus AT-SPI, percorso `/org/a11y/atspi/accessible/root` | **No** | il mittente di un segnale D-Bus non si falsifica: la sorgente per Orca è `:1.N`, un oggetto che non esiste. Log: `Ignoring defunct object: [DEAD …]` |
| B | programma fuori dall'app, che si registra come applicazione AT-SPI minima (`Socket.Embed`) e poi emette | **Sì**: `'B: from a registered probe application'` | la sorgente è viva. Orca non guarda il fuoco né l'app: la dice anche con il fuoco in Galleria |
| C | finestra GTK3 (stessa pila di Galleria) che emette `announcement(testo)` sul proprio oggetto accessibile | **Sì** | il mittente è l'app stessa |
| D | come C, `notification(testo, 2)` (assertive) | **Sì**, priorità più alta (Ubuntu 2, Fedora HIGHEST) | idem |

**E poi la prova decisiva, dentro Galleria.** Un patch non committato (`lab/vm/orca-spike-inapp.patch`: 40 righe in
`lib.rs`, `gtk = "0.18"` nel `Cargo.toml`, già nell'albero di tauri) emette dal processo `galleria-gui`, 12 s dopo l'avvio e
poi ogni 12 s, `announcement`, `notification` assertive, `announcement` sull'oggetto accessibile della finestra
(`WebviewWindow::gtk_window()` → `accessible()` → `emit_by_name`, nel thread principale GTK con `run_on_main_thread`).
Orca le dice tutte e tre, con il fuoco in Galleria:

```
Ubuntu  SPEECH OUTPUT: 'IN-APP announcement signal: first notice' · 'IN-APP notification signal assertive: second notice' · 'IN-APP announcement signal: third notice'
Fedora  (le stesse tre)
        EVENT MANAGER: Dequeued object:announcement for [frame: 'Galleria'] in galleria-gui-spike (gtk 3.24.52) (2, 0, IN-APP notification signal assertive: second notice) priority: …
```

**Anche l'immagine Docker (Orca 46.1, at-spi2-core 2.52, Xvfb) le dice** (`lab/vm/docker-announce.sh`, prove C e D):
`'Docker C: atk announcement from a GTK3 window'`, `'Docker D: atk notification assertive'`. Quindi il gate a11y di Q22
potrebbe provarla senza VM.

Cosa **non** è provato: l'annuncio dall'app **con** il testo dell'avviso dopo un'azione reale dell'utente (il patch
emette a tempo, non da un comando chiamato dalla pagina); la voce (nessun audio); Orca 46 su un desktop vero; altri
lettori (NVDA e Narrator sono di Windows, fase 3); l'effetto della doppia voce se un giorno Orca o WebKitGTK
cominciassero a leggere le regioni vive.

### Proposta di Q27 · come far arrivare a Orca gli esiti e i cambi di stato

**Q27 è approvata** da Aitano il 25/11/2026 su delega di Peppe, come raccomandata qui sotto: alternativa 1 per esiti non
bloccanti e cambi di stato (*polite* di norma, *assertive* per gli errori), alternativa 3 per conferme ed errori che
chiedono una risposta, alternativa 4 per tutto il resto; niente `aria-live` come unico canale, niente notifiche del
desktop. Il testo di riferimento è nella spec (§12.1 e §18); il primo uso è nel piano di B3
(`docs/superpowers/plans/2026-11-25-fase2-b3-rubrica.md`, T2). Delle due cose «da decidere insieme» in fondo: il comando
entra in B3; la VM Ubuntu 24.04 è la Q28 di quel piano.

**Domanda.** Orca non pronuncia nessuna regione viva di WebKitGTK (confermato su GNOME 50/51: 10 forme iniettate nella
review, più l'avviso, la barra di stato e la pagina «Start the service» qui). Gli esiti di salvataggio ed eliminazione
(B3) e i cambi di stato della connessione (B5) devono arrivare a chi usa Orca. Come?

**Alternative.**

1. **Segnale ATK `announcement`/`notification` dal lato Rust** (provato dentro Galleria: Orca 50.3 e 51.1, e in Docker 46.1).
   Un comando Tauri nuovo (`announce(text, urgenza)`) che chiama `run_on_main_thread` e `emit_by_name` sulla finestra;
   per i cambi di stato della connessione, che nascono già in Rust (il supervisor), niente comando: li emette lo
   stesso codice che manda `galleria:state`. *Pro*: è l'evento che Orca già gestisce; nessun permesso, nessun processo
   in più; il testo è quello dei cataloghi (la UI passa la stringa già tradotta); verificabile nel gate a11y con lo stesso
   `SPEECH OUTPUT`. *Contro*: codice solo Linux (`cfg`), un comando e una dipendenza esplicita (`gtk` è già nel `Cargo.lock`
   tramite tauri, `cargo deny` da riguardare); la sorgente è la finestra, non l'elemento; richiede un ATK/at-spi2-core
   recenti (provati dalla 2.52 in su; su un sistema più vecchio il segnale potrebbe non esserci: cade nel limite dichiarato); l'avviso esiste due volte, nel DOM e
   nel segnale: se Orca o WebKitGTK cominciano a leggere le regioni vive, si sentirebbe due volte.
2. **Notifiche del desktop** (libnotify/portale). Orca le legge, ma sono un'altra cosa: finiscono nel centro notifiche, «non
   disturbare» le silenzia, serve un crate e il portale, e un avviso a scomparsa di 4 s non è una notifica. **Non provata.**
3. **Spostare il fuoco sull'avviso**, come la conferma (S5 è verde). Funziona, ma ruba il fuoco dal campo in cui si sta
   scrivendo: giusto per errori che bloccano, sbagliato per un «Salvato».
4. **Limite dichiarato**: l'avviso e la barra di stato restano visibili e raggiungibili con la tastiera, ma Orca non li
   dice da solo. Nessun codice, nessuna regressione, ma un esito di eliminazione che non si sente è un buco di G12.
5. **Altro `aria-live`**: scartata, le dieci forme iniettate dalla review e la prova di qui sono mute.

**Raccomandazione**: **1** per gli esiti non bloccanti e i cambi di stato (urgenza *polite*; *assertive* solo per
errori), **3** solo per le conferme e gli errori che già chiedono una risposta (come oggi), **4** scritto come limite
per tutto il resto (altri lettori, sistemi con un ATK senza il segnale). Scartare **5**; **2** si riprende solo se la 1 cade su
un'altra distro. Per B3: il comando `announce` entra con il primo esito vero (salvataggio) e ha una prova nel gate a11y
(il vecchio `S6b`, che oggi è `it.fails`, diventa una prova positiva: «Orca dice l'esito»); le prove di controllo
sulle regioni vive di Nello restano, così il giorno che Orca le legge la prova lo dice e l'annuncio si toglie. La passata
a orecchio di B6 (Q22) resta, su VM, con voce.

**Da decidere insieme** (per Aitano e Peppe): il comando va nel perimetro di B3 o prima, come micro-blocco; Orca 46.1 è la
versione di Ubuntu 24.04 LTS, e il campo 2 non è stato provato lì: se interessa chiuderlo si apre una VM Ubuntu 24.04
(una card piccola).

### Non verificato (spike)

- Nessun ascolto: tutto è letto dal file di debug di Orca, non dalla voce. Il patch in-process emette a tempo; il
  comando chiamato dalla pagina non è scritto.
- Orca 46 su desktop vero (Ubuntu 24.04), GNOME su X11, KDE/Plasma e Orca su altre distro.
- Il comportamento della coda di Orca se gli annunci sono molti e ravvicinati (urgenza, interruzioni).
- Le due VM sono «Cloud» con il desktop aggiunto sopra (come in `t_4a5ce3ed`): non sono installazioni da ISO.

## B3 · Rubrica e profili

Piano: `docs/superpowers/plans/2026-11-25-fase2-b3-rubrica.md`. Q28–Q34 approvate da Aitano il 2026-11-25 su delega di Peppe,
tutte come raccomandate (segnate nella spec, §18). Ambiente: quello di B2. Il gruppo A (T1, T2) parte da `main` con il piano e
lo spike di Orca (`b5f3240`).

### `state.toml`, lingua e tema salvati (T1)

Il file è `$XDG_STATE_HOME/galleria/state.toml` (altrimenti `~/.local/state/galleria/`), cartella `0700` e file `0600`,
scritto con rename atomico. `ui_state.rs` non importa Tauri; `window.rs` salva la dimensione; i comandi sono `ui_state_get` e
`ui_state_set` (il `patch` ha solo `lang`, `theme`, `sort` e rifiuta ogni altra chiave: `window` e `last_used` li scrive solo
Rust).

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S21 | L'attributo del tema è sulla radice prima del primo disegno, anche con il tema salvato | **Vero.** `main.tsx` mette il tema del sistema subito e quello salvato appena `ui_state_get` risponde, prima di montare React; la prova `language_and_theme_survive_a_restart` sceglie il tema **opposto** a quello del sistema (WebKitGTK nell'immagine dichiara «chiaro»: sceglie «Scuro») e al secondo avvio trova quello salvato. `theme_attribute_is_set_before_first_paint` resta verde | `ui/e2e/smoke.e2e.ts` | nessuno |
| S23 | La dimensione salvata viene applicata all'avvio sotto Xvfb | **Vero.** `window_size_survives_a_restart`: `setWindowSize(900, 620)`, la app scrive `[window]` entro mezzo secondo, riaperta la finestra è 900×620 (uscita: «S23: saved 900×620, the window opens at 900×620»). Su Wayland resta **non verificato** (nessuna sessione Wayland qui) | `ui/e2e/smoke.e2e.ts` | nessuno |

Scelte da sapere:

- `tauri.conf.json` crea la finestra **nascosta** (`"visible": false`); `window::restore` applica la dimensione salvata e poi
  la mostra: la finestra si apre una volta, già della misura giusta. Se il setup fallisce la finestra non compare.
- La dimensione si salva alla chiusura e 500 ms dopo l'ultimo ridimensionamento (la sessione di WebDriver uccide l'app senza
  evento di chiusura). Una finestra massimizzata tiene larghezza e altezza di prima (`WindowSize::after`): quelle dello
  schermo non sono la misura a cui tornare. Una misura fuori da 400–16000 px non si applica.
- `configure_with` ha ora un quarto parametro, il file di stato: le prove di livello 2 lo mettono nella cartella
  temporanea e nessuna scrive nella casa di chi le lancia. `lab/tests/gui-lib.sh` esporta `XDG_CONFIG_HOME` e
  `XDG_STATE_HOME` dentro la cartella temporanea del giro, e `openApp()` parte da uno `state.toml` vuoto (`openApp(true)` per
  la prova che riavvia di proposito): senza, la lingua scelta da una prova finiva nella successiva.
- `Preferences.setLang/setTheme` applicano la scelta subito e restituiscono la promessa della scrittura; `AppMenu`, che sta
  sotto il `ToastProvider`, mostra l'errore `gui_state_not_saved` come avviso assertivo.
- `main.tsx` aspetta `ui_state_get` al più 300 ms (`READ_LIMIT_MS`); oltre, o in caso di errore, parte con «Sistema».
- `localStorage` non è mai stato usato in `ui/src` (la ricerca non trova nulla).

Non verificato (T1): il cancellare `state.toml` a finestra aperta e la scelta successiva che lo ricrea («Fatto quando»), a
mano: `update` rilegge e riscrive a ogni scelta e la prova `broken_toml_gives_the_defaults_and_the_next_write_replaces_it`
copre il caso del file rotto, ma non l'ho fatto nella finestra vera.

### Comando `announce` e prova con Orca nell'immagine (T2, Q27)

`announce.rs` (senza `tauri` né `gtk`): `Urgency`, il tratto `Announcer`, `clean` (via i caratteri di controllo, gli spazi e gli
a capo diventano uno spazio, al più 300 caratteri) e `pick_signal` (la tabella P3). `announce_atk.rs` (solo Linux) è l'unico
file che tocca `gtk`: `emit_on` cerca i segnali con `SignalId::lookup` e solo dopo emette; `AtkAnnouncer` lo chiama dal thread
principale (`run_on_main_thread`) sull'oggetto accessibile della finestra `main`. Il comando `announce(text, assertive)` pulisce
il testo, chiama l'`Announcer` nello stato dell'app e non fallisce mai per un segnale assente. `ToastProvider` ha la proprietà
`announce` e la chiama una volta per ogni avviso mostrato (`assertive` per il tono `error`); `app/announce.ts` è l'unico file
che invoca il comando (prova `only_one_file_invokes_announce`) e ingoia un rifiuto. `App` passa la funzione a `Shell`.

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S18 | L'annuncio emesso da un comando chiamato dalla pagina, subito dopo un tasto, arriva a Orca e non è coperto dal parlato del fuoco che torna sul pulsante | **Vero, senza ritardo.** `S6b` (ora positiva): dopo «Duplicate» Orca dice «Sample data: nothing was changed» **una volta** (contate di nuovo dopo 1,5 s), il fuoco resta su «More actions». Subito dopo dice «More actions combo box.»: il fuoco che torna non copre l'avviso. 7 giri di `scripts/gui.sh a11y` con l'annuncio acceso: `S6b` verde in tutti e sette; con `announce` spento a mano (A/B, 3 giri) è rossa in tutti e tre, come deve | `ui/e2e/shell.a11y.ts` | nessuno: i ritardi di 200 e 500 ms del tetto non servono |
| S19 | Un segnale ATK mancante non manda in panico l'app | **Vero.** `a_missing_signal_is_skipped_without_a_panic`: un `GObject` semplice non ha né `announcement` né `notification`, `emit_on` torna `None` senza emettere (e scrive una riga su stderr una volta sola). In più `the_signal_atk_has_is_emitted_with_the_text_and_the_urgency`, **con ATK vero** (2.52 nell'immagine, `NoOpObject`, senza display): `notification` arriva con il testo e `1`/`2` come urgenza | `tests/announce.rs` | nessuno |
| S20 | Nel file di debug di Orca si legge che l'annuncio viene dal processo `galleria-gui` | **Vero.** Forma della riga in Orca 46.1: `EVENT MANAGER: Dequeued object:announcement for [frame: 'Galleria'] in [application: 'galleria-gui'] (1, 0, Sample data: nothing was changed)`; il primo numero è `Atk.Live` (1 = polite). `S6c` la cerca e verifica frame e applicazione | `ui/e2e/shell.a11y.ts` | nessuno |

Altre verità e numeri:

- **`gtk` 0.18** come dipendenza esplicita solo su Linux: `Cargo.lock` guadagna **una riga** (`"gtk"` tra le dipendenze di
  `galleria-gui`), nessun crate o versione nuovi. `cargo deny check` sull'host: `advisories ok, bans ok, licenses ok, sources ok`,
  come prima. Non eseguito nell'immagine né sul runner (job `gui`): lì `cargo deny` non c'è, resta da vedere alla prima CI.
  `deny.toml`: aggiunta al motivo di `RUSTSEC-2024-0429` («`gtk` è diretto, per il solo annuncio»).
- `gtk_is_the_one_tauri_uses` (una sola voce `gtk` in `Cargo.lock`, versione 0.18) e `no_glib_or_atk_direct_dependency`
  tengono fermo il vincolo.
- Le prove di controllo sulle regioni vive (`control_orca_does_not_say_any_form_of_live_region`) sono rimaste come erano e
  verdi: l'avviso esiste due volte (nel DOM e nel segnale) e Orca ne dice una. Il giorno che dirà anche la regione viva, `S6b`
  conterà due volte e diventerà rossa insieme al controllo.
- Due righe `object:announcement` nel file di debug per lo stesso testo: sono i due «Duplicate» del giro (`S6` e `S6b`), non un
  doppione.
- **Controllo della `textarea` instabile (già prima di T2).** `control_orca_does_not_say_the_name_of_a_bare_text_field` è
  fallita su `textarea` in 2 giri su 4 con l'annuncio acceso e in 1 su 3 con `announce` spento a mano (esperimento A/B, poi
  tolto): Orca 46.1 qualche volta dice «Probe foxtrot entry Some value.» per una `textarea` nuda, altre volte tace. Non
  dipende da noi. La forma `textarea` è ora **registrata e non asserita**, come già quella del campo vuoto; le altre sette forme
  con un valore restano asserite («il nome non si sente»). Dopo la modifica: 2 giri di fila verdi, 23 prove su 23.
- `shell_never_calls_the_daemon` ora accetta `announce` tra le chiamate: parla con l'app, non con il demone.

Non verificato (T2): la voce (nessun audio, si legge il file di debug di Orca); Orca 46 su un desktop vero (Q28, in B6); un
errore *assertive* detto da Orca: la strada è provata per il *polite* (urgenza 1) con l'app vera, per l'*assertive* solo con
ATK vero in `the_signal_atk_has_is_emitted_with_the_text_and_the_urgency` (il segnale parte con 2) e con lo spike (riga D,
`notification(testo, 2)` detta con priorità più alta); `R4b` e `R4c` del piano (esito di salvataggio, eliminazione) nascono con
i task che producono quegli esiti (T5, T8).

### Comandi di lettura e guscio sui profili veri (T3)

Il guscio non ha più dati finti: la lista e il dettaglio vengono dalla cartella dei profili che usa anche la CLI
(`$XDG_CONFIG_HOME/galleria`, altrimenti `~/.config/galleria`), letta con le funzioni di `galleria-profile`, senza il demone.

- **`galleria-profile`, solo aggiunte** (Q29, Q30): `settings.rs` con `RubricaSettings` e `ProfileStore::settings()` (legge
  `galleria.toml`; «`schema_version = 1`» da solo resta valido; una chiave sconosciuta è un errore, uno schema più nuovo è
  `NewerSchema`); `ProfileStore::list_lenient()` e `Skipped` (un profilo che non si legge torna con il nome del file e il motivo, gli
  altri restano; i permessi o il proprietario della cartella fanno ancora fallire tutto). Nessuna firma esistente cambia.
- **`galleria-gui`**: `rubrica.rs` (viste `ProfileSummary`, `ProfileDetail`, `OpenVpnView`, `RubricaView`; senza `tauri` e
  senza segreti: `secret_refs` porta i nomi, mai i valori), `commands/profiles.rs` (`profiles_list`, `profile_get(id: Uuid)`; async,
  fuori dal thread principale), `galleria-profile` e `galleria-ovpn` come dipendenze normali (`Cargo.lock`: due righe,
  nessun crate nuovo). `cargo tree -p galleria-gui -e normal` non contiene `galleriad` né un backend.
- **Codici nuovi** (in `GUI_CODES` e nei due cataloghi): `gui_store_insecure` (`data`: `path`, `mode`, `fix` con il comando `chmod 700|600 '<percorso>'`,
  o `chown` per un proprietario sbagliato), `gui_store_unreadable` (`data.path`), `gui_profile_not_found`; per un file saltato
  `gui_profile_newer` e `gui_profile_invalid`.
- **Webview**: `rubrica/store.ts` (`useRubrica`: legge all'avvio, quando la finestra torna in primo piano e quando `reload()` viene
  chiamata; tiene la selezione, e se il profilo sparisce sceglie quello che ne prende il posto; vale solo l'ultima risposta se due
  letture si accavallano), `rubrica/arrange.ts` (puro: gruppi nell'ordine di `galleria.toml`, poi quelli noti solo ai profili in ordine alfabetico,
  in fondo «Senza gruppo»), `shell/RubricaNotices.tsx` (store illeggibile con il comando da copiare, file saltati per nome, rubrica
  vuota con `galleria import <file>`), `shell/CommandRow.tsx`. La scheda «Generale» è **in sola lettura** (campi `readOnly`,
  trasporto come `Segmented` in sola lettura); le altre schede dicono `pane.arrives_later`; Duplica, Esporta ed Elimina dicono
  `action.arrives_later` (P10). Spariti: `demo-data.ts`, i testi `demo.*`, la fascia «Dati di esempio», il banner, la finestra di conferma
  di «Elimina» (torna con T8).
- **Prove**: `galleria-profile/tests/settings.rs` (6), `galleria-gui/tests/rubrica.rs` (31, tra cui `no_read_command_returns_a_secret`, che prima
  controlla che il seme contenga davvero dei segreti, e `works_without_any_daemon`), `rubrica/{store,arrange}.test.ts`, `Shell.test.tsx` e
  `ProfileDetail.test.tsx` riscritte sulla rubrica di prova, `boundary.test.ts` (`shell_only_calls_the_listed_commands`: i comandi invocati
  da `ui/src` sono esattamente quelli di `capabilities/main.json`). `examples/seed-rubrica.rs` (con `tests/support/seed.rs`,
  incluso con `#[path]`) scrive cinque profili con le funzioni di import, tre gruppi, due preferiti e segreti `SEED-SECRET-…`; `lab/tests/gui-lib.sh`
  genera lo script `rubrica-seed` e `openApp()` riparte da una rubrica nota (`openApp(false, "empty")` per quella vuota).
  Fumo nuovo: `rubrica_lists_the_seeded_profiles` (a demone fermo la lista c'è) e `empty_rubrica_shows_the_import_hint`.

Scostamenti dal piano, da sapere:

- `rubrica/arrange.ts` nasce in T3 (con le sole regole dei gruppi e le sue prove) invece che in T4: serve già per mostrare i gruppi
  e per sapere quale profilo prende il posto di quello che sparisce. T4 ci aggiunge ricerca, preferiti, ordinamento.
- Il campo «Riconnessione automatica» è un campo di testo in sola lettura («Sì»/«No»), non un interruttore: un interruttore che non
  risponde è peggio di niente, e C lo riporta com'è. Per questo `S4` (l'interruttore sotto Orca) e `S5` (la conferma di «Elimina» sotto Orca) sono
  `it.skip` in `shell.a11y.ts` fino a T5 e T8; il dialogo resta provato da `overlays.a11y.ts` (D1–D4) e da `ConfirmDialog.test.tsx`.
- Dentro un gruppo i profili restano nell'ordine del disco (per nome): «Ufficio Napoli», preferito, sta dopo «Datacenter Milano» finché T4 non
  mette i preferiti in cima. Le prove di fumo e di Orca che nominavano il primo profilo ora nominano «Datacenter Milano».
- `no_command_takes_a_path` (sulle firme) è del T8; qui `profile_get_refuses_what_is_not_an_id_and_says_when_there_is_none` prova che un
  nome o un percorso non passano (l'argomento è un `Uuid`).
- La prova `shell_never_calls_the_daemon` di B2 è sostituita da `shell_only_calls_the_listed_commands` (come da piano).

Esito dei comandi (T3): `scripts/gui.sh check` verde (289 prove JS e le prove Rust), `smoke` 17/17, `a11y` 21 + 2 saltate, due giri di fila;
host: `cargo fmt --check`, `clippy -D warnings`, `cargo test` (535), `cargo deny check`, `check-boundaries.sh`.
`git diff main -- crates/galleria-ipc crates/galleriad crates/galleria-backend*` vuoto.

Non verificato (T3): permessi sbagliati sul vero `~/.config/galleria` di una persona (provato solo su cartelle temporanee); una rubrica con
molte centinaia di profili (la lettura è sincrona nel comando async, non misurata); `galleria.toml` rotto fa fallire la lista intera con
`gui_store_unreadable` (scelta: meglio un errore chiaro che gruppi che spariscono in silenzio).

### Ricerca, preferiti, ordinamento e stato sulla riga (T4)

- **`rubrica/arrange.ts`** (puro, senza React né Tauri) ha ora tutte le regole del piano: i gruppi nell'ordine di `galleria.toml`, poi quelli noti solo ai
  profili in ordine alfabetico, in fondo «Senza gruppo»; un gruppo senza profili si vede solo a ricerca vuota; dentro il gruppo i preferiti prima (P5) in
  tutti e due gli ordinamenti, poi per nome (`Intl.Collator` della lingua della finestra, a parità per id) oppure, per «Più recenti» (P6), per ultimo uso
  decrescente e poi per nome (chi non è mai stato usato, o ha una data illeggibile, viene dopo). La ricerca guarda nome, server e gruppo, senza maiuscole né
  accenti, e ogni parola deve comparire («citta mi» trova «Città Milano»).
- **`SearchBox`**: etichetta visibile solo allo screen reader («Cerca connessioni»), `Ctrl+K` porta il fuoco lì da qualunque punto, `Esc` svuota, `↓`
  passa alla lista (`ProfileList` espone `focusRef`). Il conto dei risultati è testo visibile in un elemento `role="status"` e **non** passa da `announce`
  (limite dichiarato, spec §12.1 regola 3; prova `result_count_is_a_status_and_is_not_announced`).
- **`SortControl`**: `Segmented` «Per nome · Più recenti», scrive con `ui_state_set` e, se la scrittura fallisce, dice `gui_state_not_saved` come lingua e tema.
  L'ordinamento salvato lo legge `main.tsx` insieme a lingua e tema (`App sort=…`); lo tiene `App` perché serve anche alla selezione (il profilo che prende
  il posto di uno che sparisce è il successivo **nell'ordine che la lista mostra**, `displayOrder(view, sort, lang)`).
- **Stato sulla riga**: se `status.profile.id` è un profilo della rubrica e lo stato non è `disconnected`, la riga mostra l'etichetta `state.<s>` come testo e la porta
  nel nome accessibile («Homelab Connected»); il pallino ha solo il colore in più. Una sessione di un profilo che non è in rubrica non compare in nessuna riga (D21 è B5).
- **Nessun risultato**: la `listbox` resta nel documento, senza opzioni, e `search.no_match` lo dice; il dettaglio resta quello che era aperto (una ricerca non cambia
  il profilo aperto).
- **Effetti sulle prove di B2/T3**: l'ordine di Tab diventa «Menu» → ricerca → ordinamento → lista → … (`keyboard.test.tsx`, `App.test.tsx`, fumo, Orca); con i preferiti in cima
  il primo profilo torna «Ufficio Napoli» (in T3 era «Datacenter Milano»). `search-count` è un secondo `status` nella pagina: le prove degli avvisi cercano la regione
  dentro `.gx-toasts`. `ui/src/test/shell.tsx` (`renderShell`) porta anche l'ordinamento.
- Fumo nuovo: `search_filters_the_list_and_sort_order_survives_a_restart` (cerca «milano», svuota, sceglie «Più recenti», legge `sort = "recent"` in `state.toml`, riavvia e
  ritrova la scelta).

Non verificato (T4): «Più recenti» con date vere (nessuno scrive `last_used` prima di B5: provato solo con date messe a mano nelle prove); `Ctrl+K` sotto Wayland o con un
gestore di scorciatoie del desktop che lo prenda prima; il comportamento di Orca sul campo di ricerca (stesso limite del campo di testo nudo nell'immagine, Orca 46.1).

### Schede Generale, Autenticazione e Rete; `profile_update` (T5)

- **`galleria-gui`** (nessuna modifica a `galleria-profile`): `edit.rs` (senza `tauri` e senza segreti) ha `ProfilePatch` (`deny_unknown_fields`;
  nessun campo per un segreto, un percorso, l'id o le date; `group: null` è «nessun gruppo» e un gruppo assente non chiede niente),
  `FieldIssue { field, code }`, `check_label` e `check_wireguard`. `Rubrica::update(id, expected_modified, patch)` apre lo store, rilegge il
  profilo, confronta l'ora di modifica (`gui_profile_changed` se è un'altra: niente viene scritto), applica la patch tutta o per niente,
  aggiorna `modified` e salva con `ProfileStore::save` (rename atomico, `0600`). **Non apre l'archivio dei segreti.** `RubricaError` è ora
  un `enum` (`Store { dir, source }`, `Changed`, `Invalid(Vec<FieldIssue>)`). Comando `profile_update(id, expected_modified, patch)` in
  `COMMANDS`, nella capability e in `tests/config.rs`; codici nuovi `gui_profile_changed` e `gui_invalid_field` (`data.fields`: l'elenco
  `{ field, code }`) nei due cataloghi.
- **Regole** (P8): nome 1–64 caratteri senza caratteri di controllo (tolti gli spazi ai lati), gruppo 1–48; per WireGuard indirizzi e reti
  in forma CIDR, DNS come IP, domini, MTU 576–9000, endpoint `host:porta`, chiave pubblica di 32 byte in base64, keepalive 0–65535, almeno un
  indirizzo e una rete per peer (**più severa del demone** su queste ultime due, come chiede D26: `the_window_is_stricter_where_the_daemon_is_not`).
  Il numero dei peer non cambia (`peer_count_cannot_change`: i segreti sono per posizione del peer). Il testo di una configurazione OpenVPN non
  si cambia da qui (`not_editable`, è di T7).
- **Prova contro il demone** (P8): `galleria-backend-wireguard` è una dipendenza **di sola prova** di `galleria-gui` (`cargo tree -p galleria-gui -e normal` non
  contiene né quel crate né `galleriad`). `what_the_gui_accepts_the_daemon_accepts` (9 profili validi: `to_connect_params` + `validate` riescono) e
  `what_the_daemon_refuses_the_gui_refuses` (17 campi rotti uno alla volta: il demone rifiuta, e il campo che nomina è tra quelli che la finestra segnala).
  `Cargo.lock`: tre righe (`base64`, `ipnet` e il crate dei test tra le dipendenze di `galleria-gui`), nessun crate nuovo.
- **Il seme**: `tests/support/seed.rs` scrive ora chiavi pubbliche di 32 byte in base64 (`SEED-PUBLIC-<tag>` riempito di punti): prima erano parole e la finestra,
  giustamente, le avrebbe rifiutate.
- **Webview**: `rubrica/validate.ts` (stesse regole, puro: serve a dire l'errore mentre si scrive), `rubrica/draft.ts` (la bozza: ogni campo è testo, `build` dà
  la patch dei soli campi cambiati o l'elenco dei campi sbagliati; `fieldOrder` e `fieldTab` dicono dove sta ogni campo), `useRubrica().update(id, patch,
  expectedModified)` (non rilegge la lista: la rilegge chi scrive, quando ha messo la risposta al suo posto), `shell/SaveBar` (appare solo con una differenza;
  `Ctrl+S`), `shell/tabs/{General,Authentication,Network}`. `ProfileDetail` tiene il profilo com'è sul disco, la bozza, le scritture e gli errori; il guscio
  tiene la guardia («Salva · Scarta · Annulla») e la apre quando si sceglie un altro profilo con modifiche non salvate. Il `ConfirmDialog` ha una terza scelta
  opzionale; `Field` ha la proprietà `name` (come il modulo trova il campo a cui dare il fuoco). La stella è un pulsante a due stati nell'intestazione e scrive subito.
- **Comportamento**: interruttori e stella scrivono subito, mostrano il valore nuovo mentre la scrittura va e tornano com'erano se fallisce (avviso d'errore,
  *assertive*); una scrittura alla volta. Il testo si scrive con «Salva» (avviso `profile.saved`, *polite*, il fuoco resta dov'è; col pulsante il fuoco passa
  alla scheda scelta, perché la barra sparisce). Un errore di un campo (della finestra o di `gui_invalid_field`) **non è un avviso**: il fuoco va sul primo campo
  sbagliato nell'ordine delle schede (la scheda si apre da sola), l'errore è collegato con `aria-describedby`, e scrivendo nel campo l'errore se ne va. Un profilo
  cambiato sul disco (CLI, altra finestra) senza modifiche in corso viene preso senza chiedere; con modifiche in corso queste restano e compare l'avviso in linea con
  «Ricarica». Un salvataggio che trova `gui_profile_changed` lascia la bozza, mostra lo stesso avviso e dice l'errore come avviso *assertive*.
- **Prove**: Rust, `tests/rubrica.rs` passa da 31 a 71 prove (contando i casi delle tabelle); JS (440 in tutto): `validate` (57), `draft` (6), `SaveBar` (6), `tabs/*` (22), `ProfileDetail`
  (34: la parte di lettura e la parte di modifica), `store.update` (2), `ConfirmDialog` e `Field` (una ciascuno). Fumo: `a_profile_is_renamed_and_the_list_follows`
  (rinomina, «Salva», la lista e il titolo seguono, il file sul disco ha il nome nuovo; poi un interruttore scrive `ignore_pushed_routes`). Orca:
  `S4` (l'interruttore) torna verde sul modulo vero; `R4a` (il salvataggio è detto **una volta**, urgenza 1 nella riga `object:announcement`), `R4d` (la prova riscrive il file come
  farebbe la CLI, poi un interruttore: Orca dice «This profile was changed somewhere else since you opened it.» una volta, urgenza **2**), `R5a`.

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S24 | Un errore *assertive* detto da Orca arriva dall'app vera | **Vero.** `R4d`: riga `EVENT MANAGER: Dequeued object:announcement … (2, 0, This profile was changed somewhere else since you opened it.)`; Orca la dice una volta. Chiude il «non verificato» di T2 sull'urgenza 2 | `ui/e2e/rubrica.a11y.ts` | nessuno |
| S25 | Con il fuoco che torna da solo su un campo sbagliato Orca legge l'errore con il campo | **Non confermato nell'immagine.** L'albero è giusto (il campo è `invalid`, con `described-by` verso l'errore) e `R5a` lo asserisce; ma Orca 46.1 dice solo «Text unselected.»: né il nome né la descrizione. È lo stesso limite del campo di testo (`S4b`), quindi `R5a` ha l'attesa del parlato **capovolta** (diventa positiva quando l'immagine avrà un Orca che dice i campi). Su GNOME vero la descrizione di un campo non è stata provata | `ui/e2e/rubrica.a11y.ts` | registrato, non asserito |
| S26 | L'urgenza arriva a Orca come `(1, 0, testo)` per il *polite* e `(2, 0, testo)` per l'*assertive* | **Vero** (due prove: `R4a` e `R4d`) | `ui/e2e/rubrica.a11y.ts` | nessuno |

Scostamenti dal piano, da sapere:

- La logica sta in `edit.rs` (nuovo) e non tutta in `rubrica.rs`: `rubrica.rs` resta di sola lettura più `update`, che chiama `edit::apply`.
- `profile_update` risponde con `data: { fields: [...] }` (un oggetto) e non con la sola lista: lascia posto ad altro senza cambiare forma.
- La stella è un pulsante con `aria-pressed` e il nome «Favourite»/«Preferito» (uno stato, non un'immagine): Tab ha una sosta in più tra la lista e «More actions»
  (`keyboard.test.tsx`, fumo e Orca aggiornati).
- Se non c'è niente in modifica, un profilo cambiato altrove viene preso in silenzio anche quando a scoprirlo è un interruttore che fallisce con
  `gui_profile_changed`: l'avviso d'errore c'è (e Orca lo dice), l'avviso in linea con «Ricarica» no, perché non c'è niente da proteggere.
- Il testo di `error.gui_profile_changed` (avviso a scomparsa) e quello dell'avviso in linea sono **frasi diverse**: la stessa frase due volte sarebbe letta due volte dai
  lettori di schermo che dicono le regioni vive.
- «Chiudere con modifiche non salvate» (la chiusura della **finestra**) non è coperta: la guardia c'è per il cambio di profilo. Intercettare la chiusura vuole l'evento di
  chiusura della finestra (un permesso in più alla webview o un gestore in Rust) e non è nel piano di T5; da decidere.

Non verificato (T5): la finestra «Salva · Scarta · Annulla» con il lettore di schermo (nessuna prova di Orca; la conferma di «Elimina» di T8 ha la sua `S5`); il fuoco
che torna sul campo sbagliato dopo la chiusura della finestra di conferma è affidato a un `setTimeout(0)` (provato in jsdom, non nella finestra vera); un profilo
WireGuard con più peer nella finestra vera (provato in jsdom e con le prove Rust); la lettura dell'errore di un campo con Orca su un desktop vero (S25).

### Gruppi in `galleria.toml` (T6)

- **`galleria-profile`, solo aggiunta** (Q29, Q30): `ProfileStore::save_settings(&RubricaSettings)`: rename atomico di un file temporaneo privato (`0600`), rifiuta uno schema più
  nuovo sia nelle impostazioni date sia in quelle già sul disco (`NewerSchema`, il file resta com'è), e non scrive impostazioni di uno schema diverso da quello che questa versione capisce.
  `open`, `list` e `find` non cambiano (`the_cli_store_still_opens_a_folder_with_groups`).
- **`galleria-gui`**: `rubrica/groups.rs` (figlio di `rubrica.rs`, che ne vede i campi privati): `group_create`, `group_rename`, `group_delete`, `groups_reorder`, ognuna restituisce l'elenco
  nuovo. Nome: spazi tolti ai lati, 1–48 caratteri, niente caratteri di controllo, unico senza guardare le maiuscole (con le maiuscole di un gruppo rinominato in se stesso non c'è
  scontro). **Prima i profili, poi l'elenco** (rinominare o eliminare riscrive più file e non è atomico): ogni profilo spostato è scritto con una nuova `modified`, i segreti non si aprono.
  `RubricaError` guadagna `GroupInvalid(motivo)`, `GroupExists`, `GroupNotFound`; i quattro comandi (`commands/groups.rs`, solo nomi, nessun percorso né id) entrano in `COMMANDS`, nella
  capability e in `tests/config.rs`; codici `gui_group_invalid` (`data.reason`), `gui_group_exists`, `gui_group_not_found` nei due cataloghi.
- **Webview**: `GroupsDialog` («Gestisci gruppi», aperta dal fondo della colonna della lista, P9): una riga per gruppo (campo del nome, che si rinomina con Invio o con il pulsante «Rinomina» che compare
  quando il nome è cambiato; su, giù, elimina; quanti profili ha), i gruppi che solo i profili hanno con «Aggiungi all'elenco», e il campo del nuovo gruppo. «Elimina» su un gruppo con profili chiede
  conferma e dice quanti restano senza gruppo. Il fuoco segue la riga: un gruppo nuovo prende il fuoco sul suo nome, dopo uno spostamento il fuoco resta sul pulsante (o sull'altro, se per il posto nuovo
  quello è spento), dopo un'eliminazione va alla riga che ne ha preso il posto. Creato, rinominato, eliminato e «aggiunto all'elenco» sono avvisi (*polite*); lo spostamento non ha avviso. Un errore
  su un nome (della finestra o di `gui_group_*`) va sul campo e prende il fuoco; un altro errore è un avviso d'errore. `useRubrica().groups` ha le quattro operazioni e rilegge la rubrica dopo ogni scrittura riuscita.
  Il selettore «Gruppo» di «Generale» elenca i gruppi dell'elenco, il gruppo che il profilo ha anche se l'elenco no, e «Nessun gruppo». Tre icone nuove (`arrowUp`, `arrowDown`, `trash`).
- **Prove**: `galleria-profile/tests/settings.rs` (4 nuove: 10 nel file), `galleria-gui/tests/groups.rs` (16 con i casi), `GroupsDialog.test.tsx` (16), `General.test.tsx` (3 nuove),
  `ProfileDetail.test.tsx` (2 nuove sul giro finestra → lista → selettore). Fumo nuovo: `a_group_is_created_renamed_and_survives_a_restart` (crea «Casa», lo rinomina «Famiglia», rinomina «Clienti» in «Clienti VIP»:
  `galleria.toml` e il profilo sul disco seguono, riavviata la finestra i gruppi ci sono). Tab ha una sosta in più («Manage groups» tra la lista e la stella): `keyboard.test.tsx`, `App.test.tsx`, fumo e Orca aggiornati.

Scostamenti dal piano, da sapere:

- `rename_is_resumable_after_a_failure` mette a mano lo stato che un fallimento a metà lascia (un profilo già col nome nuovo, l'elenco ancora col vecchio) e controlla che la stessa chiamata
  lo finisca. Non vede l'ordine delle scritture: lo tengono `rename_writes_the_profiles_before_the_list` e `delete_writes_the_profiles_before_the_list`, che forzano un fallimento
  vero della scrittura dell'elenco senza permessi (le prove girano come root, dove togliere i permessi non ferma niente): la prova rilancia se stessa in un processo figlio sotto `ulimit -f`,
  un limite in cui un profilo sta e l'elenco dei gruppi (reso lungo apposta) no; la scrittura dell'elenco fallisce con l'errore vero del sistema (`EFBIG`) e il figlio guarda cosa è rimasto: i profili
  già spostati e l'elenco col vecchio nome. Con l'ordine invertito (elenco prima) le due prove sono rosse. Che una scrittura che fallisce restituisca un errore è già coperto da `store_errors_map_to_gui_codes` e dalle prove di T5.
- Un profilo che non si legge (saltato da `list_lenient`) non è spostato da una rinomina o eliminazione: tiene il vecchio nome di gruppo (si vede finché è saltato).
- Il piano diceva che la finestra «offre “Aggiungi all'elenco”»: lo fa con `group_create` (quello è anche il modo), niente comando in più.

Esito dei comandi (gruppo C): `scripts/gui.sh check` verde (461 prove JS, prove Rust di `galleria-gui` e clippy `-D warnings`), `smoke` 20/20, `a11y` 25 + 1 saltata (`S5`, T8); host `fmt`, `clippy -D warnings`,
`cargo test`, `cargo deny`, `check-boundaries.sh`. `git diff main -- crates/galleria-ipc crates/galleriad crates/galleria-backend*` vuoto.

Non verificato (T6): la finestra «Gestisci gruppi» sotto Orca (nessuna prova di Orca: solo l'albero e i ruoli in jsdom con axe); un gruppo con centinaia di profili (ogni rinomina riscrive i suoi file, uno alla volta); gruppi con la stessa grafia
che differiscono solo per forma Unicode (NFC/NFD): per «unico» conta il minuscolo, non la normalizzazione.

**«Fatto quando» di T6, a mano:** una cartella con il seme, in cui le funzioni di `Rubrica` hanno creato «Casa», rinominato «Lavoro» in «Ufficio» ed eliminato «Clienti» (`galleria.toml`:
`groups = ["Ufficio", "Personale", "Casa"]`), letta da `galleria list` della fase 1 sull'host (`XDG_CONFIG_HOME` puntato alla cartella): i cinque profili escono senza errori. La prova automatica è del T9
(`cli_lists_what_the_gui_saved`). Fatto con una prova temporanea, poi tolta.

### Editor raw con validazione (T7, Q32)

- **`galleria-profile`, solo aggiunte** (Q29): `raw.rs` con `ConfigIssueKind` (`Syntax`, `Dangerous`, `Unsupported`, `DaemonOwned`, `FileReference`, `SecretBlock`, `NoTlsClient`), `ConfigIssue { line, directive, kind }`
  e `check_openvpn_config(text)`: la classificazione della fase 1 (`galleria_ovpn::classify`, come la applica `check_strict`), riga per riga, senza leggere file. Un errore di sintassi è l'unico
  problema (dopo non si può dire altro); l'avviso `NoTlsClient` (né `client` né `tls-client`) viene per ultimo, a riga 0, e non impedisce il salvataggio (`blocks_saving`). `directive` è il
  **nome** della direttiva o del blocco, mai un argomento né il contenuto di un blocco (`an_issue_never_carries_the_content_of_a_block_or_an_argument`). `render_wireguard(profilo, chiavi)`
  rifà il `.conf`: senza chiavi non ci sono le righe `PrivateKey` e `PresharedKey`, con le chiavi è il file che T8 esporterà e `import_wireguard` lo rilegge nello stesso profilo e negli stessi segreti.
  `galleria_profile::openvpn_text` riesporta `parse`, `Item` e `ParseError`: `galleria-gui` non nomina più `galleria-ovpn` nel suo `Cargo.toml` (lo nominava dal T3, per leggere i `remote`).
- **`galleria-gui`**: `rubrica::check_raw` è l'unico punto che dice cosa impedisce un salvataggio. `profile_validate_raw(id, text)` risponde `{ issues: [{ line, directive, kind }], can_save }`, non scrive
  niente e per un profilo WireGuard dà `gui_invalid_field` (`wrong_protocol`). `profile_update` accetta `openvpn_config` e **ricontrolla il testo da solo**, qualunque cosa abbia detto prima la
  finestra: se qualcosa impedisce il salvataggio risponde `gui_invalid_field` con `data.fields = [{ field: "openvpn_config", code: "config_issues" }]` e `data.issues` con la stessa lista
  (`RubricaError::InvalidConfig`); altrimenti salva il testo **com'è scritto**, commenti e righe vuote compresi, con la solita rename atomica a `0600`. Se insieme al testo c'è un altro campo
  sbagliato, la risposta è la lista dei campi (con `config_issues` dentro) e senza `issues`: la finestra, che controlla da sé prima di inviare, non ci arriva. `ProfileDetail` guadagna
  `wireguard_config` (il `.conf` senza chiavi). Il comando entra in `COMMANDS`, nella capability e in `tests/config.rs`. `galleria-backend-openvpn` è dipendenza **di sola prova** (come il
  backend WireGuard per P8): `cargo tree -p galleria-gui -e normal` non lo contiene.
- **Webview**: `shell/tabs/Configuration.tsx` (`Field as="textarea" mono`, 16 righe, senza correzione ortografica; sotto, l'elenco dei problemi, una voce per riga, ognuna un pulsante che porta il cursore
  sulla riga e la seleziona; la voce di riga 0 è testo, non pulsante). `Draft.config` tiene il testo; `build` lo manda **senza toccarlo**. `ProfileDetail` controlla il testo 400 ms dopo l'ultimo tasto e **sempre
  prima di salvare**: se qualcosa impedisce il salvataggio non invia niente, apre la scheda, mette l'errore sul campo e il fuoco sul primo problema dell'elenco (nessun avviso: Q27, regola 2); lo stesso
  se è Rust a rifiutare. Un avviso (manca `client`) si vede e non ferma il salvataggio. Il testo di D19 («Galleria non esegue gli script dei profili») è per `up`, `down`, `route-up`, `plugin` e le altre
  direttive che lanciano uno script; le altre «pericolose» (`log`, `user`, `auth-user-pass file`) hanno una frase che non dice «script». Un blocco segreto e un riferimento a un file dicono di importare di nuovo il profilo.
  Sotto il testo, «Nell'archivio dei segreti: …» con i nomi tradotti (`secretLabel`: «chiave privata», «chiave condivisa del peer 2»; un nome sconosciuto resta com'è). Un profilo WireGuard mostra il suo
  `.conf` in sola lettura, senza chiavi. `Field` ha le proprietà `rows` e `spellCheck`. Tolto il testo `issue.not_editable`, che non nasce più.
- **Prove**: `galleria-profile/tests/raw.rs` (21 con i casi), `galleria-gui/tests/raw.rs` (10), `Configuration.test.tsx` (20), `draft.test.ts` (2 nuove); fumo nuovo
  `a_script_directive_cannot_be_saved_from_the_editor` (si scrive `up /tmp/x.sh`, «Salva»: la lista lo mostra, il fuoco va sul problema e i file sul disco sono quelli di prima) e
  `a_comment_typed_in_the_editor_is_saved_as_written`.

| # | Supposizione | Esito | Dove | Ripiego applicato |
|---|---|---|---|---|
| S24 | Un testo di configurazione salvato com'è scritto, con i commenti, passa il validatore del demone | **Vero.** `saved_raw_config_is_accepted_by_the_daemon_validator`: un testo con commenti `#` e `;`, righe vuote e un commento in coda a una riga si salva identico, `to_connect_params` e `galleria_backend_openvpn::config::validate` lo accettano (il parser toglie i commenti prima di classificare) | `tests/raw.rs` | nessuno: non serve `render(parse(testo))` |

Scostamenti dal piano, da sapere:

- `ConfigIssueKind` ha `blocks_saving()` (non c'era nel piano): è il punto unico che dice che `NoTlsClient` è un avviso.
- Il piano elencava `ui/src/shell/ProfileDetail.tsx` e `Configuration.tsx` ma non il tipo `RawCheck`/`RawIssue` in `rubrica/types.ts`, né `validateRaw` in `useRubrica` (la finestra deve chiedere il controllo mentre si scrive).
- Il rifiuto di Rust è `gui_invalid_field` con `data.issues`, come chiede il piano; la forma di `fields` resta quella di T5, così la finestra la tratta come ogni altro campo sbagliato.
- La stella, gli interruttori e le altre schede non sono toccate. La scheda «Log» dice ancora `pane.arrives_later`.

Non verificato (T7): l'editor con un testo di molte migliaia di righe (la validazione è sincrona in Rust e il testo passa tutto dall'IPC a ogni controllo, 400 ms dopo l'ultimo tasto); l'editor sotto Orca (`entry` multilinea, elenco dei problemi: T9);
il caret portato sulla riga e lo scorrimento in WebKitGTK (provato in jsdom: `setSelectionRange`; lo scorrimento usa `getComputedStyle(...).lineHeight`, che in jsdom non c'è, e nella finestra vera non è provato).

### Duplica, esporta, elimina (T8, Q31)

- **Duplica**: la copia nasce in Rust, segreti compresi, col primo nome libero «X (copy)», «X (copy 2)»… (accorciato dal nome dell'originale se sfora `NAME_MAX`). Tutto o niente: si scrivono prima i segreti e, se il profilo non si scrive, si cancellano. La copia non è preferita e viene scelta; il fuoco va su «Altre azioni» della copia.
- **Esporta**: dopo una conferma («il file conterrà le chiavi private») Rust apre la finestra «Salva» (`gtk::FileChooserNative`, `export_gtk.rs`) e scrive il file dall'archivio dei segreti senza passare dalla webview: la risposta è solo `{ saved }`, mai testo, percorso o chiavi. Annullare la finestra non dice nulla. Un errore dice «il file non si è potuto salvare» senza il percorso. Scelta (S22): niente `tauri-plugin-dialog`, che avrebbe portato 4 crate in più, un secondo major di `toml` e i comandi di due plugin; `gtk` è già una dipendenza (Q27). Un test della configurazione verifica che la capability non dia alla webview alcun permesso di dialoghi o file.
- **Elimina**: dopo una conferma; rifiutata (`gui_profile_in_use`) se il profilo è quello della sessione aperta dal servizio, e la voce del menu lo dice (`aria-disabled` + descrizione, resta una sosta delle frecce). Dopo l'eliminazione il fuoco va alla lista o, se è vuota, al testo «Nessun profilo».
- `Rubrica::with_secrets(SecretFactory)` è il punto unico che B5 sostituirà con l'archivio a strati.

Non verificato (T8): smoke ed a11y della conferma di Elimina (S5) non rieseguiti dopo T8 (rieseguiti in T9). La finestra «Salva» GTK vera, non pilotata in T8, lo è dopo la review di B3: `R4e` e `R4f` (sotto, «Tastiera, fumo…»).

### Tastiera, fumo, screen reader e chiusura del blocco (T9)

**Cosa ha trovato il giro da tastiera.** Un buco vero: dal campo di ricerca `↓` porta il fuoco sulla prima riga trovata, ma la riga **non veniva scelta**
(la scelta segue il fuoco solo con le frecce, e la riga scelta prima restava fuori dai risultati): `Enter` e `Spazio` non facevano niente. Corretto in `ProfileList.tsx` (`Enter` e
`Spazio` scelgono la riga che ha il fuoco) con la prova `enter_and_space_choose_the_row_that_has_the_focus`. Nient'altro nei componenti.

**Prove nuove.**

| Livello | Prova | Cosa dice |
|---|---|---|
| Vitest (`keyboard.test.tsx`) | `a_full_rubrica_tour_needs_no_pointer` | `Ctrl+K`, ricerca, scelta, modifica e `Ctrl+S`, gruppo, editor raw e `Ctrl+S`, «Gestisci gruppi», duplica, elimina con conferma: nessun puntatore |
| | `every_control_of_every_tab_and_of_the_new_dialogs_is_reachable` | ogni controllo delle cinque schede dei due protocolli (editor compreso), le tre voci del menu e ogni controllo del dialogo dei gruppi è una sosta del Tab; il fuoco non esce dal dialogo |
| | `focus_stays_in_the_page_after_save_duplicate_delete_and_the_groups_dialog` | il fuoco non finisce sul `body` dopo salva (anche rifiutato), chiusura di «Gestisci gruppi» (due volte), esporta, annulla e conferma di elimina |
| | `no_outcome_depends_on_a_live_region_alone` | ogni `ToastProvider` fuori da `design/` ha `announce`; salva, duplica, esporta, gruppo ed elimina arrivano a `announce` con il testo dell'avviso (*polite*), un errore con `assertive`; sei chiamate per sei avvisi, nessuno detto due volte |
| Rust (`tests/rubrica.rs`) | `no_read_command_returns_a_secret_after_a_round_of_edits` | tredici risposte (lista, due `profile_get`, `profile_update`, un rifiuto, `profile_validate_raw`, creazione e rinomina di un gruppo, duplica, esporta, elimina, lista finale) senza `SEED-SECRET-`, senza `PRIVATE KEY` e senza il percorso della macchina; il file esportato invece le chiavi le ha |
| Fumo | `cli_lists_what_the_gui_saved` | dopo rinomina e duplica nella finestra, il binario `galleria` della fase 1 (`galleria list`, costruito nell'immagine da `scripts/gui.sh`, variabile `GALLERIA_SMOKE_CLI`) elenca i sei profili con i nomi nuovi |
| | `rubrica_works_with_the_daemon_stopped` | a demone fermo: rinomina, duplica, elimina con conferma; i file sul disco sono quelli attesi e il fuoco resta sulla lista |
| | `no_csp_violation_during_the_rubrica_tour` | ricerca, editor, salva, dialogo dei gruppi, duplica, esporta e elimina (le due conferme): nessuna violazione |
| Orca (`rubrica.a11y.ts`) | `R4b`, `R4c` | duplica ed elimina detti **una volta**, *polite*, dal processo `galleria-gui`; con `R4a` (salva) e `R4d` (errore) sono le quattro di Q27 |
| | `R4e`, `R4f` | **dopo la review**: «Esporta» con la finestra «Salva» GTK vera, pilotata via AT-SPI (`lab/tests/atspi-save-window.py`: la finestra non è nella pagina, WebDriver non la vede). `R4e`: il nome proposto è `Ufficio Napoli.ovpn`, col percorso scritto nel campo e «Save» il file nasce `0600` con le chiavi (`<tls-crypt>`, `SEED-SECRET-`) e nessun altro file accanto, «Exported.» detto **una volta**, *polite* (`(1, 0, Exported.)`), il percorso e le chiavi non sono nella pagina, il fuoco torna su «More actions». `R4f`: con «Cancel» nessun avviso (né parlato né `announcement`), niente di scritto nella rubrica, fuoco su «More actions» |
| | `R5b`…`R5e` | ricerca e stati della lista, rubrica vuota, schede nuove e editor, «Gestisci gruppi» |
| | `S5` (rimessa) e `S6`, `S6b`, `S6c` (riscritte) | la conferma di Elimina sotto Orca; con «Duplicate» che ora duplica davvero, il testo dell'avviso si legge dalla pagina e non è più scritto nel codice |

Numeri: **499** prove JS in 41 file (`scripts/gui.sh check`: 494 dopo T8, 4 nuove di tastiera e 1 della lista); **225** prove Rust di `galleria-gui` (221 al T9, 4 della correzione dopo la review: due su `params_of`/`split_params`, due sull'ordine delle scritture dei gruppi); host **565** (`cargo test --workspace --exclude galleria-gui`);
fumo **25** prove; `a11y` **34** prove (32 al T9, `R4e` e `R4f` dopo la review), due giri di fila verdi.

**Criteri di uscita del blocco.**

| # | Esito | Prova |
|---|---|---|
| R1 | **Verde.** Letta, cercata, ordinata, modificata, duplicata, esportata (anche con la finestra «Salva» GTK vera, via AT-SPI), eliminata nella finestra vera, anche a demone fermo, e la CLI della fase 1 la legge. Riserva rimasta: la finestra «Salva» è provata nell'immagine (Xvfb, `GtkFileChooserDialog` semplice), non sotto un portale del desktop né su Wayland (vedi «Non verificato») | `rubrica_lists_the_seeded_profiles`, `cli_lists_what_the_gui_saved`, `rubrica_works_with_the_daemon_stopped`; `R4e`, `R4f`; `export.rs` (selettore finto) |
| R2 | **Verde** | `ui_state.rs`, `language_and_theme_survive_a_restart`, `window_size_survives_a_restart` (S21, S23) |
| R3 | **Verde** | `no_read_command_returns_a_secret` e `..._after_a_round_of_edits`, `no_command_takes_a_path` (firme di tutti i comandi, T8: dopo la review ogni comando si legge nel suo file, in ordine fisso, fino alla `)` che chiude l'elenco dei parametri; `params_end_at_the_parenthesis_that_closes_the_list` lo tiene fermo), un test della capability: la webview non ha permessi di dialoghi o file |
| R4 | **Verde, una volta sola, con la trascrizione sotto.** Salva, duplica, elimina, esporta: *polite* (`(1, 0, …)`), errore: *assertive* (`(2, 0, …)`); la riga di Orca dice `[application: 'galleria-gui']` | `R4a`, `R4b`, `R4c`, `R4d`, `R4e`, `R4f`, `S6b`, `S6c` |
| R5 | **Verde, con i buchi di Orca già dichiarati** (sotto). Tastiera in jsdom per ogni componente e per il giro intero; axe per ogni componente (nei file di prova di ciascuno); albero AT-SPI e Orca nella finestra vera | `keyboard.test.tsx`, `R5a`–`R5e`, `S1`–`S5` |
| R6 | **Verde** | `git diff 9dfc366 -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-cli` vuoto; in `galleria-profile/src` **zero righe tolte** (solo `export.rs`, `raw.rs` e aggiunte a `lib.rs`, `settings.rs`, `store.rs`); `a_profile_saved_by_the_gui_keeps_the_phase_1_format`; `cli_lists_what_the_gui_saved` |

**Punti [S] di B3.**

| # | Esito | Ripiego |
|---|---|---|
| S18 | **Vero** (T2), riconfermato con avvisi veri: salva, duplica ed elimina sono detti subito, una volta, con il fuoco dove era | nessuno |
| S19 | **Vero** (T2) | nessuno |
| S20 | **Vero** (T2); `S6c` ora cerca il testo vero della copia | nessuno |
| S21 | **Vero** (T1) | nessuno |
| S22 | **Non usato, per scelta di T8**: niente `tauri-plugin-dialog` (4 crate in più, un secondo major di `toml`, due plugin di comandi); la finestra «Salva» la apre Rust con `gtk::FileChooserNative`. Il ripiego scritto nel piano | applicato (vedi T8); provato con la finestra vera nell'immagine (`R4e`, `R4f`, via AT-SPI), non sotto un portale del desktop né su Wayland |
| S23 | **Vero** (T1) | nessuno |
| S24 | **Vero** (T7): il testo salvato com'è scritto passa il validatore del demone. Nota: nella tabella di T5 `S24`–`S26` sono altri punti (errore *assertive*, fuoco su un campo sbagliato, forma dell'urgenza), numerati dopo il piano | nessuno |

**Trascrizione di Orca** (Orca 46.1 nell'immagine, righe `SPEECH OUTPUT` di `target/gui-a11y/transcript.txt` così come sono; il secondo giro ha le stesse).

```
## R4a the save of a name, Ctrl+S
SPEECH OUTPUT: Selection deleted.
SPEECH OUTPUT: Profile saved.
## R4b the copy of a profile, «Duplicate»
SPEECH OUTPUT: Duplicated as “Ufficio Napoli (copy)”.
SPEECH OUTPUT: More actions combo box.
SPEECH OUTPUT: opens popup
## R4c the delete of a profile, confirmed
SPEECH OUTPUT: Deleted “Ufficio Napoli”.
SPEECH OUTPUT: Datacenter Milano.
SPEECH OUTPUT: gw.dc-mi.example:443 OpenVPN.
## R4d the write of a switch after the file was changed (error, assertive)
SPEECH OUTPUT: This profile was changed somewhere else since you opened it.
## S5 confirmation opened (role seen: alert)
SPEECH OUTPUT: alert Delete Datacenter Milano?
SPEECH OUTPUT: The profile and its secrets are removed from this computer. This cannot be undone.
SPEECH OUTPUT: Cancel push button.
## R5e a group added
SPEECH OUTPUT: Group Casa created.
## R4e the export of a profile, saved through the real window
SPEECH OUTPUT: file chooser.
SPEECH OUTPUT: Name: text.
SPEECH OUTPUT: /tmp/galleria-export-k4cKia/Ufficio.ovpn.
SPEECH OUTPUT: Galleria frame.
SPEECH OUTPUT: Exported.
## R4f the export of a profile, cancelled in the real window
SPEECH OUTPUT: More actions combo box.
SPEECH OUTPUT: opens popup
```

La riga dell'evento (file di debug di Orca): `EVENT MANAGER: Dequeued object:announcement for [frame: 'Galleria'] in [application: 'galleria-gui'] (1, 0, Deleted “Ufficio Napoli”.)`; per l'errore `(2, 0, This profile was changed somewhere else since you opened it.)`.
Dopo l'esito, Orca dice il controllo dove il fuoco è tornato («More actions combo box.») o la riga che lo ha preso (dopo Elimina: «Datacenter Milano.»): il fuoco non finisce mai nel vuoto.

**Cosa si è visto sul resto del giro** (registrato in `transcript.txt`):

- Il campo di ricerca: nel giro registrato Orca dice «Search connections entry» (poi il valore). Il nome del campo resta **registrato, non asserito**: nell'immagine, con Orca 46.1, un campo di testo spesso non dice il nome (`S4b`, «Buco 1»).
- L'editor: l'albero ha un `entry` multilinea (`multi-line`) di nome «Configuration»; Orca lo dice «Configuration scroll pane».
- Il pannello di una scheda è una sosta del Tab per conto suo: dalla linguetta il primo Tab può non arrivare al campo.
- Il testo dei paragrafi e delle righe di stato (il conto «1 result», «No connection matches your search.», il comando di import) **non è nel nome del nodo** AT-SPI (è nell'interfaccia del testo): le prove lo leggono dalla pagina.
- Per il dialogo «Gestisci gruppi» all'apertura e per il fuoco sul primo problema dell'editor dopo un salvataggio rifiutato la prova asserisce l'albero (ruolo `dialog`, pulsanti, il fuoco), **non** il parlato: nella trascrizione di quei passi non c'è una riga `SPEECH OUTPUT` nuova.

**Il limite dichiarato di Q27** (cosa Orca non dice da solo):

- La barra di stato e il conto dei risultati della ricerca (regioni vive: mute sotto Orca con WebKitGTK, sull'immagine e, per lo spike, anche su GNOME vero): non sono annunciati, sono testo da leggere.
- Un errore di campo (`R5a`): non c'è avviso, il fuoco torna sul campo e l'errore è la sua descrizione. Nell'immagine Orca 46.1 non la pronuncia; l'albero è giusto (`invalid`, `described-by`).
- Interruttori e stella: nessun annuncio (Orca dice già il nuovo stato).
- Il parlato dei nomi dei campi di testo nell'immagine (Orca 46.1): vedi `S4b`; su GNOME vero Orca 50.3 e 51.1 lo dicono (spike).
- Altri lettori di schermo (NVDA, VoiceOver, Narrator): non provati; la fase 2 verifica solo Orca.
- Un ATK senza i segnali `announcement`/`notification` (più vecchio di 2.46/2.50): l'app non va in panico (`S19`) ma l'esito non viene annunciato; resta il testo nella pagina.

**Non verificato (B3).**

- **La voce:** nessun audio, si legge il file di debug di Orca.
- **GNOME vero con il comando definitivo:** `announce` e gli esiti sono provati nell'immagine (Ubuntu 24.04, Orca 46.1) e con lo spike su due desktop GNOME veri a B2 (`93c309a`); l'app di B3 non è stata riprovata là.
- **Orca 46 su un desktop vero (Q28):** si decide in B6.
- **La finestra «Salva» di «Esporta» (dopo la review):** provata nell'immagine con `R4e` (salva) e `R4f` (annulla), pilotando la finestra GTK via AT-SPI con `lab/tests/atspi-save-window.py` (Xvfb, `GtkFileChooserDialog` semplice: senza portale `FileChooserNative` ricade su quello). **Non provata** sotto un portale di un desktop vero (GNOME: `xdg-desktop-portal`, una finestra d'altro processo, con un altro albero AT-SPI e un altro nome proposto) né su Wayland. Il percorso si scrive nel campo del nome con `set_text_contents`: la scelta con il mouse in una cartella, la creazione di una cartella dalla finestra e la sovrascrittura di un file che c'è (la finestra chiede conferma) non sono provate; che la scrittura sostituisca un file che c'è e lo tenga `0600` lo prova `export_replaces_a_file_that_is_there_and_keeps_it_private`. Orca non è stato ascoltato nella finestra «Salva» stessa, oltre a ciò che la trascrizione registra («file chooser», «Name: text»).
- **La dimensione della finestra su Wayland:** `window_size_survives_a_restart` gira sotto Xvfb (X11); su Wayland un client non sceglie la propria posizione e la dimensione salvata non è provata (Q34).
- Il fuoco dopo la conferma di Elimina e dopo Duplica in WebKitGTK sotto Orca è provato (`R4b`, `R4c`: fuoco su «More actions» e su una riga; `R5e`: dopo «Gestisci gruppi» sul suo pulsante). Il caret portato su una riga dell'editor, lo scorrimento di un testo grande e il comportamento con migliaia di righe lo sono solo in jsdom (o per nulla).
- Il parlato del dialogo «Gestisci gruppi» all'apertura e del primo problema dell'editor (vedi sopra); la parola che Orca usa per il pulsante del problema («Line 18 · up · …»).
- Il job `gui` e `gui-smoke` di CI su un runner con il binario `galleria` costruito dalla nuova riga di `scripts/gui.sh`: non girato.
- **Una prova instabile, già in T2 e fuori dalla correzione:** `atk::the_signal_atk_has_is_emitted_with_the_text_and_the_urgency` (`tests/announce.rs`) fallisce in circa 1 giro su 10 di `cargo test -p galleria-gui` (28 su 300 lanci del binario `announce`; 0 su 300 con `--test-threads=1` e 0 su 300 da sola): `assertion failed: self.is::<T>()` in `NoOpObject::new`, una gara tra le prove del modulo `atk` che girano in parallelo. Si vede anche in `scripts/gui.sh check` e nel job `gui` di CI; non è stata corretta qui (scope).
- Il campo del gruppo (`select` nativo) è provato in jsdom con un `change` (jsdom non lo guida con i tasti): il cambio di gruppo con la sola tastiera in WebKitGTK non è provato.
