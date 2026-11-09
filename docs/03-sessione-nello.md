# 03 — Scaletta sessione con Nello (CTO)

Obiettivo: chiudere le decisioni tecniche che bloccano la fase 0 e assegnare gli spike. Durata: 60 minuti.
Prima di arrivare: leggere `01-analisi.md` §4.1, §5, §8, §11 e `02-modello-minacce.md`.

Le raccomandazioni sono di Aitano (CEO): la decisione tecnica è di Nello, quella finale di Peppe.

## 1. Contesto (5')

- Scope v0.1 invariato: OpenVPN + WireGuard, Linux + Windows, tetto ~8 settimane.
- Novità D9: roadmap protocolli (OpenConnect in v0.2, AmneziaWG in v0.3). Impatto sulla v0.1: solo l'architettura (A9).

## 2. Decisioni (40')

| ID | Domanda | Opzioni | Raccomandazione | Perché |
|---|---|---|---|---|
| A9 | Backend intercambiabili | interfaccia comune subito · solo quando serve | **subito** | costa poco ora, evita di riscrivere demone e IPC in v0.2 |
| A9b | OpenConnect | `libopenconnect` · processo figlio | **processo figlio** per partire | stesso schema di OpenVPN; meno FFI da mantenere |
| A3 | IPC | JSON-RPC su socket/pipe · gRPC (tonic) | **JSON-RPC** | un solo client (la GUI + CLI), facile da ispezionare e testare; gRPC aggiunge build e dipendenze |
| A5 | Archivio profili | TOML · SQLite | **TOML** | leggibile, versionabile, backup banale; volumi piccoli |
| A4 | OpenVPN su Windows | incluso · prerequisito | **prerequisito in v0.1**, inclusione dopo | elimina R4 (licenze driver, aggiornamenti di sicurezza) dal percorso critico; costo: un passo in più per l'utente |
| A10 | Blocco IPv6 in v0.1 | sì · rinviare al kill switch | **sì** | è la fuga più comune e la correzione è piccola |
| A13 | Aggiornamenti | vedi §8 | Windows: updater Tauri; Linux: solo avviso in v0.1 | evita di gestire un repository di pacchetti adesso |
| A12 | Matrice di test | vedi §8 | Ubuntu LTS, Fedora, Windows 11 obbligatori; il resto best effort | dimensione realistica per una persona più gli agenti |

## 3. Fase 0: spike e verifiche (10')

| Cosa | Esito atteso | Chi |
|---|---|---|
| Spike WireGuardNT su Windows | tunnel su da Rust, con licenza compatibile con SignPath (R3) o alternativa `wireguard-go` + wintun | da assegnare |
| Spike management interface OpenVPN | connessione, stato, byte, credenziali passate senza file su disco | da assegnare |
| ~~Licenza `defguard_wireguard_rs`~~ | FATTO 2026-11-09: Apache-2.0, compatibile con GPL-3.0-or-later → sì all'uso. Su Windows chiede la toolchain `windows-gnu`: da provare nello spike WireGuardNT | — |
| ~~`management ... unix` e `pull-filter` in OpenVPN 2.6~~ | FATTO 2026-11-09: entrambi presenti; il socket Unix va ristretto con `--management-client-user/group` | — |
| ~~Updater Tauri con MSI + servizio Windows~~ | FATTO 2026-11-09 sulla doc: updater riusa l'MSI, servizio via frammento WiX; prova pratica nello spike WireGuardNT | — |

## 4. Chiusura (5')

- Decisioni scritte in `01-analisi.md` (§2 come D10+, §12 stato RISOLTA).
- Data di inizio della fase 0.
