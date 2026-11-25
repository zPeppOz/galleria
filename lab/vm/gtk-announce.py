#!/usr/bin/env python3
"""Una finestra GTK3 minima che emette il segnale ATK `announcement` sul proprio oggetto accessibile (spike Orca,
docs/08, «Spike Orca»). È la stessa pila di Galleria (GTK3 + atk-bridge): il segnale lo trasforma in
`object:announcement` sul bus AT-SPI, e il mittente è l'app stessa. Serve a sapere se Orca lo legge quando viene da
dentro un'app viva, e se è la stessa strada che dovrebbe prendere il lato Rust di Galleria.

    gtk-announce.py [--delay SEC] [--assertive] "testo" ["altro testo" ...]

Ogni testo viene annunciato una volta, a `--delay` secondi di distanza; poi la finestra resta aperta 5 s e si chiude.
"""
import argparse

import gi

gi.require_version("Atk", "1.0")
gi.require_version("Gtk", "3.0")
from gi.repository import Atk, GLib, GObject, Gtk  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("texts", nargs="+")
ap.add_argument("--delay", type=float, default=3.0)
ap.add_argument("--assertive", action="store_true")
ap.add_argument("--signal", default="announcement")
args = ap.parse_args()

win = Gtk.Window(title="announce-probe-gtk")
win.set_default_size(400, 120)
button = Gtk.Button(label="probe")
win.add(button)
win.connect("destroy", Gtk.main_quit)
win.show_all()
acc = win.get_accessible()
q = GObject.signal_query(GObject.signal_lookup(args.signal, acc.__gtype__))
print("segnale", args.signal, "parametri:", [t.name for t in q.param_types], flush=True)


def announce(text):
    politeness = Atk.Live.ASSERTIVE if args.assertive else Atk.Live.POLITE
    print("annuncio:", args.signal, repr(text), int(politeness), flush=True)
    # ATK 2.60: `announcement(text)` e `notification(text, urgenza)`; il ponte li gira entrambi in object:announcement
    acc.emit(args.signal, text, *([] if args.signal == "announcement" else [int(politeness)]))
    return False


for i, text in enumerate(args.texts):
    GLib.timeout_add(int((args.delay * (i + 1)) * 1000), announce, text)
GLib.timeout_add(int((args.delay * len(args.texts) + 5) * 1000), Gtk.main_quit)
Gtk.main()
