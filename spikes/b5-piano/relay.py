#!/usr/bin/env python3
"""Prototype of the relay of Q42.

  relay.py listen SOCKET CMD...   host side: listens on SOCKET; for every connection runs CMD and joins the two byte streams
  relay.py join SOCKET            client side (run by CMD through `docker exec -i -u alice`): joins stdin/stdout to SOCKET

Bytes only: it never parses a frame. When one side closes, the other is closed too.
"""
import os, socket, subprocess, sys, threading


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

    def to_out(data):
        stdout.write(data)

    a = threading.Thread(target=pump, args=(stdin.read, s.sendall, lambda: s.shutdown(socket.SHUT_RDWR)), daemon=True)
    a.start()
    pump(s.recv, to_out, lambda: None)
    os._exit(0)


def listen(path, cmd):
    if os.path.exists(path):
        os.unlink(path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(path)
    os.chmod(path, 0o600)
    srv.listen(16)
    print("relay listening", path, flush=True)
    while True:
        c, _ = srv.accept()
        p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)

        def close_child(p=p):
            try:
                p.stdin.close()
            except OSError:
                pass

        def close_conn(c=c, p=p):
            try:
                c.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
            c.close()
            p.wait()

        threading.Thread(target=pump, args=(c.recv, p.stdin.write, close_child), daemon=True).start()
        threading.Thread(target=pump, args=(p.stdout.read, c.sendall, close_conn), daemon=True).start()


if sys.argv[1] == "join":
    join(sys.argv[2])
else:
    listen(sys.argv[2], sys.argv[3:])
