# Fase 2 · Blocco B5 — Connessione: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** dalla finestra ci si connette. Archivio dei segreti con il portachiavi e il ripiego sul file (D25);
connetti, disconnetti e sostituisci (D28); sette stati e statistiche dal vivo; credenziali chieste solo su
`needs_input`, con «Ricorda»; errori tradotti; scheda Log con «Esporta log…» (D20); sessione estranea (D21). Chiude
**G4, G5, G6, G7, G9**: da qui la GUI si usa ogni giorno al posto della CLI.

**Architettura:** `galleria-profile` riceve due archivi nuovi (`KeyringSecretStore`, `LayeredSecretStore`) accanto a
`FileSecretStore`, perché li usano GUI e CLI. Il lato Rust di `galleria-gui` guadagna tre moduli senza `tauri`
(`calls`, `session`, `logs`) e otto comandi sottili. **Profilo e segreti diventano i parametri di `connect` in Rust**
(`to_connect_params`, che c'è già): la pagina manda un identificativo e riceve stati. Le credenziali che l'utente
digita fanno un solo viaggio, dalla finestra a Rust al demone (sotto, «Il viaggio di una credenziale»). **L'IPC della
fase 1 non cambia**: si usano `connect`, `disconnect`, `provide_input`, `logs` e gli eventi che ci sono.

**Stack:** quello di B4, più un crate per Secret Service (Q44, misurato: sotto). Nell'immagine `galleria-gui-dev`
entrano `gnome-keyring` e `libsecret-tools`.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md`, §3.2, §3.3, §4.2, §4.3, §8, §9, §10, §12.1, §14
(«spec §n»). Stato di partenza: `docs/08-esito-fase2.md`, sezione «B4». D20, D21, D25, D28: `docs/07-proposte-fase2.md`
§2.2, §2.3, §3.1, §3.4. Decisioni D5–D32 e Q12–Q41: chiuse. Q35 resta in B6.

**Prima di partire:** le questioni Q42–Q47 (in fondo) devono avere una risposta. Se nessuno risponde vale la
raccomandazione scritta lì: il piano è scritto su quelle. **Q44 cambia il gruppo A, Q43 il T4, Q42 il gruppo D**: vanno
decise prima di aprire la card di quel gruppo. Worktree dedicato a partire da `main` (`900ccf8` o successivo), che deve
contenere questo piano.

Legenda: **[V]** verificato oggi, con la fonte · **[S]** supposto, con il task che lo verifica.

## Gruppi di lavoro: una card per gruppo

Undici task in quattro gruppi (la spec §15 ne stimava 8: tre in più sono il laboratorio e i gate in CI). I gruppi vanno
in fila: ognuno parte dal ramo del precedente, già rivisto.

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Archivio dei segreti** | T1, T2, T3 | `KeyringSecretStore` e `LayeredSecretStore` in `galleria-profile`; portachiavi vero nell'immagine e in CI, con il controllo dei gate; la CLI legge dai due archivi; la GUI scrive nel portachiavi, con l'avviso del ripiego | `main` con questo piano, Q44 decisa | 15 h | ciò che la finestra importa finisce nel portachiavi; la CLI lo legge; senza portachiavi tutto va nel file, e si vede |
| **B · Connetti** | T4, T5, T6 | comandi `connect_profile` e `disconnect`, sostituzione, annunci di stato; pulsante, sette stati, statistiche, errori tradotti; finestra delle credenziali, «Ricorda», «Dimentica» | A, con Q43 decisa | 18 h | ci si connette e ci si disconnette dalla finestra contro il demone finto, credenziali comprese |
| **C · Log e sessione estranea** | T7, T8 | scheda Log con «Copia» ed «Esporta log…»; riga fissa, barra e conferme per la sessione estranea | B | 10 h | la finestra è completa contro il demone finto |
| **D · Laboratorio e chiusura** | T9, T10, T11 | la finestra parla con il demone vero dei tre client (Q42); G4–G9 nel job `lab-connect`; tastiera, fumo, screen reader, esito | C, con Q42 decisa | 17 h | **B5 chiuso**: G4, G5, G6, G7, G9 con la loro prova in CI |

Le stime contano **un solo giro di `scripts/gui.sh smoke` e uno di `scripts/gui.sh a11y` per gruppo, a fine gruppo**
(un'ora per gruppo, dentro il numero della tabella). Durante un task si lanciano `check` e la sola prova nuova con il
filtro. I giri ripetuti («tre volte di fila», «due volte») si fanno una volta sola in tutto il blocco, alla fine del
gruppo D.

Ogni card riporta: numero dei task, i comandi di «Dove girano i comandi» verdi sull'ultimo commit, la sezione «B5» di
`docs/08-esito-fase2.md` aggiornata con i propri task. La review è una per gruppo.

## Criteri di uscita del blocco

| # | Criterio | Come si verifica | Task |
|---|---|---|---|
| C1 | **G4**: connetti, disconnetti e sostituisci dalla finestra con WireGuard e OpenVPN sui server del laboratorio, sui tre client; mai due backend vivi; IP, byte e ultimo handshake si aggiornano | job `lab-connect`: `G4a`–`G4e` e `t18`; livello 2 `replace_*` | T4, T5, T9 |
| C2 | **G5**: si vede solo il campo chiesto; «Ricorda» salva nel portachiavi e la connessione dopo non chiede; OTP e passphrase non si salvano mai | job `lab-connect`: `G5a`–`G5e`; job `gui-smoke`, passo `keyring`: archivio contro GNOME Keyring vero | T1, T2, T6, T10 |
| C3 | **G6**: un profilo importato dalla CLI si connette dalla finestra e viceversa | job `check`: prove di `galleria-profile` sui due archivi; `gui-smoke`/`keyring`: CLI e archivio sullo stesso portachiavi; `lab-connect`: `G6a`, `G6b` | T1, T2, T3, T10 |
| C4 | **G7**: nessun segreto noto nella scheda Log, nel file esportato, su `stderr` della GUI, negli argomenti e nell'ambiente dei processi, nel journal; nessun comando restituisce un segreto | job `gui`: `no_command_returns_a_secret`; job `lab-connect`: `G7a` e `t18` | T4, T6, T7, T10 |
| C5 | **G9**: una sessione avviata da un altro utente del gruppo è mostrata come estranea; chiuderla o sostituirla chiede conferma | job `gui`: `a_session_outside_the_rubrica_is_foreign`; job `lab-connect`: `G9a`–`G9c` e `t18` (due uid nel log del demone) | T8, T10 |
| C6 | Spec §3.2, regole 2, 3 e 5: nessun comando prende un percorso; le credenziali digitate fanno un solo viaggio; niente segreti in `argv` o nell'ambiente | `no_command_takes_a_path` (di B3); le prove di «Il viaggio di una credenziale» | T6, T7 |
| C7 | G12 e Q27 per i componenti di B5: tutto da tastiera; la finestra delle credenziali e le conferme prendono il fuoco; i cambi di stato sono detti da Orca **una volta** | axe e tastiera per componente; `scripts/gui.sh a11y`: `K5a`–`K5f` (locale, Q22) | T4–T8, T11 |
| C8 | La fase 1 non se ne accorge: `git diff main -- crates/galleria-ipc crates/galleriad 'crates/galleria-backend*' crates/galleria-net crates/galleria-ovpn` è vuoto; `lab` e `lab-g3` restano verdi | il comando; i job | ogni task |

### Dove gira ogni gate in CI

La lezione di B4: una prova che si salta risulta verde. In B5 **ogni gate ha un job, e ogni job che dichiara un gate
finisce con `scripts/check-gates.py`**, che legge `lab/tests/gates-b5.tsv` (gate, job, tipo, nome della prova) e
fallisce se una prova elencata per quel job manca dall'uscita, è saltata, ignorata o fallita. Nasce nel T2.

| Gate | Job | Comando del job | Prove che il job deve aver eseguito |
|---|---|---|---|
| G4 | **`lab-connect`** (nuovo; matrice 26, 27, 79) | `LAB_BUILD=host scripts/gui.sh lab-connect <client>` | `G4a`–`G4e` (finestra), `gui_session_never_had_two_backends`, `gui_session_left_no_residue` (`t18`, nel client) |
| G5 | `lab-connect` · `gui-smoke` | come sopra · `scripts/gui.sh keyring` | `G5a`–`G5e` · le prove `keyring_*` contro GNOME Keyring |
| G6 | `check` · `gui-smoke` · `lab-connect` | `cargo test --workspace --exclude galleria-gui` · `scripts/gui.sh keyring` · come sopra | `layered_*` · `the_cli_reads_what_the_keyring_holds` · `G6a`, `G6b` |
| G7 | `gui` · `lab-connect` | `scripts/gui.sh check-here` · come sopra | `no_command_returns_a_secret`, `no_event_carries_a_secret` · `G7a`, `gui_session_left_no_secret_in_journal_or_argv` |
| G9 | `gui` · `lab-connect` | come sopra | `a_session_outside_the_rubrica_is_foreign`, `disconnecting_a_foreign_session_is_not_automatic` · `G9a`–`G9c`, `gui_session_was_driven_by_two_users` |

`lab-connect` è la copia di `lab-g3` con il verbo nuovo; `gui-smoke` guadagna il passo `scripts/gui.sh keyring` (come
ha `socket-owner`). Una prova di `tests/config.rs` (`every_b5_gate_has_a_job`) legge `gates-b5.tsv` e `ci.yml`: ogni
gate ha almeno una riga, ogni job nominato esiste e il suo ultimo passo utile chiama `check-gates.py`.

**In locale** Docker è rootless: OpenVPN 2.7 sui client 27 e 79 prova il DCO e riceve `EPERM` **[V: esito di B4, T6]**.
Quindi in locale `lab-connect 26` è intero, su 27 e 79 è verde solo WireGuard; **la prova di G4–G9 sui tre client è il
run di CI**, che parte dopo il push (lo fa Aitano: chi implementa non pusha). La card del gruppo D dice quali prove ha
visto verdi in locale e quali aspettano il run; la review legge i nomi delle prove nei log del run, come per B4.

## Vincoli globali

Valgono i vincoli dei piani di B1–B4 (un task = un commit, messaggi in inglese minuscolo imperativo, TDD, nessun push,
niente `unwrap()` fuori dalle prove, la GUI non esegue processi, CSP invariata, testi solo nei cataloghi,
`#![forbid(unsafe_code)]`, ogni codice nuovo in `GUI_CODES` e nei due cataloghi nello stesso commit, ogni comando nuovo
in `COMMANDS`, in `capabilities/main.json` e in `tests/config.rs` nello stesso commit). In più:

- **Non si toccano**: `crates/galleriad`, `galleria-ipc`, `galleria-backend*`, `galleria-net`, `galleria-ovpn`,
  `design/`, `docs/00`–`07`. **L'IPC non cambia**: se a un task sembra servire un metodo, un campo o un codice nuovo,
  ci si ferma e si riporta; è una Q, non una scelta di chi implementa.
- **Eccezioni dichiarate, e solo queste**: in `galleria-profile` solo aggiunte (Q29), compreso un metodo nuovo nel
  tratto `SecretStore`; in `galleria-cli` solo il punto in cui `connect` apre l'archivio dei segreti (T2, D25); in
  `galleria-e2e` un file di prova nuovo (`t18`); in `lab/run.sh` la guardia di `t17` estesa a `t18`.
- **Segreti.** Nessun comando e nessun evento porta un segreto alla pagina, mai: né le chiavi, né la password
  ricordata, né il nome utente ricordato come «valore da riempire» (il nome utente salvato lo usa Rust; alla pagina va
  al più un sì o un no). `secret_refs` porta nomi, come in B3.
- **Percorsi.** Nessun comando ha un parametro che è un percorso o un nome di file. «Esporta log…» apre la finestra
  «Salva» di GTK da Rust con `SavePicker` (B3) e risponde `{ saved }`.
- **Processi e ambiente.** Nessun segreto in un argomento o in una variabile d'ambiente, nemmeno nelle prove del
  laboratorio (le credenziali di prova vanno su stdin o nei campi della finestra).
- **`stderr` della GUI.** Mai parametri di `connect`, valori digitati, contenuti di profili o righe del log del demone.
- **Portachiavi.** La finestra non chiede mai lo sblocco da sola: lo sblocco può partire solo da un gesto (connetti,
  importa, salva, «Ricorda», duplica, esporta, elimina). L'avvio, la lista e `secret_store_status` non leggono segreti.
- **Niente prove che si saltano da sole.** Una prova nuova che non trova ciò che le serve (bus, portachiavi,
  laboratorio) **fallisce** dicendo cosa manca. Le prove che girano solo in un job stanno in un file a parte, lanciato
  da quel job, e sono nel manifesto dei gate.
- **Annunci (Q27).** I cambi di stato li annuncia Rust (Q43); la finestra delle credenziali, le conferme e gli errori
  che chiedono una risposta prendono il fuoco e non annunciano.
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

  Dal T2 anche `scripts/gui.sh keyring`. Ogni **gruppo** finisce, una volta sola e sull'ultimo commit, con
  `scripts/gui.sh smoke` e `scripts/gui.sh a11y` (locale, Q22); il gruppo D anche con `scripts/gui.sh lab-connect 26`.
  Filtri e `git commit --fixup` come nel piano di B4.
- **API di Tauri, `gtk`, del crate di Secret Service**: nomi e firme dai sorgenti **della versione nel `Cargo.lock`**.
  I nomi di questo piano valgono per il nostro codice.
- Se un punto **[S]** risulta falso si applica il ripiego scritto e lo si annota in `docs/08-esito-fase2.md`, sezione
  «B5». Se il ripiego tocca una decisione D o Q chiusa, ci si ferma e si riporta.

## Fatti letti e misurati per questo piano

Script e uscite in `spikes/b5-piano/` (README con le tabelle intere).

### Il codice della fase 1 che B5 usa **[V: letto oggi a `900ccf8`]**

| Fatto | Fonte | Conseguenza |
|---|---|---|
| `SecretStore` ha `get`, `set`, `delete_all`: **non c'è la cancellazione di una chiave sola** (la spec §9 scrive `delete`) | `galleria-profile/src/secrets.rs` | «Dimentica» e la regola 1 del §9 chiedono un metodo nuovo: `delete(id, key)`, aggiunta al tratto (T1) |
| `ProfileStore::delete` cancella i segreti aprendo `FileSecretStore` da sé | `store.rs`, riga 201 | eliminando un profilo dalla GUI le voci del portachiavi resterebbero: `Rubrica::delete` le toglie dall'archivio a strati (T3) |
| `to_connect_params` legge solo i blocchi `openvpn.block.*`, le chiavi WireGuard e mette `username`, `password`, `otp`, `key_passphrase` a `None` | `lib.rs` | utente e password ricordati li aggiunge il chiamante dopo, come fa la CLI con `--username` e stdin (T6) |
| `connect` con una sessione attiva risponde `already_active` con `data.profile` (id e nome della sessione attiva) | `galleriad/src/manager.rs`, riga 462 | Rust decide «propria o estranea» cercando quell'id nella rubrica (T4) |
| Un errore di autenticazione non viene mai ritentato dal demone | `machine.rs`, riga 83 | una password ricordata sbagliata fallirebbe a ogni connessione: Q47 |
| `InputRequest` porta `request`, `kind` (`credentials`, `otp`, `key_passphrase`), `prompt`, `echo`, `expires_in_secs` (120) | `types.rs`; `manager.rs`, riga 660 | la finestra mostra il campo di `kind`; con `echo: true` il testo si vede |
| `provide_input` fuori tempo o con `request` vecchia risponde `input_not_expected` | `manager.rs`, riga 556 | la finestra si chiude e segue lo stato; non è un errore da mostrare come guasto |
| `logs` dà al più 1000 righe per chiamata, dalle più vecchie; il buffer ne tiene 2000 e si svuota all'inizio di ogni sessione | `methods.rs`, `logbuf.rs`, `manager.rs` riga 278 | `logs_get` si chiama a pagine con `after`, come fa la CLI |
| `InputValues` e `Secret` si deserializzano; `Secret` si azzera alla distruzione e non si stampa | `types.rs`, `secret.rs` | il comando `provide_input` riceve direttamente `InputValues` |
| Il demone autorizza con `SO_PEERCRED` e `SO_PEERGROUPS` del socket, senza leggere `/proc/<pid>` | `auth.rs`, `peer_groups.rs` | Q42: conta chi apre la connessione dentro il client |
| Nei client del laboratorio c'è un solo utente del gruppo `galleria` (`alice`); `bob` è un utente della VPN, non del sistema | `lab/images/client.Dockerfile`, `lab/server/verify.sh` | per G9 serve un secondo utente del gruppo: lo crea il laboratorio a tempo di prova (`carla`), senza toccare le immagini |
| Il segreto TOTP del laboratorio è una costante (`JBSWY3DPEHPK3PXP`) | `t13`, `verify.sh` | la prova della finestra calcola il codice in Node; non serve `oathtool` nell'immagine |

### Il crate per Secret Service (S12 della spec, misurato)

`cargo add` in `galleria-profile`, crate in più in `Cargo.lock` (base 537), `cargo deny check`, e
`cargo build -p galleria-cli` su questo host, che **non ha `libdbus-1-dev`** **[V: `spikes/b5-piano/`]**:

| Candidato | Crate in più | `cargo deny` | Si compila sull'host | Librerie di sistema a runtime |
|---|---|---|---|---|
| `dbus-secret-service` 4.2.0, `vendored` | +8 | verde | sì | nessuna (`ldd` senza dbus) |
| `dbus-secret-service` 4.2.0, senza `vendored` | +1 | verde | **no** (`pkg-config` non trova `dbus-1`) | `libdbus-1.so.3` |
| `secret-service` 6.0.0, `rt-tokio` | +21 (`zbus` 6.5.0) | verde | sì | nessuna |
| `oo7` 0.7.3 | +33 | verde | non provato | — |
| `keyring` 4.2.0, predefinito | +50 | **rosso**: `RUSTSEC-2026-0370`, `-0371`, `-0372` | non provato | — |

`dbus` 0.9.12 e `libdbus-sys` 0.2.7 sono già in `Cargo.lock`, portati da `tao` per la sola GUI **[V: `cargo tree -i
dbus`]**. `dbus-secret-service` è sincrono, come il tratto `SecretStore`; `secret-service` 6 è asincrono. Da qui Q44.

### Il viaggio di una credenziale chiesta su `needs_input`

È la regola che la review controlla. Ciò che si può garantire e ciò che no:

| Tappa | Cosa succede | Chi lo tiene fermo |
|---|---|---|
| 1. Campo | I campi della finestra delle credenziali sono **non controllati**: il valore sta nell'elemento `<input>`, non in uno stato di React, non in uno store, non in una chiusura che sopravvive all'invio | `the_credentials_dialog_keeps_no_value_in_state` (statica: il file non ha `useState` né store per i valori; `provide_input` è invocato in un solo file di `ui/src`) |
| 2. Invio | Al click si leggono i valori dai riferimenti, si chiama **una** `invoke("provide_input", …)` e i campi si svuotano **prima** di aspettare la risposta; niente nuovo tentativo automatico con gli stessi valori | `the_fields_are_empty_before_the_answer_arrives`; `provide_input_is_invoked_once_per_submit` |
| 3. Tauri | Il corpo dell'`invoke` arriva a Rust come `serde_json::Value` **[V: `tauri` 2.12.7, `ipc/mod.rs`, `InvokeBody::Json`]** e il comando lo deserializza in `InputValues`, cioè in `Secret` | il tipo del parametro; nessuna `String` intermedia nel nostro codice |
| 4. Rust | `calls::provide_input` apre una connessione breve, `hello`, `provide_input`, chiude. I valori cadono (e si azzerano) alla fine della chiamata | `provide_input_reaches_the_daemon_and_nothing_else` |
| 5. «Ricorda» | Solo con `remember: true` e `kind == credentials`: utente e password restano in `session::Pending`, come `Secret`, **finché quella sessione arriva a `connected`**; allora si scrivono nell'archivio e si buttano. A `error`, `disconnected` o sessione diversa si buttano senza scrivere. OTP e passphrase non ci entrano mai | `remember_saves_only_after_connected`, `a_failed_login_saves_nothing`, `otp_and_passphrase_are_never_kept` |
| 6. Risposta | `provide_input` risponde `()`; nessun evento ripete i valori | `no_command_returns_a_secret`, `no_event_carries_a_secret` |

**Limite dichiarato, da scrivere nell'esito:** le copie che non sono nostre non si azzerano. La stringa JavaScript del
campo vive finché il motore non la raccoglie; il processo di WebKit e il `serde_json::Value` di Tauri tengono una
copia per la durata della chiamata. «Non più del necessario» qui vuol dire: nessuna copia **in più** fatta dal nostro
codice, e nessuna che sopravviva alla chiamata per colpa nostra. L'alternativa che evita la pagina del tutto (una
finestra GTK aperta da Rust, come «Apri» e «Salva») è fuori da B5: la spec §3.2, regola 2, mette la finestra delle
credenziali nella pagina.

## Punti di attenzione per la revisione

1. **Il portachiavi non deve sorprendere.** `secret_store_status` chiede al bus solo se il servizio
   `org.freedesktop.secrets` c'è (o si può attivare): non apre collezioni e non sblocca. Una prova lo tiene fermo
   contro un portachiavi **bloccato**: nessuna richiesta di sblocco all'avvio. → T3
   `starting_the_app_never_asks_to_unlock`.
2. **Bloccato non è assente** (spec §9, regola 2; Q20). Servizio assente: si scrive nel file, con l'avviso fisso.
   Servizio presente ma bloccato, o sblocco rifiutato: `gui_keyring_locked`, e **niente** nel file. → T1
   `a_locked_keyring_is_an_error_and_writes_no_file`.
3. **Mai due valori diversi** (spec §9, regola 1). Dopo una scrittura riuscita nel portachiavi la copia nel file si
   cancella; in lettura vale il portachiavi. Se la cancellazione dal file fallisce la scrittura è comunque riuscita, e
   la lettura dopo dà il valore nuovo. → T1 `after_a_keyring_write_the_file_copy_is_gone`.
4. **Tutto o niente resta vero su due archivi.** Import e «Duplica» scrivono più segreti e poi il profilo: se uno
   fallisce (portachiavi bloccato a metà) ciò che è stato scritto si toglie da **tutti e due**. → T3.
5. **La password si ricorda solo se ha funzionato** (tappa 5 sopra). Altrimenti una password sbagliata salvata
   bloccherebbe ogni connessione dopo (Q47).
6. **«Propria o estranea» lo decide Rust**, sulla rubrica vera, nel momento della risposta: la pagina riceve
   `already_active` con `foreign: true|false`. Una rubrica che non si legge vale «estranea»: nel dubbio si chiede.
   → T4 `an_unreadable_rubrica_makes_the_session_foreign`.
7. **La sostituzione senza conferma vale solo per `foreign: false`.** La seconda chiamata con `replace: true` la fa la
   pagina; Rust non sostituisce mai da solo. → T5 `a_foreign_session_is_never_replaced_without_the_confirmation`.
8. **Lo stato vero è il flusso degli eventi** (spec §3.3). La risposta di `connect_profile` serve solo per l'errore di
   quella chiamata; pulsanti e testi seguono `galleria:state`. Tra il click e il primo evento il pulsante è occupato,
   non «Connesso». → T5 `the_button_follows_the_events_not_the_answer`.
9. **`needs_input` può arrivare più volte** nella stessa sessione (credenziali e poi OTP; una riconnessione chiede un
   OTP nuovo **[V: `t13`, `reconnect_of_otp_profile_asks_for_a_new_otp`]**). La finestra si riapre per ogni `request`
   nuova, anche a connessione già avvenuta, e una `request` già servita non si ripresenta. → T6.
10. **Il conto alla rovescia** parte da `expires_in_secs` quando arriva l'evento, non quando la finestra si disegna; a
    zero la finestra si chiude e lo stato che arriva (`error`, `input_timeout`) dice il resto.
11. **Il log non è nostro.** Le righe si mostrano come arrivano, non si traducono e non si filtrano di nuovo: il filtro
    è del demone. La GUI non le scrive su `stderr`. Il file esportato è `0600`.
12. **Una sessione estranea non ha un profilo da mostrare**: nome e protocollo vengono da `Status`, il dettaglio ha
    solo lo stato e la scheda Log. Nessuna scheda del profilo, nessun «Elimina».
13. **Il ponte del laboratorio è codice di prova** (Q42): sta in `lab/tests/`, non in un crate, non parla JSON, non
    entra in nessun pacchetto. Il binario provato è quello di `scripts/gui.sh build`.
14. **Niente regressioni**: le prove di fumo e di screen reader di B1–B4 restano. `rubrica_works_with_the_daemon_stopped`
    e `import_works_with_the_daemon_stopped` devono restare verdi con il pulsante «Connetti» presente (e spento).

## Decisioni di progetto prese qui

Valgono per tutti i gruppi; chi implementa non le riapre.

| # | Decisione | Perché |
|---|---|---|
| P1 | Comandi di B5: i sette della spec §4.2 più **`announce_phrases`** (Q43). Firme: `secret_store_status() -> { store }`; `connect_profile(id, replace) -> ()`; `disconnect() -> ()`; `provide_input(session, request, values, remember) -> ()`; `forget_credentials(id) -> ()`; `logs_get(after, limit) -> { entries, last }`; `logs_export() -> { saved }` | la risposta di `connect` e `disconnect` non porta `Status`: lo stato arriva dagli eventi (punto 8) |
| P2 | `already_active` esce da `connect_profile` con `data: { profile: { id, name }, foreign }` | punto 6 |
| P3 | Codici nuovi: `gui_keyring_locked`, `gui_keyring_failed` (il portachiavi risponde con un errore che non è «bloccato»), `gui_secret_missing` (al profilo manca una chiave nell'archivio: «importa di nuovo il profilo»), `gui_daemon_unreachable` (la connessione breve del comando non si apre) | i codici del demone passano uguali |
| P4 | «Ricorda» è una casella nella finestra delle credenziali, solo per `kind == credentials`, spenta la prima volta. Le chiavi sono `openvpn.username` e `openvpn.password` (spec §9, regola 4) e i due nomi entrano in `secret_refs` del profilo (Q46) | la scheda Autenticazione sa che ci sono credenziali salvate senza aprire il portachiavi |
| P5 | Alla connessione Rust manda utente e password ricordati, se ci sono. Una credenziale che `secret_refs` nomina ma l'archivio non ha più (tolta a mano dal portachiavi) **non è un errore**: non si manda, il demone la chiede, e i due nomi escono da `secret_refs` | chi pulisce il portachiavi non deve rompere il profilo |
| P6 | «Dimentica le credenziali salvate» sta nella scheda Autenticazione (solo se ci sono) e nell'errore `auth_failed` di una sessione che le ha usate (Q47) | un posto fisso e uno dove serve |
| P7 | «Annulla» nella finestra delle credenziali e «Annulla» durante `connecting` sono `disconnect` (spec §8). La finestra delle credenziali non si chiude con un click fuori | un click sbagliato non deve buttare una connessione |
| P8 | Durante `connecting`, `reconnecting`, `needs_input`, `disconnecting` il pulsante principale del profilo attivo è «Annulla» o è spento (`disconnecting`); sugli altri profili «Connetti» resta attivo e porta alla sostituzione | D28: si cambia VPN con un click |
| P9 | Statistiche: indirizzi locali da `Status.local_addresses`, byte da `galleria:stats`, «ultimo handshake» solo se il backend lo dichiara (`capabilities.handshake`). I byte si mostrano in unità leggibili, il valore esatto nel `title` | OpenVPN non ha un handshake da mostrare |
| P10 | `detail` di `Status` si traduce per le chiavi note (`state.detail.<valore>`) e si mostra grezzo altrimenti (spec §6). L'elenco dei valori noti lo scrive il T5 leggendo `galleriad` e i due backend | D22 |
| P11 | La scheda Log tiene in pagina al più 2000 righe (quelle del buffer del demone); «Copia» copia ciò che la scheda mostra (`navigator.clipboard`, S7 è vera); «Esporta log…» riscarica tutto da Rust, non prende il testo dalla pagina | il file esportato non passa dalla webview |
| P12 | Nome proposto per l'export: `galleria-log-AAAAMMGG-HHMM.txt`. Formato di una riga: quello di `galleria logs` (`ts level source: message`) | chi allega un log lo confronta con la CLI |
| P13 | Per una sessione estranea la scheda Log mostra, copia ed esporta come sempre: «sola lettura» (D21) vuol dire che non c'è altro da fare che «Disconnetti» | i membri del gruppo vedono già stato e log (spec F1 §5.1) |
| P14 | Annunci di stato (Q43): Rust annuncia quando cambia la terna (stato, id del profilo, codice d'errore), non a ogni evento. `connected`, `disconnected`, `reconnecting` sono *polite*; `error` è *assertive*; `connecting` e `disconnecting` non si annunciano; `needs_input` nemmeno (prende il fuoco la finestra) | Q27, regole 1 e 2; un annuncio per fatto |
| P15 | «Ultimo uso» (`state.toml`, D32) si aggiorna quando una sessione di un profilo della rubrica arriva a `connected` (`UiStateFile::touch_last_used`, che c'è già) | B3 lo mostra, nessuno lo scriveva |
| P16 | Il demone finto del fumo (`examples/fake-daemon.rs`) prende `--scenario <nome>`: una lista di copioni `FakeStep`, uno per `connect`. Gli scenari stanno in `tests/support/scenarios.rs`, usati anche dal livello 2 | il fumo di `gui-smoke` prova credenziali, errori e sostituzione senza laboratorio |

## Mappa dei file

```
crates/galleria-profile/src/secrets.rs            SecretStore::delete (aggiunta), FileSecretStore::delete          T1
crates/galleria-profile/src/secrets_layered.rs    LayeredSecretStore, Writes, StoreKind (solo aggiunta)            T1
crates/galleria-profile/src/secrets_keyring.rs    KeyringSecretStore, keyring_present (cfg linux)                  T1
crates/galleria-profile/src/store.rs              StoreError::KeyringLocked, ::Keyring (varianti aggiunte)         T1
crates/galleria-profile/tests/{secrets_layered,keyring}.rs                                                         T1, T2
crates/galleria-cli/src/commands.rs               `connect` legge dai due archivi (una funzione)                   T2
crates/galleria-cli/tests/keyring.rs                                                                               T2
crates/galleria-gui/src/
  calls.rs                    connessioni brevi: connect, disconnect, provide_input, logs      — senza `tauri`     T4, T6, T7
  session.rs                  Sessions: connetti, estranea, Pending di «Ricorda», annunci      — senza `tauri`     T4, T6
  logs.rs                     pagine di `logs`, testo dell'export                              — senza `tauri`     T7
  commands/{connection,logs,secrets}.rs   involucri sottili                                                        T3, T4, T6, T7
  rubrica.rs  rubrica/actions.rs          archivio a strati; delete toglie da tutti e due                          T3
  supervisor.rs  events.rs                gli stati passano da `Sessions` prima di andare alla pagina              T4
  error.rs  lib.rs  build.rs  capabilities/main.json  tests/config.rs      ogni task che aggiunge un comando
crates/galleria-gui/tests/{secrets,connection,credentials,logs,foreign}.rs  + support/scenarios.rs                 T3–T8
crates/galleria-gui/examples/fake-daemon.rs       --scenario                                                       T5
crates/galleria-e2e/tests/t18_gui_connect.rs      ciò che il client ha visto mentre la finestra connetteva         T9, T10
lab/images/gui.Dockerfile                         gnome-keyring, libsecret-tools                                   T2
lab/tests/gates-b5.tsv  scripts/check-gates.py  scripts/tests/check-gates.sh                                       T2
lab/tests/gui-keyring.sh                          bus privato, portachiavi sbloccato, bloccato, assente            T2
lab/tests/{gui-daemon-relay.py,gui-lab-watch.sh,gui-lab-connect.sh}                                                T9
lab/run.sh (guardia di t17 estesa a t18)  lab/README.md  scripts/gui.sh (verbi keyring, lab-connect)               T2, T9
.github/workflows/ci.yml                          passo `keyring` in gui-smoke; job lab-connect; check-gates       T2, T9
ui/src/connection/{ConnectButton,StatusPanel,Stats,CredentialsDialog,ForeignRow,ReplaceConfirm}.tsx + store.ts     T5, T6, T8
ui/src/log/{LogTab.tsx,store.ts}                                                                                   T7
ui/src/shell/{Shell,ProfileDetail,ProfileList,StatusBar,RubricaNotices}.tsx  ui/src/shell/tabs/Authentication.tsx  T3, T5–T8
ui/src/app/announce.ts                            consegna delle frasi di stato a Rust (Q43)                       T4
ui/src/i18n/{en,it}.json                          testi e codici nuovi                                             ogni task
ui/e2e/{smoke.e2e.ts,connection.a11y.ts,lab-connect.e2e.ts,support.ts,vitest.config.ts}                            T5–T11
README.md                                         portachiavi, gruppo `galleria`, `journalctl -u galleriad`        T11
docs/08-esito-fase2.md                            sezione «B5», aggiornata a ogni task
ROADMAP.md  .claude/handoffs/                     T11
```

---

## Gruppo A · Archivio dei segreti

### Task 1: `KeyringSecretStore` e `LayeredSecretStore` in `galleria-profile`

Scritto su **Q44 (a)**: `dbus-secret-service` 4.2.0 con `vendored`, senza cifratura della sessione. Se Q44 passa come
(b), `KeyringSecretStore` avvolge `secret-service` 6 su un thread suo con un runtime tokio a un solo thread; il resto
del task non cambia.

**Stima:** 6 h. **Scioglie:** S12 (della spec), S34. **File — crea:** `crates/galleria-profile/src/secrets_layered.rs`,
`src/secrets_keyring.rs`, `tests/secrets_layered.rs`. **Modifica:** `crates/galleria-profile/Cargo.toml`,
`src/{lib.rs,secrets.rs,store.rs}`, `crates/galleria-gui/tests/{rubrica,import,export}.rs` (i tre archivi finti
guadagnano `delete`), `Cargo.lock`, `docs/08-esito-fase2.md`.

**Produce** (solo aggiunte, Q29):

```rust
// secrets.rs — il tratto guadagna un metodo; le tre firme che c'erano non cambiano
pub trait SecretStore {
    fn get(&self, id: &Uuid, key: &str) -> Result<Option<Secret>, StoreError>;
    fn set(&self, id: &Uuid, key: &str, value: &Secret) -> Result<(), StoreError>;
    fn delete(&self, id: &Uuid, key: &str) -> Result<(), StoreError>;        // nuovo; una chiave che non c'è non è un errore
    fn delete_all(&self, id: &Uuid) -> Result<(), StoreError>;
}
// store.rs — due varianti in più
StoreError::KeyringLocked            // il servizio c'è ma è bloccato, o lo sblocco è stato rifiutato
StoreError::Keyring(String)          // ogni altro errore del servizio; il testo non contiene mai un valore

// secrets_keyring.rs — #[cfg(target_os = "linux")]
/// Vero se sul bus di sessione c'è (o si può attivare) `org.freedesktop.secrets`. Non apre collezioni, non sblocca.
pub fn keyring_present() -> bool;
pub struct KeyringSecretStore { /* connessione, collezione predefinita, attesa massima dello sblocco */ }
impl KeyringSecretStore {
    /// `None` se il bus o il servizio non ci sono. Non sblocca niente.
    pub fn open() -> Option<Self>;
    /// Etichetta «Galleria: <nome profilo> (<chiave>)» per le voci che `set` crea da qui in poi.
    pub fn labelled(self, profile_name: &str) -> Self;
}

// secrets_layered.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Writes { Keyring, File }   // GUI: Keyring; CLI: File
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum StoreKind { Keyring, File }
pub struct LayeredSecretStore { /* keyring: Option<Box<dyn SecretStore>>, file: Box<dyn SecretStore>, writes */ }
impl LayeredSecretStore {
    pub fn new(keyring: Option<Box<dyn SecretStore>>, file: Box<dyn SecretStore>, writes: Writes) -> Self;
    /// Dove finisce una scrittura: `Keyring` solo se `writes == Keyring` e il portachiavi c'è.
    pub fn kind(&self) -> StoreKind;
}
```

Voce del portachiavi (spec §9): attributi `service=galleria`, `profile=<uuid>`, `key=<secret_ref>`; tipo
`text/plain`; collezione predefinita. `get` cerca per attributi: nessuna voce → `Ok(None)`; voce bloccata → prova lo
sblocco (è un gesto dell'utente che ha portato qui) e, se non riesce, `KeyringLocked`.

Regole dell'archivio a strati (spec §9, Q20):

| Operazione | `Writes::Keyring` (GUI) | `Writes::File` (CLI) |
|---|---|---|
| `get` | portachiavi; se non ha la chiave, file. Un portachiavi bloccato è `KeyringLocked`, **non** si passa al file | uguale |
| `set` | portachiavi assente → file. Portachiavi presente → portachiavi, poi `delete` della copia nel file. Bloccato → `KeyringLocked`, niente nel file | file |
| `delete`, `delete_all` | da tutti e due; riesce se riesce dove la chiave c'è | da tutti e due |

- [ ] **1. Prove che falliscono.** `tests/secrets_layered.rs` usa un portachiavi finto in memoria (un `SecretStore`
  con un interruttore «bloccato»): gira nel job `check`, senza bus.

  | Prova | Asserzione |
  |---|---|
  | `layered_reads_the_keyring_first_then_the_file` | stessa chiave nei due: vale il portachiavi; solo nel file: si legge dal file |
  | `layered_gui_writes_go_to_the_keyring_and_the_file_copy_is_removed` | punto 3 della revisione |
  | `layered_without_a_keyring_writes_the_file` · `kind_says_where_a_write_goes` | `StoreKind::File` |
  | `a_locked_keyring_is_an_error_and_writes_no_file` | `get` e `set`: `KeyringLocked`; la cartella `secrets/` ha gli stessi byte di prima |
  | `layered_cli_writes_stay_in_the_file` | `Writes::File` con il portachiavi presente |
  | `delete_removes_one_key_from_both_and_keeps_the_others` · `delete_all_empties_both` | — |
  | `deleting_a_key_that_is_not_there_is_fine` | — |
  | `to_connect_params_reads_through_the_layers` | chiave privata nel portachiavi finto, blocco `ca` nel file: i parametri hanno tutti e due (G6) |
  | `file_store_delete_keeps_the_file_private` | dopo `delete` il file resta `0600` e senza la chiave |
  | `a_keyring_error_never_carries_a_value` | `Display` e `Debug` di ogni errore non contengono `SEED-SECRET-` |

- [ ] **2.** Falliscono. **3. Implementa.** Prima il crate: `cargo add` con le feature di Q44, poi `cargo deny check`
  e `scripts/gui.sh check` **subito** (**S34**: nella GUI `libdbus-sys` riceve insieme `vendored` e la feature
  predefinita di `tao`; va visto compilare e partire nell'immagine). I nomi degli 8 crate nuovi vanno nell'esito.
- [ ] **4.** I comandi del task.
- [ ] **5. Commit:** `add a keyring store and a layered store next to the file store`

**Fatto quando:** `git diff main -- crates/galleria-profile/src` non toglie e non cambia nessuna firma pubblica
esistente; `cargo deny check` è verde senza voci nuove in `deny.toml`; S34 ha un esito scritto.

---

### Task 2: il portachiavi vero nell'immagine e in CI, il controllo dei gate, la CLI sui due archivi

**Stima:** 5 h, con un tetto per KWallet (sotto). **Scioglie:** S35, S36, S37, S38, S45. **File — crea:**
`crates/galleria-profile/tests/keyring.rs`, `crates/galleria-cli/tests/keyring.rs`, `lab/tests/gui-keyring.sh`,
`lab/tests/gates-b5.tsv`, `scripts/check-gates.py`, `scripts/tests/check-gates.sh`. **Modifica:**
`lab/images/gui.Dockerfile`, `scripts/gui.sh` (verbo `keyring`; `check-here` scrive le uscite delle prove in
`target/gates/`), `crates/galleria-cli/src/commands.rs`, `.github/workflows/ci.yml`, `crates/galleria-gui/tests/config.rs`,
`docs/08-esito-fase2.md`.

**Produce:**

- **`scripts/gui.sh keyring`** (nell'immagine): `lab/tests/gui-keyring.sh` lancia le prove di `keyring.rs` tre volte,
  ognuna sotto un bus di sessione suo, e poi `check-gates.py --job gui-smoke`:

  | Giro | Ambiente | Prove |
  |---|---|---|
  | sbloccato | `dbus-run-session`, `gnome-keyring-daemon --unlock --components=secrets` con la password di prova su stdin (**S35**) | `keyring_roundtrip`, `keyring_items_carry_the_three_attributes_and_the_label` (riletti con `secret-tool search`), `keyring_delete_one_and_all`, `keyring_two_profiles_do_not_mix`, `keyring_value_with_newlines_survives` (un blocco PEM), `the_cli_reads_what_the_keyring_holds` |
  | bloccato | portachiavi avviato senza sblocco, nessun programma che chiede la password (**S37**) | `keyring_locked_is_reported_within_five_seconds`, `keyring_present_does_not_unlock` |
  | assente | un `dbus-daemon` con una configurazione senza cartelle dei servizi, così niente si attiva da solo (**S36**) | `keyring_absent_opens_as_none`, `the_cli_without_a_keyring_reads_the_file` |

  Le prove leggono `GALLERIA_KEYRING_CASE` (`unlocked`, `locked`, `absent`) e **falliscono** se la variabile manca:
  `cargo test --workspace` non le lancia (sono `#[ignore]`), e nessuna si salta da sola.
- **`scripts/check-gates.py --job <nome> [--cargo-log FILE…] [--vitest-json FILE…]`**: Python 3 senza dipendenze. Per
  ogni riga di `gates-b5.tsv` con quel job cerca la prova nelle uscite date: per `cargo`, la riga `test <nome> ... ok`
  (una riga `ignored` o `FAILED`, o nessuna riga, è un errore); per `vitest`, lo stato `passed` nel JSON del reporter
  (`skipped`, `todo`, `failed` o assente è un errore). Stampa una riga per prova ed esce con 1 al primo gate non
  provato. `scripts/tests/check-gates.sh` lo prova su uscite finte: una prova ignorata, una mancante, un job senza righe.
- **CLI (D25)**: in `galleria connect` l'archivio diventa `LayeredSecretStore::new(KeyringSecretStore::open(), file,
  Writes::File)`. `import` e `delete` restano come sono (file). Senza bus di sessione la CLI non stampa niente di
  nuovo e non aspetta (**S45**).
- **CI**: in `gui-smoke`, dopo `socket-owner`, il passo `scripts/gui.sh keyring`. In `check` e `gui`, l'uscita delle
  prove va in un file e l'ultimo passo è `check-gates.py --job check` (o `gui`).

- [ ] **1. Prove che falliscono.** Quelle della tabella, più:

  | File | Prova | Asserzione |
  |---|---|---|
  | `scripts/tests/check-gates.sh` | `an_ignored_test_fails_the_gate` · `a_missing_test_fails_the_gate` · `a_skipped_vitest_test_fails_the_gate` · `all_listed_tests_passed_is_green` | — |
  | `galleria-gui/tests/config.rs` | `every_b5_gate_has_a_job` | G4, G5, G6, G7, G9 hanno almeno una riga; ogni job del manifesto è in `ci.yml` e chiama `check-gates.py --job <sé>` |
  | `galleria-cli/tests/keyring.rs` | `the_cli_without_a_session_bus_reads_the_file_and_stays_quiet` | senza `DBUS_SESSION_BUS_ADDRESS`: stessa uscita di prima, entro 2 s (gira nel job `check`) |

- [ ] **2.** Falliscono. **3. Implementa.** Il manifesto nasce con le righe dei gate che T1 e T2 chiudono (`check`,
  `gui-smoke`); ogni task dopo aggiunge le sue **nello stesso commit della prova**.
- [ ] **4.** I comandi del task; `scripts/gui.sh keyring`.
- [ ] **5. Commit:** `run the keyring store against a real secret service and check the gates`

**KWallet (S38), tetto di un'ora:** in un contenitore usa e getta (Ubuntu 26.04, `kwalletd6` o il pacchetto che oggi
offre `org.freedesktop.secrets` su Plasma), sotto `dbus-run-session`, si lancia il giro «sbloccato». Se parte: l'esito
dice versione ed esito. Se chiede una finestra o non si attiva entro l'ora: «non verificato su KWallet» nell'esito e
nel README, e la prova va nella passata manuale di B6. In nessun caso entra nell'immagine o in CI.

**Ripieghi.** S35 falsa (GNOME Keyring non si sblocca senza schermo): un Secret Service di prova scritto con
`python3-gi` (c'è già nell'immagine) che tiene le voci in memoria, e l'esito dice che GNOME Keyring vero resta alla
passata di B6. S36 falsa: il caso «assente» si prova togliendo il bus (`DBUS_SESSION_BUS_ADDRESS` verso un socket che
non c'è) e «bus senza servizio» resta al portachiavi finto del T1. S37 falsa (lo sblocco resta appeso): attesa massima
dello sblocco a 60 s nell'app e a 3 s nelle prove (`connect_with_max_prompt_timeout` esiste nel crate **[V]**).

**Fatto quando:** nel log di `scripts/gui.sh keyring` ci sono i nomi delle dieci prove con `ok`; togliendo a mano una
riga `ok` dal log, `check-gates.py` esce con 1 (provato una volta).

---

### Task 3: la GUI scrive nel portachiavi; avviso del ripiego; `secret_store_status`

**Stima:** 4 h. **File — crea:** `crates/galleria-gui/src/commands/secrets.rs`, `tests/secrets.rs`. **Modifica:**
`crates/galleria-gui/src/{rubrica.rs,rubrica/actions.rs,rubrica/imported.rs,lib.rs,error.rs}`, `build.rs`,
`capabilities/main.json`, `tests/config.rs`, `ui/src/shell/{RubricaNotices,Shell}.tsx`, `ui/src/import/Preview.tsx`,
`ui/src/daemon/commands.ts`, cataloghi, `ui/e2e/smoke.e2e.ts`, `lab/tests/gui-lib.sh`.

**Produce:**

- `Rubrica::new` usa l'archivio a strati (`Writes::Keyring`) dal punto unico `with_secrets` di B3. `Rubrica::delete`
  chiama `delete_all` sull'archivio a strati **prima** di `ProfileStore::delete`.
- `secret_store_status() -> { "store": "keyring" | "file" }`: `keyring_present()`, niente altro (punto 1).
- Webview: con `store == "file"` un avviso in linea **fisso** in cima alla lista («Nessun portachiavi di sistema: le
  chiavi sono salvate in un file leggibile solo da te.»), non chiudibile, riletto quando la finestra torna in primo
  piano. L'anteprima dell'import dice «nel portachiavi di sistema» o «in un file leggibile solo da te» secondo lo stato.
- `gui_keyring_locked` da import, «Duplica», «Esporta», «Elimina», salvataggio: avviso in linea che prende il fuoco,
  «Sblocca il portachiavi e riprova». Niente si scrive a metà (punto 4).
- `gui-lib.sh`: il fumo parte con un portachiavi sbloccato suo (la funzione del T2), così le prove di B3 e B4 girano
  sul portachiavi; `GALLERIA_SMOKE_KEYRING=absent` dà il bus senza servizio.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/secrets.rs` | `import_commit_writes_the_keyring_and_no_secret_file` | con il portachiavi finto: `secrets/<id>.toml` non esiste |
  | | `without_a_keyring_import_writes_the_file_and_the_status_says_file` | — |
  | | `a_locked_keyring_stops_the_import_and_leaves_nothing` | `gui_keyring_locked`; nessun profilo, nessun segreto nei due archivi |
  | | `a_duplicate_stopped_halfway_leaves_nothing_in_either_store` | il portachiavi finto si blocca alla seconda scrittura |
  | | `delete_removes_the_keyring_items_too` | — |
  | | `a_profile_imported_by_the_cli_is_read_from_the_file` | profilo scritto con `FileSecretStore`: `export` e `duplicate` riescono (G6, metà) |
  | | `secret_store_status_never_reads_a_secret` · `starting_the_app_never_asks_to_unlock` | il portachiavi finto conta le letture e gli sblocchi: zero dopo avvio, `profiles_list`, `profile_get`, `secret_store_status` |
  | | `no_command_returns_a_secret` (di B3 e B4, esteso) | con i segreti nel portachiavi finto |
  | `RubricaNotices.test.tsx` | `the_file_fallback_notice_stays_and_cannot_be_dismissed` · `no_notice_with_a_keyring` | — |
  | `Preview.test.tsx` | `the_preview_says_where_the_keys_will_go` | sostituisce `found_secrets_are_named_and_go_to_the_secret_store` |
  | `e2e/smoke.e2e.ts` | `an_imported_profile_lands_in_the_keyring` | dopo l'import dalla finestra `secret-tool search service galleria` ha le voci e `secrets/` è vuota |
  | | `without_a_keyring_the_window_says_so_and_writes_the_file` | `GALLERIA_SMOKE_KEYRING=absent` |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `keep the secrets of the window in the keyring and say when it is a file`

**Fine del gruppo A:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** `grep -rn "FileSecretStore" crates/galleria-gui/src` trova solo la costruzione dell'archivio a strati.

---

## Gruppo B · Connetti

### Task 4: `connect_profile`, `disconnect`, sostituzione e annunci di stato, in Rust

Scritto su **Q43 (a)**: la pagina consegna a Rust le frasi di stato già tradotte.

**Stima:** 6 h. **Scioglie:** S44. **File — crea:** `crates/galleria-gui/src/{calls.rs,session.rs}`,
`src/commands/connection.rs`, `tests/connection.rs`, `tests/support/scenarios.rs`. **Modifica:**
`src/{lib.rs,supervisor.rs,events.rs,error.rs,commands/mod.rs}`, `build.rs`, `capabilities/main.json`,
`tests/{config.rs,commands.rs}`, `ui/src/app/announce.ts` e la prova, `ui/src/daemon/commands.ts`, cataloghi.

**Produce:**

```rust
// calls.rs — nessun tipo di tauri. Una connessione per chiamata: apri, hello, chiama, chiudi (spec §3.3).
pub async fn connect(socket: &Path, params: &ConnectParams) -> Result<ConnectResult, CallError>;
pub async fn disconnect(socket: &Path) -> Result<(), CallError>;
pub enum CallError { Unreachable(String), Rpc { code: String, message: String, data: Option<Value> } }

// session.rs — nessun tipo di tauri
pub struct Sessions { /* socket, Arc<Rubrica>, Arc<UiStateFile>, frasi, ultima terna annunciata */ }
impl Sessions {
    /// Legge profilo e segreti, chiama `to_connect_params`, manda `connect`. I parametri cadono alla fine.
    pub async fn connect_profile(&self, id: Uuid, replace: bool) -> Result<(), SessionFailure>;
    pub async fn disconnect(&self) -> Result<(), SessionFailure>;
    pub fn set_phrases(&self, phrases: Phrases);
    /// Chiamata dal supervisor per ogni `event.state`, prima che l'evento vada alla pagina: «ultimo uso» a
    /// `connected` (P15) e, se la terna è cambiata, la frase da annunciare (P14).
    pub fn on_state(&self, status: &Status) -> Option<(String, Urgency)>;
}
#[derive(Deserialize)] pub struct Phrases { pub connected: String, pub disconnected: String, pub reconnecting: String,
    pub error: BTreeMap<String, String>, pub error_other: String }      // «{name}» è il solo segnaposto
```

- `connect_profile`: `already_active` esce con `data.foreign` (P2, punto 6). `gui_secret_missing` se
  `to_connect_params` non trova una chiave; `gui_keyring_locked` se il portachiavi è bloccato; `gui_daemon_unreachable`
  se la connessione breve non si apre. `connect` con `replace: true` resta in attesa finché il demone ha smontato la
  sessione di prima: il comando è asincrono e non blocca gli eventi.
- `announce_phrases(phrases)`: la pagina lo chiama al montaggio e a ogni cambio di lingua (`ui/src/app/announce.ts`,
  che resta l'unico file che parla di annunci). Stringhe lunghe al più 200 caratteri, ripulite con `announce::clean`
  di B3. Finché le frasi non sono arrivate Rust non annuncia.
- Il supervisor, per ogni `event.state`, chiama `Sessions::on_state` e passa la frase all'`Announcer` di B3 (ATK, sul
  thread principale: **S44**), poi emette `galleria:state` come oggi. La pagina **non** chiama `announce` per gli stati.

- [ ] **1. Prove che falliscono.** Contro il demone vero con `FakeBackend` (`tests/support`), con gli scenari nuovi.

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/connection.rs` | `connect_profile_sends_what_the_cli_would_send` | il demone finto registra i `ConnectParams`: uguali a `to_connect_params` sullo stesso profilo |
  | | `connect_reaches_connected_through_the_events` | `galleria:state`: `connecting`, poi `connected` con il `profile.id` giusto |
  | | `disconnect_returns_to_disconnected` · `disconnect_without_a_session_is_not_an_error_for_the_page` | `not_active` passa uguale |
  | | `a_second_profile_of_the_rubrica_is_already_active_and_not_foreign` | `data.foreign == false`, `data.profile.name` |
  | | `a_session_outside_the_rubrica_is_foreign` | sessione avviata da un altro client con un id che la rubrica non ha (G9) |
  | | `an_unreadable_rubrica_makes_the_session_foreign` | punto 6 |
  | | `replace_true_swaps_the_session_and_never_runs_two_backends` | il contatore `live` del demone finto non supera 1 |
  | | `disconnecting_a_foreign_session_is_not_automatic` | nessun comando disconnette senza che la pagina chiami `disconnect` (G9) |
  | | `a_missing_key_is_gui_secret_missing` · `a_locked_keyring_is_reported_and_nothing_is_sent` | il demone finto non riceve `connect` |
  | | `without_the_daemon_connect_is_gui_daemon_unreachable` | — |
  | | `connected_touches_the_last_use_of_a_rubrica_profile_only` | P15; una sessione estranea non scrive `state.toml` |
  | | `state_changes_are_announced_once_each` | con le frasi date: `connected` → «Connesso a Ufficio», *polite*; tre eventi `connected` di fila (statistiche, `detail`) → un annuncio |
  | | `an_error_is_announced_assertive_with_its_translated_text` · `connecting_and_needs_input_are_not_announced` | P14 |
  | | `no_phrases_no_announcement` | — |
  | | `no_command_returns_a_secret` · `no_event_carries_a_secret` | risposte ed eventi di un giro intero, con `SEED-SECRET-` nelle chiavi del profilo (G7) |
  | | `stderr_never_carries_connect_params` | l'uscita di errore catturata durante `connect_profile` fallita e riuscita |
  | `app/announce.test.ts` | `phrases_are_sent_at_start_and_on_language_change` · `state_events_do_not_call_announce` | — |
  | `tests/config.rs` | elenco dei permessi | più `allow-connect-profile`, `allow-disconnect`, `allow-announce-phrases` |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `connect and disconnect a profile from Rust and announce the state changes`

**Fatto quando:** `cargo tree -p galleria-gui -e normal` non contiene `galleriad` né un backend; i `ConnectParams` non
compaiono in nessun `Debug`, `eprintln!` o payload (cercato con `grep -n "ConnectParams" crates/galleria-gui/src`).

---

### Task 5: pulsante, sette stati, statistiche, errori tradotti

**Stima:** 6 h. **File — crea:** `ui/src/connection/{ConnectButton,StatusPanel,Stats,ReplaceConfirm}.tsx`,
`ui/src/connection/store.ts`, con `.css` e prove. **Modifica:** `ui/src/shell/{Shell,ProfileDetail,ProfileList,StatusBar,ServiceArea}.tsx`
e le prove, `ui/src/shell/keyboard.test.tsx`, `ui/src/shell/boundary.test.ts`, `crates/galleria-gui/examples/fake-daemon.rs`,
`lab/tests/gui-lib.sh` (`daemon-start <scenario>`), cataloghi, `ui/e2e/{smoke.e2e.ts,support.ts}`.

**Produce:**

- **Pulsante principale** nel dettaglio del profilo e nel menu della riga: «Connetti» · «Annulla» (`connecting`,
  `needs_input`, `reconnecting`) · «Disconnetti» (`connected`) · spento con «Disconnessione in corso»
  (`disconnecting`) · «Riprova» più «Chiudi» (`error`: «Chiudi» è `disconnect`, che riporta a `disconnected`). `Invio`
  sulla riga selezionata è il pulsante principale. Con il collegamento al demone non `connected` il pulsante è spento
  e `aria-describedby` porta al motivo (la fascia di B1).
- **Sostituzione (D28):** «Connetti» su un altro profilo con una sessione propria attiva: `connect_profile(id,
  false)`, alla risposta `already_active` con `foreign: false` subito `connect_profile(id, true)` e un avviso a
  scomparsa «Disconnesso da «X»». Con `foreign: true`: la conferma del T8 (fino ad allora l'errore in linea).
- **Stati:** i sette di `State` con il testo dei cataloghi, `detail` (P10), il numero del tentativo durante
  `reconnecting` («tentativo N»), il tempo da `since` per `connected`.
- **Statistiche (P9):** indirizzi, server (`remote`), interfaccia, DNS e rotte effettivi da `Status.net` in sola
  lettura (spec §16, punto 2), byte ricevuti e mandati, ultimo handshake. Si aggiornano con `galleria:stats` senza
  ridisegnare il resto; non sono una regione viva (Q27, regola 3).
- **Errori:** `Status.error.code` e i codici dei comandi con il testo del catalogo; codice sconosciuto → `message`
  inglese (D22). Un errore di comando (`gui_secret_missing`, `backend_unavailable`, `profile_rejected`…) è un avviso in
  linea nel dettaglio, che prende il fuoco; lo stato `error` della sessione è il pannello di stato, annunciato da Rust.
- **Barra in basso:** accanto allo stato del collegamento, la sessione: «Connesso a «X» · 10.8.0.2».
- **Scenari del demone finto** (P16): `plain` (connette), `credentials`, `otp`, `passphrase`, `auth-failed`,
  `unreachable`, `stats` (byte che crescono), `slow` (resta in `connecting`).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `ConnectButton.test.tsx` | `the_button_names_the_action_for_each_of_the_seven_states` | parametrica |
  | | `the_button_follows_the_events_not_the_answer` | punto 8 |
  | | `cancel_while_connecting_is_a_disconnect` · `dismiss_on_error_is_a_disconnect` | — |
  | | `without_the_daemon_the_button_is_disabled_and_says_why` | — |
  | | `enter_on_the_selected_row_connects` | — |
  | `store.test.ts` | `an_own_session_is_replaced_at_once_with_one_notice` | due chiamate, la seconda con `replace: true`; un solo avviso «Disconnesso da «X»» |
  | | `a_foreign_session_is_never_replaced_without_the_confirmation` | punto 7 |
  | | `a_command_error_takes_the_focus_and_keeps_the_state` | — |
  | `StatusPanel.test.tsx` | `each_state_has_its_text_in_both_languages` · `an_unknown_detail_is_shown_raw` · `an_unknown_error_code_shows_the_daemon_message` | D22 |
  | `Stats.test.tsx` | `bytes_and_handshake_follow_the_stats_events` · `no_handshake_row_without_the_capability` · `the_numbers_are_not_a_live_region` | P9 |
  | `ProfileList.test.tsx` | `the_row_of_the_active_profile_says_its_state` (di B3) | resta verde con gli stati veri |
  | `keyboard.test.tsx` | `a_connection_needs_no_pointer` | selezione, `Invio`, stato, `Invio` di nuovo per disconnettere |
  | `has_no_accessibility_violations` | per ogni componente nuovo | — |
  | `e2e/smoke.e2e.ts` | `a_profile_connects_and_disconnects` | scenario `plain`: stato «Connected», indirizzo nella barra, poi «Disconnected» |
  | | `the_numbers_move_while_connected` | scenario `stats`: i byte letti due volte crescono |
  | | `switching_profile_replaces_without_asking` | un avviso, un solo profilo attivo |
  | | `an_unreachable_server_shows_the_translated_error_in_both_languages` | scenario `unreachable`, cambio di lingua a caldo |
  | | `rubrica_works_with_the_daemon_stopped` (di B3) | «Connect» c'è ed è spento |

- [ ] **2.** Falliscono. **3.** Implementa. P10: l'elenco dei `detail` si ricava leggendo `galleriad/src/manager.rs` e
  le `Progress` dei due backend, e va nell'esito. **4.** I comandi del task.
- [ ] **5. Commit:** `connect from the window and show the state, the numbers and the errors`

**Fatto quando:** nessun testo di stato o d'errore è scritto in un componente (`boundary.test.ts`); il mockup e la
finestra differiscono solo per i punti 4 e 5 del §16 della spec.

---

### Task 6: credenziali su `needs_input`, «Ricorda», «Dimentica»

Scritto su **Q46 (a)** e **Q47 (a)**.

**Stima:** 6 h. **Scioglie:** S43. **File — crea:** `ui/src/connection/CredentialsDialog.tsx` con `.css` e prova,
`crates/galleria-gui/tests/credentials.rs`. **Modifica:** `crates/galleria-gui/src/{calls.rs,session.rs,commands/connection.rs,error.rs}`,
`build.rs`, `capabilities/main.json`, `tests/config.rs`, `ui/src/connection/{store.ts,StatusPanel.tsx}`,
`ui/src/shell/tabs/Authentication.tsx` e la prova, `ui/src/shell/boundary.test.ts`, cataloghi, `ui/e2e/smoke.e2e.ts`.

**Produce:**

```rust
// calls.rs
pub async fn provide_input(socket: &Path, params: ProvideInputParams) -> Result<(), CallError>;   // prende e consuma
// session.rs
impl Sessions {
    pub async fn provide_input(&self, session: u64, request: u64, values: InputValues, remember: bool)
        -> Result<(), SessionFailure>;
    pub fn forget_credentials(&self, id: Uuid) -> Result<(), SessionFailure>;
}
struct Pending { session: u64, profile: Uuid, username: String, password: Secret }   // Debug scritto a mano: nessun valore
```

- Comandi: `provide_input(session, request, values, remember) -> ()`, `forget_credentials(id) -> ()`.
- **Finestra delle credenziali** (modale, prende il fuoco sul primo campo; Q27, regola 2): si apre quando
  `Status.state == needs_input` e `input_request.request` è nuova; **solo** il campo di `kind`:
  `credentials` → «Utente» e «Password», più «Ricorda» · `otp` → un campo, con il `prompt` del server com'è, non
  tradotto, come etichetta · `key_passphrase` → «Passphrase della chiave». `echo: false` → `type="password"`. Il conto
  alla rovescia è testo, aggiornato ogni 10 s e nell'ultimo minuto ogni secondo, non annunciato. «Connetti» e «Annulla»
  (P7). Il nome del profilo nel titolo. Se l'utente è ricordato e manca solo la password, «Utente» non si mostra: lo
  aggiunge Rust.
- **«Il viaggio di una credenziale»**, tappe 1–6, com'è scritto sopra.
- **Alla connessione** (in `Sessions::connect_profile`): se `secret_refs` nomina `openvpn.username` e
  `openvpn.password`, Rust li legge e li mette nei parametri (P5).
- **«Ricorda»** a `connected`: `set` delle due chiavi, poi `secret_refs` del profilo aggiornato (`ProfileStore::save`,
  con `modified` che non cambia l'ordine della lista: si scrive solo se i nomi mancano).
- **«Dimentica»** (P6): `delete` delle due chiavi e i due nomi fuori da `secret_refs`. Nella scheda Autenticazione
  («Credenziali salvate nel portachiavi» · «Dimentica») e nel pannello di un `auth_failed` la cui sessione ha mandato
  credenziali ricordate («Le credenziali salvate non sono state accettate.» · «Dimentica le credenziali salvate»).
  Che la sessione abbia usato credenziali salvate la pagina lo ricava da ciò che ha già: `secret_refs` del profilo le
  nomina e `error.code` è `auth_failed`. `Status` è del demone e non guadagna campi.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/credentials.rs` | `provide_input_reaches_the_daemon_and_nothing_else` | il backend finto riceve i valori; risposta `()`; nessun evento li contiene |
  | | `remember_saves_only_after_connected` | prima di `connected` l'archivio non ha le chiavi; dopo sì, e `secret_refs` le nomina |
  | | `a_failed_login_saves_nothing` · `a_disconnect_before_connected_saves_nothing` | scenario `auth-failed` |
  | | `otp_and_passphrase_are_never_kept` | `remember: true` con `kind` `otp` e `key_passphrase`: niente nell'archivio, `Pending` vuoto |
  | | `the_next_connection_sends_the_remembered_credentials` | i `ConnectParams` registrati hanno utente e password; nessun `needs_input` |
  | | `a_remembered_credential_gone_from_the_store_is_not_an_error` | P5 |
  | | `forget_removes_the_two_keys_and_their_names` | le chiavi del profilo (`openvpn.block.*`) restano |
  | | `an_expired_request_is_input_not_expected_and_changes_nothing` | — |
  | | `pending_is_dropped_when_another_session_starts` | — |
  | | `no_command_returns_a_secret` · `no_event_carries_a_secret` (estese) | dopo un giro con password, OTP e passphrase: nessuna risposta, nessun evento e nessun `Debug` contiene i tre valori (G7) |
  | `CredentialsDialog.test.tsx` | `only_the_asked_field_is_shown` | parametrica sui tre `kind` (G5) |
  | | `the_server_prompt_is_shown_untranslated` | «Enter OTP» anche in italiano |
  | | `remember_is_offered_for_credentials_only` | — |
  | | `the_dialog_takes_the_focus_and_is_not_announced` | Q27, regola 2 |
  | | `the_fields_are_empty_before_the_answer_arrives` · `provide_input_is_invoked_once_per_submit` | tappa 2 |
  | | `a_new_request_reopens_the_dialog_and_an_answered_one_does_not` | punto 9 |
  | | `the_countdown_starts_from_the_event` · `at_zero_the_dialog_closes` | punto 10 |
  | | `cancel_is_a_disconnect` · `a_click_outside_does_not_close_it` | P7 |
  | | `has_no_accessibility_violations` | — |
  | `shell/boundary.test.ts` | `the_credentials_dialog_keeps_no_value_in_state` | tappa 1: nel file nessun `useState`, nessuno store, nessun `console.`; `"provide_input"` compare in un solo file di `ui/src` |
  | `Authentication.test.tsx` | `saved_credentials_are_named_and_can_be_forgotten` | — |
  | `StatusPanel.test.tsx` | `a_refused_saved_password_offers_to_forget_it` | Q47 |
  | `e2e/smoke.e2e.ts` | `credentials_are_asked_once_and_remembered` | scenario `credentials`: finestra, «Ricorda», connesso; `secret-tool` ha le due voci; seconda connessione senza finestra |
  | | `the_otp_is_asked_every_time_and_never_saved` | scenario `otp` |
  | | `no_typed_value_is_left_in_the_page` | dopo l'invio: nessun `<input>` della pagina ha un valore; il testo della pagina non contiene la password di prova (**S43**) |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `ask for credentials only when the daemon does and remember them on request`

**Fine del gruppo B:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** la tabella «Il viaggio di una credenziale» ha una prova verde per ogni tappa; il limite dichiarato è
nell'esito con le stesse parole.

---

## Gruppo C · Log e sessione estranea

### Task 7: scheda Log con «Copia» ed «Esporta log…» (D20)

**Stima:** 5 h. **File — crea:** `crates/galleria-gui/src/logs.rs`, `src/commands/logs.rs`, `tests/logs.rs`,
`ui/src/log/{LogTab.tsx,store.ts}` con `.css` e prove, `ui/e2e/connection.a11y.ts`. **Modifica:**
`crates/galleria-gui/src/{calls.rs,lib.rs}`, `build.rs`, `capabilities/main.json`, `tests/{config.rs,export.rs}`,
`ui/src/shell/ProfileDetail.tsx`, cataloghi, `ui/e2e/smoke.e2e.ts`.

**Produce:**

```rust
// calls.rs
pub async fn logs(socket: &Path, after: Option<u64>, limit: usize) -> Result<LogsResult, CallError>;
// logs.rs — nessun tipo di tauri
/// Tutte le righe del buffer, a pagine da 1000 finché una pagina è corta.
pub async fn all(socket: &Path) -> Result<Vec<LogEntry>, CallError>;
pub fn render(entries: &[LogEntry]) -> String;                 // P12: il formato di `galleria logs`
pub fn suggested_name(now: OffsetDateTime) -> String;          // galleria-log-AAAAMMGG-HHMM.txt
```

- Comandi: `logs_get(after: Option<u64>, limit: usize) -> { entries, last }` (limite al più 1000, come il demone);
  `logs_export() -> { saved }`: **nessun parametro**. Rust riscarica il buffer, chiede il posto con `SavePicker`
  (B3) e scrive con `write_export` (`0600`). Annullato → `{ saved: false }`. Il percorso non torna alla pagina.
- Scheda **Log** nel dettaglio, per il profilo della sessione attiva o dell'ultima: titolo «Sessione corrente» se lo
  stato non è `disconnected`, altrimenti «Ultima sessione»; all'apertura `logs_get` a pagine, poi `galleria:log`
  (righe con `seq` già vista si scartano); al più 2000 righe (P11). Elenco con `role="log"`, **senza** annunci (Q27,
  regola 3); resta in fondo solo se l'utente era in fondo. Livello e sorgente come testo, non solo come colore. Le
  righe non si traducono. «Copia» e «Esporta log…»; esito con un avviso («Log copiato.», «Log esportato.»).
- Per un profilo che non è quello della sessione la scheda dice «Nessuna sessione per questo profilo» e non mostra
  righe di altri.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/logs.rs` | `all_pages_until_a_short_page` | 2000 righe nel buffer del demone finto: due chiamate piene e una corta, nessuna riga doppia o persa |
  | | `logs_get_refuses_more_than_a_thousand` | `gui_invalid_field`, senza chiamare il demone |
  | | `export_writes_what_logs_returned` | con un `SavePicker` finto: i byte del file sono `render(all())`; file `0600` |
  | | `export_cancelled_writes_nothing` | `{ saved: false }` |
  | | `the_export_never_carries_a_known_secret` | sessione finta avviata con `SEED-SECRET-` come password e chiave, il backend finto le ripete in una riga di log: nel file c'è `[redatto]` e non il valore (G7) |
  | | `the_answer_holds_no_path` | — |
  | `tests/export.rs` | `no_command_takes_a_path` (di B3) | resta verde: `logs_export` non ha parametri |
  | `tests/config.rs` | elenco dei permessi | più `allow-logs-get`, `allow-logs-export` |
  | `LogTab.test.tsx` | `the_title_follows_the_state` | parametrica sui sette stati |
  | | `live_lines_are_appended_and_duplicates_dropped` · `at_most_two_thousand_lines` | P11 |
  | | `lines_are_not_translated` · `level_and_source_are_text` | — |
  | | `the_log_is_not_a_live_region_that_announces` | nessun `aria-live="assertive"`, nessuna chiamata a `announce` per riga |
  | | `copy_and_export_say_the_outcome_once` | un avviso ciascuno |
  | | `another_profile_shows_no_lines` | — |
  | | `has_no_accessibility_violations` | — |
  | `e2e/smoke.e2e.ts` | `the_log_tab_shows_the_session_and_copies_it` | scenario `plain`: righe del demone finto; `xclip -o` dopo «Copy» |
  | `e2e/connection.a11y.ts` | `K5e_the_log_is_exported_through_the_real_save_window` | con `atspi-file-window.py save`: file scritto, `0600`, Orca dice «Log exported.» una volta |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `show the log of the session and export it through the save window`

**Fatto quando:** `grep -rn "eprintln" crates/galleria-gui/src/logs.rs crates/galleria-gui/src/commands/logs.rs` non
trova niente che stampi una riga del log.

---

### Task 8: sessione estranea (D21)

**Stima:** 4 h. **File — crea:** `ui/src/connection/ForeignRow.tsx` con `.css` e prova,
`crates/galleria-gui/tests/foreign.rs`. **Modifica:** `ui/src/connection/{store.ts,ReplaceConfirm.tsx,ConnectButton.tsx}`,
`ui/src/shell/{Shell,ProfileList,StatusBar,ProfileDetail}.tsx` e le prove, `ui/src/shell/keyboard.test.tsx`, cataloghi,
`ui/e2e/smoke.e2e.ts`.

**Produce:**

- **Estranea** = `Status.state != disconnected` e `Status.profile.id` non è tra i profili di `profiles_list`. La
  pagina lo ricalcola quando arriva uno stato e quando la rubrica si rilegge (un profilo eliminato dalla CLI rende
  estranea la sua sessione).
- **Riga fissa** in cima alla lista, fuori dai gruppi e dalla ricerca: nome da `Status.profile.name`, protocollo,
  stato, e il testo «avviata fuori da questa rubrica». Selezionabile: il dettaglio mostra il pannello di stato, le
  statistiche e la sola scheda Log (punto 12, P13).
- **Barra in basso:** «Connessione attiva: «nome» — avviata fuori da questa rubrica».
- **«Disconnetti»** sulla riga e nel dettaglio: conferma («Questa connessione non è partita da questa rubrica. Un
  altro utente di questo computer potrebbe starla usando. Disconnettere?»), fuoco su «Annulla». Solo dopo, `disconnect`.
- **«Connetti»** su un profilo proprio con una sessione estranea attiva: `already_active` con `foreign: true` → la
  stessa conferma con «Sostituisci», poi `connect_profile(id, true)`.
- Le conferme sono `AlertDialog` del design system: prendono il fuoco e non annunciano (Q27, regola 2).

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tests/foreign.rs` | `a_session_of_a_deleted_profile_becomes_foreign` | il profilo si elimina da fuori mentre è connesso: `already_active` di un altro `connect_profile` ha `foreign: true` |
  | | `delete_of_the_active_profile_is_still_refused` (di B3) | resta verde |
  | `ForeignRow.test.tsx` | `a_session_outside_the_rubrica_gets_the_fixed_row_and_the_bar_text` | — |
  | | `the_row_ignores_search_and_groups` | con una ricerca che non la trova, la riga c'è |
  | | `its_detail_has_the_state_and_the_log_only` | nessuna scheda del profilo, nessun «Elimina», «Duplica», «Esporta» |
  | | `disconnect_asks_first_and_cancel_does_nothing` | nessuna chiamata a `disconnect` prima di «Disconnetti» nella conferma |
  | | `the_confirmation_focuses_cancel` | — |
  | | `connecting_over_it_asks_first_then_replaces` | una chiamata con `replace: false`, conferma, una con `replace: true` |
  | | `when_the_profile_appears_in_the_rubrica_the_row_goes` | — |
  | | `has_no_accessibility_violations` | — |
  | `keyboard.test.tsx` | `a_foreign_session_is_closed_without_a_pointer` | — |
  | `e2e/smoke.e2e.ts` | `a_session_started_by_the_cli_of_another_rubrica_is_foreign` | `galleria connect` (`GALLERIA_SMOKE_CLI`) con un'altra `XDG_CONFIG_HOME` contro il demone finto: riga fissa, testo della barra |
  | | `closing_a_foreign_session_asks_first` | «Cancel»: `galleria status` dice ancora `connected`; conferma: `disconnected` |

- [ ] **2.** Falliscono. **3.** Implementa. **4.** I comandi del task.
- [ ] **5. Commit:** `show a session started outside the rubrica and ask before closing it`

**Fine del gruppo C:** `scripts/gui.sh smoke` e `scripts/gui.sh a11y`, una volta, sull'ultimo commit.

**Fatto quando:** le differenze 4, 5 e 6 del §16 della spec sono nella finestra; contro il demone finto non resta
niente della spec §8 e §10 senza una prova.

---

## Gruppo D · Laboratorio e chiusura

### Task 9: la finestra parla con il demone vero dei client; `lab-connect`; G4

Scritto su **Q42 (b)**: un ponte di prova tra un socket sul lato dell'host e il socket del demone dentro il client. Se
Q42 passa come (a) (cartella del socket montata nei due contenitori) cambiano `lab/up.sh` e l'utente con cui parte
l'immagine della GUI: ci si ferma e si riporta prima di cominciare.

**Stima:** 7 h. **Scioglie:** S39, S40. **File — crea:** `lab/tests/{gui-daemon-relay.py,gui-lab-watch.sh,gui-lab-connect.sh}`,
`ui/e2e/lab-connect.e2e.ts`, `crates/galleria-e2e/tests/t18_gui_connect.rs`. **Modifica:** `scripts/gui.sh` (verbi
`lab-connect <client>`, `lab-connect-window`, `lab-connect-here`), `lab/run.sh` (la guardia di `t17` vale anche per
`t18` con `GALLERIA_E2E_GUI_CONNECT`; la variabile passa al client), `lab/README.md`, `ui/e2e/{vitest.config.ts,support.ts}`,
`.github/workflows/ci.yml` (job `lab-connect`), `lab/tests/gates-b5.tsv`, `docs/08-esito-fase2.md`.

**Produce:** `scripts/gui.sh lab-connect <26|27|79>`, sull'host, in quest'ordine.

1. `GALLERIA_E2E_GUI_CONNECT=1 LAB_BEFORE_TESTS='scripts/gui.sh lab-connect-window <client>' lab/run.sh <client> t18`:
   il laboratorio sale come sempre.
2. **`lab-connect-window`** (sull'host, tra `up.sh` e le prove):
   - nel client crea l'utente `carla` nel gruppo `galleria` (`docker exec … useradd -m -G galleria carla`): le immagini
     non cambiano;
   - nel client avvia **`gui-lab-watch.sh`** (`docker exec -d`): ogni 200 ms scrive in `/tmp/gui-lab-watch.log` l'ora,
     i processi `openvpn` con i loro argomenti, le interfacce `glr-*`; all'avvio scrive `residue` (le stesse righe di
     `galleria_e2e::residue`); una volta al secondo, con un tunnel su, fa un `ping` dentro il tunnel perché i byte si
     muovano;
   - sull'host avvia **due ponti** (`gui-daemon-relay.py`, da `spikes/b5-piano/relay.py`): `target/gui-lab/run/carla.sock`
     e `target/gui-lab/run/alice.sock`. Ogni connessione che arriva lancia `docker exec -i -u <utente> glab-client<N>
     python3 /lab/lab/tests/gui-daemon-relay.py join /run/galleria/galleriad.sock` e unisce i due flussi di byte. Il
     demone vede `carla` o `alice` **veri**, con il loro uid e i loro gruppi **[V: `spikes/b5-piano/`, sotto Docker
     rootless]**; il ponte non legge i messaggi; quando un lato chiude chiude l'altro;
   - lancia nell'immagine **`lab-connect-here`** (come `lab-import`: app costruita come per il rilascio, Xvfb, bus
     privato, portachiavi sbloccato del T2) con `GALLERIA_SOCKET=/work/target/gui-lab/run/carla.sock`,
     `GALLERIA_LAB_SOCKET_OTHER=…/alice.sock`, `GALLERIA_LAB_OUT`, `GALLERIA_LAB_CONNECT=1`; `stderr` dell'app va in
     `target/gui-lab/app.stderr`;
   - ferma i ponti e lancia `check-gates.py --job lab-connect --vitest-json target/gui-lab/vitest.json`. Un errore
     qui ferma `lab/run.sh`, che smonta il laboratorio.
3. **`t18_gui_connect`** nel client (ignorata come le altre; `lab/run.sh` non la lancia senza la variabile, e lo dice):
   legge ciò che il client ha visto mentre la finestra lavorava. Poi `check-gates.py --job lab-connect --cargo-log …`.

`ui/e2e/lab-connect.e2e.ts` (gira solo con `GALLERIA_LAB_CONNECT=1`, come `lab-import`). I profili del laboratorio
(`wg-split.conf`, `base.ovpn`) entrano dalla finestra come in B4. Prove di questo task (**G4**):

| Prova | Cosa fa e cosa asserisce |
|---|---|
| `G4a_wireguard_connects_and_the_numbers_move` | «Connect» su `wg-split`: «Connected»; l'indirizzo mostrato è quello di `wg-split.conf`; i byte letti due volte a 3 s crescono; «Last handshake» ha un valore |
| `G4b_openvpn_connects_after_the_credentials` | `base`: la finestra chiede utente e password (`alice`), poi «Connected» con un indirizzo `10.8.0.x`; nessuna riga «Last handshake» |
| `G4c_switching_profile_replaces_without_asking` | da `wg-split` a `base` e ritorno: un avviso «Disconnected from …» per cambio, nessuna conferma, alla fine un solo profilo «Connected» |
| `G4d_disconnect_returns_to_disconnected` | «Disconnect»: «Disconnected»; `galleria --socket carla.sock status` (CLI nell'immagine) dice `disconnected` |
| `G4e_cancel_while_connecting_is_a_disconnect` | `unreachable`: «Connecting», «Cancel», «Disconnected» entro 10 s |

`t18` (nel client):

| Prova | Asserzione |
|---|---|
| `gui_session_never_had_two_backends` | nel registro dell'osservatore nessun campione ha insieme un processo `openvpn` e l'interfaccia `glr-wg0`, né due processi `openvpn`; almeno un campione con ciascuno dei due (D5, come E2). **Fallisce** se il registro manca o ha meno di 20 campioni |
| `gui_session_left_no_residue` | `residue()` di adesso è uguale a quello scritto dall'osservatore all'avvio; nessun processo `openvpn` |

- [ ] **1. Prove che falliscono.** Le cinque `G4*` e le due di `t18`, con le loro righe in `gates-b5.tsv`.
- [ ] **2.** Falliscono. **3.** Implementa. Il `ping` dell'osservatore: gli indirizzi sono quelli che usano `t16`
  (`10.8.0.1` per `base`) e `t08` per WireGuard; si leggono lì.
- [ ] **4.** I comandi del task; `scripts/gui.sh lab-connect 26` (intero) e, su 27 e 79, le sole prove WireGuard con il
  filtro (**S40**: OpenVPN 2.7 sotto Docker rootless dà `EPERM` sul DCO; lì la prova è il job).
- [ ] **5. Commit:** `drive the real daemon of the lab clients from the window`

**Ripiego per S39** (il socket del ponte non è raggiungibile dall'immagine su un runner con Docker non rootless): il
ponte dell'host si lancia **dentro** l'immagine della GUI montando `/var/run/docker.sock`, solo nel job; se nemmeno
così, Q42 (a), che è misurata, e si riporta.

**Fatto quando:** l'esito ha l'uscita di `lab-connect 26` con i nomi delle sette prove e la riga finale di
`check-gates.py`; `lab/run.sh 26` senza la variabile non lancia `t18` e lo dice; il job `lab-connect` è in `ci.yml` con
la matrice dei tre client e **non** ha `continue-on-error`.

---

### Task 10: G5, G6, G7 e G9 nel laboratorio

**Stima:** 6 h. **Scioglie:** S41. **File — modifica:** `ui/e2e/{lab-connect.e2e.ts,support.ts}`,
`crates/galleria-e2e/tests/t18_gui_connect.rs`, `lab/tests/{gui-lab-connect.sh,gui-lab-watch.sh}`,
`lab/tests/gates-b5.tsv`, `docs/08-esito-fase2.md`.

Segreti noti del laboratorio, cercati dappertutto: `alice-secret`, `bob-secret`, `key-passphrase`, il codice OTP
usato, una riga della chiave privata di `base.ovpn`, la chiave privata di `wg-split.conf`.

| Prova (finestra) | Cosa fa e cosa asserisce |
|---|---|
| `G5a_only_the_asked_field_is_shown` | `base`: utente e password, nessun campo OTP · `otp` (utente `bob`): credenziali, poi **una seconda** finestra con il solo campo e l'etichetta «Enter OTP» · `enckey`: solo la passphrase. Ogni volta «Connected» |
| `G5b_remember_saves_to_the_keyring_and_the_next_connection_does_not_ask` | `base` con «Remember»: `secret-tool search service galleria` ha `openvpn.username` e `openvpn.password` di quel profilo; nessun file in `secrets/` li contiene; «Disconnect», «Connect»: nessuna finestra, «Connected» |
| `G5c_the_otp_is_never_saved` | `otp` con «Remember»: nel portachiavi utente e password, **nessuna** voce con il codice; alla connessione dopo la finestra chiede solo l'OTP |
| `G5d_the_passphrase_is_never_saved` | `enckey` due volte: la passphrase è chiesta tutte e due; nessuna voce nel portachiavi la contiene |
| `G5e_a_refused_saved_password_offers_to_forget_it` | password sbagliata con «Remember» → errore tradotto e **niente** nel portachiavi; poi una voce sbagliata messa a mano con `secret-tool` (da stdin): «Authentication failed», «Forget the saved credentials», la connessione dopo chiede |
| `G6a_a_profile_imported_by_the_cli_connects_from_the_window` | `galleria import` (CLI nell'immagine, stessa `XDG_CONFIG_HOME`) di `wg-split.conf`: i segreti sono nel file; la finestra lo elenca e lo connette |
| `G6b_a_profile_imported_by_the_window_connects_from_the_cli` | `base.ovpn` importato dalla finestra: chiavi nel portachiavi, `secrets/` senza quel profilo; `galleria --socket carla.sock connect base --username alice --password-stdin`: `connected`; la finestra mostra la sessione come **propria** |
| `G7a_no_known_secret_reaches_the_log_the_export_or_the_stderr` | dopo `otp` ed `enckey`: il testo della scheda Log, il file di «Export log…», `target/gui-lab/app.stderr`, `/proc/*/cmdline` e `/proc/*/environ` dei processi dell'utente nell'immagine non contengono nessuno dei segreti noti |
| `G9a_a_session_started_by_another_user_is_shown_as_foreign` | la CLI come `alice` (altro ponte, altra `XDG_CONFIG_HOME`) importa e connette `wg-split`; la finestra (`carla`) mostra la riga fissa e il testo della barra; la scheda Log ha righe |
| `G9b_disconnecting_it_asks_first` | «Disconnect», «Cancel»: `galleria --socket alice.sock status` dice ancora `connected`; di nuovo, conferma: `disconnected` |
| `G9c_replacing_it_asks_first` | `alice` riconnette; «Connect» su un profilo di `carla`: conferma, «Replace», «Connected» sul profilo di `carla` |

| Prova (`t18`, nel client) | Asserzione |
|---|---|
| `gui_session_left_no_secret_in_journal_or_argv` | `journalctl -u galleriad` e ogni riga di argomenti nel registro dell'osservatore non contengono i segreti noti (il codice OTP usato la finestra lo lascia in `target/gui-lab/used-otp`, letto da `/lab`) |
| `gui_session_was_driven_by_two_users` | nel log del demone ci sono `uid <alice> connect` e `uid <carla> disconnect`, con i due uid veri del client |

Il file esportato in `G7a` passa dalla finestra «Salva» vera, pilotata da `atspi-file-window.py` **senza Orca**
(**S41**: il bus AT-SPI e `toolkit-accessibility` bastano; oggi la sonda gira solo sotto `a11y`).

- [ ] **1. Prove che falliscono.** Le tredici sopra, con le righe nel manifesto.
- [ ] **2.** Falliscono. **3.** Correggi ciò che trovano, nel task a cui appartiene (`--fixup`).
- [ ] **4.** I comandi del task; `scripts/gui.sh lab-connect 26`.
- [ ] **5. Commit:** `prove credentials, both stores, secrets and the foreign session in the lab`

**Ripiego per S41:** se la sonda non regge senza Orca nel job, in `G7a` il file esportato si controlla al livello 2
(`the_export_never_carries_a_known_secret`, job `gui`, che entra nel manifesto per G7) e il passaggio dalla finestra
vera resta a `K5e`, locale; l'esito lo dice.

**Fatto quando:** `gates-b5.tsv` ha per `lab-connect` le venti prove (cinque `G4`, undici qui, quattro di `t18`) e
`scripts/gui.sh lab-connect 26` finisce con la riga verde di `check-gates.py`.

---

### Task 11: tastiera, fumo, screen reader, esito del blocco

**Stima:** 4 h. **File — modifica:** `ui/src/shell/keyboard.test.tsx`, `ui/e2e/{smoke.e2e.ts,connection.a11y.ts}`,
`crates/galleria-gui/tests/connection.rs`, `README.md`, `docs/08-esito-fase2.md`, `ROADMAP.md`,
la spec (§4.2, §9, §17, §18), `.claude/handoffs/`.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `keyboard.test.tsx` | `a_full_connection_tour_needs_no_pointer` | connetti, credenziali, «Ricorda», Log, «Copia», disconnetti, sostituzione, sessione estranea con le due conferme |
  | | `focus_is_never_lost_to_the_body` (di B2) | esteso: dopo la finestra delle credenziali chiusa (inviata, annullata, scaduta), dopo le conferme, dopo un errore di comando |
  | | `no_outcome_depends_on_a_live_region_alone` (di B3) | esteso agli avvisi di B5 |
  | `tests/connection.rs` | `no_command_returns_a_secret_after_a_full_round` | ogni comando di B5 e ogni evento, con chiavi, password, OTP e passphrase noti (G7) |
  | `e2e/smoke.e2e.ts` | `no_csp_violation_during_the_connection_tour` | — |
  | | `the_cli_sees_the_session_the_window_started` | `galleria status` |
  | `e2e/connection.a11y.ts` | `K5a_connected_is_said_once_polite` | riga `object:announcement` con urgenza 1 e «Connected to …», una volta, senza che la pagina chiami `announce` |
  | | `K5b_an_error_is_said_assertive` | scenario `auth-failed` |
  | | `K5c_the_credentials_dialog_is_read_as_a_dialog` | albero: `dialog` con il nome, fuoco sul primo campo; nessun annuncio |
  | | `K5d_the_confirmation_of_a_foreign_session_is_read` | ruolo e fuoco su «Cancel» |
  | | `K5f_the_switch_says_disconnected_then_connected` | l'avviso e l'annuncio di stato, in quest'ordine, una volta ciascuno |

- [ ] **2.** Falliscono. **3.** Correggi ciò che le prove trovano, nei componenti a cui appartiene.
- [ ] **4.** `scripts/gui.sh check`; gruppo dell'host; `scripts/gui.sh keyring`; poi, **una volta sola in tutto il
  blocco**: `scripts/gui.sh smoke` **tre volte di fila**, `scripts/gui.sh a11y` **due volte**,
  `scripts/gui.sh lab-connect 26` **due volte**.
- [ ] **5. Esito e roadmap.** In `docs/08-esito-fase2.md`, sezione «B5»: tabella C1–C8 e **G4, G5, G6, G7, G9** con
  prova, job ed esito (locale e, quando c'è, il numero del run); S12 della spec e S34–S45 con esito e ripiego; il
  limite dichiarato di «Il viaggio di una credenziale»; la trascrizione di Orca; numero di prove per livello; «Non
  verificato», che contiene almeno: KWallet se S38 non è passata; GNOME Keyring su un desktop vero (sblocco con la
  finestra del sistema); la finestra su Ubuntu 26.04 e Fedora; OpenVPN 2.7 in locale; ciò che aspetta il run di CI.
  `README.md`: portachiavi e ripiego sul file; chi è nel gruppo `galleria` comanda la VPN della macchina (D21);
  `journalctl -u galleriad` (D20). `ROADMAP.md`: le caselle di «B5» secondo i fatti. Nella spec: §4.2 (i comandi
  definitivi), §9 (`delete`, il crate scelto), §17 (S12), §18 (esito di Q42–Q47). Handoff in `.claude/handoffs/`.
- [ ] **6. Commit:** `cover the connection with keyboard, smoke and screen reader tests`

**Fatto quando:** C1–C8 hanno ciascuno una prova verde, oppure l'esito dice quale non lo è e perché; nessuna casella
della roadmap è spuntata senza la sua prova; la card dice in chiaro che G4–G9 sui tre client aspettano il run di CI.

---

## Copertura della spec

| Spec | Dove |
|---|---|
| §4.2 comandi di B5 | T3 (`secret_store_status`), T4 (`connect_profile`, `disconnect`, `announce_phrases` in più: P1), T6 (`provide_input`, `forget_credentials`), T7 (`logs_get`, `logs_export`) |
| §3.2 regole 2, 3, 5 | C6: «Il viaggio di una credenziale», T6; `no_command_takes_a_path`, T7; `G7a`, T10 |
| §3.3 connessioni brevi per i comandi | T4 `calls.rs` |
| §8 connetti e `already_active`; sessione estranea; credenziali; errori | T4, T5; T8; T6; T5 |
| §9 archivio dei segreti, regole 1–5 | T1 (1, 2, 3), T6 (4), T3 (5); il crate: Q44, T1 |
| §9 «provandoli con GNOME Keyring e KWallet» | T2 (GNOME Keyring in CI; KWallet con un tetto, S38) |
| §10 Log | T7 |
| §12.1 annunci dei cambi di stato dal lato Rust | T4, con Q43 |
| §14 «G3–G6 e G9 si chiudono nel laboratorio» | T9, T10, con Q42 |
| §16 punti 4, 5, 6 | T5, T6, T8 |
| G4, G5, G6, G7, G9 | C1–C5 e «Dove gira ogni gate in CI» |
| D32 «ultimo uso» | T4, P15 |

Punti [S]. Quello della spec che B5 doveva sciogliere:

| # | Supposizione | Stato dopo questo piano | La chiude | Se è falsa |
|---|---|---|---|---|
| S12 | Crate per Secret Service e comportamento con GNOME Keyring e KWallet | **misurato** il costo di cinque candidati (crate, `cargo deny`, build); non eseguito contro un portachiavi | T1, T2 | ripiego sul file (D25); crate di riserva in Q44 |

Punti [S] di questo piano (la numerazione continua da S33):

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S34 | `dbus-secret-service` con `vendored` si compila e parte nell'immagine della GUI insieme al `dbus` di `tao` (stesso crate, feature unite) | T1 | Q44 (b): `secret-service` 6 su tokio |
| S35 | `gnome-keyring-daemon --unlock` da stdin funziona senza schermo, nell'immagine e sul runner | T2 | Secret Service di prova con `python3-gi` |
| S36 | Un bus di sessione senza cartelle dei servizi dà il caso «servizio assente» anche con GNOME Keyring installato | T2 | si toglie il bus; «bus senza servizio» resta al portachiavi finto |
| S37 | Un portachiavi bloccato senza nessuno che chieda la password risponde «bloccato» in pochi secondi, senza restare appeso | T2 | attesa massima dello sblocco (60 s nell'app, 3 s nelle prove) |
| S38 | KWallet si comporta come Secret Service con lo stesso crate | T2, tetto di un'ora | «non verificato», README, passata di B6 |
| S39 | Il socket del ponte, creato sull'host in `target/`, è raggiungibile dall'immagine anche con Docker non rootless (runner, uid 1001) | T9, sul primo run | ponte dentro l'immagine con il socket di Docker; poi Q42 (a) |
| S40 | In locale OpenVPN 2.7 (client 27 e 79) resta rosso per il DCO negato da Docker rootless; sul runner è verde come in `lab-g3` | T9 | è già così per `t13`, `t16`, `t17`: si scrive |
| S41 | La sonda AT-SPI pilota la finestra «Salva» nel job senza Orca | T10 | export al livello 2 più `K5e` locale |
| S42 | `event.stats` arriva una volta al secondo da `connected` e `last_handshake` è una data che si può mostrare | T5, leggendo i backend | il campo si mostra grezzo |
| S43 | WebDriver scrive nei campi `password` e vede i campi svuotati dopo l'invio | T6 | l'asserzione resta al livello 1 |
| S44 | L'annuncio ATK emesso mentre arriva un evento del demone (thread della consegna) passa dal thread principale come quello del comando `announce` | T4, `K5a` | l'annuncio si accoda con `run_on_main_thread`, come fa già `AtkAnnouncer` |
| S45 | Senza bus di sessione (SSH, client del laboratorio) la CLI con l'archivio a strati non aspetta e non stampa niente di nuovo | T2; `t16` nel job `lab` | `KeyringSecretStore::open` non si chiama se `DBUS_SESSION_BUS_ADDRESS` manca |

## Cosa resta fuori da B5, per scelta

- Tray, chiusura nella tray, avviso di versione, chiusura con modifiche non salvate (Q35): B6.
- La finestra delle credenziali come finestra GTK aperta da Rust: la spec la mette nella pagina.
- La CLI che usa le credenziali ricordate (Q45); la migrazione automatica dal file al portachiavi (Q20).
- Storico dei log, file di log, «Apri cartella» (D20). Proprietario della sessione (D21).
- Kill switch, connessione automatica, avvio con il sistema (dopo la v0.1).
- La finestra su Ubuntu 26.04 e Fedora dentro il laboratorio: la finestra gira nell'immagine Ubuntu 24.04 e comanda i
  demoni dei tre client (Q42); le altre due distribuzioni per la finestra restano alla passata manuale di B6.
- Prova di Orca in CI: resta locale (Q22). G12 non è tra i gate di B5.
- Notifiche del desktop per i cambi di stato (Q27).

## Cosa ho verificato e cosa no

Verificato oggi (26 novembre 2026), in questo worktree a `900ccf8`:

- Il codice della tabella «Il codice della fase 1 che B5 usa», letto file per file; `ci.yml`, `scripts/gui.sh`,
  `lab/run.sh`, `lab/up.sh`, le immagini del laboratorio, `gui-lib.sh`, `gui-lab-import.sh`, `t17`.
- Il costo dei cinque crate candidati: `Cargo.lock`, `cargo deny check` (cargo-deny 0.20.2) e build della CLI
  sull'host, **eseguiti**; il worktree è tornato pulito (`git status`).
- I due modi di raggiungere un socket in un client da un altro contenitore (cartella montata, ponte), **eseguiti** con
  Docker 29.7.2 rootless, le immagini `glab-client26` e `galleria-gui-dev` e un server di prova che stampa
  `SO_PEERCRED` e `SO_PEERGROUPS`: uscita in `spikes/b5-piano/README.md`.
- `InvokeBody::Json` e la deserializzazione degli argomenti dei comandi in `tauri` 2.12.7 (`ipc/mod.rs`,
  `ipc/command.rs`), **letti**.
- La licenza della libdbus dentro `libdbus-sys` 0.2.7 (`vendor/dbus/COPYING`), **letta**.
- Uid e gid di `alice` e `galleria` nelle immagini dei client 26 e 79; `python3` c'è in tutte e due; nell'immagine
  della GUI c'è `libsecret-1-0` e non ci sono `gnome-keyring` né `secret-tool`.

Non verificato:

- Nessuna riga di codice di B5 è stata scritta o eseguita: moduli, comandi e prove di questo piano sono progetto.
- Il crate scelto contro un portachiavi vero: né GNOME Keyring né KWallet (S35–S38). Non so se la sessione senza
  cifratura (`EncryptionType::Plain`) è accettata da tutti e due.
- La GUI compilata con `dbus-secret-service` (S34): ho letto le feature con `cargo tree`, non ho compilato.
- Il ponte con il demone vero sotto systemd e sul runner (S39); l'ho provato con un server di prova.
- La cartella del socket montata sotto la unit vera (`RuntimeDirectory=galleria` sopra un punto di montaggio): per
  questo non è la raccomandata in Q42.
- La sonda AT-SPI senza Orca (S41); il formato di `last_handshake` e l'elenco dei valori di `detail` (S42, P10).
- Che `docker exec -i` regga una connessione lunga con un evento al secondo per i minuti del giro (l'ho provato con
  una riga sola).
- Quanto dura `lab-connect` su un runner: la stima è un quarto d'ora per client, non una misura.

## Questioni aperte nuove

La numerazione continua Q12–Q41. Ognuna ha la mia raccomandazione, che vale come **predefinito per chi implementa**
se nessuno decide prima del task indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q42 | Come la finestra, che gira nell'immagine `galleria-gui-dev` (Q37), parla con il demone vero dei client del laboratorio. Il demone autorizza chi apre la connessione guardandone uid e gruppi | (a) montare la cartella del socket del client anche nell'immagine della GUI · (b) un ponte di prova: un socket sul lato dell'host, unito a quello del demone con `docker exec -i -u <utente>` · (c) WebKit nelle tre immagini dei client (scartata in Q37) · (d) il demone vero dentro l'immagine della GUI | **(b)**. Misurate tutte e due: (a) funziona, ma il demone vede l'utente **dell'immagine della GUI** (sotto Docker rootless è root, che passa senza essere nel gruppo), quindi per provare «un membro del gruppo» e «due utenti» (G9) l'immagine va lanciata con uid e gid numerici di ogni client, diversi tra Ubuntu e Fedora, e con quell'utente non scrive più in `target/`; in più cambia `lab/up.sh` per tutti i job e non l'ho provata sotto la unit vera. Con (b) il demone vede `alice` e `carla` veri, il laboratorio non cambia, e i due utenti sono due ponti. Costa 60 righe di Python in `lab/tests/` e il fatto che l'autorizzazione del processo della GUI non è esercitata lì: lo è già in B1 (G2) e in `t16`. (d) proverebbe la finestra su un solo sistema, non sui tre client | T9 (prima della card del gruppo D) |
| Q43 | Chi dà a Rust il testo tradotto degli annunci di stato. Q27 dice che i cambi di stato li annuncia Rust senza passare dalla pagina, D22 che i cataloghi stanno nella pagina | (a) la pagina consegna a Rust le frasi di stato nella lingua corrente, all'avvio e a ogni cambio di lingua (`announce_phrases`); Rust le usa quando lo stato cambia · (b) Rust incorpora i due cataloghi e legge la lingua da `state.toml` · (c) la pagina chiama `announce` a ogni `galleria:state` | **(a)**. Un comando in più, nessun catalogo doppio, e l'annuncio parte dallo stesso punto che emette l'evento: continua a funzionare quando in B6 la finestra sarà nascosta nella tray. (b) porta in Rust la scelta della lingua «di sistema», che oggi fa solo la pagina. (c) è la più corta ma è ciò che Q27 esclude, e lega l'annuncio a una pagina che può essere nascosta | T4 (prima della card del gruppo B) |
| Q44 | Con quale crate si parla con Secret Service (S12). Misurato: tabella sopra | (a) `dbus-secret-service` 4.2.0 con `vendored`, sessione senza cifratura · (b) `secret-service` 6.0.0 su tokio · (c) `keyring` 4.2.0 · (d) `oo7` 0.7.3 | **(a)**. È sincrono come il tratto `SecretStore`, quindi niente runtime dentro `galleria-profile` e niente giri per chiamarlo dalla CLI; usa il `dbus` che Tauri porta già; 8 crate contro 21; `cargo deny` verde; con `vendored` si compila senza `libdbus-1-dev` (che sull'host di Peppe non c'è, e Q15 esclude `sudo`) e la CLI non chiede librerie nei client. La libdbus compilata dentro è AFL-2.1 o GPL-2.0-or-later, compatibile con la nostra licenza. Senza cifratura della sessione: il bus di sessione è dell'utente, e chi può ascoltarlo può già chiedere il segreto al portachiavi; la cifratura costerebbe altri 13 crate. (c) nella forma predefinita fa fallire `cargo deny` con gli stessi tre avvisi di Q36. **Da sapere:** la CLI e `galleria-profile`, crate della fase 1, guadagnano una dipendenza: è il prezzo di D25 | T1 (prima della card del gruppo A) |
| Q45 | La CLI usa le credenziali ricordate dalla finestra? D25 dice che GUI e CLI leggono dai due archivi | (a) no: la CLI legge dai due archivi il materiale delle chiavi; utente e password come oggi (`--username`, stdin, terminale) · (b) sì: `galleria connect` senza `--password-stdin` manda quelle ricordate | **(a)**. D25 nasce perché un profilo importato da una parte si connetta dall'altra, e per quello bastano le chiavi (G6). (b) cambia il comportamento di un comando della fase 1 e apre la domanda di come dire alla CLI «questa volta chiedimela». Si aggiunge dopo se serve | T2 |
| Q46 | Dove si scrive che un profilo ha credenziali ricordate | (a) i due nomi `openvpn.username` e `openvpn.password` entrano in `secret_refs` del profilo · (b) lo si chiede all'archivio ogni volta | **(a)**. Il formato del profilo non cambia (è una lista di nomi) e `to_connect_params`, `export_config` e la CLI ignorano i nomi che non conoscono **[V: letti]**; la lista dei profili non deve mai aprire il portachiavi. Conseguenza voluta: «Duplica» copia anche le credenziali ricordate, «Esporta» no. Con (b) ogni apertura del dettaglio interroga il portachiavi | T6 |
| Q47 | Una password ricordata che il server non accetta più. Il demone non ritenta mai dopo un errore di autenticazione, quindi fallirebbe a ogni connessione | (a) l'errore offre «Dimentica le credenziali salvate»; la connessione dopo le chiede · (b) la finestra le dimentica da sola e riprova chiedendole · (c) niente: si va nella scheda Autenticazione | **(a)**. (b) fa partire un secondo tentativo di accesso che l'utente non ha chiesto, e su un server con il blocco dopo N errori è un rischio; (c) lascia l'utente davanti a un errore che si ripete senza dirgli perché | T6 |
