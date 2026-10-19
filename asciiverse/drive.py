#!/usr/bin/env python3
"""Records asciiverse/armcom.cast, the catalog preview, at 100x30.

Usage: drive.py <armcom binary> <out.cast> [seconds]

asciinema rec runs `armcom --encounter battle` (by name, from a throwaway
folder on PATH) in a detached 100x30 tmux session. This script reads the
pane and plays the way a player would: the gunner on Fire Main Gun each
round, every shot taken, every roll and report read for a moment, the rest
of the round left to run. After `seconds` (default 55) it saves and quits
from the encounter menu, or stops when the battle ends on its own. Needs
tmux and asciinema 3."""
import os, shutil, subprocess, sys, tempfile, time

binary, out = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 40.0

S = "armcom-preview"
TMUX = ["tmux", "-L", S]
home = tempfile.mkdtemp()
shutil.copy(binary, os.path.join(home, "armcom"))
env = dict(os.environ, HOME=home, PATH=home + ":" + os.environ["PATH"],
           TERM="xterm-256color", COLORTERM="truecolor", LANG="C.UTF-8",
           USER="player", LOGNAME="player",
           # no sound while recording
           PIPEWIRE_REMOTE="/nonexistent", PULSE_SERVER="unix:/nonexistent")
for k in ("VTE_VERSION", "TERM_PROGRAM", "TERM_PROGRAM_VERSION", "TMUX", "XDG_DATA_HOME", "RUST_LOG"):
    env.pop(k, None)

def key(*k):
    subprocess.run(TMUX + ["send-keys", "-t", S, *k])

def screen():
    r = subprocess.run(TMUX + ["capture-pane", "-p", "-t", S], capture_output=True, text=True)
    return [l.ljust(100) for l in (r.stdout.split("\n") + [""] * 30)[:30]]

def running():
    return subprocess.run(TMUX + ["has-session", "-t", S], capture_output=True).returncode == 0

def gunner_order(rows):
    # the line under the gunner's name; a selected crewman's starts with ►
    for i, r in enumerate(rows[:-1]):
        if r[1:].startswith("Gunner "):
            return rows[i + 1][:26].strip()
    return None

def set_gunner():
    """Orders phase: find the gunner by changing an order and seeing whose
    line moves, then step his orders round to Fire Main Gun."""
    for _ in range(6):
        before = screen()
        if gunner_order(before) == "Fire Main Gun":
            return
        key("d"); time.sleep(0.3)
        if gunner_order(screen()) != gunner_order(before):
            for _ in range(8):
                if gunner_order(screen()) == "Fire Main Gun":
                    return
                key("d"); time.sleep(0.3)
            return
        key("a"); time.sleep(0.2); key("s"); time.sleep(0.3)

cmd = ("env TERM=xterm-256color asciinema rec --overwrite -q --window-size 100x30 --idle-time-limit 2"
       " --title 'Armoured Commander' --capture-env TERM,COLORTERM,LANG"
       " -c 'armcom --encounter battle' " + out)
subprocess.run(TMUX + ["new-session", "-d", "-s", S, "-x", "100", "-y", "30",
                       "-c", home, cmd], env=env, check=True)
try:
    time.sleep(2.5)
    t0, shots = time.time(), 0
    while running() and time.time() - t0 < secs:
        text = "".join(screen())
        if "[Enter] fire" in text:
            # take the shot while there is one; then the phase is done
            if shots < 3:
                time.sleep(1.0); key("Enter"); shots += 1; time.sleep(1.0)
                if "[Enter] fire" in "".join(screen()):
                    shots = 3
            else:
                key("Space"); shots = 0; time.sleep(0.6)
        elif "Enter" in text and ("continue" in text or "[Enter" in text):
            # a roll, a report or a label: read it, then go on
            time.sleep(1.0); key("Enter"); time.sleep(0.3)
        elif "[A/D] order" in text:
            set_gunner()
            time.sleep(0.8); key("Space"); time.sleep(0.5)
        else:
            time.sleep(0.5); key("Space"); time.sleep(0.4)
    if running():
        time.sleep(1.5); key("C-c")
    for _ in range(100):
        if not running():
            break
        time.sleep(0.1)
finally:
    subprocess.run(TMUX + ["kill-server"], capture_output=True)
    # asciinema outlives the tmux server; a stray one would go on writing
    subprocess.run(["pkill", "-f", "asciinema rec .*" + out], capture_output=True)
    shutil.rmtree(home, ignore_errors=True)
