#!/usr/bin/env python3
"""PoC 3 -- the pointer lane: click a button that was never drawn.

Proves, against the wire contract:
  - the snapshot's ``ui.targets`` census is what makes a named click honest:
    the driver clicks what it was told exists, where it was told it is;
  - a click is move / press / release on separate ticks, and it activates on
    RELEASE-OVER -- pressing here and releasing elsewhere activates nothing,
    exactly the bevy_picking Activate rule;
  - raw logical pixels work as the fallback for a target with no Name.
"""

from channel import Channel, check


def main() -> None:
    c = Channel()
    print("drive_pointer: Escape to the menu, click Resume by name, then by pixels\n")

    # Escape opens the pause menu; the census appears.
    c.key("Escape")
    snap = c.step(1)
    names = [t["name"] for t in snap["ui"]["targets"]]
    print(f"tick {c.tick:>5}  Escape             pause={snap['ui']['pause']}  "
          f"targets={names}")
    check(snap["ui"]["pause"] == "Paused", "the pause menu is up")
    check("Resume Button" in names, "the census advertises Resume")

    # A press that releases somewhere else activates nothing.
    c.pointer_to("Resume Button")
    c.step(1)
    c._stamp({"pointer": {"press": "left"}})
    c.step(1)
    c.pointer_to([10, 10])
    c.step(1)
    c._stamp({"pointer": {"release": "left"}})
    snap = c.step(1)
    print(f"tick {c.tick:>5}  press, drag away   pause={snap['ui']['pause']}")
    check(snap["ui"]["pause"] == "Paused",
          "release-over is the rule: dragging off the button cancels the click")

    # The named click.
    snap = c.click("Resume Button")
    print(f"tick {c.tick:>5}  click 'Resume Button'     pause={snap['ui']['pause']}")
    check(snap["ui"]["pause"] == "Unpaused", "the named click resumed the game")

    # Pause again and click the same button by raw logical pixels.
    c.key("Escape")
    snap = c.step(1)
    rect = c.target_rect("Resume Button")
    check(rect is not None, "the census carries the rect a raw click needs")
    centre = [rect[0] + rect[2] / 2, rect[1] + rect[3] / 2]
    snap = c.click(centre)
    print(f"tick {c.tick:>5}  click {centre}     pause={snap['ui']['pause']}")
    check(snap["ui"]["pause"] == "Unpaused", "the raw-pixel fallback works too")

    check(not c.errors, "no error lines in the whole session")
    print("\nPASS  clicked a menu that only exists as layout, by name and by pixel")
    c.close()


if __name__ == "__main__":
    main()
