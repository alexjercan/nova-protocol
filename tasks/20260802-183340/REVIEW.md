# Review: Port the harness completion protocol into nova_autopilot

- TASK: 20260802-183340
- BRANCH: feat/autopilot-completion

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_autopilot/src/completion.rs:135 - `register`
  calls `add_systems(Last, completion_watch)` once per REGISTRANT, and Bevy
  does not dedupe, so with N collectors the watcher runs N times per frame and
  `completion.elapsed += time.delta_secs()` accumulates N*delta. Measured over
  200 frames: two collectors give `elapsed/wall = 2.0000014`, one gives
  `0.99999964`. The 120s deadline therefore expires at 60s wall with autopilot
  + screenshot, and at 40s once `nova_probe` registers its `capture` collector
  (Notes) - a false error-exit that discards the still-running capture, which
  is the exact failure the module doc says the protocol prevents. Add the
  system only on first registration:
  `let first = world.get_resource::<HarnessCompletion>().is_none();` before
  `get_resource_or_insert_with`, then `if first { app.add_systems(...) }`.
  Delete the `// NOTE: once per REGISTRANT is deliberate ...` comment with it -
  `exited` makes the EXIT idempotent, not the elapsed accumulation, so the
  claim is false as written.
  - Response: fixed. `register` now computes
    `let first = world.get_resource::<HarnessCompletion>().is_none();` and only
    adds the watcher when `first`; the false NOTE comment is replaced by one
    stating why once-per-app is required. New test
    `the_deadline_clock_tracks_wall_time_whatever_the_collector_count` pins it
    (red at 1.9999992x before the fix, green after). TASK.md Notes corrected -
    the plan had asked for the verbatim comment.

- [x] R1.2 (MAJOR) crates/nova_autopilot/src/completion.rs:213 -
  `deadline_error_exits_naming_the_laggards` is the DoD proof for "an error
  exit that NAMES the pending collectors", but it only asserts
  `exits[0] != AppExit::Success`. Deleting `{:?}`/`completion.pending` from the
  `error!` call leaves all five tests green, so the naming clause is unpinned.
  Either capture the log (a `tracing` layer collecting the `error!` and
  asserting it contains `"screenshot"`) or, if that is too heavy, assert the
  laggard set survives to the exit -
  `assert!(app.world().resource::<HarnessCompletion>().is_pending(SCREENSHOT))`
  - and rename the test to what it actually checks.
  - Response: fixed with the log capture, not the fallback.
    `tracing`/`tracing-subscriber` are now dev-dependencies; a `LogBuf`
    `MakeWriter` plus `capturing_logs` runs the app under
    `tracing::subscriber::with_default` and the test asserts the captured
    output contains `SCREENSHOT`. The watcher's `Last` schedule is pinned to
    `SingleThreadedExecutor` inside that test because the sink is
    thread-local and task-pool workers do not see it. Mutation-checked:
    deleting `{:?}`/`completion.pending` from the `error!` fails the test with
    `logged: ... still pending`. Name kept - it now describes what it checks.

- [x] R1.3 (NIT) crates/nova_autopilot/src/lib.rs:23 - the outer
  `/// The run-completion protocol.` was dropped from `pub mod completion;`
  while `autopilot` and `reel` kept theirs. The inner `//!` satisfies
  `missing_docs`, but the module list now reads inconsistently in rustdoc.
  Restore the one-line outer doc.
  - Response: pushback - restoring it breaks the DoD's clean-rustdoc proof.
    Tried it; `RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot --no-deps`
    then fails with `unresolved link to AppExit`, `DEADLINE_ENV`,
    `DEFAULT_DEADLINE_SECS`, `AppExit::error`: the outer `///` concatenates
    ahead of the file's `//!` block and the merged docs resolve their
    intra-doc links in `lib.rs`'s scope, where those items do not exist.
    `autopilot` and `reel` have no `//!` block, which is why they are
    unaffected. Left the outer doc off and replaced it with a `//` comment on
    `lib.rs:23` recording the constraint, so the next reader does not re-add
    it. Making the module list uniform would mean moving the siblings to
    `//!` docs, which is a docs-task change, not this branch's.

Verified: all five DoD proofs run and pass on the branch - `cargo test --lib -p
nova_autopilot completion` is `ok. 5 passed`, the `NOVA_AUTOPILOT_DEADLINE` /
no-`BCS_` grep pair exits 0, and `RUSTDOCFLAGS=-Dwarnings cargo doc -p
nova_autopilot --no-deps` is clean. The diff is a faithful line-for-line port of
`/home/alex/personal/bevy-common-systems/src/completion.rs` with only the
constant docs and `DEADLINE_ENV` changed, as the Steps asked. R1.1 was
re-derived independently with a temporary probe test (reverted, not committed).

Process signal: R1.1 and R1.2 are inherited from the BCS source, not introduced
here. A verbatim-port task hides upstream defects behind "matches the source" -
worth a fix-forward task against bevy-common-systems, and worth naming in
future port plans that the port is the moment to audit, not just to copy.

- Pending user checks: none. No `manual:` proofs on this task.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R2.1 (NIT) crates/nova_autopilot/src/completion.rs:296 -
  `the_deadline_clock_tracks_wall_time_whatever_the_collector_count` compares
  `counted <= wall * 1.01`, which is vacuously true if `wall` came back 0.0
  (and the panic message would then print `inf`). 64 frames of `Time` never
  measure zero in practice, so this is insurance, not a live hole: add
  `assert!(wall > 0.0, "the clock never advanced; the comparison would be
  vacuous");` above it.

Verified for round 1's fixes, all three resolved:

- R1.1 - the fix is at `completion.rs:106`/`113`; the watcher is added only
  when the resource was absent. Re-derived independently before the fix
  existed with a throwaway two-collector probe: `elapsed/wall` was
  `2.0000014` with two registrants against `0.99999964` with one. The
  committed test reproduces that failure (`1.9999992x`) and is green now.
- R1.2 - re-ran the mutation myself: dropping `{:?}`/`completion.pending` from
  the `error!` fails the test with
  `logged: ... expired with collectors still pending`, so the naming clause is
  genuinely pinned. `Cargo.lock` gains only two dependency EDGES; both crates
  were already in the graph via `bevy_log`, so no new vendoring.
- R1.3 - pushback accepted, and confirmed first-hand: with the outer `///`
  restored on `lib.rs`, `RUSTDOCFLAGS=-Dwarnings cargo doc -p nova_autopilot
  --no-deps` fails with `unresolved link to DEFAULT_DEADLINE_SECS` and
  `AppExit::error`. The finding was wrong; the `//` note left in its place is
  the right resolution.

No regressions in the fixes. Re-ran on the branch head: `cargo test --lib -p
nova_autopilot completion` is `ok. 6 passed`, `cargo doc` with
`-Dwarnings` is clean, the `NOVA_AUTOPILOT_DEADLINE`/no-`BCS_` proof exits 0,
and `cargo fmt --check -p nova_autopilot` is clean. TASK.md's DoD was amended
from 5 to 6 tests with the reason recorded, and its Notes now correct the plan
clause that had called the false NOTE comment load-bearing - both honest.

- Pending user checks: none.
