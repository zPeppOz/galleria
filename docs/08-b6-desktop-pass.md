
## Gruppo D — desktop veri (B6, T8–T10), giro parziale

**Non è la chiusura di G10/G12 né della fase 2.** La card permette la consegna parziale a budget finito.
T8 è stato eseguito; di T9/T10 sono scritte qui sia le prove sia le parti non fatte. I limiti del banco
non diventano automaticamente limiti ammessi del prodotto. In particolare il menu Fedora è obbligatorio:
la sua mancata verifica non si può sanare scrivendo «best effort».

### T8: banco e provenienza

- ff-only da `bcb3f39` (T7 già chiuso), commit T8 `b900b5a`. Nessun push.
- Nuova VM `ubuntu24`, HDD `/mnt/b/vm/disks/ubuntu24.qcow2`, 40 GiB virtuali, 4 vCPU e 4 GiB RAM.
  GNOME da `ubuntu-desktop-minimal`, autologin `lab` senza password; SSH solo con chiave.
  Cloud-init finito, `status: done` (`/mnt/b/vm/evidence/b6/ubuntu24/cloud-init.txt`), spegnimento
  regolare e riaccensione provati (`ubuntu24/T8-stop.txt`). Anche le altre VM sono state spente e riaccese.
- Immagine ufficiale Noble, SHA256 `04e8f2461a16a44e78a370284471f29616cdd9baf1893df25807b71f985b6c94`,
  firma buona UEC Image Automatic Signing Key, fingerprint `D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81`.
  Firma in `ubuntu24/image-signature.txt`; manifesti e immagine in `/mnt/b/vm/images/`.
- `gui-vm.sh install` copia binario, esempi, CLI/demone del lab, `.desktop`, icona e database MIME.
  Usato il GUI costruito dal gruppo C a `bcb3f39`, identico al sorgente di T8: hash in
  `binaries-bcb3f39.sha256`. `install.log` delle tre VM: nessuna libreria mancante.
  `app` avvia direttamente il binario con un'unità utente: **non prova M1 (menu applicazioni)**.
- Fotografie finestra + icona: `ubuntu24/T8-window.png`, `ubuntu/T8-window.png`, `fedora/T8-window2.png`.
  Sulla Fedora inizialmente appariva il greeter, benché `lab` fosse già su tty2: `Ctrl+Alt+F2` ha
  mostrato la sessione. Le tre sessioni sono Wayland (non dedotto dall'ambiente esportato degli script:
  `loginctl`, per la nuova VM anche `ubuntu24/session.txt`).

Percorsi relativi in questa sezione: **`/mnt/b/vm/evidence/b6/`**, salvo dove scritto altrimenti.

| VM | GNOME Shell | Orca | WebKitGTK 4.1 | Libreria indicatore |
|---|---|---|---|---|
| Ubuntu 24.04.5, kernel 6.8.0-147 | 46.0 | 46.1-1ubuntu1 | 2.52.6-0ubuntu0.24.04.2 | libayatana-appindicator3.so.1 |
| Ubuntu 26.04.1, kernel 7.0.0-43 | 50.1 | 50.3 | 2.52.6-0ubuntu0.26.04.2 | libayatana-appindicator3.so.1 |
| Fedora 45, kernel 7.2.11-300.fc45 | 51.1 | 51.1 | 2.54.2-1.fc45 | **libappindicator3.so.1.0.0** (dal `/proc/<pid>/maps`) |

Fonte: `<vm>/versions.txt`. Il tipo `tty` di SSH **non** è il tipo della sessione grafica.
AppIndicator era ACTIVE sulle tre. La libreria caricata su Fedora è provata, il menu funzionante no.

### T9: tabella 12 × 3 — nessuna casella spacciata per una passata completa

`NF` = **non fatto per intero**, non «fallito». Ogni cella nomina la prova parziale o dice che non c'è.
Gli avvii del T8 non contano come M1; il tunnel in Docker del gruppo C non conta come M3–M6 in una VM.

| Caso | Ubuntu 24.04 | Ubuntu 26.04 | Fedora 45 |
|---|---|---|---|
| M1 avvio dal menu | NF; solo T8-window.png (avvio diretto) | NF; solo T8-window.png (avvio diretto) | NF; solo T8-window2.png (avvio diretto) |
| M2 menu tray | NF; M2-usb.png, M2-key-menu.png, tentativi non conclusivi | NF; M2-menu-hmp.png, M2-shell-tree.txt: etichette nel bus/albero, non menu aperto | NF; M2-retry.png: indicatore non stabilmente disponibile, vedi sotto |
| M3 preferito WG, demone vero | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M4 X, tunnel ancora su | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M5 disconnetti/credenziali dal menu | NF; T10-credentials.txt non passa dal menu, demone finto | NF; nessuna prova del menu | NF; T10-credentials.txt non passa dal menu, demone finto |
| M6 esci col tunnel su | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M7 Apri mouse + tastiera/import | NF; **M7-native.png/.txt**: chooser GTK del programma aperto via AT-SPI e annullato; non selezione mouse+tastiera | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M8 esporta profilo/log e 0600 | NF; nessuna prova VM | NF; nessuna prova VM | NF; nessuna prova VM |
| M9 doppio click Nautilus, chiusa/nascosta | NF; nessuna prova nuova | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M10 trascina da Nautilus | NF; nessuna prova nuova | NF; nessuna prova nuova | NF; nessuna prova nuova |
| M11 dimensioni/secondo avvio | NF; nessuna prova completa | NF; nessuna prova completa | NF; nessuna prova completa |
| M12 portachiavi bloccato/sblocco | NF; **NO_KEYRING=1** nei giri credenziali, non è questa prova | NF; nessuna prova nuova | NF; **NO_KEYRING=1** nei giri credenziali, non è questa prova |

**Risultato: 0/36 casi completi verificati in questo giro**, non 36 fallimenti del prodotto.
Non fatte nemmeno la prova con estensione AppIndicator spenta e la prova vera della persistenza del tunnel
in VM. Motivo della consegna parziale: il tempo è andato in provisioning, avvii lenti sul disco meccanico,
input/compositore e raccolta di evidenze riproducibili; non si trasformano tentativi in verdi.

Particolarità da riprendere **prima di G10 manuale**:

- `Ctrl+Alt+Tab` arriva ad Activities (AT-SPI); la sequenza per aprire l'indicatore non è stata completata.
  Tentati anche puntatore VNC e `mouse_move`/`mouse_button` qemu: nessuna fotografia prova il menu aperto.
  La tavoletta USB ora è nello script qemu. Non concluso S54; non è prova che il menu non funzioni.
- Fedora: un avvio ha mostrato icona e caricato la libreria (T8); dopo avvii successivi `app.log` ha scritto
  `libappindicator-WARNING ... Unable to connect to the Notification Watcher: Timeout was reached` e
  `Gtk-CRITICAL ... gtk_widget_get_scale_factor: assertion 'GTK_IS_WIDGET (widget)' failed`.
  **S56 secondo pezzo (menu): non verificato, causa non trovata. Non dichiarato limite accettabile di Fedora.**
  Il testo non è il `WIDGET_REALIZED_FOR_EVENT` di R7: non sono lo stesso avviso.
- M7 su 24.04: `FileChooserNative` appare nell'albero del processo Galleria, non in un processo portale;
  `M7-native.txt` riporta stderr senza il `WIDGET_REALIZED_FOR_EVENT` in quell'apertura. Era un'apertura via AT-SPI,
  **nessuna persona alla tastiera**: non chiude R7 e non prova il percorso col tasto `/`.

### Riserve ereditate: stato a questa consegna

| Riserva | Esito/prova o limite preciso |
|---|---|
| R1 modifiche non salvate alla chiusura | Verificata nel gruppo C (G10f e tastiera), nessuna nuova prova VM qui. |
| R2 Q28/Q49, Orca 46 | Vedi T10: **nome dei campi generali letto** su GNOME46; **dialogo credenziali non annunciato all'apertura e User name non detto nel giro**. Limite Q49(a) per il parlato su 24.04; albero con nome/ruolo/fuoco presente. Non estendere quel limite a tutti i campi. |
| R3 Q22/Q56 | Passata parziale dal debug; nessun ascolto umano. G12 non interamente verificato (mancano i quattro giri nuovi per VM). Lista cuffie sotto, non bloccante. |
| R4 Wayland/portale | Chooser GTK osservato su Wayland 24.04 (M7-native). Apri/Salva sulle altre due e scrittura file: non verificati qui. **Portale/sandbox resta limite dichiarato**: niente Flatpak/Snap in fase 2. |
| R5 file manager vero | Limite delle prove: doppio click e trascinamento Nautilus **non verificati**; resta l'associazione provata con `gio launch` in B4, non una prova col file manager. |
| R6 primo piano S31 | Limite di supporto già deciso: Wayland può negare il fuoco. L'apparizione da sola non prova il primo piano; M11 non completato. |
| R7 WIDGET_REALIZED_FOR_EVENT | Non verificata con tastiera umana; un'apertura AT-SPI senza quell'avviso non chiude la riserva. Nessun nuovo difetto attribuito all'app su questa base. |
| R8 GNOME Keyring vero | Limite delle prove: sblocco del sistema + Ricorda bloccato non verificati in VM. I giri NO_KEYRING non contano. |
| R9 KWallet | Limite già deciso: KDE best effort; non provato. |
| R10 finestra intera sulle tre | Avvio verificato T8; **non verificata la passata completa**. Non usare la sola finestra aperta per chiudere questo requisito. |
| R11 dimensioni su Wayland | Limite delle prove: persistenza dimensioni non verificata su Wayland in questo giro. |
| R12 S42–S44 | T11 (gruppo E), non riesaminati qui. |
| R13 instabilità a11y | Card dedicata e gruppo C; numeri di quel giro non riciclati come nuova passata VM. |
