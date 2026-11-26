
## Gruppo D — desktop veri (B6, T8–T10), giro parziale

> **Completamento (card `t_a16d68aa`):** giro quotidiano, Q49 e S56 in [08-b6-giro-quotidiano.md](08-b6-giro-quotidiano.md). Solo Ubuntu 24.04 aveva il giro intero. **Card `t_6670f193`: giro quotidiano fatto anche su Ubuntu 26.04 e Fedora 45** (tabella e S56 in fondo, «Giro quotidiano sulle tre VM»); T10 su 26.04 e Fedora resta «non verificato».

**Non è la chiusura di G10/G12 né della fase 2.** La card permette la consegna parziale a budget finito.
T8 è stato eseguito; di T9/T10 sono scritte qui sia le prove sia le parti non fatte. I limiti del banco
non diventano automaticamente limiti ammessi del prodotto. In particolare il menu Fedora è obbligatorio:
la sua mancata verifica non si può sanare scrivendo «best effort».

### T8: banco e provenienza

- ff-only da `bcb3f39` (T7 già chiuso), commit T8 `b900b5a`. Nessun push.
- Nuova VM `ubuntu24`, HDD `/mnt/b/vm/disks/ubuntu24.qcow2`, 40 GiB virtuali, 4 vCPU e 4 GiB RAM.
  GNOME da `ubuntu-desktop-minimal`, autologin `lab` senza password; SSH solo con chiave.
  Cloud-init finito, `status: done` (`/mnt/b/vm/evidence/b6/ubuntu24/cloud-init.txt`), spegnimento
  regolare e riaccensione provati (`ubuntu24/T8-stop.txt`). Anche le altre VM sono state spente e riaccese.
- Immagine ufficiale Noble, SHA256 `04e8f2461a16a44e78a370284471f29616cdd9baf1893df25807b71f985b6c94`,
  firma buona UEC Image Automatic Signing Key, fingerprint `D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`.
  Firma in `ubuntu24/image-signature.txt`; manifesti e immagine in `/mnt/b/vm/images/`.
- `gui-vm.sh install` copia binario, esempi, CLI/demone del lab, `.desktop`, icona e database MIME.
  Usato il GUI costruito dal gruppo C a `bcb3f39`, identico al sorgente di T8: hash in
  `binaries-bcb3f39.sha256`. `install.log` delle tre VM: nessuna libreria mancante.
  `app` avvia direttamente il binario con un'unità utente: **non prova M1 (menu applicazioni)**.
- Fotografie finestra + icona: `ubuntu24/T8-window.png`, `ubuntu/T8-window.png`, `fedora/T8-window2.png`.
  Sulla Fedora inizialmente appariva il greeter, benché `lab` fosse già su tty2: `Ctrl+Alt+F2` ha
  mostrato la sessione. Le tre sessioni sono Wayland (non dedotto dall'ambiente esportato degli script:
  `loginctl`, per la nuova VM anche `ubuntu24/session.txt`).

Percorsi relativi in questa sezione: **`/mnt/b/vm/evidence/b6/`**, salvo dove scritto altrimenti.

| VM | GNOME Shell | Orca | WebKitGTK 4.1 | Libreria indicatore |
|---|---|---|---|---|
| Ubuntu 24.04.5, kernel 6.8.0-147 | 46.0 | 46.1-1ubuntu1 | 2.52.6-0ubuntu0.24.04.2 | libayatana-appindicator3.so.1 |
| Ubuntu 26.04.1, kernel 7.0.0-43 | 50.1 | 50.3 | 2.52.6-0ubuntu0.26.04.2 | libayatana-appindicator3.so.1 |
| Fedora 45, kernel 7.2.11-300.fc45 | 51.1 | 51.1 | 2.54.2-1.fc45 | **libappindicator3.so.1.0.0** (dal `/proc/<pid>/maps`) |

Fonte: `<vm>/versions.txt`. Il tipo `tty` di SSH **non** è il tipo della sessione grafica.
AppIndicator era ACTIVE sulle tre. La libreria caricata su Fedora è provata, il menu funzionante no.

### T9: tabella 12 × 3 — nessuna casella spacciata per una passata completa

`NF` = **non fatto per intero**, non «fallito». Ogni cella nomina la prova parziale o dice che non c'è.
Gli avvii del T8 non contano come M1; il tunnel in Docker del gruppo C non conta come M3–M6 in una VM.

| Caso | Ubuntu 24.04 | Ubuntu 26.04 | Fedora 45 |
|---|---|---|---|
| M1 avvio dal menu | NF; solo T8-window.png (avvio diretto) | NF; solo T8-window.png (avvio diretto) | NF; solo T8-window2.png (avvio diretto) |
| M2 menu tray | NF; M2-usb.png, M2-key-menu.png, tentativi non conclusivi | NF; M2-menu-hmp.png, M2-shell-tree.txt: etichette nel bus/albero, non menu aperto | NF; M2-retry.png: indicatore non stabilmente disponibile, vedi sotto |
| M3 preferito WG, demone vero | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M4 X, tunnel ancora su | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M5 disconnetti/credenziali dal menu | NF; T10-credentials.txt non passa dal menu, demone finto | NF; nessuna prova del menu | NF; T10-credentials.txt non passa dal menu, demone finto |
| M6 esci col tunnel su | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M7 Apri mouse + tastiera/import | NF; **M7-native.png/.txt**: chooser GTK del programma aperto via AT-SPI e annullato; non selezione mouse+tastiera | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M8 esporta profilo/log e 0600 | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M9 doppio click Nautilus, chiusa/nascosta | NF; nessuna prova nuova | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M10 trascina da Nautilus | NF; nessuna prova nuova | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M11 dimensioni/secondo avvio | NF; nessuna prova completa | NF; nessuna prova completa | NF; nessuna prova completa |
| M12 portachiavi bloccato/sblocco | NF; **NO_KEYRING=1** nei giri credenziali, non è questa prova | NF; nessuna prova nuova | NF; **NO_KEYRING=1** nei giri credenziali, non è questa prova |

**Risultato: 0/36 casi completi verificati in questo giro**, non 36 fallimenti del prodotto.
Non fatte nemmeno la prova con estensione AppIndicator spenta e la prova vera della persistenza del tunnel
in VM. Motivo della consegna parziale: il tempo è andato in provisioning, avvii lenti sul disco meccanico,
input/compositore e raccolta di evidenze riproducibili; non si trasformano tentativi in verdi.

Particolarità da riprendere **prima di G10 manuale**:

- `Ctrl+Alt+Tab` arriva ad Activities (AT-SPI); la sequenza per aprire l'indicatore non è stata completata.
  Tentati anche puntatore VNC e `mouse_move`/`mouse_button` qemu: nessuna fotografia prova il menu aperto.
  La tavoletta USB ora è nello script qemu. Non concluso S54; non è prova che il menu non funzioni.
- Fedora: un avvio ha mostrato icona e caricato la libreria (T8); dopo avvii successivi `app.log` ha scritto
  `libappindicator-WARNING ... Unable to connect to the Notification Watcher: Timeout was reached` e
  `Gtk-CRITICAL ... gtk_widget_get_scale_factor: assertion 'GTK_IS_WIDGET (widget)' failed`.
  **S56 secondo pezzo (menu): non verificato, causa non trovata. Non dichiarato limite accettabile di Fedora.**
  Il testo non è il `WIDGET_REALIZED_FOR_EVENT` di R7: non sono lo stesso avviso.
- M7 su 24.04: `FileChooserNative` appare nell'albero del processo Galleria, non in un processo portale;
  `M7-native.txt` riporta stderr senza il `WIDGET_REALIZED_FOR_EVENT` in quell'apertura. Era un'apertura via AT-SPI,
  **nessuna persona alla tastiera**: non chiude R7 e non prova il percorso col tasto `/`.

### Riserve ereditate: stato a questa consegna

| Riserva | Esito/prova o limite preciso |
|---|---|
| R1 modifiche non salvate alla chiusura | Verificata nel gruppo C (G10f e tastiera), nessuna nuova prova VM qui. |
| R2 Q28/Q49, Orca 46 | Vedi T10: **nome dei campi generali letto** su GNOME46; **dialogo credenziali non annunciato all'apertura e User name non detto nel giro**. Limite **delle prove Q49**: apertura inizialmente inattiva, non isolata la causa focus/Orca46; riapertura da frame già attivo non eseguita. Albero con nome/ruolo/fuoco presente; Q49(a) invariata. Non estendere quel limite a tutti i campi. |
| R3 Q22/Q56 | Passata parziale dal debug; nessun ascolto umano. G12 non interamente verificato (mancano i quattro giri nuovi per VM). Lista cuffie sotto, non bloccante. |
| R4 Wayland/portale | Chooser GTK osservato su Wayland 24.04 (M7-native). Apri/Salva sulle altre due e scrittura file: non verificati qui. **Portale/sandbox resta limite dichiarato**: niente Flatpak/Snap in fase 2. |
| R5 file manager vero | Limite delle prove: doppio click e trascinamento Nautilus **non verificati**; resta l'associazione provata con `gio launch` in B4, non una prova col file manager. |
| R6 primo piano S31 | Limite di supporto già deciso: Wayland può negare il fuoco. L'apparizione da sola non prova il primo piano; M11 non completato. |
| R7 WIDGET_REALIZED_FOR_EVENT | Non verificata con tastiera umana; un'apertura AT-SPI senza quell'avviso non chiude la riserva. Nessun nuovo difetto attribuito all'app su questa base. |
| R8 GNOME Keyring vero | Limite delle prove: sblocco del sistema + Ricorda bloccato non verificati in VM. I giri NO_KEYRING non contano. |
| R9 KWallet | Limite già deciso: KDE best effort; non provato. |
| R10 finestra intera sulle tre | Avvio verificato T8; **non verificata la passata completa**. Non usare la sola finestra aperta per chiudere questo requisito. |
| R11 dimensioni su Wayland | Limite delle prove: persistenza dimensioni non verificata su Wayland in questo giro. |
| R12 S42–S44 | T11 (gruppo E), non riesaminati qui. |
| R13 instabilità a11y | Card dedicata e gruppo C; numeri di quel giro non riciclati come nuova passata VM. |

### T10: parlato di Orca, non ascolto umano

**Nessun ascolto umano.** È letto il file di debug, come Q56(a). Le frasi sono ciò che Orca ha mandato
alla sintesi, non una valutazione di volume/velocità/comprensibilità. Lista non bloccante per Peppe:
[dieci passi in venti minuti](b6-ascolto-20-minuti.md).

Il vecchio giro dello spike aveva due difetti di banco: la 24.04 nuova non aveva una rubrica nella
configurazione predefinita, e i vecchi Tab contavano controlli prima dell'aggiunta di Import/Search.
La prima passata 24.04 (`T10-all.txt`) ha perciò visitato «No connections yet»/Manage groups: **non è
una prova dei campi**. Lo script ora usa la rubrica seminata in `~/spike/cfg`, ancora il giro a Name e
More actions via AT-SPI e registra il fuoco reale dopo ogni tasto. Il tour usa il `vmctl.sh` del worktree,
con il terzo nome; una SSH fallita non diventa più silenzio del lettore.

Altro difetto del banco: un `grab_focus` AT-SPI riuscito non significa che il compositore abbia dato il
fuoco alla finestra. Il primo giro 26 leggeva Welcome to Ubuntu, quello Fedora era nella panoramica.
Ora il tour esige la proprietà **active del frame** prima di partire, prova Alt+Tab e fallisce se non
la ottiene. `fedora/T10-all-active-run.txt` conserva quel fallimento; `T10-no-focus.png` mostra la
panoramica. Dopo Esc è stato rifatto il giro (`T10-all-after-escape-run.txt`). I primi giri non sono
usati per attribuire difetti all'app o a Orca.

| Giro | VM | Detto (ordine/estratti) | Non detto o non verificato | Detto due volte nel giro |
|---|---|---|---|---|
| Campi generali | 24.04/46.1 | Name entry Ufficio Napoli → Group combo box/Lavoro → Server read only entry selected → Transport/UDP → Notes entry; al ritorno stessi controlli in ordine inverso | — per questi campi; non prova tutti i form | nessun nome di campo duplicato nello stesso passo; i ritorni con Shift+Tab sono voluti |
| Campi generali | 26.04/50.3 | main content → Name/entry/Ufficio Napoli/Focus mode; Group, Server, Transport/UDP, Notes; ritorno inverso | — per questi campi | alcuni «tab»/«left shift» due volte, non il contenuto del campo |
| Campi generali | Fedora/51.1 | main content → Name/entry/Ufficio Napoli/Focus mode | Nei Tab successivi il fuoco registrato è rimasto Name: **resto del giro inconclusivo**, non dichiarare gli altri campi muti | alcuni «tab» e «return» due volte |
| More actions/Duplicate | 24.04 | More actions combo box → More actions menu → Duplicate → More actions combo box | non trovato nel parlato il testo di conferma della duplicazione; nessun ascolto | «More actions combo box» poi la stessa frase con punto; non doppio annuncio di successo |
| More actions/Duplicate | 26.04 | More actions/menu/Duplicate; dopo Invio una stringa vuota/di spazi e Browse mode | conferma della duplicazione non letta in quel passo (`T10-all-active-orca.log`, riga SPEECH OUTPUT multilinea) | nessun successo duplicato osservato |
| More actions/Duplicate | Fedora | tentativo registrato | giro non arrivato al menu Duplicate: **inconclusivo** | un «return» doppio in un tentativo precedente; non un avviso dell'app |
| Barra di stato, daemon-down/up | tutte e tre | nessuna frase di stato nei 7 s osservati per ciascun passo | In down l'albero mostra Start the service; in up a 7 s lo mostra ancora: **ripristino non verificato in questa finestra**, non un verde e non una diagnosi della causa | nessuna |
| Credenziali | 24.04/46.1 | all'apertura niente; dopo attivazione alt-tab Galleria frame; Tab legge Password password text | titolo del dialogo e User name non detti nel giro, anche se il fuoco AT-SPI è User name | non osservata duplicazione del contenuto; «left alt» due volte |
| Credenziali | 26.04/50.3 | main content → Cancel/button → Browse mode → Credentials for «Datacenter Milano»/dialog → User name/entry required → invalid entry/Focus mode | non ascoltato l'audio | nessuna ripetizione del titolo/nome osservata |
| Credenziali | Fedora/51.1 | primo giro: titolo, dialog/form, User name entry required, invalid entry; giro finale: solo required, invalid entry, Focus mode | titolo/nome non ripetuti nel giro finale; **risultato variabile**, non una promessa basata sul primo giro | nessun titolo doppio osservato |

Fonti (tutte sotto la radice delle prove):
- 24.04: `T10-all-seeded.txt`, `T10-all-orca.log`, `T10-credentials.txt`,
  `T10-credentials-activated.txt`, `T10-credentials-full.txt`, `T10-credentials.png`.
- 26.04: **`T10-all-active.txt`**, `T10-all-active-orca.log`, `T10-credentials.txt`.
- Fedora: **`T10-all-active.txt`**, `T10-all-active-orca.log`, `T10-all-after-escape-run.txt`,
  `T10-credentials.txt` (primo giro), `T10-credentials-final.txt`.
- `orca-summary.json` raccoglie 14 passi per VM e il parlato per passo; è un indice, non sostituisce
  il debug originale per le stringhe multilinea.

**Q28/S57/R2:** i nomi dei campi generali funzionano con Orca46 nel desktop vero, quindi il buco
osservato nell'immagine non va esteso a tutti i campi della 24.04. Il dialogo credenziali conserva il
limite delle prove osservato sopra; **l'apertura su 24.04 era inizialmente inattiva**. Dopo Alt+Tab il frame
è attivo, Tab legge Password e il ritorno a User name resta muto, ma non è stato riaperto il dialogo
partendo da un frame già attivo. **Non è isolata la causa focus/compositore contro Orca46**: ripetere
questo controllo prima di sciogliere S57 per il dialogo. Q49(a) non cambia, né cambia il codice del dialogo. `NO_KEYRING=1` nei giri credenziali
isola il lettore dal portachiavi bloccato: **R8 non è provata**. Il primo script non trovava Connect perché
AT-SPI 2.52 lo chiama `push button` (non `button`): prima prova conservata, poi il helper accetta entrambi
e fallisce se il click non c'è. Non è stato corretto il prodotto per un errore della sonda.

**R3/G12 ancora parziali:** non eseguiti i quattro giri nuovi completi per ogni VM (rubrica con salvataggio,
import con anteprima, connessione/stato/errore, tray/guardia/annuncio a finestra nascosta). Nessuno di questi
viene dedotto dal giro di campi o dalla suite a11y del gruppo C. Prima della chiusura della fase occorre
riprenderli, insieme a T9.

### G13 / Q57(a): avvio a freddo, tre numeri per due VM

`lab/vm/startup-time.sh <vm> [tag]` fa tre volte `sync; echo 3 > /proc/sys/vm/drop_caches` **nella sola VM**,
mai sull'host, avvia l'app installata con `GALLERIA_STARTUP_TRACE=1`, esige i quattro segni una volta ciascuno
e conserva anche gli stderr dei giri falliti. È stato visto **rosso** senza la strumentazione:
`ubuntu24/startup-red-without-trace/run.txt`, exit 1, «main occurred 0 times». Dopo la modifica, entrambe
le serie sotto escono con 0. L'app senza variabile non scrive i segni (giro normale, `ubuntu/T10-without-trace.txt`).

| VM/giro | main | fine setup | ritorno di show | prima chiamata pagina |
|---|---:|---:|---:|---:|
| 24.04 #1 | 0 ms | 212 ms | 212 ms | **512 ms** |
| 24.04 #2 | 0 ms | 229 ms | 229 ms | **517 ms** |
| 24.04 #3 | 0 ms | 237 ms | 237 ms | **537 ms** |
| 26.04 #1 | 0 ms | 3834 ms | 3834 ms | **4307 ms** |
| 26.04 #2 | 0 ms | 220 ms | 220 ms | **420 ms** |
| 26.04 #3 | 0 ms | 208 ms | 208 ms | **408 ms** |

Fonti: `ubuntu24/startup-trace-green/{run.txt,stderr.txt}` e `ubuntu/startup-trace-green/{run.txt,stderr.txt}`.
Il **4307 ms** non è tolto come anomalo. G13 non blocca, né si dichiara «sempre sotto 1 s».

Macchina: Intel Core i9-9900KF @ 3.60 GHz, KVM, 4 vCPU e 4 GiB assegnati per VM; `free` nel giro misura
3909 MiB accessibili sulla 24.04 e 3391 MiB sulla 26.04. Dischi su `/mnt/b`, HDD Seagate ST2000DM008
rotazionale, non SSD. **È una VM sulla macchina di Peppe, non la macchina di Peppe.** I cache dell'host
non sono stati svuotati. L'altra VM e il build/check hanno potuto contendere risorse: misura locale,
non un benchmark isolato e non un confronto delle distribuzioni.

Confine della misura, importante: l'orologio parte entrando in `run()`, **non comprende il caricatore
dinamico prima del codice Rust**. `window_shown` è il ritorno della richiesta di show, non il primo
fotogramma dipinto. `page_first_call` è `ui_state_get`, già invocata dalla pagina prima di montare React:
**non è ancora il primo paint né la piena usabilità**. Per questo questi numeri non dimostrano alla lettera
«finestra usabile entro un secondo». Scostamento da P14: non serve modificare `main.tsx` né aggiungere un
comando IPC: si marca la prima chiamata che la pagina fa già.

### Verifiche e uscita del banco

- `scripts/gui.sh build`: exit 0 (`build-trace.log`); il GUI strumentato è installato sulle tre.
- `scripts/gui.sh check`: exit 0 (`check-trace.log`), **Vitest 710/710**, cargo GUI **484/0**, clippy/typecheck verdi.
- `cargo fmt --all --check` e `scripts/check-boundaries.sh`: verdi. Crate vietati, Cargo.lock, design e docs/00–07 non toccati.
- Nessun nuovo `smoke`, `a11y` di container o CI in questo gruppo: non riciclare i numeri del C come eseguiti sul nuovo trace.
- Fine banco: `final-vm-status.txt`, **ubuntu24/ubuntu/fedora stopped**. Dischi lasciati in `/mnt/b/vm/disks/`, nessuna immagine nel repo.

## Giro quotidiano sulle tre VM (card `t_6670f193`): Fedora 45 e Ubuntu 26.04

Stessa prova di `t_a16d68aa` (`lab/vm/daily-vm.sh`, server WireGuard vero in un namespace con `wg-peer-vm.sh`), sugli stessi binari del gruppo C
(`bcb3f39`, hash in `/mnt/b/vm/evidence/b6/binaries-bcb3f39.sha256`; dopo `bcb3f39` il codice di prodotto cambia solo per la traccia di avvio
opzionale `GALLERIA_STARTUP_TRACE`). Prove in **`/mnt/b/vm/evidence/b6/daily2/`** (`fedora/`, `ubuntu/` = 26.04; per 24.04 resta `daily/ubuntu24/`).
Come prima: i pulsanti della finestra con l'azione AT-SPI `press` (su queste due il ruolo si chiama `button`, non `push button`), file chooser e menu della
tray con la tastiera vera (`sendkey`), il puntatore VNC non consegna clic.

| Passo | Ubuntu 24.04 | Ubuntu 26.04 (GNOME 50.1) | Fedora 45 (GNOME 51.1) |
|---|---|---|---|
| 1 Import | verificato | **limite** (F1): senza portachiavi predefinito «The keyring did not answer» (`ubuntu/06-F1-keyring-did-not-answer.png`); dopo aver creato il portachiavi (`keyring-create.py`, `06b`–`06e`) verificato (`06f-after-import2.png`) | **verificato** (`05c-preview.png`, `06b-unlock-empty.png`): qui il «Login» esiste ma è **bloccato**, GNOME chiede lo sblocco (`06-after-import.png`), con password vuota Import finisce e mostra «Imported “daily-office”» |
| 2 Connetti dalla finestra | verificato | **verificato** (`ubuntu/07-connected-state.txt`: connected, handshake, ping 10.99.0.1 ok, `glr-wg0`) | **verificato** (`fedora/07-connected-state.txt`, stesso esito) |
| 3 Chiudi nella tray | verificato | **verificato** (`08-closed.png`: finestra chiusa, pallino verde in barra; `08-closed-state.txt`) | **verificato**, ma con **F2** (`fedora/08-closed.png`: «…» al posto dell’anello) |
| 4 Il tunnel resta su | verificato | **verificato** (`08-closed-state.txt`: connected, ping ok) | **verificato** (`08-closed-state.txt`) |
| 5 Disconnetti dal menu della tray | verificato | **verificato** (`09-tray-menu.png`, `09b-sel.png`, `10-disconnected-state.txt`: disconnected, nessuna interfaccia, ping FAIL), con **F2** | **verificato** (`09-tray-menu.png`, `09b-sel.png`, `10-disconnected-state.txt`) |
| 6 Credenziali nel portachiavi | verificato (chiave WG); **limite** utente/password | **verificato per la chiave WG** (`07-connected-state.txt`, elemento «Galleria: daily-office (wireguard.private_key)»); **limite**: password Ricorda di OpenVPN non verificata; portachiavi creato **non cifrato** | **verificato per la chiave WG** (stesso elemento nel «Login»); **limite**: Ricorda di OpenVPN non verificato |
| 7 Riapertura | verificato | **verificato** il riavvio completo dal menu delle applicazioni (profilo presente, riconnessione ok: `16-relaunched.png`, `17-reconnected-state.txt`); «Open Galleria» dal menu della tray: **non verificato** (la sequenza di tasti è finita nella Panoramica) | **verificato** («Open Galleria» dalla tray `15-reopened-tray.png`; riavvio dal menu applicazioni e riconnessione `16-relaunched.png`, `17-reconnected-state.txt`) |
| T10 (screen reader) | parziale (24.04) | **non verificato** (non si fa per VM, deciso) | **non verificato** (idem) |
| Q49 | verificato come esito | **non verificato** | **non verificato** |

Cautele: ogni passo è **un solo giro** per VM. Le VM erano rimaste con file a zero byte dopo lo spegnimento brusco dell’altra card (`keyring-create.py`,
`galleria-gui` in `/usr/local/bin` sulla 26.04); ricopiati e controllati con l’hash prima del giro.

### S56, menu della tray su Fedora: esito

- **Il menu della tray funziona su Fedora 45**: si apre (`fedora/09-tray-menu.png`), con «Connected to «daily-office»», «Disconnect», «Open Galleria», «Quit (the VPN stays on)»,
  si naviga con la tastiera e «Disconnect» e «Open Galleria» fanno ciò che dicono (passi 5 e 7). La libreria caricata è `libappindicator3.so.1` (`fedora/versions.txt`).
- **Il timeout «Unable to connect to the Notification Watcher: Timeout was reached» non si è riprodotto** in nessuno dei 17 avvii di questo giro (3 dal menu, 14 con
  `s56-repeat.sh`, alcuni con `SIGKILL` e pausa di 3 s: sempre registrato e con icona). Il tentativo di causarlo:
  sessione bloccata (`s56-locked.txt`: nessun watcher, l’app parte **senza** icona, compaiono i due `Gtk-CRITICAL` identici a quelli di T9, e **l’icona si registra da sola allo sblocco**),
  VT in secondo piano (`s56-vt.txt`: nessun effetto), `gnome-shell` fermato con `SIGSTOP` per 55 s (`s56-freeze.txt`: nessun avviso; l’icona si registra alla ripresa).
- **Conclusione onesta: la causa del timeout non è stata trovata, ma nulla indica il nostro codice.** I due `Gtk-CRITICAL` compaiono quando l’estensione non c’è al momento dell’avvio
  (sessione bloccata, greeter davanti, shell sotto carico come durante il provisioning di T9); in tutti i casi provati l’app si riprende da sola quando l’estensione torna. **Limite scritto
  (ambiente)**: se all’avvio l’estensione AppIndicator non risponde (schermata bloccata, GNOME ancora in avvio) l’icona compare più tardi, non subito. Per riprovare:
  `lab/vm/daily-vm.sh launch` e `journalctl -b | grep -i 'Notification Watcher'`; `s56-locked.sh`, `s56-freeze.sh`, `s56-repeat.sh` (in `daily2/`).

### F2 — l’icona della tray resta «…»: causa trovata, è un corto circuito tra noi e l’estensione (riprodotto su 24.04, 26.04 e Fedora 45)

Prova (`fedora/icon-watch.log`, `fedora/tray-icon-journal.txt`, `ubuntu/tray-icon-watch-and-journal.txt`; `icon-watch.py` annota i file dell’icona al millisecondo):
a ogni cambio di stato l’app passa da `On` a `Busy` e subito allo stato finale (30–40 ms dopo: `tray-icon-galleria-4.png` esiste 37 ms, poi è
cancellato e scritto `-5`). Il crate `tray-icon` cancella il PNG precedente prima di scrivere il successivo. L’estensione AppIndicator legge `-4` quando ormai
non c’è più e registra «Impossible to read image info … Failed to recognize image format / unable to update icon» (stesso messaggio, stessi tempi, su
`ubuntu-appindicators` e su `appindicatorsupport@rgcjonas`), **e non si riprende con l’icona `-5` che segue**: la barra resta con «…» finché non si riavvia l’app
(riavviata: anello giusto, `16-relaunched.png`). Il file su disco e `IconName` erano già quelli giusti. È **nostro nel senso che lo provochiamo** (due cambi in ~40 ms); il difetto
è anche dell’estensione, che non si riprende. **Non corretto in questa card** (fuori dai quattro punti): la correzione minima è non applicare l’icona `Busy`
se lo stato finale arriva entro ~200–300 ms (raccogliere i cambi in `TauriSurface::show`, `crates/galleria-gui/src/tray_tauri.rs`) e rifare la prova con `icon-watch.py` + `journalctl`. Su 26.04
il pallino verde compare ma dopo la disconnessione si vede «…» (`ubuntu/12-menu-disconnected.png`).

#### F2 — correzione (card `t_479c2489`): i cambi ravvicinati si raccolgono e si applica solo l’ultimo

**Che cosa**: `IconSettler` (`crates/galleria-gui/src/tray.rs`, senza tipi di Tauri) tiene l’icona che il modello vuole e la mette sul crate `tray-icon` solo quando
è rimasta quella per `ICON_SETTLE` = **300 ms**; un cambio che torna a ciò che è già sulla barra non mette niente. `TauriSurface::show`
(`tray_tauri.rs`) chiama `want(icon)` e riscrive subito il menu (il menu non ha il problema); l’icona la mette il timer, sul thread della finestra. Due icone messe non
stanno mai a meno di 300 ms una dall’altra, quindi il file precedente, che il crate cancella, ha avuto 300 ms per essere letto (la lettura in ritardo è di qualche decina di ms).

**Perché questa strada e non un nome di file sempre diverso**: `tray-icon` 0.25.1 (Linux, `platform_impl/gtk/mod.rs`, `set_icon`) scrive già un nome nuovo a ogni `set_icon` (`tray-icon-galleria-N.png`, con un contatore) e **cancella
comunque il precedente come prima riga di `set_icon`, prima di scrivere il successivo** (si vede in `icon-watch.log`: `- …-4.png` e `+ …-5.png` allo stesso millisecondo): non ha un’opzione per tenerlo, e per
non cancellarlo dovremmo scrivere noi i file e dare al crate un percorso, cioè riscrivere la parte del crate che fa l’icona. Raccogliere i cambi sta in una struttura nostra di una quarantina di righe, si prova
senza VM (`tests/tray.rs`) e toglie anche un «Busy» che non serve a nessuno (non si vede per meno di 300 ms). Costo: l’icona segue lo stato con 300 ms di ritardo; il menu no.

**Prova rossa, poi verde** (senza VM, `cargo test -p galleria-gui --test tray`, nell’immagine): con un `want` che applica subito, `two_changes_closer_than_the_wait_put_one_icon_the_last`
(`Busy` e dopo 40 ms `Off` con l’attesa vera) dava `[Busy, Off]` invece di `[Off]`, e due delle altre tre prove nuove cadevano; con la correzione 37/37. Le altre
(`a_change_that_comes_back_to_what_is_shown_puts_nothing`, `a_change_that_stays_is_put_after_the_wait_and_not_before`, `icons_that_are_put_are_never_closer_than_the_wait`)
fissano il ritorno a ciò che è già mostrato, l’attesa e la distanza minima.

**Prova su Ubuntu 24.04 (GNOME 46, estensione `ubuntu-appindicators`), build release dell’albero della card**, demone finto (`examples/fake-daemon --scenario plain`) e clic AT-SPI
su Connect/Disconnect della finestra (`lab/vm/f2-run.sh`; prove in `/mnt/b/vm/evidence/b6/f2/`, copia dalla VM in `vm-out/`). Stesso giro, due binari:

| | binario del giro quotidiano (`b47de8e`, `bcb3f39` per la tray) | binario con la correzione |
|---|---|---|
| file dell’icona (`icon-watch.log`) | al primo Connect `…-2.png` (Busy) esiste 45 ms e `…-3.png` lo segue; poi coppie da 3 ms (`…-4.png`) | **un solo file per cambio**, il Busy c’è solo se il tunnel ci mette più di 300 ms |
| journal (`journal.txt`) | `Impossible to read image info from path …/tray-icon-galleria-2.png` + `unable to update icon: … Failed to recognize image format` | **0** «unable to update icon» in 3 giri (4 cambi a 5 s, 4 cambi a 2 s, 2 cambi a mano) |
| barra (fotografie) | anello dopo la disconnessione (`03-control-old-final-disconnected.png`; il «…» non è comparso in questo giro, l’errore sì) | pallino verde dopo Connect (`05-fixed-connected.png`), anello dopo Disconnect (`04`, `06`) |

Il controllo con il binario vecchio riproduce l’errore nello stesso giro (`journal-since-control.txt`: una sola riga di errore da quando c’è il vecchio binario, nessuna dopo il nuovo).
**Limite onesto**: con il binario vecchio il «…» **permanente** non si è rivisto in questo giro (l’errore sì, una volta, e l’icona seguente era giusta): l’estensione perde il
file in una corsa, non a ogni cambio. La prova che la correzione toglie la corsa è il file che esiste ≥ 300 ms e la riga di errore che non compare in 10 cambi; non è
«il “…” non c’è più in 100 cambi». Non rifatto su 26.04 e Fedora (la card chiedeva una sola VM).

### F1 anche su 26.04, F3 non confermata

F1 (nessun portachiavi predefinito con accesso automatico senza password → «The keyring did not answer») si riproduce su **26.04**; su **Fedora** il «Login» esiste ma è bloccato e il
prodotto aspetta lo sblocco (corretto). F3 (due separatori di fila): visto su Fedora a menu «disconnesso» in una fotografia (`13c-menu.png`), un solo separatore in un’altra (`14d.png`): non determinato.

#### F1 — esito (card `t_479c2489`): **non** era il comportamento di D25, corretto

**Che cosa succedeva** (letto nel codice, `secrets_keyring.rs`): il servizio Secret Service c’è (GNOME Keyring parte da solo sul bus), quindi `KeyringSecretStore::open()` dava
«c’è un portachiavi»; ma senza collezione predefinita `set` dava `Keyring("no default collection")` → `gui_keyring_failed` → «The keyring did not answer. Nothing was saved.»,
e Import si fermava. D25 dice invece: la GUI scrive nel portachiavi **e ripiega sul file, con avviso, se il portachiavi non c’è**, e «il blocco non si ferma». Un servizio
senza una collezione dove mettere un segreto non è un portachiavi (crearla è una finestra che chiede una password, e non va fatta da sola).

**Prova rossa, poi verde, su un Secret Service vero** (nell’immagine, nuovo giro `nodefault` di `scripts/gui.sh keyring`: GNOME Keyring avviato senza password, nessuna collezione;
`keyring_without_a_default_collection_is_no_keyring`, riga G5 in `lab/tests/gates-b5.tsv`, argomento in più al controllo di CI): prima `keyring_present()` dava `true` («a service with no
default collection is not a keyring»), poi `false`; con l’archivio a strati la scrittura finisce nel file `0600` (`StoreKind::File`, quindi `secret_store_status` dà `file` e la finestra
mostra l’avviso «No system keyring: the keys are saved in a file only you can read.») e si legge indietro. Gli altri tre giri (sbloccato, bloccato, assente) restano verdi.

**Correzione minima**: `keyring_present()` vuole anche che esista la collezione predefinita (`get_default_collection`, una `ReadAlias` che non apre né sblocca niente). Una collezione
predefinita **bloccata** è ancora un portachiavi (Fedora: chiede lo sblocco, nessun ripiego silenzioso, come la regola 2 di D25). Vale anche per la CLI, che usa lo stesso `open()`.
**Limiti**: se il portachiavi viene creato dopo, dalla chiamata successiva (ogni chiamata riapre una connessione) si scrive lì e si legge da tutti e due; i segreti messi nel file non si
migrano da soli (regola 3). Una collezione che non è la predefinita non è letta. **Non rifatto sulla VM 26.04** (la card ammetteva la prova nell’immagine): il caso è lo stesso
che `ubuntu/06-F1-keyring-did-not-answer.png` mostra, e il giro `nodefault` lo riproduce con lo stesso GNOME Keyring dell’immagine, non con quello di Ubuntu 26.04.

### Non verificato, riepilogo

26.04: «Open Galleria» dalla tray; Ricorda di OpenVPN; Q49; T10. Fedora: Ricorda di OpenVPN; Q49; T10; il timeout S56 (non riprodotto). Matrice M1–M12: invariata (fuori giro quotidiano).
