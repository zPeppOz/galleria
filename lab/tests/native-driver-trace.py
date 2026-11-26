#!/usr/bin/env python3
"""WebKitWebDriver as tauri-driver starts it (`--native-driver`), with its life written down (lab/tests/gui-a11y.sh).

It runs the real driver (GALLERIA_NATIVE_DRIVER) with the same arguments, copies what it prints to its own stderr and to
$GALLERIA_A11Y_OUT/webkitwebdriver.log, and writes there how it ended: the exit status, or the signal. A driver that
disappears leaves nothing else behind. tauri-driver ends the wrapper with SIGKILL when the session is over; the real
driver is then ended too (PR_SET_PDEATHSIG), so that none is left behind to hold the window of a session.
"""
import ctypes
import os
import signal
import subprocess
import sys
import threading
import time

LOG = os.path.join(os.environ["GALLERIA_A11Y_OUT"], "webkitwebdriver.log")


def note(text: str) -> None:
    with open(LOG, "a", encoding="utf-8") as log:
        log.write(f"{time.strftime('%H:%M:%S')}.{int(time.time() * 1000) % 1000:03d} {text}\n")


def die_with_parent() -> None:
    ctypes.CDLL("libc.so.6", use_errno=True).prctl(1, signal.SIGKILL)  # PR_SET_PDEATHSIG


def main() -> int:
    child = subprocess.Popen([os.environ["GALLERIA_NATIVE_DRIVER"], *sys.argv[1:]], stderr=subprocess.PIPE, preexec_fn=die_with_parent)
    note(f"start wrapper={os.getpid()} driver={child.pid} args={' '.join(sys.argv[1:])}")

    def forward() -> None:
        assert child.stderr is not None
        for line in child.stderr:
            sys.stderr.buffer.write(line)
            sys.stderr.buffer.flush()
            with open(LOG, "ab") as log:
                log.write(b"    stderr: " + line)

    pump = threading.Thread(target=forward, daemon=True)
    pump.start()
    # Fault injection, to prove that a driver that dies fails the test that uses it and no other: with
    # GALLERIA_NATIVE_DRIVER_DIE_AFTER=<seconds> the first driver of the run is killed after that time (the marker keeps the
    # drivers of the next tests alive).
    after = os.environ.get("GALLERIA_NATIVE_DRIVER_DIE_AFTER")
    marker = os.path.join(os.environ["GALLERIA_A11Y_OUT"], "driver-killed")
    if after and not os.path.exists(marker):
        open(marker, "w").close()
        threading.Timer(float(after), lambda: (note("fault injection: SIGKILL to the driver"), child.send_signal(signal.SIGKILL))).start()
    for name in ("SIGTERM", "SIGINT", "SIGHUP"):
        signal.signal(getattr(signal, name), lambda number, _frame: (note(f"wrapper got {signal.Signals(number).name}, passing it on"), child.send_signal(number)))
    status = child.wait()
    pump.join(timeout=2)
    note(f"ended driver={child.pid} by signal {signal.Signals(-status).name}" if status < 0 else f"ended driver={child.pid} status={status}")
    return 128 - status if status < 0 else status


sys.exit(main())
