#!/usr/bin/env bash
# Probe for Q42: how can the window in the galleria-gui-dev image reach a Unix socket that lives in a lab client?
#   A. the folder of the socket bind-mounted into both containers
#   B. a relay: a socket on the host side, joined to the client's socket through `docker exec -i -u alice`
# Throwaway containers named gprobe-*, no network, removed at the end. Nothing of the laboratory is touched.
set -uo pipefail
here=/home/g7aro/.hermes/profiles/nello/cache/scratch/b5/sock
mkdir -p "$here/shared" "$here/relay"
chmod 777 "$here/shared" "$here/relay"
docker rm -f gprobe-s >/dev/null 2>&1
docker info --format 'docker: {{.ServerVersion}} {{.SecurityOptions}}'

echo '--- server: glab-client26 image, socket root:galleria 0660 in /run/galleria (bind mount of shared/)'
docker run -d --name gprobe-s --network none -v "$here/probe:/probe:ro" -v "$here/shared:/run/galleria" glab-client26 \
  python3 /probe/server.py /run/galleria/galleriad.sock >/dev/null
sleep 1
ls -ln "$here/shared"

echo '--- A1. gui image, default user of scripts/gui.sh under rootless (0:0)'
docker run --rm --network none --user 0:0 -v "$here/probe:/probe:ro" -v "$here/shared:/s" galleria-gui-dev python3 /probe/client.py /s/galleriad.sock
echo '--- A2. gui image, uid of alice in the client (1001) with the gid of galleria in the client (1001) as a supplementary group'
docker run --rm --network none --user 1001:1002 --group-add 1001 -v "$here/probe:/probe:ro" -v "$here/shared:/s" galleria-gui-dev python3 /probe/client.py /s/galleriad.sock
echo '--- A3. gui image, a uid without the group (like mallory)'
docker run --rm --network none --user 1003:1003 -v "$here/probe:/probe:ro" -v "$here/shared:/s" galleria-gui-dev python3 /probe/client.py /s/galleriad.sock

echo '--- B. relay on the host -> docker exec -i -u alice -> socket in the client'
python3 "$here/probe/relay.py" listen "$here/relay/galleriad.sock" docker exec -i -u alice gprobe-s python3 /probe/relay.py join /run/galleria/galleriad.sock &
relay=$!
sleep 1
docker run --rm --network none --user 0:0 -v "$here/probe:/probe:ro" -v "$here/relay:/s" galleria-gui-dev python3 /probe/client.py /s/galleriad.sock
echo '--- B, from the host itself'
python3 "$here/probe/client.py" "$here/relay/galleriad.sock"
sleep 1
kill "$relay" 2>/dev/null
echo '--- what the server saw'
docker logs gprobe-s 2>&1
docker rm -f gprobe-s >/dev/null 2>&1
docker ps -a --format '{{.Names}}' | grep gprobe || echo 'no gprobe container left'
