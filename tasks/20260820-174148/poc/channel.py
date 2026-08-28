"""Driver-side client for the nova_channel wire.

One JSON object per line, both directions. The client owns the tick counter:
every payload is stamped, and a bare tick line is the step instruction. This
is the library the four drive_*.py scripts share; when the real crate exists
they pass ``--cmd "nova --norender --channel step"`` and nothing else changes.
"""

from __future__ import annotations

import json
import shlex
import subprocess
import sys
from pathlib import Path


class ChannelError(RuntimeError):
    """An error line came back from the game."""


def cmd_from_argv() -> list[str] | None:
    """The ``--cmd "nova --norender --channel step"`` a driver was run with,
    split into an argv - or None, which means the mock."""
    if "--cmd" in sys.argv:
        return shlex.split(sys.argv[sys.argv.index("--cmd") + 1])
    return None


class Channel:
    """A stepped session against a process speaking the channel schema."""

    def __init__(self, cmd: list[str] | None = None, verbose: bool = True):
        if cmd is None:
            cmd = cmd_from_argv()
        if cmd is None:
            mock = Path(__file__).parent / "mock_game.py"
            cmd = [sys.executable, str(mock), "--step"]
        self.proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.tick = 0
        self.verbose = verbose
        self.errors: list[dict] = []
        self.snapshot: dict = {}

    # -- transport ----------------------------------------------------------

    def send(self, obj: dict) -> None:
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(obj) + "\n")
        self.proc.stdin.flush()

    def _read_snapshot(self) -> dict:
        """Read lines until a snapshot arrives; error lines are collected."""
        assert self.proc.stdout is not None
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise ChannelError("the game closed stdout")
            obj = json.loads(line)
            if "error" in obj:
                self.errors.append(obj)
                if self.verbose:
                    print(f"  ! error: {obj['error']}")
                continue
            self.snapshot = obj
            return obj

    # -- the clock ----------------------------------------------------------

    def step(self, ticks: int = 1) -> dict:
        """Run the clock ``ticks`` forward and return the snapshot."""
        self.tick += ticks
        self.send({"tick": self.tick})
        return self._read_snapshot()

    # -- the lanes ----------------------------------------------------------

    def _stamp(self, payload: dict) -> None:
        self.tick += 1
        self.send({"tick": self.tick, **payload})

    def press(self, name: str) -> None:
        self._stamp({"input": name, "phase": "start"})

    def release(self, name: str) -> None:
        self._stamp({"input": name, "phase": "stop"})

    def hold(self, name: str, ticks: int) -> dict:
        """Press, keep the clock running, release. One gesture, three lines."""
        self.press(name)
        self.step(ticks)
        self.release(name)
        return self.step(1)

    def aim(self, name: str, delta: tuple[float, float]) -> None:
        self._stamp({"aim": {"name": name, "delta": list(delta)}})

    def type_text(self, text: str) -> None:
        self._stamp({"text": text})

    def key(self, key: str) -> None:
        self._stamp({"key": key})

    def pointer_to(self, target) -> None:
        """``target`` is a UI Name or an [x, y] logical-pixel pair."""
        self._stamp({"pointer": {"to": target}})

    def click(self, target) -> dict:
        """Move, press, release on consecutive ticks: Activate is
        release-over, and the forwarded pointer carries a frame of lag."""
        self.pointer_to(target)
        self.step(1)
        self._stamp({"pointer": {"press": "left"}})
        self.step(1)
        self._stamp({"pointer": {"release": "left"}})
        return self.step(1)

    # -- reading back -------------------------------------------------------

    def applied(self, name: str) -> dict | None:
        """The latest ack for ``name`` in the current snapshot."""
        for entry in reversed(self.snapshot.get("applied", [])):
            if entry.get("input") == name:
                return entry
        return None

    def target_rect(self, name: str) -> list | None:
        for target in self.snapshot.get("ui", {}).get("targets", []):
            if target["name"] == name:
                return target["rect"]
        return None

    def ship(self, ship_id: str) -> dict | None:
        for ship in self.snapshot.get("ships", []):
            if ship["id"] == ship_id:
                return ship
        return None

    def close(self) -> None:
        if self.proc.stdin is not None:
            self.proc.stdin.close()
        self.proc.wait(timeout=5)


def check(condition: bool, label: str) -> None:
    """A one-line assertion that narrates the transcript."""
    status = "ok  " if condition else "FAIL"
    print(f"  {status}  {label}")
    if not condition:
        sys.exit(1)
