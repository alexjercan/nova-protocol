# Fix crate-scoped tests workspace-wide via self dev-dep feature (kill the AGENTS incantation)

- STATUS: OPEN
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

- [ ] Sweep the workspace for the trap: `grep -rln 'cfg(feature\|cfg_attr(feature'
      crates/*/src crates/*/tests` (known: nova_scenario, nova_gameplay,
      nova_core - confirm the full list, do NOT assume only these three).
- [ ] For each affected crate, add a self dev-dependency enabling its feature,
      e.g. in `crates/nova_scenario/Cargo.toml`:
      `[dev-dependencies]` `nova_scenario = { path = ".", features = ["serde"] }`.
      Do NOT use `required-features` on the test targets - that SKIPS the tests
      when the feature is off (a plain `cargo test -p X` would silently run
      nothing), which is worse than the current failure.
- [ ] Prove each one standalone: `cargo test -p <crate>` compiles and RUNS with
      no extra flags, for every previously-affected crate.
- [ ] Run the full `cargo test --workspace` once to confirm nothing else moved
      (the self dev-dep must not change workspace-wide behavior).
- [ ] Docs: update the AGENTS.md "Build, run, test" paragraph (drop the
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
