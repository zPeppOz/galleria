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
