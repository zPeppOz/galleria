#!/usr/bin/env python3
"""The tray of Galleria seen from the session bus (plan B6, T1; derived from spikes/b6-piano/tray-probe.py).

An AppIndicator is read and clicked without a screen and without a desktop: one process stands in for
`org.kde.StatusNotifierWatcher` (`watch`, stays up), the others read the menu with `com.canonical.dbusmenu.GetLayout`, click an
entry with `Event(id, "clicked")` and tell which icon is shown. The state of the watcher is a file in `$XDG_RUNTIME_DIR` (or
`$TRAY_PROBE_DIR`), so each command is a short process.

    tray-probe.py watch                  stays up as the watcher; without it nothing registers and `tray_is_shown` is false
    tray-probe.py layout                 the entries as JSON: [{"id", "text", "enabled"}, ...] (separators are left out)
    tray-probe.py click "<text>"         clicks the entry with exactly that text; exits 1 when there is none or it is disabled
    tray-probe.py icon                   `off`, `on` or `busy`, from the colour of the icon file that is shown
    tray-probe.py wait-item [seconds]    waits until the app has registered an item; exits 1 when it did not

The menu is the one the desktop would draw: the mnemonics of the labels (`_x`) are taken out, `__` is a literal `_`.
"""
import json
import os
import sys
import time

import gi

gi.require_version("GdkPixbuf", "2.0")
from gi.repository import Gio, GLib, GdkPixbuf  # noqa: E402

DIR = os.environ.get("TRAY_PROBE_DIR") or os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "tray-probe")
ITEMS = os.path.join(DIR, "items.json")
CALL_MS = 3000

WATCHER_XML = """<node><interface name='org.kde.StatusNotifierWatcher'>
<method name='RegisterStatusNotifierItem'><arg type='s' direction='in'/></method>
<method name='RegisterStatusNotifierHost'><arg type='s' direction='in'/></method>
<property name='RegisteredStatusNotifierItems' type='as' access='read'/>
<property name='IsStatusNotifierHostRegistered' type='b' access='read'/>
<property name='ProtocolVersion' type='i' access='read'/>
<signal name='StatusNotifierItemRegistered'><arg type='s'/></signal>
<signal name='StatusNotifierHostRegistered'/>
</interface></node>"""


def bus():
    return Gio.bus_get_sync(Gio.BusType.SESSION)


def split_item(sender, service):
    """The bus name and the object path of an item, from what it passed to RegisterStatusNotifierItem."""
    if service.startswith("/"):
        return sender, service
    if "/" in service:
        name, rest = service.split("/", 1)
        return name, "/" + rest
    return service or sender, "/StatusNotifierItem"


def watch():
    os.makedirs(DIR, exist_ok=True)
    items = []
    loop = GLib.MainLoop()
    connection = bus()

    def save():
        with open(ITEMS + ".tmp", "w") as out:
            json.dump(items, out)
        os.replace(ITEMS + ".tmp", ITEMS)

    def on_call(conn, sender, path, iface, method, params, invocation):
        if method == "RegisterStatusNotifierItem":
            (service,) = params.unpack()
            name, item_path = split_item(sender, service)
            items.append({"name": name, "path": item_path, "sender": sender, "service": service})
            save()
            print(f"registered {name} {item_path}", flush=True)
        invocation.return_value(None)

    def on_get(conn, sender, path, iface, prop):
        return {
            "RegisteredStatusNotifierItems": GLib.Variant("as", [i["name"] + i["path"] for i in items]),
            "IsStatusNotifierHostRegistered": GLib.Variant("b", True),
            "ProtocolVersion": GLib.Variant("i", 0),
        }[prop]

    node = Gio.DBusNodeInfo.new_for_xml(WATCHER_XML)
    connection.register_object("/StatusNotifierWatcher", node.interfaces[0], on_call, on_get, None)
    Gio.bus_own_name_on_connection(connection, "org.kde.StatusNotifierWatcher", Gio.BusNameOwnerFlags.NONE, None, None)
    save()
    print("watching", flush=True)
    loop.run()


def items():
    try:
        with open(ITEMS) as source:
            return json.load(source)
    except (OSError, ValueError):
        return []


def item():
    found = items()
    return found[-1] if found else None


def get(connection, name, path, interface, prop):
    reply = connection.call_sync(
        name, path, "org.freedesktop.DBus.Properties", "Get",
        GLib.Variant("(ss)", (interface, prop)), None, Gio.DBusCallFlags.NONE, CALL_MS, None,
    )
    return reply.unpack()[0]


def plain(label):
    """What the desktop shows of a label of dbusmenu: `_x` is the mnemonic (gone), `__` is a literal underscore."""
    out, i = "", 0
    while i < len(label):
        if label[i] == "_":
            if label[i + 1:i + 2] == "_":
                out += "_"
                i += 1
        else:
            out += label[i]
        i += 1
    return out


def layout():
    registered = item()
    if registered is None:
        raise SystemExit("no item registered")
    connection = bus()
    menu = get(connection, registered["name"], registered["path"], "org.kde.StatusNotifierItem", "Menu")
    reply = connection.call_sync(
        registered["name"], menu, "com.canonical.dbusmenu", "GetLayout",
        GLib.Variant("(iias)", (0, -1, ["label", "enabled", "visible", "type"])), None, Gio.DBusCallFlags.NONE, CALL_MS, None,
    )
    _, (_, _, children) = reply.unpack()
    entries = []
    for child in children:
        props = child[1]
        if props.get("type") == "separator" or props.get("visible", True) is False:
            continue
        entries.append({"id": child[0], "text": plain(props.get("label", "")), "enabled": bool(props.get("enabled", True))})
    return registered, menu, entries


def click(text):
    registered, menu, entries = layout()
    wanted = [e for e in entries if e["text"] == text]
    if not wanted:
        print(f"no entry {text!r}; the menu has {[e['text'] for e in entries]}", file=sys.stderr)
        return 1
    if not wanted[0]["enabled"]:
        print(f"the entry {text!r} is disabled", file=sys.stderr)
        return 1
    try:
        bus().call_sync(
            registered["name"], menu, "com.canonical.dbusmenu", "Event",
            GLib.Variant("(isvu)", (wanted[0]["id"], "clicked", GLib.Variant("i", 0), int(time.time()))), None,
            Gio.DBusCallFlags.NONE, CALL_MS, None,
        )
    except GLib.Error as failure:
        # «Quit» ends the process while it handles the click: sometimes the app is gone from the bus before the reply is sent
        # («NoReply: Message recipient disconnected from message bus without replying»). That is the answer of an app that quit, and the
        # test that clicked asks the process whether it is gone. No other entry, and no other error, is excused.
        if not (text.startswith("Quit") and failure.matches(Gio.dbus_error_quark(), Gio.DBusError.NO_REPLY)):
            raise
    return 0


def icon():
    registered = item()
    if registered is None:
        raise SystemExit("no item registered")
    connection = bus()
    interface = "org.kde.StatusNotifierItem"
    name = get(connection, registered["name"], registered["path"], interface, "IconName")
    theme = get(connection, registered["name"], registered["path"], interface, "IconThemePath")
    path = name if os.path.isabs(name) else os.path.join(theme, name + ".png") if theme else name
    if not os.path.exists(path):
        # the name can already carry the extension, or be a file of the folder
        candidates = [os.path.join(theme, f) for f in os.listdir(theme) if f.startswith(name)] if theme and os.path.isdir(theme) else []
        if not candidates:
            raise SystemExit(f"no icon file for {name!r} in {theme!r}")
        path = candidates[0]
    pixbuf = GdkPixbuf.Pixbuf.new_from_file(path)
    data, channels = pixbuf.get_pixels(), pixbuf.get_n_channels()
    r = g = b = n = 0
    for i in range(0, len(data), channels):
        if channels == 4 and data[i + 3] < 128:
            continue
        r, g, b, n = r + data[i], g + data[i + 1], b + data[i + 2], n + 1
    if n == 0:
        raise SystemExit("the icon is empty")
    r, g, b = r / n, g / n, b / n
    if g > r + 40 and g > b + 40:
        return "on"
    if r > 180 and r > b + 80:
        return "busy"
    return "off"


def main(argv):
    command = argv[1] if len(argv) > 1 else ""
    if command == "watch":
        watch()
    elif command == "layout":
        print(json.dumps(layout()[2], ensure_ascii=False))
    elif command == "click" and len(argv) > 2:
        sys.exit(click(argv[2]))
    elif command == "icon":
        print(icon())
    elif command == "wait-item":
        deadline = time.time() + (float(argv[2]) if len(argv) > 2 else 10.0)
        while time.time() < deadline:
            if item() is not None:
                return
            time.sleep(0.1)
        sys.exit(1)
    else:
        print(__doc__, file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main(sys.argv)
