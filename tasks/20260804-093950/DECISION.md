# Decision: reload gate, hull rig shape, and how the invariant roster is proven

- DATE: 20260804-093950
- STATUS: ACCEPTED
- TASK: 20260804-093950
- TAGS: examples, testing, harness

## Context

Deepening the five `sections/` runs to multi-round, multi-scene predicate
scripts and absorbing `com_range` and `torpedo_guidance` forces four choices
before implementation: how a script observes that a rig RELOADED, what shape
`hull_section`'s rig has to take to carry the merged mass-properties and
camera invariants, how "every listed invariant is asserted" becomes a
checkable proof, and where the shared-builder line sits.

## Decision

1. **Reload is a two-step gone-then-present gate, using existing vocabulary
   only.**

   ```rust
   .step("tear the rig down")
   .on_enter(reload_rig)
   .until(nova_autopilot::predicate::not(any_entity::<With<SpaceshipRootMarker>>()))
   .deadline(10.0)
   .add()
   .step("wait for the fresh rig")
   .until(any_entity::<With<SpaceshipRootMarker>>())
   .deadline(30.0)
   .add()
   ```

   The gap is real, which is what makes the pair sound: `on_load_scenario`
   tears the old scenario down (despawning every `ScenarioScopedMarker`
   entity) and the ship respawns later off `OnStartEvent`
   (`crates/nova_scenario/src/loader/lifecycle.rs:161`, `:252`). A single
   `until(any_entity::<...>())` after the trigger would pass on the OLD ship.
   `not` must be qualified - it clashes with `bevy::prelude::not`.

2. **`hull_section`'s rig becomes player-controlled and five sections long.**
   Absorbing `com_range` brings the chase-camera-anchor invariant
   (`com_range.rs:401-412`), and the chase camera only follows a player ship;
   today `hull_section` spawns `SpaceshipController::None` with three sections
   (`hull_section.rs:76`). The rig adopts `com_range`'s shape - five sections
   in a line under `SpaceshipController::Player` with an empty `input_mapping`
   (`com_range.rs:135-148`), so no input is synthesized and the ship still
   only moves when the script damages it.

   Consequence: `com_range`'s `local_com.z > 2.4` does NOT port. The merged run
   destroys a rear hull first (invariants 1-3 need a live controller), so the
   surviving set at COM-assert time differs. Recompute the aft threshold from
   the surviving sections rather than copying the number.

3. **The invariant roster is pinned by a source-grep test.** Each invariant
   carries one `nova_probe::probe_marker` named `outcome: <slug>` beside its
   `assert!`, and a new display-free `sections_assert_their_invariant_roster`
   in `tests/examples_smoke.rs` pins those names and the count of 27. Same
   class and same file as `examples_name_drivers_through_the_nova_harness`
   (`tests/examples_smoke.rs:217`), which already gates a failure `cargo check`
   cannot see. The existing `outcome:` prefix is reused rather than inventing
   an `invariant:` one: no tooling keys on the prefix, and one vocabulary
   across `sections/`, `systems/` and `ui/` beats a second one.

4. **Ship builders stay local; the reload gate is not a "builder".** Each run
   builds its own `ScenarioConfig` (owner call 2026-08-04);
   `20260804-094006` extracts the shared builder as the third caller. Point 1
   is explicitly outside that call - it is two lines of existing predicate
   vocabulary per run, not a helper.

## Alternatives considered

- **Stateful identity reload predicate** (capture the old root `Entity`, wait
  for a different one). Correct, but needs a per-run resource plus a new
  `nova_debug::harness` predicate for five callers. The gone-then-present pair
  is two script lines and no API.
- **`outcomes.rs`'s reload gate**
  (`scenario_variable_is("player_down", 0.0)`, `systems/outcomes.rs:84-87`).
  Works there because the value CHANGES to 1 between loads. The section rigs
  have no such latch, so a re-seeded constant proves nothing.
- **Keep `hull_section`'s controller-less three-section rig** and assert the
  camera invariant elsewhere. Rejected: it would keep a `com_range` remnant
  alive, which is exactly what the merge exists to remove.
- **Reviewer-reads-the-diff as the only roster proof.** Kept as the `manual:`
  DoD item for "no beat waits on the clock", which is a judgment call. A
  27-item roster is mechanical, and without a gate deleting one invariant
  leaves the run green.

## Consequences

- All five runs move off the `nova_autopilot()` wall-clock preset onto explicit
  step lists with per-step deadlines; a stall then names its beat instead of
  reporting a generic timeout. Deadline sums stay well under
  `DEFAULT_DEADLINE_SECS` (120s).
- `hull_section` gets materially bigger (it inherits `com_range`'s gizmos,
  hotkeys and status log along with the beats). That is the merge, not scope
  creep.
- The roster test is a new maintenance surface: adding or renaming an
  invariant means editing the roster. That is the intent - it is the task's
  stopping rule made executable.
- `torpedo_section` invariant 5 is NEW work, not a port: `torpedo_guidance`
  only LOGS closest approach (`torpedo_guidance.rs:234-245`) and asserts
  nothing. Roster totals become 14 existing / 4 merged / 9 new = 27.
