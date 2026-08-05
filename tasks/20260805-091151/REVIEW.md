# Review: A stray cursor event mid-click cancels the driven click

- TASK: 20260805-091151
- BRANCH: master (landed in place, `87bcb956`)

## Round 1

- REVIEWER: in-session self-review of the landed diff
- VERDICT: APPROVE

- [x] R1.1 (MINOR) crates/nova_autopilot/src/input.rs:288 - the held test was
  `window.cursor_position() == Some(pinned.0)`, exact float equality on a value
  that round-trips through PHYSICAL pixels: the driver's own OS-level warp
  echoes back through winit as physical, and on a fractional scale factor the
  trip back to logical need not land on the same float. The pin would read its
  own echo as a stray and re-warp every frame - a warp/correction loop nothing
  in the run would explain. Xvfb runs at scale factor 1, so no test here would
  have caught it.
  - Response: fixed. Half a logical pixel of tolerance (`HELD_TOLERANCE_SQ`),
    with the reason in the rustdoc. No real gesture lands inside half a pixel
    of the pin, so the detector loses nothing.

- [x] R1.2 (MINOR) web/src/wiki/dev/automation-harness.md - the new paragraph
  states the mechanism but not the consequence a developer meets first: the pin
  holds the REAL cursor, because `Window::cursor_position` is what moves it. A
  driven run on a desktop now pulls the mouse back whenever it is moved off,
  for the length of the run. That is intended, and it is exactly the sort of
  thing that reads as a bug when it is undocumented.
  - Response: fixed, one sentence added.

### Considered and deliberately left alone

- **`PinnedCursor` is never cleared.** The pin therefore holds until the
  process exits. Correct for a driven run - there is no "after the script" in
  which a pointer should drift - and a clear-on-last-step would be a lifecycle
  with no caller asking for it.
- **The pin writes `Window::cursor_position` as well as the messages.**
  Picking only needs the messages, and dropping the window write would stop the
  run stealing the OS cursor. It stays because `nova_gameplay`'s NOVA OS CRT
  polls `Window::cursor_position` (`crates/nova_gameplay/src/hud/nova_os/crt.rs:269`):
  a pin that fixed picking and left the window reading the stray would put two
  halves of the app on different pointers - the exact split `set_cursor`'s
  docs already warn about.
- **No `hovered_named` / `pressed_named` predicates.** They do not close this
  class (a stray can still land between an observed press and the release) and
  belong to the epic's frame-count anti-pattern, not to this fix. Recorded in
  `DECISION.md` as idea 2 rather than dropped.

### Verification (re-derived in-session, not taken from the reviewer)

| Check | Result |
| --- | --- |
| `cargo test -p nova_autopilot --lib --test pointer_pin` | 38 + 2 pass |
| Same, with the `register_pointer_pin` call deleted | `a_foreign_cursor_event_mid_click_does_not_cancel_the_click` FAILS |
| `menu_newgame`, `editor`, `widget_zoo` after the round-1 fixes | 3/3 pass |
| All 8 pointer-driving examples, `ui/` + `systems/` | 8/8 pass |
| Suite-shaped round, 5 categories in parallel | 23/23 pass |
| `RUSTFLAGS=-Dwarnings cargo check -p nova_autopilot --all-targets`, `cargo fmt --check` | clean |

Not run locally, per the standing instruction: full `cargo test` and
`cargo clippy`. CI owns both.

One failure was seen while running the DoD command and is NOT this fix:
`menu_scenarios` killed by a signal, 1 run in 5, mid scenario load, no panic
and no stall. Filed as `20260805-111329`.
