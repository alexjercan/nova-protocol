# Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- PRIORITY: 83
- TAGS: v0.10.0, content, examples, testing, ui
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-003301

## Story

Rebuild `ui/` so the runs DRIVE the interface with synthesized pointer input
instead of asserting around it, and check the live tree afterwards so nothing
ghosts or duplicates on a state change.

Pointer synthesis (`click_at`, `move_cursor`, `press_mouse`) landed with the
predicate autopilot (`20260802-120025`); without it the `ui/` contract was
assertion-only.

## Steps

- [ ] Add `GameStates` to `widget_zoo`'s own `App` (owner call 2026-08-04,
      option A) so `AutopilotPlugin<GameStates>` applies to it, then remove it
      from `NOT_SMOKED` (`tests/examples_smoke.rs:74-78,82`) and add it to `UI`.
- [ ] Drive `widget_zoo` with synthesized pointer input: hover, press, reskin,
      segmented select, checkbox/toggle, slider drag.
- [ ] Deepen `editor` into a real build-and-inspect sequence.
- [ ] Narrow `menu_newgame` to assert reaching gameplay state only.
- [ ] Deepen `menu_scenarios`: pointer-driven picker navigation plus the
      pane-width verdict.
- [ ] Add the RTT element test that inherits `nova_os_rtt_poc`'s coverage,
      alongside the other widget tests - this is new code, and it is this
      task's, not the retire task's.
- [ ] Cover opening the NOVA OS computer and exercising the RTT screen, or
      record explicitly why that coverage lands elsewhere.

## Definition of Done

- The `ui/` fleet drives real widgets with synthesized pointer input, asserts
  the live tree, and completes headlessly.
  (cmd: `nix develop --command cargo run -p nova_probe -- run ui`)
- The RTT pipeline keeps element-level coverage after the POC retires.
  (test: `rtt_element_renders_its_subtree`)
- `examples/ui/` holds only runnable examples.
  (cmd: `! ls examples/ui/*.html`)

## Notes

Roster per the spike (`20260804-003244`) - five runs, `nova_os_rtt_poc` retired
by `20260804-093910`:

| Run | Change |
| --- | --- |
| `widget_zoo` | Drive it. Already functional; gains real pointer input (hover, press, reskin, segmented select, checkbox/toggle, slider drag) and live-tree assertions. |
| `hud_range` | KEEP. Already predicate-driven; screen-projected indicators. |
| `editor` | Deepen. Today one editor action; needs a real build-and-inspect sequence. |
| `menu_newgame` | Narrow. Assert only that gameplay state is reached - NOT `shakedown_run`'s content, which is story. |
| `menu_scenarios` | Deepen. Picker navigation plus the pane-width verdict, driven by pointer. |

- `widget_zoo` sits in `NOT_SMOKED` (`tests/examples_smoke.rs:75-78`) because it
  runs its own `App` (`widget_zoo.rs:32-62`: `App::new()` + `DefaultPlugins` +
  `nova_ui::widget::register`, no `AppBuilder`) with NO `GameStates` at all.
  RESOLVED 2026-08-04, owner call: add `GameStates` to the zoo (option A).
  Simple and local. The rejected option B - teaching `nova_autopilot` to drive
  a stateless app - is the more general fix, but nothing else needs it and it
  would change the crate the whole sprint depends on. If a second stateless app
  ever needs driving, B becomes worth its own task.
- Adding `GameStates` means the zoo joins the CI smoke run: 533 lines of
  interactive app asserted to reach Playing and exit clean, on every `cargo
  test` with a display. Expect to find things; budget for it.
- `widget_zoo`'s `NOVA_ZOO_CAPTURE=1` two-skin capture path is a screenshot
  producer living in a `ui/` example. Per the category contract that is a
  `screenshots/` job, and `20260804-093855`'s contract test may reject it.
  Not in this task's Steps - flag it if the contract test fires.
- The `*_poc.html` relocation is owned by epic child `20260804-003301`, not by
  the retire task. This task depends on it for its "only runnable examples"
  end-state.
- `menu_newgame` boots `shakedown_run`, a story scenario. That is NOT story
  coupling as long as it asserts reaching gameplay state and nothing about
  scenario internals. If that assertion ever grows into scenario content, it
  has drifted - the run proves the boot flow and menu teardown, nothing else.
- Pointer input is right HERE and only here. Owner call 2026-08-04, the split:
  `ui/` drives with real synthesized pointer input because the interface IS its
  subject - reachability, hover, press, hit-testing are the thing under test.
  `systems/outcomes` triggers `Activate` directly instead, because its subject
  is the outcome chain and pixel coordinates would only add layout coupling.
  Two idioms, each coupled to what it actually tests.
- The residual risk this leaves, stated plainly: `ui/` runs are now coupled to
  LAYOUT the way the retired story runs were coupled to CONTENT. Moving a panel
  can break several runs at once. That is accepted - it is the cost of testing
  a UI as a UI - but resolve click targets by `Name` rather than by literal
  coordinates wherever possible, so a move is survivable and only a rename
  breaks a run.
- Live-tree assertions are the point: `cargo check` misses duplicate-component
  panics and TextShadow ghosting. Examples must be RUN under Xvfb :99.
- `ui/` carries no fps window.
- `examples/ui/` must hold only runnable examples once `20260804-003301`
  relocates the `*_poc.html` sources.
