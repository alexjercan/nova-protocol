# Final Tally (ch3b): gravity-well anchorage finale + campaign ending

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.8.0,content,scenario

## Story

Chapter 3 part two per the spike (tasks/20260721-155249/SPIKE.md): "Final
Tally" - the finale at the gang's claim, a wrecked-megahauler anchorage
inside the base chain's first combat gravity well. Orbital picket, then the
flagship with an escort - the campaign's only simultaneous capital+escort
fight - and a proper campaign-complete ending instead of a dead-end
omission. Hidden continuation reached from lifeline's victory.

Picket mechanism is variant-dependent: orbit-directive guards if the
mechanisms rig (20260721-160906) proved them; patrol-ring fallback
otherwise.

## Steps

- [ ] Confirm the rig verdict for the orbit-directive picket (Notes).
- [ ] New builder crates/nova_assets/src/scenario/final_tally.rs registered
      in the base bundle: planetoid with `surface_gravity`, Ring-region
      asteroid belt scatter, two big invulnerable wreck-rocks as the
      anchorage (Ledger Ceres-Matron pattern); layout derived from measured
      gravity constants (LESSONS: authored-vs-derived-values - SOI vs
      spawn/beacon positions asserted in the rig, not eyeballed).
- [ ] Arrival beats: coast into the SOI (tutorial gravity-coast callback),
      announce line; survey beat - OnTravelLock the anchorage wreck to
      confirm the claim (gate the recurring lock event on a one-shot flag).
- [ ] Fight 1: two light racers as the picket (orbit directive around the
      well, or patrol-ring fallback); telegraphed. Breathe + Tallyman line.
- [ ] Fight 2: the Final Tally (cargoB, full grade, 2 turrets + 2 tubes)
      casts off from the anchorage with one corvette screening it; staged
      on picket-down + breathe gate; spawn distances vs envelopes checked,
      Auditor-precedent ack if the cast-off is intended close drama.
- [ ] Ending: flagship down -> confirm beat, two closing comms lines
      clock-gated AFTER the kill, then Victory overlay with the
      campaign-complete message; NO NextScenario (the chain ends here by
      design). Lose = player death; lingering retry of final_tally only.
- [ ] Rewire lifeline victory: lingering NextScenario -> final_tally;
      adjust its temporary ending text to the hook.
- [ ] `hidden: true` + thumbnail + description; `content gen`; `content
      lint` (refs + balance).
- [ ] Harness test (gauntlet_course.rs style): layout invariants (SOI vs
      positions), survey gate one-shot, phase gating picket -> flagship,
      win wiring (kill -> gated comms -> Victory, no next), lose wiring,
      lifeline -> final_tally chain (test: names recorded when written).
- [ ] Probe evidence: autopilot example + `nova_probe -- run`; record
      verdict (watch the broadside-high hitch history, task 20260718-004856:
      keep object counts in the broadside band).
- [ ] Docs in-task: scenarios.md finale blurb (no spoilers beyond the
      Broadside precedent), CHANGELOG.

## Definition of Done

- The chain completes: lifeline victory queues final_tally, final_tally
  victory queues nothing
  (cmd: `grep -n "final_tally" assets/base/scenarios/lifeline.content.ron`;
  cmd: `grep -c "NextScenario" assets/base/scenarios/final_tally.content.ron`).
- final_tally is hidden with a thumbnail
  (cmd: `grep -n "hidden\|thumbnail" assets/base/scenarios/final_tally.content.ron`).
- content lint green incl. balance; acks carry reasons
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- Harness tests green (test: names recorded in Steps when written).
- Probe run verdict recorded (cmd: `cargo run -p nova_probe -- run <example>`).
- Docs updated (cmd: `grep -n "Final Tally" web/src/wiki/scenarios.md CHANGELOG.md`).
- manual: finale difficulty peak feels earned - winnable AND losable
  (batched to flow Finish).

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
- Spike: tasks/20260721-155249/SPIKE.md. Umbrella: 20260721-160425.
- Depends on: 20260721-160957 (Lifeline - chain source + cast in place),
  20260721-160906 (rig - picket mechanism).
