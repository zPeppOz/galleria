# Laboratorio fase 1

Eseguire dalla radice del worktree: `LAB_BUILD=host lab/run.sh 26 t07`.
Client ammessi: 26 (Ubuntu 24.04), 27 (Ubuntu 26.04), 79 (Fedora corrente).
Il filtro seleziona un file (`t07`, `t08`, `t09`, `t12`, `t13`, `t14`, `t16`) oppure un nome di test.
Senza filtro esegue tutti i test ignorati, in serie, con il demone vero.

Docker e il modulo WireGuard devono essere disponibili. In WSL2 il modulo è già
caricato: non eseguire modprobe sull'host. Le prove modificano solo container
`glab-*` e rete dual-stack `galleria-fase1`, mai rotte o DNS dell'host.
I client usano NET_ADMIN, NET_RAW, SYS_ADMIN, SYS_PTRACE e un cgroup privato scrivibile;
seccomp/AppArmor del container sono disattivati per consentire le sandbox systemd.
La unit del prodotto conserva tutte le restrizioni della spec.

Le immagini vengono costruite una volta e riusate; `LAB_REBUILD=1` le aggiorna.
Il build predefinito compila in Ubuntu 24.04 usando la toolchain globale montata
in sola lettura; `LAB_BUILD=host` usa direttamente Cargo sull'host.
Non si copia rustup nel worktree. Prima dei build si richiedono almeno 8 GiB liberi.
PKI e profili con chiavi generate stanno solo in `lab/pki` e `lab/out`, ignorati da git.
I dati alice/bob e la passphrase di laboratorio sono sintetici, mai credenziali reali.

`lab/run.sh` esegue sempre `lab/down.sh` in uscita, anche su errore; `KEEP=1`
conserva i container per la diagnosi. `CLIENTS="" lab/up.sh` avvia solo i server.
`lab/down.sh` rimuove tutti i container `glab-*` e la sola rete `galleria-fase1`.
Il job CI `lab` è definito per i tre client; l'esecuzione remota richiede un push.

Le unit binfmt sono mascherate nelle immagini. Ogni run controlla WSLInterop e,
se necessario, applica la riparazione autorizzata con un container temporaneo.
Il controllo non è applicabile sui runner Linux senza WSL.

`lab/tests/check-wsl.sh` verifica la riparazione con letture proc e Docker simulati:
nessuna registrazione binfmt reale viene cambiata dal test. Eseguito anche dal job check.

OpenVPN: `LAB_BUILD=host lab/run.sh 26 t12` (poi 27 e 79). Il test DCO si salta
se non è impostato `GALLERIA_E2E_DCO=1`; il runner inoltra la variabile al client.
Il job `lab-dco` carica il modulo `ovpn` del kernel (OpenVPN 2.7, client 27 e 79) ed esegue solo il filtro `dco`;
OpenVPN 2.6 (client 26) non parla l'API del modulo in-tree e non è nella matrice DCO.
Il server `dns-option` usa DNS `10.9.0.1`; quello base usa `10.8.0.1`.
Il build in container usa uid/gid dell'utente e HOME nel worktree, così target e
cache Cargo restano rimovibili dall'utente. Con Docker rootless il build gira come
root del container, che corrisponde già all'utente sull'host. PKI/profili sono generati root 0600.

T13 prova credenziali, errori e riconnessione; T14 misura IPv6 (controlli inclusi);
T16 usa la CLI come alice e verifica anche il rifiuto di mallory. SYS_PTRACE serve
soltanto al processo osservatore dei test per leggere i fd del demone non dumpabile:
la CapabilityBoundingSet della unit del prodotto resta invariata.
La CLI si installa insieme al demone. Password e OTP dei test passano esclusivamente
su stdin, senza valori segreti negli argomenti. L'archivio personale usa file 0600
in directory 0700; il demone non legge la home.

Docker monta /proc/sys in sola lettura: il client riceve src_valid_mark=1 alla
creazione del proprio namespace tramite --sysctl. Il runner di rete verifica il
valore dopo il comando: procps 4.0.4 può restituire successo senza scrivere,
mentre procps 4.0.6 segnala errore anche se il valore è già quello richiesto.
Nessun sysctl dell'host viene modificato.

G3 (import dalla finestra, fase 2 B4 T6): `scripts/gui.sh lab-g3 <26|27|79>` porta su il laboratorio, fa importare alla
finestra vera (immagine `galleria-gui-dev`, sotto Xvfb) i profili `base.ovpn`, `malicious.ovpn` e `wg-split.conf` di
`lab/out` e lascia la sua cartella dei profili in `target/gui-lab/config/galleria`; poi `t17_gui_import` gira sul client:
la CLI e il demone veri elencano e connettono ciò che la finestra ha importato, e nessun `PWNED-*` compare. Le chiavi
WireGuard di `wg-split.conf` cambiano a ogni `lab/up.sh`, quindi l'import avviene dopo `up.sh` e prima dei test:
`lab/run.sh` esegue `LAB_BEFORE_TESTS` in quel punto. A mano: `GALLERIA_E2E_GUI_STORE=1 LAB_BEFORE_TESTS='scripts/gui.sh
lab-import' lab/run.sh 26 t17`. Senza `GALLERIA_E2E_GUI_STORE=1` `lab/run.sh` non esegue `t17` e lo dice in una riga (una prova che si
salta risulterebbe «ok»): nel giro intero lo salta, con `t17` come filtro esce con errore. Nella CI lo fa il job `lab-g3`
(`LAB_BUILD=host scripts/gui.sh lab-g3 <client>`, client 26, 27 e 79). La finestra gira
solo nell'immagine Ubuntu 24.04 (Q37 (a)): sui tre client girano la CLI e il demone, non WebKit.

G4–G9 (finestra contro il demone vero, fase 2 B5 T9 e T10): `scripts/gui.sh lab-connect <26|27|79>` porta su il laboratorio e, tra
`up.sh` e i test (`LAB_BEFORE_TESTS='scripts/gui.sh lab-connect-window <client>'`), fa questo sull'host: crea nel client `carla`, un
secondo utente del gruppo `galleria` (le immagini non cambiano; `alice` c'è già); avvia nel client l'osservatore
`lab/tests/gui-lab-watch.sh` (ogni 200 ms i processi `openvpn` con i loro argomenti e l'interfaccia `glr-wg0`, in
`/tmp/gui-lab-watch.log`; una volta al secondo un `ping` dentro il tunnel perché i byte si muovano); avvia due **ponti** sull'host
(`lab/tests/gui-daemon-relay.py`, `target/gui-lab/run/carla.sock` e `alice.sock`): ogni connessione che arriva lancia
`docker exec -i -u <utente> glab-client<N> python3 … join /run/galleria/galleriad.sock` e unisce i due flussi di byte, così il demone
vede **`carla` o `alice` veri**, con il loro uid e i loro gruppi (Q42 (b)); il ponte non legge i messaggi e non è in nessun pacchetto;
lancia nell'immagine della GUI (`lab-connect-here`, Xvfb, bus privato, GNOME Keyring sbloccato) la finestra vera, con
`GALLERIA_SOCKET` sul ponte di `carla`, che esegue `ui/e2e/lab-connect.e2e.ts`; ferma i ponti e l'osservatore. Poi `t18_gui_connect` gira
nel client e legge ciò che il client ha visto. `lab/run.sh` non lancia `t18` senza `GALLERIA_E2E_GUI_CONNECT=1` e lo dice (come `t17`:
una prova che si salta risulta «ok»), e con `LAB_TEST_LOG=<file>` tiene ciò che i test stampano. Il verbo finisce con
`scripts/check-gates.py --job lab-connect`: ogni prova del manifesto `lab/tests/gates-b5.tsv` per quel job deve essere nell'uscita con
`ok` o `passed`. Nella CI lo fa il job `lab-connect` (client 26, 27 e 79). **In locale** con Docker rootless OpenVPN 2.7 (client 27 e
79) prova il DCO e riceve `EPERM`: lì sono verdi solo le prove WireGuard e il giro intero è quello del job.
