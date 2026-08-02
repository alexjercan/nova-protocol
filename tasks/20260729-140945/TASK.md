# Shakedown rehearsal test red on master: RADAR emphasis guard fails

- PRIORITY: 30
- TAGS: backlog
- KIND: TASK
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Story

`cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight` is
RED on master (reproduced at c235a429, and again on the 20260728-175742 branch
before any of its changes - `git stash` A/B confirmed it is inherited, not
caused there).

The failure is the test's own delivery guard, before the behaviour under test:

```
crates/nova_assets/src/scenario/shakedown.rs:2522
delivery guard: the rehearsal was mid-lesson
  assert!(app.world().resource::<HintEmphasis>().contains("RADAR"))
```

So `walk_to_rehearsal` no longer leaves the shakedown parked on beat 10 with
the paint lesson up and RADAR emphasized. Either the rehearsal pacing moved
under the test (a beat was retimed or reordered) or the emphasis is being
cleared earlier than it was. The guard is doing its job - it refuses to assert
the out-of-order-kill skip against a scenario that is not actually mid-lesson.

The regression this test pins is real and player-facing (playtest 2026-07-13:
shooting the hulk before ever locking it soft-locked the run), so a red guard
here means that skip path is currently UNPROVEN.

## Steps

- [ ] Bisect which commit turned it red (`git log --oneline` over
      `crates/nova_assets/src/scenario/shakedown.rs` + the scenario RON) and
      say whether the SCENARIO changed or only the test's assumptions.
- [ ] Fix the right end: retime/repair `walk_to_rehearsal` if the beat map
      legitimately moved, or fix the scenario if the paint lesson stopped
      emphasizing RADAR when it should.
- [ ] Re-check the sibling shakedown tests around it for the same stale
      assumption (they walk the same beats).

## Definition of Done

1. cmd: `cargo test -p nova_assets --lib shakedown` is green.
2. The out-of-order kill -> skip-to-the-fight path is asserted again (not
   deleted or weakened to pass).

## Notes

- Found during 20260728-175742 (HUD icon dock) while running the touched
  crates' suites; that task's own changes are unrelated (it renames the
  keybind-hint RENDERER, not `HintEmphasis`, whose API and verb vocabulary are
  unchanged).


## Dropped

- REASON: false report. Exact test passes on current tree.
