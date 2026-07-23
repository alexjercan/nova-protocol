# Review: ch5 raid playtest tuning

- TASK: 20260723-200643
- BRANCH: feature/ch5-raid-tuning

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [ ] R1.1 (MINOR) webmods/the-ledger/ledger_ch5_the_raid.content.ron:2534 - the
  leash bounds the base only while it is NOT `recently_damaged`: the engine's
  tether logic (ai.rs ~793-797) lets a shot ship "defend itself until the memory
  fades", overriding the leash. The base IS the torpedo target, so once the
  player hits it, it can transiently nudge toward the player with its 2 RCS
  thrusters before the damage memory decays. Disclosed, not claimed-fixed (the
  Outcome + GOAL flag "RCS-hold vs actual-drift is a playtest question" and list
  the levers). Keep it on the pending playtest checklist.
  - Response: ACCEPTED as a disclosed playtest item, no code change now (a MINOR
    left to discretion under APPROVE). The lurch is bounded by the base's weak
    2-basic-thruster authority and decays with the damage memory; if the playtest
    shows the base walking toward the player once torpedoes land, the documented
    levers apply (tighten the leash, lower planetoid_3 gravity, or move the base
    fully out of the well). Added to the umbrella Manual acceptance checklist.

What the out-of-context reviewer verified (re-confirmed in-session; I had already
traced the AI-engage/leash model and the SOI geometry myself):

- lint `--target webmods/the-ledger`: 0 error/warning/finding, 6 scenarios
  balance-audited, 1 pre-existing ack. No R-key flight conflict; thruster + turret
  mounts validated (base-against occupied cells).
- `ledger_ch5_raid` 11/11, `ledger_ch4_ending` 10/10, `webmods_validation` 1/1;
  `cargo fmt --check -p nova_assets` clean.
- Geometry (base spawn (0,15,-520)): p1 416u vs SOI 112 (clear); p2 269u vs SOI
  96 (clear); p3 98.6u vs SOI 128 -> base in p3's MILD well, residual accel
  mu/dist^2 = 768/9722 ~= 0.079 u/s^2 (~1/265 of ship authority), so 2 RCS
  thrusters trivially hold it. Matches the task's ~0.08 claim.
- Base config: `AI((leash: Some(15.0)))`, no patrol/orbit -> passive Idle holds
  spawn; gained rcs_xp/rcs_xm thrusters, dropped the two x-arm turrets -> 2
  turrets + 2 thrusters. Torpedoes `Mouse(Right)` -> `Keyboard(KeyR)` (gamepad
  RightTrigger2 kept); `hidden: false` with a re-hide comment.
- Test meaningfulness (mutation-checked by reading): the R-key test does an exact
  `BindingInput::try_from(b) == Keyboard(KeyCode::KeyR)` (fails on RMB); the base
  test fails without the thrusters/leash/turret-trim; the bundle pin was UPDATED
  to a real new value (1.11.0), not weakened.
- Docs match the RON (bundle 1.11.0, CHANGELOG 1.11.0, README/news "R key",
  mod-guide walk 1.11.0); the only surviving 1.10.0 strings are dated history.

Pending manual check (human-acceptance gate, batched at Finish): playtest ch5 -
gravity calm for the small ships; the base holds its post AND does not walk toward
you once the torpedoes start landing (the R1.1 risk); torpedoes fire on R; 2
turrets on the base; launches from the picker.
