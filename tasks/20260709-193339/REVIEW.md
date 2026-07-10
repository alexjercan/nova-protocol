# Review: ORBIT autopilot verb - circularize and station-keep inside a gravity well

- TASK: 20260709-193339
- BRANCH: orbit-verb

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commits b01a76b + bfe5852 against master with an independent
adversarial pass over the diff, the spike (decision 6), and the consuming
seams (AI, HUD, input contexts, targeting feeds). The orbit math (tangential
sign, plane fallbacks, band clamp well-formedness), the STOP/GOTO
restructure (verified byte-equivalent semantics), the plan-once state
machine, the Hold hysteresis, the capability/dead-well disengages, and the
targeting fix's interaction with hostility/turret/torpedo feeds are all
sound. One blocker, confirmed by running the failing test.

- [x] R1.1 (BLOCKER) crates/nova_gameplay/src/flight.rs
  (NovaFlightPlugin::build) - autopilot_system now takes
  `Res<GravitySettings>` but the flight plugin never initializes it. Masked
  in the game binary (NovaGameplayPlugin adds NovaGravityPlugin right
  after), but input/ai.rs's patrol physics test builds the flight plugin
  without gravity and panics on the missing resource - CONFIRMED:
  `a_patrol_ship_flies_its_first_leg_and_turns_onto_the_next` FAILS on this
  branch. The affected-modules test run (flight/gravity/targeting/HUD)
  missed input::ai. Suggested change:
  `app.init_resource::<GravitySettings>()` in NovaFlightPlugin::build
  (idempotent alongside NovaGravityPlugin's own init), and add input::ai to
  the affected-test list whenever autopilot_system's signature changes.
  - Response: fixed - NovaFlightPlugin::build init_resources GravitySettings
    with a comment naming the standalone-flight-layer reason; the patrol
    test now passes (run locally to confirm the blocker and the fix).

- [x] R1.2 (MINOR) crates/nova_gameplay/src/hud/flight_status.rs
  (update_flight_status_text) - the `AP ORBIT ... r <radius>` readout
  resolves the well from `DominantWell`, not from the engaged action. The
  two diverge when dominance flips mid-orbit in overlapping SOIs (readout
  shows the wrong rock's distance) or when the ship transiently exits the
  SOI (DominantWell removed while ORBIT keeps flying - the radius silently
  drops off the line). Suggested change: when the action is Orbit, resolve
  name/radius from the action's well; keep DominantWell only for the
  MAN/GRAV arm.
  - Response: fixed - update_flight_status_text resolves the reported well
    from the Orbit action when engaged, DominantWell otherwise, with a
    comment explaining the divergence cases.

- [x] R1.3 (MINOR) crates/nova_gameplay/src/input/player.rs (ORBIT
  bindings) - `GamepadButton::South` collides with the scenario-advance
  binding (`NextScenarioInput`: Enter + South, nova_scenario/loader.rs:205,
  alive for the whole scenario). A pad press during a lingering scenario
  end would both skip the scenario and toggle ORBIT; a pad player parking
  can skip the scenario. Suggested change: bind ORBIT to a free button -
  DPadDown is unused (DPadLeft/Right, North, East, West, triggers are
  taken).
  - Response: fixed - ORBIT binds KeyO + GamepadButton::DPadDown, with a
    comment warning off South.

- [x] R1.4 (MINOR) crates/nova_gameplay/src/flight.rs (orbit_target_radius
  + plan block) - a well whose clearance radius exceeds the trusted band
  (the shipped tiny-well unit test: body 10, SOI 12 -> ring at 16.5, beyond
  the SOI edge) plans a "orbit" outside the well: zero gravity assist, a
  permanently powered centripetal burn, DominantWell removed, so the GRAV
  line, the [O] cue, and the radius readout all vanish while ORBIT flies an
  incoherent circle. Suggested change: treat "no stable band" as
  unorbitable - have the plan step disengage (like a dead well) when
  clearance exceeds `orbit_band_safety * fade_start`, and update the helper
  + unit test to express that (e.g. return Option).
  - Response: fixed - orbit_target_radius returns Option (None when the
    clearance exceeds the trusted band), the plan block disengages on None
    like a dead well, and the tiny-well unit test now pins None for both
    probes.

- [x] R1.5 (NIT) crates/nova_gameplay/src/flight.rs (desired match, Goto |
  GotoPos arm) - the inner `_ => unreachable!()` is safe today but the
  outer and inner matches can drift independently; splitting into separate
  Goto/GotoPos arms sharing the arrival code removes the panic arm.
  - Response: fixed - split into separate arms sharing an arrival_desired
    closure; the unreachable! is gone.

- [x] R1.6 (NIT) crates/nova_gameplay/src/flight.rs (q_wells) - the
  `Without<SpaceshipRootMarker>` filter is not needed for aliasing; its
  real effect is that a ship carrying a GravityWell could never be orbited.
  No ship is a well today - state the intent in a comment (or drop the
  filter) so the future surprise is authored, not accidental.
  - Response: fixed - comment states the filter is a design statement (a
    ship is never an orbit target; ORBIT treats it as well-gone).

- [x] R1.7 (NIT) crates/nova_gameplay/src/flight.rs (Orbit desired arm) -
  `let Some(plan) = plan else { continue }` silently skips throttle
  zeroing for a state that is unreachable by construction (the plan block
  either filled it or disengaged this tick). Document the invariant
  (comment or debug_assert) instead of a silent skip.
  - Response: fixed - comment documents the skip as unreachable by
    construction, defensive only.

Checked and found sound: tangential direction never fights existing motion
(n = r x v, t = n x r_hat); plane-normal fallback chain incl. up-parallel-
radial; band clamp min>max well-formedness; STOP/GOTO desired-velocity
semantics byte-equivalent through the restructure; braking_plan closure
captures; done-gating adds only the Orbit conjunct; no non-exhaustive
AutopilotAction matches elsewhere (AI constructs Stop/GotoPos only);
plan-once lifecycle incl. re-engage and teleport; Hold enter/exit straddles
the attitude deadband and outranks Burn per spec; dead well/engines/
controller disengages pinned by tests; stale-DominantWell engage double-
covered; targeting fix cannot make rocks hostile (no Allegiance ->
Neutral) and restores pre-gravity lock behavior; screen indicators hide on
unresolved entities so well death in the frame gap is safe; orbit cue
despawn wired; status-line formats pinned by tests.

## Round 2

- VERDICT: APPROVE

Verified every round-1 response against the new diff: GravitySettings is
initialized in NovaFlightPlugin::build and the previously failing patrol
physics test passes; the ORBIT status line resolves the action's well (with
the divergence cases documented); the pad binding moved to DPadDown
(confirmed unbound anywhere else in the workspace); orbit_target_radius
returns None for band-less wells, the plan block disengages on it, and the
tiny-well test pins None; the goal match has separate Goto/GotoPos arms
with no unreachable!; the q_wells filter and the defensive plan skip are
documented as intent. All affected modules green (51 flight incl. the
patrol-adjacent machinery, 14 gravity, 25 targeting, 7 HUD, 1 AI patrol);
fmt + check --workspace --examples clean. No new findings; the branch is
ready to land.
