#!/usr/bin/env python3
"""Probe: a Unix socket server that prints who connects (SO_PEERCRED, SO_PEERGROUPS), like galleriad does, and echoes lines."""
import os, socket, struct, sys, grp

path = sys.argv[1]
if os.path.exists(path):
    os.unlink(path)
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.bind(path)
os.chown(path, 0, grp.getgrnam("galleria").gr_gid)
os.chmod(path, 0o660)
s.listen(8)
print("listening", path, flush=True)
SO_PEERGROUPS = 59
while True:
    c, _ = s.accept()
    pid, uid, gid = struct.unpack("3i", c.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    raw = c.getsockopt(socket.SOL_SOCKET, SO_PEERGROUPS, 4 * 64)
    groups = list(struct.unpack(f"{len(raw)//4}I", raw))
    print(f"peer pid={pid} uid={uid} gid={gid} groups={groups}", flush=True)
    data = c.recv(4096)
    c.sendall(b"echo:" + data)
    # half-close behaviour: wait for the peer to close, then say so
    rest = c.recv(4096)
    print(f"peer closed (read {len(rest)} more bytes)", flush=True)
    c.close()
