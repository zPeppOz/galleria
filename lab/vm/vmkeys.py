#!/usr/bin/env python3
"""Pilota una VM di lab/vm/vmctl.sh dal monitor qemu (/mnt/b/vm/<vm>.mon): tastiera vera. Le fotografie dello schermo si fanno con vncshot.py: con
virtio-vga e GNOME `screendump` restituisce la schermata vecchia dell'avvio.

    vmkeys.py <vm> key  <tasto> [<tasto>...]   sendkey di qemu, un tasto o una combinazione per argomento
                                               (es. tab  shift-tab  ret  spc  down  esc  alt-f4)
    vmkeys.py <vm> hmp  <comando...>           un comando grezzo del monitor

Uso: spike Orca (docs/08, «Spike Orca»): i tasti passano dalla tastiera virtuale, quindi da Mutter, come quelli di una persona.
Un'attesa di 0,6 s dopo ogni tasto lascia a Orca il tempo di parlare.
"""
import os
import socket
import sys
import time

B = "/mnt/b/vm"


def hmp(vm, command, wait=0.3):
    s = socket.socket(socket.AF_UNIX)
    s.connect(f"{B}/{vm}.mon")
    s.settimeout(wait + 3)
    time.sleep(0.1)
    try:
        s.recv(4096)
    except socket.timeout:
        pass
    s.sendall(command.encode() + b"\r\n")
    time.sleep(wait)
    out = b""
    try:
        while True:
            chunk = s.recv(65536)
            if not chunk:
                break
            out += chunk
            if len(chunk) < 65536:
                break
    except socket.timeout:
        pass
    s.close()
    return out.decode(errors="replace")


def main():
    vm, action, *rest = sys.argv[1:]
    if action == "key":
        for k in rest:
            hmp(vm, f"sendkey {k}")
            time.sleep(float(os.environ.get("KEY_WAIT", "0.6")))
    elif action == "hmp":
        print(hmp(vm, " ".join(rest)))
    else:
        sys.exit(__doc__)


main()
