# KISS: nova_gameplay flight, camera, audio, juice

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.9.0, refactor, chore, gameplay

## Story

As a maintainer I want the flight, camera, audio and juice layer to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/*.rs (flight, camera_controller, gravity, juice, audio, damage, settings, plugin, lib, and the small siblings)
Current size: ~13.8k lines across 13 files. Largest file: flight.rs at 5812 lines.

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
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/*.rs (flight, camera_controller, gravity, juice, audio, damage, settings, plugin, lib, and the small siblings)` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_gameplay/src/*.rs (flight, camera_controller, gravity, juice, audio, damage, settings, plugin, lib, and the small siblings)` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

## Close-out

**What / why.** Split the three oversized files into folder modules -
`flight.rs` (5812) into `flight/` with a `flight/tests/` folder for the
physics-level integration suite, `audio.rs` (2264) into `audio/` by cue
family, `camera_controller.rs` (1752) into `camera_controller/` by concern -
and applied the epic's comment rubric across all 36 files in scope.
`gravity.rs` and `juice.rs` were left whole: both are ~1050 lines and one
cohesive concern each. Public paths, preludes and behavior are unchanged; the
only edited lines are visibility keywords, imports and comments.

**Alternatives.** Splitting `gravity.rs`/`juice.rs` on the prod/tests seam was
considered and rejected - it would have been motion, not simplification, and
the epic's rule is "more than one concern", not "over some line count".
Keeping flight's integration tests co-located with the production file each
one happens to touch was also rejected: they build the whole plugin and assert
on hull motion, so they belong to the module, not to any one file.

**Difficulties / diagnosis.** The mechanical splits surfaced three classes of
breakage, each fixed by narrowing rather than widening: (1) private items that
now cross a boundary - widened to `pub(super)`, `pub(crate)` only where an
out-of-module caller already existed; (2) items used ONLY from `#[cfg(test)]`
call sites in other modules (`flight::hull_turn_rate`) - re-exported under
`#[cfg(test)]` so the lib target stays warning-free; (3) a first cut of
`audio/` that put mixing constants in `mod.rs`'s import group when they live
in `mixing.rs`, plus `SfxThrottle`'s inherent methods that a column-0
visibility sweep missed. The comment pass then needed a re-wrap step: deleting
a mid-paragraph clause leaves ragged lines, and rustfmt does not touch
comments. Paragraphs whose text is unchanged were restored to master's exact
line breaks so the diff carries no cosmetic churn.

**Evidence.** `cargo check --workspace --all-targets` green; `cargo fmt
--check` clean; per-module tests all pass (flight 75, camera_controller 14,
audio 30, gravity 18, juice 21, settings 8, damage 8; 0 failed). `#[test]`
count conserved exactly across the three splits: 119 before, 119 after.
`grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the scope returns zero hits. No file
over 1500 lines (largest: juice.rs 1050). Full record in NOTES.md.

**Reflection.** The split method that worked was mechanical and verifiable:
map exact line ranges of the original, emit each new file by concatenating
ranges under a hand-written header, then iterate on `check --all-targets` and
let the compiler find every boundary crossing. Because no line is retyped,
"did behavior change" reduces to reading the visibility and import diff. The
comment pass is the opposite - it is judgement per site, and the useful
discipline was to delete the citation while keeping the sentence, since a
clause like "measured at 0.6 and 0.75" earns its place and "(spike
20260711-140234)" does not.
