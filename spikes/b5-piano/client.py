#!/usr/bin/env python3
"""Probe: connect to a Unix socket, send a line, print the answer, close."""
import os, socket, sys

s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
try:
    s.connect(sys.argv[1])
except OSError as e:
    print(f"connect failed as uid={os.getuid()} groups={os.getgroups()}: {e}")
    sys.exit(1)
s.sendall(b"hello\n")
print(f"uid={os.getuid()} groups={os.getgroups()} got {s.recv(4096)!r}")
s.close()
