#!/usr/bin/env python3
"""Writes tray-off.png, tray-on.png and tray-busy.png (32x32, RGBA): a ring for «not connected», a filled disc for «connected»,
a disc with a gap for «in progress or error». Plain shapes so that the state reads on a light and on a dark panel.
Run it from this folder: `python3 make-tray-icons.py`. Only the standard library."""
import math
import struct
import zlib

SIZE = 32


def png(path, pixel):
    rows = []
    for y in range(SIZE):
        row = bytearray([0])
        for x in range(SIZE):
            row += bytes(pixel(x + 0.5, y + 0.5))
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    data += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    with open(path, "wb") as out:
        out.write(data)


def shape(colour, ring=False, gap=False):
    cx = cy = SIZE / 2

    def pixel(x, y):
        d = math.hypot(x - cx, y - cy)
        inside = d <= 13
        if ring and d < 8:
            inside = False
        if gap and abs(x - cx) < 2.5 and y < cy:
            inside = False
        return (*colour, 255) if inside else (0, 0, 0, 0)

    return pixel


png("tray-off.png", shape((140, 140, 150), ring=True))
png("tray-on.png", shape((46, 160, 90)))
png("tray-busy.png", shape((230, 160, 30), gap=True))
