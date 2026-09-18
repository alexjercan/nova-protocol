#!/usr/bin/env python3
"""Deterministic `cmd:` pilot for the two bench proofs of the governor removal.

Speaks the referee protocol on NOVA_BENCH_SOCKET (one JSON request per line).
No model, no judgement: a fixed script, so two runs of one plan compare.

Plans
-----
straight-burn 12 s of held throttle from rest, down the range's own nose.
              Records what an uncapped drive reaches, and what it reaches it
              at: Kestrel sits 7 km dead ahead of the spawn.
control-feel  turn -> accelerate -> coast -> turn across the velocity -> hold
              the main drive while crossing -> STOP. Prints speed and the
              along-nose component of the velocity at every sample, which is
              the same quantity `before.txt` measured at the cap. Flown
              across the planetoid's line rather than down it, so nothing in
              the numbers is a collision or a gravity well.
tutorial      Basic Training's opening card, flown by the objectives the
              scenario posts: burn to mark ALPHA with the throttle held, then
              STOP when the range asks for it. Records the speed the leg
              reaches and whether the gate still fires.

Usage: NOVA_BENCH_SOCKET=... drive.py <plan>
"""

import json
import math
import os
import socket
import sys

SOCK = os.environ["NOVA_BENCH_SOCKET"]


class Referee:
    def __init__(self):
        self.conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.conn.connect(SOCK)
        self.rx = self.conn.makefile("r")

    def call(self, request):
        self.conn.sendall((json.dumps(request) + "\n").encode())
        line = self.rx.readline()
        if not line:
            raise SystemExit("referee closed the socket")
        reply = json.loads(line)
        if "error" in reply:
            raise SystemExit("referee refused: %s" % reply["error"])
        return reply["ok"]

    def observe(self):
        return self.call({"observe": {}})

    def act(self, gestures, ticks):
        return self.call({"act": {"gestures": gestures, "ticks": ticks}})

    def finish(self, status, report):
        return self.call({"finish": {"status": status, "report": report}})


def along_nose(view):
    """Speed component along the nose, in m/s. Positive is forward."""
    me = view["me"]
    speed = me.get("speed_mps") or 0.0
    bearing = me.get("velocity_bearing_deg")
    if not bearing:
        return 0.0
    az = math.radians(bearing[0])
    el = math.radians(bearing[1])
    return speed * math.cos(az) * math.cos(el)


def line(tag, view):
    me = view["me"]
    pos = me.get("position_m") or [0, 0, 0]
    print(
        "%-14s t=%7.2fs speed=%8.2f along_nose=%9.2f vel_bearing=%s pos=[%.0f %.0f %.0f]"
        % (
            tag,
            view.get("game_seconds", 0.0),
            me.get("speed_mps") or 0.0,
            along_nose(view),
            me.get("velocity_bearing_deg"),
            pos[0],
            pos[1],
            pos[2],
        ),
        flush=True,
    )


def objectives(view):
    return [o.get("text") or o.get("id") for o in (view.get("objectives") or [])]


def log_has(view, kind, card):
    return any(
        e.get("kind") == kind and e.get("id") == card
        for e in (view.get("objective_log") or [])
    )


def posted(view, card):
    return log_has(view, "posted", card)


def completed(view, card):
    return log_has(view, "completed", card)


def turn_starboard(ref, degrees, tag):
    """Yaw the hull `degrees` to starboard and let it settle. 27 px per degree."""
    ref.act(
        [{"aim": "camera.camera_rotate", "delta": [degrees * 27.0 / 30.0, 0.0], "ticks": 30}],
        30,
    )
    view = None
    for _ in range(5):
        view = ref.act([], 60)
        line(tag, view)
    return view


def straight_burn(ref):
    view = ref.observe()
    line("start", view)
    print("withheld: %s" % view["me"].get("withheld_capabilities"), flush=True)
    ref.act([{"press": "flight.main_drive"}], 1)
    for _ in range(12):
        view = ref.act([], 60)
        near = [b for b in (view.get("bodies") or {}).get("near", [])]
        line("burn", view)
        for body in near:
            print(
                "    near: %s surface=%.0f m closing=%.0f m/s"
                % (body.get("kind"), body.get("surface_m", -1), body.get("closing_mps", 0)),
                flush=True,
            )
    view = ref.act([{"release": "flight.main_drive"}], 60)
    line("after", view)
    print("hull_plates: %s health: %s" % (view["me"].get("hull_plates"), view["me"].get("health")))
    ref.finish("done", "straight-burn plan flown")


def control_feel(ref):
    view = ref.observe()
    line("start", view)
    print("withheld: %s" % view["me"].get("withheld_capabilities"), flush=True)

    # 0. Turn off the planetoid's line first: this plan measures the drive,
    #    not a collision and not a gravity well.
    turn_starboard(ref, 90.0, "turn-off-line")

    # 1. ACCELERATE. Hold the main drive for 6 s, sampling every second.
    ref.act([{"press": "flight.main_drive"}], 1)
    for _ in range(6):
        view = ref.act([], 60)
        line("accelerate", view)
    view = ref.act([{"release": "flight.main_drive"}], 1)
    speed_after_burn = view["me"]["speed_mps"]

    # 2. Releasing the throttle must preserve the velocity.
    view = ref.act([], 300)
    line("coast-5s", view)
    speed_coast = view["me"]["speed_mps"]

    # 3. TURN. 90 degrees to starboard puts the velocity across the nose.
    view = turn_starboard(ref, 90.0, "turn")
    speed_after_turn = view["me"]["speed_mps"]
    bearing_after_turn = view["me"]["velocity_bearing_deg"]
    nose_before_cross = along_nose(view)

    # 4. CROSS-BURN. The before artifact's exact state: the ship is fast, the
    #    velocity is across the nose, and the pilot holds the throttle down.
    ref.act([{"press": "flight.main_drive"}], 1)
    for _ in range(10):
        view = ref.act([], 60)
        line("cross-burn", view)
    view = ref.act([{"release": "flight.main_drive"}], 30)
    line("release", view)
    nose_after_cross = along_nose(view)
    speed_after_cross = view["me"]["speed_mps"]

    # 5. STOP. Does it still converge to rest from here?
    ref.act([{"tap": "flight.autopilot_stop"}], 1)
    stop_start = view.get("game_seconds", 0.0)
    stopped_at = None
    for _ in range(40):
        view = ref.act([], 60)
        line("stop", view)
        if (view["me"]["speed_mps"] or 0.0) < 1.0 and not (
            view["me"].get("autopilot") or {}
        ).get("engaged"):
            stopped_at = view.get("game_seconds", 0.0)
            break

    print("")
    print("SUMMARY")
    print("  speed after a 6 s burn from rest  : %.2f m/s" % speed_after_burn)
    print("  speed 5 s after releasing the burn: %.2f m/s" % speed_coast)
    print("  speed after the 90 deg turn       : %.2f m/s" % speed_after_turn)
    print("  velocity bearing after the turn   : %s" % (bearing_after_turn,))
    print("  along-nose before the cross-burn  : %.2f m/s" % nose_before_cross)
    print("  along-nose after the cross-burn   : %.2f m/s" % nose_after_cross)
    print(
        "  along-nose delta-v while crossing : %.2f m/s"
        % (nose_after_cross - nose_before_cross)
    )
    print("  speed at the end of the cross-burn: %.2f m/s" % speed_after_cross)
    if stopped_at is None:
        print("  STOP                              : DID NOT converge inside 40 s")
    else:
        print(
            "  STOP converged in                 : %.2f s, final speed %.3f m/s"
            % (stopped_at - stop_start, view["me"]["speed_mps"] or 0.0)
        )
    ref.finish("done", "control-feel plan flown")


def tutorial(ref):
    view = ref.observe()
    line("boot", view)

    # The opening scene plays itself out; wait for the helm and the first card.
    for _ in range(60):
        if view.get("cinematic") is None and objectives(view):
            break
        view = ref.act([], 60)
    print("objectives after the briefing: %s" % objectives(view), flush=True)
    print("withheld: %s" % view["me"].get("withheld_capabilities"), flush=True)
    line("helm", view)

    # BEAT 1: burn to mark ALPHA, 900 m dead ahead behind a 300 m gate.
    ref.act([{"press": "flight.main_drive"}], 1)
    peak = 0.0
    entered = None
    alpha_speed = 0.0
    alpha_pos = [0.0, 0.0, 0.0]
    for _ in range(40):
        view = ref.act([], 30)
        peak = max(peak, view["me"]["speed_mps"] or 0.0)
        line("burn-alpha", view)
        if completed(view, "burn"):
            entered = view.get("game_seconds", 0.0)
            alpha_speed = view["me"]["speed_mps"] or 0.0
            alpha_pos = view["me"]["position_m"]
            break
    view = ref.act([{"release": "flight.main_drive"}], 1)
    print("objectives after ALPHA: %s" % objectives(view), flush=True)
    print("comms: %s" % [c["text"] for c in (view.get("comms") or [])[-2:]], flush=True)

    # BEAT 2: the range asks for STOP, and hands the capability over with the
    # card. Wait for both, then tap [X] and keep hands off.
    posted_at = None
    for _ in range(60):
        view = ref.act([], 30)
        if posted(view, "stop") and "Stop" not in (
            view["me"].get("withheld_capabilities") or []
        ):
            posted_at = view.get("game_seconds", 0.0)
            break
    print("STOP card posted at %s, withheld now %s"
          % (posted_at, view["me"].get("withheld_capabilities")), flush=True)
    line("stop-card", view)
    stop_speed = view["me"]["speed_mps"] or 0.0
    stop_pos = view["me"]["position_m"]
    ref.act([{"tap": "flight.autopilot_stop"}], 1)
    stopped_at = None
    for _ in range(60):
        view = ref.act([], 60)
        line("stop", view)
        if completed(view, "stop"):
            stopped_at = view.get("game_seconds", 0.0)
            break
    rest_pos = view["me"]["position_m"]
    print("objectives after STOP: %s" % objectives(view), flush=True)
    print("comms: %s" % [c["text"] for c in (view.get("comms") or [])[-2:]], flush=True)
    print("objective_log: %s" % [
        "%s %s" % (e["kind"], e["id"]) for e in (view.get("objective_log") or [])
    ], flush=True)

    print("")
    print("SUMMARY")
    print("  peak speed on the ALPHA leg       : %.2f m/s" % peak)
    if entered is None:
        print("  ALPHA card                        : NEVER completed inside 20 s of burn")
    else:
        print("  ALPHA card completed at           : %.2f game seconds" % entered)
    print(
        "  speed and position at that moment : %.2f m/s at [%.0f %.0f %.0f]"
        % (alpha_speed, alpha_pos[0], alpha_pos[1], alpha_pos[2])
    )
    print(
        "  at the STOP card                  : %.2f m/s at [%.0f %.0f %.0f]"
        % (stop_speed, stop_pos[0], stop_pos[1], stop_pos[2])
    )
    if stopped_at is None:
        print("  STOP card                         : DID NOT complete inside 60 s")
    else:
        print("  STOP card completed at            : %.2f game seconds" % stopped_at)
    print(
        "  at rest                           : [%.0f %.0f %.0f], mark ALPHA is at [0 0 -900]"
        % (rest_pos[0], rest_pos[1], rest_pos[2])
    )
    line("end", view)
    ref.finish("done", "tutorial opening card flown")


PLANS = {
    "straight-burn": straight_burn,
    "control-feel": control_feel,
    "tutorial": tutorial,
}

if __name__ == "__main__":
    plan = sys.argv[1]
    PLANS[plan](Referee())
