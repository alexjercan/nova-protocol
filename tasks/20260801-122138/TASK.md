# Fix shakedown an_early_derelict_kill_skips_to_the_fight failing on master

- PRIORITY: 0
- TAGS: backlog, bug, gameplay
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Context

Found during the KISS pass on nova_assets (20260731-170409); PRE-EXISTING on
master at e038c34e, not introduced by that refactor.

`cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`
panics at the rig's delivery guard:

```
panicked at crates/nova_assets/src/scenario/shakedown/tests/walk.rs:671:
delivery guard: the rehearsal was mid-lesson
```

The other 95 lib tests pass. Either the early-derelict-kill shortcut no longer
lands where the guard expects (a real script regression) or the guard's
precondition drifted with the pacing rework; decide which before changing
either side.

## Steps

- [x] Reproduce the narrow red proof on the current tree:
      `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`.
      Record the exact panic and line in this task.
- [x] Inspect `crates/nova_assets/src/scenario/shakedown/tests/walk.rs` and
      `crates/nova_assets/src/scenario/shakedown/mod.rs`: `walk_to_rehearsal`,
      `settle_beat`, the beat 9/10/12 handlers, and the early-derelict-kill
      test. Confirm the fixture's actual beat, objective set, and
      `HintEmphasis` state immediately after `walk_to_rehearsal`.
- [x] Run `git log -S 'the rehearsal was mid-lesson'` and open the relevant
      diffs around the pacing and HUD changes. Use the diff behavior, not the
      pickaxe hit alone, to identify why the guard started failing.
- [x] Decide script vs guard:
      if beat 10 no longer posts the paint lesson or RADAR emphasis, fix the
      shakedown script;
      if the script still reaches the intended skip state but the guard pins a
      stale UI side effect, replace the guard with the current invariant
      (`beat == 10`, derelict spawned, fight not spawned, and OBJ_B10 present or
      the documented current equivalent).
- [x] Sweep sibling shakedown walk tests for the same stale rehearsal guard or
      RADAR-emphasis assumption; update only cases invalidated by this bug.
- [x] Update generated content only if the scenario script changes:
      `nix develop --command cargo run -p nova_assets --bin content -- gen`.
- [x] Re-read edited source and generated RON, then run the focused and crate
      proofs from Definition of Done.

## Definition of Done

- Focused regression test is green. (cmd: `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`)
- Touched crate lib suite is green. (cmd: `nix develop --command cargo test -p nova_assets --lib`)
- Close-out names which side was wrong: script or guard, not both. (manual: final summary)

## Notes

- Current source path after the split is
  `crates/nova_assets/src/scenario/shakedown/tests/walk.rs`; the script is in
  `crates/nova_assets/src/scenario/shakedown/mod.rs`.
- Similar backlog records exist: `20260730-161545` and `20260731-215407`.
  This task is the active flow record.
- Local attempts could not run Nix: `cannot connect to socket at
  /nix/var/nix/daemon-socket/socket: Operation not permitted`. Plain `cargo`
  is also unavailable outside Nix in this sandbox.
- Diagnosis: fixture drift, not script drift. The script defers beat 1 behind
  the opening conversation hand-off (`finish_opening` in the end-to-end walk);
  `walk_to_rehearsal` skipped that hand-off and drove beacon events too early.
  The beat 10 script still posts OBJ_B10 plus RADAR, and the derelict skip
  handler still accepts `destroyed(ID_DERELICT)` while `beat < 12`.
- Fix: `walk_to_rehearsal` now calls `finish_opening(app)` immediately after
  `boot(app)`, matching the end-to-end walk before driving beacon 1.
- No generated content update: test-only fixture change, scenario script and
  generated RON unchanged.
- Proofs attempted:
  `tatr proofs 20260801-122138` listed the two expected cmd proofs and manual
  summary;
  `tatr check 20260801-122138` passed;
  both required `nix develop --command cargo test ...` proofs failed before
  Cargo started because the sandbox cannot connect to the Nix daemon socket.
- Proofs passed after unrestricted shell resume:
  `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`
  passed, 1 test;
  `nix develop --command cargo test -p nova_assets --lib` passed, 96 tests.
  Both emitted existing `nova_gameplay` future-incompat visibility warnings.

## Close-out

Fixture was wrong, not the shakedown script. The scenario's current beat 1
starts behind the opening conversation hand-off; `the_five_beats_walk_end_to_end`
already calls `finish_opening` before firing beacon 1, but the shortcut helper
used by the fight tests skipped that hand-off and drove stale beat events.
Adding the same hand-off to `walk_to_rehearsal` restores the intended beat 10
state before the early derelict kill.
