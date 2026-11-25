#!/usr/bin/env python3
"""Emette il segnale AT-SPI `object:announcement` sul bus dell'accessibilità, da un programma fuori dall'app
(spike Orca, docs/08, «Spike Orca»; vale quanto dice la prova sulla VM, non una promessa del prodotto).

    atspi-announce.py [--polite|--assertive] [--path OGGETTO] [--hold SEC] "testo da annunciare"

Il segnale è `org.a11y.atspi.Event.Object.Announcement` (firma `siiva{sv}`): detail = "", detail1 = urgenza
(1 = polite, 2 = assertive: `Atspi.Live`), detail2 = 0, any_data = il testo. Il mittente è la connessione di questo
programma sul bus (non si può falsificare), il percorso è `--path` (default: la radice di un'app AT-SPI,
`/org/a11y/atspi/accessible/root`). Con `--register` il programma si registra anche come applicazione AT-SPI
minima (Socket.Embed sul registro), così Orca può risolvere l'oggetto sorgente; `--hold` tiene la connessione
aperta qualche secondo dopo l'emissione.
"""
import argparse
import threading
import time

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument("text")
ap.add_argument("--assertive", action="store_true")
ap.add_argument("--polite", action="store_true")
ap.add_argument("--path", default="/org/a11y/atspi/accessible/root")
ap.add_argument("--hold", type=float, default=0.5)
ap.add_argument("--register", action="store_true")
args = ap.parse_args()

session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
addr = session.call_sync("org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus", "GetAddress", None, GLib.VariantType("(s)"), 0, -1, None).unpack()[0]
conn = Gio.DBusConnection.new_for_address_sync(addr, Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION, None, None)
# le chiamate del registro e di Orca (GetName, GetState...) vanno servite mentre il thread principale aspetta
threading.Thread(target=GLib.MainLoop().run, daemon=True).start()
print("bus a11y:", addr, "mittente:", conn.get_unique_name())

if args.register:
    # Un'app AT-SPI minima: un oggetto radice che implementa org.a11y.atspi.Accessible (Name, Role, ChildCount...) e
    # org.a11y.atspi.Application, e il Socket.Embed che la mette sotto la radice del registro.
    XML = """<node>
      <interface name="org.a11y.atspi.Accessible">
        <property name="Name" type="s" access="read"/><property name="Description" type="s" access="read"/>
        <property name="Parent" type="(so)" access="read"/><property name="ChildCount" type="i" access="read"/>
        <property name="Locale" type="s" access="read"/><property name="AccessibleId" type="s" access="read"/>
        <method name="GetRole"><arg type="u" direction="out"/></method>
        <method name="GetRoleName"><arg type="s" direction="out"/></method>
        <method name="GetLocalizedRoleName"><arg type="s" direction="out"/></method>
        <method name="GetState"><arg type="au" direction="out"/></method>
        <method name="GetAttributes"><arg type="a{ss}" direction="out"/></method>
        <method name="GetApplication"><arg type="(so)" direction="out"/></method>
        <method name="GetChildAtIndex"><arg type="i" direction="in"/><arg type="(so)" direction="out"/></method>
        <method name="GetChildren"><arg type="a(so)" direction="out"/></method>
        <method name="GetIndexInParent"><arg type="i" direction="out"/></method>
        <method name="GetRelationSet"><arg type="a(ua(so))" direction="out"/></method>
        <method name="GetInterfaces"><arg type="as" direction="out"/></method>
      </interface>
      <interface name="org.a11y.atspi.Application">
        <property name="ToolkitName" type="s" access="read"/><property name="Version" type="s" access="read"/>
        <property name="AtspiVersion" type="s" access="read"/><property name="Id" type="i" access="readwrite"/>
        <method name="GetLocale"><arg type="u" direction="in"/><arg type="s" direction="out"/></method>
        <method name="GetApplicationBusAddress"><arg type="s" direction="out"/></method>
      </interface></node>"""
    me = conn.get_unique_name()
    root = ("%s" % me, "/org/a11y/atspi/accessible/root")
    props = {"Name": GLib.Variant("s", "announce-probe"), "Description": GLib.Variant("s", ""), "Parent": GLib.Variant("(so)", ("", "/org/a11y/atspi/null")),
             "ChildCount": GLib.Variant("i", 0), "Locale": GLib.Variant("s", "en"), "AccessibleId": GLib.Variant("s", ""),
             "ToolkitName": GLib.Variant("s", "probe"), "Version": GLib.Variant("s", "0"), "AtspiVersion": GLib.Variant("s", "2.1"), "Id": GLib.Variant("i", -1)}

    def on_method(c, sender, path, iface, method, params, inv):
        v = {"GetRole": ("(u)", (75,)), "GetRoleName": ("(s)", ("application",)), "GetLocalizedRoleName": ("(s)", ("application",)),
             "GetState": ("(au)", ([0, 0],)), "GetAttributes": ("(a{ss})", ({},)), "GetApplication": ("(so)", root), "GetChildren": ("(a(so))", ([],)),
             "GetIndexInParent": ("(i)", (-1,)), "GetRelationSet": ("(a(ua(so)))", ([],)), "GetInterfaces": ("(as)", (["org.a11y.atspi.Accessible", "org.a11y.atspi.Application"],)),
             "GetLocale": ("(s)", ("en",)), "GetApplicationBusAddress": ("(s)", ("",)), "GetChildAtIndex": None}.get(method)
        if v is None:
            inv.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", "nope")
        else:
            inv.return_value(GLib.Variant(v[0], v[1]))

    def on_get(c, sender, path, iface, prop):
        return props[prop]

    node = Gio.DBusNodeInfo.new_for_xml(XML)
    for i in node.interfaces:
        conn.register_object("/org/a11y/atspi/accessible/root", i, on_method, on_get, None)
    reply = conn.call_sync("org.a11y.atspi.Registry", "/org/a11y/atspi/accessible/root", "org.a11y.atspi.Socket", "Embed",
                           GLib.Variant("((so))", (root,)), GLib.VariantType("((so))"), 0, 3000, None)
    print("registrata come applicazione, padre:", reply.unpack())
    time.sleep(1.0)

body = GLib.Variant("(siiva{sv})", ("", 2 if args.assertive else 1, 0, GLib.Variant("s", args.text), {}))
conn.emit_signal(None, args.path, "org.a11y.atspi.Event.Object", "Announcement", body)
conn.flush_sync(None)
print(f"emesso Announcement({'assertive' if args.assertive else 'polite'}) su {args.path}: {args.text!r}")
time.sleep(args.hold)
