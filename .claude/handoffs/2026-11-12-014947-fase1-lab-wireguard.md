# Handoff — fase 1, laboratorio e WireGuard (Task 7–9)

## Session Metadata

- Data: 2026-11-12, Europe/Rome.
- Worktree: /home/whoami/.worktrees/galleria/galleria-fase1-t7-9.
- Branch: aitano/galleria-fase1-t7-9, base 115d77c113ea8009d70e887ac7e7a03eadd98778.
- Continua da [handoff Task 5–6](./2026-11-12-004434-fase1-review-task5-6.md).
- Le verifiche riguardano questo worktree; nessun checkout principale modificato.

## Current State Summary

Task 7, 8 e 9 implementati e provati nel laboratorio Docker. Correzioni preliminari della review
precedente nel primo commit; due varianti DNS emerse nelle prove reali sono state chiuse nel T9.
Review indipendente conclusa in un solo giro, con un passaggio di correzione e regressioni rosso→verde.
Le correzioni finali sono incluse nel commitT9 (amend di c6ba6d8, sostituito da9db3fda); un commit per task.
Nessuna attività remota di pubblicazione: solo commit locali. Prossimo blocco Task 10–12.

## Important Context

Giuseppe non può rispondere durante l'esecuzione. Solo questo worktree è scrivibile, eccetto le cache
Cargo registry/git e i dati Docker espressamente autorizzati. Niente push, PR, merge, deploy, VM,
Windows, Cloudflare, database remoti o messaggi. Spec e piano sono approvati; prevale la spec e
non si riaprono le decisioni D/Q. Non eseguire galleriad direttamente sull'host: recover usa il runner
reale e cancella interfacce/regole riservate. Gli esperimenti di rete vanno nei container glab-*.

Toolchain globale 1.96.0; niente copie di rustup. `source target/local/env.sh` configura PATH,
CARGO_TARGET_DIR, CARGO_HOME e TMPDIR nel worktree. Solo registry/git puntano alle cache consentite;
advisory-db e contabilità di Cargo restano locali. Dipendenze scaricate online, mai --offline.
Il laboratorio predefinito monta la toolchain globale in sola lettura e compila in Ubuntu 24.04;
`LAB_BUILD=host` evita un secondo target e riusa il build dell'host Ubuntu 24.04.

## Work Completed

| Ambito | Commit | Stato |
| --- | --- | --- |
| Review precedente | 6e9df06 | C1 e I1/I2/I3 pertinenti corretti, cinque fallimenti reali prima dei fix |
| T7 | e53755f | immagini, rete, server, PKI/profili, harness e2e, installazione e unit, job CI lab |
| T8 | bc90ab7 | validazione, adapter kernel, sessione, factory e discovery WireGuard |
| T9 | 9db3fda | full tunnel, blocco IPv6, crash recovery, regressioni DNS/teardown/WSL |

T7: nove test sul client26, ripetuti sui client27 e79, tutti verdi. Provato anche il build predefinito
in container, sempre nove verdi sul26. Utenti alice/mallory, socket root:galleria0660, LimitCORE0
anche in /proc, hardening, restart e mascheramento binfmt verificati dal vivo. Nessuna restrizione
rimossa dalla unit del prodotto. I client Docker usano NET_ADMIN, NET_RAW, SYS_ADMIN, namespace cgroup
privato, tmpfs e seccomp/AppArmor del container disattivati per permettere le sandbox di systemd.
Nessun client con --privileged. Il solo comando di riparazione WSL mantiene --privileged come autorizzato.

T8: 25 test unitari del backend e otto e2e sul26. Endpoint risolti prima di creare il dispositivo,
scadenza5s, IPv6 letterale accettato; NetUp confermato prima di Up; statistiche ogni secondo;
connected non aspetta handshake; un solo avviso a10s senza handshake/keepalive.
DNS applicato prima di connected, ping/HTTP nel tunnel, contatori e handshake, replace e pulizia verificati.

T9: nove prove del piano più regressioni su resolved mascherato e resolved sospeso.
Policy routing, fwmark, unreachable, LAN IPv6, errore immediato, ripiego sequenziale, residue uguali,
giornale e cartelle orfane dopo SIGKILL verificati sul demone vero sotto systemd.
Le prove a zero pacchetti hanno un controllo che deve vedere traffico, e tcpdump deve restare attivo
per tutta la finestra: un errore di cattura non può valere come zero.

## Architecture Overview

Il Manager resta proprietario della sessione e della rete. NetManager mantiene piano, journal e rollback;
errori diagnostici arrivano al logger centrale con sorgente Net, redatti prima di buffer/stdout/eventi,
mentre le risposte IPC mantengono i codici e i testi generici. Lo stato PEM è separato per sorgente.

WireGuard ha il trait WgDevice di quattro operazioni. Solo device.rs importa defguard0.12.5.
Creazione/configurazione/lettura usano defguard; rimozione tramite rtnetlink diretto con scadenza2s,
non tramite remove_interface di defguard. La libreria chiamava helper DNS senza timeout e poteva
fermare la rimozione se resolved mancava o non rispondeva. Il demone possiede già DNS e regole:
la rimozione del dispositivo cancella solo glr-wg0. Nessuna shell, chiave in argv o file privato nuovo.
Le copie delle chiavi nei nostri tipi usano Zeroizing; la stringa temporanea del config defguard è azzerata.

## Critical Files

- crates/galleria-backend-wireguard/src/device.rs: adapter, chiavi, statistica primo peer, cancellazione bounded.
- crates/galleria-backend-wireguard/src/session.rs: risoluzione, NetUp/Up, timer, Stop.
- crates/galleria-backend-wireguard/src/validate.rs: errori invalid_config con data.field, NetConfig.
- crates/galleriad/src/factory.rs: wiring reale; OpenVPN ancora backend_unavailable fino al T12.
- crates/galleria-net/src/runner.rs: rotta già presente accettata solo con verifica dell'interfaccia.
- crates/galleria-net/src/manager.rs: rollback/recover senza resolved o famiglia IPv6.
- crates/galleria-e2e/tests/t07_daemon.rs, t08_wireguard.rs, t09_wireguard_full.rs: prove dal vivo.
- lab/run.sh, up.sh, down.sh, check-wsl.sh e tests/check-wsl.sh: lifecycle e protezione WSL.
- packaging/linux/galleriad.service e install.sh: installazione solo test/VM autorizzata.

## Files Modified

Nuovi crate galleria-backend-wireguard e galleria-e2e, laboratorio lab e packaging/linux;
modifiche a galleria-net, galleriad, Cargo.lock, .gitignore e ci.yml. Nessuna modifica a spikes,
docs00–05, design, workflow spike, checkout principale o altri worktree.

## Prove rosso→verde

- C1: fake runner fallisce ogni resolvectl, sia assenza binario sia bus fermo: due assert sul journal residuo
  rossi (exit101), verdi tollerando solo indisponibilità nota durante undo. Fallire apply resta dns_unavailable.
- I1: mancano comando/stderr nei logs (exit101); ora arrivano redatti e lo stato mantiene testo generico.
- I2: recover con EAFNOSUPPORT IPv6 fallisce (exit101); undo/recover tollerano famiglia assente, apply no.
  Prova reale aggiuntiva: container glab-ipv6-disabled con sysctl di lancio disable_ipv6=1, recover e hello exit0.
  Un tentativo iniziale di impostarlo nel client già avviato è fallito perché /proc/sys è read-only; non è stato
  interpretato come prova riuscita. Nessun tentativo di cambiare impostazioni IPv6 dell'host.
- I3: un BEGIN PEM backend senza END nasconde uid42 disconnect di Daemon (exit101); sorgenti separate verdi.
- T7: lab/run.sh inizialmente assente, exit127. Primo boot reale fallito sul mount cgroup già occupato;
  unmount/remount nel namespace privato risolve. Nove test verdi poi su tutti i client.
- T8 unit: compile rosso per API mancanti; dopo implementazione25 verdi. E2e funzionale con main precedente:
  otto fallimenti su disponibilità/No backend available; ripristinata factory reale e corretti i difetti reali.
  Due errori dell'harness incontrati e risolti: disconnect restituisce {status}, e fixture in /lab/lab/out.
- T8 reale: rotta di subnet già creata dal kernel dà File exists; regressione runner rossa e poi verde.
  Non si ignora File exists in generale: query exact nella tabella corretta, verifica CIDR e dev glr-wg0.
  iproute2 omette dev se si filtra per dev: caratterizzato nel container e nel fake, query corretta senza filtro.
- T9 prima suite del piano9/9 già verde. Mutazione intenzionale che salta unreachable: ipv6_connect_fails_immediately
  diventa rosso perché connect riesce; mutazione completamente rimossa prima dei gate.
- T9 DNS mascherato: due nuove varianti stderr di systemd riproducono journal residuo (exit101); corrette.
  Regressione e2e rossa prima sul codice NetSetupFailed, poi sull'interfaccia rimasta; infine verde.
- Review finale: test no-DNS con SIGSTOP di resolved scade a4s (exit101); usando solo cancellazione netlink
  termina e può connettere di nuovo. SIGCONT garantito dal guard anche se il test fallisce.
- Review finale WSL: test controllato salta riparazione su release microsoft-standard-WSL2 (exit1), poi verde
  con grep -E. Casi registrazione assente, disabilitata, abilitata e kernel non WSL; Docker è uno stub,
  nessuna reale cancellazione/registrazione di WSLInterop durante questi test.

## Verifiche eseguite e risultati

I sei gate a ogni commit di codice: fmt, clippy --workspace --all-targets -- -D warnings,
cargo test --workspace, cargo deny check, scripts/check-boundaries.sh, git diff --check.
Tutti exit0 prima dei commit. Typecheck cargo check --workspace --all-targets --all-features exit0.

| Albero | Test Rust passati | Falliti | Ignorati intenzionali |
| --- | --- | --- | --- |
| Review preliminare | 180 | 0 | 0 |
| T7 | 180 | 0 | 9 |
| T8 | 206 | 0 | 17 |
| T9 prima review finale | 208 | 0 | 27 |
| Finale dopo correzioni | 208 | 0 | 28 |

I test ignorati sono e2e: vengono eseguiti nei container, non saltati dal laboratorio.
T7 client26/27/79:9/9 ciascuno; build container26:9/9. Sequenza obbligatoria t08 && t09 già exit0 prima
review (8/8 +10/10), ripetuta dopo il passaggio di correzione con11 testT9.
Misure prima review: controllo1680 pacchetti globali in15s, secondo controllo1680, protezione0;
client sequenziale prima IPv6 in0.000119s, poi IPv4 in0.000446s con blocco.
Risultati finali: t08 && t09 exit0,8/8 e11/11; controlli1680/1680, protezione0;
sequenziale IPv6 in0.000130s e IPv4 in0.000389s. Sei gate final-code exit0, typecheck finale exit0,
lab/tests/check-wsl.sh exit0, sintassi bash di tutti gli script exit0.

cargo-deny: quattro sezioni ok, nessun errore. Sette warning non bloccanti: cinque tipi di duplicati
(getrandom, syn, thiserror, thiserror-impl, windows-sys) e due licenze ammesse non usate (MPL-2.0, Zlib).
Syn duplicato era preesistente; gli altri duplicati vengono dal grafo della dipendenza WireGuard, comprese
le dipendenze Windows. Nessun allow di vulnerabilità/licenze o modifica di deny.toml.
Output completo ignorato in target/local/*-gates.log, *-0..5.log, final-t08.log, final-t09.log,
review-*-red/green.log, t07-client27.log, t07-client79.log, t07-container-build.log.

## Review indipendente finale

Un solo reviewer gpt-6-astra/high, sola lettura, range115d77c..c6ba6d8 prima dei fix finali.
Nessun Critical, un Important (attesa DNS non limitata nella rimozione) e un Minor (regex WSL).
Entrambi corretti nello stesso passaggio, con rossi reali prima delle modifiche. Il Minor WSL è stato
promosso a importante operativamente: la riparazione è una protezione esplicitamente richiesta dall'utente.
Il reviewer non ha eseguito i gate o Docker; ha verificato il diff, le API locali fissate e la regex con stdin.
Non c'è stato un secondo giro di review dei fix. Gate e prove finali sono dell'implementatore.

Ambiti messi da parte dal reviewer: T10 .ovpn CRLF/BOM/inline e T15 nomi profilo → ai rispettivi task;
CI remota → non autorizzata; NetworkManager/DCO/Windows/prestazioni → scope successivo/manuale;
coabitazione D15 e topologie IPv6 più ampie → prescrizioni della spec, non riaperte; nuove release →
nessuna affermazione su versioni correnti, API verificate sul sorgente fissato nel lockfile.

## Decisions Made

- S1: la parte di rimozione di defguard è smentita con DNS indisponibile e sospeso. Ripiego minimo dietro
  WgDevice: cancellazione rtnetlink diretta,2s, idempotente anche se link assente; niente helper DNS in remove.
  Creazione/configurazione/statistiche restano su defguard. Il reviewer conferma che è il ripiego appropriato.
- fwmark anche con default solo IPv6: prevale spec7.3 sul testo abbreviato del testT8 che cita solo IPv4.
- Rotta di subnet kernel identica: accettata solo dopo verifica exact+dev; non si sostituiscono rotte altrui.
- Modprobe locale saltato perché modulo già caricato e istruzione esplicita; nel job CI resta sudo modprobe.
- Build container con toolchain globale montata read-only, anziché installare/copiare rustup nel worktree.
- Nessun menu merge/push: la scelta di mantenere branch e worktree locali è già imposta dalla delega.

## Assumptions Made — punti [S]

- S1: creazione (link già UP), configurazione e read confermati nel kernel; remove sostituito come sopra.
- S2: systemd e modulo WireGuard funzionano in Docker locale. Parte GitHub runner non verificata: niente push.
- S3: policy routing wg-quick IPv4 installato/rimosso, provato. Policy IPv6 presente nei test unitari;
  non si rivendica una prova dal vivo di un tunnel full-IPv6, che il profiloT9 non porta.
- S4: confermato WireGuard con unreachable: zero pacchetti globali, errore immediato e LAN intatta;
  controllo positivo in ogni prova che dipende dal blocco. Nessuna modifica a D15.
- S5: chiusa la parte WireGuard, con journal e cartelle sessione presenti prima di SIGKILL e assenti dopo restart.
- S6: unit compatibile col demone e WireGuard senza togliere restrizioni. OpenVPN resta al T12.
- Altri [S]: rimangono dove li colloca il piano; NetworkManager reale non è simulato dal laboratorio.

## Pending Work

## Immediate Next Steps

1. Continuare con Task10: parser .ovpn, classificazione/lista ammessa e test nominati nel piano.
2. Task11: parti pure del backend OpenVPN, righe reali management e push/hookenv.
3. Task12: processo/helper/factory, e2e sui tre client. Usare server e profili già pronti nel nuovo lab.

### Cosa NON è stato fatto

Task10–16, CLI/profili persistenti, prove su desktop/VM/Windows, NetworkManager, DCO e prestazioni.
Job lab in ci.yml scritto per matrice26/27/79 ma NON eseguito su GitHub: senza push non è possibile.
Nessun push, PR, merge, deploy, registry publish, servizio remoto o messaggio esterno.
Nessuna modifica a rotte, DNS, pacchetti, unit o impostazioni dell'host. ModuloWireGuard host non toccato.
Nessuna riparazione WSL reale è stata necessaria: WSLInterop è rimasto enabled dopo ogni run.

## Potential Gotchas — rischi e rilievi differiti

- Report precedente riporta8 Minori senza dettaglio: review.md/review.log non presenti nella delega;
  gli altri limiti già elencati nell'handoffT5–6 restano differiti, senza un ulteriore giro di modifiche.
- Pulizia fissa fwmark51820 può interferire con wg-quick già attivo: limite della spec preesistente,
  non dimostrato innocuo né risolto da questo blocco. Eseguire solo nel laboratorio su host condiviso.
- Retry opzionale accept, client lenti dopo hello, stato since nullo, supervisione del Manager e altri limiti
  minori delle reviewT3–6 restano al seguito; non si rivendica di averli corretti tutti.
- PKI di laboratorio riutilizzata finché presente, certificati30giorni: per rigenerarla usare container
  Galleria e cancellare solo i dati ignorati del laboratorio, non chiavi/configurazioni di altri progetti.
- Immagini client27/79 seguono i pacchetti correnti; le tre immagini qui costruite sono riusate per le prove.
  Per aggiornarle LAB_REBUILD=1, ricordando il minimo8GiB liberi.
- Le operazioni netlink upstream di create/configure/read rimangono sincrone sul worker; non si rivendica
  un timeout generale del kernel netlink. La rimozione è invece bounded e non chiama subprocess DNS.

## Environment State

Prove e ledger ignorati in target/local; immagini glab-* conservate per riuso. PKI e profili sintetici
solo in lab/pki e lab/out (ignorati, chiavi0600). Nessun segreto reale, nessun file .env letto o stampato.
Nessun aggiornamento delle memorie. A fine blocco lab/down.sh e controllo WSLInterop, container glab-*
assenti, rete galleria-fase1 assente, WSLInterop enabled. Ultimo controllo disco:20GiB liberi,
sempre sopra8GiB. Il commit di consegna include questo handoff e lascia il worktree pulito.

## Related Resources

- [Piano](../../docs/superpowers/plans/2026-11-10-fase1-demone.md)
- [Spec](../../docs/superpowers/specs/2026-11-10-fase1-demone-design.md)
- [Laboratorio](../../lab/README.md)
