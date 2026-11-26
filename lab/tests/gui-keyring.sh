#!/usr/bin/env bash
# The keyring store against a real Secret Service, in three rounds (plan B5, T2; scripts/gui.sh keyring).
#
# Each round has a session bus of its own (`dbus-run-session`) and a data folder of its own, so nothing of the user who runs
# it is read or written, and runs the `#[ignore]` tests that belong to it with GALLERIA_KEYRING_CASE:
#
#   unlocked  GNOME Keyring started with `--unlock` and a password on stdin (a throw-away keyring, no screen needed) -
#             galleria-profile/tests/keyring.rs and galleria-cli/tests/keyring.rs
#   locked    the same, with one item stored and then the collection locked; there is no prompter in the image, so a request
#             to unlock fails at once, and with a timeout the tests do not depend on that
#   absent    a bus whose configuration has no folder of services: nothing can be started, `org.freedesktop.secrets` is not there
#
# The tests fail if their case is not the one asked for, so a round cannot pass by running another. The log of each round
# is kept in target/gates/keyring-<case>.log, and at the end scripts/check-gates.py wants every proof of the job `gui-smoke`
# that is a cargo test (G5, G6) to be there with `ok`: an ignored or a missing one is an error.
#
# Runs where GNOME Keyring, secret-tool, gdbus and dbus-run-session are installed: the galleria-gui-dev image
# (scripts/gui.sh keyring) and nothing else.
set -uo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
repo=$(cd -- "$here/../.." && pwd)

PASSWORD=galleria-test-keyring
export SEEDED_PROFILE=5ee0ed00-0000-4000-8000-000000000001

if [[ ${1:-} == --round ]]; then
  case=$2
  cd "$repo"
  data=$(mktemp -d)
  run=$(mktemp -d)
  chmod 700 "$run"
  trap 'pkill -u "$(id -u)" -f "gnome-keyring-daemon.*$run" 2>/dev/null; rm -rf -- "$data" "$run"' EXIT
  export XDG_DATA_HOME=$data XDG_RUNTIME_DIR=$run GALLERIA_KEYRING_CASE=$case
  unset DISPLAY WAYLAND_DISPLAY

  start_keyring() {
    echo -n "$PASSWORD" | gnome-keyring-daemon --unlock --components=secrets > /dev/null 2>&1 \
      || { echo "gui-keyring: GNOME Keyring did not start" >&2; exit 1; }
  }
  collection_locked() {
    gdbus call --session -d org.freedesktop.secrets -o /org/freedesktop/secrets/collection/login \
      -m org.freedesktop.DBus.Properties.Get org.freedesktop.Secret.Collection Locked 2> /dev/null
  }

  case $case in
    unlocked)
      start_keyring
      [[ $(collection_locked) == *false* ]] || { echo "gui-keyring: the keyring is not unlocked" >&2; exit 1; }
      profile_tests=(keyring_roundtrip keyring_items_carry_the_three_attributes_and_the_label keyring_delete_one_and_all
        keyring_two_profiles_do_not_mix keyring_value_with_newlines_survives)
      cli_tests=(the_cli_reads_what_the_keyring_holds) ;;
    locked)
      start_keyring
      printf 'SEED-SECRET-SEEDED' | secret-tool store --label='seeded before the lock' \
        service galleria profile "$SEEDED_PROFILE" key wireguard.private_key \
        || { echo "gui-keyring: cannot seed the keyring" >&2; exit 1; }
      gdbus call --session -d org.freedesktop.secrets -o /org/freedesktop/secrets -m org.freedesktop.Secret.Service.Lock \
        "['/org/freedesktop/secrets/collection/login']" > /dev/null || { echo "gui-keyring: cannot lock the keyring" >&2; exit 1; }
      [[ $(collection_locked) == *true* ]] || { echo "gui-keyring: the keyring is not locked" >&2; exit 1; }
      profile_tests=(keyring_locked_is_reported_within_five_seconds keyring_present_does_not_unlock)
      cli_tests=() ;;
    absent)
      profile_tests=(keyring_absent_opens_as_none)
      cli_tests=(the_cli_without_a_keyring_reads_the_file) ;;
    *) echo "gui-keyring: unknown round $case" >&2; exit 2 ;;
  esac

  status=0
  cargo test -p galleria-profile --test keyring -- --ignored --test-threads=1 "${profile_tests[@]}" || status=1
  if (( ${#cli_tests[@]} )); then
    cargo test -p galleria-cli --test keyring -- --ignored --test-threads=1 "${cli_tests[@]}" || status=1
  fi
  exit "$status"
fi

cd "$repo"
mkdir -p target/gates
rm -f target/gates/keyring-*.log
# a bus without any service the keyring could be started from: no folder of services, nothing is activated
cat > target/gates/no-services.conf <<'CONF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dtds/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=/tmp</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
CONF

# build once, outside the rounds
cargo test -p galleria-profile --test keyring --no-run || exit 1
cargo test -p galleria-cli --test keyring --no-run || exit 1
cargo build -p galleria-cli --bin galleria || exit 1

failed=0
for case in unlocked locked absent; do
  echo "=== keyring round: $case"
  # the stdout of the round is the log the gates are read from (`test <name> ... ok` must be on a line of its own), so what the
  # bus, GNOME Keyring and cargo write on stderr goes to a file of its own, shown only when a round fails
  if [[ $case == absent ]]; then
    dbus-run-session --config-file="$repo/target/gates/no-services.conf" -- bash "$0" --round "$case" \
      2> "target/gates/keyring-$case.stderr" | tee "target/gates/keyring-$case.log"
  else
    dbus-run-session -- bash "$0" --round "$case" 2> "target/gates/keyring-$case.stderr" | tee "target/gates/keyring-$case.log"
  fi
  [[ ${PIPESTATUS[0]} == 0 ]] || { echo "gui-keyring: the round $case failed; stderr:" >&2; cat "target/gates/keyring-$case.stderr" >&2; failed=1; }
done
(( failed == 0 )) || exit 1

scripts/check-gates.py --job gui-smoke --kind cargo \
  --cargo-log target/gates/keyring-unlocked.log --cargo-log target/gates/keyring-locked.log --cargo-log target/gates/keyring-absent.log
