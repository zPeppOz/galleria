from pathlib import Path
import os
import subprocess
os.umask(0o077)
# Binary PKCS#12 import fixture, generated with the rest of the private lab material.
subprocess.run(["openssl", "pkcs12", "-export", "-inkey", "/pki/client.key", "-in", "/pki/client.crt",
                "-certfile", "/pki/ca.crt", "-out", "/out/client.p12", "-passout", "stdin"],
               input=b"\n", check=True, stdout=subprocess.DEVNULL)
for name, remote in [("base",10),("otp",10),("enckey",10),("unreachable",99),("dns-option",11),("split",13),("v6",14),("malicious",10)]:
    text=f"client\nproto udp\nremote 172.31.0.{remote} 1194\nnobind\nremote-cert-tls server\nauth-user-pass\nkey-direction 1\n"
    for kind,file in [("ca","ca.crt"),("cert","client.crt"),("key","client-enc.key" if name=="enckey" else "client.key"),("tls-auth","ta.key")]:
        text+=f"<{kind}>\n"+Path("/pki",file).read_text()+f"</{kind}>\n"
    if name=="otp": text+="static-challenge \"Enter OTP\" 1\n"
    if name=="malicious": text+="dev tun\nverb 3\nscript-security 3\nup /usr/bin/touch /tmp/PWNED-up\nplugin /tmp/PWNED-plugin\nmanagement 0.0.0.0 7505\nstatus /tmp/PWNED-status\nwritepid /tmp/PWNED-writepid\nsetenv EVIL 1\n"
    Path("/out",name+".ovpn").write_text(text)
