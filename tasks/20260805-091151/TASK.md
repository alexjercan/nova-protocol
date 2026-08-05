# click_named presses in the same frame it warps the cursor

- PRIORITY: 84
- TAGS: v0.10.0,testing,automation
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

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

## Notes

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
