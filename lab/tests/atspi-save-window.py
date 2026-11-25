#!/usr/bin/env python3
"""Drive the GTK «Save» window of «Export» over AT-SPI (ui/e2e/rubrica.a11y.ts, lab/tests/gui-a11y.sh).

    atspi-save-window.py save PATH    wait for the window, put PATH in the name field, activate «Save»
    atspi-save-window.py cancel       wait for the window, activate «Cancel»
    atspi-save-window.py dump         wait for the window and print the buttons and fields it has (to debug)

The window is not part of the web page, so WebDriver cannot reach it (it belongs to Rust: the page never sees a path,
spec §3.2 rule 3). Among the windows of the application `galleria-gui` on the AT-SPI desktop it is the one with the
role `file chooser`. The field of the name is the first `text` node that can be edited; `set_text_contents` accepts a
full path. Prints one JSON line: {"offered": <the name the window proposed>, "pressed": "Save"|"Cancel", "closed": bool}.
Exit 3 when the window never appeared, 4 when it has no name field, 5 when it has no such button.
"""
import json
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

WAIT_SECONDS = 15


def application():
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        child = desktop.get_child_at_index(i)
        if child is not None and "galleria" in (child.get_name() or "").lower():
            return child
    return None


def chooser():
    root = application()
    if root is None:
        return None
    for i in range(root.get_child_count()):
        window = root.get_child_at_index(i)
        if window is not None and window.get_role_name() == "file chooser":
            return window
    return None


def descendants(node, depth=0):
    """Every node under `node`, in document order (an object that vanishes while we walk is skipped)."""
    found = []
    try:
        for i in range(node.get_child_count()):
            child = node.get_child_at_index(i)
            if child is not None and depth < 30:
                found.append(child)
                found.extend(descendants(child, depth + 1))
    except Exception:
        pass
    return found


def wait_for(found, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        window = chooser()
        if bool(window) == found:
            return window
        time.sleep(0.2)
    return chooser()


def label(node):
    return (node.get_name() or "").replace("_", "")


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode not in ("save", "cancel", "dump") or (mode == "save") != (len(sys.argv) == 3):
        print(__doc__, file=sys.stderr)
        return 2
    window = wait_for(True, WAIT_SECONDS)
    if window is None:
        print("the Save window never appeared", file=sys.stderr)
        return 3
    nodes = descendants(window)
    entries = [n for n in nodes if n.get_role_name() == "text" and n.get_editable_text_iface() is not None]
    buttons = [n for n in nodes if n.get_role_name() == "push button"]
    if mode == "dump":
        print(json.dumps({"entries": [label(n) for n in entries], "buttons": [label(n) for n in buttons]}))
        return 0
    if not entries:
        print("the Save window has no name field", file=sys.stderr)
        return 4
    offered = Atspi.Text.get_text(entries[0].get_text_iface(), 0, -1)
    wanted = "Save" if mode == "save" else "Cancel"
    pressing = [n for n in buttons if label(n) == wanted]
    if not pressing:
        print(f"the Save window has no «{wanted}» button: {[label(n) for n in buttons]}", file=sys.stderr)
        return 5
    if mode == "save":
        Atspi.EditableText.set_text_contents(entries[0].get_editable_text_iface(), sys.argv[2])
        time.sleep(0.3)  # let the window take the text before the button is pressed
    pressing[0].get_action_iface().do_action(0)
    closed = wait_for(False, WAIT_SECONDS) is None
    print(json.dumps({"offered": offered, "pressed": wanted, "closed": closed}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
