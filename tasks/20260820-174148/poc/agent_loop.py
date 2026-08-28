#!/usr/bin/env python3
"""PoC 4 -- the third payoff: an agent plays, with the clock waiting on it.

Runs against the real binary and the acceptance scenario, like
drive_flight.py. The loop is observe -> decide -> act; ``decide`` is the
pluggable policy in the LLM's chair: it reads the snapshot as data and
answers with lane calls. Step mode means the world moves ONLY between
snapshots -- however long the deciding takes, the observation never goes
stale. That is the whole reason step mode exists.
"""

import math
import sys

from channel import Channel, check, cmd_from_argv


def decide(c: Channel, snap: dict, held: set) -> None:
    """The policy in the LLM's chair: close, lock, raise, fire."""
    me, raider = c.ship("player"), c.ship("raider_1")
    if raider is None:
        return
    distance = math.dist(raider["position"], me["position"])

    def want(name: str, condition: bool) -> None:
        if condition and name not in held:
            c.press(name)
            held.add(name)
        elif not condition and name in held:
            c.release(name)
            held.discard(name)

    if "targeting.radar_hold" in snap["input"]["live"]:
        want("flight.main_drive", distance > 250)
        # Stance up before the sweep: the radar commits a COMBAT lock only
        # while weapons are raised.
        want("targeting.combat_stance", True)
        want("targeting.radar_hold", me["combat_lock"] is None)
        want("section.turret_port", me["combat_lock"] is not None)
        want("section.turret_starboard", me["combat_lock"] is not None)


def main() -> None:
    if cmd_from_argv() is None:
        sys.exit("agent_loop targets the real binary; see drive_flight.py for --cmd")
    c = Channel()
    print("agent_loop: observe -> decide -> act, on the step clock\n")

    held: set = set()
    snap = c.step(1)
    raider = c.ship("raider_1")
    for round_no in range(1, 41):
        decide(c, snap, held)  # the slow part would happen here; the world waits
        snap = c.step(30)  # then grant the world half a second
        me, raider = c.ship("player"), c.ship("raider_1")
        if raider is None or raider["defeated"]:
            break
        print(
            f"round {round_no:>2}  frame {snap['frame']:>5}  "
            f"dist={math.dist(raider['position'], me['position']):>7.1f}  "
            f"lock={str(me['combat_lock']):<8}  "
            f"raider hull={raider['health']['current']:>7.1f}"
        )

    if raider is not None and raider["defeated"]:
        print(f"round {round_no:>2}  raider defeated    "
              f"hull={raider['health']['current']:>7.1f}")
    check(raider is None or raider["defeated"], "the agent hunted the raider down")
    check(not c.errors, "no error lines in the whole session")
    print("\nPASS  an agent played the game and the clock waited for it")
    c.close()


if __name__ == "__main__":
    main()
