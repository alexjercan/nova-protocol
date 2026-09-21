# Review 05: Ship flight, input, and AI

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Specialist: ECS ordering follow-up
- Verdict: minor findings

## Findings

### MINOR - `crates/nova_ship/src/input/point_defense/mod.rs:112-137` - A cross-schedule ordering edge does not enforce its stated invariant

Point defense runs in `FixedUpdate` and declares `.after(SpaceshipTargetingSystems)`. Targeting runs only in `Update`. Bevy ordering edges are schedule-local, and the main schedule runs fixed updates before `Update`. The edge is therefore vacuous: point-defense ownership reads the `CombatLock` and `WeaponsRaised` values left by the previous frame's update, not values derived earlier in the same tick as the comment claims.

The focused ownership fixture registers the ownership system directly in `Update`, so it does not prove production scheduling. The practical effect is normally a one-frame lag, with existing regrasp windows much larger than that. No player-visible failure was reproduced.

Why not higher: the claimed same-tick invariant is false, but the stale window is small and no harmful transition was demonstrated.

### MINOR - `crates/nova_ship/src/input/player/test_support.rs:56-57` and `input/targeting/gesture.rs:287` - Comments reference deleted `WithheldVerbs`

`WithheldVerbs` no longer exists. The live root capability gate is `ShipCapabilities`. One site is a broken rustdoc link, and both direct test authors toward a component that cannot compile. This is documentation-only.

## Adjudication

- Dropped the per-ship autopilot scratch-vector allocation as a performance finding. The churn exists when vectors grow, but no real frame cost was measured.
- Checked and cleared flight order resolution, authority and capability precedence, player intent, AI acquisition/behavior/weapons, target locking and occlusion, point-defense assignment, and PD controller logic.

## Coverage

Fully read all 58 files under `nova_ship/src/flight/`, `input/`, and `physics/`, including inline and dedicated flight tests. An ECS specialist then verified the production schedule order against pinned Bevy 0.19 and traced all relevant readers and writers.

Not checked: no Cargo command, content lint, game, probe, wasm build, workspace test, or Clippy run. Camera, section implementations, and ship audio remain for reviews 06-07. Shipped AI tuning and content values remain for the content review.
