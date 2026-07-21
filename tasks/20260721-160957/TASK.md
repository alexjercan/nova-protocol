# Lifeline (ch3a): convoy-defense scenario, gunship chain hook, picker wiring

- STATUS: OPEN
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

- [ ] Read the rig task's verdict (20260721-160906 + this task's Notes);
      confirm primary or fallback variant before authoring.
- [ ] New builder crates/nova_assets/src/scenario/lifeline.rs registered in
      the base bundle: lane arena (two nav beacons ~lane ends, 3-4
      invulnerable boulders staggered along it, light chaff scatter), two
      cargoA haulers (variant-dependent allegiance/AI patrol crawling the
      lane), player spawn trailing the convoy.
- [ ] Beats per the beat sheet: Belt Relay + Halloran announce lines;
      "Screen the convoy" objective; relief countdown HudReadout
      (OnUpdate recomputes relief_remaining = T - scenario_elapsed, Time
      format; ~4 min first pass).
- [ ] Waves via scenario_elapsed + wave-cleared gates, each telegraphed
      (warning line, spawn outside own weapon envelope, engage_delay):
      W1 two light racers one vector; breathe; W2 three light split
      vectors (one flanker); breathe + Tallyman taunt; W3 one full-turret
      corvette + one light.
- [ ] Outcomes: win = relief timer expires with >=1 hauler alive (clearing
      W3 early also wins): Victory + intercepted-transmission hook line
      folded into the banner, temporary campaign end (Final Tally task
      rewires the chain). Lose = player death, or both haulers destroyed
      (distinct Defeat messages); lingering retry of lifeline only.
- [ ] Gunship hook: rewrite broadside_gunship victory message (keeps the
      door open) + lingering NextScenario -> lifeline.
- [ ] Picker wiring: lifeline visible (chapter head precedent), description,
      thumbnail Some("self://banner.png") (real art stays 20260715-220011).
- [ ] `content gen`; balance: spawns authored outside envelopes by
      construction; `content lint` (refs + balance); ack only intended
      drama, with reason + task id (Auditor precedent).
- [ ] Harness test in the gauntlet_course.rs style (event-driven beats, no
      wall-clock): arena/layout invariants derived from measured constants,
      wave gating sequence, countdown variable wiring, win path (timer
      expiry -> Victory + linger), both lose paths, gunship->lifeline chain
      (test: names recorded here when written).
- [ ] Probe evidence: autopilot example per the existing broadside example
      pattern; `cargo run -p nova_probe -- run <example>`; record verdict.
- [ ] Docs in-task: web/src/wiki/scenarios.md chapter-three blurb; CHANGELOG.

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

- Spike: tasks/20260721-155249/SPIKE.md. Umbrella: 20260721-160425.
- Depends on: 20260721-160906 (mechanisms rig - variant decision),
  20260721-160929 (voice pass - cast constants).
- Balance-lint floor: opening hostile in own effective range of player
  spawn = ERROR, never ackable; triggered close spawn = WARN, ackable.
