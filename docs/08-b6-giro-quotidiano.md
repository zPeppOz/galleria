
## Gruppo D, completamento (card `t_a16d68aa`): giro quotidiano, Q49, S56

Decisione di Aitano (26/11, su delega di Peppe): Galleria la usano Peppe e i colleghi, non è un prodotto. La matrice da 36 casi
non si completa; si fa per intero il **giro quotidiano** (import → connetti dalla finestra → chiudi nella tray → il tunnel resta su
→ disconnetti dal menu della tray → credenziali nel portachiavi → riapertura). Il resto è «non verificato», col motivo.

**Dove sono arrivato: solo Ubuntu 24.04 ha il giro intero. Ubuntu 26.04 e Fedora 45 non sono state toccate in questa card**
(le iterazioni di lavoro sono finite sulla 24.04: banco del tunnel, portachiavi, tastiera per la tray). S56 non è stata riaperta.

Prove in **`/mnt/b/vm/evidence/b6/daily/`** (`ubuntu24/`, più gli script `h.sh`, `h2.sh`, `type.py`). Percorsi sotto relativi a lì.

### Banco del tunnel vero

- `lab/vm/wg-peer-vm.sh up`: server WireGuard **vero** in un namespace `glabpeer` collegato alla VM da un veth (UDP su 10.77.0.2:51820),
  tunnel diviso 10.99.0.0/24 (la SSH non si tocca), scrive `~/spike/daily-office.conf`. Con la CLI, prima della GUI: import, connect,
  handshake lato server, ping 10.99.0.1 attraverso il tunnel, disconnect, delete: tutto ok.
- Il demone è quello vero (`gui-vm.sh <vm> daemon real`, servizio systemd, utente `lab` nel gruppo `galleria`). L'appartenenza al
  gruppo vale solo dal login dopo `usermod`: la VM è stata riavviata, come farebbe una persona.
- `lab/vm/daily-vm.sh`: avvio dal menu applicazioni (`gtk-launch`), stato (demone, interfaccia, handshake, ping, portachiavi **solo
  attributi, mai segreti**), `orca-on`/`said` per il giro con Orca. `lab/vm/keyring-create.py`: vedi F1.
- Come si preme: **il puntatore VNC/qemu non consegna i clic** (provato quattro volte, anche su Activities): i pulsanti della finestra
  si premono con l'azione AT-SPI `press` (`act.py click <nome> 'push button'`); tutto il resto (file chooser, tray, chiusura) con la
  tastiera vera via `sendkey`. Il menu della tray si apre con `Ctrl+Alt+Tab`, frecce, `Invio`; la sequenza dipende da dove sta il fuoco
  (vedi limite sotto).

### Giro quotidiano, Ubuntu 24.04 (GNOME 46, Wayland)

| # | Passo | Esito | Prova |
|---|---|---|---|
| 1 | Import | **verificato** (dopo F1) | `ubuntu24/04-preview.png` (anteprima WG, «private key … nel portachiavi»), `05-imported.png` («Imported “daily-office”») |
| 2 | Connetti dalla finestra | **verificato** | `07-connected.png`, `07-connected-state.txt`: connected, handshake, ping 10.99.0.1 ok, interfaccia `glr-wg0`; icona tray verde |
| 3 | Chiudi nella tray | **verificato** | `08-closed.png`: finestra sparita, icona verde in barra, processo vivo |
| 4 | Il tunnel resta su | **verificato** | `08-closed-state.txt`: stato connected, `glr-wg0` presente, ping ok a finestra chiusa |
| 5 | Disconnetti dal menu della tray | **verificato** | `09-tray-menu.png` (menu aperto: «Connected to «daily-office»», «Disconnect «daily-office»», «Open Galleria», «Quit (the VPN stays on)»), `09b-sel.png`, `10-disconnected-state.txt`: disconnected, nessuna interfaccia, ping FAIL |
| 6 | Credenziali nel portachiavi | **verificato per la chiave privata WG**; **limite** per utente/password | `11-keyring.txt`: elemento «Galleria: daily-office (wireguard.private_key)» nel portachiavi; nessuna chiave in `~/.config/galleria` (`secrets/` vuota, grep senza risultati). Un profilo WG non ha utente/password: il «Ricorda» di OpenVPN **non verificato** nel giro. Il portachiavi è stato creato **non cifrato** (password vuota) |
| 7 | Riapertura | **verificato** | «Open Galleria» dalla tray (`13-reopened-tray.png`); uscita completa del processo e nuovo avvio dal menu applicazioni: profilo presente, Connect senza richieste, di nuovo connected (`15-relaunched.png`, `16-reconnected-state.txt`) |

Ubuntu 26.04 e Fedora 45: eseguiti nella card `t_6670f193`, vedi [08-b6-desktop-pass.md](08-b6-desktop-pass.md), «Giro quotidiano sulle tre VM» (qui sotto restano le celle di quella consegna).

### Trovato nel giro

- **F1 — primo salvataggio col portachiavi che manca.** Sulla VM con accesso automatico senza password **non esiste il portachiavi
  predefinito**; Import mostra «The profile was not imported. The keyring did not answer. Nothing was saved.» (`05-imported.png` della
  prima passata, riga `crates/galleria-profile/src/secrets_keyring.rs` `set`: «no default collection»). Non è un portachiavi bloccato:
  il prodotto non crea la collezione. Su un desktop con accesso a password il portachiavi «login» c'è già e il caso non si presenta.
  Per proseguire la collezione è stata creata dal prompt di GNOME (`06-keyring-prompt.png`; `lab/vm/keyring-create.py`; password vuota →
  «Store passwords unencrypted?»). ~~Da decidere: messaggio più chiaro o creazione della collezione; non toccato il codice.~~
  **Chiusa nella card `t_479c2489`**: senza collezione predefinita il portachiavi non conta, i segreti vanno nel file con l’avviso (D25), senza fermare Import
  (`08-b6-desktop-pass.md`, «F1 — esito»).
- **F2 — icona della tray dopo la disconnessione.** Dopo qualunque ritorno allo stato «spento» (dal menu o dal pulsante della finestra)
  la barra di GNOME 46 mostra «…» (tre puntini) al posto dell'anello; il file scritto in `/run/user/1000/galleria/tray-icon-galleria-N.png`
  è invece l'anello giusto (`tray-icon-galleria-5.png`, `tray-icon-probe.txt`: `IconName` cambia nome a ogni cambio). Un nuovo avvio
  mostra l'anello (`15-relaunched.png`); il verde in connessione si vede (`07-connected.png`). Il menu funziona anche con i puntini
  («Disconnected»). **Causa non isolata** (estensione `ubuntu-appindicators` che non ricarica l'icona o scrittura del file): da guardare
  su 26.04 e Fedora prima di dire che è nostro o dell'ambiente (`lab/vm/tray-icon-probe.sh`).
- **F3 — due separatori di fila** nel menu della tray quando non ci sono preferiti (`12h-menu-disconnected.png`, `13a.png`). Estetico.
- Il banco: `Ctrl+Alt+Tab` dà il fuoco alla barra solo se non c'è un altro frame attivo. Due volte ha sbagliato bersaglio: ha aperto
  «Home» dalle icone del desktop e, un'altra, ha attivato il tasto «Wired» (rete spenta, SSH persa; riacceso). Prova da rifare con
  `Super`+tasti, o dopo aver controllato il fuoco con una fotografia prima di `Invio`.

### Q49 su 24.04: un tentativo (`ubuntu24/q49-open.txt`, `q49-orca-debug.log`, `q49-opened.png`)

`orca-credentials-vm.sh prep` poi `open`: **frame `active` prima dell'apertura** (AT-SPI: `yes`; Orca ha detto «Galleria frame.»),
poi apertura delle credenziali di «Datacenter Milano» (demone finto, scenario `credentials`).
Esito: la finestra si apre (`q49-opened.png`), AT-SPI mette il fuoco su **User name** (entry, `focused`, `required`) e Orca riceve
`state-changed:focused` e `text-caret-moved` per quella entry, ma **dice solo «Cancel push button.»**: né il titolo del dialogo né
User name. Nel debug il «Locus of focus» resta `[push button: 'Cancel']` e compaiono «Ignoring because event is not from active app».
Quindi **il silenzio si riproduce con il frame già attivo: non è il fuoco del compositore.** È dalla parte di Orca 46.1 (che non sposta
il suo fuoco logico sul campo e scarta l'evento), non una mancanza di etichette nell'app. Q49(a) non cambia, nessun codice toccato.
Cautela: un solo tentativo, una sola VM; quanto a «perché Orca scarta l'evento» è un'ipotesi dal debug, non provata.

### T10, un giro con Orca sul giro quotidiano, 24.04 (`ubuntu24/T10-daily-orca.txt`, `T10-daily-orca-debug.log`)

Detto: apertura del dialogo di Import («Import profile dialog … Paste the text of the profile entry»), scelta file (`Choose a file… push button`,
chooser «Files»), `Galleria frame. Name entry daily-office.` all'anteprima, `Group combo box`, `Cancel/Back/Import push button`, **«Imported “daily-office”.»**,
**«Connected to daily-office»** dopo Connect (annuncio dello stato). Non detto: nessuna frase alla chiusura nella tray. **Non completato**:
la sequenza di tastiera per la tray è finita sul fuoco delle icone del desktop («Desktop Icons 1 frame», «scroll pane»): il menu della tray **non è
stato letto** da Orca e la disconnessione dal menu non è avvenuta in questo giro (tunnel rimasto su, poi spento col demone). Il giro con Orca
sul menu della tray è quindi **non verificato** su 24.04; 26.04 e Fedora: **non verificato** (non eseguiti).

### S56, menu della tray su Fedora

**Non eseguita in questa card** (il budget è finito sulla 24.04). Resta come nel report precedente: libreria `libappindicator3` caricata, timeout
del Notification Watcher negli avvii successivi, causa non trovata; **non è un limite accettato**. Passi per riprendere: avviare Fedora 45,
`gui-vm.sh fedora daemon real`, riavviare la VM (gruppo), `daily-vm.sh launch`, leggere `daily-app.log` per il Notification Watcher,
`gdbus call … org.kde.StatusNotifierWatcher … RegisteredStatusNotifierItems` per vedere se l'elemento è registrato, e controllare
`gnome-extensions list --enabled` per l'estensione AppIndicator.

### Stato delle celle (giro quotidiano)

| Passo | Ubuntu 24.04 | Ubuntu 26.04 | Fedora 45 |
|---|---|---|---|
| Import | verificato | non verificato (giro non eseguito) | non verificato (giro non eseguito) |
| Connetti dalla finestra | verificato | non verificato | non verificato |
| Chiudi nella tray | verificato | non verificato | non verificato |
| Tunnel su a finestra chiusa | verificato | non verificato | non verificato |
| Disconnetti dal menu della tray | verificato | non verificato | non verificato; S56 aperta |
| Portachiavi | verificato (chiave WG); limite per utente/password | non verificato | non verificato |
| Riapertura | verificato | non verificato | non verificato |
| Q49 | verificato come esito (silenzio con frame attivo) | n/a | n/a |
| T10 giro Orca | parziale (import e connessione; tray non letta) | non verificato | non verificato |
| Matrice M1–M12 restante | non verificato (fuori dal giro quotidiano, decisione 26/11) | idem | idem |
