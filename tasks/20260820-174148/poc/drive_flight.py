#!/usr/bin/env python3
"""PoC 1 -- the play loop: fly, lock, fire, all by name over stdin.

Runs against the real binary and the acceptance scenario (a hostile
raider parked 280 u down the burn line):

    python3 drive_flight.py --cmd "target/debug/nova-protocol --norender \
        --scenario-file tasks/20260820-174148/poc/acceptance.content.ron \
        --channel step"

Proves, against the shipping game:
  - a held named action is one start and one stop, surviving the frames
    between (main_drive accelerates for the whole hold);
  - the radar TAP clears while the HOLD latches -- the gesture decides;
  - a hull SECTION fires by its authored id, not by a named action;
  - an axis is a per-tick delta on its own lane;
  - the snapshot acknowledges every line in ``applied``.
"""

import sys

from channel import Channel, check, cmd_from_argv


def main() -> None:
    if cmd_from_argv() is None:
        sys.exit("drive_flight targets the real binary; see the docstring for --cmd")
    c = Channel()
    print("drive_flight: burn, lock, fire -- by name\n")

    snap = c.step(1)
    check("flight.main_drive" in snap["input"]["live"], "flight vocabulary is live")

    # Burn toward the raider: one start, frames of silence, one stop.
    c.press("flight.main_drive")
    snap = c.step(120)
    vel = c.ship("player")["linear_velocity"]
    print(f"tick {c.tick:>5}  main_drive held    vel={[round(v, 2) for v in vel]}")
    check(-vel[2] > 10, "two held seconds of burn accelerated the ship down -Z")
    c.release("flight.main_drive")
    c.step(1)

    # The tap: press and release inside the 0.25 s window. No lock.
    c.press("targeting.radar_hold")
    c.step(5)
    c.release("targeting.radar_hold")
    snap = c.step(1)
    ship = c.ship("player")
    print(f"tick {c.tick:>5}  radar tapped       lock={ship['combat_lock']}")
    check(ship["combat_lock"] is None, "a tap inside the window clears, not latches")

    # Raise the stance first: the radar sweep commits into the COMBAT slot
    # only while weapons are raised; stance-down it banks a travel lock.
    c.press("targeting.combat_stance")
    c.step(1)

    # The hold: cross the threshold, dwell on the candidate, release. The
    # lock sticks.
    c.press("targeting.radar_hold")
    snap = c.step(90)
    ship = c.ship("player")
    print(f"tick {c.tick:>5}  radar held         lock={ship['combat_lock']}")
    check(ship["combat_lock"] == "raider_1", "the threshold latched the lock")
    c.release("targeting.radar_hold")
    snap = c.step(1)
    check(c.ship("player")["combat_lock"] == "raider_1",
          "the release sticks; the lock stays")

    # Fire the SECTION by id.
    hull_before = c.ship("raider_1")["health"]["current"]
    c.press("section.turret_port")
    snap = c.step(180)
    hull_after = c.ship("raider_1")["health"]["current"]
    print(f"tick {c.tick:>5}  turret_port firing raider_1 hull "
          f"{hull_before} -> {hull_after}")
    check(hull_after < hull_before, "a section fired by its authored id")
    turret = next(s for s in c.ship("player")["sections"] if s["id"] == "turret_port")
    ammo = turret["weapon"]["ammo"]
    check(ammo["rounds"] < ammo["capacity"], "and the magazine paid for it")
    c.release("section.turret_port")
    c.release("targeting.combat_stance")
    c.step(1)

    # Aim deltas on the axis lane: the nose picks up angular velocity.
    for _ in range(3):
        c.aim("flight.rcs_aim", (3.0, -1.0))
    snap = c.step(1)
    spin = c.ship("player")["angular_velocity"]
    check(any(abs(w) > 1e-3 for w in spin), "aim deltas turned the nose")

    check(not c.errors, "no error lines in the whole session")
    print("\nPASS  combat lock through a driven hold, section fired by id")
    c.close()


if __name__ == "__main__":
    main()
