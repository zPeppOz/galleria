#!/bin/bash
# Conta i pacchetti IPv6 globali di una cattura (stesso filtro dello spike Linux), divisi per destinazione.
# Uso: count6.sh <file.pcapng>   (tcpdump gira nel container del laboratorio)
f="$(readlink -f "$1")"
docker run --rm -v "$(dirname "$f"):/e:ro" galleria-lab-server bash -c "
  F='ip6 and not net fe80::/10 and not net ff00::/8'
  echo \"totale: \$(tcpdump -nn -r /e/$(basename "$f") \"\$F\" 2>/dev/null | wc -l)\"
  echo \"verso il sito di prova 2001:db8:99::1: \$(tcpdump -nn -r /e/$(basename "$f") \"\$F and dst host 2001:db8:99::1\" 2>/dev/null | wc -l)\"
  echo 'per sorgente > destinazione:'
  tcpdump -nn -r /e/$(basename "$f") \"\$F\" 2>/dev/null | awk '{print \$3, \$4, \$5}' | sed -E 's/\.[0-9]+ > / > /; s/\.[0-9]+:\$//' | sort | uniq -c
"
