# Reset scenario progress (salvaged crates) on scenario start/exit

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.5.0, bug, scenario

## Closed not-reproducible (2026-07-12)

User could not reproduce it on retry. The read-only trace below falsified every
concrete mechanism (the crate tally resets on replay via `NovaEventWorld::clear`
+ the scenario's `OnStart`; no false completion cue on the tally rebuild; pickup
is edge-triggered; completion gate is exact `== 3`). The most likely original
observation was the clustered crates swept in one pass. No code change; the
reset path is already guarded by the teardown tests (e.g.
`teardown_clears_hint_emphasis`). Reopen with a fresh-vs-replay repro if it
recurs - the trace notes where an integration reload test would go.

## Diagnosis (2026-07-12) - reported mechanism NOT found; likely perceptual

Read-only trace of the salvage/reset machinery (user flagged uncertainty:
"maybe I didn't see it right ... the objective completed itself and I only
collected 1"). Every concrete mechanism checks out as correct:

- Persistence across replay - FALSIFIED. `NovaEventWorld::clear()` clears ALL
  variables (nova_scenario/src/world.rs:101) and is called by
  `teardown_scenario_entities` on unload/reload; the shakedown scenario's
  `OnStart` re-sets `crates_recovered = 0` (nova_assets/src/scenario/
  shakedown.rs:458) on every load. So the crate tally resets on replay.
- Objective/beat persistence - FALSIFIED. Objectives live in the same
  event-world (cleared) and are re-posted by OnStart; the beat is `VAR_BEAT`
  (a variable, cleared). Write-on-diff sync rebuilds GameObjectives.
- False "completed" cue on the 1/3, 2/3 tally rebuild - FALSIFIED. The tally
  handlers do `complete(OBJ_B3)` + re-add for a flicker-free panel line, but
  `objective_change_feedback` diffs completions BY ID (hud/objective_feedback.rs
  :151-154); the re-add keeps id `b3_salvage`, so no completion chime/green.
- Per-frame double-count on pickup - FALSIFIED. OnEnter fires on `CollisionStart`
  (nova_scenario/src/objects/area.rs:60), an EDGE event - one crate = one
  `add_one(VAR_CRATES)`. Completion gate is exact `crates == 3`
  (shakedown.rs:576).

Most likely explanation (matches the user's caveat): the 3 crates sit in a
tight cluster - `CRATE_POSITIONS` span ~29-37u apart around `DEBRIS_CENTER`
with `CRATE_AREA_RADIUS = 8` - so a fast pass can sweep 2-3 pickup volumes while
the player only registers one, and the beat completes "after 1".

## Open / next

- NEEDS A LIVE REPRO to go further. Ask: does it happen on a FRESH first run,
  or only after replaying? What exactly was seen - the completion chime, the
  green "done" ghost, or the objective text jumping to 3/3? If it only happens
  after a replay, the trace above is wrong somewhere and this becomes a real
  integration bug (add a reload regression test through the real
  LoadScenario/UnloadScenario observers, like `teardown_clears_hint_emphasis`).
- If it is the cluster-sweep perception: NOT a reset bug. Options are a UX
  tweak (space the crates further so each is a deliberate pickup, or a distinct
  per-crate pickup cue) - a separate polish task, not this one.

## Original report / where to look

## Goal

Playtest bug (2026-07-12): exiting the main scenario to the main menu and
playing again keeps the previous run's progress - the salvaged crate tally (and
likely other scenario variables / beat state) persists instead of resetting.
Starting (or exiting) a scenario should reset its progress so a replay begins
fresh.

## Notes / where to look

- The salvage tally lives in the scenario event world's variables
  (`NovaEventWorld`, bevy_common_systems event system - see the shakedown
  scenario's crate count and the OnUpdate/variable machinery referenced in
  `nova_scenario/src/loader.rs`). `teardown_scenario_entities` already calls
  `world.clear()` on unload/reload and clears `HintEmphasis`; confirm whether
  `world.clear()` actually resets scenario VARIABLES (the crate tally) or only
  entities/handlers - the bug suggests the variables (or `GameObjectives`
  progress) survive.
- Also check `GameObjectives` (bcs ObjectivesPlugin) and any beat/progress
  resource: they may need an explicit reset on `LoadScenario` / `UnloadScenario`
  the same way `HintEmphasis` is cleared (state-reset class, task 20260712-125342
  / the emphasis reset in loader.rs).
- Repro: New Game -> salvage some crates -> back to main menu -> New Game again
  -> observe the tally starts non-zero / the salvage beat is already partly done.

## Steps (to be planned)

- [ ] Reproduce and identify exactly which state persists (event-world variable,
      GameObjectives, or a beat resource).
- [ ] Reset it on scenario load (and/or unload), alongside the existing
      `world.clear()` + emphasis clear in `teardown_scenario_entities`.
- [ ] Regression test: load a scenario, mutate the progress variable, reload,
      assert it is back to zero (driven through the real LoadScenario observer,
      like the `teardown_clears_hint_emphasis` test).
