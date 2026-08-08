# Retire the diegetic objective reveal card: the top-centre chip IS the posting

- STATUS: CLOSED
- PRIORITY: 53
- TAGS: v0.9.0, feedback, ui, hud

## Story

Owner playtest (2026-07-29) of 20260729-163816 (the objective chip stack): "the
objective still appears first as a big thing on the spaceship HUD, lets leave it
only in the top middle as a 'chat message' like popup".

The big thing is the diegetic reveal card (`hud/objective_reveal.rs`, task
20260721-211520): a posting spawns a rotated 1.35x card at 0.5/0.34 of the
viewport that appears (0.35s), holds (2.3s) and tucks (0.55s) into the stack,
and only THEN does the chip appear and pop (`pop_chip_on_reveal_tuck`; a posting
with no card hands over after `REVEAL_TOTAL_SECS` anyway). The owner wants the
chip to BE the posting: a top-centre notification, no cockpit card.

## Steps

- [x] Retire the card: delete `hud/objective_reveal.rs`, its plugin
      registration in `hud/mod.rs`, and its `spawn_objective_reveal` call in
      `objective_feedback.rs`. Record the removal in DECISION.md (it supersedes
      the presentation half of 20260721-211520, which stays CLOSED as history).
- [x] Post the chip immediately: drop the handover gate in `objective_stack.rs`
      (`ObjectiveRevealTucked`, `pop_chip_on_reveal_tuck`, `hand_over` /
      `handed_over` / the `REVEAL_TOTAL_SECS` fallback) so a posting spawns its
      chip and starts its pop on the SAME frame, and the read dwell runs from
      the posting. Keep everything else the stack already does: newest-first
      stacking, pop then breath, read-by-dwell or by opening NOVA OS, the TAB
      keycap riding the stack, completion dropping the chip.
- [x] Remove what only the card used: `NovaOsTabAnchor` (defined in
      `hud/nova_os.rs`, published by the stack, consumed ONLY by the card) and
      the stack's `publish_*` anchor system - grep to prove no other consumer
      before deleting.
- [x] Fix the tests that encode the card: `objective_feedback`'s
      `ObjectiveRevealMarker` assertion, and the stack tests that advance by
      `REVEAL_TOTAL_SECS` - the chip must now be asserted present on the frame
      of the posting.
- [x] Doc sweep (keep-docs-in-sync): wiki `hud.md`, CHANGELOG [Unreleased]
      (three lines name the reveal card / "pops as the card tucks in" - rewrite
      them from the final diff), `web/src/index.html` if it says it. Leave
      dated history (tasks/, released CHANGELOG sections) alone.

## Definition of Done

1. test: App-driven - a posting spawns its chip on the SAME frame (no
   multi-second wait), it pops and settles to the breath, and no reveal-card
   entity is ever spawned.
2. test: the existing stack behaviour still holds - two postings stack, a chip
   leaves after its dwell, opening NOVA OS marks every shown chip read, a
   re-worded objective re-posts unread.
3. cmd: `grep -rn "objective_reveal\|NovaOsTabAnchor\|ObjectiveRevealTucked"
   crates/ examples/ tests/` returns nothing (task records excluded).
4. cmd: `cargo run -p nova_probe -- run playable` passes. Run 2026-07-30 on
   this branch: verdict OK, 5/6 - process_exit, run_completed, reached_playing,
   invariants_held (0 violations over 1382 frames) and log_clean (0 panic/ERROR
   lines) all PASS; fps_within_baseline SKIPPED (no baseline capture, i.e. not
   measured).
5. manual: owner playtest - a posted objective shows ONLY as the top-centre
   chip, arriving like a chat notification, with no cockpit card first.

## Notes

- Follow-up to 20260729-163816 from the owner's playtest; the chip stack itself
  is the shipped shape and is NOT being redesigned here - this only removes the
  card that precedes it and the delay it imposed.
- The completion green ghost (`objective_feedback`) and the world-anchored
  `objective_markers` chips are untouched.
