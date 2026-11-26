# Misure per il piano di B6

Fatte il 26 novembre 2026 su `main` a `7e017c9`, per `docs/superpowers/plans/2026-11-26-fase2-b6-tray-chiusura.md`.
Nessuna riga di codice di B6: sono le misure su cui il piano decide.

## 1. Il menu di una tray AppIndicator si legge e si clicca dal bus di sessione

`tray-probe.py`, nell'immagine `galleria-gui-dev` (Ubuntu 24.04, `libayatana-appindicator3-1` 0.5.93), sotto
`dbus-run-session` e `xvfb-run`, senza nessun desktop:

```bash
docker run --rm -v "$PWD/spikes/b6-piano/tray-probe.py:/probe.py:ro" galleria-gui-dev:latest \
  bash -c 'dbus-run-session -- xvfb-run -a python3 /probe.py'
```

Un processo fa da `org.kde.StatusNotifierWatcher` finto (40 righe con `Gio`, che `python3-gi` porta già); un altro
apre un AppIndicator GTK 3 con tre voci. Uscita:

```text
REGISTERED sender=:1.3 arg=/org/ayatana/NotificationItem/galleria_probe
item path=/org/ayatana/NotificationItem/galleria_probe menu=/org/ayatana/NotificationItem/galleria_probe/Menu status=Active icon=network-vpn
LAYOUT before: [(2, 'Disconnected', False), (3, 'Connect Ufficio', True), (4, 'Quit', True)]
app: CLICKED Connect Ufficio
LAYOUT after: [(2, 'Disconnected', False), (3, 'Disconnect Ufficio', True), (4, 'Quit', True)]
```

Quindi: le voci, il loro stato «attiva o spenta» e il click (`com.canonical.dbusmenu.Event(id, "clicked", …)`) passano
dal bus, e il menu cambiato si rilegge. Con `--no-watcher` l'indicatore **non si registra** (nessun oggetto sul bus) e
il processo resta vivo: è il caso «desktop senza tray».

Non provato: la stessa sonda contro `galleria-gui` (il menu lo costruisce `muda` dentro Tauri, non PyGObject), e un
desktop vero.

## 2. Costo dei crate

`measure-crates.sh` e `measure-tray-and-licence.sh` lavorano su una copia dei manifest (`git archive`), non sul
worktree. `Cargo.lock` di partenza: 545 pacchetti; albero normale di `galleria-gui` su Linux: 264 crate;
`cargo deny check` (0.20.2) verde.

| Aggiunta a `galleria-gui` | `Cargo.lock` | Crate nuovi nell'albero | `cargo deny` |
|---|---|---|---|
| `tauri` con la feature `tray-icon` | +0 | 4: `tray-icon` 0.25.1, `libappindicator` 0.9.0, `libappindicator-sys` 0.9.0, `libloading` 0.7.4 | verde |
| `tauri` con `tray-icon` e `image-png` | +4 | 10 (in più `image`, `bytemuck`, `moxcms`…) | verde |
| `soup3` 0.5 (quello che `webkit2gtk` porta già) | +0 | 0 | verde |
| `semver` 1 | +0 | 0 | verde |
| `ureq` 3 con rustls | +7 | 12 | **rosso**: `webpki-roots` è `CDLA-Permissive-2.0`, non in elenco |
| `ureq` 3 con native-tls | +13 | 18 righe | **rosso** (licenze; quale, non l'ho letto) |
| `reqwest` 0.13 con rustls | +27 | 39 righe | **rosso** (licenze; quale, non l'ho letto) |
| `reqwest` 0.13 con native-tls | +8 | 36 righe | verde |

Dove c'è «righe» il numero viene dal primo script, che conta le righe distinte di `cargo tree` (358 di partenza,
perché la stessa versione compare con più insiemi di feature); gli altri vengono dal secondo, che conta i nomi.

`reqwest` 0.13.5 è già in `Cargo.lock` solo perché `tauri` lo chiede su Android e iOS: su Linux non si compila.

## 3. Sorgenti letti (versioni di `Cargo.lock`)

- `libappindicator-sys` 0.9.0, `src/lib.rs`: carica a runtime `libayatana-appindicator3.so.1`, poi
  `libappindicator3.so.1`, e **va in panico** se non ne trova nessuna.
- `tray-icon` 0.25.1: su Linux scrive l'icona come PNG in `$XDG_RUNTIME_DIR/tray-icon` (o nella cartella temporanea),
  salvo `temp_dir_path`; ha una feature `ksni` (StatusNotifierItem senza librerie di sistema) che `tauri` 2.12.7
  **non** espone.
- `tauri` 2.12.7, `src/tray/mod.rs`: su Linux gli eventi dell'icona non arrivano; il tooltip non c'è; un menu, una
  volta messo, non si toglie e non si sostituisce, ma se ne cambia il contenuto. `tauri::include_image!` esiste
  (`src/lib.rs`, riga 86).

## 4. L'endpoint delle release

```text
GET https://api.github.com/repos/tauri-apps/tauri/releases/latest   senza User-Agent → 403; con → 200, 39 kB di JSON, x-ratelimit-limit: 60
GET https://github.com/tauri-apps/tauri/releases/latest             senza User-Agent → 302, Location: …/releases/tag/tauri-cli-v2.12.6
GET https://github.com/zPeppOz/galleria/releases/latest             → 302, Location: …/releases   (nessuna release oggi)
GET https://api.github.com/repos/zPeppOz/galleria/releases/latest   → 404
```

## 5. Le VM

- `/mnt/b/vm`: `ubuntu` (26.04) e `fedora` (45), spente; 1,7 TB liberi; `/dev/kvm` c'è; 31 GB di RAM, 16 core.
- Fedora 45, accesa e rispenta per questa misura: `libappindicator-gtk3` 12.10.1 (`/usr/lib64/libappindicator3.so.1`,
  la **seconda** scelta di `libappindicator-sys`; `libayatana` non c'è), `gnome-shell-extension-appindicator` 66,
  `gnome-shell` 51.1 su Wayland, `nautilus` 51.1, `xdg-desktop-portal-gnome` 51.0, `webkit2gtk4.1` 2.54.2,
  `libsoup3` 3.8.1, `glib-networking` 2.90.0, `orca` 51.1.
- L'immagine cloud di Ubuntu 24.04 c'è: `https://cloud-images.ubuntu.com/releases/noble/release/ubuntu-24.04-server-cloudimg-amd64.img`
  (200, 621 MB). Non scaricata.
