# Name the signal when a smoke example dies without an exit code

- STATUS: CLOSED
- PRIORITY: 46
- TAGS: v0.10.0, testing, examples, dx

## Story

When a smoke example dies on a signal, `tests/examples_smoke.rs` reports
`example <name> exited with None` - `ExitStatus::code()` is `None` for a signal
death, and the message names nothing: no signal, no core, no hint that the
process was killed rather than exiting badly.

That cost an hour of triage on `20260805-111329`, where the real answer was
"SIGSEGV in the NVIDIA driver during teardown". `ExitStatusExt::signal()` would
have said so in one line.

Split off `20260805-111329` on purpose (see its `DECISION.md`): that task fixes
the crash, this one fixes the report of it. They are independently committable
and the fix for one does not need the other.

## Steps

- [x] Add `crates/nova_autopilot/src/exit.rs` with
      `pub fn describe(status: &ExitStatus) -> String`, and register it as
      `pub mod exit;` in `crates/nova_autopilot/src/lib.rs` (beside
      `pub mod completion;` at line 84). Module docs state the remit: it names
      what `ExitStatus::code()` cannot, so an assertion message diagnoses a
      signal death on its own.
  - Under `cfg(unix)`, `ExitStatusExt::signal()` first: six named arms
        (SIGSEGV 11, SIGABRT 6, SIGKILL 9, SIGBUS 7, SIGILL 4, SIGTERM 15),
        `signal N` otherwise. Append ` (core dumped)` when
        `ExitStatusExt::core_dumped()`, and ` - likely the OOM killer` for
        SIGKILL.
  - Fall through to `code()` on every target: `exited with code N`, and
        `exited with no code` when even that is `None` (non-unix signal death).
  - ASCII-adjacent text only (`AGENTS.md`); no new dependency
        (`DECISION.md`, alternative 3).
- [x] Add `#[cfg(test)] mod tests` in the same file, matching the crate's
      in-file convention (`completion.rs:191`). Build statuses with
      `ExitStatusExt::from_raw` so no process is actually killed: raw `11`
      (SIGSEGV), raw `139` (SIGSEGV + core dumped bit), raw `9` (SIGKILL, OOM
      hint), raw `0x1F00` (exit code 31), and an unnamed signal (raw `5`)
      degrading to `signal 5`. Assert the message text, not just that it is
      non-empty.
- [x] Swap the three call sites from `output.status.code()` to
      `nova_autopilot::exit::describe(&output.status)`, rewording each format
      string from `exited with {:?}` to `{}` so the message reads
      `example foo was killed by SIGSEGV (core dumped)`:
      `tests/examples_smoke.rs:314-319`,
      `crates/nova_autopilot/tests/autopilot_example.rs:50-55` and `:130-135`.
      Every assertion keeps its `output.status.success()` predicate unchanged -
      this changes what a failure SAYS, not which runs fail.
- [x] Add `nova_autopilot = { path = "crates/nova_autopilot" }` under the root
      `[dev-dependencies]` (`Cargo.toml:173-197`), with a comment naming why it
      is a plain path dep and not feature-gated: the catalog/drift tests in
      `tests/examples_smoke.rs` must keep compiling on a bare `cargo test`
      (existing comment at `Cargo.toml:181-185`). The autopilot test file needs
      no manifest change - it is inside the crate.
- [x] `nix develop --command cargo fmt`, then
      `nix develop --command cargo check -p nova_autopilot --all-targets` and
      `nix develop --command cargo check --test examples_smoke`. Per repo
      policy, the only tests run locally are the newly written ones
      (`cargo test -p nova_autopilot --lib exit::tests`); CI owns the suite.

## Definition of Done

- `nova_autopilot::exit::describe` exists and is public.
  (cmd: `grep -n 'pub mod exit;' crates/nova_autopilot/src/lib.rs && grep -n 'pub fn describe' crates/nova_autopilot/src/exit.rs`)
- The new unit tests exist and pass, proving the SIGSEGV, core-dumped, SIGKILL
  and plain-code messages without killing a process.
  (cmd: `nix develop --command cargo test -p nova_autopilot --lib exit::tests 2>&1 | grep -q 'test exit::tests::'`)
- No call site formats a bare `status.code()` any more.
  (cmd: `! grep -rn 'status\.code()' tests/examples_smoke.rs crates/nova_autopilot/tests/autopilot_example.rs`)
- Both test trees still compile, including the bare-`cargo test` catalog path.
  (cmd: `nix develop --command cargo check -p nova_autopilot --all-targets && nix develop --command cargo check --test examples_smoke`)

## Notes

- `tests/examples_smoke.rs:314` is the assertion; it prints
  `output.status.code()`.
- `std::os::unix::process::ExitStatusExt::signal()` gives the number; the suite
  is unix-only in practice but the import needs a cfg or a helper.
- Worth naming the signal by name (SIGSEGV/SIGKILL) rather than the number -
  SIGKILL in particular should point the reader at the OOM killer.
- Proofs verified red on the base branch before planning: the `status.code()`
  grep returns the three live sites (so the negated proof exits 1), and
  `cargo test -p nova_autopilot --lib exit::tests` runs 0 tests (so the
  `test exit::tests::` grep finds nothing). The looser filter `--lib exit`
  is NOT a valid proof - it already matches `exits_success_only_when_...` in
  `completion.rs` and passes green on base.
- The three call sites also share a `tail(&stderr)` helper duplicated across
  the two files. Out of scope, no named requirement (`NOTES.md`).
- Assumption: `ExitStatusExt::from_raw` takes the raw wait status on Linux, so
  `139` encodes SIGSEGV plus the core bit and `0x1F00` encodes exit code 31.
  If a construction does not round-trip, adjust the raw values in the test -
  the asserted messages are what matter, not the encoding.
- The working tree carries unrelated staged changes (screenshots, web docs,
  `Cargo.toml`) from adjacent work. `work` runs in a sprout worktree off
  `master`, so they do not travel; nothing here stages them.

## Close-out

- **What/why.** `nova_autopilot::exit::describe(&ExitStatus) -> String` names
  the signal a process died on, so the three example assertions read
  `example foo was killed by SIGSEGV (core dumped)` instead of
  `exited with None`. Call sites swapped in `tests/examples_smoke.rs:315`,
  `crates/nova_autopilot/tests/autopilot_example.rs:52` and `:132`; every
  `status.success()` predicate is unchanged, so this changes what a failure
  says, never which runs fail. Root `[dev-dependencies]` gains
  `nova_autopilot`, unconditional like `nova_debug`, so the bare-`cargo test`
  catalog path still compiles.
- **Alternatives.** As planned in `DECISION.md`: a duplicated private helper
  (undoable to unit-test without really killing a process), bare signal numbers
  (the exact lookup cost this task removes), and a `nix`/`signal-hook`
  dependency for `strsignal` (YAGNI against a six-arm match).
- **Difficulties.** None material. Two deviations from the literal plan, both
  smaller than written: the tests module is `#[cfg(all(test, unix))]` rather
  than `#[cfg(test)]` with a `cfg(unix)` on each test - every case is built
  from a raw unix wait status, and the per-test form left `use super::*`
  unused on non-unix. `rustfmt` also reordered the test imports. The
  `from_raw` assumption held: `139` round-tripped as SIGSEGV plus the core bit
  and `0x1F00` as exit code 31, so no raw value needed adjusting.
- **Evidence.** All four DoD proofs green in the worktree:
  `pub mod exit;`/`pub fn describe` greps hit; `cargo test -p nova_autopilot
  --lib exit::tests` = 5 passed; the `status.code()` grep finds nothing in
  either test file; `cargo check -p nova_autopilot --all-targets` and
  `cargo check --test examples_smoke` both finish clean. Also
  `cargo test -p nova_autopilot --doc exit` = 1 passed (the `describe`
  doctest) and `cargo fmt --all -- --check` clean. Per repo policy the wider
  suite is CI's.
- **Reflection.** The unit-shaped seam paid off exactly as planned - the
  segfault and OOM messages are proven without a real death, which is the only
  reason this behaviour is testable at all. `nova_probe`'s supervisor and the
  asset/portal command tests still print the old message; the helper is now
  there to reach for when one of them earns it.
