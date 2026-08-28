#!/usr/bin/env python3
"""PoC 2 -- the mode ladder: open NOVA OS, type at the prompt, run an app.

Proves, against the wire contract:
  - the mode toggle is an Always action, live from anywhere;
  - the ``text`` and ``key`` lanes reach the prompt while the named-action
    lane goes quiet there (Viewer is down at the prompt on purpose);
  - launching an app raises Viewer + ViewerApp, and the live set follows;
  - a flight verb sent while the viewer owns the screen is REFUSED, not
    pressed blind onto whatever holds the same key;
  - Escape walks the ladder back out, and the frozen clock never advanced.
"""

from channel import Channel, check


def main() -> None:
    c = Channel()
    print("drive_novaos: Tab, type map, Enter, refuse flight, Escape out\n")

    c.step(1)

    # The toggle is Always: live from flight.
    c.press("system.novaos_toggle")
    c.release("system.novaos_toggle")
    snap = c.step(1)
    ui = snap["ui"]
    # The freeze reference, taken once the pause state is up: the press-to-
    # pause transition itself spends real frames, so earlier ticks still ran.
    elapsed_before = snap["t_game"]
    print(f"tick {c.tick:>5}  novaos_toggle      pause={ui['pause']}  "
          f"mode={ui['computer']['mode']}")
    check(ui["pause"] == "NovaOs", "the computer owns the screen")
    check(ui["computer"]["mode"] == "prompt", "and the prompt has focus")
    check("flight.main_drive" not in snap["input"]["live"],
          "flight went quiet -- the clocks are frozen")
    check("nova_os.novaos_orbit_left" not in snap["input"]["live"],
          "and Viewer is down at the prompt: the keyboard is typing now")

    # Type at the prompt over the text lane, edit with the key lane.
    c.type_text("mapp")
    c.key("Backspace")
    snap = c.step(1)
    print(f"tick {c.tick:>5}  typed + backspace  prompt={ui_prompt(snap)!r}")
    check(ui_prompt(snap) == "map", "characters and edits reached the buffer")

    c.key("Enter")
    snap = c.step(1)
    mode = snap["ui"]["computer"]["mode"]
    print(f"tick {c.tick:>5}  Enter              mode={mode}")
    check(mode == "app:map", "the command launched the map viewer")
    check("nova_os.novaos_orbit_left" in snap["input"]["live"],
          "the viewer verbs are live in the app")
    check("map.map_goto" in snap["input"]["live"], "and so is the app's own verb")

    # The context rule: a flight verb is refused while the viewer is up.
    c.press("flight.autopilot_goto")
    snap = c.step(1)
    ack = c.applied("flight.autopilot_goto")
    print(f"tick {c.tick:>5}  autopilot_goto     state={ack['state']}")
    check(ack["state"] == "refused",
          "G means map_goto here; the flight verb is refused, not misfired")

    # The app's G, by its own name, and the shared viewer verb beside it.
    c.press("map.map_goto")
    c.release("map.map_goto")
    c.press("nova_os.novaos_orbit_left")
    c.release("nova_os.novaos_orbit_left")
    snap = c.step(1)
    goto = c.applied("map.map_goto")
    orbit = c.applied("nova_os.novaos_orbit_left")
    check(goto is not None and goto["state"] != "refused",
          "map_goto fires by its own name")
    check(orbit is not None and orbit["state"] != "refused",
          "and so does the shared orbit verb")

    # Escape: app -> prompt -> closed. The world never moved while frozen.
    c.key("Escape")
    snap = c.step(1)
    check(snap["ui"]["computer"]["mode"] == "prompt", "Escape backs out to the prompt")
    print(f"tick {c.tick:>5}  frozen stretch     frame={snap['frame']}  "
          f"t_game {elapsed_before} -> {snap['t_game']}")
    check(snap["t_game"] == elapsed_before,
          "frames ticked but the virtual clock held still throughout")
    # The close is a CRT power-off drawer, 0.22 s of real time, not a flag
    # flip: give it the slide plus a margin before asserting.
    c.key("Escape")
    snap = c.step(20)
    print(f"tick {c.tick:>5}  Escape x2          pause={snap['ui']['pause']}")
    check(snap["ui"]["computer"] is None, "the computer is down")
    check(snap["ui"]["pause"] == "Unpaused", "and the game resumes")
    snap = c.step(5)
    check(snap["t_game"] > elapsed_before, "and the clock runs again")

    # Back in flight, the refused verb now fires. The press ack carries the
    # Fired state; the release ack that follows is back to None.
    c.press("flight.autopilot_goto")
    c.release("flight.autopilot_goto")
    snap = c.step(1)
    acks = [e for e in snap.get("applied", [])
            if e.get("input") == "flight.autopilot_goto"]
    check(any(e["phase"] == "start" and e["state"] == "Fired" for e in acks),
          "the same line fires once its context is live")

    check(not c.errors, "no error lines in the whole session")
    print("\nPASS  typed into NOVA OS, drove the viewer, contexts refused honestly")
    c.close()


def ui_prompt(snap: dict) -> str:
    return snap["ui"]["computer"]["prompt"]


if __name__ == "__main__":
    main()
