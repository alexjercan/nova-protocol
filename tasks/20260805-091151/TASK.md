# A stray cursor event mid-click cancels the driven click

- STATUS: CLOSED
- PRIORITY: 84
- TAGS: v0.10.0, testing, automation

TITLE CORRECTED. The brief below diagnosed a same-frame press; the
investigation disproved it (`NOTES.md`). The press always lands. What fails is
the RELEASE: `Pointer<Click>` is dispatched from the previous frame's hover
map, so any pointer event that moves the hover off the widget between the two
beats cancels the click silently.

## Story

An intermittent `examples_smoke` failure, seen by the owner 2026-08-05 and
diagnosed (not fixed) under `20260804-095507`:

```
ERROR nova_autopilot::autopilot: autopilot: step `menu_newgame: release New Game`
  stalled after 90.0s (run 91.7s, state MainMenu)
test result: FAILED. 8 passed; 1 failed
```

REPRODUCED 2026-08-05 on a full workspace suite run, in the same category but
a DIFFERENT example - so it is the shared `click_named` mechanism, not one
example's script:

```
autopilot: step `editor: release Sandbox` begins      06:18:53.986944
ERROR nova_autopilot::completion: harness completion: deadline (120s) expired
  with collectors still pending: ["autopilot"]        06:20:52.074566
example editor exited with Some(1)
```

Rate is roughly 1 in 3 full runs (2 failures across 3 `examples_smoke` runs
on 2026-08-05; the other passed 9/9).

State stayed `MainMenu`, so the release never produced an `Activate`. Two
candidates, one root cause: **the click beat has no OBSERVED precondition.**

| # | Candidate | Mechanism | Log signature |
|-|-|-|-|
| 1 | Press lands before hover resolves | `click_named` warps the cursor and presses in the SAME frame (`crates/nova_autopilot/src/input.rs:157-158`); the picking backend raycasts the new position a system later, so the widget never enters `Pressed` and the release emits no `Activate` | none - just the stall |
| 2 | Button not laid out yet | the preceding beat waits `frames(SETTLE)` = 10 (`examples/ui/menu_newgame.rs:100-102`), not an observed state; `resolve` then warns and returns WITHOUT pressing | ``autopilot: click on `New Game Button` found no laid-out UI node with that Name`` |

**Candidate 2 is ELIMINATED**: the reproduction's log carries no
`found no laid-out UI node` warn (0 occurrences), so the node resolved and the
press WAS issued. Candidate 1 is the confirmed mechanism.

Both are the epic's own anti-pattern - advancing on a frame count instead of
observed state (`tasks/20260802-115955/TASK.md:32`).

`editor.rs` is accidentally immune for the hull card ONLY, because a tooltip
assertion hovers it in an earlier beat (`editor.rs:146-171`). Every other
`.on_enter(click_named(...))` call site carries the race:
`menu_newgame.rs:104`, `editor.rs:107,120,205,227`,
`widget_zoo.rs:721,743,761,769`.

## Steps

1. Reproduce and pin the mechanism rather than trusting the brief - DONE,
   `NOTES.md`. The brief's candidate 1 is disproved; the real cause is the
   hover moving off the widget between the press and release beats.
2. Decide the fix shape - DONE, `DECISION.md`: pin the driven pointer.
3. Land the pin in `nova_autopilot::input`, armed by the autopilot's own
   `build`, with the failing-first guard beside it.
4. Sync the harness wiki.

## Definition of Done

- A driven click survives a foreign cursor event landing between its press and
  release beats. (test: `a_foreign_cursor_event_mid_click_does_not_cancel_the_click`)
- The rig can break the click, so the guard above is not vacuous.
  (test: `a_pointer_the_run_moves_away_does_cancel_the_click`)
- The guard FAILS without the fix, so a green run is evidence of something.
  (manual: delete the `register_pointer_pin` call and rerun - numbers in Notes)
- The pointer-driving examples still pass untouched.
  (cmd: `nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui`)
- The harness wiki states that a driven run owns the pointer.
  (manual: read `web/src/wiki/dev/automation-harness.md`)

## Notes

Fail-first numbers, all on `:99` unless stated:

| What | Without the fix | With it |
| --- | --- | --- |
| `tests/pointer_pin.rs` | 1 of 2 FAILS (`a_foreign_cursor_event_mid_click_does_not_cancel_the_click`) | 2/2 pass |
| `menu_newgame` + `editor` under a faithful stray every 7 frames (temporary rig, reverted) | 2/2 FAIL | 4/4 pass |
| `menu_newgame` alone / x5 concurrent / 6 suite-shaped rounds | 218 runs, 0 failures - the ambient trigger never came out on this box | - |

Out of band, found running the DoD command: `menu_scenarios` is intermittently
KILLED BY A SIGNAL (`exited with None`), 1 run in 5, mid scenario load with no
panic and no stall. A DIFFERENT fault from this one, not introduced by this
fix, and filed as `20260805-111329`.

The stray the rig writes sets `Window::cursor_position` AND both message
halves, which is what `bevy_winit` does for a real one
(`bevy_winit/src/state.rs:292`) and what the landed pin keys off. An earlier
message-only rig (`prototype/`) predates the fix and no longer matches it.

Superseded by the investigation, kept for the record:

- Discriminate the two candidates first: the warn line in candidate 2 is
  present or it is not. A reproduction loop (`menu_newgame` under
  `NOVA_AUTOPILOT=1`, N iterations) is the cheapest way to catch one.
- Shape of a fix is a real decision, not obvious: split every call site into
  hover-then-click beats (mechanical, precedent in `editor.rs`, leaves the
  trap armed for new call sites), or make `click_named` itself defer the press
  a frame (fixes it once, needs machinery in `nova_autopilot` since
  `on_enter` is a single `Fn(&mut World)`), or add a predicate that waits for
  the widget to report hovered/pressed.
- Whatever lands must be provable. The failure is intermittent, so "it passed
  once" is not evidence.
