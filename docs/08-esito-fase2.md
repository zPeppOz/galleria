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
