#!/bin/bash
# PKI di prova (EC P-256). Usa l'immagine del server per avere openssl/openvpn.
set -euo pipefail
cd "$(dirname "$0")"; mkdir -p pki
docker run --rm -v "$PWD/pki:/pki" galleria-lab-server bash -euc '
cd /pki
ec() { openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$1"; }
ec ca.key
openssl req -x509 -new -key ca.key -sha256 -days 30 -subj "/CN=Galleria Lab CA" -out ca.crt
mk() { # $1 nome, $2 estensioni
  ec "$1.key"
  openssl req -new -key "$1.key" -subj "/CN=$1" -out "$1.csr"
  printf "%s\n" "$2" > "$1.ext"
  openssl x509 -req -in "$1.csr" -CA ca.crt -CAkey ca.key -CAcreateserial -days 30 -sha256 -extfile "$1.ext" -out "$1.crt"
}
mk server "extendedKeyUsage=serverAuth
keyUsage=digitalSignature"
mk client "extendedKeyUsage=clientAuth
keyUsage=digitalSignature"
# chiave privata del client cifrata con passphrase (criterio 2.5)
openssl pkey -in client.key -aes256 -passout pass:key-passphrase -out client-enc.key
openvpn --genkey secret ta.key
chmod 644 /pki/*
'
