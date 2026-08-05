# Decision: Name the signal when a smoke example dies without an exit code

- DATE: 20260805-124851
- STATUS: ACCEPTED
- TASK: 20260805-114935
- TAGS: testing, examples, dx

## Context

A signal death makes `ExitStatus::code()` return `None`, so every example
assertion that formats it prints `exited with None` - naming no signal, no core
dump, and no hint that the process was killed rather than exiting badly. Task
`20260805-111329` lost an hour to exactly that: the real answer was "SIGSEGV in
the NVIDIA driver during teardown".

Three sites share the pattern, all confirmed in scope with the user:
`tests/examples_smoke.rs:317` and `crates/nova_autopilot/tests/autopilot_example.rs`
at `:53` and `:133`. Full detail in `NOTES.md`.

## Decision

Add `nova_autopilot::exit::describe(&ExitStatus) -> String` and call it from all
three sites in place of `status.code()`.

- Exit-status naming belongs to `nova_autopilot`, whose remit is already
  "scripted automation drivers and the run-completion protocol".
- `describe` names the common signals - SIGSEGV, SIGABRT, SIGKILL, SIGBUS,
  SIGILL, SIGTERM - and falls back to `signal N` for anything else.
- It flags `core dumped` from `ExitStatusExt::core_dumped()`, and points SIGKILL
  at the OOM killer, the overwhelmingly likely cause on a CI box.
- The signal branch is `cfg(unix)`; other targets fall back to the code.
- Shape of the output:

  ```
  example foo was killed by SIGSEGV (core dumped)
  example bar was killed by SIGKILL - likely the OOM killer
  example baz exited with code 101
  ```

- Unit tests build statuses with `ExitStatusExt::from_raw`, so the segfault and
  OOM messages are proven without killing a real process.
- `nova_autopilot` gains one line under the root `[dev-dependencies]`; it is not
  feature-gated, so the bare-`cargo test` catalog/drift tests keep compiling.

Every assertion keeps its existing `status.success()` predicate. This changes
what a failure SAYS, never which runs fail.

## Alternatives considered

- **Private helper duplicated in both test files.** Avoids touching
  `Cargo.toml`, but duplicates ~15 lines across two crates, lets the signal
  table drift out of phase, and cannot be unit-tested short of really killing a
  process. Maintainability and provability both lose.
- **Bare signal numbers, no name table.** Less code to keep in phase, but leaves
  the reader looking up `11` - which is the exact cost this task exists to
  remove. Rejected by the user in favour of names.
- **`nix` / `signal-hook` for `strsignal`.** A new dependency to replace a
  six-arm match over numbers POSIX has fixed. YAGNI.
- **Sweeping every `Command`-running test** (`nova_assets`, `nova_probe`,
  `portal_install`). Ruled out by the user: those are not gameplay processes
  that die on signals, so the blast radius buys nothing.

## Consequences

- A signal death now diagnoses itself in the assertion message; the SIGSEGV that
  cost an hour would have read as one line.
- One new public module on `nova_autopilot` and one new root dev-dependency.
- The signal name table is a fixed six-arm match; an unlisted signal degrades to
  `signal N`, which is still strictly better than `None`.
- `nova_probe`'s supervisor and the asset/portal command tests keep the old
  message. If one of those ever dies on a signal, this helper is there to reach
  for - but nothing in this task justifies wiring it now.
- The `tail(&stderr)` duplication across the two files stays. Untouched, noted.
