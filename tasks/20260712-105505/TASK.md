# Bullets affected by gravity wells

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: feature, gameplay, spike, v0.5.0

Make turret rounds (bullets) feel gravity wells, the same way ships and
torpedoes already do. Today only ship roots and torpedo projectiles opt into
`GravityAffected`; turret rounds and section debris deliberately skip it.

## Spike outcome (read first)

Spike: docs/spikes/20260712-112113-bullets-affected-by-gravity.md
(RECOMMENDED - conditional). It measured the curvature and settled the scope:

- Curvature is perceptible ONLY on close grazing passes near a strong well
  (~2-4u miss); at typical combat geometry (b >= 50u) it is sub-degree / ~1u -
  the original "imperceptible" call was mostly right.
- Correctness is subtle and may be a NET WIN. The turret already aims behind
  the target in a well today: the target is `GravityAffected`, so it is
  accelerating while the lead solver assumes constant velocity. A bullet and
  target near the same point share common-mode acceleration that largely
  cancels in the relative frame, so bullet gravity can REDUCE that existing
  miss (only first-order - degrades when they sit at very different radii).
- Decision: build **Option C1** - opt bullets in, add a measured perf guard,
  and OBSERVE net PDC accuracy in wells (don't assume it worsens). Bullets
  only; debris stays deferred. If the free common-mode cancellation is not
  enough in playtest, build the follow-up C2 (full gravity feedforward in the
  intercept solve for BOTH target and bullet) - user is willing to fund it.

## Steps

- [x] Confirm the force path a bullet would take BEFORE choosing the opt-in
      shape. Read `gravity_well_system` (`crates/nova_gameplay/src/gravity.rs`,
      the `pub(crate) fn` near line 306): it inserts/removes `DominantWell` via
      `Commands` every time an entity crosses an SOI boundary. That is an
      archetype move + command-buffer entry per crossing. Decide, with a note
      in the task, whether ~500 live rounds/turret can ride the same system or
      need a lighter bullet-only force path that applies the dominant well's
      pull WITHOUT maintaining `DominantWell` (bullets need no hysteresis, no
      HUD/ORBIT readout). Default assumption to verify by measurement (below):
      the shared system is fine at nova's well count; only add a lighter path
      if the measurement says so.
      DECISION: ride the shared system. The `Commands` churn is NOT per-tick -
      the system only touches `DominantWell` when the owner CHANGES
      (`if current.map(|d| **d) != Some(owner)`), i.e. ~2 archetype moves over
      a round's whole life (enter SOI, leave). Out-of-SOI rounds cost only the
      well loop + a per-tick Vec alloc, and nova's well count is a handful.
      Simplest, matches the torpedo pattern; revisit only if step 5 measures a
      problem.
- [x] Opt turret rounds into gravity. Add a third observer
      `insert_gravity_affected_on_turret_round(add: On<Add,
      TurretBulletProjectileMarker>, ...)` mirroring
      `insert_gravity_affected_on_torpedo` in `gravity.rs`, and register it in
      `NovaGravityPlugin::build` next to the other two `add_observer` calls.
      `TurretBulletProjectileMarker` is defined at
      `crates/nova_gameplay/src/sections/turret_section.rs:128`; confirm it is
      reachable via `crate::prelude::*` (gravity.rs already imports the prelude
      and the torpedo marker through it) - if not, add the import.
- [x] Extend the observer-wiring test `ship_roots_and_torpedoes_opt_into_gravity`
      (gravity.rs, ~line 467) to also spawn a `TurretBulletProjectileMarker`
      and assert it receives `GravityAffected`. Rename to reflect the three
      opt-in sources.
- [x] Add a physics-level regression proving a round actually CURVES under a
      well, A/B against no-gravity (lesson `fail-first-regression-ab` +
      `would-it-fail-without-it`). Use the existing gravity test harness
      (`gravity_app` / `spawn_well` / `spawn_probe` in the gravity.rs test
      module): spawn a well, spawn a bullet-like dynamic probe on a tangential
      pass, settle N ticks, assert lateral deflection > a threshold that a
      straight-line round (no `GravityAffected`) would NOT meet. Record both
      numbers (deflected vs straight) in the assertion/comment. Use raw
      `Position` for the readout (FixedUpdate clock, lesson `two-clocks`), and
      derive the well geometry from the runtime `GravityWell`
      (`from_surface_gravity`), not nominal constants (lesson
      `authored-vs-derived-values`).
- [x] Measure per-frame cost with a full PDC stream near a well. Added the
      `gravity_system_marginal_cost` `#[ignore]` bench (times 1500 affected
      bodies + a well vs 1500 plain bodies, so avian's cost cancels and the
      delta is the gravity system) and a visible gravity planetoid to
      `examples/08_turret_range.rs`. Measured: the ORIGINAL Vec-per-entity
      system cost ~2.2 ms/tick over 1500 bodies (~1.5us/entity) - not free.
- [x] If and only if the measurement warrants it, add the perf guard. It was
      warranted: the ~2.2 ms/tick was almost entirely the two heap allocations
      per affected entity per tick, not the O(wells) math. Fixed at the source
      by reusing two `Local<Vec>` scratch buffers across entities (clear() keeps
      capacity, so zero allocs after warmup). Re-measured: ~0.07-0.13 ms/tick
      over 1500 bodies (~30x). No SOI cull or separate no-`DominantWell` bullet
      path needed - the shared system is now cheap enough for thousands of
      rounds, and the win benefits ships/torpedoes too.
- [x] Update the `GravityAffected` doc comment in gravity.rs (currently states
      "Turret rounds and section debris deliberately skip v1") so it reflects
      that turret rounds now opt in and only debris remains out. Sweep the
      workspace for other prose asserting rounds skip gravity - the
      gravity-wells spike's decision 5 and open-questions text describe the old
      behavior; leave the historical spike as-is but do not let live code
      comments or CHANGELOG contradict reality (lesson `sweep-then-delete`
      prose variant).
- [x] Playtest hook (documentation, not code): the net PDC-accuracy-in-a-well
      question is answered by PLAYTEST, not the suite. The turret range shows
      bullet curvature against a FIXED target (shooter and gates do not fall);
      the interesting cancellation case needs a FALLING target (an AI ship near
      a well in a combat scenario, e.g. asteroid_field). Baseline aims behind
      the falling target; bullet gravity is a free common-mode correction;
      measure net accuracy before/after. If it does not net out, that is the
      trigger to file the C2 follow-up (full target+bullet gravity feedforward
      in `lead_intercept_point`).

## Notes

- Relevant files:
  - `crates/nova_gameplay/src/gravity.rs` - `GravityAffected`,
    `insert_gravity_affected_on_torpedo` (~line 211), `NovaGravityPlugin::build`
    (~line 180, three `add_observer` calls), `gravity_well_system` (~line 306,
    maintains `DominantWell` via `Commands`), and the test module (wiring test
    ~line 467, `gravity_app`/`spawn_well`/`spawn_probe` helpers).
  - `crates/nova_gameplay/src/sections/turret_section.rs` -
    `TurretBulletProjectileMarker` (line 128), bullet spawn (~line 903),
    intercept solver `lead_intercept_point` / `update_turret_aim_point`
    (constant-velocity, no gravity term - the C2 seam).
  - `examples/08_turret_range.rs` - measurement rig.
- Perf shape: fire_rate 100 rounds/s x 5s lifetime = ~500 live rounds per
  turret; pre-opt-in the affected set was "tens" (ships + torpedoes).
  Measured marginal cost of the gravity system over 1500 affected bodies:
  ~2.2 ms/tick with the old Vec-per-entity path, ~0.1 ms/tick after the
  scratch-buffer reuse. Reproduce: `cargo test -p nova_gameplay
  gravity_system_marginal_cost -- --ignored --nocapture`. Scope: this is the
  steady-state per-tick force cost (bodies start inside the SOI); the
  per-crossing DominantWell insert/remove churn is not separately measured but
  is bounded by SOI-crossings/sec (a round crosses at most a couple of
  boundaries in its whole life).
- Curve regression measured 3.25u lateral deflection (gravity-free control
  0.0u) on its rig - `a_turret_round_curves_under_a_well_and_a_gravity_free_body_does_not`.
- Measured curvature (spike impulse approx, v=100 u/s, 20u rock): grazing pass
  ~2-4u miss; b>=50u sub-degree/~1u. The regression threshold in step 4 should
  sit comfortably above the physics-integrator noise but is only asserting
  "curves at all", not a specific miss distance.
- Out of scope: debris gravity (stays deferred); the gravity-aware turret
  intercept term C2 (target + bullet feedforward in `lead_intercept_point`) -
  file as its own follow-up if the playtest asks for it.
- Spike: docs/spikes/20260712-112113-bullets-affected-by-gravity.md

## Outcome

What shipped (Option C1):
- `insert_gravity_affected_on_turret_round` observer in gravity.rs opts turret
  rounds into `GravityAffected`, wired in `NovaGravityPlugin::build`. Rounds
  ride the existing `gravity_well_system`; no separate bullet path.
- Tests: wiring test extended to cover rounds; new physics regression proving a
  round curves 3.25u under a well while a gravity-free control stays straight
  (in-test A/B, fails if the opt-in is removed); `#[ignore]` perf bench.
- Perf: `gravity_well_system` now reuses two `Local<Vec>` scratch buffers
  instead of allocating two Vecs per affected entity per tick. This was the fix
  the measurement demanded and it drops the system's marginal cost ~30x
  (~2.2 -> ~0.1 ms/tick over 1500 bodies), benefiting ships/torpedoes too.
- Range: `examples/08_turret_range.rs` gained a gravity planetoid and a
  shooter-anchor system so you can watch rounds bend; doc comment updated.
- Docs: `GravityAffected` doc comment + CHANGELOG updated; prose swept.

Alternatives considered and rejected:
- A lighter no-`DominantWell` bullet-only force path (step 1's fallback):
  unnecessary once the alloc was removed - the shared system is cheap enough,
  and one path is simpler than two.
- An SOI broadphase / out-of-SOI early-out: the per-entity cost was the alloc,
  not the well scan, so a cull would have added complexity for little gain.

Difficulties / diagnosis:
- The perf number was a genuine surprise: I expected the O(wells x affected)
  scan to dominate, but the A/B bench (affected+well vs plain bodies, equal
  avian cost) isolated the gravity system at ~1.5us/entity, and swapping the
  two per-entity Vecs for reused buffers collapsed it to ~47ns/entity. Lesson:
  measure before optimizing the loop you assumed was hot - it was the
  allocator, not the arithmetic.
- Absolute ms/tick numbers drift with machine load; only the within-run delta
  (both cases spawn N bodies) is meaningful. The bench prints all three.

Self-reflection:
- The curve regression's own gravity-free control is the A/B, so it satisfies
  would-it-fail-without-it without a sabotage recompile - cheaper and it lives
  in the committed test forever.
- The turret range shows curvature against a fixed target only; the net-accuracy
  question (falling target + common-mode cancellation) still needs a combat-
  scenario playtest. Documented rather than faked in a test, since the suite
  cannot answer "does it feel better".
- What could go better: I guessed "~6.6u" and "~1.5 ms/tick" in comments before
  measuring and had to correct both. Should have measured first, then written
  the number - never write a number I have not read off a run.
