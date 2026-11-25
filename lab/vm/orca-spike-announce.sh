#!/usr/bin/env bash
# Dentro la VM, da utente `lab` (spike Orca, docs/08 «Spike Orca»): le quattro strade per far dire una frase a Orca con
# l'evento AT-SPI `object:announcement`. Richiede Orca già avviato (orca-spike-vm.sh orca) e ~/spike/atspi-announce.py,
# ~/spike/gtk-announce.py. Per ognuna stampa le frasi che Orca ha detto nei due secondi dopo.
#   A  programma fuori dall'app, mittente non registrato come app AT-SPI  (la sorgente non esiste)
#   B  programma fuori dall'app, registrato come app AT-SPI minima         (la sorgente è viva)
#   C  finestra GTK3, segnale ATK `announcement(testo)`                    (mittente = l'app stessa, come farebbe il lato Rust di Galleria)
#   D  finestra GTK3, segnale ATK `notification(testo, urgenza)` assertive
set -uo pipefail
export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus WAYLAND_DISPLAY=wayland-0
S=$HOME/spike/orca-spike-vm.sh
quiet() { grep -v -i 'deprec\|register_object\|warn\|^$'; }
n=$($S count); echo "== A: fuori dall'app, mittente non registrato"
python3 ~/spike/atspi-announce.py --polite --hold 2 "A: from outside, not a live accessible" 2>&1 | quiet; sleep 2; $S since "$n"; echo "(fine A)"
n=$($S count); echo "== B: fuori dall'app, registrato come app AT-SPI minima"
python3 ~/spike/atspi-announce.py --register --polite --hold 3 "B: from a registered probe application" 2>&1 | quiet; sleep 2; $S since "$n"; echo "(fine B)"
n=$($S count); echo "== C: ATK announcement dentro una finestra GTK3"
python3 ~/spike/gtk-announce.py --delay 3 "C: atk announcement from a GTK3 window" 2>&1 | quiet; $S since "$n"; echo "(fine C)"
n=$($S count); echo "== D: ATK notification assertive dentro una finestra GTK3"
python3 ~/spike/gtk-announce.py --signal notification --assertive --delay 3 "D: atk notification assertive" 2>&1 | quiet; $S since "$n"; echo "(fine D)"
echo "-- eventi announcement visti da Orca (ultimi 4):"
grep "Dequeued object:announcement" ~/spike/orca-debug.log | tail -4 | cut -c1-210
