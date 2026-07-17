# Arrival telegraphs: engage_delay on the AI controller + the warning-beat authoring pattern

- STATUS: CLOSED
- PRIORITY: 38
- TAGS: spike,v0.7.0,ai,scenario,gameplay

Goal: enemies arrive instead of appearing. An engage_delay/spawn-passive
option on AIControllerConfig: the ship spawns on its patrol/idle routine
and goes hot after N seconds or immediately when fired upon (the leash
machinery's damage-override precedent). Pair with the authored
warning-beat pattern (clock-spaced StoryMessage + marker before the
spawn) and document it as the arrival convention. Mod-facing schema:
failure paths + literal RON syntax in the same change. Spike:
tasks/20260717-155740/SPIKE.md.

Verified at plan time: passive->Engage lives in the pure
next_behavior_state (ai.rs:704, ~12 table-test callers to re-pin on the
new contract - the decision-function variant of
pin-the-fix-at-its-boundary); the leash is the structural precedent
(component from config at spawn, spaceship.rs:322-331; damage override
via ThreatSignals.recently_damaged). PD fire is deliberately
state-independent (on_projectile_input's defending bypass), so a graced
ship still shoots down inbound torpedoes - correct and worth pinning.

## Steps

- [x] Engine: AIEngageGrace { timer } component (prelude-exported);
  next_behavior_state gains grace_held: bool - while held and NOT
  recently damaged, passive states refuse the engage pull (leash-style
  early return); damage ends the grace PERMANENTLY (the system pins the
  timer to finished so a shot telegraphed ship stays hot).
  update_behavior_state ticks the grace and derives grace_held.
- [x] Config: AIControllerConfig.engage_delay: Option<f32> (serde
  default; values <= 0 mean no grace, documented); spawn inserts
  AIEngageGrace only for positive delays (spaceship.rs, next to the
  leash insert).
- [x] Re-pin the decision table on the new contract: all existing
  callers pass grace_held=false; new table rows - grace holds passive in
  range, damage overrides grace, grace + beyond_leash compose (passive
  either way).
- [x] System tests (manual clock): graced ship stays passive in engage
  range until expiry then engages (delivery guard: same rig without
  grace engages immediately); damage during grace engages NOW and the
  grace never re-arms; PD-under-grace pin (graced ship still fires at an
  inbound torpedo via the defending bypass).
- [x] nova_scenario: spawn wiring test (positive delay -> component,
  None/zero -> none); strict-RON parse test (engage_delay: Some(6.0) +
  omitted default).
- [x] Docs: guide-author-scenario.md AI controller fields + the
  WARNING-BEAT authoring pattern (clock-spaced announce line + marker ->
  spawn far -> engage_delay grace covers the approach); scenario-system
  mention; CHANGELOG (Gameplay & Flight). NOTES.md.
- [x] Verify: cargo test -p nova_gameplay input::ai::; -p nova_scenario
  --features serde; content_lint; workspace --all-targets; fmt last.

## Close-out record

All seven steps landed; the design, the Timer-finished-flag catch and the
caller sweep are in NOTES.md. Verification: input::ai:: 92/92 (4 new
grace tests + re-pinned decision table), nova_scenario --features serde
green (spawn wiring + RON parse), content_lint clean, workspace
--all-targets green, fmt last. Full suite on CI per standing instruction.
