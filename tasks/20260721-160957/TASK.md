# Lifeline (ch3a): convoy-defense scenario, gunship chain hook, picker wiring

- STATUS: CLOSED
- PRIORITY: 53
- TAGS: v0.8.0,content,scenario

## Story

Chapter 3 part one per the spike (tasks/20260721-155249/SPIKE.md,
Recommendation): "Lifeline" - defend a two-hauler convoy crawling a freight
lane against three telegraphed raider waves under a relief countdown, then
chain onward. Also un-dead-ends broadside_gunship: its victory currently
offers no Continue (broadside.rs:446-540) - it gains the hook line and a
lingering NextScenario into lifeline. Encounter shape is NEW on every axis:
protect objective, light-wave composition, clock pressure (HudReadout).

The PRIMARY variant uses `allegiance: Some(Player)` AI haulers; if the
mechanisms rig (20260721-160906) came back red, apply the documented
fallback: same lane, same wave schedule, haulers turn Neutral, objective
wiring swaps to recovering jettisoned cargo pods; lose = player death only.

## Steps

- [x] Read the rig task's verdict (20260721-160906 + this task's Notes);
      confirm primary or fallback variant before authoring.
- [x] New builder crates/nova_assets/src/scenario/lifeline.rs registered in
      the base bundle: lane arena (two nav beacons ~lane ends, 3-4
      invulnerable boulders staggered along it, light chaff scatter), two
      cargoA haulers (variant-dependent allegiance/AI patrol crawling the
      lane), player spawn trailing the convoy.
- [x] Beats per the beat sheet: Belt Relay + Halloran announce lines;
      "Screen the convoy" objective; relief countdown HudReadout
      (OnUpdate recomputes relief_remaining = T - scenario_elapsed, Time
      format; ~4 min first pass).
- [x] Waves via scenario_elapsed + wave-cleared gates, each telegraphed
      (warning line, spawn outside own weapon envelope, engage_delay):
      W1 two light racers one vector; breathe; W2 three light split
      vectors (one flanker); breathe + Tallyman taunt; W3 one full-turret
      corvette + one light.
- [x] Outcomes: win = relief timer expires with >=1 hauler alive (clearing
      W3 early also wins): Victory + intercepted-transmission hook line
      folded into the banner, temporary campaign end (Final Tally task
      rewires the chain). Lose = player death, or both haulers destroyed
      (distinct Defeat messages); lingering retry of lifeline only.
- [x] Gunship hook: rewrite broadside_gunship victory message (keeps the
      door open) + lingering NextScenario -> lifeline.
- [x] Picker wiring: lifeline visible (chapter head precedent), description,
      thumbnail Some("self://banner.png") (real art stays 20260715-220011).
- [x] `content gen`; balance: spawns authored outside envelopes by
      construction; `content lint` (refs + balance); ack only intended
      drama, with reason + task id (Auditor precedent).
- [x] Harness test in the gauntlet_course.rs style (event-driven beats, no
      wall-clock): arena/layout invariants derived from measured constants,
      wave gating sequence, countdown variable wiring, win path (timer
      expiry -> Victory + linger), both lose paths, gunship->lifeline chain
      (test: `on_start_stages_the_lane`, `base_bundle_ships_lifeline`,
      `raider_spawns_keep_the_design_floor_from_every_friendly`,
      `waves_stage_on_clock_and_clears_and_the_early_clear_wins`,
      `the_relief_bell_wins_and_the_banner_tracks_the_convoy`,
      `losing_the_whole_convoy_is_the_defeat`, `player_death_retries_the_lane`,
      `the_countdown_tracks_the_clock` in tests/lifeline_convoy.rs; chain:
      `killing_the_gunship_declares_victory_and_chains_into_lifeline`).
- [x] Probe evidence: autopilot example per the existing broadside example
      pattern; `cargo run -p nova_probe -- run <example>`; record verdict.
- [x] Docs in-task: web/src/wiki/scenarios.md chapter-three blurb; CHANGELOG.

## Definition of Done

- The chain reaches lifeline: gunship victory queues it
  (cmd: `grep -n "lifeline" assets/base/scenarios/broadside_gunship.content.ron`).
- Lifeline is picker-visible with a thumbnail; no `hidden: true`
  (cmd: `grep -n "hidden\|thumbnail" assets/base/scenarios/lifeline.content.ron`).
- content lint green incl. balance; any ack carries a reason
  (cmd: `cargo run -p nova_assets --bin content -- lint`).
- Harness tests green (test: names recorded in Steps when written).
- Probe run verdict recorded (cmd: `cargo run -p nova_probe -- run <example>`).
- Docs updated (cmd: `grep -n "Lifeline" web/src/wiki/scenarios.md CHANGELOG.md`).
- manual: first-pass difficulty feels fair - winnable AND losable (batched
  to flow Finish).

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
- Depends on: 20260721-160906 (mechanisms rig - variant decision),
  20260721-160929 (voice pass - cast constants).
- Balance-lint floor: opening hostile in own effective range of player
  spawn = ERROR, never ackable; triggered close spawn = WARN, ackable.

## Record (2026-07-21)

What shipped: crates/nova_assets/src/scenario/lifeline.rs (the convoy
defense: stalled two-hauler convoy, three clock+clear-gated raider waves,
relief countdown HudReadout, four fate-tracking Victory variants, two
Defeat paths, 8 comms beats with the Tallyman's debut), the gunship victory
rewire (both variants chain lingering into lifeline with door-open text),
picker wiring (visible chapter head + placeholder thumbnail), bundle +
build_scenarios registration, `gt_num`/pub `var` helpers, TALLYMAN cast
const, tests/lifeline_convoy.rs (8 tests: stage, layout floor, wave
staging + early win, bell win variants, both defeats, countdown), the
broadside_assault chain-test update, examples/gameplay/lifeline.rs (full
defeat/retry/waves/victory walk with legitimate clock fast-forward -
tick_scenario_clock accumulates on the stored variable, verified at
loader.rs:377) + smoke registration, scenarios.md + CHANGELOG.

Design decision (records the variant): the PRIMARY ally mechanism, but
with `controller: None` instead of AI-flown haulers. Verified in source
that every ship root gets SpaceshipRootMarker (spaceship.rs:253) and the
allegiance override applies to any controller kind, so a None+Player
hauler is targetable while structurally unable to chase - the AI-flown
alternative would Engage-chase raiders (unarmed rammers) and need leash
gymnastics. The stalled-convoy narrative (drives cold on a transfer fault)
absorbs the difference; the spike's crawling-convoy image is a playtest
question, not a mechanism.

Balance: all raider spawns authored outside the design floor (>=700u from
player spawn and both haulers, pinned by test); `content lint` balance
audit clean over 12 scenarios with ZERO new findings and no acks needed.

Verification: lifeline_convoy 8 green, broadside_assault 14 green, parity
2 green, content lint 0 errors, cargo check + fmt green, `probe run
lifeline` verdict OK (process_exit/run_completed/reached_playing/
invariants_held/log_clean PASS; fps SKIPPED, no baseline) with the full
14-stage walk (13 stage transitions) in the run log. Full clippy/test suite left to CI.

Pending manual (flow Finish): difficulty first-pass fairness (relief 240s,
wave sizes 2/3/2, W3 full-gun corvette), the crawling-vs-stalled convoy
image, cast names.

Reflection: the None-controller discovery (targetable without AI) came
from reading the spawn path BEFORE authoring - the verify-first habit
turned a possible AI-behavior rabbit hole into a one-line design choice.
The example's clock fast-forward needed the accumulate-vs-recompute fact
verified at the source; writing it into the example doc comment keeps the
next author from assuming it.
