# Notes - Name the signal when a smoke example dies without an exit code

## Problem Statement

When an example process dies on a SIGNAL rather than exiting, the test failure
message names nothing. `ExitStatus::code()` is `None` for a signal death, so
`tests/examples_smoke.rs:317` prints `example <name> exited with None` - no
signal, no core-dump flag, no hint that the process was killed rather than
returning a bad status. The reader cannot tell a segfault from an OOM kill from
a `SIGTERM`, and has to re-run under a debugger to find out. That cost an hour
of triage on task `20260805-111329`, where the answer was "SIGSEGV in the NVIDIA
driver during teardown" - one line, had the message said it.

The same `exited with {:?}` / `status.code()` pattern exists at
`crates/nova_autopilot/tests/autopilot_example.rs:53` and `:133`. Confirmed with
the user: all three sites are in scope.

This is NOT:

- a fix for any crash. `20260805-111329` owns the SIGSEGV itself and is already
  landed; this task fixes the REPORT of a signal death, not its cause. Split off
  deliberately (see that task's `DECISION.md`); independently committable.
- a change to which runs pass or fail. Every assertion keeps its current
  predicate (`status.success()`); only the failure message changes.
- a rework of the `tail(&stderr)` duplication that also exists in both files.
  Out of scope, no named requirement here.
- a change to `nova_probe`'s supervisor
  (`crates/nova_probe/src/bin/probe/native/supervise.rs:145`) or to the
  `nova_assets` / `portal_install` command tests. The user ruled the wider sweep
  out: those are not gameplay processes that die on signals.

## Context

Call sites, all three in scope:

| Site | Line | Message today |
|-|-|-|
| `tests/examples_smoke.rs` | 314-319 | `example {example} exited with {:?}` |
| `crates/nova_autopilot/tests/autopilot_example.rs` | 50-55 | `driven_app exited with {:?}` |
| `crates/nova_autopilot/tests/autopilot_example.rs` | 130-135 | `driven_app exited with {:?}` |

Constraints the answer must respect:

- `std::os::unix::process::ExitStatusExt::signal()` is unix-only. The suite runs
  on Linux in practice (CI is `xvfb-run` on ubuntu), but the code must still
  compile elsewhere - so the signal branch needs a `cfg(unix)` and a non-unix
  fallback, not a bare import.
- `nova_autopilot` is described as "Scripted automation drivers and the
  run-completion protocol". Exit-status naming is inside that remit.
- `nova_debug` already depends on `nova_autopilot`, so the root test tree sees
  it transitively; using it directly from `tests/examples_smoke.rs` needs one
  added line under the root `[dev-dependencies]`.
- `tests/examples_smoke.rs`'s catalog/drift tests must keep compiling on a bare
  `cargo test` with no `debug` feature (existing comment at root `Cargo.toml`
  line 185-189). A plain path dep on `nova_autopilot` satisfies that; it is not
  feature-gated.
- The user chose named signals over bare numbers, with an OOM hint on SIGKILL
  and a core-dumped flag (`ExitStatusExt::core_dumped()`).
- Per AGENTS.md: ASCII-adjacent characters only in the message text.
- Per repo policy: no local `cargo test` / `clippy` run; CI owns the suite.
  `cargo check` plus the newly written unit tests only.

## Ideas

1. **Shared `describe` helper in `nova_autopilot`** (chosen). A small
   `exit` module exposing `describe(&ExitStatus) -> String`. One name table, one
   set of unit tests, three call sites. Unit tests construct statuses with
   `ExitStatusExt::from_raw` so a segfault report is proven without segfaulting
   anything. Cost: one dev-dep line in the root `Cargo.toml`.
2. **Private helper duplicated in both test files.** No manifest change, but
   ~15 lines duplicated across two crates, a signal-name table free to drift out
   of phase, and no way to test either copy short of actually killing a process.
   Loses on both maintainability and provability.
3. **Pull `nix` or `signal-hook` for `strsignal`.** A new dependency to replace
   a six-arm match on numbers that are fixed by POSIX. Loses on YAGNI.

Rejected extras, all YAGNI - no named requirement in this task:

- a `Display` newtype wrapper instead of returning `String`,
- a configurable signal table or a caller-supplied hint map,
- extending the helper to `nova_probe`'s supervisor (explicitly out of scope).
