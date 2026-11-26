#!/usr/bin/env bash
# The owner check of the single-instance channel, with real users (plan B4, review defect 2; scripts/gui.sh socket-owner).
#
# crates/galleria-gui/src/instance.rs trusts a runtime folder only if it is ours (`meta.uid() == euid()`) and a socket only if
# it is ours and private. tests/instance.rs runs as one user, so it cannot hold that: without the comparison it stays green.
# This one runs as root in the container of the `galleria-gui-dev` image, starts the real app as `setpriv` users (1001 and
# 1002 are not in the image: only the numbers matter) and checks what each of them gets. Run it with GUI_UID=0 (the verb does).
#
#   * a folder `galleria` planted by another user is not used: no channel, the app starts on its own, nothing is written in it,
#     whoever starts the app (a user, or root, who could write there and so would use it if the owner were not looked at);
#   * a socket made by another user is not trusted: the second launch ends with 1 (a socket with wide modes too);
#   * a connection of another uid (modes widened by hand, or root, who the modes do not stop) is closed with no `ok`.
#
# It never skips: it ends with an error if one check fails, and also if it ran fewer checks than it should (EXPECTED).
#
# Needs GALLERIA_SMOKE_APP (the galleria-gui binary built as for a release).
set -uo pipefail
EXPECTED=36
: "${GALLERIA_SMOKE_APP:?path of the galleria-gui binary}"
(( $(id -u) == 0 )) || { echo "gui-socket-owner: it must run as root (GUI_UID=0), to start the app as other users" >&2; exit 2; }
here=$(cd -- "$(dirname -- "$0")" && pwd)

A=1001 # runs the first window
B=1002 # the other user
checks=0
failed=0

pass() { checks=$((checks + 1)); echo "  ok   $1"; }
fail() { checks=$((checks + 1)); failed=$((failed + 1)); echo "  FAIL $1" >&2; }
expect() { # description, actual, expected
  if [[ $2 == "$3" ]]; then pass "$1: $2"; else fail "$1: got <$2>, wanted <$3>"; fi
}
expect_in() { # description, text, wanted piece of it
  if [[ $2 == *"$3"* ]]; then pass "$1"; else fail "$1: <$3> is not in <$2>"; fi
}
expect_not_in() {
  if [[ $2 != *"$3"* ]]; then pass "$1"; else fail "$1: <$3> is in <$2>"; fi
}

base=$(mktemp -d)
chmod 755 "$base"
mkdir -p "$base/bin" "$base/run" "$base/log"
chmod 755 "$base/run"
# The repository is mounted from a home folder that another uid may not enter: the app and the client are copied out of it.
cp -- "$GALLERIA_SMOKE_APP" "$base/bin/galleria-gui"
cp -- "$here/gui-socket-client.py" "$base/bin/client.py"
chmod 755 "$base/bin/galleria-gui" "$base/bin/client.py"
APP=$base/bin/galleria-gui
CLIENT=$base/bin/client.py
printf 'client\nremote vpn.example 1194\n' > "$base/x.ovpn"
chmod 644 "$base/x.ovpn"
for u in $A $B 0; do
  mkdir -p "$base/home/$u/config" "$base/home/$u/state"
  chown -R "$u:$u" "$base/home/$u"
done

pids=()
cleanup() {
  local pid
  for pid in "${pids[@]}"; do kill "$pid" 2> /dev/null; done
  wait 2> /dev/null
  rm -rf -- "$base"
}
trap cleanup EXIT

export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 GALLERIA_SOCKET=$base/no-daemon.sock
Xvfb :77 -screen 0 1280x800x24 > /dev/null 2>&1 &
pids+=($!)
export DISPLAY=:77
for _ in $(seq 1 50); do [[ -e /tmp/.X11-unix/X77 ]] && break; sleep 0.1; done

as() { # uid, command...
  local uid=$1
  shift
  setpriv --reuid "$uid" --regid "$uid" --clear-groups env "HOME=$base/home/$uid" "XDG_CONFIG_HOME=$base/home/$uid/config" \
    "XDG_STATE_HOME=$base/home/$uid/state" "$@"
}

wait_for() { # seconds, command...: true as soon as the command is
  local n=$(($1 * 10))
  shift
  for _ in $(seq 1 "$n"); do "$@" && return 0; sleep 0.1; done
  return 1
}
has_line() { grep -q -- "$2" "$1" 2> /dev/null; }

# The app as the first window of `uid`, with `rundir` as its runtime folder. Sets app_pid.
start_app() { # uid, rundir, log
  as "$1" env "XDG_RUNTIME_DIR=$2" "$APP" > "$3" 2>&1 &
  app_pid=$!
  pids+=("$app_pid")
}
stop_app() {
  kill "$1" 2> /dev/null
  wait "$1" 2> /dev/null
}

# The app as a launch that finds no usable channel: it says so (in the log, the line the person sees) and goes on alone.
alone() { # uid, rundir, log
  start_app "$1" "$2" "$3"
  local pid=$app_pid
  wait_for 30 has_line "$3" 'starts on its own' || echo "the app of uid $1 never said it starts on its own" >> "$3"
  if kill -0 "$pid" 2> /dev/null; then pass "uid $1 starts on its own and is still running"; else fail "the app of uid $1 is gone"; fi
  stop_app "$pid"
}

second_launch() { # uid, rundir, log: the exit code of a launch given a file
  as "$1" env "XDG_RUNTIME_DIR=$2" timeout 20 "$APP" "$base/x.ovpn" > "$3" 2>&1
  echo $?
}

echo "== 1. a first window of uid $A, and what the other users get from its socket"
RD=$base/run/$A
install -d -o $A -g $A -m 700 "$RD"
start_app $A "$RD" "$base/log/first.log"
first=$app_pid
wait_for 30 test -S "$RD/galleria/gui.sock" || fail "the first window made no socket"
sleep 1
expect "folder (mode, owner)" "$(stat -c '%a %u' "$RD/galleria")" "700 $A"
expect "socket (mode, owner)" "$(stat -c '%a %u' "$RD/galleria/gui.sock")" "600 $A"
mine=$(as $A python3 "$CLIENT" "$RD/galleria/gui.sock" "$base/x.ovpn")
expect_in "the owner gets ok" "$mine" "answer=b'ok"
theirs=$(as $B python3 "$CLIENT" "$RD/galleria/gui.sock" "$base/x.ovpn")
expect_in "uid $B is stopped by the modes" "$theirs" "refused"
asroot=$(python3 "$CLIENT" "$RD/galleria/gui.sock" "$base/x.ovpn")
expect_in "root gets through the modes and is closed by SO_PEERCRED" "$asroot" "closed"
expect_not_in "root gets no ok" "$asroot" "answer="
alone $B "$RD" "$base/log/b-in-a.log"
expect_in "uid $B, pointed at the folder of uid $A, does not use it" "$(cat "$base/log/b-in-a.log")" "the runtime folder cannot be used"

echo "== 2. modes widened by hand behind the back of the first window (folders 0755, socket 0666)"
chmod 755 "$RD" "$RD/galleria"
chmod 666 "$RD/galleria/gui.sock"
wide=$(as $B python3 "$CLIENT" "$RD/galleria/gui.sock" "$base/x.ovpn")
expect_in "uid $B connects through the wide modes and is closed" "$wide" "closed"
expect_not_in "uid $B gets no ok" "$wide" "answer="
alone $B "$RD" "$base/log/b-wide.log"
expect_in "uid $B does not use a folder of uid $A, even with the modes it can read" "$(cat "$base/log/b-wide.log")" \
  "the folder is not a folder of this user"
expect "the owner does not trust a 0666 socket: second launch" "$(second_launch $A "$RD" "$base/log/a-666.log")" "1"
expect_in "it says why" "$(cat "$base/log/a-666.log")" "the socket is not private to this user"
chmod 600 "$RD/galleria/gui.sock"
chown $B:$B "$RD/galleria/gui.sock"
expect "the owner does not trust a 0600 socket of uid $B: second launch" "$(second_launch $A "$RD" "$base/log/a-b.log")" "1"
expect_in "it says why" "$(cat "$base/log/a-b.log")" "the socket is not private to this user"
chown $A:$A "$RD/galleria/gui.sock"
expect "control: with the socket of the owner again the second launch hands the file over" \
  "$(second_launch $A "$RD" "$base/log/a-ok.log")" "0"
if kill -0 "$first" 2> /dev/null; then pass "the first window is still running"; else fail "the first window died"; fi
stop_app "$first"

echo "== 3. a folder \`galleria\` planted by uid $B in a runtime folder anybody can write in"
for mode in 777 700; do
  RP=$base/run/planted-$mode
  install -d -m 1777 "$RP"
  as $B install -d -m $mode "$RP/galleria"
  for who in $A 0; do
    alone $who "$RP" "$base/log/planted-$mode-$who.log"
    expect_in "uid $who, folder planted $mode: not used" "$(cat "$base/log/planted-$mode-$who.log")" \
      "the folder is not a folder of this user"
    expect "uid $who, folder planted $mode: nothing written in it" "$(find "$RP/galleria" -mindepth 1 | wc -l)" "0"
    expect "uid $who, folder planted $mode: still uid $B's, same mode" "$(stat -c '%a %u' "$RP/galleria")" "$mode $B"
  done
done

echo "== 4. control: a folder of the user that is too open is taken back to 0700 and used"
RA=$base/run/open-$A
install -d -o $A -g $A -m 700 "$RA"
install -d -o $A -g $A -m 777 "$RA/galleria"
start_app $A "$RA" "$base/log/open.log"
wait_for 30 test -S "$RA/galleria/gui.sock" || fail "no socket in the folder of the user"
expect "folder tightened (mode, owner)" "$(stat -c '%a %u' "$RA/galleria")" "700 $A"
expect "socket (mode, owner)" "$(stat -c '%a %u' "$RA/galleria/gui.sock")" "600 $A"
stop_app "$app_pid"

echo
echo "$checks checks, $failed failed"
if ((failed > 0)); then exit 1; fi
if ((checks != EXPECTED)); then
  echo "gui-socket-owner: $checks checks ran, $EXPECTED were expected: a check was skipped or added without changing EXPECTED" >&2
  exit 1
fi
