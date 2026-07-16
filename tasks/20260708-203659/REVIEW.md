# Review: Broadside - the capital-combat vertical slice

- TASK: 20260708-203659
- BRANCH: feature/broadside-vertical-slice

## Round 1

- VERDICT: REQUEST_CHANGES
- Reviewer: out-of-context pass (fresh-context agent; ran fmt/check/all
  suites + the full example-19 walk live, verified staging math against the
  AI constants, re-derived the act machine's re-fire safety, eyeballed the
  victory capture).

- [x] R1.1 (MAJOR) assets/base/scenarios/asteroid_field.content.ron:397 +
  CHANGELOG.md + TASK.md - the zone-clear Victory retrofit is CLAIMED but
  not landed: the reach_zone handler still switches silently (only the
  death-Defeat half of R1.8 shipped); the changelog describes a Victory
  screen that never appears. Root cause: the zone-clear edit was lost in a
  read-error retry and never re-applied. Fix: add Outcome(Victory) before
  the zone-clear NextScenario in the builder, regenerate, extend the chain
  pin to cover it.
  - Response: fixed - the zone-clear Victory re-landed in the builder (scenario.rs reach_zone handler), RON regenerated, and the chain pin now asserts BOTH halves (story_chain_declares_outcomes_at_both_ends); the CHANGELOG claim is true again. Root cause noted in the retro: a failed-Edit retry re-applied 2 of 3 edits and nothing verified the third.
- [x] R1.2 (MINOR) examples/19_broadside.rs:210,252 - the overlay asserts
  race the one-frame gap between CurrentOutcome (PostUpdate write) and the
  overlay spawn (next frame's Update): today the wait=10 cushion hides it;
  any kill-chain lengthening turns it into a CI flake. Fix: make overlay
  presence part of the stage GATE (outcome == X && overlay exists), letting
  the stall deadline catch a genuinely missing overlay.
  - Response: fixed - overlay presence joined the stage GATES (stages 4 and 9); a missing overlay parks the stage for the stall deadline to name. Re-ran the live walk green after the change.
- [x] R1.3 (MINOR) crates/nova_assets/src/scenario/broadside.rs:390 - the
  player-death Defeat handler is act-ungated: a death in act 3 (gunship
  death blast, a rock) overwrites the earned VICTORY with DEFEAT + retry.
  Fix: filter lt_num(VAR_ACT, 3.0) (already pub(crate) in shakedown.rs).
  - Response: fixed - lt_num(act, 3) gate on the player-death handler + behavior test player_death_after_the_win_declares_nothing (the act-1 test is its delivery guard).
- [x] R1.4 (NIT) crates/nova_assets/src/scenario/broadside.rs:73 - the
  gunship (and corvettes) spawn with Quat::IDENTITY facing -Z, AWAY from
  the fight at +Z, so the torpedo alignment gate fails until the hull slews
  ~180 degrees - "tubes open through the whole approach" is overstated.
  Fix: author spawn rotations facing the hauler (or soften the comment).
  - Response: fixed - corvettes and gunship spawn facing_the_fight() (a PI yaw; ships forward -Z, fight at +Z); the tubes-open claim is now authored true rather than eventually true.
- [x] R1.5 (NIT) crates/nova_assets/src/scenario/broadside.rs:377 - the
  hauler-lost handler is act-ungated: a post-victory hauler death pushes a
  fresh objective under the Victory overlay. Same lt_num gate.
  - Response: fixed - same lt_num(act, 3) gate on the hauler-lost handler.
- [x] R1.6 (NIT) tasks/20260708-203659/NOTES.md - the hauler's
  untargetability is oversold as coming from `Some(Neutral)`: with
  controller None the ship carries no Allegiance at all and relation(None,..)
  is already Neutral. The field's real value is the general surface (a
  neutral AI ship genuinely needs it, proven by the delivery guard). Amend
  the wording.
  - Response: fixed - NOTES.md gained an honesty note: the controller-None hauler is a bystander with or without the field; the surface earns its keep for neutral ships WITH a controller (the path the delivery guard pins).

Clean areas verified by the reviewer: act-machine re-fire safety (OnUpdate
self-disarms synchronously; expression filters fail closed; flags
idempotent), allegiance schema (single production path + ScatterObjects
delegation + serde compat + compiler-enforced literals), skybox defer
(menu-ambience gating covered, editor untouched, pin fails on revert),
smoke OR-sentinel soundness (guard panics on unfinished exit), structural
pins on committed RON, staging math vs measured constants (726u vs ~720u),
full live example-19 walk + victory capture eyeball.

## Round 2

(pending)

## Round 2

- VERDICT: APPROVE

Verified each Round 1 response against the new diff:

- R1.1: the regenerated asteroid_field.content.ron carries BOTH Outcome
  blocks (Defeat + Victory, grep x2); the extended chain pin asserts the
  zone-clear Victory and runs green; CHANGELOG/TASK.md claims are now
  factual. Ticked.
- R1.2: overlay presence is the gate in stages 4 and 9; the live walk
  re-ran green end to end after the change (defeat -> retry -> victory ->
  sentinel exit, exit 0). Ticked.
- R1.3/R1.5: both handlers carry the lt_num(act,3) gate - two Less nodes in
  the committed RON; the new behavior test (act-3 death declares nothing,
  with the act-1 test as its delivery guard) runs green. Ticked.
- R1.4: corvettes + gunship spawn facing_the_fight() (PI yaw); the comment
  claim is authored true. Ticked.
- R1.6: NOTES.md honesty note in place. Ticked.

Suites after fixes: broadside_assault 7, parity 2 (x2, assets clean),
nova_scenario 72 + nova_menu 46 + skybox e2e 1, fmt clean, workspace
check --all-targets clean; live example-19 walk green. Full workspace suite
+ clippy run in CI per the repo's standing instruction. No new findings.
APPROVED.
