# Roadmap — Galleria

Client desktop open source per OpenVPN e WireGuard: rubrica di connessioni in stile VirtualBox, una connessione
attiva alla volta, Linux e Windows. Fonti: `docs/01-analisi.md` (§4, §4.1, §10), esiti in `docs/04`–`06`, decisioni della fase 2 in `docs/07`,
spec e piano in `docs/superpowers/`, handoff in `.claude/handoffs/`.

## Fase 0 · Analisi, validazione e spike
Decidere cosa costruire e provare su Linux e Windows i punti tecnici più rischiosi prima di scrivere il prodotto.

- [x] Analisi di mercato e tecnico-funzionale <!-- branch: main · commit: cbae648 · by: zPeppOz -->
  - [x] concorrenti e buco di mercato in `docs/00-mercato-e-concorrenti.md`
  - [x] decisioni, funzionalità, architettura, fasi e rischi in `docs/01-analisi.md`
  - [x] modello delle minacce M1–M7 in `docs/02-modello-minacce.md`
- [x] Mockup cliccabile della rubrica <!-- branch: main · commit: 823e6ca · by: zPeppOz -->
  - [x] `design/mockup/Rubrica.html` con gli stati demo `#on`, `#log`, `#import`, `#tray`, `#light`
- [x] Nome e licenza <!-- branch: main · commit: 6cb5aca · by: zPeppOz -->
  - [x] nome Galleria (D7)
  - [x] GPL-3.0-or-later con `LICENSE` (D8)
- [x] Decisioni tecniche con il CTO <!-- branch: main · by: nello -->
  - [x] D10–D14 e D16 proposte in `docs/03-sessione-nello.md`, ratificate da Aitano su delega di Peppe il 2026-11-10
  - [x] matrice di test A12: Ubuntu 26.04 e 24.04, Fedora corrente, Windows 11 x64
- [x] Spike 2 · management interface OpenVPN su Linux <!-- branch: main · commit: c04a0ce · by: nello -->
  - [x] criteri dello spike su OpenVPN 2.6.19, 2.7.0 e 2.7.9, esito in `docs/04-esito-spike-2-linux.md`
  - [x] configurazione via pipe e segreti via management, nessun file su disco
- [x] Spike 1 e spike 2 su Windows <!-- branch: spike/windows · by: nello -->
  - [x] 1.1–1.3 e 1.8: WireGuardNT da un servizio Rust, statistiche, smontaggio pulito, pipe con ACL
  - [x] 1.5: errore IPv6 immediato con un filtro WFP (12 ms con WireGuard)
  - [x] 1.6–1.7: MSI di Tauri con il servizio e updater che rifiuta firme sbagliate, provati solo da amministratore senza UAC
  - [x] 1.4: job `spike-windows-build` verde su runner Windows, MSI con validazione ICE (run 42895494397 su `cbc613f`)
  - [x] spike 2 su Windows: 2.1, 2.3–2.6 e 2.9 superati, 2.7 e 2.8 in parte; esito in `docs/05-esito-spike-windows.md`
- [x] Decisioni dopo gli spike <!-- branch: spike/windows · commit: d1a4bc3 · by: zPeppOz -->
  - [x] D15 corretta dopo lo spike 2 e approvata da Peppe; D17 e D18 decise da Aitano su delega
  - [x] nessuna telemetria (A11), nessuna soglia di stop (A6), design system nuovo per Galleria (A8)
  - [x] SignPath rinviata al rilascio della v0.1

## Fase 1 · Demone Linux e CLI minima
`galleriad` con WireGuard e OpenVPN dietro l'interfaccia comune, IPC definitivo della v0.1, CLI `galleria`, laboratorio Docker e CI: criteri E1–E7 della spec.

- [x] Spec e piano della fase 1 <!-- branch: aitano/galleria-fase1-piano · by: claude -->
  - [x] spec `docs/superpowers/specs/2026-11-10-fase1-demone-design.md` e piano in 16 task
  - [x] Q3, Q6 e Q7 risolte; parte Linux di D15 allineata in `docs/01-analisi.md` (`556b89e`)
- [x] T1–T2 · workspace Rust, CI, tipi e codec dell'IPC <!-- branch: aitano/galleria-fase1-t1-4 · by: codex -->
  - [x] `00ec034` e `93a55dc`; correzioni della negoziazione di hello in `9fdb1d7` e `9f96d0f`
- [x] T3–T4 · interfaccia dei backend, macchina a stati, server IPC con autorizzazione <!-- branch: aitano/galleria-fase1-t3-4b · by: codex -->
  - [x] sessione unica (D5) e client asincrono; rilievi della review corretti in `f936ebc`
- [x] T5–T6 · log con filtro dei segreti, modulo di rete comune con giornale <!-- branch: aitano/galleria-fase1-t5-6 · by: codex -->
  - [x] rilievi della review corretti in `6e9df06`; `aitano/galleria-fase1-t6-fix` è un doppione fermato, da non usare
- [x] T7–T9 · laboratorio Docker, unit systemd, backend WireGuard, blocco IPv6 <!-- branch: aitano/galleria-fase1-t7-9 · by: codex -->
  - [x] tutto nel tunnel, errore IPv6 immediato e pulizia dopo `kill -9` provati sul demone sotto systemd
- [x] T10–T12 · parser `.ovpn`, lista delle direttive ammesse, backend OpenVPN via management <!-- branch: aitano/galleria-fase1-t10-12 · by: codex -->
  - [x] stesso codice su OpenVPN 2.6.19, 2.7.0 e 2.7.9
- [x] T13–T16 · credenziali OpenVPN, blocco IPv6, profili TOML, CLI ed esito <!-- branch: aitano/galleria-fase1-t13-16 · by: codex -->
  - [x] E1–E7 verificati in locale sui tre client, esito in `docs/06-esito-fase1.md`
  - [x] 511 test passati e 0 falliti, gate locali 6/6, laboratorio 80 casi per client
  - [x] copertura sopra l'80% su IPC, rete, parser OpenVPN e macchina a stati
- [x] Correggi gli Importanti della review finale T13–T16 <!-- branch: galleria/t_17a7470c-galleria-correggi-gli-importanti-della-r · base: aitano/galleria-fase1-t13-16 · task: t_17a7470c · by: aitano -->
  - [x] il lavoro parte da `aitano/galleria-fase1-t13-16`, non da `spike/windows` né da `main`
  - [x] I1: `galleria logs` mostra le ultime righe e `-f` non salta quelle oltre le prime 1000, con test di paginazione (`c43cd53`)
  - [x] I2: l'import rifiuta file referenziati con percorso assoluto o `..` fuori dalla cartella del `.ovpn` (spec §8.2), con test (`6feb783`)
  - [x] minore 3: un dominio con `~` spinto dal server viene scartato con una riga di log, senza far cadere la connessione (`09e3cdb`)
  - [x] gate locali 6/6 verdi: 529 test passati e 0 falliti
  - [ ] laboratorio T13 e T16 verdi sui tre client: verde su Ubuntu 24.04; su 26.04 e Fedora OpenVPN 2.7 usa il DCO del kernel 7.3 e Docker rootless lo nega (EPERM), da rifare sulla CI
- [x] CI remota verde sulla fase 1 (S2) <!-- branch: aitano/galleria-fase1-t13-16 · commit: b38215a · by: genny -->
  - [x] `check` e `coverage` verdi su GitHub Actions (run 43707067359, ramo `t_17a7470c`; già verdi sul primo push, run 43661067765)
  - [x] `lab` verde su Ubuntu 24.04, Ubuntu 26.04 e Fedora (stesso run: job 26, 27 e 79)
  - [x] `lab-dco` riparato con il modulo `ovpn` del kernel sui client 27 e 79 (il modulo `openvpn-dco-dkms` non compila sul kernel 7.0.0-1019-azure): verde sul run 45625594806 (`b38215a`), job ora bloccante
  - [x] S2 in `docs/06-esito-fase1.md` §4 aggiornato con il link al run 45625594806
- [x] Integra la fase 1 in `main` <!-- branch: main · commit: e4a2d71 · by: aitano -->
  - [x] con l'ok di Peppe: `spike/windows` e la catena `aitano/galleria-fase1-*` sono entrati in `main` con un fast-forward a `e4a2d71`
  - [x] `docs/06-esito-fase1.md` §7 corretto: la parte Linux di D15 è già allineata da `556b89e`
  - [x] branch locali superati eliminati; il doppione `aitano/galleria-fase1-t6-fix` non è entrato in `main` e resta da solo
- [x] Prove su desktop reale: NetworkManager e DCO (S14, S15) <!-- branch: galleria/t_4a5ce3ed-galleria-vm-desktop-ubuntu-e-fedora-su-m · by: genny -->
  - [x] VM Ubuntu 26.04 e Fedora 45 desktop (GNOME, NetworkManager, AppIndicator) in `/mnt/b/vm`, spente su disco; binari installati con `packaging/linux/install.sh`
  - [x] `nmcli device` mostra `glr-wg0` e `glr-ovpn0` non gestite; DNS interni applicati prima di connected e ripristinati dopo
  - [x] catture IPv6 e ripiego IPv4 di T9 e T14 ripetute (suite e2e 79/79 su Fedora e Ubuntu); nessun residuo dopo `kill -9` e riavvio
  - [x] DCO aperto davvero, connected e bytecount con traffico (`ovpn_dco_state_and_bytecount` non saltato, `ip -d link` tipo `ovpn`)
  - [x] difetto trovato: `install.sh` non ricaricava NetworkManager, `glr-wg0` restava gestito fino al riavvio; corretto
- [x] Risposte alle questioni aperte della spec di fase 1 <!-- branch: galleria/t_e8440dfb-galleria-applica-d19-d32-spec-della-fase · task: t_e8440dfb · by: nello -->
  - [x] D19–D32 proposte in `docs/07-proposte-fase2.md`, approvate da Aitano su delega di Peppe il 2026-11-25
  - [x] Q5: script del profilo mai eseguiti, nemmeno con conferma (D19), con il mockup allineato
  - [x] Q8, Q10, Q11: nessun file di log (D20), nessun proprietario della sessione (D21), traduzioni nel frontend per codice (D22)
  - [x] Q1, Q2, Q4, Q9: testo di `docs/01-analisi.md` precisato (D23, D24); «fase 2» è solo la GUI

## Fase 2 · GUI Linux
La rubrica in Tauri 2 + React sopra il demone della fase 1, in sei blocchi B1–B6 che finiscono ognuno con qualcosa che si avvia: criteri G1–G13 della spec.

- ~~Completa la validazione dei concorrenti (A7)~~ — tolto per decisione di Peppe del 2026-11-25: Galleria è uno strumento per Peppe e i colleghi, non un prodotto da validare sul mercato

- [x] Spec della fase 2 e piano del blocco B1
  - [x] spec `docs/superpowers/specs/2026-11-25-fase2-gui-design.md` con i criteri G1–G13, sopra le decisioni D19–D32
  - [x] piano di B1 in `docs/superpowers/plans/2026-11-25-fase2-b1-fondamenta.md` (6 task); gli altri blocchi hanno un piano ciascuno, scritto prima di partire
  - [x] l'IPC della fase 1 resta senza modifiche e la webview passa dai comandi Tauri (D12)
  - [x] questioni aperte nuove della spec (Q12–Q21) decise prima di costruire: approvate da Aitano su delega di Peppe il 2026-11-25, come raccomandate dalla spec
- [ ] B1 · Fondamenta <!-- branch: galleria/t_f4f14d76-galleria-fase-2-blocco-b1-fondamenta-6-t · task: t_f4f14d76 · by: genny -->
  - [x] parte dalla fase 1 integrata in `main` (`e4a2d71` è antenato del ramo)
  - [ ] crate `galleria-gui` e `ui/` nel workspace, job CI della GUI, permessi minimi della webview (D29): crate, `ui/`, immagine e permessi fatti e provati nell'immagine; il job `gui` di `ci.yml` non è ancora verde su un runner (rosso per `cargo deny` sul run 45633910830, corretto in `deny.toml`)
  - [x] connessione al demone con ricollegamento ed eventi inoltrati al frontend (G1): `daemon_restart_relinks_without_user_action` e `relinks_after_a_daemon_restart`
  - [x] quattro stati del primo avvio, senza chiedere privilegi (D27, G2): sette prove di `classify`, schermate in Vitest e `first_run_without_daemon` nel fumo
  - [x] fumo WebDriver nel laboratorio provato: `scripts/gui.sh smoke` verde 3 volte di fila nell'immagine; il job `gui-smoke` non è ancora girato su un runner
  - [ ] ci si può fermare qui: finestra che mostra lo stato vero del demone, in sola lettura (manca il job `gui` verde su un runner)
- [ ] B2 · Design system e guscio <!-- branch: galleria/t_b510159d-galleria-b2-task-t7-tastiera-fumo-screen · task: t_b510159d · by: genny -->
  - [x] design system nuovo e nel repo, con la licenza del progetto; quello di XR1 non si usa (A8): `ui/src/design/`, nessun file fuori da lì importa la libreria (`boundary.test.ts`)
  - [x] token dal mockup, componenti di base su primitive «headless», tema chiaro e scuro (D30): Base UI 1.9.0 (Q12 verde con lo screen reader), token con S8 vero su WebKitGTK 2.52.6 (aperto su Ubuntu 26.04 e Fedora)
  - [ ] disposizione lista + dettaglio e navigazione da tastiera (G12): tastiera e ruoli verdi in jsdom e nella finestra vera con Orca; resta un buco di Orca con WebKitGTK, non del markup: nessuna regione viva viene pronunciata, `role="alert"` compreso (`S6b`), confermato su GNOME vero dallo spike `t_b81c3909`; la strada è Q27 (segnale ATK dal lato Rust), primo uso in B3. Il nome dei campi di testo non pronunciato (`S4b`) esiste solo nell'immagine con Orca 46.1: su GNOME vero si sente; vedi `docs/08` «Buco 1», «Buco 2» e «Spike Orca»
  - [x] cataloghi `en` e `it` con il test dei codici del demone (D22, G8): `tests/catalogs.rs` e cambio di lingua a caldo nel fumo
  - [x] ci si può fermare qui: guscio navigabile con dati finti, nelle due lingue (U1: fumo 13/13 tre volte, `a11y` 22 prove verdi due volte, con i due buchi di G12 registrati come prove)
- [ ] B3 · Rubrica e profili <!-- branch: galleria/t_aea4ab04-galleria-b3-task-t9-tastiera-fumo-screen · task: t_aea4ab04 · by: genny -->
  - [x] lista con gruppi, ricerca, preferiti, ordinamento e icona di stato; gruppi in `galleria.toml`: fumo `rubrica_lists_the_seeded_profiles`, `search_filters_the_list_and_sort_order_survives_a_restart`, `a_group_is_created_renamed_and_survives_a_restart`; tastiera in `keyboard.test.tsx`
  - [x] schede Generale, Autenticazione e Rete: campi veri per WireGuard, sola lettura più interruttori per OpenVPN (D26): `a_profile_is_renamed_and_the_list_follows`, `cli_lists_what_the_gui_saved`, `a_profile_saved_by_the_gui_keeps_the_phase_1_format`; letti con Orca in `R5d`
  - [x] editor raw con la validazione delle direttive della fase 1: `a_script_directive_cannot_be_saved_from_the_editor`, `a_comment_typed_in_the_editor_is_saved_as_written`, `R5d`
  - [x] duplica, esporta ed elimina profilo; «ultimo uso» in `state.toml` (D32): duplica ed elimina provati nella finestra vera, da tastiera e con Orca (`rubrica_works_with_the_daemon_stopped`, `R4b`, `R4c`); «Esporta» provato anche con la finestra «Salva» GTK vera, pilotata via AT-SPI (`R4e` salva, `R4f` annulla, `a11y` verde due giri di fila; resta non provata sotto un portale del desktop o Wayland, docs/08 «Non verificato»)
  - [ ] ci si può fermare qui: rubrica completa sui profili importati dalla CLI, senza connettere: R1–R6 verdi tranne Orca su un GNOME vero (docs/08, «B3 · chiusura»)
- [ ] B4 · Import <!-- branch: galleria/t_39398d55-galleria-b4-gruppo-c-t6-e-t7-g3-nel-labo · task: t_39398d55 · by: genny -->
  - [x] `.ovpn` con blocchi inline o file separati e `.conf` WireGuard, da file, trascinamento o testo incollato: `import_file_reads_inline_and_separate_files_into_the_same_profile`, `a_profile_pasted_as_text_is_imported`, `I5b` (file scelto nella finestra «Apri» GTK vera), `I5e` (trascinamento vero), `gui_import_equals_cli_import` (la finestra e la CLI della fase 1 salvano gli stessi file)
  - [x] anteprima con gli avvisi e il testo di D19, salvataggio solo dopo «Importa» (G3): `nothing_is_written_before_import`, `the_malicious_profile_shows_the_d19_text_and_runs_nothing`, `I5g`; nel laboratorio `t17_gui_import` sul client 26 (profilo malevolo importato dalla finestra, connesso dalla CLI, nessun `PWNED-*`) e `find` vuoto anche su 27 e 79
  - [ ] connessione OpenVPN dei profili importati su Ubuntu 26.04 e Fedora: rossa per il DCO che Docker rootless nega (EPERM, lo stesso limite di T13 e T16); da rifare sul job `lab-g3` della CI (`scripts/gui.sh lab-g3 27` e `79`, senza il modulo `ovpn`), scritto ma **non ancora girato** su un runner (niente push); il job `lab` non esegue più `t17`; WireGuard e `galleria list` sono verdi sui tre client
  - [x] associazione di `.ovpn` e istanza unica; QR WireGuard fuori dalla fase 2 (D31): `an_ovpn_is_opened_by_the_desktop_entry` (con `gio launch`), `a_second_launch_hands_the_file_to_the_first_window`, `tests/instance.rs`; non provati un file manager vero e Wayland
  - [x] ci si può fermare qui: si importa dalla GUI e si connette ancora dalla CLI (I1–I6 e G3 con la loro prova, i buchi sono in `docs/08-esito-fase2.md`, «B4 · chiusura»; il job `gui`, `gui-smoke` e `lab-g3` di CI non sono ancora girati su un runner, niente push)
- [ ] B5 · Connessione
  - [x] archivio dei segreti con Secret Service e ripiego sul file `0600`, lettura da entrambi anche nella CLI (D25, G6): `dbus-secret-service` 4.2.0 con `vendored` (Q44), provato contro GNOME Keyring vero sbloccato, bloccato e assente (`scripts/gui.sh keyring`, job `gui-smoke`); `G6a` e `G6b` nel laboratorio sul client 26; KWallet **non verificato** (docs/08 «Non verificato»)
  - [ ] connetti, disconnetti e sostituisci senza conferma tra profili propri (D5, D28, G4); sette stati, `detail` e statistiche dal vivo: CI run **45769841084** su `f33d093` verde **15/15**, con G4–G9 eseguiti sui tre client (S39 sciolta sul runner). Il controllo `t18` ora è stretto su ogni campione: nei tre giri locali su 26 **20/20, 20/20, 19/20**; campione con `glr-wg0` e `openvpn --version` da decidere, nessun nuovo allentamento (docs/08, correzioni della review)
  - [x] credenziali, OTP e passphrase chiesti solo su `needs_input`; «Ricorda» mai per l'OTP (G5): `G5a`–`G5e` sul client 26; «Ricorda» contro un portachiavi **bloccato** che chiede lo sblocco non provato
  - [x] errori tradotti, scheda Log con «Esporta log…» e nessun segreto (D20, G7): `G7a`, `gui_session_left_no_secret_in_journal_or_argv`, `no_command_returns_a_secret_after_a_full_round`, export dalla finestra «Salva» vera con AT-SPI senza Orca
  - [x] sessione estranea mostrata e chiusa solo con conferma (D21, G9): `G9a`–`G9c` con due utenti veri del gruppo (`alice` e `carla`, due ponti) sul client 26, e `gui_session_was_driven_by_two_users`
  - [ ] ci si può fermare qui: **non ancora**. Correzioni locali `t_4edd2f74`: check **678 Vitest + 368 Rust**, host **598/0**, fumo **50/50**; `K5c` ora aspetta e asserisce titolo e «User name», ma resta rossa (prima accettava «blank»). Giro Orca finale **26 passate / 10 fallite / 11 saltate**, incluso timeout `R5d` e driver non più raggiungibile; la misura stretta D5 ha il campione da decidere sopra. Nessun push; B5 resta aperto (docs/08, «Correzioni della review»)
- [ ] B6 · Tray, versione, chiusura
  - [ ] tray con stato, preferiti, connetti e disconnetti; su Linux tutte le azioni nel menu, perché Tauri non emette i click sull'icona (G10)
  - [ ] chiudere la finestra lascia la connessione attiva; uscita esplicita dal menu
  - [ ] avviso di nuova versione: una sola richiesta dalla GUI all'endpoint delle release di GitHub, senza identificativi, disattivabile e scritta nel README (D16, G11)
  - [ ] fumo end-to-end completo, prova manuale su Ubuntu e Fedora desktop, avvio a freddo misurato (G13)
  - [ ] esito in `docs/08-esito-fase2.md`

## Fase 3 · Porting Windows
Demone come servizio Windows con WireGuardNT e OpenVPN installato come prerequisito, MSI con updater (analisi §10).

- [ ] Spec e piano della fase 3
  - [ ] criteri Windows rinviati dalla fase 1 (Q2) e punti aperti di `docs/05-esito-spike-windows.md` §8
- [ ] Servizio e named pipe
  - [ ] servizio LocalSystem e pipe con ACL a SYSTEM, Administrators e INTERACTIVE (D18); un utente non autorizzato è rifiutato
- [ ] Backend WireGuardNT
  - [ ] tunnel, statistiche e smontaggio dello spike 1 dietro l'interfaccia comune (D10)
- [ ] Backend OpenVPN su Windows
  - [ ] `openvpn.exe` trovato da `exe_path` in `HKLM\SOFTWARE\OpenVPN`, versione minima 2.6, guida all'installazione se manca (D14)
  - [ ] DNS letto dal `PUSH_REPLY` della management e applicato dal modulo comune (D17), servizio interattivo solo come ripiego
- [ ] Blocco IPv6 con WFP per tutti i backend (D15)
  - [ ] zero pacchetti IPv6 globali ed errore immediato anche con OpenVPN (oggi [S])
  - [ ] sessione WFP dinamica che sparisce se il demone muore
- [ ] MSI e updater
  - [ ] MSI con il servizio nel frammento WiX e updater di Tauri (D16)
  - [ ] aggiornamento da utente standard con UAC deciso e provato (`docs/05-esito-spike-windows.md` §9)
  - [ ] prove sulla matrice Windows 11 x64 (A12)

## Fase 4 · Rilascio v0.1
Pacchetti, firma, documentazione e lancio pubblico (analisi §10).

- [ ] Pacchetti Linux di rilascio
  - [ ] `.deb` e `.rpm` costruiti da GitHub Actions con checksum pubblicati (M6)
- [ ] Firma Windows con SignPath
  - [ ] richiesta "Apply" a SignPath con la domanda su `wireguard.dll` e `wintun.dll` (R3)
  - [ ] se la risposta è no, decide Peppe tra certificato a pagamento, MSI non firmato o WireGuard su Windows rinviato
- [ ] Verifica di marchio, dominio e nome del repo (A1)
  - [ ] nessun conflitto trovato su marchio e dominio per Galleria, prima del lancio
- [ ] Rischi differiti della fase 1
  - [ ] i 15 minori della review T10–T12, l'M1 dell'import e i minori della review T13–T16 corretti o accettati per scritto
- [ ] Documentazione e README
  - [ ] installazione, prerequisito OpenVPN su Windows, unico contatto di rete fuori dalla VPN (D16), limiti noti (R8)
- [ ] Lancio pubblico
  - [ ] post su r/selfhosted, r/WireGuard e Hacker News; poi uso reale e segnali, senza soglie di stop (A6)

## Dopo la v0.1
Funzioni e protocolli che entrano solo dopo il lancio, uno alla volta (analisi §4 e §4.1, R9).

- [ ] Protezione e automazione: kill switch e protezione DNS leak per profilo, split tunneling per subnet, avvio con il sistema, connessione automatica su reti non fidate
- [ ] CLI estesa e backup/ripristino della rubrica
- [ ] Repository APT e DNF per gli aggiornamenti su Linux (D16)
- [ ] v0.2 · VPN aziendali con OpenConnect come processo figlio (D11), dopo uno spike su `--script` con `cscript.exe` e login SSO
- [ ] v0.3 · reti filtrate: AmneziaWG e OpenVPN su TCP 443 come opzione guidata
- [ ] Solo con trazione: macOS, Flatpak, AUR e winget
