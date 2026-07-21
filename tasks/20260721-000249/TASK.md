# Fix crate-scoped tests workspace-wide via self dev-dep feature (kill the AGENTS incantation)

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.8.0,tooling,testing

## Story

As an agent/human, I want `cargo test -p <crate>` to just work for every
workspace crate, so nobody needs the AGENTS.md feature-unification incantation
(`--features serde` / a unifying sibling / a workspace-wide run) ever again.

Decided by spike 20260719-002512: crates whose tests use feature-gated derives
(`serde`) fail to compile standalone because a solo `-p` run does not unify the
feature in from a sibling. The fix (settled by ledger
`crate-solo-tests-miss-unified-features` x6): give each affected crate a self
dev-dependency that enables its own feature.

## Steps

- [x] Sweep the workspace for the trap: `grep -rln 'cfg(feature\|cfg_attr(feature'
      crates/*/src crates/*/tests` (known: nova_scenario, nova_gameplay,
      nova_core - confirm the full list, do NOT assume only these three).
- [x] For each affected crate, add a self dev-dependency enabling its feature,
      e.g. in `crates/nova_scenario/Cargo.toml`:
      `[dev-dependencies]` `nova_scenario = { path = ".", features = ["serde"] }`.
      Do NOT use `required-features` on the test targets - that SKIPS the tests
      when the feature is off (a plain `cargo test -p X` would silently run
      nothing), which is worse than the current failure.
- [x] Prove each one standalone: `cargo test -p <crate>` compiles and RUNS with
      no extra flags, for every previously-affected crate.
- [x] Run the full `cargo test --workspace` once to confirm nothing else moved
      (the self dev-dep must not change workspace-wide behavior).
- [x] Docs: update the AGENTS.md "Build, run, test" paragraph (drop the
      incantation) and mark the LESSONS.md `crate-solo-tests-miss-unified-features`
      entry FIXED-at-root (like the tatr same-second collision was), plus the
      dev wiki if it repeats the gotcha.

## Definition of Done

- `cargo test -p nova_scenario` (and every previously-affected crate) compiles
  and runs standalone with no feature flag (test: run each).
- `cargo test --workspace` is unchanged.
- AGENTS.md + LESSONS.md + dev wiki no longer tell anyone to add `--features
  serde` for a crate's own tests.

## Notes

- Spike: tasks/20260719-002512/SPIKE.md (the crate-scoped-tests option).
- Independent of the sccache task 20260721-000229 - land in either order.
- The self-dev-dep trick works because dev-dependencies unify into the test
  build, forcing the feature ON for `cargo test` without a sibling.

## Close-out (2026-07-21)

Reproduce-first, as required. Swept `cfg(feature|cfg_attr(feature)` over
crates/*/{src,tests}: three candidates surfaced - nova_scenario, nova_gameplay,
nova_core. Cross-checked each with a solo `cargo test -p <crate> --no-run`:

- **nova_scenario: GENUINELY FAILED.** The loader RON round-trip tests
  (`crates/nova_scenario/src/loader.rs` ~2557+, e.g.
  `thumbnail_and_hidden_default_when_absent_and_round_trip_when_present`) call
  `ron::from_str`/`ron::to_string` on `ScenarioConfig` WITHOUT a
  `#[cfg(feature = "serde")]` gate, but the derives are feature-gated. Solo run
  -> feature off -> missing derives:
  `error[E0277]: the trait bound loader::ScenarioConfig: serde::Serialize is not
  satisfied` (and the matching Deserialize), 4 errors, lib test failed to compile.
- **nova_gameplay: compiles solo, NOT affected.** Its serde test (asset_ref RON
  round-trip) is itself behind `#[cfg(feature = "serde")]`, so with the feature
  off the test code is simply not compiled - no failure. No dep added.
- **nova_core: compiles solo, NOT affected.** Its only `cfg(feature = ...)`
  gates are `debug`, not `serde`; no serde test code. False positive from the
  sweep. No dep added.

Fix: added ONE self dev-dependency to `crates/nova_scenario/Cargo.toml`:
`nova_scenario = { path = ".", features = ["serde"] }` (with a comment). No
`required-features` (would silently skip). Did NOT add needless deps to the two
crates that compile fine solo.

Proof (all via `nix develop --command`, sccache-warm):
- `cargo test -p nova_scenario` AFTER fix: compiles + runs, **131 passed; 0
  failed** (lib) + 1 passed (bench/doc target) + 0. Before: failed to compile.
- `cargo test --workspace`: all green, **0 failed** across every target
  (nova_scenario still 131, unchanged). The self dev-dep did not move
  workspace-wide behavior.
- No new cargo warning from the self-dep; no duplicate-crate-instance issue
  (path points at itself, same version -> cargo treats it as the same package,
  just unifies the feature into the test build).

Docs: AGENTS.md "Build, run, test" paragraph rewritten to say
`cargo test -p <crate>` works standalone now (dropped the
`--features serde` / unifying-sibling incantation). LESSONS.md
`crate-solo-tests-miss-unified-features` marked FIXED-AT-ROOT with the mechanism
and the only-nova_scenario finding. Dev wiki: the sole related passage
(`web/src/wiki/dev/modding-ron.md`) is an accurate architecture note about the
off-by-default serde feature and `cargo build -p nova_scenario` (which builds
fine) - it is NOT the failing-test gotcha, so it was left unchanged. No web/
md touched -> no npm run needed.

Self-reflection: the spike listed all three crates as "affected", but
reproduce-first showed only nova_scenario actually fails standalone - the other
two correctly gate their serde test code behind the feature. Confirming before
fixing avoided adding two needless self-deps. The real trap is specifically an
UNGATED test that uses feature-gated derives; the durable fix is either gate the
test or add the self dev-dep, and this crate had enough serde-dependent tests
that the self dev-dep (feature always-on for tests) is the cleaner choice.
