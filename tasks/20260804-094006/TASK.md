# Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

- PRIORITY: 76
- TAGS: v0.10.0, content, examples, testing, perf
- KIND: STORY
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855, 20260804-093950, 20260804-093934

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

- [ ] `git mv examples/perf/perf_baseline.rs examples/stress/scene_baseline.rs`
      and `rmdir examples/perf`. Rename the clap `#[command(name)]`, the
      module doc, and the panic message inside it; the run stays a pure move
      otherwise - `NOVA_PERF_SCENARIO` / `--scenario` / `NOVA_PERF_QUALITY` /
      `NOVA_PERF_COMBAT` all keep their names and defaults, so the
      release-over-release number stays comparable (see DECISION.md D4).
- [ ] `Cargo.toml`: replace the `# perf/ - TRANSITIONAL` comment and the
      `perf_baseline` block (:145-152) with a `# stress/` section header
      matching the contract text already at :39-43, and a `scene_baseline`
      block. Place it after `ui/` and before `screenshots/` so catalog order
      matches the contract's listing order.
- [ ] `crates/nova_probe/src/catalog.rs:188-197`: delete the TRANSITIONAL
      `("perf", ...)` row. The `("stress", ...)` row above it is already
      correct (`probed: true, frame_time: true`) - do not touch it.
- [ ] `tests/examples_smoke.rs:80-82`: rename the `NOT_SMOKED` entry and its
      rationale comment `perf_baseline` -> `scene_baseline`. It stays
      unsmoked: probe owns it and a smoke pass would only measure noise.
- [ ] Rename the remaining `perf_baseline` string references. Exact list from
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
- [ ] `web/src/wiki/dev/development.md`: update :152 (the "still carrying a
      policy row until `perf_baseline` lands" sentence is now stale - delete
      it), :204 (`perf/` roster line -> the four `stress/` runs), :231
      (unsmoked list), :552, :564, :566 (the sweep command lines).
- [ ] Confirm `fps_exempt` is already gone from `Cargo.toml` - `20260804-093910`
      removed it, and `rg` on the base tree finds it only in `CHANGELOG.md`.
      If the grep still finds a live one, delete only the manifest key.

### 2. The shared fixture builders

- [ ] Add `crates/nova_probe/src/fixtures.rs` and export it from
      `crates/nova_probe/src/lib.rs` (next to `pub mod invariants;`). Home
      chosen in DECISION.md D1: `examples/` has no shared-module mechanism
      that survives `catalog_matches_disk`'s disk scan, and `nova_probe`
      already depends on `nova-protocol` + `nova_scenario` and is already an
      unconditional dev-dependency of the root package.
- [ ] Seed it from the three visible shapes, not from a guess. Read
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
- [ ] Retarget the three callers onto it, deleting the inline copies and the
      "third caller owns extracting this" doc comment at
      `examples/sections/torpedo_section.rs:226-230`. Three callers is the
      whole justification; do NOT sweep the other four `SpaceshipConfig`
      builders in `examples/` into it in this task.
- [ ] Unit-test the builders in `fixtures.rs` (`#[cfg(test)] mod tests`): a
      built ship carries its sections in order, and `spawn_on_start` wraps
      every object in exactly one `OnStart` event.

### 3. `stress/many_bodies`

- [ ] Add `examples/stress/many_bodies.rs`: N asteroids on
      `fixtures::asteroid`, under physics + gravity + render, loaded via
      `LoadScenario` on `OnEnter(GameAssetsStates::Loaded)` - the same shape
      as `examples/systems/player_path.rs:206-215`.
- [ ] Count knob: `NOVA_STRESS_COUNT`, read once in `main`, defaulting to a
      named `const DEFAULT_COUNT` (DECISION.md D2 - env only, no clap flag).
      Record the chosen default and the llvmpipe measurement that picked it
      in a doc comment on the const.
- [ ] Autopilot script with `.loop_from(LOAD_STEP).on_loop(...)` and
      `nova_probe::capture_reload_end` on the post-reload step, copying
      `examples/systems/player_path.rs:87-200`. Spawn the swarm, hold it,
      tear it down, loop - so `--fps` measures ACTIVITY, not an idle tail.
- [ ] The correctness claim, as an in-example assertion that panics (not a new
      probe API - DECISION.md D3): count the live `AsteroidMarker` entities
      after teardown and before the next spawn, and assert the swarm returned
      to baseline. Push it as a timeline marker too, via the `beat` pattern.
- [ ] Catalog block + a new `STRESS` smoke list and
      `stress_reach_playing_without_panic` test in `tests/examples_smoke.rs`,
      alongside the existing per-category tests.

### 4. `stress/many_sections`

- [ ] Add `examples/stress/many_sections.rs`: ONE ship with N sections, built
      by `fixtures::ship` from an N-long `Vec<SectionSpec>`. This is the
      second consumer of the extracted builder and the reason `ship` takes a
      slice rather than a fixed triple.
- [ ] Same knob + `loop_from` + teardown-to-baseline assertion as step 3, on
      the ship's section entities and its aggregate mass/COM.

### 5. `stress/many_projectiles`

- [ ] Add `examples/stress/many_projectiles.rs`: a turret + torpedo ship on
      `fixtures::ship` firing into a field of `fixtures::asteroid` targets -
      collision, particles and despawn churn under saturation. Reuse the
      `infinite_ammo` player-controller shape from
      `examples/sections/torpedo_section.rs:239-252`.
- [ ] Same knob + `loop_from` + teardown-to-baseline assertion, asserted on
      projectile entities returning to zero after the round.

### 6. Close out

- [ ] Add all three to the `STRESS` smoke list and the catalog.
- [ ] Run every DoD proof below under `nix develop`, with `Xvfb :99` up.

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
