# Name the signal when a smoke example dies without an exit code

- PRIORITY: 46
- TAGS: v0.10.0, testing, examples, dx
- ACTIVITY: UNDERSTANDING
- GATES: -
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

## Notes

- `tests/examples_smoke.rs:314` is the assertion; it prints
  `output.status.code()`.
- `std::os::unix::process::ExitStatusExt::signal()` gives the number; the suite
  is unix-only in practice but the import needs a cfg or a helper.
- Worth naming the signal by name (SIGSEGV/SIGKILL) rather than the number -
  SIGKILL in particular should point the reader at the OOM killer.
