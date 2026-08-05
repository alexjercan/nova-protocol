# menu_scenarios is killed by a signal in the ui smoke, roughly 1 run in 5

- PRIORITY: 83
- TAGS: v0.10.0,bug,examples,testing
- ACTIVITY: -
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955

## Story

`menu_scenarios` is intermittently KILLED BY A SIGNAL during the ui smoke
category - not a panic, not a stall, no exit code at all:

```
thread 'ui_reach_playing_without_panic' panicked at tests/examples_smoke.rs:314:9:
example menu_scenarios exited with None
```

Observed 2026-08-05 on `DISPLAY=:99` (Xvfb 1280x720), 1 failure in 5 runs of
`nix develop --command env DISPLAY=:99 cargo test --test examples_smoke ui`.
The other 4 passed, and a 23-example suite-shaped round passed 23/23.

`exited with None` is `ExitStatus::code() == None`: the process died on a
signal. The captured stderr tail (48 KB) ends mid-scenario-load, in the racer
section configs and the integrity/collider observers, with no panic, no
autopilot stall and no completion deadline - so the process was still doing
useful work when it went. The pointer beats before it had already succeeded.

Found while closing `20260805-091151` (the driven-click flake); it is a
DIFFERENT fault - that one exits 1 with an explicit stall line - and was not
introduced by its fix.

## Notes

- First job is the signal: run it under a loop until it dies and catch the
  status (`WTERMSIG`), a core, or `dmesg`. `dmesg` was not readable from the
  agent session, so the OOM-killer hypothesis is UNTESTED.
- The prime suspects are memory (a scenario load under llvmpipe, several
  example processes alive at once) and a crash in the software GL stack.
- Do not scope a fix before the signal is known.
