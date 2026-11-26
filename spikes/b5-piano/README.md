# Misure fatte per il piano di B5 (2026-11-26)

Script e uscite delle due misure citate in `docs/superpowers/plans/2026-11-26-fase2-b5-connessione.md` (Q42, Q44).
Gli script sono com'erano quando sono girati: i percorsi assoluti puntano alla cartella di lavoro di quella sessione.
Niente di tutto questo entra nel prodotto; `relay.py` è il prototipo da cui parte il T9.

## 1. Crate per Secret Service (Q44, S12 della spec)

`measure-keyring.sh`: per ogni candidato, `cargo add` in `crates/galleria-profile`, conteggio dei crate in `Cargo.lock`
(base: 537), `cargo deny check` (cargo-deny 0.20.2), poi `git checkout` dei due file.

| Candidato | Crate in più | `cargo deny check` | Cosa entra nella CLI |
|---|---|---|---|
| `dbus-secret-service` 4.2.0, senza cifratura | +1 | verde | `dbus` 0.9.12 e `libdbus-sys` 0.2.7 (già in `Cargo.lock` per `tao`) |
| `dbus-secret-service` 4.2.0, `vendored` | +8 (`deny-vendored.sh`) | verde | gli stessi, con libdbus compilata dentro |
| `dbus-secret-service` 4.2.0, `crypto-rust` | +14 | verde | più `aes`, `cbc`, `hkdf`, `hmac`, `num*` |
| `secret-service` 6.0.0, `rt-tokio` + `crypto-rust` | +21 | verde | `zbus` 6.5.0 su tokio |
| `oo7` 0.7.3 | +33 | verde | `zbus` 6.5.0, `ashpd`, `rand`… |
| `keyring` 4.2.0, predefinito | +50 | **rosso**: `RUSTSEC-2026-0370`, `-0371`, `-0372` (`zvariant`, `zbus_names`, `zvariant_derive` non più mantenuti) | `zbus` 5.19.0 e la famiglia `async-io` |
| `keyring` 4.2.0, solo `dbus-secret-service-keyring-store` | +24 | verde | `openssl` con `openssl-src` |

`build-keyring.sh`: `cargo build -p galleria-cli` su questo host (WSL, senza `libdbus-1-dev`: `pkg-config --modversion dbus-1`
non trova niente).

| Candidato | Build | `ldd` del binario `galleria` |
|---|---|---|
| `dbus-secret-service`, senza `vendored` | **fallisce**: `libdbus-sys` chiede `dbus-1 >= 1.6` a `pkg-config` | — |
| `dbus-secret-service`, `vendored` | riesce (9 s oltre il resto) | nessuna libreria dbus o ssl |
| `secret-service` 6.0.0, `rt-tokio` | riesce (12 s) | nessuna libreria dbus o ssl |

Con `vendored`, in `galleria-gui` `libdbus-sys` riceve insieme le feature `vendored` (da `galleria-profile`) e `default`
(da `tao`): letto con `cargo tree -i libdbus-sys -e features`, **non compilato** nell'immagine della GUI.

La libdbus dentro `libdbus-sys` 0.2.7 è «Academic Free License version 2.1, or the GNU General Public License version 2 (or,
at your option any later version)» (`vendor/dbus/COPYING`, prime righe).

## 2. Come la finestra raggiunge un socket che sta in un client del laboratorio (Q42)

`socket-probe.sh`, con Docker 29.7.2 **rootless**, immagini `glab-client26` e `galleria-gui-dev`, contenitori `gprobe-*` senza
rete, tolti alla fine. `server.py` fa ciò che fa il demone: socket `root:galleria` `0660`, e stampa `SO_PEERCRED` e
`SO_PEERGROUPS` di chi si collega.

```
--- A1. gui image, default user of scripts/gui.sh under rootless (0:0)
uid=0 groups=[0] got b'echo:hello\n'
--- A2. gui image, uid of alice in the client (1001) with the gid of galleria in the client (1001) as a supplementary group
uid=1001 groups=[1001, 1002] got b'echo:hello\n'
--- A3. gui image, a uid without the group (like mallory)
connect failed as uid=1003 groups=[1003]: [Errno 13] Permission denied
--- B. relay on the host -> docker exec -i -u alice -> socket in the client
uid=0 groups=[0] got b'echo:hello\n'
--- B, from the host itself
uid=1000 groups=[4, 24, 27, 30, 46, 100, 101, 999, 1000] got b'echo:hello\n'
--- what the server saw
peer pid=0 uid=0 gid=0 groups=[0]                    (A1: il demone vedrebbe root)
peer pid=0 uid=1001 gid=1002 groups=[1001, 1002]     (A2)
peer pid=7 uid=1001 gid=1002 groups=[1001, 1002]     (B, dalla finestra)
peer pid=14 uid=1001 gid=1002 groups=[1001, 1002]    (B, dall'host)
```

- **A, cartella del socket montata in tutti e due i contenitori**: funziona. Chi si collega è visto con i numeri di uid e
  gid del proprio contenitore; con l'utente predefinito di `scripts/gui.sh` sotto Docker rootless il demone vedrebbe
  **root**, e per vedere `alice` bisogna lanciare l'immagine della GUI con l'uid e il gid numerici di quel client
  (Ubuntu: `alice` 1001, `galleria` 1001; Fedora: 1000 e 1000).
- **B, ponte**: funziona. Il demone vede `alice` vera, con i suoi gruppi e un pid del client, chiunque sia l'utente
  dall'altra parte.
- **Non provato**: A sotto la unit vera (`RuntimeDirectory=galleria` sopra un punto di montaggio); A e B su un runner con
  Docker non rootless.
