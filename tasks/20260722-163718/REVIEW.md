## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Checked the branch against master and the task spec, ran the full check suite, and re-derived the load-bearing claims rather than trusting NOTES.md.

Check-suite results (all green):
- `cargo fmt --check -p nova_assets -p nova_gameplay`: clean (exit 0).
- `cargo check -p nova_assets -p nova_gameplay`: clean (only the pre-existing proc-macro-error2 future-incompat warning).
- `cargo test -p nova_assets --lib scenario::`: 22 passed, 0 failed (incl. the new `instruction_objectives_land_mid_read_not_after_the_full_reveal_gap` pin).
- `cargo test -p nova_assets --test content_ron_parity`: 2 passed (the guard --lib skips; ran explicitly - committed RON matches builders, and base bundle ships exactly the generated files).
- `cargo test -p nova_gameplay --lib hud::comms_panel`: 5 passed.
- `cargo run -p nova_assets lint`: 0 errors, 1 warning, 1 finding (all pre-existing the-ledger acks, unrelated to this task).

Constants and derivation (pacing.rs): REVEAL_GAP = (COMMS_DWELL_SECS + COMMS_FADE_OUT_SECS) = 8.4; INSTRUCTION_GAP = COMMS_MIN_SECS = 4.0; MID_GAP = (COMMS_DWELL_SECS + COMMS_MIN_SECS)/2 = 6.0. Ordering INSTRUCTION (4) < MID (6) < REVEAL (8.4) holds; all three derive from comms_panel constants so they cannot drift. COMMS_MIN_SECS was correctly made `pub` and added to the comms_panel prelude. BEAT_GAP was fully migrated (no dangling alias); all four scenarios import the new names.

Classification vs. RON (re-derived from the generated content, not the notes):
- shakedown_run.content.ron: exactly 8x Number(4.0) [INSTRUCTION - beats 1->2,2->3,3->4,4->5,5->6,7->8,8->9 plus the OnStart seed] and 2x Number(6.0) [MID - 6->7 coast, 9->10 paint]; the scavenger telegraph stays 8.4 (no diff line). Matches TASK.md.
- final_tally.content.ron: survey 8.4->6.0 and picket 8.4->6.0 (MID); cast-off/break stays 8.4 (REVEAL). Matches spec.
- broadside.content.ron: contact_gate 8.4->6.0, and the ObjectiveMarkerAttach(hauler) moved out of the gated contact handler INTO the OnStart block (verified in the RON, not just the .rs) - so the nav marker exists during the distress line while only the objective TEXT waits. Defend gate and gunship gate stay 8.4 (REVEAL).
- lifeline.content.ron: byte-identical to master (empty `git diff --stat`), confirming the .rs change is a no-op rename BEAT_GAP->REVEAL_GAP (both 8.4).

Marker hand-off: there is no dedicated broadside marker test anywhere in the repo (grepped crates/*/src), so nothing needed updating there; the RON move is the correctness evidence. The shakedown `the_marker_rides_every_leg_and_hands_off` test is shakedown-scoped and passed. The OnStart seed value change (8.4->4.0) technically differs from TASK.md's "seed stamp stays as-is" note, but is harmless: OBJ_B1 posts inline (ungated) at the opening handoff, and the first gate reader beat_setup(2.0) only fires after the 1->2 transition re-stamps VAR_GATE, so the seed value is overwritten before it is ever read (it exists only to avoid an undefined-read). Not a defect.

Test meaningfulness: the new pin advances only INSTRUCTION_GAP past the beat 1->2 transition, asserts OBJ_B2 is absent just before and present just after, and self-guards `INSTRUCTION_GAP + 1.0 < REVEAL_GAP`. Reverting beat 1->2 to REVEAL_GAP would push the deadline to 8.4s and the final `has_obj(OBJ_B2)` assert would fail - so it genuinely pins the split. `settle_beat` was widened to `now + REVEAL_GAP + 1.0` (the longest gap), so it still clears every category and the beat-walk + not-posted-before/posted-after guards still hold. The `_and_beats_breathe` gate-stamp count uses `>= 9` counting VariableSet on VAR_GATE regardless of delay value, so per-beat delay changes do not disturb it. No tests were weakened.

OOM honesty: the claim that changing timing constants + moving a marker cannot cause a wgpu render OOM is sound. The diff is pure content data (delay literals and which handler emits an already-existing ObjectiveMarkerAttach); no new render resources are introduced, and the marker moving earlier means at most one marker exists slightly sooner. Decisively, lifeline's RON is byte-identical to master yet the notes report lifeline OOMs at the same frame as broadside - an unchanged scenario failing identically proves the failure is independent of this change (environmental host render limit), not a regression.

Clean, well-scoped, correctly derived. No open findings.
