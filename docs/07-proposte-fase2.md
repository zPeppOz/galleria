# 07 — Proposte per la fase 2 (GUI Linux)

Versione 0.1 — 2026-11-25 — stato: **proposta di Nello, da approvare da Peppe**. Nessun codice e nessun altro
documento è stato modificato: i diff del §4 si applicano dopo l'ok.

Base letta: ramo `galleria/t_17a7470c-galleria-correggi-gli-importanti-della-r` a `e4a2d71`.

**Come leggere le marcature.** *verificato (fonte)*: l'ho letto oggi nel file, nel codice o nella pagina
indicata. *dedotto*: è un ragionamento mio su quei fatti, non una prova. Dove serve una prova pratica lo scrivo.

**Scope (nota di Peppe del 25/11).** Galleria è uno strumento per Peppe e i colleghi, su macchine di lavoro
Linux e Windows, con le VPN aziendali e dei clienti (OpenVPN, WireGuard, più avanti OpenConnect). La validazione
dei concorrenti (A7) è tolta: qui non c'è e la fase 2 non la prevede. Le decisioni D5–D18, Q3, Q6, Q7, A6, A8,
A11 e l'IPC della fase 1 non sono riaperti.

## 1. Decisioni da approvare in blocco

| ID | Tema | Raccomandazione | Dettagli |
|---|---|---|---|
| D19 | Q5, script dei profili | **Mai eseguiti in v0.1**, nemmeno con conferma. L'import li toglie e lo dice | §2.1 |
| D20 | Q8, log | **Nessun file di log.** La GUI mostra il buffer del demone e offre «Esporta log…»; «Apri cartella log» sparisce | §2.2 |
| D21 | Q10, più utenti | **Nessun proprietario della sessione.** La GUI riconosce la sessione che non è partita dalla sua rubrica e chiede conferma prima di chiuderla o sostituirla | §2.3 |
| D22 | Q11, traduzioni | **Catalogo nel frontend, per codice**, con un test che fallisce se manca un codice del demone. Log e testi del server non si traducono | §2.4 |
| D23 | Q1, nomi delle fasi | «Fase 2» è solo la GUI; l'elenco del §4 dell'analisi diventa «Dopo la v0.1» | §4 |
| D24 | Q2, Q4, Q9 | Tre precisazioni di testo dell'analisi, senza effetti sul codice | §4 |
| D25 | Segreti: portachiavi e file | GUI e CLI **leggono da tutti e due** (Secret Service, poi file `0600`); la GUI scrive nel portachiavi e ripiega sul file, con avviso, se il portachiavi non c'è | §3.1 |
| D26 | Cosa si modifica dalle schede in v0.1 | WireGuard: campi veri. OpenVPN: valori in sola lettura più tre interruttori; il resto dall'editor raw. Fuori dalla GUI v0.1: kill switch, protezione DNS leak, connetti all'avvio | §3.2 |
| D27 | Primo avvio e demone assente | Schermata guidata con il comando da copiare; la GUI **non** chiede privilegi | §3.3 |
| D28 | Cambio di connessione | Tra profili propri **senza conferma** (un click, più avviso a scomparsa); conferma solo per la sessione estranea (D21) | §3.4 |
| D29 | Struttura del codice e prove della GUI | Crate `galleria-gui` nel workspace più `ui/`; solo i comandi Tauri parlano con il socket; prove su tre livelli | §3.5 |
| D30 | Design system (A8): base dei componenti | Token e stile nostri, ricavati dal mockup; comportamento accessibile da una libreria «headless» con licenza permissiva | §3.6 |
| D31 | Import: QR e associazione dei file | QR WireGuard **fuori** dalla fase 2; associazione solo per `.ovpn`, i `.conf` entrano da «Importa», trascinamento o testo incollato | §3.7 |
| D32 | «Ultimo uso» | In un file di stato a parte, non nel profilo | §3.8 |

## 2. Questioni della GUI (Q5, Q8, Q10, Q11)

### 2.1 Q5 — Script dei profili (D19)

Fatti:
- Il demone non ha nessun percorso per eseguire uno script di un profilo e rifiuta `up`, `down`, `route-up`,
  `plugin` e le altre direttive pericolose con `profile_rejected`. *verificato (spec fase 1 §6.3; `06-esito-fase1.md` §5)*
- L'import le toglie con un avviso di tipo `Dangerous`; per WireGuard toglie `PreUp`/`PostUp`/`PreDown`/`PostDown`.
  *verificato (`crates/galleria-profile/src/lib.rs`, `WarningKind`; spec §8.2)*
- Il mockup dice «gli script non vengono eseguiti senza conferma esplicita» e il §8 dell'analisi «bloccate o
  confermate esplicitamente»; M2 nel modello delle minacce ha la stessa formula. *verificato (`Rubrica.html` riga 172; `01-analisi.md` §8; `02-modello-minacce.md` M2)*
- I DNS spinti dal server li applica già il modulo comune del demone. *verificato (spec §7.2; E4 in `06-esito-fase1.md`)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Mai eseguiti** (raccomandata) | Coerente con il demone com'è e con M1/M2; nessun design nuovo; messaggio semplice | Un profilo che dipende davvero da uno script non funziona per intero |
| B. Conferma, poi eseguiti dalla GUI come utente normale | Copre gli script che non chiedono root | Design a parte (quando, con quale ambiente, cosa succede se la GUI è chiusa); gli script tipici dei `.ovpn` toccano DNS e rotte, quindi senza root falliscono comunque |
| C. Conferma, poi eseguiti dal demone come root | Compatibilità piena | Contraddice M1 («il demone non accetta comandi») e M2; un click sbagliato dà root a un file scaricato |

Raccomandazione: **A**. Motivo *dedotto*: negli `.ovpn` aziendali lo script più comune è l'aggiornamento dei
DNS (`update-resolv-conf`, `update-systemd-resolved`), cioè proprio il lavoro che Galleria fa da sé; il caso
che A lascia scoperto è piccolo. B si riapre solo se un profilo reale di Peppe o di un collega lo richiede, con
un design suo. Quanti profili veri abbiano script non l'ho misurato.

Cambi al mockup (finestra di import):
- riga dell'avviso: «Direttiva `up /etc/openvpn/update-dns` **rimossa**: Galleria non esegue gli script dei
  profili. I DNS del server li applica Galleria.» La seconda frase compare solo se lo script è uno dei due noti
  sopra; negli altri casi: «Se il profilo ne dipende, potrebbe non funzionare del tutto.»
- messaggio dopo l'import: «1 direttiva rimossa» al posto di «1 direttiva bloccata».
- scheda Log: via la riga finta «WARNING: direttiva 'up' ignorata (script disabilitati)». La direttiva è tolta
  all'import, OpenVPN non la vede mai e quindi non la scrive nel log. *dedotto dal flusso di import*

### 2.2 Q8 — Log (D20)

Fatti:
- Il demone tiene un buffer di 2000 righe, già filtrate dai segreti, che si svuota all'inizio di ogni sessione;
  lo servono `logs` ed `event.log`. Il log persistente è journald. Il demone non scrive fuori da `/run/galleria`.
  *verificato (spec §5.4, §8.1)*
- Lo stato `error` resta finché non arriva una nuova `connect` o `disconnect`, quindi il log di una connessione
  fallita resta leggibile fino al tentativo successivo. *verificato (spec §4.4); la conseguenza sul buffer è dedotta*
- Il journal di sistema lo leggono solo root e i membri di `systemd-journal`, `adm` o `wheel`: un membro del solo
  gruppo `galleria` non vede `journalctl -u galleriad`. *verificato (man di journalctl, freedesktop.org)*
- Il mockup ha «Ultima sessione», «Copia» e «Apri cartella log»; l'analisi §6 parla di «file di log ruotato».
  *verificato (`Rubrica.html` riga 324; `01-analisi.md` §6)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Nessun file: buffer del demone + «Esporta log…»** (raccomandata) | IPC e demone invariati; nessun file con dati di rete lasciato sul disco senza che l'utente lo sappia; basta per allegare un log a una segnalazione (A11) | Il log della sessione precedente si perde quando ne parte una nuova |
| B. File scritti dalla GUI in `$XDG_STATE_HOME/galleria/logs/`, ruotati | Storico delle sessioni; «Apri cartella» ha senso | Registra solo mentre la GUI è aperta; rotazione e pulizia da scrivere e provare; un secondo posto da controllare per M7 |
| C. File scritti dal demone in `/var/log/galleria` | Storico completo | Cambia il demone e i suoi permessi (`ProtectSystem=strict`), duplica journald, apre la questione di chi può leggerli |

Raccomandazione: **A**. «Esporta log…» apre la finestra di salvataggio e scrive quello che la scheda mostra;
essendo testo già filtrato dal demone, l'esportazione non aggiunge rischi a M7 *(dedotto; il test di M7 sulla
GUI resta comunque nei criteri, §5)*. B si aggiunge dopo se a qualcuno serve lo storico.

Cambi al mockup (scheda Log): «Apri cartella log» → «Esporta…»; titolo «Sessione corrente» quando lo stato non
è `disconnected`, altrimenti «Ultima sessione»; righe d'esempio in inglese e con i nomi veri (`glr-wg0`,
`glr-ovpn0`; su Linux niente «management 127.0.0.1», il canale è un socket Unix). Per chi amministra la
macchina, il README indica `journalctl -u galleriad`.

Diff dell'analisi collegato: §4, modifica 6.

### 2.3 Q10 — Più utenti sulla stessa macchina (D21)

Fatti:
- Ogni membro del gruppo `galleria` comanda l'unica connessione della macchina e ne vede stato e log, mai i
  segreti; l'uid di chi chiede finisce solo nel log di audit. *verificato (spec §5.1, §4.4)*
- `Status` porta `profile{id,name}` e `session`, non chi ha avviato la sessione. *verificato (spec §4.4)*
- Rotte e DNS della VPN valgono per tutta la macchina, non per l'utente che ha connesso. *dedotto (sono
  impostazioni di sistema applicate da `galleria-net`)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Sessione di sistema, senza proprietario; la GUI la rende visibile** (raccomandata) | IPC invariato; rispecchia la realtà (la rete cambia per tutti); chi si trova una VPN «tutto nel tunnel» lasciata accesa da un altro la può chiudere | Un membro del gruppo può chiudere la connessione di un collega |
| B. Proprietario nel demone: solo lui o root chiude, sostituisce e legge i log | Nessuno tocca la sessione altrui | Cambia l'IPC (errore nuovo, stato ridotto per gli altri); l'altro utente resta dentro una rete che non può spegnere; su una macchina di lavoro con un solo utente non dà nulla |
| C. Proprietario solo per i log | Riservatezza dei log | Cambia l'IPC per un dato che non contiene segreti |

Raccomandazione: **A**. Le macchine in gioco sono portatili e postazioni di lavoro personali *(dalla nota di
Peppe)*, quindi il caso a più utenti è raro; mettere un utente nel gruppo `galleria` equivale a dargli il
comando della VPN della macchina, e va scritto nel README.

Come la GUI la gestisce, senza toccare l'IPC *(dedotto dai campi di `Status`)*:
- **Sessione estranea**: `Status.profile.id` non è tra i profili della rubrica dell'utente. Succede con un altro
  utente e anche con un profilo che la rubrica non ha più.
- In quel caso la barra di stato e una riga fissa in cima alla lista dicono «Connessione attiva: *nome* —
  avviata fuori da questa rubrica», con «Disconnetti» e la scheda Log in sola lettura.
- Disconnettere o sostituire una sessione estranea chiede sempre conferma (D28).

Cambi al mockup: aggiungere la riga fissa e la variante della barra di stato; nient'altro.

### 2.4 Q11 — Traduzioni (D22)

Fatti:
- Il demone risponde con codici stabili (`data.code`, `Status.error.code`) e testo inglese; i codici sono due
  enum in `galleria-ipc` (`ErrorCode`, `SessionErrorCode`). *verificato (`crates/galleria-ipc/src/error.rs`; spec §4.6)*
- La CLI ha la sua tabella inglese/italiano in Rust e sceglie la lingua da `LC_ALL`/`LANG`.
  *verificato (`crates/galleria-cli/src/messages.rs`)*
- Gli avvisi dell'import sono già dati, non testo: riga, direttiva, tipo (`Dangerous`, `Unsupported`,
  `NoTlsClient`). Gli errori dell'import invece hanno il testo inglese dentro l'enum. *verificato (`crates/galleria-profile/src/lib.rs`)*
- `detail` è lo stato grezzo della management (`WAIT`, `AUTH`, `GET_CONFIG`…); `input_request.prompt` è il testo
  mandato dal server. *verificato (spec §4.4)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Catalogo nel frontend, chiave = codice; la CLI tiene la sua tabella** (raccomandata) | Un solo sistema di traduzione nella GUI (i testi dell'interfaccia sono comunque lì); nessuna modifica al codice della fase 1 | Una ventina di messaggi esistono due volte, nella CLI e nella GUI |
| B. Crate Rust condiviso, i comandi Tauri restituiscono testo già tradotto | Una sola tabella per gli errori | Due sistemi dentro la GUI (Rust per gli errori, frontend per tutto il resto); cambio di lingua a caldo più scomodo |
| C. File di catalogo unico letto da CLI e frontend | Nessun doppione | Rifattorizza la CLI per venti stringhe; formato comune da scegliere e mantenere |

Raccomandazione: **A**, con queste regole:
1. I comandi Tauri passano al frontend **codici e dati**, mai frasi. Per gli errori dell'import serve un codice
   per variante (`Syntax`, `MissingFile`, `OutsideFolder`, `Invalid`): si aggiunge nel livello dei comandi, senza
   toccare `galleria-profile`.
2. Un test Rust nel crate della GUI legge i cataloghi `en` e `it` e fallisce se manca un codice dei due enum di
   `galleria-ipc`: così un codice nuovo del demone non resta senza traduzione. Per un codice sconosciuto a
   runtime la GUI mostra il `message` inglese del demone.
3. Lingua: quella del sistema, con scelta manuale nelle impostazioni; inglese se la lingua non c'è.
4. Non si traducono: le righe di log, il `prompt` del server, i nomi delle direttive. `detail` si mappa su
   etichette tradotte («Autenticazione…», «Ricezione configurazione…») con il valore grezzo come ripiego.

Cambi al mockup: nessuno alla struttura; le righe di log d'esempio tornano in inglese (§2.2). Il mockup è solo
in italiano: la versione inglese nasce con il catalogo.

## 3. Altre scelte della GUI emerse confrontando mockup e demone

### 3.1 Segreti tra GUI e CLI (D25)

Fatti: la CLI salva i segreti solo in `FileSecretStore` (file `0600`); il portachiavi è previsto come seconda
implementazione dello stesso trait, con la GUI. *verificato (spec §8.3; `crates/galleria-profile/src/secrets.rs`)*

Problema *dedotto*: se la GUI scrive solo nel portachiavi e la CLI legge solo dal file, un profilo importato da
una non si connette dall'altra, pur stando nella stessa rubrica. E l'import mette nell'archivio anche le chiavi
private, quindi senza Secret Service (sessione SSH, desktop minimale) la GUI non potrebbe nemmeno importare.

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Lettura da entrambi (portachiavi, poi file); la GUI scrive nel portachiavi, e nel file se il portachiavi non risponde, con avviso visibile** (raccomandata) | Rubrica unica davvero; la GUI funziona anche senza portachiavi | Piccola modifica alla CLI (lettura dal portachiavi quando c'è) |
| B. GUI solo portachiavi, CLI solo file | Nessuna modifica alla CLI | Due rubriche di fatto; GUI inutilizzabile senza portachiavi |
| C. Tutti e due solo file | Semplicissimo | Rinuncia al portachiavi deciso in Q6 e nell'analisi §5 |

Raccomandazione: **A**. Quale crate usare per Secret Service e come si comporta con GNOME Keyring e KWallet
resta **[S]**: è il primo task del blocco B5 (§5). La spunta «Ricorda» riguarda utente e password, mai l'OTP.

### 3.2 Cosa si modifica dalle schede in v0.1 (D26)

Fatti:
- Il profilo TOML ha tre impostazioni: `ipv6_block`, `auto_reconnect`, `ignore_pushed_routes`. Per WireGuard ha
  campi strutturati (indirizzi, DNS, domini, MTU, peer); per OpenVPN ha il testo della configurazione.
  *verificato (spec §8.2; `crates/galleria-profile/src/model.rs`)*
- Con OpenVPN, DNS e rotte arrivano spesso dal server e si conoscono solo a connessione fatta, in `Status.net`.
  *verificato (spec §4.4, M3)*
- Kill switch, protezione DNS leak e avvio con il sistema sono fuori dalla v0.1. *verificato (`ROADMAP.md`, «Dopo la v0.1»)*
- Il mockup mostra interruttori per kill switch, «Connetti all'avvio» e «Protezione DNS leak», e campi
  modificabili per server, trasporto, rotte, DNS e MTU anche per OpenVPN. *verificato (`Rubrica.html` righe 258–293)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. WireGuard a campi; OpenVPN in sola lettura più interruttori; il resto dall'editor raw** (raccomandata) | La GUI mostra solo cose che funzionano; nessuna riscrittura strutturata dei `.ovpn` | Per cambiare server o porta di un profilo OpenVPN si passa dall'editor raw |
| B. Campi modificabili anche per OpenVPN (la GUI riscrive `remote`, `proto`, `route`, `dhcp-option`) | Più comodo | Riscrivere un `.ovpn` con più `remote` o blocchi `<connection>` senza romperlo è un lavoro a sé, con molti casi limite |
| C. Solo editor raw per tutti | Minimo | Butta via il dettaglio a schede di D6 |

Raccomandazione: **A**. In concreto, rispetto al mockup:

| Scheda | Cambia così |
|---|---|
| Generale | Restano nome, gruppo, riconnessione automatica. Server e trasporto: modificabili per WireGuard (endpoint), in sola lettura per OpenVPN. **Via** kill switch e «Connetti all'avvio» |
| Autenticazione | «Metodo» diventa un'etichetta ricavata dal profilo (`auth-user-pass`, `static-challenge`), non un selettore. Scadenza dei certificati: resta, è utile sulle VPN dei clienti, ma è un task a parte e il primo a saltare se serve tempo. «Dimentica» cancella la password dal portachiavi |
| Rete | WireGuard: indirizzi, DNS, domini, MTU, reti instradate come campi. OpenVPN: DNS e rotte **effettivi** in sola lettura quando è connesso (è ciò che M3 chiede di mostrare), più gli interruttori «Ignora le rotte del server» e «Blocco IPv6». **Via** «Protezione DNS leak». «Tutto il traffico nel tunnel» diventa un'indicazione (`net.full_tunnel`), non un interruttore |
| Configurazione | Niente segnaposto `<portachiavi:…>` dentro il testo: i blocchi segreti non compaiono e una riga sotto l'editor dice quali sono nel portachiavi. La validazione è la classificazione delle direttive della fase 1, con gli avvisi riga per riga. L'esempio non deve contenere `dev`, `verb`, `persist-key`, `persist-tun`, che l'import toglie *(verificato: classe «del demone», spec §6.3)* |
| Stato (testata) | Gli stati veri sono sette, non quattro: aggiungere «Servono credenziali», «Riconnessione (tentativo n)», «Disconnessione…»; «Annulla» durante la connessione è una `disconnect` |

### 3.3 Primo avvio e demone assente (D27)

Fatti: il socket è `0660 root:galleria`; chi non è nel gruppo viene rifiutato dal kernel alla `connect()`.
L'installazione di prova crea il gruppo ma non ci mette nessun utente. `hello` può rispondere
`incompatible_protocol`, e un backend può essere `available: false` con il motivo. *verificato (spec §5.1,
§4.2; `packaging/linux/install.sh`)*. Nel mockup la barra dice sempre «Demone galleriad · connesso». *verificato (`Rubrica.html` riga 160)*

Raccomandazione *(dedotta)*: quattro stati espliciti, ognuno con la sua schermata o il suo avviso:
1. demone non in esecuzione → «Avvia il servizio» con il comando `systemctl` da copiare;
2. utente fuori dal gruppo → comando `usermod -aG galleria <utente>` da copiare e l'avviso che serve rientrare
   nella sessione;
3. versione incompatibile → «Aggiorna Galleria e riavvia il demone»;
4. OpenVPN assente o più vecchio della 2.6 → avviso sui soli profili OpenVPN; WireGuard resta utilizzabile.

La rubrica resta consultabile e modificabile anche senza demone (i profili sono file dell'utente). L'alternativa,
far chiedere alla GUI i privilegi con polkit per sistemare gruppo e servizio, la scarto: aggiunge un percorso
privilegiato alla GUI, che per architettura non ne ha (analisi §5). Se il pacchetto di rilascio debba mettere
nel gruppo chi installa è una questione della fase 4.

### 3.4 Cambio di connessione (D28)

Fatti: con una sessione attiva `connect` risponde `already_active` salvo `replace: true`; il demone aspetta lo
smontaggio e avvia la nuova. La spec lascia alla GUI se chiedere conferma. Il mockup sostituisce senza chiedere
e mostra «una connessione alla volta». *verificato (spec §4.5; `Rubrica.html` riga 370)*

Raccomandazione: tra profili della propria rubrica **nessuna conferma**: un click, `replace: true`, avviso a
scomparsa «Disconnesso da X». Chi lavora su più clienti cambia VPN più volte al giorno e una finestra di
conferma ogni volta è solo attrito *(dedotto dall'uso descritto da Peppe)*. Conferma obbligatoria solo se la
sessione attiva è estranea (D21).

Credenziali: il mockup apre la finestra prima di connettere, con utente, password e OTP fissi. Il flusso vero è
guidato dal demone: la GUI manda quello che ha nel portachiavi, e se arriva `needs_input` mostra **solo** i
campi chiesti (`credentials`, `otp`, `key_passphrase`), con il `prompt` del server e il conto alla rovescia dei
120 secondi. Alla riconnessione l'OTP viene richiesto di nuovo. *verificato (spec §4.4, §5.2)*

### 3.5 Struttura del codice e prove (D29)

Vincoli già decisi: la webview non parla con il socket, passa dai comandi Tauri (D12); il controllo delle
versioni lo fa la GUI, una richiesta sola, disattivabile (D16). *verificato (`01-analisi.md` D12, D16)*

Proposta *(dedotta)*:
- `crates/galleria-gui/` (il lato Rust di Tauri) nel workspace, dipende da `galleria-ipc` con la feature
  `client` e da `galleria-profile`; `ui/` con React e TypeScript. `scripts/check-boundaries.sh` guadagna una
  regola: `galleriad` non dipende da `galleria-gui`.
- Un solo task Rust tiene la connessione al demone, si ricollega se il demone riparte e inoltra `event.state`,
  `event.stats`, `event.log` al frontend come eventi Tauri.
- I segreti non entrano mai nella webview: l'import, la lettura dal portachiavi e la composizione dei parametri
  di `connect` avvengono in Rust. La webview manda solo ciò che l'utente digita nella finestra delle
  credenziali. È l'estensione di M1 e M7 alla GUI.
- Permessi della webview al minimo: nessun accesso generico a file, shell o rete dal frontend; anche la
  richiesta delle versioni parte da Rust.

Prove, su tre livelli:
1. **Frontend**: test dei componenti con i comandi Tauri simulati.
2. **Comandi Rust**: contro un demone vero con il `FakeBackend` della fase 1 su socket temporaneo, senza privilegi.
3. **Fumo end-to-end** nel laboratorio: l'app vera pilotata via WebDriver. Su Linux Tauri usa `WebKitWebDriver`
   tramite `tauri-driver`, e senza schermo si usa Xvfb. *verificato (documentazione Tauri «WebDriver» e
   «Manual setup»; WebdriverIO «Platform Support»)*. Che giri nel container del laboratorio e sul runner GitHub è
   **[S]**: lo verifica il primo blocco; se non regge, il fumo resta manuale con una lista di controllo scritta.

### 3.6 Design system (D30)

Deciso: nuovo, nel repo, con la licenza del progetto, componenti ricavati dal mockup (A8). *verificato (`01-analisi.md` A8; `ROADMAP.md`)*

| Alternativa | Pro | Contro |
|---|---|---|
| **A. Token e CSS nostri + primitive «headless» per finestre, menu, schede, selettori** (raccomandata) | Tastiera, focus e ruoli ARIA corretti senza riscriverli; aspetto tutto nostro | Una dipendenza in più, licenza da controllare |
| B. Tutto a mano | Zero dipendenze | Finestre modali e menu accessibili sono la parte dove si sbaglia di più |
| C. Libreria di componenti già stilizzata | Veloce | Non è un design system «di Galleria»; va contro A8 |

Raccomandazione: **A**. Quale libreria: da scegliere nella spec, con licenza compatibile con GPL-3.0-or-later
verificata prima di adottarla. I token (colori, spazi, caratteri, tema chiaro e scuro) si estraggono dal CSS del
mockup, che li ha già come variabili. *verificato (`Rubrica.html`, blocco `<style>`: `var(--ink)`, `var(--muted)`, `var(--line)`, `var(--mono)`)*

### 3.7 Import: QR e associazione dei file (D31)

Fatti:
- Il QR WireGuard nell'analisi è «[S: QR da valutare]»; il mockup lo offre già. *verificato (`01-analisi.md` §4; `Rubrica.html` riga 168)*
- Tauri scrive le associazioni di `bundle.fileAssociations` come `MimeType=` nel file `.desktop` su Linux.
  *verificato (riferimento di configurazione di Tauri 2, `FileAssociation`)*
- Su Linux l'apertura di un file con l'app già aperta lancia un secondo processo con il percorso come
  argomento; serve il plugin single-instance per passarlo alla finestra esistente. *letto su una guida non
  ufficiale (techxcelerate), da confermare sulla documentazione di Tauri nella spec*
- Sulla tray Linux Tauri non emette i click sull'icona: funziona solo il menu. *verificato (documentazione
  Tauri «System Tray»)*. Un backend nuovo che li supporta è annunciato per la prossima versione di `tray-icon`
  *(verificato: issue tauri-apps/tray-icon #104, commento del manutentore dell'8 settembre 2026)*: non ci contiamo.

Raccomandazione:
- **QR fuori dalla fase 2.** Su un PC serve una webcam o un'immagine da decodificare, e i colleghi ricevono i
  profili come file *(dedotto)*. Dal mockup va tolto «scansiona un QR WireGuard».
- **Associazione solo per `.ovpn`.** `.conf` è l'estensione di mezza `/etc`: registrarsi come programma per i
  `.conf` farebbe comparire Galleria su file che non c'entrano *(dedotto)*. I `.conf` WireGuard entrano da
  «Importa», trascinamento o testo incollato.
- L'import passa sempre dall'anteprima con gli avvisi, anche da doppio click: mai un salvataggio silenzioso (M2).

### 3.8 «Ultimo uso» (D32)

Fatto: `ultimo_uso` è rinviato alla fase 2 perché nel profilo obbligherebbe a riscriverlo a ogni connessione.
*verificato (spec §8.2)*. Il mockup lo mostra e lo usa per ordinare. *verificato (`Rubrica.html` riga 346)*

Raccomandazione: un file `state.toml` in `$XDG_STATE_HOME/galleria/`, scritto dalla GUI, con l'ultimo uso per
profilo e lo stato della finestra. Se si perde non succede nulla. L'alternativa è togliere il dato dal mockup;
la scarto perché l'ordinamento «usati di recente» serve a chi ha molte VPN di clienti *(dedotto)*.

## 4. Testo corretto di `docs/01-analisi.md` (Q1, Q2, Q4, Q9, più Q5 e Q8)

Da applicare dopo l'ok, in un solo commit. Le righe di tabella dell'analisi sono lunghissime: per quelle indico
la sostituzione di parole, come ha fatto la spec per Q3; per le righe normali do il diff. Tutti i testi «da»
sono *verificati* sul file a `e4a2d71`.

**Intestazione**

```diff
-Versione 0.6 — 2026-11-10 — stato: D10–D16 decise (D15 approvata da Peppe; le altre ratificate da Aitano su delega di Peppe dopo gli spike), nuove D17–D18
+Versione 0.7 — 2026-11-25 — stato: D10–D18 decise; precisazioni di testo da Q1, Q2, Q4, Q5, Q8 e Q9 della spec di fase 1 (`07-proposte-fase2.md`)
```

**1. Q1 — «CLI» e «fase 2» con due significati (§4)**

```diff
-### Fase 2
+### Dopo la v0.1
 - Kill switch e protezione DNS leak (per profilo).
 - Split tunneling per rotte/subnet (per app è molto più complesso, fuori).
 - Avvio con il sistema, connessione automatica su reti non fidate.
-- CLI `galleria` che parla con lo stesso demone (utile su SSH/server).
+- CLI `galleria` estesa (completamento, output per script, gestione dei gruppi). La CLI minima che parla con
+  lo stesso demone (import, connect, disconnect, status, logs) è già nella fase 1 del §10.
 - Backup/ripristino della rubrica.
 
-### Fase 3 (solo con trazione)
+### Solo con trazione
```

Nello stesso senso, dove «fase 2» vuol dire «dopo la v0.1» e non «GUI»:
- riga D13: `(fase 2, connessione automatica)` → `(dopo la v0.1, connessione automatica)`
- riga D16: `repository APT/DNF in fase 2` → `repository APT/DNF dopo la v0.1`
- §8 Aggiornamenti: `APT/DNF in fase 2.` → `APT/DNF dopo la v0.1.`
- §8 IPv6: `del kill switch completo, che resta in fase 2.` → `del kill switch completo, che arriva dopo la v0.1.`
- riga R1: `kill switch solo in fase 2` → `kill switch solo dopo la v0.1`
- `02-modello-minacce.md`, riga M4: `kill switch in fase 2` → `kill switch dopo la v0.1`

Da qui in poi «fase N» indica solo le fasi del §10 e della roadmap.

**2. Q2 — prove Windows attribuite alla fase 1**

- riga D15: `con OpenVPN **[S]**, primo test della fase 1.` → `con OpenVPN **[S]**, primo test della fase 3 (porting Windows).`
- riga D17: `è **[S]**: è il primo criterio della fase 1, e se non regge si ripiega sul servizio interattivo`
  diventa:

  > è **[S]** su Windows: è il primo criterio della fase 3, e se non regge si ripiega sul servizio interattivo. La parte comune, la lettura del `PUSH_REPLY`, è provata su Linux in fase 1: dal log della management non dà garanzie sui tempi, quindi il demone lo legge da `stdout` di OpenVPN (`06-esito-fase1.md`, S9)

La seconda frase riporta un fatto *verificato (`06-esito-fase1.md` §4, S9)* che su Windows andrà riprovato.

**3. Q4 — rimando sbagliato in D13**

- riga D13: `Il demone non legge questi file (M2)` → `Il demone non legge questi file (M1: non accetta percorsi; i file referenziati da un profilo si copiano all'import, M2)`

**4. Q9 — management «con password» (§5)**

```diff
-  **management interface** (socket locale con password). Su Linux si dipende dal pacchetto della distro;
+  **management interface**: su Linux un socket Unix in una cartella `0700` di root con `management-client-user`,
+  senza password; su Windows una porta TCP locale con password casuale per sessione (M5). Su Linux si dipende dal pacchetto della distro;
```

**5. Q5 — script dei profili (§8, solo se D19 è approvata)**

```diff
-  autorizzati; nessun segreto nei log; l'import non esegue mai direttive che lanciano script
-  (`up`, `down`, `script-security` in OpenVPN vanno bloccate o confermate esplicitamente).
+  autorizzati; nessun segreto nei log; Galleria non esegue mai gli script dei profili: l'import toglie con un
+  avviso le direttive che li lanciano (`up`, `down`, `route-up`, `plugin` e simili in OpenVPN, `PostUp` e
+  simili in WireGuard) e il demone rifiuta un profilo che le contiene (D19).
```

- `02-modello-minacce.md`, riga M2: `direttive pericolose bloccate o confermate esplicitamente (già in §8 dell'analisi)` → `direttive pericolose sempre rimosse con avviso, mai eseguite nemmeno con conferma (§8 dell'analisi, D19); il demone rifiuta i profili che le contengono`

**6. Q8 — file di log (§6, solo se D20 è approvata)**

```diff
-- `LogEntry`: profile_id, timestamp, livello, messaggio (buffer circolare + file di log ruotato).
+- `LogEntry`: seq, timestamp, livello, sorgente, messaggio. Buffer circolare in memoria nel demone, svuotato a
+  ogni nuova sessione; su Linux il log persistente è journald. Nessun file di log proprio: la GUI esporta su richiesta (D20).
```

**7. Tabelle delle decisioni e dei punti aperti**

- §2: aggiungere le righe D19–D32 approvate, con il testo della colonna «Raccomandazione» del §1 di questo
  documento e fonte «Nello, 2026-11-25 — approvata da Peppe il …».
- §12, riga A7: stato `PARZIALE: …` → `CHIUSA senza validazione (Peppe, 2026-11-25): Galleria è uno strumento per Peppe e i colleghi, non un prodotto da validare sul mercato`.

Non propongo modifiche a §3 (differenziazione), §10 (lancio pubblico) e R6, che parlano ancora di mercato: sono
fuori da questa card. Lo segnalo perché dopo la nota del 25/11 quel testo non descrive più l'obiettivo.

**8. `ROADMAP.md`**

```diff
-- [ ] Completa la validazione dei concorrenti (A7)
-  - [ ] difetti di Eddie e prova pratica di Pritunl scritti in `docs/00-mercato-e-concorrenti.md`
-  - [ ] elenco dei punti di UX in cui la GUI deve essere nettamente migliore (analisi §3)
+- ~~Completa la validazione dei concorrenti (A7)~~ — tolto per decisione di Peppe del 2026-11-25: Galleria è uno
+  strumento per Peppe e i colleghi, non un prodotto da validare sul mercato
```

Nella fase 1, lo step «Risposte alle questioni aperte della spec di fase 1» si spunta quando le D19–D24 sono
approvate e i diff sopra applicati. Gli step della fase 2 si riallineano ai blocchi del §5: «QR WireGuard solo
se la verifica lo conferma» diventa «QR WireGuard fuori dalla fase 2 (D31)» e «associazione delle estensioni»
diventa «associazione di `.ovpn` (D31)».

## 5. Fase 2: dove si spezza

Sei blocchi, 38 task in tutto. Ogni blocco è un piano a sé (un task = un commit, TDD, review indipendente) e
finisce con qualcosa che si avvia e si prova: se ci si ferma tra un blocco e l'altro non restano mezzi lavori.
Il numero dei task è una stima *dedotta* dal perimetro; quello definitivo esce dalla spec.

| Blocco | Task | Contenuto | Dipende da | Dove ci si può fermare |
|---|---|---|---|---|
| B1 · Fondamenta | 6 | Crate `galleria-gui` e `ui/` nel workspace; job CI della GUI; connessione al demone con ricollegamento; eventi del demone inoltrati al frontend; permessi minimi della webview; quattro stati del primo avvio (D27); prova che il fumo WebDriver gira nel laboratorio (**[S]**) | fase 1 in `main` | Finestra che mostra lo stato vero del demone, sola lettura |
| B2 · Design system e guscio | 6 | Token dal mockup; componenti di base (pulsante, campo, interruttore, schede, finestra modale, avviso); disposizione lista + dettaglio; tema chiaro e scuro; cataloghi `en` e `it` con il test dei codici (D22); navigazione da tastiera | B1 | Guscio navigabile con dati finti, nelle due lingue |
| B3 · Rubrica e profili | 7 | Lista con gruppi, ricerca, preferiti, ordinamento; gruppi in `galleria.toml`; schede Generale, Autenticazione e Rete secondo D26; editor raw con validazione; duplica, esporta, elimina; `state.toml` (D32) | B2 | Rubrica completa sui profili già importati dalla CLI, senza connettere |
| B4 · Import | 5 | Scelta del file; trascinamento; testo incollato; anteprima con avvisi e testo di D19; associazione di `.ovpn` e istanza unica (D31) | B3 | Si importa dalla GUI e si connette ancora dalla CLI |
| B5 · Connessione | 8 | Archivio dei segreti con Secret Service e ripiego sul file, lettura da entrambi anche nella CLI (D25, **[S]**); connetti, disconnetti, sostituisci (D28); sette stati e `detail`; statistiche dal vivo; finestra delle credenziali guidata da `needs_input`; errori tradotti; scheda Log con «Esporta…» (D20); sessione estranea (D21) | B3 (B4 per il giro completo) | **Uso quotidiano possibile**: da qui Peppe e i colleghi possono usarla al posto della CLI |
| B6 · Tray, versione, chiusura | 6 | Menu della tray con stato, preferiti, connetti e disconnetti; chiusura nella tray e uscita esplicita; avviso di nuova versione (D16); fumo end-to-end completo; prova manuale su Ubuntu e Fedora desktop; `docs/08-esito-fase2.md` | B5 | Fase 2 chiusa |

B3 e B4 possono scambiarsi di posto solo in parte: l'anteprima dell'import riusa i componenti delle schede. B5 è
il blocco più grosso e quello con l'incognita vera (Secret Service): se il portachiavi dà problemi, il ripiego
sul file fa già parte di D25 e il blocco non si ferma.

### Criteri di uscita della fase 2

Come E1–E7: ognuno con una prova automatica, salvo dove è scritto «manuale».

| # | Criterio | Come si verifica |
|---|---|---|
| G1 | La GUI parte da utente normale, si collega al demone e ne mostra lo stato; se il demone riparte si ricollega da sola | test dei comandi contro il demone con `FakeBackend`; fumo: riavvio di `galleriad` a GUI aperta |
| G2 | I quattro stati del primo avvio (D27) hanno ognuno il suo messaggio: demone fermo, utente fuori dal gruppo, protocollo incompatibile, OpenVPN assente | test dei comandi con socket assente, socket `0600`, `hello` fuori intervallo, backend `available: false` |
| G3 | Import da file, trascinamento e testo di un `.ovpn` e di un `.conf` del laboratorio: anteprima con gli avvisi, salvataggio solo dopo «Importa»; un profilo con `up` mostra il testo di D19 e lo script non viene mai eseguito | fumo nel laboratorio con il profilo malevolo della fase 1: nessun file `PWNED-*`; test dei componenti sull'anteprima |
| G4 | Connetti, disconnetti e sostituisci dalla GUI con tutti e due i backend sui server del laboratorio; mai due backend vivi (D5); IP, byte e ultimo handshake si aggiornano | fumo nel laboratorio sui tre client; controllo dei processi come in E2 |
| G5 | Credenziali: password, OTP e passphrase chiesti solo quando il demone li chiede; «Ricorda» salva nel portachiavi e la connessione successiva non chiede la password; l'OTP non si salva mai | fumo con gli utenti `alice` e `bob` del laboratorio; test dell'archivio con un Secret Service di prova |
| G6 | Un profilo importato dalla CLI si connette dalla GUI e viceversa (D25) | test di `galleria-profile` sui due archivi; fumo incrociato |
| G7 | Nessun segreto noto nella scheda Log, nel file esportato, nei log della GUI e negli argomenti dei processi (M7); nessun segreto salvato passa dalla webview | ricerca dei segreti del laboratorio, come T13 e T16; test che i comandi Tauri di lettura non restituiscono mai campi segreti |
| G8 | Ogni codice d'errore del demone ha testo inglese e italiano; cambiando lingua l'interfaccia cambia senza riavvio | test Rust sui cataloghi (D22); test dei componenti |
| G9 | Sessione estranea: con una connessione avviata da un altro utente del gruppo la GUI la mostra e chiede conferma prima di chiuderla (D21) | test dei comandi con un profilo che non è in rubrica; fumo con due utenti |
| G10 | Tray: stato, preferiti, connetti e disconnetti dal solo menu; chiudere la finestra non chiude la connessione | **manuale** su Ubuntu 24.04 e 26.04 (GNOME con AppIndicator) e Fedora, lista di controllo nell'esito |
| G11 | Avviso di nuova versione: una sola richiesta, senza identificativi, niente richiesta se disattivato (D16, A11) | test con un server finto che conta le richieste e ne controlla le intestazioni |
| G12 | Tutta la rubrica si usa da tastiera; finestre e schede hanno i ruoli corretti | test dei componenti con un controllo automatico di accessibilità; passata manuale con lo screen reader, annotata |
| G13 | Avvio a freddo della GUI sotto 1 s sulla macchina di Peppe (obiettivo [S] dell'analisi §8) | **manuale**, tre misure annotate nell'esito; non blocca la chiusura della fase, ma il numero va scritto |

Restano fuori dalla fase 2: Windows (fase 3), pacchetti di rilascio e firma (fase 4), kill switch, protezione DNS
leak, avvio con il sistema, QR, esecuzione di script, OpenConnect.

## 6. Cosa non ho verificato

- Nessuna prova pratica: né Tauri, né WebDriver, né Secret Service sono stati eseguiti. I punti segnati **[S]**
  (fumo WebDriver nel laboratorio, crate e comportamento del portachiavi) sono i primi task dei blocchi B1 e B5.
- Il comportamento di Tauri con l'istanza unica e l'apertura dei file su Linux l'ho letto su una guida non
  ufficiale: va confermato sulla documentazione di Tauri quando si scrive la spec.
- Non ho contato quanti profili reali di Peppe e dei colleghi contengono script (§2.1): la raccomandazione su Q5
  regge su un ragionamento, non su quel conteggio.
- I numeri dei task per blocco sono stime.

## Fonti esterne lette il 2026-11-25

- Tauri 2, tray: https://v2.tauri.app/learn/system-tray ; API: https://v2.tauri.app/reference/javascript/api/namespacetray
- `tray-icon`, click su Linux: https://github.com/tauri-apps/tray-icon/issues/104
- Tauri 2, configurazione (`fileAssociations`, `desktopTemplate`): https://v2.tauri.app/reference/config/
- Tauri 2, WebDriver: https://v2.tauri.app/develop/tests/webdriver ; https://tauri.app/develop/tests/webdriver/manual-setup
- WebdriverIO, Tauri su Linux: https://webdriver.io/docs/desktop-testing/tauri/platform-support
- Associazioni e istanza unica (guida non ufficiale): https://techxcelerate.ntxm.org/docs/tauri/configuration/bundle-configuration/file-associations/
- journalctl, chi legge il journal di sistema: https://www.freedesktop.org/software/systemd/man/latest/journalctl.html
