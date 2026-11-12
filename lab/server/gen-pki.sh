#!/bin/bash
set -euo pipefail

    umask 077
    cd /pki
    ec() { openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$1"; }
    ec ca.key
    openssl req -x509 -new -key ca.key -sha256 -days 30 -subj "/CN=Galleria Lab CA" -out ca.crt
    for name in server client; do
      ec "$name.key"
      openssl req -new -key "$name.key" -subj "/CN=$name" -out "$name.csr"
      printf "extendedKeyUsage=%sAuth\nkeyUsage=digitalSignature\n" "$name" > "$name.ext"
      openssl x509 -req -in "$name.csr" -CA ca.crt -CAkey ca.key -CAcreateserial -days 30 -sha256 -extfile "$name.ext" -out "$name.crt"
    done
    printf "%s\n" "key-passphrase" | openssl pkey -in client.key -aes256 -passout stdin -out client-enc.key
    openvpn --genkey secret ta.key
