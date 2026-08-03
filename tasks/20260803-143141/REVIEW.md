# Review: Fix the hud_range example smoke: the scripted run never reaches its last beat

- TASK: 20260803-143141
- BRANCH: fix/hud-range-runway

## Round 1

- REVIEWER: out-of-context
- VERDICT: APPROVE

- [ ] R1.1 (MINOR) examples/ui/hud_range.rs:1010 - the kill-cam comment still
  justifies the assert with "the 6s autopilot window ends before the linger
  does", which this diff made doubly wrong: the hold is a 30s runway and the
  run ends because the script reports done, not because a window closes.
  Rewrite it to say the assert runs ~0.4s after the kill, inside the linger,
  and the run self-ends on this beat.
  - Response:

- [ ] R1.2 (NIT) examples/ui/hud_range.rs:113 - the guard's panic prints
  `drop={}` from `script.done`, which the enclosing `!script.done` condition
  pins to `false` on every path (carried over verbatim from the deleted
  backstop, where the same was true). Drop the `drop=` field from the message.
  - Response:

- [ ] R1.3 (NIT) examples/ui/hud_range.rs:37 - the module-header smoke doc,
  rewritten by this diff, still says "+4s despawn the target ... +4.5s assert"
  while the beats are `t > 4.4` (:962) and `t > 4.8` (:974). Correct the two
  times while the sentence is being touched.
  - Response:

- [ ] R1.4 (NIT) examples/ui/hud_range.rs:91 - the guard registration gates on
  `std::env::var_os("NOVA_AUTOPILOT")` with a hardcoded literal while
  `AutopilotPlugin` gates on `std::env::var(AUTOPILOT_ENV)`
  (`autopilot.rs:50`, re-exported at `lib.rs:86`). Use
  `harness::AUTOPILOT_ENV` so the two gates cannot diverge; same at
  `examples/sections/com_range.rs:75`.
  - Response:

### Verification (re-derived in-session, not taken from the reviewer)

- Load-bearing claim re-derived: the guard cannot catch a *silent success*,
  only an already-failing exit. `completion_watch` (`completion.rs:161-188`)
  is the sole `AppExit::Success` writer and writes it only when the pending
  set is empty; `AutopilotPlugin::build` registers `AUTOPILOT`
  (`autopilot.rs:202`), and under `self_completing` only the final beat clears
  it. So `Success` with `!script.done` is unreachable. Nothing in the diff or
  the record claims otherwise - the guard docstring scopes itself correctly to
  "an exit written before that schedule" (the `PreUpdate` runway error at
  `autopilot.rs:298-306`), and RETRO describes the two paths firing *in
  sequence*. Not a finding.
- Ordering caveat on the same mechanism: guard and `completion_watch` both sit
  in `Last`, unordered, with conflicting `Messages<AppExit>` access, so on a
  runway expiry the guard's extra beat detail is executor-order-dependent. The
  run still fails loudly either way (harness `AppExit::error`), so this costs
  diagnostics, not detection.
- Reporting done from inside the input callback is legal: `autopilot_drive`
  removes `AutopilotState` for the call and its top-of-frame
  `self_completing && !is_pending(AUTOPILOT)` check (`autopilot.rs:222-229`)
  makes the driver inert afterwards, so the runway cannot expire after a
  completed script.
- DoD `cmd:` proofs re-run in the worktree: `cargo test --test examples_smoke`
  -> 6 passed / 0 failed in 100.5s under Xvfb :99 (Xvfb killed by recorded PID
  3314247); `! rg 'elapsed > 7\.5' ...` -> 0; `rg 'self_completing|probe:
  script complete' ...` -> 4 hits across both examples; `cargo fmt --check` +
  `cargo check --examples --features debug` -> clean; tree clean.
- Behavior change worth an eye, already called out in NOTES: under `probe run`
  the script now stops driving ~3s earlier while the frame capture keeps
  sampling, so a slightly larger tail of captured frames is un-driven.
- Process signal: all 8 Steps and all 5 DoD items are satisfied with no scope
  drift, and the Step-1 correction (the planned `hold` -> 5.5 lever yields a
  vacuous pass, not the planned panic) is recorded in RETRO honestly rather
  than glossed.

### Pending user checks

- `manual:` falsification transcript in `tasks/20260803-143141/RETRO.md` -
  present, with both sabotages (hud final beat gated to `t > 999.0`,
  com_range assert beat gated to `t > 999.0`) showing the harness error exit
  and the guard panic, and both reverted. Reproducing it requires editing
  example source, which review does not do, so it stays a user check. Does not
  block APPROVE.

### Inspection commands

```bash
cd "$(sprout show fix/hud-range-runway)"
git diff master...HEAD -- examples/
nix develop --command cargo test --test examples_smoke   # needs DISPLAY=:99
```
