# Review: smoke suite command-error gate

- TASK: 20260713-203709
- BRANCH: fix/smoke-command-error-gate

## Round 1

- VERDICT: APPROVE

- [x] R1.1 (MINOR) tests/examples_smoke.rs:99-103 - when the gate trips, the
  panic message prints only `tail(&stderr)` (last 48 KB), but the offending
  "Encountered an error in command" line can scroll out of that window: a
  clean 6 s run of 01_controller_section already emits 48,336 bytes of
  stderr (measured; the window is 48,000), longer examples (10_playable,
  12_menu_newgame) emit more, and this error class fires at load/teardown
  transitions, i.e. early in the run. The assert already knows the matching
  line - include it. Concrete change: collect
  `stderr.lines().filter(|l| l.contains("Encountered an error in command"))`
  and assert the collection is empty, printing the matched lines above the
  tail. Diagnosability only; the gate itself fires correctly either way.
  - Response: fixed - the assertion collects the matching lines and
    prints them above the tail.

- [x] R1.2 (NIT) examples/12_menu_newgame.rs:13-16 - "the ECS fallback error
  handler is swapped to panic, so an UNHANDLED command error ... aborts the
  smoke run instead of scrolling by" slightly overstates what the swap
  changes in bevy 0.19: the default `BevyError` severity is `Panic`
  (bevy_ecs error/bevy_error.rs:20) and the default fallback is
  `match_severity`, so an unhandled command error aborts even without the
  swap. Nothing in bevy 0.19 tags command errors below Panic (the only
  sub-Panic tagging anywhere is bevy_dev_tools schedule_data, system-side),
  so the swap is belt-and-braces against future severity-tagged errors or a
  default change - useful, but not the thing that stops warns "scrolling
  by". The claim errs conservative (coverage stated is real), so not
  blocking; if this doc is touched again, say "pinned to panic regardless
  of error severity" rather than implying the default would only warn.
  - Response: fixed - the doc now credits bevy's default severity and
    frames the swap as pinning the contract.

## Round 2 (verification of 9cfb0c0)

- VERDICT: APPROVE (stands)

- R1.1 verified against the diff: the assertion now collects
  `stderr.lines().filter(|line| line.contains("Encountered an error in
  command"))` into `command_errors`, asserts `is_empty()`, and prints the
  joined matches above the tail. Predicate is exactly equivalent to the old
  `contains` (the needle has no newline, so any occurrence lies within a
  single line) - no coverage change, diagnosability fixed. Ticked.
- R1.2 verified against the diff: the doc now reads "(Bevy 0.19's default
  severity already panics these; the explicit swap pins the contract
  against upstream default changes.)" - factually correct per the round-1
  source dig. Ticked.
- `cargo fmt --check`: clean. `cargo check -p nova-protocol --test
  examples_smoke --features debug`: clean (0.4 s). Suite not rerun: the
  test change only reshapes the failure message of an assert that passes on
  green runs, and the doc change is comment-only.

## Checks

- `cargo fmt --check`: clean.
- `cargo check -p nova-protocol --test examples_smoke --features debug`:
  clean (0.7 s, warm cache).
- Sabotage A/B rerun (independent, per the Record's recipe): a stale
  `remove::<Transform>` on a same-queue-despawned entity in
  01_controller_section turned the suite red in 9.21 s, failing at exactly
  the new assert (tests/examples_smoke.rs:99). Crucially, the sabotaged
  example itself exited 0 and logged "autopilot: cycle complete, no panic",
  i.e. the panic gate did NOT catch it - the grep is load-bearing, exactly
  as the task claims. Sabotage reverted via git checkout; a clean 01 run is
  green (exit 0, both harness markers, zero occurrences of the string).
  Matches the Record's "red in 10 s, green after revert".
- Full 12-example suite not rerun (implementer ran it green with the gate
  armed; the diff only adds an assert, so per-example behavior is
  unchanged).
- Workspace grep for benign uses of the string: only doc comments
  (crates/nova_gameplay/src/flight.rs:3628, the test's own docs, example
  12's module doc) - no runtime logging, no false-positive source.
- Branch hygiene: worktree clean, exactly 2 commits on top of merge-base,
  nothing leaked onto master.

## Bevy-source claim (re-derived, bevy_ecs-0.19.0)

CONFIRMED on all three legs:

- src/error/handler.rs:93-102: every handler (panic:145, error:152,
  warn:159, info, debug, trace) and the default `match_severity` fallback
  format through the same `inner!` macro:
  `"Encountered an error in {} `{}`: {}"` with `ErrorContext::kind()` =
  "command" for command errors. "Encountered an error in command" is
  therefore the stable shared prefix of both the baked-in-warn and any
  fallback-handled (or panicking) message.
- src/system/commands/mod.rs:1435: `insert` uses `self.queue(...)` ->
  `handle_error()` -> fallback handler (unhandled).
- src/system/commands/mod.rs:1726-1727 and 1906-1907: `remove` and
  `despawn` use `queue_handled(entity_command::..., warn)` - the WARN
  handler is baked in at queue time and the fallback swap can never see
  these errors. The gate closes a real gap.
