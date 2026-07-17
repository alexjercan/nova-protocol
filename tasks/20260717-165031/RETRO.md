# Retro: author-configurable orbit-hold and lock-refire durations

- TASK: 20260717-165031
- BRANCH: feature/configurable-event-durations
- REVIEW ROUNDS: 1 (APPROVE)

Process notes only; what/why/evidence in TASK.md + NOTES.md, findings in
REVIEW.md.

## What went well

- Planned from the actual data model, not a guess: a read-only investigation
  mapped every hop (config -> component -> tracker) and the RON authoring shape
  before any code, so the plan named exact structs and line numbers.
- Pivoted design mid-implementation when the evidence changed the calculus: the
  planned `ScenarioConfig.lock_refire_secs` would have broken ~15 struct literals
  across 5 crates, so I moved it to `PlayerControllerConfig` (symmetric with the
  orbit side, far fewer sites) and updated TASK.md's Goal/Design/Steps to match
  reality instead of silently diverging.
- Compounded last cycle's retro: sprouted FIRST (before `tatr new`) and ran the
  crate tests with `--features serde` from the very first run - so this time I did
  NOT pay the `crate-solo-tests-miss-unified-features` cold-compile penalty.
- Doubled the review against the shared-session blind spot (self-pass + an
  out-of-context reviewer agent); both confirmed default-parity and threading.

## What went wrong

- **Landed a broken master.** Root cause: I validated with crate-scoped commands
  (`grep ... crates/`, `cargo check -p nova_scenario -p nova_assets -p nova_editor`,
  `cargo test -p nova_scenario`) that compile neither `examples/` nor, in the case
  of `cargo check`, any `#[cfg(test)]` code. Adding the non-Default
  `lock_refire_secs` field broke 8 `examples/*.rs` `PlayerControllerConfig`
  literals that no pre-land check touched; the break surfaced only from editor
  diagnostics AFTER the squash-merge. This is the ledger's
  `check-all-targets-for-struct-field` (now x3) - I even fixed a `#[cfg(test)]`
  literal earlier in the SAME task (caught by `cargo test`) and still did not
  generalize to "grep the whole repo + `--all-targets`" before landing.
- Minor: the same struct-field-breaks-literals hazard bit twice in one task (the
  test-only literal, then the examples) - one `--all-targets` pass up front would
  have caught both at once.

## What to improve next time

- When a change adds a field to a widely-constructed struct (or otherwise can
  break exhaustive literals), the pre-land gate is `grep -rn "<Type> {" .` over the
  WHOLE repo AND `cargo check --all-targets` - BEFORE the squash-merge, not after.
  Crate-scoped `-p` checks are for fast iteration only, never the landing gate.
- Treat "cargo check passed" as necessary-not-sufficient for a struct change;
  examples and tests are separate compile targets.

## Action items

- [x] Fixed the 8 broken examples (commit 67a63ca4) and verified with
      `cargo check --all-targets` (clean).
- [x] Bumped `check-all-targets-for-struct-field` to x3 and moved it into the
      ledger's Pending promotions (candidate for an AGENTS.md landing-gate rule).
