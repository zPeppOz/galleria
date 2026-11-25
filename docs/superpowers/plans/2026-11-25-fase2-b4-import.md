# Fase 2 · Blocco B4 — Import: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** i profili entrano dalla finestra. Tre ingressi (file scelto, file trascinato, testo incollato) più il
doppio click su un `.ovpn`, un solo flusso: anteprima con gli avvisi e il testo di D19, salvataggio solo dopo
«Importa». Associazione dei `.ovpn` e istanza unica (D31). Chiude **G3**.

**Architettura:** il lato Rust di `galleria-gui` guadagna tre moduli senza `tauri` (`import`, `picker`, `instance`) e
cinque comandi sottili. Leggere il file, riconoscerne il formato, togliere le direttive e tenere profilo e segreti
fino a «Importa» succede tutto in Rust, con le funzioni di import della fase 1; `galleria-profile` riceve solo
aggiunte (Q29). **Nessun comando prende un percorso**: il file scelto lo chiede Rust con la finestra GTK (come
«Esporta» in B3), quello trascinato arriva a Rust da `WindowEvent::DragDrop`, quello aperto dal file manager arriva
come argomento del processo. Nessun comando parla con il demone: si importa anche a demone fermo.

**Stack:** quello di B3. Nessun crate nuovo se Q36 passa come raccomandata; `gtk` 0.18 c'è già.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md`, §3.2, §4.2, §4.3, §7, §14 («spec §n»). Stato di
partenza: `docs/08-esito-fase2.md`, sezione «B3». D19 e D31: `docs/07-proposte-fase2.md` §2.1 e §3.7. Decisioni
D5–D32 e Q12–Q35: chiuse. Q35 (la chiusura della finestra con modifiche non salvate va in B6) non tocca questo piano.

**Prima di partire:** le questioni Q36–Q41 (in fondo) devono avere una risposta. Se nessuno risponde vale la
raccomandazione scritta lì: il piano è scritto su quelle. **Q36 e Q37 cambiano il contenuto dei gruppi B e C**: vanno
decise prima di aprire la card del gruppo B. Worktree dedicato a partire da `main` (`39f5a6d` o successivo), che deve
contenere questo piano.

Legenda: **[V]** verificato oggi, con la fonte · **[S]** supposto, con il task che lo verifica.

## Gruppi di lavoro: una card per gruppo

Sette task in tre gruppi. I gruppi vanno in fila: ognuno parte dal ramo del precedente, già rivisto.

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Anteprima, testo e file** | T1, T2, T3 | cuore dell'import in Rust con l'anteprima in memoria; finestra «Importa profilo» con il testo incollato e il testo di D19; file scelto con la finestra GTK «Apri», pilotata dalla sonda AT-SPI | `main` con questo piano | 16 h | si importa dalla finestra un `.ovpn` o un `.conf`, da file o da testo; la CLI lo legge |
| **B · Trascinamento, apertura, istanza unica** | T4, T5 | file trascinato sulla finestra; file passato come argomento, istanza unica, associazione dei `.ovpn` | A, con Q36 decisa | 12 h | doppio click su un `.ovpn`: si apre l'anteprima nella finestra che c'è già |
| **C · G3 nel laboratorio e chiusura** | T6, T7 | profili del laboratorio importati dalla finestra e connessi dalla CLI sui tre client, nessun `PWNED-*`; tastiera, fumo, screen reader, esito | B, con Q37 decisa | 10 h | **B4 chiuso**: G3 con la sua prova |

Le stime contano **un solo giro di `scripts/gui.sh smoke` e uno di `scripts/gui.sh a11y` per gruppo, a fine gruppo**
(un'ora per gruppo, dentro il numero della tabella: i due giri e la correzione di ciò che trovano). Durante un task
si lanciano `check` e, se serve, la sola prova nuova con il filtro (vedi «Dove girano i comandi»). I giri ripetuti
(«tre volte di fila», «due volte») si fanno una volta sola in tutto il blocco, alla fine del gruppo C.

Ogni card riporta: numero dei task, i comandi di «Dove girano i comandi» verdi sull'ultimo commit, la sezione «B4» di
`docs/08-esito-fase2.md` aggiornata con i propri task. La review è una per gruppo.

## Criteri di uscita del blocco

| # | Criterio | Come si verifica | Task |
|---|---|---|---|
| I1 | **G3, flusso**: file scelto, file trascinato, testo incollato e file aperto dal file manager portano tutti alla stessa anteprima; sul disco non cambia niente prima di «Importa»; «Annulla», la chiusura e la scadenza non lasciano niente | prove di `import` (la cartella dei profili ha gli stessi byte fino a `import_commit`); fumo `nothing_is_written_before_import` | T1–T5 |
| I2 | **G3, D19**: una direttiva che lancia uno script è tolta e l'anteprima lo dice con il testo di D19; dopo l'import «N direttive rimosse»; lo script non viene mai eseguito | prove dei componenti sull'anteprima; fumo con il profilo malevolo; nel laboratorio, dopo la connessione dalla CLI, nessun file `PWNED-*` sui tre client | T1, T2, T6 |
| I3 | Spec §3.2, regole 2 e 3: nessun comando di import restituisce un segreto, nessun comando prende un percorso, la pagina non riceve percorsi né il testo del file | `no_import_command_returns_a_secret`; `no_command_takes_a_path` (di B3, che legge tutte le firme); `the_page_never_listens_to_the_drag_events_of_tauri`; nessun permesso nuovo oltre ai comandi | T1, T3, T4 |
| I4 | D31: solo i `.ovpn` sono associati; con l'app aperta, un secondo avvio con un file consegna il file alla finestra che c'è e termina; senza il mezzo per trovare la prima istanza l'app parte lo stesso | prove di `instance`; fumo `a_second_launch_hands_the_file_to_the_first_window`; prova della configurazione sul file `.desktop` | T5 |
| I5 | G12 e Q27 per i componenti di B4: tutto da tastiera; l'anteprima è una finestra modale e prende il fuoco; un errore di import prende il fuoco; l'esito è detto da Orca **una volta** | axe e tastiera per componente; `keyboard.test.tsx` esteso; `scripts/gui.sh a11y`: `I5a`–`I5e`, con la finestra «Apri» GTK vera pilotata via AT-SPI | T2, T3, T7 |
| I6 | La fase 1 non se ne accorge: IPC, demone, CLI e formato dei profili invariati; ciò che la finestra importa è ciò che importa la CLI dallo stesso file | gruppo dell'host verde; `git diff main -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-cli` vuoto; fumo `gui_import_equals_cli_import` | ogni task, T7 |

## Vincoli globali

Valgono i vincoli dei piani di B1, B2 e B3 (un task = un commit, messaggi in inglese minuscolo imperativo, TDD, nessun
push, niente `unwrap()` fuori dalle prove, la GUI non esegue processi, CSP invariata, testi solo nei cataloghi,
`#![forbid(unsafe_code)]`, ogni codice nuovo in `GUI_CODES` e nei due cataloghi nello stesso commit, ogni comando
nuovo in `COMMANDS`, in `capabilities/main.json` e in `tests/config.rs` nello stesso commit). In più:

- **Non si toccano**: `crates/galleriad`, `galleria-ipc`, `galleria-backend*`, `galleria-net`, `galleria-ovpn`,
  `galleria-cli`, `design/`, `docs/00`–`07`. In **`galleria-profile` solo aggiunte** (Q29): nessuna firma esistente
  cambia. In `galleria-e2e` solo un file di prova nuovo (T6). Se un task sembra chiedere una modifica a una funzione
  della fase 1, ci si ferma e si riporta.
- **Percorsi.** Nessun comando ha un parametro che è un percorso o un nome di file. Un percorso che arriva da fuori
  (trascinamento, argomento, altra istanza) è **dato non fidato**: passa da una sola funzione, `Imports::offer_path`,
  che fa i controlli del punto 1 della revisione. Alla pagina va al più il **nome del file** (`file_name`, l'ultima
  parte del percorso), mai la cartella.
- **Segreti.** L'anteprima porta i **nomi** dei segreti trovati (`secret_refs`), mai i valori né il testo della
  configurazione. Profilo e segreti restano in memoria in Rust fino a `import_commit` o `import_discard`. In B4
  l'archivio è `FileSecretStore`, preso dal punto unico `Rubrica::with_secrets` (B5 lo sostituisce).
- **Niente si salva prima di «Importa»** (M2): nessuna funzione prima di `import_commit` apre la cartella dei profili
  in scrittura.
- **Annunci (Q27).** L'esito dell'import è un avviso (*polite*). L'anteprima e un errore di import sono finestre o
  avvisi in linea che **prendono il fuoco** e non annunciano. Il testo annunciato è quello visibile.
- **Dove girano i comandi.** Ogni **task** finisce con:

  ```bash
  # host: la fase 1 e galleria-profile
  cargo fmt --all --check
  cargo clippy --workspace --exclude galleria-gui --all-targets -- -D warnings
  cargo test --workspace --exclude galleria-gui
  cargo deny check
  scripts/check-boundaries.sh
  # immagine: la GUI
  scripts/gui.sh check
  ```

  Ogni **gruppo** finisce, una volta sola e sull'ultimo commit, con:

  ```bash
  scripts/gui.sh smoke
  scripts/gui.sh a11y        # locale (Q22)
  ```

  Dentro un task, per la sola prova che il task aggiunge: `scripts/gui.sh run cargo test -p galleria-gui --test
  <nome>`; `GALLERIA_SMOKE_FILTER=<nome> scripts/gui.sh smoke`; `GALLERIA_A11Y_FILTER=<nome> scripts/gui.sh a11y` (il
  filtro di `a11y` nasce nel T3, come quello del fumo). Se il giro di fine gruppo trova un difetto in un task già
  committato, la correzione entra **nel commit di quel task** (`git commit --fixup` e `git rebase --autosquash`,
  prima di consegnare la card; mai dopo): resta un commit per task.
- **API di Tauri, `gtk`, `gio`**: nomi e firme dai sorgenti **della versione nel `Cargo.lock`** (`tauri` 2.12.7,
  `tauri-runtime-wry` 2.12.5, `wry` 0.57.4, `tao` 0.37.3, `gtk` 0.18.2). I nomi di questo piano valgono per il nostro
  codice.
- Se un punto **[S]** risulta falso si applica il ripiego scritto e lo si annota in `docs/08-esito-fase2.md`,
  sezione «B4». Se il ripiego tocca una decisione D o Q chiusa, ci si ferma e si riporta.

## Come Tauri consegna i percorsi (verifica chiesta dalla card)

Letto oggi nei sorgenti delle versioni fissate in `Cargo.lock`, nel registro di Cargo di questa macchina. **Letto, non
eseguito**: la prima esecuzione è nei task indicati.

| Ingresso | Cosa fa Tauri **[V]** | Conseguenza per noi |
|---|---|---|
| **Trascinamento** | `dragDropEnabled` è `true` se non lo si spegne (`tauri-utils` 2.10.4, `config.rs`). `wry` aggancia i segnali GTK del widget della webview (`drag-data-received`, `drag-motion`, `drag-drop`, `drag-leave`: `wry` 0.57.4, `webkitgtk/drag_drop.rs`) e chiama una funzione Rust; `tauri-runtime-wry` 2.12.5 la trasforma in `WindowEvent::DragDrop(DragDropEvent::Drop { paths, position })`. I gestori registrati con `Builder::on_window_event` lo ricevono in Rust (`tauri` 2.12.7, `manager/window.rs`, riga 98 e seguenti). Con il gestore acceso il trascinamento HTML5 nella pagina non c'è | **S9 della spec è vera a metà**: i percorsi arrivano a Rust senza passare dalla pagina, e il comando che prende i percorsi (il ripiego di S9) non serve |
| | **Ma** la stessa funzione, prima dei nostri gestori, **manda i percorsi anche alla pagina**: emette `tauri://drag-enter` e `tauri://drag-drop` con `paths` verso la finestra (`manager/window.rs`, `on_window_event`), e chi nella pagina si mette in ascolto li riceve. La nostra capability ha `core:event:allow-listen`, che serve per gli eventi `galleria:*` | la pagina **potrebbe** leggere i percorsi, se ascoltasse. Non è la regola 3 (che parla dei comandi) e un percorso non è un segreto, ma è un fatto che la spec non conosceva: **Q38**. Il piano è scritto su «la pagina non ascolta, e una prova lo tiene fermo» |
| | Alla consegna Tauri aggiunge i percorsi agli «scope» dell'app (`scopes.allow_file`) | senza plugin `fs` e con il protocollo `asset:` spento (lo è se non lo si accende, e la CSP non lo ammette) non apre niente. Una prova della configurazione lo tiene spento |
| **File aperto dal file manager** | Su Linux nessun evento: il percorso è un argomento di un processo nuovo (`RunEvent::Opened` è solo macOS, iOS e Android; l'esempio ufficiale `file-associations` legge `std::env::args()` e accetta sia percorsi sia URL `file://`) | Rust legge gli argomenti all'avvio; la pagina non c'entra |
| **App già aperta** | `tao` 0.37.3 crea `gtk::Application::new(app_id, ApplicationFlags::empty())` e `app_id` è `None` se `app.enableGTKAppId` è `false`, che è il predefinito (`tauri` 2.12.7, `app.rs` riga 2446): **GTK non fa l'istanza unica da solo**. Serve un meccanismo: **Q36** | vedi sotto |
| **Associazione** | `bundle.fileAssociations[].mimeType` «is written as `MimeType=` in the `.desktop` file» (`tauri-utils` 2.10.4, commento di `FileAssociation`). Il file `.desktop` lo scrive il bundler, cioè solo quando si fa un pacchetto; oggi `bundle.active` è `false` e i pacchetti sono fase 4 | il file `.desktop` in B4 lo scriviamo noi: **Q41** |

### Il plugin ufficiale di istanza unica, misurato

Eseguito oggi sull'host, in questo worktree, aggiungendo `tauri-plugin-single-instance = "2.5.5"` a
`crates/galleria-gui/Cargo.toml` e poi togliendolo (`cargo-deny` 0.20.2):

| Cosa | Esito **[V]** |
|---|---|
| Licenza | `Apache-2.0 OR MIT`, già in elenco |
| `Cargo.lock` | **32 crate nuovi**: `zbus` 5.19.0 con `zvariant`, `zbus_names`, `zbus_macros` e la famiglia `async-io` (`async-executor`, `async-process`, `async-signal`, `blocking`, `polling`, `piper`…): un secondo esecutore asincrono accanto a tokio |
| `cargo deny check` | **`advisories FAILED`**: tre avvisi «non più mantenuto», `RUSTSEC-2026-0370` (`zvariant`), `-0371` (`zbus_names`), `-0372` (`zvariant_derive`), «No safe upgrade is available». Licenze, ban e sorgenti verdi. Per passare servirebbero tre voci nuove in `deny.toml` |
| Senza bus di sessione (S11 della spec) | letto nel sorgente (`platform_impl/linux.rs`): l'app **parte normalmente** con una riga di avviso; non eseguito |
| Nome sul bus | `<identificatore>.SingleInstance`, cioè `io.github.zpeppoz.galleria.SingleInstance` (la spec §7 scrive `org.<id>…`: non è così in 2.5.5). Il metodo `ExecuteCallback(argv, cwd)` lo può chiamare qualunque processo della sessione |

È lo stesso tipo di conto che in B3 ha fatto scartare `tauri-plugin-dialog` (4 crate). Da qui Q36.

### Il tipo MIME dei `.ovpn` (S10 della spec)

Nell'immagine `galleria-gui-dev` (Ubuntu 24.04, `shared-mime-info` 2.4) **[V]**: `/usr/share/mime/globs2` ha
`application/x-openvpn-profile` per `*.ovpn` e `*.openvpn`, e `gio info` su un file `x.ovpn` risponde quel tipo. Su
Ubuntu 26.04 e Fedora **non l'ho guardato**: il comando che lo chiedeva alle immagini dei client è stato fermato dal
controllo dei comandi di questa sessione e non l'ho riformulato. Lo fa il T5.

Nell'immagine **non ci sono** `xdg-utils`, `desktop-file-utils` e `xdotool`; ci sono `gio` e `dpkg-deb` **[V]**.

## Punti di attenzione per la revisione

1. **Un percorso che arriva da fuori non è fidato.** Chi trascina o apre un file può dare una FIFO, un dispositivo,
   un file enorme, un collegamento simbolico. `offer_path` apre senza bloccarsi (`O_NONBLOCK`), guarda **il file
   aperto** (non il percorso, per non controllare una cosa e leggerne un'altra): deve essere un file normale, al più
   1 MiB, UTF-8. I file a cui un `.ovpn` rimanda si leggono solo dalla cartella vera del profilo (`canonicalize`, poi
   `starts_with`), come fa la CLI **[V: `galleria-cli/src/commands.rs`]**, e hanno lo stesso tetto. →
   T1 `a_fifo_a_device_and_a_huge_file_are_refused_without_hanging`.
2. **`ImportWarning` non porta gli argomenti** della direttiva tolta, solo riga e nome **[V: `galleria-profile/src/lib.rs`]**.
   Per scegliere la seconda frase di D19 («I DNS del server li applica Galleria») serve sapere se lo script è
   `update-resolv-conf` o `update-systemd-resolved`: Rust rilegge il testo con `openvpn_text::parse`, trova la
   direttiva a quella riga e manda alla pagina **solo un sì o un no** (`dns_script`). Gli argomenti non vanno mai alla
   pagina (in B3: `an_issue_never_carries_the_content_of_a_block_or_an_argument`). →
   T1 `a_notice_never_carries_an_argument`.
3. **L'evento può partire prima che la pagina ascolti.** Con `galleria-gui file.ovpn` l'anteprima è pronta prima che
   React sia montato, e un evento emesso lì si perde. L'offerta resta in attesa in Rust; la pagina la chiede al
   montaggio con `import_pending` (comando nuovo, senza parametri) e dopo ascolta `galleria:open-file`. →
   T4 `an_offer_made_before_the_page_is_ready_is_not_lost`.
4. **L'anteprima vive in memoria**: scade dopo 10 minuti (spec §4.2), al più 4 anteprime insieme (la più vecchia si
   butta: la pagina non deve poter riempire la memoria chiamando `import_from_text` in ciclo), i segreti si azzerano
   quando l'anteprima cade (`Secret` lo fa da sé alla distruzione: **[S29]**).
5. **Tutto o niente al salvataggio**, come la CLI: prima i segreti, poi il profilo; se il profilo non si scrive, i
   segreti si tolgono. È lo stesso ordine di «Duplica» (B3, T8). → T1 `commit_is_all_or_nothing`.
6. **Il testo incollato passa dalla pagina**, e può contenere una chiave privata: è ciò che l'utente digita (regola 2)
   e non si evita. La pagina svuota la casella appena l'anteprima arriva; Rust non lo scrive in nessun log e nessun
   errore lo ripete. → T2 `the_pasted_text_is_cleared_once_the_preview_arrives`, T1 `an_error_never_echoes_the_text`.
7. **Il runtime finto di Tauri non ha GTK né il trascinamento** **[V: B3, `gtk_window()` è `unimplemented!()`]**. La
   scelta del file sta dietro un tratto (`FilePicker`, come `SavePicker`); il gestore di `DragDrop` è di tre righe e
   chiama `offer_path`. La strada vera si prova nell'immagine: «Apri» con la sonda AT-SPI (T3), il trascinamento con un
   trascinamento vero sotto Xvfb (T4, **[S28]**).
8. **Il canale dell'istanza unica è un ingresso in più** nell'app: accetta solo percorsi, li tratta come il punto 1,
   non fa altro che mostrare un'anteprima. Con Q36 (b): file `0600` in una cartella `0700`, stesso utente controllato
   dal lato del socket, messaggi piccoli e a tempo. → T5.
9. **Le chiavi non vanno nel portachiavi in B4.** Il mockup dice «la chiave andrà nel portachiavi di sistema»: fino
   a B5 vanno nel file `0600` della fase 1. Il testo dell'anteprima dice «nell'archivio dei segreti», come in B3.
10. **Niente regressioni**: le prove di fumo e di screen reader di B1–B3 restano. Il Tab ha una sosta in più
    («Importa…» nella barra in alto): `keyboard.test.tsx`, `App.test.tsx`, fumo e Orca si aggiornano nel T2.

## Decisioni di progetto prese qui

Valgono per tutti i gruppi; chi implementa non le riapre.

| # | Decisione | Perché |
|---|---|---|
| P1 | Un solo punto in Rust tiene le anteprime (`import::Imports`, nello stato dell'app); i quattro ingressi sono quattro modi di chiamarlo (`offer_text`, `offer_path` con l'origine `picker`, `drop`, `open`) | «tre ingressi, un solo flusso» (spec §7) si prova una volta |
| P2 | Comandi: i quattro della spec §4.2 più **`import_pending`** (punto 3 della revisione). `import_pick_file` restituisce `null` se la finestra «Apri» viene annullata: un annullo non è un errore | come `profile_export` con `{ saved: false }` |
| P3 | L'anteprima mostra della direttiva tolta **riga e nome**, non la riga intera come nel mockup | un argomento può essere un segreto o un percorso della macchina di chi ha scritto il profilo; in B3 l'editor fa lo stesso |
| P4 | Il nome proposto è il nome del file senza estensione (per il testo incollato: «Profilo importato», dal catalogo, lo mette la pagina); se c'è già, «X (2)», «X (3)». L'utente può cambiarlo: stesse regole di `check_label` (B3). Due profili con lo stesso nome non sono vietati, come nella CLI | `ProfileStore::find` conosce già i nomi doppi (`Ambiguous`) |
| P5 | Il gruppo si sceglie tra quelli dell'elenco o «Nessun gruppo»; l'import non crea gruppi | «Gestisci gruppi» esiste già |
| P6 | Tetti: 1 MiB per il file o il testo, 1 MiB per ogni file a cui un `.ovpn` rimanda, 4 anteprime, 10 minuti | un `.ovpn` con quattro blocchi PEM sta in poche decine di KiB |
| P7 | Un'offerta che arriva da fuori (trascinamento, apertura) mentre la finestra di import è aperta **sostituisce** quella mostrata; l'anteprima di prima si scarta. Se c'è una modifica non salvata in un profilo, la guardia «Salva · Scarta · Annulla» di B3 scatta quando l'import riuscito sposta la selezione | una sola anteprima a schermo; nessuna coda da spiegare |
| P8 | Dopo «Importa»: la selezione va al profilo nuovo, il fuoco sulla sua riga nella lista, un solo avviso «Importato «X»» che, se ci sono direttive tolte, continua con «N direttive rimosse» | un avviso, un annuncio (Q27); il trascinamento non ha un pulsante a cui restituire il fuoco |
| P9 | Mentre un file è trascinato sopra la finestra Rust emette `galleria:drop-hover` con `{ active }`, senza percorsi; la pagina mostra «Rilascia per importare». È solo visivo (`aria-hidden`): da tastiera c'è `Ctrl+O` | la pagina non ascolta gli eventi di Tauri che portano i percorsi (Q38) |
| P10 | Il testo di D19 vale per le direttive che lanciano uno script: l'elenco è quello di B3 (`SCRIPT_DIRECTIVES` in `Configuration.tsx`), spostato in `ui/src/rubrica/scripts.ts` e usato da tutti e due; per WireGuard `PreUp`, `PostUp`, `PreDown`, `PostDown`. `dns_script` è vero se il primo argomento, tolta la cartella e un eventuale `.sh`, è `update-resolv-conf` o `update-systemd-resolved` | un elenco solo; i due nomi sono quelli della spec §7. Il suffisso `.sh` è una mia aggiunta (alcune distribuzioni installano lo script con quel nome: lo ricordo, non l'ho riletto) |
| P11 | Lo stato «rubrica vuota» non mostra più il comando della CLI ma il pulsante «Importa un profilo…» | da B4 la finestra basta a sé |

## Mappa dei file

```
crates/galleria-profile/src/import_file.rs        import_file, import_text, detect_protocol (solo aggiunta)     T1
crates/galleria-profile/tests/import_file.rs                                                                    T1
crates/galleria-gui/src/
  import.rs                   Imports, ImportPreview, offer_text, offer_path, commit, discard  — senza `tauri`  T1, T4
  picker.rs                   tratto FilePicker (accanto a SavePicker)                         — senza `tauri`  T3
  picker_gtk.rs               GtkFilePicker: «Apri» di GTK (cfg linux)                                          T3
  instance.rs                 istanza unica e argomenti                                        — senza `tauri`  T5
  commands/import.rs          cinque involucri sottili                                                          T1, T3, T4
  events.rs                   galleria:open-file, galleria:drop-hover                                           T4
  error.rs  lib.rs  main.rs   codici, registrazione, gestore di DragDrop, argomenti                             T1–T5
crates/galleria-gui/build.rs  capabilities/main.json  tests/config.rs     ogni task che aggiunge un comando
crates/galleria-gui/tauri.conf.json               bundle.fileAssociations (solo ovpn)                           T5
crates/galleria-gui/tests/{import,instance}.rs  + support/fixtures.rs                                           T1, T3–T5
crates/galleria-gui/examples/seed-rubrica.rs      opzione --fixtures <cartella>                                 T1
crates/galleria-e2e/tests/t17_gui_import.rs       il laboratorio legge ciò che la finestra ha importato         T6
packaging/linux/io.github.zpeppoz.galleria.desktop                                                              T5
lab/tests/atspi-file-window.py                    al posto di atspi-save-window.py: save, open, cancel, dump    T3
lab/tests/gtk-drag-source.py                      sorgente di un trascinamento vero                             T4
lab/tests/{gui-lib.sh,gui-a11y.sh,gui-lab-import.sh}  lab/run.sh (una riga)  scripts/gui.sh                     T1, T3, T6
ui/src/import/{ImportDialog.tsx,Preview.tsx,store.ts,types.ts}  con .css e prove                                T2–T4
ui/src/rubrica/scripts.ts                         elenco delle direttive che lanciano script (da B3)            T2
ui/src/shell/{TopBar,Shell,RubricaNotices}.tsx    «Importa…», Ctrl+O, rubrica vuota, «Rilascia per importare»   T2, T4
ui/src/i18n/{en,it}.json                          testi e codici nuovi                                          ogni task
ui/e2e/{smoke.e2e.ts,import.a11y.ts,lab-import.e2e.ts,support.ts}                                               T2–T7
docs/08-esito-fase2.md                            sezione «B4», aggiornata a ogni task
ROADMAP.md  .claude/handoffs/                     T7
```

---

## Gruppo A · Anteprima, testo e file

### Task 1: il cuore dell'import in Rust, con l'anteprima in memoria

**Stima:** 5 h. **Scioglie:** S29. **File — crea:** `crates/galleria-profile/src/import_file.rs`,
`crates/galleria-profile/tests/import_file.rs`, `crates/galleria-gui/src/import.rs`, `src/commands/import.rs`,
`tests/import.rs`, `tests/support/fixtures.rs`. **Modifica:** `crates/galleria-profile/src/lib.rs` (una riga `mod` e
una `pub use`), `crates/galleria-gui/src/{lib.rs,error.rs,commands/mod.rs}`, `build.rs`, `capabilities/main.json`,
`tests/config.rs`, `examples/seed-rubrica.rs`, `ui/src/i18n/{en,it}.json` (i codici), `lab/tests/gui-lib.sh`,
`docs/08-esito-fase2.md`.

**Produce**, in `galleria-profile` (solo aggiunte, Q29):

```rust
// import_file.rs
pub const IMPORT_MAX_BYTES: u64 = 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum ImportFileError {
    #[error("Cannot read the file")]            Unreadable(#[source] std::io::Error),
    #[error("Not a regular file")]              NotAFile,
    #[error("The file is too large")]           TooLarge,
    #[error("Not a .ovpn or .conf profile")]    UnsupportedFile,
    #[error(transparent)]                       Import(#[from] ImportError),
}
/// WireGuard se la prima riga che non è vuota né un commento è `[Interface]`; altrimenti OpenVPN.
pub fn detect_protocol(text: &str) -> Protocol;
/// Testo senza una cartella: un rimando a un file dà `ImportError::MissingFile`.
pub fn import_text(text: &str, name: &str) -> Result<Imported, ImportError>;
/// `.ovpn` è OpenVPN; `.conf` si decide dal contenuto (Q39); ogni altra estensione è `UnsupportedFile`.
/// I file a cui il profilo rimanda si leggono solo dalla cartella vera del profilo (spec F1 §8.2).
pub fn import_file(path: &Path, name: Option<&str>) -> Result<Imported, ImportFileError>;
```

In `galleria-gui`:

```rust
// import.rs — nessun tipo di tauri; tutte le viste sono Serialize e senza segreti
#[derive(Serialize, Clone, Copy)] #[serde(rename_all = "snake_case")]
pub enum Origin { Picker, Drop, Open, Text }
#[derive(Serialize, Clone)]
pub struct ImportNotice { pub line: usize, pub directive: String, pub kind: NoticeKind, pub dns_script: bool }   // kind: dangerous | unsupported | no_tls_client
#[derive(Serialize, Clone)]
pub struct ImportPreview {
    pub id: Uuid, pub origin: Origin, pub file_name: Option<String>,        // il nome, mai la cartella
    pub suggested_name: String, pub protocol: Protocol, pub server: Option<String>,
    pub auth: AuthFlags, pub found: Vec<String>,                            // i nomi di secret_refs
    pub notices: Vec<ImportNotice>, pub removed: usize,                     // dangerous + unsupported
    pub groups: Vec<String>, pub expires_in_secs: u64,
}
pub struct Imports { /* anteprime in attesa, orologio, Arc<Rubrica> */ }
impl Imports {
    pub fn offer_text(&self, text: &str) -> Result<ImportPreview, ImportFailure>;
    pub fn offer_path(&self, path: &Path, origin: Origin) -> Result<ImportPreview, ImportFailure>;
    pub fn commit(&self, id: Uuid, name: &str, group: Option<&str>) -> Result<ProfileSummary, ImportFailure>;
    pub fn discard(&self, id: Uuid);                                        // non fallisce se l'id non c'è più
}
```

`server` e `auth` si calcolano con le funzioni che `rubrica.rs` usa già per `ProfileSummary` e `AuthFlags`.
`Imports` prende l'orologio da fuori (un `Fn() -> Instant`), così le prove non aspettano dieci minuti.

Comandi di questo task: `import_from_text(text: String) -> ImportPreview`, `import_commit(id: Uuid, name: String,
group: Option<String>) -> ProfileSummary`, `import_discard(id: Uuid) -> ()`.

Codici (la spec §4.2 fissa i primi quattro; `tests/catalogs.rs` accetta già il prefisso `import_` **[V]**):

| Codice | Quando | `data` |
|---|---|---|
| `import_syntax` | `ImportError::Syntax(riga)` | `line` |
| `import_missing_file` | un file a cui il profilo rimanda non si legge; per il testo incollato sempre | `file` (com'è scritto nel profilo), `origin` |
| `import_outside_folder` | il rimando esce dalla cartella del profilo | `file` |
| `import_invalid` | `ImportError::Invalid` | — |
| `gui_import_unreadable` | il file non si apre o non è UTF-8 | `file_name` |
| `gui_import_too_large` | oltre 1 MiB (file, testo o file a cui si rimanda) | — |
| `gui_import_unsupported_file` | non è un file normale, o l'estensione non è `.ovpn` né `.conf` | `file_name` |
| `gui_import_expired` | `import_commit` su un'anteprima scaduta o scartata | — |

`gui_invalid_field` (nome) e `gui_group_not_found` sono quelli di B3.

`tests/support/fixtures.rs` (incluso anche da `seed-rubrica` con `#[path]`, come `seed.rs`) scrive in una cartella i
file di prova, con blocchi PEM finti che contengono `SEED-SECRET-…`:

| File | Contenuto |
|---|---|
| `inline.ovpn` | `client`, `remote`, `auth-user-pass`, blocchi `<ca>`, `<cert>`, `<key>`, `<tls-crypt>` |
| `separate.ovpn` + `ca.crt`, `client.crt`, `client.key` | gli stessi con `ca ca.crt`… al posto dei blocchi |
| `malicious.ovpn` | `inline.ovpn` più **le stesse righe** del profilo malevolo del laboratorio (`lab/server/make-profiles.py`: `script-security 3`, `up /usr/bin/touch /tmp/PWNED-up`, `plugin`, `management`, `status`, `writepid`, `setenv`) |
| `dns.ovpn` | `inline.ovpn` più `up /etc/openvpn/update-resolv-conf` e `down /etc/openvpn/update-systemd-resolved` |
| `outside.ovpn` | `ca ../ca.crt` |
| `office.conf` | WireGuard con `PostUp` e una chiave condivisa |
| `server.conf` | un profilo OpenVPN con estensione `.conf` (Q39) |

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `galleria-profile/tests/import_file.rs` | `detect_protocol_follows_the_first_meaningful_line` | parametrica: BOM, commenti `#` e `;`, righe vuote, `[interface]` minuscolo, un `.ovpn` che contiene «[Interface]» in un commento |
  | | `import_file_reads_inline_and_separate_files_into_the_same_profile` | `inline.ovpn` e `separate.ovpn` danno la stessa configurazione e gli stessi segreti |
  | | `a_conf_is_told_apart_by_its_content` | `office.conf` → WireGuard, `server.conf` → OpenVPN (Q39) |
  | | `another_extension_is_refused` | `.txt`, nessuna estensione |
  | | `a_reference_outside_the_folder_is_refused` · `a_symlink_that_leaves_the_folder_is_refused` | `outside.ovpn`; un `ca.crt` che è un collegamento a `/etc/hostname` |
  | | `a_fifo_a_device_and_a_huge_file_are_refused_without_hanging` | FIFO senza scrittore, `/dev/zero`, un file di 2 MiB, un `.ovpn` che rimanda a un file di 2 MiB; ognuna torna entro 2 s |
  | | `import_text_has_no_folder` | `ca ca.crt` nel testo → `MissingFile("ca.crt")` |
  | | `the_phase_1_import_functions_are_untouched` | `import_openvpn` e `import_wireguard` sugli stessi testi danno lo stesso risultato di `import_text` |
  | `galleria-gui/tests/import.rs` | `preview_names_the_secrets_and_never_their_values` | `found` ha `openvpn.block.key`…; il JSON non contiene `SEED-SECRET-` né `PRIVATE KEY` |
  | | `no_import_command_returns_a_secret` | il JSON di `import_from_text`, `import_commit`, di ogni errore e dell'anteprima di ogni file di prova (I3) |
  | | `the_malicious_profile_lists_every_removed_directive` | per `malicious.ovpn`: una voce per ognuna delle righe tolte, con riga e nome; `removed` è il loro numero |
  | | `a_notice_never_carries_an_argument` | nessuna voce contiene `/tmp/PWNED`, `7505`, `EVIL` |
  | | `dns_script_is_true_only_for_the_two_resolver_scripts` | parametrica: i due nomi, con e senza cartella e `.sh`; `up /usr/bin/touch` → falso; `plugin` → falso |
  | | `wireguard_hooks_are_dangerous_notices` | `office.conf`: `PostUp` |
  | | `nothing_is_written_before_commit` | dopo `offer_*` la cartella dei profili ha gli stessi file con gli stessi byte; dopo `discard` anche (I1) |
  | | `commit_writes_what_the_cli_import_writes` | il profilo salvato è uguale campo per campo a quello di `import_file` + `ProfileStore::save`, salvo id e date; i segreti sono nell'archivio |
  | | `commit_is_all_or_nothing` | archivio dei profili reso non scrivibile dopo i segreti (come `duplicate_is_all_or_nothing`): nessun segreto resta |
  | | `commit_checks_the_name_and_the_group` | nome vuoto, 65 caratteri, gruppo che non c'è |
  | | `suggested_name_avoids_the_names_in_use` | «inline», poi «inline (2)» |
  | | `a_preview_expires_after_ten_minutes` · `the_fifth_preview_drops_the_oldest` · `commit_and_discard_consume_the_preview` | con l'orologio finto; `gui_import_expired` |
  | | `an_error_never_echoes_the_text` | un testo rotto che contiene `SEED-SECRET-`: `message` e `data` non lo contengono |
  | | `import_works_without_any_daemon` | nessun socket |
  | | `fixture_matches_the_lab_malicious_profile` | ogni riga della stringa `malicious` di `lab/server/make-profiles.py` è in `malicious.ovpn` |
  | `tests/config.rs` | elenco dei permessi | più `allow-import-from-text`, `allow-import-commit`, `allow-import-discard` |

- [ ] **2.** Falliscono. **3. Implementa.** Prima di scrivere `ImportNotice` si legge `galleria_ovpn::import_filter`
  per sapere cosa viene tolto con un avviso `Unsupported` e cosa passa: `removed` conta solo ciò che è davvero tolto.
  **S29**: si guarda in `galleria-ipc` se `Secret` si azzera alla distruzione; se no, `Imports` azzera a mano e l'esito
  lo dice. `seed-rubrica --fixtures <cartella>` scrive i file di prova; `gui-lib.sh` genera lo script
  `import-fixtures` accanto a `rubrica-seed`.
- [ ] **4.** I comandi del task.
- [ ] **5. Commit:** `import profiles into a preview held in memory until confirmed`

**Fatto quando:** `git diff main -- crates/galleria-profile/src` ha solo righe aggiunte; `cargo tree -p galleria-gui
-e normal` non contiene `galleriad` né un backend.

---

### Task 2: finestra «Importa profilo», testo incollato, anteprima con il testo di D19

**Stima:** 5 h. **File — crea:** `ui/src/import/{ImportDialog.tsx,Preview.tsx,store.ts,types.ts}` con `.css` e prove,
`ui/src/rubrica/scripts.ts`. **Modifica:** `ui/src/shell/{TopBar,Shell,RubricaNotices}.tsx` e le prove,
`ui/src/shell/tabs/Configuration.tsx` (usa `scripts.ts`), `ui/src/shell/keyboard.test.tsx`, `ui/src/App.test.tsx`,
`ui/src/shell/boundary.test.ts`, `ui/src/daemon/commands.ts` (via `IMPORT_PROFILE`), cataloghi, `ui/e2e/smoke.e2e.ts`.

**Produce:** «Importa…» nella barra in alto e `Ctrl+O` aprono la finestra modale «Importa profilo», che ha due passi.

1. **Sorgente.** Il testo «Trascina qui un file .ovpn o .conf» (vero dal T4; fino ad allora la riga non c'è),
   il pulsante «Scegli un file…» (dal T3; fino ad allora non c'è: niente pulsanti che non rispondono), una casella
   «Oppure incolla il testo del profilo» (`Field as="textarea" mono`, senza correzione ortografica) e «Anteprima».
   Niente QR (spec §16, punto 7).
2. **Anteprima.** Nome del file (se c'è) e protocollo; «Trovati: …» con i nomi dei segreti tradotti
   (`secretLabel` di B3) e «saranno salvati nell'archivio dei segreti»; se `auth.user_pass`, «le credenziali saranno
   chieste alla prima connessione»; l'elenco degli avvisi; i campi «Nome» e «Gruppo»; «Importa», «Indietro»,
   «Annulla».

Testo degli avvisi (spec §7, D19; P3, P10):

| Voce | Testo (it) |
|---|---|
| `dangerous`, direttiva che lancia uno script, `dns_script` | «Riga N · Direttiva `up` **rimossa**: Galleria non esegue gli script dei profili. I DNS del server li applica Galleria.» |
| `dangerous`, direttiva che lancia uno script, altro | «… **rimossa**: Galleria non esegue gli script dei profili. Se il profilo ne dipende, potrebbe non funzionare del tutto.» |
| `dangerous`, altra direttiva (`management`, `status`, `writepid`…) | «… **rimossa**: Galleria non la permette nei profili.» |
| `unsupported` | «… **rimossa**: Galleria non la supporta.» |
| `no_tls_client` | «Manca `client`: il profilo potrebbe non essere un profilo client.» (non conta tra le rimosse) |

Comportamento (Q27, P4–P8):

- La finestra è modale: prende il fuoco all'apertura (sulla casella del testo; nel passo 2 sul campo «Nome»).
- Un errore di import (`import_*`, `gui_import_*`) è un avviso in linea **dentro la finestra che prende il fuoco**,
  con il testo del catalogo; `import_syntax` dice la riga; `import_missing_file` con origine `text` dice di importare
  dal file. Nessun avviso a scomparsa.
- Un nome rifiutato da Rust: fuoco sul campo, errore collegato con `aria-describedby` (come in B3, T5).
- «Annulla», `Esc` e la chiusura chiamano `import_discard`. «Indietro» anche, e torna al passo 1.
- «Importa» riuscito: la finestra si chiude, la rubrica si rilegge, la selezione va al profilo nuovo e il fuoco sulla
  sua riga; un avviso «Importato «X»», seguito da «N direttive rimosse» se N > 0 (plurali nei cataloghi).
- `gui_import_expired`: si torna al passo 1 con l'avviso in linea «L'anteprima è scaduta: scegli di nuovo il file».
- La casella del testo si svuota appena l'anteprima arriva (punto 6 della revisione).
- Rubrica vuota: pulsante «Importa un profilo…» al posto del comando della CLI (P11).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `import/Preview.test.tsx` | `script_directive_shows_the_d19_text` | `up` con `dns_script: false`: le due frasi di D19 |
  | | `resolver_script_says_galleria_applies_the_dns` | `dns_script: true`: la seconda frase è quella dei DNS e l'altra non c'è |
  | | `other_dangerous_and_unsupported_directives_do_not_say_script` | `management`, una direttiva inventata |
  | | `a_notice_shows_line_and_name_only` | — |
  | | `found_secrets_are_named_and_go_to_the_secret_store` | nessun «portachiavi» nel documento |
  | | `missing_client_is_a_warning_and_is_not_counted` | — |
  | | `wireguard_hook_shows_the_d19_text` | `PostUp` |
  | | `has_no_accessibility_violations` | — |
  | `import/ImportDialog.test.tsx` | `ctrl_o_and_the_button_open_it_and_focus_the_text` | — |
  | | `preview_step_focuses_the_name` | — |
  | | `the_pasted_text_is_cleared_once_the_preview_arrives` | — |
  | | `an_import_error_takes_the_focus_and_shows_no_notice` | Q27, regola 2 |
  | | `a_syntax_error_says_the_line` · `a_file_reference_in_pasted_text_says_to_import_the_file` | — |
  | | `cancel_escape_and_back_discard_the_preview` | tre chiamate a `import_discard` in tre giri |
  | | `import_selects_the_new_profile_and_focuses_its_row` | — |
  | | `the_notice_says_how_many_directives_were_removed` | 0, 1, 6: «Importato «X»», «… 1 direttiva rimossa», «… 6 direttive rimosse»; un solo avviso |
  | | `a_refused_name_moves_the_focus_to_the_field` | — |
  | | `an_expired_preview_goes_back_and_says_so` | — |
  | | `no_qr_and_no_dead_button` | nessun «QR»; «Scegli un file…» non c'è finché il comando non esiste |
  | `Shell.test.tsx` | `empty_rubrica_offers_the_import_button` | sostituisce `empty_rubrica_explains_how_to_import` |
  | `keyboard.test.tsx` | `tab_order_follows_the_layout` | «Importa…» prima di «Menu» |
  | | `an_import_from_text_needs_no_pointer` | `Ctrl+O`, testo, anteprima, nome, «Importa» |
  | `shell/boundary.test.ts` | `shell_only_calls_the_listed_commands` (di B3) | resta verde con i tre comandi nuovi |
  | `e2e/smoke.e2e.ts` | `a_profile_pasted_as_text_is_imported` | il testo di `inline.ovpn` (letto dal lato Node): anteprima, «Importa», la lista ha il profilo, i file sono sul disco |
  | | `nothing_is_written_before_import` | con l'anteprima aperta la cartella dei profili ha gli stessi file; dopo «Annulla» anche |
  | | `the_malicious_profile_shows_the_d19_text_and_runs_nothing` | testo di `malicious.ovpn`: la frase di D19 per `up` e `plugin`, «6 direttive rimosse» (o il numero vero: T1), nessun `/tmp/PWNED-*` |
  | | `empty_rubrica_shows_the_import_hint` (di B3) | diventa `empty_rubrica_offers_the_import_button` |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task; il fumo nuovo, se serve, con il filtro.
- [ ] **5. Commit:** `add the import window with the preview and the removed directives`

**Fatto quando:** `grep -rn "galleria import" ui/src` non trova nulla; `SCRIPT_DIRECTIVES` è in un file solo.

---

### Task 3: file scelto con la finestra «Apri» di GTK, e la sonda AT-SPI

**Stima:** 5 h, con un tetto (sotto). **Scioglie:** S27. **File — crea:** `crates/galleria-gui/src/picker.rs`,
`src/picker_gtk.rs`, `lab/tests/atspi-file-window.py`, `ui/e2e/import.a11y.ts`. **Elimina:**
`lab/tests/atspi-save-window.py` (diventa un modo della sonda nuova). **Modifica:** `src/commands/import.rs`,
`src/lib.rs`, `build.rs`, `capabilities/main.json`, `tests/config.rs`, `tests/import.rs`,
`ui/src/import/ImportDialog.tsx` e la prova, `lab/tests/gui-a11y.sh` (variabile `GALLERIA_A11Y_FILE_WINDOW`, filtro
`GALLERIA_A11Y_FILTER`), `ui/e2e/{support.ts,rubrica.a11y.ts}` (`R4e`, `R4f` passano alla sonda nuova), cataloghi.

**Produce:**

```rust
// picker.rs — nessun tipo di tauri
pub trait FilePicker: Send + Sync { fn pick(&self) -> Option<PathBuf>; }      // None = annullato
pub struct NoFilePicker;                                                       // fuori da Linux: annulla sempre
// picker_gtk.rs — #[cfg(target_os = "linux")]
pub struct GtkFilePicker<R: Runtime>(pub AppHandle<R>);
```

`GtkFilePicker` è `GtkSavePicker` di B3 con `FileChooserAction::Open`: thread principale di GTK, finestra `main` come
genitore, due filtri: i profili (`*.ovpn`, `*.conf`) e tutti i file. I testi dei pulsanti sono di GTK, nella lingua
del sistema. I due filtri non hanno un nome tradotto, perché Rust non ha cataloghi (D22): si chiamano
`*.ovpn, *.conf` e `*`.

Comando: `import_pick_file() -> Option<ImportPreview>`. Nessun parametro. Chiama il `FilePicker` nello stato
dell'app, poi `Imports::offer_path(percorso, Origin::Picker)`. Il percorso non esce mai da Rust: nella risposta c'è
`file_name`.

Webview: «Scegli un file…» nel passo 1. Annullato: niente, il fuoco resta sul pulsante. Riuscito: passo 2.

`lab/tests/atspi-file-window.py` generalizza la sonda di B3 (stessa uscita JSON, stessi codici di uscita):

```
atspi-file-window.py save PATH    «Salva»: scrive PATH nel campo del nome, attiva «Save»            (com'era)
atspi-file-window.py open PATH    «Apri»: fa comparire il campo del percorso, scrive PATH, attiva «Open»
atspi-file-window.py cancel       attiva «Cancel»
atspi-file-window.py dump         stampa campi, pulsanti e filtri della finestra
```

Nella finestra «Apri» di GTK 3 il campo del percorso non c'è finché non lo si chiede (`Ctrl+L`, oppure scrivendo `/`):
la sonda manda `Ctrl+L` con `Atspi.generate_keyboard_event`, aspetta il campo `text` modificabile, poi fa come per
«Salva» (**S27**).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/import.rs` | `pick_file_returns_the_preview_of_the_picked_file` | con un `FilePicker` finto che dà `inline.ovpn`; `origin` è `picker`, `file_name` è `inline.ovpn` |
  | | `pick_file_cancelled_is_null_and_not_an_error` | — |
  | | `the_response_never_holds_the_folder` | il JSON non contiene la cartella temporanea |
  | | `a_picked_file_gets_the_same_checks_as_a_dropped_one` | il selettore finto dà una FIFO: `gui_import_unsupported_file` |
  | `tests/export.rs` | `no_command_takes_a_path` (di B3) | resta verde con i comandi nuovi: `import_pick_file` non ha parametri |
  | `tests/config.rs` | `main_capability_grants_only_listed_permissions` | più `allow-import-pick-file`; ancora nessun permesso `dialog:*` o `fs:*` |
  | `ImportDialog.test.tsx` | `choose_a_file_shows_the_preview` · `a_cancelled_choice_keeps_the_focus_on_the_button` | — |
  | `e2e/import.a11y.ts` | `I5a_the_open_window_is_a_file_chooser_with_the_two_filters` | `dump`: ruolo `file chooser`, «Open» e «Cancel» |
  | | `I5b_a_file_picked_in_the_real_window_is_previewed_and_imported` | `open <cartella>/separate.ovpn`: l'anteprima mostra `separate.ovpn`, i tre segreti trovati; «Importa»; i file sono sul disco; Orca dice «Imported “separate”.» **una volta**, *polite* (`(1, 0, …)`); la cartella non è nella pagina |
  | | `I5c_cancel_in_the_real_window_changes_nothing` | nessun avviso, nessuna anteprima, fuoco su «Choose a file…» |
  | | `I5d_the_preview_is_read_as_a_dialog` | albero: `dialog` con il nome «Import profile», il fuoco su «Name»; il parlato è **registrato**, e asserito solo per il nome della finestra (i campi di testo: `S4b`) |
  | `e2e/rubrica.a11y.ts` | `R4e`, `R4f` (di B3) | restano verdi con `atspi-file-window.py save` e `cancel` |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task; `GALLERIA_A11Y_FILTER=I5 scripts/gui.sh a11y`
  per le prove nuove.
- [ ] **5. Commit:** `pick the file to import in the open window opened from Rust`

**Tetto e ripieghi per S27** (un'ora e mezza in tutto):

1. Se `Ctrl+L` mandato via AT-SPI non fa comparire il campo: la sonda scrive il percorso carattere per carattere con
   `Atspi.generate_keyboard_event` (il primo `/` apre il campo da solo), poi `Enter`.
2. Se nemmeno così: la finestra «Apri» resta coperta da `I5a` (si apre, è un `file chooser`) e `I5c` (annulla); `I5b`
   usa il file passato come argomento (T5) e l'esito dice che la **scelta** nella finestra vera è provata a mano una
   volta, e dove.

**Fine del gruppo A:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** `ls lab/tests/atspi-save-window.py` non trova nulla e nessun file lo nomina; S27 ha un esito scritto.

---

## Gruppo B · Trascinamento, apertura, istanza unica

### Task 4: file trascinato sulla finestra

**Stima:** 5 h, con un tetto (sotto). **Scioglie:** S9 (della spec), S28. **File — crea:** `lab/tests/gtk-drag-source.py`.
**Modifica:** `crates/galleria-gui/src/{import.rs,events.rs,lib.rs,commands/import.rs}`, `build.rs`,
`capabilities/main.json`, `tests/{config.rs,import.rs}`, `ui/src/import/{store.ts,ImportDialog.tsx}` e le prove,
`ui/src/shell/Shell.tsx`, `ui/src/shell/boundary.test.ts`, cataloghi, `ui/e2e/{import.a11y.ts,support.ts}`,
`lab/tests/gui-a11y.sh`.

**Produce:**

```rust
// import.rs
#[derive(Serialize, Clone)] #[serde(rename_all = "snake_case")]
pub enum ImportOffer { Preview(ImportPreview), Failed { error: CmdError, file_name: Option<String> } }
impl Imports {
    /// Fa l'anteprima (o l'errore) dei percorsi arrivati da fuori e la tiene in attesa per la pagina.
    pub fn offer_from_outside(&self, paths: &[PathBuf], origin: Origin) -> ImportOffer;
    pub fn take_pending(&self) -> Option<ImportOffer>;
}
```

- Gestore in `lib.rs` (tre righe per ramo, niente logica): su `WindowEvent::DragDrop`, `Enter` e `Leave` emettono
  `galleria:drop-hover` con `{ active }`; `Drop` chiama `offer_from_outside(paths, Origin::Drop)` fuori dal thread
  principale ed emette `galleria:open-file` con l'`ImportOffer`.
- Più di un file nello stesso gesto: `Failed` con `gui_import_many` (Q40). Zero file (si è trascinato un testo):
  niente.
- Comando `import_pending() -> Option<ImportOffer>`: `take_pending`. La pagina lo chiama al montaggio e poi ascolta
  `galleria:open-file`; chi consuma l'offerta la toglie dall'attesa (punto 3 della revisione).
- Webview: `galleria:open-file` apre la finestra di import già al passo 2, oppure al passo 1 con l'errore in linea
  (che prende il fuoco). `galleria:drop-hover` mostra «Rilascia per importare» sopra il guscio (P9). Nel passo 1
  compare la riga «Trascina qui un file .ovpn o .conf». **La pagina non ascolta nessun evento `tauri://…`.**
- `tauri.conf.json` non cambia: `dragDropEnabled` resta al predefinito. Una prova lo tiene fermo insieme al protocollo
  `asset:` spento.

`lab/tests/gtk-drag-source.py PATH`: una finestra GTK 3 piccola, messa in un angolo dello schermo di Xvfb, che offre
`PATH` come `text/uri-list`. La prova preme, sposta e rilascia il mouse con `Atspi.generate_mouse_event` (`b1p`,
`abs`, `b1r`) dalla finestra sorgente alla finestra di Galleria: è un trascinamento vero, lo stesso protocollo che
usa un file manager su X11 (**S28**).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/import.rs` | `a_dropped_file_becomes_a_pending_preview` | `origin` è `drop` |
  | | `two_dropped_files_are_refused_and_nothing_is_previewed` | `gui_import_many` |
  | | `a_dropped_folder_or_fifo_is_refused` | `gui_import_unsupported_file`, senza restare appesi |
  | | `an_offer_made_before_the_page_is_ready_is_not_lost` | `offer_from_outside`, poi `import_pending` la restituisce una volta sola |
  | | `a_new_offer_replaces_the_pending_one_and_discards_its_preview` | P7 |
  | | `the_offer_payload_holds_no_path_and_no_secret` | il JSON di `ImportOffer` non contiene la cartella né `SEED-SECRET-` |
  | `tests/config.rs` | `drag_drop_stays_on_and_the_asset_protocol_stays_off` | in `tauri.conf.json` nessuna finestra ha `dragDropEnabled: false`; `app.security.assetProtocol` assente o spento; la CSP non nomina `asset:` |
  | | elenco dei permessi | più `allow-import-pending` |
  | `shell/boundary.test.ts` | `the_page_never_listens_to_the_drag_events_of_tauri` | in `ui/src` non compaiono `tauri://`, `onDragDropEvent`, `getCurrentWebview`, e ogni `listen(` ha un nome che comincia per `galleria:` (I3, Q38) |
  | `import/store.test.ts` | `asks_for_a_pending_offer_at_start` · `an_open_file_event_opens_the_preview` · `a_failed_offer_shows_the_error_with_the_focus` | — |
  | `Shell.test.tsx` | `drop_hover_shows_and_hides_the_hint` | l'elemento è `aria-hidden` |
  | `e2e/import.a11y.ts` | `I5e_a_file_dragged_onto_the_window_is_previewed` | `gtk-drag-source.py office.conf`, trascinamento: anteprima WireGuard con `PostUp` e il testo di D19; «Importa»; Orca dice l'esito una volta |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task; `GALLERIA_A11Y_FILTER=I5e scripts/gui.sh a11y`.
- [ ] **5. Commit:** `preview a file dropped onto the window without sending its path to the page`

**Tetto e ripieghi per S28** (due ore in tutto):

1. Se il trascinamento sintetico via AT-SPI non parte: `xdotool` nell'immagine (`lab/images/gui.Dockerfile`, un
   pacchetto) per `mousedown`, `mousemove`, `mouseup`.
2. Se nemmeno così (senza gestore di finestre il protocollo XDND può non partire): `I5e` resta scritta e saltata con
   il motivo; il trascinamento vero si fa **una volta a mano** su una delle VM di `lab/vm/` (GNOME vero) e l'esito
   dice dove e cosa si è visto. La parte Rust resta coperta dal livello 2.

**Fatto quando:** S9 e S28 hanno un esito scritto; la prova `the_page_never_listens_to_the_drag_events_of_tauri` è
rossa se si aggiunge a mano un `listen("tauri://drag-drop")` (provato una volta, poi tolto).

---

### Task 5: file come argomento, istanza unica, associazione dei `.ovpn` (D31)

Scritto su **Q36 (b)** e **Q41 (a)**. Se Q36 passa come (a) (plugin ufficiale), `instance.rs` si riduce alla lettura
degli argomenti, il T5 aggiunge il plugin registrato per primo e le tre voci in `deny.toml` con il motivo, e le prove
di `instance` sul socket diventano prove nell'immagine sotto `dbus-run-session`.

**Stima:** 6 h. **Scioglie:** S10, S11 (della spec), S30, S31, S32. **File — crea:**
`crates/galleria-gui/src/instance.rs`, `tests/instance.rs`, `packaging/linux/io.github.zpeppoz.galleria.desktop`.
**Modifica:** `crates/galleria-gui/src/{main.rs,lib.rs}`, `tauri.conf.json`, `tests/config.rs`, `ui/e2e/{smoke.e2e.ts,support.ts}`,
`lab/tests/gui-lib.sh`, `README.md` (due righe: associazione e istanza unica), `docs/08-esito-fase2.md`.

**Produce:**

```rust
// instance.rs — nessun tipo di tauri
/// Gli argomenti che sono file: salta ciò che comincia per `-`, accetta `file://…` e percorsi, rende assoluto
/// ciò che è relativo rispetto a `cwd`. Non guarda il disco.
pub fn files_in_args(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Vec<PathBuf>;
pub enum Claim { Primary(Primary), Handed, Alone }      // Handed: i file sono andati alla prima istanza
/// `runtime_dir` è `$XDG_RUNTIME_DIR/galleria`; `None` (variabile assente) → `Alone`.
pub fn claim(runtime_dir: Option<&Path>, files: &[PathBuf]) -> Claim;
impl Primary { pub async fn serve(self, on_files: impl Fn(Vec<PathBuf>) + Send + 'static); }
```

Meccanismo (Q36 b): un blocco esclusivo su `gui.lock` (`File::try_lock`, **[V: stabile nella toolchain 1.96 del
workspace, provato oggi]**) dice chi è la prima istanza; il blocco cade da solo quando il processo muore. La prima
toglie un `gui.sock` rimasto e ne apre uno nuovo, `0600`, nella cartella `0700`. La seconda si collega, manda **una
riga JSON** (`{"v":1,"files":[…]}`), aspetta `ok` ed esce con 0; se la prima non risponde entro 2 s esce con 1 e una
riga su `stderr`. La prima accetta solo connessioni dello stesso utente (`peer_cred`), al più 64 KiB e 16 percorsi per
messaggio, 2 s di attesa; ogni altra cosa chiude la connessione. I percorsi ricevuti vanno a
`Imports::offer_from_outside(…, Origin::Open)`, poi la finestra viene mostrata, tolta dall'icona e portata davanti
(**S31**).

- `main.rs`: legge gli argomenti, chiama `claim` **prima** di costruire l'app (una seconda istanza non apre nemmeno
  WebKit). `Handed` → esce. `Primary` e `Alone` → l'app parte; i file degli argomenti vanno a `offer_from_outside`
  durante il `setup` e la pagina li trova con `import_pending` (T4). `Alone` scrive una riga su `stderr`: senza
  cartella di runtime l'istanza unica non c'è, l'app funziona lo stesso (S11).
- Senza file negli argomenti, un secondo avvio porta solo la finestra davanti.
- Gli URL `file://` si leggono con il crate `url`, che è già nell'albero di Tauri: come dipendenza diretta è una riga
  in `Cargo.lock` e nessun crate nuovo (da confermare con `cargo deny check`, come per `gtk` in B3); se non è così, si
  decodificano a mano prefisso e `%XX`.
- **Associazione** (D31, Q41): `packaging/linux/io.github.zpeppoz.galleria.desktop` con
  `MimeType=application/x-openvpn-profile;` e `Exec=galleria-gui %f`; in `tauri.conf.json`,
  `bundle.fileAssociations: [{ "ext": ["ovpn"], "mimeType": "application/x-openvpn-profile" }]`, con `bundle.active`
  che resta `false` (i pacchetti sono fase 4). Nessuna voce per `conf`. `packaging/linux/install.sh` **non si
  tocca**: installa demone e CLI, e la GUI installata è fase 4.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/instance.rs` | `files_in_args_skips_flags_and_takes_paths_and_file_urls` | parametrica: `--flag`, `a.ovpn` relativo, `/x/b.ovpn`, `file:///x/c%20d.ovpn`, `https://…` (saltato), argomento non UTF-8 |
  | | `the_first_claim_is_primary_and_the_second_hands_its_files` | due `claim` sulla stessa cartella temporanea: la prima riceve i percorsi, assoluti |
  | | `a_second_launch_without_files_only_raises_the_window` | la prima riceve una lista vuota |
  | | `the_lock_dies_with_the_process` | un processo figlio prende il blocco e viene ucciso: il `claim` successivo è `Primary` e sostituisce il socket rimasto |
  | | `without_a_runtime_dir_the_app_starts_alone` | `None` → `Alone` (S11) |
  | | `the_socket_is_private` | cartella `0700`, socket `0600` |
  | | `oversized_slow_or_garbled_messages_are_dropped` | 1 MiB senza a capo, una connessione che non scrive, JSON rotto, versione sconosciuta: la prima istanza resta viva e non chiama `on_files` |
  | | `a_primary_that_does_not_answer_makes_the_second_fail_loudly` | uscita 1 entro 3 s, niente attesa infinita |
  | | `handed_files_get_the_same_checks_as_dropped_ones` | una FIFO passata come argomento: `gui_import_unsupported_file` nell'offerta |
  | `tests/config.rs` | `only_ovpn_is_associated` | `bundle.fileAssociations` ha una voce sola, `ovpn`; nessuna `conf` (D31) |
  | | `the_desktop_file_and_the_bundle_agree` | `MimeType=` del file `.desktop` è il `mimeType` di `tauri.conf.json`; `Exec=` ha `%f`; il nome del file è l'identificatore |
  | | `bundle_stays_off` | `bundle.active` è `false` |
  | `e2e/smoke.e2e.ts` | `the_app_started_with_a_file_opens_its_preview` | l'app parte con `inline.ovpn` come argomento (**S30**): anteprima con origine «aperto», «Importa», profilo sul disco |
  | | `a_second_launch_hands_the_file_to_the_first_window` | con l'app aperta, dal lato Node si lancia `galleria-gui <cartella>/office.conf`: il secondo processo esce con 0 entro 5 s, la prima finestra mostra l'anteprima; resta **un** processo `galleria-gui` |
  | | `a_second_launch_without_a_runtime_dir_opens_a_second_window` | con `XDG_RUNTIME_DIR` tolto ai due processi: tutti e due vivono (S11); il secondo viene chiuso dalla prova |
  | | `an_ovpn_is_opened_by_the_desktop_entry` | il file `.desktop` copiato in una `XDG_DATA_HOME` temporanea, con `Exec` che punta al binario di prova: `gio mime application/x-openvpn-profile` lo elenca e `gio launch <file.desktop> inline.ovpn` apre l'anteprima (**S32**) |

- [ ] **2.** Falliscono. **3. Implementa.** `gui-lib.sh` esporta una `XDG_RUNTIME_DIR` dentro la cartella temporanea
  del giro, `0700`: le prove non toccano quella di chi le lancia. **S10**: `gio info -a standard::content-type` su un
  `.ovpn` in un contenitore Ubuntu 26.04 e in uno Fedora con `shared-mime-info` installato; l'esito nella tabella. Se
  su una delle due il tipo manca: una definizione MIME nostra in `packaging/linux/` (ripiego di S10), **non**
  installata da nessuno script in B4, e lo si scrive.
- [ ] **4.** I comandi del task; il fumo nuovo con il filtro.
- [ ] **5. Commit:** `hand a file opened from outside to the window that is already there`

**Fine del gruppo B:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** `cargo deny check` è verde senza voci nuove in `deny.toml`; `Cargo.lock` non ha crate nuovi (con
Q36 b); S10, S11, S30–S32 hanno un esito scritto.

---

## Gruppo C · G3 nel laboratorio e chiusura

### Task 6: i profili del laboratorio importati dalla finestra, connessi dalla CLI

Scritto su **Q37 (a)**: la finestra gira nell'immagine `galleria-gui-dev`, la cartella dei profili che ha scritto passa
ai client del laboratorio. Se Q37 passa come (b) (WebKit nelle tre immagini dei client) questo task si riscrive e
cresce: ci si ferma e si riporta prima di cominciare.

**Stima:** 5 h. **Scioglie:** S33. **File — crea:** `ui/e2e/lab-import.e2e.ts`, `lab/tests/gui-lab-import.sh`,
`crates/galleria-e2e/tests/t17_gui_import.rs`. **Modifica:** `scripts/gui.sh` (verbo `lab-import`),
`lab/run.sh` (una riga: inoltra `GALLERIA_E2E_GUI_STORE` al client, come già `GALLERIA_E2E_DCO`), `lab/README.md`,
`ui/e2e/vitest.config.ts` (il file `lab-import` gira solo quando è chiesto), `docs/08-esito-fase2.md`.

**Produce:** due passi, in quest'ordine.

1. **`scripts/gui.sh lab-import`** (nell'immagine della GUI, sotto Xvfb, come il fumo). Legge i profili veri del
   laboratorio da `lab/out/` (`base.ovpn`, `malicious.ovpn`, `wg-split.conf`: quelli che usano `t12` e `t16`),
   li importa **dalla finestra** passandoli come argomento a un secondo avvio (T5), e lascia la cartella dei profili
   in `target/gui-lab/config/galleria`. Per `malicious.ovpn` asserisce l'anteprima: la frase di D19 per `up` e
   `plugin`, una voce per ogni riga tolta, «N direttive rimosse» nell'avviso; e che dopo l'import non c'è nessun
   `/tmp/PWNED-*` nel contenitore. Se `lab/out/` non ha i profili, fallisce dicendo quale passo del laboratorio li
   genera (non li inventa).
2. **`GALLERIA_E2E_GUI_STORE=1 lab/run.sh <26|27|79> t17`**. `t17_gui_import.rs` (ignorata come le altre; senza la
   variabile si salta dicendolo, come la prova DCO) gira nel client:
   - copia `/lab/target/gui-lab/config/galleria` in `/home/alice/.config/galleria`, di `alice`, `0700` e `0600`;
   - `gui_imported_profiles_are_listed_by_the_cli`: `galleria list` come `alice` elenca i tre profili;
   - `gui_imported_wireguard_connects_from_the_cli`: come `cli_wireguard_roundtrip` di `t16`, senza l'import;
   - `gui_imported_openvpn_connects_from_the_cli`: `base`, con le credenziali di `alice` su stdin come in `t16`;
   - `gui_imported_malicious_profile_runs_no_script`: `galleria connect` del profilo malevolo importato dalla
     finestra; poi `find /tmp -name 'PWNED-*'` è vuoto, nessun processo `openvpn` ha `--up`, `--plugin` o
     `--management 0.0.0.0` tra gli argomenti, e dopo `disconnect` non restano residui (`residue()`). **L'esito
     della connessione si registra e si asserisce com'è**: mi aspetto `connected` (tolte le direttive resta il
     profilo `base`), ma non l'ho eseguito; se il demone lo rifiuta, è un fatto da riportare nell'esito, non da
     aggirare.

- [ ] **1. Prove che falliscono.** Le quattro prove di `t17` e `lab-import.e2e.ts`
  (`lab_profiles_are_imported_through_the_window`, `the_lab_malicious_profile_shows_the_d19_text`).
- [ ] **2.** Falliscono. **3.** Implementa. Prima di scrivere `gui-lab-import.sh` si legge quale script del
  laboratorio scrive `lab/out/*.ovpn` e `wg-split.conf` (oggi li usa `lab_file` in `galleria-e2e`; `make-profiles.py`
  gira nel server): **S33** è che esistano sull'host dopo `lab/gen-pki.sh` e `lab/up.sh` e che l'utente
  dell'immagine della GUI li possa leggere (sono `0600`). Se non è così, `lab-import` li copia con un `docker cp` dal
  contenitore del server, e lo si scrive.
- [ ] **4.** I comandi del task; poi i due passi sopra sui **tre** client (26, 27, 79), una volta ciascuno.
- [ ] **5. Commit:** `import the lab profiles through the window and connect them from the cli`

**Fatto quando:** l'esito ha, per ognuno dei tre client, l'uscita di `t17` (quattro prove) e la riga del `find`
vuoto; `lab/run.sh <client>` senza la variabile resta com'era (la prova si salta).

---

### Task 7: tastiera, fumo, screen reader, esito del blocco

**Stima:** 4 h. **File — modifica:** `ui/src/shell/keyboard.test.tsx`, `ui/e2e/{smoke.e2e.ts,import.a11y.ts}`,
`crates/galleria-gui/tests/import.rs`, `docs/08-esito-fase2.md`, `ROADMAP.md`, `.claude/handoffs/`.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `keyboard.test.tsx` | `a_full_import_tour_needs_no_pointer` | `Ctrl+O`, testo, anteprima, cambio di nome e gruppo, «Importa»; poi `Ctrl+O`, «Scegli un file…» (simulato), «Annulla» |
  | | `focus_is_never_lost_to_the_body` (di B2) | esteso: dopo import riuscito, annullato, errore, anteprima scaduta, offerta arrivata da fuori |
  | | `no_outcome_depends_on_a_live_region_alone` (di B3) | esteso all'avviso dell'import |
  | `tests/import.rs` | `no_import_command_returns_a_secret_after_a_full_round` | testo, file scelto, file trascinato, file aperto, errore, `import_pending`, `import_commit`, `import_discard`: nessuna risposta e nessun evento contiene `SEED-SECRET-`, `PRIVATE KEY` o la cartella (I3) |
  | `e2e/smoke.e2e.ts` | `gui_import_equals_cli_import` | `inline.ovpn`, `separate.ovpn` e `office.conf` importati dalla finestra e, in un'altra `XDG_CONFIG_HOME`, da `galleria import` (`GALLERIA_SMOKE_CLI`): i file dei profili sono uguali salvo id e date, i segreti uguali salvo l'id (I6) |
  | | `the_cli_lists_what_the_window_imported` | `galleria list` |
  | | `import_works_with_the_daemon_stopped` | — |
  | | `no_csp_violation_during_the_import_tour` | — |
  | `e2e/import.a11y.ts` | giro completo con Orca | `I5a`–`I5e` più: un errore di import prende il fuoco (albero: l'avviso ha il fuoco; parlato registrato); il testo di D19 è nell'albero come testo dell'elenco |

- [ ] **2.** Falliscono. **3.** Correggi ciò che le prove trovano, nei componenti a cui appartiene.
- [ ] **4.** `scripts/gui.sh check`; gruppo dell'host; poi, **una volta sola in tutto il blocco**:
  `scripts/gui.sh smoke` **tre volte di fila** e `scripts/gui.sh a11y` **due volte**.
- [ ] **5. Esito e roadmap.** In `docs/08-esito-fase2.md`, sezione «B4»: tabella I1–I6 e **G3** con prova ed esito;
  S9–S11 della spec e S27–S33 con esito e ripiego; la trascrizione di Orca del giro (righe `SPEECH OUTPUT`, così come
  sono) con la frase dell'esito e la riga `object:announcement`; numero di prove per livello; «Non verificato», che
  contiene almeno: il trascinamento da un file manager vero e su Wayland; la finestra «Apri» sotto un portale del
  desktop; il doppio click in un file manager vero (in B4 l'associazione è provata con `gio launch`, non con
  l'app installata da un pacchetto); la finestra portata davanti su Wayland; l'import nella finestra su Ubuntu 26.04
  e Fedora (Q37); il tipo MIME dove non è stato guardato. `ROADMAP.md`: le caselle di «B4» secondo i fatti.
  Nella spec: §17, esito di S9–S11; §7, il nome sul bus e il meccanismo scelto con Q36. Handoff in `.claude/handoffs/`.
- [ ] **6. Commit:** `cover the import with keyboard, smoke and screen reader tests`

**Fatto quando:** I1–I6 e G3 hanno ciascuno una prova verde, oppure l'esito dice quale non lo è e perché; nessuna
casella della roadmap è spuntata senza la sua prova.

---

## Copertura della spec

| Spec | Dove |
|---|---|
| §4.2 comandi di B4 | T1 (`import_from_text`, `import_commit`, `import_discard`), T3 (`import_pick_file`), T4 (`import_pending`, in più: P2) |
| §4.2 anteprima: identificativo, nome proposto, protocollo, avvisi come dati; in memoria fino a conferma; scade dopo 10 minuti; un codice per variante di `ImportError` | T1 |
| §4.3 `galleria:open-file` | T4 (trascinamento), T5 (file aperto) |
| §3.2 regole 2 e 3 | I3: T1, T3, T4, T7 |
| §7 tre ingressi, un flusso, anche dal doppio click | T2 (testo), T3 (file), T4 (trascinamento), T5 (apertura) |
| §7 testo di D19, «N direttive rimosse» | T1 (`dns_script`), T2 |
| §7 associazione di `.ovpn`, non dei `.conf` (D31) | T5 |
| §7 istanza unica | T5, con Q36 |
| §14 laboratorio: G3 e i pacchetti di WebKit nelle immagini dei client («lo decide il piano di B4») | T6, con Q37 |
| §16 punto 7: niente QR | T2 `no_qr_and_no_dead_button` |
| G3 | I1, I2: T1–T6 |
| G12 per i componenti di B4 | I5: T2, T3, T7 |
| ROADMAP «`.ovpn` con blocchi inline o file separati e `.conf` WireGuard» | T1 `import_file_reads_inline_and_separate_files_into_the_same_profile`, T7 `gui_import_equals_cli_import` |

Punti [S]. Quelli della spec che B4 doveva sciogliere:

| # | Supposizione | Stato dopo questo piano | La chiude | Se è falsa |
|---|---|---|---|---|
| S9 | I percorsi del trascinamento arrivano a Rust senza passare dalla webview | **letto nei sorgenti: vero**, con l'aggiunta che Tauri li manda anche alla pagina come evento (Q38). Non eseguito | T4, `I5e` | il ripiego della spec (un comando che prende i percorsi) non serve più: se il gestore non riceve niente, stop e si riporta |
| S10 | `.ovpn` ha un tipo MIME registrato sulle tre distribuzioni | **vero su Ubuntu 24.04** (`application/x-openvpn-profile`); 26.04 e Fedora non guardate | T5 | definizione MIME nostra in `packaging/linux/` |
| S11 | Comportamento dell'istanza unica senza bus di sessione | per il plugin, letto nel sorgente: parte lo stesso. Con Q36 (b) il bus non serve: conta `XDG_RUNTIME_DIR` | T5 | l'app parte comunque e lo scrive su `stderr` |

Punti [S] di questo piano (la numerazione riparte da S27: `S24`–`S26` sono già usate due volte tra il piano e l'esito di B3):

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S27 | La finestra «Apri» di GTK sotto Xvfb mostra il campo del percorso con `Ctrl+L` mandato via AT-SPI, e `set_text_contents` più «Open» scelgono il file | T3 | percorso scritto tasto per tasto; poi prova a mano (T3) |
| S28 | Un trascinamento da una finestra GTK, con il mouse mosso via AT-SPI sotto Xvfb senza gestore di finestre, arriva al gestore di `wry` | T4 | `xdotool`; poi prova a mano su una VM (T4) |
| S29 | `Secret` di `galleria-ipc` azzera il valore quando viene distrutto | T1, leggendo il tipo | `Imports` azzera a mano |
| S30 | `tauri-driver` passa all'app gli argomenti di `tauri:options.args` | T5 | la prova lancia il primo processo senza argomenti e usa il secondo avvio |
| S31 | Mostrare, togliere dall'icona e dare il fuoco alla finestra funziona sotto Xvfb; su Wayland il fuoco può essere negato dal compositore | T5 (solo Xvfb) | resta «non verificato» su Wayland; l'anteprima c'è comunque |
| S32 | `gio launch` con un file `.desktop` in una `XDG_DATA_HOME` temporanea lancia il binario con il percorso, nell'immagine senza `xdg-utils` | T5 | la prova asserisce `gio mime` e legge `Exec=` dal file, poi lancia il binario da sé con il percorso |
| S33 | I profili del laboratorio sono in `lab/out/` sull'host dopo `lab/up.sh`, leggibili dall'utente dell'immagine della GUI | T6 | `docker cp` dal contenitore del server |

## Cosa resta fuori da B4, per scelta

- Connettere dalla finestra, portachiavi, «Ricorda»: B5. In B4 si importa dalla finestra e si connette dalla CLI.
- Più file in un gesto, una coda di anteprime (Q40).
- QR WireGuard; associazione dei `.conf` (D31).
- Profilo nuovo da zero, senza un file: non è nella spec.
- Sostituire un profilo esistente reimportandolo («aggiorna le chiavi»): l'import crea sempre un profilo nuovo; in B3
  l'editor dice «importa di nuovo il profilo» e oggi vuol dire importarlo ed eliminare il vecchio.
- Il file `.desktop` installato da un pacchetto, l'icona, l'app come predefinita per i `.ovpn`: fase 4.
- La finestra che gira su Ubuntu 26.04 e Fedora dentro il laboratorio (Q37); la passata manuale è B6.
- La chiusura della finestra con modifiche non salvate: B6 (Q35).
- Prova di Orca in CI: resta locale (Q22).

## Cosa ho verificato e cosa no

Verificato oggi (notte tra il 25 e il 26 novembre 2026), in questo worktree a `39f5a6d`:

- La catena del trascinamento in `wry` 0.57.4, `tauri-runtime-wry` 2.12.5 e `tauri` 2.12.7, compresa l'emissione di
  `tauri://drag-drop` con i percorsi verso la pagina e l'aggiunta agli «scope»; `drag_drop_enabled: true` come
  predefinito in `tauri-utils` 2.10.4; `enable_gtk_app_id: false` come predefinito e `gtk::Application::new(app_id,
  ApplicationFlags::empty())` in `tao` 0.37.3. **Letti nei sorgenti del registro di Cargo, non eseguiti.**
- `tauri-plugin-single-instance` 2.5.5: licenza, dipendenza da `zbus`, 32 crate in più in `Cargo.lock`,
  `cargo deny check` rosso con i tre avvisi (eseguito, poi tolto: `git status` pulito); il comportamento senza bus e
  il nome sul bus, letti nel sorgente.
- `File::try_lock` della libreria standard: compila e funziona con `rustc` 1.96.0 (un programma di prova di sei righe).
- Il tipo MIME dei `.ovpn` e i pacchetti presenti e assenti nell'immagine `galleria-gui-dev`.
- Il commento di `FileAssociation::mime_type` in `tauri-utils` 2.10.4. Il modello del file `.desktop` del bundler
  (`Exec={{exec}}{{exec_arg}}`, `MimeType={{mime_type}}`) e l'esempio `file-associations` li ho letti **sul ramo
  `dev`** di Tauri, non nella versione della CLI fissata.
- Il codice della fase 1 che B4 usa: `import_openvpn`, `import_openvpn_bytes`, `import_wireguard`, `Imported`,
  `ImportWarning` (riga, nome, tipo: nessun argomento), `ImportError` (quattro varianti), il ramo `Import` della CLI
  (estensione, cartella vera, tutto o niente), `tests/catalogs.rs` che accetta già il prefisso `import_`, la riga del
  profilo malevolo in `lab/server/make-profiles.py`, `lab_file` e `t16` in `galleria-e2e`.

Non verificato:

- Nessuna riga di codice di B4 è stata scritta o eseguita: moduli, comandi e prove di questo piano sono progetto.
- Il trascinamento vero e la finestra «Apri» sotto Xvfb (S27, S28): non so se la sonda li pilota.
- Il tipo MIME su Ubuntu 26.04 e Fedora: il comando è stato fermato dal controllo dei comandi della sessione.
- Che un `GApplication` nostro conviva con quello di `tao` (alternativa c di Q36): non provato, per questo non è la
  raccomandata.
- Quante righe del profilo malevolo l'import toglie davvero (il «6» delle prove è da leggere sull'uscita del T1), e
  cosa risponde il demone al profilo malevolo ripulito (T6).
- Quale script del laboratorio scrive `lab/out/*.ovpn` sull'host e con quali permessi (S33).
- Cosa scrive in `Exec=` il bundler della CLI fissata quando ci sono associazioni (serve in fase 4, non qui).
- I nomi `update-resolv-conf.sh` e simili sulle distribuzioni (P10): li ricordo.
- Quanto durano oggi `scripts/gui.sh smoke` e `a11y`: l'ora per gruppo nelle stime è una mia stima, non una misura.

## Questioni aperte nuove

La numerazione continua Q12–Q35. Ognuna ha la mia raccomandazione, che vale come **predefinito per chi implementa**
se nessuno decide prima del task indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q36 | Con cosa si fa l'istanza unica. La spec §7 nomina il plugin ufficiale; misurato oggi, porta 32 crate (`zbus` e un secondo esecutore asincrono) e fa fallire `cargo deny` con tre avvisi «non più mantenuto» senza aggiornamento | (a) `tauri-plugin-single-instance` 2.5.5, con tre voci nuove in `deny.toml` · (b) codice nostro: blocco su un file e socket Unix in `$XDG_RUNTIME_DIR/galleria`, con tokio che c'è già · (c) un `GApplication` nostro con `gtk::gio`, che usa il bus di sessione | **(b)**. Nessun crate, nessun avviso da ignorare, non serve il bus di sessione (S11 sparisce), e si prova al livello 2 senza schermo: è lo stesso mestiere del socket del demone. Costa un centinaio di righe nostre da tenere. (a) è la strada della spec e funziona, ma per passare un percorso tra due processi mette in un client VPN una pila D-Bus intera e tre eccezioni di sicurezza; in B3 abbiamo scartato `tauri-plugin-dialog` per molto meno. (c) non l'ho provata accanto al `GApplication` di `tao`. È uno scostamento dal testo della spec §7: per questo la chiedo | T5 (prima della card del gruppo B) |
| Q37 | Dove gira il fumo di G3. La spec §14 dice «GUI installata sui client `glab-client*`» e lascia a questo piano i pacchetti di WebKit | (a) la finestra gira nell'immagine `galleria-gui-dev` e importa i profili veri del laboratorio; la cartella dei profili passa ai tre client, dove la CLI e il demone veri li connettono · (b) WebKitGTK, WebDriver, Node e Xvfb nelle tre immagini dei client | **(a)**. L'import non parla con il demone: ciò che G3 chiede al laboratorio è che il profilo importato dalla finestra si connetta e che lo script non giri, e questo lo provano CLI e demone veri sui tre client. (b) triplica le immagini (Fedora ha altri nomi di pacchetto) per rieseguire lo stesso codice di import. Il costo di (a): in B4 la finestra gira solo su Ubuntu 24.04; le altre due distribuzioni restano alla passata manuale di B6. In B5 la finestra deve parlare con il demone vero e la domanda torna: lì si decide (una strada è condividere il socket del client con l'immagine della GUI) | T6 (prima della card del gruppo C) |
| Q38 | Tauri manda i percorsi del trascinamento anche alla pagina, come evento `tauri://drag-drop`, a chi ascolta; la nostra capability permette l'ascolto degli eventi | (a) lasciarlo com'è: la pagina non ascolta, e una prova fallisce se qualcuno aggiunge l'ascolto · (b) spegnere il gestore di Tauri (`dragDropEnabled: false`) e agganciare noi i segnali GTK della webview · (c) il ripiego di S9: un comando che prende i percorsi | **(a)**. La regola 3 vieta che un **comando** prenda un percorso dalla pagina, e resta rispettata; un percorso non è un segreto. Con (b) si riattiva il trascinamento HTML5, e la pagina potrebbe leggere il **contenuto** del file, chiavi comprese: peggio. (c) è ciò che la regola vieta. Resta vero che una pagina compromessa potrebbe vedere i percorsi dei file trascinati: lo si scrive nell'esito | T4 |
| Q39 | Come si riconosce il formato. La CLI guarda solo l'estensione: `.conf` è WireGuard, `.ovpn` è OpenVPN, il resto è rifiutato. Un profilo OpenVPN che si chiama `client.conf` oggi fallisce con «profilo non valido» | (a) `.ovpn` è OpenVPN; `.conf` e il testo incollato si decidono dal contenuto (`[Interface]` in testa = WireGuard); altre estensioni rifiutate · (b) come la CLI | **(a)**. Per il testo incollato il riconoscimento dal contenuto serve comunque; usarlo anche per `.conf` non costa niente e toglie un errore che non spiega. La CLI non si tocca e resta più stretta della finestra su questo solo caso: lo si scrive nell'esito | T1 |
| Q40 | Più file trascinati o aperti insieme | (a) uno per gesto; con più di uno, un errore che lo dice · (b) una coda di anteprime, una alla volta · (c) import di tutti senza anteprima | **(a)**. (c) salta l'anteprima, che è il punto di M2. (b) è comodo per chi riceve dieci profili, ma è una finestra in più da progettare e provare; si riapre se qualcuno lo chiede davvero | T4 |
| Q41 | Dove si dichiara l'associazione dei `.ovpn`, visto che il file `.desktop` Tauri lo scrive solo quando fa un pacchetto e i pacchetti sono fase 4 | (a) `bundle.fileAssociations` in `tauri.conf.json` (pronto per la fase 4) **e** un file `.desktop` scritto da noi in `packaging/linux/`, tenuti uguali da una prova; `bundle.active` resta `false` · (b) accendere il bundler adesso · (c) solo `tauri.conf.json`, e l'associazione si prova in fase 4 | **(a)**. D31 mette l'associazione in B4, e senza un file `.desktop` non c'è niente da provare. (b) anticipa la fase 4 (formati, icone, dipendenze dei pacchetti). In B4 il file si prova con `gio launch`; nessuno script lo installa sul sistema | T5 |
