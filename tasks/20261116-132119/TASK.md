# Railgun AI: verify behaviour and remove the stale lance TODO

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.19.0, ai, cleanup

Planned 2026-11-16 from the owner review of the game-feel research doc
(claude.ai artifact Hq6GmMBXm2zAfVFwGnQg45).

## User facts
- The owner has seen AI ships fire the railgun. The research claim that "AI
  never sets up a lance shot" is "worth investigating".
- AI is decent for now; no larger AI work in v0.19.0.

## Agent findings
- AI fires the railgun when its normal orbit swings the fixed bore across
  the target. `commit_is_open` (`crates/nova_ship/src/input/ai/railgun.rs:200-252`)
  needs: 14 s cadence (`railgun.rs:33`), charge Ready, magazine not empty,
  engaging and not Evade, a ship target inside 0.6 of the slug reach
  (`railgun.rs:39`), the bore in the hit cone (`railgun.rs:72-78`), and a
  clear line of fire.
- The AI never flies to set up a shot. That is what the TODO at
  `railgun.rs:4-11` means.
- The TODO is stale: it cites closed task `20260901-104359`, which says the
  lance-run framing is superseded and the AI "can already fire and land
  railgun shots" (`tasks/20260901-104359/TASK.md:3,10-24`). AGENTS.md allows
  `TODO(<task-id>)` only for an active task.

## Delivery
1. Delete or rewrite the stale TODO with a concrete reason and removal rule.
2. Record whether a lance run is worth doing later; no AI behaviour change
   in this task unless the owner asks.

## Done when
- No TODO cites a closed task in `railgun.rs`, and the decision is recorded
  here.

## Findings and decision (2026-11-21)

### Verification: not reproduced in the closest live fight
- No permanent proof exists for "an AI ship fires and lands a lance shot".
  The unit tests in `crates/nova_ship/src/input/ai/railgun.rs:275-322` cover
  only the envelope geometry. `system_railgun_lance`, `system_railgun_hulls`,
  `stress_hull_collapse` and `system_open_world` fire the lance by script or
  player input. `system_ai_combat` flies with empty magazines.
- Closest live fight: `wfc_arena` (`examples/playable/wfc_arena.rs`). Each
  drafted hull carries two bow lances under the AI (log:
  `insert_railgun_section: entity 1658v0`, `1659v0`, `3743v0`, `3744v0`).
  Its smoke walk ends at the first merge (`autopilot: cycle complete, no
  panic (t=13.0s)`), so it says nothing about lances.
- A temporary, uncommitted hold step kept the default matchup
  (`--seed 20260816 --ship amber:industrial:20260816 --ship
  onyx:industrial:20260818`) fighting for 180 s on lavapipe, twice. Result:
  0 `on_railgun_fired_flash` and 0 `on_slug_impact_flash` lines in both runs,
  while the turrets fired thousands of rounds (`AMBER x1 fired 3212 Kinetic
  rounds + 2003 Pierce rounds - dealt 33029.4 ... | ONYX x1 fired 2274
  Kinetic rounds - dealt 15167.2`). The hulls then drifted apart with no
  further fire (range 159 -> 3229 engine units).
- Temporary gate counters in `commit_is_open` (removed after the run), final
  totals: 446 evaluations; state gate (passive, or Evade) 372; bore cone 74;
  cadence, charge, magazine, target, anchor, hit radius, reach and line of
  fire 0. No evaluation reached the line-of-fire check. From 4 s to 17 s
  into the brawl (range 36-93) 168 of 168 new evaluations failed on state, so the
  AI was in Evade or not yet engaging (inferred; the counter did not split
  the two). In the next 12 s, 66 of 70 failed on the bore cone. Evaluations
  stopped at about 35 s of fight, which suggests every
  lance was destroyed or disabled by then (inferred, not checked).
- Limit: lavapipe ran Update at roughly 2-3 evaluations per lance per
  second. The gate samples once per Update, so a fast orbit sweep can jump
  the narrow hit cone at that rate. One seed, one matchup; the owner's
  sighting of AI lance fire is not contradicted, only not reproduced here.

### TODO change
- `railgun.rs:4-11` no longer cites closed task `20260901-104359`. The
  module doc now states the behaviour: the AI never flies a setup run; it
  fires only when its orbit carries the bore across the ship it fights and
  every gate in `commit_is_open` holds (cadence, charge, magazine, engaging
  and not Evade, ship target inside `AI_RAILGUN_REACH_FACTOR` of reach, bore
  in the hit cone, clear line of fire).
- No other `TODO`/`FIXME`/`WTF`/`XXX` exists in `crates/nova_ship/src/input/ai/`.

### Lance run later (agent recommendation, unapproved)
- Measure before building it. First run the same hold on two or three more
  matchups at hardware frame rate, counting `RailgunFired` per AI ship. If
  AI lance fire stays near zero, the lance run (break orbit, roll onto the
  line, commit, peel off) is worth doing under the combat-feel work, since
  the evidence above says Evade and the bore cone, not reach or cadence,
  keep the lance quiet. No AI behaviour change in this task.
- Smallest permanent proof, if the owner wants one: a `systems/` example
  with one AI hull carrying a loaded lance against one parked target hull
  at about 1 km, asserting at least one `RailgunFired` from the AI lance and
  one slug `SurfaceImpact` on the target within a step deadline.
