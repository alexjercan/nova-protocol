# Fix the red shakedown test: an_early_derelict_kill_skips_to_the_fight

- PRIORITY: 0
- TAGS: backlog
- ACTIVITY: -
- GATES: -
- RESOLUTION: WONTDO

## Story

`nova_assets::scenario::shakedown::tests::an_early_derelict_kill_skips_to_the_fight`
fails on master (verified at d1460fc5, "fix(hud): the world-anchored chips back
their whole label"):

```
panicked at crates/nova_assets/src/scenario/shakedown.rs:2522:9:
delivery guard: the rehearsal was mid-lesson
```

Found while running `cargo test -p nova_assets` during task 20260730-122940
(keycap aspect sizing). Confirmed inherited, not caused there: stashing that
task's changes and re-running the single test reproduces the same failure.

The assertion is a DELIVERY GUARD - it exists to prove the fixture reached the
state the test is about - so a red one means the shakedown rehearsal no longer
gets where the test assumes, and the skip-to-the-fight path it was written for
is currently unverified.

## Steps

- [ ] Reproduce on master and read the guard's surroundings: what state does the
      rehearsal have to be in, and what does it actually reach now?
- [ ] `git log -S 'the rehearsal was mid-lesson'` and blame the scenario beats
      around it to find WHICH change moved the fixture (open the diff, do not
      trust the pickaxe hit alone).
- [ ] Fix whichever is wrong - the scenario or the test's assumption - and say
      which in the close-out.
- [ ] Check the rest of the shakedown tests for the same stale assumption.

## Definition of Done

1. `cargo test -p nova_assets --lib shakedown` is green (cmd:
   `cargo test -p nova_assets --lib shakedown`).
2. The close-out names whether the scenario or the test was wrong, and why the
   guard started failing.


## Dropped

- REASON: duplicate of above; test passes.
