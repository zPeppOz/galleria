#!/bin/bash
# Galleria desktop VMs: qemu diretto, rete utente, tutto in /mnt/b/vm.
# Uso: vmctl.sh start|stop|ssh|status <ubuntu|ubuntu24|fedora> [comando...]
# DISPLAY_OPT=vnc=127.0.0.1:N apre un display VNC locale; il monitor qemu è sempre su /mnt/b/vm/<vm>.mon (sendkey, screendump).
set -euo pipefail
cmd=$1 vm=$2; shift 2
B=/mnt/b/vm
case $vm in ubuntu) port=2201;; fedora) port=2202;; ubuntu24) port=2203;; *) echo "vm?"; exit 2;; esac
pidf=$B/$vm.pid
case $cmd in
start)
  [[ -f $B/ovmf/$vm.VARS.fd ]] || cp /usr/share/OVMF/OVMF_VARS_4M.fd "$B/ovmf/$vm.VARS.fd"
  seed=""
  if [[ -f $B/seeds/$vm-seed.iso && ${NOSEED:-0} != 1 ]]; then
    seed="-drive file=$B/seeds/$vm-seed.iso,if=virtio,format=raw,media=cdrom,readonly=on"
  fi
  sg kvm -c "qemu-system-x86_64 -accel kvm -machine q35 -cpu host -smp ${SMP:-4} -m 4096 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive if=pflash,format=raw,file=$B/ovmf/$vm.VARS.fd \
    -drive file=$B/disks/$vm.qcow2,if=virtio,format=qcow2 $seed \
    -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:$port-:22 \
    -device virtio-vga -device qemu-xhci,id=xhci -device usb-tablet,bus=xhci.0 -display ${DISPLAY_OPT:-none} -monitor unix:$B/$vm.mon,server,nowait -serial file:$B/logs/$vm.serial.log \
    -object rng-random,id=rng0,filename=/dev/urandom -device virtio-rng-pci,rng=rng0 \
    -name galleria-$vm -daemonize -pidfile $pidf"
  ;;
stop)
  [[ -f $pidf ]] || { echo "no pidfile"; exit 0; }
  pid=$(cat "$pidf")
  "$0" ssh "$vm" 'sudo systemctl poweroff' >/dev/null 2>&1 || true
  for i in $(seq 60); do
    kill -0 "$pid" 2>/dev/null || { echo stopped; rm -f "$pidf"; exit 0; }
    sleep 2
  done
  kill "$pid"; sleep 3; rm -f "$pidf"; echo killed
  ;;
ssh)
  exec ssh -i $B/ssh/id_vm -p $port -o StrictHostKeyChecking=no -o UserKnownHostsFile=$B/ssh/known_hosts_$vm -o LogLevel=ERROR -o ConnectTimeout=5 lab@127.0.0.1 "$@"
  ;;
status)
  if [[ -f $pidf ]] && kill -0 "$(cat "$pidf")" 2>/dev/null; then echo "running pid $(cat "$pidf")"; else echo stopped; fi
  ;;
esac
