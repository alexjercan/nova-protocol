# The cost list, measured

Every number below came out of `probe scenario` / `probe run` on one machine,
one build, one capture window. Read the ranking first; the reasoning is under
it. Nothing here is graded: probe reports the frames and a human decides.

## How it was measured

```sh
cargo run --features debug probe scenario editor_sandbox
cargo run --features debug probe scenario broadside
cargo run --features debug probe run wfc_arena          # fields 4v4 under the probe
cargo run --features debug probe run carve_asteroids    # sustained PDC into rock
```

- Host: NVIDIA GeForce RTX 3060 Ti, vulkan, nixos, throwaway Xvfb, 1280x720.
- Build: `dev` profile (first-party `opt-level = 1`, dependencies 3). A dev
  build is not a shipping build; these numbers RANK, they do not certify.
- Window: the standard 180 warm-up frames + 900 captured, so the load spike and
  the first shader compiles are outside every number in the table.
- Frame numbers come from the dedicated fps pass. System numbers come from the
  separate traced pass, which is throttled by tracing - the two are never
  compared with each other, only within themselves.
- Tree: `223ed486` plus this task's harness changes. Nothing here adds or moves
  a gameplay system.

## THE RANKING

Worst frame first, because a stutter is a tail.

| # | case | worst frame | worst FPS | 1% low FPS | mean frame | mean FPS |
|--:|------|------------:|----------:|-----------:|-----------:|---------:|
| 1 | `wfc_arena` 4v4 | **295.76 ms** | **3.4** | 5.1 | 93.08 ms | 10.7 |
| 2 | `broadside` (shipped chapter) | 52.92 ms | 18.9 | 23.6 | 26.25 ms | 38.1 |
| 3 | `editor_sandbox` | 42.38 ms | 23.6 | 27.1 | 26.30 ms | 38.0 |
| 4 | `carve_asteroids` (firing) | 42.39 ms | 23.6 | 34.7 | 21.09 ms | 47.4 |

`wfc_arena` 4v4 is not in the same bracket as the rest. It is 5.6x the mean
frame of anything else and its worst frame is seven times the worst frame of
the two scenarios. Everything else in this list sits in one band around 26 ms
mean / ~40-50 ms worst.

**Run-to-run spread on the tail.** `editor_sandbox` was measured twice with an
unchanged scene: worst 57.87 ms then 42.38 ms, 1% low 22.8 then 27.1 FPS.
`broadside`: 58.38 then 52.92 ms. So a worst-frame difference under ~30% is
noise on this host, and a claim smaller than that needs repeats. The means were
stable to 0.5%.

## Where the cost is

Top spans of the traced run, by total span time. Shares OVERLAP (a parent span
counts its children) so they must not be summed; they rank.

### 1. `wfc_arena` 4v4 - 295.76 ms worst frame

| share | span | mean ms/call |
|------:|------|-------------:|
| 27.5% | `bevy_render::run_render_schedule` | 112.5 |
| 12.9% | `bevy_render::renderer::render_system` | 52.9 |
| 11.3% | `bevy_time::fixed::run_fixed_main_schedule` | 45.8 |
| 7.5% | `avian3d::schedule::run_physics_schedule` | 4.1 |
| 5.9% | `write_binned_instance_buffers<Opaque3d, MeshPipeline>` | 24.0 |
| 3.9% | `bevy_pbr::render::gpu_preprocess::prepare_preprocess_bind_groups` | 16.1 |
| 3.1% | `avian3d::...::run_substep_schedule` | 1.7 |

Worst SINGLE call of anything in that run, load-time spans excluded:

| worst call | span |
|-----------:|------|
| 188.77 ms | `bevy_render::run_render_schedule` |
| 138.43 ms | `bevy_time::fixed::run_fixed_main_schedule` |
| 90.57 ms | `avian3d::schedule::run_physics_schedule` |
| 68.09 ms | `PipelineCache::process_pipeline_queue_system` |
| 66.48 ms | `avian3d::...::update_solver_body_angular_inertia` |
| 65.98 ms | `write_binned_instance_buffers<Opaque3d, MeshPipeline>` |
| 64.91 ms | `avian3d::dynamics::integrator::integrate_positions` |
| **59.87 ms** | `avian3d::...::collect_collision_pairs<nova_gameplay::projectile_hooks::ProjectileHooks>` |

The frame is split between the render schedule and the avian solver. The
biggest Nova-attributable single call in the whole run is the broad phase over
projectile colliders at 59.87 ms - and no Nova SYSTEM of our own reaches 21 ms:

| worst call | Nova system |
|-----------:|-------------|
| 20.77 ms | `nova_ship::sections::shell_skin::spawn_ship_skin` (commands, at spawn) |
| 20.10 ms | `NovaEventWorld::state_to_world_system` |
| 17.16 ms | `nova_ship::...::torpedo_detonate_system` |
| 1.66 ms | `nova_scenario::objects::asteroid_carve::collect_asteroid_remeshes` (commands) |

One more Nova cost hides under a bevy name: `prepare_erased_assets` for the
`ThrusterExhaustMaterial` extended material, 673 ms over 114 frames (5.9
ms/call, 1.5% share) - the fifth-largest non-schedule row. Every ship's thruster
material is re-prepared every frame.

### 2. `broadside` - 52.92 ms worst frame

| share | span | mean ms/call |
|------:|------|-------------:|
| 31.7% | `bevy_render::run_render_schedule` | 21.7 |
| 19.0% | `bevy_render::renderer::render_system` | 13.1 |
| 5.1% | `bevy_time::fixed::run_fixed_main_schedule` | 3.5 |
| 3.3% | `avian3d::schedule::run_physics_schedule` | 1.4 |
| 2.6% | `NovaEventWorld::state_to_world_system` | 1.8 |

### 3. `editor_sandbox` - 42.38 ms worst frame

| share | span | mean ms/call |
|------:|------|-------------:|
| 30.4% | `bevy_render::run_render_schedule` | 20.5 |
| 19.4% | `bevy_render::renderer::render_system` | 13.1 |
| **5.5%** | `NovaEventWorld::state_to_world_system` | **3.68** |
| 4.9% | `bevy_time::fixed::run_fixed_main_schedule` | 3.3 |
| 2.9% | `avian3d::schedule::run_physics_schedule` | 1.3 |

`state_to_world_system` is twice as expensive here as in `broadside` (3.68 vs
1.81 ms/call) for a scene with fewer live combatants. That is the scenario
engine re-evaluating the sandbox's standing objective and three dormant-picket
proximity trips every frame, and it is the only place a Nova system is in the
top three of anything.

### 4. `carve_asteroids` while firing - 42.39 ms worst frame

The one case that actually carves. The whole `asteroid_carve` path over 755
traced frames of sustained PDC fire:

| total ms | worst call | span |
|---------:|-----------:|------|
| 52.36 | 1.69 | `collect_asteroid_remeshes` (commands) |
| 23.32 | **19.24** | `collect_asteroid_remeshes` |
| 9.47 | 0.50 | `carve_asteroid_fields` |
| 2.42 | 0.23 | `collect_asteroid_field_seeds` |
| 2.30 | 0.02 | `seed_asteroid_fields` |
| 2.58 | 0.08 | the three matching command flushes |

**92.48 ms over 755 frames - 0.12 ms per frame on average, and one frame that
was not average.** Exactly ONE call of `collect_asteroid_remeshes` cost 19.24
ms; the other 754 cost 0.04 ms or less. The seed and the remesh themselves run
on `AsyncComputeTaskPool` and never appear on the main thread. What is left is
the APPLY - taking a finished worker result and putting the mesh and the
collider on the body - and that is a real, rare, whole-frame spike: 19.24 ms is
roughly half the 42.39 ms worst frame this run measured.

## What this means for the blocked tasks

### `20260818-221031` - worker + placeholder (p85)

**Its cost list is stale in all four entries.** Every caller it names has
moved off the main thread, shrunk to nothing, or been deleted:

1. "Asteroid field seed / remesh / collider rebuild - 12.7 + 10.7 + 10.0 ms at
   64^3." The grid is 40^3 and the seed and remesh are already on
   `AsyncComputeTaskPool`; neither appears on the main thread at all. The whole
   remaining path costs 0.12 ms/frame under sustained fire.
2. Section solidify - already struck in the task itself.
3. `FinaleQueue` - deleted. `nova_gameplay::integrity::explode` totals 5.65 ms
   over 1997 calls in the 4v4 (0.003 ms/call).
4. "The mesh slicer - 33.1 ms, the largest single item in the measured death
   frame" - deleted.

**One caller survives, and it is not the one the task names.** The APPLY step,
`asteroid_carve::collect_asteroid_remeshes`, spiked to **19.24 ms in a single
frame** against a 0.02 ms median - half a measured worst frame. Handing
the compute to a worker moved the compute; putting the result on the body is
still a main-thread event, and it is the one that is left.

So the task is not dead, it is MIS-AIMED. Three of its four callers are gone
and the fourth has moved; what it should be about is the swap-in - applying a
mesh and rebuilding a collider without owning the frame it lands in. That is
one caller, not a shared abstraction, and at p85 it is over-ranked for one
19 ms spike in a 755-frame run. **Recommend: rewrite around the apply step and
re-rank, or fold into `PERF-REGRESSION`.**

### `20260818-221036` - surface, not volume (p80)

**Not justified on frame rate.** The claim ("`SignedField` scans every cell of
a `count^3` grid") is still true of the algorithm, but the cost it was ranked
on is no longer on the main thread and no longer at 64^3. Carving a rock costs
0.12 ms/frame of main-thread time, and the one spike that IS a frame is the
apply step, which a narrower field would not shrink - a smaller band produces
the same mesh and the same collider.

There is a real remaining claim, and it is a different one: a narrow band would
cut how LONG a remesh takes on the worker, which is how long a shot rock wears
its placeholder, and how much memory a field holds. That is a latency and
footprint argument, not a frame-rate one, and the task's own "Done when" - "no
frame in the `PERF-HARNESS` cases owns a computation" - is already satisfied
without it. **Recommend: re-scope to worker latency, or close.**

## What nobody has named yet

The measurement's real finding is that the release's headline case is not bound
by any of the CPU work the epic listed.

1. **The 4v4 frame belongs to the RENDER SCHEDULE.** 27.5% of the traced run,
   112 ms/call mean, 188 ms worst. Under it, `write_binned_instance_buffers`
   costs 24 ms/call and `prepare_preprocess_bind_groups` 16 ms/call - both
   scale with the number of binned mesh entities. Eight clad, greebled WFC
   hulls put an enormous number of distinct meshes and materials in front of
   the camera. Nothing on the tracker is about this.
2. **The avian solver is the second owner.** `run_physics_schedule` 90.57 ms
   worst, `update_solver_body_angular_inertia` 66.48 ms and
   `integrate_positions` 64.91 ms as single calls. Hundreds of projectiles and
   eight multi-section hulls, all dynamic.
3. **The projectile broad phase is the biggest Nova-attributable spike.**
   `collect_collision_pairs<ProjectileHooks>` peaks at 59.87 ms in one frame
   (618 ms over 833 calls). A round is a physics body; a thousand rounds is a
   thousand bodies in the BVH.
4. **`ThrusterExhaustMaterial` is re-prepared every frame** for every thruster
   on the field - 5.9 ms/call in the 4v4, more than any Nova system of ours.
5. **`process_pipeline_queue_system` spikes to 68 ms mid-run.** That is
   `synchronous_pipeline_compilation = true` (kept deliberately - it fixes an
   exit-time SIGSEGV), paying for a shader the warm-up did not touch. Any
   "never block the main thread" rule has to say something about this one,
   because it is a deliberate block.

If the release wants the 4v4 to hold frame rate, the work is in 1-3, not in the
two tasks that were ranked against the old list.

## Two things that were assumed and are not true

- **`asteroid_field` does not exist.** The epic's definition of done names "the
  `asteroid_field` sandbox", and commit `d20a37c4` ("Drop the Asteroid Field
  sandbox and its relay") deleted the scenario, its builder, its bundle entry
  and its thumbnail earlier the same day. `editor_sandbox` is the rock sandbox
  now, and `carve_asteroids` is the rock-destruction bench. Both are measured
  above; the epic's DoD needs rewording against what ships.
- **`editor_sandbox` is not at 2 FPS.** It measures 38.0 FPS mean, 23.6 FPS
  worst frame - the same band as the shipped `broadside` chapter. Whatever the
  2 FPS report was against, this tree is not it, and it is now reachable by id
  so the claim can be re-checked any time with one command.

## Harness findings from doing this

- **The profiled pass never traced the game binary.** The root package's `bevy`
  entry is a dev-dependency, so `trace = ["bevy/trace", ...]` did not reach a
  `cargo build --bin`: the first two scenario runs produced no `trace.json` at
  all and the report said so. The feature now lives on `nova_core`. Examples
  were never affected, which is why it went unnoticed.
- **The worst single spans of every run are load-time** (cubemap decode ~280
  ms, `RenderPlugin` build ~230 ms, the first `GpuImage` extract/prepare ~115
  ms). The 180-frame warm-up keeps them out of the frame numbers, which is
  correct - but it means the frame table says nothing about load, and the
  epic's rule is about the main thread being blocked OUTSIDE a loading screen.
  A separate load-time measurement is still owed.
