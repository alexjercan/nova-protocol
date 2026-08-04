# Notes: Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

Goal in one line: one category owns frame time, and it fills its window with
repeated ACTIVITY at scale rather than by idling - which is what lets every
other category stop padding.

## What changes

Before: `examples/perf/` holds one run, `perf_baseline` (132 lines). It is the
only example probe gives the full 180/900 capture window to (`env.rs:65`,
`if category == "perf"`), it is in `NOT_SMOKED` because it is not harnessed,
and it is driven entirely by env vars. Everything else that runs `--fps` gets a
short 60/240 window whether or not it has anything moving in it.

After: `examples/stress/` holds four runs. All four carry the full window, all
four fill it with a spawn/run/teardown loop under `loop_from`, and all four
make a correctness claim as well as a number: no panic, and entity counts
return to baseline after teardown. `perf/` is gone.

| Run | Origin | Content |
|-|-|-|
| `scene_baseline` | MOVE from `perf/perf_baseline.rs` | shipped SANDBOX scenario (`asteroid_field` via `NOVA_PERF_SCENARIO`) - the release-over-release comparable number |
| `many_bodies` | NEW | N asteroids under physics + gravity + render |
| `many_sections` | NEW | one ship with N sections: mass/COM aggregation + integrity graph at scale |
| `many_projectiles` | NEW | turret + torpedo saturation: collision, particles, despawn churn |

## Surfaces

| File | Why |
|-|-|
| `examples/perf/perf_baseline.rs` -> `examples/stress/scene_baseline.rs` | Move. Uses `nova_probe::{combat_burst_driver, nova_frametime}`; env-driven, `--scenario` is the fallback. `perf/` directory deleted. |
| `examples/stress/many_bodies.rs` | NEW. Reuses the ring builder from `systems/` fixtures with a large count. |
| `examples/stress/many_sections.rs` | NEW. This task EXTRACTS the shared ship builder as the third caller (owner call 2026-08-04) - which is why it depends on both `20260804-093950` and `20260804-093934`: it needs to SEE their inline shapes to design the real signature. Budget the refactor; it is deliberate, not accidental scope. |
| `examples/stress/many_projectiles.rs` | NEW. |
| `Cargo.toml` | `perf/` catalog section becomes `stress/`; three blocks added; `fps_exempt = ["broadside"]` (:34-35) deleted. |
| `tests/examples_smoke.rs` | `perf_baseline` leaves `NOT_SMOKED:78`; a `STRESS` list and `stress_reach_playing_without_panic` appear. Atomic with the move. |
| `crates/nova_probe/src/bin/probe/native/env.rs` | `NON_PERF_WARMUP:17`/`NON_PERF_FRAMES:18` and `resolve_fps_window:65`. Renamed/repointed by 093855; this task supplies the category that makes the branch true. |
| `crates/nova_autopilot/src/autopilot.rs` | `loop_from(name)` (:238) and `on_loop(f)` (:247) - the looping contract each scale run declares. |

## Data and interfaces

Each scale run declares a count and a loop point:

```rust
/// Entities spawned per round. Chosen so the fps window fills on the CI box
/// under llvmpipe; overridable for local sweeps.
const BODIES: usize = /* measured, with the reason recorded */;

nova_autopilot()
    .step("spawn").on_enter(spawn_swarm).until(entity_count_at_least(BODIES)).add()
    .step("run").until(frames(N)).add()
    .step("teardown").on_enter(despawn_swarm).until(entity_count_back_to_baseline()).add()
    .loop_from("spawn")
```

The correctness claim needs a baseline-count predicate that does not exist yet
in `nova_debug::harness` (which today offers `scenario_variable_is`,
`section_gone`, `script_reports_done`, `player_ship_present`). Either a new
Nova predicate or `resource_where`/`any_entity` from `nova_autopilot::predicate`.

## Sketches

Illustrative only.

```diff
-# perf/ - the frame-time baseline rig.
-[[example]]
-name = "perf_baseline"
-path = "examples/perf/perf_baseline.rs"
+# stress/ - scale sweeps; the ONLY category carrying a frame-time window.
+[[example]]
+name = "scene_baseline"
+path = "examples/stress/scene_baseline.rs"
```

```diff
-[package.metadata.nova_probe]
-fps_exempt = ["broadside"]
```

## Shape

```
probe run stress --fps
   |
   +-- category_policy("stress") = { correctness, frame_time, in_all }
   |        |
   |        +--> full 180/900 window (was: `if category == "perf"`)
   v
per run:   spawn N  ->  run  ->  teardown  ->  [loop_from("spawn")]
             |           |           |
             |           |           +-- correctness: counts back to baseline
             |           +-- the frame-time capture measures THIS
             +-- correctness: no panic, no command errors

scene_baseline   many_bodies   many_sections   many_projectiles
  (shipped         (ring         (ship_with_      (turret + torpedo
   sandbox          builder,      sections,        saturation)
   scenario)        systems/)     sections/)
```

## Consequences and open questions

- `fps_exempt` disappears as an INPUT, but `RunManifest.fps_exempt` is a
  serialized checks.json field rendered by `html.rs:181-193`. Whoever deletes
  the Cargo.toml key has to say what happens to the field. Flagged in 093855's
  notes too; it is one decision, not two.
- Whichever of this task and `20260804-093910` lands second deletes the
  Cargo.toml key. That is a merge hazard, not a design question - both should
  expect the other may have done it.
- Count defaults must be chosen against the CI box under llvmpipe, and the
  chosen value AND its reason recorded. A count tuned on the dev box is a
  flaky CI window.
- OPEN: `scene_baseline` keeps loading a shipped scenario (`asteroid_field`).
  That is deliberate - it is the comparable release-over-release number - but
  it does sit against this sprint's "fixtures are code-built" rule. Worth an
  explicit sentence in the task close-out rather than leaving the exception
  looking like an oversight.
- OPEN: `perf_baseline` is in `NOT_SMOKED` today because it is not harnessed.
  Adding a `loop_from` and an autopilot to `scene_baseline` would make it
  smokeable, but a smoke pass over a measurement rig "would only measure noise"
  per the existing comment. Decide: stays NOT_SMOKED, or gets harnessed.
- OPEN: `many_projectiles` has no named source of projectile saturation. Does
  the turret fire on its own under an autopilot, or does the run need a driver
  like `combat_burst_driver` (which `perf_baseline` already imports)? Reading
  that driver first is probably the cheapest planning step here.
