# Fase 3 — Galleria su Windows: design

Versione 0.2 — 2026-11-28 — stato: **approvato il solo blocco P1** (Q60 = (a), Peppe, 2026-11-27); dopo P1 Peppe
decide se andare avanti, e su P2–P8 non si lavora. Non c'è codice e non è stato scaricato né installato niente. Il
piano a blocchi con le ore è `docs/superpowers/plans/2026-11-26-fase3-windows.md`; il piano di dettaglio di P1 è
`docs/superpowers/plans/2026-11-27-fase3-p1.md`.

Cosa cambia dalla 0.1 (2026-11-26): le decisioni Q58, Q59, Q60 e Q68 (§18); la correzione sull'host degli agenti,
che **non è WSL**: è il server di casa, Linux nativo con KVM (§13, §19). Dove resta scritto «WSL» si parla dello
spike del 10/11, che fu pilotato da WSL sulla macchina di Peppe.

## Riassunto per Peppe (una pagina)

**Cosa si ottiene.** I colleghi con Windows 11 installano un MSI, importano i loro `.ovpn` e `.conf`, e si connettono
dalla stessa finestra e dalla stessa tray di Linux, con WireGuard e con OpenVPN (che resta un programma da installare a
parte, D14). Le credenziali ricordate stanno cifrate nel profilo Windows dell'utente. La parte «essenziale» finisce
qui. Aggiornamento automatico, firma del codice, lettore di schermo e prove dei tunnel in CI sono «rifinitura» e si
decidono dopo.

**Quanto costa.**

| | Ore stimate | Card | Soldi |
|---|---|---|---|
| Solo il primo blocco (ambiente e tre prove di rischio) | 32 h | 3 | 0 € |
| Essenziale, blocchi P1–P6 | **181 h** (forchetta 160–240) | 14 | 0 € |
| Rifinitura, blocchi P7–P8 | 56 h | 5 | 0 €, oppure la firma (sotto) |
| Tutto | 237 h | 19 | |

Le card di P1 sono 3 e non 2 perché il gruppo A si esegue in due (in un run entra una VM sola): le ore non cambiano.

Per confronto, i piani della fase 2 sommavano 275 h e sono stati eseguiti in due giorni di calendario (25 e 26/11,
144 commit): l'essenziale di Windows è circa due terzi di quella fase. Le ore sono la stessa unità dei piani
precedenti, cioè lavoro degli agenti; le ore vere della fase 2 non sono state registrate, quindi la forchetta è una
mia stima e non una misura. Su Windows ogni giro di prova passa da una VM lenta, quindi il calendario sarà più lungo
a parità di ore.

**I rischi principali.**

1. **La rete su Windows non l'abbiamo mai fatta noi.** Nello spike il server era su `127.0.0.1` (dietro il relay) e il
   DNS lo metteva OpenVPN da solo. «Tutto nel tunnel» verso un server vero, e il DNS messo dal nostro demone, su
   Windows sono supposizioni. Per questo il primo blocco le prova con il codice dello spike prima di spendere il resto.
2. **L'ambiente di prova.** La VM Windows gira con KVM sul server di casa, su un disco meccanico (`/mnt/b`): quindi
   è lenta. La licenza di valutazione dura 90 giorni ed è pensata per chi valuta Windows, non per chi prova un suo
   programma: la scelta era tua ed è presa (Q59). La Sandbox dello spike non è più un ripiego alla portata degli
   agenti: sta sulla tua macchina Windows, e gli agenti girano sul server (Q75, nel piano di P1).
3. **Senza firma del codice** l'MSI mostra l'avviso di SmartScreen, e dove è acceso Smart App Control, o un
   antivirus aziendale severo, MSI e servizio non firmati possono essere bloccati del tutto. La firma costa soldi o
   passa da Addway (Q61).
4. **Serve un amministratore** a ogni installazione e a ogni aggiornamento: l'MSI installa un servizio di sistema.
   Se i colleghi non sono amministratori del loro PC, ogni versione passa da chi lo è (Q67).
5. **Manutenzione doppia.** Da qui in poi ogni modifica al demone o alla finestra va provata su due sistemi, e la
   parte Windows della CI non prova i tunnel (il runner è Windows Server, senza i server del laboratorio).

**Cosa mi serve da te.**

Già deciso il 2026-11-27: si fa solo P1 (Q60); runner, VM di valutazione, e la tua macchina solo alla fine (Q58);
ISO e `swtpm` li metti tu (Q59). Resta da fare, **prima che parta la card della VM** (P1-A1):

- **La ISO.** Scaricare dall'Evaluation Center di Microsoft la ISO di Windows 11 Enterprise 26H2 x64 di valutazione
  e metterla in `/mnt/b/vm`, con il nome che ha (basta che finisca in `.iso` e che lì ce ne sia una sola).
- **Il TPM emulato.** Sul server: `sudo apt install swtpm swtpm-tools`. Gli agenti non hanno `sudo`.
- La card controlla le due cose all'avvio (`ls /mnt/b/vm/*.iso`, `which swtpm swtpm_setup`) e se ne manca una si
  chiude senza lavorare. Il gruppo B di P1 (il workspace che compila su Windows) non le aspetta.
- Due risposte nuove, nate scrivendo il piano di P1 (in fondo a quel piano). **Q74**: per la prova R-A serve un
  server che la VM raggiunga passando dal gateway; la via più corta è pubblicare per meno di un'ora il server
  WireGuard di prova sull'indirizzo Tailscale del server di casa. Senza il tuo sì non si pubblica niente e R-A
  rischia di restare «non concludente». **Q75**: cosa si fa se la VM non è usabile in 8 h, ora che la Sandbox non è
  alla portata degli agenti (la mia raccomandazione: ci si ferma e si torna da te).

Per dopo P1, se si va avanti:

- Due fatti sui colleghi: sono amministratori del loro PC? I PC sono gestiti dall'azienda (antivirus, criteri sulle
  applicazioni)? Cambiano Q61 e Q67.
- La decisione sulla firma (Q61). La mia raccomandazione è niente firma in fase 3.
- Un'ora tua alla fine, sulla tua macchina, con il profilo vero di Addway: gli agenti lì non installano niente (Q73).

Se la fase 3 non si fa, Galleria resta uno strumento Linux e chi ha Windows continua con OpenVPN GUI e con il client
ufficiale di WireGuard. Niente del lavoro già fatto si perde.

---

Fonti: `ROADMAP.md` (fase 3), `docs/01-analisi.md` v0.7 (D5–D18, D25, D27; qui non si riaprono),
`docs/02-modello-minacce.md` (M1–M7), `docs/05-esito-spike-windows.md` (citato come «spike §n»),
`docs/08-esito-fase2.md` («Per la fase 3»), le spec delle fasi 1 e 2 («spec F1», «spec F2»), il codice in `crates/`
a `c26fe63` e quello degli spike in `spikes/`. Le fonti esterne lette oggi sono in fondo.

Legenda: **[V]** verificato oggi su una fonte letta o sul codice, con la fonte tra parentesi · **[S]** supposto, con
il task che lo verifica (§17) · *dedotto*: ragionamento mio su fatti verificati.

Restano fermi: D5–D32, Q3–Q57, A6, A8, A11, A12 e **l'IPC della fase 1** (`PROTOCOL_VERSION` resta 1, nessun metodo
nuovo). Galleria è uno strumento per Peppe e i colleghi, non un prodotto (A7 tolto): il dimensionamento segue questo.

## 1. Obiettivo e criteri di uscita

Stato di partenza **[V: `grep` sui crate, oggi]**: nessun crate compila per Windows. Codice Unix senza `cfg` sta in
`galleria-ipc` (`client.rs`), `galleriad` (sette file), `galleria-net` (`plan.rs`, `runner.rs`),
`galleria-backend-openvpn` (`plat.rs`, `hook.rs`), `galleria-backend-wireguard` (`device.rs`), `galleria-profile`
(`store.rs`, `import_file.rs`), `galleria-cli` (`prompt.rs`) e `galleria-gui` (sei file). In `tauri.conf.json` il
pacchetto è spento (`bundle.active: false`).

I criteri si chiamano W1–W13. La terza colonna dice a quale criterio di Linux corrispondono; l'ultima se servono
all'uso quotidiano («E», essenziale) o sono rifinitura («R»).

| # | Criterio | Corrisponde a | Come si verifica | |
|---|---|---|---|---|
| W1 | Il servizio `galleriad` gira come LocalSystem e parte con il sistema; la finestra e la CLI di un utente normale gli parlano dalla named pipe senza UAC; chi non è SYSTEM, amministratore o utente interattivo è rifiutato (D18) | E7, G1, spike 1.8 | prove sul runner con un demone a `FakeBackend`; in VM con due utenti veri | E |
| W2 | WireGuard: tunnel con WireGuardNT verso il server del laboratorio, traffico, statistiche, smontaggio senza residui | E1, spike 1.1–1.3 | VM | E |
| W3 | OpenVPN: `openvpn.exe` trovato dal registro, versione minima 2.6, configurazione e segreti mai su disco, credenziali, OTP e passphrase dalla management | E3, spike 2.1–2.6 | VM, con OpenVPN 2.6 e 2.7 | E |
| W4 | DNS del profilo o del server applicati dal demone prima di `connected` e tolti alla disconnessione, con tutti e due i backend (D17) | E4 | VM | E |
| W5 | Blocco IPv6 con un filtro WFP, per tutti e due i backend: zero pacchetti IPv6 globali ed errore immediato; il filtro sparisce se il demone muore (D15) | E5, spike 1.5 | VM, con la cattura dello spike | E |
| W6 | Dopo `disconnect`, dopo la morte del servizio e dopo un riavvio non restano adattatori, rotte, DNS, filtri o processi `openvpn.exe` | E6 | VM | E |
| W7 | Un profilo con direttive pericolose non esegue niente (M2); nessun segreto in file, righe di comando e log (M7) | E7, G3, G7 | VM con il profilo malevolo; prove sul runner | E |
| W8 | Le credenziali ricordate stanno cifrate per l'utente; finestra e CLI leggono lo stesso archivio (D25 estesa) | G5, G6 | prove sul runner | E |
| W9 | La finestra su WebView2: primo avvio con i testi di Windows, rubrica, import (file, trascinamento, testo, doppio click su un `.ovpn`), connetti, disconnetti e sostituisci, tray con il click, chiusura nella tray | G1–G4, G8–G10 | prove di livello 2 e fumo ridotto sul runner; giro quotidiano in VM | E |
| W10 | L'MSI installa finestra, servizio e `wireguard.dll`; un aggiornamento tiene i profili; la disinstallazione toglie tutto tranne i profili dell'utente; provato da utente normale con UAC | spike 1.6 | runner (senza UAC) e VM (con UAC) | E |
| W11 | Aggiornamenti: l'avviso di versione nuova funziona come su Linux (E). L'updater di Tauri scarica e installa l'MSI firmato con la nostra chiave e rifiuta una firma sbagliata (R) | G11, spike 1.7, D16 | prove sul runner; VM | E / R |
| W12 | Tastiera come su Linux (E, già provata da Vitest). Narratore e NVDA dicono esiti e cambi di stato (R) | G12 | VM, dal registro del parlato di NVDA | R |
| W13 | Matrice manuale su Windows 11 e avvio a freddo misurato | G10 manuale, G13 | VM | R |

**Q2: i criteri Windows rimandati dalla fase 1.** Sono due, e `docs/01` li chiama «primo test» e «primo criterio»
della fase 3 **[V: righe D15 e D17]**:

1. **D17**, la lettura del `PUSH_REPLY` su Windows. In fase 1 il demone lo legge da `stdout` di OpenVPN, perché il
   log della management non dà garanzie sui tempi (S9). Su Windows resta **[S]**: è W4 con OpenVPN.
2. **D15**, il filtro WFP con OpenVPN e come sessione dinamica. Provato solo con WireGuard e con una regola di
   `netsh advfirewall` **[V: spike §5]**. È W5.

Per rispettare «primo» senza aspettare il servizio e i backend, tutti e due si provano **nel primo blocco** (P1),
con il codice degli spike, insieme alla terza supposizione grossa (§7): se una cade, si torna da Peppe prima di
spendere il resto.

## 2. Fuori scope

Windows 10 e Windows su ARM (A12 dice Windows 11 x64); installazione per utente senza amministratore (un servizio
la esclude); OpenVPN incluso nel pacchetto (D14); `winget`; kill switch, protezione dalle fughe di DNS, avvio della
finestra con il sistema, connessione automatica (dopo la v0.1); OpenConnect; il file temporaneo di ripiego per la
configurazione di OpenVPN e `--management-external-key` (spike 2.6: la via della pipe basta); una VM Hyper-V (§13).
Sul lato Linux non cambia niente di ciò che si vede: i 15 job di oggi restano verdi a ogni card.

## 3. I punti aperti dello spike, uno per uno

Dall'elenco «Aperti» di spike §8 e da «Cosa resta a Peppe» di spike §9.

| Punto aperto | Stato oggi | Cosa ne fa la fase 3 |
|---|---|---|
| Aggiornamento da utente normale o con UAC | non provato | §11, Q62: in VM, nella rifinitura; nell'essenziale l'aggiornamento è un MSI lanciato a mano, che chiede l'elevazione |
| Validazione ICE su una macchina con VBScript | **chiuso** **[V: run 42895494397 e 43661084501 di `spike-windows-build`, `success`]** | resta il rischio che Microsoft spenga VBScript (§11) |
| Aggiornamento con il tunnel attivo | non provato | P6 (MSI lanciato a mano) e P7 (updater) |
| Durata delle transazioni MSI fuori dalla Sandbox | non misurata | P6, in VM |
| 1.4, job su runner Windows | **chiuso** **[V: stessi run]** | — |
| Perché le rotte verso l'adattatore non fermavano l'IPv6; quale servizio apre quelle connessioni | non cercato | **non si cerca**: D15 ha scelto il filtro WFP, che ferma anche quei pacchetti |
| WFP con OpenVPN e come sessione dinamica | [S] | P1 (prova), P3 (codice) |
| 2.7, DNS spinto dal server | deciso (D17), [S] | P1 (prova), P4 (codice) |
| ACL del file temporaneo di ripiego; `--management-external-key` con RSA | aperti | **fuori scope** (§2) |
| 2.10, prove Linux dopo il porting; 2.4 con DCO su Linux | **chiusi** dalla fase 1 (S8, job `lab-dco`) **[V: ROADMAP]** | — |
| Proprietario della porta della management | non provato | §6.2, P4 |
| ACL della pipe per una finestra non elevata | deciso (D18), non provato | §4, P2 |
| `defguard_wireguard_rs` contro `wireguard-nt` | provata solo la seconda | Q64: si usa la seconda |

## 4. Servizio e named pipe (D12, D18, M1)

**Servizio.** `galleriad.exe service` sotto il gestore dei servizi con il crate `windows-service` (lo stesso dello
spike **[V: `spikes/wireguard-nt/galleriad/Cargo.toml`]**), account LocalSystem, avvio automatico, riavvio dopo un
errore. `galleriad.exe run` resta per le prove, in console. L'arresto del servizio e lo spegnimento del sistema
fanno ciò che su Linux fa `SIGTERM`. Gli errori fatali d'avvio vanno nel registro eventi di Windows, perché sotto
il gestore dei servizi `stderr` non lo legge nessuno; il resto dei log resta nel buffer del demone (D20).

**Pipe.** Nome fisso `\\.\pipe\galleria`. Regole:

1. Descrittore di sicurezza esplicito e protetto: accesso completo a SYSTEM e Administrators, lettura e scrittura
   agli utenti INTERACTIVE, rifiuto esplicito ai logon di rete (D18). Lo spike ha provato la forma con i soli SYSTEM
   e Administrators **[V: `pipe.rs`, `D:P(A;;GA;;;SY)(A;;GA;;;BA)`, criterio 1.8]**; la voce per INTERACTIVE è **[S]**.
2. **Agli utenti interattivi non va il diritto di creare istanze della pipe.** Su una pipe quel diritto ha lo stesso
   bit di «aggiungi dati», che la scrittura generica comprende **[S: da memoria, non riletto oggi]**: la voce si
   scrive con la maschera esplicita, e una prova mostra che un utente normale non riesce a creare una seconda istanza.
3. Client remoti rifiutati (è il predefinito di tokio **[V: docs.rs, `ServerOptions`]**) e prima istanza creata con
   `FILE_FLAG_FIRST_PIPE_INSTANCE`, così il servizio si accorge se il nome è già occupato **[V: spike, `pipe.rs`]**.
4. Più istanze, una per connessione: la finestra tiene una connessione lunga per gli eventi e una breve per comando
   (Q16), più la CLI. Il descrittore si passa a tokio con `create_with_security_attributes_raw` **[V: docs.rs]**.
5. **Il client controlla chi possiede la pipe prima di scrivere** (Q70). Su Linux il socket sta in una cartella di
   root; su Windows, a servizio fermo, qualunque processo dell'utente può creare `\\.\pipe\galleria` e ricevere la
   chiave privata e la password che la finestra manda con `connect`. Finestra e CLI aprono la pipe con il livello
   «solo identificazione» e proseguono solo se il proprietario è SYSTEM o Administrators **[S]**.
6. Il servizio identifica il chiamante (D18) e ne scrive il SID nel log. Non lo usa per altro: non c'è un
   proprietario della sessione (D21).

**Chi entra.** Tutti gli utenti interattivi della macchina, come su Linux tutti i membri del gruppo `galleria`. Un
secondo utente collegato in RDP può chiudere la connessione del primo: è la sessione estranea di D21.

**Primo avvio (D27).** Quattro stati, con testi nuovi. La finestra non chiede mai l'elevazione: mostra il comando.

| Stato | Come si riconosce su Windows | Testo |
|---|---|---|
| servizio fermo o non installato | la pipe non esiste | avviare il servizio «Galleria» da Servizi, o `sc start galleriad` da un prompt da amministratore |
| accesso negato | accesso negato all'apertura | la sessione non è interattiva |
| protocollo incompatibile | `hello` fuori intervallo | come su Linux |
| OpenVPN assente o vecchio | backend `available: false` | `winget install OpenVPNTechnologies.OpenVPN` **[V: D14]**, o l'MSI ufficiale |

**Cartella del servizio.** `%PROGRAMDATA%\Galleria` per il giornale, creata con il descrittore già giusto: nello
spike un file creato e poi ristretto aveva ereditato decine di voci `NT SERVICE\*` **[V: spike 2.6 (c)]**.
`wireguard.dll` si carica con il percorso completo dalla cartella d'installazione **[V: spike 1.1]**.

## 5. WireGuard e rete

**Backend.** Una seconda implementazione del tratto `WgDevice` (`create`, `configure`, `read`, `remove`
**[V: `device.rs`]**) con il crate `wireguard-nt` 0.5, che lo spike ha provato per tunnel, statistiche e smontaggio
(Q64). `wireguard.dll` è quella ufficiale, presa dallo zip con l'hash fissato nel job **[V: `spike-windows-build.yml`]**
e firmata da WireGuard LLC **[V: spike §2]**.

**Rete.** `galleria-net` oggi è un piano di comandi (`ip`, `resolvectl`, `sysctl`) eseguiti da un `CommandRunner`,
con un giornale per la pulizia **[V: `plan.rs`, `runner.rs`]**. Su Windows il piano produce comandi `netsh` per
indirizzi, rotte e DNS, eseguiti con il percorso completo di `System32` (Q65): così giornale, pulizia e prove del
piano si riusano. Fa eccezione il blocco IPv6, che è un'interfaccia di programmazione e non un comando.

**Blocco IPv6 (D15).** Una sessione WFP dinamica tenuta aperta dal servizio, con i filtri in uscita verso
`2000::/3` e `fc00::/7`. Gli oggetti di una sessione dinamica sono tolti da Windows quando la sessione finisce, anche
se il processo muore **[V: Microsoft, «Object Management» e `FwpmEngineOpen0`]**. Si applica quando su Linux si
mettono le rotte `unreachable`: profilo «tutto nel tunnel» senza IPv6 nel tunnel. Le chiamate sono FFI con
`windows-sys`, mai fatte da noi: tetto di 6 h, poi il ripiego è la regola di `netsh advfirewall` dello spike, tolta
dal giornale al riavvio (Q69). Con il ripiego, se il servizio muore l'IPv6 resta bloccato finché non riparte.

**Pulizia (W6).** L'adattatore di WireGuardNT sparisce alla chiusura dell'handle **[V: spike 1.3]**; che succeda
anche quando il processo muore è **[S]**. `openvpn.exe` invece sopravvive al servizio: va messo in un «job» di
Windows che lo chiude insieme al padre **[S]**. All'avvio il servizio ripulisce dal giornale, come su Linux.

**Limite dichiarato.** Windows può mandare le richieste DNS anche sulle altre schede. Fermarlo è la protezione dalle
fughe di DNS, che l'analisi mette dopo la v0.1: qui si scrive come limite e si misura in VM quanto succede davvero.

## 6. OpenVPN su Windows (D14, D17)

### 6.1 Ciò che lo spike ha già provato

Il modulo `plat` del backend ha oggi solo la parte Linux **[V: `plat.rs`, `probe()` cerca `/usr/sbin/openvpn`]**. La
parte Windows riprende lo spike **[V: spike §4 e §6]**: `openvpn.exe` dal valore `exe_path` di
`HKLM\SOFTWARE\OpenVPN`; management su `127.0.0.1` con porta casuale e password di 256 bit data su `stdin`;
configurazione da una named pipe a uso singolo con ACL a SYSTEM e Administrators; un comando alla volta sulla
management.

### 6.2 Ciò che è nuovo

1. **Proprietario della porta.** Prima di mandare la password il servizio controlla che la porta in ascolto sia del
   suo processo figlio, altrimenti un altro processo locale che la occupasse per primo riceverebbe la password e poi
   potrebbe chiedere le credenziali della VPN **[V: spike §6, rischio descritto e non provato]**. La tabella delle
   porte con il processo proprietario è **[S]**.
2. **Niente `--up` su Windows** (Q66). Su Linux il nostro programma fisso riceve da OpenVPN DNS e rotte
   nell'ambiente. Su Windows quel canale non porta il DNS **[V: spike 2.7]** e la riga di comando ha un problema di
   barre e virgolette non risolto **[V: spike §6]**. Il servizio decide quindi sul `PUSH_REPLY` letto da `stdout`
   (`push.rs` esiste già) e sullo stato `CONNECTED` della management, e dichiara `connected` solo dopo aver messo il
   DNS (M4).
3. **DNS.** OpenVPN su Windows applica da solo il DNS spinto con `dhcp-option` e non quello spinto con `dns`
   **[V: spike 2.7]**. Lo applichiamo sempre noi (D17), con `pull-filter ignore "dhcp-option DNS"` nel profilo
   generato per non averlo due volte **[S]**. L'adattatore di OpenVPN resta anche dopo la disconnessione: il DNS va
   tolto per esplicito e scritto nel giornale.
4. **Blocco IPv6.** Il filtro WFP si mette quando il `PUSH_REPLY` dice `redirect-gateway`, o prima del lancio se è
   nel profilo. Tra l'arrivo delle rotte e il filtro può esserci una finestra breve: si misura e si scrive.
5. **Convivenza.** L'MSI ufficiale installa anche OpenVPN GUI **[V: D14]**, e OpenVPN ha un suo servizio interattivo **[V: spike 2.7]**. Galleria
   non li usa. Due client che alzano la stessa VPN si disturbano: lo dice la guida.

## 7. Le tre supposizioni che decidono la fase

Si provano in P1, in VM, con il codice degli spike, prima di scrivere codice di prodotto.

| # | Supposizione | Perché non è già provata | Se è falsa |
|---|---|---|---|
| R-A | «Tutto nel tunnel» con WireGuardNT verso un server che non è su loopback: il traffico verso il server non rientra nel tunnel | nello spike il server era `127.0.0.1` dietro il relay **[V: spike §3]** | rotta esplicita verso il server sulla scheda fisica, da rifare a ogni cambio di rete: più codice in P3 (+6 h) |
| R-B | Il `PUSH_REPLY` si legge da `stdout` di `openvpn.exe` lanciato da SYSTEM, prima di `CONNECTED`, su 2.6 e 2.7; il DNS messo da noi sull'adattatore funziona | su Windows il DNS lo ha sempre messo OpenVPN | il servizio interattivo di OpenVPN come ripiego (D17): cambia P4, da ristimare |
| R-C | Il filtro WFP in sessione dinamica blocca l'IPv6 anche con OpenVPN e sparisce alla morte del processo | provata solo la regola `advfirewall`, con WireGuard | il ripiego di Q69 |

## 8. Archivio dei segreti (D25 estesa)

Su Linux ci sono due archivi a strati: il portachiavi, e il file `0600` come ripiego. Su Windows le strade sono due.

| | Credential Manager | File cifrato con DPAPI |
|---|---|---|
| Dove sta | archivio di Windows, per utente | un file per profilo in `%APPDATA%\galleria\secrets`, cifrato con la chiave dell'utente |
| Limite di grandezza | **2560 byte per voce** **[V: Microsoft, `CREDENTIALW`]** | nessuno |
| Si vede in | Gestione credenziali | da nessuna parte |
| C'è sempre | sì | sì |

Tra i segreti di un profilo OpenVPN c'è il blocco `<key>` intero **[V: `import_ovpn.rs`, `SECRET_BLOCKS`]**: una
chiave RSA da 4096 bit in PEM supera i 2560 byte *(dedotto dalla grandezza del formato)*, quindi con Credential
Manager andrebbe spezzata in più voci.

**Scelta (Q63): DPAPI.** Un solo archivio, sempre presente, senza limite e senza strati: su Windows spariscono i casi
«portachiavi assente», «bloccato» e «senza collezione» (Q20, F1). È una seconda implementazione del tratto
`SecretStore` (`get`, `set`, `delete`, `delete_all` **[V: `secrets.rs`]**), usata dalla finestra e dalla CLI. Due
limiti da scrivere nella guida: come su Linux con il portachiavi sbloccato, non protegge da un programma che gira
come lo stesso utente; e se un amministratore reimposta la password dell'utente, o il profilo passa a un altro PC,
i segreti non si aprono più e il profilo va reimportato.

I file dei profili e di stato oggi si proteggono con i permessi Unix (`0600`, controllo del proprietario
**[V: `store.rs`, `ui_state.rs`]**). Su Windows stanno in `%APPDATA%`, che è già dell'utente; il file esportato a
mano prende un'ACL del solo utente.

## 9. La finestra su WebView2: cosa vale e cosa no

Elenco file per file, sulla tabella di `docs/08` «Per la fase 3». «Vale» vuol dire che il file non si tocca.

**Frontend (`ui/`).**

| Pezzo | Verdetto | Nota |
|---|---|---|
| componenti, stato, design system, cataloghi | **vale** | React e TypeScript non dipendono dal sistema |
| testi del primo avvio | **si adatta** | oggi dicono `systemctl` e `usermod`: una variante per Windows nei due cataloghi (§4) |
| colori `oklch()`, caratteri, barre di scorrimento | **da riguardare** | WebView2 è Chromium, non WebKit: una passata con le fotografie nei due temi **[S]** |
| prove di livello 1 (Vitest, 710) | **valgono, restano su Linux** | non c'è motivo di rieseguirle su Windows |

**Lato Rust di `galleria-gui`.**

| File | Verdetto | Cosa serve |
|---|---|---|
| `supervisor.rs`, `calls.rs`, `events.rs`, `session.rs`, `rubrica*`, `edit.rs`, `logs.rs`, `error.rs`, `window.rs`, `closing.rs`, `update.rs`, `announce.rs`, `picker.rs`, i comandi | **vale** | usano il client IPC e i tratti; `closing` riceve `tray_shown` sempre vero |
| `link.rs` | **si adatta** | la classificazione di D27 sugli errori della pipe |
| `tray.rs` (modello del menu) | **vale** | `IconSettler` resta: su Windows non serve e non disturba |
| `tray_tauri.rs` | **si adatta** | icone `.ico`; il click sinistro apre la finestra: su Windows l'evento arriva, su Linux no **[V: docs.rs, `TrayIconEvent`]** |
| `tray_host.rs` | **si corregge** | fuori da Linux oggi risponde `false` **[V: `tray_is_shown`]**; su Windows deve dire `true` |
| `instance.rs` | **si rifà** | oggi `flock`, socket Unix e `SO_PEERCRED` (Q36); su Windows un mutex con nome e una pipe del solo utente |
| `ui_state.rs`, `import.rs`, `export.rs`, `commands/app.rs` | **si adattano** | percorsi `%APPDATA%` e `%LOCALAPPDATA%`, protezione dei file, nome dell'utente |
| `picker_gtk.rs`, `export_gtk.rs` | **restano di Linux** | accanto, le finestre «Apri» e «Salva» di Windows, sempre da Rust (spec F2 §3.2, regola 3); oggi fuori da Linux sono un annullo **[V: `lib.rs`]** |
| `update_soup.rs` | **resta di Linux** | accanto, la richiesta delle versioni per Windows, con un crate HTTP (Q51 aveva misurato `reqwest` e `ureq`) |
| `announce_atk.rs` | **resta di Linux** | su Windows `NoAnnouncer` nell'essenziale; W12 nella rifinitura |
| `lib.rs`, `tauri.conf.json` | **si adattano** | i collegamenti per piattaforma; il pacchetto MSI acceso solo su Windows; `fileAssociations` c'è già |

**Fuori da `galleria-gui`:** `galleria-profile` (`store.rs` e `import_file.rs` si adattano, `secrets_*` §8),
`galleria-ipc` (`client.rs`: `connect` prende un percorso, diventa socket o pipe; è il punto di innesto di spec F1
§3.3), `galleria-cli` (`prompt.rs`: la password senza eco con la console di Windows).

**Annunci (W12, rifinitura).** Su Linux le regioni vive di WebKitGTK sono mute e l'annuncio parte da Rust. WebView2
è Chromium: se Narratore e NVDA leggono le regioni vive, basta che la pagina le scriva e Rust non fa niente **[S]**.
Se no, serve un annuncio via UIA. Come su Linux (Q56), si legge il registro del parlato di NVDA, senza ascolto.

## 10. Prove su tre livelli, su Windows

| Livello | Su Windows | Dove |
|---|---|---|
| 1 · Frontend | non si riesegue | — |
| 2 · Comandi Rust | demone vero con `FakeBackend` su una pipe con nome casuale; le prove che usano permessi e utenti Unix restano di Linux | runner |
| 3 · Fumo | l'app costruita, pilotata con `tauri-driver` e `msedgedriver`: su `windows-latest` le prove WebDriver girano senza schermo finto **[V: Tauri, «WebDriver · Continuous Integration»]** e il driver di Edge è nell'immagine **[V: `Windows2025-Readme.md`]**. Fumo **ridotto**: avvio, primo avvio, import da testo, connessione al demone finto. Il menu della tray non si legge (su Linux si leggeva dal bus) | runner |
| Laboratorio | tunnel veri, UAC, riavvio, MSI: i server Docker del laboratorio sul server di casa, il client nella VM | VM (§13) |

## 11. Pacchetto MSI e aggiornamenti (D16)

**MSI.** Quello di Tauri con WiX 3, con il servizio in un frammento, come nello spike **[V: `service.wxs`, criterio
1.6]**. Installazione per macchina: chiede l'elevazione. Dentro: finestra, `galleriad.exe`, `galleria.exe`,
`wireguard.dll`. In più rispetto allo spike: il riavvio del servizio dopo un errore, l'associazione dei `.ovpn`
(`bundle.fileAssociations` c'è già **[V: `tauri.conf.json`]**; che l'MSI la scriva è **[S]**), l'aggiornamento che
tiene i profili. La disinstallazione toglie servizio, adattatore e `%PROGRAMDATA%\Galleria`; restano i profili in
`%APPDATA%` e il driver di WireGuard nel deposito dei driver **[V: spike 1.6]**. WebView2 non si installa: Windows 11
lo ha già **[S]**.

**Rischio VBScript.** La validazione ICE di WiX 3 usa VBScript. Microsoft lo spegnerà per impostazione predefinita
«intorno al 2027» e poi lo toglierà **[V: Windows IT Pro Blog, «VBScript deprecation»]**. Oggi sul runner funziona
**[V: run del 15/11]**. Il giorno che smette, la build salta la validazione (`-sval`, come nella Sandbox) e lo si
scrive.

**Aggiornamenti.** Due livelli (Q62):

1. **Essenziale: l'avviso.** Quello della fase 2: una richiesta al giorno, senza identificativi, disattivabile
   (G11). L'utente scarica l'MSI nuovo e lo lancia; Windows chiede l'elevazione; l'MSI ferma il servizio, quindi la
   VPN cade, e lo dice la guida. Costa solo la richiesta HTTP per Windows.
2. **Rifinitura: l'updater di Tauri.** Scarica l'MSI, ne verifica la firma con la nostra chiave e lo installa. Lo
   spike lo ha provato da amministratore senza UAC e in modo `quiet` **[V: spike 1.7]**, che però non può chiedere
   l'elevazione da solo; il modo predefinito, `passive`, mostra una barra e non chiede niente all'utente
   **[V: Tauri, «Updater»]**. Che con `passive` compaia la richiesta UAC, e cosa succede a un utente che non è
   amministratore, è **[S]**. Servono in più: la chiave di firma degli aggiornamenti nei segreti del repository, un
   flusso di rilascio che pubblica `latest.json`, una release vera. Va riverificato che il plugin faccia una sola
   richiesta e senza identificativi (A11).

D16 resta la destinazione. Cambia solo l'ordine: prima l'avviso, che c'è già.

## 12. Firma del codice

Due firme diverse. Quella **degli aggiornamenti** è la chiave di Tauri: è nostra, è gratis e serve solo con
l'updater. Quella **Authenticode** è ciò che Windows guarda su MSI ed eseguibili, e ha queste strade:

| Strada | Costo | Chi può | Limiti |
|---|---|---|---|
| (a) Nessuna | 0 | — | avviso di SmartScreen sull'MSI scaricato («Ulteriori informazioni», «Esegui comunque»). Con Smart App Control acceso, un eseguibile non firmato è bloccato e un servizio non parte (errore 4551) **[V: issue di Sunshine e di Cryptomator]**; Microsoft dice che Smart App Control di solito non si usa sui PC gestiti da un'azienda **[V: nota di Autodesk che cita Microsoft]** |
| (b) Azure Artifact Signing (già Trusted Signing) | 9,99 $ al mese **[V: due fonti secondarie del 14 e 17/11/2026; la pagina dei prezzi di Microsoft non l'ho letta]** | organizzazioni dell'UE sì; **persone fisiche solo negli Stati Uniti e in Canada** **[V: Microsoft, FAQ e «Quickstart»]** | Peppe da privato non può. Può Addway, e allora l'editore che Windows mostra è Addway: è una decisione dell'azienda su un progetto personale. Serve una sottoscrizione Azure a pagamento **[V: FAQ]**. Non toglie subito SmartScreen: la reputazione si costruisce con i download |
| (c) Certificato di una CA | da circa 100 a circa 1000 $ l'anno, più un dispositivo per la chiave **[V: fonte secondaria]** | anche una persona | stessa reputazione da costruire |
| (d) SignPath Foundation | 0 | progetti open source | chiede un progetto già rilasciato nella forma da firmare, una reputazione verificabile, e l'approvazione a mano di ogni release **[V: signpath.org, «Terms»]**; resta la domanda su `wireguard.dll` (R3). La roadmap la mette in fase 4 |
| (e) Certificato fatto da noi | 0 | — | va installato tra gli editori attendibili su ogni PC; non dà reputazione |

**Raccomandazione (Q61): (a) in fase 3**, con gli hash SHA-256 pubblicati accanto all'MSI (M6) e i passi di
SmartScreen nella guida. Se i PC dei colleghi hanno Smart App Control o un criterio che blocca i non firmati, (a)
non basta, e la strada realistica è (b) con Addway. Per questo servono i due fatti chiesti nel riassunto.

## 13. Ambiente di prova

Tre strade. Non ho scaricato niente e non ho creato VM.

### 13.1 VM Windows 11 di valutazione in `/mnt/b/vm`

Come le VM Linux della fase 2: qemu con KVM diretto sul server di casa, che è Linux nativo **[V il 28/11:
`/dev/kvm` c'è, `systemd-detect-virt` dice `none`, kernel `7.3.0-11-generic`]**, comandata da `vmctl.sh`
**[V: `lab/vm/vmctl.sh`]**.

- **Costo.** 0 €. Sul disco c'è posto: 1,7 TB liberi **[V: `df`]**. Una ISO di alcuni GB e un disco di almeno 64 GB.
- **Licenza.** Windows 11 Enterprise di valutazione, 90 giorni, senza codice prodotto, dopo una registrazione
  sull'Evaluation Center. Microsoft la descrive per i professionisti IT che valutano Windows per la loro
  organizzazione. Alla scadenza lo sfondo diventa nero e il PC si spegne ogni ora **[V: Microsoft Evaluation
  Center]**. Che provare un proprio programma rientri nella valutazione **non è scritto**: una risposta su Microsoft
  Q&A dice che la documentazione pubblica non basta a confermarlo **[V]**. Valgono i termini mostrati al download,
  che non ho letto perché chiedono la registrazione. La registrazione è a nome di chi scarica: decide Peppe (Q59).
- **Limiti.**
  - Windows 11 vuole TPM 2.0 e Secure Boot (requisiti noti, non riletti oggi). Il firmware con Secure Boot c'è
    **[V il 28/11: `/usr/share/OVMF/OVMF_CODE_4M.secboot.fd` e `OVMF_VARS_4M.ms.fd`]**; il TPM emulato no: `swtpm`
    non è installato **[V il 28/11: `which`]**, e lo installa Peppe con `sudo apt install swtpm swtpm-tools` (Q59).
    L'alternativa è saltare il controllo nell'installazione, che Microsoft non supporta.
  - La pagina dice che la 26H2 chiede un account Microsoft per entrare **[V]**; la LTSC 2024 è l'altra ISO offerta.
    Quale somiglia di più ai PC dei colleghi lo sa Peppe (Q59).
  - Disco meccanico **[V: `lsblk`, `sdb`, `ROTA=1`]**, quindi lenta: le VM Linux ci girano, ma lì il primo avvio a
    freddo della finestra aveva dato 4,3 s e i due dopo 0,4 s **[V: docs/08, G13]**. Windows sarà più lento. La
    virtualizzazione non è annidata: è KVM diretto. Tetto in P1: 8 h per averla installata e comandabile; se
    scade ci si ferma e si torna da Peppe (Q75).
  - La scadenza dei 90 giorni: la VM si rifà dalla ISO. Reinstallare per allungare la valutazione è una domanda di
    licenza, non tecnica.
- **Cosa dà che le altre non danno.** UAC vero e utenti normali, riavvio, uno snapshot a cui tornare, e una via di
  rete verso i server Docker del laboratorio: niente relay su file. Docker sul server è «rootless» **[V il 28/11:
  `docker info`]**, quindi gli indirizzi dei container non si raggiungono dall'host: i server pubblicano le loro
  porte sull'host e la VM le raggiunge dalla rete utente di qemu **[S]**. Così però il server sta sulla stessa
  rete della VM, e la prova R-A vuole un server che non lo sia (Q74, nel piano di P1). Si comanda via SSH (OpenSSH
  è un componente di Windows) e con il monitor di qemu.
- **Compilazione.** L'MSI di riferimento è quello costruito sul runner (`msvc`). Per il giro veloce in VM si
  compila dal server con l'immagine `xbuild` dello spike (`windows-gnu`) **[V: `spikes/windows-lab/xbuild`;
  l'immagine sul server non è ancora costruita]**; le prove finali si fanno sull'MSI scaricato dal runner (Q68).

### 13.2 Runner di GitHub (`windows-latest`)

- **Costo.** 0 €: i runner standard sono gratuiti e senza limite sui repository pubblici **[V: GitHub Docs]**, e
  `zPeppOz/galleria` è pubblico **[V: `gh repo view`]**.
- **Licenza.** Di GitHub.
- **Cos'è.** Windows **Server 2025**, non Windows 11, 4 CPU e 16 GB **[V: GitHub Docs e `runner-images`]**. Ha WiX
  3.14, Edge con il suo driver e Rust **[V: `Windows2025-Readme.md`]**.
- **Cosa ci si prova.** §14.
- **Limiti.** Nessun riavvio, macchina nuova a ogni job, niente UAC (**[S]**: l'utente del runner è amministratore),
  niente Smart App Control né SmartScreen, niente server Linux del laboratorio. Che il driver di WireGuardNT e
  l'MSI di OpenVPN si installino lì è **[S]**.

### 13.3 La macchina di Peppe (Windows 11 Pro for Workstations)

Tre modi diversi di usarla.

| Modo | Costo e licenza | Limiti |
|---|---|---|
| **Windows Sandbox**, come nello spike | 0 €, è una funzione del Windows di Peppe | provata e funziona **[V: docs/05]**, il 10/11, pilotata da WSL su quella macchina; ma utente amministratore senza UAC, niente riavvio, nessuna rete verso il laboratorio (relay su file, 18 ms), VBScript assente, si rifà da capo a ogni avvio, e a volte non parte. Quello spike ruppe una volta l'interoperabilità di WSL **[V: spike §7]**. **Oggi gli agenti girano sul server di casa e non la possono avviare** (`/mnt/c` lì non c'è **[V il 28/11]**): la può usare solo chi lavora sulla macchina di Peppe (Q75) |
| **Installata davvero**, da Peppe | 0 € | è il solo Windows 11 vero con una rete vera e con il profilo di Addway. Ma è la macchina di lavoro di Peppe: un driver, un servizio di sistema e i filtri di rete si installano solo quando la VM è verde, e li installa Peppe a mano con una lista (Q73) |
| **VM Hyper-V** | — | **esclusa**: starebbe sulla macchina di Peppe, fuori da `/mnt/b/vm`, che è il solo posto ammesso per le VM, e fuori dalla portata degli agenti |

### 13.4 Proposta (Q58)

Tre gradini, ognuno per ciò che sa fare:

1. **Runner** per tutto ciò che è automatico e non chiede un tunnel: a ogni push, gratis.
2. **VM in `/mnt/b/vm`** per tunnel, DNS, IPv6, UAC, riavvio e MSI: è il laboratorio della fase 3. Se in 8 h non è
   usabile ci si ferma e si torna da Peppe: la Sandbox dello spike non è alla portata degli agenti (Q75).
3. **La macchina di Peppe**, alla fine, un'ora, da Peppe: il profilo vero.

## 14. Gate e job di CI

Tre job nuovi in `ci.yml`, tutti su `windows-latest`. I 15 job di Linux non cambiano.

| Job | Contenuto | Blocca |
|---|---|---|
| `win-check` | build `msvc` del workspace, `clippy`, prove dei crate che non dipendono dal sistema, prove di pipe, ACL e servizio con `FakeBackend`, archivio DPAPI | sì, da P1 |
| `win-gui` | `pnpm build`, `cargo test -p galleria-gui` (la parte che vale su Windows), fumo ridotto | le prove sì; il fumo no finché non è stabile, come fu per `gui-smoke` |
| `win-msi` | MSI con la validazione ICE, installazione silenziosa, servizio avviato, `galleria status`, disinstallazione, controllo dei residui | sì, da P6 |

I manifesti dei gate restano quelli di `check-gates.py`: un file nuovo `lab/tests/gates-w.tsv`.

**Cosa si può provare su `windows-latest` e cosa no.**

| Si può | Fonte |
|---|---|
| compilare e costruire l'MSI con ICE | **[V: run 42895494397]** |
| prove WebDriver dell'app | **[V: Tauri, esempio di CI con `windows-latest`]** |
| installare MSI e servizio, creare un utente normale e provare che la pipe lo accetta e rifiuta gli altri | **[S]**, primo task di P2 |
| DPAPI, percorsi, istanza unica, avviso di versione con il server finto | **[S]** |

| Non si può, o non si sa | Dove si prova |
|---|---|
| tunnel veri: mancano i server; driver di WireGuardNT e OpenVPN sul runner **[S]** | VM; in CI solo con P8 (Q71) |
| UAC, SmartScreen, Smart App Control: è un Server, con un amministratore | VM per UAC; gli altri due solo su un PC vero |
| riavvio e avvio automatico del servizio | VM |
| la tray di Windows 11 e il lettore di schermo | VM |
| IPv6 globale | VM, con l'indirizzo finto dello spike |

**I gate G della fase 2, su Windows.**

| G | Su Windows | Dove |
|---|---|---|
| G1, G2 | le stesse prove di livello 2 sulla pipe; il riavvio del servizio nel fumo | `win-gui` |
| G3 | anteprima e «niente scritto prima di Importa» al livello 2 e nel fumo; il profilo malevolo che non esegue niente con OpenVPN vero | `win-gui`; VM (W7) |
| G4 | connetti, sostituisci, mai due backend, con `FakeBackend`; con i backend veri | `win-gui`; VM (W2, W3) |
| G5, G6 | archivio DPAPI, «Ricorda», CLI e finestra sullo stesso archivio; OTP vero | `win-check`; VM |
| G7 | nessun comando restituisce un segreto; righe di comando dei processi e log | `win-gui`; VM |
| G8 | i cataloghi, con le chiavi nuove di Windows: la prova è già indipendente dal sistema | `win-gui` |
| G9 | sessione estranea al livello 2; due utenti veri solo a mano | `win-gui`; VM, rifinitura |
| G10 | il modello del menu (già indipendente dal sistema); la tray vera a mano | `win-gui`; VM |
| G11 | le prove `update_*` con la richiesta di Windows | `win-gui` |
| G12 | tastiera: Vitest, su Linux; lettore di schermo | —; VM, rifinitura (W12) |
| G13 | avvio a freddo | VM, rifinitura (W13) |

## 15. Blocchi

Il dettaglio, con i task, i gruppi e le ore, è nel piano. Ogni blocco finisce con qualcosa che si avvia.

| Blocco | Contenuto | Criteri | Ore | Dove ci si può fermare |
|---|---|---|---|---|
| **P1 · Ambiente e fondamenta** | VM, le tre prove di rischio, il workspace che compila su Windows, percorsi | R-A, R-B, R-C | 32 | **si sa se la fase regge**, e la CI compila su Windows |
| **P2 · Servizio e pipe** | trasporto su named pipe con le ACL, servizio, CLI | W1 | 24 | `galleria status` parla con il servizio |
| **P3 · WireGuard e rete** | WireGuardNT, indirizzi, rotte, DNS, filtro WFP, pulizia | W2, W4, W5, W6 | 29 | WireGuard dalla CLI |
| **P4 · OpenVPN** | `plat` per Windows, DNS dal `PUSH_REPLY`, credenziali, profilo malevolo | W3, W4, W5, W7 | 26 | tutti e due i backend dalla CLI |
| **P5 · La finestra** | WebView2, archivio DPAPI, finestre di sistema, istanza unica, tray, fumo ridotto | W8, W9, W11 (avviso) | 43 | uso quotidiano dalla finestra, installata a mano |
| **P6 · MSI e consegna** | MSI, UAC, guida, rilascio senza firma, giro quotidiano, esito | W10 | 27 | **i colleghi lo installano** |
| *Essenziale* | | | **181** | |
| **P7 · Updater e firma** | updater di Tauri, firma Authenticode se decisa | W11 | 22 | aggiornamenti senza scaricare a mano |
| **P8 · Accessibilità, CI di rete, matrice** | Narratore e NVDA, tunnel sul runner, fumo completo, matrice manuale | W12, W13 | 34 | fase 3 chiusa |
| *Rifinitura* | | | **56** | |

## 16. Sicurezza: cosa cambia rispetto a Linux

| Minaccia | Su Linux | Su Windows | Resta aperto |
|---|---|---|---|
| M1, chi parla col demone | gruppo `galleria`, socket in una cartella di root | tutti gli utenti interattivi (D18); il client controlla il proprietario della pipe (Q70) | la platea è più larga che su Linux: scelta di D18 |
| M2, profilo malevolo | direttive tolte, demone che rifiuta | uguale; `openvpn.exe` gira come SYSTEM **[V: spike 2.1]**, quindi un errore qui costa di più | — |
| M4, rete ostile | rotte `unreachable`, DNS prima di `connected` | filtro WFP, DNS prima di `connected`; fughe di DNS sulle altre schede | limite dichiarato (§5) |
| M5, management di OpenVPN | socket Unix con utente limitato | porta TCP locale, password su `stdin`, controllo del proprietario della porta | — |
| M6, distribuzione | checksum | MSI senza firma con hash pubblicati; l'updater ha la sua firma | senza Authenticode chi scarica non ha modo di verificare l'editore |
| M7, log | filtro dei segreti | uguale; in più il registro eventi, dove vanno solo gli errori d'avvio | — |

## 17. Punti [S] e task che li verifica

| # | Supposizione | Task | Se è falsa |
|---|---|---|---|
| S61 | R-A, R-B, R-C (§7) | P1-T3 | §7 |
| S62 | La VM Windows 11 gira sotto KVM sul server di casa, su `/mnt/b/vm`, e si comanda via SSH; dalla rete utente di qemu raggiunge le porte che i server del laboratorio pubblicano sull'host | P1-T1, T2 | ci si ferma e si torna da Peppe (Q75): la Sandbox con il relay non è alla portata degli agenti |
| S63 | Voce INTERACTIVE senza il diritto di creare istanze; un utente normale entra, un logon di rete no | P2-T1, T2 | gruppo locale dedicato, che D18 prevede come ripiego |
| S64 | Il proprietario di una pipe creata da LocalSystem si legge come SYSTEM o Administrators | P2-T1 | controllo dell'identità del processo servente |
| S65 | Sul runner si installano MSI e servizio e si crea un utente normale per la prova delle ACL | P2-T2 | la prova delle ACL solo in VM |
| S66 | `netsh` basta per indirizzi, rotte e DNS; l'adattatore di WireGuardNT sparisce anche alla morte del processo | P3-T2, T4 | interfaccia di programmazione di Windows al posto dei comandi (+6 h) |
| S67 | Il «job» di Windows chiude `openvpn.exe` con il servizio | P4-T1 | pulizia dal giornale all'avvio |
| S68 | La tabella delle porte dice il processo proprietario prima che la password parta | P4-T1 | resta la sola password, limite scritto |
| S69 | `pull-filter` evita il doppio DNS; il nome dell'adattatore di OpenVPN si ricava da `stdout` | P4-T2 | il DNS di `dhcp-option` lo lascia OpenVPN, noi mettiamo solo quello di `dns` |
| S70 | `oklch()` e il resto dell'aspetto su WebView2 | P5-T7 | correzioni ai token |
| S71 | Il fumo con `tauri-driver` e `msedgedriver` gira sul runner per la nostra app | P5-T6, tetto 5 h | lista di controllo a mano in VM |
| S72 | L'MSI di Tauri scrive l'associazione dei `.ovpn`; Windows 11 ha già WebView2 | P6-T1 | voci di registro nel frammento WiX; installatore di WebView2 nel pacchetto |
| S73 | Updater in modo `passive`: la richiesta UAC compare; il caso dell'utente non amministratore | P7-T1 | resta l'avviso (D16, punto 3) |
| S74 | WebView2 fa dire le regioni vive a Narratore e NVDA | P8-T1 | annuncio via UIA |
| S75 | Driver di WireGuardNT, OpenVPN e un server del laboratorio sul runner | P8-T2, tetto 3 h | i tunnel restano solo in VM |

Le supposizioni che riguardano solo l'esecuzione di P1 (S76–S87: installazione non presidiata, TPM, OpenSSH,
snapshot, rete utente, cattura, compilazione incrociata) stanno nel piano di dettaglio di P1, con i loro ripieghi.

## 18. Questioni aperte nuove

La numerazione continua Q12–Q57. Ognuna ha la mia raccomandazione, che vale come predefinito se nessuno decide prima
del blocco indicato. **Q58–Q61, Q67 e Q73 chiedono una risposta di Peppe; le altre sono tecniche.**

**Decise, e non si riaprono:** Q58, Q59 e Q60, approvate da Peppe il 2026-11-27; Q68, adottata come predefinito da
Aitano il 2026-11-27. Le altre restano aperte e, con Q60 = (a), non servono prima che Peppe decida su P2. Le
questioni nate scrivendo il piano di P1 (Q74–Q80) stanno in quel piano: **Q74 e Q75 chiedono una risposta di Peppe**.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q58 | Dove si prova Windows | (a) runner, VM di valutazione in `/mnt/b/vm`, macchina di Peppe alla fine · (b) runner e Sandbox, come lo spike · (c) solo runner | **(a)**: è la sola che prova UAC, riavvio e tunnel senza toccare la macchina di lavoro. (c) non prova nessun tunnel. (b) era il ripiego se la VM non regge: per gli agenti non lo è più (Q75). **Approvata da Peppe il 2026-11-27: (a)**; la macchina di Peppe solo alla fine, non in P1 | prima di P1 |
| Q59 | La ISO e il TPM | chi si registra e scarica; 26H2 Enterprise o LTSC 2024; `swtpm` o saltare il controllo | **Si registra Peppe**, perché la licenza è a suo nome e la lettura dei termini è sua; **26H2 Enterprise x64**, la più vicina ai PC dei colleghi; **`sudo apt install swtpm`**. **Approvata da Peppe il 2026-11-27:** la ISO di Windows 11 Enterprise 26H2 x64 di valutazione la scarica Peppe e la mette in `/mnt/b/vm`; `swtpm` e `swtpm-tools` li installa Peppe con `sudo`, perché gli agenti non lo hanno | prima di P1 |
| Q60 | Fin dove si fa | (a) solo P1, poi si decide · (b) essenziale P1–P6 · (c) tutto | **(a), poi (b)**: 32 h dicono se i rischi 1 e 2 sono veri. La rifinitura si decide quando i colleghi lo usano. **Approvata da Peppe il 2026-11-27: (a)**, solo P1; dopo P1 decide Peppe, e su P2+ non si lavora | subito |
| Q61 | Firma Authenticode | (a) nessuna · (b) Artifact Signing con Addway · (c) certificato di una CA · (d) SignPath, in fase 4 | **(a)** in fase 3. Diventa **(b)** se i PC dei colleghi bloccano i non firmati | prima di P6 |
| Q62 | Aggiornamenti | (a) avviso nell'essenziale, updater nella rifinitura · (b) updater subito, come dice D16 alla lettera | **(a)**: l'avviso c'è già; l'updater porta chiave, flusso di rilascio e il caso UAC ancora da provare. D16 resta la destinazione | P5 |
| Q63 | Archivio dei segreti | (a) file cifrato con DPAPI · (b) Credential Manager, con le chiavi lunghe spezzate · (c) tutti e due a strati, come su Linux | **(a)**: un solo archivio, senza il limite di 2560 byte e senza i casi limite di D25 | P5 |
| Q64 | Crate per WireGuard su Windows | (a) `wireguard-nt` 0.5, provato nello spike · (b) `defguard_wireguard_rs`, che usiamo su Linux, se copre Windows | **(a)**: è quello con le prove. Non ho verificato cosa faccia (b) su Windows | P3 |
| Q65 | Rete su Windows | (a) comandi `netsh` nel piano di `galleria-net` · (b) interfaccia di programmazione di Windows | **(a)**: riusa piano, giornale e prove. (b) se `netsh` non basta (S66) | P3 |
| Q66 | `--up` di OpenVPN su Windows | (a) nessun programma `--up`: si decide su `PUSH_REPLY` e `CONNECTED` · (b) portare `hook.rs` | **(a)**: su Windows quel canale non porta il DNS e la riga di comando ha un problema aperto | P4 |
| Q67 | Chi installa | MSI per macchina, con un amministratore a ogni installazione e aggiornamento | **Si accetta**: un servizio non si installa altrimenti. Serve sapere se i colleghi sono amministratori; se no, la guida è per chi lo è | prima di P6 |
| Q68 | Toolchain | (a) `msvc` sul runner per ciò che si consegna, `windows-gnu` dal server di casa per il giro in VM · (b) solo `msvc`, con gli strumenti di Visual Studio nella VM | **(a)**: niente strumenti di Microsoft da installare e da licenziare nella VM. Le prove finali si fanno sull'MSI del runner. **Adottata come predefinito da Aitano il 2026-11-27: (a)** | P1 |
| Q69 | Se il filtro WFP via FFI sfora il tetto | (a) la regola `advfirewall` dello spike, tolta dal giornale · (b) insistere | **(a)**, con il limite scritto | P3 |
| Q70 | Il client controlla il proprietario della pipe | (a) sì · (b) no | **(a)**: senza, a servizio fermo un programma dell'utente riceve chiave e password | P2 |
| Q71 | Tunnel veri in CI su Windows | (a) no nell'essenziale, tentativo in P8 con un tetto · (b) subito | **(a)**: tre supposizioni in fila (driver, OpenVPN, un server sul runner) | P8 |
| Q72 | Lettore di schermo su Windows | (a) rifinitura, dal registro del parlato di NVDA, non blocca · (b) nell'essenziale | **(a)**, come Q56 | P8 |
| Q73 | La macchina di Peppe | (a) solo Peppe, a mano, alla fine, con una lista · (b) anche gli agenti · (c) mai | **(a)**: è la sola prova con il profilo vero, e un errore lì ferma il lavoro di Peppe | P6 |

## 19. Cosa non ho verificato

- **Niente è stato eseguito su Windows per questo documento.** Tutto ciò che riguarda Windows viene da `docs/05`
  (del 10/11) e dalla documentazione letta oggi.
- I termini di licenza della ISO di valutazione, e la sua grandezza.
- Che una VM Windows 11 giri sotto KVM sul server di casa. Il 28/11 ho solo guardato l'host: `/dev/kvm`,
  `qemu-system-x86_64` 11.0.3, `virt-install`, il firmware con Secure Boot, 31 GB di RAM, 16 core, 1,7 TB liberi su
  `/mnt/b`, Docker «rootless»; `swtpm` manca. Nessuna VM è stata creata o accesa.
- Che la rete utente di qemu porti la VM fino alle porte pubblicate dai container di Docker «rootless» (S62).
- La pagina dei prezzi di Azure Artifact Signing: il prezzo viene da due fonti secondarie.
- La voce di ACL per INTERACTIVE, il bit del diritto di creare istanze, il proprietario della pipe, la tabella delle
  porte con il processo, il «job» di Windows: li conosco, non li ho riletti né provati (§17).
- `defguard_wireguard_rs` su Windows; l'updater di Tauri con UAC; le regioni vive su WebView2.
- Cosa c'è sui PC dei colleghi: edizione di Windows, diritti di amministratore, Smart App Control, antivirus.
- Le ore: nessuna fase ha registrato le ore vere (`docs/08`, «Numeri»).

## Fonti esterne lette il 2026-11-26

- Microsoft Evaluation Center, «Windows 11 Enterprise»: `microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise`
- Microsoft Q&A, «Windows 11 Enterprise Evaluation License for Hyper-V Software Testing»
- GitHub Docs, «GitHub-hosted runners reference» e «Actions runner pricing»
- `actions/runner-images`, `images/windows/Windows2025-Readme.md` (immagine 20261116.335.1)
- Tauri: «WebDriver», «WebDriver · Manual setup», «WebDriver · Continuous Integration», «Updater», «Windows Installer», «System Tray»; docs.rs, `tauri::tray::TrayIconEvent` 2.12.7
- docs.rs, `tokio::net::windows::named_pipe::ServerOptions`
- Microsoft Learn: `CreateNamedPipeW`, `CREDENTIALW`, `CredWriteW`, «Object Management» di WFP, `FWPM_SESSION0`, `FwpmEngineOpen0`
- Microsoft Learn, Artifact Signing: «FAQ» e «Quickstart»; my-ssl.com (14/11/2026) e licenseseat.com (17/11/2026) per i prezzi
- signpath.org, «Terms»
- Windows IT Pro Blog, «VBScript deprecation: Timelines and next steps»; Microsoft 365 Developer Blog (9/9/2025)
- Smart App Control: nota di supporto Autodesk (16/4/2026), `cryptomator/cryptomator#3130`, `LizardByte/Sunshine#5903`
