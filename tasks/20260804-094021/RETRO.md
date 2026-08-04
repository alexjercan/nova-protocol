# Retro: Rebuild ui/ to drive real widgets with pointer input and assert the live tree

- TASK: 20260804-094021
- BRANCH: feature/ui-pointer-driven
- REVIEW ROUNDS: 4

## What went well

- Every claim that mattered was settled by SABOTAGE, not by reading: delete the
  guard, re-run, see red. R2.4 (the duplicate-name warn), R2.3 (the coverage
  floor) and R3.1 (the physical-to-logical size conversion) were each caught
  that way, and two of them were tests that looked like coverage and were
  satisfied by the code with the mechanism removed.
- The record names the environment its numbers came from after round 2 flagged
  the mismatch (13 scenarios direct-run vs the six-row probe environment).
- A pre-existing flake found mid-branch was filed as its own task
  (`20260804-174231`, non-zero exit on an otherwise clean `menu_scenarios`
  run) with a measured rate - 1 in 7 - and an explicit non-attribution, rather
  than absorbed into this diff or silently reruns-until-green.

## What went wrong

- Four rounds, and three of them found the same defect shape: a guard that
  cannot fail. R1.1's measurement could not fail when a click missed, R2.4's
  test could not fail with its warn deleted, R3.1's test could not fail with
  its conversion deleted. The decision that produced all three seemed sound at
  the time - "assert the observable, warn on the impossible" is the right
  default for an example rig - but a `warn!` path is unfalsifiable unless a
  test captures the log, and a scale conversion is unfalsifiable at scale 1.
- Each hardening fix opened a SKIP path and the skip path then needed its own
  guard in the NEXT round: R1.1's assert opened a skip, R2.1 gave it a settle
  budget, R2.3 gave it a coverage floor, R3.2 made the drop path reach that
  floor, and R4.1 now observes that the R3.2 clear removed the measurement's
  only retry. That is one property discovered in five instalments.
- A checkpoint commit inherited from a prior context did not build
  (`editor.rs`, unbalanced `.add())` plus three unresolvable names). The
  checkpoint rule exists precisely to prevent that.
- Two Step clauses were planned against an editor that does not exist - no
  selection surface, `SectionChoice` is `pub(crate)`. They were corrected
  against the code rather than faked, but the plan had asserted an internal API
  an example cannot name.

## What to improve next time

- Breadth: the diff is large (~2.5k lines across five examples, the autopilot
  crate and the smoke suite) because the Story is genuinely one contract - the
  shared pointer vocabulary is what makes all five runs possible, and landing
  the vocabulary without a caller proves nothing. The one real split available
  was `menu_scenarios`, which absorbed three of the four rounds on a property
  (coverage under a fold) the other four runs do not have. Splitting the picker
  walk out would have let the other four land after round 1.
- Churn: the plan-time question that would have prevented most of the rework is
  `plan`'s from-scratch challenge applied to the EVIDENCE, not the code - "for
  each assertion this Step adds, what would I delete to make it fail?" A Step
  whose answer is "nothing, it only warns" needs a log-capture test or a panic
  in the same Step, and that covers R1.1, R2.3, R2.4 and R3.1 at plan time.
- Also worth a planning check: a Step's assertion target must be reachable from
  where the Step runs. Two clauses named `pub(crate)` items an example cannot
  see.
- Context: no compaction warning or threshold crossing is recorded. One
  observed pressure point is real - a checkpoint handed over a non-building
  tree, so the next context started by repairing rather than working. Verify
  the build before the checkpoint commit, not after the handoff.

## Action items

- Seeded `20260804-190142`: add wheel synthesis to `nova_autopilot::input` so
  the scenarios walk reaches every row. The permanent 5-of-6 coverage is a
  HARNESS gap - the picker DOES scroll (`scroll_menu_lists`,
  `crates/nova_ui/src/widgets.rs:72`) - not a property of the UI.
- R4.1 (MINOR) and R4.2 (NIT) are open on this branch by verdict: the
  measurement lost its retry budget when R3.2 cleared `pending_measure` on the
  first miss, and the `warn!` still says "dropping that measurement" when the
  row is now recorded as skipped. Both are in
  `examples/ui/menu_scenarios.rs:288-301`.
- `20260804-174231` stays open: `menu_scenarios` exits non-zero on an otherwise
  clean run, ~1 in 7, reproduced on master.

## Landing message

```
feat(examples): drive ui/ with real pointer input and assert the live tree

Rebuild the five ui/ runs to DRIVE the interface with synthesized pointer
input instead of asserting around it, and check the live tree after each
state change so nothing ghosts or duplicates.

Adds a Name-resolved pointer vocabulary to nova_autopilot (ui_node_centre,
ui_node_rect, click_named, hover_named), promotes the smoke sentinel to
nova_debug::harness::REACHED_PLAYING, and joins widget_zoo to the harness
fleet. widget_zoo now hovers, presses, reskins, toggles and drags a slider,
then asserts exactly one ZooBody, one entity per driven Name and no
TextShadow under the root. editor builds and inspects a ship through the
real pointer; menu_newgame narrows to the boot flow and drops
NOVA_MENU_PATH; menu_scenarios walks the picker and states its coverage on
the passing path.
```
