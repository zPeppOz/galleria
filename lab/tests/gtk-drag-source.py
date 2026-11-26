#!/usr/bin/env python3
"""A real drag of a file onto the window of Galleria, for the screen reader check (plan B4, T4, S28).

    gtk-drag-source.py PATH [--cancel]

A small GTK 3 window is put in the corner of the screen. It offers PATH as `text/uri-list`, which is what a file manager
offers on X11 when a file is dragged out of it. The mouse is then pressed on it, moved over the window of Galleria and
released there with `Atspi.generate_mouse_event` (`b1p`, `abs`, `b1r`: the events go through XTest, so they are real
pointer events for the X server and the toolkit). The drag that follows is the XDND protocol between two real windows,
the one that `wry` listens to (`drag-data-received`), not a message of the page.

With `--cancel` the button is released outside of any window that takes a drop (the corner of the screen): the drag
ends with nothing dropped.

Prints one JSON line: {"source": [x, y], "target": [x, y], "ended": bool, "dropped": bool}. `dropped` is what the
source was told when the drag ended (a target accepted the file). Exit 0 when the drag ended, 3 when the window of
Galleria is not on the AT-SPI desktop, 4 when the drag never ended.
"""
import json
import sys
import time

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, Gdk, GLib, Gtk  # noqa: E402

WAIT_SECONDS = 20
STEPS = 24
STEP_MS = 40


def galleria_frame():
    """The window of Galleria on the AT-SPI desktop, or None."""
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app is None or "galleria" not in (app.get_name() or "").lower():
            continue
        for j in range(app.get_child_count()):
            window = app.get_child_at_index(j)
            if window is not None and window.get_role_name() == "frame":
                return window
    return None


def centre_of(frame):
    box = frame.get_component_iface().get_extents(Atspi.CoordType.SCREEN)
    return box.x + box.width // 2, box.y + box.height // 2


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    cancel = "--cancel" in sys.argv[1:]
    if len(args) != 1:
        print("usage: gtk-drag-source.py PATH [--cancel]", file=sys.stderr)
        return 2
    uri = GLib.filename_to_uri(args[0], None)

    deadline = time.time() + WAIT_SECONDS
    frame = None
    while time.time() < deadline:
        frame = galleria_frame()
        if frame is not None:
            break
        time.sleep(0.2)
    if frame is None:
        print("the window of Galleria is not on the AT-SPI desktop", file=sys.stderr)
        return 3
    target = centre_of(frame)

    window = Gtk.Window(title="drag-source")
    window.set_default_size(100, 60)
    window.set_decorated(False)
    window.move(1170, 725)
    label = Gtk.Label(label="drag")
    box = Gtk.EventBox()
    box.add(label)
    window.add(box)
    box.drag_source_set(Gdk.ModifierType.BUTTON1_MASK, [Gtk.TargetEntry.new("text/uri-list", 0, 0)], Gdk.DragAction.COPY)
    result = {"ended": False, "dropped": False, "failed": False}

    def on_data_get(_widget, _context, data, _info, _time):
        data.set_uris([uri])

    def on_drag_failed(_widget, _context, reason):
        # no target took the file (or the drag was cancelled): GTK would animate the file going back
        result["failed"] = True
        return True

    def on_drag_end(_widget, _context):
        result["ended"] = True
        result["dropped"] = not result["failed"]
        GLib.timeout_add(300, Gtk.main_quit)

    box.connect("drag-data-get", on_data_get)
    box.connect("drag-failed", on_drag_failed)
    box.connect("drag-end", on_drag_end)

    path = []

    def plan(source):
        end = (1275, 795) if cancel else target
        path.append(("abs", source[0], source[1]))
        path.append(("b1p", source[0], source[1]))
        for n in range(1, STEPS + 1):
            x = source[0] + (end[0] - source[0]) * n // STEPS
            y = source[1] + (end[1] - source[1]) * n // STEPS
            path.append(("abs", x, y))
        # a last pause over the target, then the button goes up
        path.append(("abs", end[0], end[1]))
        path.append(("b1r", end[0], end[1]))

    def step():
        if not path:
            return False
        name, x, y = path.pop(0)
        Atspi.generate_mouse_event(x, y, name)
        return True

    def start():
        alloc = box.get_allocation()
        origin = window.get_window().get_origin()
        source = (origin[1] + alloc.width // 2, origin[2] + alloc.height // 2)
        result["source"] = list(source)
        result["target"] = list(target)
        plan(source)
        GLib.timeout_add(STEP_MS, step)
        return False

    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    # the window must be mapped and at its place before the mouse goes to it
    GLib.timeout_add(1000, start)
    GLib.timeout_add_seconds(WAIT_SECONDS, Gtk.main_quit)
    Gtk.main()
    print(json.dumps(result))
    return 0 if result["ended"] else 4


if __name__ == "__main__":
    sys.exit(main())
