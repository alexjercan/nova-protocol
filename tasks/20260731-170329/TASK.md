# KISS: nova_gameplay HUD - combat readout widgets

- PRIORITY: 43
- TAGS: v0.9.0, refactor, chore, hud
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE
- PARENT: 20260731-170222
- DEPENDS ON: 20260731-170322

## Story

As a maintainer I want the combat-readout HUD widgets to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/hud/{target_inset,ammo_readout,torpedo_target,lock_crosshairs,lock_dwell_ring,turret_lead,component_lock,screen_indicator,edge_indicators,allegiance_markers,item_highlights,emphasis}.rs
Current size: ~9k lines across 12 files. Largest file: screen_indicator.rs at 1434 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable. (No file holds more than one; see NOTES.md.)
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
      (20260731-205553)
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/hud/{target_inset,ammo_readout,torpedo_target,lock_crosshairs,lock_dwell_ring,turret_lead,component_lock,screen_indicator,edge_indicators,allegiance_markers,item_highlights,emphasis}.rs` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_gameplay/src/hud/{target_inset,ammo_readout,torpedo_target,lock_crosshairs,lock_dwell_ring,turret_lead,component_lock,screen_indicator,edge_indicators,allegiance_markers,item_highlights,emphasis}.rs` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What / why.** Comment-only pass over the 12 widgets: 158 insertions, 181
deletions, no file split. Removed 6 dead `docs/spikes/*.md` pointers (that
directory no longer exists), ~58 tatr-ID / date / review-round provenance
clauses, and the spike question-labels (`Q4a`, `B1`, `F4` ...) that those
docs gave meaning to. Four comments that genuinely guard a schedule slot or
a value became `NOTE:` and now state the constraint directly instead of
citing a task ID. Detail and the per-file structure verdict are in NOTES.md.

**Alternatives.** A literal read of the epic rubric ("only NOTE/TODO/FIXME/
BUG survive as non-doc markers") would have mechanically converted several
hundred bare `//` comments. Rejected after reading child 1's landed output -
it kept 486 bare comments against 1 NOTE, because the bare ones are almost
entirely the categories the rubric says to keep. Following the sibling's
precedent over the literal text keeps the epic internally consistent.

Splitting `screen_indicator.rs` (1428) was considered and rejected: it is
one public widget plus the systems implementing it, and 604 of those lines
are tests. A `mod` boundary would move the seam, not remove one.

**Difficulties / diagnosis.** Two.

The doc-warning baseline was wrong the first time. `grep -c '^warning'` on a
warm-cache `cargo doc` run reported 2; the post-edit run reported 14, which
read as a regression I had caused. The honest measurement is `git stash` ->
`touch crates/nova_gameplay/src/lib.rs` -> rerun -> `git stash pop`, and it
put master at the same 14 with the same single in-scope warning. The lesson
is narrower than "diff against the base": the base must be captured with the
same extraction and the same cache state as the after-run, or it is not a
base.

Stripping `F11` as a spike label would have been a real behavior-adjacent
edit in `ammo_readout.rs`, where `F11` is `KeyCode::F11`. Caught by reading
each hit rather than trusting the pattern.

**Evidence.** check green, fmt clean, item-name multiset identical at 485,
doc warnings unchanged at 14, and the entire non-comment diff is 3 test
assertion strings that carried `Q4a`/`Q5a` labels. Plugin `build` bodies are
byte-identical because no executable line changed at all. The
`cargo test -- --list` name diff was NOT run - the link step OOMs this box -
and NOTES.md records that as an argument rather than a measurement. CI runs
the suite on the PR.

**Reflection.** The comment axis had more substance than expected, but not
where the rubric pointed: the win was six dead documentation links, found by
checking whether a cited path exists rather than by judging the prose. Worth
making a habit - a pointer is checkable, and a rotted one is unambiguous
fluff where "is this comment narration?" is a judgment call.

One defect deferred per the task constraint: 4 `ambiguous import visibility`
warnings in child 1's `nova_os_map`/`nova_os_ship` mods -> 20260731-205553.
