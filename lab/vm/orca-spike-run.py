#!/usr/bin/env python3
"""Giro dello spike Orca su una VM (docs/08, «Spike Orca»): tasti veri dal monitor qemu, parlato letto dal file di
debug di Orca dentro la VM. Va lanciato dall'host con Orca e l'app già su (orca-spike-vm.sh orca, app).

    orca-spike-run.py <ubuntu|fedora> <fields|toast|statusbar|all> [file.txt]

Ogni passo: conta le frasi di Orca, preme i tasti, aspetta e stampa le frasi dette dopo il conteggio, con il fuoco AT-SPI.
Sul passo con `shot=` fa anche una fotografia dal VNC (vncshot.py: 5901 ubuntu, 5902 fedora).
La trascrizione va anche nel file indicato (default /mnt/b/vm/spike/<vm>-<scenario>.txt).
"""
import subprocess
import sys
import time

VM = sys.argv[1] if len(sys.argv) > 1 else "ubuntu"
S = "~/spike/orca-spike-vm.sh"


def ssh(cmd):
    result = subprocess.run([__file__.rsplit("/", 1)[0] + "/vmctl.sh", "ssh", VM, cmd], capture_output=True, text=True, timeout=90)
    if result.returncode:
        raise RuntimeError(f"{VM}: {cmd}: exit {result.returncode}: {result.stderr}\n{result.stdout}")
    return result.stdout


def keys(*ks, wait=0.6):
    subprocess.run([sys.executable, __file__.rsplit("/", 1)[0] + "/vmkeys.py", VM, "key", *ks], check=True, env={"KEY_WAIT": str(wait), "PATH": "/usr/bin:/bin"})


out_lines = []


PORT = {"ubuntu": 5901, "fedora": 5902, "ubuntu24": 5903}


def step(label, *ks, wait=2.5, action=None, shot=None):
    before = int(ssh(f"{S} count").strip() or 0)
    if action:
        action()
    if ks:
        keys(*ks)
    extra = []
    if shot:
        time.sleep(1)
        subprocess.run([sys.executable, __file__.rsplit("/", 1)[0] + "/vncshot.py", str(PORT[VM]), shot], check=False)
        tree = ssh(f"{S} dump | grep -E '^[0-9]+.(status bar|alert|log|push button.Dismiss|.*[Ss]ervice|.*[Dd]isconnect)'").strip()
        extra = ["albero AT-SPI a metà passo (righe status bar / alert / Dismiss / service):"] + ["  " + l.replace("\t", " | ") for l in tree.splitlines()] + [f"fotografia: {shot}"]
        wait = max(wait - 1, 0)
    time.sleep(wait)
    said = [l for l in ssh(f"{S} since {before}").splitlines() if l.strip()]
    focus = ssh(f"{S} dump | awk -F'\\t' '$4 ~ /focused/ && $2 != \"frame\" {{n=$2\" «\"$3\"»\"}} END{{print n}}'").strip()
    block = [f"## {label}  [{' '.join(ks)}]  fuoco AT-SPI: {focus}"] + [f"SPEECH OUTPUT: {s}" for s in said] + ([] if said else ["(niente)"]) + extra + [""]
    print("\n".join(block), flush=True)
    out_lines.extend(block)


def fields():
    # The current app has Import and Search stops which the B2 tour did not have. Anchor to the Name entry and record the actual focus at every
    # step, instead of calling whatever the old Tab count reaches a field. The rubrica in ~/spike/cfg must be seeded (gui-vm.sh install + seed).
    step("focus Name (AT-SPI)", action=lambda: ssh(f"{S} focus Name"))
    for i in range(1, 5):
        step(f"Tab {i} from Name", "tab")
    for i in range(1, 5):
        step(f"Shift+Tab {i}", "shift-tab")


def toast():
    step("focus More actions (AT-SPI)", action=lambda: ssh(f"{S} focus 'More actions'"))
    step("Enter (opens More actions)", "ret")
    step("Enter su Duplicate (avviso a scomparsa)", "ret", wait=6, shot=f"/mnt/b/vm/spike/{VM}-toast.png")


def statusbar():
    step("fermo il demone (la barra di stato cambia)", action=lambda: ssh(f"{S} daemon-down"), wait=7, shot=f"/mnt/b/vm/spike/{VM}-daemon-down.png")
    step("riavvio il demone", action=lambda: ssh(f"{S} daemon-up"), wait=7, shot=f"/mnt/b/vm/spike/{VM}-daemon-up.png")


scenarios = {"fields": fields, "toast": toast, "statusbar": statusbar}
which = sys.argv[2]
# A successful AT-SPI grab_focus does not give the window compositor focus on Wayland. Without this check the old tour can
# log the Welcome window's speech while its AT-SPI target is a Galleria entry. Try the real window switcher, then fail visibly.
for attempt in range(4):
    tree = ssh(f"{S} dump")
    if any(len(c := line.split("\t")) > 3 and c[1] == "frame" and "active" in c[3].split(",") for line in tree.splitlines()):
        break
    if attempt == 3:
        raise RuntimeError(f"{VM}: Galleria does not have compositor focus; no speech conclusions can be drawn")
    keys("alt-tab")
    time.sleep(1)
for name in (scenarios if which == "all" else [which]):
    out_lines.append(f"# {VM} · {name}")
    scenarios[name]()
dest = sys.argv[3] if len(sys.argv) > 3 else f"/mnt/b/vm/spike/{VM}-{which}.txt"
open(dest, "w").write("\n".join(out_lines) + "\n")
