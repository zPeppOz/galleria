# 00 — Mercato e concorrenti

Versione 0.1 — 2026-11-06 — stato: bozza

Fonti lette il 2026-11-06. Dove una cosa non è stata provata a mano è segnata **da verificare**.

## Il problema (come lo vede Peppe)

Manca un client VPN desktop che sia insieme moderno, veloce, facile da usare e molto configurabile,
che gestisca **OpenVPN e WireGuard** nello stesso posto, come una "rubrica" di connessioni in stile
VirtualBox (lista a sinistra, dettaglio a destra), una connessione attiva alla volta, su Linux e Windows
(macOS rinviato).

## Concorrenti

| Prodotto | Licenza | OS | Protocolli | Note |
|---|---|---|---|---|
| **Pritunl Client** | open source ("Other" su GitHub) | Linux, Win, macOS, Flatpak | OpenVPN + WireGuard | Il più vicino. Si presenta come "Free and open source alternative to Viscosity". Import di profili OpenVPN standard; WireGuard dichiarato "directly from Pritunl servers" → **da verificare** se importa un `.conf` WireGuard generico. GUI Electron + servizio Go + CLI/TUI (v1.4, set 2026). Giudizio di Peppe: "l'interfaccia mi fa un po' cagare". |
| **Eddie** (AirVPN) | GPLv3 | Linux, Win, macOS | OpenVPN + WireGuard | Molto configurabile (direttive OpenVPN, rotte, network lock), ma costruito attorno al servizio AirVPN. **Da verificare** quanto bene gestisce profili di terzi. |
| **Defguard client** | "Other" su GitHub | Linux, Win, macOS | solo WireGuard | Stesso stack che proponiamo (Rust + Tauri + React). Qualsiasi server WireGuard; punto forte MFA con server Defguard. |
| **Viscosity** | proprietario, $14 una tantum | Win, macOS | solo OpenVPN | Il riferimento UX "rubrica di connessioni". |
| **Client WireGuard ufficiale** | MIT | tutti | solo WireGuard | Spartano, poco configurabile. |
| **OpenVPN Connect / Tunnelblick** | proprietario / GPL | Win, macOS / macOS | solo OpenVPN | — |
| **NetworkManager** (Linux) | GPL | Linux | entrambi (plugin) | Funziona, UX dispersa nei pannelli del desktop. |
| **Qomui** | GPL | Linux | OpenVPN + WireGuard | Qt, solo Linux, orientato ai provider. **Da verificare** se è mantenuto. |

## Lettura da CEO

- Il buco **non è** "non esiste un client open source multi-OS con entrambi i protocolli": Pritunl ed Eddie esistono.
- Il buco reale è più stretto: **UX moderna + configurabilità + nessun legame con un server/provider**.
  Pritunl è pensato per Pritunl server, Eddie per AirVPN, Defguard per Defguard (e solo WireGuard).
- È un progetto open source da portfolio/reputazione, non una fonte di ricavi: va messo un tempo massimo
  (vedi `01-analisi.md`, fasi e criterio di stop).

## Validazione prima di costruire (azione per Peppe, ~30 min)

Installare Pritunl Client ed Eddie e importare i profili veri (OpenVPN di lavoro, WireGuard della VPS).
Per ciascuno annotare:
1. cosa non riesce a importare o a configurare;
2. cosa è lento o scomodo nell'uso quotidiano (cambio connessione, credenziali, log);
3. cosa manca rispetto alla "rubrica stile VirtualBox".

Quella lista diventa il backlog di differenziazione del MVP.

## Fonti

- Pritunl Client: https://client.pritunl.com/ , https://github.com/pritunl/pritunl-client
- Eddie: https://eddie.website/ , https://github.com/AirVPN/Eddie
- Defguard client: https://github.com/DefGuard/client , https://docs.defguard.net/using-defguard-for-end-users/desktop-client
- Viscosity: https://viscosityapp.com/viscosity/ , https://www.sparklabs.com/support/kb/article/purchasing-viscosity/
- Qomui: https://github.com/corrad1nho/qomui
