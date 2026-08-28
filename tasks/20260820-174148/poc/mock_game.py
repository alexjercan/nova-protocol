#!/usr/bin/env python3
"""A stand-in for ``nova --norender`` speaking the nova_channel wire.

This is the design page's schema made executable: five input lanes, the
reserved refusals, the registry with contexts and the live set, the step
clock, the freeze model, and the pointer census. The world behind it is a
toy -- one player corvette, one raider, straight-line kinematics -- because
the mock exists to prove the CONTRACT, not the simulation.

Rules kept honest on purpose, because drivers must design around them:
  - the radar hold has a latch threshold and a tap window, like the real rig;
  - a click is release-over, inside the same target the press started in;
  - a named input whose context is not live is refused, not pressed blind;
  - ``action`` and ``command`` lanes are refused with the follow-up task id;
  - the virtual clocks freeze while a menu or NOVA OS owns the screen, but
    the frame counter keeps climbing -- schedules run, the world does not.

Snapshot records carry a representative subset of capture_snapshot's real
fields; the header, ``applied``, ``input`` and ``ui`` blocks are the full
proposed shape.
"""

from __future__ import annotations

import json
import sys

TICK_DT = 1.0 / 60.0
RADAR_LATCH_TICKS = 60  # hold this long to take the lock
RADAR_TAP_TICKS = 15  # release before this clears it
TURRET_DAMAGE_PER_TICK = 0.35
MAIN_DRIVE_ACCEL = 40.0
WINDOW = (1280, 720)

# name -> (group, context). Context: "always" | "flight" | "viewer"
# | "viewer:<app>". The full 33-action table from the bindings registry;
# sources are modelled only where a driver needs them.
REGISTRY = {
    "main_drive": ("flight", "flight"),
    "autopilot_stop": ("flight", "flight"),
    "autopilot_goto": ("flight", "flight"),
    "autopilot_orbit": ("flight", "flight"),
    "autopilot_off": ("flight", "flight"),
    "rcs_modifier": ("flight", "flight"),
    "rcs_aim": ("flight", "flight"),
    "radar_hold": ("targeting", "flight"),
    "radar_clear": ("targeting", "flight"),
    "component_next": ("targeting", "flight"),
    "component_prev": ("targeting", "flight"),
    "combat_stance": ("targeting", "flight"),
    "camera_rotate": ("camera", "flight"),
    "free_look": ("camera", "flight"),
    "scenario_advance": ("scenario", "flight"),
    "novaos_toggle": ("system", "always"),
    "hud_cinematic": ("system", "always"),
    "novaos_orbit_left": ("nova_os", "viewer"),
    "novaos_orbit_right": ("nova_os", "viewer"),
    "novaos_orbit_up": ("nova_os", "viewer"),
    "novaos_orbit_down": ("nova_os", "viewer"),
    "novaos_pan_forward": ("nova_os", "viewer"),
    "novaos_pan_back": ("nova_os", "viewer"),
    "novaos_pan_left": ("nova_os", "viewer"),
    "novaos_pan_right": ("nova_os", "viewer"),
    "novaos_reframe": ("nova_os", "viewer"),
    "novaos_next": ("nova_os", "viewer"),
    "novaos_prev": ("nova_os", "viewer"),
    "map_goto": ("map", "viewer:map"),
    "ship_mates": ("ship", "viewer:ship"),
    "ship_reload": ("ship", "viewer:ship"),
    "ship_repair": ("ship", "viewer:ship"),
    "ship_rebind": ("ship", "viewer:ship"),
}

AXES = {"rcs_aim", "camera_rotate"}
SECTIONS = {"turret_port", "turret_starboard"}
EDIT_KEYS = {
    "Enter", "Tab", "Backspace", "Delete", "Escape", "Space",
    "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "PageUp", "PageDown",
}
TERMINAL_COMMANDS = {"help", "map", "ship", "exit", "clear", "version"}


def wire_name(name: str) -> str:
    group, _ = REGISTRY[name]
    return f"{group}.{name}"


class Game:
    def __init__(self) -> None:
        self.tick = 0
        self.elapsed = 0.0  # virtual: holds still while frozen
        self.pause = "Unpaused"  # Unpaused | Paused | NovaOs
        self.computer = None  # None | {"mode","prompt","cursor","yaw"}
        self.goto_marker = None
        self.player = {
            "pos": 0.0, "vel": 0.0, "hull": 100.0, "aim_yaw": 0.0,
            "weapons_hot": False, "combat_lock": None,
            "ammo": {"turret_port": 120, "turret_starboard": 120},
        }
        self.raider = {"pos": 800.0, "hull": 62.0, "defeated": False}
        self.pressed: set[str] = set()  # named actions currently held
        self.sections_firing: set[str] = set()
        self.radar_held_ticks = 0
        self.cursor = None  # [x, y] logical px
        self.press_started_in = None  # target name the LMB press began over
        self.applied: list[dict] = []
        self.line_no = 0

    # -- contexts and the live set ------------------------------------------

    def live_contexts(self) -> list[str]:
        live = ["always"]
        if self.pause == "Unpaused" and self.player["hull"] > 0:
            live.append("flight")
        if self.computer and self.computer["mode"].startswith("app:"):
            live.append("viewer")
            live.append("viewer:" + self.computer["mode"][4:])
        return live

    def live_actions(self) -> list[str]:
        live = self.live_contexts()
        return [wire_name(n) for n, (_, ctx) in REGISTRY.items() if ctx in live]

    # -- lane application ----------------------------------------------------

    def ack(self, line_no: int, name: str, phase: str, state: str) -> None:
        self.applied.append(
            {"line": line_no, "input": name, "phase": phase, "state": state}
        )

    def error(self, message: str, line_no: int) -> None:
        emit({"schema": 1, "error": message, "line": line_no})

    def apply_input(self, line_no: int, wire: str, phase: str) -> None:
        if wire.startswith("section."):
            section = wire.split(".", 1)[1]
            if section not in SECTIONS:
                return self.error(f"no section `{section}` on the ship", line_no)
            if "flight" not in self.live_contexts():
                return self.ack(line_no, wire, phase, "refused")
            if phase == "start":
                self.sections_firing.add(section)
            else:
                self.sections_firing.discard(section)
            return self.ack(line_no, wire, phase, "Fired")

        name = wire.split(".", 1)[1] if "." in wire else wire
        if name not in REGISTRY or wire != wire_name(name):
            return self.error(f"no action named `{wire}`", line_no)
        if name in AXES:
            return self.error(f"`{wire}` has no button; it is an axis", line_no)
        _, ctx = REGISTRY[name]
        if ctx not in self.live_contexts():
            return self.ack(line_no, wire, phase, "refused")

        if phase == "start":
            self.pressed.add(name)
            self.on_press(name)
        else:
            self.pressed.discard(name)
            self.on_release(name)
        self.ack(line_no, wire, phase, self.trigger_state(name))

    def trigger_state(self, name: str) -> str:
        if name == "radar_hold":
            if self.player["combat_lock"]:
                return "Fired"
            return "Ongoing" if name in self.pressed else "None"
        return "Fired"

    def on_press(self, name: str) -> None:
        if name == "radar_hold":
            self.radar_held_ticks = 0
        elif name == "combat_stance":
            self.player["weapons_hot"] = True
        elif name == "novaos_toggle":
            if self.pause == "Unpaused":
                self.pause = "NovaOs"
                self.computer = {"mode": "prompt", "prompt": "", "cursor": 0, "yaw": 0.0}
        elif name == "map_goto":
            self.goto_marker = self.raider["pos"]
        elif name == "novaos_orbit_left":
            self.computer["yaw"] -= 15.0
        elif name == "novaos_orbit_right":
            self.computer["yaw"] += 15.0

    def on_release(self, name: str) -> None:
        if name == "radar_hold":
            if self.radar_held_ticks < RADAR_TAP_TICKS:
                self.player["combat_lock"] = None  # the tap clears
            # past the latch the release sticks: the lock stays
        elif name == "combat_stance":
            self.player["weapons_hot"] = False

    def apply_aim(self, line_no: int, payload: dict) -> None:
        wire = payload.get("name", "")
        name = wire.split(".", 1)[1] if "." in wire else wire
        if name not in AXES:
            return self.error(f"`{wire}` is not driven by an axis", line_no)
        if "flight" not in self.live_contexts():
            return self.ack(line_no, wire, "delta", "refused")
        delta = payload.get("delta", [0.0, 0.0])
        self.player["aim_yaw"] += delta[0] * 0.1
        self.ack(line_no, wire, "delta", "Fired")

    def apply_text(self, line_no: int, text: str) -> None:
        if not (self.computer and self.computer["mode"] == "prompt"):
            return self.ack(line_no, "text", "type", "refused")
        c = self.computer
        c["prompt"] = c["prompt"][: c["cursor"]] + text + c["prompt"][c["cursor"]:]
        c["cursor"] += len(text)
        self.ack(line_no, "text", "type", "Fired")

    def apply_key(self, line_no: int, key: str) -> None:
        if key not in EDIT_KEYS:
            return self.error(f"`{key}` is not an editing key", line_no)
        self.ack(line_no, f"key.{key}", "tap", "Fired")
        if key == "Escape":
            return self.on_escape()
        if self.computer and self.computer["mode"] == "prompt":
            return self.prompt_key(key)

    def on_escape(self) -> None:
        if self.computer:
            if self.computer["mode"].startswith("app:"):
                self.computer["mode"] = "prompt"
            else:
                self.computer = None
                self.pause = "Unpaused"
        elif self.pause == "Paused":
            self.pause = "Unpaused"
        elif self.pause == "Unpaused":
            self.pause = "Paused"

    def prompt_key(self, key: str) -> None:
        c = self.computer
        if key == "Enter":
            word = c["prompt"].strip()
            c["prompt"], c["cursor"] = "", 0
            if word in ("map", "ship"):
                c["mode"] = "app:" + word
            elif word == "exit":
                self.computer = None
                self.pause = "Unpaused"
        elif key == "Backspace" and c["cursor"] > 0:
            c["prompt"] = c["prompt"][: c["cursor"] - 1] + c["prompt"][c["cursor"]:]
            c["cursor"] -= 1
        elif key == "ArrowLeft":
            c["cursor"] = max(0, c["cursor"] - 1)
        elif key == "ArrowRight":
            c["cursor"] = min(len(c["prompt"]), c["cursor"] + 1)

    def apply_pointer(self, line_no: int, payload: dict) -> None:
        if "to" in payload:
            target = payload["to"]
            if isinstance(target, str):
                rect = dict((t["name"], t["rect"]) for t in self.ui_targets()).get(target)
                if rect is None:
                    return self.error(f"no visible target named `{target}`", line_no)
                self.cursor = [rect[0] + rect[2] / 2, rect[1] + rect[3] / 2]
            else:
                self.cursor = [float(target[0]), float(target[1])]
            return self.ack(line_no, "pointer", "move", "Fired")
        if "press" in payload:
            self.press_started_in = self.target_under_cursor()
            return self.ack(line_no, "pointer", "press", "Fired")
        if "release" in payload:
            over = self.target_under_cursor()
            if over is not None and over == self.press_started_in:
                self.activate(over)  # release-over: the same rule as Activate
            self.press_started_in = None
            return self.ack(line_no, "pointer", "release", "Fired")
        if "wheel" in payload:
            return self.ack(line_no, "pointer", "wheel", "Fired")
        self.error("pointer payload needs to/press/release/wheel", line_no)

    def target_under_cursor(self) -> str | None:
        if self.cursor is None:
            return None
        x, y = self.cursor
        for t in self.ui_targets():
            rx, ry, rw, rh = t["rect"]
            if rx <= x <= rx + rw and ry <= y <= ry + rh:
                return t["name"]
        return None

    def activate(self, name: str) -> None:
        if name == "Resume Button":
            self.pause = "Unpaused"
        elif name == "Exit Game":
            emit(self.snapshot("exit"))
            sys.exit(0)

    def ui_targets(self) -> list[dict]:
        if self.pause == "Paused":
            w, x = 128, WINDOW[0] / 2 - 64
            return [
                {"name": "Resume Button", "rect": [x, 312, w, 36]},
                {"name": "Settings", "rect": [x, 360, w, 36]},
                {"name": "Exit Game", "rect": [x, 408, w, 36]},
            ]
        return []

    # -- one tick of the world ----------------------------------------------

    def advance(self) -> None:
        self.tick += 1
        if self.pause != "Unpaused":
            return  # schedules run; the clocks and the world do not
        self.elapsed += TICK_DT

        if "radar_hold" in self.pressed:
            self.radar_held_ticks += 1
            if self.radar_held_ticks == RADAR_LATCH_TICKS and not self.raider["defeated"]:
                self.player["combat_lock"] = "raider_1"

        if "main_drive" in self.pressed:
            self.player["vel"] += MAIN_DRIVE_ACCEL * TICK_DT
        self.player["pos"] += self.player["vel"] * TICK_DT

        can_fire = (
            self.player["weapons_hot"]
            and self.player["combat_lock"]
            and not self.raider["defeated"]
        )
        if can_fire:
            for section in self.sections_firing:
                if self.player["ammo"][section] > 0:
                    self.player["ammo"][section] -= 1
                    self.raider["hull"] -= TURRET_DAMAGE_PER_TICK
        if self.raider["hull"] <= 0 and not self.raider["defeated"]:
            self.raider["defeated"] = True
            self.player["combat_lock"] = None

    # -- output --------------------------------------------------------------

    def snapshot(self, reason: str) -> dict:
        ui = {"pause": self.pause, "computer": None, "targets": self.ui_targets()}
        if self.computer is not None:
            ui["computer"] = {
                "mode": self.computer["mode"],
                "prompt": self.computer["prompt"],
                "cursor": self.computer["cursor"],
                "yaw": self.computer["yaw"],
            }
        snapshot = {
            "schema": 1,
            "reason": reason,
            "frame": self.tick,
            "scenario": "poc_skirmish",
            "game_state": "Playing",
            "elapsed": round(self.elapsed, 4),
            "t_game": round(self.elapsed, 4),
            "t_real": round(self.tick * TICK_DT, 4),
            "applied": self.applied,
            "input": {"live": sorted(self.live_actions()),
                      "contexts": self.live_contexts()},
            "ui": ui,
            "ships": [
                {
                    "id": "player", "allegiance": "player",
                    "position": [round(self.player["pos"], 2), 0.0, 0.0],
                    "linear_velocity": [round(self.player["vel"], 2), 0.0, 0.0],
                    "health": {"current": self.player["hull"], "max": 100.0},
                    "defeated": False,
                    "weapons_hot": self.player["weapons_hot"],
                    "combat_lock": self.player["combat_lock"],
                    "aim_yaw": round(self.player["aim_yaw"], 2),
                    "goto_marker": self.goto_marker,
                    "sections": [
                        {"id": s, "alive": True,
                         "firing": s in self.sections_firing,
                         "ammo": {"rounds": self.player["ammo"][s], "capacity": 120}}
                        for s in sorted(SECTIONS)
                    ],
                },
                {
                    "id": "raider_1", "allegiance": "raider",
                    "position": [round(self.raider["pos"], 2), 0.0, 0.0],
                    "linear_velocity": [0.0, 0.0, 0.0],
                    "health": {"current": round(max(self.raider["hull"], 0.0), 2),
                               "max": 62.0},
                    "defeated": self.raider["defeated"],
                    "sections": [],
                },
            ],
            "ordnance": [],
        }
        self.applied = []
        return snapshot

    # -- the step loop -------------------------------------------------------

    def run_step_mode(self) -> None:
        scheduled: dict[int, list[tuple[int, dict]]] = {}
        target = 0
        for raw in sys.stdin:
            raw = raw.strip()
            if not raw:
                continue
            self.line_no += 1
            try:
                line = json.loads(raw)
            except json.JSONDecodeError as err:
                self.error(f"not a JSON object: {err}", self.line_no)
                continue
            for reserved in ("action", "command"):
                if reserved in line:
                    self.error(
                        f"`{reserved}` is reserved; the console vocabulary "
                        "is task 20260827-120347",
                        self.line_no,
                    )
                    break
            else:
                tick = line.get("tick", self.tick + 1)
                if tick <= self.tick:
                    self.error(f"tick {tick} is in the past", self.line_no)
                    continue
                payload = {k: v for k, v in line.items() if k != "tick"}
                if len(payload.keys() - {"phase"}) > 1:
                    self.error("one payload key per line", self.line_no)
                    continue
                target = max(target, tick)
                if payload:
                    scheduled.setdefault(tick, []).append((self.line_no, payload))
                else:
                    # a bare tick is the step instruction: run to the target
                    while self.tick < target:
                        for line_no, queued in scheduled.pop(self.tick + 1, []):
                            self.apply_line(line_no, queued)
                        self.advance()
                    emit(self.snapshot("step"))

    def apply_line(self, line_no: int, payload: dict) -> None:
        if "input" in payload:
            self.apply_input(line_no, payload["input"],
                             payload.get("phase", "start"))
        elif "aim" in payload:
            self.apply_aim(line_no, payload["aim"])
        elif "text" in payload:
            self.apply_text(line_no, payload["text"])
        elif "key" in payload:
            self.apply_key(line_no, payload["key"])
        elif "pointer" in payload:
            self.apply_pointer(line_no, payload["pointer"])
        else:
            self.error(f"unknown lane {sorted(payload)}", line_no)


def emit(obj: dict) -> None:
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


if __name__ == "__main__":
    if "--step" not in sys.argv:
        sys.exit("only --step is modelled; free-running is the crate's default")
    Game().run_step_mode()
