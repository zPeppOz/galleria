#!/bin/bash
set -euo pipefail
# Only Galleria phase-one resources; never touch spike or other project containers.
mapfile -t containers < <(docker ps -a --format '{{.Names}}' | awk '/^glab-/' || true)
if ((${#containers[@]})); then docker rm -f "${containers[@]}" >/dev/null; fi
if docker network inspect galleria-fase1 >/dev/null 2>&1; then docker network rm galleria-fase1 >/dev/null; fi
