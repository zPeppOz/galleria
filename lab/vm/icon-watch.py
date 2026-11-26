"""In VM, as lab: records, with a millisecond clock, when tray-icon-galleria-N.png files appear and disappear
(the tray-icon crate removes the old one before it writes the next). Poll every 2 ms. Ctrl+C or SIGTERM to stop.
usage: icon-watch.py [seconds]"""
import os, sys, time
d = f"/run/user/{os.getuid()}/galleria"
end = time.time() + float(sys.argv[1] if len(sys.argv) > 1 else 60)
seen = {}
t0 = time.time()
while time.time() < end:
    try:
        now = {n: os.stat(os.path.join(d, n)).st_size for n in os.listdir(d) if n.startswith("tray-icon")}
    except (FileNotFoundError, NotADirectoryError):
        now = {}
    except OSError:
        continue
    for n in sorted(set(now) | set(seen)):
        if n not in seen:
            print(f"{time.time()-t0:8.3f}s + {n} size={now[n]}", flush=True)
        elif n not in now:
            print(f"{time.time()-t0:8.3f}s - {n}", flush=True)
        elif now[n] != seen[n]:
            print(f"{time.time()-t0:8.3f}s ~ {n} size {seen[n]}->{now[n]}", flush=True)
    seen = now
    time.sleep(0.002)
