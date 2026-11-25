# Fase 2 · Blocco B3 — Rubrica e profili: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** la rubrica vera al posto dei dati finti: lista con gruppi, ricerca, preferiti e ordinamento; schede
Generale, Autenticazione e Rete secondo D26; editor raw con validazione; duplica, esporta, elimina; `state.toml`
(D32) con lingua e tema salvati (Q25). Dentro B3 c'è anche il **primo uso di Q27**: il comando `announce`, che fa
arrivare a Orca gli esiti di salvataggio, duplica ed elimina.

**Architettura:** il lato Rust di `galleria-gui` guadagna tre moduli senza `tauri` (`ui_state`, `announce`,
`rubrica`) e i comandi che li espongono. Legge e scrive i file dell'utente con `galleria-profile`, che riceve solo
funzioni nuove (Q29): formato dei profili, funzioni esistenti, demone e IPC non cambiano. Nessun comando parla con
il demone: la rubrica si usa anche a demone fermo (spec §5). Nella webview i dati finti spariscono e arriva
`ui/src/rubrica/`.

**Stack:** quello di B2. In più, lato Rust: `galleria-profile` come dipendenza normale, `toml`, `uuid`, `time` e
`tempfile` del workspace, e **`gtk` 0.18 come dipendenza esplicita solo su Linux** (già nell'albero di Tauri, vedi
«`cargo deny` e la dipendenza da `gtk`»). Per «Esporta»: `tauri-plugin-dialog`, usato solo da Rust **[S22]**.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md`, §3.2, §4.2, §6, §12.1, §16 («spec §n»). Stato di
partenza: `docs/08-esito-fase2.md`, sezioni «B2» e «Spike Orca». D26 e D32: `docs/07-proposte-fase2.md` §3.2 e §3.8.
Decisioni D5–D32 e Q12–Q27: chiuse.

**Prima di partire:** le questioni Q28–Q34 (in fondo) devono avere una risposta. Se nessuno risponde vale la
raccomandazione scritta lì: il piano è scritto su quelle. Worktree dedicato a partire da `main`, che deve contenere
questo piano e il merge dello spike (`lab/vm/`, sezione «Spike Orca» di `docs/08`).

Legenda: **[V]** verificato oggi, con la fonte · **[S]** supposto, con il task che lo verifica.

## Gruppi di lavoro: una card per gruppo

Nove task in quattro gruppi da due o tre, la dimensione delle card di B1 e B2. I gruppi vanno in fila: ognuno parte
dal ramo del precedente, già rivisto. Dentro il gruppo A i due task non dipendono l'uno dall'altro.

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Stato e annunci** | T1, T2 | `state.toml` con lingua, tema, ordinamento e finestra; comando `announce` con la prova di Orca nell'immagine (Q27) | `main` con questo piano | 11 h | lingua e tema che sopravvivono al riavvio; l'avviso a scomparsa che Orca pronuncia |
| **B · Rubrica in lettura** | T3, T4 | comandi di lettura sui profili veri, guscio senza dati finti; ricerca, preferiti, ordinamento, stato sulla riga | A | 11 h | rubrica vera in sola lettura sui profili importati dalla CLI |
| **C · Modifica** | T5, T6 | schede Generale, Autenticazione e Rete con `profile_update` (D26); gruppi in `galleria.toml` | B | 12 h | profili e gruppi modificabili dalle schede |
| **D · Editor, azioni, chiusura** | T7, T8, T9 | editor raw con validazione; duplica, esporta, elimina; tastiera, fumo, screen reader, esito | C | 16 h | **B3 chiuso**: rubrica completa, senza connettere |

Ogni card riporta: numero dei task, i comandi di «Dove girano i comandi» verdi sull'ultimo commit, la sezione «B3» di
`docs/08-esito-fase2.md` aggiornata con i propri task. La review è una per gruppo.

## Criteri di uscita del blocco

La spec non assegna a B3 un criterio G: chiude il punto «ci si può fermare qui» del §15 e la parte di G12 sui suoi
componenti e sugli annunci.

| # | Criterio | Come si verifica | Task |
|---|---|---|---|
| R1 | Rubrica completa sui profili importati dalla CLI, senza connettere: si leggono, si cercano, si ordinano, si modificano, si duplicano, si esportano, si eliminano; funziona a demone fermo | fumo nella finestra vera su una rubrica preparata con le funzioni di import della fase 1; `cli_lists_what_the_gui_saved` | T3–T9 |
| R2 | Lingua, tema, ordinamento e dimensione della finestra sopravvivono al riavvio (D32, Q25); un `state.toml` assente o rotto non dà errore | prove di `ui_state`; fumo `language_and_theme_survive_a_restart` | T1 |
| R3 | Nessun comando restituisce un segreto e nessuno accetta un percorso dalla webview (spec §3.2, regole 2 e 3) | `no_read_command_returns_a_secret` cerca i segreti della rubrica di prova nel JSON di ogni comando; `no_command_takes_a_path` sulle firme | T3, T8, T9 |
| R4 | Q27: Orca nell'immagine pronuncia l'esito di salvataggio, duplica ed elimina, **una volta sola**, e un errore con urgenza *assertive*; il segnale parte dal processo `galleria-gui` | `scripts/gui.sh a11y`: `S6b`, `R4a`–`R4d`; trascrizione nell'esito | T2, T5, T8, T9 |
| R5 | G12 per i componenti di B3: tutto da tastiera, ruoli corretti, conferme ed errori che chiedono una risposta prendono il fuoco | axe e tastiera per componente; `keyboard.test.tsx` esteso; albero AT-SPI e Orca | T3–T9 |
| R6 | La fase 1 non se ne accorge: IPC e formato dei profili invariati; un profilo salvato dalla GUI lo legge la CLI e viceversa | gruppo dell'host verde; `git diff main -- crates/galleria-ipc crates/galleriad crates/galleria-backend*` vuoto; `a_profile_saved_by_the_gui_keeps_the_phase_1_format` | ogni task |

## Vincoli globali

Valgono i vincoli dei piani di B1 e B2 (un task = un commit, messaggi in inglese minuscolo imperativo, TDD, nessun
push, niente `unwrap()` fuori dalle prove, la GUI non esegue processi, CSP invariata, testi solo nei cataloghi, nessun
file fuori da `ui/src/design/` importa la libreria «headless»). In più:

- **Non si toccano**: `crates/galleriad`, `galleria-ipc`, `galleria-backend*`, `galleria-net`, `galleria-ovpn`,
  `galleria-cli`, `design/`, `docs/00`–`07`. In **`galleria-profile` solo aggiunte** (Q29): funzioni e tipi nuovi con
  le loro prove; nessuna firma esistente cambia, `Profile` e `SCHEMA_VERSION` restano come sono. Se un task sembra
  chiedere una modifica a una funzione della fase 1, ci si ferma e si riporta.
- **Segreti.** Nessun comando restituisce un segreto: `profile_get` dà i nomi di `secret_refs`, mai i valori. Duplica
  ed esporta leggono e scrivono i segreti in Rust. In B3 l'archivio è `FileSecretStore`, preso da un solo punto
  (`rubrica::secrets()`), così B5 lo sostituisce con quello a strati senza toccare i comandi.
- **Percorsi.** Nessun comando accetta un percorso: «Salva» la apre Rust. I codici d'errore possono portare un
  percorso in `data` (non è un segreto), mai il contenuto di un file.
- **Scritture.** Sempre rename atomico e permessi della fase 1 (`0600` i file, `0700` le cartelle). `profile_update`
  porta l'ora di modifica che la webview conosce (`expected_modified`): se sul disco è diversa, perché la CLI o
  un'altra finestra hanno scritto, risponde `gui_profile_changed` e non scrive.
- **Codici d'errore.** Ogni codice nuovo (`gui_…`) entra in `GUI_CODES` e nei due cataloghi nello stesso commit
  (`tests/catalogs.rs` lo pretende già).
- **Permessi.** Ogni comando nuovo entra in `COMMANDS` di `build.rs`, in `capabilities/main.json` e nell'elenco di
  `tests/config.rs`, nello stesso commit. Nessun permesso di plugin alla webview: `tauri-plugin-dialog` si usa solo
  da Rust.
- **Annunci (Q27, spec §12.1).** L'avviso a scomparsa è l'unico punto della webview che annuncia. Un esito non
  bloccante è un avviso (*polite*); un errore che non chiede risposta è un avviso con tono `error` (*assertive*);
  conferme ed errori che chiedono una risposta spostano il fuoco e **non** annunciano. Niente annuncio per
  interruttori e stella: Orca dice già il nuovo stato del controllo. Il testo annunciato è quello visibile.
- **Dove girano i comandi**: come in B1 e B2. Ogni task finisce con i due gruppi verdi:

  ```bash
  # host: la fase 1 e galleria-profile
  cargo fmt --all --check
  cargo clippy --workspace --exclude galleria-gui --all-targets -- -D warnings
  cargo test --workspace --exclude galleria-gui
  cargo deny check
  scripts/check-boundaries.sh
  # immagine: la GUI
  scripts/gui.sh check
  scripts/gui.sh smoke
  ```

  `scripts/gui.sh a11y` (locale, Q22) è richiesto dove il task lo dice: T2, T5, T8, T9. Una prova sola:
  `scripts/gui.sh run cargo test -p galleria-gui --test <nome>`; un solo fumo: `GALLERIA_SMOKE_FILTER=<nome>
  scripts/gui.sh smoke`.
- **API di Tauri, `gtk`, `glib`, `atk`**: nomi e firme dai sorgenti **della versione nel `Cargo.lock`**. I nomi di
  questo piano valgono per il nostro codice.
- Se un punto **[S]** risulta falso si applica il ripiego scritto e lo si annota in `docs/08-esito-fase2.md`,
  sezione «B3». Se il ripiego tocca una decisione D o Q chiusa, ci si ferma e si riporta.

## Punti di attenzione per la revisione

1. **`emit_by_name` va in panico se il segnale non esiste** **[V: `glib` 0.18.5, `object.rs`, sezione «Panics»]**.
   `announcement` esiste da ATK 2.46 e `notification` da 2.50 **[V: feature `v2_46` e `v2_50` del crate `atk`
   0.18.2]**: su un sistema più vecchio un annuncio farebbe cadere l'app. Prima di emettere si chiede a GLib se il
   segnale c'è (`SignalId::lookup`, **[V: esiste in `glib` 0.18.5, `subclass/signal.rs`]**); se manca, niente
   annuncio e una riga su `stderr` una volta sola. → T2 `a_missing_signal_is_skipped_without_a_panic`.
2. **Il runtime finto di Tauri non ha la finestra GTK**: `gtk_window()` lì è `unimplemented!()` **[V: `tauri`
   2.12.7, `test/mock_runtime.rs`]**. Il comando `announce` parla con un tratto (`Announcer`); le prove di livello 2
   ne usano uno che registra, e la strada vera si prova solo nell'immagine con Orca.
3. **La frase non si deve sentire due volte.** L'avviso resta anche nella regione viva. Oggi Orca non la legge; se un
   giorno lo fa, `S6b` conta due volte la frase e diventa rossa, e le prove di controllo sulle regioni vive
   diventano rosse insieme: è il segnale per togliere l'annuncio.
4. **Lo spike ha emesso a tempo, non dopo un'azione.** Che l'annuncio emesso da un comando chiamato dalla pagina,
   subito dopo un tasto, arrivi a Orca e non venga coperto dal parlato del fuoco che torna sul pulsante è **[S18]**:
   lo chiude T2.
5. **Un profilo rotto non deve nascondere la rubrica.** `ProfileStore::list()` fallisce per intero se un solo file
   non si legge **[V: `store.rs`]** (per esempio un profilo scritto da una versione più nuova). → T3
   `list_lenient`, con i profili saltati mostrati per nome di file.
6. **I segreti di WireGuard sono indicizzati per posizione del peer** (`wireguard.peer.{i}.preshared_key` **[V:
   `import_wg.rs`]**): aggiungere o togliere un peer sposterebbe le chiavi. In B3 il numero dei peer non cambia. →
   T5 `peer_count_cannot_change`.
7. **Rinominare un gruppo riscrive più file** e non è atomico. L'ordine è: prima i profili, poi l'elenco; rifare
   l'operazione la completa. → T6 `rename_is_resumable_after_a_failure`.
8. **Il file esportato contiene le chiavi** (Q31): conferma prima, `0600`, mai passato dalla webview. → T8
   `export_never_sends_the_content_to_the_webview`.
9. **La finestra «Salva» del sistema non si pilota da WebDriver**, e non si mette nel binario una scorciatoia di prova
   (stesso principio di Q13). La scelta del file sta dietro un tratto; il fumo non la copre e lo si scrive.
10. **Niente regressioni**: le prove di fumo e di screen reader di B1 e B2 restano; quelle che usavano i dati finti
    passano alla rubrica di prova con gli stessi cinque nomi, così le asserzioni cambiano il meno possibile.

## `cargo deny` e la dipendenza esplicita da `gtk`

Eseguito oggi sull'host, in questo worktree, con `cargo-deny` 0.20.2 e l'archivio degli avvisi aggiornato al
2026-11-25, aggiungendo a `crates/galleria-gui/Cargo.toml` le due righe dello spike e poi togliendole:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
gtk = "0.18"
```

| Cosa | Esito **[V]** |
|---|---|
| `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok`, come prima |
| `Cargo.lock` | una riga in più (`gtk` tra le dipendenze di `galleria-gui`); nessun crate nuovo, nessuna versione nuova: resta `gtk` 0.18.2, la stessa di `tauri`, `tao`, `wry`, `muda` e `webkit2gtk` |
| Licenza | `gtk` 0.18.2 e `atk` 0.18.2: MIT, già in elenco |
| Avvisi «non più mantenuto» sui binding GTK3 (`RUSTSEC-2024-0413` `atk`, `-0415` `gtk`) | nell'archivio sono **ritirati** (`withdrawn = "2026-08-14"`): non scattano, né oggi né con la dipendenza diretta |
| `RUSTSEC-2024-0429` (`glib` 0.18, non solido in `VariantStrIter`) | già ignorato in `deny.toml` con il motivo. Sull'host `cargo deny` lo segnala come «advisory was not encountered» sia prima sia dopo: un avviso, non un errore |

Conseguenze per il T2:

- Si aggiunge **solo `gtk`**, con la stessa versione minore di Tauri (`"0.18"`), sotto `cfg(target_os = "linux")`.
  `glib` e `atk` si usano dalle riesportazioni (`gtk::glib`, il tipo restituito da `accessible()`), **non** come
  dipendenze dirette: il motivo scritto in `deny.toml` per `RUSTSEC-2024-0429` dice «Galleria depends on no glib
  crate directly» e deve restare vero.
- Noi non chiamiamo `VariantStrIter` né l'API delle varianti: `emit_by_name` passa una stringa e un intero. Il T2
  aggiunge a quel motivo mezza riga: `gtk` è diretto, per il solo annuncio di Q27.
- Una prova tiene ferma la versione: `gtk_is_the_one_tauri_uses` legge `Cargo.lock` e fallisce se compaiono due
  versioni di `gtk`. Quando Tauri lascerà GTK3 la dipendenza va rivista insieme alle due voci di `deny.toml`.
- **Cosa non so**: perché l'host dica «not encountered» per `-0429` mentre il commit `9c9296c` l'ha aggiunto per un
  job rosso in CI. La mia ipotesi, non verificata, è che la versione di `cargo-deny` del runner guardi gli avvisi
  «unsound» su tutto l'albero e questa solo sulle dipendenze dirette. Non cambia il piano: la voce resta. Non ho
  eseguito `cargo deny` nell'immagine né sul runner con la dipendenza aggiunta: lo fa il T2 (job `gui`).

## Decisioni di progetto prese qui

Valgono per tutti i gruppi; chi implementa non le riapre.

| # | Decisione | Perché |
|---|---|---|
| P1 | `announce` è **sia** un servizio Rust (`announce::Announcer`) **sia** un comando sottile sopra | i testi stanno nei cataloghi della webview (D22), quindi gli esiti nati lì passano il testo già tradotto; i cambi di stato di B5 nascono in Rust e useranno il servizio senza comando |
| P2 | Il comando prende `text` e `assertive`; pulisce il testo (spazi, caratteri di controllo, al più 300 caratteri) e non fallisce mai per un segnale assente | la webview non è fidata: può far dire a Orca una frase, non altro |
| P3 | Urgenza → segnale: `notification(testo, 1)` *polite*, `notification(testo, 2)` *assertive*; se `notification` manca, `announcement(testo)`; se manca anche quello, niente | sono le due forme provate dallo spike (righe C e D) |
| P4 | `profiles_list` restituisce anche i gruppi; non c'è `groups_get`. `groups_set` della spec diventa quattro comandi: `group_create`, `group_rename`, `group_delete`, `groups_reorder` | un solo giro per disegnare la lista; operazioni piccole si provano e si annullano meglio di un «sostituisci tutto» |
| P5 | I preferiti stanno in cima **dentro il proprio gruppo**, in tutti e due gli ordinamenti; la stella si cambia dal dettaglio | la lista resta una sola, senza una sezione in più; nel mockup la stella è solo un segno |
| P6 | Ordinamenti: `name` e `recent` (ultimo uso, poi nome). In B3 nessuno scrive ancora l'ultimo uso: lo scrive B5 alla connessione, con la funzione `touch_last_used` che nasce e si prova qui | D32; senza B5 «recente» coincide con «nome», e si vede |
| P7 | Campi di testo: «Salva» e «Annulla» espliciti quando il modulo è cambiato (Q33). Interruttori e stella scrivono subito | un nome a metà non deve finire sul disco; un interruttore è già un gesto compiuto |
| P8 | Validazione dei campi WireGuard in `galleria-gui`, sintattica, con le stesse regole di `galleria-backend-wireguard/src/validate.rs`; il demone resta l'autorità alla connessione | il backend non può essere una dipendenza normale della GUI (spec §3.2); in prova sì, per confrontare le due |
| P9 | La gestione dei gruppi è una finestra «Gestisci gruppi» aperta dal fondo della lista | i titoli dei gruppi dentro una `listbox` non sono punti di sosta: un menu lì non si raggiunge da tastiera |
| P10 | Fino al T8 le voci «Duplica», «Esporta», «Elimina» mostrano l'avviso `action.arrives_later`; fino al T5 e al T7 le schede non ancora fatte mostrano `pane.arrives_later` | ogni gruppo finisce con un'app che non mente, e il T2 ha un avviso vero su cui provare l'annuncio |
| P11 | La scadenza dei certificati (D26: «un task a parte e il primo a saltare») **non è in B3** | va insieme alle credenziali, in B5; qui la scheda Autenticazione mostra metodo e segreti presenti |

## Mappa dei file

```
crates/galleria-gui/Cargo.toml                                            T1 (toml, uuid, time, tempfile), T2 (gtk), T3 (galleria-profile), T8 (dialog)
crates/galleria-gui/build.rs  capabilities/main.json  tests/config.rs     ogni task che aggiunge un comando
crates/galleria-gui/src/
  ui_state.rs                 state.toml: lettura tollerante, scrittura atomica        — senza `tauri`      T1
  announce.rs                 Announcer, pulizia del testo, scelta del segnale         — senza `tauri`      T2
  announce_atk.rs             AtkAnnouncer: la sola parte che tocca gtk (cfg linux)                        T2
  rubrica.rs                  viste senza segreti, lettura, modifica, validazione      — senza `tauri`      T3, T5–T8
  export.rs                   SavePicker e scrittura del file esportato                — senza `tauri`      T8
  commands/{ui_state,announce,profiles,groups}.rs   involucri sottili                                      T1–T8
  error.rs                    GUI_CODES                                                                    T3, T5–T8
  lib.rs                      registrazione dei comandi, dimensione della finestra                         T1–T8
crates/galleria-gui/examples/seed-rubrica.rs      rubrica di prova per fumo e screen reader                 T3
crates/galleria-gui/tests/{ui_state,announce,rubrica,groups,raw,export}.rs  + support/                      T1–T8
crates/galleria-profile/src/settings.rs           galleria.toml: gruppi (solo aggiunta)                     T3, T6
crates/galleria-profile/src/{store.rs,lib.rs}     list_lenient, check_openvpn_config, render, export       T3, T7, T8
crates/galleria-profile/tests/{settings,raw,export}.rs                                                      T3, T6–T8
deny.toml                                         mezza riga nel motivo di RUSTSEC-2024-0429                T2
lab/tests/gui-lib.sh                              XDG_CONFIG_HOME e XDG_STATE_HOME di prova, seed          T1, T3
ui/src/app/announce.ts                            il solo file che invoca `announce`                        T2
ui/src/design/Toast.tsx                           proprietà `announce`                                      T2
ui/src/prefs.tsx  ui/src/main.tsx                 lingua e tema da state.toml                               T1
ui/src/rubrica/{store,types,arrange,validate}.ts  dati veri, ordinamento e ricerca puri                     T3–T5
ui/src/shell/{Shell,ProfileList,ProfileDetail}.tsx  SearchBox.tsx  SortControl.tsx                          T3, T4
ui/src/shell/tabs/{General,Authentication,Network,Configuration}.tsx  SaveBar.tsx                           T5, T7
ui/src/shell/GroupsDialog.tsx                                                                               T6
ui/src/i18n/{en,it}.json                          testi e codici nuovi                                      ogni task
ui/e2e/{smoke.e2e.ts,shell.a11y.ts,rubrica.a11y.ts,support.ts}                                              T1–T9
docs/08-esito-fase2.md                            sezione «B3», aggiornata a ogni task
ROADMAP.md  .claude/handoffs/                     T9
```

Spariscono al T3: `ui/src/shell/demo-data.ts`, i testi `demo.*`, la prova `demo_banner_is_visible`.

---

## Gruppo A · Stato e annunci

### Task 1: `state.toml`, lingua e tema salvati (D32, Q25)

**Stima:** 5 h. **Scioglie:** S21, S23.

**File — crea:** `crates/galleria-gui/src/ui_state.rs`, `src/commands/ui_state.rs`, `tests/ui_state.rs`.
**Modifica:** `Cargo.toml` (`toml`, `uuid`, `time`, `tempfile` del workspace come dipendenze normali), `src/lib.rs`,
`src/commands/mod.rs`, `build.rs`, `capabilities/main.json`, `tests/config.rs`, `ui/src/prefs.tsx`,
`ui/src/prefs.test.tsx`, `ui/src/main.tsx`, `ui/e2e/smoke.e2e.ts`, `lab/tests/gui-lib.sh`, `docs/08-esito-fase2.md`.

**Produce:**

```rust
// ui_state.rs — nessun tipo di tauri
pub const STATE_SCHEMA: u32 = 1;
#[derive(Serialize, Deserialize, Clone, Default)] #[serde(default)]
pub struct UiState {
    pub schema_version: u32,
    pub lang: LangChoice,                         // system | en | it
    pub theme: ThemeChoice,                       // system | light | dark
    pub sort: SortOrder,                          // name | recent
    pub window: Option<WindowSize>,               // { width, height, maximized }
    pub last_used: BTreeMap<Uuid, String>,        // RFC 3339; lo scrive B5
}
pub struct UiStateFile { path: PathBuf }
impl UiStateFile {
    pub fn default_path() -> PathBuf;             // $XDG_STATE_HOME/galleria/state.toml, altrimenti ~/.local/state/galleria/state.toml
    pub fn load(&self) -> UiState;                // mai un errore: assente, illeggibile o di uno schema più nuovo → predefiniti, con una riga su stderr
    pub fn update(&self, change: impl FnOnce(&mut UiState)) -> Result<UiState, UiStateError>;   // rilegge, cambia, rename atomico, 0600 in una cartella 0700
    pub fn touch_last_used(&self, id: Uuid, now: OffsetDateTime) -> Result<(), UiStateError>;
    pub fn forget(&self, id: Uuid) -> Result<(), UiStateError>;                                 // per «Elimina» (T8)
}
```

Comandi: `ui_state_get() -> UiState` e `ui_state_set(patch: { lang?, theme?, sort? })`. La webview non può scrivere
`window` né `last_used`: non sono nel `patch`. Le chiavi sconosciute si tollerano in lettura (uno `state.toml` di una
versione più nuova non è un errore) e si perdono alla prima scrittura: è un file che «se si perde non succede nulla»
(D32).

Finestra (Q34): Rust salva larghezza, altezza e «massimizzata» alla chiusura e, con un ritardo di mezzo secondo,
dopo un ridimensionamento; all'avvio le applica prima di mostrare la finestra. La posizione non si salva.

Webview: `main.tsx` chiede `ui_state_get` **prima** di montare React e applica il tema; se la chiamata fallisce o
tarda più di 300 ms si parte con «Sistema». `Preferences` riceve i valori iniziali e, a ogni scelta, chiama
`ui_state_set`; se la scrittura fallisce la scelta resta valida per la sessione e compare un avviso d'errore.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/ui_state.rs` | `missing_file_gives_the_defaults_without_an_error` | — |
  | | `broken_toml_gives_the_defaults_and_the_next_write_replaces_it` | — |
  | | `newer_schema_gives_the_defaults` | `schema_version = 99` |
  | | `unknown_keys_are_tolerated` | — |
  | | `state_dir_and_file_are_private` | cartella `0700`, file `0600`, anche se la cartella non esisteva |
  | | `update_keeps_what_it_does_not_touch` | cambiare `theme` non perde `last_used` |
  | | `path_follows_xdg_state_home_and_falls_back_to_home` | parametrica |
  | | `touch_and_forget_last_used` | — |
  | | `patch_cannot_set_window_or_last_used` | il tipo del `patch` non li ha; un JSON che li contiene viene rifiutato |
  | | `commands_round_trip_through_the_fake_runtime` | `ui_state_set` poi `ui_state_get`, con `tauri::test` |
  | `tests/config.rs` | elenco dei permessi | più `allow-ui-state-get`, `allow-ui-state-set` |
  | `prefs.test.tsx` | `saved_language_and_theme_are_applied_at_start` | con `ui_state_get` simulato |
  | | `a_choice_is_written_through_ui_state_set` | — |
  | | `a_failed_write_keeps_the_choice_and_shows_an_error` | — |
  | | `a_slow_or_failing_read_starts_with_the_system_values` | — |
  | `e2e/smoke.e2e.ts` | `language_and_theme_survive_a_restart` | «Italiano» e «Chiaro» dal menu, chiusura dell'app, riapertura: «Avvia il servizio» o «Generale» in italiano, `data-theme="light"` |
  | | `theme_attribute_is_set_before_first_paint` (di B2) | resta verde con il tema salvato (**S21**) |

- [ ] **2.** Falliscono. **3. Implementa.** `lab/tests/gui-lib.sh` esporta `XDG_CONFIG_HOME` e `XDG_STATE_HOME`
  dentro la cartella temporanea della prova: nessuna prova tocca la casa di chi la lancia. **S23**: annota
  nell'esito se la dimensione salvata viene applicata sotto Xvfb; su Wayland resta «non verificato».
- [ ] **4.** I due gruppi di comandi.
- [ ] **5. Commit:** `save language, theme, sort order and window size in state.toml`

**Fatto quando:** `grep -rn "localStorage" ui/src` non trova nulla; a mano, una volta: cancellato `state.toml` a
finestra aperta, la scelta successiva lo ricrea.

---

### Task 2: comando `announce` e prova con Orca nell'immagine (Q27)

**Stima:** 6 h, con un tetto (sotto). **Scioglie:** S18, S19, S20.

**File — crea:** `crates/galleria-gui/src/announce.rs`, `src/announce_atk.rs`, `src/commands/announce.rs`,
`tests/announce.rs`, `ui/src/app/announce.ts`, `ui/src/app/announce.test.ts`. **Modifica:** `Cargo.toml` (`gtk`),
`deny.toml` (motivo di `RUSTSEC-2024-0429`), `src/lib.rs`, `build.rs`, `capabilities/main.json`, `tests/config.rs`,
`ui/src/design/Toast.tsx` e la sua prova, `ui/src/App.tsx`, `ui/e2e/shell.a11y.ts`, `docs/08-esito-fase2.md`.

**Produce:**

```rust
// announce.rs — nessun tipo di tauri né di gtk
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Urgency { Polite, Assertive }
pub trait Announcer: Send + Sync { fn announce(&self, text: &str, urgency: Urgency); }
pub struct NoAnnouncer;                                         // fuori da Linux, e dove il segnale manca
pub fn clean(text: &str) -> Option<String>;                     // via i caratteri di controllo, spazi ripetuti in uno, al più 300 caratteri; None se resta vuoto
#[derive(PartialEq, Eq, Debug)]
pub enum Signal { Notification(i32), Announcement }             // 1 = polite, 2 = assertive (Atk.Live)
pub fn pick_signal(has_notification: bool, has_announcement: bool, urgency: Urgency) -> Option<Signal>;   // P3

// announce_atk.rs — #[cfg(target_os = "linux")]
pub struct AtkAnnouncer<R: Runtime>(pub AppHandle<R>);          // run_on_main_thread → finestra `main` → gtk_window() → accessible() → emit_by_name
```

Comando: `announce(text: String, assertive: bool) -> Result<(), CmdError>`. Chiama `clean`, poi l'`Announcer` nello
stato dell'app. Non restituisce mai un errore per un segnale assente o una finestra che non c'è più.

Dentro `AtkAnnouncer`, nel thread principale di GTK: si chiede a GLib quali dei due segnali esistono sul tipo
dell'oggetto accessibile (`SignalId::lookup`), si sceglie con `pick_signal`, si emette. Mai `emit_by_name` su un
segnale non trovato (punto 1 della revisione). `#![forbid(unsafe_code)]` resta: non serve codice `unsafe`.

Webview: `ToastProvider` guadagna la proprietà `announce?: (text: string, assertive: boolean) => void` e la chiama
una volta per ogni avviso mostrato (`assertive` per il tono `error`). `App` le passa la funzione di
`app/announce.ts`, che è il solo file a invocare il comando e che ingoia un eventuale rifiuto (un annuncio fallito
non deve rompere l'azione).

`scripts/gui.sh a11y` non cambia: l'immagine ha già Orca 46.1 e ATK 2.52 **[V: `dpkg-query` nell'immagine, oggi]**.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/announce.rs` | `clean_strips_control_characters_and_caps_the_length` | parametrica: `"a\u{7}b"`, tre righe, 500 caratteri, solo spazi → `None` |
  | | `pick_signal_follows_the_table` | le sei righe di P3 |
  | | `announce_command_forwards_the_cleaned_text_and_the_urgency` | con `tauri::test` e un `Announcer` che registra |
  | | `empty_text_is_dropped_without_an_error` | — |
  | | `a_missing_signal_is_skipped_without_a_panic` | (solo Linux) `SignalId::lookup("galleria-no-such-signal", tipo)` è `None` e la funzione di emissione torna senza emettere (**S19**) |
  | | `gtk_is_the_one_tauri_uses` | in `Cargo.lock` c'è una sola voce `name = "gtk"` |
  | | `no_glib_or_atk_direct_dependency` | `Cargo.toml` di `galleria-gui` non nomina `glib` né `atk` |
  | `tests/config.rs` | elenco dei permessi | più `allow-announce` |
  | `Toast.test.tsx` | `every_notice_is_announced_once` | due avvisi → due chiamate, con il testo visibile |
  | | `error_notices_are_announced_as_assertive` | — |
  | | `works_without_an_announcer` | senza la proprietà non lancia |
  | `app/announce.test.ts` | `only_one_file_invokes_announce` | in `ui/src`, il letterale `"announce"` come nome di comando compare solo in `app/announce.ts` |
  | | `a_rejected_announce_does_not_throw` | — |
  | `e2e/shell.a11y.ts` | `S6b_notice_is_spoken_by_orca` **diventa positiva** | dopo «Duplicate» Orca dice il testo dell'avviso **esattamente una volta** entro 4 s; il fuoco resta su «More actions» (**S18**) |
  | | `S6c_the_announcement_comes_from_the_app` | nel file di debug di Orca c'è un `object:announcement` la cui sorgente è la finestra «Galleria» di `galleria-gui` (**S20**: la forma della riga si legge sull'uscita vera; se in Orca 46.1 non c'è, la prova la registra e asserisce solo il parlato) |
  | | `control_orca_does_not_say_any_form_of_live_region` (di B2) | resta com'è e resta verde |

- [ ] **2.** Falliscono: il comando non esiste; `S6b` oggi aspetta il silenzio.
- [ ] **3. Implementa**, in quest'ordine, annotando gli esiti in `docs/08-esito-fase2.md`, sezione «B3»:
  1. `announce.rs` e le prove pure.
  2. `gtk` nel `Cargo.toml`; `cargo deny check` sull'host; la mezza riga in `deny.toml`. Se `cargo deny` segnala
     qualcosa che la tabella di questo piano non prevede: **stop** e si riporta.
  3. `announce_atk.rs`, comando, permessi.
  4. `Toast` e `app/announce.ts`.
  5. `scripts/gui.sh a11y`: `S6b`, `S6c`, e la trascrizione nell'esito.
- [ ] **4.** I due gruppi di comandi, poi `scripts/gui.sh a11y` **due volte di fila**.
- [ ] **5. Commit:** `announce notices to the screen reader through the atk signal`

**Tetto e ripieghi** (non toccano Q27, che è chiusa):

1. Se Orca non pronuncia l'annuncio dopo un'azione perché il parlato del fuoco lo copre (S18): si emette dopo
   200 ms, poi dopo 500 ms. Un'ora al più. Il ritardo scelto si scrive nell'esito con le due trascrizioni.
2. Se l'annuncio non arriva affatto nell'immagine, mentre `lab/vm/docker-announce.sh` continua a funzionare: **stop**.
   Si committa ciò che è verde (modulo puro, comando, `Toast`), `S6b` resta la prova del silenzio, e la card si blocca
   con le due trascrizioni: il difetto è nel modo in cui emettiamo, non nella decisione.

**Fatto quando:** `S6b` è positiva due volte di fila; le prove di controllo di B2 sono verdi; `cargo deny check` è
verde sull'host e nel job `gui`; S18–S20 hanno un esito scritto.

---

## Gruppo B · Rubrica in lettura

### Task 3: comandi di lettura e guscio sui profili veri

**Stima:** 6 h. **File — crea:** `crates/galleria-profile/src/settings.rs`, `crates/galleria-profile/tests/settings.rs`,
`crates/galleria-gui/src/rubrica.rs`, `src/commands/profiles.rs`, `tests/rubrica.rs`, `examples/seed-rubrica.rs`,
`ui/src/rubrica/{store,types}.ts` con le prove. **Modifica:** `crates/galleria-profile/src/{lib.rs,store.rs}`,
`crates/galleria-gui/Cargo.toml` (`galleria-profile`), `src/error.rs`, `src/lib.rs`, `build.rs`,
`capabilities/main.json`, `tests/config.rs`, `ui/src/shell/{Shell,ProfileList,ProfileDetail}.tsx` e le prove,
`ui/src/shell/boundary.test.ts`, `ui/src/i18n/{en,it}.json`, `ui/e2e/*`, `lab/tests/gui-lib.sh`, `scripts/gui.sh`
(costruisce anche `seed-rubrica`). **Elimina:** `ui/src/shell/demo-data.ts`.

**Produce**, in `galleria-profile` (solo aggiunte, Q29 e Q30):

```rust
// settings.rs — galleria.toml
#[derive(Serialize, Deserialize, Clone, Debug)] #[serde(deny_unknown_fields)]
pub struct RubricaSettings { pub schema_version: u32, #[serde(default)] pub groups: Vec<String> }
impl ProfileStore {
    pub fn settings(&self) -> Result<RubricaSettings, StoreError>;          // «schema_version = 1» da solo resta valido
    pub fn list_lenient(&self) -> Result<(Vec<Profile>, Vec<Skipped>), StoreError>;
}
pub struct Skipped { pub file_name: String, pub error: StoreError }
```

In `galleria-gui`:

```rust
// rubrica.rs — nessun tipo di tauri; tutte le viste sono Serialize e senza segreti
pub struct ProfileSummary { pub id: Uuid, pub name: String, pub protocol: Protocol, pub group: Option<String>,
    pub favorite: bool, pub server: Option<String>, pub last_used: Option<String>, pub modified: String }
pub struct ProfileDetail { #[serde(flatten)] pub summary: ProfileSummary, pub notes: String, pub tags: Vec<String>,
    pub created: String, pub settings: Settings, pub secret_refs: Vec<String>,
    pub openvpn: Option<OpenVpnView>, pub wireguard: Option<WireGuardProfile> }
pub struct OpenVpnView { pub config: String, pub remotes: Vec<Remote>, pub auth: AuthFlags }
pub struct Remote { pub host: String, pub port: Option<u16>, pub transport: Option<String> }    // dalle direttive `remote` e `proto`
pub struct AuthFlags { pub certificate: bool, pub user_pass: bool, pub otp: bool }              // da `auth-user-pass`, `static-challenge`, blocchi e secret_refs
pub struct RubricaView { pub profiles: Vec<ProfileSummary>, pub groups: Vec<String>, pub skipped: Vec<SkippedView> }
pub struct Rubrica { /* cartella dei profili e file di stato */ }
impl Rubrica {
    pub fn open_default() -> Result<Self, RubricaError>;        // ProfileStore::default_dir() e UiStateFile::default_path()
    pub fn list(&self) -> Result<RubricaView, RubricaError>;
    pub fn get(&self, id: Uuid) -> Result<ProfileDetail, RubricaError>;
}
```

`server` è il primo `remote` di OpenVPN o l'`endpoint` del primo peer di WireGuard. `WireGuardProfile` non contiene
chiavi private né condivise **[V: `model.rs`; stanno nell'archivio dei segreti]**.

Comandi: `profiles_list() -> RubricaView`, `profile_get(id) -> ProfileDetail`. Codici nuovi: `gui_store_insecure`
(permessi o proprietario sbagliati; `data`: percorso e modo, con il comando `chmod` nel testo), `gui_store_unreadable`,
`gui_profile_not_found`. Un profilo saltato porta un codice suo: `gui_profile_newer` o `gui_profile_invalid`.

`examples/seed-rubrica.rs`: scrive in `XDG_CONFIG_HOME` una rubrica di prova con le funzioni di import della fase 1
(`import_openvpn`, `import_wireguard`) su testi inventati: cinque profili con i nomi di B2 (Ufficio Napoli,
Datacenter Milano, Homelab, VPS Hetzner, Cliente Rossi), tre gruppi, due preferiti, segreti finti riconoscibili
(`SEED-SECRET-…`) che le prove di R3 cercano. Opzione `--empty` per la rubrica vuota.

Webview: `rubrica/store.ts` carica la lista all'avvio, quando la finestra torna in primo piano e dopo ogni propria
scrittura (spec §6); tiene il profilo scelto. Il guscio mostra: la lista vera; il dettaglio con l'intestazione e la
scheda «Generale» **in sola lettura**; le altre schede con `pane.arrives_later`; le tre azioni con
`action.arrives_later` (P10). Stati nuovi: rubrica vuota (testo e il comando `galleria import <file>` da copiare,
fino a B4); profili saltati (avviso in linea con i nomi dei file); archivio illeggibile (avviso d'errore con il
codice tradotto e il comando da copiare).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `galleria-profile/tests/settings.rs` | `a_fresh_store_has_no_groups` · `groups_are_read_in_order` · `unknown_keys_are_rejected` · `newer_schema_is_reported` | — |
  | | `list_lenient_skips_a_broken_profile_and_keeps_the_others` | un file di schema 99 e uno con TOML rotto: tre profili buoni tornano, due saltati con il loro errore |
  | | `list_lenient_still_fails_on_insecure_permissions_of_the_folder` | l'errore di permessi non si salta |
  | `tests/rubrica.rs` | `lists_what_the_cli_import_wrote` | profili scritti con `import_*` e `ProfileStore::save` |
  | | `server_comes_from_the_first_remote_or_endpoint` | parametrica: `remote host 1194 udp`, più `remote`, blocco `<connection>`, WireGuard senza endpoint |
  | | `auth_flags_follow_the_directives` | parametrica |
  | | `no_read_command_returns_a_secret` | il JSON di `profiles_list` e di `profile_get` per ogni profilo non contiene nessun `SEED-SECRET-` (R3) |
  | | `store_errors_map_to_gui_codes` | `0644` sulla cartella → `gui_store_insecure` con percorso e modo |
  | | `works_without_any_daemon` | nessun socket: i comandi rispondono |
  | `rubrica/store.test.ts` | `loads_on_start_and_on_window_focus` · `keeps_the_selection_when_the_list_reloads` · `selection_moves_when_the_selected_profile_disappears` | — |
  | `Shell.test.tsx` | `shows_the_real_profiles` · `empty_rubrica_explains_how_to_import` · `skipped_profiles_are_listed_by_file_name` · `store_error_shows_the_translated_code` | — |
  | | `demo_banner_is_gone` | nessun testo `demo.*` nei cataloghi né nel documento |
  | `shell/boundary.test.ts` | `shell_only_calls_the_listed_commands` | sostituisce `shell_never_reaches_the_daemon`: i comandi invocati da `ui/src` sono quelli di `capabilities/main.json` |
  | `e2e/smoke.e2e.ts` | `rubrica_lists_the_seeded_profiles` | cinque profili, tre gruppi; a demone fermo la lista c'è lo stesso |
  | | `empty_rubrica_shows_the_import_hint` | con `seed-rubrica --empty` |

  Le prove di B2 sul guscio (`shell_is_navigable_with_sample_data`, `shell_works_in_both_languages`, `S1`–`S6`)
  passano alla rubrica di prova: cambiano i dati di partenza e, dove serve, il testo dell'avviso
  (`action.arrives_later`), non le asserzioni.

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I due gruppi di comandi.
- [ ] **5. Commit:** `show the real profiles in the list and the detail`

**Fatto quando:** `grep -rn "demo" ui/src` non trova nulla; `scripts/check-boundaries.sh` è verde (`galleriad` non
dipende da `galleria-profile`); `cargo tree -p galleria-gui -e normal` non contiene `galleriad` né un backend.

---

### Task 4: ricerca, preferiti, ordinamento, stato sulla riga

**Stima:** 5 h. **File — crea:** `ui/src/rubrica/arrange.ts` e la prova, `ui/src/shell/{SearchBox,SortControl}.tsx`
con `.css` e `.test.tsx`. **Modifica:** `ui/src/shell/{Shell,ProfileList}.tsx` e le prove, `ui/src/shell/keyboard.test.tsx`,
`ui/src/i18n/{en,it}.json`, `ui/e2e/smoke.e2e.ts`.

**Produce:**

```ts
// rubrica/arrange.ts — puro
export interface Arranged { groups: { title: string | null; profiles: ProfileSummary[] }[]; total: number }
export function arrange(input: { profiles: ProfileSummary[]; groups: string[]; query: string; sort: "name" | "recent" }): Arranged;
```

Regole: i gruppi nell'ordine di `galleria.toml`; poi quelli che compaiono solo sui profili, in ordine alfabetico; in
fondo «Senza gruppo» (`title: null`). Un gruppo dell'elenco senza profili si vede solo quando la ricerca è vuota.
Dentro il gruppo: preferiti prima (P5), poi per nome, oppure per ultimo uso decrescente e poi per nome (P6);
confronto con `Intl.Collator` della lingua corrente. La ricerca guarda nome, server e gruppo, senza distinguere
maiuscole e accenti; più parole devono comparire tutte.

- `SearchBox`: campo con etichetta visibile solo allo screen reader («Cerca connessioni»), `Ctrl+K` porta il fuoco
  lì da qualunque punto, `Esc` lo svuota, `↓` passa alla lista. Sotto, il conto dei risultati in un elemento
  `role="status"`: è testo visibile e, per Q27, **non** è annunciato (limite dichiarato: regola 3 della spec §12.1).
- `SortControl`: `Segmented` «Nome · Recenti», salvato con `ui_state_set`.
- Stato sulla riga: se `daemon_view().status.profile.id` è un profilo della rubrica e lo stato non è `disconnected`
  **[V: `Status.profile` è un `Option<ProfileRef>` in `galleria-ipc/src/types.rs`]**, la riga mostra l'etichetta
  dello stato (`state.<s>`) come testo, non solo un colore, e la porta nel nome accessibile. In B3 nessuno connette
  dalla GUI: lo stato arriva da una connessione avviata dalla CLI.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `arrange.test.ts` | `groups_follow_the_saved_order_then_the_unknown_ones_then_no_group` | — |
  | | `favourites_come_first_inside_their_group` | nei due ordinamenti |
  | | `recent_order_uses_last_used_and_falls_back_to_the_name` | — |
  | | `search_ignores_case_and_accents_and_needs_every_word` | «citta mi» trova «Città Milano» |
  | | `search_looks_at_name_server_and_group` | — |
  | | `empty_groups_show_only_without_a_query` | — |
  | | `names_sort_with_the_collator_of_the_language` | — |
  | `SearchBox.test.tsx` | `ctrl_k_focuses_the_field` · `escape_clears_it` · `arrow_down_moves_to_the_list` · `result_count_is_a_status_and_is_not_announced` · `has_no_accessibility_violations` | — |
  | `SortControl.test.tsx` | `choice_is_saved_through_ui_state_set` · `starts_from_the_saved_order` | — |
  | `ProfileList.test.tsx` | `active_session_row_says_its_state_in_words` · `no_state_is_shown_for_a_foreign_session` | la sessione estranea è B5 (D21) |
  | | `no_match_says_so` | lista vuota con il testo `search.no_match`; la `listbox` resta nel documento |
  | `keyboard.test.tsx` | `tab_order_follows_the_layout` (di B2) | ordine nuovo: «Menu» → ricerca → ordinamento → lista → … |
  | `e2e/smoke.e2e.ts` | `search_filters_the_list_and_sort_order_survives_a_restart` | — |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I due gruppi di comandi.
- [ ] **5. Commit:** `add search, favourites first, sort order and the session state to the list`

**Fatto quando:** `arrange.ts` non importa nulla di React né di Tauri.

---

## Gruppo C · Modifica

### Task 5: schede Generale, Autenticazione e Rete; `profile_update` (D26)

**Stima:** 7 h. **File — crea:** `ui/src/shell/tabs/{General,Authentication,Network}.tsx`, `ui/src/shell/SaveBar.tsx`,
`ui/src/rubrica/validate.ts`, con `.css` e prove; `ui/e2e/rubrica.a11y.ts`. **Modifica:**
`crates/galleria-gui/src/rubrica.rs`, `src/commands/profiles.rs`, `src/error.rs`, `tests/rubrica.rs`, `Cargo.toml`
(`galleria-backend-wireguard` tra le dipendenze **di sola prova**), `build.rs`, `capabilities/main.json`,
`tests/config.rs`, `ui/src/shell/ProfileDetail.tsx`, `ui/src/rubrica/store.ts`, cataloghi.

**Produce:**

```rust
#[derive(Deserialize)] #[serde(deny_unknown_fields)]
pub struct ProfilePatch {
    pub name: Option<String>, pub group: Option<Option<String>>,        // Some(None) = nessun gruppo
    pub favorite: Option<bool>, pub notes: Option<String>, pub settings: Option<Settings>,
    pub wireguard: Option<WireGuardProfile>,                            // solo per i profili WireGuard
    pub openvpn_config: Option<String>,                                 // rifiutato fino al T7
}
impl Rubrica {
    pub fn update(&self, id: Uuid, expected_modified: &str, patch: ProfilePatch) -> Result<ProfileDetail, RubricaError>;
}
pub fn check_wireguard(p: &WireGuardProfile) -> Vec<FieldIssue>;        // FieldIssue { field: "peers.0.endpoint", code }
```

Comando: `profile_update(id, expected_modified, patch) -> ProfileDetail`. Codici nuovi: `gui_profile_changed`,
`gui_invalid_field` (`data`: elenco di `{ field, code }`).

Cosa si modifica, per scheda (D26, spec §16 punti 1–2):

| Scheda | WireGuard | OpenVPN |
|---|---|---|
| Generale | nome, gruppo, note, stella, `auto_reconnect`; endpoint di ogni peer | nome, gruppo, note, stella, `auto_reconnect`; server e trasporto **in sola lettura** (da `remotes`) |
| Autenticazione | chiave pubblica di ogni peer (campo); «Chiave privata: nell'archivio», mai mostrata | «Metodo» come etichetta (da `AuthFlags`); elenco dei segreti presenti per nome |
| Rete | indirizzi, DNS, domini di ricerca, MTU, reti instradate e keepalive per peer; «Tutto il traffico nel tunnel» come indicazione (`0.0.0.0/0` o `::/0` tra le reti); `ipv6_block` | `ignore_pushed_routes`, `ipv6_block`; DNS e rotte effettivi: «Disponibili durante la connessione» (li mostra B5 da `Status.net`) |

Niente kill switch, «Connetti all'avvio», «Protezione DNS leak», selettore del metodo (spec §16). «Dimentica» è B5.

Comportamento (P7, Q27):

- Interruttori e stella scrivono subito; se la scrittura fallisce il controllo torna com'era e compare un avviso
  d'errore (annuncio *assertive*). Nessun avviso quando riesce.
- I campi di testo cambiano una copia. Con una differenza compare `SaveBar` («Salva», «Annulla»; `Ctrl+S` salva).
  «Salva» riuscito: avviso `profile.saved` (annuncio *polite*) e fuoco che resta dov'è.
- **Errore di validazione** (dalla webview o da `gui_invalid_field`): è un errore che chiede una risposta, quindi
  **il fuoco va sul primo campo sbagliato**, il cui errore è collegato con `aria-describedby` e viene letto con il
  campo. Nessun avviso a scomparsa.
- `gui_profile_changed`: avviso in linea nel dettaglio con «Ricarica» e annuncio *assertive* (non chiede una
  risposta immediata e la copia in modifica non si butta da sola).
- Cambiare profilo o chiudere con modifiche non salvate: finestra di conferma «Salva · Scarta · Annulla».

`check_wireguard`: indirizzi e reti in forma CIDR, DNS come indirizzi IP, MTU e keepalive negli intervalli del
demone, endpoint `host:porta`, chiave pubblica di 32 byte in base64, almeno un indirizzo e una rete per peer.
**Prima di scriverla si legge `crates/galleria-backend-wireguard/src/validate.rs`**: gli intervalli sono i suoi.
`rubrica/validate.ts` fa gli stessi controlli per dare l'errore mentre si scrive; l'autorità è Rust.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/rubrica.rs` | `update_changes_only_the_patched_fields_and_bumps_modified` | — |
  | | `a_stale_expected_modified_is_refused_and_nothing_is_written` | `gui_profile_changed`; il file ha gli stessi byte |
  | | `openvpn_profile_refuses_a_wireguard_patch_and_the_other_way_round` | — |
  | | `peer_count_cannot_change` | `gui_invalid_field` su `peers` |
  | | `invalid_fields_are_all_reported_with_their_path` | parametrica sulle regole |
  | | `what_the_gui_accepts_the_daemon_accepts` | per una tabella di profili validi: `to_connect_params` + `galleria_backend_wireguard::validate` riesce (P8) |
  | | `what_the_daemon_refuses_the_gui_refuses` | la stessa tabella con un campo rotto alla volta |
  | | `a_profile_saved_by_the_gui_keeps_the_phase_1_format` | il file riletto con `Profile::from_toml` è uguale campo per campo, salvo i campi del `patch` e `modified` (R6) |
  | | `empty_name_and_overlong_name_are_refused` | 1–64 caratteri, senza caratteri di controllo |
  | | `update_never_touches_the_secrets` | il file dei segreti ha gli stessi byte |
  | `tabs/*.test.tsx` | `openvpn_server_and_transport_are_read_only` · `method_is_a_label_not_a_selector` · `no_kill_switch_autostart_or_dns_leak_control` · `full_tunnel_is_an_indication` · `private_key_is_never_rendered` · `has_no_accessibility_violations` | — |
  | `SaveBar.test.tsx` | `appears_only_when_dirty` · `ctrl_s_saves` · `cancel_restores_the_values` | — |
  | `ProfileDetail.test.tsx` | `save_shows_a_notice_and_keeps_the_focus` | — |
  | | `a_validation_error_moves_the_focus_to_the_first_bad_field_and_shows_no_notice` | Q27, regola 2 |
  | | `switches_write_at_once_and_revert_on_failure` | — |
  | | `leaving_with_unsaved_changes_asks_first` | — |
  | | `a_changed_profile_offers_reload_and_keeps_the_draft` | — |
  | `e2e/rubrica.a11y.ts` | `R4a_orca_says_saved_once` | nome cambiato, `Ctrl+S`: Orca dice il testo di `profile.saved` una volta |
  | | `R4d_orca_says_an_error_as_assertive` | la prova riscrive il file del profilo dal lato Node (come farebbe la CLI), poi un interruttore: Orca dice il testo di `gui_profile_changed`; l'urgenza 2 si asserisce sulla riga `object:announcement` se S20 l'ha trovata, altrimenti si registra |
  | | `R5a_field_error_is_read_with_the_field` | MTU non valido, «Salva»: il fuoco è sul campo e Orca dice il testo dell'errore |
  | `e2e/smoke.e2e.ts` | `a_profile_is_renamed_and_the_list_follows` | — |

- [ ] **2.** Falliscono. **3.** Implementa.
- [ ] **4.** I due gruppi di comandi, poi `scripts/gui.sh a11y`.
- [ ] **5. Commit:** `edit profiles from the general, authentication and network tabs`

**Fatto quando:** nessun campo che la spec §16 toglie è nel documento; `cargo tree -p galleria-gui -e normal` non
contiene `galleria-backend-wireguard`.

---

### Task 6: gruppi in `galleria.toml`

**Stima:** 5 h. **File — crea:** `crates/galleria-gui/src/commands/groups.rs`, `tests/groups.rs`,
`ui/src/shell/GroupsDialog.tsx` con `.css` e `.test.tsx`. **Modifica:** `crates/galleria-profile/src/settings.rs` e
`tests/settings.rs`, `crates/galleria-gui/src/rubrica.rs`, `src/error.rs`, `build.rs`, `capabilities/main.json`,
`tests/config.rs`, `ui/src/shell/{Shell,ProfileList}.tsx`, `ui/src/shell/tabs/General.tsx`, cataloghi, `ui/e2e/smoke.e2e.ts`.

**Produce:** in `galleria-profile`, `ProfileStore::save_settings(&RubricaSettings) -> Result<(), StoreError>` (rename
atomico, `0600`; rifiuta uno schema più nuovo). In `galleria-gui`:

```rust
impl Rubrica {
    pub fn group_create(&self, name: &str) -> Result<Vec<String>, RubricaError>;
    pub fn group_rename(&self, from: &str, to: &str) -> Result<Vec<String>, RubricaError>;   // prima i profili, poi l'elenco
    pub fn group_delete(&self, name: &str) -> Result<Vec<String>, RubricaError>;             // i profili restano, senza gruppo
    pub fn groups_reorder(&self, names: &[String]) -> Result<Vec<String>, RubricaError>;     // deve essere una permutazione
}
```

Quattro comandi con gli stessi nomi; ognuno restituisce l'elenco nuovo. Regole del nome: spazi tolti ai lati, 1–48
caratteri, niente caratteri di controllo, unico senza distinguere le maiuscole. Codici nuovi: `gui_group_invalid`,
`gui_group_exists`, `gui_group_not_found`.

Un gruppo è un nome (Q30): l'appartenenza resta `Profile.group`, `galleria.toml` tiene l'ordine e i gruppi vuoti.
Un gruppo che la CLI o un file scritto a mano mettono su un profilo senza metterlo nell'elenco si vede lo stesso
(T4) e la finestra offre «Aggiungi all'elenco».

`GroupsDialog` (P9): «Gestisci gruppi» in fondo alla lista apre una finestra modale con una riga per gruppo (campo
del nome, «Sposta su», «Sposta giù», «Elimina») e in fondo «Nuovo gruppo». «Elimina» su un gruppo con profili chiede
conferma e dice quanti profili restano senza gruppo. Dopo uno spostamento il fuoco segue la riga spostata. Esiti:
avviso (annuncio *polite*) per creato, rinominato, eliminato; nessun avviso per lo spostamento (il fuoco che segue
la riga basta). Il selettore «Gruppo» della scheda Generale elenca i gruppi dell'elenco più «Nessun gruppo».

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `galleria-profile/tests/settings.rs` | `save_settings_is_atomic_and_private` · `save_refuses_a_newer_schema` · `the_cli_store_still_opens_a_folder_with_groups` | `ProfileStore::open` e `list` invariati |
  | `tests/groups.rs` | `create_appends_and_refuses_duplicates_ignoring_case` | — |
  | | `rename_moves_every_member_and_keeps_the_position` | — |
  | | `rename_is_resumable_after_a_failure` | un profilo reso non scrivibile a metà: errore; tolto l'ostacolo, la stessa chiamata finisce il lavoro |
  | | `delete_leaves_the_profiles_without_a_group` | — |
  | | `reorder_needs_a_permutation` | — |
  | | `names_are_trimmed_and_checked` | parametrica |
  | | `group_commands_never_touch_the_secrets` | — |
  | `GroupsDialog.test.tsx` | `lists_groups_with_their_controls` · `new_group_appears_at_the_end_and_gets_the_focus` · `move_keeps_the_focus_on_the_moved_row` · `delete_with_members_asks_and_says_how_many` · `a_duplicate_name_puts_the_focus_on_the_field_with_the_error` · `escape_closes_and_returns_focus_to_the_opener` · `has_no_accessibility_violations` | — |
  | `General.test.tsx` | `group_select_lists_the_saved_groups_and_none` | — |
  | `e2e/smoke.e2e.ts` | `a_group_is_created_renamed_and_survives_a_restart` | — |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I due gruppi di comandi.
- [ ] **5. Commit:** `manage groups stored in galleria.toml`

**Fatto quando:** `galleria list` (CLI della fase 1, sull'host) legge senza errori una cartella in cui la GUI ha
creato e rinominato gruppi: provato una volta a mano e annotato; la prova automatica è nel T9.

---

## Gruppo D · Editor, azioni, chiusura

### Task 7: editor raw con validazione

**Stima:** 5 h. **Scioglie:** S24. **File — crea:** `crates/galleria-profile/tests/raw.rs`,
`crates/galleria-gui/tests/raw.rs`, `ui/src/shell/tabs/Configuration.tsx` con `.css` e `.test.tsx`. **Modifica:**
`crates/galleria-profile/src/lib.rs`, `crates/galleria-gui/src/rubrica.rs`, `src/commands/profiles.rs`,
`src/error.rs`, `Cargo.toml` (`galleria-backend-openvpn` tra le dipendenze di sola prova), `build.rs`,
`capabilities/main.json`, `tests/config.rs`, `ui/src/shell/ProfileDetail.tsx`, cataloghi, `ui/e2e/smoke.e2e.ts`.

**Produce**, in `galleria-profile` (solo aggiunte):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigIssueKind { Syntax, Dangerous, Unsupported, DaemonOwned, FileReference, SecretBlock, NoTlsClient }
pub struct ConfigIssue { pub line: usize, pub directive: String, pub kind: ConfigIssueKind }
/// La classificazione della fase 1 (`galleria_ovpn::classify`, `check_strict`), riga per riga, senza leggere file.
pub fn check_openvpn_config(text: &str) -> Vec<ConfigIssue>;
/// Il `.conf` di un profilo WireGuard; senza `keys` le righe PrivateKey e PresharedKey non ci sono.
pub struct WireGuardKeys { pub private_key: Secret, pub preshared: Vec<Option<Secret>> }      // uno per peer, nell'ordine dei peer
pub fn render_wireguard(p: &WireGuardProfile, keys: Option<&WireGuardKeys>) -> String;
```

In `galleria-gui`: comando `profile_validate_raw(id, text) -> { issues: [{ line, directive, kind }], can_save }`, e
`profile_update` accetta `openvpn_config`. Regole (Q32):

- **OpenVPN.** Si salva solo se non resta nessun problema che blocca: tutto tranne `NoTlsClient`, che è un avviso.
  Nulla viene tolto in silenzio: ciò che è nell'editor è ciò che finisce nel file. Una direttiva che lancia script
  ha il testo di D19 («Galleria non esegue gli script dei profili»). Un blocco segreto (`<key>`, `<tls-crypt>`…) e
  un riferimento a un file (`ca ca.crt`) sono rifiutati con un testo che dice di reimportare il profilo: chiavi e
  file entrano solo dall'import (B4). Rust rivalida sempre al salvataggio, qualunque cosa abbia detto prima alla
  webview. Il testo si salva com'è scritto, commenti compresi (**S24**).
- **WireGuard.** La scheda mostra `render_wireguard(profilo, None)` in sola lettura: tutti i suoi campi si
  modificano dalle schede.
- Sotto l'editor, per tutti e due: «Nell'archivio dei segreti: …» con i nomi di `secret_refs` tradotti (spec §16
  punto 3). Nessun segnaposto dentro il testo.

`Configuration.tsx`: `Field as="textarea" mono`, validazione 400 ms dopo l'ultima modifica e sempre prima di
salvare; sotto, l'elenco dei problemi (`riga N · direttiva · testo`), ogni voce un pulsante che porta il cursore su
quella riga. Stessa `SaveBar` del T5. Salvataggio rifiutato: il fuoco va sul primo problema dell'elenco (Q27,
regola 2). Salvataggio riuscito: avviso `profile.saved`.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `galleria-profile/tests/raw.rs` | `a_config_written_by_the_import_has_no_issue` | per i profili di prova della fase 1 |
  | | `each_class_of_directive_gets_its_kind_and_line` | parametrica: `up /x.sh` → `Dangerous`; `dev tun` → `DaemonOwned`; una direttiva inventata → `Unsupported`; `ca ca.crt` → `FileReference`; `<key>` → `SecretBlock`; virgolette non chiuse → `Syntax` |
  | | `missing_tls_client_is_a_warning_at_line_0` | — |
  | | `render_wireguard_without_keys_has_no_key_line` · `render_wireguard_round_trips_through_the_import` | con le chiavi: `import_wireguard(render(...))` dà lo stesso profilo |
  | `tests/raw.rs` | `validate_reports_and_never_writes` | — |
  | | `update_refuses_a_config_with_a_blocking_issue` | `gui_invalid_field`, campo `openvpn_config`, con i problemi in `data` |
  | | `update_revalidates_even_after_a_clean_validate` | — |
  | | `a_secret_typed_in_the_editor_is_not_stored_and_not_echoed` | il `<key>` rifiutato non compare nel file, in `data` né nei log |
  | | `saved_raw_config_is_accepted_by_the_daemon_validator` | `to_connect_params` + `galleria_backend_openvpn::validate`, con un testo che ha commenti e righe vuote (**S24**) |
  | | `wireguard_profile_refuses_openvpn_config` | — |
  | `Configuration.test.tsx` | `issues_are_listed_with_line_and_directive` · `an_issue_button_moves_the_caret_to_its_line` · `save_is_blocked_while_a_blocking_issue_remains` · `a_refused_save_moves_the_focus_to_the_first_issue` · `script_directive_shows_the_d19_text` · `wireguard_config_is_read_only_and_has_no_key` · `stored_secrets_are_named_below_the_editor` · `has_no_accessibility_violations` | — |
  | `e2e/smoke.e2e.ts` | `a_script_directive_cannot_be_saved_from_the_editor` | si scrive `up /tmp/x.sh`, «Salva»: il file sul disco non cambia |

- [ ] **2.** Falliscono. **3.** Implementa. **S24**: se il validatore del demone rifiuta il testo com'è scritto, si
  salva `render(parse(testo))` e l'esito dice che commenti e allineamento si perdono al salvataggio.
- [ ] **4.** I due gruppi di comandi.
- [ ] **5. Commit:** `add the raw configuration editor with the phase 1 directive checks`

**Fatto quando:** `cargo tree -p galleria-gui -e normal` non contiene `galleria-backend-openvpn`; `galleria-gui`
non nomina `galleria-ovpn` nel suo `Cargo.toml`.

---

### Task 8: duplica, esporta, elimina

**Stima:** 6 h. **Scioglie:** S22. **File — crea:** `crates/galleria-gui/src/export.rs`, `tests/export.rs`,
`crates/galleria-profile/tests/export.rs`. **Modifica:** `crates/galleria-profile/src/lib.rs`,
`crates/galleria-gui/src/rubrica.rs`, `src/commands/profiles.rs`, `src/error.rs`, `src/lib.rs`, `Cargo.toml`
(`tauri-plugin-dialog`), `build.rs`, `capabilities/main.json`, `tests/config.rs`, `ui/src/shell/ProfileDetail.tsx` e
la prova, `ui/src/rubrica/store.ts`, cataloghi, `ui/e2e/{smoke.e2e.ts,shell.a11y.ts,rubrica.a11y.ts}`.

**Produce**, in `galleria-profile`: `pub fn export_config(p: &Profile, secrets: &dyn SecretStore) -> Result<Zeroizing<String>, StoreError>`
(OpenVPN: la configurazione con i blocchi segreti rimessi in linea; WireGuard: `render_wireguard` con le chiavi). In
`galleria-gui`:

```rust
// export.rs — nessun tipo di tauri
pub trait SavePicker: Send + Sync { fn pick(&self, suggested_name: &str) -> Option<PathBuf>; }   // None = annullato
pub fn write_export(path: &Path, text: &str) -> Result<(), ExportError>;                         // rename atomico, 0600
impl Rubrica {
    pub fn duplicate(&self, id: Uuid, name: &str) -> Result<ProfileSummary, RubricaError>;       // nuovo id, segreti copiati in Rust
    pub fn export(&self, id: Uuid, picker: &dyn SavePicker) -> Result<bool, RubricaError>;       // false = annullato
    pub fn delete(&self, id: Uuid, active: Option<&ProfileRef>) -> Result<(), RubricaError>;
}
```

Comandi: `profile_duplicate(id, name) -> ProfileSummary`, `profile_export(id) -> { saved: bool }`,
`profile_delete(id) -> ()`. Codici nuovi: `gui_profile_in_use`, `gui_export_failed`.

- **Duplica.** Il nome lo propone la webview dal catalogo («X (copia)», poi «X (copia 2)») e Rust lo valida come nel
  T5. Copia il profilo con un id nuovo, `created` e `modified` di adesso, non preferito, e **tutti i segreti** di
  `secret_refs` leggendoli e scrivendoli in Rust. Se la copia di un segreto fallisce, il profilo nuovo viene tolto:
  o tutto o niente. Esito: avviso «Duplicato come «X (copia)»» (annuncio *polite*), la selezione passa al profilo
  nuovo, il fuoco resta su «Altre azioni».
- **Esporta** (Q31). Finestra di conferma nella webview: «Il file conterrà le chiavi private di questo profilo».
  Poi Rust apre «Salva» con il nome proposto (`<nome>.ovpn` o `<nome>.conf`, senza `/` né caratteri di controllo) e
  scrive il file `0600`. Il testo non passa mai dalla webview. Annullato: nessun avviso. Riuscito: avviso
  «Esportato» senza il percorso. `SavePicker` vero: `tauri-plugin-dialog` chiamato da Rust (**S22**).
- **Elimina.** Conferma (pericolosa, fuoco su «Annulla», come in B2). Rust rifiuta con `gui_profile_in_use` se quel
  profilo è la sessione attiva del demone (stato diverso da `disconnected` nella fotografia del `supervisor`); la
  webview lo sa già e dice perché la voce è spenta. `ProfileStore::delete` toglie segreti e profilo **[V:
  `store.rs`]**; poi `UiStateFile::forget`. Esito: avviso «Eliminato «X»» (annuncio *polite*); il fuoco va sulla
  riga della lista che prende il posto di quella eliminata, o sul testo della rubrica vuota.
- Gli errori dei tre comandi sono avvisi con tono `error` (annuncio *assertive*): non chiedono una risposta.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `galleria-profile/tests/export.rs` | `openvpn_export_round_trips_through_the_import` · `wireguard_export_round_trips_through_the_import` | stessi campi e stessi segreti dopo `import_*` del testo esportato |
  | | `export_fails_when_a_referenced_secret_is_missing` | nessun file a metà |
  | `tests/export.rs` | `duplicate_copies_profile_and_secrets_under_a_new_id` | — |
  | | `duplicate_is_all_or_nothing` | archivio dei segreti reso non scrivibile: nessun profilo nuovo resta |
  | | `duplicate_returns_no_secret` | R3 |
  | | `export_writes_a_private_file_at_the_picked_path` | `0600`; con un `SavePicker` finto |
  | | `export_cancelled_writes_nothing_and_is_not_an_error` | — |
  | | `export_never_sends_the_content_to_the_webview` | il JSON della risposta è `{"saved":true}` e basta |
  | | `suggested_name_is_made_safe` | parametrica: `a/b`, `..`, nome vuoto, 300 caratteri |
  | | `delete_removes_profile_secrets_and_last_used` | — |
  | | `delete_refuses_the_profile_of_the_active_session` | con il demone finto in `connected` |
  | | `no_command_takes_a_path` | nessun parametro dei comandi in `COMMANDS` ha tipo `PathBuf` o si chiama `path`/`file` (letto dai sorgenti) |
  | `tests/config.rs` | `main_capability_grants_only_listed_permissions` | nessun permesso `dialog:*` |
  | `ProfileDetail.test.tsx` | `duplicate_selects_the_copy_and_keeps_the_focus_on_the_menu` · `export_asks_first_and_says_the_file_holds_the_keys` · `export_cancelled_shows_nothing` · `delete_asks_then_moves_the_focus_to_the_list` · `delete_is_disabled_with_a_reason_for_the_active_session` · `a_failed_action_shows_an_error_notice` | — |
  | `e2e/shell.a11y.ts` | `S6`, `S6b`, `S6c` | passano al «Duplicate» vero: testo «Duplicated as …» |
  | `e2e/rubrica.a11y.ts` | `R4b_orca_says_duplicated_once` · `R4c_orca_says_deleted_once_and_the_focus_is_on_the_list` | — |
  | `e2e/smoke.e2e.ts` | `a_profile_is_duplicated_and_deleted` | i file sul disco compaiono e spariscono, segreti compresi |

- [ ] **2.** Falliscono. **3.** Implementa. **S22**: prima di scrivere codice, versione e licenza di
  `tauri-plugin-dialog` dal registro e la sua API Rust dalla versione fissata; se chiede un permesso alla webview, se
  la licenza non è in elenco o se la finestra non si apre nell'immagine, il `SavePicker` vero diventa
  `gtk::FileChooserNative`, con la dipendenza `gtk` che c'è già dal T2. Quale dei due si usa va nell'esito.
- [ ] **4.** I due gruppi di comandi, poi `scripts/gui.sh a11y`.
- [ ] **5. Commit:** `duplicate, export and delete profiles`

**Fatto quando:** nei cataloghi non resta `action.arrives_later`; la finestra «Salva» è stata aperta **una volta a
mano** nell'immagine o su una VM e l'esito dice dove (il fumo non la copre: punto 9 della revisione).

---

### Task 9: tastiera, fumo, screen reader, esito del blocco

**Stima:** 5 h. **File — modifica:** `ui/src/shell/keyboard.test.tsx`, `ui/e2e/{smoke.e2e.ts,shell.a11y.ts,rubrica.a11y.ts}`,
`crates/galleria-gui/tests/rubrica.rs`, `docs/08-esito-fase2.md`, `ROADMAP.md`, `.claude/handoffs/`.

- [ ] **1. Prove che falliscono.**

  `keyboard.test.tsx`, sul guscio intero, senza il puntatore:

  | Prova | Asserzione |
  |---|---|
  | `a_full_rubrica_tour_needs_no_pointer` | ricerca con `Ctrl+K`, scelta di un profilo, modifica e `Ctrl+S`, cambio di gruppo, editor raw, «Gestisci gruppi», duplica, elimina con conferma |
  | `every_interactive_element_is_reachable` (di B2) | esteso ai controlli nuovi |
  | `focus_is_never_lost_to_the_body` (di B2) | esteso: dopo salva, duplica, elimina, chiusura di «Gestisci gruppi» |
  | `no_outcome_depends_on_a_live_region_alone` | ogni chiamata a `useToast` del guscio passa da un `ToastProvider` con `announce` (Q27, regola 4) |

  Livello 2 e fumo:

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/rubrica.rs` | `no_read_command_returns_a_secret` | esteso a tutti i comandi di B3 che restituiscono dati, dopo un giro di modifica, duplica ed esporta |
  | `e2e/smoke.e2e.ts` | `cli_lists_what_the_gui_saved` | dopo il giro del fumo, il binario `galleria` della fase 1 (`galleria list`, costruito nell'immagine) elenca i profili senza errori, con i nomi nuovi |
  | | `rubrica_works_with_the_daemon_stopped` | il giro di lettura e una modifica a demone fermo |
  | | `no_csp_violation_during_the_rubrica_tour` | — |

  `rubrica.a11y.ts`, giro completo con Orca: ricerca (il nome del campo: **registrato, non asserito**, perché
  nell'immagine Orca 46.1 non lo dice, vedi `S4b`), lista con lo stato vuoto e pieno, schede nuove, «Gestisci
  gruppi» (`dialog`, righe e pulsanti pronunciati), editor raw (`entry` multilinea, l'elenco dei problemi), le
  quattro prove `R4a`–`R4d`.

- [ ] **2.** Falliscono. **3.** Correggi ciò che le prove trovano, nei componenti a cui appartiene.
- [ ] **4.** `scripts/gui.sh check`; `scripts/gui.sh smoke` **tre volte di fila**; `scripts/gui.sh a11y` due volte;
  gruppo dell'host.
- [ ] **5. Esito e roadmap.** In `docs/08-esito-fase2.md`, sezione «B3»: tabella R1–R6 con prova ed esito; S18–S24
  con esito e ripiego; la **trascrizione di Orca** del giro (righe `SPEECH OUTPUT`, così come sono) con le quattro
  frasi degli esiti e la riga dell'evento `object:announcement`; numero di prove per livello; il **limite
  dichiarato** di Q27 in un elenco (cosa Orca non dice da solo: barra di stato, conto dei risultati, altri lettori,
  ATK senza il segnale); «Non verificato», che contiene almeno: la voce (nessun audio), GNOME vero con il comando
  definitivo, la finestra «Salva» nel fumo, la dimensione della finestra su Wayland, Orca 46 su un desktop vero
  (Q28). `ROADMAP.md`: le caselle di «B3» secondo i fatti. Handoff in `.claude/handoffs/`.
- [ ] **6. Commit:** `cover the rubrica with keyboard, smoke and screen reader tests`

**Fatto quando:** R1–R6 hanno ciascuno una prova verde, oppure l'esito dice quale non lo è e perché; nessuna casella
della roadmap è spuntata senza la sua prova.

---

## Copertura della spec

| Spec | Dove |
|---|---|
| §4.2 comandi di B3 | T1 (`ui_state_*`), T2 (`announce`), T3 (`profiles_list`, `profile_get`), T5 (`profile_update`), T6 (`group_*`, al posto di `groups_get` e `groups_set`: P4), T7 (`profile_validate_raw`), T8 (`profile_duplicate`, `profile_export`, `profile_delete`) |
| §3.2 regole 2 e 3 (segreti, percorsi) | R3: T3, T8, T9 |
| §5 rubrica consultabile e modificabile senza demone | T3 `works_without_any_daemon`, T9 `rubrica_works_with_the_daemon_stopped` |
| §6 archivio della fase 1, rename atomico, rilettura al ritorno in primo piano | T3 (`store.ts`), T5 (`expected_modified`) |
| §6 schede e campi di D26 | T5; editor raw T7 |
| §6 `state.toml` (D32): ultimo uso, finestra, ordinamento | T1; l'ultimo uso lo scrive B5 (P6); la posizione no (Q34) |
| §12 tema scelto a mano salvato; Q25 lingua e tema | T1 |
| §12.1 annunci (Q27) | T2 (comando e prova), T5, T6, T8 (usi), T9 (limite dichiarato nell'esito) |
| §16 differenze dal mockup, punti 1–3 | T5 (1, 2), T7 (3) |
| G12 per i componenti di B3 | R5: axe e tastiera in ogni task, T9 |
| ROADMAP «icona di stato» sulla lista | T4: testo dello stato sulla riga |

Punti [S] di questo piano (la numerazione continua S1–S17):

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S18 | L'annuncio emesso da un comando subito dopo un'azione dell'utente arriva a Orca e non è coperto dal parlato del fuoco (lo spike emetteva a tempo) | T2 | ritardo di 200 o 500 ms; se non arriva affatto, stop (T2) |
| S19 | `SignalId::lookup` dice se il segnale c'è senza andare in panico, e basta a proteggere un ATK più vecchio di 2.46 | T2, con un nome di segnale inventato; un ATK vecchio vero non c'è nell'immagine | resta «non verificato» su un sistema vecchio, scritto nel limite dichiarato |
| S20 | Il file di debug di Orca 46.1 ha una riga leggibile per `object:announcement`, con la sorgente e l'urgenza | T2 | si asserisce solo il parlato; sorgente e urgenza si provano al livello 2 |
| S21 | Il tema salvato è applicato prima del primo disegno pur arrivando da un comando | T1 | la finestra parte con il tema del sistema per un istante: lo si scrive, non si aggiunge altro |
| S22 | `tauri-plugin-dialog` si usa da Rust senza permessi alla webview, ha una licenza in elenco e apre la finestra nell'immagine | T8 | `gtk::FileChooserNative` |
| S23 | Dimensione e «massimizzata» si leggono e si applicano prima di mostrare la finestra | T1, sotto Xvfb | la dimensione non si ripristina; resta lo stato salvato e lo si scrive |
| S24 | Un testo di configurazione salvato com'è scritto, con i commenti, passa il validatore del demone | T7 | si salva `render(parse(testo))` |

## Cosa resta fuori da B3, per scelta

- Import da file, trascinamento e testo; profilo nuovo da zero: B4. Fino ad allora i profili arrivano dalla CLI.
- Connetti e disconnetti, pallino dello stato, sessione estranea, credenziali, «Dimentica», portachiavi, DNS e rotte
  effettivi, scrittura dell'ultimo uso, scadenza dei certificati (P11): B5.
- Aggiungere o togliere un peer WireGuard; chiavi nuove dall'editor: si reimporta (B4).
- Etichette (`tags`): il campo resta nel profilo e non si mostra.
- Finestra delle impostazioni, tray con i preferiti: B6.
- Annuncio del conto dei risultati e della barra di stato: limite dichiarato (Q27, regola 3).
- Prova di Orca in CI: resta locale (Q22). La voce e GNOME vero: passata di B6.

## Cosa ho verificato e cosa no

Verificato oggi (2026-11-25), in questo worktree:

- Il merge dello spike in `main` (`9dfc366`): un solo conflitto, in `docs/08`, risolto tenendo le due sezioni.
- `cargo deny check` con e senza `gtk = "0.18"` in `galleria-gui`: tabella sopra. `gtk` 0.18.2 e `atk` 0.18.2 sono
  nel `Cargo.lock` e hanno licenza MIT.
- `WebviewWindow::gtk_window()` esiste in `tauri` 2.12.7 e nel runtime finto è `unimplemented!()`;
  `WidgetExt::accessible()` in `gtk` 0.18.2; `emit_by_name` e `SignalId::lookup` in `glib` 0.18.5; i segnali
  `announcement` e `notification` sono dietro `v2_46` e `v2_50` in `atk` 0.18.2. Letti nei sorgenti del registro.
- Nell'immagine `galleria-gui-dev`: Ubuntu 24.04.5, Orca 46.1, `libatk1.0` 2.52.0, `at-spi2-core` 2.52.0.
- Il modello della fase 1: `Profile`, `Settings`, `WireGuardProfile`, `ProfileStore` (`list`, `find`, `save`,
  `delete`), `FileSecretStore`, `import_openvpn`, `import_wireguard`, `check_strict`, `classify`; `galleria.toml` oggi
  contiene solo `schema_version = 1` e nessun codice lo legge; la CLI ha `import`, `list`, `delete` e nessun
  «esporta»; `Status.profile`.
- `ui/e2e/shell.a11y.ts` dopo le correzioni di questa card (commenti e messaggi sui buchi noti): `tsc --noEmit -p
  e2e` verde nell'immagine. **`scripts/gui.sh a11y` non è stato rilanciato**: sono cambiati solo commenti e stringhe.

Non verificato:

- Nessuna riga di codice di B3 è stata scritta o eseguita: moduli, comandi e prove di questo piano sono progetto.
- L'annuncio da un comando dopo un'azione vera (S18): lo spike ha emesso a tempo.
- `cargo deny` nell'immagine e sul runner con `gtk` diretto; il motivo dell'avviso «not encountered» sull'host.
- `tauri-plugin-dialog`: versione, licenza, API Rust (S22). Non è nel `Cargo.lock`.
- Le regole esatte di `galleria-backend-wireguard/src/validate.rs`: so che il file c'è, non l'ho letto riga per riga.
- Che un client Wayland non possa scegliere la propria posizione (Q34): lo ricordo, oggi non l'ho riletto.
- Le parole che Orca usa per i controlli nuovi.

## Questioni aperte nuove

La numerazione continua Q12–Q27. Ognuna ha la mia raccomandazione, che vale come **predefinito per chi implementa**
se nessuno decide prima del task indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q28 | Il nome dei campi di testo che Orca non dice: su GNOME vero con Orca 50 e 51 si sente, nell'immagine con Orca 46.1 no. Orca 46 è quello di Ubuntu 24.04 LTS, che è tra le distribuzioni obbligatorie, e lì su un desktop vero non è stato provato | (a) una card ora: VM Ubuntu 24.04 desktop e il giro `fields` dello spike · (b) insieme alla passata di B6, dove Ubuntu 24.04 è già in elenco per G10 · (c) non provarlo | **(b)**. Nessuna correzione di markup lo cambierebbe (le nove forme di controllo sono mute) e non blocca B3–B5. Fino ad allora sta in «Non verificato». Se in B6 il buco c'è anche sul desktop vero, diventa un limite dichiarato per Ubuntu 24.04 | B6 |
| Q29 | Dove sta il codice nuovo che legge e scrive i file della fase 1 (`galleria.toml`, lista tollerante, controllo della configurazione, testo esportato). I piani di B1 e B2 vietavano di toccare `galleria-profile` | (a) solo aggiunte in `galleria-profile`, con le loro prove; nessuna firma esistente cambia · (b) tutto in `galleria-gui`, copiando le funzioni private di scrittura atomica e controllo dei permessi · (c) `galleria-gui` dipende anche da `galleria-ovpn` | **(a)**. Le regole sui permessi restano in un posto solo, e la CLI potrà usare gruppi ed esportazione senza riscriverli. La spec §9 mette già lì l'archivio dei segreti di B5. Formato dei profili e `SCHEMA_VERSION` non cambiano | T3 |
| Q30 | Come si scrivono i gruppi in `galleria.toml` (D13 dice solo che stanno lì) | (a) `groups = ["Lavoro", "Clienti"]`: nomi in ordine; l'appartenenza resta `Profile.group` · (b) tabelle con un identificativo, e i profili che puntano all'identificativo | **(a)**. Non tocca il formato dei profili e la CLI continua a leggere tutto. Il costo: rinominare un gruppo riscrive i suoi profili, uno alla volta (T6) | T3 |
| Q31 | Cosa contiene il file di «Esporta» | (a) il file nativo (`.ovpn`, `.conf`) **con le chiavi**, dopo una conferma, `0600` · (b) senza chiavi · (c) si sceglie nella finestra | **(a)**. Un `.conf` WireGuard senza chiave privata non serve a niente e un `.ovpn` senza `<key>` non si reimporta. Il file lo scrive Rust e il testo non passa dalla webview. La conferma dice che il file contiene le chiavi. (c) aggiunge una scelta che quasi nessuno capirebbe | T8 |
| Q32 | Quanto è severo l'editor raw | (a) OpenVPN: si salva solo senza problemi; blocchi segreti e riferimenti a file rifiutati («reimporta»); WireGuard in sola lettura · (b) come l'import: toglie e avvisa · (c) anche WireGuard modificabile dal testo | **(a)**. Ciò che si vede è ciò che si salva, e una chiave non entra da una casella di testo. Con (b) una riga sparirebbe dopo «Salva». (c) vorrebbe una chiave privata dentro il testo, o un secondo modo di scrivere gli stessi campi | T7 |
| Q33 | Quando si scrive sul disco | (a) «Salva» e «Annulla» per i campi di testo; subito per interruttori e stella · (b) tutto subito, campo per campo · (c) tutto con «Salva» | **(a)**. Con (b) un nome a metà o un indirizzo incompleto finirebbero nel file, e la CLI li leggerebbe. Con (c) un interruttore chiederebbe due gesti | T5 |
| Q34 | La spec §6 dice «dimensione e posizione della finestra» | (a) solo dimensione e «massimizzata» · (b) anche la posizione | **(a)**. Su Wayland, che è la sessione predefinita di Ubuntu e Fedora, un'app non sceglie dove comparire: la posizione salvata varrebbe solo su X11 e andrebbe controllata contro schermi che non ci sono più. È una riduzione rispetto al testo della spec: per questo la chiedo | T1 |
