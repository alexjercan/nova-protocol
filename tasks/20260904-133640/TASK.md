# Quiet the combat log spam and the PD controller root errors

- STATUS: CLOSED
- PRIORITY: 65
- TAGS: v0.13.0, logging, physics

## Problem

Running `first_shift_07_attack_approach` and `first_shift_08_attack_salvo` with
`--features debug` floods stdout. `--features debug` sets `nova=debug` by
default (`nova_core::log_filter_str`), so every `debug!` on a per-collision or
per-section path prints in every dev run.

Three offenders, all on the destruction path:

- `on_impact_collision_deal_damage` - one line per contact pair. Measured worst
  frame in a salvo: 1967 lines.
- `on_destroyed_entity` / `detach_destroyed_body` / `on_section_disable` -
  three to four lines per section, and one collapse peels 812 sections.
- `update_controller_root_torque` - `error!` at the fixed rate for the rest of
  the scene: `root entity 1846v0 not found in q_root`.

## The dangling PD target

`insert_controller_section_target` is an `On<Add, ControllerSectionMarker>`
observer: it reads `ChildOf` once, at spawn.
`sever_disconnected_structures` reparents a split hull's sections onto a fresh
`ShipWreckFragmentMarker` root and never re-pointed `PDControllerTarget`. The
severed controller went on targeting the hull it left; once that hull was
despawned the PD pass errored every fixed tick for the life of the scene.

A controller only moves when its component LOSES the sever ranking, which
happens two ways on shipped content: a hull with several flight computers
splits so one component holds fewer live ones, or a wreck fragment
(`is_spaceship` false, so controllers do not count at all) splits again.

Nothing applied the wrong torque today - the sever also inserts
`SectionInactiveMarker`, and `sync_controller_section_forces` is gated on its
absence - but the target was wrong, and it was the error's source.

## Fix

- `sync_controller_section_target` (new, `ControllerSectionSystems::SyncTarget`,
  first in the FixedUpdate helm chain) re-points a controller section whose
  `ChildOf` changed.
- `update_controller_root_torque` treats a target that is not a body as the
  normal teardown state it is: zero the output, `trace!`, no error. Zeroing
  matters because `PDControllerOutput` is the only value the force sync
  applies.
- The per-contact and per-section lines moved to `trace!`. Two frame tallies in
  `IntegrityCorePlugin` (`ImpactTally`, `DestructionTally`) emit one `debug!`
  summary each per frame instead. A ship DEATH keeps its own `debug!` line.

## Proof

Tests (`cargo test -p nova_ship --lib`, 76 in the three affected modules, all
pass; `cargo test -p nova_gameplay --lib integrity`, 69 pass):

- `a_split_ship_re_points_the_helm_that_left` drives the real
  `sever_disconnected_structures` with two live helms against one. Verified to
  FAIL without the fix: the severed helm reported `Some(166v0)` (the hull it
  left) where the wreck is `171v0`.
- `a_severed_controller_section_targets_the_hull_it_left_with` covers the
  re-point system directly, including that a preview controller still gains no
  target.
- `a_controller_whose_body_is_gone_holds_no_torque` covers the zeroing.

Live runs, `first_shift_08_attack_salvo --features debug` under Xvfb, 90 s,
same scene both times (the ship dies and severs in both):

| | lines | `on_impact_collision` | destruction lines | ERROR |
|---|---|---|---|---|
| before | 13095 | 2766 | 2436 | 0 |
| after | 763 | 0 | 0 | 0 |

The worst single frame went from 1967 impact lines to one:
`impacts: 1967 contacts for 2751.37 damage (worst 28.86 on Some(9010v1))`, and
a collapse frame from ~2400 lines to
`integrity: destroyed 812 nodes (basic_controller_section x1, reinforced_hull_section x811)`.

`first_shift_07_attack_approach --features debug`, 90 s: 370 lines, no errors.

## Not measured

Neither run reproduced the `q_root` error at HEAD - the autopilot exits before
the wrecked hull is despawned, and the interactive session that produced the
reported log had the player's own ship come apart. The mechanism is proven by
the failing-without-the-fix test instead.

The railgun lag spike is UNJUDGED. Whether the log volume was causing it is
the user's call after playing this build; if it is not, the perf investigation
continues from here.
