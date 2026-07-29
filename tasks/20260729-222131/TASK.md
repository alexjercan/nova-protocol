# Smoke red on master: screenshot_nova_os exits before completing its cycle

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.9.0,bug,test,tooling

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

## Steps

- [ ] Reproduce and record: run it on master with the harness, capture the
      full log and the exit path (which `AppExit` fires, and from where).
- [ ] Trace why the run ends before the autopilot window: compare with a
      passing sibling (`screenshot_ui`) - completion collectors
      (`HarnessCompletion`), the capture plugin, and whether the example ever
      registers an autopilot collector at all.
- [ ] Fix so the example either runs out its window and prints
      `autopilot: cycle complete, no panic`, or self-ends properly with
      `probe: script complete, exiting` plus a completion guard (the broadside
      pattern) - and its capture beats actually fire.
- [ ] Re-run the whole smoke suite green.

## Definition of Done

1. cmd: `DISPLAY=:99 cargo test --test examples_smoke` - all five tests green.
2. cmd: the example's captures still land under `NOVA_SHOT_DIR` with `BCS_SHOT`
   / its documented capture env.
3. test: whatever made this silently pass-then-exit is pinned so it cannot
   regress silently again.

## Notes

- Related: task 20260727-143752 (smoke-listed this example), and the standing
  red tracked in 20260729-140945 (a different failure - the shakedown rehearsal
  guard).

## Flow State

- FLOW STEP: UNDERSTANDING
