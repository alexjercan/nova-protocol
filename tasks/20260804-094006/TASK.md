# Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

- PRIORITY: 78
- TAGS: v0.10.0, content, examples, testing, perf
- KIND: STORY
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-093950

## Story

Create `stress/`: the one category that carries frame-time windows. It absorbs
`perf/` and adds scale sweeps that prove both "nothing breaks at scale" and a
frame-time number.

Per the roster spike (`20260804-003244`), no other category runs fps - which is
what frees `sections/`, `systems/` and `ui/` to be short and assertion-dense
rather than padded to fill a window.

## Steps

- [ ] Move `examples/perf/perf_baseline.rs` -> `examples/stress/scene_baseline.rs`,
      atomic with its `tests/examples_smoke.rs` and catalog edits; delete the
      `perf/` directory.
- [ ] Add `stress/many_bodies`: N asteroids under physics + gravity + render,
      with a count knob and a declared `loop_from` point.
- [ ] Add `stress/many_sections`: one ship with N sections, reusing the
      `sections/` ship builder.
- [ ] Add `stress/many_projectiles`: turret + torpedo saturation.
- [ ] Delete `fps_exempt` from `Cargo.toml` now that category policy owns it.

## Definition of Done

- `stress/` runs fill a frame-time window without per-example loop plumbing.
  (cmd: `nix develop --command cargo run -p nova_probe -- run stress --fps`)
- Each scale run makes a correctness claim too: no panic, and entity counts
  return to baseline after teardown.
  (cmd: `nix develop --command cargo run -p nova_probe -- run stress`)
- `perf/` and `fps_exempt` are gone from the tree.
  (cmd: `! rg -n 'fps_exempt|examples/perf' Cargo.toml crates tests`)
- The catalog, disk and smoke lists agree after the move.
  (test: `catalog_matches_disk`)

## Notes

The roster:

| Run | Change |
| --- | --- |
| `perf_baseline` -> `stress/scene_baseline` | MOVE from `perf/`. Loads a shipped SANDBOX scenario (`asteroid_field` via `NOVA_PERF_SCENARIO`, not story), and stays the release-over-release comparable number. |
| `stress/many_bodies` | NEW. N asteroids under physics + gravity + render. |
| `stress/many_sections` | NEW. One ship with N sections: mass/COM aggregation and the integrity graph at scale. Reuses the `sections/` ship builder with a count knob. |
| `stress/many_projectiles` | NEW. Turret + torpedo saturation: collision, particles, despawn churn. |

- Each scale run takes a count knob and a declared `loop_from` point, so the
  fps window is filled by repeated ACTIVITY - spawn the swarm, run it, tear it
  down, loop - rather than by idling.
- Every scale run makes a CORRECTNESS claim too, not just a number: nothing
  panics, nothing desyncs, entity counts return to baseline after teardown.
- Pick each count default so the window fills on the CI box under llvmpipe, and
  record the chosen value and why.
- `fps_exempt` disappears entirely: `stress/` runs fps and nothing else does,
  so the per-category run policy replaces the hand-listed exemption.
- Examples must be RUN under Xvfb :99, not only checked.
