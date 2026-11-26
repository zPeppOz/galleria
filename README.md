# Galleria

Client desktop open source per OpenVPN e WireGuard (poi le VPN aziendali via OpenConnect e le reti filtrate): una rubrica di connessioni in stile VirtualBox,
una connessione attiva alla volta, Linux e Windows (macOS in seguito).

Stato: demone Linux, CLI e finestra (Tauri, solo Linux) implementati, con laboratorio Docker per WireGuard e OpenVPN.
La fase 2 (la finestra) è chiusa in locale il 26/11/2026; **ciò che non è provato e i limiti sono scritti in
«Chiusura della fase 2» di [`docs/08-esito-fase2.md`](docs/08-esito-fase2.md)**, e il run di CI sull'ultimo commit
si legge dopo il push. Windows è la fase 3 (vedi [`ROADMAP.md`](ROADMAP.md)).
Vedi [esito della fase 1](docs/06-esito-fase1.md) e [laboratorio](lab/README.md).

![Mockup rubrica](design/mockup/rubrica-connesso.png)

**Tray e chiusura.** L'icona nell'area di notifica dice lo stato (spenta, accesa, in corso) e il suo menu ha i preferiti
(al più otto, «Connect» o «Disconnect»), «Open Galleria» e «Quit». Su Linux Tauri non riceve i click sull'icona: tutto
sta nel menu. Con la tray visibile la «X» **nasconde** la finestra e la VPN resta su; «Quit» (nel menu della tray, in
quello dell'app e con `Ctrl+Q`) chiude la finestra, non la VPN, e lo dice («Quit (the VPN stays on)»). Con modifiche non
salvate, la «X» e «Quit» chiedono prima «Save · Discard · Cancel». **Su GNOME la tray si vede solo con l'estensione
AppIndicator** (le tre VM di prova la avevano attiva); senza l'estensione, o senza la libreria dell'indicatore,
l'app parte lo stesso senza tray e la «X» esce dall'app, con la VPN ancora su.

Un solo `galleria-gui` alla volta: avviato con un file (`galleria-gui profilo.ovpn`, o il doppio click) mentre la finestra è già aperta,
le consegna il file e termina; il canale è un socket `0600` in `$XDG_RUNTIME_DIR/galleria` (senza quella cartella l'app parte lo stesso, per conto suo).
Solo i `.ovpn` sono associati all'app (`packaging/linux/io.github.zpeppoz.galleria.desktop`, non installato dall'installer del prodotto: i pacchetti sono della fase 4; `lab/vm/gui-vm.sh` lo installa solo nelle VM del laboratorio).

**Portachiavi.** La finestra salva le chiavi dei profili (e utente e password, solo se si spunta «Ricorda») nel portachiavi di sistema (Secret Service: GNOME Keyring; KWallet non è stato verificato). Senza portachiavi ripiega su un file `0600` in `~/.config/galleria/secrets/` e lo dice con un avviso fisso; un portachiavi bloccato è un errore, mai un ripiego silenzioso. Il codice OTP e la passphrase della chiave non si salvano mai. La CLI legge dai due archivi, ma scrive solo nel file.

**Limiti noti** (l'elenco intero, con le prove, è «Limiti dichiarati» in [`docs/08-esito-fase2.md`](docs/08-esito-fase2.md)).
Provato solo su GNOME (Wayland): Ubuntu 24.04, Ubuntu 26.04 e Fedora 45, con il giro quotidiano (importa, connetti,
chiudi nella tray, il tunnel resta, disconnetti dal menu, riapri) eseguito da un agente con un tunnel WireGuard vero, non
da una persona alla tastiera; la matrice più larga dei casi
manuali (file manager, finestre «Apri» e «Salva» con il mouse, dimensioni su Wayland, portachiavi bloccato) **non è stata
verificata**. KDE, KWallet e altri desktop non sono provati; i portali (Flatpak, Snap) neppure.
- **Tray su GNOME.** L'icona segue lo stato con 300 ms di ritardo: l'app raccoglie i cambi ravvicinati e mette solo
  l'ultimo (prima cambiava l'icona due volte in ~40 ms, l'estensione perdeva il file e la barra restava su «…» fino al
  riavvio; il menu non ha ritardo). **Corretto e provato su Ubuntu 24.04** (nessun errore dell'estensione in circa 10
  cambi, il binario vecchio ne lasciava); su 26.04 e Fedora non è stato rifatto.
  Se l'estensione non risponde all'avvio (schermata bloccata, sessione che parte) l'icona compare più tardi.
- **Senza portachiavi predefinito** (accesso automatico senza password) l'import ripiega sul file `0600`, con l'avviso
  fisso: un servizio senza collezione predefinita non è un portachiavi, e l'app non la crea. Provato con GNOME Keyring
  vero nell'immagine di prova, non sulla VM 26.04. Su Fedora il «Login» è bloccato e la finestra aspetta lo sblocco del sistema.
- **Screen reader.** Le regioni vive di WebKitGTK sono mute con Orca: gli annunci arrivano dal lato Rust (segnale ATK).
  Con Orca 46 (Ubuntu 24.04) i campi generali si leggono, ma la finestra delle credenziali non è letta all'apertura
  (titolo e «User name» muti anche con la finestra attiva); con Orca 50 e 51 sì. Il parlato è stato letto dal file di
  debug di Orca: **nessun ascolto con le cuffie** (lista di venti minuti in [`docs/b6-ascolto-20-minuti.md`](docs/b6-ascolto-20-minuti.md)).
- **Wayland.** Il compositore può negare il primo piano a una finestra già aperta; si salvano dimensione e «massimizzata»,
  non la posizione.
- **Avvio a freddo.** Misurato in VM (tre avvii per due VM, fino alla prima richiesta della pagina, non al primo
  fotogramma): 408–537 ms sulle due Ubuntu, con un avvio da 4307 ms sulla 26.04; Fedora non misurata.

Dettagli e prove: [passata desktop B6](docs/08-b6-desktop-pass.md).

**Chi è nel gruppo `galleria` comanda la VPN della macchina** (D21): può connettere, disconnettere e vedere stato e log di ogni sessione, anche di un altro utente; la finestra mostra una sessione avviata da altri come «avviata fuori da questa rubrica» e chiede conferma prima di chiuderla o sostituirla. **Log del demone** (D20): `journalctl -u galleriad`; la finestra ne mostra il buffer (2000 righe) ed «Esporta log…» lo salva in un file `0600`.

## Cosa esce dalla tua macchina

La finestra ha **un solo contatto di rete**: la ricerca di una versione nuova (D16). Tutto il resto parla con il demone sul socket locale.

- **Dove**: un solo indirizzo, scritto nel codice, `https://github.com/zPeppOz/galleria/releases/latest` (la pagina dell'ultima release). La richiesta parte da Rust, mai dalla pagina; la CSP della finestra non ammette nessun indirizzo remoto.
- **Quando**: dieci secondi dopo l'avvio e poi al più **una volta ogni 24 ore** (un'occhiata all'orologio ogni ora, senza rete). Una richiesta fallita conta lo stesso per le 24 ore, e non si ritenta. Una finestra che si apre e si chiude non fa richieste.
- **Cosa porta**: un `GET` con tre intestazioni, quelle che libsoup manda da sé e che nessuno ha scelto: `Host: github.com`, `Accept-Encoding: gzip, deflate`, `Connection: Keep-Alive`. **Nessun `User-Agent`**, nessun cookie, nessun `Referer`, nessun parametro nell'indirizzo, nessun corpo, **nemmeno la versione installata**: GitHub non sa chi chiede né da quale versione (vede, come ogni sito, l'indirizzo IP e il suo certificato). La richiesta usa il proxy e i certificati di sistema. È la stessa cosa che asserisce la prova `update_asks_once_and_sends_no_identifier`.
- **Cosa legge della risposta**: il solo `Location` del rinvio che GitHub risponde, e di quello solo un **numero di versione** (`…/releases/tag/v<numero>`, stabile, dello stesso repository). Il rinvio non si segue, il corpo si butta, e l'indirizzo che la finestra mostra lo ricostruisce Rust dal numero. Un repository senza release risponde con un rinvio verso l'elenco: «nessuna versione nota», senza errore.
- **Come si spegne**: «Controlla se c'è una versione nuova» nel menu dell'app, oppure `check = false` nella tabella `[update]` di `~/.local/state/galleria/state.toml`. Spento, **non parte nessuna richiesta** (lo prova `update_disabled_asks_nothing`). L'avviso è una riga in cima alla lista, con l'indirizzo come testo, «Copia link» e «Non ricordarmelo»; non si annuncia e non apre niente. Una versione dell’app precedente a questa funzione non conosce la tabella `[update]` e, scrivendo il file, la perde: il controllo tornerebbe acceso.

## Documenti

- [`docs/00-mercato-e-concorrenti.md`](docs/00-mercato-e-concorrenti.md) — concorrenti, buco reale, validazione
- [`docs/01-analisi.md`](docs/01-analisi.md) — decisioni, funzionalità, architettura, fasi, rischi, punti aperti
- [`docs/02-modello-minacce.md`](docs/02-modello-minacce.md) — cosa proteggiamo, da chi, come
- [`docs/03-sessione-nello.md`](docs/03-sessione-nello.md) — scaletta delle decisioni tecniche con il CTO
- [`design/mockup/Rubrica.html`](design/mockup/Rubrica.html) — mockup cliccabile (aprire nel browser; stati demo con `#on`, `#log`, `#import`, `#tray`, `#light`)

## Licenza

GPL-3.0-or-later. Vedi [`LICENSE`](LICENSE).
