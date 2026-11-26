#!/usr/bin/env python3
"""Piano B6, sonda: il menu di una tray AppIndicator si legge e si clicca dal bus di sessione, senza schermo vero?

Un processo fa da `org.kde.StatusNotifierWatcher` (finto), un altro apre un AppIndicator GTK3 con un menu di tre voci.
Il primo legge il menu con `com.canonical.dbusmenu.GetLayout` e clicca una voce con `Event(id, "clicked", …)`.
Uso: dbus-run-session -- xvfb-run -a python3 tray-probe.py [--no-watcher]
"""
import os, subprocess, sys, time
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gio, GLib

APP = r'''
import gi, sys
gi.require_version("Gtk", "3.0"); gi.require_version("AyatanaAppIndicator3", "0.1")
from gi.repository import Gtk, AyatanaAppIndicator3 as AI
ind = AI.Indicator.new("galleria-probe", "network-vpn", AI.IndicatorCategory.APPLICATION_STATUS)
ind.set_status(AI.IndicatorStatus.ACTIVE)
menu = Gtk.Menu()
def add(label, enabled=True):
    item = Gtk.MenuItem(label=label); item.set_sensitive(enabled)
    item.connect("activate", lambda i: (print("CLICKED " + i.get_label(), flush=True)))
    menu.append(item); return item
add("Disconnected", False); first = add("Connect Ufficio"); add("Quit")
first.connect("activate", lambda i: i.set_label("Disconnect Ufficio"))
menu.show_all(); ind.set_menu(menu)
print("READY", flush=True)
Gtk.main()
'''

WATCHER_XML = """<node><interface name='org.kde.StatusNotifierWatcher'>
<method name='RegisterStatusNotifierItem'><arg type='s' direction='in'/></method>
<method name='RegisterStatusNotifierHost'><arg type='s' direction='in'/></method>
<property name='RegisteredStatusNotifierItems' type='as' access='read'/>
<property name='IsStatusNotifierHostRegistered' type='b' access='read'/>
<property name='ProtocolVersion' type='i' access='read'/>
<signal name='StatusNotifierItemRegistered'><arg type='s'/></signal>
<signal name='StatusNotifierHostRegistered'/>
</interface></node>"""

items = []
loop = GLib.MainLoop()
bus = Gio.bus_get_sync(Gio.BusType.SESSION)

def on_call(conn, sender, path, iface, method, params, inv):
    if method == "RegisterStatusNotifierItem":
        (service,) = params.unpack()
        # ayatana passes an object path; the bus name is the sender
        items.append((sender, service if service.startswith("/") else "/StatusNotifierItem"))
        print(f"REGISTERED sender={sender} arg={service}", flush=True)
    inv.return_value(None)

def on_get(conn, sender, path, iface, prop):
    return {"RegisteredStatusNotifierItems": GLib.Variant("as", [s + p for s, p in items]),
            "IsStatusNotifierHostRegistered": GLib.Variant("b", True),
            "ProtocolVersion": GLib.Variant("i", 0)}[prop]

if "--no-watcher" not in sys.argv:
    node = Gio.DBusNodeInfo.new_for_xml(WATCHER_XML)
    bus.register_object("/StatusNotifierWatcher", node.interfaces[0], on_call, on_get, None)
    Gio.bus_own_name_on_connection(bus, "org.kde.StatusNotifierWatcher", Gio.BusNameOwnerFlags.NONE, None, None)

app = subprocess.Popen([sys.executable, "-c", APP], stdout=subprocess.PIPE, text=True)
print("app:", app.stdout.readline().strip(), flush=True)

def pump(seconds):
    GLib.timeout_add(int(seconds * 1000), loop.quit); loop.run()

def layout(sender, menu):
    r = bus.call_sync(sender, menu, "com.canonical.dbusmenu", "GetLayout",
                      GLib.Variant("(iias)", (0, -1, ["label", "enabled", "visible"])), None, 0, 3000, None)
    rev, (root_id, props, children) = r.unpack()
    return [(c[0], c[1].get("label"), c[1].get("enabled", True)) for c in children]

pump(2)
if not items:
    print("NO ITEM REGISTERED (no watcher?)", flush=True)
    names = bus.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "ListNames", None, None, 0, 3000, None).unpack()[0]
    print("names on the bus:", [n for n in names if not n.startswith("org.freedesktop")], flush=True)
    app.kill(); sys.exit(1)
sender, path = items[0]
menu = bus.call_sync(sender, path, "org.freedesktop.DBus.Properties", "Get",
                     GLib.Variant("(ss)", ("org.kde.StatusNotifierItem", "Menu")), None, 0, 3000, None).unpack()[0]
status = bus.call_sync(sender, path, "org.freedesktop.DBus.Properties", "Get",
                       GLib.Variant("(ss)", ("org.kde.StatusNotifierItem", "Status")), None, 0, 3000, None).unpack()[0]
icon = bus.call_sync(sender, path, "org.freedesktop.DBus.Properties", "Get",
                     GLib.Variant("(ss)", ("org.kde.StatusNotifierItem", "IconName")), None, 0, 3000, None).unpack()[0]
print(f"item path={path} menu={menu} status={status} icon={icon}", flush=True)
before = layout(sender, menu)
print("LAYOUT before:", before, flush=True)
target = [i for i, label, _ in before if label == "Connect Ufficio"][0]
bus.call_sync(sender, menu, "com.canonical.dbusmenu", "Event",
              GLib.Variant("(isvu)", (target, "clicked", GLib.Variant("i", 0), 0)), None, 0, 3000, None)
pump(1)
print("app:", app.stdout.readline().strip(), flush=True)
print("LAYOUT after:", layout(sender, menu), flush=True)
app.kill()
