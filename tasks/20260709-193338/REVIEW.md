# Review: Gravity wells - bounded one-way gravity with sphere of influence

- TASK: 20260709-193338
- BRANCH: gravity-wells

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed commit 24a9030 against master, spec (TASK.md + spike
tasks/20260709-193147/SPIKE.md), with an
independent adversarial pass over the diff and the avian 0.7 source for the
force/Position semantics. The math core, hysteresis, one-way invariant,
scheduling, and test assertions are sound (details at the end). Two findings
rise to MAJOR because the follow-up ORBIT task consumes exactly the surfaces
they weaken.

- [x] R1.1 (MAJOR) crates/nova_scenario/src/objects/asteroid.rs
  (insert_asteroid_gravity_well) - well sources stay `RigidBody::Dynamic`.
  The task Goal ("sources stay static, so the world cannot clump" in the
  original description) and spike option B ("bodies on rails", "sources stay
  RigidBody::Static") both pin sources on rails; the branch only documents
  the deviation. One-way gravity does prevent clumping, but a dynamic well
  source can be shoved by ship rams, torpedo blasts, and destruction recoil,
  dragging its SOI - and every orbit in it - along. The ORBIT verb
  (20260709-193339) would station-keep around a driftable center. Suggested
  change: when the designation observer attaches a `GravityWell`, also
  insert `RigidBody::Static` on the source (overriding the base bundle's
  Dynamic), and pin it with a test (a rammed/impulsed well source does not
  move). Small field rocks without wells stay dynamic, unchanged.
  - Response: fixed - insert_asteroid_gravity_well now inserts
    RigidBody::Static alongside the well, pinned by component asserts in
    a_big_rock_gets_a_default_well_and_a_field_rock_gets_none (well rock
    Static, well-less rock stays Dynamic).

- [x] R1.2 (MAJOR) crates/nova_gameplay/src/gravity.rs (DominantWell) - a
  despawned well leaves a dangling `Entity` in `DominantWell` until the next
  FixedUpdate rebuild, and the path is untested. The playtest Gravity Rock is
  destructible (health 2000), so "the well I orbit just died" is a likely
  runtime path, and `DominantWell` is the stated public API for the HUD GRAV
  line and the ORBIT verb - an Update-schedule consumer reading it in that
  window gets a dead entity. Suggested change: add an
  `On<Remove, GravityWell>` observer that removes matching `DominantWell`
  components, and an integration test: orbit a well, despawn the well,
  assert `DominantWell` is gone and the velocity stops changing (no panic).
  - Response: fixed - remove_dominant_well_on_well_removed observer
    (On<Remove, GravityWell>) strips matching handles in the same flush; new
    test despawning_the_owned_well_releases_dominance_without_panic orbits,
    kills the well, asserts DominantWell is gone and the probe coasts
    force-free. DominantWell doc tells consumers to still handle a failed
    get.

- [x] R1.3 (MINOR) crates/nova_gameplay/src/gravity.rs (gravity_well_system)
  - one-tick spurious `DominantWell` from avian's placeholder positions.
  Avian 0.7 spawns rigid bodies with `Position::PLACEHOLDER` (= Vector::MAX;
  verified in avian source, rigid_body/mod.rs:270, transform.rs:54), synced
  from `Transform` only inside the physics schedule. When a well and an
  affected body spawn in the same flush (the scenario loader does exactly
  this), both read Vector::MAX on the first tick: distance = 0, well_accel
  returns the clamped surface value, and a false `DominantWell(Gravity Rock)`
  is inserted for one tick (no force is applied - try_normalize on the zero
  vector fails - but the HUD would flash GRAV at scenario start). Suggested
  change: compute the offset first and only push a candidate when the
  direction normalizes and r is finite and positive; then reuse that
  direction for the force application.
  - Response: fixed - the candidate loop computes the offset first and
    requires try_normalize (rejects zero and non-finite offsets, i.e.
    coincident Position::PLACEHOLDER spawns) before a well can become a
    candidate; the direction is reused for the force application.

- [x] R1.4 (MINOR) crates/nova_gameplay/src/gravity.rs (tests) - nothing
  exercises `NovaGravityPlugin::build` itself: the physics tests add
  `gravity_well_system` raw and register the observers by hand, so dropping
  an `add_observer`/`init_resource` line from the plugin passes every test.
  Suggested change: one physics-level test that adds the real
  `NovaGravityPlugin` to the harness and asserts a spawned
  `SpaceshipRootMarker` rigid body ends up `GravityAffected` and is actually
  pulled inside an SOI.
  - Response: fixed - gravity_app now builds the real NovaGravityPlugin on
    the physics harness, and a_ship_root_is_pulled_through_the_real_plugin_
    wiring asserts a bare SpaceshipRootMarker body is opted in by the
    observer and actually pulled.

- [x] R1.5 (MINOR) crates/nova_gameplay/src/gravity.rs
  (GravitySettings::max_surface_gravity) - the "never out-muscles a live
  ship" guardrail is asserted in comments ("main drives deliver tens of
  u/s^2") but tied to nothing: ship acceleration is emergent from thruster
  magnitudes and live mass, and no test or cross-reference keeps the cap
  below it. Acceptable to resolve with documentation (a doc comment citing
  the measured flagship accel and naming the invariant as a tuning contract),
  but say so explicitly rather than implying it is enforced.
  - Response: fixed via documentation as suggested - the doc now states it
    is a tuning contract, not enforced, cites the measured harness-ship
    acceleration (~21 u/s^2) and the retuning rule.

- [x] R1.6 (NIT) crates/nova_gameplay/src/gravity.rs (gravity_well_system) -
  `candidates` Vec is allocated per affected entity per tick and `q_wells`
  is re-iterated per affected entity (O(wells x affected)). Trivial at
  current scale; worth a one-line comment acknowledging the scale assumption.
  - Response: fixed - comment on the loop states the O(wells x affected)
    scale assumption.

- [x] R1.7 (NIT) crates/nova_gameplay/src/gravity.rs (gravity_well_system) -
  the `candidates.iter().find(...) else continue` and `q_wells.get(owner)
  else continue` fallbacks after `dominant_well` are unreachable by
  construction (the helper only returns candidate entities). Either comment
  them as defensive or restructure so the chosen candidate's pull and
  direction come back directly, saving the re-find.
  - Response: fixed - the q_wells.get re-find is gone (candidates carry
    accel + direction; single find on candidates), and the remaining
    defensive else is commented as unreachable by construction.

- [x] R1.8 (NIT) crates/nova_scenario/src/objects/asteroid.rs
  (AsteroidSurfaceGravity) - double optionality: the component is always
  inserted and wraps an `Option`, and the observer reads
  `Option<&AsteroidSurfaceGravity>` around it. Works, but two ways to say
  "none"; consider dropping the query-side `Option` since the bundle
  guarantees presence.
  - Response: fixed - the observer query now takes &AsteroidSurfaceGravity
    directly; the inner Option remains as the authored-vs-default signal.

- [x] R1.9 (NIT) crates/nova_gameplay/src/gravity.rs (DominantWell) - holds
  an `Entity` without entity-mapping reflection; if scenes/serialization
  ever touch it the id will not remap. Note only, no current consumer.
  - Response: fixed via documentation - DominantWell doc notes it is not
    entity-mapped for reflection and no consumer serializes it.

Checked and found sound: well_accel math (inverse square, surface clamp
exactness, smoothstep to exact zero at the SOI edge, guards), dominant_well
hysteresis (incumbent-left, empty set, tie behavior, total_cmp), the
apply_linear_acceleration substitution (verified mass-independent,
per-step-cleared semantics in avian source; accumulates correctly with
thruster impulses on the same root), the one-way invariant including the
Without<GravityWell> belt-and-braces and untouched Gravity::ZERO, observer
ordering (bundle lands before Add observers fire), threshold + override +
cap semantics at all call sites (loader, editor, assets), Gravity Rock
placement genuinely clear of the scatter cube, and the test suite asserting
behavior (exact values, full-lap radius band, exact-zero velocities,
stepwise hysteresis handoff) rather than mere execution.

## Round 2

- VERDICT: APPROVE

Verified every round-1 response against commit 344e999: the designation
observer inserts RigidBody::Static with the well and the asteroid test
asserts both body types; remove_dominant_well_on_well_removed is registered
in the plugin and the despawn-mid-orbit test passes (DominantWell gone,
force-free coast afterward); the candidate loop gates on try_normalize
before candidacy; gravity_app builds the real NovaGravityPlugin and the
bare-ship-root test proves the observer + force path end to end; the cap,
scale, defensive-else, and DominantWell docs read as promised; the observer
query drops the redundant Option. All 14 gravity + 4 asteroid tests pass;
fmt + workspace check clean (full suite and clippy on CI per project
policy).

New code since round 1 (the user-requested F11 gravity debug overlay in
nova_debug) was reviewed as part of this round:

- [x] R2.1 (MINOR) crates/nova_debug/src/gravity.rs - draw_gravity_wells
  takes Res<GravitySettings>, which panics in a debug-only app built without
  NovaGravityPlugin. Suggested change: init_resource in
  GravityDebugPlugin::build, same defensive pattern as AsteroidPlugin.
  - Response: fixed in-round - GravityDebugPlugin::build now init_resources
    GravitySettings with a comment.

The overlay otherwise follows the SectionsDebugPlugin pattern exactly
(PostUpdate after transform propagation, DebugSystems set, tailwind palette)
and its dominant-well link query already tolerates a just-despawned well.
No other findings; the branch is ready to land.
