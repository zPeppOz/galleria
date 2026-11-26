#!/usr/bin/env python3
"""Dentro la VM, da `lab`, sessione grafica: crea il portachiavi predefinito («Login») che un'installazione desktop con
accesso a password ha già e che un autologin senza password non ha. Il prodotto non lo crea (set: «no default
collection»). Il prompt di GNOME (password nuova) lo compila chi guida la prova: questo script non scrive password.
Esce 0 se alla fine la collezione `default` esiste."""
import gi

gi.require_version("Secret", "1")
from gi.repository import Secret

svc = Secret.Service.get_sync(Secret.ServiceFlags.OPEN_SESSION, None)
coll = Secret.Collection.for_alias_sync(svc, "default", Secret.CollectionFlags.NONE, None)
if coll is None:
    coll = Secret.Collection.create_sync(svc, "Login", "default", Secret.CollectionCreateFlags.NONE, None)
print("default collection:", coll.get_label(), "locked" if coll.get_locked() else "unlocked")
