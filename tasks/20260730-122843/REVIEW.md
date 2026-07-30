# REVIEW: dock shows only currently-available verbs

- VERDICT: APPROVE
- ROUNDS: 1
- REVIEWER: out-of-context agent (round 1)

## Round 1

Reviewed the single-commit branch out of context against TASK.md, DECISION.md,
NOTES.md, the full diff and the whole of `keybind_dock.rs`. The reviewer ran the
module's tests and four independent mutation experiments, and did its own doc
grep rather than trusting the task's claim.

### Findings

**R1.1 MINOR - `keybind_dock.rs` hidden branch: the plan deviation was
unpinned, and in the only state where it differs it is the worse choice.**
Reverting the hidden branch's `state.set_if_neq(next)` back to
`set_if_neq(DockChipState::Dim)` left all 14 tests green - the only substantive
part of the change that survived deletion. Not inert, either: `chip_visible`
guarantees a `Hot` chip is on screen unless the key is empty, so the sole
divergence is "no flight rig while `HudSituations` still reports a maneuver",
and in that frame `grow_hot_chips` holds an OFF-SCREEN chip grown (it pops in
mid-shrink when the rig returns) and the pulse restore writes inverted `Hot`
paint onto it. Safe today only because `sense_hud_situations` resets to idle
with no player ship - a cross-module invariant nothing in the file stated.

FIXED, taking both halves of the reviewer's suggestion: the hidden branch writes
`Dim` again (identical for every reachable hidden chip, harmless for the
unreachable one) with a comment naming the `sense_hud_situations` invariant, AND
the case is now pinned by `a_chip_that_leaves_the_dock_stops_being_hot`, which
despawns the rig mid-ORBIT and asserts the chip is hidden, no longer `Hot`, and
no longer held grown. A/B-verified: that test goes red against the `next`
variant it was written to rule out.

**R1.2 NIT - `pulse_emphasized_chips` re-derived half of the visibility rule.**
Its gold branch tested `!hint.key.is_empty() && emphasis.contains(verb)`
independently, so the "one place answers is-this-chip-rendered" claim in
`chip_visible`'s doc was not quite true and the two could drift.

FIXED: the gold branch now calls `chip_visible(...) && emphasis.contains(verb)`
(logically identical today), with the comment saying why it asks rather than
re-derives - this system cannot see `Node`.

**R1.3 NIT - the module doc contradicted itself.** "the docked chips read in two
states - available and hot" versus the spotlight paragraph eight lines below: an
emphasized unavailable chip IS docked and IS painted from the `Dim` band.

FIXED: "In normal play a docked chip reads in two states ... plus the dim band a
scenario spotlight can reveal".

**R1.4 NIT - third copy of the same silent index clamp** (`verb_hint`'s `_` arm,
`pulse_emphasized_chips`, and the new `chip_visible`).

FIXED: extracted `dock_verb(index)` next to `verb_hint`; both name-lookup sites
call it.

**R1.5 NIT - the emphasis-band assertion only bounded alpha from above**, so a
fully transparent chip would have passed.

FIXED: the assertion is now a closed range over `EMPHASIS_ALPHA_UNAVAILABLE`.

### Confirmed positively by the reviewer

- Mutation results: stubbing `chip_visible` to `true` reddens both new rigs;
  dropping the `emphasis.contains(...)` clause reddens the emphasis rig;
  dropping `&& !emphasis.is_changed()` from the change gate reddens the emphasis
  rig. NOTES.md's A/B claim checks out and the quiet-frame sequencing really
  does isolate emphasis as the only changed input.
- Visibility holds across state combinations: the only hide paths are key-empty
  and unemphasized-`Dim`; every show path repaints before the chip can be seen,
  so a chip carrying stale paint out of the dock can never be displayed with it.
  No stuck-gold or one-frame-wrong-paint case found.
- Docs: every live surface checked independently and consistent.
  `CHANGELOG.md:272` ("rows appear only while their verb can act") is a
  historical entry this change RESTORES rather than contradicts, and
  `input/player.rs`'s `rcs` field doc ("the row shows only where RCS is
  enabled") is true again.
- Deleted assertions: the only removed one ("dim chips stay on screen") was
  deliberately inverted and re-homed in `unavailable_verbs_leave_the_dock`; the
  host test was re-fixtured with `all_available_hints()` so it still sweeps all
  seven keycaps.
- The exception is load-bearing on shipped content:
  `assets/base/scenarios/shakedown_run.content.ron:1341` emphasizes `GOTO`,
  which is unavailable until the player has a nav lock - exactly the case
  DECISION.md's Option 1 preserves.

Post-fix: 15 tests green, `cargo check --workspace --all-targets` clean,
`cargo fmt --all --check` clean.
