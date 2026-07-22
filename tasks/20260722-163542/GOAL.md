# Goal: mainline objective pacing - instructional objectives land mid-read

- DATE: 20260722
- UMBRELLA TASK: 20260722-163542
- LANDING SCOPE: This bg session works in place (no sprout worktree per session
  config). Each task is worked on an in-place feature branch in the main
  checkout and squash/ff-merged to LOCAL master. No push (owner's call, as with
  the preceding pacing task). Verify `git branch --show-current` before every
  commit (shared checkout).

## Goal

The previous pacing pass (task 20260722-142341) made EVERY mainline objective
post a full comms-dwell gap (8.4s) after its intro line - correct for threat/
reveal beats, too rigid for instructional beats where the objective echoes a
coaching line in real time ("Now hand her to the computer" -> "Press [G]"). An
out-of-context pacing review classified every line->objective beat and found
the single global gap conflates two relationships. This run splits the gap into
named, comms-derived categories and applies the right one per beat, so an
instructional objective lands mid-read while a reveal still gets the full
absorb beat. It also fixes one feel bug the uniform gate introduced: Broadside's
contact beat withholds the hauler's nav marker for the full gap.

## Done means

1. `pacing.rs` defines named, comms-derived gap constants (REVEAL_GAP = current
   8.4s; INSTRUCTION_GAP ~= COMMS_MIN_SECS 4s; MID_GAP ~= 6s), each documented
   as playtest-tunable. (cmd: `grep -n "INSTRUCTION_GAP\|REVEAL_GAP" crates/nova_assets/src/scenario/pacing.rs`)
2. Each mainline line->objective beat uses its classified gap at the stamp site
   (shakedown instruction beats ~4s, mid beats ~6s, reveals 8.4s; combat
   chapters mostly 8.4s). (test: `cargo test -p nova_assets --lib scenario::`)
3. Broadside contact beat marks the hauler at the TRANSITION, not inside the
   gated objective, so a nav target exists during the line. (test: the
   marker-hand-off / broadside tests)
4. Regenerated base content committed and parity green. (cmd: `cargo test -p nova_assets --test content_ron_parity`)
5. Content lints clean. (cmd: `cargo run -p nova_assets lint` -> 0 errors)
6. Scenarios still load and run. (manual/cmd: probe menu_newgame + lifeline + broadside OK)

Overall: `cargo test -p nova_assets --lib scenario::` + the parity test green,
fmt/check clean, probes OK.

## Tasks

Updated as tasks land (one line per land).

- [x] 20260722-163718 (p82, nova_assets) Per-beat objective pacing gaps: instruction vs reveal (mainline)
      landed 26bf80f9 (+718ccdf0 review); 1 review round, APPROVE no findings;
      8 instruction / 2 mid / reveal beats classified, broadside hauler marker
      moved to OnStart, content regenerated, new mid-read pin test.

## Manual acceptance (batched for the user at Finish)

- (pending) 20260722-163542: playtest the tutorial (shakedown) - do the
  instructional objectives now land as you read to the keypress, and do the
  threat reveals still get their beat? The gap constants are tunable if a value
  feels off.
