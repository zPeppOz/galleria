#!/usr/bin/env python3
"""Talks to the single-instance socket of galleria-gui as whoever runs this, and prints one line (lab/tests/gui-socket-owner.sh).

usage: gui-socket-client.py SOCKET FILE

  uid=N refused errno=E      the connection was not made (the modes of the folder or of the socket)
  uid=N answer=b'ok\\n'      the first window took the file
  uid=N closed <reason>      the connection was made and closed with no answer (the first window refuses the user)
"""
import json
import os
import socket
import sys

path, target = sys.argv[1], sys.argv[2]
who = f"uid={os.geteuid()}"
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(4)
try:
    s.connect(path)
except OSError as e:
    print(f"{who} refused errno={e.errno}")
    sys.exit(0)
try:
    s.sendall((json.dumps({"v": 1, "files": [target]}) + "\n").encode())
    answer = s.recv(16)
except OSError as e:
    print(f"{who} closed {e.__class__.__name__}")
    sys.exit(0)
print(f"{who} answer={answer!r}" if answer else f"{who} closed EOF")
