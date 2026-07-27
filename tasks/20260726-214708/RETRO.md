# Retro: NOVA OS terminal UX parity

- TASK: 20260726-214708
- BRANCH: feature/nova-os-terminal-ux
- REVIEW ROUNDS: 1 (APPROVE, 3 NITs)

See TASK.md for what/why and NOTES.md for the per-behavior implementation
notes. This is process only.

## What went well

- Mapped the 7500-line `drawer.rs` with an out-of-context Explore agent BEFORE
  touching it, so the parser rework and the boot/footer plumbing landed against
  the real struct/fn names and schedule instead of guesses. Big-file work is
  cheap to get wrong from memory.
- After the load-bearing signature change (`terminal_snapshot_from_world` +1
  arg, `TerminalCommandSnapshot` +2 fields), ran `cargo check -p nova_gameplay
  --tests` BEFORE writing any new test. It listed the four caller sites and the
  one struct-literal fanout in seconds; fixing them was mechanical.
- Reused the manual-`Time::<Real>` rig from `slide_drives_single_monitor_openness`
  for the staggered-boot test rather than hand-rolling a clock (the
  `nextstate-input-test-needs-clear-and-two-updates` family habit paid off).
- The out-of-context reviewer earned its keep even on a clean diff: it flagged
  R1.1 (the empty-`extend` change-detection thrash), which the in-session pass
  had independently spotted - a good confirmation of the default's value.

## What went wrong

- R1.1: `announce_objectives_in_terminal` called `scrollback.extend(fresh)`
  unconditionally inside `if open`, so an empty announce still mutably-derefed
  the `ResMut` and marked the terminal changed, forcing a `rebuild_terminal_ui`
  that snaps the scroll to the bottom - which would fight the new paging offset.
  Root cause: wrote the mutation without accounting for Bevy marking a resource
  changed on any `&mut` deref, no-op or not. Caught in review, fixed with a
  `!fresh.is_empty()` guard.

## What to improve next time

- When a Bevy system holds a `ResMut<T>` that other systems watch with
  `run_if(resource_changed::<T>)`, gate every mutation behind an actual-change
  check; a no-op `&mut` deref still triggers the dependents.

## Action items

- [x] Fixed R1.1 (guard the announce `extend`) and R1.3 (doc the Ctrl-held
      app-key guard) on the branch.
- [x] Added `resmut-noop-deref-marks-changed` to the lessons ledger.
- No follow-up tasks: R1.2 (announce only at the prompt) is deliberate and
      documented; the caret/banner screenshot is a pending manual user check.
