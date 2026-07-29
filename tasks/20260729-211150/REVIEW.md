# REVIEW - Scenarios picker: pin pane widths + indent campaign members

## Round 1 - out-of-context reviewer (2026-07-29)

- VERDICT: REQUEST_CHANGES

### R1.1 major - the indented row overflows the pane and crosses the details divider

`crates/nova_menu/src/lib.rs` (spawn_scenario_row). `list_row()` sets
`width: percent(100)`; a left MARGIN is outside that box, so an indented row is
`100% + 24px` of the pane - it slides right instead of being inset, and its
right border and click target cross the divider (`Scenarios List` scrolls on y
only, so nothing clips it). Measured by the reviewer in the task's own captures:
header right border x~428, member rows x~452, divider x~446.
FIX: also set `width: Val::Auto` on the indented row (the column stretches it to
the pane minus the margin), and assert the width in the test.

### R1.2 minor - `inverse_scale_factor` used in the wrong direction

`examples/ui/menu_scenarios.rs`. `ComputedNode::size` is PHYSICAL; logical px is
`size * inverse_scale_factor`, not `/`. Correct only at scale factor 1.
FIX: multiply, drop the epsilon guard.

### R1.3 minor - the rig's verdict is not enforced

A CHANGED verdict was only an `error!`; the smoke suite greps for reach-Playing
and a clean exit, so the very regression this example exists to catch would pass
CI green. "No measurements" passed too.
FIX: panic on both under `BCS_AUTOPILOT`.

### R1.4 minor - fixed 30 s window vs a frame-counted walk that grows

The walk costs ~21 frames per listed scenario against a wall-clock budget; on a
software-rendered CI GPU, or once more scenarios ship, it can expire mid-walk
and fail as "never reached Playing" - a slow-flake, not a regression.
FIX: self-end (`probe: script complete, exiting` + `HarnessCompletion`), keep
the window as a safety bound, and guard an unfinished exit with a panic.

### R1.5 minor - the layout pin covers only half the invariant

The pinned list pane only stays inside the panel because the DETAILS pane keeps
`flex_grow: 1.0` + `min_width: px(0)`; dropping those left the test green.
FIX: assert the details side too, for both two-pane screens.

### R1.6 nit - CHANGELOG entry carries rationale and a measurement

AGENTS.md asks for one commit-title line.
FIX: trimmed.

### Verified, no finding

- Removing `min_width: px(0)` from the PINNED pane is safe: taffy clamps the
  automatic minimum size by the item's specified size, so a definite
  `width: percent(40)` plus `flex_shrink: 0` bounds it both ways (checked
  against taffy 0.10.1 flexbox.rs:823-825), the mods left pane's nowrap tab row
  included.
- `row.entry::<Node>().and_modify(...)` after the bundle spawn is sound:
  commands apply in order and the `ListRow` reconciler only writes
  `BackgroundColor`/`BorderColor`, never `Node`, so hover/selection repaint does
  not clobber the indent. The indent does not affect the y-only scroll viewport.
- Doc sweep over `web/`, `README.md`, `docs/` (tasks/ excluded as history):
  nothing else describes the picker layout or the example catalog. The
  `widget_zoo` -> `NOT_SMOKED` addition is correctly justified.

## Round 2 - out-of-context reviewer (2026-07-29)

R1.1-R1.6 all verified fixed (the reviewer re-derived the stretch mechanism for
R1.1 through bevy_ui's `AlignItems::Default -> taffy None -> Stretch`, and
eyeballed a fresh capture: member rows inset at x~122, right border x~429 level
with the header's, divider at x~446). Three new findings.

- VERDICT: REQUEST_CHANGES

### R2.1 minor - `guard_run_completion` never fires

`examples/ui/menu_scenarios.rs`. The guard reads `AppExit` in `Last`, but the
exit it must catch is written by `completion_watch`, also in `Last` and ordered
after it - so the message is never observed. Broadside's copy works only because
it adds `.self_completing()`, which makes the autopilot write `AppExit::error`
from `PreUpdate`. Without it a timeline expiry reports "cycle complete" over an
unfinished walk. Reproduced by the reviewer with `BCS_HARNESS_DEADLINE=4`: 6 of
13 rows measured, no panic.
FIX: add `.self_completing()`.

### R2.2 minor - the guard is registered outside the `BCS_AUTOPILOT` gate

An ordinary interactive run would panic on window close (`AppExit` from
`PostUpdate`, walk never finished).
FIX: register it inside the env-gated block, as broadside does.

### R2.3 nit - the 180 s "safety bound" is unreachable

The harness completion deadline defaults to 120 s, so the watcher error-exits
first and the runway never expires as documented.
FIX: size the runway under the deadline.

### R2.4 nit (not taken) - the test pins Node values, not geometry

A post-layout assertion on the row's computed right edge would pin R1.1's actual
invariant. Not taken: computed widths in a headless rig need real text measure
to be meaningful for this screen, which is precisely why the example rig exists;
the rig captures the geometry and it was eyeballed. Recorded rather than done.

## Round 3 (2026-07-29)

R2.1-R2.3 applied: `.self_completing()` added, the guard moved inside the
`BCS_AUTOPILOT` block, the runway cut to 100 s with the deadline relationship
documented at the constant.

The guard is now PROVEN to fire, not just reasoned: temporarily cutting the
runway to 5 s ended the run at exit 101 with
`menu_scenarios: run ended with the walk unfinished (9 of the picker's rows
measured, launched=false)`; restored to 100 s the normal run reports HELD and
`probe: script complete, exiting`.

Re-verified: `cargo test -p nova_menu` 75 passed; `cargo check --all-targets
--features debug` green; `cargo test --test examples_smoke ui_` green (44.8 s).

The reviewer re-derived the success path through the bcs autopilot (with
`self_completing`, `autopilot_drive` early-returns once `AUTOPILOT` is no longer
pending, so the runway-expiry error branch is unreachable after the script
reports done - the clean exit is unchanged), ran the example 8 times, and
confirmed REVIEW.md/TASK.md do not overstate the evidence.

One residual, recorded not fixed: on THIS box the run intermittently SIGSEGVs at
process teardown (2 of 8 runs, exit 139) inside
`wgpu_hal::vulkan::CommandEncoder::drop` -> `libnvidia-glcore`, AFTER
`harness completion: all collectors done, exiting`. A local nvidia+Xvfb driver
teardown race, not branch logic (the commit touches no rendering), and CI runs
lavapipe where that library is not involved. Noted so a future session does not
chase it as a regression.

- VERDICT: APPROVE
