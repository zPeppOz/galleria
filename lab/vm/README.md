# VM desktop per le prove S14 e S15

Due VM manuali, **Ubuntu 26.04** e **Fedora 45**, con GNOME, NetworkManager e systemd-resolved veri.
Servono a provare ciò che il laboratorio in container non può mostrare: NetworkManager reale,
SELinux, un kernel con il modulo DCO `ovpn`. Dischi e immagini stanno solo in `/mnt/b/vm` (mai sugli SSD).

| File | Cosa fa |
| --- | --- |
| `ubuntu-user-data.tmpl`, `fedora-user-data.tmpl` | cloud-init non presidiato: utente `lab` solo con chiave SSH (nessuna password), desktop, pacchetti di prova, riavvio finale. `@SSH_PUBKEY@` va sostituito con la chiave pubblica. |
| `vmctl.sh` | `start`/`stop`/`ssh`/`status`: qemu diretto con KVM, UEFI, rete utente, SSH su `127.0.0.1:2201` (Ubuntu) e `:2202` (Fedora). |
| `vm-lab.sh` | dentro la VM, da root: `prepare <ottetto>` (utenti, `install.sh`, immagini), `up <ottetto>` (server `glab-*` e `eth0`), `run <filtro>` (i test e2e). |
| `s15-check.sh` | dentro la VM: l'evidenza di S15 (nmcli, resolvectl, DNS prima di `connected`). |
| `shims/pgrep` | solo per `vm-lab.sh run`: nasconde gli `openvpn` dei container server. |

## Come è costruito il laboratorio nella VM

Le immagini sono quelle ufficiali dei cloud image (hash verificati; Ubuntu anche la firma GPG). Il desktop si
installa sopra con cloud-init: `ubuntu-desktop-minimal` e `gnome-shell-extension-appindicator` su Ubuntu,
il gruppo `workstation-product-environment` e `gnome-shell-extension-appindicator` su Fedora. Su Ubuntu il
renderer di netplan passa a NetworkManager, come in un'installazione desktop.

I test e2e sono quelli del laboratorio (`lab/out/tests/*`), compilati nel container `glab-build`
(glibc 2.39) e copiati in `/lab` nella VM. I server `glab-*` girano come container Docker **nella VM**
sulla rete `galleria-fase1`. La VM stessa fa da client: `eth0` è un veth attaccato al bridge, gestito da
NetworkManager (connessione `glab-eth0`, 172.31.0.<ottetto>/24 e 2001:db8:31::<ottetto>/64, gateway la
`glab-target`, metrica 50), come l'`eth0` del client Docker. La NIC utente (`enp0s1`) resta per SSH con
metrica alta.

Differenze dichiarate rispetto al client Docker:

- `/.dockerenv` esiste (è la sola cosa che la guardia `assert_lab` controlla); la VM è usa e getta.
- `binfmt_units_are_masked` viene saltato: controlla la riparazione WSL2 delle immagini del laboratorio.
- `pgrep`/`pkill` filtrano gli `openvpn` dei container server, che qui condividono `/proc` con il client.
- `residue()` ignora `expires Nsec`, il conto alla rovescia delle rotte RA di un link vero.
- Sulla VM Fedora le cartelle `lab/pki` e `lab/out` hanno etichetta `container_file_t` (SELinux enforcing).
- Mentre il laboratorio è su la VM non ha internet (come il client Docker): per installare pacchetti
  `nmcli connection down glab-eth0`, poi di nuovo `vm-lab.sh up`.

## Passi

```
lab/build.sh 26 && lab/gen-pki.sh          # sull'host: binari, test, PKI
vmctl.sh start ubuntu                      # prima accensione = installazione del desktop (~30 min)
rsync -a --exclude target --exclude .git . lab@127.0.0.1:/lab/   # (porta 2201/2202)
sudo bash /lab/lab/vm/vm-lab.sh prepare 27 && sudo bash /lab/lab/vm/vm-lab.sh up 27   # Fedora: 79
sudo GALLERIA_E2E_DCO=1 bash /lab/lab/vm/vm-lab.sh run dco      # solo DCO, dopo `modprobe ovpn`
sudo bash /lab/lab/vm/vm-lab.sh run ""                          # tutto
sudo bash /lab/lab/vm/s15-check.sh
```

Dopo ogni riavvio della VM serve di nuovo `vm-lab.sh up <ottetto>`.

## Spike Orca (sessione GNOME vera)

Per provare Orca con la GUI di Galleria nel desktop vero delle due VM (esito in `docs/08-esito-fase2.md`, «Spike Orca»).
`DISPLAY_OPT=vnc=127.0.0.1:1 vmctl.sh start ubuntu` (`:2` per Fedora) apre un VNC locale; il monitor qemu è sempre su
`/mnt/b/vm/<vm>.mon`.

| File | Cosa fa |
| --- | --- |
| `vmkeys.py` | dall'host: `sendkey` dal monitor qemu (tastiera vera: `tab`, `shift-tab`, `ret`, `esc`…) |
| `vncshot.py` | dall'host: fotografia dello schermo dal VNC (RFB senza librerie; `screendump` di qemu non serve con GNOME) |
| `orca-spike-vm.sh` | in VM, da `lab`: `orca` (Orca con file di debug a righe), `app`, `app-spike`, `speech`, `count`, `since N`, `dump`, `daemon-down/up`, `versions` |
| `orca-spike-run.py` | dall'host: i giri `fields`, `toast`, `statusbar`, `all`, con parlato, fuoco AT-SPI e trascrizione |
| `orca-line-buffered.py` | Orca con il file di debug a righe (copia di `lab/tests/orca-line-buffered.py`) |
| `atspi-announce.py` | in VM: emette `object:announcement` da un programma fuori dall'app (`--register` lo fa comparire come applicazione AT-SPI) |
| `gtk-announce.py` | in VM e nell'immagine: finestra GTK3 che emette il segnale ATK `announcement` o `notification` |
| `orca-spike-announce.sh` | in VM: le quattro prove A–D degli annunci, con il parlato di Orca |
| `docker-announce.sh` | dall'host: le prove C e D nell'immagine `galleria-gui-dev` (Orca 46.1, Xvfb) |
| `orca-spike-inapp.patch` | **non applicato nel repo**: emette gli annunci da dentro `galleria-gui` (build di prova, `GALLERIA_SPIKE_ANNOUNCE=1`) |

Passi: copiare `galleria-gui`, `fake-daemon`, `atspi-dump.py` e gli script in `~lab/spike/` della VM, poi
`orca-spike-vm.sh orca`, `orca-spike-vm.sh app` e `orca-spike-run.py <vm> all`. GNOME blocca lo schermo dopo 5 minuti:
`gsettings set org.gnome.desktop.session idle-delay 0` e `…screensaver lock-enabled false`. Su Fedora, al primo accesso,
chiudere la panoramica (`Esc`) prima di avviare l'app, altrimenti non ha il fuoco.

## B6: tre desktop, compresa Ubuntu 24.04

`ubuntu24` è il terzo nome di `vmctl.sh`: SSH 2203, VNC `:3` (passare
`DISPLAY_OPT=vnc=127.0.0.1:3`). Gli altri due restano 2201/`:1` e 2202/`:2`.
Qemu ha una tavoletta USB per il puntatore su Wayland. Sempre KVM, 4 vCPU, 4 GiB,
UEFI, dischi e semi **solo in `/mnt/b/vm`**. Alla fine `vmctl.sh stop <vm>`.

La 24.04 nasce come le altre: immagine ufficiale
`https://cloud-images.ubuntu.com/releases/noble/release/ubuntu-24.04-server-cloudimg-amd64.img`,
`SHA256SUMS` e `SHA256SUMS.gpg`, firma con `/usr/share/keyrings/ubuntu-cloudimage-keyring.gpg`
(`gpgv`), hash con `sha256sum -c` sulla sola riga dell'immagine. La prova è in
`/mnt/b/vm/evidence/b6/ubuntu24/image-signature.txt`. Il disco `disks/ubuntu24.qcow2`
è 40 GiB virtuali; seme da `ubuntu24-user-data.tmpl` con `@SSH_PUBKEY@` sostituita
con la chiave **pubblica** del laboratorio. Il desktop si installa con cloud-init:
`ubuntu-desktop-minimal`, Orca, AppIndicator e WebKitGTK, autologin di `lab` senza password.
`cloud-init status --long` deve dire `done`: la sola comparsa di SSH non è la fine.
Il primo giro ha richiesto circa 25 minuti (compreso il Firefox snap del metapacchetto).

Dall'host, con la VM accesa:

```bash
scripts/gui.sh build
# se non già costruiti dai gate:
scripts/gui.sh run cargo build -p galleria-gui --examples
# galleria e galleriad sono quelli di lab/out/bin, compilati nel laboratorio
lab/vm/gui-vm.sh ubuntu24 install
lab/vm/gui-vm.sh ubuntu24 versions
lab/vm/gui-vm.sh ubuntu24 daemon fake day
lab/vm/gui-vm.sh ubuntu24 app GALLERIA_SOCKET=/home/lab/spike/run/galleriad.sock
```

`GUI_TARGET` e `LAB_OUT` consentono di riusare un build precedente **dello stesso commit**;
non c'è un ripiego silenzioso a un altro worktree. Nel T8 il GUI è il build del gruppo C,
`bcb3f39`, identico al codice di questa card: hash in `evidence/b6/binaries-bcb3f39.sha256`.
`install` aggiorna `/usr/local/bin/galleria-gui`, il `.desktop` dell'utente, icona, database
MIME e gli script in `~/spike`; `app` avvia direttamente il binario con un'unità utente e
`stderr` in `~/spike/app.log` (**non prova l'avvio dal menu**, che è M1).
`daemon real` usa `packaging/linux/install.sh`; `vm-lab.sh prepare/up` resta il laboratorio
per il tunnel vero. Dopo aver aggiunto `lab` al gruppo `galleria` serve una nuova sessione
(anche il gestore systemd dell'utente deve avere il gruppo): non allargare i permessi del socket.

Per le versioni, il `XDG_SESSION_TYPE` di SSH è `tty`: non è il desktop. `versions` stampa
anche `loginctl` di ogni sessione; quella locale deve essere `wayland`. Su Fedora un avvio
lento ha lasciato il greeter su tty1 con la sessione di `lab` già su tty2: `Ctrl+Alt+F2`
dal monitor qemu ha mostrato il desktop. Questa è una particolarità della VM, non una prova
di login con password. Numeri, fotografie e limiti sono in `docs/08-esito-fase2.md`, gruppo D.
