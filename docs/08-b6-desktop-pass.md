
## Gruppo D — desktop veri (B6, T8–T10), giro parziale

> **Completamento (card `t_a16d68aa`):** giro quotidiano, Q49 e S56 in [08-b6-giro-quotidiano.md](08-b6-giro-quotidiano.md). Solo Ubuntu 24.04 ha il giro intero; le altre due sono «non verificato».

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
