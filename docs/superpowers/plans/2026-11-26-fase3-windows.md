# Fase 3 — Galleria su Windows: piano a blocchi

Versione 0.1 — 2026-11-26 — stato: **proposta di Nello, per decidere**. Nessun task è partito.

**Spec:** `docs/superpowers/specs/2026-11-26-fase3-windows-design.md` («spec §n»). In testa alla spec c'è il riassunto
di una pagina per Peppe.

Questo è il piano **dei blocchi**: contenuto, gruppi, ore, dove ci si può fermare. Come in fase 2, ogni blocco avrà il
suo piano di dettaglio (un task, un commit, TDD, i passi con le caselle), scritto quando tocca a lui e dopo la
decisione di Peppe. Le ore sono la stessa unità dei piani B1–B6: ore di lavoro stimate per chi esegue. La fase 2 non
ha registrato le ore vere, quindi non c'è un fattore di correzione misurato; il solo dato è che il piano di B6 ha
avuto il doppio dei task stimati dalla spec.

## Prima di partire

- La decisione di Peppe su Q60 (fin dove) e le risposte a Q58 e Q59 (ambiente, ISO, `swtpm`). Senza la ISO, P1 parte
  dal gruppo B e il gruppo A usa la Sandbox.
- I due fatti sui colleghi (amministratori del PC? PC gestiti?) servono prima di P6, non prima.
- Le altre questioni Q62–Q72 hanno la raccomandazione come predefinito.
- Worktree dedicato da `main` per ogni card. Niente push senza l'ok di Peppe.

## Vincoli che valgono per tutti i blocchi

1. **Linux non si rompe.** I 15 job di oggi restano verdi a ogni card. Il codice di Windows entra con `cfg`, accanto
   a quello di Linux, non al suo posto.
2. **L'IPC non cambia**: `PROTOCOL_VERSION` 1, nessun metodo o campo nuovo.
3. **Niente sulla macchina di Peppe.** Tutto ciò che installa driver, servizi o regole gira nella VM o sul runner
   (Q73). In WSL il laboratorio Docker per le prove Windows si avvia senza i client: `CLIENTS= bash lab/up.sh`, per
   l'incidente di `binfmt_misc` dello spike (spike §7).
4. **Le VM solo in `/mnt/b/vm`.**
5. Un tetto di ore su ogni supposizione: quando scade si prende il ripiego scritto nella spec §17 e lo si annota.
6. La review è una per gruppo. Ogni card riporta i comandi verdi sull'ultimo commit e aggiorna
   `docs/09-esito-fase3.md`, che nasce in P1.

## Essenziale: ciò che serve perché i colleghi lo usino ogni giorno

### P1 · Ambiente e fondamenta — 32 h

Chiude le tre supposizioni che decidono la fase (spec §7) e porta il workspace a compilare su Windows.

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · VM e prove di rischio** | T1, T2, T3 | **T1** (8 h, tetto) VM Windows 11 in `/mnt/b/vm`: installazione non presidiata, TPM emulato, OpenSSH, snapshot, `vmctl.sh win11`. **T2** (5 h) rete tra la VM e i server Docker del laboratorio; nello snapshot OpenVPN 2.6 e 2.7, un utente normale e un amministratore. **T3** (6 h) le tre prove R-A, R-B, R-C con il codice di `spikes/`, con le catture | `main`; Q58, Q59 | 19 h | **si sa se la fase regge.** Se una prova cade, si torna da Peppe con il costo nuovo |
| **B · Il workspace compila** | T4, T5 | **T4** (7 h) `cfg` sul codice Unix di tutti i crate, risposte «non supportato» dove manca la parte Windows, job `win-check` con build `msvc`, `clippy` e le prove dei crate indipendenti dal sistema. **T5** (6 h) percorsi (`%APPDATA%`, `%LOCALAPPDATA%`, `%PROGRAMDATA%`) e protezione dei file in `galleria-profile` e in `ui_state.rs` | `main` (non dipende da A) | 13 h | la CI compila e prova su Windows a ogni push |

I gruppi A e B possono andare in parallelo: A tocca solo `lab/vm/` e `/mnt/b/vm`.

### P2 · Servizio e named pipe — 24 h (W1)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · La pipe** | T1, T2 | **T1** (7 h) trasporto dell'IPC su named pipe: server con il descrittore di sicurezza di spec §4, più istanze, client che controlla il proprietario (Q70). **T2** (5 h) identità del chiamante nel log; le prove delle ACL: un utente normale entra, uno non interattivo no, un utente normale non crea istanze | P1-B | 12 h | demone e client si parlano su Windows, con `FakeBackend` |
| **B · Il servizio e la CLI** | T3, T4 | **T3** (6 h) `galleriad` come servizio: avvio, arresto, spegnimento, modo console, cartella in `%PROGRAMDATA%`, errori d'avvio nel registro eventi. **T4** (6 h) la CLI su Windows (pipe, password senza eco, percorsi) e l'attrezzatura delle prove di livello 2 su una pipe con nome casuale | A | 12 h | `galleria status` parla con il servizio vero |

### P3 · WireGuard e rete — 29 h (W2, W4, W5, W6)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Tunnel e rete** | T1, T2 | **T1** (6 h) `WgDevice` con `wireguard-nt`, `wireguard.dll` con l'hash fissato e caricata dal percorso completo. **T2** (8 h) indirizzi, rotte e DNS su Windows nel piano di `galleria-net`, con il giornale; «tutto nel tunnel» verso il server del laboratorio | P2; esito di R-A | 14 h | `galleria connect` alza WireGuard nella VM |
| **B · IPv6 e pulizia** | T3, T4 | **T3** (8 h, di cui 6 di tetto sul filtro) blocco IPv6 con la sessione WFP dinamica, o il ripiego di Q69. **T4** (7 h) pulizia dopo la morte del servizio e dopo il riavvio; le prove `t08` e `t09` che valgono su Windows, eseguite nella VM | A; esito di R-C | 15 h | **WireGuard dalla CLI, con W5 e W6** |

### P4 · OpenVPN — 26 h (W3, W4, W5, W7)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Avvio e DNS** | T1, T2 | **T1** (9 h) `plat` per Windows: ricerca dal registro, versione, management su TCP con la password su `stdin`, configurazione dalla pipe a uso singolo, «job» che chiude il figlio, controllo del proprietario della porta. **T2** (8 h) DNS dal `PUSH_REPLY` applicato dal modulo comune, `pull-filter`, DNS tolto dall'adattatore che resta | P3; esito di R-B | 17 h | OpenVPN connesso dalla CLI, con il DNS giusto |
| **B · Credenziali e sicurezza** | T3, T4 | **T3** (6 h) password, OTP e passphrase; blocco IPv6 con OpenVPN; il profilo malevolo (nessun `PWNED-*`); nessun segreto in file e righe di comando; su 2.6 e 2.7. **T4** (3 h) OpenVPN assente o vecchio; nota sulla convivenza con OpenVPN GUI | A | 9 h | **tutti e due i backend dalla CLI**: un collega pratico potrebbe già usarla |

### P5 · La finestra — 43 h (W8, W9, avviso di W11)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Si avvia e ricorda** | T1, T2 | **T1** (7 h) la finestra compila e parte su WebView2: collegamenti per piattaforma, `link.rs` sugli errori della pipe, testi del primo avvio nei due cataloghi, `tray_is_shown`. **T2** (6 h) archivio DPAPI dietro `SecretStore`, usato da finestra e CLI (Q63) | P2 (per provare con i backend veri: P4) | 13 h | la finestra mostra lo stato del servizio e ricorda le credenziali |
| **B · I pezzi di sistema** | T3, T4, T5 | **T3** (5 h) finestre «Apri» e «Salva» di Windows da Rust; protezione del file esportato. **T4** (6 h) istanza unica con mutex e pipe dell'utente; il `.ovpn` passato come argomento. **T5** (6 h) tray: icone, click sinistro, chiusura nella tray; richiesta delle versioni per Windows | A | 17 h | import, tray e avviso di versione funzionano |
| **C · Fumo e aspetto** | T6, T7 | **T6** (8 h, tetto di 5 sul driver) fumo ridotto con `tauri-driver` e `msedgedriver` nel job `win-gui`. **T7** (5 h) passata sull'aspetto in VM: `oklch()`, caratteri, due temi, 100% e 150%, con le fotografie | B | 13 h | **uso quotidiano dalla finestra**, installata a mano |

### P6 · MSI e consegna — 27 h (W10)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Il pacchetto** | T1, T2 | **T1** (9 h) MSI con il frammento del servizio, `wireguard.dll`, riavvio dopo un errore, associazione dei `.ovpn`, aggiornamento che tiene i profili, disinstallazione pulita; job `win-msi`. **T2** (6 h) in VM: installazione, aggiornamento e disinstallazione da utente normale con UAC e da amministratore; avvio al riavvio; durate; aggiornamento con il tunnel su | P5; Q67 | 15 h | un MSI che si installa su un Windows 11 pulito |
| **B · Consegna e chiusura** | T3, T4 | **T3** (5 h) flusso di rilascio: da un tag, MSI e SHA-256 nella release, senza firma (Q61); guida per i colleghi: OpenVPN prima, i passi di SmartScreen, i limiti. **T4** (7 h) giro quotidiano in VM con i due backend; `docs/09-esito-fase3.md` con W1–W10, limiti e «non verificato»; roadmap | A; Q61 | 12 h | **i colleghi lo installano** |

Poi, fuori dalle ore: **l'ora di Peppe** sulla sua macchina con il profilo di Addway, con la lista scritta in T4.

**Totale dell'essenziale: 32 + 24 + 29 + 26 + 43 + 27 = 181 h, in 13 card.**

## Rifinitura: si decide dopo, blocco per blocco

### P7 · Updater e firma — 22 h (W11)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Updater di Tauri** | T1, T2 | **T1** (10 h) plugin, chiave degli aggiornamenti, `latest.json` nel flusso di rilascio, modo `passive` con UAC, il caso dell'utente non amministratore, una sola richiesta senza identificativi. **T2** (4 h) aggiornamento con il tunnel su; firma sbagliata rifiutata; ritorno all'avviso se l'updater non regge | P6; una release vera | 14 h | gli aggiornamenti arrivano da soli |
| **B · Firma Authenticode** | T3 | **T3** (8 h) firma di eseguibili e MSI nel flusso di rilascio, con la strada scelta in Q61. Fuori dalle ore: i tempi di validazione dell'identità, da 1 a 20 giorni lavorativi con Artifact Signing | P6; Q61 diversa da (a) | 8 h | l'MSI ha un editore |

### P8 · Accessibilità, CI di rete, matrice — 34 h (W12, W13)

| Gruppo | Task | Contenuto | Parte da | Stima | Alla fine si può fermare con |
|---|---|---|---|---|---|
| **A · Lettore di schermo** | T1 | **T1** (8 h, tetto) Narratore e NVDA sulle regioni vive di WebView2, dal registro del parlato; se tacciono, l'annuncio via UIA | P5 | 8 h | esiti e cambi di stato letti, o il limite scritto |
| **B · Tunnel in CI** | T2 | **T2** (10 h, tetto di 3 sulla prova) driver di WireGuardNT, OpenVPN e un server sul runner; se regge, le prove di P3 e P4 nel job `win-lab` | P4 | 10 h | i tunnel provati a ogni push, o «solo in VM» scritto |
| **C · Fumo completo e matrice** | T3, T4 | **T3** (10 h) il resto del fumo della fase 2 che vale su Windows. **T4** (6 h) matrice manuale su Windows 11, avvio a freddo, chiusura dell'esito | P6 | 16 h | **fase 3 chiusa** |

**Totale della rifinitura: 22 + 34 = 56 h, in 5 card.** I tre gruppi di P8 non dipendono l'uno dall'altro.

## Totali

| | Ore | Card |
|---|---|---|
| P1, da solo | 32 | 2 |
| Essenziale (P1–P6) | 181 | 13 |
| Rifinitura (P7–P8) | 56 | 5 |
| **Tutto** | **237** | **18** |

**Forchetta dell'essenziale: 160–240 h.** In basso se le tre prove di P1 passano, `netsh` basta e i tetti non si
consumano (in 181 h i tetti sono contati per intero). In alto se cade
R-A (+6 h), se serve l'interfaccia di programmazione per la rete (+6 h), se la VM non regge e si lavora con la
Sandbox e il relay (ogni giro di prova più lento: stimo +20 h sui blocchi P3, P4 e P6), più i giri di correzione
dopo le review, che in fase 2 ci sono stati quasi a ogni blocco. Se cade R-B, P4 va ristimato e la forchetta non vale.

Non sono nelle ore: le review (una per gruppo, 18 in tutto), il tempo di Peppe (registrazione e ISO, `sudo`, le
risposte, l'ora finale: due o tre ore), le attese esterne della firma.

## Ordine e dipendenze

```
P1-A (VM, prove di rischio) ──┐
P1-B (compila) ── P2 ── P3 ── P4 ──┐
                   └──── P5-A ─────┴─ P5-B ── P5-C ── P6 ──┬─ P7
                                                           └─ P8-C
                                          P5 ── P8-A      P4 ── P8-B
```

P5-A può partire dopo P2, in parallelo a P3 e P4: con il demone a `FakeBackend` la finestra si prova senza tunnel.

## Dove ci si può fermare, e cosa si ha in mano

| Dopo | Ore spese | Cosa c'è |
|---|---|---|
| P1 | 32 | la risposta sui rischi 1 e 2; la CI che compila su Windows; una VM riusabile |
| P2 | 56 | il servizio e la pipe con le ACL provate: niente che un collega possa usare |
| P4 | 111 | Galleria da riga di comando su Windows, con i due backend, installata a mano |
| P5 | 154 | la finestra, installata a mano |
| P6 | 181 | **l'MSI per i colleghi** |
| P7 | 203 | aggiornamenti automatici, firma se decisa |
| P8 | 237 | fase 3 chiusa, con la matrice |

Fermarsi tra P2 e P4 lascia codice che non serve a nessuno: se si parte oltre P1, conviene arrivare almeno a P4.

## Cosa resta fuori anche dalla rifinitura

Windows 10 e ARM; `winget`; OpenVPN incluso; kill switch e protezione dalle fughe di DNS; avvio con il sistema;
SignPath (fase 4); la prova su un PC con Smart App Control acceso; due utenti interattivi veri sulla stessa macchina
(sessione estranea provata solo al livello 2).
