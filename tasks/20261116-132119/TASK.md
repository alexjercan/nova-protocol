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
