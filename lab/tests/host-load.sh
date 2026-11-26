#!/usr/bin/env bash
# CPU load on the host for a while, to see whether a suite that is green on a quiet machine stays green on a busy one
# (the rounds of `a11y` that went red did so while other work ran on the same host).
#
#   lab/tests/host-load.sh <seconds> [busy loops]      default: as many loops as there are cores minus two
set -uo pipefail
seconds=${1:?usage: lab/tests/host-load.sh <seconds> [busy loops]}
loops=${2:-$(( $(nproc) - 2 ))}
for _ in $(seq 1 "$loops"); do
  timeout "$seconds" bash -c 'while :; do :; done' &
done
wait
