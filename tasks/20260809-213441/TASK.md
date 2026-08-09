# Re-key the benchmark, draw the after-run conclusions, close the refactor

- STATUS: OPEN
- PRIORITY: 39
- TAGS: v0.10.0,benchmark,tooling,project

PROBLEM: The 20260806-121625 after-run was graded with a stale ruler, so the
raw numbers read as a regression that is mostly not real. Re-key the benchmark
properly, re-grade from the existing transcripts, write the conclusions, and
close the refactor epic as a success.

What is wrong with the current after-run measurement:

- `keys/tier2.md` was never re-derived. Its Required surfaces still cite
  `nova_gameplay/src/sections/`, `nova_gameplay/src/hud/`,
  `nova_assets/src/sections.rs`. The graders punished answers that correctly
  named `nova_ship`, `nova_hud`, `nova_os_ui` and `nova_authoring` - the crates
  the epic created. Every collapsed tier 2 cell (rustdoc 2a/2b, tree 2a/2b,
  docs 2b) is this artifact.
- Four tier 1 questions kept a pre-fix `expect` after the epic fixed the defect
  they probe: t1-008 (hud folder), t1-023 (menu triplication, collapsed by L7),
  t1-026 (render gate, made real by F47), t1-027 (debug feature, fixed by F52).
  Correct after-answers scored 0 on all channels. The re-key policy said
  "record as a FINDING, never retarget", but the pipeline has no mechanism, so
  the questions stayed live. Build the mechanism or flip the expects.
- H1 (grade k=3 for Ownership and No-phantom-structure) was declared required
  before the after-run and never implemented; `grade.sh` has no loop.
- `modder` tier 3 has no `verdict.json` (row reads not-verdicted).
- Channel bug, owner ruling: `stage_modder` (`sandbox.sh:136`) stages 4 wiki
  pages plus `webmods/` only. A real modder can read the base mods from the
  game's assets folder, so stage `assets/base/` too. This is what made the
  controller-cube swap unknowable: `racer_cube_i0_j1_k0` is the Racer
  Controller, and that fact exists only in `nova_authoring` and in the
  `assets/base/` prototype names, both outside the channel. GAPS.md gap 3
  predicted the failure verbatim.

Corrected tier 1 reading (inverted questions excluded): blind 0.99 -> 0.96,
rustdoc 0.93 -> 0.94, tree 0.87 -> 0.88, docs 0.75 -> 0.58, with navigation
cost down ~30% on the source channels (blind 334s -> 214s, rustdoc 52 -> 40
calls). Source channels flat and cheaper; docs regressed because the wiki
prose is stale (separate task); tier 2 unreadable until re-keyed; tier 3
surfaced the best finding of the run.

Scope:

- Re-key tier 2 (and tier 3 pass criteria if needed) against the current tree.
- Fix or retire the four inverted tier 1 questions with a real mechanism.
- Implement k=3 grading for the judgement dimensions.
- Re-run the grade step only - all transcripts are on disk; grading containers
  are a fraction of a persona run.
- Stage `assets/base/` into the modder image for the next run.
- Write `verdict.json` for the after modder run (owner).
- Write `tasks/20260806-121625/notes/19-benchmark-after.md` with the corrected
  deltas and the B1-B6 verdicts (B1 unmoved, B6 partial: rustdoc 52 -> 40
  calls, near blind but not below).
- Close 20260806-121625 as a success.
