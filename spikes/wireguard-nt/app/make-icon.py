#!/usr/bin/env python3
"""Genera icons/icon.ico (32x32, tinta unita) per l'app Tauri dello spike: serve solo a far passare la build."""
import struct, sys

W = H = 32
pixels = b"".join(struct.pack("<BBBB", 0x8A, 0x4B, 0x1F, 0xFF) for _ in range(W * H))  # BGRA
and_mask = b"\x00" * (((W + 31) // 32) * 4 * H)
dib = struct.pack("<IiiHHIIiiII", 40, W, H * 2, 1, 32, 0, len(pixels) + len(and_mask), 0, 0, 0, 0) + pixels + and_mask
ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", W, H, 0, 0, 1, 32, len(dib), 22) + dib
open(sys.argv[1], "wb").write(ico)
