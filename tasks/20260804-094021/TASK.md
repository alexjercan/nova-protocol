# Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- PRIORITY: 77
- TAGS: v0.10.0, content, examples, testing, ui
- KIND: STORY
- ACTIVITY: -
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

- [ ] Drive `widget_zoo` with synthesized pointer input: hover, press, reskin,
      segmented select, checkbox/toggle, slider drag. Resolve its
      `NOT_SMOKED` status first (see Notes).
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
  runs its own `App` with NO `GameStates` at all. "Drive it" is therefore not a
  free add-on: it needs either `GameStates` added to the zoo, or an autopilot
  that can drive a stateless app. Pick one and record why in the task's
  close-out.
- The `*_poc.html` relocation is owned by epic child `20260804-003301`, not by
  the retire task. This task depends on it for its "only runnable examples"
  end-state.
- `menu_newgame` boots `shakedown_run`, a story scenario. That is NOT story
  coupling as long as it asserts reaching gameplay state and nothing about
  scenario internals. If that assertion ever grows into scenario content, it
  has drifted - the run proves the boot flow and menu teardown, nothing else.
- Live-tree assertions are the point: `cargo check` misses duplicate-component
  panics and TextShadow ghosting. Examples must be RUN under Xvfb :99.
- `ui/` carries no fps window.
- `examples/ui/` must hold only runnable examples once `20260804-003301`
  relocates the `*_poc.html` sources.
