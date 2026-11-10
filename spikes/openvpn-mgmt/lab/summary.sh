#!/bin/bash
# Riassunto delle prove per versione (usato per scrivere docs/04)
cd "$(dirname "$0")/../evidence"
for v in 26 27 79; do
  d=ovpn$v; echo "################ $d: $(grep -m1 -o 'OpenVPN 2[^ ]*' $d/2.1.txt)"
  echo "2.1 control PWNED: $(grep -c 'PWNED-' <(sed -n '/A) controllo/,/B) demone/p' $d/2.1.txt | grep '^-rw'))  sanitized: $(sed -n '/B) demone/,$p' $d/2.1.txt | grep -c '^-rw')  removed: $(grep -c profile_directive_removed $d/2.1-sanitized.log)"
  echo "2.2 alice: $(grep -m1 -o 'Permission denied' $d/2.2.txt)  D: $(grep -c 'connection rejected' $d/2.2.txt)"
  echo "2.4 states: $(grep -o '"state":"[A-Z_]*"' $d/2.4.log | cut -d'"' -f4 | uniq | tr '\n' ' ') bytecount: $(grep -c '"event":"bytecount"' $d/2.4.log) last: $(grep '"event":"bytecount"' $d/2.4.log | tail -1 | cut -c1-60) dco: $(grep -o 'Kernel support for ovpn-dco missing' $d/2.4.log | head -1)"
  for t in auth otp keypass; do echo "2.5 $t: $(grep -c '"event":"connected"' $d/2.5-$t.log) $(grep -o 'Need .[A-Za-z ]*. [a-z/]*[^"]*' $d/2.5-$t.log | tr '\n' '|')"; done
  echo "2.6 pipe scan: $(grep -m1 'file con materiale' $d/2.6.txt)  sighup: $(grep -m1 -o 'Options error[^"]*' $d/2.6-pipe.log) retry: $(grep -c '"event":"retry"' $d/2.6-pipe.log) reconnect: $(grep -c '"event":"connected"' $d/2.6-pipe.log)"
  echo "2.6 extkey: $(grep -c pk_sign_done $d/2.6-extkey.log) connected: $(grep -c '"event":"connected"' $d/2.6-extkey.log) $(grep -o '"algorithm":"[^"]*"' $d/2.6-extkey.log | head -1)"
  echo "2.6 tempfile removed: $(grep -c tempfile_removed $d/2.6-tempfile.log)"
  for n in dhcpopt dnsopt; do
    echo "2.7 $n env: $(grep -m1 '"script_type":"up"' $d/2.7-$n.log | grep -o '"dns_channels":\[[^]]*\]') resolvectl ok: $(grep -c '"ok":true' $d/2.7-$n.log) redirect var: $(grep -m1 '"script_type":"up"' $d/2.7-$n.log | grep -o '"[a-z_]*redirect[a-z_0-9]*":"[0-9]"')"
    echo "2.7 $n noup dns-updown: $(grep -c 'libexec/openvpn/dns-updown' $d/2.7-$n-noup.log)  withup: $(grep -c 'libexec/openvpn/dns-updown' $d/2.7-$n.log)"
  done
  echo "2.7 resolvectl query: $(grep -c -- '-- link: tun0' $d/2.7.txt)"
  for m in off native native6 common reject; do
    [ -f $d/2.8-$m.txt ] || continue
    echo "2.8 $m: pkts=$(grep -o 'totale: [0-9]*' $d/2.8-$m.txt) curl=$(grep -o '^remote=[^ ]*' $d/2.8-$m.txt | sort | uniq -c | tr -s ' ' | tr '\n' ';') maxtotal=$(grep -o 'total=[0-9.]*' $d/2.8-$m.txt | cut -d= -f2 | sort -n | tail -1) seq=$(grep -o 'peer=.*' $d/2.8-$m.txt)"
  done
  echo "2.9 badpass: $(grep -o '"kind":"auth_failed"' $d/2.9-badpass.log) attempts=$(grep -c '"event":"attempt"' $d/2.9-badpass.log)  badotp: $(grep -o '"kind":"auth_failed"' $d/2.9-badotp.log)  unreachable: $(grep -c '"event":"attempt"' $d/2.9-unreachable.log) attempts, backoff $(grep -o '"after_secs":[0-9]*' $d/2.9-unreachable.log | cut -d: -f2 | tr '\n' ' ') $(grep -o '"kind":"gave_up"' $d/2.9-unreachable.log) $(grep -o 'durata [0-9]*s' $d/2.9.txt)"
  echo "mgmt version: $(grep -m1 -o 'Management Interface Version [0-9]' $d/2.4.log)"
done
