#!/usr/bin/env python3
"""Fotografia dello schermo di una VM dal suo display VNC (127.0.0.1:590N), senza librerie: RFB 3.8, senza password,
codifica raw. Uso: vncshot.py <porta> <file.png>"""
import socket
import struct
import sys
import zlib


def recvn(s, n):
    buf = b""
    while len(buf) < n:
        chunk = s.recv(n - len(buf))
        if not chunk:
            raise EOFError
        buf += chunk
    return buf


def png(w, h, rgb, dst):
    rows = b"".join(b"\x00" + rgb[y * w * 3:(y + 1) * w * 3] for y in range(h))

    def chunk(tag, body):
        c = struct.pack(">I", len(body)) + tag + body
        return c + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    with open(dst, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 6)) + chunk(b"IEND", b""))


def main():
    port, dst = int(sys.argv[1]), sys.argv[2]
    s = socket.create_connection(("127.0.0.1", port), timeout=10)
    recvn(s, 12)
    s.sendall(b"RFB 003.008\n")
    n = recvn(s, 1)[0]
    recvn(s, n)  # tipi di sicurezza
    s.sendall(b"\x01")  # None
    assert struct.unpack(">I", recvn(s, 4))[0] == 0
    s.sendall(b"\x01")  # shared
    w, h = struct.unpack(">HH", recvn(s, 4))
    recvn(s, 16)
    (nlen,) = struct.unpack(">I", recvn(s, 4))
    recvn(s, nlen)
    # 32 bpp little endian, r=16 g=8 b=0
    s.sendall(struct.pack(">BxxxBBBBHHHBBBxxx", 0, 32, 24, 0, 1, 255, 255, 255, 16, 8, 0))
    s.sendall(struct.pack(">BxHi", 2, 1, 0))  # raw
    s.sendall(struct.pack(">BBHHHH", 3, 0, 0, 0, w, h))
    img = bytearray(w * h * 3)
    got = 0
    while got < w * h:
        t = recvn(s, 1)[0]
        if t != 0:
            raise SystemExit(f"messaggio {t}")
        recvn(s, 1)
        (nrect,) = struct.unpack(">H", recvn(s, 2))
        for _ in range(nrect):
            x, y, rw, rh, enc = struct.unpack(">HHHHi", recvn(s, 12))
            if enc != 0:
                raise SystemExit(f"codifica {enc}")
            data = recvn(s, rw * rh * 4)
            for row in range(rh):
                line = data[row * rw * 4:(row + 1) * rw * 4]
                out = bytearray(rw * 3)
                out[0::3] = line[2::4]
                out[1::3] = line[1::4]
                out[2::3] = line[0::4]
                img[((y + row) * w + x) * 3:((y + row) * w + x + rw) * 3] = out
            got += rw * rh
    png(w, h, bytes(img), dst)


main()
