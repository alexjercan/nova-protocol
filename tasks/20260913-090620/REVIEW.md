# Performance review, 2026-09-13

Two reviewer lanes over `b3c6f579c..24cedcf99`, performance only. One lane
held the measurement slot; one ran static. Adjudicated in the main session,
with every load-bearing claim re-verified there. No code changed.

## Verdict

The eight-worker compute cap does NOT fix this regression. It is a real,
large, PRE-EXISTING win that was already available at v0.13.2, and it is
orthogonal to the post-v0.13.2 cost. Under a matched cap the regression is
proportionally worse, not smaller. Shipping the cap and closing this task
would bank an unrelated improvement and leave the defect in place.

Measured at matched worker counts, stock release `wfc_arena`:

| Revision | 16 workers | 8 workers |
|---|---:|---:|
| v0.13.2 `b3c6f579c` | 13.2363 ms | 8.0997 ms |
| Cone parent `f894d4c20` | 17.6846 ms | 8.9237 ms |
| HEAD `24cedcf99` | 19.6870 ms | 13.0632 ms |

Regression at 16 workers: +48.7%. At 8 workers: +61.3%. v0.13.2 gains MORE
from the cap than HEAD does (-38.8% against -33.6%), which rules out "the cap
removes a contention regime only the heavier post-cone workload enters".

## Finding 1 - MAJOR - the cap is orthogonal

`crates/nova_core/src/lib.rs:247` is where the proposed cap would land.
`WORKERS.md` measured it only at `24cedcf99`, so it had no pre-regression
reference and the 13.41 ms result read as restoration of the 13.5 ms baseline.
It is a coincidence of two independent effects.

Static evidence agreed before the measurement ran. The surviving dev trace
aggregates `/tmp/nova-release-perf/{b3c6f579c,7b07528d2}-systems.json` put
`propagate_parent_transforms` at 1142.78 us per call at v0.13.2 and 1143.39 us
at HEAD, over 1414 and 1507 calls. Flat to 0.05%. Propagation never regressed.

Change: none proposed here. This is a framing correction that invalidates the
premise the shipping decision was being made on.

Verification: nine five-repeat release sets, 45 captures, same binary per
revision per arm, pool counts asserted in every run log. Reference statistics
recomputed independently from the raw `frametime.csv` files; they match.

## Finding 2 - MAJOR - the 16-worker regime distorted the bisection

At 16 workers the cone parent `f894d4c20` carries most of the apparent
regression (13.24 -> 17.68, +33.5%) and HEAD adds only +11.4% more. At 8
workers the same parent carries almost nothing (8.10 -> 8.92, +10.2%) and
HEAD adds +46.4%.

Contention amplifies small workload increases, so a bisection run at 16
workers attributes cost to whichever commit crossed the contention knee, not
to the commit that added the work. `IDENTIFICATION.md` ran entirely in the
amplified regime. Its conclusion still holds - `6dadd95f2` is the boundary -
but the "smaller pre-cone regression" it left open is mostly amplifier.

Change: none. Run future performance bisections at a capped worker count, or
at both, and say which regime a number came from.

## Finding 3 - MAJOR - full-hierarchy propagation runs twice per frame

`avian3d-0.7.0/src/physics_transform/mod.rs:96-105` registers `mark_dirty_trees
+ propagate_parent_transforms + sync_simple_transforms` into `FixedPostUpdate`,
behind `PhysicsTransformConfig::propagate_before_physics`. `impl Default` at
`:159` sets it `true`. Nova never mentions `PhysicsTransformConfig` anywhere in
`crates/`; grep returns zero hits. So the second pass is a dependency default
nobody chose.

The trace call counts confirm it exactly: 659 frames + 754 fixed steps = 1413
against a recorded 1414. That is 2.1-2.3 passes per frame, each entering a
`ComputeTaskPool` scope that spawns 15 spin workers.

`crates/nova_core/src/lib.rs:867-871` puts the fixed schedules on the
single-threaded executor. That does not suppress this: the parallelism is
inside the system body, not in the executor fan-out.

Turning the flag off ALONE is a correctness break, not a tuning risk. Avian's
`transform_to_position` is chained directly after the pass, in the same set,
and consumes its output:

```rust
// avian3d-0.7.0/src/physics_transform/mod.rs:184-189
/// To account for hierarchies, transform propagation should be run before
/// this system.
pub fn transform_to_position(
    mut query: Query<(&GlobalTransform, &mut Position, &mut Rotation)>,
```

With the flag off the freshest `GlobalTransform` is the previous frame's
`PostUpdate` value, which holds the EASED render pose. Every interpolated body
would have `Position` pulled back by a fraction of a step, every step.

Change, narrower variant that survives: turn the flag off and restore only the
part that is load-bearing.

```rust
// In PhysicsSystems::Prepare,
// .before(PhysicsTransformSystems::TransformToPosition)
Query<
    (Ref<Transform>, &mut GlobalTransform),
    (Without<ChildOf>, With<RigidBody>),
>
// global.set_if_neq(GlobalTransform::from(*transform))
```

The deep walk is dead weight in that schedule. `update_child_collider_position`
(`avian3d-0.7.0/src/collision/collider/collider_transform/plugin.rs:67-95`)
rewrites every child collider's pose from the body pose unconditionally in
`PhysicsStepSystems::First`, after `Prepare`, so the deep pass's child writes
are overwritten before anything reads them. Only roots survive the step.

Blast radius: two `FixedPostUpdate` readers move from tick-old to frame-old -
`shed_dead_fixtures` (`crates/nova_ship/src/sections/fixture.rs:306`) and
`deal_contact_impact_damage` (`crates/nova_gameplay/src/integrity/core.rs:253`).
Both already run `.after(PhysicsSystems::Last)`. Three `FixedUpdate` readers
are already frame-old on step 1 of every frame. `autopilot_system` and
`update_turret_aim_point` are immune; the latter uses `TransformHelper`, which
walks the hierarchy itself, consistent with the refusal documented at
`crates/nova_ship/src/sections/turret_section/firing.rs:195-202`.

Load-bearing precondition, unproven: no Nova `RigidBody` carries a `ChildOf`.
A grep is not proof; this needs the assertion.

Verification, no GPU needed: spawn one `RigidBody::Dynamic` with
`TransformInterpolation` and a known velocity, set the flag false, step ~20
fixed ticks, assert `Position` advances by `v * dt` per step. It fails today.
Add the root sync and it passes.

Rejected alternative: leave the flag on and make the fixed-loop pass cheap.
`bevy_transform_interpolation-0.5.0` writes `translation` and `rotation`
through `DerefMut` unconditionally in `FixedFirst` for every interpolated
entity, so every ship root is dirty on every step before any Nova system runs.
Nova cannot quiet that pass from its own side while ships move. This also
removes any benefit from tidying the unconditional turret-joint write at
`crates/nova_ship/src/sections/turret_section/aim.rs:622`.

## Finding 4 - MAJOR - severed wreck fragments keep every plate and greeble
collider, and never expire

`crates/nova_ship/src/sections/integrity.rs:463-474` re-parents whole sections
onto a new dynamic root and strips nothing. The destroyed-section path does
strip them, and records the measurement in its own comment:

```rust
// crates/nova_gameplay/src/integrity/explode.rs:452-470
// "... Measured on the arena's worst frame, that is the difference
//  between 85 ms and 736 ms."
while let Some(node) = stack.pop() {
    commands.entity(node).try_remove::<Collider>();
```

Plates carry colliders (`crates/nova_ship/src/sections/shell_skin.rs:631-645`,
spawned `ChildOf(*section)` at `:737`, decor `ChildOf(plate)` at `:761`).

Second half: a wreck fragment has no lifetime and no cap. It is the only debris
spawn in the codebase without a `TempEntity` - compare `explode.rs:85` (30 s),
`fixture.rs:40` (12 s), `chunk.rs:132` (30 s), `spew.rs:289` (2.5 s),
`damage_sparks.rs:48` (0.35 s). The only despawn,
`cleanup_empty_wreck_fragments` (`integrity.rs:654`), fires only at zero
section children, and a wreck can sever again (`integrity.rs:337`).

`SAMPLY.md` puts the broad phase at 11.0-11.5% of sampled CPU, 6.5-6.7% in
`aabb_traverse` alone. `STRESS.md` measured whole-period collider peaks of
535-569 against 414-453. That note is right that no wreck census was
collected; this review did not collect one either.

Change: the same descendant `try_remove::<Collider>` walk `explode.rs:459-470`
already performs, plus a `TempEntity` or a fragment cap.

Blast radius: a wreck becomes something you hit at section resolution rather
than plate resolution. That is a visible gameplay change and the owner's call.

Verification: a unit test on the sever path asserting zero `Collider` on any
descendant of a `ShipWreckFragmentMarker`. It fails today.

Evidence: code-only. UNTESTED.

## Finding 5 - MAJOR - `update_sensor_contacts` has no spatial partition

`crates/nova_ship/src/input/targeting/sensing.rs:215`, scheduled every Update
at `crates/nova_ship/src/input/targeting/mod.rs:143-148`. Per rendered frame in
the trace aggregates: `update_ai_target` 906.1 us at v0.13.2 against
`update_sensor_contacts` 1496.2 us at HEAD. Net +590 us per frame in a LIGHT
combat window, on the Update critical path.

The candidate query (`sensing.rs:141-160`) walks every entity carrying
`Transform + RigidBody`, excluding only gun rounds. Torpedoes, rocks, severed
fragments, shed skin plates and damage sparks are all iterated. Most are
rejected on the first `RigidBody::Dynamic` test at `:265-271`, so each is
cheap, but the loop is observers x candidates and the candidate count grows
with the debris populations Finding 4 leaves unbounded.

NEW in range: `68c3ebe6f`. `IDENTIFICATION.md` measured that commit in
isolation at 13.245 ms against 13.505 ms for v0.13.2, so this is a real new
cost that is NOT the regression by itself.

## Finding 6 - MINOR - every live round is a transform root WITH a child

`crates/nova_ship/src/sections/turret_section/render.rs:274` puts the round's
art on a child entity. `propagate_parent_transforms`'s roots query is
`Without<ChildOf>` WITH `&Children`
(`bevy_transform-0.19.0/src/systems.rs:508-517`), while the cheap
`sync_simple_transforms` requires `Without<Children>` (`:42-50`). So all 1,616
live rounds at the `stress_bullets` peak enter the expensive system. Their
tracers also take an unconditional Transform write every frame
(`render.rs:453,460`).

Rounds add no work-queue tasks - the tracer is a leaf, and
`propagate_descendants_unchecked` enqueues only children that themselves have
children (`systems.rs:722-726`). So round volume is not what created the spin.

Change: move `Mesh3d`/`MeshMaterial3d`/`RoundTracer` onto the round entity.
One thing blocks it: the back-slide at `render.rs:455-459` exists because the
art mesh is centred on the round. Authoring the mesh nose-at-origin makes
`scale.z` alone correct. `advance_rounds` reads translation only
(`crates/nova_gameplay/src/rounds.rs:352-358`) and the round carries no
collider, so scaling it cannot change what it hits.

Pre-existing (`c92dbf0f9`), amplified 3.27x by `6dadd95f2`.

## Finding 7 - MINOR - four new full-section scans per fixed step

`crates/nova_ship/src/sections/mod.rs:236-259` registers `publish_hull_radii`
(`hull_radius.rs:188`), `publish_ship_signatures` (`signature.rs:158`),
`publish_target_hit_radii` (`hull_radius.rs:90`), `publish_integrity_envelopes`
(`hull_radius.rs:136`) and `publish_flight_authority`, all in `FixedUpdate`,
all zero at v0.13.2. About 144 us per rendered frame in a light window.

All of them write through `set_if_neq`/`try_insert` (`hull_radius.rs:155,255,
265`; `signature.rs:196`), so none dirties a Transform, and the
`Local<EntityHashMap>` at `hull_radius.rs:190` avoids a per-tick allocation.
Listed for completeness, not as a defect.

NEW in range: `f894d4c20`, extended by `6dadd95f2` and `f89ce5ef6`.

## Finding 8 - MINOR - unconditional Transform writes on joints and camera

`crates/nova_ship/src/sections/turret_section/aim.rs:622` rewrites every
turret hinge each tick with no compare. A `Changed<>` filter would not help:
the producer at
`crates/nova_gameplay/src/transform/smooth_look_rotation.rs:116-143` also
writes through `DerefMut` unconditionally. Both sites need a value compare.

`crates/nova_gameplay/src/shake.rs:257,271` and
`crates/nova_ship/src/camera/chase.rs:225` dirty the camera subtree three
times per frame at zero trauma.

Neither buys anything in combat - see Finding 3's rejected alternative. They
matter only for menus, backdrops and parked scenes, where the propagation
early-return at `bevy_transform-0.19.0/src/systems.rs:561-568` could fire.

Pre-existing. Not in range.

## Cap feasibility, if it is shipped separately

`bevy_app-0.19.0/src/task_pool_plugin.rs:71-92,113-148,161-256`. IO and async
are allocated first (`percent 0.25, min 1, max 4` each); compute takes what is
left (`percent 1.0, min 1, max usize::MAX`). `max_threads = 8` resolves to
`min(remaining, 8)`.

| Logical CPUs | compute default | compute with max=8 |
|---:|---:|---:|
| 4 | 2 | 2 |
| 8 | 4 | 4 |
| 16 | 8 | 8 |
| 24 | 16 | 8 |
| 32 | 24 | 8 |
| 64 | 56 | 8 |

It binds only at 17+ logical CPUs and can never make a smaller host worse.
It is a no-op on CI runners and a no-op on wasm, where `bevy_tasks` always
uses the single-threaded pool (`bevy_tasks-0.19.0/src/lib.rs:22-24,92-100`;
`single_threaded_task_pool.rs:51` ignores `num_threads`).

Nova draws on the compute pool nowhere directly: zero `par_iter`, zero
`ComputeTaskPool` across `crates/` and `examples/`. Nova's own pool use is
`AsyncComputeTaskPool` for asteroid carving
(`crates/nova_scenario/src/objects/asteroid_carve.rs:598,690`) and
`IoTaskPool` for mod install and settings
(`crates/nova_assets/src/portal/install.rs:576,645`,
`crates/nova_assets/src/mod_set.rs:344`). Neither is affected.

Two hazards if it ships:

- `create_default_pools` uses `get_or_init` (`task_pool_plugin.rs:177,207,
  237`). The FIRST initializer in the process wins, silently. Any test or
  plugin group that touches a pool before `DefaultPlugins` - such as
  `crates/nova_scenario/src/loader/lifecycle.rs:1925` - makes the cap a no-op
  with no error. The assertion must read the ACTUAL pool, not the options.
- It is not settable from `PersistedSettings`. Those are applied by a startup
  system (`crates/nova_menu/src/settings_store.rs:410-415`), long after
  `TaskPoolPlugin::build`. An override would have to be launch-time.

Shape: `max_threads = 8` and `min(available, 8)` are the same expression under
Bevy's clamp, so that alternative buys nothing. A ratio policy would need a
second host to distinguish. On this i9-12900F, 8 is simultaneously the P-core
count, half the default allocation, and a third of the logical CPUs; the
three-point curve cannot separate those policies.

Unmeasured under the cap: headless, editor, menu, `probe` throughput, any
second host, any non-hybrid CPU. `docs/performance.md:257` would need a
sibling paragraph, because the cap's mechanism is the opposite lesson: the
fixed loop's EXECUTOR is serial while the systems inside it still fan out.

## Ranked next work

1. A step-matched, simulated-time-bounded 4v4 fixture with asserted live
   projectile, ship, section and wreck populations. Every comparison in this
   task is frame-bounded, so the faster arm simulates less world. This is
   Required investigation item 2 and it gates honest attribution of the
   remaining ~4.4 ms.
2. The Finding 3 unit test. No GPU, no measurement slot. It kills the naive
   flag flip and proves or kills the root-sync variant in one file.
3. Finding 4's sever-path collider assertion, then a wreck and collider census
   during a fight - the control `STRESS.md` names as missing.
4. Finding 6, measured on `stress_bullets`, which is the fixture where round
   roots dominate and hull depth does not.

## Verification and limits

- 45 release captures, 1920x1080, default quality, Vulkan, display `:0`,
  `--features debug`, draft `--seed 20260816` and gameplay
  `NOVA_SEED=20260816`, 60 warmup / 360 capture frames. Zero ERROR lines,
  zero capture refusals, zero refresh-cap suspicion. Same binary per revision
  serves both arms; pool counts asserted in every run log.
- `examples/playable/wfc_arena.rs`, `crates/nova_probe/src/capabilities/
  frametime.rs` and `Cargo.lock` are byte-identical across all three
  revisions. Bevy 0.19.0 and avian3d 0.7.0 throughout.
- Fired rounds at the third simulation-clock report are comparable within each
  revision (v0.13.2 1327-1406 at 16w against 1316-1384 at 8w; HEAD 3270-3488
  against 3498-3648). The cap does not win by suppressing fire. Four roots per
  team in all 45 runs; roots do not prove intact controllers.
- Unequal fixed steps remain. The single-fixed-step frame bucket is the
  amplifier-free control: v0.13.2 13.0842 -> 9.2001 ms at the cap, HEAD
  16.4256 -> 13.5510 ms. The regression on that bucket is +47.3%, about
  4.35 ms. Zero-step frames remain in every arm.
- The v0.13.2 ceiling arm is not workload-matched: at ~7.5 ms frames all five
  8-worker runs ended before the third status report. Direction only.
- One host, one GPU, one display. Other agent sessions shared the box;
  loadavg 0.94-3.49, gated below 3.5, arm order rotated.
- One rendered 1920x1080 v0.13.2 eight-worker combat frame inspected
  (`b3c6f579c-stock-8-98/combat.png`), marked visual-only and excluded from
  every set.
- Capture-only. No correctness recording, no new `checks.json` verdict at any
  revision. No full workspace tests, no Clippy.
- Trace aggregates are dev-profile spans from light-combat windows. Call
  counts and per-call means are what this review relies on; absolute
  milliseconds are not release frame time.
- Static findings marked UNTESTED are code-only. The no-`ChildOf` precondition
  in Finding 3 and the wreck census in Finding 4 are the two unproven points.

## Artifacts

Outside Git, disposable: `/tmp/nova-perf-20260913/cap-baseline/` (334 MB) with
per-run manifests, `frametime.csv`, census, logs, both patched source copies,
both new binaries, `summary.json` and the helper scripts. Earlier sets remain
under `/tmp/nova-perf-20260913/` and `/tmp/nova-release-perf/`.

Profiling sysctls are still `kernel.perf_event_paranoid=1` and
`kernel.perf_event_mlock_kb=16384`. Restoring them needs the user's password:

```sh
sudo sysctl -w kernel.perf_event_paranoid=2 kernel.perf_event_mlock_kb=516
```
