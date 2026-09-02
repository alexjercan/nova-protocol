#!/usr/bin/env python3
"""The command lane, driven against the real binary.

Proves the contract this task exists for: the wire carries the exact text a
player types at the CRT prompt, the same parser resolves it, the same
dispatcher runs it, and the answer comes back naming the command, its class
and its result - never the scenario action behind it.

Run from the REPO ROOT:

    export NOVA_CONFIG_ROOT=/tmp/nova-command-config
    export BEVY_ASSET_ROOT=$PWD
    POC=tasks/20260827-120347/poc
    CMD="target/debug/nova-protocol --norender --scenario shakedown_run --channel step"
    python3 $POC/drive_commands.py --cmd "$CMD"

Exits non-zero on the first failed check.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1].parent / "20260820-174148" / "poc"))

from channel import Channel, check, cmd_from_argv  # noqa: E402


class Shell:
    """The command lane on top of the shared client."""

    def __init__(self, channel: Channel):
        self.ch = channel

    def run(self, text: str) -> dict:
        """Send one command line and return its acknowledgement.

        A command ack is the entry carrying a ``command`` key; every other
        entry in ``applied`` belongs to an input lane. One command per
        snapshot, so there is never more than one to pick.
        """
        self.ch.tick += 1
        self.ch.send({"tick": self.ch.tick, "command": text})
        # Two ticks: the lane applies in PreUpdate, the dispatcher runs in
        # Update, and the runner collects after the frame.
        snapshot = self.ch.step(2)
        acks = [a for a in snapshot.get("applied", []) if "command" in a]
        if len(acks) != 1:
            sys.exit(f"expected one acknowledgement for {text!r}, got {acks}")
        ack = acks[0]
        print(f"  cmd> {text}")
        print(f"       {ack['state']} [{ack['class']}] {ack['detail']}")
        for row in ack.get("rows", []):
            print(f"       | {row}")
        return ack


def rows_of(ack: dict) -> str:
    return "\n".join(ack.get("rows", []))


def main() -> None:
    cmd = cmd_from_argv()
    if cmd is None:
        sys.exit("drive_commands needs the real binary: pass --cmd \"...\"")
    ch = Channel(cmd)
    shell = Shell(ch)
    ch.step(120)

    print("\n== the reserved key is refused, permanently ==")
    ch.tick += 1
    ch.send({"tick": ch.tick, "action": "SetSpeedCap"})
    ch.step(1)
    refusal = " ".join(e["error"] for e in ch.errors)
    check("not a wire lane" in refusal, "`action` is refused and will not become a lane")
    check("command" in refusal, "the refusal points at the command lane")

    print("\n== read-only: the world answers by name ==")
    status = shell.run("status")
    check(status["state"] == "ok", "status runs")
    check(status["class"] == "readonly", "status is read-only")
    check("shakedown_run" in status["detail"], "status names the live scenario")

    ships = shell.run("ships")
    check(ships["state"] == "ok", "ships runs")
    # The first column of each indented row is the ship's scenario id.
    listed = [r.strip().split()[0] for r in ships["rows"] if r.startswith("  ")]
    check(bool(listed), f"ships lists at least one id: {listed}")
    ship_id = listed[0]

    one = shell.run(f"ship {ship_id}")
    check(one["state"] == "ok", f"ship {ship_id} resolves")
    sections = shell.run(f"sections {ship_id}")
    check(sections["state"] == "ok", "sections lists the ship's sections")

    print("\n== an unknown id is a clear error, not a silent nothing ==")
    missing = shell.run("ship no_such_ship")
    check(missing["state"] == "error", "an unknown ship id is an error")
    check("no_such_ship" in missing["detail"], "the error names what was asked for")

    print("\n== the catalog answers help without touching the world ==")
    helped = shell.run("help")
    check(helped["state"] == "ok", "help answers")
    typo = shell.run("graphix")
    check(typo["state"] == "error", "an unknown command is an error")
    check("did you mean" in rows_of(typo).lower(), "and it suggests the near miss")

    print("\n== settings: the command without a value reads it ==")
    before = shell.run("graphics")
    check(before["state"] == "ok", "graphics reads")
    changed = shell.run("graphics low")
    check(changed["state"] == "ok", "graphics low applies")
    check(changed["class"] == "setting", "graphics is a setting, not a cheat")
    after = shell.run("graphics")
    check("low" in after["detail"].lower(), f"the change stuck: {after['detail']}")

    vol = shell.run("volume master 0.5")
    check(vol["state"] == "ok", "volume master 0.5 applies")
    check("0.5" in rows_of(vol) + vol["detail"], "and reports the value it set")

    print("\n== a setting never marks the run ==")
    clean = shell.run("cheats status")
    check("clean" in clean["detail"], f"the run is still clean: {clean['detail']}")

    print("\n== a cheat is refused until it is armed ==")
    refused = shell.run(f"ammo refill {ship_id}")
    check(refused["state"] == "refused", "an unarmed cheat is refused")
    check("cheats enable" in refused["detail"], "the refusal says what to do")
    still_clean = shell.run("cheats status")
    check("clean" in still_clean["detail"], "a refused cheat does not mark the run")

    print("\n== arming marks the run, immediately ==")
    armed = shell.run("cheats enable")
    check(armed["state"] == "ok", "cheats enable runs unarmed - it IS the arming")
    marked = shell.run("cheats status")
    check("marked" in marked["detail"], f"the run is marked: {marked['detail']}")

    print("\n== the bounded cheat catalog ==")
    on = shell.run(f"ammo infinite {ship_id} on")
    check(on["state"] == "ok", "unlimited ammunition goes on")
    off = shell.run(f"ammo infinite {ship_id} off")
    check(off["state"] == "ok", "and back off")
    refill = shell.run(f"ammo refill {ship_id}")
    check(refill["state"] == "ok", "a refill runs once armed")
    cap = shell.run(f"speed-cap {ship_id} 50")
    check(cap["state"] == "ok", "the speed cap is installed")
    uncap = shell.run(f"speed-cap {ship_id} off")
    check(uncap["state"] == "ok", "and removed")

    print("\n== what is deliberately not in the catalog ==")
    for forbidden in ["win", "lose", "outcome victory", "spawn raider", "variable set beat 9"]:
        denied = shell.run(forbidden)
        check(denied["state"] == "error", f"`{forbidden}` is not a command")

    print("\n== a fresh scenario is a fresh run ==")
    reload = shell.run("scenario load shakedown_run")
    check(reload["state"] == "ok", "scenario load runs")
    check(reload["class"] == "utility", "abandoning a run is utility, not a cheat")
    ch.step(60)
    fresh = shell.run("cheats status")
    check("clean" in fresh["detail"], f"the mark is gone: {fresh['detail']}")

    ch.close()
    print("\nall checks passed")


if __name__ == "__main__":
    main()
