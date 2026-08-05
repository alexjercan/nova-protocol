# Name the signal when a smoke example dies without an exit code

- PRIORITY: 46
- TAGS: v0.10.0, testing, examples, dx
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -
- PARENT: 20260802-115955

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

- [ ] Add `crates/nova_autopilot/src/exit.rs` with
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
- [ ] Add `#[cfg(test)] mod tests` in the same file, matching the crate's
      in-file convention (`completion.rs:191`). Build statuses with
      `ExitStatusExt::from_raw` so no process is actually killed: raw `11`
      (SIGSEGV), raw `139` (SIGSEGV + core dumped bit), raw `9` (SIGKILL, OOM
      hint), raw `0x1F00` (exit code 31), and an unnamed signal (raw `5`)
      degrading to `signal 5`. Assert the message text, not just that it is
      non-empty.
- [ ] Swap the three call sites from `output.status.code()` to
      `nova_autopilot::exit::describe(&output.status)`, rewording each format
      string from `exited with {:?}` to `{}` so the message reads
      `example foo was killed by SIGSEGV (core dumped)`:
      `tests/examples_smoke.rs:314-319`,
      `crates/nova_autopilot/tests/autopilot_example.rs:50-55` and `:130-135`.
      Every assertion keeps its `output.status.success()` predicate unchanged -
      this changes what a failure SAYS, not which runs fail.
- [ ] Add `nova_autopilot = { path = "crates/nova_autopilot" }` under the root
      `[dev-dependencies]` (`Cargo.toml:173-197`), with a comment naming why it
      is a plain path dep and not feature-gated: the catalog/drift tests in
      `tests/examples_smoke.rs` must keep compiling on a bare `cargo test`
      (existing comment at `Cargo.toml:181-185`). The autopilot test file needs
      no manifest change - it is inside the crate.
- [ ] `nix develop --command cargo fmt`, then
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
