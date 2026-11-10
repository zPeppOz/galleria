#!/bin/bash
# Verifica utente/password (+ OTP TOTP per bob, formato SCRV1:b64(pw):b64(otp)).
f="$1"; user="$(sed -n 1p "$f")"; pass="$(sed -n 2p "$f")"
log() { echo "verify.sh: user=$user $*" >&2; }
case "$user" in
  alice) [ "$pass" = "alice-secret" ] && { log "ok (password)"; exit 0; } ;;
  bob)
    if [[ "$pass" == SCRV1:* ]]; then
      IFS=: read -r _ b64pw b64otp <<<"$pass"
      pw="$(echo "$b64pw" | base64 -d)"; otp="$(echo "$b64otp" | base64 -d)"
      if [ "$pw" = "bob-secret" ] && oathtool --totp -b -w 1 JBSWY3DPEHPK3PXP "$otp" >/dev/null 2>&1; then
        log "ok (password + totp $otp)"; exit 0
      fi
      log "bad password or otp ($otp)"; exit 1
    fi ;;
esac
log "rejected"; exit 1
