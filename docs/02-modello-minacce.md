# 02 — Modello delle minacce (prima versione)

Versione 0.1 — 2026-11-09 — stato: bozza da rivedere con Nello. Una pagina, da aggiornare prima della v0.1
e da usare come base per l'audit (R5).

Legenda come in `01-analisi.md`: **[V]** verificato su fonte letta · **[S]** supposto, da verificare.

## Cosa proteggiamo

1. **Segreti**: chiavi private WireGuard, certificati e chiavi OpenVPN, password, OTP, cookie di sessione OpenConnect.
2. **Il sistema**: il demone `galleriad` gira come root / LocalSystem; un suo difetto diventa un'escalation di privilegi.
3. **Il traffico dell'utente**: niente fughe (DNS, IPv6, finestre durante connessione e riconnessione).
4. **La catena di distribuzione**: installer e aggiornamenti.

## Chi attacca e cosa facciamo

| # | Attaccante | Cosa prova a fare | Mitigazioni |
|---|---|---|---|
| M1 | Utente locale non privilegiato o malware nella sessione utente | Usare l'IPC del demone per fare cose da root: lanciare comandi, scrivere file, cambiare rotte, leggere i segreti di altri utenti | IPC solo per il gruppo `galleria` (Linux) / ACL sulla named pipe (Windows); ogni richiesta validata contro uno schema; il demone non accetta percorsi di file arbitrari né comandi; i segreti passano dalla GUI al demone solo per la connessione e non vengono salvati dal demone |
| M2 | Profilo malevolo (`.ovpn` scaricato o ricevuto) | Far eseguire script come root (`up`, `down`, `script-security`, `plugin`), leggere file di sistema tramite percorsi (`ca /etc/shadow`), aprire la management interface verso l'esterno | All'import: direttive pericolose bloccate o confermate esplicitamente (già in §8 dell'analisi); `plugin` e `management` del profilo sempre rimosse; i file referenziati vengono copiati all'import da utente normale, mai letti dal demone |
| M3 | Server VPN malevolo o compromesso | Spingere rotte e DNS per dirottare traffico non destinato alla VPN; con OpenConnect, far eseguire il binario CSD | Mostrare all'utente rotte e DNS ricevuti; opzione per profilo per ignorare le rotte spinte dal server (`pull-filter accept|ignore|reject` in OpenVPN 2.6 **[V]**); CSD mai eseguito come root, solo script di simulazione (`csd-post.sh`) o utente a privilegi minimi, come indica OpenConnect **[V]** |
| M4 | Rete ostile (Wi-Fi pubblico) | Leggere DNS e traffico IPv6 fuori dal tunnel, sfruttare la finestra durante la riconnessione | Blocco IPv6 non instradato (A10, D15) già in v0.1, con errore immediato e non scarto silenzioso; DNS del profilo applicati prima di segnalare "connesso"; kill switch in fase 2 |
| M5 | Altro processo locale | Collegarsi alla management interface di OpenVPN, che controlla il tunnel | Linux: `management <socket> unix` **[V]**; di default il socket accetta qualsiasi processo **[V]**, quindi sempre con `--management-client-user`/`--management-client-group` limitati al demone **[V]**. Windows: la porta TCP locale è l'unica opzione **[V]** (nel sorgente di OpenVPN 2.6 e 2.7 il supporto ai socket Unix è escluso su `_WIN32`, e l'opzione `unix` dà errore) e senza password il manuale la definisce INSECURE **[V]** → password casuale per sessione, passata con `pw-file stdin` per non scriverla su disco **[V]** |
| M6 | Catena di distribuzione | Pubblicare un installer o un aggiornamento falso | Aggiornamenti firmati (Tauri, firma obbligatoria **[V]**); build solo da GitHub Actions; firma Windows SignPath (R3); checksum pubblicati per i pacchetti Linux |
| M7 | Log e segnalazioni | Segreti finiti nei log o nei file esportati per le issue | Filtro dei segreti centralizzato nel demone e nella GUI; test che verificano che chiavi e password note non compaiano nei log |

## Fuori ambito

- Attaccante già root / amministratore sul PC.
- Analisi del traffico cifrato (tempi, volumi) e attacchi ai protocolli VPN in sé.
- Sicurezza dei server VPN di terzi.

## Da fare prima della v0.1

- Test automatici per M1 (chiamante non autorizzato rifiutato), M2 (import di profili con direttive pericolose), M7 (nessun segreto nei log).
- M5 su Windows: il [S] sul socket Unix è chiuso (2026-11-09, [sorgente `syshead.h`](https://github.com/OpenVPN/openvpn/blob/release/2.6/src/openvpn/syshead.h), [`options.c`](https://github.com/OpenVPN/openvpn/blob/release/2.6/src/openvpn/options.c)). Resta da provare in pratica nello spike della management interface: password con `pw-file stdin` quando OpenVPN è figlio del servizio, e rifiuto di un secondo client mentre il demone è collegato **[S]**.
