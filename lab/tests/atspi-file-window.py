#!/usr/bin/env python3
"""Drive the GTK file window of Rust («Save» of «Export», «Open» of «Import») over AT-SPI
(ui/e2e/*.a11y.ts, lab/tests/gui-a11y.sh).

    atspi-file-window.py save PATH    wait for the window, put PATH in the name field, activate «Save»
    atspi-file-window.py open PATH    wait for the window, bring up the field of the path, put PATH in it, activate «Open»
    atspi-file-window.py cancel       wait for the window, activate «Cancel»
    atspi-file-window.py dump         wait for the window and print the buttons, fields and filters it has (to debug)

The window is not part of the web page, so WebDriver cannot reach it (it belongs to Rust: the page never sees a path,
spec §3.2 rule 3). Among the windows of the application `galleria-gui` on the AT-SPI desktop it is the one with the
role `file chooser`. The field of the name (or of the path) is the first `text` node that can be edited; `set_text_contents`
accepts a full path. In «Save» that field is there from the start; in «Open» GTK 3 shows it only when it is asked for
(`Ctrl+L`, or typing `/`), so «open» asks first (S27). Prints one JSON line:
{"offered": <what the field held when it was found>, "pressed": "Save"|"Open"|"Cancel", "closed": bool}.
Exit 3 when the window never appeared, 4 when it has no field (even after asking for one), 5 when it has no such button.
"""
import json
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402

WAIT_SECONDS = 15
# the keysyms of the keys that are sent
KEY_CONTROL_L = 0xFFE3
KEY_L = 0x6C


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


def showing(node):
    return node.get_state_set().contains(Atspi.StateType.SHOWING)


def fields(window):
    """The fields that can be edited and are on the screen: «Open» has a hidden one for the search and a hidden one for the
    path, and only the one that was asked for counts."""
    return [n for n in descendants(window) if n.get_role_name() == "text" and n.get_editable_text_iface() is not None and showing(n)]


def buttons(window):
    return [n for n in descendants(window) if n.get_role_name() == "push button"]


def filters(window):
    """The names of the filters of the window: the combo box that holds them, and what it offers."""
    names = []
    for combo in (n for n in descendants(window) if n.get_role_name() == "combo box"):
        names.append(label(combo))
        names.extend(label(n) for n in descendants(combo) if label(n))
    return names


def press_keys(*steps):
    """Sends keys to the window that has the focus: (keysym, "press"|"release"|"tap") steps, in order."""
    kinds = {"press": Atspi.KeySynthType.PRESS, "release": Atspi.KeySynthType.RELEASE, "tap": Atspi.KeySynthType.PRESSRELEASE}
    for keysym, kind in steps:
        Atspi.generate_keyboard_event(keysym, None, kinds[kind] | Atspi.KeySynthType.SYM)
        time.sleep(0.05)


def wait_for_fields(window, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        found = fields(window)
        if found:
            return found
        time.sleep(0.2)
    return []


def bring_up_the_field(window):
    """«Open» has no field for the path until it is asked for. A `/` does it by itself (GTK opens the field with the slash
    in it); `Ctrl+L` is the shortcut, and goes second because the modifier of a synthetic key does not always arrive (the
    `L` alone is then a letter typed into the search of the list)."""
    Atspi.generate_keyboard_event(0, "/", Atspi.KeySynthType.STRING)
    found = wait_for_fields(window, 3)
    if found:
        return found
    press_keys((KEY_CONTROL_L, "press"), (KEY_L, "tap"), (KEY_CONTROL_L, "release"))
    return wait_for_fields(window, 3)


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode not in ("save", "open", "cancel", "dump") or (mode in ("save", "open")) != (len(sys.argv) == 3):
        print(__doc__, file=sys.stderr)
        return 2
    window = wait_for(True, WAIT_SECONDS)
    if window is None:
        print("the file window never appeared", file=sys.stderr)
        return 3
    entries = fields(window)
    if mode == "dump":
        print(json.dumps({"entries": [label(n) for n in entries], "buttons": [label(n) for n in buttons(window)], "filters": filters(window)}))
        return 0
    if mode == "open" and not entries:
        entries = bring_up_the_field(window)
    if mode != "cancel" and not entries:
        print("the file window has no field for the name or the path", file=sys.stderr)
        return 4
    offered = Atspi.Text.get_text(entries[0].get_text_iface(), 0, -1) if entries else ""
    wanted = {"save": "Save", "open": "Open", "cancel": "Cancel"}[mode]
    pressing = [n for n in buttons(window) if label(n) == wanted]
    if not pressing:
        print(f"the file window has no «{wanted}» button: {[label(n) for n in buttons(window)]}", file=sys.stderr)
        return 5
    if mode in ("save", "open"):
        Atspi.EditableText.set_text_contents(entries[0].get_editable_text_iface(), sys.argv[2])
        time.sleep(0.3)  # let the window take the text before the button is pressed
    pressing[0].get_action_iface().do_action(0)
    closed = wait_for(False, WAIT_SECONDS) is None
    answer = {"offered": offered, "pressed": wanted, "closed": closed}
    if not closed and mode != "cancel":
        # what a test needs to see to know why: the field as the window holds it now
        answer["field"] = Atspi.Text.get_text(entries[0].get_text_iface(), 0, -1)
    print(json.dumps(answer))
    return 0


if __name__ == "__main__":
    sys.exit(main())
