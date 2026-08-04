# Review: Create stress/: absorb perf_baseline and add the many-bodies, many-sections, many-projectiles sweeps

- TASK: 20260804-094006
- BRANCH: examples/stress-category

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (MINOR) examples/stress/many_bodies.rs:186 - `swarm_count()` returns
  the raw env value while both siblings clamp (`many_sections.rs:187`,
  `many_projectiles.rs:246` end `.max(1)` with a one-line rationale).
  `NOVA_STRESS_COUNT=0` therefore makes `many_bodies` spawn nothing, satisfy
  `swarm_is_up(0)` instantly and pass its returned-to-baseline assertion
  vacuously - under the same variable the docs say scales all three together.
  Append `.max(1)` after `.unwrap_or(DEFAULT_COUNT)` with a matching comment.
  - Response:
- [ ] R1.2 (NIT) crates/nova_probe/src/bin/probe/native/env.rs:66 - the doc
  comment still reads "This replaced a short non-`perf/` window", naming the
  category this diff deletes; the diff already retouched this file at :106.
  Reword to "non-frame-time window".
  - Response:
- [ ] R1.3 (NIT) crates/nova_probe/src/fixtures.rs:1 - the new
  `nova_probe::fixtures` shares its name with the crate's existing
  `crates/nova_probe/src/bin/probe/native/fixtures.rs` (probe's synthetic
  catalog), which this same commit edits. Add one line to the module doc
  distinguishing it from the bin-side `fixtures`, or rename to
  `scenario_fixtures`.
  - Response:
- [ ] R1.4 (NIT) examples/stress/many_projectiles.rs:307 - `target_position` is
  body-identical to `many_bodies.rs:207 rock_position` (same Fibonacci-sphere
  maths, only the radius constant differs). Add a one-line comment at both
  sites stating the duplication is deliberate example self-containment, or give
  `fixtures` a `fibonacci_shell(i, count, radius) -> Vec3`.
  - Response:

Verification, re-run in-session on top of the out-of-context reviewer's pass:

- All nine `tatr proofs` pass. `probe run stress` -> aggregate OK, 5/6 per run
  (`fps_within_baseline` SKIPPED, no baseline). `probe run stress --fps` ->
  aggregate OK, 6/6 per run, all four captures filled the 900-frame window.
  `stress_reach_playing_without_panic` ok (65 s), `catalog_matches_disk` ok,
  `every_category_has_a_probe_policy` ok, `nova_probe --lib fixtures::` 3
  passed, both `rg` proofs empty, `rg -l 'nova_probe::fixtures' examples` -> 6.
- Re-derived independently: the `fixtures::ship` / `asteroid` /
  `spawn_on_start` extraction is behavior-preserving at all three refactored
  callers (`allegiance: None`, `Quat::IDENTITY`, empty `modifications`, and the
  `lock_signature` each caller passed are all carried through unchanged), and
  a repo-wide `perf_baseline|examples/perf` sweep outside `CHANGELOG.md` and
  `tasks/` returns nothing.
- The close-out's llvmpipe frame-time tables are not reproducible on either
  reviewing box (different raster), but the shape of the claim - windows fill,
  nothing times out - reproduced on both.
- The teardown-to-zero assertions each carry a paired delivery guard
  (`swarm_is_up` / `structure_is_up` / `field_is_up` gate on the full count
  before teardown, and `MIN_PEAK_ROUNDS` is an explicit dud detector), so none
  of them can pass vacuously at the default count. No existing test was
  weakened; the `cli.rs` / `spec.rs` test edits are pure string renames.
- Every ticked clause of steps 1-6 re-read against the diff and delivered,
  including all seven named `perf_baseline` string sites, all six wiki line
  edits, the `rmdir examples/perf`, and the `NOVA_PERF_*` names and defaults
  preserved verbatim in the `scene_baseline` move.
- Process signal: `fixtures::ship` gained a `controller` parameter and
  `asteroid` a `lock_signature` parameter beyond step 2's literal signature.
  Both are load-bearing (callers pass `Player(...)` vs `None`, and
  `player_path` needs `Some(1000.0)`), so not a YAGNI finding - but the
  close-out records the extraction without noting the deviation.
- Process signal: the reflection already names the `on_enter`-replaces trap as
  deserving a builder-level fix in its own task. It is real, cost a full
  measurement cycle here, and is currently mitigated by three copies of a
  warning comment.
- Out of scope: `Cargo.toml` places the `stress/` blocks after `ui/` and before
  `screenshots/` as step 1 literally directs, which is not the order the
  contract comment at `Cargo.toml:32-52` lists. Pre-existing inconsistency
  between the two, not introduced here.

No open `manual:` proofs, so there are no pending user checks.
