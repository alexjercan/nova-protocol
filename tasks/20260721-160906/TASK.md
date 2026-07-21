# Harness-prove ally allegiance + orbit-directive combat guards (ch3 mechanisms)

- STATUS: CLOSED
- PRIORITY: 55
- TAGS: v0.8.0,testing,scenario,gameplay

## Story

The spike (tasks/20260721-155249/SPIKE.md) leans on two engine mechanisms
that are supported in source but have never shipped in content: (1) an
AI-controlled ship with `allegiance: Some(Player)` being acquired as a
target by enemy AI (relation-driven `update_ai_target`,
crates/nova_gameplay/src/input/ai.rs:242; override insert at
crates/nova_scenario/src/actions.rs:2527), which Lifeline's convoy defense
needs; and (2) an AI ship holding an `orbit:` directive around a gravity
well while still engaging hostiles that come in range, which Final Tally's
picket needs. Prove both in production-faithful rigs BEFORE the content is
authored; a red rig here flips a documented one-scenario fallback, not the
chapter.

Either verdict closes this task - a falsification is a result
(ledger: production-faithful-rigs, would-it-fail-without-it).

## Steps

- [x] Read the rig style of the existing suites; reuse their scaffolding.
      (Amended in execution: the config-slice style of
      ledger_ch2_encounter.rs cannot run the AI systems; the right homes
      are the system-level rig modules ai.rs already carries, and the
      existing spawn-path test in nova_scenario's actions.rs.)
- [x] Rig 1 (ally acquisition): new `ally_relation_tests` module in
      crates/nova_gameplay/src/input/ai.rs - enemy and Player-allegiance AI
      ships acquire EACH OTHER through the real
      update_ai_target/update_behavior_state pipeline (Idle -> Engage pull
      asserted both sides); Neutral control case acquired by neither; PLUS
      the Lifeline screening premise: the nearest hostile draws the fire
      (raider targets the closer ally hauler over the farther player).
      Spawn-path half: extended nova_scenario's
      `authored_allegiance_overrides_the_controller_default` with the
      Player case.
- [x] Rig 2 (defeat wiring) - amended to source verification: OnDestroyed
      emission is allegiance-agnostic at the source
      (crates/nova_gameplay/src/integrity/explode.rs `on_destroyed_entity`
      fires for ANY id-carrying entity gaining IntegrityDestroyMarker; no
      allegiance in the query). A rig here would re-test bcs Integrity
      machinery already covered by its own suite; the scenario-level Defeat
      wiring gets its pin in Lifeline's harness test (20260721-160957).
- [x] Rig 3 (orbit-directive guard) - amended to citation: ALREADY PINNED
      by shipped tests `combat_interrupts_the_orbit_and_calm_resumes_it`
      and `an_orbit_directive_wins_the_passive_fallback` (ai.rs
      orbit_directive_tests / behavior_state_tests): a hostile inside
      AI_ENGAGE_RANGE pulls an orbiting guard into Engage, calm resumes the
      orbit, and a far-off acquired hostile does NOT abort the orbit. No
      new test would add coverage; writing a duplicate fails
      would-it-fail-without-it.
- [x] Record the verdicts in this file. Verdict written into
      tasks/20260718-152313/TASK.md, tasks/20260721-160957/TASK.md and
      tasks/20260721-161020/TASK.md Notes ("T1 verdict"): PRIMARY variant
      everywhere - ally convoy for Lifeline, orbit-directive picket for
      Final Tally, no fallback needed.

## Definition of Done

- Rig tests exist and run standalone
  (test: `cargo test -p nova_gameplay --lib ally_relation` - the 3 rigs:
  `enemy_and_ally_ai_ships_acquire_each_other`,
  `a_neutral_ai_ship_is_acquired_by_neither_side`,
  `the_nearest_hostile_draws_the_fire`; plus
  `cargo test -p nova_scenario --lib authored_allegiance` for the
  spawn-path Player case).
- Each mechanism has a recorded verdict backed by its rig - green, or red
  with the fallback decision written into the dependent tasks' Notes
  (cmd: `grep -n "T1 verdict" tasks/20260718-152313/TASK.md`).
- No content/RON changes in this task (cmd: `git diff --stat master -- assets/` empty on the branch).

## Notes

- T1 verdict (20260721-160906, 2026-07-21): PRIMARY variant confirmed.
  `allegiance: Some(Player)` on an AI ship survives the spawn path
  (nova_scenario `authored_allegiance_overrides_the_controller_default`,
  Player case) and the relation model treats the ally as a first-class
  combatant both ways, with nearest-hostile fire draw
  (nova_gameplay `ally_relation_tests`: 3 rigs). Orbit-directive picket:
  already pinned by shipped tests - use the orbit directive for Final
  Tally's picket, no fallback needed. OnDestroyed emission is
  allegiance-agnostic at the source, so ally-death Defeat wiring is
  authorable; pin it in the Lifeline harness test.
- Spike: tasks/20260721-155249/SPIKE.md. Flow umbrella: 20260721-160425.
- BCS memory: never run the full workspace test suite locally; run the new
  rigs with `-p <crate> --lib <filter>` only.
- Rig failure here is NOT task failure; it is the cheap moment to learn it.

## Record (2026-07-21)

What changed: 3 new system-level rigs in a new `ally_relation_tests` module
(crates/nova_gameplay/src/input/ai.rs, placed after target_selection_tests)
and one new assert case in nova_scenario's existing spawn-path override
test. No production code changed; no content/RON changed.

Alternatives considered: a full scenario-spawn-to-AI-acquisition end-to-end
rig (spawn via SpawnScenarioObject, run gameplay systems) was rejected -
ship spawning hydrates sections from assets, which the headless test
harness cannot do cheaply, and the two halves (spawn path -> component;
component -> relation behavior) are each already pinned at their own
boundary, meeting the production-faithful-rigs bar without a synthetic
asset stack. The Lifeline harness test (20260721-160957) adds the
config-level end of the chain.

Verification: `cargo test -p nova_gameplay --lib "input::ai::"` 95 green
(whole module suite, shared-system rule); `cargo test -p nova_scenario
--lib "actions::tests"` 31 green; cargo check green; fmt clean. Full
clippy/test suite left to CI per repo policy.

Reflection: the plan's rig list was written before reading the existing
test inventory - two of three rigs were already covered (orbit guard) or
better answered by source verification (event emission). Reading the test
modules FIRST turned a three-rig task into one rig module plus two
citations; the step texts were amended to match reality rather than
padding tests that cannot fail.
