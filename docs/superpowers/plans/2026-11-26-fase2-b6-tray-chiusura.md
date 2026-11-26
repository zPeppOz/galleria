# Fase 2 · Blocco B6 — Tray, versione, chiusura: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** la finestra si chiude e Galleria resta nella tray, con stato, preferiti, connetti e disconnetti nel menu;
l'uscita è esplicita; chi ha modifiche non salvate viene fermato prima (Q35). Un avviso dice quando c'è una versione
nuova, con una sola richiesta al giorno e senza identificativi (D16). Poi la chiusura: fumo completo, prove manuali sui
tre desktop obbligatori, le riserve di B3–B5 chiuse o dichiarate come limiti, `docs/08-esito-fase2.md`. Chiude **G10,
G11, G13** e, con loro, **la fase 2**.

**Architettura:** il lato Rust di `galleria-gui` guadagna tre moduli senza `tauri` (`tray`, `closing`, `update`) e sei
comandi sottili. **Il menu della tray è un modello puro** (`tray::model`): stato, preferiti e testi entrano, una lista
di voci esce; il pezzo che parla con Tauri la copia nel menu vero. **Le azioni del menu passano dalla pagina** (Q52):
la pagina è viva anche a finestra nascosta e ha già tutta la logica di B5 (sostituzione, sessione estranea,
credenziali). **La chiusura la decide Rust** nel gestore `CloseRequested` (Q35), su un solo bit che la pagina tiene
aggiornato. **La richiesta delle versioni parte da Rust** con la libsoup che WebKitGTK ha già caricato (Q51): nessun
crate nuovo. L'IPC della fase 1 non cambia e non viene nemmeno usato da questo blocco, se non per ciò che B5 fa già.

**Stack:** quello di B5. In più: la feature `tray-icon` di `tauri` (zero pacchetti nuovi in `Cargo.lock`, misurato),
`soup3` 0.5 e `semver` 1 come dipendenze dirette (già in `Cargo.lock`, zero pacchetti nuovi). Una VM nuova, Ubuntu
24.04 desktop, in `/mnt/b/vm`.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md`, §1 (G10, G11, G12, G13), §3.2, §3.4, §4.2, §12.1,
§13, §14, §15 («spec §n»). Stato di partenza: `docs/08-esito-fase2.md`, sezione «B5». D16: `docs/01-analisi.md` §2.
Decisioni D5–D32 e Q12–Q49: chiuse.

**Prima di partire:**

- `main` deve contenere questo piano **e il commit della card `t_016cb5ea`** (stabilità della suite `a11y`: ogni prova
  o ogni file con la sua sessione). B6 non tocca quello scope: le prove Orca nuove vanno in un file nuovo
  (`ui/e2e/tray.a11y.ts`) e seguono la struttura che quella card lascia. Se la card ha chiuso senza la causa, B6 **non**
  la riapre: riporta i numeri dei suoi giri e l'instabilità resta scritta nell'esito.
- Le questioni Q50–Q57 (in fondo) devono avere una risposta. Se nessuno risponde vale la raccomandazione: il piano è
  scritto su quelle. **Q52 e Q53 cambiano il gruppo A, Q50 e Q51 il T4, Q56 e Q57 il gruppo D.**
- Worktree dedicato a partire da `main`.

Legenda: **[V]** verificato oggi, con la fonte · **[S]** supposto, con il task che lo verifica.

## Gruppi di lavoro: una card per gruppo

Dodici task in cinque gruppi (la spec §15 ne stimava 6: il doppio viene dalle riserve ereditate, dalla VM nuova e
dalla chiusura della fase). I gruppi A, B, C ed E vanno in fila: ognuno parte dal ramo del precedente, già rivisto.
**Il T8 (la VM Ubuntu 24.04) può partire subito, in parallelo al gruppo A**: tocca solo `lab/vm/` e `/mnt/b/vm`, e la
prima accensione installa il desktop da sola in mezz'ora.

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Tray e chiusura** | T1, T2, T3 | menu della tray dal modello puro, con la sonda che lo legge dal bus; azioni del menu; chiusura nella tray, uscita esplicita, guardia delle modifiche non salvate (Q35) | `main` con questo piano e `t_016cb5ea`; Q52, Q53, Q54 decise | 16 h | la finestra si chiude, la VPN resta, dal menu ci si connette e si esce |
| **B · Avviso di versione** | T4, T5 | richiesta delle versioni da Rust con il server finto che conta (G11); interruttore, avviso, «Esci» nel menu dell'app | A; Q50, Q51, Q55 decise | 9 h | l'app dice quando c'è una versione nuova, e tace se glielo si chiede |
| **C · Gate in CI e fumo completo** | T6, T7 | G10 e G11 nei job, con il manifesto; il tunnel vero che sopravvive alla finestra chiusa, sui tre client; il giro completo; Orca a finestra nascosta | B | 10 h | **tutto ciò che è automatico è in CI** |
| **D · Desktop veri** | T8, T9, T10 | VM Ubuntu 24.04; passata manuale di G10 e delle riserve di B3–B5 sui tre desktop; Orca 46 (Q28, Q49), passata con lo screen reader (Q22), avvio a freddo (G13) | C (il T8 anche prima); Q56, Q57 decise | 14 h | ogni riserva è chiusa o è un limite scritto |
| **E · Chiusura della fase 2** | T11, T12 | esito con G1–G13, limiti e «non verificato»; README; spec; roadmap; cosa resta per la fase 3 | D | 6 h | **fase 2 chiusa** |

Le stime contano **un solo giro di `scripts/gui.sh smoke` e uno di `scripts/gui.sh a11y` per gruppo, a fine gruppo**
(gruppi A, B, C). I giri ripetuti si fanno una volta sola in tutto il blocco, alla fine del gruppo C. Il gruppo D non
tocca il codice, salvo le correzioni che la passata trova (con `--fixup` sul task a cui appartengono, e allora si
rilancia il giro di quel gruppo).

Ogni card riporta: numero dei task, i comandi di «Dove girano i comandi» verdi sull'ultimo commit, la sezione «B6» di
`docs/08-esito-fase2.md` aggiornata con i propri task. La review è una per gruppo.

## Criteri di uscita del blocco

| # | Criterio | Come si verifica | Task |
|---|---|---|---|
| F1 | **G10**, parte automatica: il menu dice lo stato e i preferiti; dal solo menu ci si connette e ci si disconnette, a finestra nascosta; chiudere la finestra non chiude la sessione; l'uscita è esplicita | job `gui-smoke`: `G10a`–`G10f`, con il menu letto e cliccato dal bus; job `lab-connect`: `G10g` e `gui_closed_window_kept_the_tunnel` (`t18`) sui tre client | T1, T2, T3, T6 |
| F2 | **G10**, parte manuale (è la forma che la spec chiede): la stessa lista su Ubuntu 24.04, Ubuntu 26.04 e Fedora 45, GNOME con AppIndicator | lista M1–M12 annotata nell'esito, con le prove in `/mnt/b/vm/evidence/b6/` | T9 |
| F3 | **G11**: una sola richiesta, senza identificativi; nessuna richiesta se disattivato; al più una al giorno | job `gui`: `update_*` contro un server finto che conta richieste e intestazioni; job `gui-smoke`: `G11a` con l'app costruita come per il rilascio | T4, T5, T6 |
| F4 | **Q35**: con modifiche non salvate né la chiusura né «Esci» buttano via niente senza chiedere; lo decide Rust | job `gui`: `closing_*`; job `gui-smoke`: `G10f` | T3 |
| F5 | **G13**: tre misure dell'avvio a freddo, scritte. Non blocca | manuale, `lab/vm/startup-time.sh` (Q57); **nessun job**, per spec | T10 |
| F6 | **G12**, il resto: passata con lo screen reader su un desktop vero (Q22); Orca 46 su Ubuntu 24.04 con il nome dei campi e la finestra delle credenziali (Q28, Q49); gli annunci di stato a finestra nascosta | trascrizioni annotate nell'esito (Q56); `K6a`, `K6b` in `scripts/gui.sh a11y` (locale, Q22) | T7, T10 |
| F7 | Le riserve di B3–B5 hanno ognuna un esito: **chiusa** con la prova, o **limite** scritto nell'esito e nel README | tabella «Riserve ereditate» sotto, riga per riga | T9, T10, T11 |
| F8 | La fase 1 non se ne accorge: `git diff main -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-net crates/galleria-ovpn crates/galleria-profile crates/galleria-cli` è vuoto; `lab`, `lab-dco`, `lab-g3` restano verdi | il comando; i job | ogni task |
| F9 | **G1–G13**: ognuno ha la sua prova, il suo job e il numero di un run verde sull'ultimo commit, oppure la riga dice cosa manca | tabella finale dell'esito | T11 |

### Dove gira ogni gate in CI

Vale la regola di B5: ogni gate ha un job, e ogni job che dichiara un gate finisce con `scripts/check-gates.py`. Il
manifesto di B6 è un file a parte, `lab/tests/gates-b6.tsv`, stesso formato; `check-gates.py` legge **tutti** i
`lab/tests/gates-*.tsv` (oggi ha il solo `gates-b5.tsv` come predefinito **[V: `scripts/check-gates.py`, riga 94]**).

| Gate | Job | Comando del job | Prove che il job deve aver eseguito |
|---|---|---|---|
| G10 | **`gui-smoke`** | `scripts/gui.sh smoke`, poi `check-gates.py --job gui-smoke` con il JSON del fumo | `G10a`–`G10f` (`ui/e2e/tray.e2e.ts`) |
| G10 | **`lab-connect`** (matrice 26, 27, 79) | `LAB_BUILD=host scripts/gui.sh lab-connect <client>` | `G10g_the_tunnel_survives_the_closed_window` (finestra), `gui_closed_window_kept_the_tunnel` (`t18`, nel client) |
| G10 | — (manuale, come dice la spec) | lista M1–M12 su tre VM | nessun job: l'esito ha la lista annotata |
| G11 | **`gui`** | `scripts/gui.sh check-here` | `update_asks_once_and_sends_no_identifier`, `update_disabled_asks_nothing`, `update_asks_at_most_once_a_day`, `update_never_follows_a_redirect` |
| G11 | **`gui-smoke`** | come sopra | `G11a_the_built_app_asks_once_and_never_when_disabled` |
| G13 | — (manuale, non blocca) | `lab/vm/startup-time.sh` in una VM | nessun job: tre numeri nell'esito |

Oggi l'ultimo passo di `gui-smoke` controlla solo le prove `cargo` del portachiavi (`--kind cargo`) **[V: `ci.yml`,
riga 201]**: il T6 fa scrivere al fumo il JSON del reporter, come fa già `lab-connect`, e lo passa a `check-gates.py`.
La prova `every_b5_gate_has_a_job` (`tests/config.rs`) diventa `every_gate_has_a_job` e vale anche per G10 e G11.

**In locale** vale ciò che valeva in B5: `lab-connect 26` è intero, su 27 e 79 OpenVPN 2.7 è rosso per il DCO che
Docker rootless nega; `G10g` usa WireGuard, quindi **in locale è verde su tutti e tre**. La prova dei job resta il run
di CI dopo il push, che fa Aitano.

### Riserve ereditate

Sono gli impegni che B3, B4 e B5 hanno lasciato a B6. Ognuna esce da questo blocco **chiusa** o **limite dichiarato**:
«da rivedere» non è un esito ammesso.

| # | Riserva | Da dove | Come si chiude | Se non si chiude |
|---|---|---|---|---|
| R1 | Chiusura della finestra con modifiche non salvate | Q35 (B3, T5) | T3, in CI | — (è codice: si fa) |
| R2 | Orca 46 su Ubuntu 24.04 desktop vero: nome dei campi di testo e finestra delle credenziali | Q28, Q49, `S4b`, `K5c` | T10: `orca-spike-run.py fields` e `orca-credentials-vm.sh` sulla VM nuova | limite scritto per 24.04: «con Orca 46 i campi di testo si leggono dall'albero, non dal parlato»; Q49 (a) resta |
| R3 | Passata con lo screen reader su un desktop vero | Q22 | T10, con Q56 | la parte «a orecchio» resta una lista per Peppe, non bloccante (Q56) |
| R4 | Finestre «Apri» e «Salva» di GTK su Wayland e sotto un portale | B3 (T8), B4 (T3), B5 (T7) | T9: M7, M8 sulle tre VM (GNOME su Wayland) | il portale (app in sandbox: Flatpak, Snap) è **limite dichiarato** comunque: la fase 2 non fa pacchetti |
| R5 | File manager vero: doppio click su un `.ovpn` e trascinamento da Nautilus | B4 (T4, T5) | T9: M9, M10 | limite scritto: «associazione provata con `gio launch`» |
| R6 | La finestra esistente portata davanti (S31) | B4 (T5) | T9: M11; in più il T3 la usa per riaprire la finestra nascosta | limite scritto: «su Wayland il compositore può negare il fuoco; la finestra compare comunque» |
| R7 | `Gtk-CRITICAL … WIDGET_REALIZED_FOR_EVENT` della finestra «Apri» | B4 (T3) | T9: M7 guarda `stderr` con una persona vera alla tastiera. Nell'esito di B4 compare solo con il tasto `/` sintetico mandato via AT-SPI **[V: docs/08, riga 1073]** | se compare anche a mano: un'ora per la causa, poi limite scritto con il testo esatto |
| R8 | GNOME Keyring su un desktop vero: sblocco con la finestra del sistema; «Ricorda» contro un portachiavi bloccato | B5 | T9: M12 (la Fedora ha il portachiavi bloccato dopo l'accesso automatico **[V: docs/08, Q49]**) | limite scritto |
| R9 | KWallet (S38) | B5 | **non si prova**: KDE è «best effort» nell'analisi §8 | limite dichiarato, già nel README |
| R10 | La finestra intera su Ubuntu 26.04 e Fedora (oggi solo l'immagine 24.04) | B4, B5 | T9: la lista gira sulle tre VM | — |
| R11 | Dimensione della finestra salvata su Wayland (S23, Q34) | B3 | T9: M11 | limite scritto |
| R12 | S42, S43, S44 di B5 senza un esito scritto | esito di B5, ultima riga | T11: letti uno per uno sulle prove che li toccano | — |
| R13 | Instabilità della suite `a11y` | B5 | card `t_016cb5ea`, fuori da B6 | B6 riporta i numeri dei suoi giri |

## Vincoli globali

Valgono i vincoli dei piani di B1–B5 (un task = un commit, messaggi in inglese minuscolo imperativo, TDD, nessun push,
niente `unwrap()` fuori dalle prove, la GUI non esegue processi, CSP invariata, testi solo nei cataloghi,
`#![forbid(unsafe_code)]`, ogni codice nuovo in `GUI_CODES` e nei due cataloghi nello stesso commit, ogni comando nuovo
in `COMMANDS`, in `capabilities/main.json` e in `tests/config.rs` nello stesso commit, nessun comando con un percorso,
nessun comando che restituisce un segreto, niente prove che si saltano da sole). In più:

- **Non si toccano**: `crates/galleriad`, `galleria-ipc`, `galleria-backend*`, `galleria-net`, `galleria-ovpn`,
  **`galleria-profile`, `galleria-cli`** (B6 non ha eccezioni), `design/`, `docs/00`–`07`.
- **Nessun crate nuovo in `Cargo.lock`.** La tray, la richiesta e il confronto delle versioni usano pacchetti che ci
  sono già (misurato: sotto). Se a un task ne serve uno, ci si ferma e si riporta.
- **Rete.** L'unica connessione di rete della GUI è la richiesta delle versioni (D16): parte da Rust, verso un solo
  indirizzo scritto nel codice, mai dalla pagina. La CSP non cambia. Nessun'altra richiesta, nemmeno per l'icona o per
  il testo delle note di rilascio.
- **Niente che identifichi.** La richiesta non porta `User-Agent`, cookie, `Referer`, `If-None-Match`, parametri, né
  la versione installata. Una prova lo tiene fermo sulle intestazioni che il server finto riceve.
- **Ciò che arriva dalla rete non è fidato.** Dalla risposta esce al più **un numero di versione**, riletto con
  `semver`; l'indirizzo che la pagina mostra lo ricostruisce Rust dal numero. Nessuna stringa della risposta arriva alla
  pagina o su `stderr` com'è.
- **La tray è un accessorio** (analisi §8). Tutto ciò che il menu fa si fa anche dalla finestra; senza tray l'app è
  completa, non va in panico e non resta nascosta senza un modo per tornare (Q53).
- **Annunci (Q27).** Le azioni del menu non annunciano niente di nuovo: i cambi di stato li annuncia già Rust (Q43,
  P14 di B5), una volta, anche a finestra nascosta.
- **Dove girano i comandi.** Ogni **task** dei gruppi A, B, C finisce con:

  ```bash
  # host: la fase 1 (deve restare com'è)
  cargo fmt --all --check
  cargo clippy --workspace --exclude galleria-gui --all-targets -- -D warnings
  cargo test --workspace --exclude galleria-gui
  cargo deny check
  scripts/check-boundaries.sh
  # immagine: la GUI
  scripts/gui.sh check
  ```

  Ogni **gruppo** (A, B, C) finisce, una volta sola e sull'ultimo commit, con `scripts/gui.sh smoke` e
  `scripts/gui.sh a11y` (locale, Q22); il gruppo C anche con `scripts/gui.sh keyring` e `scripts/gui.sh lab-connect 26`.
- **API di Tauri, `gtk`, `soup3`**: nomi e firme dai sorgenti **della versione nel `Cargo.lock`**. I nomi di questo
  piano valgono per il nostro codice.
- **VM.** Dischi, immagini e prove stanno solo in `/mnt/b/vm`. Le VM si spengono a fine task (`vmctl.sh stop`). Nel
  repo entrano script e modelli (`lab/vm/`), mai immagini, chiavi o fotografie.
- Se un punto **[S]** risulta falso si applica il ripiego scritto e lo si annota in `docs/08-esito-fase2.md`, sezione
  «B6». Se il ripiego tocca una decisione D o Q chiusa, ci si ferma e si riporta.

## Fatti letti e misurati per questo piano

Script e uscite in `spikes/b6-piano/` (README con le tabelle intere).

### Il codice che B6 trova **[V: letto oggi a `7e017c9`]**

| Fatto | Fonte | Conseguenza |
|---|---|---|
| L'app finisce quando si chiude l'ultima finestra: `run()` costruisce e chiama `.run(generate_context!())`, senza gestore degli eventi dell'app | `crates/galleria-gui/src/lib.rs`, `run` | T3: `CloseRequested` va fermato (`prevent_close`) e la finestra nascosta; l'uscita diventa `AppHandle::exit` |
| `on_window_event` ha già un gestore: a `CloseRequested` salva la dimensione (`WindowSaver::save_now`) | `src/window.rs`, `on_event` | T3 aggiunge la decisione **dopo** il salvataggio, nello stesso punto |
| `window::raise` fa `show`, `unminimize`, `set_focus`; la usa il secondo avvio | `src/window.rs` | «Apri Galleria» e il secondo avvio riaprono la finestra nascosta con il codice che c'è (S31, R6) |
| «Modifiche non salvate» vive solo nella pagina: `isDirty(draft, base)` in `ui/src/rubrica/draft.ts`, la guardia «Salva · Scarta · Annulla» in `Shell.tsx` (`guard.current?.dirty`) | i due file | T3: la pagina tiene Rust informato con un bit (`unsaved_set`); la guardia che c'è si riusa per la chiusura |
| Lingua e tema si scelgono nel menu dell'app (`ui/src/app/AppMenu.tsx`) e si salvano con `ui_state_set` (`UiPatch`) | i file | T5: l'interruttore dell'avviso e «Esci» stanno lì; niente schermata «Impostazioni» nuova |
| `tauri` è dichiarato con `features = []`: la tray non è compilata | `crates/galleria-gui/Cargo.toml` | T1 aggiunge `tray-icon` |
| `gtk` 0.18 è già una dipendenza diretta, «la stessa che Tauri porta», usata per l'annuncio ATK e per le finestre «Apri» e «Salva» | `Cargo.toml`, `announce_atk.rs`, `picker_gtk.rs` | lo stesso schema vale per `soup3` (Q51) e per leggere il bus con `gtk::gio` |
| Le frasi tradotte degli annunci di stato le consegna la pagina a Rust (`announce_phrases`, Q43) | `src/session.rs`, `ui/src/app/announce.ts` | i testi del menu arrivano allo stesso modo (`tray_labels`, P3) |
| `Sessions::on_state` è chiamata dal supervisor per ogni `event.state`, prima che l'evento vada alla pagina | `src/session.rs`, `src/supervisor.rs` | è il punto in cui il menu si aggiorna e in cui la finestra si riapre su `needs_input` |
| Le icone dell'app sono tre PNG (`icons/32x32.png`, `128x128.png`, `icon.png`) | `crates/galleria-gui/icons/` | T1 ne aggiunge tre per la tray |

### La tray di Tauri su Linux **[V: sorgenti delle versioni in `Cargo.lock`]**

| Fatto | Fonte | Conseguenza |
|---|---|---|
| La feature `tray-icon` aggiunge **0** pacchetti a `Cargo.lock` e 4 crate all'albero di `galleria-gui`; `cargo deny` resta verde | `spikes/b6-piano/`, misurato | nessuna questione sul crate |
| `libappindicator-sys` carica a runtime `libayatana-appindicator3.so.1`, poi `libappindicator3.so.1`, e **va in panico** se non ne trova | `libappindicator-sys` 0.9.0, `src/lib.rs`, righe 14–50 | la costruzione della tray va protetta: senza libreria niente tray e niente panico (S48) |
| Sulla Fedora 45 della VM c'è `libappindicator3.so.1` (`libappindicator-gtk3`), non la `ayatana` | VM accesa oggi | la seconda scelta del caricamento è quella che la Fedora usa: va vista funzionare (S56) |
| Gli eventi dell'icona su Linux non arrivano; il tooltip non c'è | `tauri` 2.12.7, `src/tray/mod.rs`, righe 66, 264 | tutto nel menu (spec §13) |
| Un menu, una volta messo, non si toglie e non si sostituisce; se ne cambia il contenuto | stessa fonte, riga 240 | il menu si crea una volta e poi se ne riscrivono le voci (S47) |
| L'icona si scrive come PNG in `$XDG_RUNTIME_DIR/tray-icon`, o nella cartella temporanea, salvo `temp_dir_path` | `tray-icon` 0.25.1, `platform_impl/gtk/mod.rs` | P5: la cartella è `$XDG_RUNTIME_DIR/galleria`, che è già `0700` (Q36) |
| `tauri::include_image!` incorpora un PNG già decodificato | `tauri` 2.12.7, `src/lib.rs`, riga 86 | non serve la feature `image-png` (+4 pacchetti) |
| `tray-icon` ha un secondo motore, `ksni`, senza librerie di sistema; `tauri` 2.12.7 non lo espone | `tray-icon` 0.25.1, `Cargo.toml` e `src/lib.rs` | fuori da B6: è ciò che `docs/07` §3.6 chiamava «non ci contiamo» |
| **Il menu di un AppIndicator si legge e si clicca dal bus di sessione**, nell'immagine, sotto Xvfb, con un `StatusNotifierWatcher` finto di 40 righe: voci, stato «attiva», click, menu riletto dopo il click | `spikes/b6-piano/tray-probe.py`, **eseguito** | G10 ha una prova automatica in `gui-smoke`, non solo la lista manuale. Resta [S] contro il menu di `muda` (S46) |
| Senza nessuno che faccia da `StatusNotifierWatcher` l'indicatore non si registra, e il processo resta vivo | stessa sonda, `--no-watcher` | è il segnale per «qui la tray non si vede» (Q53, S51) |

### La richiesta delle versioni (misurato)

| Fatto | Fonte | Conseguenza |
|---|---|---|
| `GET https://github.com/<proprietario>/<repo>/releases/latest` risponde **302** con `Location: …/releases/tag/<tag>`, **anche senza `User-Agent`** | `curl`, oggi, su `tauri-apps/tauri` | il numero di versione sta in un'intestazione; non serve leggere un corpo |
| L'API (`api.github.com/repos/…/releases/latest`) senza `User-Agent` risponde **403**; con, 200 e 39 kB di JSON; limite di **60 richieste all'ora per indirizzo IP** | `curl`, oggi | l'API chiede un'intestazione in più, e in un ufficio dietro un solo indirizzo il limite è di tutti: Q50 |
| `zPeppOz/galleria` è pubblico e **non ha release**: la prima forma dà 302 verso `…/releases`, l'API 404 | `gh repo view`, `curl` | «nessuna versione nota» è un caso normale, e oggi è l'unico che si vede contro GitHub vero |
| `soup3` 0.5.0 è già nell'albero (lo porta `webkit2gtk`): 0 pacchetti e 0 crate in più, `cargo deny` verde | misurato | Q51 |
| `ureq` 3 con rustls: +7 pacchetti e `cargo deny` **rosso** (`webpki-roots` è `CDLA-Permissive-2.0`); `reqwest` 0.13 con native-tls: +8 pacchetti, verde | misurato | Q51 |
| `soup3` ha `Session`, `Message::request_headers`, `response_headers`, `status`, `set_flags`, e `MessageFlags::NO_REDIRECT` | `soup3` 0.5.0, `src/auto/` | la richiesta si fa senza seguire il rinvio; il nome della chiamata asincrona lo legge il T4 |
| `semver` 1.0.28 è già in `Cargo.lock` | misurato | il confronto non si scrive a mano |

### Le VM

| Fatto | Fonte | Conseguenza |
|---|---|---|
| In `/mnt/b/vm` ci sono `ubuntu` (26.04) e `fedora` (45), spente, con GNOME su Wayland, l'estensione AppIndicator, Orca, Nautilus; 1,7 TB liberi, KVM, 31 GB di RAM | `ls`, `df`, VM Fedora accesa oggi | la terza VM ci sta |
| `vmctl.sh` conosce solo `ubuntu` (porta 2201) e `fedora` (2202); le VM nascono da un'immagine cloud più cloud-init (`lab/vm/*-user-data.tmpl`), con firma e hash verificati | `lab/vm/vmctl.sh`, `lab/vm/README.md`, `/mnt/b/vm/images/fetch.sh` | T8: terzo nome `ubuntu24`, porta 2203, stesso modello |
| L'immagine cloud di Ubuntu 24.04 esiste all'indirizzo `…/releases/noble/release/ubuntu-24.04-server-cloudimg-amd64.img` (621 MB) | `curl -I`, oggi | non scaricata |
| Per Q49 la finestra è già andata sulle due VM con **i binari costruiti nell'immagine** `galleria-gui-dev` (Ubuntu 24.04, glibc 2.39), pilotata con `vmkeys.py`, `vncshot.py`, AT-SPI e il file di debug di Orca, senza audio | `lab/vm/orca-credentials-vm.sh`, docs/08 «Orca e la finestra delle credenziali» | la passata di B6 usa gli stessi attrezzi (P12) |
| Sulla macchina di Peppe (WSL) la finestra non si può avviare: mancano le librerie di WebKitGTK, e Q15 esclude `sudo` | spec §14, Q15 | G13 «sulla macchina di Peppe» non si può misurare alla lettera: Q57 |

## Punti di attenzione per la revisione

1. **Una finestra nascosta deve sempre poter tornare.** Si nasconde solo se c'è chi mostra la tray (un
   `StatusNotifierWatcher` con un ospite registrato, chiesto al bus **nel momento della chiusura**); altrimenti la
   chiusura è un'uscita, come oggi (Q53). In ogni caso il secondo avvio riapre la finestra (`window::raise`, che c'è).
   → T3 `without_a_tray_host_a_close_is_a_quit`, `a_second_launch_shows_the_hidden_window`.
2. **Senza la libreria la tray non c'è, e l'app parte.** Nessun panico arriva all'utente. → T1
   `the_app_starts_without_the_indicator_library` (nell'immagine, con la libreria nascosta).
3. **Il menu non decide niente.** Una voce del menu manda alla pagina la stessa azione di un pulsante: la sostituzione
   senza conferma tra profili propri, la conferma per una sessione estranea, la finestra delle credenziali restano
   dove B5 le ha messe e non hanno una seconda copia in Rust. Quando serve l'utente (conferma, credenziali, errore di
   comando) **la finestra si riapre**. → T2 `a_foreign_session_is_never_closed_from_the_menu_without_the_confirmation`.
4. **`needs_input` a finestra nascosta la riapre**, e lo fa Rust, dal punto che riceve lo stato: non dipende da una
   pagina che potrebbe essere ferma. → T2 `needs_input_shows_the_hidden_window`.
5. **La chiusura la decide Rust, in modo sincrono**, sul bit che la pagina ha consegnato: non aspetta una risposta
   della pagina dentro il gestore dell'evento. Una pagina bloccata non impedisce di chiudere una finestra senza
   modifiche. → T3 `a_close_without_unsaved_changes_does_not_wait_for_the_page`.
6. **«Esci» non chiude la VPN.** La sessione è del demone (spec §3.4). Il menu lo dice nella voce stessa quando c'è
   una sessione (Q54): nessuno deve scoprirlo dopo.
7. **Una richiesta vuol dire una.** Nessun secondo tentativo dopo un errore, nessun rinvio seguito, nessuna richiesta
   per l'icona, nessuna alla riapertura della finestra. Anche una richiesta fallita conta per le 24 ore. → T4.
8. **Il segno di una versione nuova non è un `Location` qualsiasi.** Vale solo `…/releases/tag/v<semver>` dello stesso
   repository; tutto il resto è «nessuna versione nota», senza errore. Una pre-release non fa comparire l'avviso.
   → T4 `only_a_newer_stable_version_of_this_repository_counts` (parametrica, con rinvii ostili).
9. **`state.toml` resta leggibile da chi c'era prima.** I campi nuovi sono facoltativi e hanno un valore predefinito;
   `schema_version` resta 1 solo se un file di oggi si rilegge uguale e un file con i campi nuovi è letto da B5 senza
   perdere il resto (lo guarda il T4 su `ui_state.rs`; se `UiState` rifiuta i campi sconosciuti, la versione sale e si
   scrive).
10. **Il laboratorio non deve arrossire per la tray.** `lab-connect` oggi non ha nessun `StatusNotifierWatcher`: con
    Q53 (a) la chiusura lì sarebbe un'uscita. `G10g` avvia il finto prima dell'app; le ventuno prove di B5 girano
    come prima, nello stesso giro, e la misura stretta di Q48 non cambia.
11. **«Manuale» non vuol dire «a parole».** Ogni riga della lista M1–M12 ha la sua prova conservata: la fotografia
    dello schermo (`vncshot.py`), l'uscita di un comando, o la riga del file di debug di Orca. Una riga senza prova è
    «non verificato».
12. **Niente regressioni**: le 50 prove di fumo, le prove Orca e le 21 di `lab-connect` di B5 restano. Tutte le prove
    di fumo che chiudevano l'app chiudendo la finestra vanno riviste nel T3: ora la chiusura può nascondere.

## Decisioni di progetto prese qui

Valgono per tutti i gruppi; chi implementa non le riapre.

| # | Decisione | Perché |
|---|---|---|
| P1 | Comandi di B6, sei: `tray_labels(labels) -> ()` · `unsaved_set(dirty) -> ()` · `window_hide() -> { done }` · `window_show() -> ()` · `app_quit() -> { done }` · `update_view() -> { current, latest, url, enabled }`. La spec §4.2 elencava `settings_get`, `settings_set`, `update_check`, `app_quit` «di riferimento»: le impostazioni passano da `ui_state_get` e `ui_state_set`, che ci sono già, con due campi in più in `UiPatch` (`update_check`, `update_dismissed`) | un solo posto per ciò che l'utente sceglie; la richiesta non si comanda dalla pagina (punto 7) |
| P2 | Il menu, dall'alto: una riga di stato spenta («Connesso a «X»» · «Disconnesso» · «Connessione in corso…» · «Servizio non raggiungibile»); un separatore; i **preferiti** nell'ordine della lista, al più 8, ognuno «Connetti «X»», e quello attivo «Disconnetti «X»»; se la sessione attiva è di un profilo non preferito o estranea, una riga «Disconnetti «X»» in più; un separatore; «Apri Galleria»; «Esci» | spec §13; il mockup mostrava anche i non preferiti: la spec dice «preferiti» |
| P3 | I testi del menu li consegna la pagina (`tray_labels`), al montaggio e a ogni cambio di lingua, come le frasi di Q43. La tray si crea **quando arrivano i testi la prima volta**: prima non c'è | D22: i cataloghi stanno nella pagina; un menu in inglese per un istante è peggio di un menu che compare mezzo secondo dopo |
| P4 | Con il collegamento al demone non `connected` le voci «Connetti» sono spente; «Apri Galleria» ed «Esci» restano | lo stesso del pulsante di B5 |
| P5 | Tre icone nostre, PNG 32×32 incorporati con `include_image!`: disconnesso, connesso, in corso o errore. La cartella dell'icona temporanea è `$XDG_RUNTIME_DIR/galleria` | lo stato si vede senza aprire il menu; niente file in `/tmp` |
| P6 | Un'azione del menu è un evento alla pagina: `galleria:tray-action` con `{ action: "connect", id }` o `{ action: "disconnect" }`. «Apri Galleria» ed «Esci» li fa Rust | Q52; punto 3 |
| P7 | La finestra si riapre da sola in tre casi: arriva `needs_input` (Rust); la pagina deve chiedere una conferma o mostrare un errore di comando nato da un'azione del menu (`window_show`); arriva un secondo avvio | punto 4 |
| P8 | `unsaved_set(dirty)` porta **solo il bit**. A una chiusura o a un «Esci» con il bit acceso Rust non chiude: mostra la finestra ed emette `galleria:close-blocked` con `{ intent: "hide" \| "quit" }`; la pagina apre la guardia che c'è («Salva · Scarta · Annulla»), e dopo «Salva» riuscito o «Scarta» richiama `window_hide` o `app_quit`. «Annulla» non fa niente | Q35; la guardia è una sola |
| P9 | `window_hide` e `app_quit` passano **tutti e due** dalla stessa decisione di Rust (`closing::decide`), anche quando li chiama la pagina: `{ done: false }` vuol dire «fermato dalle modifiche non salvate» | un solo punto che decide |
| P10 | «Esci» è anche nel menu dell'app e risponde a `Ctrl+Q`. La «X» della finestra nasconde (se c'è la tray) | spec §13: «l'uscita è esplicita» |
| P11 | La richiesta delle versioni parte 10 secondi dopo l'avvio e poi quando sono passate 24 ore dall'ultima (un controllo ogni ora, senza rete, guarda l'orologio). `state.toml` tiene `[update]`: `check` (predefinito `true`), `last_check`, `latest`, `dismissed`. L'avviso è una riga in linea in cima alla lista, non a scomparsa: «È disponibile Galleria X (hai Y)», l'indirizzo come testo, «Copia link» (Q55), «Non ricordarmelo» (per quella versione) | D16: «disattivabile» vuol dire acceso di suo; una al giorno anche per chi non chiude mai |
| P12 | Le prove manuali le esegue un agente con gli attrezzi dello spike (`vmkeys.py`, `vncshot.py`, AT-SPI, file di debug di Orca) sulle tre VM, con il **binario costruito nell'immagine** e installato con uno script (`lab/vm/gui-vm.sh`): file `.desktop`, icona, `update-desktop-database`. Ciò che un agente non può fare (sentire la voce; giudicare «si vede bene») è scritto come tale | è ciò che Q49 ha già fatto; un pacchetto vero è fase 4 |
| P13 | Per le prove c'è `GALLERIA_UPDATE_URL`: sostituisce l'indirizzo della richiesta. Vale solo se è `http://127.0.0.1:<porta>/…`; ogni altro valore è ignorato con una riga su `stderr` | il fumo prova l'app costruita come per il rilascio; la variabile non può mandare la richiesta altrove |
| P14 | Per G13 c'è `GALLERIA_STARTUP_TRACE=1`: quattro righe su `stderr` con i millisecondi dall'avvio del processo (`main`, fine di `setup`, finestra mostrata, prima chiamata della pagina) | una misura che non dipende da chi guarda lo schermo |

## Mappa dei file

```
crates/galleria-gui/Cargo.toml                 tauri: feature tray-icon (T1); soup3, semver diretti (T4)
crates/galleria-gui/icons/tray-{off,on,busy}.png                                                       T1
crates/galleria-gui/src/
  tray.rs                     modello del menu: voci, icona, azioni                — senza `tauri`     T1
  tray_tauri.rs               il menu vero: crea, riscrive, inoltra i click                            T1, T2
  tray_host.rs                c'è chi mostra la tray? (bus di sessione, `gtk::gio`; cfg linux)         T3
  closing.rs                  decide: nascondi, esci, fermati                      — senza `tauri`     T3
  update.rs                   quando chiedere, cosa vale come versione nuova       — senza `tauri`     T4
  update_soup.rs              la richiesta con libsoup (cfg linux)                                     T4
  commands/{tray,window,update}.rs   involucri sottili                                                 T1, T3, T4
  session.rs  supervisor.rs  events.rs  window.rs  ui_state.rs  lib.rs  error.rs                        T1–T5
  build.rs  capabilities/main.json  tests/config.rs      ogni task che aggiunge un comando
crates/galleria-gui/tests/{tray,closing,update}.rs                                                      T1–T4
crates/galleria-e2e/tests/t18_gui_connect.rs   gui_closed_window_kept_the_tunnel                        T6
lab/tests/tray-probe.py                        watcher finto, `layout`, `click` (da spikes/b6-piano/)   T1
lab/tests/update-server.py                     server finto che conta richieste e intestazioni          T4
lab/tests/gates-b6.tsv  scripts/check-gates.py (tutti i manifesti)  scripts/tests/check-gates.sh        T6
lab/tests/{gui-lib.sh,gui-smoke.sh,gui-lab-connect.sh,gui-lab-watch.sh}                                 T1, T6
.github/workflows/ci.yml                       gui-smoke: JSON del fumo a check-gates                   T6
ui/src/app/{tray.ts,closing.ts,AppMenu.tsx,announce.ts}                                                 T2, T3, T5
ui/src/shell/{Shell,RubricaNotices,ProfileDetail}.tsx  ui/src/update/UpdateNotice.tsx                   T3, T5
ui/src/i18n/{en,it}.json                       testi e codici nuovi                                     ogni task
ui/e2e/{tray.e2e.ts,tray.a11y.ts,lab-connect.e2e.ts,smoke.e2e.ts,support.ts}                            T1–T7
lab/vm/{vmctl.sh,ubuntu24-user-data.tmpl,gui-vm.sh,startup-time.sh,README.md}                           T8–T10
README.md                                      tray, uscita, avviso di versione, limiti                 T5, T11
docs/08-esito-fase2.md                         sezione «B6» a ogni task; «Chiusura della fase 2»        T11
docs/superpowers/specs/2026-11-25-fase2-gui-design.md   §4.2, §13, §17, §18                             T11
ROADMAP.md  .claude/handoffs/                                                                           T12
```

---

## Gruppo A · Tray e chiusura

### Task 1: il menu della tray dal modello, e la sonda che lo legge dal bus

Scritto su **Q52 (a)**: le voci portano un'azione, che il T2 inoltra alla pagina.

**Stima:** 6 h. **Scioglie:** S46, S47, S48. **File — crea:** `crates/galleria-gui/src/{tray.rs,tray_tauri.rs}`,
`src/commands/tray.rs`, `tests/tray.rs`, `icons/tray-{off,on,busy}.png`, `lab/tests/tray-probe.py`,
`ui/src/app/tray.ts` e la prova, `ui/e2e/tray.e2e.ts`. **Modifica:** `Cargo.toml`, `src/{lib.rs,session.rs,supervisor.rs}`,
`build.rs`, `capabilities/main.json`, `tests/config.rs`, cataloghi, `lab/tests/gui-lib.sh`, `ui/e2e/support.ts`,
`docs/08-esito-fase2.md`.

**Produce:**

```rust
// tray.rs — nessun tipo di tauri
#[derive(Clone, Debug, PartialEq, Eq)] pub enum TrayAction { Connect(Uuid), Disconnect, Open, Quit }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum TrayEntry {
    Label { text: String },                                   // la riga di stato: spenta
    Item { text: String, enabled: bool, action: TrayAction },
    Separator,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum TrayIcon { Off, On, Busy }
#[derive(Deserialize)] pub struct TrayLabels { /* open, quit, quit_connected, connect, disconnect, i quattro stati */ }
pub struct TrayInput<'a> { pub link: LinkState, pub status: &'a Status, pub favourites: &'a [(Uuid, String)],
    pub labels: &'a TrayLabels }
pub fn model(input: &TrayInput<'_>) -> (Vec<TrayEntry>, TrayIcon);      // P2, P4, P5: pura, provata con una tabella
```

- `tray_labels(labels)`: stringhe lunghe al più 200 caratteri, ripulite con `announce::clean`; «{name}» è il solo
  segnaposto. La prima consegna crea la tray; le successive riscrivono le voci (P3).
- `tray_tauri`: crea l'icona con il menu **dentro una barriera** (`std::panic::catch_unwind`): se la libreria manca la
  tray non c'è, una riga su `stderr` lo dice una volta, e `TrayState::available()` è falso (punto 2, S48). Le voci si
  riscrivono togliendo e rimettendo il contenuto dello stesso menu (S47).
- Il menu si ricalcola quando cambia uno dei suoi ingressi: stato (`Sessions::on_state`), collegamento, rubrica
  (preferiti, nomi, profili aggiunti o tolti: dal punto unico in cui `Rubrica` scrive o rilegge), testi. Se il modello
  è uguale a quello di prima non si tocca niente.
- **`lab/tests/tray-probe.py`** (da `spikes/b6-piano/`): `watch` resta acceso, fa da `org.kde.StatusNotifierWatcher` e
  scrive in un file chi si è registrato; `layout` stampa le voci come JSON (`text`, `enabled`); `click "<testo>"` manda
  `Event(id, "clicked")`; `icon` dice il nome dell'icona. `gui-lib.sh` lo avvia prima dell'app
  (`GALLERIA_SMOKE_TRAY=absent` non lo avvia).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/tray.rs` | `the_menu_for_each_state` | parametrica sui sette stati e sui quattro collegamenti non `connected`: riga di stato, icona, voci accese e spente (P2, P4, P5) |
  | | `favourites_keep_the_order_of_the_list_and_stop_at_eight` | — |
  | | `the_active_favourite_offers_disconnect_and_the_others_connect` | — |
  | | `an_active_profile_that_is_not_a_favourite_gets_its_own_row` · `a_foreign_session_gets_a_disconnect_row_with_its_name` | il nome viene da `Status.profile.name` |
  | | `open_and_quit_are_always_there_and_enabled` | — |
  | | `quit_says_the_vpn_stays_while_a_session_is_up` | Q54 |
  | | `a_name_with_markup_or_newlines_is_shown_flat` | un nome di profilo con `_`, `&`, a capo: una riga, nessun acceleratore |
  | | `labels_longer_than_two_hundred_characters_are_refused` | `gui_invalid_field` |
  | | `no_labels_no_tray` | P3 |
  | `app/tray.test.ts` | `labels_are_sent_at_start_and_on_language_change` | — |
  | `tests/config.rs` | elenco dei permessi | più `allow-tray-labels` |
  | `e2e/tray.e2e.ts` | `G10a_the_menu_says_the_state_and_lists_the_favourites` | con la sonda: «Disconnected», i due preferiti del seme nell'ordine della lista, «Open Galleria», «Quit»; cambiata la lingua, le stesse voci in italiano; tolta una stella nella finestra, la voce sparisce |
  | | `the_app_starts_without_the_indicator_library` | app avviata con le due librerie nascoste (S48): la finestra c'è, `stderr` ha la riga, nessun `panicked` |

- [ ] **2.** Falliscono. **3.** Implementa. Prima la feature e `scripts/gui.sh check`, poi **subito** la sonda contro
  l'app vera (**S46**: è il punto da cui dipende la forma di tutto il gruppo). **4.** I comandi del task.
- [ ] **5. Commit:** `show the state and the favourites in a tray menu built from a pure model`

**Ripieghi.** S46 falsa (il menu di `muda` non si legge o non si clicca dal bus): G10 in CI si prova al livello 2 sul
modello e sull'inoltro delle azioni (`tests/tray.rs`), `G10a`–`G10e` chiamano l'azione con un comando di sola prova
dietro la feature `e2e`, assente dal build di rilascio, e il menu vero resta alla lista manuale; lo si scrive. S47
falsa (riscrivere il contenuto lascia il menu vecchio sul bus): voci fisse, undici, a cui si cambiano testo e stato;
i preferiti oltre l'ottavo non compaiono comunque. S48 falsa (il panico non si ferma): la libreria diventa un
requisito scritto nel README e nello script di installazione, e l'esito lo dice.

**Fatto quando:** `cargo tree -p galleria-gui -e normal` ha gli stessi pacchetti di `Cargo.lock` di prima (`git diff
main -- Cargo.lock` tocca solo le righe di `galleria-gui`); S46, S47 e S48 hanno un esito scritto.

---

### Task 2: connetti e disconnetti dal solo menu

**Stima:** 5 h. **Scioglie:** S49. **File — crea:** `crates/galleria-gui/src/commands/window.rs`. **Modifica:**
`crates/galleria-gui/src/{tray_tauri.rs,session.rs,events.rs,window.rs}`, `build.rs`, `capabilities/main.json`,
`tests/{tray.rs,config.rs}`, `ui/src/app/tray.ts` (la parte delle azioni) e la prova, `ui/src/connection/store.ts`,
`ui/src/shell/Shell.tsx`, cataloghi, `ui/e2e/tray.e2e.ts`.

**Produce:**

- Un click su una voce: `Open` → `window::raise`; `Quit` → la decisione del T3 (fino ad allora `AppHandle::exit`);
  `Connect(id)` e `Disconnect` → evento `galleria:tray-action` (P6).
- Pagina (`app/tray.ts`, l'unico file che ascolta l'evento): `connect` chiama **la stessa funzione del pulsante**
  dello store di B5; `disconnect` la stessa di «Disconnetti». Se la funzione deve chiedere (sessione estranea) o
  finisce con un errore di comando, prima chiama `window_show` (P7): la conferma e l'avviso compaiono in una finestra
  che si vede.
- Rust: quando arriva `needs_input` e la finestra è nascosta, `window::raise` (punto 4), dallo stesso punto che
  aggiorna il menu.
- `window_show() -> ()`: `window::raise`. Nessun parametro.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/tray.rs` | `a_click_on_connect_reaches_the_page_as_an_action_with_the_id` · `open_and_quit_never_reach_the_page` | con `tauri::test` e un ascoltatore dell'evento |
  | | `needs_input_shows_the_hidden_window` | scenario `credentials`, finestra nascosta: dopo lo stato la finestra è visibile |
  | | `no_tray_action_carries_more_than_an_action_and_an_id` | il payload dell'evento, per ogni voce |
  | `app/tray.test.ts` | `a_tray_connect_runs_the_same_path_as_the_button` | una chiamata `connect_profile(id, false)`; con `already_active` e `foreign: false`, la seconda con `replace: true` e un avviso |
  | | `a_foreign_session_is_never_closed_from_the_menu_without_the_confirmation` | `window_show`, poi la conferma; nessuna `disconnect` prima |
  | | `a_command_error_from_the_menu_shows_the_window` | — |
  | `tests/config.rs` | elenco dei permessi | più `allow-window-show` |
  | `e2e/tray.e2e.ts` | `G10b_a_favourite_connects_and_disconnects_from_the_menu_alone` | scenario `plain`, finestra **nascosta**: `click "Connect «Ufficio»"`, entro 10 s la riga di stato è «Connected to «Ufficio»», l'icona è quella accesa e `galleria status` dice `connected`; `click "Disconnect «Ufficio»"`: `disconnected`. Nessuna chiamata WebDriver tra i due click |
  | | `switching_favourite_from_the_menu_replaces_without_asking` | un solo profilo attivo alla fine |
  | | `credentials_asked_after_a_menu_connect_bring_the_window_back` | scenario `credentials`: la finestra torna visibile con la finestra delle credenziali a fuoco |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `connect and disconnect from the tray menu through the page that already knows how`

**Ripiego per S49** (a finestra nascosta la pagina non riceve gli eventi, o li riceve con secondi di ritardo): i tre
casi che non chiedono niente (connetti senza sessione, disconnetti una sessione propria, sostituisci una propria) li
esegue Rust con `Sessions`, che ha già le tre chiamate; tutto il resto riapre la finestra e passa dalla pagina. È
Q52 (b) per quei tre casi: lo si riporta prima di applicarlo, perché tocca il punto 7 di B5 («Rust non sostituisce mai
da solo»).

**Fatto quando:** `grep -rn "tray-action" ui/src` trova un solo file che ascolta; nessuna funzione dello store di B5 è
stata copiata.

---

### Task 3: chiusura nella tray, uscita esplicita, modifiche non salvate (Q35)

Scritto su **Q53 (a)** e **Q54 (a)**.

**Stima:** 5 h. **Scioglie:** S51, S53, S60. **File — crea:** `crates/galleria-gui/src/{closing.rs,tray_host.rs}`,
`tests/closing.rs`, `ui/src/app/closing.ts` e la prova. **Modifica:** `src/{lib.rs,window.rs,tray_tauri.rs}`,
`src/commands/window.rs`, `build.rs`, `capabilities/main.json`, `tests/config.rs`, `ui/src/shell/{Shell,ProfileDetail}.tsx`,
`ui/src/app/AppMenu.tsx`, cataloghi, `ui/e2e/{tray.e2e.ts,smoke.e2e.ts,support.ts}`, `lab/tests/gui-lib.sh`.

**Produce:**

```rust
// closing.rs — nessun tipo di tauri
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Intent { Hide, Quit }       // la «X» chiede Hide, «Esci» chiede Quit
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Decision { Hide, Quit, Blocked(Intent) }
/// `unsaved`: il bit della pagina. `tray_shown`: la tray esiste **e** qualcuno la mostra, adesso.
pub fn decide(intent: Intent, unsaved: bool, tray_shown: bool) -> Decision;
// Hide + !tray_shown → Quit (Q53);  unsaved → Blocked(intent), con l'intento già corretto

// tray_host.rs — cfg linux; fuori da Linux è sempre falso
/// Sul bus di sessione `org.kde.StatusNotifierWatcher` ha un proprietario e `IsStatusNotifierHostRegistered` è vero.
/// Senza bus, senza proprietario o entro 300 ms senza risposta: falso.
pub fn tray_is_shown() -> bool;
```

- Gestore `CloseRequested` della finestra `main`, **in Rust** (Q35), dopo il salvataggio della dimensione che c'è:
  `api.prevent_close()` sempre, poi `decide`: `Hide` → `window.hide()`; `Quit` → `AppHandle::exit(0)`; `Blocked` →
  `window::raise` ed evento `galleria:close-blocked` con l'intento (P8).
- `unsaved_set(dirty)`: la pagina lo chiama quando `isDirty` cambia valore (non a ogni tasto) e quando il profilo in
  modifica cambia. `window_hide()` e `app_quit()` rispondono `{ done }` (P9).
- Pagina (`app/closing.ts`): su `galleria:close-blocked` apre la guardia di B3 con il terzo pulsante che c'è; «Salva»
  riuscito o «Scarta» → `unsaved_set(false)` e poi il comando dell'intento; «Annulla» → niente. Un salvataggio fallito
  lascia la finestra dov'è, con l'errore di B3.
- «Esci» nel menu dell'app e `Ctrl+Q` → `app_quit` (P10). «Esci» nel menu della tray → la stessa decisione.
- La voce «Esci» della tray dice «Esci (la VPN resta attiva)» quando lo stato non è `disconnected` (Q54); lo stesso
  testo nella voce del menu dell'app.
- Il secondo avvio riapre la finestra nascosta: è `window::raise`, già chiamata da `instance` (B4).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/closing.rs` | `decide_table` | le otto combinazioni di intento, bit e tray |
  | | `without_a_tray_host_a_close_is_a_quit` | punto 1 |
  | | `unsaved_changes_block_both_the_close_and_the_quit` | `Blocked(Hide)`, `Blocked(Quit)`; l'evento porta l'intento |
  | | `a_close_without_unsaved_changes_does_not_wait_for_the_page` | con una pagina che non risponde a niente, la finestra è nascosta entro 200 ms (punto 5) |
  | | `window_hide_and_app_quit_go_through_the_same_decision` | con il bit acceso tutti e due rispondono `{ done: false }` |
  | | `the_size_is_saved_before_the_window_hides` | `state.toml` ha la dimensione (la prova di B3 resta verde) |
  | | `tray_is_shown_is_false_without_a_bus_and_without_an_owner` | S51, nell'immagine: bus senza watcher, e senza bus |
  | `app/closing.test.ts` | `a_blocked_close_opens_the_guard_and_discard_hides` · `save_then_quit_quits_only_if_the_save_worked` · `cancel_keeps_the_window_and_the_draft` | — |
  | | `unsaved_set_is_called_when_dirtiness_changes_not_on_every_key` | dieci tasti: una chiamata |
  | `AppMenu.test.tsx` | `quit_is_in_the_menu_and_on_ctrl_q` · `quit_says_the_vpn_stays_while_connected` | — |
  | `tests/config.rs` | elenco dei permessi | più `allow-unsaved-set`, `allow-window-hide`, `allow-app-quit` |
  | `e2e/tray.e2e.ts` | `G10c_closing_the_window_hides_it_and_the_session_stays` | scenario `plain`, connesso; alla finestra arriva la richiesta di chiusura che manderebbe un gestore di finestre (`WM_DELETE_WINDOW`: sotto Xvfb un gestore non c'è, con cosa mandarla lo fissa il task, **S60**); il processo è vivo, la finestra non è mappata, `galleria status` dice `connected`, il menu dice «Connected to …» |
  | | `G10d_open_shows_the_window_and_quit_ends_the_app` | `click "Open Galleria"`: la finestra è mappata e la pagina ha ancora il profilo scelto; `click "Quit (the VPN stays on)"`: il processo finisce con 0 entro 5 s, e `galleria status` dice ancora `connected` |
  | | `G10e_without_a_tray_host_closing_quits_and_the_session_stays` | `GALLERIA_SMOKE_TRAY=absent`: il processo finisce, `connected` resta |
  | | `G10f_unsaved_changes_stop_the_close_and_the_quit` | un nome cambiato e non salvato; chiusura: la finestra resta, la guardia ha il fuoco su «Cancel»; «Discard»: nascosta, il file sul disco ha il nome di prima. Di nuovo con «Quit» dalla tray e «Save»: il processo finisce e il file ha il nome nuovo |
  | | `a_second_launch_shows_the_hidden_window` | dopo la chiusura, secondo avvio con un `.ovpn`: la finestra è mappata con l'anteprima |

- [ ] **2.** Falliscono. **3.** Implementa. Le prove di fumo di B1–B5 che finivano chiudendo la finestra ora finiscono
  con `app_quit` o con `GALLERIA_SMOKE_TRAY=absent` (punto 12): elenco nell'esito. **4.** I comandi del task.
- [ ] **5. Commit:** `hide the window in the tray on close, quit on request and stop both for unsaved changes`

**Fine del gruppo A:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Ripieghi.** S51 falsa (il bus non dice in modo affidabile se la tray si vede): la chiusura nella tray diventa una
scelta dell'utente nel menu dell'app («Chiudendo la finestra resta nella tray»), spenta di suo; è un cambio rispetto
alla spec §13, quindi ci si ferma e si riporta. S53 falsa (dopo `hide` la pagina si ferma o si ricarica): la finestra
si riduce a icona invece di nascondersi, e lo si scrive.

**Fatto quando:** nessun percorso chiude l'app senza passare da `closing::decide` (`grep -rn "exit(" crates/galleria-gui/src`
trova solo il punto che la esegue); spec §3.4 («fino a B6 chiude l'app») è superata e lo dice l'esito.

---

## Gruppo B · Avviso di versione

### Task 4: la richiesta delle versioni, da Rust, contata da un server finto (G11)

Scritto su **Q50 (a)** (la pagina delle release, leggendo il rinvio) e **Q51 (a)** (`soup3`). Se Q50 passa come (b)
cambia `update::parse` e la richiesta ha un `User-Agent` fisso; se Q51 passa come (b) cambia solo `update_soup.rs`.

**Stima:** 5 h. **Scioglie:** S52. **File — crea:** `crates/galleria-gui/src/{update.rs,update_soup.rs}`,
`src/commands/update.rs`, `tests/update.rs`, `lab/tests/update-server.py`. **Modifica:** `Cargo.toml`,
`src/{lib.rs,ui_state.rs,error.rs}`, `build.rs`, `capabilities/main.json`, `tests/{config.rs,ui_state.rs}`.

**Produce:**

```rust
// update.rs — nessun tipo di tauri, nessuna rete
pub const RELEASES_LATEST: &str = "https://github.com/zPeppOz/galleria/releases/latest";
pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);
/// Si chiede solo se l'avviso è acceso e dall'ultima richiesta (riuscita o no) sono passate 24 ore.
pub fn due(state: &UpdateState, now: OffsetDateTime) -> bool;
/// Dallo stato della risposta e dal valore di `Location`: la versione, solo se è `…/zPeppOz/galleria/releases/tag/v<semver>`
/// senza pre-release. Tutto il resto è `None`. Non restituisce mai un pezzo della stringa ricevuta.
pub fn parse(status: u16, location: Option<&str>) -> Option<semver::Version>;
/// Da mostrare: `latest` è più nuova di `current` e non è quella che l'utente ha chiesto di non ricordare.
pub fn notice(current: &semver::Version, state: &UpdateState) -> Option<Notice>;     // { version, url }: l'url lo costruisce questa
pub trait Fetch: Send + Sync { fn latest(&self, url: &str) -> Result<(u16, Option<String>), FetchError>; }
pub struct Updater { /* Arc<UiStateFile>, Arc<dyn Fetch>, url, orologio */ }
impl Updater { pub async fn tick(&self) -> Option<Notice>; }      // al più una richiesta; scrive `last_check` **prima** di chiedere
```

- `update_soup.rs`: una `soup::Session` nuova per richiesta, senza `User-Agent`, senza cookie, tempo massimo 10 s,
  `NO_REDIRECT`; un `GET`; si leggono stato e `Location`, il corpo si butta. Usa il proxy del sistema, come fa
  libsoup. Gira sul contesto principale di GLib (S52).
- `state.toml`, tabella `[update]` (P11); `UiPatch` guadagna `update_check: Option<bool>` e `update_dismissed:
  Option<String>` (una versione `semver` valida, altrimenti `gui_invalid_field`).
- `update_view() -> { current, latest, url, enabled }`: legge lo stato, **non** fa richieste. `latest` e `url` ci sono
  solo se `notice` dà qualcosa. Evento `galleria:update` con lo stesso carico quando una richiesta trova una versione.
- Avvio: 10 s dopo `setup`, poi un controllo dell'orologio ogni ora (P11). `GALLERIA_UPDATE_URL` secondo P13.
- Errori di rete: una riga su `stderr` con il tipo dell'errore, senza indirizzi e senza testo della risposta; niente
  alla pagina.

- [ ] **1. Prove che falliscono.** Il server finto delle prove Rust è un `TcpListener` di `std` in un thread: conta le
  connessioni, conserva la richiesta com'è arrivata, risponde ciò che la prova gli dice.

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/update.rs` | `update_asks_once_and_sends_no_identifier` | un `tick` con l'avviso acceso: **una** connessione; la riga di richiesta è `GET /… HTTP/1.1`; le intestazioni ricevute sono un sottoinsieme di `Host`, `Accept-Encoding`, `Connection` (l'elenco esatto lo fissa S52); nessuna tra `User-Agent`, `Cookie`, `Referer`, `Authorization`, `If-None-Match`, `If-Modified-Since`, `X-*`; nessun parametro nell'indirizzo; nessun corpo. Dieci `tick` di fila: ancora una (G11) |
  | | `update_disabled_asks_nothing` | `check = false`: zero connessioni dopo dieci `tick` e dopo `update_view` (G11) |
  | | `update_asks_at_most_once_a_day` | orologio finto: a 23 h 59 min zero richieste nuove, a 24 h una; una richiesta **fallita** conta lo stesso |
  | | `update_never_follows_a_redirect` | il server risponde 302 verso un secondo server finto: questo conta zero |
  | | `only_a_newer_stable_version_of_this_repository_counts` | parametrica su `parse` e `notice`: `…/releases/tag/v0.2.0` sì; `…/releases` no (è la risposta di oggi); un altro proprietario, un altro host, `v0.2.0-rc.1`, `v0.2.0/../../x`, `v0.2.0?x=1`, 200 senza `Location`, 404, una versione uguale o più vecchia, mille caratteri: no, e mai un errore |
  | | `a_dismissed_version_is_not_shown_again_and_a_newer_one_is` | — |
  | | `the_view_never_carries_a_string_of_the_answer` | `Location` con un marcatore nel percorso dopo il tag: il marcatore non è in `update_view`, nell'evento, su `stderr` |
  | | `a_failed_request_says_nothing_to_the_page` | server chiuso, server che non risponde (10 s con orologio finto), TLS non provato qui |
  | | `the_override_is_honoured_only_for_loopback` | `GALLERIA_UPDATE_URL=https://example.org/…` → ignorata, riga su `stderr`, indirizzo predefinito (P13) |
  | | `update_view_makes_no_request` | — |
  | `tests/ui_state.rs` | `a_state_file_of_b5_reads_the_same_and_gets_the_defaults` · `the_update_table_round_trips` | punto 9 |
  | `tests/config.rs` | elenco dei permessi | più `allow-update-view`; `csp_is_set_and_has_no_remote_host` (di B1) resta verde |

- [ ] **2.** Falliscono. **3.** Implementa. Prima `update.rs` con il `Fetch` finto (tutta la tabella tranne le
  intestazioni), poi `update_soup.rs` contro il `TcpListener` (**S52**: quali intestazioni manda libsoup senza che
  gliele si chieda si legge lì, e l'elenco va nell'esito e nel README).
- [ ] **4.** I comandi del task. **Una** richiesta vera, a mano, dall'immagine verso GitHub (`scripts/gui.sh run …` con
  un piccolo esempio che chiama `update_soup` una volta): oggi deve dare «nessuna versione nota» (302 verso
  `…/releases`). È la sola prova che il TLS di libsoup funziona nell'immagine; l'uscita va nell'esito.
- [ ] **5. Commit:** `ask github once a day for the latest release without saying who asks`

**Ripiego per S52** (libsoup aggiunge un'intestazione che identifica, o da Rust non si riesce a usarla sul contesto
principale, o nell'immagine manca il TLS di `glib-networking`): Q51 (b), `reqwest` con native-tls su tokio (+8
pacchetti, `cargo deny` verde, misurato); ci si ferma e si riporta, perché cade il vincolo «nessun crate nuovo».

**Fatto quando:** `grep -rn "https\?://" crates/galleria-gui/src` trova un solo indirizzo remoto, in `update.rs`;
`cargo tree -p galleria-gui -e normal -i reqwest` non trova niente.

---

### Task 5: interruttore, avviso, README

Scritto su **Q55 (a)**: l'indirizzo è testo, con «Copia link».

**Stima:** 4 h. **File — crea:** `ui/src/update/UpdateNotice.tsx` con `.css` e prova, `ui/src/update/store.ts`.
**Modifica:** `ui/src/app/AppMenu.tsx` e la prova, `ui/src/shell/{Shell,RubricaNotices}.tsx`, `ui/src/daemon/commands.ts`,
`ui/src/shell/keyboard.test.tsx`, cataloghi, `ui/e2e/{smoke.e2e.ts,tray.e2e.ts}`, `lab/tests/{gui-lib.sh,update-server.py}`,
`README.md`.

**Produce:**

- Menu dell'app: una voce a spunta «Controlla se c'è una versione nuova» (`ui_state_set { update_check }`), sopra
  «Informazioni su Galleria». Spegnerla toglie anche l'avviso mostrato.
- **Avviso in linea** in cima alla lista (P11), sotto l'avviso del portachiavi se c'è: testo, indirizzo come testo
  selezionabile, «Copia link» (`navigator.clipboard`, S7 è vera; esito con un avviso a scomparsa «Link copiato.»),
  «Non ricordarmelo» (`ui_state_set { update_dismissed }`). Non è una regione viva e non si annuncia (Q27, regola 3):
  è testo che si trova con Tab.
- «Informazioni su Galleria» guadagna una riga: «Ultima versione nota» o «Controllo disattivato».
- `lab/tests/update-server.py`: il server finto per il fumo, in ascolto su `127.0.0.1`; scrive ogni richiesta in un
  file (riga di richiesta e intestazioni) e risponde con il rinvio che gli si dà sulla riga di comando.
- `README.md`, sezione nuova «Cosa esce dalla tua macchina»: l'indirizzo, quando parte, le intestazioni che porta
  (l'elenco di S52), che non c'è nessun identificativo, come si spegne (menu dell'app, oppure `check = false` in
  `state.toml`), e che è l'unico contatto di rete della finestra (D16, A11).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `UpdateNotice.test.tsx` | `the_notice_names_both_versions_and_shows_the_address_as_text` | nessun `<a href>`, nessuna chiamata che apre un indirizzo |
  | | `copy_copies_the_address_and_says_so_once` · `dismiss_hides_it_and_writes_the_version` | — |
  | | `no_notice_when_the_check_is_off_or_nothing_is_newer` | — |
  | | `the_notice_is_not_a_live_region` · `has_no_accessibility_violations` | — |
  | `AppMenu.test.tsx` | `the_check_can_be_turned_off_and_the_choice_is_saved` | — |
  | `keyboard.test.tsx` | `the_update_notice_is_reached_and_dismissed_without_a_pointer` | — |
  | `e2e/tray.e2e.ts` | `G11a_the_built_app_asks_once_and_never_when_disabled` | app costruita come per il rilascio, `GALLERIA_UPDATE_URL` verso il server finto che rinvia a `…/releases/tag/v9.9.9`: entro 20 s il file del server ha **una** richiesta, senza `User-Agent`; la finestra mostra l'avviso con «9.9.9»; app chiusa e riaperta: ancora una richiesta (le 24 ore). Poi stato azzerato, `check = false`: 30 s, **zero** richieste |
  | | `turning_the_check_off_removes_the_notice` | — |
  | `e2e/smoke.e2e.ts` | `no_csp_violation_with_the_update_notice` | — |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `show a notice for a newer version and let the user turn the check off`

**Fine del gruppo B:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** il README dice dell'unica richiesta di rete ciò che la prova `update_asks_once_and_sends_no_identifier`
asserisce, con le stesse intestazioni.

---

## Gruppo C · Gate in CI e fumo completo

### Task 6: G10 e G11 nei job, con il manifesto; il tunnel vero a finestra chiusa

**Stima:** 5 h. **Scioglie:** S59. **File — crea:** `lab/tests/gates-b6.tsv`. **Modifica:** `scripts/check-gates.py`,
`scripts/tests/check-gates.sh`, `scripts/gui.sh` (`smoke-here` scrive `target/gates/smoke-vitest.json`),
`ui/e2e/{vitest.config.ts,lab-connect.e2e.ts,support.ts}`, `lab/tests/{gui-lab-connect.sh,gui-lab-watch.sh}`,
`crates/galleria-e2e/tests/t18_gui_connect.rs`, `crates/galleria-gui/tests/config.rs`, `.github/workflows/ci.yml`,
`lab/README.md`, `docs/08-esito-fase2.md`.

**Produce:**

- `check-gates.py`: senza `--manifest` legge tutti i `lab/tests/gates-*.tsv`; un gate presente in due file è un errore.
  `gates-b6.tsv` ha le righe della tabella «Dove gira ogni gate in CI»: **tredici** (sei `G10` e una `G11` per
  `gui-smoke`, le quattro `update_*` per `gui`, le due di `lab-connect`).
- `ci.yml`: l'ultimo passo di `gui-smoke` passa anche `--vitest-json target/gates/smoke-vitest.json`, senza `--kind
  cargo`. `gui` e `lab-connect` non cambiano comando: leggono righe in più.
- `every_gate_has_a_job` (già `every_b5_gate_has_a_job`): G4, G5, G6, G7, G9, **G10, G11** hanno almeno una riga; ogni
  job nominato esiste e chiama `check-gates.py --job <sé>`.
- **`G10g_the_tunnel_survives_the_closed_window`** (`lab-connect.e2e.ts`, demone vero): `gui-lab-connect.sh` avvia la
  sonda `watch` prima dell'app; la finestra connette `wg-split`, scrive nel registro dell'osservatore un segno
  (`mark closed`, attraverso il file che `gui-lab-watch.sh` già legge per il `ping`), si chiude; per 10 s il `ping`
  nel tunnel continua; `click "Disconnect «wg-split»"` dalla sonda; segno `mark disconnected`.
- **`gui_closed_window_kept_the_tunnel`** (`t18`, nel client): tra i due segni **ogni** campione ha `wg=1` e sono
  almeno 30; dopo il secondo, entro 5 s, `wg=0`. Fallisce se un segno manca.

- [ ] **1. Prove che falliscono.** Le due sopra, con le righe nel manifesto; in `scripts/tests/check-gates.sh`:
  `two_manifests_are_read_together`, `a_gate_in_two_manifests_is_an_error`, `a_skipped_smoke_test_fails_the_gate`.
- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task; `scripts/gui.sh keyring`;
  `LAB_BUILD=host scripts/gui.sh lab-connect 26` (intero) e, su 27 e 79, `G10g` con il filtro.
- [ ] **5. Commit:** `prove the tray and the version check in the jobs and keep a real tunnel up with the window closed`

**Ripiego per S59** (il segno nel registro dell'osservatore non si riesce a scrivere dalla finestra, che gira in
un'altra immagine): i due segni li scrive `gui-lab-connect.sh` con `docker exec`, intorno alla prova; se nemmeno
così, `G10g` asserisce solo dal lato della finestra (`galleria status` attraverso il ponte) e `t18` non guadagna la
prova: lo si scrive e G10 sul demone vero resta con una prova sola.

**Fatto quando:** togliendo a mano la riga di `G10b` dal JSON del fumo, `check-gates.py --job gui-smoke` esce con 1
(provato una volta); il job `lab-connect` non ha guadagnato `continue-on-error`.

---

### Task 7: il giro completo, la tastiera, Orca a finestra nascosta

**Stima:** 5 h. **Scioglie:** S50. **File — crea:** `ui/e2e/tray.a11y.ts`. **Modifica:** `ui/src/shell/keyboard.test.tsx`,
`ui/e2e/{smoke.e2e.ts,tray.e2e.ts}`, `crates/galleria-gui/tests/{export.rs,connection.rs,config.rs}`, `docs/08-esito-fase2.md`.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `e2e/smoke.e2e.ts` | `the_whole_day_in_one_window` | **il fumo completo**, una sessione sola: primo avvio senza demone (testo di D27), demone su, import di un `.ovpn` dal testo, connessione con credenziali e «Ricorda», numeri che si muovono, scheda Log con «Copia», chiusura nella tray, sostituzione dal menu, riapertura con un secondo avvio, modifica non salvata e guardia, «Esci»: il processo finisce con 0, `galleria status` dice `connected`, nessuna violazione di CSP, niente `panicked` su `stderr` |
  | `tests/export.rs` · `tests/connection.rs` | `no_command_takes_a_path` · `no_command_returns_a_secret` (di B3–B5) | estese ai sei comandi di B6 |
  | `tests/config.rs` | `the_commands_are_the_thirty_six_of_the_capability` | `COMMANDS`, `capabilities/main.json` e `generate_handler!` hanno gli stessi nomi (30 di B5 più 6) |
  | `keyboard.test.tsx` | `quit_and_the_guard_need_no_pointer` | `Ctrl+Q` con una modifica non salvata, guardia, «Scarta» |
  | | `focus_is_never_lost_to_the_body` (di B2) | esteso: dopo la guardia aperta da una chiusura, dopo l'avviso di versione tolto |
  | `e2e/tray.a11y.ts` | `K6a_a_state_change_is_said_with_the_window_hidden` | finestra nascosta, `click` sulla sonda: nel file di debug di Orca la riga `object:announcement` con «Connected to …», urgenza 1, **una volta** (S50) |
  | | `K6b_the_guard_opened_by_a_close_is_read` | ruolo, testo, fuoco su «Cancel»; nessun annuncio da Rust |
  | | `K6c_the_update_notice_is_text_not_an_announcement` | nessuna riga `announcement` quando compare |

- [ ] **2.** Falliscono. **3.** Correggi ciò che le prove trovano, nei componenti a cui appartiene (`--fixup`).
- [ ] **4.** `scripts/gui.sh check`; gruppo dell'host; `scripts/gui.sh keyring`; poi, **una volta sola in tutto il
  blocco**: `scripts/gui.sh smoke` **tre volte di fila**, `scripts/gui.sh a11y` **due volte** (o quante ne chiede la
  misura lasciata da `t_016cb5ea`), `scripts/gui.sh lab-connect 26` **due volte**. I numeri, giro per giro, vanno
  nell'esito, anche se uno è rosso.
- [ ] **5. Commit:** `cover a whole day of use in one smoke run and the tray with the screen reader`

**Ripiego per S50** (Orca non dice un annuncio emesso da una finestra nascosta): `K6a` diventa la prova del limite
(l'asserzione capovolta, come `K5c`), il limite va nell'esito e nel README («a finestra chiusa lo stato si legge dal
menu della tray»), e nessun codice cambia: le notifiche del desktop restano escluse da Q27.

**Fine del gruppo C.** La card dice in chiaro: quali prove sono verdi in locale; che G10 e G11 nei job e `G10g` su 27
e 79 con OpenVPN intorno aspettano il run di CI dopo il push di Aitano.

**Fatto quando:** ogni riga di `gates-b6.tsv` ha una prova verde in locale; il fumo completo è passato tre volte di
fila, oppure l'esito dice quante su quante.

---

## Gruppo D · Desktop veri

Niente codice nuovo dell'app, salvo le correzioni. Tutto ciò che si vede finisce in `/mnt/b/vm/evidence/b6/<vm>/`
(fotografie, uscite, file di debug di Orca) e, riassunto, nell'esito.

### Task 8: la VM Ubuntu 24.04 desktop, e l'app installata sulle tre

**Può partire in parallelo al gruppo A.** Il binario dell'app, per la prima accensione, è quello di `main`; il T9 lo
sostituisce con quello dell'ultimo commit del gruppo C.

**Stima:** 4 h (di cui mezz'ora di attesa). **Scioglie:** S55. **File — crea:** `lab/vm/ubuntu24-user-data.tmpl`,
`lab/vm/gui-vm.sh`. **Modifica:** `lab/vm/vmctl.sh` (terzo nome `ubuntu24`, porta 2203, VNC `:3`), `lab/vm/README.md`.

**Produce:**

- `/mnt/b/vm/images/ubuntu-24.04-server-cloudimg-amd64.img` con `SHA256SUMS` e firma verificati (come `fetch.sh` fa
  per la 26.04), disco `disks/ubuntu24.qcow2` da 40 GB, seme cloud-init: utente `lab` con la sola chiave SSH,
  `ubuntu-desktop-minimal`, `gnome-shell-extension-appindicator`, `orca`, `openvpn`, `wireguard-tools`, i pacchetti
  delle altre due. Accesso automatico di `lab` alla sessione grafica.
- **`lab/vm/gui-vm.sh <vm> install`** (dall'host): copia nella VM `galleria-gui`, `galleria`, `galleriad`,
  `fake-daemon` costruiti nell'immagine (`scripts/gui.sh build`), il file `.desktop` e l'icona in
  `~/.local/share/applications` e `~/.local/share/icons`, lancia `update-desktop-database`; `versions` stampa GNOME
  Shell, WebKitGTK, Orca, la libreria dell'indicatore trovata, lo stato dell'estensione AppIndicator, il tipo di
  sessione; `daemon fake <scenario>` e `daemon real` (il demone vero con `packaging/linux/install.sh` e il laboratorio
  di `vm-lab.sh`).
- Tabella delle versioni delle tre VM nell'esito, dall'uscita di `versions`.

- [ ] **1.** Scarica e verifica l'immagine; crea il disco e il seme; `vmctl.sh start ubuntu24`; aspetta la fine di
  cloud-init. **2.** `gui-vm.sh ubuntu24 install` e `versions` sulle tre VM. **3.** Una fotografia per VM con la
  finestra aperta e l'icona nella barra: è la prova che il binario parte e che la tray si vede (**S56**, primo pezzo).
- [ ] **4. Commit:** `add an ubuntu 24.04 desktop vm and a script that installs the app on the three`

**Ripiego per S55** (l'immagine cloud più cloud-init non dà una sessione GNOME usabile su 24.04): installazione dal
supporto desktop (`ubuntu-24.04-desktop-amd64.iso`) con `autoinstall`, sempre in `/mnt/b/vm`; se costa più di due ore,
ci si ferma e si riporta: Ubuntu 24.04 è obbligatoria e non si dichiara come limite.

**Fatto quando:** le tre VM si accendono, mostrano la finestra e si spengono con `vmctl.sh`; nel repo non è entrato
nessun file di `/mnt/b/vm`.

---

### Task 9: la passata manuale di G10 e delle riserve, sui tre desktop

**Stima:** 6 h. **Scioglie:** S54, S56, S58. **File — modifica:** `lab/vm/{gui-vm.sh,README.md}`,
`docs/08-esito-fase2.md`; il codice solo per le correzioni (`--fixup`).

La lista. Ogni riga si fa **su ciascuna delle tre VM** (Ubuntu 24.04, Ubuntu 26.04, Fedora 45; GNOME su Wayland con
l'estensione AppIndicator accesa), con il demone finto salvo dove dice «vero». L'esito ha una tabella 12 × 3 con
«sì», «no» o «non fatto», e per ogni casella il nome della prova conservata.

| # | Cosa si fa | Cosa si deve vedere | Chiude |
|---|---|---|---|
| M1 | Avvio dell'app dal menu delle applicazioni | la finestra; l'icona nella barra in alto, nella forma «disconnesso» | G10 |
| M2 | Menu della tray aperto | riga di stato, i preferiti, «Apri Galleria», «Esci», nella lingua del sistema | G10 |
| M3 | «Connetti» su un preferito dal menu, con il **demone vero** e `wg-split` | l'icona cambia; la riga di stato dice il nome; `ip link` ha `glr-wg0` | G10 |
| M4 | Chiusura della finestra con la «X» | la finestra sparisce, l'icona resta, `glr-wg0` c'è ancora, `galleria status` dice `connected` | G10 |
| M5 | «Disconnetti» dal menu, poi «Connetti» su un profilo OpenVPN con credenziali | la finestra torna da sola con la finestra delle credenziali | G10, R6 |
| M6 | «Apri Galleria», poi «Esci» con una sessione attiva | la voce dice che la VPN resta; il processo finisce; il tunnel c'è ancora | G10 |
| M7 | «Importa» con la finestra «Apri» vera, scegliendo il file **con il mouse e con la tastiera** | la finestra di GTK (quale: del programma o del portale, S58), il file importato; `stderr` senza `Gtk-CRITICAL` | R4, R7 |
| M8 | «Esporta» di un profilo ed «Esporta log…» con la finestra «Salva» vera | file scritto, `0600`, nel posto scelto | R4 |
| M9 | Doppio click su un `.ovpn` in Nautilus, con l'app chiusa e con l'app nascosta nella tray | l'anteprima di import, in **una** finestra | R5, R6 |
| M10 | Trascinamento di un `.ovpn` da Nautilus sulla finestra | l'anteprima | R5 |
| M11 | Finestra ridimensionata, chiusa, riaperta; secondo avvio con la finestra dietro un'altra | la dimensione di prima (o no, su Wayland: si scrive); la finestra viene davanti, o GNOME avvisa che è pronta | R6, R11 |
| M12 | «Ricorda» con il portachiavi **bloccato** (sulla Fedora lo è dopo l'accesso automatico) | la finestra di sblocco del sistema; dopo lo sblocco le credenziali sono salvate (`secret-tool search service galleria`) | R8 |

In più, **una volta sola** e su una VM: l'estensione AppIndicator **spenta** (`gnome-extensions disable …`): la «X»
chiude l'app e la VPN resta (Q53, S51 su un desktop vero); sulla Fedora, quale libreria dell'indicatore è stata
caricata (`/proc/<pid>/maps`: **S56**).

- [ ] **1.** `gui-vm.sh <vm> install` con il binario dell'ultimo commit del gruppo C. **2.** La lista, VM per VM. Il
  menu della barra si apre dalla tastiera vera (`vmkeys.py`: `Ctrl+Alt+Tab` fino alla barra in alto, frecce, `Invio`;
  **S54**); ogni passo ha la sua fotografia (`vncshot.py`). **3.** Per ogni «no»: è un difetto dell'app (si corregge,
  `--fixup`, e si rifà la riga sulle tre) o un limite del desktop (si scrive con il testo esatto di ciò che si vede).
- [ ] **4.** La tabella 12 × 3 e la tabella «Riserve ereditate» con l'esito di R4–R8, R10, R11, in `docs/08`.
- [ ] **5. Commit:** `record the manual pass of the tray and of the inherited reservations on three desktops`

**Ripieghi.** S54 falsa (la barra non si raggiunge con i tasti di qemu): il mouse dal monitor di qemu (`mouse_move`,
`mouse_button`) sulle coordinate lette dalla fotografia; se nemmeno così, le righe M2, M3, M5, M6 le fa una persona con
un visore VNC seguendo la lista, e l'esito dice chi. S56 falsa (sulla Fedora il binario non carica
`libappindicator3.so.1` o il menu non compare): è un difetto, non un limite: Fedora è obbligatoria; ci si ferma e si
riporta. S58: qualunque sia la finestra che si apre, lo si scrive; il portale resta limite dichiarato (R4).

**Fatto quando:** nessuna casella della tabella è vuota; ogni «no» ha accanto «corretto in `<commit>`» o il limite.

---

### Task 10: Orca 46 su Ubuntu 24.04, la passata con lo screen reader, l'avvio a freddo

Scritto su **Q56 (a)** e **Q57 (a)**.

**Stima:** 4 h. **Scioglie:** S57. **File — crea:** `lab/vm/startup-time.sh`. **Modifica:** `lab/vm/{orca-spike-vm.sh,orca-credentials-vm.sh,README.md}`
(il terzo nome di VM), `crates/galleria-gui/src/lib.rs` e `ui/src/main.tsx` (P14, quattro righe dietro la variabile),
`docs/08-esito-fase2.md`.

- [ ] **1. Orca 46 su Ubuntu 24.04 (R2: Q28, Q49).** Sulla VM nuova, con la procedura dello spike: `orca-spike-run.py
  ubuntu24 fields` (il nome dei campi di testo, `S4b`) e `orca-credentials-vm.sh run` (la finestra delle credenziali,
  `K5c`). Nell'esito, accanto alla tabella di Q49, la riga «Ubuntu 24.04, GNOME Shell 46, Orca 46.x»: il parlato com'è
  scritto nel file di debug e il fuoco AT-SPI. **S57**: se Orca 46 su GNOME vero li legge, il buco è solo
  dell'immagine; se non li legge, è il limite di R2, scritto nel README per Ubuntu 24.04. In nessuno dei due casi
  cambia il codice (Q49 (a)).
- [ ] **2. Passata con lo screen reader (R3, Q22).** Sulle tre VM, il giro `all` dello spike più quattro giri nuovi
  sull'app di oggi: rubrica (scegli, modifica, salva), import (testo incollato, anteprima), connessione (credenziali,
  stato, errore), tray e chiusura (annuncio a finestra nascosta, guardia). Per ogni giro: cosa Orca dice, in
  quest'ordine, dal file di debug. Nell'esito, una tabella «detto · non detto · detto due volte» per giro e per VM, e
  la **lista per l'ascolto** (Q56): venti minuti, dieci passi, cosa ci si aspetta di sentire a ogni passo. La lista è
  per Peppe o per chi ha le cuffie; non blocca.
- [ ] **3. Avvio a freddo (G13).** `lab/vm/startup-time.sh <vm>`: tre volte `echo 3 > /proc/sys/vm/drop_caches`, poi
  l'app con `GALLERIA_STARTUP_TRACE=1`; stampa i quattro tempi di ogni avvio. Su `ubuntu24` e su una seconda VM.
  Nell'esito: i numeri, la macchina (CPU dell'host, 4 core e 4 GB alla VM, disco su `/mnt/b`), e la frase «è una VM
  sulla macchina di Peppe, non la macchina di Peppe» (Q57). Sotto o sopra 1 s, il numero si scrive com'è.
- [ ] **4. Commit:** `measure the cold start and record orca 46 and the screen reader pass on real desktops`

**Fatto quando:** R2 e R3 hanno un esito nella tabella delle riserve; G13 ha tre numeri; la lista per l'ascolto sta in
una pagina.

---

## Gruppo E · Chiusura della fase 2

### Task 11: l'esito, il README, la spec

**Stima:** 4 h. **File — modifica:** `docs/08-esito-fase2.md`, `README.md`, la spec (§4.2, §13, §17, §18).

- [ ] **1. `docs/08-esito-fase2.md`, sezione «B6»**, come le altre: F1–F9 con prova, job ed esito; S46–S60 con esito e
  ripiego applicato; la tabella «Riserve ereditate» riga per riga (R12: S42, S43 e S44 di B5 letti uno per uno);
  numeri per livello; «Non verificato (B6)».
- [ ] **2. Sezione nuova in testa al file, «Chiusura della fase 2»**, che si legge da sola:

  | Parte | Contenuto |
  |---|---|
  | **G1–G13** | una riga per criterio: prova, job, numero del run verde **sull'ultimo commit** (o «aspetta il push»), e per i manuali la lista annotata |
  | **Limiti dichiarati** | un elenco solo, raccolto dalle sei sezioni: Orca e WebKitGTK (regioni vive, Q27); Orca 46 nell'immagine o su 24.04 (R2); altri lettori di schermo; KWallet; il portale del desktop; Wayland (fuoco, posizione, dimensione); la tray senza AppIndicator e senza la libreria; le copie di una credenziale che non sono nostre (B5); KDE e le altre distribuzioni; OpenVPN 2.7 in locale sotto Docker rootless; `Ctrl+L` nelle finestre di GTK via AT-SPI |
  | **Non verificato** | ciò che resta senza prova e senza essere un limite voluto, con il motivo. Se la lista non è vuota, lo dice la prima riga della sezione |
  | **Scostamenti dalla spec** | Q36 (istanza unica senza plugin), Q34 (posizione), i comandi di B6 (P1), `announce_phrases`, la richiesta delle versioni senza API (Q50) |
  | **Numeri** | prove per livello sull'ultimo commit; job di CI; ore stimate e spese per blocco, se si sanno |
  | **Decisioni** | D19–D32 e Q12–Q57, una riga ciascuna con l'esito, e dove sta il testo |

- [ ] **3. `README.md`**: per chi usa Galleria. Tray (serve l'estensione AppIndicator su GNOME; senza, chiudere la
  finestra esce), «Esci» e la VPN che resta, l'avviso di versione (già del T5), i limiti che toccano l'utente.
- [ ] **4. La spec**: §3.4 (la chiusura da B6), §4.2 (i sei comandi), §13 (com'è fatta), §17 (S46–S60), §18 (esito di
  Q50–Q57), e una riga in testa: «fase 2 chiusa il …, esito in `docs/08`».
- [ ] **5. Commit:** `write the outcome of block b6 and of phase 2`

**Fatto quando:** nella tabella G1–G13 non c'è una riga senza prova **e** senza la parola che dice cosa manca; nessun
limite è scritto solo in una sezione di blocco.

---

### Task 12: la roadmap e ciò che resta per la fase 3

**Stima:** 2 h. **File — modifica:** `ROADMAP.md`, `docs/08-esito-fase2.md` («Per la fase 3»), `.claude/handoffs/`.

- [ ] **1. `ROADMAP.md`, fase 2.** Le caselle di B1–B6 secondo i fatti di oggi, non secondo ciò che era vero quando la
  riga è stata scritta: molte di B1–B5 sono aperte per «il job non è ancora girato su un runner», e il run
  45769841084 le ha chiuse; ogni casella che si spunta porta il numero del run o il nome della prova. Una casella che
  resta aperta dice cosa manca. In fondo alla fase 2, una riga: **«Fase 2 chiusa»**, con il commit, il run e il
  rinvio all'esito, **solo se** vale «Quando la fase 2 è chiusa» (sotto).
- [ ] **2. `ROADMAP.md`, fase 3.** Alle voci che ci sono (servizio e pipe, WireGuardNT, OpenVPN su Windows, WFP, MSI e
  updater) si aggiunge **«La finestra su Windows»**, che oggi manca, con le caselle dell'elenco «Cosa resta per la
  fase 3» qui sotto.
- [ ] **3. `docs/08`, «Per la fase 3»**: lo stesso elenco, con il file e il modulo per ogni voce. **4.** Handoff in
  `.claude/handoffs/`.
- [ ] **5. Commit:** `close phase 2 in the roadmap and list what the window needs on windows`

**Fatto quando:** nessuna casella della fase 2 è spuntata senza la sua prova; la fase 3 ha le voci della finestra.

---

## Chiusura della fase 2

### Quando la fase 2 è chiusa

Tutte e sei, non una di meno:

1. I gruppi A–E sono rivisti e in `main`.
2. Aitano ha pushato e **un run di CI è verde sull'ultimo commit**, con i log dei gate letti dalla review (i nomi delle
   prove di `gates-b5.tsv` e `gates-b6.tsv`, non i segni verdi), come per B4 e B5.
3. La tabella G1–G13 dell'esito ha quel run in ogni riga automatica; G10 ha anche la lista M1–M12 sulle tre VM; G13 ha
   i tre numeri (non blocca, ma devono esserci).
4. Ogni riserva R1–R13 è «chiusa» o «limite», e i limiti sono nel README.
5. «Non verificato» è vuoto, oppure Peppe (o Aitano su delega) ha letto la lista e ha detto che la fase si chiude lo
   stesso: la riga della roadmap lo dice.
6. La roadmap e la spec sono allineate (T11, T12).

Chi lo dichiara: la review del gruppo E, che controlla i sei punti uno per uno; poi Aitano con Peppe.

### Cosa resta per la fase 3

Non è lavoro di B6: è l'elenco che il T12 scrive, perché chi apre la fase 3 non lo ricavi dal codice. La fase 3 è il
porting su Windows (analisi §10); la roadmap ha già le voci del demone. **Mancano quelle della finestra**, che in fase
2 è stata scritta e provata solo su Linux:

| Pezzo della finestra | Com'è oggi (Linux) | Cosa serve su Windows |
|---|---|---|
| Collegamento al demone | socket Unix, `link.rs` classifica gli errori di D27; i testi dicono `systemctl` e `usermod` | named pipe (D18); classificazione e testi del primo avvio rifatti per servizio e ACL |
| Archivio dei segreti | `KeyringSecretStore` è `cfg(linux)`: Secret Service; ripiego sul file `0600` | un archivio per Windows (Credential Manager o DPAPI) dietro lo stesso tratto; D25 va estesa |
| Istanza unica | `flock`, socket Unix, `SO_PEERCRED` (Q36) | un mutex con nome e una pipe, con lo stesso controllo dell'utente |
| Annunci | segnale ATK (`announce_atk.rs`); fuori da Linux `NoAnnouncer` | da provare: se WebView2 fa dire le regioni vive a Narratore e NVDA l'annuncio da Rust non serve; altrimenti UIA |
| Finestre «Apri» e «Salva» | GTK da Rust (`picker_gtk.rs`, `export_gtk.rs`); fuori da Linux sono un annullo | le finestre di Windows, sempre da Rust (spec §3.2, regola 3) |
| Tray | solo menu (Tauri su Linux) | i click sull'icona arrivano: click sinistro apre la finestra |
| «C'è chi mostra la tray?» | `tray_host.rs`, bus di sessione | su Windows c'è sempre |
| Versione nuova | avviso con l'indirizzo (D16) | updater di Tauri con l'MSI firmato (D16); l'avviso resta il ripiego (D16, punto 3) |
| Richiesta delle versioni | libsoup (`update_soup.rs`, `cfg(linux)`) | un secondo `Fetch` (WinHTTP o un crate): qui il vincolo «nessun crate nuovo» non vale più |
| Percorsi | XDG (`state.toml`, rubrica, cartella d'esecuzione) | `%APPDATA%`, `%LOCALAPPDATA%` |
| Associazione dei `.ovpn` | file `.desktop` | nell'MSI |
| Prove | immagine Docker, Xvfb, `WebKitWebDriver`, Orca, laboratorio Docker | runner Windows, `msedgedriver`, un laboratorio (`spikes/windows-lab` è il punto di partenza) |
| Aspetto | `oklch()` provato su WebKitGTK | da riprovare su WebView2 |

Restano fuori anche dalla fase 3, e vanno nella fase 4 o dopo la v0.1: pacchetti `.deb` e `.rpm` con le dipendenze
(WebKitGTK, libreria dell'indicatore) e repository; AppImage o Flatpak (dove torna il portale, R4); KWallet e KDE
provati; kill switch, connessione automatica, avvio con il sistema; QR di WireGuard; la CLI che usa le credenziali
ricordate (Q45); la migrazione dal file al portachiavi (Q20).

## Copertura della spec

| Spec | Dove |
|---|---|
| §13 tray: stato, preferiti, connetti e disconnetti, «Apri Galleria», «Esci»; solo menu | T1, T2; P2 |
| §13 «da B6 chiudere la finestra la nasconde; l'uscita è esplicita» | T3; Q53 per il desktop senza tray |
| §13 versione: una `GET` da Rust, senza identificativi, al più una al giorno, disattivabile, nel README | T4, T5; Q50, Q51 |
| §3.2 regola 4: nessuna rete dalla webview | T4: `update_view` non fa richieste; CSP invariata |
| §3.4 chiusura della finestra | T3 |
| §4.2 comandi di B6 | P1: sei comandi, e `ui_state_set` al posto di `settings_set` |
| §12.1 annunci a finestra nascosta (Q43: «continua a funzionare quando in B6 la finestra sarà nascosta») | T7 `K6a` |
| §14 fumo di livello 3, completo | T7 `the_whole_day_in_one_window` |
| §18 Q35 | T3 |
| §18 Q28, Q22 (e Q49 del piano di B5) | T10 |
| G10 · G11 · G13 | F1, F2 · F3 · F5, e «Dove gira ogni gate in CI» |
| G12, passata manuale annotata | T10 |
| §15 «fase 2 chiusa» | gruppo E |

Punti [S] di questo piano (la numerazione continua da S45):

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S46 | Il menu che Tauri costruisce (`muda`, AppIndicator) si legge e si clicca dal bus come quello della sonda | T1 | G10 in CI al livello 2 e con un comando di sola prova; il menu vero alla lista manuale |
| S47 | Riscrivere il contenuto dello stesso menu aggiorna ciò che il bus mostra, senza registrare di nuovo l'indicatore | T1 | undici voci fisse a cui si cambiano testo e stato |
| S48 | Il panico di `libappindicator-sys` senza libreria si ferma con `catch_unwind` e l'app parte senza tray | T1 | la libreria è un requisito scritto |
| S49 | A finestra nascosta la pagina riceve gli eventi e chiama i comandi senza ritardo che si noti | T2 | i tre casi senza domande li esegue Rust (Q52 (b), da riportare) |
| S50 | Orca dice un annuncio ATK emesso mentre la finestra è nascosta | T7 | limite scritto; lo stato si legge dal menu |
| S51 | `org.kde.StatusNotifierWatcher` con `IsStatusNotifierHostRegistered` dice se la tray si vede: vero su GNOME con l'estensione, falso senza | T3 (immagine), T9 (VM) | la chiusura nella tray diventa una scelta spenta di suo; da riportare |
| S52 | Con `soup3` la richiesta parte da Rust sul contesto principale, porta solo `Host`, `Accept-Encoding` e `Connection`, e il TLS c'è sulle tre distribuzioni (sulla Fedora `glib-networking` c'è **[V]**) | T4 | Q51 (b), da riportare |
| S53 | `prevent_close` più `hide` tiene vivo il processo e la pagina com'era; `show` la ritrova | T3 (Xvfb), T9 (Wayland) | riduzione a icona |
| S54 | L'indicatore nella barra di GNOME si raggiunge con i tasti mandati da qemu | T9 | mouse dal monitor di qemu; poi una persona con un visore VNC |
| S55 | L'immagine cloud di Ubuntu 24.04 con cloud-init dà un desktop GNOME come il modello della 26.04 | T8 | installazione dal supporto desktop con `autoinstall` |
| S56 | Il binario costruito su Ubuntu 24.04 parte sulle tre VM e sulla Fedora carica `libappindicator3.so.1` | T8, T9 | è un difetto: ci si ferma |
| S57 | Orca 46 su GNOME 46 vero legge il nome dei campi e la finestra delle credenziali | T10 | limite scritto per Ubuntu 24.04 (R2) |
| S58 | Su GNOME, senza sandbox, `FileChooserNative` apre la finestra di GTK dentro il programma, non il portale | T9 | si scrive quale; il portale resta limite |
| S59 | La finestra, da un'altra immagine, riesce a lasciare un segno nel registro dell'osservatore del client | T6 | i segni li scrive lo script con `docker exec` |
| S60 | Sotto Xvfb, senza gestore di finestre, si può mandare alla finestra la richiesta di chiusura (`WM_DELETE_WINDOW`) e leggere se è mappata, con ciò che l'immagine ha già (GTK e AT-SPI da Python) | T3 | il comando WebDriver che chiude la finestra; poi un gestore di finestre minimo nell'immagine, solo per `tray.e2e.ts`; l'esito dice quale |

## Cosa resta fuori da B6, per scelta

- Pacchetti, firma, repository; l'updater che installa (fasi 3 e 4): B6 avvisa e basta.
- Una schermata «Impostazioni»: le due scelte nuove stanno nel menu dell'app.
- Il motore `ksni` della tray e i click sull'icona su Linux.
- Notifiche del desktop, anche per la versione nuova (Q27).
- «Controlla adesso»: la richiesta la decide l'orologio, non un pulsante (punto 7).
- Aprire il browser dall'app (Q55).
- Avvio con il sistema, avvio nascosto nella tray, connessione automatica (dopo la v0.1).
- La stabilità della suite `a11y` (card `t_016cb5ea`).
- Orca in CI: resta locale (Q22). KDE, KWallet (R9).
- Qualsiasi modifica al demone, all'IPC, a `galleria-profile`, alla CLI.

## Cosa ho verificato e cosa no

Verificato oggi (26 novembre 2026), in questo worktree a `7e017c9`:

- Il codice della tabella «Il codice che B6 trova», letto file per file; `ci.yml`, `scripts/gui.sh`,
  `scripts/check-gates.py`, `gates-b5.tsv`, `lab/vm/` e `/mnt/b/vm`; la sezione «B5» e i «Non verificato» di B3, B4 e
  B5 in `docs/08`.
- La sonda della tray, **eseguita** nell'immagine `galleria-gui-dev` con e senza `StatusNotifierWatcher`
  (`spikes/b6-piano/tray-probe.py`): contro un AppIndicator di PyGObject, non contro `galleria-gui`.
- Il costo dei crate candidati (`Cargo.lock`, albero, `cargo deny` 0.20.2), **eseguito** su una copia dei manifest: il
  worktree non è stato toccato.
- I sorgenti di `tauri` 2.12.7 (`tray/mod.rs`, `Cargo.toml`), `tray-icon` 0.25.1, `libappindicator-sys` 0.9.0,
  `soup3` 0.5.0 (i nomi in `src/auto/`), **letti**.
- Le quattro risposte di GitHub nella tabella della richiesta, con `curl`.
- La VM Fedora 45, **accesa e rispenta**: i pacchetti della tabella. L'indirizzo dell'immagine cloud di Ubuntu 24.04.

Non verificato:

- Nessuna riga di codice di B6 è stata scritta o eseguita: moduli, comandi e prove di questo piano sono progetto.
- La GUI compilata con la feature `tray-icon`: ho letto l'albero con `cargo tree`, non ho compilato.
- La sonda contro il menu di `muda` (S46), la riscrittura del menu (S47), il panico fermato (S48).
- `soup3` usato da Rust: né la chiamata asincrona, né le intestazioni che libsoup aggiunge, né il TLS nell'immagine
  (S52). Le tre intestazioni dell'elenco sono ciò che ricordo di libsoup, non una misura.
- Quali licenze fanno fallire `cargo deny` con `ureq` su native-tls e `reqwest` su rustls: ho letto solo quella di
  `ureq` su rustls.
- La VM Ubuntu 26.04 oggi non l'ho accesa; la VM Ubuntu 24.04 non esiste ancora. Che su 24.04 l'estensione
  AppIndicator sia accesa di suo lo dice l'analisi §8, non l'ho visto.
- Il comportamento di `hide` e `show` su Wayland, e della pagina a finestra nascosta (S49, S53).
- Che la card `t_016cb5ea` chiuda con la causa trovata: è in corso mentre scrivo.
- Le stime: sono ore di lavoro previste, non misurate.

## Questioni aperte nuove

La numerazione continua Q12–Q49. Ognuna ha la mia raccomandazione, che vale come **predefinito per chi implementa**
se nessuno decide prima del task indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q50 | A quale indirizzo si chiede l'ultima versione. D16 dice «l'endpoint delle release su GitHub» | (a) la pagina `github.com/zPeppOz/galleria/releases/latest`, senza seguire il rinvio: la versione è nel `Location` · (b) l'API `api.github.com/repos/…/releases/latest`, che risponde JSON | **(a)**. Misurate tutte e due: (b) rifiuta con 403 una richiesta senza `User-Agent`, quindi obbliga a un'intestazione in più; ha un limite di 60 richieste all'ora **per indirizzo IP**, che in un ufficio dietro un solo indirizzo è di tutti i colleghi insieme; e manda 39 kB di cui serve un campo. (a) risponde senza `User-Agent`, non ha un corpo da leggere, e ciò che si legge è un indirizzo da cui si accetta solo un numero di versione. Costa che la forma del rinvio non è un contratto scritto di GitHub: se cambia l'avviso smette di comparire, senza errori, e lo si vede nella prova a mano del T4 | T4 |
| Q51 | Con cosa si fa la richiesta HTTPS da Rust. Oggi `galleria-gui` non ha un client HTTP | (a) `soup3`: la libsoup che WebKitGTK ha già caricato nel processo, 0 pacchetti nuovi · (b) `reqwest` 0.13 con native-tls: +8 pacchetti, `cargo deny` verde · (c) `ureq` 3 con rustls: +7 pacchetti e una licenza da aggiungere a `deny.toml` (`CDLA-Permissive-2.0`, i certificati di `webpki-roots`) | **(a)**. Nessun crate e nessuna libreria TLS in più in un client VPN; certificati e proxy sono quelli del sistema; è lo stesso schema del `gtk` che usiamo già per l'annuncio e per le finestre. Costa che vale solo su Linux (su Windows servirà un secondo `Fetch`: è nell'elenco della fase 3) e che non l'ho ancora eseguita: S52, con (b) come ripiego misurato | T4 |
| Q52 | Chi esegue un'azione del menu della tray | (a) Rust la passa alla pagina, che fa ciò che fa il pulsante · (b) Rust la esegue con `Sessions`, senza la pagina | **(a)**. La pagina resta viva a finestra nascosta e ha già sostituzione, conferma della sessione estranea, credenziali ed errori: con (b) le stesse regole avrebbero una seconda copia in Rust, e cadrebbe il punto 7 di B5 («Rust non sostituisce mai da solo»). Quando serve l'utente la finestra si riapre. Il rischio è che una pagina nascosta sia lenta: S49, con (b) limitata ai tre casi senza domande come ripiego | T1 (prima della card del gruppo A) |
| Q53 | Cosa fa la «X» quando la tray non si vede (GNOME senza l'estensione AppIndicator, libreria assente). La spec §13 dice «chiudere la finestra la nasconde»; l'analisi §8 dice che la tray è un accessorio | (a) senza tray la «X» esce dall'app, come oggi; la VPN resta · (b) nasconde sempre: si torna con un secondo avvio · (c) riduce a icona | **(a)**. Con (b) su una Fedora appena installata l'app sparisce e resta accesa senza niente che lo mostri. (a) costa che lo stesso gesto fa due cose secondo il desktop: lo dice il README, e in tutti e due i casi la VPN resta com'è | T3 |
| Q54 | «Esci» con una sessione attiva | (a) esce; la VPN resta, e la voce lo dice («Esci (la VPN resta attiva)») · (b) chiede «Disconnetti ed esci · Esci · Annulla» · (c) disconnette sempre | **(a)**. La sessione è del demone e può essere di un altro utente del gruppo (D21); la CLI si comporta così. Una finestra di domanda a ogni uscita è peggio di una voce che dice cosa succede. Chi vuole spegnere la VPN ha «Disconnetti» una riga sopra | T3 |
| Q55 | L'avviso di versione porta a un indirizzo. Come ci si arriva, visto che la finestra non esegue processi e la pagina non ha rete | (a) l'indirizzo è testo, con «Copia link» · (b) «Apri» lo passa al browser predefinito con `gio` (GTK avvia il browser) | **(a)**. È ciò che la finestra fa già con i comandi di D27, e tiene vera la frase «la GUI non avvia programmi» senza un'eccezione da spiegare. (b) sono cinque righe e si aggiunge quando qualcuno lo chiede | T5 |
| Q56 | La «passata a orecchio» di Q22. Un agente non sente: per Q49 il parlato è stato letto dal file di debug di Orca, senza audio | (a) la passata la fa un agente sulle tre VM, dal file di debug, e chiude G12; in più una lista di venti minuti per chi ha le cuffie, **non bloccante** · (b) la fase 2 resta aperta finché una persona non ha ascoltato | **(a)**. Il file di debug dice cosa Orca manda alla voce, nell'ordine: è ciò che le prove asseriscono da B2. Ciò che non dice (volume, velocità, se una frase è comprensibile) non dipende dal nostro codice. Con (b) la chiusura della fase dipende dal tempo di Peppe per una cosa che non cambierebbe il codice. Nell'esito si scrive «nessun ascolto umano» finché la lista non è fatta | T10 |
| Q57 | Dove si misura l'avvio a freddo. G13 dice «sulla macchina di Peppe», che è Windows con WSL e non ha WebKitGTK (Q15) | (a) in una VM Ubuntu sulla macchina di Peppe, con KVM: tre misure, e la frase che dice che è una VM · (b) su WSLg, dopo un `sudo apt install` che Q15 lascia a Peppe · (c) non si misura | **(a)**. È l'unica che si può fare senza toccare la macchina di Peppe, e gira su un desktop vero di una distribuzione obbligatoria. Il numero è prudente (una VM con 4 core, su un disco che non è un SSD): se è sotto 1 s lì, lo è anche fuori. G13 non blocca in nessun caso | T10 |
