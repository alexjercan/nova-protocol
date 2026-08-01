# Fix shakedown an_early_derelict_kill_skips_to_the_fight failing on master

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog, bug, gameplay
- KIND: TASK
- FLOW STEP: PLANNING
- PLAN STATUS: DRAFT

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

- [ ] Reproduce the narrow red proof on the current tree:
      `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight`.
      Record the exact panic and line in this task.
- [ ] Inspect `crates/nova_assets/src/scenario/shakedown.rs`: `walk_to_rehearsal`,
      `settle_beat`, the beat 9/10/12 handlers, and the early-derelict-kill
      test. Confirm the fixture's actual beat, objective set, and
      `HintEmphasis` state immediately after `walk_to_rehearsal`.
- [ ] Run `git log -S 'the rehearsal was mid-lesson'` and open the relevant
      diffs around the pacing and HUD changes. Use the diff behavior, not the
      pickaxe hit alone, to identify why the guard started failing.
- [ ] Decide script vs guard:
      if beat 10 no longer posts the paint lesson or RADAR emphasis, fix the
      shakedown script;
      if the script still reaches the intended skip state but the guard pins a
      stale UI side effect, replace the guard with the current invariant
      (`beat == 10`, derelict spawned, fight not spawned, and OBJ_B10 present or
      the documented current equivalent).
- [ ] Sweep sibling shakedown walk tests for the same stale rehearsal guard or
      RADAR-emphasis assumption; update only cases invalidated by this bug.
- [ ] Update generated content only if the scenario script changes:
      `nix develop --command cargo run -p nova_assets --bin content -- gen`.
- [ ] Re-read edited source and generated RON, then run the focused and crate
      proofs from Done Means.

## Done Means

- cmd: `nix develop --command cargo test -p nova_assets --lib an_early_derelict_kill_skips_to_the_fight` - green.
- cmd: `nix develop --command cargo test -p nova_assets --lib` - green.
- manual: the fix names which side was wrong (script vs guard), not both.

## Notes

- Current source path: `crates/nova_assets/src/scenario/shakedown.rs`; the old
  `tests/walk.rs` path in the original panic is stale.
- Similar backlog records exist: `20260730-161545` and `20260731-215407`.
  This task is the active flow record.
- Local planning attempt could not run Nix: `cannot connect to socket at
  /nix/var/nix/daemon-socket/socket: Operation not permitted`.
- Initial source read points to likely guard drift: the derelict skip handler
  still accepts `destroyed(ID_DERELICT)` while `beat < 12` and clears RADAR;
  the failing assertion is the delivery guard before the skip path runs.
