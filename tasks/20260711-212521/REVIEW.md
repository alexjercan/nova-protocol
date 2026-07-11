# Review: AI orbit directive: config, passive behavior state, autopilot wiring

- TASK: 20260711-212521
- BRANCH: feat/ai-orbit-directive

## Round 1

- VERDICT: REQUEST_CHANGES
- Method: fresh-context agent review (out-of-context pass) which re-ran the
  suites itself (orbit 21, behavior_state 9, scenario spaceship 2, all
  green), re-derived the is_passive() refactor against the old match arms,
  audited every AIBehaviorState/engages() consumer for the new variant,
  verified all 18 mechanically-updated call sites, and traced flight.rs
  completion semantics for the stale-GOTO question.

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/input/ai.rs (test
  a_mid_flight_orbit_is_left_alone) - vacuous: autopilot_system never runs
  in the pipeline, so a re-engaged Autopilot is bit-identical to the
  original and the assert_eq cannot fail even if the !has_autopilot guard
  is deleted. Mutate a sentinel (e.g. a Some(OrbitPlan) with a marker
  radius) into the component between pipelines and assert it survives.
  The pre-existing a_mid_leg_maneuver_is_left_alone has the identical
  weakness (copied pattern); fixing the new one is the minimum.
  - Response: fixed in the round-1 fixup commit - the orbit test plants a
    sentinel OrbitPlan (radius 123) and asserts it survives a second
    pipeline; the pre-existing patrol test got the same hardening via a
    phase sentinel (AutopilotPhase::Burn).
- [x] R1.2 (MINOR) ai.rs Orbit arm - editing AIOrbitDirective.well while an
  ORBIT is engaged is silently ignored forever (ORBIT never
  self-completes; the arm engages only when no autopilot is present).
  Either re-engage on well mismatch (the ORBIT analogue of the patrol
  arm's leg_changed) or document the limitation on AIOrbitDirective.
  - Response: implemented the re-engage-on-mismatch (leg_changed analogue),
    with a new test a_retargeted_directive_re_engages_on_the_new_well; a
    non-ORBIT maneuver still flies out first (documented in the arm).
- [x] R1.3 (NIT) ai.rs Orbit arm - the well scan + debug_once miss path run
  every frame even when an autopilot is engaged and the result discarded;
  hoist behind the engage decision (interacts with R1.2's choice).
  - Response: resolved by R1.2's shape - a non-ORBIT engaged maneuver skips
    the scan via `continue`; an engaged ORBIT still resolves (needed for
    the mismatch check), which is the minimal scan R1.2 requires.
- [x] R1.4 (NIT) ai.rs debug_once! fires once per call site, not per
  ship/id; a second ship with a different bad id logs nothing. Accept or
  key the message.
  - Response: accepted as-is - it is a spawn-order debugging aid, the
    drift-until-well behavior is by design and tested; per-key logging
    machinery is not worth the weight here.
- [x] R1.5 (NIT) crates/nova_scenario/src/objects/spaceship.rs mapping test
  - add a both-set spawn asserting both components are inserted (patrol
  shadowed, not dropped), pinning the config comment's contract.
  - Response: added the both-set spawn to
    ai_config_maps_to_directive_components.

## Round 2

- VERDICT: APPROVE

All five round-1 findings verified against the new diff: the orbit no-churn
test now fails if the engage guard is deleted (sentinel plan would reset to
None), the patrol twin got the same hardening, the retarget test pins the
new re-engage-on-mismatch behavior, the both-set mapping spawn is in, and
the scan-skip falls out of the R1.2 shape. Suites re-run green (orbit
filter 22, patrol_idle 10, scenario spaceship 2; workspace check clean).

Verified clean: is_passive() exactly reproduces old Idle/Patrol arms; no
AIBehaviorState consumer outside ai.rs; every in-file consumer keys on
engages() (false for Orbit - helm frozen, no chase, guns cleared, no fire,
no torpedoes) or handles Orbit explicitly; call-site migration faithful;
new tests fail on feature deletion/precedence inversion; unresolvable-id
test has a late-well delivery guard; registration/reflection/prelude
complete; docs honest (stale-GOTO fly-out, drift-until-well). Stale-GOTO
stuck-state ruled out by trace: AI-engaged GOTOs complete on arrival; the
never-completing auto-park ORBIT path is player-input only.
