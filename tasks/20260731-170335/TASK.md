# KISS: nova_gameplay HUD - chrome and objective surfaces

- STATUS: CLOSED
- PRIORITY: 42
- TAGS: v0.9.0, refactor, chore, hud
- KIND: STORY
- FLOW STEP: DONE
- PLAN STATUS: APPROVED
- PARENT: 20260731-170222
- DEPENDS ON: 20260731-170329

## Story

As a maintainer I want the HUD chrome and objective surfaces to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/hud/{mod,keybind_dock,objective_stack,objective_markers,objective_feedback,velocity,comms_panel,maneuver_instruments,flight_status,beacon_chips,holo_instruments,chip_layout_rig,key_glyphs,readout,situation}.rs
Current size: ~10k lines across 15 files. Largest file: keybind_dock.rs at 1913 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*([0-9]{8}-[0-9]{6}|[0-9]{4}-[0-9]{2}-[0-9]{2}|review R[0-9]|docs/)' crates/nova_gameplay/src/hud/{mod,keybind_dock,objective_stack,objective_markers,objective_feedback,velocity,comms_panel,maneuver_instruments,flight_status,beacon_chips,holo_instruments,chip_layout_rig,key_glyphs,readout,situation}.rs` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md. Widened from
   the tatr-ID-only form the previous child shipped: that grep was narrower
   than the claim it gated and let bare dates, `(review Rn.n)` clauses and
   dead `docs/` pointers through (20260731-170329 R1.4).
4. cmd: `wc -l crates/nova_gameplay/src/hud/{mod,keybind_dock,objective_stack,objective_markers,objective_feedback,velocity,comms_panel,maneuver_instruments,flight_status,beacon_chips,holo_instruments,chip_layout_rig,key_glyphs,readout,situation}.rs` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What / why.** Comment-only pass over the 15 chrome and objective-surface
files: 240 insertions, 271 deletions across 134 hunks for the branch as a
whole (the pass commit was 217/245; the review round's reflows account for
the rest), no file split. Removed
the tatr-ID clauses, the bare-date and playtest provenance the previous
child's narrower grep could not see, the `(review Rn.n)` clauses, and two
record pointers (`tasks/20260730-122843/DECISION.md`, "see the task's
DECISION.md"). One dead-history block in `mod.rs` - four lines naming four
functions removed a week ago - was deleted outright; the fact that survives
it is already stated at the `ObjectivesPlugin` registration above. Twelve
comments that guard a schedule slot or a value became `NOTE:` and now state
the constraint directly. Per-file structure verdict and the full marker
inventory are in NOTES.md.

**Alternatives.** Splitting `keybind_dock.rs` (1911) was the obvious move and
was rejected: the dock and the verb cues share their keycap SIZING path, and
`keycap_sizing_tests` exists precisely to assert that they do. Splitting
either breaks that guarantee or requires extracting a shared rig - a new
abstraction this pass forbids. DoD 4's escape hatch (justify the exception in
NOTES) is the right answer, and NOTES names the seam to extract if the file
is split later. `mod.rs` at 1463 is under the threshold and is one concern,
so it needed neither a split nor an exception.

As with the sibling, a literal read of the epic rubric would have converted
several hundred bare `//` comments to markers; both earlier children rejected
that and this one follows, for the same reason - read one by one, the bare
comments are the categories the rubric says to keep.

**Difficulties / diagnosis.** The pass inherited 13 files already edited by a
scripted substitution, and that substitution had left four comment lines past
the file's fill because it deleted a clause without rewrapping. One of them,
`keybind_dock`'s `EMPHASIS_PERIOD_SECS` doc, put a sentence dash at column 1,
which CommonMark parses as a list item - the exact R1.1 finding from the
previous child, reproduced by a tool rather than by hand. Found by grepping
added comment lines for length and for a leading `- `, not by reading; that
check is worth keeping as a standing post-substitution step.

The prompt also warned that the substitution had silently duplicated a clause
in `beacon_chips.rs`. Re-read both edited regions of that file: no
duplication present, so it was either already repaired or a false alarm.
Recorded rather than assumed either way.

The review round then caught the same class of mistake one level up: fixing
the ragged wrapping SHORTENED three files by a line each, which silently
falsified the prod/tests table in NOTES.md that had been measured before the
reflow. A record full of measured line numbers has to be re-measured after
any edit to the files it measures, not just after the edit that motivated it.

**Evidence.** check green (exit 0), fmt clean (exit 0), the widened DoD 3
grep returns exactly one hit - the deliberate `TODO(20260710-231927)`, which
DoD 3 permits because NOTES.md lists it - and the whole non-comment diff is
ONE deleted blank line. `cargo test --lib -p nova_gameplay` passes 785/785 (1
ignored) and the `hud::` filter passes 307/307 - the tests DID run this time,
because this branch carries the link-RAM fix (20260731-210651). The workspace
suite was left to CI; a local full run exits 101 on the known pre-existing
`nova_assets` shakedown failure (20260731-215407) regardless of this diff.

**Reflection.** The widened DoD 3 grep earned its keep immediately: of the
provenance removed here, the bare-date and playtest clauses (`(owner playtest
2026-07-30)`, `(playtest round 4)`, `measured ... on 2026-07-30`) would all
have survived the tatr-ID-only form the previous child shipped. Widening the
proof command to match the claim it gates, rather than widening the claim,
was the cheap fix.

One defect found and deferred per the task constraint: `holo_instruments`'s
ribbon doc may be describing a superseded state now that the gravity-aware
arrival task is CLOSED -> 20260731-232634. The 4 pre-existing `ambiguous
import visibility` warnings remain filed as 20260731-205553.
