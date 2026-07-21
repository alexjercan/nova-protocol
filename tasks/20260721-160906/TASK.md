# Harness-prove ally allegiance + orbit-directive combat guards (ch3 mechanisms)

- STATUS: OPEN
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

- [ ] Read the rig style of crates/nova_assets/tests/ledger_ch2_encounter.rs
      and tests/gauntlet_course.rs; reuse their App-driven scaffolding.
- [ ] Rig 1 (ally acquisition): spawn an enemy AI ship and an AI ship with
      `allegiance: Some(Player)` in detection range; assert the enemy's
      AITarget acquires the ally (and the ally's acquires the enemy);
      control case: a `Some(Neutral)` ship is NOT acquired.
- [ ] Rig 2 (defeat wiring): destroy the ally ship in the rig; assert the
      scenario OnDestroyed event fires for its id (Lifeline's lose path).
- [ ] Rig 3 (orbit-directive guard): AI ship with `orbit:` around a
      surface_gravity asteroid; hostile enters range; assert the guard
      engages (behavior state/target), and note what it does when the
      hostile dies (returns to orbit or drifts - either is fine, record it).
- [ ] Record the verdicts in this file. Write the variant decision into
      tasks/20260718-152313/TASK.md Notes and the Lifeline/Final Tally task
      Notes: primary (ally convoy / orbit picket) or fallback
      (salvage-under-fire / patrol-ring picket).

## Definition of Done

- Rig tests exist and run standalone
  (test: `cargo test -p nova_assets --test ch3_mechanisms` or the crate the
  rigs land in; exact names recorded here when written).
- Each mechanism has a recorded verdict backed by its rig - green, or red
  with the fallback decision written into the dependent tasks' Notes
  (cmd: `grep -n "T1 verdict" tasks/20260718-152313/TASK.md`).
- No content/RON changes in this task (cmd: `git diff --stat master -- assets/` empty on the branch).

## Notes

- Spike: tasks/20260721-155249/SPIKE.md. Flow umbrella: 20260721-160425.
- BCS memory: never run the full workspace test suite locally; run the new
  rigs with `-p <crate> --test <name>` only.
- Rig failure here is NOT task failure; it is the cheap moment to learn it.
