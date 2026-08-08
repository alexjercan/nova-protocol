# Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

- STATUS: CLOSED
- PRIORITY: 76
- TAGS: v0.10.0, content, examples, testing, perf

## Story

Create `stress/`: the one category that carries frame-time windows. It absorbs
`perf/` and adds scale sweeps that prove both "nothing breaks at scale" and a
frame-time number.

Per the roster spike (`20260804-003244`), no other category runs fps - which is
what frees `sections/`, `systems/` and `ui/` to be short and assertion-dense
rather than padded to fill a window.

## Steps

Ordered so each numbered group is a clean commit boundary. Every example is
RUN under `Xvfb :99`, never only `cargo check` (memory:
`nix-develop-cargo-and-run-examples`).

### 1. The move: `perf/` -> `stress/`

- [x] `git mv examples/perf/perf_baseline.rs examples/stress/scene_baseline.rs`
      and `rmdir examples/perf`. Rename the clap `#[command(name)]`, the
      module doc, and the panic message inside it; the run stays a pure move
      otherwise - `NOVA_PERF_SCENARIO` / `--scenario` / `NOVA_PERF_QUALITY` /
      `NOVA_PERF_COMBAT` all keep their names and defaults, so the
      release-over-release number stays comparable (see DECISION.md D4).
- [x] `Cargo.toml`: replace the `# perf/ - TRANSITIONAL` comment and the
      `perf_baseline` block (:145-152) with a `# stress/` section header
      matching the contract text already at :39-43, and a `scene_baseline`
      block. Place it after `ui/` and before `screenshots/` so catalog order
      matches the contract's listing order.
- [x] `crates/nova_probe/src/catalog.rs:188-197`: delete the TRANSITIONAL
      `("perf", ...)` row. The `("stress", ...)` row above it is already
      correct (`probed: true, frame_time: true`) - do not touch it.
- [x] `tests/examples_smoke.rs:80-82`: rename the `NOT_SMOKED` entry and its
      rationale comment `perf_baseline` -> `scene_baseline`. It stays
      unsmoked: probe owns it and a smoke pass would only measure noise.
- [x] Rename the remaining `perf_baseline` string references. Exact list from
      `rg -n 'perf_baseline|examples/perf'` on the base tree:
      `crates/nova_probe/src/lib.rs:53` (doc),
      `crates/nova_probe/src/bin/perf_web.rs:13` (doc),
      `crates/nova_probe/src/bin/probe/native/env.rs:106` (doc),
      `crates/nova_probe/src/bin/probe/native/cli.rs:141` (help text),
      `cli.rs:330,344` (test argv + expectation),
      `crates/nova_probe/src/bin/probe/native/spec.rs:276` (the `--all`
      expectation vector),
      `crates/nova_probe/src/bin/probe/native/fixtures.rs:16`
      (`("perf_baseline", "perf")` -> `("scene_baseline", "stress")`; the
      fixture already carries `("many_bodies", "stress")`, so the stress
      category stays covered). Leave `CHANGELOG.md` alone - it is history.
- [x] `web/src/wiki/dev/development.md`: update :152 (the "still carrying a
      policy row until `perf_baseline` lands" sentence is now stale - delete
      it), :204 (`perf/` roster line -> the four `stress/` runs), :231
      (unsmoked list), :552, :564, :566 (the sweep command lines).
- [x] Confirm `fps_exempt` is already gone from `Cargo.toml` - `20260804-093910`
      removed it, and `rg` on the base tree finds it only in `CHANGELOG.md`.
      If the grep still finds a live one, delete only the manifest key.

### 2. The shared fixture builders

- [x] Add `crates/nova_probe/src/fixtures.rs` and export it from
      `crates/nova_probe/src/lib.rs` (next to `pub mod invariants;`). Home
      chosen in DECISION.md D1: `examples/` has no shared-module mechanism
      that survives `catalog_matches_disk`'s disk scan, and `nova_probe`
      already depends on `nova-protocol` + `nova_scenario` and is already an
      unconditional dev-dependency of the root package.
- [x] Seed it from the three visible shapes, not from a guess. Read
      `examples/sections/torpedo_section.rs:231-330` (the shape whose doc
      comment names this task as the extractor),
      `examples/systems/scenario_grammar.rs:262-340` and
      `examples/systems/player_path.rs:230-300`. The union they actually
      share is three builders and nothing more:
      `ship(sections, &[SectionSpec])  -> SpaceshipConfig`,
      `asteroid(assets, id, name, pos, radius, health) -> ScenarioObjectConfig`,
      `spawn_on_start(Vec<ScenarioObjectConfig>) -> Vec<ScenarioEventConfig>`.
      Concept budget: no count knob parameter on `ship` - the count lives in
      the caller's `Vec<SectionSpec>` (DECISION.md D2). Nothing else moves in
      unless a third caller needs it.
- [x] Retarget the three callers onto it, deleting the inline copies and the
      "third caller owns extracting this" doc comment at
      `examples/sections/torpedo_section.rs:226-230`. Three callers is the
      whole justification; do NOT sweep the other four `SpaceshipConfig`
      builders in `examples/` into it in this task.
- [x] Unit-test the builders in `fixtures.rs` (`#[cfg(test)] mod tests`): a
      built ship carries its sections in order, and `spawn_on_start` wraps
      every object in exactly one `OnStart` event.

### 3. `stress/many_bodies`

- [x] Add `examples/stress/many_bodies.rs`: N asteroids on
      `fixtures::asteroid`, under physics + gravity + render, loaded via
      `LoadScenario` on `OnEnter(GameAssetsStates::Loaded)` - the same shape
      as `examples/systems/player_path.rs:206-215`.
- [x] Count knob: `NOVA_STRESS_COUNT`, read once in `main`, defaulting to a
      named `const DEFAULT_COUNT` (DECISION.md D2 - env only, no clap flag).
      Record the chosen default and the llvmpipe measurement that picked it
      in a doc comment on the const.
- [x] Autopilot script with `.loop_from(LOAD_STEP).on_loop(...)` and
      `nova_probe::capture_reload_end` on the post-reload step, copying
      `examples/systems/player_path.rs:87-200`. Spawn the swarm, hold it,
      tear it down, loop - so `--fps` measures ACTIVITY, not an idle tail.
- [x] The correctness claim, as an in-example assertion that panics (not a new
      probe API - DECISION.md D3): count the live `AsteroidMarker` entities
      after teardown and before the next spawn, and assert the swarm returned
      to baseline. Push it as a timeline marker too, via the `beat` pattern.
- [x] Catalog block + a new `STRESS` smoke list and
      `stress_reach_playing_without_panic` test in `tests/examples_smoke.rs`,
      alongside the existing per-category tests.

### 4. `stress/many_sections`

- [x] Add `examples/stress/many_sections.rs`: ONE ship with N sections, built
      by `fixtures::ship` from an N-long `Vec<SectionSpec>`. This is the
      second consumer of the extracted builder and the reason `ship` takes a
      slice rather than a fixed triple.
- [x] Same knob + `loop_from` + teardown-to-baseline assertion as step 3, on
      the ship's section entities and its aggregate mass/COM.

### 5. `stress/many_projectiles`

- [x] Add `examples/stress/many_projectiles.rs`: a turret + torpedo ship on
      `fixtures::ship` firing into a field of `fixtures::asteroid` targets -
      collision, particles and despawn churn under saturation. Reuse the
      `infinite_ammo` player-controller shape from
      `examples/sections/torpedo_section.rs:239-252`.
- [x] Same knob + `loop_from` + teardown-to-baseline assertion, asserted on
      projectile entities returning to zero after the round.

### 6. Close out

- [x] Add all three to the `STRESS` smoke list and the catalog.
- [x] Run every DoD proof below under `nix develop`, with `Xvfb :99` up.

## Definition of Done

- The `stress/` category exists with all four runs and probe expands it.
  (cmd: `nix develop --command cargo run -p nova_probe -- run stress`)
- `stress/` runs fill a frame-time window without per-example loop plumbing.
  (cmd: `nix develop --command cargo run -p nova_probe -- run stress --fps`)
- Each scale run makes a correctness claim too: no panic, and entity counts
  return to baseline after teardown. Its harnessed cycle is a `cargo test`
  gate, not only a probe verdict.
  (test: `stress_reach_playing_without_panic`)
- `perf/` and `fps_exempt` are gone from the tree.
  (cmd: `! rg -n 'fps_exempt|examples/perf' Cargo.toml crates tests`)
- The dev wiki no longer documents a `perf/` category or a `perf_baseline`
  run.
  (cmd: `! rg -n 'examples/perf|perf_baseline' web/src/wiki/dev/development.md`)
- The catalog, disk and smoke lists agree after the move, and every category
  on disk has a policy row.
  (test: `catalog_matches_disk`)
  (test: `every_category_has_a_probe_policy`)
- The extracted builders have one home and three callers - the refactor
  landed, rather than a fourth inline copy.
  (cmd: `test (rg -l 'nova_probe::fixtures' examples | count) -ge 3`)
- The fixture builders are covered where they live.
  (test: `nova_probe::fixtures::tests`)

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

## Close-out

### What and why

`stress/` is now the one category that carries frame-time windows, with four
runs. `perf/perf_baseline` moved in verbatim as `stress/scene_baseline` (only
the clap name, module doc and panic string changed), so the
release-over-release number stays comparable: `NOVA_PERF_SCENARIO`,
`--scenario`, `NOVA_PERF_QUALITY` and `NOVA_PERF_COMBAT` all kept their names
and defaults. The TRANSITIONAL `perf` catalog row and `fps_exempt` are gone -
category policy replaces the hand-listed exemption.

Three new scale sweeps, each with a count knob (`NOVA_STRESS_COUNT`), a
`loop_from` point so `--fps` measures spawn -> hold -> teardown ACTIVITY rather
than an idle tail, and an in-example panicking assertion so each run makes a
CORRECTNESS claim alongside its number:

| Run | Scales | Correctness claim |
| --- | --- | --- |
| `many_bodies` | N independent physics bodies (400) | swarm returns to zero after teardown |
| `many_sections` | one body's structure, N sections (250) | aggregate mass/COM stay finite and positive; sections + root return to zero |
| `many_projectiles` | entity churn rate, N targets (120) | peak rounds in flight clears a dud-detector floor, the volley drains, everything returns to zero |

The three inline `SpaceshipConfig`/asteroid builders that `sections/` and
`systems/` had been copying became `nova_probe::fixtures` (`ship`, `asteroid`,
`spawn_on_start`), with unit tests. Home per DECISION.md D1: `examples/` has no
shared-module mechanism that survives `catalog_matches_disk`'s disk scan, and
`nova_probe` is already an unconditional dev-dependency. Six callers now.

### Alternatives considered

- A new probe check API for the baseline claim, rejected in DECISION.md D3: the
  subject is ECS entity counts inside one example's script, so an in-example
  assertion fails at the point of the leak with the surviving count in the
  message, and gates under `cargo test` rather than only under a probe verdict.
- A clap `--count` flag alongside the env knob (D2). Env only: `probe run
  stress` scales all three sweeps together with one variable and no per-example
  argv plumbing.
- An `examples/support/` shared module, rejected above.
- Sweeping the other four inline `SpaceshipConfig` builders in `examples/` into
  `fixtures`. Out of scope: three callers is what paid for the extraction.

### Difficulties and diagnosis

- **The count knob did nothing.** `many_bodies` first grew its shell radius
  with the cube root of the count, to hold inter-rock spacing constant. That
  keeps DENSITY constant, so the rocks per view frustum and per broad-phase
  cell barely move and the measured cost saturates - 400 and 800 rocks both
  came out ~40 ms/frame under llvmpipe. Pinning `SHELL_RADIUS` puts the count
  back into the number.
- **The fps window could never fill.** Two chained `.on_enter(...)` calls on
  one step silently drop the first: `on_enter` REPLACES the hook rather than
  appending. `capture_reload_end` was the one dropped, so the reload gate
  latched open and every frame after the first loop was excluded from the
  capture. Fixed by having ONE closure do both jobs; the comment is repeated on
  all three sweeps because the trap is invisible at the call site.
- **A silently dud turret.** Pressing fire while the weapons safety is cold
  latches nothing forever (a held key produces no fresh edge once hot). Hence
  the explicit `raise the weapons` step gated on `WeaponsHot`, `cease_fire`
  RELEASING the button so the next loop cycle gets a real edge, and
  `MIN_PEAK_ROUNDS` as a dud detector - without it a turret that fired nothing
  would report an honest-looking fast frame time.
- **The drain is asserted BEFORE teardown**, on purpose: rounds carry
  `TempEntity(projectile_lifetime)` and are not scenario-scoped, so they outlive
  `UnloadScenario`. Unloading first would hide the projectile-lifecycle claim
  rather than prove it.
- **A perf cliff at 240 targets**: software raster falls to 1.3 fps, below the
  2 fps floor probe sizes its completion deadline against, so that count TIMES
  OUT on a CI-shaped box. The default sits well clear; the const doc records it.

### Evidence

All under `nix develop`, `Xvfb :99`, lavapipe software raster.

- `cargo run -p nova_probe -- run stress` -> aggregate OK, 5/6 checks each
  (`fps_within_baseline` SKIPPED: no baseline recorded yet).
- `cargo run -p nova_probe -- run stress --fps` -> aggregate OK, all four
  captures filled the full 900-frame window:

  | Run | frames | mean | p50 | mean fps | wall |
  | --- | ------ | ---- | --- | -------- | ---- |
  | `scene_baseline` | 900 | 97.7 ms | 96.7 ms | 10.2 | 122 s |
  | `many_bodies` | 900 | 37.4 ms | 45.6 ms | 26.7 | 371 s |
  | `many_sections` | 900 | 148.2 ms | 20.1 ms | 6.7 | 182 s |
  | `many_projectiles` | 900 | 56.0 ms | 61.9 ms | 17.8 | 175 s |

- `cargo test --test examples_smoke stress_reach_playing_without_panic` -> ok
  (88 s, all three sweeps).
- `cargo test --test examples_smoke catalog_matches_disk` -> ok.
- `cargo test --test examples_smoke every_category_has_a_probe_policy` -> ok.
- `cargo test -p nova_probe --lib fixtures::` -> 3 passed.
- `rg -n 'fps_exempt|examples/perf' Cargo.toml crates tests` -> no matches.
- `rg -n 'examples/perf|perf_baseline' web/src/wiki/dev/development.md` -> no
  matches.
- `rg -l 'nova_probe::fixtures' examples | wc -l` -> 6 (>= 3).
- Direct autopilot runs of `many_sections` and `many_projectiles` reached
  `Playing`, hit every assertion and exited `cycle complete, no panic`.
- `cargo fmt --check` clean; `cargo check --examples --features debug` clean.

### Reflection

The `on_enter`-replaces-rather-than-appends trap cost a full measurement cycle
and would cost the next author the same - it deserves a builder-level fix
(append, or reject a second call) rather than three copies of a warning
comment. Worth its own task.

Picking the count defaults needed real sweeps, not judgment: the first
`many_bodies` default looked reasonable and measured nothing, and the
`many_projectiles` peak column showed the field density was doing something
different from what the frame time suggested. A throwaway sweep script paid for
itself twice; the tables in the `DEFAULT_COUNT` docs are the durable half of it.
