# Galleria

Client desktop open source per OpenVPN e WireGuard (poi le VPN aziendali via OpenConnect e le reti filtrate): una rubrica di connessioni in stile VirtualBox,
una connessione attiva alla volta, Linux e Windows (macOS in seguito).

Stato: demone Linux e CLI minima implementati, con laboratorio Docker per WireGuard e OpenVPN.
La GUI resta un mockup; CI remota, desktop con NetworkManager e DCO restano da verificare.
Vedi [esito della fase 1](docs/06-esito-fase1.md) e [laboratorio](lab/README.md).

![Mockup rubrica](design/mockup/rubrica-connesso.png)

Un solo `galleria-gui` alla volta: avviato con un file (`galleria-gui profilo.ovpn`, o il doppio click) mentre la finestra è già aperta,
le consegna il file e termina; il canale è un socket `0600` in `$XDG_RUNTIME_DIR/galleria` (senza quella cartella l'app parte lo stesso, per conto suo).
Solo i `.ovpn` sono associati all'app (`packaging/linux/io.github.zpeppoz.galleria.desktop`, non installato dall'installer del prodotto: i pacchetti sono della fase 4; `lab/vm/gui-vm.sh` lo installa solo nelle VM del laboratorio).

**Portachiavi.** La finestra salva le chiavi dei profili (e utente e password, solo se si spunta «Ricorda») nel portachiavi di sistema (Secret Service: GNOME Keyring; KWallet non è stato verificato). Senza portachiavi ripiega su un file `0600` in `~/.config/galleria/secrets/` e lo dice con un avviso fisso; un portachiavi bloccato è un errore, mai un ripiego silenzioso. Il codice OTP e la passphrase della chiave non si salvano mai. La CLI legge dai due archivi, ma scrive solo nel file.

**Limiti delle prove sui desktop (B6).** Su Ubuntu 24.04/GNOME 46/Orca 46.1 i nomi Name, Server e Notes
sono stati letti nel giro dal file di debug; l'apertura del dialogo credenziali e il primo campo User name
sono invece rimasti muti in quel giro (apertura inizialmente inattiva su Wayland), pur avendo nome, ruolo
e fuoco nell'albero AT-SPI. Con Tab è stato letto Password (non il valore). La causa del silenzio non è
isolata dal fuoco del compositore: **la prova Q49 con apertura già attiva va ripetuta**; non è l'assenza
delle etichette. Nessun ascolto
umano è stato eseguito. KWallet, portali in sandbox, doppio click/trascinamento Nautilus, sblocco del
portachiavi in VM e l'intera passata della tray non sono verificati da questa consegna. Su Wayland il
compositore può negare il primo piano. **La fase 2 non è dichiarata chiusa**: tabella e prove in
[passata desktop B6](docs/08-b6-desktop-pass.md), [lista cuffie di 20 minuti](docs/b6-ascolto-20-minuti.md).

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
