# Negative result: removing the in-loop propagation pass, 2026-09-13

## Claim

Avian runs a SECOND full transform-hierarchy propagation inside the fixed
loop, on top of Bevy's once-per-frame pass. Removing it is mechanically
sound, correct, and worth almost nothing: the saving moves to `PostUpdate`
instead of disappearing. The change was built, proven, measured, and then
REVERTED. Nothing ships. The code and its tests are preserved outside Git.

This also removes "too many propagation passes" as an explanation for the
post-v0.13.2 regression, and it leaves the compute-worker cap as the only
established lever on transform contention on this host.

## The duplicate pass is real

`avian3d-0.7.0/src/physics_transform/mod.rs:96-105` registers
`mark_dirty_trees + propagate_parent_transforms + sync_simple_transforms`
into `FixedPostUpdate`, behind
`PhysicsTransformConfig::propagate_before_physics`. `impl Default` at `:159`
sets it `true`. Nova never mentions
`PhysicsTransformConfig` anywhere in `crates/`; grep returns zero hits. So
the second pass was a dependency default, not a Nova decision.

Trace call counts confirm the doubling exactly: 659 frames + 754 fixed steps
= 1413 against a recorded 1414 at v0.13.2.

`crates/nova_core/src/lib.rs:867-871` puts the fixed schedules on the
single-threaded executor. That does NOT suppress this: the parallelism is
inside the system body, not in the executor fan-out.

## Flipping the flag alone is a correctness break

Avian's `transform_to_position` is chained directly after the pass, in the
same set, and consumes its output:

```rust
// avian3d-0.7.0/src/physics_transform/mod.rs:184-189
/// To account for hierarchies, transform propagation should be run before
/// this system.
pub fn transform_to_position(
    mut query: Query<(&GlobalTransform, &mut Position, &mut Rotation)>,
```

With the flag off and nothing replacing it, the freshest `GlobalTransform`
is the previous frame's `PostUpdate` value, which holds the EASED render
pose. A unit test measured the result: a body under constant velocity keeps
only **63.7%** of it, because every step hauls `Position` back toward the
eased pose. That is a 36% loss, not an epsilon.

## The change that was built

A `BodyTransformPlugin` in `nova_gameplay` turned the flag off and replaced
the deep walk with a root-body sync:

```rust
// In PhysicsSystems::Prepare,
// .before(PhysicsTransformSystems::TransformToPosition)
Query<
    (&Transform, &mut GlobalTransform),
    (With<RigidBody>, Without<ChildOf>),
>
// global.set_if_neq(GlobalTransform::from(*transform))
```

The deep walk is dead weight in that schedule.
`update_child_collider_position` (avian's
`collision/collider/collider_transform/plugin.rs:62,67-95`)
queries exactly `Without<RigidBody>` child colliders - the ones the root sync
skips - and rewrites `position.0 = rb_pos.0 + rb_rot * collider_transform
.translation` unconditionally, in `PhysicsStepSystems::First`, after prepare
and before the narrow phase. `propagate_collider_transforms`, which maintains
`ColliderTransform`, is registered at `:50-52` in the `Propagate` set but
WITHOUT the flag, so it still runs.

Rounds are unaffected: they are explicitly neither bodies nor colliders and
carry no `Position`
(`crates/nova_ship/src/sections/turret_section/firing.rs:311-319`).

The `Without<ChildOf>` filter is a correctness precondition, not an
optimization. It was asserted on a real cut, not argued from spawn sites:
severing is the one path that re-parents live entities onto a fresh body, and
a test over a real sever confirmed zero parented rigid bodies before and
after.

## The mechanism worked

Traced dev build of the changed tree against an unchanged `7b07528d2`
reference, same aggregator and window rule:

| Span | Changed | Unchanged |
|---|---|---|
| `propagate_parent_transforms` | **1.004** / frame | **2.039** / frame |
| `mark_dirty_trees` | 1.004 / frame | 2.039 / frame |
| `sync_body_global_transforms` | 1.000 / step, **0.004 ms** | absent |
| `transform_to_position` | 1.000 / step | 1.000 / step |

Call counts: 243 against 734. The unchanged pass averaged 1.303 ms.

The removed pass cost 1.303 ms per call. Its replacement costs 0.004 ms.

## The frame did not move

Fifteen release captures, five repeats per arm, all admitted 5/5, no
refresh-cap suspicion, no capture refusal, no ERROR line, pool counts
asserted per run.

| Arm | Mean ms | Median ms | P99 ms | 1-step bucket ms |
|---|---:|---:|---:|---:|
| A unchanged `24cedcf99` @16 | 18.8285 | 16.7673 | 44.3649 | 16.9299 |
| B changed @16 | 18.4728 | 16.0878 | 42.7515 | 15.7833 |
| C changed @8 | 12.2624 | 12.1560 | 25.6751 | 13.0717 |

- B against A: mean **-1.89%**, ranges overlapping. One-fixed-step bucket
  **-6.77%**, 22 of 25 pairs, but A's minimum sits inside B's range and five
  repeats cannot resolve it (Mann-Whitney U=3, two-tailed p approx 0.095).
  P99 -3.64%, ranges overlap completely. B also fired about 6% fewer rounds
  by the third status report, so even that edge is flattered.
- C against B: mean **-33.62%**, bucket **-17.18%**, all four ranges
  DISJOINT, 25 of 25 pairs. The worker cap is NOT made redundant.

## Why the saving is handed back

From the `nova framecost:` schedule tables of the capture runs themselves,
medians over five repeats:

| Schedule | A unchanged | B changed |
|---|---:|---:|
| `RunFixedMainLoop` per fixed step | 8.432 ms | 7.821 ms (-7.2%) |
| `PostUpdate` per frame | 4.027 ms | **4.541 ms (+12.8%)** |
| `Update` per frame | 2.899 ms | 3.055 ms |

The fixed-loop pass was PRE-CLEANING trees that `PostUpdate` then found
clean. Remove it and `PostUpdate` inherits the dirt. At about 1.2 steps per
frame those nearly cancel. `PostUpdate` rose in 24 of 25 pairs.

The work was not redundant. It was SHIFTED. The lever that matters is the
cost PER pass, not the number of passes, and per-pass cost is dominated by
the queue spin that `SAMPLY.md` located. Nova cannot quiet that from its own
side while ships move: `bevy_transform_interpolation-0.5.0` writes
`translation` and `rotation` through `DerefMut` unconditionally in
`FixedFirst` for every interpolated entity, so `mark_dirty_trees` marks every
ship root dirty on every step before any Nova system runs.

## Correctness was clean

Not the reason for rejection. All three probe passes on the changed tree
report OK: every claimed check PASS, zero invariant violations over 334,
467 and 1363 frames, ordnance drained, teardown clean. The two `N/A` entries
per run are the frame-cost checks, correctly not claimed under
`--correctness-only`. Nothing was SKIPPED.

A matched interleaved dev A/B on a quiet host, 12 runs, compared the changed
build against preserved unchanged binaries. Bullet peaks identical pairwise
(1624/1616 both arms). Every point-defence counter overlaps and tracks that
run's frame rate rather than the arm: intercepts 54/48 changed against 62/41
unchanged.

One rendered 1920x1080 combat frame was inspected. Hulls, plating, plumes,
tracer streams, spark clusters, tumbling plates and asteroids all read
correctly. The decisive detail: world-space HUD target markers sit ON their
hulls, and a stale or wrong `GlobalTransform` would offset them.

## Limits

- One host, one GPU, one display, one fixture. i9-12900F, RTX 3060 Ti.
- Five repeats per arm cannot resolve a 7% difference. B against A is a
  point estimate with overlapping ranges, not a measurement.
- The window is frame-bounded, so a faster arm simulates less world. C runs
  33% fewer fixed steps and has 85-96 zero-step frames against A's 10-30.
  Quote C's one-step bucket, not its whole-window mean, for the cap's size.
- No live-population control. The pre-combat census is byte-identical across
  all 15 runs (29,061 entities, 14,099 mesh instances, 214 meshes, 739
  archetypes), but that is taken before combat.
- Mechanism reference is `7b07528d2`, not `24cedcf99`.
- Not measured: `stress_*` release timing under the change, a Samply
  spin-share of the changed build, the fixed-step ceiling, any second host,
  headless, menu, editor or wasm.

## Disposition

REVERTED. The working tree is back to its pre-change state; `cargo check`
passes on `nova_gameplay` and `nova_ship`. Before reverting, the six unit
tests passed, the `nova_ship` sever precondition test passed,
`nova_gameplay --lib` 331 passed, `nova_ship --lib` 962 passed, and
`cargo fmt` was clean.

The rejected change is preserved outside Git, not durable:

- `/tmp/nova-bodytx-20260913/rejected-change/body_transform.rs`
- `/tmp/nova-bodytx-20260913/rejected-change/wiring-and-tests.patch`
- `/tmp/nova-bodytx-20260913/` - captures, traces, correctness runs, visual.

Rebuild it from those two files if a future change makes `PostUpdate` cheap
enough that the fixed-loop saving is no longer handed back. On its own it is
not worth the blast radius on every body's world pose.
