# Retro: Name the signal when a smoke example dies without an exit code

- TASK: 20260805-114935
- BRANCH: test/name-signal-on-example-death
- REVIEW ROUNDS: 1

## What went well

- Splitting the report from the crash (`20260805-111329` owns the SIGSEGV
  itself) kept this to one module and three call sites, independently
  committable and independently reviewable.
- Proofs were verified red on base before planning, including the note that
  the looser `--lib exit` filter already passes green on base because it
  matches `completion.rs`. The planned proof was the exact one that could
  distinguish done from not-done.
- `ExitStatusExt::from_raw` as the test seam made a segfault, a core dump and
  an OOM kill assertable without a real death. That choice, made at plan time,
  is the only reason the behaviour is testable at all.
- Review round 1 returned two NITs and no BLOCKER/MAJOR: scope matched the
  plan with no unrequested knob or abstraction.

## What went wrong

- Nothing material. Two deviations from the literal plan, both smaller than
  written: `#[cfg(all(test, unix))]` on the whole test module instead of a
  `cfg` per test (the per-test form left `use super::*` unused on non-unix),
  and a rustfmt import reorder. Both disclosed in the close-out rather than
  silently absorbed.
- Breadth: 155 lines across 7 files, driven by the three call sites the Story
  names. No missed split, no late scope.
- Churn: zero review rework. No plan-time question would have prevented the
  two NITs - both are style observations on delivered, tested code.

## What to improve next time

- Context: on resume, the main checkout's `TASK.md` still read
  `ACTIVITY: WORKING` while the sprout read `REVIEWING`. `sprout ls` before
  `tatr show` resolved it in one step, exactly as `resume.md` prescribes.
  Routing off the main checkout's copy would have re-run `work` over finished
  work. Read the sprout's record, never main's, whenever `sprout ls` names a
  worktree for the ID.
- The unallocated-prefix NIT (`R1.1`) is worth folding in whenever this module
  is next touched; not worth a follow-up task on its own.

## Action items

- None. `nova_probe`'s supervisor and the `nova_assets` / `portal_install`
  command tests still format a bare `status.code()`; `describe` is there to
  reach for when one of them earns it, per `DECISION.md`. No task created.

## Landing message

```
test(examples): name the signal when an example dies without an exit code

A smoke example killed by a signal reported "exited with None" - no signal,
no core dump, no hint the process was killed rather than exiting badly.

Add nova_autopilot::exit::describe(&ExitStatus) -> String, which names the
signal on unix (SIGSEGV, SIGABRT, SIGKILL, SIGBUS, SIGILL, SIGTERM, else
"signal N"), flags a core dump, and points SIGKILL at the OOM killer, falling
through to the exit code everywhere else. The three example assertions in
tests/examples_smoke.rs and crates/nova_autopilot/tests/autopilot_example.rs
now read "example foo was killed by SIGSEGV (core dumped)".

Every status.success() predicate is unchanged: this changes what a failure
says, never which runs fail. Unit tests build statuses with
ExitStatusExt::from_raw, so no process is killed to prove the messages.
```
