# Handoff — fase 2, B4, gruppo A: chiusura (T3 e giro finale)

- Task kanban `t_517b1d11`, ramo `galleria/t_517b1d11-…`, partito da `54321de` (ff-only dal ramo di `t_1ecf64e6`). Nessun push.
- Commit nuovo: `pick the file to import in the open window opened from Rust` (T3). Nessun commit `fix`: sul rilancio le correzioni delle sequenze di Tab (già nel ramo) bastavano, smoke 28/28 e Orca 34/34 su T1–T2 senza toccare il codice.
- Dopo T3, a fine gruppo: smoke 28/28; Orca 38/38 per due giri di fila; `scripts/gui.sh check` verde (533 prove frontend); gate host verdi.
- Sonda: `lab/tests/atspi-file-window.py` (save/open/cancel/dump). Il campo del percorso di «Apri» compare scrivendo `/`; `Ctrl+L` via AT-SPI non arriva. Solo i campi `showing` contano. `GALLERIA_A11Y_FILTER` / `GALLERIA_SMOKE_FILTER` passano nell'immagine.
- Riserve: un `Gtk-CRITICAL … WIDGET_REALIZED_FOR_EVENT` per ogni prova che scrive `/` nella finestra (evento sintetico; non indagato); finestra «Apri» non provata sotto portale/Wayland/mouse.
- Resta alla card `t_bd96b1bf`: T4 (trascinamento) e T5 (argomento, istanza unica, associazione `.ovpn`).
