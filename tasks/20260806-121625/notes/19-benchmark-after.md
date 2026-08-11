# Benchmark after - reading

The after-run of `benchmark/`, transcripts captured 2026-08-09 against the
post-epic tree (images built from `ff56c0fb`), graded 2026-08-11 under task
20260809-213441 with the re-keyed rulers: `keys/tier1.json` re-derived at HEAD
`f8d95128`, `keys/tier2.md` re-derived against the post-refactor crates, and
`grade.sh` grading k=3 (mean) on Ownership and No-phantom-structure (H1,
`grade.sh:134`). The baseline was re-aggregated under the same key, so both
columns below are like-for-like. The first grading of this run used the stale
baseline key; those numbers are void and must not be quoted (see TASK.md of
20260809-213441 for the autopsy).

Status: complete. Four agent personas both tiers, `modder` tier 3 with owner
verdict. No `owner` persona in this run.

## Headline

| Persona | tier 1 base -> after | calls | tier 2 mean base -> after |
| --- | --- | --- | --- |
| `blind` | 0.99 -> 0.96 | 40 -> 36 (334s -> 214s) | 0.91 -> 0.94 |
| `rustdoc` | 0.93 -> 0.94 | 52 -> 40 (607s -> 406s) | 0.88 -> 0.86 |
| `tree` | 0.87 -> 0.88 | 3 -> 3 | 0.74 -> 0.77 |
| `docs` | 0.75 -> 0.58 | 25 -> 31 | 0.73 -> 0.63 |

`modder` tier 3: **PASS** (`results/after/modder/tier3/verdict.json`). Lints
clean, loads, plays. One authoring defect - the mod's hull section sits in the
Racer Controller slot (`racer_cube_i0_j1_k0`) - ruled a channel gap, not a mod
failure: the slot-to-section mapping exists only in `nova_authoring` and the
`assets/base/` prototype names, neither staged in the image. The run's own
GAPS.md gap 3 predicted it verbatim. `sandbox.sh` now stages `assets/base/`.

Cost: $24.86, 51 min, 17 containers (baseline: $27.14, 59 min). Network hits: 0.

## The reading

- **Source channels flat at ceiling, ~30% cheaper.** Tier 1 was declared a
  regression guard in note 18; it did not regress. Same comprehension, less
  navigation: blind 334s -> 214s, rustdoc 52 -> 40 calls.
- **Tier 2, the declared headline, moved the right way for the channels the
  epic touched.** blind 0.91 -> 0.94 with zero missed Required and zero phantom
  paths across all three papers. Completeness - the one dimension note 18 says
  a delta can be read from - is flat-to-up for the source channels, with the
  tier2c cell up 0.85 -> 0.95 (blind) and 0.81 -> 0.95 (rustdoc) on the
  corrected emitter-seam row.
- **docs -0.17 is stale prose, not structure.** The misses cluster exactly on
  surfaces the epic moved: `t1-014` answers pre-refactor
  `nova_assets/src/sections.rs` where the key now reads
  `nova_authoring/src/sections.rs` (the same path is the channel's only
  phantom); `t1-005` denies the new nova_hud -> nova_ui theme edge; `t1-016`
  misses the new `nova_scenario/src/lint/`; `t1-012` misses
  `nova_invariants()`. And the channel fails **confidently**: 3 zeros (baseline
  0), two of them assertions rather than gave-ups. The transcripts predate the
  `712f4275` wiki sync (20260809-213446), so the number measures a wiki that no
  longer exists. A docs-only re-run is the cheap confirmation.
- `aggregate.py`'s stated failure mode ("docs rising alone") fired in reverse,
  as designed: the tree held while the prose fell.

## The four deleted questions are findings

`t1-008`, `t1-023`, `t1-026`, `t1-027` are deleted from `keys/tier1.json`, not
re-keyed. Each probed a defect the epic fixed, so their pre-fix `expect`s
inverted and correct after-answers scored 0:

| Id | Probed | Fixed by |
| --- | --- | --- |
| t1-008 | the hud folder misplacement | the nova_hud extraction |
| t1-023 | the menu list+details triplication | L7 |
| t1-026 | the render gate that gated nothing | F47 |
| t1-027 | the debug feature leaking into every build | F52 |

Four questions the benchmark could only ask about the broken tree. Their
inversion is direct evidence the epic fixed what it targeted.

## B1-B6 verdicts

- **B1: unmoved by the epic, overturned by the re-key.** The baseline key
  required the `OnDocked`-style emitter in `nova_gameplay`; the tree said
  otherwise at `89c049fd` already and says otherwise now -
  `nova_scenario/src/objects/area.rs` detects and fires area events itself.
  The 4/4 "miss" was a key error, not a persona error. Against the corrected
  row, blind and rustdoc both model the seam correctly (tier2c 0.95/0.95, no
  missed Required). What survives of B1 is one stale line:
  `nova_events/src/lib.rs:7` still says "`nova_gameplay` emits these events".
- **B2: stands, derived channels only.** tree and docs still miss the NOVA OS
  registration chain (now `nova_os_ui/src/terminal/mod.rs:147`, the map/ship
  `.register` sites, `nova_menu/src/lib.rs:113`); blind misses nothing;
  rustdoc drops only the `init_resource` site.
- **B3: half-recovered.** tree reaches `nova_events/src/engine.rs` this run
  (its remaining tier2c miss is the event-kind pair at `lib.rs:244-251`);
  docs still cannot.
- **B4: transformed.** The baseline phantom - docs inventing
  `nova_os/src/apps/*` - is gone, partly because the epic built the structure
  the prose guessed at. The after phantom is staleness (the pre-refactor
  sections path). New signal in the other direction: **rustdoc now invents
  file layout** - three plausible-but-wrong paths in tier2a
  (`nova_hud/src/hud/mod.rs`, `nova_editor/src/ui.rs`,
  `scenario_generation/`), no_phantom_structure 0.52 - because rustdoc sees
  the API surface, never the file tree. blind: zero phantoms. Source access,
  not API knowledge, is what kills phantom structure.
- **B5: fixed by the epic.** The menu triplication collapsed (t1-023 above).
  Max-scroll has one owner: `nova_ui::screen::max_scroll_y`
  (`screen/scroll.rs`), and `nova_os_ui`'s terminal calls it
  (`shell.rs:456`) instead of recomputing.
- **B6: partial.** rustdoc 52 -> 40 calls, 607s -> 406s, score 0.93 -> 0.94.
  Near blind's 36 calls but not below it. The public API got cheaper to
  navigate; it did not get cheaper than the source.

## Epic verdict

Close 20260806-121625 as a success:

- Tier 1 guarded: no regression, and the four deleted questions certify four
  targeted defects fixed.
- Navigation cost down ~30% on the source channels at flat scores.
- Tier 2 up where it should be (blind at ceiling with clean Required and
  phantom columns), B5 fixed, B6 improved.
- The external modding contract held (tier 3 PASS on the same brief and wiki).
- The one regression (docs) is prose staleness, already addressed by
  20260809-213446/20260809-213449 after the transcripts were captured.

## Carried out of this run

- Docs-only re-run against the synced wiki to confirm the -0.17 recovers.
  Cheapest possible check of the 213446/213449 work.
- Modder re-run picks up the `assets/base/` staging; the controller-slot class
  of error becomes knowable.
- `nova_events/src/lib.rs:7` stale emitter sentence - one-line doc fix.
- Open harness items from note 18: H2 (delta sign in `aggregate.py`), H3
  (tree's pinned call count), H4 (stage the two `guide-author-*` pages or
  reword the tier 3 brief), H5 (baseline records `model: "default"`; after
  records `claude-opus-5`; transcripts say both ran opus - pin it if runs
  continue), H6 (results live only on the owner's disk).
