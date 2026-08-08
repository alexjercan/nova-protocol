# Port the harness completion protocol into nova_autopilot

- STATUS: CLOSED
- PRIORITY: 98
- TAGS: v0.10.0, tooling, autopilot

## Story

Port the completion protocol into `nova_autopilot::completion`. Collectors
register at plugin build and report done; a watcher writes `AppExit::Success`
only when the pending set empties, and a deadline backstop error-exits naming
the laggards. Success negotiates; failures abort directly.

## Steps

- [x] Write the five App-driven tests first in
      `crates/nova_autopilot/src/completion.rs` (`#[cfg(test)] mod tests`,
      `MinimalPlugins`, draining `Messages<AppExit>`): negotiated success,
      single-collector parity, deadline error naming the laggards, unknown
      `done`, duplicate registration. Confirm red.
- [x] Port the protocol into the same file: the module doc (protocol rules,
      success-negotiates / failure-aborts), `HarnessCompletion` with
      `done`/`is_pending`/`others_pending`, `register(&mut App, &'static str)`,
      the `Last` `completion_watch` system, and the constants `AUTOPILOT`,
      `SCREENSHOT`, `DEADLINE_ENV`, `DEFAULT_DEADLINE_SECS`.
- [x] Set `DEADLINE_ENV = "NOVA_AUTOPILOT_DEADLINE"`; drop the `BCS_` name with
      no alias. Keep `DEFAULT_DEADLINE_SECS = 120.0`.
- [x] Every public item carries a doc comment - the crate is
      `#![warn(missing_docs)]` and the workspace denies warnings.
- [x] Green the suite and the crate's rustdoc.

## Definition of Done

- Every collector must finish before the app exits successfully.
  (test: `exits_success_only_when_every_collector_is_done`)
- An expired deadline is an error exit that names the pending collectors.
  (test: `deadline_error_exits_naming_the_laggards`)
- The module suite is green with all six tests present, not vacuously green
  on an empty module. (Was five; review round 1 added
  `the_deadline_clock_tracks_wall_time_whatever_the_collector_count`.)
  (cmd: `nix develop --command cargo test --lib -p nova_autopilot completion 2>&1 | rg -q 'test result: ok. 6 passed'`)
- The deadline env is Nova's, with no BCS alias left in the crate.
  (cmd: `rg -q 'NOVA_AUTOPILOT_DEADLINE' crates/nova_autopilot/src/completion.rs && ! rg -n 'BCS_' crates/nova_autopilot/src`)
- Rustdoc is clean.
  (cmd: `nix develop --command env RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`)

## Notes

- Parent: `20260802-120019`. Depends on the crate shell.
- `register` stays public: `nova_probe`'s frame-time capture registers its own
  `capture` collector through it.
- Source: `/home/alex/personal/bevy-common-systems/src/completion.rs`. Both
  crates are on `bevy 0.19.0`, so the `MessageWriter<AppExit>` /
  `Messages<AppExit>` API ports verbatim; no API adaptation expected.
- Type name `HarnessCompletion` is kept as-is. Renaming it is not needed by any
  DoD proof and no sibling task references it; revisit under the docs task
  `20260802-183355` if the crate's vocabulary should shed "harness".
- No `REEL` collector constant yet - the reel joins the protocol in
  `20260802-183349`, which adds its own name there.
- Prelude re-exports stay out of scope; `20260802-183355` owns the prelude.
- The `register` duplicate-`add_systems` NOTE comment was planned as a verbatim
  port, but its claim is false: `exited` makes the EXIT idempotent, not the
  `elapsed` accumulation, so N registrants burned the deadline N times too
  fast. Review round 1 (R1.1) replaced the comment and the behavior; the
  upstream BCS module still carries the bug.

## Close-out

What/why: ported BCS `completion.rs` into `crates/nova_autopilot/src/completion.rs`
verbatim except for the two planned deltas - `DEADLINE_ENV` is now
`NOVA_AUTOPILOT_DEADLINE` (no `BCS_` alias) and the two collector-name constants'
doc comments name the drivers rather than the BCS plugin types that do not exist
here. Tests written first against the empty module, confirmed red (17 unresolved
-name errors), then green after the port.

Alternatives: none material - the plan fixed the type name, the constant set, and
the file layout, and the bevy versions match, so no API adaptation was needed.

Difficulties/diagnosis: `cargo doc -D warnings` failed on intra-doc links that
point at same-module items (`DEADLINE_ENV`, `DEFAULT_DEADLINE_SECS`) and at
`AppExit::error`. Cause: `lib.rs` carried an outer `///` doc on
`pub mod completion;`, which merges with the module's own `//!` block and makes
rustdoc resolve the merged text in `lib.rs`'s scope, where neither the module
constants nor the `bevy::prelude` glob are visible. Dropping that one outer doc
line (the module now documents itself) resolved every link. The sibling empty
modules keep their outer docs until their own ports land.

Evidence: 5/5 tests pass; both `cmd:` proofs green; `cargo fmt --check`,
`cargo clippy -p nova_autopilot --all-targets`, and
`RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps` all clean.

Reflection: the scaffold's placeholder outer module docs are a rustdoc trap for
every remaining port in this epic - each porting task should expect to delete its
`pub mod` doc line as part of moving the module doc into the file.

### Review round 1

What/why: R1.1 - `register` added `completion_watch` once per registrant, so the
watcher ran N times per frame and `elapsed` accumulated N*delta; the 120s
backstop would have expired at 60s with autopilot + screenshot and 40s once
`nova_probe` adds `capture`. Now added only on the first registration, guarded
by the resource's absence. R1.2 - the deadline test asserted only "not
Success", leaving the NAMING clause unpinned; it now captures the `tracing`
output and asserts the laggard appears in it. R1.3 - pushed back with the
rustdoc evidence already recorded above; left a `//` note at `lib.rs:23` so the
constraint is visible where someone would re-add the doc.

Alternatives: for R1.2, asserting `is_pending(SCREENSHOT)` survives to the exit
needs no dev-dependency, but it pins the pending SET, not the log line - the
`{:?}` could still be deleted with tests green. Two dev-only crates already in
the lock file were the cheaper price. For R1.1, tracking a separate
"watcher added" marker resource was rejected: the completion resource's own
absence already answers the question.

Difficulties/diagnosis: the log capture read empty at first. Cause: the schedule
runs systems on task-pool threads and `tracing::subscriber::with_default` is
thread-local, so the watcher's `error!` never reached the sink. Fixed by giving
that one test's `Last` schedule a `SingleThreadedExecutor`.

Evidence: `cargo test --lib -p nova_autopilot completion` is 6/6; both `cmd:`
proofs and `RUSTDOCFLAGS=-Dwarnings cargo doc` green; `cargo fmt --check` and
`cargo check -p nova_autopilot --all-targets` clean. R1.1's test measured
1.9999992x wall time before the fix. R1.2's assertion was mutation-checked by
deleting `{:?}`/`completion.pending` from the `error!` - the test fails.

Reflection: a verbatim-port task is the moment to audit the source, not just
copy it. Both MAJORs were inherited from BCS and the plan actively protected
one of them by calling its false comment load-bearing. The upstream module
still has both; that is worth a fix-forward task against bevy-common-systems.
