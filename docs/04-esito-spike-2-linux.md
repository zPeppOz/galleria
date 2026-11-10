# 04 — Esito spike 2 (management interface OpenVPN), parte Linux

Data: 2026-11-10 — autore: Nello (CTO) — branch locale `spike/openvpn-mgmt` (da `main`, non pubblicato).
Definizione dello spike: `03-sessione-nello.md` §3. Codice, laboratorio e prove: `spikes/openvpn-mgmt/`.
Le decisioni D10–D16 restano proposte da ratificare da Peppe: dove lo spike le smentisce lo scrivo qui (§4), non le ho cambiate.

## 1. In sintesi

Su Linux la management interface regge: il demone ha pilotato `openvpn` 2.6 e 2.7 **senza toccare il disco con
segreti né con la configurazione**, con lo stesso binario su tre versioni. Tutti i criteri Linux sono superati,
tranne una parte del 2.4 (DCO, non provabile qui). Due risultati cambiano D15 (IPv6, §4).

## 2. Ambiente

- Host: WSL2, kernel 6.18.54 (senza `CONFIG_OVPN`, nessun modulo ovpn-dco), Docker 28.0.4. Tutto dentro Docker:
  nessuna rotta, DNS o pacchetto dell'host toccato.
- Server: Ubuntu 24.04, OpenVPN 2.6.19, due istanze: `gl-server` spinge `dhcp-option DNS/DOMAIN`, `gl-server-dns`
  spinge la nuova opzione `dns`. Entrambe spingono `redirect-gateway def1` e una rotta (`192.0.2.0/24`), nessun IPv6
  nel tunnel. Autenticazione: certificato + utente/password (`alice`) o password + TOTP (`bob`), via `verify.sh`.
- Client (systemd + systemd-resolved nel container): **Ubuntu 24.04 → 2.6.19**, **Ubuntu 26.04 → 2.7.0**,
  **Fedora 45 → 2.7.9**. La Fedora non era chiesta (bastava una tra 26.04 e Fedora), ma la 26.04 ha la 2.7.0 e la
  differenza IPv6 del §4 andava controllata sull'ultima 2.7.
- Rete dual-stack (`172.30.0.0/24`, `2001:db8:30::/64`) con un "sito" dual-stack fuori dalla LAN
  (`198.51.100.1`, `2001:db8:99::1`) raggiunto via router, come una LAN domestica.
- Demone: `galleriad-spike` (Rust). Riceve su stdin la richiesta che la GUI manderebbe via IPC (profilo + segreti),
  lancia `openvpn --config /dev/stdin` con la configurazione generata, parla solo con la management interface
  (socket Unix in `/run/galleria/s<pid>/`, cartella 0700 root, `--management-client-user root`) e riceve DNS/rotte
  da un programma fisso usato come `--up`/`--down` (`galleriad-spike netcfg`). Eventi del demone in JSON.

## 3. Criteri

Prove in `spikes/openvpn-mgmt/evidence/ovpn26|ovpn27|ovpn79/` (26 = 2.6.19, 27 = 2.7.0, 79 = 2.7.9);
riassunto automatico in `evidence/summary.txt`.

| # | Esito | Prova | Note |
|---|---|---|---|
| 2.1 | **superato** (3/3) | `2.1.txt`, `2.1-control.log`, `2.1-sanitized.log` | Profilo con `up`, `down`, `route-up`, `ipchange`, `plugin` (down-root), `management 0.0.0.0 7505`, `script-security 3`, `status`, `writepid`, `setenv`. Controllo senza pulizia: compaiono 6 file `/tmp/PWNED-*` (prova che il profilo funziona). Con il demone: 11 direttive tolte, nessun file, nessuna porta 7505 (anche nel controllo la 7505 non si apre: la nostra `management`, scritta dopo, vince sulla sua). La pulizia è una lista di direttive vietate: in fase 1 meglio una lista di quelle **ammesse**, perché ogni versione ne aggiunge (la 2.7 ha `dns-updown`) |
| 2.2 | **superato** (3/3) | `2.2.txt` | Utente normale: `Permission denied` sul socket e sulla cartella. Difesa in profondità (socket 777 in una cartella aperta): OpenVPN rifiuta da solo, `client connection rejected -- UID of socket peer (1001) doesn't match required value (0)`. In più: mentre il demone è collegato, un secondo client (anche root) resta in coda senza risposta, OpenVPN serve un client per volta |
| 2.4 | **superato senza DCO; DCO non provato** | `2.4.txt`, `2.4.log` | Con `state on`: `WAIT → AUTH → ASSIGN_IP (10.8.0.2) → ADD_ROUTES → CONNECTED,SUCCESS,10.8.0.2,172.30.0.10`, poi `EXITING`. Con `bytecount 1` un evento al secondo, i byte crescono durante il ping. DCO: il kernel WSL non ha il modulo (`Kernel support for ovpn-dco missing`, `DCO version: N/A`); caricarlo vuol dire cambiare il kernel dell'host, fuori scope. **"anche con DCO attivo" resta aperto** |
| 2.5 | **superato** (3/3) | `2.5.txt`, `2.5-*.log`, log di `gl-server` | Utente/password su `>PASSWORD:Need 'Auth' username/password`; OTP con `static-challenge`: arriva `SC:1,Enter OTP`, il demone risponde `SCRV1:b64(pw):b64(otp)` e il server accetta (`verify.sh: user=bob ok (password + totp ...)`); passphrase su `>PASSWORD:Need 'Private Key' password`. Nella config generata `auth-user-pass` è senza file. OpenVPN scrive la password nel log come `password [...]`. Non provati: challenge dinamico (`CRV1`), `auth-nocache` (vedi §6) |
| 2.6 | **superato con la prima via (pipe)**; provate anche la seconda e la terza | `2.6.txt`, `2.6-pipe.log`, `2.6-extkey.log`, `2.6-tempfile.log` | (a) `--config /dev/stdin`: la riga di comando è solo `openvpn --config /dev/stdin`, i fd sono pipe/socket/tun, la ricerca di righe della chiave privata e di `ta.key` nel filesystem del client (esclusi `/proc`, `/sys`, `/dev` e i mount di prova con PKI, profili e prove) trova **0 file**. Limite: `SIGHUP` rilegge la configurazione, dalla pipe non c'è più niente e OpenVPN esce (`Options error`); il demone se ne accorge e rilancia con la config che ha in memoria (evento `retry`, poi di nuovo `CONNECTED`). Regola: il demone non manda mai SIGHUP, i riavvii sono SIGUSR1 (provato: `RECONNECTING → CONNECTED`, stesso processo). (b) `--management-external-key`: `<key>` tolta dalla config, il demone firma su `>PK_SIGN:...,ECDSA` (digest di 32 byte) e la connessione sale: la chiave non entra nemmeno nella memoria di OpenVPN. Provata solo con chiave EC P-256; RSA (padding) no. (c) ripiego file 0600 in `/run/galleria/s<pid>/`, cancellato al primo `>HOLD` (la config è già letta): funziona. Chiude il [S] di `01-analisi.md` §5 **per Linux** |
| 2.7 | **superato** (3/3) | `2.7.txt`, `2.7-*.log`, log dnsmasq dei server | Canale: il nostro programma fisso come `--up`, che passa al demone l'ambiente di OpenVPN; DNS e domini arrivano da `foreign_option_N=dhcp-option DNS/DOMAIN...` su tutte e tre le versioni e per tutte e due le opzioni lato server (`dhcp-option` e `dns`). Il demone applica con `resolvectl dns/domain/default-route`: `resolvectl status tun0` mostra `DNS Servers: 10.8.0.1`, `DNS Domain: galleria.test ~.`; `resolvectl query intranet.galleria.test` risponde via `tun0` e la query arriva al dnsmasq del server. Le rotte invece le mette OpenVPN da solo (`0/1`, `128/1`, `/32` verso il server, `192.0.2.0/24`); il demone le riceve come `route_*` ma non le applica (§4, D10) |
| 2.8 | **superato, con scelta**: `redirect-gateway ipv6` + `block-ipv6` + un `ifconfig-ipv6` locale | `2.8-<modo>.txt`, `2.8-<modo>.pcap` (60 s su `eth0`), `2.8-native*-probe.txt`, `2.8-native-local-rg.txt` | Cinque modi provati su tre versioni, tabella in §4. Senza protezione il sito vede l'IPv6 del client (85 pacchetti IPv6 globali in 60 s). La via nativa da sola funziona sulla 2.6 ma **non sulla 2.7** (nessuna rotta IPv6 installata). La variante con `ifconfig-ipv6 fd15:53b6:dead::2/64 fd15:53b6:dead::1` (l'esempio del manuale di `block-ipv6`) dà 0 pacchetti e ripiego immediato su tutte e tre. Il meccanismo comune di D15 così com'è scritto (scarto silenzioso nel tunnel) non perde pacchetti ma fa aspettare le app senza Happy Eyeballs: 10 s, cioè tutto il timeout del client di prova |
| 2.9 | **superato** (3/3) | `2.9.txt`, `2.9-*.log` | Password sbagliata: `>PASSWORD:Verification Failed: 'Auth'` e `>STATE:...,EXITING,auth-failure` → evento `auth_failed` con "Accesso negato dal server: nome utente, password o codice OTP non validi.", nessun nuovo tentativo, uscita 2. OTP sbagliato: stesso evento (il server non distingue password e OTP, noi nemmeno). Server irraggiungibile: `server-poll-timeout 5`, un processo `openvpn` per tentativo (`connect-retry-max 1`), attesa decisa dal demone 1 → 2 → 4 s, dopo 4 tentativi `gave_up` "Server non raggiungibile dopo 4 tentativi" (27–30 s in tutto) |
| 2.10 | **superato** per 2.6 e 2.7 | i punti sopra su `ovpn26`, `ovpn27`, `ovpn79` | Stesso binario e stesso codice sulle tre versioni. Le uniche cose che dipendono dalla versione sono in §5; nessuna richiede due backend diversi |

Fuori scope come da task: 2.3 e tutta la parte Windows (anche le metà Windows di 2.6, 2.7, 2.8 e 2.10).

## 4. Dove lo spike smentisce o precisa D10–D16

**D15 (blocco IPv6), due smentite.** Risultati su un profilo "tutto nel tunnel" solo IPv4, 60 s di cattura su `eth0`
filtrata su IPv6 non link-local e non multicast; `curl` (che fa Happy Eyeballs) e un client che prova gli indirizzi
uno dopo l'altro (`socket.create_connection` di Python, timeout 10 s):

| Modo | 2.6.19 | 2.7.0 | 2.7.9 | `curl` | client sequenziale |
|---|---|---|---|---|---|
| nessuna protezione | 85 pacchetti | 85 | 86 | via IPv6 (fuga) | via IPv6 (fuga) |
| nativo: `redirect-gateway ipv6` + `block-ipv6` | 0 | **85** | **86** | 2.6: IPv4 in 2 ms; 2.7: fuga | come `curl` |
| nativo + `ifconfig-ipv6` locale | 0 | 0 | 0 | IPv4 in ≤ 3 ms | IPv4 in 1 ms |
| comune D15: `::/1` e `8000::/1` verso il tunnel, scarto silenzioso | 0 | 0 | 0 | IPv4 dopo ~205 ms | **IPv4 dopo 10 s** |
| comune con rotte `unreachable ::/1` e `8000::/1` | 0 | 0 | 0 | IPv4 in ≤ 3 ms | IPv4 in 1 ms |

1. La "via nativa `redirect-gateway ipv6` + `block-ipv6`" di D15 (segnata **[V]** sulla documentazione) sulla 2.7 non
   basta: senza un indirizzo IPv6 sul tun OpenVPN 2.7 avvisa e **non installa** le rotte IPv6, anche con
   `redirect-gateway def1 ipv6` messo nel profilo invece che spinto (`2.8-native-local-rg.txt`). Serve un
   `ifconfig-ipv6` locale, come nell'esempio del manuale.
2. Il meccanismo comune "tutto l'IPv6 verso il tunnel e lì scartato" non soddisfa il criterio di D15 "le app
   ripiegano su IPv4 senza attese lunghe": chi non fa Happy Eyeballs aspetta il proprio timeout (o quello di
   TCP, minuti). Il modulo comune deve rispondere con un **errore immediato**, non scartare. Su Linux funziona con
   rotte `unreachable` (l'ordine di `getaddrinfo` mette perfino l'IPv4 per primo); ma non sono legate al tun e il
   demone le deve togliere da solo, anche dopo un crash. Per WireGuard il [S] "WireGuard scarta i pacchetti fuori da
   `AllowedIPs`" a questo punto conta poco: lo scarto silenzioso è proprio il problema.

Proposta (da decidere con Peppe, non applicata a D15): per OpenVPN il modo "nativo + `ifconfig-ipv6` locale", tutto
gestito da OpenVPN e legato alla vita del tun; il meccanismo comune con errore immediato resta per WireGuard e va
provato su Windows nello spike 1.

**Esito (2026-11-10)**: Peppe ha approvato la proposta, D15 è corretta in `01-analisi.md` §2. Aitano ha ricontato le
catture del 2.8 in modo indipendente, con gli stessi numeri: nessuna protezione 85–86 pacchetti IPv6 globali, nativo
senza `ifconfig-ipv6` 85/86 su 2.7.0/2.7.9 e 0 sulla 2.6, comune 0 su tutte.

**D10 (modulo di rete comune), una precisazione.** Il canale per DNS funziona (`foreign_option_*` è identico su 2.6
e 2.7). Le rotte invece OpenVPN le applica da solo e bene; portarle nel modulo comune richiederebbe `route-noexec`,
non provato. Proposta: per OpenVPN il modulo comune fa DNS e IPv6, le rotte restano a OpenVPN.

**D12, D13, D14, D16**: lo spike non le tocca. **D11**: niente da dire, ma lo schema "processo figlio + programma
fisso come script" ha funzionato senza sorprese ed è lo stesso previsto per OpenConnect.

## 5. Differenze tra 2.6 e 2.7

| Tema | 2.6.19 | 2.7.0 | 2.7.9 | Cosa fa il demone |
|---|---|---|---|---|
| IPv6 nativo senza IPv6 sul tun | rotte installate | nessuna rotta | nessuna rotta | aggiunge sempre `ifconfig-ipv6` locale (§4) |
| DNS senza il nostro `--up` | non fa niente | lancia `/usr/libexec/openvpn/dns-updown` e configura resolved da solo (senza `~.`) | idem | con il nostro `--up` `dns-updown` non parte su nessuna (0 esecuzioni); conviene comunque `dns-updown disable` dalla 2.7 |
| Opzione `dns-updown` | non esiste | esiste | esiste | da aggiungere solo se versione ≥ 2.7 |
| Ambiente di `--up` se il server spinge `dns` | `dns_server_*` **e** `foreign_option_*` | solo `foreign_option_*` | solo `foreign_option_*` | leggere `foreign_option_*` |
| "tutto nel tunnel" visibile in `--up` | no (`redirect_gateway` solo in `--down`) | `route_redirect_gateway_ipv4=1` | idem | sulla 2.6 va ricavato altrove (§6) |
| Versione della management interface | 5 | 5 | 6 | nessuna differenza nei comandi usati |
| `SIGHUP` con config da pipe | esce (`You must define TUN/TAP device`) | idem | esce (`DEPRECATION: No tls-client or tls-server`) | mai SIGHUP; rilancio se succede |
| Config senza TLS (modo a chiave statica `secret` o nessuna cifratura; prova `no-tls.txt`) | accettata con avviso di deprecazione | rifiutata (`Options error: DEPRECATION`, salvo `--allow-deprecated-insecure-static-crypto`) | idem | l'import deve avvisare dei profili senza `client`/`tls-client` |
| Static challenge | `SC:1,...` | `SC:1,...` | `SC:1,...` | la 2.7 aggiunge il formato `concat` (`static-challenge ... concat`): il demone ha un ramo per gestirlo, non provato |

Nessuna differenza da chiudere con due backend: bastano un controllo della versione e due righe condizionali.

## 6. [S] chiusi e aperti

Chiusi (solo Linux; aggiornati in `01-analisi.md`):
- §5, segreti di OpenVPN al connect: "file temporaneo 0600 o passaggio via management interface **[S: spike 2,
  criterio 2.6]**" → configurazione via pipe **[V su Linux]**, segreti via management interface **[V su Linux]**.
- §4, OTP per OpenVPN con `static-challenge`/`auth-user-pass` **[S: copertura da verificare]** → `static-challenge`
  **[V su Linux]**; challenge dinamico non provato.
- `03-sessione-nello.md` §3, criterio 2.6 "configurazione passata via pipe **[S]**" e criterio 2.7 "programma fisso
  come `--up` con le variabili `foreign_option_*` **[S]**": provati su Linux (non ho modificato `03`, il criterio è
  lì come definizione; l'esito è questo documento).

Aperti:
- 2.4 con DCO: serve un kernel con `ovpn` (mainline da 6.16, non abilitato nel kernel WSL) o con `ovpn-dco`: una VM Linux vera.
- Parte Windows di 2.3, 2.6 (la pipe come `--config` quando OpenVPN è figlio di un servizio), 2.7 ("cosa applica OpenVPN
  da solo **[S]**"), 2.8.
- D15: "WireGuard scarta i pacchetti fuori da `AllowedIPs`" **[S]** (spike 1), e il comportamento dell'`ifconfig-ipv6`
  locale con un server che spinge IPv6 vero (non provato: il nostro server non ha IPv6 nel tunnel).
- §8: systemd-resolved con NetworkManager su Ubuntu e Fedora desktop **[S]**: i container non hanno NetworkManager.
- Nuovi, emersi dallo spike, da chiudere in fase 1: `auth-nocache` (OpenVPN avvisa che altrimenti tiene la password
  in memoria; con l'opzione la richiede di nuovo a ogni riconnessione, il demone deve rispondere); come capire "tutto
  nel tunnel" al momento di `--up` sulla 2.6 (per esempio con `--route-up`); `--management-external-key` con chiavi RSA;
  lista delle direttive ammesse al posto di quella delle vietate (2.1).

## 7. Cosa serve da Peppe per la parte Windows

1. **Una VM Windows 11 x64 dual-stack** (dove: Hyper-V su questo portatile, altra macchina?) con l'installer ufficiale
   OpenVPN 2.7.x, e chi la usa: un agente con accesso remoto o Peppe che lancia gli script.
2. **Come la VM raggiunge il server di prova**: il laboratorio gira in Docker dentro WSL; serve la porta UDP 1194
   inoltrata dall'host oppure un server di prova nella VM o su xr1.
3. **Decisione su D15** (§4): OpenVPN con "nativo + `ifconfig-ipv6` locale" e modulo comune con errore immediato.
   Cambia cosa proviamo su Windows (1.5 e 2.8).
4. La mail a SignPath (R3) resta sua: non blocca la parte Windows dello spike 2, blocca lo spike 1.

## 8. Commit (branch `spike/openvpn-mgmt`, non pubblicati)

- `b28f013` add throwaway daemon driving openvpn via management interface
- `b24206b` add docker lab for openvpn management spike
- `7b91743` record spike 2 linux evidence
- `53b6da4` record openvpn behavior without tls
- questo documento e l'aggiornamento dei [S] in `01-analisi.md`: i due commit successivi.
