# Smoke red on master: screenshot_nova_os exits before completing its cycle

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.9.0,bug,test,tooling
- KIND: TASK
- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

`cargo test --test examples_smoke` is RED on master: `screenshot_nova_os` exits
about 1.5 s into its run, right after `nova harness: reached Playing`, without
ever printing `autopilot: cycle complete, no panic` (or the self-ending
`probe: script complete, exiting`), so `screenshots_reach_playing_without_panic`
fails with "example screenshot_nova_os did not complete its cycle".

Found while verifying task 20260729-211150 (a menu-layout change that touches
none of this). Confirmed INHERITED, not caused by that branch: the same run in a
clean master checkout ends the same way, exit status 0, no error or warning in
the log beyond the usual X11 noise.

The example holds `AutopilotPlugin::<GameStates>::new().hold(Loading, 24.0)` but
the process is gone in ~1.5 s, so something is reporting completion (or exiting
the app) long before the script's beats run. It was added to the smoke list by
task 20260727-143752; suspicion is that it never satisfied the completion
contract and the catalog fix only made the suite start asking.

## Reproduction (2026-07-30, master 92c153a8, RTX 3060 Ti under Xvfb :99)

`DISPLAY=:99 BCS_AUTOPILOT=1 cargo run --example screenshot_nova_os
--features debug` - deterministic across 3 runs: **exit code 0**, 137 log
lines, process wall time ~1.6 s. Last line is 10 ms after
`nova harness: reached Playing`; no `autopilot: cycle complete`, no
`harness completion:` line, no panic, no command error.

The capture run is the A/B: same command plus `BCS_REEL=1
NOVA_SHOT_DIR=...` runs ~9 s, walks all 12 script stages and writes all four
PNGs (welcome/active/map/ship), exit 0. Diffing the two logs
(timestamp-stripped) shows they are IDENTICAL except for the capture lines -
so nothing else in the app behaves differently between the modes.

Established mechanism (the part the source already proves):
`nova_os_capture_script`'s terminal arm does
`world.write_message(AppExit::Success)` directly. The example is a SELF-ENDING
script that never adopted the self-ending contract - no `.self_completing()`
on the `AutopilotPlugin`, no `probe: script complete, exiting` sentinel, no
`HarnessCompletion::done(AUTOPILOT)`, no completion guard. Its three siblings
(`broadside`, `lifeline`, `menu_scenarios`) all do. So even a fully successful
run can only ever end silently, which is exactly what the smoke suite reports.

Open at plan time: the capture run needs ~41 frames at ~34 ms to clear stage 0
alone, so the non-capture run's ~69 frames looked like they could not fit in
the 10 ms - suggesting an earlier `AppExit` from somewhere else. The completion
guard was added FIRST to settle it, as both fix and instrument.

**Traced (step 1), and the plan-time doubt was wrong.** With the guard in and
the old `AppExit::Success` arm untouched, the run panics:

```
thread 'Compute Task Pool (9)' panicked at examples/screenshots/screenshot_nova_os.rs:150:9:
screenshot_nova_os: run ended with the script stalled in stage 13
```

Exit 101, stage **13** - i.e. the script DID walk all 12 stages and the exit is
its own final arm. The 10 ms was a misread: it is the last LOG line, not the
exit, and nothing between the beats logs anything. The fixed run confirms the
real timing - `probe: script complete, exiting` lands 2.33 s after
`reached Playing` (~69 frames at ~34 ms), exactly the frame budget expected.

So the diagnosis was the source one all along: a self-ending script with no
self-ending contract. No second bug; nothing cuts the script short.

## Steps

- [x] Trace the exit: add the completion guard first and run - a premature
      `AppExit` then panics naming the stalled stage, which identifies the real
      exit path with real numbers (record them here). Result above: stage 13,
      the script's own final arm.
- [x] Adopt the self-ending contract, the same shape as `broadside` /
      `lifeline` / `menu_scenarios`: `.self_completing()` on the
      `AutopilotPlugin`, and a final stage that logs
      `probe: script complete, exiting`, marks the script done, and reports
      `HarnessCompletion::done(AUTOPILOT)` instead of writing `AppExit`
      directly.
- [x] Fix whatever the trace turns up so the script actually walks its stages
      in smoke mode (the beats must fire, not just the exit be clean). Nothing
      to fix: the trace proved all 12 stages already run in smoke mode.
- [x] Update the example's `//!` header so the documented smoke path matches
      the sentinel it now prints.
- [x] Re-run the screenshots smoke category, then the whole suite.

## Definition of Done

1. cmd: `DISPLAY=:99 cargo test --test examples_smoke` - all five tests green.
2. cmd: `DISPLAY=:99 BCS_AUTOPILOT=1 BCS_REEL=1 NOVA_SHOT_DIR=<dir> cargo run
   --example screenshot_nova_os --features debug` still writes all four PNGs
   (nova-os-welcome/active/map/ship) and exits 0.
3. test: the silent pass-then-exit is pinned in the example itself - a
   `guard_script_completion` system in `Last` that panics when an `AppExit` is
   read with the script unfinished, so any future premature exit fails the
   smoke run loudly instead of scrolling past.
4. manual: the four captured PNGs are eyeballed (render-output-eyeball) to
   confirm the beats still land on the right screens.

## Verification (branch fix/nova-os-smoke-completion, Xvfb :99, RTX 3060 Ti)

1. `DISPLAY=:99 cargo test --test examples_smoke`:
   `test result: ok. 5 passed; 0 failed; ... finished in 160.53s` - including
   `screenshots_reach_playing_without_panic`, the test this task was filed on.
2. Capture run with `BCS_REEL=1 NOVA_SHOT_DIR=...`: exit 0, all four PNGs
   written (`Screenshot saved to ...` for welcome/active/map/ship), every save
   landing BEFORE `probe: script complete, exiting` (measured margin on the
   last one: 0.53 s). The margin comes from stage 11's 20-frame settle, not
   from the completion protocol - the captures are not registered collectors
   (review R1.1).
3. Fail-first proof for the guard: with the guard in and the old
   `AppExit::Success` arm still in place, the run exits 101 with
   `run ended with the script stalled in stage 13`. The pin fails on the
   pre-fix code and passes on the fixed code.
4. Eyeballed all four PNGs (render-output-eyeball): welcome shows the POST
   banner + empty `nova>` prompt; active shows the `help` table, `ship view`
   output (3 sections, CTL-1/HULL-1/THR-1) and the `lo` prefix in the input;
   map shows the range rings + SELF blip; ship shows the RTT section blocks
   with the CTL-1 detail panel. The beats still land on the right screens.
5. `cargo fmt` clean, `cargo check --examples --features debug` clean.

Skipped per AGENTS.md: the full `cargo test` / `cargo clippy` (CI runs both).

## Notes

- Related: task 20260727-143752 (smoke-listed this example), and the standing
  red tracked in 20260729-140945 (a different failure - the shakedown rehearsal
  guard, a `nova_assets` lib test outside this suite; still red, untouched).
