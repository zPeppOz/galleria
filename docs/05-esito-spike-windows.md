# 05 — Esito degli spike su Windows (spike 1 e parte Windows dello spike 2)

Data: 2026-11-10 — autore: Nello (CTO) — branch locale `spike/windows` (da `main`, non pubblicato).
Definizione degli spike: `03-sessione-nello.md` §3. Codice, laboratorio e prove: `spikes/windows-lab/`,
`spikes/openvpn-mgmt/` (parte Windows in `lab/win/` ed `evidence/windows/`), `spikes/wireguard-nt/`.
Le decisioni D10–D16 restano di Peppe: dove le prove le smentiscono lo scrivo in §5, non le ho cambiate.

## 1. In sintesi

- **Spike 2 su Windows: la management interface regge.** Con OpenVPN 2.7.9 ufficiale il demone, lanciato come SYSTEM,
  ha pilotato `openvpn.exe` senza scrivere su disco né la configurazione né la password della management. Superati
  2.1, 2.3, 2.4 (anche con DCO attivo, che su Linux era rimasto aperto), 2.5, 2.6, 2.9. **2.7 e 2.8 sono superati
  solo in parte**, e sono i due punti che toccano le decisioni (§5).
- **Spike 1: WireGuardNT da Rust funziona.** Il servizio `galleriad` (LocalSystem) carica `wireguard.dll`, alza il
  tunnel, legge handshake e byte ogni secondo, smonta tutto senza residui, e la named pipe rifiuta un utente senza
  privilegi. Superati 1.1, 1.2, 1.3, 1.8. Per 1.5 l'errore immediato su Windows si ottiene con un blocco WFP; le
  rotte verso il tunnel non hanno funzionato.
- **1.6 e 1.7 superati (secondo giro, stessa data).** L'MSI di Tauri con il frammento WiX installa il servizio
  avviato e automatico e la disinstallazione toglie servizio e adattatore. L'updater porta GUI e servizio da 0.0.1 a
  0.0.2 e rifiuta sia una firma fatta con un'altra chiave sia un file alterato. Limiti in §4: provato solo da
  amministratore senza UAC, MSI senza firma Authenticode, validazione ICE di WiX saltata nella Sandbox. Il job
  GitHub Actions di 1.4 è solo scritto.
- Due incidenti da conoscere (§7): l'interoperabilità di WSL con gli `.exe` di Windows si è rotta a metà lavoro per
  colpa del laboratorio (ripristinata), e i container del laboratorio li ha poi tolti Aitano perché nella mia
  sessione non potevo fermarli.

## 2. Ambiente

- Host: Windows 11 Pro for Workstations 26200. Sull'host solo letture, file in `C:\Users\hackp\galleria-sbx` e
  avvio/chiusura della Sandbox. Nessuna installazione, driver, servizio, rotta o regola di firewall sull'host.
- Ospite: Windows Sandbox, Windows 11 Enterprise 26100, utente `WDAGUtilityAccount` (amministratore, senza UAC),
  20 CPU logiche, 8 GB. Rete: IPv4 in NAT, **IPv6 solo link-local**. Per contare i pacchetti IPv6 la scheda
  fisica riceve un indirizzo globale finto (`2001:db8:5::10/64`), un gateway finto e un vicino statico per il
  gateway: così i pacchetti IPv6 globali escono davvero dalla scheda (nessuno risponde) e `pktmon` li vede.
- OpenVPN 2.7.9 dall'MSI ufficiale (`/qn`), DCO 2.8.14 attivo di default. WireGuardNT 0.10.1: `wireguard.dll`
  dallo zip ufficiale (sha256 dello zip `772c0b14…`), firma Authenticode valida di WireGuard LLC.
- Server di prova: il laboratorio Docker dello spike 2 in WSL (`gl-server`, `gl-server-dns`, `gl-target`) più un
  server WireGuard (`gl-wg`). Il sito dual-stack di prova è `198.51.100.1` (raggiungibile solo dal tunnel) e
  `2001:db8:99::1`.
- Il demone dello spike 2 gira come **SYSTEM in sessione 0** (attività pianificata), come farebbe il servizio; il
  servizio dello spike 1 è un vero servizio Windows registrato con `sc create`.
- Limite delle misure IPv6: nella Sandbox l'IPv6 non arriva da nessuna parte. Le prove misurano **quanti pacchetti
  lasciano la scheda e quanto ci mette un'applicazione ad accorgersi che l'IPv6 non va**; non c'è un sito che
  risponde in IPv6 come nel laboratorio Linux.

## 3. Raggiungibilità del server di prova (punto 1 del task)

Tra Sandbox e WSL non c'è una via di rete: provato TCP, UDP e ping nei due sensi (`spikes/windows-lab/evidence/relay-latency.txt`).
Aprirne una richiede `netsh portproxy`, regole di firewall o un'altra modalità di rete di WSL, cioè toccare l'host.

Soluzione usata, senza toccare l'host: **`spikes/windows-lab/filerelay`**. Nell'ospite il client VPN parla con
`127.0.0.1:<porta>`; il relay scrive i datagrammi UDP in un file della cartella mappata scrivibile; in WSL un secondo
relay li consegna al server vero e riporta indietro le risposte. Misurato: 100 datagrammi, 0 persi, 18 ms di media
andata e ritorno; ping dentro il tunnel 15–20 ms. Va bene per prove funzionali, non per misure di prestazioni.
Conseguenza sui profili di prova: `remote 127.0.0.1` e `redirect-gateway local` (vedi §7).

## 4. Criteri

### Spike 2 — management interface OpenVPN, Windows (OpenVPN 2.7.9)

Prove in `spikes/openvpn-mgmt/evidence/windows/`: `<criterio>.txt` (comandi e output), `<criterio>-*.log` (eventi
del demone e log di OpenVPN), `2.8-*.pcapng`.

| # | Esito | Prova | Note |
|---|---|---|---|
| 2.1 | **superato** | `2.1.txt`, `2.1-sanitized.log`, `2.1-control.log` | Profilo ripulito: 10 direttive tolte, connesso, nessun file `PWNED-*`, nessuna porta 7505. Controllo senza pulizia: OpenVPN scrive `PWNED-status.txt` come SYSTEM ed esegue i `cmd.exe` del profilo (`ipchange`, `up`); i miei comandi di prova sono tornati con errore 1, quindi i file di `up`/`down` non compaiono e con `up` fallito OpenVPN esce. L'esecuzione è provata dal log, non dal file |
| 2.3 | **superato** | `2.3.txt`, `2.3*.log` | Porta TCP casuale su 127.0.0.1, password di 256 bit per sessione data a OpenVPN su **stdin** (`management 127.0.0.1 <porta> stdin`): funziona con OpenVPN figlio di un processo SYSTEM in sessione 0. Riga di comando: solo `--config \\.\pipe\…`. Ricerca della password su disco: 0 file. Senza password o con password sbagliata OpenVPN risponde solo `ENTER PASSWORD:`, anche a un utente senza privilegi. Secondo client mentre il demone è collegato: il primo resta senza risposta, i successivi ricevono connessione rifiutata (10061), anche con la password giusta. `--management-client` funziona, ma vedi §6 |
| 2.4 | **superato, anche con DCO** | `2.4.txt`, `2.4.log`, `2.4-nodco.log` | Stati fino a `CONNECTED,SUCCESS,10.8.0.2`; un evento `bytecount` al secondo che cresce durante il ping, sia con `ovpn-dco` sia con `--disable-dco` (TAP-Windows6). Chiude su Windows il punto rimasto aperto su Linux |
| 2.5 | **superato** | `2.5.txt`, `2.5-*.log` | OTP con `static-challenge` (`SCRV1`, il server registra `user=bob ok (password + totp …)`) e passphrase della chiave privata, tutto dalla management interface |
| 2.6 | **superato con la prima via**, adattata | `2.6.txt`, `2.6-*.log` | (a) Su Windows stdin serve alla password della management, quindi la configurazione passa da una **named pipe a uso singolo** con ACL SYSTEM + Administrators, che OpenVPN apre come file (`--config \\.\pipe\galleria-cfg-…`): connesso, 0 file con una riga della chiave privata. La ricerca della riga di `ta.key` trova solo `openvpn.exe`: avevo scelto la riga d'intestazione, che è una stringa del programma. (a2) `--config stdin` insieme alla password su stdin non funziona: OpenVPN legge la password come riga di configurazione. (b) `--management-external-key` funziona (ECDSA). (c) Ripiego con file temporaneo: funziona ed è cancellato al primo `HOLD`, ma **l'ACL ottenuta non è "solo SYSTEM"**: oltre a SYSTEM e Administrators ci sono decine di voci `NT SERVICE\*` ereditate dal token. Se serve il ripiego, il file va creato con il descrittore di sicurezza già giusto |
| 2.7 | **superato in parte** | `2.7.txt`, `2.7-*.log` | Rotte: OpenVPN le applica da solo (`0/1`, `128/1`, `192.0.2.0/24`) e le toglie alla chiusura. DNS spinto con `dhcp-option`: lo applica OpenVPN da solo (`netsh`, `Set-DnsClient`), `Get-DnsClientServerAddress` mostra `10.8.0.1` e `intranet.galleria.test` si risolve. **Al nostro `--up` su Windows non arriva nessun `foreign_option_*`.** DNS spinto con la nuova opzione `dns`: **non viene applicato** (`DNS: could not talk to service`) perché su Windows lo fa solo il servizio interattivo di OpenVPN, e non arriva nemmeno al nostro `--up`. Vedi §5 |
| 2.8 | **superato in parte** | `2.8-<modo>.txt`, `2.8-<modo>.pcapng` | Tabella e spiegazione in §5 |
| 2.9 | **superato** | `2.9.txt`, `2.9-*.log` | Password e OTP sbagliati: `auth_failed`, nessun nuovo tentativo, uscita 2. Server muto: 4 tentativi con attese 1 → 2 → 4 s, poi `gave_up` (31 s) |
| 2.10 | **superato per Windows 2.7.9**; Linux non rieseguito | i punti sopra | Stesso crate per Linux e Windows; le differenze sono in `src/plat.rs`. Dopo il porting il binario Linux compila ma **non ho rifatto le prove Linux**. Differenze annotate in §6 |

### Spike 1 — WireGuardNT su Windows

Prove in `spikes/wireguard-nt/evidence/windows/`.

| # | Esito | Prova | Note |
|---|---|---|---|
| 1.1 | **superato** | `1.1-1.2-1.3.txt` | Servizio `galleriad`, `LocalSystem`, Rust. `wireguard.dll` caricata dalla cartella del servizio con percorso completo; adattatore `galleria-wg` creato, `10.99.0.2/24`, ping a `10.99.0.1` 4/4, `curl` al sito di prova attraverso il tunnel |
| 1.2 | **superato** | `1.1-1.2-1.3.txt` | Una riga `stats` al secondo nel registro del servizio: ultimo handshake, `rx_bytes` e `tx_bytes` crescono durante il ping (14/14) |
| 1.3 | **superato** | `1.1-1.2-1.3.txt` | Schede, rotte e indirizzi prima e dopo: 41 righe e 41 righe, 0 differenze. Chiudere l'handle dell'adattatore toglie adattatore, indirizzo e rotte |
| 1.4 | **non provato** (job); build locale riuscita | `spikes/wireguard-nt/ci/spike-windows-build.yml` | Scelta provata: crate `wireguard-nt` 0.5 (binding a `wireguard.h`, licenza MIT). Compila per `x86_64-pc-windows-gnu` da WSL e l'app Tauri compila nella Sandbox con toolchain `windows-gnu` (3 min 19 s). `defguard_wireguard_rs` non l'ho valutato. Il job è scritto, non eseguito e non pubblicato |
| 1.5 | **superato con il blocco WFP**; fallito con le rotte | `1.5-<modo>.txt`, `1.5-<modo>.pcapng` | Tabella in §5 |
| 1.6 | **superato** | `1.6.txt`, `1.6-msi-install.txt`, `1.6-msi-uninstall.txt`, `1.6-1.7-build.txt` | MSI costruito da `cargo tauri build` (tauri-cli 2.12.4, WiX 3.14) con il frammento `app/src-tauri/wix/service.wxs`. `msiexec /i /qn` esce 0: servizio `galleriad` `Running`, `AUTO_START`, `LocalSystem`, percorso `"C:\Program Files\Galleria Spike\galleriad.exe" service`; `hello` risponde 0.0.1; il servizio installato alza l'adattatore `galleria-wg`. `msiexec /x /qn` esce 0 con il tunnel attivo: servizio assente (`sc query` 1060), 0 processi, adattatore sparito, cartella sparita. Restano il driver `wireguard.inf` nel driver store e `C:\ProgramData\galleria-spike` (registro). Limiti sotto la tabella |
| 1.7 | **superato** | `1.7.txt` | App 0.0.1 installata dall'MSI, `latest.json` e MSI serviti da `127.0.0.1:8765` dentro la Sandbox. Firma fatta con un'altra chiave: `update REJECTED: The signature was created with a different key than the one provided`, resta 0.0.1. File con un byte cambiato e firma giusta: `update REJECTED: The signature verification failed`, resta 0.0.1. Firma giusta: l'MSI 0.0.2 si installa in silenzio, il servizio 0.0.1 viene fermato e parte lo 0.0.2 (fermo 1,5 s nel registro del servizio; sha256 del binario installato uguale a quello 0.0.2), l'app riparte da sola e registra `app version 0.0.2 ; service hello -> … "version":"0.0.2"`, poi `no update available`. Limiti sotto la tabella |
| 1.8 | **superato** | `1.8.txt` | Pipe `\\.\pipe\galleria-spike` con ACL `D:P(A;;GA;;;SY)(A;;GA;;;BA)`. Amministratore: `hello` risponde. Secondo account locale del gruppo Users (`gltest`): `Accesso negato (os error 5)` sia per `hello` sia per `disconnect`; nel registro del servizio le sue richieste non compaiono |

Limiti di 1.6 e 1.7 (cosa le prove **non** dicono):

- **Solo da amministratore senza UAC.** Nella Sandbox l'utente è amministratore e l'UAC è spento. L'MSI è per
  macchina e installa un servizio: su un PC normale ogni aggiornamento chiede l'elevazione. Con
  `installMode: quiet`, usato qui, la documentazione di Tauri dice che servono già i privilegi di amministratore.
  **Non provato**: aggiornamento lanciato da un utente standard o con UAC attivo.
- **Chiave dell'updater usa e getta.** Generata dentro la Sandbox con `cargo tauri signer generate`, mai uscita
  dall'ospite e sparita con lui: **non è la chiave di release**. Nelle prove c'è solo la chiave pubblica.
- **MSI senza firma Authenticode** (`Get-AuthenticodeSignature`: `NotSigned`): SmartScreen e la firma del codice
  (R3) non sono toccati da questa prova.
- **Validazione ICE di WiX saltata.** Nella Sandbox manca VBScript e `light.exe` fallisce con l'errore 2738
  (`1.6-1.7-build-ice-failure.txt`): l'MSI è stato collegato con `-sval`. Su un runner con VBScript la validazione
  gira e potrebbe segnalare errori sul frammento che qui non ho visto. Microsoft sta togliendo VBScript da Windows:
  è un rischio per la build con WiX 3, da tenere d'occhio sul runner di GitHub.
- **Aggiornamento senza tunnel attivo.** Non ho provato cosa vede l'utente se l'aggiornamento parte con la VPN su
  (in 1.6 la disinstallazione con il tunnel attivo lo chiude in modo pulito).
- **Tempi.** Ogni transazione MSI nella Sandbox è durata circa 2 minuti (aggiornamento: 124 s tra inizio e fine
  negli eventi `MsiInstaller`), quasi tutti prima delle azioni sui servizi. Non ho cercato la causa e non so se
  vale fuori dalla Sandbox.
- Build fatta con toolchain `x86_64-pc-windows-gnu` e `updater` con `native-tls`; in 0.0.2 cambia solo il numero
  di versione. Il riavvio dell'ospite non è provato (la Sandbox non sopravvive al riavvio): che il servizio parta
  al boot lo dice solo `AUTO_START`.

## 5. Dove le prove smentiscono o precisano le decisioni

**D15 (blocco IPv6) su Windows.** Profilo "tutto nel tunnel" solo IPv4, cattura di 60 s. "Pacchetti di prova" sono
quelli verso `2001:db8:99::1` con sorgente l'indirizzo della scheda fisica; "altri" sono connessioni di sistema
(`svchost`, porta 80) aperte con la stessa sorgente. L'ultima colonna è un client che si collega direttamente
all'indirizzo IPv6, con timeout di 10 s.

| Prova | Pacchetti di prova usciti | Altri pacchetti IPv6 usciti | `curl` sul nome dual-stack | Connessione diretta all'IPv6 |
|---|---|---|---|---|
| OpenVPN, nessuna protezione | 12 | 34 | IPv4 dopo 0,24 s | scade a 10 s |
| OpenVPN, nativo (`redirect-gateway ipv6` + `block-ipv6`) | 12 | 33 | IPv4 dopo 0,24 s | scade a 10 s |
| OpenVPN, nativo + `ifconfig-ipv6` locale, DCO | **0** | 18 | IPv4 dopo 0,26 s | **scade a 10 s** |
| OpenVPN, nativo + `ifconfig-ipv6` locale, senza DCO | **0** (12 entrano nel tunnel) | 18 | IPv4 dopo 0,24 s | **scade a 10 s** |
| WireGuard, nessuna protezione | 11 | 39 | IPv4 dopo 0,24 s | scade a 10 s |
| WireGuard, rotte `::/1` e `8000::/1` verso il tunnel | **12** | 33 | IPv4 dopo 0,23 s | scade a 10 s |
| WireGuard, blocco WFP in uscita (`2000::/3`, `fc00::/7`) | **0** | **0** | IPv4 dopo **0,03 s** | **errore 10013 in 12 ms** |

Cosa dice:
1. Come su Linux, la via nativa senza `ifconfig-ipv6` sulla 2.7 non installa le rotte IPv6; con l'`ifconfig-ipv6`
   locale i pacchetti di prova non escono più.
2. **Su Windows la via nativa non dà l'errore immediato**, né con DCO né senza: chi prova l'IPv6 aspetta il proprio
   timeout. È il contrario di Linux, dove la stessa configurazione rispondeva in 1 ms. `curl` se la cava con Happy
   Eyeballs (0,24 s); un client che provasse prima l'IPv6 aspetterebbe. In queste prove Windows ha messo l'IPv4 per
   primo nella risoluzione del nome, quindi il client sequenziale sul nome ha risposto in 0,04 s in tutti i modi:
   il criterio "meno di 2 s" è rispettato, ma per l'ordine degli indirizzi, non per il blocco.
3. **Le rotte non fermano tutto.** Con le rotte IPv6 verso il tunnel restano 18 pacchetti di `svchost` che escono
   dalla scheda fisica con il suo indirizzo come sorgente. Non ho identificato il servizio.
4. Il meccanismo comune di D15 scritto come "rotte verso il tunnel" con WireGuard **non ha funzionato**: 12
   pacchetti di prova usciti. Non ho verificato se `netsh` abbia installato le rotte (la tabella delle rotte è in
   `1.5-route.txt`).
5. Il **blocco WFP** soddisfa tutto il criterio: zero pacchetti, anche quelli di sistema, ed errore immediato.
   Nello spike è una regola del firewall aggiunta dal servizio con `netsh` e tolta alla disconnessione; un demone
   vero la metterebbe in una sessione WFP dinamica, che sparisce se il demone muore. Provato solo con WireGuard:
   con OpenVPN non l'ho provato, ma il filtro non dipende dal protocollo.

Proposta, da decidere con Peppe: su Windows il blocco IPv6 è un filtro WFP per tutti i backend, e per OpenVPN la via
nativa resta solo su Linux. D15 oggi dice il contrario per OpenVPN.

**D10 e D14 (DNS di OpenVPN su Windows).** Lanciando `openvpn.exe` direttamente dal demone, come previsto, il DNS
spinto con l'opzione `dns` va perso: OpenVPN lo applica solo attraverso il suo servizio interattivo e non lo passa
al nostro `--up`. Il DNS spinto con `dhcp-option` invece lo applica OpenVPN da solo, fuori dal modulo di rete comune.
Due strade, non provate: leggere le opzioni spinte dalla management interface e applicare noi il DNS (coerente con
D10), oppure avviare OpenVPN attraverso il suo servizio interattivo. Il canale `foreign_option_*` di Linux su
Windows non esiste.

**D12 e M1 (ACL della pipe).** L'ACL provata lascia entrare SYSTEM e Administrators. Una GUI lanciata da un utente
normale, o da un amministratore non elevato, resterebbe fuori: chi è autorizzato su Windows (l'utente che ha
installato, un gruppo dedicato, gli utenti interattivi) è una scelta ancora da fare.

**D14 (OpenVPN GUI).** Confermato da Aitano: l'MSI 2.7.9 con `/qn` installa `openvpn-gui.exe` nella selezione
predefinita (`evidence/windows/aitano-sandbox-probe-transcript.txt`).

## 6. Differenze Linux / Windows nel demone dello spike 2

| Tema | Linux | Windows | Cosa fa il demone |
|---|---|---|---|
| Canale della management | socket Unix in cartella 0700 | TCP 127.0.0.1, porta casuale, password su stdin | `src/plat.rs` |
| Configurazione senza disco | `--config /dev/stdin` | `--config \\.\pipe\…` (stdin è occupato dalla password) | named pipe a uso singolo con ACL |
| Comandi scritti uno dopo l'altro | accettati | **OpenVPN 2.7.9 ha eseguito due volte `state on` e non ha mai visto `hold release`** (`2.4-pipelined.log`) | un comando alla volta, il successivo dopo la risposta. Vale per tutte e due le piattaforme |
| Riga di comando di `--up` | percorso normale | la barra rovesciata è un escape e la riga è letta due volte: `C:\\gl\\x.exe` arriva come `C:glx.exe` | percorso con barre in avanti; con spazi nel percorso (Program Files) serviranno le virgolette, non provato |
| DNS nell'ambiente di `--up` | `foreign_option_*` | niente | vedi §5 |
| DCO | assente nel kernel WSL | attivo di default | nessuna differenza nei comandi |

Tre cose di sicurezza emerse e **non provate**:
- Su TCP la sola difesa è la password. Prima di mandarla, e prima di rispondere a una richiesta di credenziali, il
  demone dovrebbe controllare che la porta appartenga al suo processo figlio: un altro processo locale che occupasse
  quella porta un attimo prima di OpenVPN riceverebbe la password e poi potrebbe chiedere utente e password della VPN.
- `--management-client` è peggio da questo punto di vista: è il demone ad ascoltare e a mandare la password a chi si
  collega per primo.
- OpenVPN non limita i tentativi di password sulla porta: con 256 bit casuali non è un problema.

## 7. Incidenti e insidie

1. **Interoperabilità WSL → Windows rotta dal laboratorio.** I client Linux dello spike 2 (`gl-client*`,
   privilegiati, con systemd) all'avvio azzerano `binfmt_misc` del kernel di WSL: sparisce la voce `WSLInterop` e da
   WSL non parte più nessun `.exe`, per tutta la distribuzione e per tutte le sessioni. Erano partiti perché
   `CLIENTS=""` in `lab/up.sh` veniva letto come "tutti" (corretto). Ho ripristinato `WSLInterop` e la voce
   `python3.12` dell'host da un container privilegiato; resta una voce `python3.14` lasciata dai container, che
   non ho tolto. È quasi certo che fosse successo anche durante lo spike 2 su Linux.
2. **Loopback della Sandbox rotto da una prova.** Con `remote 127.0.0.1` e `redirect-gateway def1` l'ospite è
   rimasto senza TCP su 127.0.0.1 e senza la rotta `127.0.0.1/32`; ho dovuto riavviare la Sandbox. Non ho isolato
   la causa. Da allora i profili di prova hanno `redirect-gateway local`. Riguarda solo il laboratorio (il server è
   su loopback per via del relay), ma un profilo vero con un server su loopback farebbe lo stesso.
3. Un server OpenVPN tratta come rinegoziazione un nuovo login dalla stessa porta sorgente ("username attempted to
   change from 'alice' to 'bob'"): era il relay, corretto con un socket per sessione.
4. Altre insidie del pilotaggio della Sandbox in `spikes/windows-lab/README.md`.
5. **Pulizia dopo il primo giro (fatta da Aitano).** Nella mia sessione `docker stop`/`docker rm` erano bloccati:
   Aitano ha tolto tutti i container `gl-*` e la rete `galleria-lab` e ha verificato che l'interop di WSL funziona
   (`WSLInterop` registrato). La voce `python3.14` di `binfmt_misc` è rimasta dov'è, per sua indicazione.
6. **Secondo giro (1.6 e 1.7).** Nessun container avviato. La Sandbox non partiva: era rimasto un
   `WindowsSandboxServer.exe` che non rispondeva (`wsb list` in timeout); il `taskkill /F` mi è stato bloccato e non
   l'ho aggirato, il processo è uscito da solo al terzo tentativo di avvio. Nella Sandbox ho installato in più il
   runtime VC di Microsoft (serve a `cargo-tauri`) e ho provato a registrare una copia di `vbscript.dll` letta
   dall'host (non è servito). A fine lavoro Sandbox chiusa con `wsb stop`; resta in esecuzione
   `WindowsSandboxServer.exe`, il processo di servizio dell'app Sandbox, senza macchina virtuale.

## 8. [S] chiusi e aperti

Chiusi (aggiornati in `01-analisi.md` e `03-sessione-nello.md`):
- `03` "Ambiente Windows": IPv6 nella Sandbox (non c'è; basta un IPv6 finto per contare); driver e servizi nella
  Sandbox (sì per OpenVPN e WireGuardNT; MSI di Tauri non provato).
- `03` 1.5: come ottenere l'errore immediato su Windows → blocco WFP.
- `03` 2.3: secondo client rifiutato mentre il demone è collegato.
- `03` 2.6 e `01` §5: configurazione via pipe su Windows → named pipe.
- `03` 2.7: cosa applica OpenVPN da solo su Windows; `foreign_option_*` su Windows non arriva.
- `01` D14: OpenVPN GUI nella selezione predefinita dell'MSI.
- `01` §"aggiornamenti" e `03` "Ambiente Windows": servizio dichiarato in un frammento WiX dell'MSI di Tauri
  (1.6) e MSI di Tauri installabile nella Sandbox.

Aperti:
- 1.6 e 1.7: aggiornamento da utente standard o con UAC attivo; validazione ICE su una macchina con VBScript;
  aggiornamento con il tunnel attivo; durata delle transazioni MSI fuori dalla Sandbox. 1.4: job su runner Windows.
- 1.5: perché le rotte verso l'adattatore WireGuard non hanno fermato i pacchetti; quale servizio apre le
  connessioni IPv6 legate alla scheda; blocco WFP con OpenVPN e come sessione dinamica.
- 2.7: come portare al modulo comune il DNS spinto dal server su Windows.
- 2.6: ACL del file temporaneo di ripiego; `--management-external-key` con RSA.
- 2.10: prove Linux da rifare dopo il porting; 2.4 con DCO su Linux.
- Controllo del proprietario della porta della management; ACL della pipe per una GUI non elevata.
- `defguard_wireguard_rs` contro `wireguard-nt`: provata solo la seconda.

## 9. Cosa resta a Peppe

1. ~~Togliere i container del laboratorio~~: fatto da Aitano (§7 punto 5).
2. ~~Decidere se far fare 1.6 e 1.7~~: fatti nel secondo giro, superati con i limiti scritti in §4. Resta da
   decidere **come si eleva l'aggiornamento su un PC con UAC** (D16): l'MSI per macchina con il servizio chiede
   l'elevazione a ogni aggiornamento, e questo non è stato provato.
3. **D15 su Windows** (§5): blocco WFP per tutti i backend al posto della via nativa.
4. **DNS di OpenVPN su Windows** (§5): applicarlo noi leggendo le opzioni spinte, o passare dal servizio interattivo.
5. **Chi può parlare con il demone su Windows** (ACL della pipe).
6. La mail a SignPath (R3) e la ratifica di D10–D14, D16 e A12 restano sue, come da task.

## 10. Commit

Sul branch locale `spike/windows`, non pubblicati: `git log --oneline main..spike/windows`.
