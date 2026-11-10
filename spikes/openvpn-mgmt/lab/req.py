#!/usr/bin/env python3
"""Costruisce la richiesta di connect (quello che la GUI manderebbe via IPC) e la stampa su stdout.
uso: req.py profilo.ovpn chiave=valore ...   (otp=totp:SEGRETO calcola il TOTP corrente)"""
import json, subprocess, sys

req = {"profile": open(sys.argv[1]).read()}
for kv in sys.argv[2:]:
    k, v = kv.split("=", 1)
    if k in ("max_attempts", "hold_secs", "mgmt_connect_delay_secs"):
        v = int(v)
    elif v.startswith("["):
        v = json.loads(v)
    elif v in ("true", "false"):
        v = v == "true"
    elif k == "otp" and v.startswith("totp:"):
        v = subprocess.check_output(["docker", "exec", "gl-server", "oathtool", "--totp", "-b", v[5:]], text=True).strip()
    req[k] = v
print(json.dumps(req))
