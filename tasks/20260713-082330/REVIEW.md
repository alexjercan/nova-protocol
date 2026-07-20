# Review: Travel/combat lock slots + deliberate radar

- TASK: 20260713-082330
- BRANCH: feature/lock-slots-radar

## Round 1

- VERDICT: APPROVE

Scope: the largest diff of the family - targeting.rs rearchitecture
(components, radar gestures, upkeep, capability), player.rs (bindings, GOTO,
torpedo commit, hints, manual re-key), five HUD modules ported + one deleted +
one added, ControllerVerbs::Lock, 12_hud_range rewrite, ~25 tests
rewritten/deleted + new radar/upkeep/gesture families.

Independent verification (shared-session blind-spot guard):
- The strongest possible end-to-end check exists and passes: 12_hud_range now
  performs the REAL gesture through the live app's input pipeline (raise,
  radar hold - the LIVE picker finds the target ship by itself - release,
  commit), and every downstream HUD assertion (dwell meter, reticle drift
  0.0 px, readout, turret feed, markers, inset lifecycle, GOTO marker,
  teardown) passes unchanged. This is not a stand-in; it exercises
  EnhancedInput -> Hold/Tap conditions -> observers -> picker -> slots ->
  consumers in the shipping binary.
- The gesture unit e2e independently verifies the event-mapping traps the
  adversarial round predicted: commit fires only from `Complete` (a
  sub-threshold release commits nothing), clear only from `Fire<Tap>`, and
  the EXACT-boundary frame (5 x 50 ms real-clock steps) commits without
  clearing - the one-shared-const rule holds.
- Re-derived the Changed<Allegiance> semantics: the deliberate-neutral case
  works only because locking happens frames after the spawn insert; the test
  rig had to use a REGISTERED system for real change detection (a
  run_system_once rig would false-positive) - the test suite is faithful to
  production scheduling here, per the production-faithful-rigs lesson.

Findings:

- [ ] R1.1 (MINOR) [recorded] Same-frame RMB+CTRL latches the TRAVEL slot: the
  raised flag derives in Update while the radar Start observer fires in
  PreUpdate, so a frame-perfect simultaneous press reads stale raised=false.
  Humanly the raise precedes the radar; the live script staggers them and the
  edge is documented in TASK.md with the remedy (PreUpdate derivation or a
  TriggerState-read latch) if playtest ever hits it. Accept as recorded;
  revisit in 082337 where the raised flag gets its safety consumers.
  - Response: recorded in TASK.md Outcome; 082337 will re-evaluate when
    wiring safety off the same flag.

- [ ] R1.2 (MINOR) The combat reticle kept its RELATION tint rather than
  becoming flat red (the spike says "red crosshair"): hostile locks - the
  common case - are red; own/neutral locks tint green/gray, which is real
  information (locking your own torpedo). Sizes still separate the pair
  (32 px combat vs 40 px travel-white). Deliberate deviation, flagged for the
  user in the report; trivially revertible to flat red if vetoed.
  - Response: deliberate, documented in lock_crosshairs.rs header and the
    task Outcome; awaiting user veto.

- [ ] R1.3 (NIT) `spawn_lock_toasts` finds its stack by Name string lookup -
  fragile against a rename. A marker component would be cleaner; harmless
  today (single spawn site).
  - Response: acknowledged; fold into 082337's HUD pass if touched anyway.

Checks: 459 nova_gameplay tests green; cargo fmt clean; cargo check
--workspace --tests clean; 12_hud_range (live gesture) + 10_gameplay +
03_scenario autopilots PASS/no-panic. Full suite + clippy in CI per repo
policy.
