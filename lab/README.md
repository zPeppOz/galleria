# Laboratorio fase 1

Eseguire dalla radice del worktree: `LAB_BUILD=host lab/run.sh 26 t07`.
Client ammessi: 26 (Ubuntu 24.04), 27 (Ubuntu 26.04), 79 (Fedora corrente).
Il filtro seleziona un file (`t07`, `t08`, `t09`) oppure un nome di test.
Senza filtro esegue tutti i test ignorati, in serie, con il demone vero.

Docker e il modulo WireGuard devono essere disponibili. In WSL2 il modulo è già
caricato: non eseguire modprobe sull'host. Le prove modificano solo container
`glab-*` e rete dual-stack `galleria-fase1`, mai rotte o DNS dell'host.
I client usano NET_ADMIN, NET_RAW, SYS_ADMIN e un cgroup privato scrivibile;
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
