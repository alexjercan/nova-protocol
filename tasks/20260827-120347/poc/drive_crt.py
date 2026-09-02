#!/usr/bin/env python3
"""The Command shell on the CRT, driven and filmed.

The wire driver (`drive_commands.py`) proves the dispatcher. This one proves
the OTHER front end: `:` opens the shell over live gameplay, the staged
introduction reveals against the live world, the prompt runs the same
commands, and Escape closes the CRT and gives the world back.

Run from the REPO ROOT with a GPU (or Xvfb) available:

    export NOVA_CONFIG_ROOT=/tmp/nova-command-config
    export BEVY_ASSET_ROOT=$PWD
    POC=tasks/20260827-120347/poc
    CMD="target/debug/nova-protocol --norender --scenario shakedown_run \\
         --channel step --record /tmp/nova-crt"
    python3 $POC/drive_crt.py --cmd "$CMD"

`--record` arms the offscreen renderer, so every tick lands in the dir as
`frame_%06d.png`: the frames named in the transcript are the proof.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1].parent / "20260820-174148" / "poc"))

from channel import Channel, check, cmd_from_argv  # noqa: E402


def main() -> None:
    cmd = cmd_from_argv()
    if cmd is None:
        sys.exit("drive_crt needs the real binary: pass --cmd \"...\"")
    ch = Channel(cmd)

    def frame(label: str) -> None:
        print(f"  frame_{ch.tick:06d}.png  {label}")

    print("\n== the world runs, no computer on screen ==")
    ch.step(90)
    frame("flight, the CRT closed")

    print("\n== ':' opens the Command shell over gameplay ==")
    ch.tick += 1
    ch.send({"tick": ch.tick, "text": ":"})
    ch.step(30)
    frame("the CRT slides in, the introduction staging")
    ch.step(60)
    frame("the introduction revealed: POST / CORE / REGISTRY / WORLD / CHEATS")

    print("\n== the prompt runs the same commands the wire does ==")
    for line in ["status", "commands cheat", "cheats enable", "cheats status"]:
        ch.tick += 1
        ch.send({"tick": ch.tick, "text": line})
        ch.step(2)
        ch.tick += 1
        ch.send({"tick": ch.tick, "key": "Enter"})
        ch.step(6)
        frame(f"after `{line}`")

    print("\n== the header carries the arming state ==")
    ch.step(30)
    frame("CHEATS: ON in amber, top right")

    print("\n== Escape closes the computer and gives the world back ==")
    ch.tick += 1
    ch.send({"tick": ch.tick, "key": "Escape"})
    ch.step(60)
    frame("back to flight, the scenario intact")

    check(not ch.errors, f"no line was refused: {ch.errors}")
    ch.close()
    print("\nall checks passed")


if __name__ == "__main__":
    main()
