# Base campaign polish + extension: make Shakedown to Broadside longer and more interesting (more beats/acts, pacing, encounters)

- STATUS: OPEN
- PRIORITY: 49
- TAGS: v0.8.0,content,scenario,playtest

## Story

As a player starting New Game, I want the base campaign to carry me through a
fuller arc - more encounters, clearer stakes, story between the fights - so
that finishing it feels like completing a short campaign rather than sampling
two scenarios.

Direction settled by spike tasks/20260721-155249/SPIKE.md (2026-07-21):
append Chapter 3 (convoy-defense "Lifeline" + gravity-well finale "Final
Tally") and give the existing chain its first StoryMessage voice pass. The
build was split at plan time (flow umbrella 20260721-160425) into:

- 20260721-160842 (p56) asteroid_field hidden-vs-wiki resolution
- 20260721-160906 (p55) ch3 mechanisms rig (ally allegiance, orbit picket)
- 20260721-160929 (p54) base chain voice pass
- 20260721-160957 (p53) Lifeline (ch3a) + gunship chain hook
- 20260721-161020 (p52) Final Tally (ch3b) + campaign ending

THIS task is the campaign-level close-out: the end-to-end verification that
the sum delivers the original DoD, the playtest question list for the owner,
and the release-post note. It lands LAST.

## Steps

- [ ] Verify the extended chain end to end from the evidence the sub-tasks
      left (harness chain-wiring tests + probe reports): New Game runs
      shakedown_run -> broadside -> broadside_gunship -> lifeline ->
      final_tally with lingering checkpoints at each seam.
- [ ] Record the encounter variety matrix in this file (per-fight comp +
      shape) and check the original DoD line "no two consecutive encounters
      share composition and shape" against it.
- [ ] Run the full `content lint` on the final tree; confirm every balance
      ack carries a reason.
- [ ] List the playtest questions for the owner in this file (difficulty
      per fight, relief timer, names/tone nod, picker policy) - decided by
      the owner, not silently.
- [ ] Write the v0.8.0 news-post note line (what the release post should
      say about the campaign) into this file; confirm CHANGELOG coherence
      across the landed sub-tasks (one voice, no duplicate entries).

## Definition of Done

- The base chain is at least one full scenario/act longer than v0.7.0's
  (it grows by two), with no two consecutive encounters sharing the same
  composition and shape (matrix recorded here; cmd:
  `grep -n "Variety matrix" tasks/20260718-152313/TASK.md`).
- Every scenario in the chain has an Outcome path for both win and lose, a
  checkpoint structure that never replays more than one fight on death, and
  comms beats following the beat-sheet convention
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- scenarios.md and CHANGELOG reflect the new chain
  (cmd: `grep -n "Lifeline\|Final Tally" web/src/wiki/scenarios.md CHANGELOG.md`).
- Playtest questions for the owner are listed in this task, not silently
  decided (cmd: `grep -n "Playtest questions" tasks/20260718-152313/TASK.md`).

## Notes

- T1 verdict (20260721-160906, 2026-07-21): PRIMARY variant confirmed.
  `allegiance: Some(Player)` on an AI ship survives the spawn path
  (nova_scenario `authored_allegiance_overrides_the_controller_default`,
  Player case) and the relation model treats the ally as a first-class
  combatant both ways, with nearest-hostile fire draw
  (nova_gameplay `ally_relation_tests`: 3 rigs). Orbit-directive picket:
  already pinned by shipped tests (`combat_interrupts_the_orbit_and_calm_
  resumes_it`, `an_orbit_directive_wins_the_passive_fallback` in ai.rs) -
  use the orbit directive for Final Tally's picket, no fallback needed.
  OnDestroyed emission is allegiance-agnostic at the source
  (integrity/explode.rs `on_destroyed_entity` fires for any id-carrying
  entity), so ally-death Defeat wiring is authorable; pin it in the
  Lifeline harness test.
- Spike: tasks/20260721-155249/SPIKE.md (2026-07-21, RECOMMENDED) - the
  extended-arc beat sheet this task's original step 2 asked a nod on. Key
  finding: AI targeting is relation-driven (ai.rs update_ai_target) and
  `allegiance: Some(Player)` is authorable; unshipped, so the rig task
  decides Lifeline's primary vs fallback variant.
- Depends on: 20260721-160842, 20260721-160906, 20260721-160929,
  20260721-160957, 20260721-161020 (all must be CLOSED).
- Base scenarios: `assets/base/scenarios/*.content.ron` are GENERATED - edit
  the `nova_assets` builders and run `content gen`, never the .ron directly
  (LESSONS.md: edit-the-builder-not-the-generated-ron).
- Feel/balance is ultimately the user's call; deliver the content + a first
  tuning pass, flag playtest questions.
- Menu ambience scenes are separate and out of scope here.
