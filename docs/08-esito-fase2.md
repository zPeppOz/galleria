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
| | `R4e`, `R4f` | **dopo la review**: «Esporta» con la finestra «Salva» GTK vera, pilotata via AT-SPI (`lab/tests/atspi-file-window.py`: la finestra non è nella pagina, WebDriver non la vede). `R4e`: il nome proposto è `Ufficio Napoli.ovpn`, col percorso scritto nel campo e «Save» il file nasce `0600` con le chiavi (`<tls-crypt>`, `SEED-SECRET-`) e nessun altro file accanto, «Exported.» detto **una volta**, *polite* (`(1, 0, Exported.)`), il percorso e le chiavi non sono nella pagina, il fuoco torna su «More actions». `R4f`: con «Cancel» nessun avviso (né parlato né `announcement`), niente di scritto nella rubrica, fuoco su «More actions» |
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
- **La finestra «Salva» di «Esporta» (dopo la review):** provata nell'immagine con `R4e` (salva) e `R4f` (annulla), pilotando la finestra GTK via AT-SPI con `lab/tests/atspi-file-window.py` (Xvfb, `GtkFileChooserDialog` semplice: senza portale `FileChooserNative` ricade su quello). **Non provata** sotto un portale di un desktop vero (GNOME: `xdg-desktop-portal`, una finestra d'altro processo, con un altro albero AT-SPI e un altro nome proposto) né su Wayland. Il percorso si scrive nel campo del nome con `set_text_contents`: la scelta con il mouse in una cartella, la creazione di una cartella dalla finestra e la sovrascrittura di un file che c'è (la finestra chiede conferma) non sono provate; che la scrittura sostituisca un file che c'è e lo tenga `0600` lo prova `export_replaces_a_file_that_is_there_and_keeps_it_private`. Orca non è stato ascoltato nella finestra «Salva» stessa, oltre a ciò che la trascrizione registra («file chooser», «Name: text»).
- **La dimensione della finestra su Wayland:** `window_size_survives_a_restart` gira sotto Xvfb (X11); su Wayland un client non sceglie la propria posizione e la dimensione salvata non è provata (Q34).
- Il fuoco dopo la conferma di Elimina e dopo Duplica in WebKitGTK sotto Orca è provato (`R4b`, `R4c`: fuoco su «More actions» e su una riga; `R5e`: dopo «Gestisci gruppi» sul suo pulsante). Il caret portato su una riga dell'editor, lo scorrimento di un testo grande e il comportamento con migliaia di righe lo sono solo in jsdom (o per nulla).
- Il parlato del dialogo «Gestisci gruppi» all'apertura e del primo problema dell'editor (vedi sopra); la parola che Orca usa per il pulsante del problema («Line 18 · up · …»).
- Il job `gui` e `gui-smoke` di CI su un runner con il binario `galleria` costruito dalla nuova riga di `scripts/gui.sh`: non girato.
- **Una prova instabile, già in T2 e fuori dalla correzione:** `atk::the_signal_atk_has_is_emitted_with_the_text_and_the_urgency` (`tests/announce.rs`) fallisce in circa 1 giro su 10 di `cargo test -p galleria-gui` (28 su 300 lanci del binario `announce`; 0 su 300 con `--test-threads=1` e 0 su 300 da sola): `assertion failed: self.is::<T>()` in `NoOpObject::new`, una gara tra le prove del modulo `atk` che girano in parallelo. Si vede anche in `scripts/gui.sh check` e nel job `gui` di CI; non è stata corretta qui (scope).
- Il campo del gruppo (`select` nativo) è provato in jsdom con un `change` (jsdom non lo guida con i tasti): il cambio di gruppo con la sola tastiera in WebKitGTK non è provato.

## B4 · Import

Piano: `docs/superpowers/plans/2026-11-25-fase2-b4-import.md`. Q36–Q41 approvate da Aitano il 25/11/2026 su delega di Peppe, tutte come raccomandate, con un'aggiunta su Q36 (cartella `0700`, socket `0600`, stesso uid con `SO_PEERCRED`): spec §18.

### Il cuore dell'import in Rust, con l'anteprima in memoria (T1)

- **`galleria-profile`, solo aggiunte** (Q29; `git diff main -- crates/galleria-profile/src` ha solo righe aggiunte): `import_file.rs` con `detect_protocol`, `import_text`, `import_file`, e `read_import_file` + `import_read_file` (la stessa cosa in due metà, perché la finestra rilegge il testo per sapere quale script era una direttiva tolta). Un percorso che arriva da fuori si apre con `O_NONBLOCK`, si guarda **il file aperto** (file normale, al più 1 MiB, UTF-8) e i file a cui un `.ovpn` rimanda si leggono solo dalla cartella vera del profilo (`canonicalize` poi `starts_with`), con lo stesso tetto. Una FIFO senza scrittore, `/dev/zero` (anche dietro un collegamento), un file di 2 MiB e un `.ovpn` che rimanda a un file di 2 MiB tornano entro 2 s (`a_fifo_a_device_and_a_huge_file_are_refused_without_hanging`).
- **`galleria-gui`**: `import.rs` (`Imports`, `ImportPreview`, `ImportNotice`, `ImportFailure`, senza `tauri`), `rubrica/imported.rs` (`Rubrica::commit_import`: prima i segreti, poi il profilo, tutto o niente; nome e gruppo controllati **prima** di aprire la cartella in scrittura), `commands/import.rs` (`import_from_text`, `import_commit`, `import_discard`), otto codici in `GUI_CODES` e nei due cataloghi.
- **Anteprima**: contiene i **nomi** dei segreti (`found`), mai i valori né il testo; degli avvisi solo riga e nome della direttiva e un sì/no (`dns_script`); del file solo il nome. Dura 10 minuti, al più 4 insieme (la più vecchia si butta), si consuma con `import_commit` e `import_discard`. Un nome o un gruppo rifiutati lasciano l'anteprima dov'è; anche un archivio che non si scrive. Ordine del salvataggio e tutto-o-niente provati (`commit_is_all_or_nothing`: primo segreto, secondo segreto, il profilo stesso).
- **Il formato** (Q39): `.ovpn` è OpenVPN; `.conf` e il testo incollato si decidono dalla prima riga che non è vuota né un commento (`#` o `;`): `[Interface]`, senza badare alle maiuscole (come `wg-quick`), è WireGuard, il resto OpenVPN.

**Punti [S].**

| # | Esito | Ripiego |
|---|---|---|
| S29 | **Vero.** `Secret` (`galleria-ipc/src/secret.rs`) deriva `ZeroizeOnDrop`: i segreti di un'anteprima si azzerano quando cade (letto nel sorgente, non misurato in memoria) | nessuno |

**Cosa si è visto e che il piano non diceva** (tutto misurato oggi con le prove di T1):

- Il **profilo malevolo del laboratorio** dà **5 direttive rimosse con avviso**: `up`, `plugin`, `status`, `writepid`, `setenv` (tutte *dangerous*, ognuna con la sua riga). `dev`, `verb`, `script-security` e `management` sono del demone e l'import della fase 1 le toglie **senza avviso**: non sono nell'elenco e non contano in «N direttive rimosse». Il «6» che il piano ipotizzava era sbagliato; la finestra mostra ciò che l'import dice, e la fase 1 non si tocca.
- L'import OpenVPN della fase 1 è **indulgente**: un testo che non è un profilo (o un `.ovpn` che contiene WireGuard) dà un profilo OpenVPN senza `client`, con un avviso `no_tls_client` e le righe sconosciute tolte. L'errore «non valido» c'è solo per un blocco che non si chiude o una riga che non si legge. Per quel caso `ImportError::Invalid` non porta la riga: la finestra rilegge il testo con il parser e dice `import_syntax` con la riga.
- `dns_script` vale solo per le direttive che lanciano uno script (l'elenco `SCRIPT_DIRECTIVES` di `import.rs`, uguale a quello di `Configuration.tsx`; da T2 anche a `ui/src/rubrica/scripts.ts`), non per `status /tmp/update-resolv-conf`.
- Il nome proposto per un testo incollato è vuoto: lo dice la pagina («Profilo importato», dal catalogo).

**Prove.** `galleria-profile/tests/import_file.rs` (17) e `galleria-gui/tests/import.rs` (33). `seed-rubrica --fixtures <cartella>` scrive i file di prova (`tests/support/fixtures.rs`, uguali a quelli di `galleria-profile`) e `lab/tests/gui-lib.sh` genera lo script `import-fixtures` accanto a `rubrica-seed`.

### Finestra «Importa profilo», testo incollato, anteprima con il testo di D19 (T2)

- **Finestra** (`ui/src/import/ImportDialog.tsx`, `Preview.tsx`): «Importa…» nella barra in alto (prima di «Menu» nel Tab) e `Ctrl+O` aprono la finestra modale in due passi. Passo 1: la casella del testo (`Field as="textarea" mono`, senza correzione ortografica) e «Anteprima»; il fuoco è sulla casella. Passo 2: file e protocollo, «Trovati: …» con i nomi dei segreti in parole e «saranno salvati nell'archivio dei segreti», le credenziali chieste alla prima connessione, l'elenco di ciò che l'import ha tolto, i campi «Nome» (col fuoco) e «Gruppo», «Importa», «Indietro», «Annulla». Niente QR (spec §16, punto 7). «Scegli un file…» sopra la casella è di T3.
- **Testo di D19**: «Riga N · Direttiva `up` **rimossa**: Galleria non esegue gli script dei profili. Se il profilo ne dipende, potrebbe non funzionare del tutto.»; per `up`/`down` con uno dei due script del resolver, la seconda frase diventa «I DNS del server li applica Galleria.»; le altre direttive pericolose «Galleria non la permette nei profili.»; le non supportate «Galleria non la supporta.»; `client` mancante è un avviso a parte, non conta tra le rimosse. Solo riga e nome: mai un argomento. L'elenco delle direttive che lanciano uno script è in `ui/src/rubrica/scripts.ts` (prima era dentro `Configuration.tsx`), uguale a `SCRIPT_DIRECTIVES` di `import.rs`: una prova Rust legge il file e confronta i due elenchi.
- **Errori** (Q27): un errore di import è un avviso in linea **dentro** la finestra e prende il fuoco; nessun avviso a scomparsa e nessun annuncio (provato con `announce` finto). `import_syntax` dice la riga; `import_missing_file` con origine `text` dice di importare il file; un nome rifiutato da Rust mette il fuoco sul campo con l'errore come descrizione; `gui_import_expired` torna al passo 1 e lo dice.
- **Dopo «Importa»**: la finestra si chiude, la rubrica si rilegge (dentro `commit`), la selezione va al profilo nuovo, la ricerca si svuota (sennò potrebbe nasconderlo), il fuoco va sulla sua riga; **un solo avviso** «Importato «X»», seguito da «N direttiva/e rimossa/e» se N > 0 (*polite*). La guardia «Salva · Scarta · Annulla» di B3 scatta se il profilo scelto ha modifiche non salvate.
- **Il testo si svuota** appena arriva l'anteprima (nessuna traccia nella pagina, nemmeno dopo «Indietro»); «Annulla», `Esc`, «×» e «Indietro» chiamano `import_discard`.
- Rubrica vuota: il pulsante «Importa un profilo…» al posto del comando della CLI (P11); `IMPORT_PROFILE` non c'è più.

**Prove finali del gruppo (T1–T3).** Dopo T2 un primo giro (smoke 27/28, Orca 18/34) era rosso solo perché i vecchi ingressi da tastiera aspettavano «Menu» al primo Tab, mentre ora il primo Tab va su «Import…»: `shell_is_navigable_with_the_seeded_rubrica` (5 Tab anziché 4), `shell.a11y.ts` S1 e `overlays.a11y.ts` M1 sono stati corretti nelle prove, non nel codice, e al rilancio sono verdi senza altro da cambiare. A fine gruppo, sull'ultimo commit: `scripts/gui.sh check` verde (533 prove frontend, typecheck e2e, build, clippy, prove Rust); gate host verdi (fmt, clippy workspace esclusa GUI, test, deny con warning sui duplicati, confini); fumo **28/28**; Orca **38/38** (`shell` 12, `rubrica` 11, `overlays` 11, `import` 4) **per due giri di fila**.

Prove scritte nel T2: `Preview.test.tsx`, `ImportDialog.test.tsx`, `store.test.ts`, `keyboard.test.tsx` (`an_import_from_text_needs_no_pointer`), `Shell.test.tsx` (`empty_rubrica_offers_the_import_button`); la sosta in più del Tab aggiorna `tab_order_*`, `focus_is_never_lost_to_the_body`, `a_full_tour_needs_no_pointer`, `App.test.tsx` e `AppMenu.test.tsx`. Fumo: `a_profile_pasted_as_text_is_imported`, `nothing_is_written_before_import`, `the_malicious_profile_shows_the_d19_text_and_runs_nothing`, `empty_rubrica_offers_the_import_button` (il testo si mette nella casella con il *setter* del browser e un evento `input`, come un «incolla»: la tastiera di WebDriver perde lettere). Orca: `R5c` cerca il pulsante «Import a profile…» nell'albero.

### File scelto con la finestra «Apri» di GTK, e la sonda AT-SPI (T3)

- **Rust**: `picker.rs` (`FilePicker`, `NoFilePicker`, senza `tauri`) e `picker_gtk.rs` (`GtkFilePicker`: la finestra «Salva» di B3 con `FileChooserAction::Open`, sul thread principale di GTK, finestra `main` come genitore, due filtri senza nome tradotto: `*.ovpn, *.conf` e `*`). Il comando `import_pick_file` non ha parametri: chiede il percorso al `FilePicker` dello stato, poi `Imports::offer_path(…, Origin::Picker)`; annullato dà `null`, non un errore. Il percorso non esce da Rust: nella risposta c'è `file_name`. Un solo permesso in più, `allow-import-pick-file`; ancora nessun permesso `dialog:*` o `fs:*` e nessun plugin.
- **Webview**: «Scegli un file…» sopra la casella del testo (passo 1). Annullato: niente, il fuoco torna sul pulsante. Riuscito: passo 2 (stessa anteprima del testo, col nome del file e il nome proposto dal file). Un errore (file non supportato, troppo grande…) è l'avviso in linea con il fuoco, come per il testo.
- **Sonda**: `lab/tests/atspi-save-window.py` è diventato `lab/tests/atspi-file-window.py` (stessa uscita JSON, stessi codici di uscita) con i modi `save`, `open`, `cancel`, `dump`. `gui-a11y.sh` ha `GALLERIA_A11Y_FILE_WINDOW` (era `…_SAVE_WINDOW`) e `GALLERIA_A11Y_FILTER`; `scripts/gui.sh` passa nell'immagine `GALLERIA_A11Y_FILTER` e `GALLERIA_SMOKE_FILTER`.

**Punti [S].**

| # | Esito | Ripiego |
|---|---|---|
| S27 | **Vero, con una correzione.** Sotto Xvfb la finestra «Apri» di GTK 3 si apre come `file chooser`, con «Open» e «Cancel» e i due filtri. Il campo del percorso si fa comparire **scrivendo `/`** (`Atspi.generate_keyboard_event` con una stringa); `Ctrl+L` mandato via AT-SPI **non** funziona: il modificatore non arriva e la `L` finisce nella ricerca della lista (la prima prova è fallita per questo: `offered: "l"`). Con il campo in vista `set_text_contents` più «Open» scelgono il file, e la finestra si chiude | nessuno servito: il ripiego 1 del piano (percorso tasto per tasto) non è servito |

**Cosa si è visto e che il piano non diceva.**

- La finestra «Apri» ha **due** campi di testo modificabili nell'albero AT-SPI fin dall'inizio (si pensa la ricerca e il percorso; non si è guardato quale sia quale): la sonda ora considera solo quelli con lo stato `showing`. Con il primo campo dell'albero, qualunque fosse, la prova scriveva il percorso e «Open» non chiudeva la finestra. Anche la finestra «Salva» passa da questo filtro, e `R4e`/`R4f` restano verdi.
- Nel giro con la scelta vera compare un `Gtk-CRITICAL … gtk_widget_event: assertion 'WIDGET_REALIZED_FOR_EVENT'` per ogni prova che scrive `/` nella finestra (non per `R4e`/`R4f`, né per il solo annullo): è un avviso di GTK all'evento di tastiera sintetico mandato alla finestra, non cambia l'esito. Non indagato oltre.
- Orca (trascrizione di `I5b`/`I5d`): «Import… push button.», «Import profile dialog Choose a file or paste the text of the profile…», «Choose a file… push button.», poi dalla finestra di GTK «file chooser.», «/», «Files.», «table with 1 row 4 columns.», «text.» e il percorso; dopo «Open» «Imported “separate”.» **una volta**.
- **Non provata**: la finestra sotto un portale di un desktop vero (GNOME, `xdg-desktop-portal`: un altro processo, un altro albero AT-SPI), Wayland, la scelta con il mouse in una cartella e la scelta di un file che il filtro nasconde.

**Prove.** Rust (`tests/import.rs`, 38): `pick_file_returns_the_preview_of_the_picked_file`, `pick_file_cancelled_is_null_and_not_an_error`, `the_response_never_holds_the_folder`, `a_picked_file_gets_the_same_checks_as_a_dropped_one` (una FIFO dà `gui_import_unsupported_file`, uguale al trascinamento); `tests/config.rs` (`allow-import-pick-file`, nessun `dialog`/`fs`) e `tests/export.rs` (`no_command_takes_a_path`) restano verdi. Frontend: `choose_a_file_shows_the_preview`, `a_cancelled_choice_keeps_the_focus_on_the_button`, `a_file_that_cannot_be_imported_is_an_alert_with_the_focus`, `store.test.ts`. Orca (`ui/e2e/import.a11y.ts`): `I5a` finestra `file chooser` con «Open», «Cancel» e il filtro `*.ovpn, *.conf`; `I5b` file scelto, anteprima («File: separate.ovpn», «Found: private key, TLS encryption key…», la cartella non è nella pagina, nulla scritto prima di «Importa»), «Imported “separate”.» detto una volta, *polite*, file `0600`, il fuoco sulla riga del profilo nuovo; `I5c` «Cancel»: nessun avviso, nessuna anteprima, niente scritto, fuoco su «Choose a file…»; `I5d` l'anteprima è un `dialog` «Import profile» col fuoco su «Name», il parlato registrato.

### File trascinato sulla finestra (T4)

- **Rust**: `Imports::offer_from_outside(paths, origin) -> Option<ImportOffer>` (`import.rs`): un percorso → anteprima o errore con il nome del file; più di un percorso → `gui_import_many` senza leggere niente (Q40); nessun percorso (si è trascinato un testo) → niente. L'offerta resta **in attesa** per la pagina (`take_pending`, una volta sola); una nuova offerta **sostituisce** quella che aspetta e butta l'anteprima della vecchia (P7). `ImportOffer` è `{ "preview": … }` oppure `{ "failed": { error, file_name } }`: né cartelle né segreti (`the_offer_payload_holds_no_path_and_no_secret`). Nuovo comando `import_pending` (nessun parametro) con il permesso `allow-import-pending`; nuovo codice `gui_import_many` nei due cataloghi.
- **Gestore** (`events.rs`, `on_drag_drop`, registrato accanto a quello della finestra in `lib.rs`): `DragDropEvent::Enter`/`Leave` emettono `galleria:drop-hover` con `{ active }`; `Drop` emette `drop-hover` falso e chiama `offer_from_outside(paths, Origin::Drop)` **fuori dal thread principale** (`spawn_blocking`), poi emette `galleria:open-file` con l'offerta. I percorsi non escono da Rust.
- **Webview**: `useIncoming()` (`ui/src/import/store.ts`) ascolta `galleria:open-file` e `galleria:drop-hover` **prima** di chiamare `import_pending` (una volta, al montaggio), come fa `useDaemon`; un'offerta sentita mentre la risposta è in viaggio non si mostra due volte; dopo ogni evento chiede `import_pending` per toglierla dall'attesa. Lo `Shell` apre la finestra di import nello stesso passaggio in cui arriva l'offerta (stato derivato mentre disegna, così la finestra si apre già col passo 2); `ImportDialog` mostra l'anteprima (fuoco su «Nome») o l'errore in linea (fuoco sull'avviso, nessun annuncio) e butta l'anteprima che c'era. Nel passo 1 la riga «Trascina qui un file .ovpn o .conf.». «Rilascia per importare» è solo visivo (`aria-hidden`, niente click, niente fuoco); da tastiera resta `Ctrl+O`.
- **La pagina non ascolta eventi di Tauri** (Q38): `the_page_never_listens_to_the_drag_events_of_tauri` (`ui/src/shell/boundary.test.ts`) cerca in tutti i sorgenti `tauri://`, `onDragDropEvent`, `getCurrentWebview`, `getCurrentWindow`, `TauriEvent` e verifica che ogni `listen(` abbia un nome `galleria:*`. **Provata rossa** aggiungendo a mano `listen("tauri://drag-drop")` in un file temporaneo (poi tolto). `tauri.conf.json` non cambia: `drag_drop_stays_on_and_the_asset_protocol_stays_off` tiene fermo `dragDropEnabled` e l'assenza del protocollo `asset:`. Resta vero (come già scritto in Q38) che una pagina compromessa potrebbe ascoltare `tauri://drag-drop` e vedere i percorsi dei file trascinati: le prove tengono fermo **il nostro** codice, non una pagina compromessa.

**Punti [S].**

| # | Esito | Ripiego |
|---|---|---|
| S9 | **Vero, ora eseguito.** Il gestore `WindowEvent::DragDrop` di Rust riceve i percorsi di un trascinamento vero, senza passare dalla pagina; il comando che prende i percorsi non serve | nessuno |
| S28 | **Vero al primo tentativo.** `lab/tests/gtk-drag-source.py` apre una piccola finestra GTK 3 nell'angolo di Xvfb che offre il file come `text/uri-list`; il mouse è premuto, mosso su Galleria in 24 passi e rilasciato con `Atspi.generate_mouse_event` (`b1p`, `abs`, `b1r`, via XTest). Sotto Xvfb **senza gestore di finestre** l'XDND parte e arriva al gestore di `wry`: `dropped: true`, anteprima WireGuard con `PostUp` e il testo di D19, «Imported “office”. 1 directive removed.» detto da Orca **una volta**, *polite*, fuoco su «Name» | nessuno servito: non sono serviti `xdotool` né la prova a mano |

**Non provato**: un file manager vero (Nautilus, Dolphin) come sorgente, Wayland (lì il protocollo è un altro e `wry` passa da un altro cammino), il trascinamento di più file (provato solo al livello 2: `gui_import_many`), il trascinamento di un testo.

**Prove.** Rust (`tests/import.rs`, 46): `a_dropped_file_becomes_a_pending_preview`, `two_dropped_files_are_refused_and_nothing_is_previewed`, `a_dropped_folder_or_fifo_is_refused` (con scadenza di 5 s: non resta appesa), `an_offer_made_before_the_page_is_ready_is_not_lost`, `a_new_offer_replaces_the_pending_one_and_discards_its_preview`, `the_offer_payload_holds_no_path_and_no_secret`, `import_pending_gives_the_waiting_offer_once_through_the_command`, `the_offer_goes_to_the_page_as_the_open_file_event` (ascolta l'evento in una app finta); `tests/config.rs` (`drag_drop_stays_on_and_the_asset_protocol_stays_off`, elenco dei permessi). Frontend: `store.test.ts` (`useIncoming`: cinque prove), `ImportDialog.test.tsx` (`an_open_file_event_opens_the_preview`, `a_failed_offer_shows_the_error_with_the_focus`, `two_files_at_once_say_that_one_is_enough`, `a_new_offer_replaces_what_the_window_shows_and_lets_go_of_the_old_preview`, `an_offer_that_was_closed_is_not_shown_again_at_the_next_opening`), `Shell.test.tsx` (`drop_hover_shows_and_hides_the_hint`), `boundary.test.ts`. Orca: `I5e_a_file_dragged_onto_the_window_is_previewed` (`GALLERIA_A11Y_FILTER=I5e scripts/gui.sh a11y`: verde).

### File come argomento, istanza unica, associazione dei `.ovpn` (T5)

Scritto su Q36 (b) e Q41 (a), come approvate: **nessun crate nuovo** (`Cargo.lock` non cambia), `cargo deny check` verde senza voci nuove, niente `tauri-plugin-single-instance`.

- **`instance.rs`** (senza `tauri`; `std` più `tokio` che c'è già): `files_in_args` (salta ciò che comincia per `-`, legge `file://…` con i `%XX` a mano, accetta `localhost` e nessun altro host, scarta gli altri URL e un argomento che non è UTF-8, rende assoluti i relativi rispetto alla cartella corrente; non guarda il disco), `runtime_dir`, `claim`, `Primary::serve`. `claim` decide chi è la prima istanza con un blocco esclusivo su `gui.lock` (`File::try_lock`, `flock`: cade da solo con il processo, anche ucciso) e **prima di costruire l'app**, così una seconda istanza non apre nemmeno WebKit. La prima toglie un `gui.sock` rimasto e ne apre uno nuovo; la seconda manda **una riga** `{"v":1,"files":[…]}`, aspetta `ok` ed esce con 0. Rispetto al piano `Claim` ha **quattro** varianti: `Primary`, `Handed`, `Alone(motivo)` e `Failed(motivo)` (la seconda istanza che non riesce a farsi sentire esce con 1 e una riga su `stderr`, non apre una seconda finestra).
- **Q36, l'aggiunta di Aitano.** Cartella `0700` (se c'era con permessi più larghi la si stringe; se non è dell'utente, niente canale), socket `0600`, blocco `0600`. **Stesso utente** controllato dal lato del socket con `SO_PEERCRED` (`peer_cred` sulla connessione accettata, **prima** di leggere un byte). In più, dal lato di chi consegna: non manda niente a un socket che non sia un socket suo con permessi `0600` (`a_socket_with_wider_permissions_is_not_trusted`). Provato rosso e poi verde con due modifiche a mano (togliere il controllo dell'uid, togliere il controllo dei permessi): le due prove cadono, poi il codice è tornato com'era. Un messaggio è al più 64 KiB, al più 16 percorsi, versione 1, tutti assoluti, nessun campo in più, e ha 2 s per arrivare; ogni altra cosa chiude la connessione senza risposta. Le connessioni si servono una alla volta.
- **Limite delle prove sull'uid** (detto in T5, **chiuso dopo la review di B4**): `tests/instance.rs` gira come un solo utente, quindi non ha un secondo uid vero da collegare: fa servire alla prima istanza *l'uid successivo* (`serve_for`), per lei la connessione del nostro utente è «di un altro», e il kernel dice l'uid vero con `SO_PEERCRED`. Prova il confronto, non il passaggio da un secondo utente reale; e togliere `meta.uid() != euid()` da `private_dir` e da `trusted_socket` lasciava le 28 verdi (la review l'ha visto). Ora lo tiene fermo **`scripts/gui.sh socket-owner`** (`lab/tests/gui-socket-owner.sh`, vedi «Il proprietario del socket con utenti veri» sotto).
- **Dentro l'app** (`lib.rs`, `main.rs`): `run()` legge gli argomenti, chiama `claim`, e restituisce `ExitCode` (`Handed` → 0, `Failed` → 1). `Alone` scrive «no single window: …; the app starts on its own» su `stderr` (S11). I file degli argomenti vanno a `offer_from_outside(…, Origin::Open)` nel `setup` e la pagina li trova con `import_pending` (T4); la prima istanza serve il canale sul runtime di Tauri, e per ogni consegna legge i file fuori dal thread del canale, poi `window::raise` (mostra, toglie dall'icona, `set_focus`). Un secondo avvio senza file porta solo la finestra davanti.
- **Associazione** (D31, Q41): `packaging/linux/io.github.zpeppoz.galleria.desktop` (`Exec=galleria-gui %f`, `MimeType=application/x-openvpn-profile;`) e `bundle.fileAssociations` in `tauri.conf.json` con la sola estensione `ovpn`; `bundle.active` resta `false`. Nessuna voce per `.conf`. `install.sh` non si tocca. `tests/config.rs`: `only_ovpn_is_associated`, `the_desktop_file_and_the_bundle_agree`, `bundle_stays_off`.
- `lab/tests/gui-lib.sh` esporta una `XDG_RUNTIME_DIR` (`0700`) dentro la cartella temporanea del giro, per fumo e Orca: le prove non toccano quella di chi le lancia.

**Punti [S].**

| # | Esito | Ripiego |
|---|---|---|
| S10 | **Vero su tutte e tre** (con `shared-mime-info` installato, `gio info -a standard::content-type` su un file `x.ovpn` non vuoto): Ubuntu 24.04 (2.4) `application/x-openvpn-profile`; **Ubuntu 26.04.1** (2.4-5build3) `application/x-openvpn-profile`; **Fedora 45** (2.5.1-3) `application/x-openvpn-profile`. Un `y.conf` dà `text/plain` ovunque: coerente con D31 (i `.conf` non si associano). Attenzione: un file **vuoto** dà `application/x-zerosize` (il tipo dipende anche dal contenuto, non solo dal nome) | nessuno: la definizione MIME nostra non serve |
| S11 | **Vero, provato.** Senza `XDG_RUNTIME_DIR` due processi partono tutti e due (`a_second_launch_without_a_runtime_dir_opens_a_second_window`), ognuno dice «no single window» su `stderr`. Con Q36 (b) il bus di sessione non serve | nessuno |
| S30 | **Vero.** `tauri-driver` passa all'app gli argomenti di `tauri:options.args`: `the_app_started_with_a_file_opens_its_preview` (anteprima di `inline.ovpn`, nome proposto «inline», niente sul disco prima di «Importa», poi il profilo è sul disco) | nessuno |
| S31 | **Chiamate fatte, effetto non verificato.** `window::raise` chiama `show`, `unminimize`, `set_focus` e nessuna dà errore, ma la prova di fumo asserisce solo che l'anteprima compare **nella prima finestra**, non che la finestra sia davanti né che abbia il fuoco. Sotto Xvfb senza gestore di finestre «davanti» e «fuoco» non si distinguono da come stavano. Su Wayland il compositore può negare il fuoco | resta «non verificato» (anche su un desktop vero) |
| S32 | **Vero, provato.** In una `XDG_DATA_HOME` temporanea con il file `.desktop` (la `Exec` punta al binario di prova) e un `mimeinfo.cache` scritto dalla prova (l'immagine non ha `desktop-file-utils`, cioè `update-desktop-database`): `gio info` dà il tipo al `.ovpn`, `gio mime application/x-openvpn-profile` elenca il `.desktop`, e **`gio launch <file.desktop> inline.ovpn`** lancia il binario con il percorso (`%f`): la prima finestra mostra l'anteprima e non resta un secondo processo | nessuno servito |

**Non provato**: un file manager vero (Nautilus, Dolphin) che apre un `.ovpn` con il doppio click (qui solo `gio launch`), Wayland, un `$XDG_RUNTIME_DIR` su un filesystem che non supporta `flock` (NFS), un `.desktop` installato dal sistema (il `mimeinfo.cache` lo scriviamo noi nella prova). Il secondo utente reale sul socket ora è provato (`scripts/gui.sh socket-owner`, sotto).

**Prove.** `tests/instance.rs` (28): `files_in_args_skips_flags_and_takes_paths_and_file_urls` (13 casi) più il non-UTF-8 e `runtime_dir`; `the_first_claim_is_primary_and_the_second_hands_its_files`, `a_second_launch_without_files_only_raises_the_window`, `the_lock_dies_with_the_process` (un processo figlio prende il blocco e viene ucciso: il `claim` successivo è `Primary` e sostituisce il socket rimasto), `without_a_runtime_dir_the_app_starts_alone`, `the_socket_is_private`, `a_connection_of_another_user_is_closed_without_being_read`, `a_socket_with_wider_permissions_is_not_trusted`, `oversized_slow_or_garbled_messages_are_dropped`, `a_primary_that_does_not_answer_makes_the_second_fail_loudly` (meno di 3 s), `a_lock_without_a_socket_is_waited_for_a_moment_and_then_it_fails`, `handed_files_get_the_same_checks_as_dropped_ones` (una FIFO come argomento: `gui_import_unsupported_file`, senza restare appesa), `a_channel_that_reads_no_more_than_the_limit_does_not_grow_with_the_input`. Fumo (`ui/e2e/smoke.e2e.ts`): le quattro del piano, tutte verdi al primo giro.

**Prove finali del gruppo B (T4 e T5).** Sull'ultimo commit, un giro solo ciascuno: `scripts/gui.sh check` verde (547 prove frontend, typecheck e2e, build, clippy, prove Rust); gate host verdi (fmt, clippy workspace esclusa GUI, test, deny, confini); fumo **32/32** (le 28 di prima più le quattro di T5); Orca **39/39** (`shell` 12, `rubrica` 11, `overlays` 11, `import` 5, con `I5e`). L'`a11y` col solo filtro `I5e` era stato lanciato due volte durante T4 (la prima rossa per l'asserzione sul testo «WG», corretta nella prova, non nel codice). Nessun codice è cambiato dopo il giro finale: l'ultimo amend tocca solo questo documento e l'handoff.

### Il proprietario del socket con utenti veri (dopo la review di B4)

La review di B4 ha tolto `meta.uid() != euid()` da `private_dir` e da `trusted_socket` e `tests/instance.rs` è rimasta 28/28 verde: l'immagine e le prove girano come **un solo utente**, quindi il confronto con l'uid del proprietario non ha mai un «altro» vero. Il comportamento era giusto (la review lo ha provato con due utenti veri e l'app vera); mancava la prova che lo tenesse fermo.

**`scripts/gui.sh socket-owner`** (`lab/tests/gui-socket-owner.sh` e `lab/tests/gui-socket-client.py`): costruisce l'app come per un rilascio (come l'utente che lancia, come ogni altra build), poi gira **come root del contenitore** (`GUI_UID=0`; per questo è un verbo di `gui.sh` e non una prova di `cargo test`) e avvia l'app vera con `setpriv` come **uid 1001** (la prima finestra), **1002** (l'altro utente) e root. Copia l'app e il client fuori dal repository: l'uid 1002 non entra nella cartella dell'utente. 36 controlli:

| Caso | Che cosa si vede |
|---|---|
| Un altro uid si collega al socket | con i permessi com'è: l'uid 1002 non arriva (`EACCES`); **root**, che i permessi non fermano, si collega e viene chiuso **senza `ok`** (`SO_PEERCRED`); allargati a mano (cartelle `0755`, socket `0666`), l'uid 1002 si collega e viene chiuso senza `ok`; il proprietario riceve `ok` |
| Un socket `0666` | il secondo avvio dello stesso utente esce con **1** e dice «the socket is not private to this user» |
| Un socket dell'uid 1002 con `0600` nella cartella del proprietario | il secondo avvio esce con 1 e dice lo stesso; rimesso al proprietario, il secondo avvio consegna il file ed esce con 0 (controllo: la prima finestra è ancora lì) |
| Una cartella `galleria` piantata dall'uid 1002 (`0777` e `0700`) in una cartella d'esecuzione che tutti scrivono | l'uid 1001 **e root** non la usano: nessun canale, l'app parte da sola e resta viva, «the folder is not a folder of this user», dentro non è scritto niente, e la cartella resta dell'uid 1002 con lo stesso permesso (root la stringerebbe a `0700` e ci aprirebbe il socket, se non guardasse l'uid) |
| L'uid 1002 punta alla cartella dell'uid 1001 | non la usa (`0700` e anche allargata) |
| Controllo | una cartella dell'utente con permessi troppo larghi viene riportata a `0700` e usata, il socket è `0600` dell'utente |

**Non può risultare verde saltando.** Ogni controllo conta, e a fine giro il numero deve essere quello atteso (`EXPECTED=36`): se uno salta o ne viene aggiunto uno senza cambiare il numero, esce con errore. Senza root esce con 2.

**Rossa con la mutazione** (una per volta su `instance.rs`, poi il file è tornato com'era; `git status` pulito):

| Mutazione | Esito |
|---|---|
| tolto `meta.uid() != euid()` da `private_dir` | **rosso**, 8 controlli (la cartella piantata `0777` dall'uid 1002 non è più rifiutata con il motivo giusto; con root, due file scritti nella cartella altrui) |
| tolto da `trusted_socket` | **rosso**, 1 controllo: il motivo non è più «not private to this user» ma «Permission denied», perché il kernel ferma l'uid 1001 sul socket `0600` dell'uid 1002 prima che la prova dell'uid arrivi a servire; l'uscita resta 1. Qui il controllo del codice e quello del kernel dicono la stessa cosa e la prova li distingue solo dal motivo |
| tutte e due | rosso, 9 controlli |
| tolto il controllo `SO_PEERCRED` della prima finestra | rosso, 4 controlli (root e l'uid 1002 ricevono `ok`) |

In CI la prova è un passo del job `gui-smoke` (stessa immagine, stessa toolchain, subito dopo il fumo). Non è girato su un runner (niente push): Docker senza rootless con `setpriv` e due uid veri dovrebbe andare, ma non è visto.

### G3 nel laboratorio: i profili importati dalla finestra, connessi dalla CLI (T6)

Scritto su **Q37 (a)**: la finestra gira nell'immagine `galleria-gui-dev` (Ubuntu 24.04) e importa i profili **veri** del laboratorio; la sua cartella dei profili passa ai tre client, dove la CLI e il demone veri li leggono e li connettono. Sui client non gira WebKit.

- **`scripts/gui.sh lab-import`** (immagine, sotto Xvfb, come il fumo; `lab/tests/gui-lab-import.sh` più `ui/e2e/lab-import.e2e.ts`): legge `base.ovpn`, `malicious.ovpn` e `wg-split.conf` da `lab/out/`, li importa **dalla finestra** (il primo come argomento del processo, gli altri due come secondo avvio, T5) e lascia la cartella in `target/gui-lab/config/galleria`. Per `malicious.ovpn` asserisce l'anteprima: la frase di D19 per `up` e `plugin`, una voce per ognuna delle **5** righe tolte (`up`, `plugin`, `status`, `writepid`, `setenv`), «Imported “malicious”. 5 directives removed.», nessun `PWNED-*` in `/tmp` del contenitore, nessuna di quelle righe nel profilo salvato. Se `lab/out/` non ha i profili, dice quale passo li genera (`lab/gen-pki.sh` per i `.ovpn`, il server WireGuard di `lab/up.sh` per `wg-split.conf`) e non li inventa. Il file `lab-import.e2e.ts` gira **solo** con `GALLERIA_LAB_IMPORT=1` (`ui/e2e/vitest.config.ts`): il fumo normale non lo prende.
- **`crates/galleria-e2e/tests/t17_gui_import.rs`** (ignorata come le altre; senza `GALLERIA_E2E_GUI_STORE=1` ogni prova si salta e lo dice. **Ma una prova che si salta risulta «ok»**: nella CI il job `lab` la lanciava così, e la review di B4 ha visto le quattro `gui_imported_*` «ok» in 0,00 s con «GUI store skipped» sui tre client (run 45735331757), cioè G3 verde senza essere eseguito. Perciò `lab/run.sh` **non esegue `t17_gui_import` senza la variabile** e lo dice in una riga: nel giro intero (il job `lab`) la salta, e `lab/run.sh 26 t17` senza la variabile esce con «no end-to-end tests matched» invece di dare «ok»): copia la cartella in `/home/alice/.config/galleria` (`alice`, `0700` e `0600`) e prova `gui_imported_profiles_are_listed_by_the_cli`, `gui_imported_wireguard_connects_from_the_cli`, `gui_imported_openvpn_connects_from_the_cli`, `gui_imported_malicious_profile_runs_no_script`. L'ultima connette il profilo malevolo **com'è dopo l'import** e registra ciò che il demone risponde; prima di dire «nessun `PWNED-*`» guarda `/tmp` del client **e** il `/tmp` privato del demone (`PrivateTmp=yes`), e una **prova di controllo** scrive un file `PWNED-control` nei due posti e verifica che lo sguardo li veda.
- **Divergenza dal piano: l'ordine.** Il piano metteva `lab-import` *prima* di `lab/run.sh`. Ma le chiavi WireGuard di `wg-split.conf` sono **nuove a ogni `lab/up.sh`** (il server le genera all'avvio), e `lab/run.sh` fa `up.sh` ogni volta: un profilo importato prima avrebbe chiavi vecchie. Perciò `lab/run.sh` ha **una seconda riga**, `LAB_BEFORE_TESTS` (un comando eseguito dopo `up.sh` e prima dei test), e `scripts/gui.sh lab-g3 <26|27|79>` fa tutto: `GALLERIA_E2E_GUI_STORE=1 LAB_BEFORE_TESTS='scripts/gui.sh lab-import' lab/run.sh <client> t17`. Senza `LAB_BEFORE_TESTS` e senza la variabile `lab/run.sh` resta com'era, salvo che salta `t17` e lo dice (sopra). **Nella CI** lo fa il job **`lab-g3`** (matrice `client: [26, 27, 79]`, dopo la review di B4): `LAB_BUILD=host scripts/gui.sh lab-g3 <client>`, con gli stessi passi di preparazione di `lab` (toolchain, cache, `modprobe wireguard`). La finestra gira nell'immagine come in `gui-smoke`; sul runner (uid 1001, Docker senza rootless) i profili di `lab/out` sono di root con `0600`, e `scripts/gui.sh lab-import` li legge da un contenitore con utente `0:0` quando l'utente dell'host non li può leggere (con Docker rootless, qui, resta l'utente dell'host: niente cambia). Il job **non carica il modulo `ovpn`**: il job `lab` fa passare `cli_openvpn_with_password_stdin` di `t16` su 27 e 79 senza il modulo (run 45735331757), quindi OpenVPN 2.7 arriva a `connected` anche così; se `lab-g3` mostrasse il contrario, si aggiunge il `sudo modprobe ovpn` di `lab-dco`. **Il job `lab-g3` non è mai girato** (niente push): è scritto, il YAML è valido, ma nessun runner l'ha eseguito. In più `scripts/lab.sh` che la card nomina **non esiste**: il laboratorio è `lab/run.sh`.

**Punto [S].**

| # | Esito | Ripiego |
|---|---|---|
| S33 | **Vero.** I profili di `lab/out/` sono dell'utente dell'host (Docker rootless mappa la radice del contenitore su di lui) e l'immagine della GUI gira come quell'utente: li legge, niente `docker cp` | nessuno |

**Esito sui tre client** (una volta ciascuno, `scripts/gui.sh lab-g3 <client>`; il giro vero è sull'ultimo commit):

| Client | OpenVPN | `list` | WireGuard | profilo malevolo | `PWNED-*` |
|---|---|---|---|---|---|
| 26 (Ubuntu 24.04, OpenVPN 2.6) | **verde**: `connected`, ping a `10.8.0.1` | verde | verde: ping a `10.99.0.1`, nessun residuo | **verde**: `connected` (tolte le direttive resta `base`), nessun residuo dopo `disconnect` | `find` vuoto (controllo visto), `openvpn --config /dev/stdin` |
| 27 (Ubuntu 26.04, OpenVPN 2.7) | **rosso: `Server unreachable after all attempts` (uscita 4)** | verde | verde | rosso, stesso errore: il demone non arriva a `connected`; **il `find` è vuoto** (controllo visto) e non gira nessun `openvpn` | vuoto |
| 79 (Fedora 45, OpenVPN 2.7) | **rosso, stesso errore** | verde | verde | rosso, stesso errore; `find` vuoto | vuoto |

Il rosso su 27 e 79 **non è di B4**: è il DCO del kernel che Docker rootless nega (`dco_new_peer: netlink reports error (-28): Operation not permitted`, `Cannot add peer to DCO`, nel giornale di `galleriad`), lo stesso limite già scritto nella roadmap per T13 e T16 («su 26.04 e Fedora OpenVPN 2.7 usa il DCO del kernel 7.3 e Docker rootless lo nega (EPERM), da rifare sulla CI»). Prova che è così: la prova già esistente `cli_openvpn_with_password_stdin` di `t16`, lanciata sullo stesso ramo e senza nulla di B4, dà **lo stesso errore, uscita 4, sul client 27**. Lo `OpenVPN` su 27 e 79 si rifà sul job **`lab-g3`** della CI (client 27 e 79, senza il modulo `ovpn`, vedi sopra), non qui; fino a che quel job non è girato verde su un runner la casella resta vuota.

**Cosa si è visto e che il piano non diceva.**

- Il profilo malevolo, importato dalla finestra e connesso dalla CLI sul 26, arriva a **`connected`**, come il piano si aspettava ma non aveva eseguito: tolte le direttive resta il profilo `base`. `openvpn` parte come `/usr/sbin/openvpn --config /dev/stdin`: la configurazione passa dallo standard input, quindi gli argomenti non dicono nulla delle direttive (il controllo su `--up`, `--plugin` e `--management 0.0.0.0` negli argomenti è debole; quello che conta è che le righe non sono nel profilo salvato e che nessun file `PWNED-*` compare, né nel `/tmp` del client né in quello privato del demone).
- `/tmp` del client contiene già, come sottocartella, il `/tmp` privato del demone (`/tmp/systemd-private-…-galleriad.service-…/tmp`): il `find` su `/tmp` basterebbe, e quello su `/proc/<pid>/root/tmp` è una seconda via sullo stesso posto, tenuta per chiarezza.
- Per la GUI il «Gtk-CRITICAL» di T3 non compare qui: nessuna finestra «Apri» è in gioco (i file arrivano come argomenti).

### B4 · chiusura: tastiera, fumo, screen reader (T7)

**Cosa ha trovato il giro da tastiera.** Un buco vero: con `Ctrl+O` premuto prima di qualunque `Tab` (il fuoco è sul `body`, come all'avvio) la finestra di import, chiusa con `Esc` o «Annulla», **ridava il fuoco al `body`**: la libreria lo restituisce a ciò che lo aveva, e allora non c'era niente. Succede anche con una finestra aperta da un file che arriva da fuori (secondo avvio, trascinamento) mentre nulla ha il fuoco. `focus_is_never_lost_to_the_body_during_the_import` è stata **rossa** («the focus after giving up the first step: expected body not to be body») e corretta in `Shell.tsx` (`closeImport`: se dopo la chiusura il fuoco è sul `body`, va a «Import…», il controllo che apre la finestra); poi verde. Nient'altro nei componenti.

**Prove nuove di T7.**

| Livello | Prova | Cosa dice |
|---|---|---|
| Vitest (`keyboard.test.tsx`) | `a_full_import_tour_needs_no_pointer` | `Ctrl+O`, testo, anteprima, un nome nostro e un gruppo scelto (`fireEvent.change` sul `select`: jsdom non lo guida con i tasti), «Importa» (fuoco sulla riga nuova, «Imported “Casa”.»); poi `Ctrl+O`, «Choose a file…» (la finestra di Rust è finta), «Cancel»: niente in lista, niente detto |
| | `focus_is_never_lost_to_the_body_during_the_import` | il fuoco non finisce sul `body` dopo: finestra annullata (primo e secondo passo), errore del testo, anteprima scaduta, offerta arrivata da fuori (anteprima e errore). **Rossa e corretta**, vedi sopra. Il piano diceva «esteso» il `focus_is_never_lost_to_the_body` di B2: è una prova sorella, perché ognuna di queste vie vuole un'app con un importatore suo |
| | `the_outcome_of_an_import_is_said_once_and_an_error_of_the_import_is_not_said` | al posto di estendere `no_outcome_depends_on_a_live_region_alone` (che resta com'è e controlla che nessun `ToastProvider` fuori da `design/` manchi di `announce`): l'esito con «2 directives removed» arriva a `announce` una volta, *polite*, con il testo dell'avviso; un errore dell'import non chiama `announce` e non fa comparire un avviso |
| Rust (`tests/import.rs`) | `no_import_command_returns_a_secret_after_a_full_round` | testo, file scelto (lasciato e poi salvato), file trascinato, file aperto, quattro errori (testo rotto, file mancante, fuori cartella, due file), `import_pending`, `import_commit` (anche su un'anteprima che non c'è più e con un nome rifiutato), `import_discard`, la lista e un `profile_get`: nessuna risposta e nessuno degli eventi sentiti dalla pagina contiene `SEED-SECRET-`, `PRIVATE KEY` o la cartella (I3). Per scelta non c'è un testo che nomina un file con `SEED-SECRET-` nel nome: `data.file` rimanda il nome del file **che l'utente stesso ha scritto**, come già ammette `no_import_command_returns_a_secret` |
| Fumo | `gui_import_equals_cli_import` | `inline.ovpn` (argomento del processo), `separate.ovpn` e `office.conf` (secondo avvio) importati dalla finestra e, in un'altra `XDG_CONFIG_HOME`, da `galleria import`: **i file dei profili sono uguali** salvo id e date, e **i file dei segreti uguali** (I6). Provata rossa cambiando a mano il file che importa la CLI (`office.conf` al posto di `inline.ovpn`: «the profile «inline» is not what the CLI saves»). Con `dns.ovpn` al posto di `inline.ovpn` resta **verde**, e non per un difetto: l'import toglie le due righe `up`/`down` e il risultato è lo stesso profilo |
| | `the_cli_lists_what_the_window_imported` | dopo due import, `galleria list` elenca i sette profili |
| | `import_works_with_the_daemon_stopped` | a demone fermo, dalla rubrica vuota: «Import a profile…», testo, nome, «Importa»; il pannello «Start the service» è ancora lì, un profilo e un file di segreti sul disco |
| | `no_csp_violation_during_the_import_tour` | testo, anteprima, «Back», «Cancel», file consegnato da un secondo avvio (profilo malevolo, salvato; hook di WireGuard, annullato; file non supportato): nessuna violazione |
| Orca (`import.a11y.ts`) | `I5f_an_error_of_the_import_takes_the_focus_and_is_not_announced` | un testo con un blocco `<key>` non chiuso: la nota dell'errore (`section`) **ha il fuoco** e dentro c'è l'avviso (`notification` con l'immagine «Error»: WebKitGTK esporta `role=alert` come `notification`); dopo 1,5 s il fuoco è ancora lì; **nessun** `object:announcement`; niente scritto |
| | `I5g_the_text_of_d19_is_in_the_tree_as_the_text_of_the_list` | il testo di D19 è nell'albero come **nome** delle voci dell'elenco: «Line 30 · Directive up removed: Galleria does not run the scripts of profiles. If the profile depends on it, it may not work fully.» (e `plugin`; `status`, `writepid`, `setenv`: «Galleria does not allow it in profiles.»): cinque voci, con il titolo «WHAT THE IMPORT CHANGED» (maiuscolo per il CSS, che l'albero riporta) |

**Cosa si è visto e che il piano non diceva.**

- **L'errore dell'import non è detto da Orca 46.1 nell'immagine.** Nel parlato registrato di `I5f` ci sono la finestra e il campo («Import profile dialog…», «Paste the text of the profile entry client.») e **nessuna riga con il testo dell'errore**, benché il fuoco sia sulla sua nota. L'albero è giusto (fuoco, `notification`, immagine «Error»). È la famiglia dei buchi già scritti in B2 e B3 (`S4b`, errore di campo `R5a`): il fuoco su un `div` non è letto da Orca 46.1 con WebKitGTK nell'immagine, mentre su un GNOME vero Orca 50.3 e 51.1 dicono i nomi dei campi (spike). **Non asserito**, come scrive il piano («parlato registrato»), e **non è un difetto da correggere in B4**: Q27 è chiusa (un errore prende il fuoco e non si annuncia). Va ricontrollato su un GNOME vero (B6, Q28).
- Il testo di D19 sta nel **nome** delle voci dell'elenco, non solo nell'interfaccia del testo: `atspi-dump.py` non ha avuto bisogno di una colonna in più (una prima prova con una colonna del testo è stata scritta e poi tolta).
- Il difetto trovato è lo stesso di B2 in un posto nuovo: chi apre una finestra senza che nulla abbia il fuoco.

**Criteri di uscita del blocco.**

| # | Esito | Prova |
|---|---|---|
| I1 | **Verde.** Testo, file scelto, file trascinato e file aperto (argomento, secondo avvio, `gio launch`) portano alla stessa anteprima; sul disco non cambia nulla prima di «Importa»; «Annulla», la chiusura e la scadenza non lasciano niente | `nothing_is_written_before_import`, `a_dropped_file_becomes_a_pending_preview` e le altre di `tests/import.rs`; `I5b`, `I5c`, `I5e`; `the_app_started_with_a_file_opens_its_preview` |
| I2 | **Verde sul 26; sul 27 e 79 verde per «nessuno script gira», rosso per la connessione OpenVPN (DCO, vedi T6).** Il testo di D19 è nell'anteprima, «N direttive rimosse» dopo l'import, lo script non gira mai | `the_malicious_profile_shows_the_d19_text_and_runs_nothing`, `I5g`; `t17`: `PWNED-*` vuoto sui tre client (con la prova di controllo) |
| I3 | **Verde** | `no_import_command_returns_a_secret`, `…_after_a_full_round`, `no_command_takes_a_path`, `the_page_never_listens_to_the_drag_events_of_tauri`; i permessi nuovi sono solo i comandi |
| I4 | **Verde, con le riserve di T5** (nessun file manager vero, nessun Wayland; il secondo utente reale sul socket è provato da `socket-owner`) | `tests/instance.rs`, `scripts/gui.sh socket-owner`, `a_second_launch_hands_the_file_to_the_first_window`, `a_second_launch_without_a_runtime_dir_opens_a_second_window`, `tests/config.rs` |
| I5 | **Verde, con un buco di Orca dichiarato** (l'errore non è pronunciato nell'immagine, vedi sopra): tastiera per ogni componente e per il giro intero; axe nei file di prova; esito detto **una volta**, *polite* (`(1, 0, Imported “separate”.)`, `(1, 0, Imported “office”. 1 directive removed.)`); l'anteprima è un `dialog` col fuoco su «Name»; un errore ha il fuoco | `keyboard.test.tsx`, `I5a`–`I5g` |
| I6 | **Verde.** `git diff main -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-cli` **vuoto**; in `galleria-profile/src` **202 righe aggiunte e 0 tolte**; ciò che la finestra importa è ciò che importa la CLI | gruppo dell'host verde; `gui_import_equals_cli_import` |
| G3 | **Verde sul 26, parziale su 27 e 79** (T6): finestra e CLI importano uguale, la CLI e il demone elencano e connettono ciò che la finestra ha importato, nessun `PWNED-*`; su 27 e 79 la connessione OpenVPN non arriva (DCO negato da Docker rootless: limite noto, stesso errore della prova `t16` già esistente); da rifare sul job `lab-g3` della CI, non ancora girato | `lab-import.e2e.ts`, `t17_gui_import` sui tre client, `gui_import_equals_cli_import` |

**Punti [S] di B4** (S9–S11 anche nella spec §17).

| # | Esito | Dove |
|---|---|---|
| S9 | **Vero**: i percorsi del trascinamento arrivano a Rust senza passare dalla pagina | T4 |
| S10 | **Vero** su Ubuntu 24.04, 26.04.1 e Fedora 45: `.ovpn` è `application/x-openvpn-profile`, `.conf` è `text/plain` | T5 |
| S11 | **Vero**: senza `XDG_RUNTIME_DIR` l'app parte e lo dice su `stderr` | T5 |
| S27 | **Vero con una correzione** (il campo del percorso compare scrivendo `/`, non con `Ctrl+L` via AT-SPI) | T3 |
| S28 | **Vero** al primo tentativo: il trascinamento vero sotto Xvfb senza gestore di finestre, con `Atspi.generate_mouse_event` | T4 |
| S29 | **Vero**: `Secret` si azzera quando cade (letto nel sorgente, non misurato) | T1 |
| S30 | **Vero**: `tauri-driver` passa gli argomenti di `tauri:options.args` | T5 |
| S31 | **Chiamate fatte, effetto non verificato**: `show`, `unminimize`, `set_focus` non danno errore; che la finestra salga davanti e prenda il fuoco non è provato (nessun gestore di finestre sotto Xvfb; su Wayland il compositore può negarlo) | T5 |
| S32 | **Vero con `gio launch`** (non con un file manager vero) | T5 |
| S33 | **Vero**: `lab/out/` è leggibile dall'immagine della GUI, niente `docker cp` | T6 |

**Trascrizione di Orca del giro** (Orca 46.1 nell'immagine, righe `SPEECH OUTPUT` di `target/gui-a11y/transcript.txt` così come sono, dal secondo giro).

```
## I5b the import of a file chosen in the real window
SPEECH OUTPUT: Imported “separate”.
SPEECH OUTPUT: landmark Profile list
SPEECH OUTPUT: Connections list box
SPEECH OUTPUT: separate.
SPEECH OUTPUT: vpn.fixture.example:1194 OpenVPN.
## I5d the preview of a file, as a dialog
SPEECH OUTPUT: Import… push button.
SPEECH OUTPUT: Import profile dialog Choose a file or paste the text of the profile. Nothing is saved until you choose Import.
SPEECH OUTPUT: Paste the text of the profile entry.
SPEECH OUTPUT: Choose a file… push button.
SPEECH OUTPUT: file chooser.
SPEECH OUTPUT: /
SPEECH OUTPUT: Files.
SPEECH OUTPUT: table with 1 row 4 columns.
SPEECH OUTPUT: text.
SPEECH OUTPUT: /tmp/tmp.gHVgkJi1el/fixtures/separate.ovpn.
## I5e the import of a dropped file
SPEECH OUTPUT: Galleria frame.
SPEECH OUTPUT: Name entry office.
SPEECH OUTPUT: Imported “office”. 1 directive removed.
SPEECH OUTPUT: office.
SPEECH OUTPUT: office.fixture.example:51820 WireGuard.
## I5f an error of the import
SPEECH OUTPUT: Import profile dialog Choose a file or paste the text of the profile. Nothing is saved until you choose Import.
SPEECH OUTPUT: Paste the text of the profile entry client.
SPEECH OUTPUT: client
```

(Le righe di `I5d` e `I5e` sono selezionate dal file: tra una e l'altra Orca dice anche altro, per esempio le finestre del trascinamento.) La riga dell'evento (file di debug di Orca): `EVENT MANAGER: Dequeued object:announcement for [frame: 'Galleria'] in [application: 'galleria-gui'] (1, 0, Imported “separate”.)` e `(1, 0, Imported “office”. 1 directive removed.)`: **una volta ciascuna**, *polite*. Per `I5f` non c'è nessuna riga `announcement`, e nemmeno una riga `SPEECH OUTPUT` con il testo dell'errore (vedi sopra).

**Numeri per livello** (sull'ultimo commit di codice; un giro ciascuno salvo dove scritto).

| Livello | Numero |
|---|---|
| Vitest (`scripts/gui.sh check`) | **550** prove in 44 file (547 dopo T5, 3 nuove di tastiera) |
| Rust, `galleria-gui` (`scripts/gui.sh check`) | **304** prove, 0 rosse (`import` 47, `instance` 28, `config` 12) |
| Rust, host (`cargo test --workspace --exclude galleria-gui`) | **582** prove, 0 rosse; fmt, clippy, `cargo deny`, confini verdi |
| Fumo (`scripts/gui.sh smoke`) | **36/36**, **tre giri di fila** (32 dopo T5, 4 nuove di T7) |
| Orca (`scripts/gui.sh a11y`) | **41/41**, **due giri di fila** (`shell` 12, `rubrica` 11, `overlays` 11, `import` 7: `I5a`–`I5g`) |
| Laboratorio (`scripts/gui.sh lab-g3 <client>`, `t17`) | **26: 4/4**; **27 e 79: 2/4** (elenco e WireGuard verdi; le due con OpenVPN rosse per il DCO) |

**Non verificato (B4).**

- **Il trascinamento da un file manager vero** (Nautilus, Dolphin) e **su Wayland**: provato solo con una finestra GTK che offre `text/uri-list` e un mouse sintetico via AT-SPI, sotto Xvfb senza gestore di finestre.
- **La finestra «Apri» sotto un portale del desktop** (GNOME, `xdg-desktop-portal`: un altro processo, un altro albero AT-SPI) e **su Wayland**; la scelta con il mouse in una cartella e la scelta di un file che il filtro nasconde.
- **Il doppio click in un file manager vero**: l'associazione è provata con `gio launch` su un `.desktop` scritto nella prova, non con l'app installata da un pacchetto (niente `desktop-file-utils` nell'immagine: il `mimeinfo.cache` lo scrive la prova).
- **La finestra portata davanti** (S31) e che prenda il fuoco, su Wayland e su un desktop vero.
- **Un secondo utente reale sul socket dell'istanza unica**: provato da `scripts/gui.sh socket-owner` (uid 1001, 1002 e root nell'immagine; vedi «Il proprietario del socket con utenti veri»). Non provato: che una connessione di un altro uid non faccia comparire un'anteprima (si vede solo che non riceve `ok`), un uid in una sessione di login vera (qui i due uid sono creati da `setpriv`, rootless li mappa con gli uid subordinati), e una cartella d'esecuzione su un filesystem senza `flock` (NFS). Il passo nel job `gui-smoke` non è girato su un runner.
- **L'import nella finestra su Ubuntu 26.04 e Fedora** (Q37 (a)): la finestra gira solo nell'immagine di Ubuntu 24.04; sui tre client girano la CLI e il demone. Resta alla passata manuale di B6. In B5 la finestra deve parlare con il demone vero e la domanda di Q37 torna.
- **La connessione OpenVPN dei profili importati su 26.04 e Fedora** (DCO negato da Docker rootless, lo stesso limite di T13 e T16): da rifare sul job **`lab-g3`** della CI (`scripts/gui.sh lab-g3 27` e `79`), che **non è ancora girato su un runner** (niente push). Il job `lab` non esegue più `t17`: prima lo «eseguiva» saltandolo e risultava verde.
- **Il tipo MIME dove non è stato guardato**: Ubuntu 24.04, 26.04.1 e Fedora 45 con `shared-mime-info` installato; non altri.
- **L'errore dell'import detto da Orca**: il parlato non c'è nell'immagine (Orca 46.1); su GNOME vero va ricontrollato.
- **Il job `gui` e `gui-smoke` di CI** con il codice di B4, e il job nuovo **`lab-g3`**: non girati su un runner (nessun push).
- **Il `Gtk-CRITICAL … WIDGET_REALIZED_FOR_EVENT` di T3** non è stato toccato.

## B5 · Connessione

Piano: `docs/superpowers/plans/2026-11-26-fase2-b5-connessione.md`. Q42–Q47 approvate da Aitano il 26/11/2026 su delega di Peppe, tutte come raccomandate: spec §18.

### Gruppo A · Archivio dei segreti

#### `KeyringSecretStore` e `LayeredSecretStore` in `galleria-profile` (T1)

- **`galleria-profile`, solo aggiunte** (Q29): `SecretStore::delete(id, key)` (una chiave che non c'è non è un errore; l'ultima chiave tolta da `FileSecretStore` toglie anche il file), `StoreError::KeyringLocked` e `StoreError::Keyring(String)`, `secrets_keyring.rs` (`keyring_present`, `KeyringSecretStore` con `open`, `labelled`, `with_prompt_timeout`; solo Linux), `secrets_layered.rs` (`LayeredSecretStore`, `Writes`, `StoreKind`). **Nessuna firma esistente cambia**; i tre archivi finti di `galleria-gui/tests/{rubrica,import,export}.rs` guadagnano `delete` (il tratto ha un metodo in più: chi lo implementa fuori dal repo deve aggiungerlo).
- **Il crate (Q44 (a))**: `dbus-secret-service` 4.2.0, feature `vendored`, sessione `Plain`, come dipendenza `cfg(target_os = "linux")`. **Otto voci nuove in `Cargo.lock`**: `dbus-secret-service`, `foreign-types`, `foreign-types-shared`, `openssl`, `openssl-macros`, `openssl-src`, `openssl-sys`, `vcpkg`. Le ultime sette stanno nel file di lock ma **non sono compilate** (`cargo tree -i openssl`: «nothing to print»): sono le dipendenze della feature `crypto-openssl`, che non è attiva. `dbus` e `libdbus-sys` c'erano già (per `tao`). `cargo deny check`: **advisories ok, bans ok, licenses ok, sources ok**, senza voci nuove in `deny.toml`.
- **S34 sciolta**: nella GUI `libdbus-sys` riceve insieme `vendored` (da `galleria-profile`) e `default` (da `tao`, letto con `cargo tree -i libdbus-sys -e features`), e `scripts/gui.sh check` **compila e passa** nell'immagine (tutti i test di `galleria-gui` verdi). Che la finestra **parta** con quella libdbus lo vede il fumo di fine gruppo.
- **Voce del portachiavi** (spec §9): attributi `service=galleria`, `profile=<uuid>`, `key=<secret_ref>`, tipo `text/plain`, collezione predefinita, etichetta «Galleria: <nome> (<chiave>)» (con `labelled`; senza, l'uuid). Ogni chiamata apre una connessione breve al bus, quindi lo store non tiene niente e si può condividere tra thread. `keyring_present` e `open` non aprono collezioni e non sbloccano: aprono una sessione sul bus (che può far partire il servizio) e basta; senza `DBUS_SESSION_BUS_ADDRESS` né `$XDG_RUNTIME_DIR/bus` rispondono subito «no» senza chiamare libdbus (che altrimenti proverebbe `dbus-launch`).
- **Regole dell'archivio a strati**: lettura prima il portachiavi, poi il file; **un portachiavi bloccato, o che sbaglia, è un errore e non manda al file** (una prova lo tiene fermo con `StoreError::Keyring` e con `KeyringLocked`). Scrittura (`Writes::Keyring`): senza portachiavi nel file; con il portachiavi lì, poi la copia nel file si toglie (se non si riesce a toglierla la scrittura vale lo stesso e la lettura dà il valore nuovo); bloccato → `KeyringLocked` e `secrets/` con gli stessi byte di prima. `Writes::File` (CLI): sempre il file. `delete` e `delete_all`: da tutti e due, e se il portachiavi è bloccato il file si ripulisce comunque e l'errore si riporta.
- **Prove** (`tests/secrets_layered.rs`, 15, senza bus, portachiavi finto in memoria con interruttore «bloccato» e «guasto»): le dieci del piano più `after_a_keyring_write_the_file_copy_is_gone`, `a_keyring_that_fails_is_an_error_not_a_silent_fallback`, `deleting_from_a_locked_keyring_is_reported_and_the_file_is_still_cleaned`. **Non ancora provato contro un portachiavi vero**: è il T2.
- **Limite noto**: `KeyringSecretStore::set` senza collezione predefinita dà `Keyring("no default collection")`, non la crea (crearla è una richiesta di password con finestra). `galleria delete` della CLI non conosce il portachiavi (D25: la CLI cancella dal file): un profilo importato dalla finestra e cancellato dalla CLI lascia le sue voci nel portachiavi.

#### Il portachiavi vero nell'immagine e in CI, il controllo dei gate, la CLI sui due archivi (T2)

- **Immagine** (`lab/images/gui.Dockerfile`): `gnome-keyring` e `libsecret-tools` (`secret-tool`), in un livello in fondo, così i livelli sopra restano nella cache. `scripts/gui.sh image` ricostruisce l'immagine (2 min 43 s, qui).
- **`scripts/gui.sh keyring`** (nuovo verbo; `lab/tests/gui-keyring.sh`): tre giri, ognuno con un bus di sessione suo (`dbus-run-session`) e una `XDG_DATA_HOME` sua, e con `GALLERIA_KEYRING_CASE`: **unlocked** (GNOME Keyring con `--unlock` e la password di prova su stdin: **S35 vera**, senza schermo), **locked** (un elemento messo prima, poi `Lock` con `gdbus`; **S37**: nell'immagine non c'è un prompter, `gcr-prompter` muore con «cannot open display» e lo sblocco fallisce subito, quindi `KeyringLocked` arriva in 50 ms; con un prompter che non risponde l'attesa massima è 60 s nell'app e 3 s nelle prove, **non provato**: niente nell'immagine lo simula), **absent** (un `dbus-daemon` con una configurazione senza cartelle dei servizi: **S36 vera**, `org.freedesktop.secrets` non c'è e non si attiva).
- **Le dieci prove** (le `#[ignore]` di `galleria-profile/tests/keyring.rs` e `galleria-cli/tests/keyring.rs`), tutte con `ok` nel log di `scripts/gui.sh keyring`: `keyring_roundtrip`, `keyring_items_carry_the_three_attributes_and_the_label` (riletti con `secret-tool search`: `service`, `profile`, `key` e l'etichetta «Galleria: <nome> (<chiave>)»), `keyring_delete_one_and_all`, `keyring_two_profiles_do_not_mix`, `keyring_value_with_newlines_survives` (un blocco PEM con accenti, e la stringa vuota), `keyring_locked_is_reported_within_five_seconds` (get di un elemento che c'è, get di uno che non c'è, set, delete: tutti `KeyringLocked`, e niente scritto), `keyring_present_does_not_unlock` (`Locked` dopo la domanda: ancora `true`), `keyring_absent_opens_as_none`, `the_cli_reads_what_the_keyring_holds` (la chiave nel portachiavi, il blocco `tls-auth` nel file: la CLI manda al demone finto tutti e due), `the_cli_without_a_keyring_reads_the_file`. Ogni prova **fallisce** se il caso non è il suo o se manca (`must_be`): non si salta da sola, e `cargo test --workspace` le vede `ignored`.
- **Una cosa vista strada facendo**: il log del giro va letto riga per riga e `test <nome> ... ok` deve stare su una riga sola; con l'stderr del bus nello stesso flusso, il rumore di `dbus-daemon` («Activating service name='org.gnome.keyring.SystemPrompter'…») si infilava tra `...` e `ok` e `check-gates.py` leggeva «dbus-daemon» al posto di «ok». Ora lo stdout del giro è il log e lo stderr va in `target/gates/keyring-<caso>.stderr`, mostrato solo se il giro fallisce.
- **`scripts/check-gates.py --job <job> [--kind K] [--cargo-log F]… [--vitest-json F]… [--lab-log F]…`** legge `lab/tests/gates-b5.tsv` (gate, job, tipo, nome): una prova **mancante, `ignored`, `FAILED`, `skipped`, `todo` o assente dal JSON** è un errore (uscita 1; 2 se il job non ha righe o un log non si legge). `scripts/tests/check-gates.sh` lo prova su uscite finte: `all_listed_tests_passed_is_green`, `an_ignored_test_fails_the_gate`, `a_missing_test_fails_the_gate`, `a_failed_test_fails_the_gate`, `a_skipped_vitest_test_fails_the_gate`, e altre cinque (11 controlli). **Provato sul vero**: nel log di `scripts/gui.sh keyring` ho messo `ignored` al posto di `ok` su `keyring_roundtrip` → uscita 1 («NOT PROVED (ignored)»); ho tolto la riga → uscita 1 («MISSING»).
- **`tests/config.rs`**: `every_b5_gate_has_a_job` (ogni gate in `GATES_WITH_PROOFS` ha una riga; ogni job del manifesto è in `ci.yml` e il suo **ultimo** passo chiama `check-gates.py --job <sé>`; **rossa** togliendo il passo da `ci.yml`, poi rimesso) e `every_cargo_proof_of_the_manifest_is_a_test_that_exists` (un nome sbagliato nel manifesto fallisce prima del push). **Da sapere per il gruppo D:** `GATES_WITH_PROOFS` oggi è `G5`, `G6`; ogni task che chiude un gate lo aggiunge con la sua prima riga, e a fine blocco la lista deve essere G4, G5, G6, G7, G9 (`ALL_GATES`).
- **CLI (D25)**: in `galleria connect` l'archivio è `LayeredSecretStore::new(KeyringSecretStore::open(), file, Writes::File)` (`layered_secrets` in `commands.rs`, l'unico punto toccato). `import` e `delete` come prima. **S45 sciolta**: senza `DBUS_SESSION_BUS_ADDRESS` né `$XDG_RUNTIME_DIR/bus` la CLI non chiama libdbus (altrimenti proverebbe `dbus-launch`), non aspetta e non stampa niente (`the_cli_without_a_session_bus_reads_the_file_and_stays_quiet`, nel job `check`, entro 2 s, stdout e stderr vuoti); con un bus **senza** servizio idem (`the_cli_without_a_keyring_reads_the_file`, nessuna riga nuova).
- **`scripts/gui.sh check-here`** tiene le uscite in `target/gates/` (`gui-cargo.log`, `gui-vitest.json`: 550/550) per il job `gui`, che avrà righe nel manifesto più avanti. **CI**: il job `check` tiene l'uscita di `cargo test` e finisce con `check-gates.sh` e `check-gates.py --job check`; `gui-smoke` guadagna `scripts/gui.sh keyring` e `check-gates.py --job gui-smoke` come ultimo passo. **Nessuno dei due è girato su un runner.**
- **KWallet (S38), tetto di un'ora, usati ~15 minuti: NON verificato su KWallet.** In un contenitore usa e getta Ubuntu 26.04 (`kwallet6` 6.24.0, il nome del pacchetto è questo, non `kwalletd6`): `ksecretd` (il demone che offre `org.freedesktop.secrets`) **non si attiva da solo** (il file `.service` dice `org.kde.secretservicecompat`, non `org.freedesktop.secrets`), a mano parte e prende il nome ma chiede `libqca-qt6-plugins`, e con quello `secret-tool store` **resta appeso** («Using kwallet without parent window!»): per creare il portafoglio serve una finestra della password. Non è automatizzabile senza schermo, quindi **non verificato su KWallet**: va alla passata manuale di B6. Non entra nell'immagine né in CI.
- **Non provato (T2)**: GNOME Keyring con un prompter che non risponde (l'attesa di 60 s); la sessione `Plain` su KWallet; un portachiavi su un runner di CI (nessun push).

#### La GUI scrive nel portachiavi; avviso del ripiego; `secret_store_status` (T3)

**STATO: fatto, ai tre livelli.** Rust e Vitest verdi; fumo **38/38** (con le due prove nuove `an_imported_profile_lands_in_the_keyring` e `without_a_keyring_the_window_says_so_and_writes_the_file`) e Orca **41/41** (shell, rubrica, overlays, import), un giro di `smoke` e uno di `a11y` verde sul commit di correzione (vedi «Allineamento di fumo e Orca» sotto).

- **`galleria-gui`**: `Rubrica::new` usa l'archivio a strati (`Writes::Keyring`, `layered_secrets` in `rubrica.rs`: l'**unico** posto che costruisce un `FileSecretStore` nel crate); `Rubrica::delete` chiama `delete_all` sull'archivio a strati **prima** di `ProfileStore::delete` (un portachiavi bloccato ferma l'eliminazione e il profilo resta). Il tipo `SecretFactory` prende anche il **nome del profilo** (l'etichetta «Galleria: <nome> (<chiave>)» delle voci), e `StoreProbe` dice dove va una scrittura senza aprire niente; costruttori nuovi `with_archive` e `with_file_secrets` (le prove che non riguardano il portachiavi lo usano, così non toccano mai quello di chi le lancia: sei punti nei test). Un profilo **rinominato** dopo il salvataggio tiene l'etichetta vecchia nelle sue voci.
- **Comando `secret_store_status() -> { store: "keyring" | "file" }`** (`commands/secrets.rs`, in `build.rs`, `capabilities/main.json`, `tests/config.rs`): chiede al bus solo se il servizio c'è. Codici nuovi `gui_keyring_locked` (errore di `KeyringLocked`, da import, «Duplica», «Esporta», «Elimina») e `gui_keyring_failed`, in `GUI_CODES` e nei due cataloghi.
- **Pagina**: `secrets/store.ts` (`useSecretStore`, letto all'avvio e quando la finestra torna in primo piano; una risposta che non è una delle due parole vale «non so» e non si dice niente), avviso **fisso e non chiudibile** in `RubricaNotices` quando è `file` (tono `warn`, `status`, non `alert`); l'anteprima dell'import dice «nel portachiavi di sistema» o «in un file leggibile solo da te» (e la frase generica finché non si sa); `ProfileDetail` mostra per `gui_keyring_*` da Duplica/Esporta/Elimina un avviso **in linea (`alert`) che prende il fuoco**, senza toast e senza annuncio, e lo toglie al tentativo dopo o cambiando profilo. L'errore dell'import usa l'avviso già esistente (che prende il fuoco).
- **Prove** (`tests/secrets.rs`, 13, portachiavi finto che conta letture, scritture e richieste di sblocco): `import_commit_writes_the_keyring_and_no_secret_file`, `without_a_keyring_import_writes_the_file_and_the_status_says_file`, `a_locked_keyring_stops_the_import_and_leaves_nothing`, `another_error_of_the_keyring_has_its_own_code`, `a_duplicate_stopped_halfway_leaves_nothing_in_either_store` (la seconda scrittura rifiutata: né profilo né voci in nessuno dei due), `delete_removes_the_keyring_items_too`, `a_locked_keyring_stops_the_delete_and_leaves_the_profile`, `a_profile_imported_by_the_cli_is_read_from_the_file` (G6, metà: export e duplica leggono dal file, il duplicato va nel portachiavi), `secret_store_status_never_reads_a_secret`, `starting_the_app_never_asks_to_unlock` (portachiavi **bloccato**: avvio, `profiles_list`, `profile_get`, `secret_store_status` danno `(0, 0, 0)` letture, scritture e richieste di sblocco), `secret_store_status_says_where_a_write_goes`, `no_command_returns_a_secret_with_the_secrets_in_the_keyring`. Vitest: `RubricaNotices.test.tsx` (`the_file_fallback_notice_stays_and_cannot_be_dismissed`, `no_notice_with_a_keyring`, …), `Preview.test.tsx` (`the_preview_says_where_the_keys_will_go`, al posto di `found_secrets_are_named_and_go_to_the_secret_store`), `ProfileDetail.test.tsx` (due prove del portachiavi bloccato), `secrets/store.test.ts` (3). Le attese di `App.test.tsx` e `Shell.test.tsx` sui comandi chiamati ora contano anche `secret_store_status`.
- **Allineamento di fumo e Orca (fatto)**: (1) `lab/tests/gui-lib.sh` ha `gui_start_keyring` (un GNOME Keyring sbloccato del giro, dati nella cartella temporanea; `GALLERIA_SMOKE_KEYRING=absent` lo lascia fuori), chiamato da `gui-smoke.sh` e `gui-a11y.sh`; (2) un lanciatore `app-no-keyring` (`GALLERIA_SMOKE_APP_NO_KEYRING`, bus di sessione che non c'è) apre la finestra come una sessione senza portachiavi: lo usano `lab-import` (G3: `t17` legge i segreti dal file) e, nel fumo, `gui_import_equals_cli_import` e la prova del ripiego (`openApp(..., "none")`); (3) attese aggiornate: «saved in the system keyring», `secrets/` resta a 5 file, `import_works_with_the_daemon_stopped` a 0, e in Orca `I5b` (testo dell'anteprima e `secrets()` invariato). **Orca, `I5d`: in un giro su tre ha fallito** (il parlato non conteneva «Import profile» alla prima apertura del dialogo: «Galleria frame.» al suo posto); stesso codice e stesse prove, verde al giro dopo. È l'attesa sul parlato (registrato, e asserito solo per il nome della finestra), non il portachiavi: da tenere d'occhio, non corretto.

### Gruppo B · Connetti

#### `connect_profile`, `disconnect`, sostituzione e annunci di stato, in Rust (T4)

- **Moduli senza `tauri`**: `calls.rs` (una connessione breve per chiamata: apri, `hello`, chiama, chiudi; `CallError::{Unreachable, Rpc}`), `session.rs` (`Sessions`: `connect_profile`, `disconnect`, `set_phrases`, `on_state`; `SessionFailure`), `rubrica/connect.rs` (`Rubrica::connect_params`: la stessa `to_connect_params` della CLI, sugli stessi archivi). `commands/connection.rs`: `connect_profile(id, replace)`, `disconnect()`, `announce_phrases(phrases)`, tutti `()` o errore, senza stato nella risposta (arriva con gli eventi). In `build.rs`, `capabilities/main.json` e `tests/config.rs`.
- **`already_active`** esce con `data: { profile: { id, name }, foreign }`. `foreign` lo decide Rust cercando l'id nella rubrica **al momento della risposta**; una rubrica che non si legge, un id che non è un uuid o un profilo che non c'è valgono «estranea». Nessun comando disconnette da solo: la sostituzione è sempre una seconda chiamata `connect_profile(…, true)` fatta dalla pagina.
- **Codici nuovi** (in `GUI_CODES` e nei due cataloghi): `gui_secret_missing` (una chiave che il profilo nomina non è nell'archivio; `RubricaError::SecretMissing`) e `gui_daemon_unreachable` (la connessione breve non si apre). Gli altri passano come sono: `gui_keyring_locked` (portachiavi bloccato: il demone non riceve niente), i codici del demone.
- **Annunci (Q43/P14)**: la pagina dà a Rust le frasi (`announce_phrases`: `ui/src/app/announce.ts`, `usePhrasesForRust`, all'avvio e a ogni cambio di lingua; testi `announce.*`, «{name}» resta nel testo e lo mette Rust). `TauriSink::state` chiama `Sessions::on_state` **prima** di emettere `galleria:state`: annuncia quando cambia la terna (stato, id del profilo, codice d'errore), `connected`/`disconnected`/`reconnecting` *polite*, `error` *assertive* con il testo del codice, `connecting`, `needs_input` e `disconnecting` mai; senza frasi niente. Le frasi sono ripulite con `announce::clean` e tagliate a 200 caratteri. La pagina non chiama `announce` per gli stati (`state_events_do_not_call_announce`).
- **«Ultimo uso» (P15)**: una sessione di un profilo della rubrica che arriva a `connected` scrive `state.toml` (una volta per sessione); una estranea non scrive niente.
- **Scelta**: `setup` ora mette nello stato rubrica, annunciatore e `Sessions` **prima** di far partire il supervisore (prima erano dopo): gli eventi li incontrano subito.
- **Scenari del demone finto** (`tests/support/scenarios.rs`: `plain`, `credentials`, `otp`, `passphrase`, `auth-failed`, `unreachable`, `stats`, `slow`) e l'`Harness` che registra i parametri di ogni `connect` e i valori di ogni `provide_input` ricevuti dal backend finto.
- **Prove** (`tests/connection.rs`, 22): le venti del piano (con `disconnect_without_a_session…`: il demone **non** risponde `not_active` a una `disconnect` senza sessione ma `()`, quindi la prova è «risponde `()`»), più `a_phrase_is_cleaned_and_capped_before_it_is_said` e `the_capture_of_stderr_sees_what_is_written_to_it` (la cattura di `stderr` con `dup2` vede ciò che scrive un thread; serve `nix` con `fs`, solo come dev-dependency, nessun crate in più nel lock). `stderr_never_carries_connect_params` cattura il vero `stderr`. Nessuna prova del piano è stata saltata o resa meno forte, tranne la scelta della sonda del collegamento: ora i test aspettano `daemon_view` e non l'evento `galleria:link` (un evento emesso prima che l'ascoltatore ci fosse poteva mancare: 4 giri su 14 si fermavano a 8 s; dopo il cambio 14 giri su 14 verdi).
- **Pagina**: `daemon/commands.ts` ha `connectProfile` e `disconnectSession` (il test di confine vuole che ogni comando del permesso sia invocato da qualche parte della pagina).
- **Fatto quando**: `cargo tree -p galleria-gui -e normal` senza `galleriad` né backend; `ConnectParams` compare solo in `calls.rs` e `rubrica/connect.rs`, in nessun `Debug`, `eprintln!` o payload.

#### Pulsante, sette stati, statistiche, errori tradotti (T5)

- **Pagina** (`ui/src/connection/`): `ConnectionProvider` (sotto i toast, sopra la finestra) dà a tutta la finestra il collegamento al servizio e la sessione **come li portano gli eventi**; `ConnectButton` («Connetti», «Annulla» in `connecting`/`needs_input`/`reconnecting`, «Disconnetti», spento in `disconnecting`, «Riprova» e «Chiudi» in `error`; occupato tra il click e il primo evento, senza servizio spento con `aria-describedby` che dice perché); `connectOrReplace` (D28: seconda chiamata con `replace: true` solo se Rust dice `foreign: false`, un solo avviso «Disconnesso da «X»»; con `foreign: true` l'errore in linea, la conferma è del T8); `CommandNotice` (errore di comando in linea, prende il fuoco, non annunciato); `Stats` (la fascia: stato, indirizzo, traffico, handshake o durata; non è una regione viva; `title` col valore esatto); `StatusPanel` (stato, `detail` tradotto, tentativo, rete in sola lettura, errore col testo del catalogo o il messaggio del servizio); `StatusBar` con la sessione. `screens/StatusView` è diventato `screens/BackendWarnings`. `Invio` sulla riga già scelta fa l'azione principale.
- **Handshake solo se il backend lo dichiara** (`capabilities.handshake`); altrimenti la cella è la durata. **Il demone non valorizza `Status.since`** (mai impostato in `galleriad`): la durata conta da quando la finestra ha visto la sessione `connected` (se la finestra si apre a sessione già attiva, parte da lì).
- **Elenco dei `detail` (P10)**: sono i nomi di stato di OpenVPN che il backend passa come `Progress` — `CONNECTING`, `RESOLVE`, `TCP_CONNECT`, `WAIT`, `AUTH`, `AUTH_PENDING`, `GET_CONFIG`, `ASSIGN_IP`, `ADD_ROUTES` — tradotti in `state.detail.<nome>`; WireGuard non ne manda; un valore sconosciuto si mostra com'è.
- **Prove**: Vitest 609/609 (nuove: `ConnectButton.test.tsx`, `store.test.tsx`, `StatusPanel.test.tsx`, `Stats.test.tsx`, `a_connection_needs_no_pointer`; aggiornate le attese di tastiera, comandi chiamati e fascia). `scripts/gui.sh check` verde.
- **Fumo** (`ui/e2e/connect.e2e.ts`, scritto dopo, con il demone finto e `daemon-start <scenario>`): `a_profile_connects_and_disconnects`, `the_numbers_move_while_connected`, `switching_profile_replaces_without_asking`, `an_unreachable_server_shows_the_translated_error_in_both_languages`, e in `smoke.e2e.ts` «Connect» c'è ed è spento in `rubrica_works_with_the_daemon_stopped`. Non c'è un pulsante nel menu della riga (non c'è un menu di riga: c'è `Invio` sulla riga scelta).

#### Credenziali su `needs_input`, «Ricorda», «Dimentica» (T6)

- **Rust** (senza `tauri` salvo l'involucro): `calls::provide_input` (consuma i parametri e li lascia cadere), `Sessions::{provide_input, forget_credentials, has_pending}`, `Rubrica::{remember_credentials, forget_credentials}` e `connect_params` che aggiunge utente e password ricordati; comandi `provide_input(session, request, values, remember) -> ()` e `forget_credentials(id) -> ()` (il secondo su `spawn_blocking`: il portachiavi può aspettare lo sblocco). `values` arriva come `InputValues` del demone, cioè come `Secret`: nessuna `String` intermedia nel codice nostro.
- **Il viaggio di una credenziale**, tappa per tappa e con la prova che la tiene ferma: (1) campi non controllati, nessuno stato nel file (`the_credentials_dialog_keeps_no_value_in_state`: niente `useState`/store/`console.`; `"provide_input"` compare in un solo file della pagina, `daemon/commands.ts`, e `provideInput(` è chiamata solo da `ConnectionProvider`); (2) i campi si svuotano prima di aspettare la risposta e la chiamata è una sola (`the_fields_are_empty_before_the_answer_arrives`, `provide_input_is_invoked_once_per_submit`); (3) il parametro è `InputValues`; (4) `provide_input_reaches_the_daemon_and_nothing_else`; (5) «Ricorda» tiene in `Pending` solo utente e password di una domanda di `credentials`, **prima** della chiamata (un `connected` più svelto della risposta lo trova) e li scrive a `connected` della stessa sessione, *prima* che la pagina senta l'evento; a `error`, `disconnected`, `disconnecting` o a un'altra sessione (anche solo l'avvio di una `connect_profile`) li butta senza scrivere (`remember_saves_only_after_connected`, `a_failed_login_saves_nothing`, `a_disconnect_before_connected_saves_nothing`, `otp_and_passphrase_are_never_kept`, `pending_is_dropped_when_another_session_starts`); (6) la risposta è `()` e nessun evento ripete i valori (`no_command_returns_a_secret` e `no_event_carries_a_secret` ora fanno un giro con password, codice e passphrase, in tre scenari, e cercano i tre valori in ogni risposta e in ogni evento; `no_debug_text_carries_what_was_typed`).
- **Limite dichiarato** (le stesse parole del piano): le copie che non sono nostre non si azzerano. La stringa JavaScript del campo vive finché il motore non la raccoglie; il processo di WebKit e il `serde_json::Value` di Tauri tengono una copia per la durata della chiamata. «Non più del necessario» qui vuol dire: nessuna copia **in più** fatta dal nostro codice, e nessuna che sopravviva alla chiamata per colpa nostra. L'alternativa che evita la pagina del tutto (una finestra GTK aperta da Rust, come «Apri» e «Salva») è fuori da B5: la spec §3.2, regola 2, mette la finestra delle credenziali nella pagina.
- **Archivio** (Q46 (a), Q47 (a)): le chiavi sono `openvpn.username` e `openvpn.password`; i due nomi entrano in `secret_refs` del profilo **senza cambiare `modified`** (la lista non si riordina a ogni connessione). Alla connessione Rust le legge e le mette nei parametri; se il profilo le nomina e l'archivio non le ha più **non è un errore** (P5): non si mandano, il demone chiede, e i due nomi escono dal profilo. «Dimentica» toglie le due chiavi e i due nomi, e lascia le chiavi del profilo (`openvpn.block.*`); se il portachiavi non risponde i nomi restano.
- **Pagina**: `CredentialsDialog` (modale, solo il campo chiesto per `kind`, il `prompt` del server com'è come etichetta del codice, `echo: false` è `type="password"`, «Ricorda» solo per `credentials` e spento la prima volta, conto alla rovescia in testo ogni 10 s e nell'ultimo minuto ogni secondo, non annunciato; prende il fuoco sul primo campo; un click fuori non la chiude, «Annulla» e la «Chiudi» in alto sono `disconnect`; a zero si chiude da sola). Per questo `Dialog` ha due prop nuove, `ignoreOutsidePress` e `initialFocus`. La domanda (`Question`) e l'«è già stata risposta» stanno in `ConnectionProvider`, che segna la domanda risposta **prima** di aspettare qualunque cosa: una domanda risposta non si ripresenta, una `request` nuova (anche a connessione avvenuta) sì. «Dimentica» sta nella scheda Autenticazione (solo con le credenziali salvate) e nel pannello di un `auth_failed` del profilo scelto che le ha (Q47); il rubrica store ha `refreshDetail`, chiamato quando una sessione arriva a `connected` e dopo «Dimentica», perché `secret_refs` cambia senza che `modified` cambi.
- **Scelte e scarti dal piano**: (a) la finestra mostra sempre «Utente» e «Password» insieme: il caso «utente ricordato e manca solo la password» non può nascere, perché i due nomi entrano e escono insieme; (b) l'offerta «Dimentica» nel pannello di errore c'è solo se il profilo della sessione è quello scelto (è l'unico di cui la pagina ha `secret_refs`); (c) scenario nuovo `stuck` nel demone finto (domanda di credenziali, poi non arriva mai a `connected`): serve a provare «risposto e non ancora connesso»; (d) le prove di Rust di T4 e T6 condividono la finestra di prova (`tests/support/world.rs`, spostata da `tests/connection.rs`, nessuna prova cambiata).
- **Non cambiato**: «Duplica» fallisce con `gui_secret_missing`-equivalente se un nome in `secret_refs` non ha più la chiave nell'archivio (il caso P5 è coperto solo alla connessione).
- **Prove**: `tests/credentials.rs` 12; Vitest 635/635 (`CredentialsDialog.test.tsx`, `Dialog.test.tsx`, `Authentication.test.tsx`, `StatusPanel.test.tsx`, `boundary.test.ts`); fumo `ui/e2e/connect.e2e.ts` (`credentials_are_asked_once_and_remembered`, `the_otp_is_asked_every_time_and_never_saved`, `no_typed_value_is_left_in_the_page`).

#### Chiusura del gruppo B

- **Fumo**: `scripts/gui.sh smoke` **45/45** (due file: `connect.e2e.ts` con le sette prove di T5 e T6, `smoke.e2e.ts` con le altre 38). **Orca/a11y**: `scripts/gui.sh a11y` **41/41, verde due giri di fila** (shell, rubrica, overlays, import).
- **Cosa ha trovato il giro finale** (due prove di B1–B4 che non sapevano del pulsante di T5, nessun difetto del prodotto): il giro di tastiera di `smoke.e2e.ts` (`shell_is_navigable_with_the_seeded_rubrica`) e `S3_tabs_are_a_page_tab_list…` di `shell.a11y.ts` contavano i `Tab` da «Manage groups» a «More actions» senza il pulsante «Connect», che ora sta in mezzo: l'ordine è «Manage groups», stella, «Connect», «More actions», le schede. Corrette le due; `S3` ora controlla anche che «Connect» sia un `push button` col fuoco. In `credentials_are_asked_once_and_remembered` le intestazioni delle schede sono in maiuscolo (`text-transform`), la prova le cerca senza badare alle maiuscole.
- **Gate dell'host** sull'ultimo commit: `cargo fmt --check`, `clippy` del workspace senza la GUI, `cargo test` del workspace senza la GUI (598 passati, 0 falliti), `cargo deny check`, `check-boundaries`: verdi; `scripts/gui.sh check` verde (clippy, `cargo test -p galleria-gui`, Vitest 635/635, typecheck, build). I crate della fase 1 non sono toccati da questo gruppo (il diff verso `main` su `galleria-cli` e `galleria-profile` è quello del gruppo A, T1–T2).
- **Non provato qui**: «Ricorda» contro un portachiavi **bloccato** che chiede lo sblocco (l'archivio delle prove è quello di GNOME Keyring già sbloccato del fumo); il gruppo D lo copre con G5. La finestra delle credenziali non ha ancora una prova con Orca (è di T11).

### Gruppo C · Log e sessione estranea

#### Scheda Log con «Copia» ed «Esporta log…» (T7)

- **Rust** (senza `tauri` salvo l'involucro): `calls::logs` (una pagina del buffer del demone), `logs.rs` (`all` e `all_with`: pagine da 1000 una dopo l'altra con `after = last` finché una pagina è corta, `render` nel formato di `galleria logs` — `ts Level Source: message` —, `suggested_name`, `save`), `commands/logs.rs` (`logs_get(after, limit)` e `logs_export()`). `Sessions::socket()` è l'unica aggiunta a `Sessions`.
- **`logs_export` non ha parametri e non risponde con un percorso**: Rust riscarica **tutto** il buffer dal demone, chiede il posto con il `SavePicker` di B3 (la finestra «Salva» di GTK) e scrive con `write_export` (`0600`, file temporaneo e rinomina). Risposta `{ saved }`; un annullo è `{ saved: false }` e non scrive niente. L'errore di scrittura è `gui_export_failed` con il posto, come per un profilo. La finestra «Salva» aspetta una persona: gira su `spawn_blocking`, non su un thread dell'async runtime.
- **`logs_get` rifiuta `limit` 0 o oltre 1000** con `gui_invalid_field` (campo `limit`) **prima** di chiamare il demone (la prova lo fa senza demone). Nessun codice nuovo: `gui_invalid_field`, `gui_export_failed` e `gui_daemon_unreachable` c'erano.
- **Pagina** (`ui/src/log/`): `LogTab` per il profilo della sessione attiva o dell'ultima. Titolo «Sessione corrente» se lo stato non è `disconnected`, «Ultima sessione» altrimenti; per un altro profilo «Nessuna sessione per questo profilo» e nessun pulsante. L'elenco è `role="log"` con **`aria-live="off"`** (un `log` è *polite* di suo e leggerebbe ogni riga), `tabIndex=0`, nome dal titolo; livello e sorgente sono parole (`info`, `daemon`), l'ora è locale; le righe non si traducono. Resta in fondo solo se la persona era in fondo. Al più 2000 righe (le ultime). Le righe che arrivano a raffica aspettano 40 ms e si disegnano insieme (un burst di 2100 righe è un disegno, non 2100).
- **«Copia»** copia ciò che la scheda mostra nello stesso formato del file; **«Esporta log…»** non prende niente dalla pagina. Ognuno dice l'esito con un avviso («Log copiato.», «Log esportato.»), una volta.
- **Scarto dal piano**: il nome proposto è nell'ora **UTC** (`galleria-log-AAAAMMGG-HHMM.txt`): `time` è nel workspace con la sola feature `formatting`, e `now_local` non è sicuro con più thread. Se un giorno serve l'ora locale è una feature di `time` e una riga in `commands/logs.rs`.
- **Scarto dal piano**: «Ultima sessione» ha bisogno di sapere di che profilo era l'ultima sessione, e il demone a sessione finita risponde `Status` vuoto. Lo ricorda `ConnectionProvider` (`lastSession`: id, nome e numero della sessione dell'ultimo stato non `disconnected`). Una finestra aperta *dopo* la sessione non lo sa: il buffer del demone ha ancora le righe, ma la scheda dice «Nessuna sessione per questo profilo».
- **Prove**: `tests/logs.rs` 11 (le sei del piano, più la lettura a pagine contro il demone vero con 2100 righe, `logs_get_answers_a_page_of_the_daemon`, l'errore di scrittura, `render` e il nome); `G7` in questo giro: `the_export_never_carries_a_known_secret` (scenario nuovo `leaky`: il backend finto ripete la chiave privata e la condivisa in una riga di log, nel file c'è `[redatto]` e nessun `SEED-SECRET-`, né nella pagina del log né negli eventi). Scenari nuovi `chatty` e `leaky` in `tests/support/scenarios.rs`; `Options.picker` in `tests/support/world.rs`. Vitest: `LogTab.test.tsx` 21 (tra cui i sette stati, il tetto di 2000, i duplicati e le righe che arrivano prima della pagina che le ha). Fumo `the_log_tab_shows_the_session_and_copies_it` (con `xclip -o`), Orca `K5e_the_log_is_exported_through_the_real_save_window`: file `0600`, il nome proposto è quello di P12, «Log exported.» detto **una** volta, e Orca non legge le righe.
- **Fatto quando**: `grep -rn "eprintln" crates/galleria-gui/src/logs.rs crates/galleria-gui/src/commands/logs.rs` non trova niente.

#### Sessione estranea (T8)

- **Estranea** = `Status.state != disconnected` e il suo profilo non è tra i profili della rubrica. Lo calcola `ConnectionProvider` (nuovo prop `ownIds`, confrontato per valore: una rubrica riletta con gli stessi profili non ridisegna la finestra) ed espone `foreign`. **La rubrica non ancora letta non dice nulla**: la riga aspetta, per non lampeggiare. Un profilo eliminato dalla CLI con la sessione su rende estranea la sua sessione; un profilo che entra nella rubrica fa sparire la riga. La decisione che conta per la sostituzione resta quella di Rust (`already_active` con `foreign`).
- **Riga fissa** (`ForeignRow`) sopra la ricerca e l'elenco, fuori dai gruppi, con nome, protocollo, stato in parole e «avviata fuori da questa rubrica»; «Disconnetti» sta sotto il nome (a fianco il nome si tagliava a 72 px). Si sceglie e il dettaglio (`ForeignDetail`) ha il nome, la fascia dei numeri e **la sola scheda Log**; il pannello di stato è quello sopra il profilo, che ogni sessione ha. Nessuna scheda del profilo, nessun «Elimina», «Duplica», «Esporta». Scegliere la riga chiede prima, come tra due profili, se c'è una modifica non salvata. Barra in basso: «Connessione attiva: «nome» — avviata fuori da questa rubrica».
- **Conferme** (`ReplaceConfirm`, un `AlertDialog` del design system, fuoco su «Annulla», non annunciano): «Disconnetti» sulla riga e nel dettaglio chiede «Questa connessione non è partita da questa rubrica. Un altro utente di questo computer potrebbe starla usando. Disconnettere?» e solo dopo chiama `disconnect`. «Connetti» su un profilo proprio con una sessione estranea attiva riceve `already_active` con `foreign: true` e apre la stessa conferma con «Sostituisci»: **una** chiamata con `replace: false`, poi, dopo il sì, una con `replace: true`; con il no non si connette niente e non c'è un errore in linea. Prima la sessione estranea si vedeva come errore in linea (B5 T5, «per ora»): quella prova è sostituita. La conferma di una disconnessione si chiude da sola se la sessione sparisce.
- **Prove**: `tests/foreign.rs` 4 (`a_session_of_a_deleted_profile_becomes_foreign`, la sostituzione solo con la seconda chiamata, `disconnect` di una sessione estranea, `profile_delete` ancora rifiutato per il profilo attivo); Vitest `ForeignRow.test.tsx` 12 e `a_foreign_session_is_closed_without_a_pointer` in `keyboard.test.tsx`; fumo (`connect.e2e.ts`) `a_session_started_by_the_cli_of_another_rubrica_is_foreign` e `closing_a_foreign_session_asks_first` con `galleria connect` di un'altra `XDG_CONFIG_HOME` contro il demone finto (`--socket`: la CLI non legge `GALLERIA_SOCKET`).
- **Fatto quando**: le differenze 4, 5 e 6 del §16 della spec sono nella finestra; contro il demone finto la spec §8 (conferma e sostituzione, riga e barra, Disconnetti con conferma, Log in sola lettura) e §10 (Log, Copia, Esporta, righe non tradotte, niente su `stderr`) hanno ciascuna una prova.

#### Chiusura del gruppo C

- **Fumo**: `scripts/gui.sh smoke` **48/48** (`connect.e2e.ts` 10, `smoke.e2e.ts` 38); **Orca/a11y**: `scripts/gui.sh a11y` **42/42** (le 41 di prima e `K5e`). Nessuna prova saltata: i due log non hanno `skipped`.
- **Cosa ha trovato il giro**: tre difetti **delle prove**, nessuno del prodotto: (1) `textContent` di una riga di log non aveva spazi tra le parole (ora ci sono, e un lettore di schermo le legge staccate); (2) la riga fissa a fianco di «Disconnetti» lasciava al nome 72 px (ora il pulsante sta sotto); (3) `getText()` di WebDriver non vede le parti di un `button` tagliate con i puntini di sospensione, quindi la prova legge il testo della riga dalla pagina. In più, un selettore CSS non può contenere `button=Testo` di WebdriverIO.
- **Una prova della tastiera instabile**: `a_full_rubrica_tour_needs_no_pointer` (`keyboard.test.tsx`) è fallita due volte di fila a metà lavoro (il fuoco restava su «More actions» invece di «Cancel» nella conferma di eliminazione) e poi non più (4 giri su 4 verdi, e il giro finale). Non so dire se sia un'instabilità che c'era già (la fa il tempo di un `waitFor`) o dovuta a questo gruppo; non l'ho corretta.
- **Gate dell'host** sull'ultimo commit: `cargo fmt --check`, `clippy` del workspace senza la GUI, `cargo test` del workspace senza la GUI (598 passati, 0 falliti), `cargo deny check`, `check-boundaries`: verdi; `scripts/gui.sh check` verde (clippy, `cargo test -p galleria-gui`, Vitest 669/669, typecheck, build).
- **Non fatto** (invariato, è del gruppo D): le righe G7/G9 del job `gui` in `lab/tests/gates-b5.tsv` e `check-gates.py` nel job `gui` di `ci.yml` (T10); «Ricorda» contro un portachiavi bloccato (G5); la prova Orca della finestra delle credenziali e della conferma della sessione estranea con Orca (T11): la conferma ha la prova di tastiera e di jsdom, non una con Orca.

### Gruppo D · Laboratorio e chiusura

#### La finestra parla con il demone vero dei client; `lab-connect`; G4 (T9)

- **Q42 (b), com'è nel piano**: nessuna modifica a `lab/up.sh` né alle immagini. `scripts/gui.sh lab-connect <26|27|79>` → `lab/run.sh <client> t18` con `GALLERIA_E2E_GUI_CONNECT=1` e `LAB_BEFORE_TESTS='scripts/gui.sh lab-connect-window <client>'`. `lab-connect-window` (sull'host, tra `up.sh` e le prove): crea `carla` nel gruppo `galleria` del client; avvia `lab/tests/gui-lab-watch.sh` nel client (`docker exec -d`); avvia due ponti (`lab/tests/gui-daemon-relay.py`, da `spikes/b5-piano/relay.py`) in `target/gui-lab/run/{carla,alice}.sock`, ognuno con `docker exec -i -u <utente> glab-client<N> python3 … join /run/galleria/galleriad.sock`; lancia nell'immagine `lab-connect-here` (app costruita come per il rilascio, Xvfb, bus privato, GNOME Keyring sbloccato) con `GALLERIA_SOCKET` sul ponte di `carla`; ferma i ponti e l'osservatore; `check-gates.py --job lab-connect --kind vitest`. L'`stderr` dell'app va in `target/gui-lab/app.stderr` (un lanciatore, `app-keep-stderr`: lo `stderr` dell'app non è quello di `tauri-driver`). Poi `t18_gui_connect` e, nel verbo, `check-gates.py` con tutto.
- **`lab/run.sh`**: `t18_gui_connect` non gira senza `GALLERIA_E2E_GUI_CONNECT=1` e lo dice in una riga (`lab/run.sh 26` senza la variabile: «t18_gui_connect not run: it needs GALLERIA_E2E_GUI_CONNECT=1»); la variabile passa al client; `LAB_TEST_LOG` tiene ciò che i test stampano (per `check-gates.py`: `test <nome> ... ok` su una riga, quindi `t18` **non stampa niente**).
- **Prove della finestra** (`ui/e2e/lab-connect.e2e.ts`, gira solo con `GALLERIA_LAB_CONNECT=1`; i cinque profili `base`, `wg-split`, `otp`, `enckey`, `unreachable` entrano dalla finestra, il primo come argomento del processo e gli altri come secondi lanci): `G4a_wireguard_connects_and_the_numbers_move` (indirizzo di `wg-split.conf`; i byte letti due volte a 3 s crescono; «Last handshake» ha un valore), `G4b_openvpn_connects_after_the_credentials` (la finestra chiede utente e password, poi `10.8.0.x`, niente «Last handshake»), `G4c_switching_profile_replaces_without_asking` (un avviso «Disconnected from …» per cambio, nessuna conferma, una sola sessione), `G4d_disconnect_returns_to_disconnected` (anche `galleria --socket … status` dice `disconnected`), `G4e_cancel_while_connecting_is_a_disconnect`.
- **Prove di `t18`** (nel client, leggono il registro dell'osservatore: **falliscono** se manca o ha meno di 20 campioni): `gui_session_never_had_two_backends` (nessun campione con un backend `openvpn` e `glr-wg0` insieme né con due backend `openvpn`; almeno un campione con ciascuno dei due; **dopo Q48** il sondaggio `openvpn --version` e un `openvpn` in uscita non contano come backend, vedi «Correzioni della review»), `gui_session_left_no_residue` (`residue()` uguale a quello dell'avvio dell'osservatore; nessun `openvpn`).
- **Uscita di `LAB_BUILD=host scripts/gui.sh lab-connect 26`** (locale, Docker rootless): le cinque prove di G4 `passed` e le due di `t18` `ok`, `check-gates: 7 proofs of lab-connect ran and passed` (la prima in fondo alla finestra, la seconda dopo `t18`). **S39 sciolta** per Docker rootless: il ponte sull'host è raggiungibile dall'immagine e il demone vede `alice` e `carla` veri; **su un runner con Docker non rootless (uid 1001) non provata** (il ripiego è nel piano: il ponte dentro l'immagine con il socket di Docker). Il job `lab-connect` è in `ci.yml` con la matrice 26, 27 e 79 e **non** ha `continue-on-error`; **non è mai girato su un runner**.
- **S40 confermata in locale** (client 27): `G4b` e `G4c` rosse perché OpenVPN 2.7 prova il DCO e il kernel lo nega sotto Docker rootless (`dco_new_peer: netlink reports error (-28): Operation not permitted` nel giornale di `galleriad`, poi `server_unreachable` dopo i quattro tentativi); `G4a`, `G4d`, `G4e` verdi. Su 79 lo stesso esito (due rosse e tre verdi; la causa nel giornale l'ho letta solo su 27). Come `t13`, `t16` e `t17`: la prova di OpenVPN 2.7 è il job.
- **Un difetto trovato dal demone vero, non dal finto**: il backend di OpenVPN manda **`request: 0` in ogni domanda**; la finestra (T6) prendeva il numero per l'identità della domanda e, dopo le credenziali, scambiava la domanda del codice o della passphrase per quella già servita: restava «In attesa di input» senza finestra fino ai 120 s. Corretto in `ConnectionProvider` (la chiave è sessione, numero **e tipo**, e la risposta si dimentica quando lo stato non è più `needs_input`: una riconnessione che chiede un codice nuovo si riapre), con la prova `every_request_numbered_zero_is_asked_in_turn`. Commit `fix the window of the credentials for a daemon that numbers every request zero`.

#### G5, G6, G7 e G9 nel laboratorio (T10)

- **Prove della finestra** (`ui/e2e/lab-connect.e2e.ts`, le undici del piano; la finestra risponde a ogni finestra delle credenziali che compare e riporta cosa ha chiesto: tipo, campi, etichetta, presenza di «Ricorda»): `G5a_only_the_asked_field_is_shown` (`base`: utente e password, nessun campo del codice · `otp` come `bob`: le credenziali, poi **una seconda** finestra con il solo campo `otp` e l'etichetta del server «Enter OTP» e senza «Ricorda» · `enckey`: le credenziali e la passphrase da sola); `G5b_remember_saves_to_the_keyring_and_the_next_connection_does_not_ask` (`secret-tool` trova `openvpn.username` e `openvpn.password`; nessun file di `secrets/` ha la password; il profilo nomina le due chiavi e non le ha; «Disconnect», «Connect»: nessuna finestra); `G5c_the_otp_is_never_saved` (nel portachiavi utente e password di `bob`, **nessun** codice di quelli usati, nessuna chiave `openvpn.otp`; alla connessione dopo la finestra chiede il solo codice); `G5d_the_passphrase_is_never_saved` (due connessioni di `enckey`: la passphrase è chiesta tutte e due, e nel portachiavi del profilo e in **tutto** quello di `service galleria` non c'è); `G5e_a_refused_saved_password_offers_to_forget_it` («Forget» toglie le credenziali; una password sbagliata con «Remember» dà «Authentication failed» e **non salva niente**; poi la password giusta ricordata, una sbagliata messa a mano con `secret-tool store` da stdin, la connessione la manda senza chiedere, l'errore offre «Forget the saved credentials», la tolta, e il «Retry» chiede). `G6a_…` (la CLI dell'immagine importa `wg-split.conf` nella rubrica della finestra: i segreti sono nel file e il portachiavi non ne ha; la finestra lo elenca e lo connette), `G6b_…` (`base` importato dalla finestra: `openvpn.block.key` nel portachiavi e nessun file di `secrets/`; `galleria --socket … connect base --username alice --password-stdin` connette, e la finestra mostra la sessione come **propria**, non come estranea). `G7a_…` (dopo `otp` ed `enckey`: il testo della scheda Log durante e dopo la sessione, il file di «Export log…» passato per la **finestra «Salva» vera pilotata con AT-SPI senza Orca**, `target/gui-lab/app.stderr`, e `/proc/*/cmdline` e `/proc/*/environ` di ogni processo dell'immagine: nessuno ha `alice-secret`, `bob-secret`, `key-passphrase`, i codici usati, una riga della chiave privata di `base.ovpn`, la chiave privata di `wg-split.conf`). `G9a_…` (la CLI come `alice`, dal **suo ponte** e da un'altra `XDG_CONFIG_HOME`, importa e connette: la finestra di `carla` mostra la riga fissa, il testo della barra e un dettaglio con le righe del log), `G9b_…` («Disconnect» e «Cancel»: `alice` risulta ancora connessa, il profilo attivo è ancora il suo; di nuovo, conferma: `disconnected`), `G9c_…` (`alice` riconnette; «Connect» su `wg-split` di `carla` chiede «Replace «alice-wg»?», e prima del sì la sessione di `alice` c'è ancora; dopo «Replace» la sessione attiva è `wg-split`).
- **Prove di `t18`** (le altre due): `gui_session_left_no_secret_in_journal_or_argv` (`journalctl -u galleriad` e ogni riga di argomenti nel registro dell'osservatore non hanno nessuno dei segreti noti; i codici usati li lascia la finestra in `target/gui-lab/used-otp`; **fallisce** se il giornale non ha una connessione o l'osservatore non ha visto nessun `openvpn`), `gui_session_was_driven_by_two_users` (nel log del demone `uid <alice> connect`, `uid <carla> connect` e `uid <carla> disconnect`, con gli uid veri del client).
- **S41 sciolta**: la sonda AT-SPI (`atspi-file-window.py`) pilota la finestra «Salva» **senza Orca**: bastano il bus AT-SPI (`at-spi-bus-launcher`) e `toolkit-accessibility`, in `lab/tests/gui-lab-connect.sh`. `G7a` passa con il file esportato dalla finestra vera (`0600`, «Log exported.» detto). Il ripiego del piano non serve; `the_export_never_carries_a_known_secret` sta comunque nel manifesto per G7.
- **Manifesto** (`lab/tests/gates-b5.tsv`): **venti** righe per `lab-connect` (cinque `G4`, sette di `G5`/`G6`, una `G7a`, tre `G9`, quattro di `t18`; **ventuno** dopo Q48, con la prova dell'osservatore) e cinque per il job `gui` (`no_command_returns_a_secret`, `no_event_carries_a_secret`, `the_export_never_carries_a_known_secret`, `a_session_outside_the_rubrica_is_foreign`, `disconnecting_a_foreign_session_is_not_automatic`). `GATES_WITH_PROOFS` di `tests/config.rs` è ora G4, G5, G6, G7, G9. Il job `gui` di `ci.yml` finisce con `scripts/check-gates.py --job gui --cargo-log target/gates/gui-cargo.log` (il log lo tiene `check-here`).
- **Uscita di `LAB_BUILD=host scripts/gui.sh lab-connect 26`** (locale): 16 prove della finestra `passed` (1 min 24 s in tutto, con il laboratorio già costruito), `check-gates: 16 proofs of lab-connect ran and passed`; `t18`: 4 `ok`; riga finale **`check-gates: 20 proofs of lab-connect ran and passed`**. **Controllo che il gate non mente**: con una prova messa `skipped` nel JSON di Vitest l'uscita è 1 («1 of 20 proofs of lab-connect are not proved»); con `gui_session_left_no_residue` messa `ignored` nel log di cargo, uscita 1 («NOT PROVED (ignored)»).
- **Cosa resta rosso in locale**: su 27 e 79 le prove di OpenVPN (S40, DCO negato da Docker rootless); la prova di G4–G9 sui tre client è il run del job `lab-connect`, che parte dopo il push (non lo faccio io).
- **Cosa ho visto strada facendo**: (1) `carla` e `alice` sono due ponti: il demone li distingue per uid (`uid <n> connect` nel suo log), e la CLI dell'immagine **non** deve leggere `GALLERIA_SOCKET` ma `--socket`; (2) il `getText()` del dialogo comprende il titolo, quindi il tipo di finestra si legge dal titolo (`Credentials for`, `Code for`, `Passphrase for`) e non dal primo campo; (3) un OpenVPN che fallisce (DCO) riprova quattro volte e ad ogni tentativo il demone **richiede le credenziali**: con la correzione di `ConnectionProvider` la finestra le rimostra a ogni tentativo.

#### Tastiera, fumo, screen reader e chiusura del blocco (T11)

Prove nuove: `a_full_connection_tour_needs_no_pointer`, `focus_is_never_lost_to_the_body_after_the_credentials_the_confirmations_and_an_error`, `a_notice_of_the_connection_is_said_once_and_shown` (`keyboard.test.tsx`, jsdom); `no_command_returns_a_secret_after_a_full_round` (`tests/connection.rs`); `no_csp_violation_during_the_connection_tour` e `the_cli_sees_the_session_the_window_started` (fumo, `connect.e2e.ts`); `K5a`, `K5b`, `K5c`, `K5d`, `K5f` (Orca, `connection.a11y.ts`; `K5e` era già di T7). Un solo giro di ogni comando di chiusura, a fine gruppo, **non** i giri ripetuti del piano (tre di fumo, due di Orca, due di `lab-connect`): la card del gruppo D chiedeva un giro solo. I giri ripetuti restano da fare a chi vuole dire «stabile».

**Numeri storici di T11 su `f33d093`** (tutti in locale, Docker rootless; le verifiche delle correzioni sono riportate più sotto):

| Livello | Comando | Esito |
|---|---|---|
| Finestra (jsdom) | `scripts/gui.sh check` | Vitest **673/673**, `cargo test -p galleria-gui` 368 passati e 0 falliti, clippy, typecheck, build |
| Host (fase 1 e `galleria-profile`) | `cargo fmt --check`, clippy del workspace senza la GUI, `cargo test --workspace --exclude galleria-gui`, `cargo deny check`, `check-boundaries.sh` | verdi: **598 passati, 0 falliti** (99 `ignored`: le prove di fase 1 che chiedono il laboratorio, invariate); nessuna differenza nei crate vietati (`git diff 900ccf8 -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-net crates/galleria-ovpn`: vuoto) |
| Portachiavi vero | `scripts/gui.sh keyring` | `check-gates: 10 proofs of gui-smoke ran and passed` |
| Fumo | `scripts/gui.sh smoke` | **50/50** (nessuna prova saltata: il log non ha `skipped`) |
| Screen reader | `scripts/gui.sh a11y` | **47/47** (`K5a`–`K5f` tutte, di cui `K5c` con la riserva sotto) |
| Laboratorio, client 26 | `LAB_BUILD=host scripts/gui.sh lab-connect 26` | `check-gates: 20 proofs of lab-connect ran and passed` |
| Job `gui` | `scripts/check-gates.py --job gui --cargo-log target/gates/gui-cargo.log` | `check-gates: 5 proofs of gui ran and passed` |

**Orca, review e correzione.** `K5a`: «Disconnect push button.» e poi «Connected to Ufficio Napoli», una volta. `K5d`: «alert Disconnect «Altra rubrica»?», il testo («This connection was not started from this rubrica. …») e «Cancel push button.». Il verde originario di **`K5c` non provava il parlato**: nella review di `f33d093` Orca diceva **«blank»**, non «niente» come era scritto qui. Dopo la decisione Q49 (sotto, «Orca e la finestra delle credenziali») `K5c` è una **prova del buco noto dell'immagine** (Orca 46.1): l'albero e il fuoco sono asseriti come sempre (dialogo «Credentials for «Datacenter Milano»», fuoco su «User name», niente della pagina dietro), il parlato è registrato e si asserisce che il titolo e «User name» **non** sono detti, e resta «nessun annuncio da Rust». Il giorno che l'immagine avrà un Orca che li dice, le due asserzioni diventano rosse e la prova passa al positivo. Su GNOME vero (Ubuntu 26.04 e Fedora 45) la stessa finestra è letta: titolo, «dialog», «User name» e «entry required».

**Criteri C1–C8 e gate** (le prove sono nel manifesto `lab/tests/gates-b5.tsv`, controllate da `scripts/check-gates.py`):

| # | Esito | Prova · job |
|---|---|---|
| C1 · G4 | **CI verde sui tre client nel run 45769841084 su `f33d093`**, con il vecchio `t18` allentato. **Con il controllo stretto e l'osservatore di Q48: tre giri locali su 26, 21/21 ciascuno** (prima dell'osservatore corretto due giri su tre rossi: sotto) | `G4a`–`G4e` + `gui_session_never_had_two_backends` + `the_observer_of_the_backends_leaves_out_only_the_probe_and_the_exiting` + `gui_session_left_no_residue` · `lab-connect` |
| C2 · G5 | **Verde** sul client 26 e contro GNOME Keyring vero; «Ricorda» contro un portachiavi **bloccato** che chiede lo sblocco **non provato** | `G5a`–`G5e` · `lab-connect`; `keyring_*` · `gui-smoke` (passo `keyring`) |
| C3 · G6 | **Verde** sul client 26 | `layered_*` · `check`; `the_cli_reads_what_the_keyring_holds` · `gui-smoke`; `G6a`, `G6b` · `lab-connect` |
| C4 · G7 | **Verde** | `no_command_returns_a_secret`, `no_command_returns_a_secret_after_a_full_round`, `no_event_carries_a_secret`, `the_export_never_carries_a_known_secret` · `gui`; `G7a`, `gui_session_left_no_secret_in_journal_or_argv` · `lab-connect` |
| C5 · G9 | **Verde** con due utenti veri del gruppo (`alice`, `carla`); la prova della sessione estranea ora usa un uuid valido assente dalla rubrica, e fallisce con la mutazione «è un uuid» | `a_session_outside_the_rubrica_is_foreign`, `disconnecting_a_foreign_session_is_not_automatic` · `gui`; `G9a`–`G9c`, `gui_session_was_driven_by_two_users` · `lab-connect` |
| C6 | **Verde** | `no_command_takes_a_path`, «Il viaggio di una credenziale» (T6) |
| C7 · G12 | **Parlato di `K5c`: prova del buco noto** (Q49 (a)), non soddisfatto nell'immagine e provato su GNOME vero; tastiera verde. Giro Orca completo: due verdi di fila al terzo e al quarto giro, ma la suite è **instabile** (sotto) | `a_full_connection_tour_needs_no_pointer`, `K5a`–`K5f` (locale, Q22) |
| C8 | **Verde** | diff dei crate vietati vuoto; `lab` e `lab-g3` non toccati da queste correzioni e **non rilanciati** in locale |

**CI già eseguita, non più in attesa.** Run **45769841084**, head **`f33d093`**, **15/15 job verdi**: la review `t_d14bbfa2` ha letto i log dei gate, non solo i segni verdi. `check`: 7 prove; `gui`: 5; `gui-smoke`: 10, con le righe `test keyring_* ... ok`; ognuno dei tre `lab-connect` (26, 27, 79): 16 prove Vitest + le 4 di `t18`, e `check-gates: 20 proofs of lab-connect ran and passed`, senza prove saltate. **S39 è sciolta anche sul runner** con Docker non rootless, uid 1001: il ponte ha consentito alla finestra di guidare i demoni veri. Questi fatti riguardano `f33d093`, **non** i commit locali di correzione, non pushati. La CI precedente non sostituisce il controllo stretto nuovo, né prova il parlato di `K5c`.

**Prova stretta ripristinata, e l'osservatore secondo Q48.** `gui_session_never_had_two_backends` (`t18`) torna ad asserire **ogni campione**: nessun backend `openvpn` insieme a `glr-wg0`, né due backend `openvpn`. Resta la doppia lettura di `glr-wg0` prima e dopo i processi. Il messaggio di errore stampa il campione completo e le righe `argv` del suo timestamp. Nei tre giri richiesti di `LAB_BUILD=host scripts/gui.sh lab-connect 26`, sul commit `fd4f6f6` (asserzione stretta, nessun filtro), il primo e il secondo passano le 20 prove; il terzo passa le 16 Vitest e tre Rust, ma fallisce proprio questa:

```text
sample 1795677936562 openvpn=1 wg=1
argv 1795677936562 1641 /usr/sbin/openvpn --version
```

È il sondaggio della versione della fase 1 (`factory::backend_infos` → `galleria_backend_openvpn::probe`, su `hello`), non un tunnel: **Q48** (Aitano, su delega) dice che D5 è «mai due tunnel insieme», non «mai due processi con lo stesso nome», e vuole che l'osservatore escluda **solo** `<path>/openvpn --version`, che l'asserzione resti stretta su ogni campione, che ci sia una prova dell'osservatore e che i giornali dei container si conservino quando un giro è rosso. Fatto così, su `b82c68c` (un solo commit con tutto):

- **Osservatore** (`crates/galleria-e2e/tests/t18_gui_connect.rs`): `is_version_probe` accetta esattamente `<pid> <path>/openvpn --version` (due parole dopo il pid, la prima con il nome `openvpn` dopo l'ultimo `/`, la seconda `--version`); ogni altra cosa conta. Il campione si ricalcola dalle righe `argv` del suo timestamp e **deve** avere tante righe quanto dice `openvpn=`, altrimenti il registro non è intero e la prova fallisce.
- **Prova dell'osservatore** `the_observer_of_the_backends_leaves_out_only_the_probe_and_the_exiting`, nel manifesto (G4, `lab-connect`), che si può lanciare anche sull'host (`cargo test -p galleria-e2e --test t18_gui_connect -- --ignored the_observer`, non chiede il laboratorio): il sondaggio con `glr-wg0` è **verde**, `openvpn --config …` con `glr-wg0` è **rosso**, due `--config` sono rossi, il sondaggio con un backend è verde, `--version --config x`, `--version=2`, `openvpn --version` senza percorso e `openvpn-other` **non** sono sondaggi. Mutazioni provate: `argument.starts_with("--")` al posto di `== "--version"`, e `two_backends_together` ridotta a «più di uno»: tutte e due **rosse**.
- **Giornali conservati**: `lab/run.sh`, se esce con un errore e non c'è `KEEP=1`, scrive in `target/lab-failures/<data>-<client>/` i log di ogni container `glab-*`, `journalctl -u galleriad` del client e il registro dell'osservatore, **prima** di `lab/down.sh`. È servito subito (sotto).

**Un fatto che Q48 non diceva, trovato con quei giornali.** Con il solo `--version` escluso, due giri su tre (di una serie di tre) sono usciti rossi, e questa volta i giornali c'erano:

```text
giro 2:  sample 1795679748081 openvpn=2 wg=0   argv 2292 /usr/sbin/openvpn --config /dev/stdin
                                                argv 2424 [openvpn]
giro 3:  sample 1795679860719 openvpn=1 wg=1   argv 4852 [openvpn]
```

`[openvpn]` è come `pgrep -a` scrive un processo la cui riga di comando non c'è più: **uno che sta uscendo** (ha già lasciato la memoria e i file, tunnel compreso) o è uno zombie `<defunct>`. Non è un secondo backend, ed è lo stato in cui l'osservatore coglie anche il sondaggio: prova fatta nell'immagine del client, `openvpn --version` lanciato 400 volte in un ciclo e un campionatore con `pgrep -a -x openvpn` che gira in parallelo: 274 campioni con un processo, `openvpn --version` in 250, **`[openvpn]` in 18 e `[openvpn] <defunct>` in 6**. In tutti e due i casi il processo compare in un solo campione da 200 ms. **Non l'ho potuto leggere come «il sondaggio»** (la riga non ha argomenti, quindi non è provato che lo fosse), ma la riga non contiene un backend vivo, e senza escluderla la misura è rossa a caso. **Ho esteso Q48 di un punto, da confermare:** `is_exiting` esclude anche `<pid> [openvpn]` e `<pid> [openvpn] <defunct>` (esattamente queste due forme); il resto resta come ha deciso Q48. Se la decisione preferisce un'altra misura (per esempio leggere lo stato del processo in `gui-lab-watch.sh`), è un cambio piccolo e sta tutto in `openvpn_backends`. Con questo, **tre giri di `LAB_BUILD=host scripts/gui.sh lab-connect 26`: `check-gates: 21 proofs of lab-connect ran and passed`, uno dopo l'altro, tutti e tre verdi**.

**Q48 estesa, decisione di Aitano (card `t_ac9eddd9`, un commit sopra `09eae5b`).** Quell'estensione guardava solo il testo della riga: un `[openvpn]` con `glr-ovpn0` ancora su e `glr-wg0` presente restava verde, e la prova dell'osservatore lo asseriva verde. La regola vincolante è: un `[openvpn]` si esclude dai backend **solo se** il processo non ha più cmdline **e** nello stesso campione `glr-ovpn0` non esiste più; altrimenti è un secondo backend ancora vivo e la prova è rossa. Fatto così:

- **Osservatore del client** (`lab/tests/gui-lab-watch.sh`): ogni campione è ora `sample <ms> openvpn=<n> wg=<0|1> ovpn_if=<0|1>`. `ovpn_if` legge `glr-ovpn0` prima e dopo `pgrep`, come `glr-wg0` ma **al contrario**: vale `1` se l'interfaccia c'era in **almeno una** delle due letture (nel dubbio c'è); `wg` resta `1` solo se c'era in tutte e due.
- **`t18`**: il campione ha `ovpn_if`; un campione senza `ovpn_if=` (o con un valore diverso da 0 e 1) non si legge e la prova è rossa (registro non intero). `openvpn_backends(processes, ovpn_if)` esclude `[openvpn]` e `[openvpn] <defunct>` solo con `ovpn_if=0`; con `ovpn_if=1` contano come backend. `is_version_probe` non è cambiato. Anche «l'osservatore ha visto almeno un backend `openvpn`» usa la stessa regola. Il messaggio di errore stampa `ovpn_if` nel campione.
- **Prova dell'osservatore** (la stessa, nel manifesto com'è, lanciabile sull'host): `[openvpn]` con `ovpn_if=1` e `glr-wg0` → **rosso**; con `ovpn_if=0` e `glr-wg0` → **verde**; lo stesso per `[openvpn] <defunct>`; backend `--config` + `[openvpn]` con `ovpn_if=1` → **rosso** (due backend), con `ovpn_if=0` → un backend. Mutazioni, sull'host: «ignora `ovpn_if`» come se valesse sempre 0 → **rossa** (`openvpn_backends(&[exiting], true)` deve essere rosso); come se valesse sempre 1 → **rossa** (`openvpn_backends(&[exiting], false)` con `glr-wg0` deve essere verde); ripristino: verde.
- **Tre giri di `LAB_BUILD=host scripts/gui.sh lab-connect 26`**, uno dopo l'altro: `check-gates: 16 proofs of lab-connect ran and passed` per la parte Vitest e **`check-gates: 21 proofs of lab-connect ran and passed`** alla fine, **tre volte su tre**, con `gui_session_never_had_two_backends` e la prova dell'osservatore verdi (e il registro nuovo letto: senza `ovpn_if=` il parser va in panico). Dopo i tre giri clippy (`-D warnings`) ha chiesto di riscrivere la condizione di `openvpn_backends` senza cambiarne il senso (`!is_version_probe(p) && (!is_exiting(p) || ovpn_if)`): prova dell'osservatore e le due mutazioni rifatte sulla forma finale (verde, rossa, rossa, verde), e **un quarto giro di `lab-connect 26`**, 21/21. **Nessun campione rosso**, quindi il caso del giro 2 (backend `--config` vivo con un `[openvpn]` e la stessa `glr-ovpn0`) non è comparso e non c'è niente da riportare ad Aitano su questo punto. Non ho la prova che un `[openvpn]` che compare con `glr-ovpn0` su sia o non sia il sondaggio: se compare, la regola lo dice rosso e non si allenta.

#### Correzioni della review (card `t_4edd2f74`, senza push)

- **Difetto 1, chiuso come prova del buco noto (Q49 (a)).** Orca 46.1 nell'immagine dice solo «blank» all'apertura della finestra delle credenziali; **su GNOME vero la stessa finestra è letta** (sotto). `K5c` ora asserisce albero e fuoco, **registra il parlato** e asserisce che titolo e «User name» non sono detti (buco noto dell'immagine, come `S4b`), più «nessun annuncio da Rust». Ha un controllo registrato (`control_orca_says_a_bare_dialog_with_an_empty_field_when_it_pleases`). Nessun codice di produzione cambiato. Da rivedere in B6 su Ubuntu 24.04 (Q28). Commit `afb4531`.
- **Difetto 2, chiuso con Q48.** Asserzione stretta su ogni campione, osservatore che esclude il sondaggio `openvpn --version` e un `openvpn` in uscita (`[openvpn]`, estensione da confermare), prova dell'osservatore, giornali conservati su giro rosso, tre giri 21/21. Dettagli sopra. Commit `b82c68c`; nessuna modifica alla fase 1.
- **Difetto 3, corretto.** `state_changes_are_announced_once_each` riconsegna tre volte lo stesso `Status` connesso allo stesso `TauriSink` usato dal supervisore: l'annuncio resta uno. Mutazione che toglie il confronto della terna: **rossa**, quattro «Connected to Homelab» invece di uno; ripristino: verde. Commit `c635dd2`.
- **Difetto 4, corretto.** Solo la prova di G9 usa `7f0a4c1e-5b3d-4e8a-9c2f-1d6b8a0e3f45` e verifica che non sia in rubrica; `params()` per le altre prove è invariata. Mutazione `is_own = Uuid::parse_str(id).is_ok()`: **quella prova è rossa**; ripristino: verde. Commit `edb6496`.
- **Difetto 5, corretto.** «Ricorda» e «Credenziali salvate» seguono lo stato già letto dalla shell (`keyring`, `file`, generico fino a risposta). Cataloghi italiano e inglese aggiornati; prove Vitest dei due componenti. Nessuna lettura aggiuntiva di segreti o nuova chiamata IPC. Commit `1ced3b0`.

#### Orca e la finestra delle credenziali (Q49)

**Su GNOME vero Orca la legge.** Con la procedura dello spike `t_b81c3909` (`lab/vm/`, VM in `/mnt/b/vm`, tasti e clic come nello spike; **senza audio**: il parlato è il testo che Orca scrive nel file di debug), lo script nuovo `lab/vm/orca-credentials-vm.sh run` semina la rubrica (`seed-rubrica`), avvia Orca, il demone finto con lo scenario `credentials` e la finestra di questa correzione (stessi binari dell'immagine `galleria-gui-dev`, HEAD di allora), sceglie «Datacenter Milano» e preme «Connect» con AT-SPI:

| | Parlato all'apertura della finestra | Fuoco AT-SPI |
|---|---|---|
| Ubuntu 26.04.1, GNOME Shell 50.1 Wayland, Orca 50.3 (due volte) | `Credentials for «Datacenter Milano»` · `dialog` · `User name` · `entry required.` · `invalid entry.` | entry «User name» |
| Fedora 45, GNOME Shell 51.1 Wayland, Orca 51.1 | `Credentials for «Datacenter Milano»` · `dialog` · `form` · `User name entry required` · `invalid entry` | entry «User name» |
| Immagine, Orca 46.1, Xvfb (`K5c`) | `blank` | entry «User name» |

Sulla Fedora il portachiavi di accesso è bloccato (accesso automatico): la finestra di Rust avvisa «The keyring is locked. Unlock the keyring and try again.» e il sistema chiede lo sblocco **prima** delle credenziali. Il comportamento è quello previsto per un portachiavi bloccato; per provare la finestra delle credenziali l'app è partita senza il bus di sessione (`NO_KEYRING=1` dello script, bus AT-SPI passato a mano), cioè con l'archivio sul file, come nell'immagine. L'Ubuntu è stata provata anche così, stesso parlato. **Quindi vale (a)**, non (b): nessun annuncio nuovo da Rust.

**Cosa ho visto sul perché dell'immagine.** Un dialogo nudo (`role=dialog`, `aria-modal`, un campo vuoto con `required`, nient'altro di nostro) è detto **a volte** da Orca 46.1 e a volte no, con lo stesso markup: nei 9 campioni della forma più fedele (bottone nella pagina, pagina `inert`, fuoco su un campo vuoto) ha detto il campo («Control field entry required.») in 4 e niente in 5; il titolo non l'ha mai detto, a meno che il fuoco non venisse dal corpo della pagina (3 giri su 3). Quindi non è `initialFocus`, né l'`inert` della libreria, né il `required`, ma il dispatch di Orca 46.1 (WebKitGtk `onCaretMoved` prima di `onFocusedChanged`, come già visto nel primo giro di questa correzione). Per questo il controllo di `K5c` è **registrato e non asserito**, come il campo vuoto e la textarea del controllo di `S4b`.

**Il giro Orca completo non è stabile; non l'ho reso tale.** Sul commit di questa correzione i giri completi (`scripts/gui.sh a11y`, 48 prove: 47 originali + il controllo di `K5c`) hanno dato risultati diversi **senza cambiare niente**: un giro con 48/48 verdi (09:35); altri con una sola prova rossa, ogni volta una diversa: `K5f` (l'ordine tra l'avviso «Disconnected from» e lo stato «Connected to Homelab», detti a 9 ms l'uno dall'altro nell'ordine inverso: l'avviso arriva dalla finestra con un comando e lo stato dal supervisore di Rust, due fili, e Orca non garantisce l'ordine), `I5d` (Orca dice il nome della finestra «Open» invece di «Import profile»), `control_orca_does_not_say_the_name_of_a_bare_text_field` (Orca ha detto «Probe bravo entry Some value selected», il nome che il controllo dà per non detto); e un giro con **`R5d` in timeout a 90 s**, dopo il quale `tauri-driver` non accetta più sessioni (`UND_ERR_SOCKET`). `R5d` da sola o con tutto il file `rubrica.a11y.ts` è passata **11 volte su 11**; il timeout è comparso solo nei giri completi (dopo circa tre minuti di sessioni nello stesso Xvfb e bus AT-SPI), con `WebKitWebDriver` che sparisce senza scrivere niente (nel log: `connection closed before message completed`, poi `Connection refused`). Ho speso circa 40 minuti su `R5d`: **non ho la causa**, e non l'ho nascosta con un riprovare nel codice delle prove. `atspiTree()` è un `execFileSync` senza tempo massimo: se il dump AT-SPI si blocca, tutto il resto aspetta; è il primo sospetto, **non provato**. Ciclo «fino a due verdi di fila» su `afb4531`: giro 1 rosso (`control_orca_does_not_say_the_name_of_a_bare_text_field`), giro 2 rosso (`I5d`), **giri 3 e 4 verdi, 48/48 ciascuno**. Il requisito «due verdi di fila» è quindi soddisfatto, ma dopo due rossi e senza aver cambiato nulla tra un giro e l'altro: la suite resta instabile.

**Verifiche delle correzioni, da non confondere con la tabella storica di T11** (HEAD `afb4531` più il commit di esito; tutto in locale, Docker rootless, nessun push):

| Comando | Esito reale |
|---|---|
| Gruppo host: fmt, clippy senza GUI con `-D warnings`, test senza GUI, deny, boundaries | verdi: **598 passate / 0 fallite** (100 `ignored`, una in più: la prova dell'osservatore) |
| `scripts/gui.sh check` | **54 file Vitest verdi**, **368 Rust / 0 fallite**, typecheck (anche `e2e`), clippy e build verdi |
| `scripts/gui.sh smoke` (ultimo giro, sui commit dei difetti 3–5; dopo non è cambiato nulla che il fumo veda) | **50/50**, nessuna saltata |
| `LAB_BUILD=host scripts/gui.sh lab-connect 26`, tre giri su `b82c68c` | **`check-gates: 21 proofs of lab-connect ran and passed` tre volte su tre**. Prima dell'estensione `[openvpn]` di Q48: due giri rossi su tre (campioni sopra), con i giornali conservati in `target/lab-failures/` |
| `LAB_BUILD=host scripts/gui.sh lab-connect 26`, tre giri con `ovpn_if` (card `t_ac9eddd9`) | **`check-gates: 21 proofs of lab-connect ran and passed` tre volte su tre**, nessun campione rosso; prova dell'osservatore e mutazioni «ignora `ovpn_if`» (sempre 0 / sempre 1) rosse sull'host |
| `scripts/gui.sh a11y`, giri completi su HEAD | **instabili**: 48/48 in tre giri su otto (i due di fila compresi); gli altri con una prova rossa ciascuno (`K5f`, `I5d`, controllo del campo nudo, `R5d` in timeout); **giri 3 e 4 di un ciclo di quattro: 48/48 e 48/48** (giri 1 e 2 rossi; sopra, «Orca e la finestra delle credenziali») |

I giri filtrati e le sonde temporanee per l'indagine Orca (`R5d` 11 volte, le varianti del dialogo nudo) non sono prove di stabilità. **B5 resta aperto** per la causa del timeout di `R5d`, che non ho (la conferma di Q48 sull'estensione `[openvpn]` è arrivata: vedi «Q48 estesa»). Il controllo breve dipendente (`t_d47d506e`) vede il lavoro così com'è.

**Non verificato (B5).**
- **KWallet** (S38): nessuna prova; si dice nel README.
- **GNOME Keyring su un desktop vero**: lo sblocco con la finestra del sistema (provato solo senza schermo, con `gnome-keyring-daemon --unlock` da stdin); «Ricorda» contro un portachiavi **bloccato** che chiede lo sblocco.
- **La finestra su Ubuntu 26.04 e Fedora**, e Wayland: la finestra **delle credenziali** è provata su GNOME vero (Ubuntu 26.04 con Orca 50.3, Fedora 45 con Orca 51.1, Wayland, senza audio, Q49); il resto della finestra resta provato solo nell'immagine (Ubuntu 24.04, WebKitGTK 2.52.6, Xvfb).
- **OpenVPN 2.7 in locale** (client 27 e 79): rosso per il DCO negato da Docker rootless (S40); CI su `f33d093` verde su entrambi, che non significa finestra provata sui rispettivi desktop.
- **CI delle correzioni**: non eseguita, nessun push; CI della base verificata dalla review: run 45769841084, 15/15.
- **`K5c`**: prova del buco noto di Orca 46.1 (Q49 (a)); su Ubuntu 24.04 con Orca 46 da rivedere in B6 (Q28).
- **D5 con misura stretta**: tre giri 21/21 dopo Q48 e altri tre dopo la regola dell'interfaccia (`ovpn_if`, `t_ac9eddd9`); l'estensione `[openvpn]` è vincolata da Aitano a `glr-ovpn0` assente nello stesso campione. Non so se un `[openvpn]` visto con l'interfaccia su (se comparisse) sia il sondaggio: nei tre giri non è comparso.
- **Giro Orca (`a11y`)**: due verdi di fila ottenuti solo al terzo e quarto giro, con prove diverse rosse nei giri prima (sopra); `R5d` in timeout senza causa trovata.
- **`a_full_rubrica_tour_needs_no_pointer`**: instabile una volta nel gruppo C, verde in tutti i giri di T11; non corretta.
- «Duplica» con un nome in `secret_refs` senza chiave (dal gruppo B): invariato.

**Punti [S] del blocco**: S12 **provata** con GNOME Keyring (KWallet no); S34, S35, S36, S37 e S45 provate (T1, T2, `keyring`); S38 **non provata**; **S39 sciolta in rootless e sul runner nel run 45769841084**; S40 confermata in locale, CI verde su `f33d093`; S41 sciolta (AT-SPI senza Orca pilota la finestra «Salva»); S42, S43 e S44 non hanno un esito scritto nelle sezioni di T4–T6: le prove che le toccano sono verdi (`G4a` per i byte che crescono e l'ultimo handshake, `G5a`/`G5b` per i campi `password` riempiti dalla finestra, `K5a` per l'annuncio emesso mentre arriva un evento del demone), ma non ho verificato che i tre punti siano stati letti uno per uno: restano da chiudere a mano.

## B6 · Tray, versione, chiusura

Piano: `docs/superpowers/plans/2026-11-26-fase2-b6-tray-chiusura.md`. Q50–Q57 approvate da Aitano il 26/11/2026 su delega di Peppe, tutte come raccomandate: spec §18. Il blocco è in cinque gruppi; qui c'è ciò che i gruppi chiusi hanno fatto.

### Gruppo A · Tray e chiusura

Parte da `4106688` (piano, ff-only dal ramo di `t_d6ce6eaa`) e dal merge del ramo di `t_016cb5ea` (stabilità di `a11y`, `2d0dc33`).

#### Il menu della tray dal modello, e la sonda che lo legge dal bus (T1)

- **Il modello è puro** (`src/tray.rs`, nessun tipo di Tauri): `model(&TrayInput) -> (Vec<TrayEntry>, TrayIcon)`. `Tray` tiene link, stato, preferiti e testi, **crea la tray alla prima consegna dei testi** (`tray_labels`, P3) e riscrive il menu solo quando il modello è venuto diverso; ogni scrittura ha un numero di generazione, e un click su un menu già riscritto non chiede niente. Il pezzo che parla con Tauri (`src/tray_tauri.rs`) **posta** la riscrittura sul thread della finestra (`run_on_main_thread`) e non la aspetta: il thread degli eventi tiene il lucchetto della tray e quello della finestra potrebbe volerlo.
- **Il menu** (P2, P4, P5): riga di stato spenta, separatore, i preferiti nell'ordine della lista (al più 8; quello della sessione dice «Disconnect», gli altri «Connect»), una riga «Disconnect» in più per una sessione che non è un preferito o che è estranea, separatore, «Open Galleria», «Quit». Senza il collegamento al demone la riga dice «Service not reachable», l'icona è quella spenta e le voci «Connect» sono spente. **Scelte che il piano non fissava:** `error` ha la riga di «Disconnected» con l'icona «in corso o errore» e offre ancora «Disconnect» (come il pulsante della finestra, che per `error` è `isLive`); `needs_input`, `reconnecting`, `connecting`, `disconnecting` hanno «Connecting…»; durante `disconnecting` la voce «Disconnect» è spenta. «Quit» dice «Quit (the VPN stays on)» (Q54) quando c'è una sessione.
- **Il nome di un profilo** entra nel menu in una riga (`announce::clean`: a capo e controlli tolti) e per Tauri si raddoppia `&` (`tray::literal`): `muda` lo prende per l'acceleratore, e così il nome si vede com'è. `_` lo raddoppia già `muda` per GTK.
- **Icone**: tre PNG 32×32 (`icons/tray-{off,on,busy}.png`), fatti da `icons/make-tray-icons.py` e incorporati con `tauri::include_image!` (nessuna feature `image-png`, nessun pacchetto). L'icona temporanea sta in `$XDG_RUNTIME_DIR/galleria` quando c'è (P5).
- **Senza la libreria** (S48): la costruzione sta in `catch_unwind` con il gancio dei panici tolto per un istante (senza, il messaggio di `thread … panicked` compare lo stesso). **Provata nell'immagine**: `the_app_starts_without_the_indicator_library` avvia l'app con una `LD_PRELOAD` che fa fallire `dlopen` per `*appindicator*`: la finestra c'è, `stderr` ha `galleria-gui: no tray: the library of the indicator …` e nessun `panicked`, nessuna tray registrata.
- **Cargo.lock**: `git diff -- Cargo.lock` **vuoto** con la feature `tray-icon` (zero pacchetti, come misurato dal piano).
- **I preferiti** si riscrivono quando `Rubrica::update` o `delete` hanno scritto (`Rubrica::on_change`) e a ogni evento del collegamento (un profilo segnato dalla CLI compare al successivo); i nomi di un profilo duplicato o importato non servono, perché nascono non preferiti.
- **La sonda** (`lab/tests/tray-probe.py`, da `spikes/b6-piano/`): `watch` fa da `org.kde.StatusNotifierWatcher`; `layout`, `click "<testo>"`, `icon` (dal colore del PNG che l'icona mostra), `wait-item`. Il watcher lo avvia **la prova** (`startTrayWatcher` in `ui/e2e/support.ts`) e non `gui-lib.sh`, come diceva il piano: con un watcher sempre acceso una prova qualunque che chiude la finestra la nasconderebbe invece di chiudere l'app (Q53), e `closeApp` di tutte le altre prove ne resterebbe con un'app viva.
- **S46 sciolta**: il menu vero di Tauri (`muda`, AppIndicator) si legge e si clicca dal bus **come quello della sonda** (`G10a`: stati, preferiti in ordine, lingua cambiata nella finestra, stella tolta: il menu si riscrive). **S47 sciolta**: riscrivere il contenuto dello stesso menu si vede sul bus senza registrare di nuovo l'indicatore. **S48 sciolta**.
- Prove: `tests/tray.rs` (27: la tabella dei sette stati e dei quattro collegamenti, l'ordine e l'ottavo preferito, la riga della sessione estranea, `Quit`, i nomi piatti, i testi da 201 caratteri, nessuna tray senza testi, una sola creazione, il click su un menu vecchio), `app/tray.test.tsx` (i testi al primo montaggio e a ogni cambio di lingua), `tests/config.rs` (`allow-tray-labels`), fumo `G10a_the_menu_says_the_state_and_lists_the_favourites` e `the_app_starts_without_the_indicator_library`. **Il rosso prima del verde** di `tests/tray.rs` non c'è: scritte dopo il codice del modello (dichiarato qui).

#### Connetti e disconnetti dal solo menu (T2)

- **Chi fa cosa** (Q52 (a), P6, P7): una voce «Connect»/«Disconnect» emette `galleria:tray-action` con `{ action: "connect", id }` o `{ action: "disconnect" }` e nient'altro (il tipo `ActionPayload` non ha altri campi); «Open Galleria» e «Quit» le fa Rust (`tray_tauri::run_action`) e non arrivano alla pagina. **`ui/src/app/tray.ts` è l'unico file che ascolta l'evento** (`grep -rn "tray-action" ui/src` ha un file di codice, più i test) e chiama **le stesse funzioni dei pulsanti** dello store di B5 (`connect`, `disconnect`, `askToDisconnect`): nessuna funzione copiata. Per una sessione estranea «Disconnect» non disconnette: apre la domanda e chiede la finestra (`window_show`).
- **La finestra si riapre** quando la pagina deve chiedere o far vedere qualcosa nato dal menu: `connect` e `disconnect` ricevono `{ onAttention }` e lo chiamano **solo** quando la chiamata è finita in una domanda (sessione estranea) o in un errore di comando; un menu che connette e basta non apre niente. `needs_input` con la finestra nascosta la riapre **Rust**, dal punto che riceve lo stato (`events.rs`, `window::show_if_hidden`): non dipende dalla pagina, e una finestra già aperta non perde il fuoco.
- **`window_show`** è il settimo comando (`allow-window-show`, `COMMANDS`, `tests/config.rs`): nessun parametro, risposta `()`. Non è generico sul runtime: i comandi parlano con `Arc<dyn WindowControl>` (`show`, `is_hidden`, `quit`; nello stato come l'annunciatore e i selettori di file), perché la prova «nessun comando prende un percorso» legge le firme `pub fn nome(`, e perché il runtime finto non sa dire che una finestra è nascosta (`is_visible` è sempre vero) né ha `request_exit`: le prove di livello 2 mettono una finestra che registra (`Options::window`).
- Prove: `tests/tray.rs` (`a_click_on_connect_reaches_the_page_as_an_action_with_the_id`, `open_and_quit_never_reach_the_page`, `no_tray_action_carries_more_than_an_action_and_an_id`, `needs_input_shows_the_hidden_window`, `a_question_leaves_a_window_that_is_shown_alone`, `window_show_is_a_command_with_no_parameter`), `app/tray-actions.test.tsx` (sei: lo stesso percorso del pulsante, la sostituzione di una sessione propria con un avviso e senza finestra, la sessione estranea mai chiusa senza conferma — per «Disconnect» e per «Connect» —, la disconnessione di una propria senza domande, l'errore di comando che mostra la finestra, eventi che non sono azioni), fumo `G10b_a_favourite_connects_and_disconnects_from_the_menu_alone` e `switching_favourite_from_the_menu_replaces_without_asking` (menu cliccato dal bus, `galleria status` dice `connected` e poi `disconnected`, una sola sessione, nessuna finestra di domanda).
- **S49** (a finestra nascosta la pagina riceve gli eventi): **non ancora provata**: fino al T3 non c'è modo di nascondere la finestra. `G10b` gira a finestra visibile e il T3 la porta a finestra nascosta (chiusa con la richiesta del gestore di finestre). `credentials_asked_after_a_menu_connect_bring_the_window_back` è del T3 per lo stesso motivo.

#### Chiusura nella tray, uscita esplicita, modifiche non salvate (T3)

- **La decisione è di Rust e sincrona** (`src/closing.rs`, nessun tipo di Tauri): `decide(intent, unsaved, tray_shown)` con la tabella delle otto combinazioni (`Hide` senza tray diventa `Quit`, Q53; con modifiche non salvate è `Blocked(intent)` con l'intento già corretto). `Closing` tiene **un bit** che la pagina aggiorna (`unsaved_set`: quando `isDirty` cambia valore e quando il form sparisce, non a ogni tasto; `app/closing.ts`) e **non aspetta la pagina**. Gestore di `CloseRequested` (`window::on_close_requested`, dopo il salvataggio della dimensione che c'è): `prevent_close` sempre, poi `Closing::request(Hide)`. «Quit» della tray, «Quit» del menu dell'app e `Ctrl+Q` passano dalla **stessa** decisione; `window_hide` e `app_quit` rispondono `{ done }` (`false`: fermati dalle modifiche). **`grep -rn "exit(" crates/galleria-gui/src`** trova un solo punto che chiude l'app, `TauriWindow::quit`.
- **Con modifiche non salvate** Rust mostra la finestra ed emette `galleria:close-blocked { intent }`; la pagina (`app/closing.ts`, l'unico file che lo ascolta) apre **la stessa guardia** «Save · Discard · Cancel» di B3 (`Shell.tsx`); «Save» riuscito o «Discard» → `unsaved_set(false)` e il comando dell'intento; «Cancel» non fa niente; un salvataggio fallito lascia la finestra dov'è con l'errore del form.
- **«C'è chi mostra la tray?»** (`src/tray_host.rs`, S51): sul bus di sessione `org.kde.StatusNotifierWatcher` ha un proprietario e `IsStatusNotifierHostRegistered` è vero; senza bus, senza proprietario o senza risposta in 300 ms è falso. Ogni domanda apre una connessione sua (quella del processo è condivisa). La tray «si vede» se esiste la nostra **e** il bus lo dice, adesso. **Provato** nell'immagine su un bus privato senza watcher e senza bus (`tray_is_shown_is_false_without_a_bus_and_without_an_owner`, la prova non si salta se manca `dbus-daemon`); la parte «vero» è provata dal fumo con il watcher della sonda (`G10c`).
- **«Esci» nel menu dell'app** (`AppMenu.tsx`), prima di «About Galleria» (così `End` arriva ancora a «About», come nelle prove di tastiera e di Orca di B2–B5) e `Ctrl+Q`; il testo è «Quit (the VPN stays on)» quando c'è una sessione (Q54). Testi: `tray.quit` e `tray.quit_connected` dei cataloghi.
- **Fumo contro l'app vera, S53 e S60 sciolte**: sotto Xvfb la richiesta di chiusura si manda con `lab/tests/x11-window.py` (`WM_DELETE_WINDOW` con `ctypes`/libX11; legge anche se la finestra è mappata). `G10c`: la finestra non è più mappata, il processo è vivo, `galleria status` dice `connected`, la riga di stato del menu dice «Connected to …»; `G10d` (nella stessa prova): «Open Galleria» la rimappa, «Quit (the VPN stays on)» finisce il processo e `connected` resta; `G10e`: senza watcher la richiesta di chiusura **finisce l'app** e `connected` resta. Tre prove, giro filtrato.
- **Provati solo in jsdom e a livello 2** (e dichiarati): `G10f` (modifiche non salvate: la guardia con il fuoco su «Cancel», «Discard» che nasconde, «Save» e «Quit» dalla tray) **non è scritta come prova di fumo**; la copertura è `tests/closing.rs` (la decisione, `window_hide`/`app_quit` con lo stesso `{ done }`, l'evento che porta l'intento, «Quit» della tray), `app/closing.test.tsx` e le prove della guardia di B3. Mancano anche `a_second_launch_shows_the_hidden_window`, `credentials_asked_after_a_menu_connect_bring_the_window_back` e il passaggio di `G10b` a finestra nascosta (S49 resta **non provata**): vedi l'handoff del gruppo.
- **Una prova che non c'è** (dichiarato): `the_size_is_saved_before_the_window_hides` non è una prova a sé: il runtime finto non manda `CloseRequested` fuori da `run`; l'ordine (`on_event` salva, poi `on_close_requested`) è nel gestore di `lib.rs`.
- Quattro comandi nuovi (`unsaved_set`, `window_hide`, `app_quit` e, da T2, `window_show`): `COMMANDS`, `capabilities/main.json` e `tests/config.rs` d'accordo; nessun comando prende un percorso né restituisce un segreto.
