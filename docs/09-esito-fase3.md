# Esito della fase 3 — Galleria su Windows

Stato: **P1 in corso** (il solo blocco approvato: Q60 = (a), Peppe, 2026-11-27). Niente è ancora stato eseguito.

Spec: `docs/superpowers/specs/2026-11-26-fase3-windows-design.md`. Piano di P1:
`docs/superpowers/plans/2026-11-27-fase3-p1.md`.

Questo file nasce diviso in tre sezioni, una per card di P1, perché due card lavorano insieme: **ogni card scrive
solo nella sua**. La tabella dei criteri qui sotto le card non la toccano: la riempie chi unisce i tre rami, dagli
esiti scritti nelle sezioni.

## Criteri di P1

| # | Criterio | Esito | Prova |
|---|---|---|---|
| U1 | R-A: «tutto nel tunnel» con WireGuardNT verso un server oltre il gateway | non eseguita | — |
| U2 | R-B: `PUSH_REPLY` da `stdout` prima di `CONNECTED`, DNS messo da noi, su 2.6 e 2.7 | non eseguita | — |
| U3 | R-C: IPv6 bloccato da una sessione WFP dinamica con OpenVPN | non eseguita | — |
| U4 | La VM si accende da uno snapshot e ci si entra via SSH | non eseguita | — |
| U5 | Dalla VM si raggiungono i server del laboratorio | non eseguita | — |
| U6 | Il workspace compila e si prova su `windows-latest`, 0 `ignored` | non eseguita | — |
| U7 | Profili e stato nelle cartelle dell'utente, non leggibili da altri | non eseguita | — |
| U8 | Linux non se ne accorge | non eseguita | — |

Gli esiti ammessi per U1–U3 sono **passa**, **cade**, **non concludente**. «Non eseguita» non è «passa».

**La fase regge?** Non lo sappiamo ancora: la riga la scrive la card P1-A2.

## P1-A1 · La VM Windows 11 e la rete verso il laboratorio (T1, T2)

Card `t_b3cede1b`. Da scrivere.

## P1-A2 · Le tre prove di rischio (T3a, T3b, T3c)

Card `t_5d2dccab`. Da scrivere.

## P1-B · Il workspace compila su Windows (T4a, T5, T4b)

Card `t_4faef896` (T4a) e `t_227bc768` (T5, T4b). **Stato: T4a, T5 e T4b fatti in locale, niente push.** Il job `win-check`
è scritto (`actionlint` pulito) ma **mai eseguito**: U6 e U7 sono «non verificati in CI».

### T4a · cosa è stato fatto e come è stato provato

- **«xcheck»** (compilazione incrociata `x86_64-pc-windows-gnu`, in un container, immagine `galleria-xbuild` costruita da
  `spikes/windows-lab/xbuild`): stato di partenza rosso (il primo errore era `netlink-sys`, poi `galleria-profile`,
  `galleria-cli`, `galleria-gui`); dopo il commit `cargo check --workspace --all-targets` e
  `cargo clippy --workspace --all-targets -- -D warnings` sono **verdi su tutto il workspace, finestra compresa** (Q76 (a)
  regge: nessun ripiego). Il build-dir è `build_cache_dir xcheck`.
- **Prove sotto wine** (Q78, tetto 1 h, usato ~35 min): `spikes/windows-lab/xbuild-wine/Dockerfile`, **su `rust:1-trixie`**
  (wine 10) e non sulla `xbuild`: con bookworm (wine 8) i binari di Rust 1.96 non partono (manca `bcryptprimitives.dll`).
  `cargo test --workspace --exclude galleria-e2e --exclude galleria-gui` sotto wine: **468 passate, 0 fallite, 0 `ignored`**.
  I binari della finestra sotto wine non partono (stato 53: `WebView2Loader.dll`, che nel build `msvc` è statico); la DLL
  sta in `webview2-com-sys-0.39.1/x64/` nel registro di Cargo e si può mettere nel `WINEPATH`: **non provato**.
  Wine non è Windows: dice che il codice parte e che le prove portabili passano, niente sul sistema vero.
- Linux: 598 passate / 0 fallite / 103 `ignored` (host, **stessi nomi di prima**: la prova di `galleriad` del server è rimasta `server::tests::…`,
  il modulo non è stato spostato); `scripts/gui.sh check` 493/0 (finestra, Vitest
  compreso); `cargo fmt --check`, `clippy -D warnings`, `cargo deny check`, `scripts/check-boundaries.sh`,
  `lab/tests/check-wsl.sh`, `lab/tests/build-user.sh`, `scripts/tests/check-gates.sh`, `check-gates.py --job check`
  («7 proofs»), `git diff main -- Cargo.lock` vuoto, diff dell'IPC di U8 vuoto, `PROTOCOL_VERSION` 1.
- **Non eseguito**: `scripts/gui.sh smoke` (il piano lo vuole una volta sull'ultimo commit di B, che non c'è ancora).

### Come è stato diviso il codice

Codice Unix **spostato**, non riscritto: `galleria-backend-wireguard/src/kernel.rs` (`KernelDevice`, solo Linux; le
dipendenze `netlink-*` e `defguard_wireguard_rs` sotto `cfg(target_os = "linux")`), `galleria-profile/src/sys/{unix,windows}.rs`
(la «giunta»: `check`, `directory`, `read_private`, `atomic`, `default_dir`, `open_for_import`; il corpo Windows dice
«non supportato» fino al T5), `galleria-gui/src/instance/{unix,windows}.rs` (su Windows ogni avvio è il primo:
`Claim::Alone`), `galleriad/src/daemon_info.rs` (il tipo `DaemonInfo` fuori dal server Unix), `galleriad` `main` (su
Windows: «galleriad: Windows is not supported yet», uscita 2). `nix`, `rustix`, `libc` stanno sotto `cfg(unix)`.
`Client::connect` su Windows risponde `Io(Unsupported)`; `SystemRunner::locate` `ToolMissing("ip")`; `probe()` `NotFound`;
`hidden_from` della CLI un errore con testo. Nessun crate nuovo, nessun `unsafe` nuovo.

### Scostamenti dal piano

1. **`crates/galleria-gui/icons/icon.ico` (11 789 byte) è un file binario nuovo**, aggiunto a `tauri.conf.json`: senza,
   `tauri-build` si ferma su Windows («icon.ico not found»). Generato con `pnpm tauri icon` da `icon.png`.
2. **Il corpo Windows di `default_dir` (profili) e di `default_path` (stato della finestra) è un segnaposto**
   (`PathBuf::new()`): lo scrive il T5, come da piano. Finché il T5 non c'è, su Windows il negozio dei profili e il file di
   stato rispondono «non supportato».
3. `tests/ui_state.rs` della finestra è diventato un file di Linux: per il T5 le due prove di Windows vanno in
   `tests/ui_state_windows.rs`, come previsto dal piano.
4. L'immagine wine non è la `xbuild` più `wine64` come scritto nel piano (vedi sopra).

### Prove che restano di Linux

Base misurata il 2026-11-28 a `8aef4f1`: `cargo test --workspace --exclude galleria-gui` = 598 passate, 103 `ignored`.
Su Windows (wine, a T4a) stesso comando senza `galleria-e2e` e senza la finestra: **468 passate, 0 `ignored`**
(= 598 − 131 escluse + 1 solo Windows). Le escluse, file per file (`#![cfg(unix)]` o `cfg(target_os = "linux")` in testa):

| File (crate) | Prove | Motivo |
|---|---|---|
| `galleriad/tests/ipc_server.rs` | 27 | server su socket Unix, `nix`, permessi |
| `galleria-profile/tests/profiles.rs` | 38 | `PermissionsExt`, `symlink`, proprietario |
| `galleria-profile/tests/import_file.rs` | 17 | `symlink`, FIFO |
| `galleria-profile/tests/secrets_layered.rs` | 15 | `PermissionsExt`, `MetadataExt` |
| `galleria-profile/tests/settings.rs` | 10 | `PermissionsExt` |
| `galleria-profile/tests/export.rs` | 5 | `PermissionsExt` |
| `galleria-profile/tests/keyring.rs` (`linux`) | 0 + 9 `ignored` | Secret Service |
| `galleria-cli/tests/{duplicate_input,logs_paging}.rs`, `import_paths.rs` | 1 + 3 + 3 | socket Unix, `symlink`, `XDG_CONFIG_HOME` |
| `galleria-cli/tests/keyring.rs` (`linux`) | 1 + 2 `ignored` | Secret Service |
| `galleria-cli/tests/unit.rs` | 2 | `nix::pty`, `termios` |
| `galleriad/tests/{hardening,session_dir}.rs` | 1 + 3 | `rustix`, `PermissionsExt` |
| `galleria-ipc/tests/client.rs` | 1 | `UnixListener` |
| `galleria-net/src/runner.rs`, `galleriad/src/{peer_groups,server}.rs` (unit) | 2 + 1 + 1 | script `sh`, `SO_PEERGROUPS`, socket |
| `galleria-ovpn/tests/parser.rs` (solo la funzione) | 0 + 1 `ignored` | laboratorio Linux |
| `galleria-e2e/tests/t*.rs` (9 file) | 0 + 91 `ignored` | laboratorio Linux (il job non li esegue) |

Finestra (`galleria-gui`, 493 su Linux): file di Linux con `#![cfg(unix)]`: `closing` 17, `commands` 8, `connection` 23,
`credentials` 12, `export` 30, `foreign` 4, `groups` 18, `instance` 28, `link` 10, `logs` 11, `rubrica` 72, `supervisor` 11,
`tray` 42, `ui_state` 20, `update` 56 = **362 escluse**. Dentro i file che restano: `announce::atk::*` 3 (ATK, `cfg(target_os = "linux")`) e,
in `import.rs`, `a_picked_file_gets_the_same_checks_as_a_dropped_one` e `a_dropped_folder_or_fifo_is_refused` (usano `mkfifo`,
`cfg(unix)`, T4b) = **5 escluse**. Restano per Windows: `src/lib.rs` 13, `announce` 20, `catalogs` 7, `config` 19, `import` 45,
`raw` 10, `secrets` 12 = **126**. Le escluse della finestra e di `galleria-profile` sono molte: una parte (`rubrica`,
`ui_state`, i `galleria-profile/tests/*`) si può far rientrare con `cfg` sulle singole prove ora che il corpo Windows c'è, e
provarla sotto wine. **Non fatto.**

**Solo Windows:** `galleria-ipc/tests/client_windows.rs::the_client_says_unsupported_without_a_transport` (1), le 8 di T5
(vedi sotto: 6 con wine + 2 solo su Windows) e `galleria-gui/tests/ui_state_windows.rs` (3).

### T5 · percorsi e protezione dei file su Windows

Commit «put profiles and state under the windows user folders and keep them private». Funzioni **pure** (il valore della
variabile entra come argomento; assente, vuoto o non assoluto — `profili`, `.\x`, `C:x`, `\x` — dà `None`).

| Cosa | Linux | Windows | Dove |
|---|---|---|---|
| profili | `$XDG_CONFIG_HOME/galleria` o `$HOME/.config/galleria` | `%APPDATA%\galleria` | `galleria_profile::windows_paths::profiles_dir`, usata da `sys/windows.rs::default_dir` |
| stato della finestra | `$XDG_STATE_HOME/galleria/state.toml` | `%LOCALAPPDATA%\galleria\state.toml` | `galleria_gui::ui_state::state_path` (Windows) |
| cartella del servizio | `/run/galleria` | `%PROGRAMDATA%\Galleria` (solo il percorso; il descrittore è P2-T3) | `galleriad::config::service_dir` |
| protezione di cartelle e file | `0700`, `0600`, proprietario | nessun permesso scritto da noi: si eredita da `%APPDATA%`/`%LOCALAPPDATA%` (spec §8). Il temporaneo nasce accanto al file finale (`tempfile::NamedTempFile::new_in` + `persist`); un collegamento simbolico o una giunzione nella cartella dei profili è rifiutato come su Unix | `galleria-profile/src/sys/windows.rs`, `ui_state.rs` |
| file da importare | `O_NONBLOCK`, file regolare, ≤ 1 MiB | aperto con `FILE_FLAG_OPEN_REPARSE_POINT` (senza `unsafe`): un collegamento si apre come tale, non è un file regolare e dà `NotAFile`, l'errore che Linux dà per lo stesso caso (un collegamento a un dispositivo) | `sys/windows.rs::open_for_import` |

Quando la variabile non è usabile (assente, vuota, relativa) `default_dir()` e `default_path()` tornano un percorso vuoto
(la firma non cambia, Linux invariato): `ProfileStore::open` risponde `Invalid` con «%APPDATA% is missing, empty or not an
absolute path…», la scrittura dello stato risponde con lo stesso testo per `%LOCALAPPDATA%`, e la lettura dello stato dà i
valori di default. `DaemonConfig::default()` ricade su `C:\ProgramData\Galleria` se `%PROGRAMDATA%` non è usabile (il
demone non gira ancora su Windows).

Prove scritte (tutte `cfg(windows)`):

| Prova | File | Dove è stata eseguita |
|---|---|---|
| `profiles_live_under_appdata`, `an_empty_or_relative_folder_is_refused` (una prova sola, sei valori: il gate cerca il nome, e un `rstest` darebbe `nome::case_1`), `a_saved_profile_reads_back_and_replaces_the_old_one`, `a_folder_is_not_imported_as_a_file` (non nel piano: una cartella rilasciata dà `NotAFile` come su Linux; ha richiesto `FILE_FLAG_BACKUP_SEMANTICS`, senza il quale una cartella non si apre) | `galleria-profile/tests/windows_paths.rs` | wine: passano |
| `the_temporary_file_is_made_next_to_the_final_one` | `galleria-profile/src/sys/windows.rs` (prova unitaria: ha bisogno di vedere il temporaneo prima del rinomina, quindi di una cucitura privata `atomic_with`; il piano la voleva in `tests/`) | wine: passa |
| `the_service_folder_is_under_programdata` | `galleriad/tests/windows_paths.rs` (nel piano stava in `galleria-profile`: `galleriad` non dipende da `galleria-profile` e un'altra dipendenza avrebbe cambiato `Cargo.lock`) | wine: passa |
| `a_saved_profile_is_not_readable_by_other_users` | `galleria-profile/tests/windows_paths.rs`: legge la DACL con `icacls /save` (SDDL, indipendente dalla lingua) e cerca `WD`/`BU`/`AU` e i SID `S-1-1-0`/`S-1-5-32-545`/`S-1-5-11` | **mai eseguita**: sotto wine non direbbe niente (come da piano) |
| `a_link_is_not_imported` | stesso file; se non può creare il collegamento **fallisce** | **mai eseguita**: wine crea il collegamento ma poi non lo vede (`symlink_metadata` dà «File not found»), quindi non prova niente; il primo run è `win-check` |
| `the_state_file_lives_in_localappdata`, `the_state_is_written_and_read_back_on_windows`, `an_unusable_variable_is_named_by_the_error_of_the_write` | `galleria-gui/tests/ui_state_windows.rs` | wine, con `WebView2Loader.dll` nel `WINEPATH` (vedi T4b): passano |

«xcheck» (`cargo clippy --workspace --all-targets -- -D warnings`, `x86_64-pc-windows-gnu`) verde. Il conto sotto wine è nel T4b.

**Due limiti da portare nella guida di P6:** (1) chi sposta `%APPDATA%` su una condivisione di rete ne eredita i
permessi: noi non scriviamo ACL, quindi proteggiamo quanto la cartella già protegge; (2) il valore delle variabili lo
decide chi lancia il programma, come `XDG_CONFIG_HOME` su Linux.

**Non fatto in T5:** rimettere in gioco con `cfg` sulle singole prove i file oggi esclusi a intero (`galleria-profile/tests/*`,
`rubrica`, `ui_state`): sono ancora le 131 + 362 della tabella qui sopra, e N di `win-check` le conta come escluse.

### T4b · il job `win-check` e il controllo che rifiuta le prove ignorate

Commit «add the win-check job and refuse ignored tests in its gate check». Fatto: `scripts/check-gates.py` impara
`--no-ignored` (una riga `test … ignored`, o una riga `test result:` con `ignored` o `filtered out` diversi da 0, è un errore;
ultima riga `check-gates: <job>: <n> passed; 0 ignored`) e `--min-passed N` (somma dei `passed` delle righe `test result:`);
`scripts/tests/check-gates.sh` ha i casi del piano più alcuni (la somma di due binari, un limite non numerico, un log senza
righe `test result:`, il manifesto letto di default); `lab/tests/gates-w.tsv` ha `WP1` e le prove di `WP2` (le sette del piano
più `a_saved_profile_reads_back_and_replaces_the_old_one`, `the_service_folder_is_under_programdata` e
`the_state_is_written_and_read_back_on_windows`); `config.rs` conosce `WP1` e `WP2`; il job sta in `ci.yml` dopo `gui`.
Il job `check` di Linux non è cambiato di una riga. **`actionlint`** (immagine `rhysd/actionlint`, l'host non lo ha): nessun
avviso su `ci.yml` (provato che segnala un errore su un file sbagliato).

**Scostamenti dal piano (T4b):**

1. **Le prove della finestra girano sotto wine**, che a T4a non era riuscito: la `WebView2Loader.dll` del build `gnu` sta in
   `webview2-com-sys-*/x64/` del registro di Cargo, e messa nel `WINEPATH` basta. Questo ha trovato due difetti **dei
   test** (non del prodotto) che il job avrebbe mostrato al primo run, e sono corretti: (a) i test della finestra invocano i
   comandi con l'origine `tauri://localhost`, che su Windows è `http://tauri.localhost`: i comandi rispondevano «not allowed.
   Plugin not found» (10 file di prova toccati, Linux identico: `if cfg!(windows) {…} else {…}`); (b) due prove di
   `import.rs` usano `mkfifo` e ora sono `cfg(unix)`. In più il prodotto: aprire una cartella con `File::open` su Windows dà
   «accesso negato» e non «non è un file» (`FILE_FLAG_BACKUP_SEMANTICS`, in `sys/windows.rs`).
2. `N` = 605, non quello del piano (598 + 493 − escluse + solo Windows, che è lo stesso calcolo): il conto è sotto.
3. Le prove di `win-check` si cercano per **nome intero**: una prova `rstest` si chiama `nome::case_1` e il gate la darebbe
   «mancante». `an_empty_or_relative_folder_is_refused` è quindi una prova sola con sei valori.

#### Il conto di N

| Voce | Prove |
|---|---|
| Linux, `cargo test --workspace --exclude galleria-gui` (host, `8aef4f1`) | 598 |
| − escluse (tabella «Prove che restano di Linux») | − 131 |
| + solo Windows: `the_client_says_unsupported_without_a_transport` | + 1 |
| = misurato sotto wine a T4a | 468 |
| + T5 eseguite sotto wine: `profiles_live_under_appdata`, `an_empty_or_relative_folder_is_refused`, `a_saved_profile_reads_back_and_replaces_the_old_one`, `a_folder_is_not_imported_as_a_file`, `the_temporary_file_is_made_next_to_the_final_one`, `the_service_folder_is_under_programdata` | + 6 |
| + T5 solo su Windows (wine non le prova): `a_saved_profile_is_not_readable_by_other_users`, `a_link_is_not_imported` | + 2 |
| **Crate che non sono la finestra** | **476** |
| Finestra, Linux (`scripts/gui.sh check`) | 493 |
| − file `cfg(unix)` (362) − `announce::atk` (3) − due prove con `mkfifo` (2) | − 367 |
| + `ui_state_windows.rs` | + 3 |
| **Finestra** | **129** (13 + 20 + 7 + 19 + 45 + 10 + 12 + 3) |
| **N** | **605** |

Misurato: `cargo test --workspace --exclude galleria-e2e --no-fail-fast -- --skip a_saved_profile_is_not_readable_by_other_users
--skip a_link_is_not_imported` sotto wine = **603 passate, 0 fallite, 0 `ignored`, 2 filtrate** (le due saltate a mano) =
605 − 2. `scripts/check-gates.py --job win-check --cargo-log … --no-ignored` sul log di wine: tutte le prove di `WP1` e `WP2` `ok`
tranne le due che wine non esegue, che il gate dice **MISSING** (è la prova che il controllo guarda).
Prova a secco del passo finale: `scripts/check-gates.py --job check --cargo-log target/gates/check-cargo.log --no-ignored`
con il log di Linux esce **1** (103 `ignored`).

**U6 e U7 non verificati in CI.** Il job non si può provare senza push.

### Cosa guardare al primo run di `win-check`

Job `win-check` (`windows-latest`, `shell: bash`). Passi, nell'ordine, e cosa può rompersi (nulla di questo è stato visto):

1. **Prima del checkout**: `git config --global core.autocrlf false` (`tests/config.rs` legge `ci.yml` e i `.tsv` riga per riga).
2. **Toolchain**: `python -c 'import tomllib…'` (serve Python ≥ 3.11 nel `PATH` di Git Bash: se `python` non c'è o è vecchio, rotto qui).
   `rustup toolchain install 1.96.0` con `rustfmt,clippy,llvm-tools-preview`: la toolchain `msvc` è quella dell'host del runner.
3. **`npm install --global "pnpm@$(scripts/gui.sh pnpm-version)"`**: `gui.sh` è bash puro per `pnpm-version`, ma non è mai girato su Git Bash.
   Poi `pnpm install --frozen-lockfile && pnpm --dir ui build`.
4. **`cargo build --workspace --all-targets` e `clippy -D warnings` per `msvc`**: finora solo `gnu` (xcheck). Le differenze possibili
   sono lint che solo `msvc` fa scattare e il link statico di `WebView2Loader` (serve l'SDK di Windows, che il runner ha).
5. **`cargo test --workspace --exclude galleria-e2e`**, log in `target/gates/win-check-cargo.log`: cosa non è stato visto sotto
   wine e che qui gira per la prima volta: `a_saved_profile_is_not_readable_by_other_users` (legge la DACL con `icacls /save`: se `icacls`
   scrive in un'altra codifica o se la cartella temporanea del runner ha una voce per `Users` ereditata, è rossa — **leggere l'SDDL
   nel messaggio**), `a_link_is_not_imported` (serve il diritto di creare collegamenti: l'utente del runner è amministratore **[S]**; se non lo
   è, la prova **fallisce** invece di saltarsi), e tutta la finestra sul runtime WebView2 vero invece che su wine.
6. **Ultimo passo**: `python scripts/check-gates.py --job win-check --cargo-log target/gates/win-check-cargo.log --no-ignored --min-passed 605`.
   Deve stampare **`check-gates: win-check: 605 passed; 0 ignored`** (o più di 605 passate) e una riga `ok` per ognuna delle dieci
   prove di `WP1`/`WP2`: `the_client_says_unsupported_without_a_transport`; `profiles_live_under_appdata`,
   `an_empty_or_relative_folder_is_refused`, `a_saved_profile_is_not_readable_by_other_users`,
   `the_temporary_file_is_made_next_to_the_final_one`, `a_link_is_not_imported`, `the_state_file_lives_in_localappdata`,
   `a_saved_profile_reads_back_and_replaces_the_old_one`, `the_service_folder_is_under_programdata`,
   `the_state_is_written_and_read_back_on_windows`. Se `passed` è sotto 605, un binario di prova non è partito: confrontare con il conto
   qui sopra.
7. **La durata del job**: oggi nessuno la conosce. Annotarla.

## Per chi viene dopo

- **Aitano fa il primo push** e guarda «Cosa guardare al primo run di `win-check`»: finché non c'è un run verde, U6 e U7 sono
  «non verificati in CI», e il resto di P1 (A1, A2) non è in questa sezione.
- Rimettere in gioco con `cfg` sulle singole prove i file oggi esclusi a intero (`galleria-profile/tests/*`, `rubrica`, `ui_state`):
  ora che i corpi Windows ci sono e che le prove della finestra girano sotto wine (`WINEPATH` con `WebView2Loader.dll`), si
  provano in locale; ogni prova rimessa in gioco alza `N` (e il conto qui sopra).
