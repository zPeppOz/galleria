# 06 — Esito della fase 1: demone Linux e CLI

Data: 2026-11-12. Branch locale `aitano/galleria-fase1-t13-16`, base `872da51`.
Piano e requisiti: [piano](superpowers/plans/2026-11-10-fase1-demone.md),
[spec](superpowers/specs/2026-11-10-fase1-demone-design.md).

## 1. Stato e limiti della prova

T13–T16 completati. I criteri E1–E7 sono verificati localmente sui tre client.
Le prove riportate come locali sono eseguite in Docker su WSL2,
con WireGuard nel kernel e tun già disponibili. Nessun modprobe o cambiamento di rete sull'host.
I job GitHub `check`, `coverage` e `lab` sono stati eseguiti sul run 43707067359 e sono verdi (vedi S2 in §4);
anche `lab-dco` (client 27 e 79) è verde sul run 45625594806 (vedi S2 in §4).
Non sono state usate VM, Windows o NetworkManager desktop. Questi limiti non sono criteri dichiarati chiusi.

## 2. Criteri E1–E7

| Criterio | Prova locale | Stato |
| --- | --- | --- |
| E1, CLI e traffico con entrambi i backend | T16 `cli_wireguard_roundtrip`, `cli_openvpn_with_password_stdin`, T12 ping | Verificato in locale |
| E2, sessione unica e replace | T7/T8/T12 e `cli_connect_while_active_exits_1_and_replace_works` | Verificato in locale |
| E3, OpenVPN 2.6/2.7, password/OTP/passphrase senza file di sessione | T12/T13, stdin della CLI e controllo argv T16 | Verificato in locale |
| E4, DNS prima di connected e dopo restart | T12 test DNS, T13 `internal_restart_removes_and_reapplies_dns_and_ipv6_block` | Verificato in locale |
| E5, IPv6 e ripiego rapido | T9 e T14, controlli con blocco spento, catture di 15 s | Verificato in locale |
| E6, disconnessione e crash senza residui | T9/T12/T14 `residue()`, cartelle di sessione e processi | Verificato in locale |
| E7, autorizzazione, allowlist, segreti | T7/T12/T13/T16: mallory, profilo malevolo, ricerca nei log/argv, ProtectHome e fd | Verificato in locale |

I segreti personali importati sono conservati nell'archivio provvisorio `0600` deciso in Q6.
La configurazione del processo OpenVPN e i segreti della sessione del demone transitano solo in memoria/pipe.

## 3. Verifiche e laboratorio

I sei gate locali sono tutti exit 0: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo deny check`, `scripts/check-boundaries.sh`, `git diff --check`.
Workspace finale: **511 passati, 0 falliti, 81 ignorati** (80 e2e più l'audit dei profili generati).
Deny segnala soltanto allowance di licenze non usate, nessuna advisory bloccante.
Anche i test shell del laboratorio e il controllo di sintassi passano.

| Laboratorio completo, senza filtro | Test contati | Falliti | Prove effettive |
| --- | ---: | ---: | ---: |
| Ubuntu 24.04 / OpenVPN 2.6.19 | 80 | 0 | 79 + DCO saltato |
| Ubuntu 26.04 / OpenVPN 2.7.0 | 80 | 0 | 79 + DCO saltato |
| Fedora 45 / OpenVPN 2.7.9 | 80 | 0 | 79 + DCO saltato |

Ogni giro include T7, T8, T9, T12, T13, T14 e T16. Il codice è rimasto invariato durante
la matrice finale. Le prove CLI includono anche PKCS#12 binario e prompt interattivo.
`lab/down.sh` eseguito: nessun glab-* né rete galleria-fase1, WSLInterop abilitato, 11 GiB liberi.

Copertura locale (`cargo llvm-cov --workspace --summary-only`, exit 0): righe complessive **76,64%**;
IPC **93,90%**, rete **87,97%**, parser OpenVPN **94,04%**, macchina a stati **94,74%**.
L'obiettivo dell'80% per ciascun componente puro della spec è raggiunto.
DCO si salta nei giri ordinari: non è una prova del modulo DCO.
Nessuna restrizione della unit systemd del prodotto è stata rimossa.
Il container osservatore ha SYS_PTRACE per il controllo dei fd del demone non dumpabile.
Docker monta /proc/sys in sola lettura: src_valid_mark=1 viene impostato nel namespace del client alla creazione.
Il runner verifica sempre il valore richiesto dopo sysctl, anche se il comando restituisce successo:
procps 4.0.4 ignora EROFS con uscita 0, procps 4.0.6 restituisce errore anche con valore già 1.
Il primo giro completo Fedora ha esposto questo difetto preesistente; regressione unitaria e ripetizione T09
verdi dopo il fix. Nessuna modifica di sysctl sull'host.

## 4. Punti S1–S16

| Punto | Stato e prova |
| --- | --- |
| S1 | Locale, T8: creazione/configurazione/lettura/rimozione WireGuard dietro WgDevice. |
| S2 | **Chiuso**. Run [43707067359](https://github.com/zPeppOz/galleria/actions/runs/43707067359) (ramo `t_17a7470c`, `e4a2d71`): `check`, `coverage` e `lab` 26/27/79 verdi. `lab-dco` rosso all'installazione di `openvpn-dco-dkms`. Causa provata: il runner è ubuntu-26.04 con kernel `7.0.0-1019-azure`; il pacchetto `0.0+git20251017` (modulo fuori albero `ovpn-dco-v2`) non compila su quel kernel. Ripetuta la compilazione a mano contro gli header del kernel 7.0.0-1019-azure: `tcp.c:482` e `udp.c:346/356` falliscono per tipi incompatibili (il kernel 7.0 ha tolto il parametro `int *addr_len` da `proto.recvmsg` e fa prendere `struct sock *` a `setup_udp_tunnel_sock`; il modulo usa ancora le firme vecchie), cioè API del kernel cambiate. Non è un problema di header mancanti. Il kernel `-azure` ha già il modulo in albero: `CONFIG_OVPN=m` e `ovpn.ko` in `linux-modules-7.0.0-1019-azure`. OpenVPN 2.7 parla solo l'API del modulo `ovpn` (`Changes.rst` 2.7.0), OpenVPN 2.6 solo quella vecchia: il client 26 non può provare il DCO su questo kernel. Rimedio in CI: `modprobe ovpn` e `lab-dco` sui client 27 e 79. Provato su GitHub: run [45625594806](https://github.com/zPeppOz/galleria/actions/runs/45625594806) (`b38215a`), tutto verde, compresi `lab-dco (27)` e `lab-dco (79)`: DCO con il modulo `ovpn` del kernel 7.0 del runner, client OpenVPN 2.7.0 e 2.7.9, `ovpn_dco_state_and_bytecount ... ok` (il test controlla nei log del demone «DCO device … opened»). Il job `lab-dco` ora è bloccante (senza `continue-on-error`). |
| S3 | Locale, T9: tabella 51820, fwmark e regole apply/teardown. |
| S4 | Locale, T9: zero IPv6 fisico e ripiego immediato con WireGuard. |
| S5 | Locale, T9/T12/T14: kill -9, cleanup prima della prima hello. |
| S6 | Locale, T7/T8/T12: unit originale; nessuna restrizione tolta. |
| S7 | Lista verificata in T10 sui sette profili laboratorio e manuale; nessuna garanzia su tutti i profili reali. |
| S8 | Locale, T12–T14: prove OpenVPN del prodotto sui tre client. |
| S9 | Garanzia temporale della sola management smentita nel blocco T12; adottato il ripiego stdout previsto dalla spec. T14 verifica il blocco applicato prima di connected. |
| S10 | Locale, T14: split senza redirect lascia IPv6 fisico raggiungibile e non installa unreachable. |
| S11 | Locale, T14: tabella delle fonti sotto; tunnel IPv6 funzionante, nessuna perdita fisica. |
| S12 | Locale, T13: auth-nocache, reneg-sec 10, soft reset osservato in 25 s, credenziali risposte senza needs_input. |
| S13 | Locale, T13: stesso pid/session con SIGUSR1, cleanup e riapplicazione; ping-restart fallito passa a nuovo processo e richiede un nuovo OTP. Corretto il rilascio del nuovo management-hold. |
| S14 | **Aperto**: DCO non provato, job mai eseguito; checklist VM sotto. |
| S15 | **Aperto**: NetworkManager/resolved su desktop reale; checklist sotto. |
| S16 | **Aperto**: concat 2.7 solo unitario, static challenge SCRV1 provato dal vivo. |

### Fonti S11 al momento di up

| Client | Flag ipv6 nel PUSH_REPLY | route_ipv6_network_N copre il default | Prova |
| --- | --- | --- | --- |
| Ubuntu 24.04 / OpenVPN 2.6.19 | sì | sì | `IPv6 default sources at up: profile=false push=true env=true` |
| Ubuntu 26.04 / OpenVPN 2.7.0 | sì | sì | stessa riga nel test v6 |
| Fedora / OpenVPN 2.7.9 | sì | sì | stessa riga nel test v6 |

Un indirizzo assegnato al tun non basta: T14 prova il profilo con redirect IPv6 filtrato e redirect IPv4 locale.
Il blocco resta, la rete IPv6 interna resta raggiungibile. I fatti del PUSH_REPLY rispettano ora i pull-filter
nel loro ordine effettivo; le regole del demone precedono le accept del profilo.

La review indipendente ha trovato due Importanti, corretti con regressioni prima della consegna:
lettura binaria e base64 per riferimenti PKCS#12, echo disattivato prima dell'output del prompt. Un Minore
resta noto: un fallimento durante il salvataggio di più segreti può lasciare un archivio UUID privato orfano.

## 5. Compatibilità e scelte conservate

T10 ha aggiunto alla lista: allow-pull-fqdn, allow-recursive-routing, client-nat, proto-force, local,
link-mtu, mtu-test, passtos, socket-flags, tcp-nodelay, max-packet-size, session-timeout.
Restano sconosciute: auth-token, auth-token-user, push-peer-info, single-session, mode, mark, replay-persist.
La lista completa e i test sono in `crates/galleria-ovpn`. Gli script dei profili sono sempre rifiutati dal demone.
`http-proxy` con basic/none/ntlm in terza posizione resta rifiutata: upstream interpreta quel parametro come file.
Limite delle righe OpenVPN: 255 byte, anche dopo rendering. La PKI sintetica del laboratorio dura 30 giorni.

## 6. Lista di controllo manuale S15 e S14

Su una VM Ubuntu desktop e su una VM Fedora desktop, dopo autorizzazione separata:

- Compilare i binari e usare `packaging/linux/install.sh <cartella-binari>`; verificare servizio, gruppo,
  socket e accesso come utente membro, rifiuto di un non membro.
- Con NetworkManager e systemd-resolved attivi, connettere ciascun backend: `nmcli device` deve mostrare
  `glr-wg0` e `glr-ovpn0` come non gestite, senza cambiare la connessione fisica.
- Controllare `resolvectl dns/domain`, risoluzione dei nomi interni e applicazione prima di connected;
  ripetere dopo riavvio interno OpenVPN e disconnessione, verificando il ripristino.
- Ripetere catture IPv6 e ripiego IPv4 di T9/T14: full IPv4, split, IPv6 default e solo indirizzo IPv6.
- Provare kill -9 del demone e riavvio: nessun residuo di DNS, regole, rotte o cartelle prima di hello.
- S14: installare il modulo DCO appropriato solo nella VM autorizzata, verificare DCO effettivamente aperto,
  connected e bytecount durante traffico; ripetere disconnessione e cleanup. Non contare un test saltato come verde DCO.

## 7. Questioni per Peppe

Restano aperte Q1 (perimetro/nome delle due CLI e delle fasi), Q2 (criteri Windows rinviati alla fase 3),
Q4 (rimando M1/M2 in D13), Q5 (nessuna esecuzione degli script anche confermata: allineare mockup o progettare
separatamente), Q8 (journald/buffer invece di file di log), Q9 (management Unix protetta dai permessi, password
solo Windows), Q10 (controllo e visibilità condivisi tra membri del gruppo), Q11 (traduzioni nei client).
Nessuna nuova decisione è stata presa. Q3, Q6 e Q7 sono risolte nella spec.

La parte Linux di D15 è già allineata da `556b89e`: un solo blocco IPv6 con rotte unreachable nel modulo comune,
anche per OpenVPN, come nella frase della spec §13. I documenti 00–05 non sono stati modificati in questa delega.
