#!/usr/bin/env python3
"""The bridge of the window to the real daemon of a client of the laboratory (plan B5, T9, Q42 (b)).

The window runs in the `galleria-gui-dev` image, the daemon in a client of the laboratory, and the daemon decides who may command
it from the uid and the groups of whoever opens its socket. So the window does not open the socket of the client: it opens a
socket on the side of the host, and every connection that arrives there is joined, byte for byte, to the daemon socket through
`docker exec -i -u <user> <client> python3 gui-daemon-relay.py join <socket>`. The daemon sees `<user>` of the client, with its
uid and its groups, and never the user of the image.

  gui-daemon-relay.py listen SOCKET CMD...   on the host: listens on SOCKET (0600); for every connection it runs CMD and joins the
                                             two byte streams
  gui-daemon-relay.py join SOCKET            in the client (CMD runs it): joins its stdin and stdout to the socket SOCKET

This is test code: it is not in a crate and in no package, and it never reads a frame, so nothing of what passes is kept or
written anywhere. When one side closes the other is closed too.
"""
import os
import signal
import socket
import subprocess
import sys
import threading


def pump(read, write, done):
    try:
        while True:
            data = read(65536)
            if not data:
                break
            write(data)
    except OSError:
        pass
    finally:
        done()


def join(path):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(path)
    stdin, stdout = sys.stdin.buffer.raw, sys.stdout.buffer.raw

    def half_close():
        try:
            s.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass

    threading.Thread(target=pump, args=(stdin.read, s.sendall, half_close), daemon=True).start()
    pump(s.recv, stdout.write, lambda: None)
    os._exit(0)


def listen(path, cmd):
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    os.chmod(path, 0o600)
    srv.listen(16)

    def stop(*_):
        try:
            os.unlink(path)
        except OSError:
            pass
        os._exit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    print("relay listening", path, flush=True)
    while True:
        conn, _ = srv.accept()
        child = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)

        def close_child(child=child):
            try:
                child.stdin.close()
            except OSError:
                pass

        def close_conn(conn=conn, child=child):
            try:
                conn.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            conn.close()
            child.wait()

        threading.Thread(target=pump, args=(conn.recv, child.stdin.write, close_child), daemon=True).start()
        threading.Thread(target=pump, args=(child.stdout.read, conn.sendall, close_conn), daemon=True).start()


if len(sys.argv) >= 3 and sys.argv[1] == "join":
    join(sys.argv[2])
elif len(sys.argv) >= 4 and sys.argv[1] == "listen":
    listen(sys.argv[2], sys.argv[3:])
else:
    sys.exit(__doc__)
