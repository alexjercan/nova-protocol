# Nova Review of the unpushed code range: WFC, scenario vocabulary, occlusion

- STATUS: OPEN
- PRIORITY: 71
- TAGS: v0.13.0, review

## Goal

Run Nova Review over the code-carrying commits in `origin/master..HEAD` (47
commits total; 10 carry code). Story, lore, comic, and task-only commits are
out of scope by owner instruction. The code range is ~17k reviewable lines, far
past the one-pass limit, so it is split into seven batches that run in sequence.

This file is the shared ledger: the plan, then one findings block per batch,
then the adjudicated verdict and the fix pass.

Owner framing (2026-09-05): review only the code commits that add features; do
not review story content; at most two agents in flight at any time; group the
commits into logical chunks; record findings here between runs and fix them at
the end.

## Rules for this run

- Two agents per batch, dispatched in one message, nothing else concurrent.
  - Lane A: correctness + performance. Holds the measurement slot.
  - Lane B: craft + contracts. Never runs a rendered example or a probe.
- Reviewers are read-only. No edits, no staging, no commits.
- No workspace test suite, no workspace Clippy.
- `assets/base/**/*.content.ron` is generated. Review the Rust builders under
  `crates/nova_authoring/src/`, not the RON.
- HEAD is the truth. The per-commit diffs show intent; a later commit may have
  already replaced code an earlier one added.
- Bundles live in the session scratchpad under `review/`; every lane in a batch
  gets the same path and does not re-derive the range.

## Work groups

The ten code commits fall into four features. Two are too large for one pass and
split further.

| Group | Feature | Commits (oldest first) |
|-|-|-|
| G1 | Line of sight for radar lock and hostile AI | `9de7370a` `a6b73ef6` `f47c385a` |
| G2 | Scenario cinematic vocabulary: scene, channel, cue, title card | `246c4c66` `ce13092d` `02806185` `aceb94c7` `40e5a550` |
| G3 | WFC ship generation in the editor | `aed4fab1` |
| G4 | Examples and probe ranges brought back to what the app does | `95a3d731` |

## Batch plan

Run order is engine-first: the crate a feature stands on is judged before the
code that sits on it.

| # | Batch | Group | Bundle | Lines |
|-|-|-|-|-|
| 1 | `nova_wfc` crate: collapse, tiles, grid, check | G3a | `g3a-wfc-crate.diff` | 2672 |
| 2 | Editor WFC integration: generate, template, files UI, bundle, placement | G3b | `g3b-editor.diff` | 3476 |
| 3 | Ship grammar, lint, merge, modding, WFC examples | G3c | `g3c-content-examples.diff` | 1908 |
| 4 | Scenario vocabulary runtime: events, actions, world, editor event | G2a | `g2a-scenario-runtime.diff` | 3350 |
| 5 | HUD presentation: title card, skip prompt, comms panel, keybind dock | G2b | `g2b-hud.diff` | 997 |
| 6 | Line of sight: occlusion, contacts, radar, AI acquisition, cover | G1 | `g1-occlusion.diff` | 1318 |
| 7 | Campaign content, generated RON, creator docs, changelog, ranges | G2c + G4 | `g2c-content-docs.diff`, `g4-examples-ranges.diff` | 1048 + 410 |

### Not reviewed

The 37 story, lore, comic, and task-only commits carry no game code and are
excluded by owner instruction: `be85d858` `3e7d19f0` `4eb0c999` `a8b2e160`
`7abcd5b5` `1414d071` `59b83b49` `8d1d15dd` `d3bd9de1` `3af5d051` `80ce02e2`
`75f31a66` `e7e59fd1` `db779df5` `ac52d0ce` `e1104ae5` `8bccf8e3` `fbb9699d`
`356024ec` `9ac8ff2a` `03bbe10b` `833fc684` `5f5c1749` `dedcdd55` `57def793`
`43421d55` `b499cc6f` `fee5696e` `50ffa0f0` `bdb12a69` `b0df82d4` `4642cda8`
`a7ba7c9c` `76444e8a` `7f14ddc7` `078deb8f` `10415913`.

`--play` was not requested, so the red team and feel lanes do not run. Nothing
in this range is judged for how it feels to play. Generated art (`web/src/assets/story/`,
`lore/src/sketches/`) is not judged.

## Progress

All seven batches adjudicated; review COMPLETE. Final totals: 4 BLOCKER (a
fourth surfaced during the fix pass), 45 MAJOR, 90 MINOR.

Fix pass COMPLETE for the grouping the verdict scheduled (A-E, 27 fixes,
committed). The findings the verdict did NOT schedule are still open and are
listed under "Still open" at the end of this file.

The owner raised the hardcoded comms channels mid-run; recorded below, ahead of
batches 4-5, and fixed as group C.

## Findings

Each batch appends its adjudicated findings here as it completes.

### Batch 1 - `nova_wfc` crate (`aed4fab1`, `crates/nova_wfc`)

Two lanes, both returned. No BLOCKER. `cargo check -p nova_wfc --all-targets`,
`cargo check -p nova_wfc --target wasm32-unknown-unknown` and
`cargo test -p nova_wfc --lib` (13 passed) are green. Every load-bearing claim
below was re-derived in the main session before it was written down.

**MAJOR - `crates/nova_wfc/src/grid.rs:55` - the grammar gate bounds the grid only from below, so an authored grid overflows the cell count.**
`Grid::cells()` is `(self.size.x * self.size.y * self.size.z) as usize`, a `u32`
multiply. `runnable` (`lib.rs:122-129,146`) checks only lower bounds, and
`lint_grammar_config` (`crates/nova_scenario/src/lint/ship.rs:178-192`) only
floors each axis at 3. A mod grammar with `grid: (half_width: 2048, height:
2048, length: 1024)` lints clean and multiplies to exactly `2^32`: a dev build
panics with `attempt to multiply with overflow`, a release build wraps to 0,
`domains()` returns an empty `Vec`, and `seed_keel` indexes off the end. No
overflow is even needed to hurt - `(200, 200, 200)` allocates 8,000,000 inner
`Vec<bool>` before any refusal is possible. A grammar is content, and the
explicit-authoring rule makes a bad one an error at lint then at load, not a
panic. Fix: a maximum cell count in `runnable` checked in `u64` (or
`checked_mul` in `cells()`), refused in the same shape as the low bounds.
Not a BLOCKER: no shipped grammar and no editor-built grammar reaches it -
`holding()` only grows the shipped 4x5x11 to fit a drive.

**MAJOR - `crates/nova_wfc/src/collapse.rs:327` and `:683` - the crate's two documented refusal paths have no test.**
`lib.rs:30-36` makes "failing rather than photographing" a headline claim. Neither
`"cell {cell} collapsed to nothing"` nor `"the collapse blocked its own exits"`
appears anywhere in `tests.rs` - verified by grep over the crate. No test drives a
grammar into a contradiction or into a self-blocked exit, so a change that made
`solve` return the first `true` in a stale domain, or that dropped `run`'s
`blocked_findings` guard, leaves all 13 tests green while handing out the hull the
module says it never hands out. Same class:
`a_grammar_the_collapse_cannot_run_in_is_refused_rather_than_run`
(`tests.rs:233-239`) asserts only `is_err()` for all eight bent grammars, so it
cannot tell "refused for the stated reason" from "refused somewhere else";
every other refusal test in the file asserts on the message. Fix: one grammar
that forces a contradiction and one whose seeded part fires into its own hull,
both asserting the message.

**MAJOR - `web/src/create/` - `Grammar` ships as a mod-authorable content kind and the creator reference documents none of it.**
`assets/base/base.bundle.ron:16` ships `grammars/base.content.ron` and
`crates/nova_assets/src/merge.rs:545` routes `Content::Grammar` through the same
merge as sections, so a mod can ship `grammars/*.content.ron` today. The
changelog entry promises exactly that ("so a mod ships its own procedural hull
line"). `standard_hull` appears in no document: grepped, it exists only in the
generated RON and in `tasks/20260905-133019/TASK.md`. `docs/keeping-docs-in-sync.md`
rule 3 makes the `/create/` page mandatory in the same task as the format.
Fix: add the `Grammar` construct to the `/create/` reference (grid, vacuum
weights, `keel` roles, per-part `weight`/`aim`/`zone`) and list `standard_hull`
in `base-content.md`'s id catalog.

**MAJOR - `docs/concept-index.md:82` - the WFC row points at a file this commit deleted and a symbol that no longer exists.**
The row names `examples/playable/shared/wfc.rs` with entry symbol `wfc_hull`.
That directory now holds only `compare.rs`, `first_shift_scene.rs` and
`first_shift_stage.rs`, and `grep -rn wfc_hull crates/ examples/ web/` returns
nothing but this row itself. The page's own header claims every row was
verified against the tree. `nova_wfc` appears NOWHERE in `docs/` or `web/`.
Fix: retarget the row at `crates/nova_wfc/src/{lib,collapse,tiles}.rs` with
`TileSet` as the entry symbol, and name the grammar content type beside it.

**MAJOR - `docs/project-tour.md:30` and `docs/architecture.md:12` - a new workspace member is missing from both crate maps and from the dependency graph.**
`project-tour.md:26` states "Slugs are the workspace members. One line each", so
both tables are exhaustive by construction; both run to `nova_meta_gen` without
`nova_wfc`. The mermaid graph at `architecture.md:44-84` draws `editor --> ship /
ui / input` with no `editor --> wfc` edge, though
`crates/nova_editor/Cargo.toml:28` now depends on it.
Fix: a `nova_wfc` row in both crate maps, the `editor --> wfc` edge, a
"Generated hulls / WFC" row in project-tour's change-X table, and a
`keeping-docs-in-sync` row key for `crates/nova_wfc`.

**MAJOR - `docs/ship-layout-sense.md:3` - the design note is written against the deleted file and every constant it cites is gone.**
It declares itself "written against `examples/playable/shared/wfc.rs`" and its
table at `:47-53` cites `HULL_GRID`, `KEEL_ROW`, `LENGTH = 11`, `HEIGHT = 5`,
`VACUUM_BOW_TAPER`, `VACUUM_STERN`, `hull_vacuum_weight`, `keel_prototype` -
none of which exist; they became authored grammar fields. It also opens "the
rest is a proposal", which is now false: the seeded stern drive, the seeded bow
gun and the zone constraint all landed here.
Fix: re-derive against `crates/nova_wfc`, or retire it and say which proposals
shipped.

**MINOR - `crates/nova_wfc/src/collapse.rs:554` - `keel_component` asserts the bow keel cell rather than reading it, so a part erosion dropped can come back.**
`kept[start] = true` is unconditional while every other cell is gated on
`standing[next]` (`:572`). `run` calls it a second time (`:675`) with the
post-erosion `kept` as `standing`, so a dropped start cell is resurrected. For a
multi-cell seeded bow gun, `drop_part` removes all segments and the resurrection
puts back only the emitting one. It cannot fire today: `exit_normal` hard-codes
`SectionKind::Railgun` to `Vec3::NEG_Z`, the gun is seeded at `z = 0`, and its
lane clips to the single cell at `z = -1`, which nothing can fill.
Fix: gate the seed like every other cell, or return the `Err` reserved for a
hull with no spine.

**MINOR - `crates/nova_wfc/src/lib.rs:210` - the `TileSet` cost model is inverted; measured, the build is the cheap half by 6x to 45x.**
The docstring calls reading the catalog "the expensive half". Four repeat runs at
`opt-level = 1`: `TileSet::build` on 7x7x15 is under ~2 ms, one `hull()` on the
shipped 4x5x11 is 11-13 ms, and one over a 5x5x3 capital drive is 92-96 ms. The
advice (build once, collapse many) survives; the reason does not, and a caller
sizing a roster off that sentence budgets the wrong half.
Fix: say the build is hoisted because it is seed-independent, not because it is
expensive. Release-profile figures unmeasured; the ratio is profile-independent.

**MINOR - `crates/nova_wfc/src/collapse.rs:699` and `:341` - the two hot loops carry none of the textbook WFC structure.**
`lowest_entropy` rescans every cell's whole domain per observation, O(cells^2 x
tiles) in entropy scanning alone; `propagate`'s arc revision recomputes support
from scratch with an inner `(0..tiles.len()).any(...)`, O(tiles^2) `compatible`
calls per (cell, face). Tile count is what bites: a 5x5x3 drive contributes 24
orientations x 75 segments = 1800 tiles, which is the 92-96 ms above against
11-13 ms for the shipped ~170. `draw` also allocates a fresh `vec![0.0f32; ..]`
per observed cell.
Fix, in decreasing value: a per-tile per-face allowed-neighbour bitset built once
in `TileSet::build` with a bitwise AND in place of the inner `any`; bitset
domains instead of `Vec<Vec<bool>>`; an entropy heap or per-cell option counter;
a scratch buffer for `draw`.
Not higher: nothing here is per-frame. The only caller runs one collapse from an
`On<Activate>` observer on a button press, so the cost lands on one frame of a
deliberate user action.

**MINOR - `crates/nova_wfc/src/tests.rs:119` - `one_seed_names_one_hull` cannot catch the failure it is named for.**
It collapses each seed twice in ONE process and compares, which pins `hull()`
being a pure function of `(TileSet, seed)` - already true by construction. The
docstring claims something stronger ("a seed is a NAME for a hull"). A refactor
that changes the order `draw` consumes the RNG, or a `rand` bump, renames every
seed and this test stays green.
Fix: pin one seed against a stored fingerprint - section count plus a hash over
`(id, position, rotation)`.

**MINOR - `crates/nova_wfc/src/collapse.rs:311` - seed stability is inherited from `rand::StdRng` and never stated.**
`StdRng::seed_from_u64` plus `random_range` on `f32` is reproducible only within a
`rand` major version, against the "a seed is a NAME for a hull" claim. NOT a
house-rule break: AGENTS.md reserves `bevy_rand` for gameplay randomness, and
`crates/nova_scenario/src/actions/spawn.rs:376` already uses `StdRng` for exactly
this kind of seeded offline generation.
Fix: say in `hull()`'s docstring that seed stability is scoped to a `rand` major
version, or name the algorithm explicitly.

**MINOR - `crates/nova_wfc/src/collapse.rs:76` - `aim_allowed`'s docstring claims an invariant the code contradicts one function away.**
"it cannot empty a domain here because `VACUUM` is compatible with everything and
is never struck. There is no backtracking in this collapse and none is needed."
The first clause is true of the unary filter in `domains()`. The second is not
true of the solve: `compatible` returns `false` for `VACUUM` against any joint
face, so propagation strikes vacuum beside an assigned multi-cell segment, and
`solve` carries `"cell {cell} collapsed to nothing"` precisely because a domain
can empty.
Fix: scope the first clause to the unary constraints and say what happens - no
backtracking, so a contradiction refuses the seed and the caller rerolls.

**MINOR - `crates/nova_wfc/src/check.rs:135` - `unmated_contacts`' `exempt` callback is called with two different argument-order contracts and documents neither.**
`:158` calls `exempt(a, b)` from a loop that guarantees `a < b`; `:199` calls
`exempt(index, other)` where `other` comes from a `position()` scan and can be
either side. A caller who reads the first loop and writes a `(low, high)`
predicate silently stops exempting in the second.
Fix: state that the pair is unordered and the predicate must be symmetric, or
sort the arguments at both call sites.

**MINOR - `crates/nova_wfc/src/tests.rs:246` - `a_cell_whose_taste_cancelled_out_still_draws_something` asserts nothing.**
The body is `let _ = set.hull(seed, false, None);` with a comment that either
answer is fine. A no-panic test is legitimate here, but the name promises a
positive claim; the test passes if `draw` returned `VACUUM` for every cell and
every hull came out a bare keel.
Fix: assert `Ok` and at least the seeded keel and drive.

**MINOR - `crates/nova_wfc/src/tests.rs:474` - zone coverage stops at the two zones with the simplest arithmetic.**
`Dorsal` and `Ventral` are simple comparisons against `keel_row`. Untested are
the four with arithmetic an off-by-one lives in: the three `z * 3` thirds
(`collapse.rs:105-107`) and the outboard half (`:110`) - and those carry the
strongest promise ("a three-cell lance zoned `Bow` must lie wholly in the forward
third"), which no test asserts for any multi-cell part. The existing assertion is
`position.y > 0.0`, which means "above the keel row" only because the shipped
height is odd.
Fix: assert against the grid row, not a world y, and add one `Bow`-zoned
multi-cell part.

**MINOR - `crates/nova_wfc/src/lib.rs:69` - `runnable`'s summary line is stranded on `stern_drive_span`.**
`stern_drive_span`'s docstring opens "Refuse a grammar the collapse cannot be run
in, BEFORE anything indexes a cell or draws a weight.", then blanks, then its
real summary. `runnable` (`:101`) begins mid-thought with "The gate exists
because a grammar is content".
Fix: move the stranded line to the top of `runnable`.

**MINOR - `crates/nova_wfc/src/tiles.rs:489` - `rotated_half_extents` is a pass-through that abstracts nothing, and it is in the public prelude.**
The body is `collider.rotated_aabb_half_extents(rotation)` and nothing else.
`SectionCollider` is already in `nova_ship::prelude`, which all four call sites
import, and each of them writes the `unwrap_or_default()` itself.
Fix: delete it and call the inherent method at `tiles.rs:301`, `check.rs:64` and
`examples/playable/wfc_arena/stamps.rs:78`.

**MINOR - `crates/nova_wfc/src/check.rs:41` - `Placed::position` and `Placed::body.0` are always the same value.**
`place` (`:65-70`) sets both from `section.position`. Two fields for one fact; an
edit that adjusts one placement path updates one of them and the contact test and
the socket probe then disagree about where a section is.
Fix: make `body` the half-extents alone and build the pair at `contact_face` and
`body_holds`, or drop `position`.

**MINOR - `crates/nova_wfc/src/collapse.rs:437` - the `erode_studs` docstring states a threshold the code does not use.**
"a part is asked for two neighbours or for as many as it has sockets, whichever
is fewer", against `neighbours < wanted.min(SPIKE_SUPPORT)` with `SPIKE_SUPPORT
= 3`.
Fix: say three, or reference the constant instead of restating its value.

**MINOR - `crates/nova_wfc/src/lib.rs:60` - the crate has one root prelude, no module preludes, and every internal import bypasses it.**
AGENTS.md: "Give exporting modules a `prelude` and export it from the crate root.
Import through preludes, including inside the same crate." `grid`, `tiles` and
`check` all export items and none has a `prelude`, unlike `nova_ship`, which
gives one to every exporting module. Sharpest instance: `tests.rs:14-19` imports
`check::{place, unmated_contacts}` AND `prelude::*` in one statement - the same
two names reachable two ways.
Fix: give `grid`, `tiles` and `check` a prelude each, re-export from the root
prelude, and route in-crate imports through them.

**MINOR - `crates/nova_wfc/src/lib.rs:77` - the seeded part's span is derived twice in the crate and a third time in the editor, because the bound is `pub(crate)`.**
`stern_drive_span`/`bow_gun_span` read the span off `SectionFootprint`;
`collapse.rs:294`/`:221` read the same span off the upright tile, with a verbatim
docstring copy. `crates/nova_editor/src/generate.rs:340-362` (`holding`)
re-implements `runnable`'s three bounds by hand and its own docstring admits it.
A new bound in `runnable` that `holding` does not learn hands the builder a
grammar the editor's own grow step produced and `nova_wfc` refuses by name.
Fix: expose a `smallest_grid(sections, grammar) -> GrammarGrid` beside
`runnable`, compare against it, and have the editor grow to it.

**MINOR - `crates/nova_wfc/src/tests.rs:60` - the tests hardcode catalog ids that `nova_ship::prelude` already exports as constants.**
`"reinforced_hull_section"`, `"basic_thruster_section"` and
`"railgun_lance_section"` are `REINFORCED_HULL_SECTION_ID` and friends in
`crates/nova_ship/src/sections/catalog_ids.rs:17-31`, and the file already
imports from `nova_ship::prelude`. A rename moves every constant-using consumer
and leaves this file reporting a regression that did not happen.
Fix: use the exported constants.

**MINOR - `crates/nova_wfc/src/lib.rs:274` - no public item in the crate says its lengths are build-grid cells.**
AGENTS.md puts world units only at a boundary "which says so locally".
`bow_face() -> f32`, `Placed::position` and `Placed::body` are all build-grid
cells at 10 m each; the type they feed says so explicitly
(`SpaceshipSectionConfig::position`). A caller reading `bow_face()` as meters
bolts a stamp 10x too far forward. The crate's error strings get it right.
Fix: name the unit on those three items.

**MINOR - `CHANGELOG.md` - the crate carve-out has no Internals & Tooling entry.**
`docs/keeping-docs-in-sync.md:65` puts "CHANGELOG (Internals)" in the "Also"
column for a `crates/*` split or move. The 2043-line move of the generator out of
`examples/playable/shared/wfc.rs` into a shipped crate the editor links is what
made the Generate verb possible, measured against v0.12.0 where it was
example-only, and it is invisible.
Fix: one Internals & Tooling line.

**NOT a finding - `crates/nova_wfc/src/tests.rs` as a crate-root test module.**
Raised as a house-rule break against "unit tests inline or in sibling
`src/**/tests/`". It is the only `crates/*/src/tests.rs` in the workspace, but
`crates/nova_editor/src/generate/tests.rs` shows the flattened sibling file IS
the house form, and `src/tests.rs` is that form for a crate-level module. Worth
splitting per module when the file next grows; not a rule break.

**Verified in this session:** the `u32` multiply in `cells()` and the absence of
any upper bound in `runnable` or `lint_grammar_config`; both refusal strings
absent from the tests, and the bent-grammar test asserting only `is_err()`; the
unconditional `kept[start]` and the second `keel_component` call passing `kept`
as `standing`; both `exempt` call sites and their differing orders;
`aim_allowed`'s doc against `solve`'s error; `one_seed_names_one_hull` collapsing
twice in one process; `StdRng` here and in `spawn.rs:376`; the empty assertion in
the cancelled-taste test; the stranded docstring at `lib.rs:69`; the
pass-through body of `rotated_half_extents` and its four call sites; `place`
setting `position` and `body.0` from one source; `SPIKE_SUPPORT = 3` against the
"two neighbours" prose; the single root prelude; `Content::Grammar` in
`merge.rs:545` and `base.bundle.ron:16` against an empty `/create/`;
`concept-index.md:82`'s dead path and the zero hits for `wfc_hull`; both crate
maps and the mermaid graph; `nova_wfc` absent from all of `docs/` and `web/`.

**Checked by the lanes:** the whole crate read at HEAD; edge arithmetic traced by
hand against the gate (`seed_keel`'s clamp, `vacuum_weight`'s divisor,
`seed_stern`'s subtractions, `Grid::neighbour`'s bounds, `style_at`'s modulo -
all sound); termination of all four fixed-point passes; contradiction handling;
determinism traced through every `HashSet` and `HashMap` (all point lookups or
dedup, never iterated for a result - same seed reproduces the same hull);
multi-cell part integrity through `drop_part`, `keel_component` and `fill_pits`;
no `unwrap`/`expect`/`panic!` in non-test crate code; four repeat timing runs at
`opt-level = 1`; `cargo check -p nova_wfc --all-targets` and
`--target wasm32-unknown-unknown` clean; the wasm ban list from
`ci/wasm-clippy/clippy.toml`; no `#[allow]` and no new workspace lints; the
`[Unreleased]` WFC entries against the v0.12.0 baseline (all under 200
characters, correctly grouped); the workspace wiring and `Cargo.lock`.

**Not checked:** release-profile timings - every number above is
`opt-level = 1`. No probe range was run: one collapse runs from a button press,
not per frame, so the performance brief's condition was not met. Whether the
shipped grammar can reach a contradiction on some seed - the tests cover seeds
0..64 on the shipped grammar and 0..8 on the bent ones, so "no shipped seed
refuses" is unmeasured. `derive_link_point_graph`, `lint_scenario` and the
`SkinStructure` cell bucketing, which `check.rs` deliberately delegates to. Any
visual judgement of a generated hull - no rendered example was run. The editor,
`ship_grammar.rs`, `nova_authoring`, `assets/` and `examples/` beyond reading
them for the caller's contract; batches 2 and 3 own them.

### Owner-raised, ahead of batches 4-5 - the comms channel vocabulary is hardcoded

Raised by the owner during batch 2. Recorded here so the batch 4 and 5 lanes do
not re-derive it. Range: `246c4c66`.

**MAJOR - `crates/nova_hud/src/comms_panel.rs:46` - `NarrativeChannel` is a closed Rust enum for something that is authored presentation data, so a mod cannot add a channel or re-voice one.**

The enum has three variants (`Comms`, `Crew`, `Guard`) and its entire body is a
data table:

- `tone()` -> `ChipTone` (Comms/Phosphor/Amber) - a colour.
- `tag()` -> `Option<&'static str>` - the string `"GUARD"`, or nothing.
- `signal_strength()` -> `f32` - `0.7` for Guard, `1.0` otherwise - an alpha.

No variant dispatches to a different code path. It is a colour, a label and an
alpha, written in Rust and compiled into the game. `crates/nova_scenario/src/actions/mission.rs:48`
mirrors it a second time as `NarrativeChannelConfig` with a hand-written `From`,
so a fourth channel is a two-crate code change and a recompile.

This contradicts the project's stated content philosophy, which
`crates/nova_modding/src/lib.rs` repeats three times in the `Content` enum's own
doc comments:

- `Style(ShipStyleConfig)` - "so a mod can ship a look and a scenario can put it
  on the enemies **without any code changing**".
- `Grammar(ShipGrammarConfig)` - "so a mod that ships a grammar changes what a
  generator builds **without any code changing**".
- `Impact(ImpactSoundConfig)` - "one row per item rather than one nested table,
  so a mod can re-voice a single (damage type, material) pair without restating
  the rest".

A channel is exactly that shape - an authored row with a look - and it is the one
that got hardcoded. AGENTS.md's explicit-authoring rule ("a missing required
field or an unrecognized id is an error at lint, then at load") describes the
mechanism this should have used.

**The stated justification for the mirror does not hold.** The doc says the
duplicate exists because "the HUD cannot depend on nova_scenario (the dependency
points the other way)". The direction is right, but `crates/nova_scenario/Cargo.toml:20`
depends on `nova_hud`, and the `From` impl names `NarrativeChannel` directly - so
nova_scenario can already see the type. The duplicate buys only the `Reflect` and
`serde` derives, not the crate split.

**Fix.** A `Content::Channel` item registering into a `GameChannels` catalog in
`nova_gameplay`, the same shape as `GameImpacts`
(`crates/nova_gameplay/src/impact_sound.rs:93`) and reachable from both nova_hud
and nova_scenario, which already depend on it. The row authors `id`, a tone, an
optional `tag`, and a signal strength. `NarrativeCueActionConfig::channel`
becomes a `String` id carrying `#[reflect(@Names::Channel)]`, so an unknown id is
a lint error and then a load error. Base content ships `comms`, `crew` and
`guard`, so nothing about the shipped look moves. Both enums and the `From` impl
are deleted rather than kept beside the catalog.

**One decision the fix has to make:** whether a channel authors a `ChipTone` by
name or a raw colour. Recommend `ChipTone`: the HUD's chip language is
deliberate and already carries Phosphor/Amber/Comms/Threat, so authoring a tone
keeps the OPEN set (which channels exist) open and the CLOSED set (what the HUD's
visual vocabulary is) closed. A raw colour lets a mod break the chip language.

**This is cheap, because the format never shipped.** `NarrativeChannel` returns
zero hits at `origin/master`; it was introduced by `246c4c66`, which is unpushed,
and the last release is v0.12.0 (2026-08-31), whose action is
`StoryMessageActionConfig`. So per AGENTS.md this needs NO `**(breaking)**`
marker and NO migration note - it is a format that has not shipped, and the fix
is to land the right shape before it does. The cost is 87 authored cues in
`first_shift` (39 `Comms`, 45 `Crew`, 3 `Guard`), all generated from the Rust
builders, so `content gen` rewrites them.

**NOT a finding - the other four mirrored `*Config` enums in `crates/nova_scenario/src/actions/`.**
The test is whether a variant dispatches to distinct CODE or only returns DATA.
Checked all four and each earns its enum:

- `HudReadoutFormatConfig` -> `HudReadoutFormat` - `render_into` branches into
  three different formatters.
- `SoundRouteConfig` -> `AudioRoute` - selects bus, attenuation and panning
  behaviour; the doc records that this routing REPLACED a special case.
- `ScreenCornerConfig` -> `ScreenCorner` - geometry; four corners is closed, and
  `is_left`/`is_top` are derived facts.
- `CameraLookAtConfig` - camera behaviour, not a look-up table.

`NarrativeChannel` is the only one whose whole body is a data table, and the only
one whose variant set is open by nature.

**MINOR - `crates/nova_hud/src/comms_panel.rs:113` - `StoryFeed`'s doc says it lives in a crate it does not live in.**
"Lives in nova_gameplay because the HUD cannot depend on nova_scenario ... the
same split as `GameObjectives`." `StoryFeed` is defined at
`crates/nova_hud/src/comms_panel.rs:116`. `GameObjectives` genuinely is in
nova_gameplay (`crates/nova_gameplay/src/objectives.rs:42`), so the comparison
names a split this type does not make. Three other doc comments cite it as
precedent for their own mirrors (`readout.rs:102`, `cinematic_prompt.rs:34`,
`cinematic_title.rs:30` and `:81`), so the wrong claim has propagated.
Fix: say where it lives and why, or move it to nova_gameplay as the comment
claims - which is also where `GameChannels` would go.

**Verified in this session:** the three method bodies of `NarrativeChannel` and
that none dispatches to distinct code; the duplicate enum and its `From`;
`nova_scenario`'s dependency on `nova_hud`; the three `Content` doc comments; the
bodies of all four other mirrored enums; `StoryFeed`'s definition site against
its doc and against `GameObjectives`; zero hits for `NarrativeChannel` at
`origin/master` and `StoryMessageActionConfig` as the released form; the 87
authored cues and their channel split; `GameImpacts` as the catalog precedent.

### Batch 2 - Editor WFC integration (`aed4fab1`, `crates/nova_editor`)

Two lanes, both returned, and both landed on the skin/style overwrite
independently. `cargo test -p nova_editor --lib` is 473 passed, 0 failed;
`cargo check -p nova_editor --all-targets` and
`--lib --target wasm32-unknown-unknown` are clean. Every load-bearing claim was
re-derived in the main session.

**BLOCKER - `crates/nova_probe_cli/tests/catalog_drift.rs:618` - the invariant roster is stale and the drift test FAILS at HEAD.**
`aed4fab1` adds three `nova_probe::probe_marker` calls to
`examples/systems/system_ship_editor.rs` - `a generated hull comes out bound`,
`a named document saves without asking again`, `a named save writes a bundle of
its own` - and touches no file under `crates/nova_probe_cli`. RUN in this
session: `cargo test -p nova_probe_cli --test catalog_drift` is
`1 passed; 1 failed`, with `systems_ranges_assert_their_invariant_roster`
naming all three slugs in the left-minus-right set. `SYSTEMS_INVARIANTS` is 234
(`:567`).
Fix: add the three slugs to `system_ship_editor`'s roster entry and raise
`SYSTEMS_INVARIANTS` to 237.
This is the same class of failure batch 1 of the PREVIOUS review found in
`b202d69f`; the roster is drifting once per feature commit.

**MAJOR - `crates/nova_editor/src/bundle.rs:90` - a save named "Sandbox" derives `editor_sandbox` and silently takes over the editor's own stage range.**
`EDITOR_BUNDLE_PREFIX` is `"editor_"` (`:63`) and `SANDBOX_ID` is
`"editor_sandbox"` (`crates/nova_editor/src/scenario.rs:56`), so
`bundle_id("Sandbox")` - or "sand box", or "SANDBOX!" - returns exactly the
reserved id. `saved_range` writes the range under the bundle id with
`hidden: false` and `apply_file_request` enables the mod; on the next load
`sandbox_unregistered` is false, `register_sandbox_scenario` never runs, and the
builder's saved range stands in for the editor's stage scenario. The Save As
window shows nothing, because `saved_bundles()` lists files on disk and the
sandbox is not one, so the collision readout at `files.rs:363` stays silent.
Fix: reject `SANDBOX_ID` in `bundle_id`, or reserve it in `sync_save_name` so the
readout says the name is taken and Save greys out. Pin it beside
`two_names_that_differ_only_in_punctuation_land_in_one_slot`.
Not a BLOCKER: it needs a specific name. Not MINOR: the recovery is invisible and
the id is the one the editor stands on.

**MAJOR - `crates/nova_editor/src/generate.rs:186` and `:379` - every Generate overwrites the ship's cladding and style, and the cladding toggle the record claims never landed.** (both lanes)
`collapse()` hard-codes `style_at(styles, 0)` and derives `skin` from
`style.is_some()`, never reading the ship it generates into; `generate_ship` then
writes both back with `node.skin = hull.skin; node.style = hull.style.clone()`.
A builder who turns the skin off in Ship Settings, or picks the third style,
loses both on the next reroll, silently - the status line reports only the part
count. The module doc sells the seed as "a thing you can turn until you like the
answer" and `:182-184` claims "The Ship Settings block above edits both
afterwards like any other ship's", which a reroll makes false. `TileSet::hull`
uses `clad`/`style` only to fill two fields on the returned `ShipHull`; the
sections it lays do not depend on them, so nothing forces this.
The landing record for commit 2 says the block carries "a seed you can read, type
and reroll, **a style, and a cladding toggle**, and a button". At HEAD the block
spawns the seed field, reroll, Generate, the hull-plan line and the part list.
There is no style control and no cladding toggle. The record and the code
disagree.
Fix: pass the edited `ShipNode`'s own `skin`/`style` into `collapse` and leave
the node's fields alone, or land the toggle the record claims. Correct the record
either way. Note `ShipNode::style` documents `None` as "the first style the
content merge loaded" (`node.rs:180-182`), so writing `Some(<first id>)` says the
same thing twice.

**MAJOR - `crates/nova_editor/src/generate.rs:381` - the lint-refusal arm has no test, though the spec names it as required proof.**
The spec's proof list includes "Unit: a generated hull that fails the lint does
not enter the document, and the status line says why". `hull_errors` appears only
at `generate.rs:29`, `:381` and `:384`; nothing in `generate/tests.rs` references
it. The three refusals that ARE covered are the other arms - no ship entered,
content not loaded, nothing ticked. The lint arm is the one deciding whether a
bad collapse reaches the document, and it is the one arm nothing pins.
Fix: a test that ticks a set collapsing into a hull `hull_errors` rejects, then
asserts the ship keeps its old children and `EditorStatus` carries the refusal.
If no ticked set can produce that today, say so in the record instead of listing
it as landed proof.

**MAJOR - `crates/nova_editor/src/bundle.rs:63` and `:488` - the `editor_` prefix is a listing convention, not a property the editor owns, so the "no `ModMeta` marker" divergence does not hold.** (both lanes)
The record justifies dropping the planned marker with "The editor is the only
writer of that prefix", and the constant repeats it. `saved_bundles()` filters
`read_index()` on `starts_with("editor_")`, and `read_index()` is the one
installed-mods index the PORTAL also writes:
`crates/nova_assets/src/portal/install.rs:600` calls the same
`mod_cache::install_local`, and `validate_entry` (`:259-263`) requires only
`is_safe_id` plus URL-safety - there is no prefix rule anywhere. A portal mod
published as `editor_toolkit` appears as a row in both Save As and Open; a name
deriving that id shows the amber overwrite line as though it were the builder's
own save, and Save rewrites the index record's `version` and `bundle`
(`mod_cache.rs:719-729`), taking the installed mod out of the game's view. The
read side degrades safely only by accident (`read_save` looks for
`{id}.content.ron`, which a portal mod need not have); the write side does not.
Fix: put the marker in `ModMeta` and filter on it, or make the prefix
enforceable by rejecting it in `validate_entry` and say so where the constant
claims ownership.
Not a BLOCKER: no portal mod uses the prefix today.

**MAJOR - `web/src/create/author-a-scenario.md:15` - the creator page still describes the single `editor_save` slot.**
"The editor owns one save slot (the `editor_save` mod) and never writes a
hand-authored mod like the one below, so the two paths do not fight over a file."
Both halves are now wrong in a way an author acts on: `bundle_id` derives
`editor_` plus the slugged name, so there is one bundle per named range, and what
keeps hand-authored mods out of reach is the prefix filter, not a fixed id. This
is the only `/create/` text telling an author what the editor does to their mod
cache.
Fix: say the editor writes one `editor_`-prefixed bundle per named range and
opens no bundle without that prefix.

**MAJOR - `web/src/wiki/keybinds.md:255` - the wiki says torpedoes fire on the left mouse button; a torpedo placed in the editor now takes `F`.**
`default_binds` (`placement.rs:158-180`) gives `SectionKind::Torpedo`
`KeyCode::KeyF` / `LeftTrigger2`, where it previously shared the turret's
`MouseButton::Left`. The sentence "The shipped ship fires both turrets and
torpedoes on the left mouse button" has no other referent: the only two
`input_mapping` blocks in shipped content bind turrets only, so nothing but the
editor's placement default ever put a torpedo on LMB. A player who builds a ship
with a bay presses LMB and nothing launches.
Fix: state the four defaults the changelog already carries - Space thrusts, LMB
the PDCs, `F` the torpedoes, `R` the railgun - and drop the "shipped ship"
framing.

**MAJOR (unmeasured) - `crates/nova_editor/src/node.rs:1310` - the per-frame document walks are O(sections) or O(sections^2), and a generated hull multiplies the section count roughly tenfold.**
Three ungated systems walk the whole document every frame:
`report_duplicate_ids` scans every sibling pair comparing `NodeId` strings -
about 13,000 comparisons per frame for a 164-section ship, up from about 200 for
a hand-built one; `wanted_rows` (`ui/mod.rs:1697`) calls `sections_of` twice per
ship (`:1732`, `:1759`), each a collect and a sort, then builds five to eight
`String`s per row, all before the `Local` compare at `:2058` decides nothing
changed; `sync_editor_probe` (`probe.rs:175`) rebuilds the snapshot in
`PostUpdate` with no run condition. `sync_hull_plan` (`ui/mod.rs:1347`) also
allocates a `Vec<Drawn>` and runs a `format!` every frame whether or not the
Generate block is shown.
Fix: gate `report_duplicate_ids` and `sync_editor_probe` behind
`Changed<NodeId>`/`Changed<ChildOf>` or a dirty flag; hoist `sections_of` to one
call and compare cheap keys before formatting; give `sync_hull_plan` a run
condition on the block being shown.
UNMEASURED, and deliberately so: the only `system_ship_editor` capture on disk is
`probe-runs/b9dcb68f/` from an unrelated commit, at 247.98 ms mean under software
rendering. That host is GPU-bound by two orders of magnitude, cannot resolve the
CPU delta, and there is no post-change reference to grade against.
Not a BLOCKER: the editor is not a shipped frame budget. Not MINOR: the tenfold
section count is what this commit makes routine.

**MINOR - `crates/nova_editor/src/generate.rs:343` - `holding` hand-copies `nova_wfc::runnable`'s bounds and already differs on one arm.**
The editor half of the batch-1 finding. `runnable` applies the `+2` keel
allowance only when a bow gun is seated; `holding` applies
`.max(bore + drive.z + 2)` with `bore = 0` when none is, so it grows a gunless
grid one cell past what `runnable` demands. Harmless today - the shipped
`standard_hull` length dominates both terms - but it is the same arithmetic
written twice with only one authority, across a crate boundary the compiler
cannot check, and no test asserts the two agree.
Fix: export `stern_drive_span`/`bow_gun_span` (or a `minimum_grid`) from
`nova_wfc`'s prelude, have `runnable` check against it and `holding` grow to it.

**MINOR - `crates/nova_editor/src/ui/files.rs:439` - `on_save` renames the document before the write, so a failed write leaves a renamed document with no file.**
`on_save` writes the typed name onto the `ScenarioNode` and then raises
`FileRequest::SaveAs`. If `write_save` fails - a read-only mods directory, a full
disk - the document is now called what the never-written file would have been,
and nothing distinguishes that from a successful save except opening the picker.
Fix: apply the rename in `apply_file_request` after the write succeeds.

**MINOR - `crates/nova_editor/src/generate.rs:59` - `GenerateSettings`'s doc states the opposite of what the code does.**
"The rail block that generates a hull, shown only OUTSIDE a ship."
`sync_context_panels` shows it when `context.ship().is_some()`, and the module doc
four lines above says "Generate is a SHIP verb ... the block sits inside one".
Left behind by the follow-up that moved the verb inside the ship.

**MINOR - `crates/nova_editor/src/node.rs:1085` - `spawn_ship_node`'s doc justifies its shape with a second caller that does not exist.**
"the two verbs that mint one disagree about it: a blank ship must be entered ...
and a generated one must not". Generate no longer mints a ship - it replaces the
hull of the ship the context is inside - and the block is the ship's, not the
scenario's. The helper has exactly one caller, `create_blank_ship`
(`placement.rs:232`), which immediately calls `context.enter(ship)`. The
`&mut EditContext` -> `&EditContext` change and the hoisted `enter` are machinery
for a caller the same commit's follow-up deleted.
Fix: fold `enter` back in, or replace the doc with what is true.

**MINOR - `crates/nova_editor/src/generate.rs:227` - `drawn_grammar`'s doc comment sits on the `Drawn` struct.**
The three-paragraph block at `:227-232` describes the grammar the roll runs and
the vacuum taper - all of which is `drawn_grammar` at `:240`, not the two-field
row struct it is attached to. `Drawn` is left with only its field docs and
`drawn_grammar` is undocumented.

**MINOR - `crates/nova_editor/src/scenario.rs:1185` and `:1193` - `retarget_retries`'s doc still names `editor_save` as the id a document takes in a file.**
The id in a file is now the chosen slot's. `bundle/tests.rs:221-228` had its
matching prose updated in this commit; this function's did not.

**MINOR - `web/src/wiki/keybinds.md:326` - three new editor verbs reached players with nothing in the wiki.**
The editor section lists "Save the document - Ctrl+S" and nothing else about
files. The first Ctrl+S on an unnamed document now raises a name window instead
of writing (`save_or_ask`, `bundle.rs:568-575`), Save As and Open are live rows
rather than the greyed `soon` one, New Scenario asks which of three worlds to
found, and the rail carries a Generate Hull block inside a ship.
`getting-started.md:111` still presents the sandbox as opening on one range.
Nothing is actively misleading, which is why this is MINOR - but the changelog
carries seven entries for behavior the manual does not mention.

**MINOR - `tasks/20260905-133019/TASK.md` - the landing record's test count and two proof items do not match HEAD.**
The record claims 472 `nova_editor` lib tests; HEAD is 473. Two spec proof items
have no test: "a generated hull lifts into document nodes and lowers back
unchanged" (nothing lowers a generated hull; `one_seed_lifts_one_hull` compares
ids only) and "each template founds a document whose lowered layout matches the
template". The record's line that `DestructiveVerb::New` "lost its confirm step"
also reads wrong against `ui/window.rs`, which still spawns the confirm window
and now carries the template rows as its answers - the substance (one modal, not
two) is right, the description is not.

**Verified in this session:** the `catalog_drift` failure, BY RUNNING IT, with
all three missing slugs named and `SYSTEMS_INVARIANTS` at 234; the
`bundle_id("Sandbox")` collision, by reading `EDITOR_BUNDLE_PREFIX = "editor_"`
against `SANDBOX_ID = "editor_sandbox"`.

**Checked by the lanes:** Generate edges (no ship entered, content not loaded,
nothing ticked, an unpriced ticked section, duplicate ids after a regenerate, a
second generate replacing rather than stacking); the chip-versus-row press,
traced into `bevy_ui_widgets-0.19.1/src/button.rs` - `Activate` is a bare
`EntityEvent` with no propagation, so `on_part_choice`'s `activate.entity` guard
is belt-and-braces and holds; every reconciler against the `Local`-guarded trap -
`sync_scene_list` HAS the required `Added<SceneList>` override and the new
reconcilers are unconditional and need none; `style_at(styles, 0)` cannot select
the debug-only `placeholder` style; template edges and the arena's two scatter
actions lifting and lowering as script; all file-window edges (empty name,
punctuation-only, two names to one slot, colliding id, cancel, a second ask not
stacking); `default_binds` per kind pure with distinct key sets; the derived-id
divergence re-validating through `is_safe_id` inside `nova_assets`;
`SAVED_RANGE.id` and the retry id following the chosen slot, with a v0.12.0
`editor_save` bundle still round-tripping (so no `**(breaking)**` and no
migration note are owed); no live consumer of `SAVE_MOD_ID`,
`SAVE_BUNDLE_FILE`, `SAVE_CONTENT_FILE`, `SAVED_RANGE` or `DestructiveVerb::Open`
left anywhere; portability - no `std::time`/`std::thread`/blocking IO added, every
native-only path has a wasm counterpart, and the deferral is honest; units - every
figure `template.rs` writes is a `Meters`/`Meters3` quantity matching
`wfc_arena.rs` exactly; no new plugin or system set, and the one new ordering
constraint (`read_seed_field.after(TextFieldSystems)`) explicit; no `#[allow]`,
the one suppression an `#[expect(..., reason)]`; all 135 `[Unreleased]` entries
under 200 characters.

**Not checked:** no rendered example run and no probe capture in either lane - every
frame-cost claim above is reasoned from code and explicitly unmeasured, for the
reason recorded in that finding. No workspace suite and no workspace Clippy. The
`wasm32` arms were read but not compiled for that target. The live picking and
drag paths were not exercised. `nova_wfc`, `ship_grammar.rs`, `nova_authoring`,
`nova_scenario/src/lint`, `nova_assets`, `nova_modding`, `nova_ui`, `assets/` and
`examples/` were read for context only; batch 3 owns them.

### Batch 3 - Ship grammar, lint, merge, examples (`aed4fab1`, everything outside `nova_wfc` and `nova_editor`)

Two lanes, both returned. They converged on the missing load-time lint and
DISAGREED on its severity: Lane A filed it BLOCKER, Lane B MAJOR. Adjudicated
below as MAJOR, with the reason recorded. Every load-bearing claim was
re-derived in the main session.

**MAJOR - `crates/nova_assets/src/merge.rs:373` - a grammar is registered at load with no lint pass, so the "error at lint, then at load" rule has no load half.** (both lanes)
`register_bundles` runs the runtime content gate over ships (`:300`), scenarios
(`:316`) and campaigns (`:336`), folding findings into `ContentIssues`.
`commands.insert_resource(GameGrammars(outcome.grammars))` at `:373` is the only
thing that happens to a grammar. Verified: `lint_grammar_config` is called from
exactly ONE place in the tree, `crates/nova_authoring/src/lint_walk.rs:256` - the
offline `content lint` CLI, which walks `assets/` and never sees an installed
mod. `Grammar` is the only content kind with a lint function and no load pass.
Failure needing no authoring error at all: enable mod A (ships a section) and mod
B (ships a `Grammar` drawing it). Toggle A off. `register_bundles` re-runs on the
`EnabledMods` change, B's grammar overlays the base one by id, `ContentIssues`
stays empty, and the Mods menu shows B clean. `TileSet::build` then fails at
`nova_wfc/src/tiles.rs:152` - a status line in the editor, but a PANIC in both
benches, which call `unwrap_or_else(|error| panic!(...))`
(`wfc_ships.rs:391`, `wfc_arena.rs:844`). The base game has silently lost its
shipped grammar.
AGENTS.md: "a missing required field or an unrecognized id is an error at lint,
then at load." The docstring on `lint_grammar_config`
(`crates/nova_scenario/src/lint/ship.rs:127`) ASSERTS compliance in those words,
and the load half does not exist.
Fix: a `for grammar in &outcome.grammars` loop beside the ship loop at `:300`,
keyed on `grammar.id`, before the `insert_resource`.
NOT raised to BLOCKER: nothing fails unconditionally at HEAD, and in the shipped
game the failure surfaces as a graceful editor status line. The panic is
bench-only. The one BLOCKER of this run remains the `catalog_drift` failure in
batch 2, which was reproduced by running it. This finding is nonetheless the top
MAJOR of the review: it breaks a named house rule and its own docstring is false.

**MAJOR - `crates/nova_scenario/src/lint/ship.rs:175` - the grid check is blind to the footprints of the parts the grammar itself seeds.**
`lint_grammar_config` floors each axis at 3 and stops. `nova_wfc::runnable`
(`lib.rs:108-152`) refuses three further shapes the lint passes: the seeded
`stern_drive` not fitting (`half_width < drive.x + 1`, `height < drive.y`,
`length < drive.z + 1`); a `bow_gun` whose footprint is not 1x1 across; and
`length < bow.z + drive.z + 2`, the two seeds meeting with no keel between.
A grammar with `grid: (4, 5, 11)` and `stern_drive: "capital_thruster_section"`
draws ZERO findings from `content lint`, ships, and only says what is wrong when
a builder presses Generate - the same rule, inverted. `nova_wfc` even has the
tests for these; nothing checks them where content is authored. The lint HAS the
data: `KnownSections::from_configs` keeps `collider` (`lint/mod.rs:69`), which is
what `runnable` uses.
Fix: resolve `keel.stern_drive` and `keel.bow_gun` through `sections.get(id)` and
run the same three bounds `runnable` runs.
Not a BLOCKER: `runnable` does catch all of them before anything panics - only
late, and in the wrong place.

**MAJOR - `crates/nova_scenario/src/lint/ship.rs:134` - 92 lines of new lint gate with no test.**
`cargo test -p nova_scenario --lib lint::` is 60 passed; `lint::ship::tests` holds
13, none of which construct a `ShipGrammarConfig`. No arm is pinned: not the
unknown-prototype loop, not the NaN/negative/zero weight refusal, not the empty
`parts` refusal, not the axis floor, not `vacuum.base <= 0.0`. The `content lint`
run over `assets/**` exercises exactly one grammar and it is clean, so the whole
gate is proven only by not firing. That is how the dead arm below survived
landing.
Fix: table-drive one test per failure mode against a fixture catalog, the shape
`unknown_prototype_is_an_error` already uses.

**MAJOR - `crates/nova_ship/src/sections/ship_grammar.rs:222` - a mod can only REPLACE `standard_hull`; a new grammar id is unreachable.**
`GameGrammars::get_grammar(id)` takes an id, and verified by grep EVERY call site
passes the same constant `STANDARD_HULL_GRAMMAR_ID`:
`nova_editor/src/generate.rs:246`, `ui/mod.rs:1250`, `:1372`, `:5088`,
`wfc_arena.rs:837`, `wfc_ships.rs:393`, `stamps.rs:145`, plus the tests. The one
site taking a parameter, `nova_wfc/src/lib.rs:253`, is fed from those. There is
no picker, no scenario field and no example flag naming a grammar, and
`ShipGrammarConfig::name` ("The name a picker would show") is read by nothing.
The base game's own second consumer proves it: `wfc_arena.rs:836-844` CLONES
`standard_hull` and patches `keel.bow_gun` in Rust rather than naming an arena
grammar, and `generate.rs:250-282` does the same with the ticks.
So a mod shipping `Grammar((id: "freighter_hull", ...))` merges into
`GameGrammars` and nothing can ever select it. The only way to affect generation
is to author `id: "standard_hull"`, retuning the editor and both benches at once,
unable to coexist with the base line. `CHANGELOG.md:277-279` ("so a mod ships its
own procedural hull line") states more than shipped.
Fix: take the grammar id from the caller so `get_grammar`'s parameter means
something, or say plainly in the doc and the changelog that a mod retunes the one
shipped grammar.
Not a BLOCKER: override-by-id genuinely works; what is missing is selection.

**MAJOR - `crates/nova_ui/src/screen/list.rs:93` - the `Hovered` fix has no test that fails without it, and the test that should cover it hand-spawns the component the bundle was missing.**
`scroll_viewport()` gains `bevy::picking::hover::Hovered::default()`. Without it
`any_hovered` in `scroll_viewports` is permanently false and one wheel notch
moves every pane - a bug that SHIPPED in v0.12.0.
`hovered_viewport_takes_the_whole_wheel` (`crates/nova_ui/src/screen/tests.rs:133`)
exists and passed the whole time the bug was live, because - verified by reading
it - it spawns `(ScrollViewport, viewport(1.0), ScrollPosition::default(),
Hovered(true))` BY HAND instead of calling `scroll_viewport()`. It proves
`scroll_viewports` reads `Hovered` correctly and proves nothing about whether
production attaches it. Revert `list.rs` and `cargo test -p nova_ui --lib` is
still 56 passed. This is exactly the fixture trap the correctness brief names:
"a fixture must spawn every component production spawns, or the test proves
nothing." The task record concedes it in its own heading - "A fix the walk found,
not the tests" - and the fix landed still uncovered.
Fix: assert the BUNDLE, not the system. Spawn `scroll_viewport()` and assert the
entity carries `Hovered`.

**MAJOR - `examples/playable/wfc_arena/stamps.rs:36` - the surviving stamp hardcodes the grid the grammar now authors.**
`SUPPORT_Z = 4.0`, the carve plane `z + half.z <= 4.5`, the beam half-extent
`4.0` and the support run `-3.5 + index` for `0..8` all encode
`half_width: 4, length: 11`. Before this commit those were consts in the same
file as the collapse; now `arena_tiles` reads the grid out of merged content and
`stamp_large_drives` takes only `(&mut ShipHull, u64, &GameSections)` - it cannot
see the grid it stamps onto. A mod overlaying `standard_hull` with `length: 13`
moves the transom to `z = 6.5`; the carve at `4.5` strips two extra rows and the
beam is planted two cells inside the hull, and `refuse_broken_ships` aborts the
example.
Fix: pass the grid in and derive the three figures from it, or refuse a grid the
stamp was not tuned for, by name.

**MAJOR - `CHANGELOG.md:373` - an unreleased entry describes the stamp this commit deleted.**
"The `wfc_arena` bench bolts a spinal railgun to every generated bow ... carves
whatever the collapse hung in front of the bore". Verified: `stamp_spinal_lance`
returns ZERO hits in the tree. The collapse seeds the pair from
`GrammarKeel::bow_gun` and, per the record, "nothing is deleted now". Both the
entry and its removal sit inside `[Unreleased]` against the v0.12.0 baseline, and
AGENTS.md requires a revision to be collapsed into its entry, not left standing
beside it.
Fix: delete `:373-375`; the surviving `bow_gun` entry at `:178-180` already says
what ships.

**MINOR - `crates/nova_scenario/src/lint/ship.rs:195` - the stern-seed arm is unreachable as a unique finding.**
`if grid.half_width >= 3 && grid.length < 3` - verified dead: `length < 3` is
already an error from the axis loop at `:184`, so every input satisfying this has
already been reported and the arm can only emit a second line for one fault. It
never checks what its own comment describes (the drive and its deck fitting the
transom - that is the MAJOR above). `vacuum.base` has the same double-report
shape at `:208`/`:216`.
Fix: delete `:192-200` and fold the real bound into the footprint check.

**MINOR - `crates/nova_ship/src/sections/ship_grammar.rs:91` and `:194` - `GrammarZone` and the keel roles are closed Rust sets for things with no code behind them.**
Every `GrammarZone` variant reduces to one predicate over grid coordinates
(`nova_wfc/src/collapse.rs:104-111`); none carries a typed config, a bundle or a
system. That is the opposite of what earns `SectionKind` its closure
(`docs/guide-add-section.md:12-23`: "each kind carries its own typed config, its
own spawn bundle, its own behavior systems"). `GrammarKeel` is the same at role
level: five named fields, four required. A mod cannot seed a sixth role, cannot
seed two of one role, and cannot say "this hull plan seats no bridge" the way
`bow_gun: None` says it seats no spinal gun.
The record's own reason for keeping `stamp_large_drives` - "a grammar seeds ONE
stern drive, so that comparison is not something a grammar can express" - is the
evidence: the base game needed an escape hatch the vocabulary could not provide.
The claim itself was verified and holds.
SAME CLASS as the comms-channel finding the owner raised: a closed Rust set
standing in for authored content. Worth folding into one decision.

**MINOR - `crates/nova_scenario/src/lint/ship.rs:165` vs `crates/nova_wfc/src/lib.rs:170` - lint and runtime disagree about weight 0.**
The lint makes any part with `weight == 0.0` an Error ("which is never drawn -
leave the part out instead"); `runnable` ACCEPTS a zero-weight part as long as one
is above zero, and `tiles::drawn_and_seeded` mints exactly that internally for a
seeded role. A mod authoring a role row at weight 0 to declare its tiles fails
the CI content gate on content the game runs happily.
Fix: make it a `Warn`, or reject it in `runnable` too.

**MINOR - `crates/nova_assets/src/merge.rs:545` and `:650` - the `Content::Grammar` merge arm is untested in both directions.**
The two merge tests were only widened to pass `&mut grammars`; neither writes a
grammar. So neither the last-wins-in-place overlay - which `MergeOutcome::grammars`
sells as "a mod retunes the generator by declaring the base grammar's id" - nor
the intra-bundle duplicate branch with its bespoke conflict message is exercised.
Fix: copy `mod_overlay_replaces_by_id_and_preserves_order` with a `Grammar` pair.

**MINOR - `examples/playable/wfc_ships.rs:446` and `wfc_arena.rs:1492` - the running benches lost the "every contact mates" assertion and the structure readout.**
`aed4fab1^:examples/playable/shared/wfc.rs:1979-2042` ran `refuse_unmated_contacts`
over every hull on every load and logged a per-prototype histogram plus section
count, warning count and skin state. The new copies run only `lint_errors`;
`unmated_contacts` now appears solely in `nova_wfc/src/tests.rs` and `stamps.rs`'s
test module, over fixed seeds. `wfc_ships` is the bench for judging the draw
table, and its only `info!` now prints the ship count.
Fix: keep the contact assertion and the histogram in the bench; the crate already
exports `place` and `unmated_contacts` for it.

**MINOR - `examples/playable/wfc_arena/stamps.rs:108` - 110 lines of tests CI never compiles.**
Verified: the `[[example]]` block at `Cargo.toml:80-82` sets no `test = true`, and
Cargo defaults examples to `test = false`, so `cargo test --workspace --features
debug` and `cargo check --workspace --all-targets` both build it as a plain
binary with no `cfg(test)`. `seeded_large_drive_stamps_mate_to_generated_sterns`
- the only remaining proof that a stamped hull mates - runs only under an explicit
`cargo test --example wfc_arena`, which nothing automates. The record notes this
target "never compiled before"; nothing stops it rotting again.
Fix: move the stamp and its tests into a crate, or set `test = true`.

**MINOR - `tasks/20260905-133019/TASK.md` - the "same hulls for the same seeds" proof line is false for `wfc_arena`.**
`arena_tiles` (`wfc_arena.rs:836-844`) sets `grammar.keel.bow_gun = Some(SPINAL_LANCE)`
BEFORE `TileSet::build`, so the lance is seeded pre-collapse and constrains the
opening domains; the retired `stamp_spinal_lance` ran AFTER the collapse and
pushed ONE lance. So for any seed the arena now collapses a different hull and
fields a mirrored PAIR. The record documents the shift elsewhere ("seed variance,
not a regression"), which is why this is MINOR - but the Proof line is a stale
contract for anyone reading the record. Nothing pins the arena's seed-to-hull
contract: `one_seed_names_one_hull` lives in `nova_wfc` and pins the crate's
output, not the post-stamp arena hull.

**MINOR - `examples/systems/system_ship_editor.rs:2476` - a range assertion with no `outcome:` marker.**
The step asserts the Duelling Arena template founds `arena_key`, `arena_rim`,
`arena_fill` and `arena_planetoid` - a substantive claim - and emits only an
`info!`. `examples/systems/README.md` requires a `probe_marker` beside each. It
is the only assertion covering templates in the whole walk.

**MINOR - `assets/base/base.bundle.ron:13` - the new comment states a load-order constraint the loader does not have.**
"After the sections it draws from, for the ships' reason: a grammar names section
prototypes by id." `register_bundles` flattens every content file into one
`Vec<Content>` before `merge_bundles` runs, and no id is resolved during the
flatten. File order has no effect on resolution, for grammars or ships. The
comment tells a mod author to order their `content` list for a reason that is not
real.

**MINOR - `crates/nova_ship/src/sections/ship_grammar.rs:3` - a docs.rs link to an unpublished crate.**
`[`nova_wfc`](https://docs.rs/nova_wfc)`; verified `crates/nova_wfc/Cargo.toml:6`
is `publish = false`, so the URL 404s. It is the only docs.rs link in `crates/`.

**MINOR - `examples/playable/wfc_arena/stamps.rs:31` - the function doc contradicts the module doc above it.**
"Large sections are not WFC tiles, and the production ship generator will own a
richer grammar later." Both large drives are drawn and seeded now via
`segment_tiles`; the module doc was rewritten and this was not.

**MINOR - `Cargo.toml:75` - the `wfc_arena` example comment still names `shared/wfc.rs`, deleted by this commit.**

**MINOR - `examples/systems/system_ship_editor.rs:3009` - three doc blocks hang off one function.**
`a_file_window_is_up` carries the docs for `the_status_reads` and
`no_menu_is_open` plus its own line; both of those are left undocumented. The
first collapse is pre-existing; this commit inserted a function into the middle
of it.

**MINOR - `CHANGELOG.md:274` - two entries for one unreleased change, in the wrong order.**
The refusal gate (`:274-276`) is a revision of the `Grammar` content item
introduced at `:277-279` - same subsystem, adjacent, both `[Unreleased]` - and
AGENTS.md collapses a revision into its entry. As written the gate is described
before the thing it gates exists.

**Verified in this session:** `lint_grammar_config` called from exactly one site,
and `merge.rs` running three load lints (`:300`, `:316`, `:336`) with no grammar
arm before `:373`; the dead branch at `:195` against the `cells < 3` loop at
`:184`; every `get_grammar` call site passing `STANDARD_HULL_GRAMMAR_ID`;
`stamp_spinal_lance` absent from the tree against the changelog entry at `:373`;
`hovered_viewport_takes_the_whole_wheel` hand-spawning `Hovered(true)` rather
than calling `scroll_viewport()`; `arena_tiles` setting `bow_gun` before
`TileSet::build` and both benches panicking on build failure; `publish = false`
on `nova_wfc`; no `test = true` on the `wfc_arena` example block.

**Checked by the lanes:** the lint arm by arm against every failure mode (unknown
prototype, negative/NaN weight, all-zero weights, empty parts, keel role -
covered; grid vs seeded footprints and `bow_gun` width - NOT covered, filed
above; a zone naming nothing - not reachable, since every zone resolves non-empty
under the 3-cell floor); `grammars.rs` field for field against
`aed4fab1^:examples/playable/shared/wfc.rs` - weights 6.0/0.15/6.4/1.4/1.0/0.6,
`aim: Some(Aft)` on the thruster only, grid 4/5/11, vacuum 0.22/1.4/9.0/24.0, keel
roles and stern cells all match, so the authored move IS behavior-preserving for
`wfc_ships`; the merge arm's dedupe, last-wins overlay and conflict message;
every exhaustive `match` over `Content` updated for the new variant; externally
tagged RON so the variant addition is not a format break; `scroll_viewport`'s 7
call sites checked for a duplicate `Hovered` (none); prelude discipline through
`ship_grammar::prelude` and `sections/mod.rs:45` (no bypass); `nova_ship` is
correctly the lowest crate its four consumers share; every `Option` in the config
documents what its absence means; `RUSTFLAGS="-D warnings"` check of all three
examples WITHOUT `--features debug` - clean; all 135 `[Unreleased]` entries under
200 characters joined. Tests run: `nova_scenario --lib` 386, `lint::` 60,
`nova_wfc --lib` 13, `nova_ui --lib` 56, `content_ron_parity` 2 passed (so the
byte-for-byte generated-RON claim holds), `--features debug --example wfc_arena`
13. `content lint`: 0 errors, 0 warnings, 0 findings.
LIVE RUN, measurement slot held: `DISPLAY=:99 NOVA_AUTOPILOT=1 --example
system_ship_editor` passes end to end, "cycle complete, no panic (t=14.3s)" - the
arena template founds 4 nodes, the lance tick reaches the HULL PLAN line, Generate
lays 98 sections and raises 12 keybind chips, and the beat's assertions are
correctly seed-independent.

**Not checked:** whether `wfc_ships` is BIT-identical to `aed4fab1^` for a given
seed - refuting it for `wfc_arena` needed only the code, but confirming it for
`wfc_ships` needs the pre-move example built and run side by side in two
worktrees. The authored-value comparison above is reasoning, not a measurement.
Whether `mirror_symmetric` comparing socket positions rejects any shipped
prototype the old test accepted, which would move the stream. No timing measured
and none claimed: nothing in this range plausibly moves a frame. `catalog_drift`
not re-run (known failing from batch 2). Whether the retired bore-clearance claim
is still covered - `lint_scenario` does not check exit lanes, and judging that gap
was batch 1's call.

### Batch 4 - Scenario vocabulary runtime (`246c4c66` `ce13092d` `02806185` `aceb94c7` `40e5a550`)

Two lanes, both returned. One BLOCKER. The biggest risk in this batch - whether
the vocabulary rewrite breaks a shipped format - was checked and came back
SOUND; see the verified list. Every load-bearing claim was re-derived here.

**BLOCKER - `examples/screenshots/screenshot_scenario_picker.rs:54` - the `second_shift` deletion left a dangling scenario id in a probe-graded range, which now fails every run.**
`CAMPAIGN_CHAPTER_ROW = "Scenario Row: second_shift"` is clicked at `:124` and
asserted at `:138`. Verified: `assets/base/scenarios/` no longer holds
`second_shift.content.ron`, and `assets/base/campaigns/nova_protocol.content.ron`
lists one member, `first_shift`. So the picker never draws that row.
`click_named` warns and continues on an unresolved name
(`crates/nova_autopilot/src/input.rs:429-437`), so the click silently misses and
the next step's assert fires: "the click on 'Scenario Row: second_shift' never
landed". This is the documented regeneration path for two SHIPPED docs figures
(`news-090-scenario-campaigns.png`, `wiki-first-scenario-picker.png`, cited at
`docs/development.md:349`), and the example wires `NovaProbePlugin`, so
`probe run screenshot_scenario_picker` fails with it.
Fix: point the constant at `first_shift`, and rewrite the const's doc comment,
which still argues for "not the first chapter" against a campaign that now has
exactly one.
Verified by grep that this is the ONLY surviving code reference to the deleted
scenario anywhere in `crates/`, `examples/`, `web/`, `docs/`, `webmods/` or
`assets/mods/`.

**MAJOR - `web/src/create/filters.md:108`, `actions.md:583`, `events.md:203` and `CHANGELOG.md:252` - four places promise a lint Warn for an unplayed `Cinematic` filter key; the code makes it an Error, which refuses the scenario at load.**
Verified all four say "Warn" (the changelog: "An unplayed key warns at lint"),
against `crates/nova_scenario/src/lint/scenario.rs:1525`, which pushes
`LintIssue::error`, with its own test at `:3288` asserting "a scene-key typo is an
impossible event and must error". A lint Error is TERMINAL:
`loader/lifecycle.rs:178,219` filters `LintSeverity::Error` and logs "refusing to
start '{}'". A modder who typos a scene key in a filter expects, per the docs, to
lose that handler; instead the whole chapter is unplayable.
Fix: say Error in all four. NOTE the neighbouring `CancelCinematic` Warn
(`actions.md:618`, code `lint/scenario.rs:746`) is CORRECT and must stay Warn -
the two are deliberately different severities, which is exactly why the pages
must not share one sentence.

**MAJOR - `docs/guide-extend-scenarios.md:167` - Recipe 3 tells a contributor to hand-edit generated code and names symbols that no longer exist.**
`:196` says to add the variant to `enum EventActionConfig` and the arm to its
`impl` - both now emitted by `scenario_actions!`; the only hit for
`enum EventActionConfig` is inside the macro body at `actions/registry.rs:95`, so
a contributor following the recipe edits the macro. `:217` names `leaf_config`
and `leaf_config_mut`, which return ZERO hits under `crates/` - verified.
`ActionChoice` is now `type ActionChoice = ActionTag` (`event.rs:583`), so the
variant, `ALL`, `label` and `stem` are the scenario crate's table row and
`action_choice` is `action.tag()`; only `stock` survives, moved to
`ActionChoiceExt`. `:217` also says "Only `Sequence` holds children";
`ActionKind::Cinematic` holds children too (`event.rs:2126`). `:171` lists the
action submodules without `audio.rs`, `cinematic.rs` or `registry.rs`. Recipe 1
(`:87-93`) has the same problem for events, and the "Two surfaces" table at
`:383` still says an action needs an `ActionChoice` variant.
`docs/keeping-docs-in-sync.md` names this guide in the `nova_scenario` row and
says it directly: "a type that no longer exists is an unambiguous defect and
needs no judgement."
Fix: rewrite both recipes around the table row, the payload struct, the editor
`stock`, the lint arm and the docs.

**MAJOR - `docs/scenario-system.md:228` - "`Sequence` is the one action whose state does not live in the action" is now false, and two neighbouring claims with it.**
`CinematicActionConfig::action` calls `start_cinematic`, which files the same
`SequenceRun` through `start_run` (`world.rs:672-712`), so the cursor machinery
has two clients. `:270` says "one group per `Sequence` step it starts" -
`action_groups` now reads `action.step_chain()` (`loader/mod.rs:454`), so a
`Cinematic`'s steps open groups too, and that sentence is what tells a rule
author "any new rule that reasons about 'one frame' reads `action_groups`". `:223`
omits `audio`, `cinematic` and `registry`.
Fix: name the mechanism after the beat chain, not after `Sequence`.

**MAJOR - `crates/nova_editor/src/scenario.rs:1201` - `retarget_retries` recurses into `Sequence` but not into `Cinematic`, so a retry authored inside a scene keeps the wrong scenario id through a save.**
It hand-rolls `EventActionConfig::Sequence(sequence) => for step in &mut
sequence.steps`, so a `NextScenario` nested in a `Cinematic` step is never
visited. A builder who retypes an Action node to Cinematic, adds a Step and a
`NextScenario` naming the document, saves: top-level and Sequence-nested retries
retarget to the save slot, the one inside the scene keeps `editor_sandbox`, and
the range refuses to start on a dangling `Names::Scenario`.
`crates/nova_editor/src/bundle.rs:253` has the same hole on the return leg. This
batch declares the very invariant that makes it a defect
(`actions/mod.rs:382-388`: every reader goes through `step_chain` so "a nesting
arm cannot be honoured by one reader and quietly missed by the others").
Fix: replace the hand-rolled match with `action.walk_mut(...)`
(`mod.rs:413`), which already covers both arms. Neither branch has a test
(`bundle/tests.rs:229,256` cover only top-level retries).

**MAJOR - `crates/nova_scenario/src/actions/audio.rs:64` - the new `PlaySound` action has no lint arm and no bound on `volume`.**
`volume: Option<f32>` goes through `unwrap_or(DEFAULT_SOUND_VOLUME)` straight
into `play_sfx`, and nothing downstream clamps it: `PlaySfx::with_volume`
(`nova_gameplay/src/audio/sfx.rs:71-74`) assigns the field and the voice reads it
verbatim. `PlaySound((sound: "...", route: Interface, volume: Some(40.0)))` in a
mod is a 40x linear gain in the player's headphones. `lint/scenario.rs` has ZERO
mentions of `PlaySound` - `check_action`'s catch-all swallows it - against the
house rule that an unauthorable value is an error at lint then at load. Every
sibling action added in this batch got a lint arm.
Fix: a `check_action` arm rejecting a non-finite or out-of-`[0,1]` gain and an
empty `sound` ref.
Not a BLOCKER: shipped content omits `volume` and `content lint` is clean today.

**MAJOR - `crates/nova_scenario/src/actions/cinematic.rs` - the Cinematic and its skip path have no harnessed range, only unit tests.**
`examples/systems/` has nothing for it and `catalog_drift.rs` names no cinematic
slug. `examples/playable/first_shift_08_strike_salvo.rs:257` reads the production
scene but is a capture bench with no marker and no assert. The unit tests drive
`NovaEventWorld` by hand, so nothing exercises what only the live chain has: the
`scenario_is_live && Unpaused && scenario_has_settled` gate on
`skip_cinematic_on_request` (`loader/clock.rs:96-112`), the `a_player_can_answer`
run condition against a real `InputBindings`, the ordering of `OnCinematicSkipped`
before `OnCinematicFinished` through the real `GameEventQueue`, and the prompt and
title sync reaching a real HUD. AGENTS.md and the correctness brief both say a
substantial feature earns a harnessed range, not only a unit test.
Fix: a `system_cinematic` range on the `AppBuilder` rig with `outcome:` slugs on
the roster.

**MINOR - `crates/nova_events/src/lib.rs:85-87` - `CINEMATIC_KEY_FIELD_NAME` and `TIMER_KEY_FIELD_NAME` are both the literal `"key"`, so the two filters alias each other.**
Verified both constants are `"key"`. `CinematicFilterConfig::filter`
(`filters.rs:186-192`) reads `data.get("key")`; so does `TimerFilterConfig`, and
nothing pairs a filter with its handler's event at lint or at runtime. A scenario
naming a timer and a scene the same thing - natural, since one paces the other -
lets `(name: OnCinematicFinished, filters: [Timer((key: "strike"))])` lint clean
(the Timer arm errors only on an UNDECLARED key) and then MATCH on the scene's
ending. `CinematicFilterConfig`'s own doc promises "Fail closed, like every other
filter here"; it fails closed only when there is no `key` field at all.
Fix: give the cinematic payload a distinct field name (`scene`), or have
`check_filter` know the handler's `EventConfig` and reject a filter that cannot
apply to it.

**MINOR - `CHANGELOG.md:81` - an entry for a bug introduced and fixed inside this release cycle.**
"The skip prompt rides above the keybind dock instead of printing through the
verb chips" is `02806185` fixing an overlap `246c4c66` created four commits
earlier. v0.12.0 has no cinematic, no skip prompt and no `cinematic_skip`
binding, so no released build ever printed the prompt through the dock.
AGENTS.md: "Omit bugs introduced and fixed inside the same release cycle."
Compounding it, `40e5a550` made both strike halves `skippable: false`
(the entry two lines above says so), so no shipped scenario offers the prompt at
all - the entry describes a fix to something a v0.13.0 player cannot reach.
Fix: delete the entry.

**MINOR - `crates/nova_scenario/src/lint/scenario.rs:211` - the duplicate-key error still says "Sequence" for a list that now holds cinematic keys.** (both lanes)
`event_sequences` is filled from `action.step_chain()` (`:143-146`), so two
`Cinematic`s sharing a key in one handler produce "duplicate Sequence key
'strike' within one handler" and the author greps the file for a `Sequence` that
is not there. This is the exact failure the batch's own `check_step_chain`
refactor was written against; its test at `:3239` says so.
Fix: carry the tag label the way `check_step_chain` does.

**MINOR - `crates/nova_scenario/src/world.rs:863` - a scene stopped by a deadline burns its key for the rest of the scenario, and the refusal says the wrong thing.**
`prune_finished_sequences` retains `run.stopped`. That is the documented intent
for a `Sequence`, but a stopped `Cinematic` has already reported
`OnCinematicFinished` (`:795`) and handed the camera back, so a later
`Cinematic((key: "brief"))` is refused at `:699` with "sequence 'brief' is already
running; ignoring the restart" - it never plays and never reports. Two faults in
one line: it calls a `Cinematic` a "sequence", and it says "already running"
about a run that is stopped.

**MINOR - `crates/nova_scenario/src/world.rs:738` - `CancelCinematic` on a scene that already ended logs an `error!` where the documentation promises a quiet no-op.**
`web/src/create/actions.md:608`: "Cancelling a key that is not running is a quiet
no-op." The code errors. The race is ordinary - a scene ends on its last beat and
is pruned, then a handler armed to cancel it fires a frame later, which is the
shape an author reaches for on a death or abort path. The sibling path (cancelling
a scene whose ending is pending but undrained) already returns silently, so the
two "already over" cases differ for no stated reason.

**MINOR - `crates/nova_scenario/src/actions/registry.rs:23` - "`Opaque` is `Sequence`" is stale, and the test naming the rule does not enforce it.**
The macro doc says "`Reflect` is the answer for every action but one", and
`mod.rs:724` asserts only that `Sequence` is `None` and `DebugMessage` is `Some`.
There are now TWO `Opaque` rows (`mod.rs:327,348`). A test claiming "all but one"
while checking one sample would pass if someone flipped `Cinematic` to `Reflect`,
silently breaking the editor, which draws the beat chain itself.
Fix: assert over `ActionTag::ALL` that exactly the beat-chain rows return `None`.

**MINOR - `crates/nova_scenario/src/actions/cinematic.rs:161` - `CinematicTitleActionConfig.seconds` has no lint and no guard.**
`seconds: 0.0` posts a card `cinematic_title()` drops on the first read, so the
beat authors a card nobody sees with no diagnostic; negative behaves the same; and
`f32::NAN` makes `age >= config.seconds` permanently false, so the card never
expires and `title.card != card` holds every frame for the rest of the scenario.
`check_step_chain` already rejects a non-finite or negative `after`/`deadline` on
the sibling step config (`lint/scenario.rs:562-575`).

**MINOR (perf, unmeasured) - `crates/nova_scenario/src/world.rs:266, 288, 305` - the per-frame sync allocates proportionally to the story log, and every frame a card or prompt is up.**
`:266` clones the whole `story_messages` vector every frame BEFORE the length
compare that decides whether anything changed - `first_shift` ends with several
dozen cues, each two `String`s plus an `Option<AssetRef>`, so ~100+ heap
allocations per frame, all discarded. `:288` allocates
`CINEMATIC_SKIP_ACTION.to_string()` every frame a skippable scene is up purely to
compare and drop. `:305-307` clones three `String`s per frame for the life of a
title card, and the comment concedes the `age` field makes the write-on-diff
guard vacuous.
Fix: compare `len()` off the borrow before cloning; `Option<&'static str>` on the
resource; split the constant part of the card from its age.
Unmeasured and no range covers this path, so the slot was not spent on it. None
of it lands on a spike, which is why it is MINOR.

**MINOR - `scripts/gen-scenario-thumbnails.py:67` - the generator still mints a thumbnail for the deleted scenario.**
Verified the row is still there and `assets/base/thumbnails/second_shift.png` is
still committed, referenced by nothing - `base.bundle.ron` ships only
`thumbnails/first_shift.png`. `--check` will keep asserting it into the tree.

**MINOR - `crates/nova_authoring/src/base_content/scenarios/nova_protocol/stage.rs:12` - the module doc cites a deleted example and a second chapter that no longer exists.**
"Layout provenance: `examples/playable/first_shift_map.rs` and
`second_shift_map.rs`" - the latter is gone from `examples/playable/`. The line
above, "both chapters read it", is false for the same reason.

**MINOR - `web/src/create/base-content.md:226` - "### Images (6)" now lists five.**
The batch correctly dropped `thumbnails/second_shift.png` without changing the
heading count. The page bills itself as the exhaustive base catalog. (Related and
pre-existing: the list also omits the seven `portraits/*.png` the bundle declares,
so the true count is 12 either way.)

**MINOR - `crates/nova_scenario/src/world.rs:166` - the `StoryMessage` -> `NarrativeCue` rename left the runtime field and two test names behind.**
`story_messages: Vec<NarrativeCueActionConfig>` with the doc "The scenario's
story-message log", plus
`story_messages_sync_clear_and_tolerate_a_missing_feed` (`:1335`) and
`story_message_icon_refs_rewrite_and_validate_like_other_assets`
(`nova_assets/src/mod_refs.rs:820`). Nothing misbehaves; it is a reader searching
for `narrative` and finding the field under the name the format no longer uses.

**MINOR - `crates/nova_editor/src/event.rs:270` - `event_label` is a pure forwarder.**
The body is `name.label()`. All four callers can call `EventConfig::label`
directly, including the `map_or("Gate", event_label)` use. Its doc comment is a
second copy of the rationale that belongs on the table row in `events.rs`.

**MINOR - `crates/nova_scenario/src/actions/mod.rs:322` - the table's row order splits the two families the creator docs group.**
`registry.rs:64-70` says the row order IS the authoring-menu order and that the
menu "has no list of its own to drift from this one". The rows run `Sequence`,
`TimerStart`, `TimerCancel`, `Cinematic`, `CinematicTitle`, `CancelCinematic`, so
the Add menu separates the two beat chains this batch insists are one shape, with
the timers between them; `PlaySound` sits between `ResumePlayerControl` and
`Screenshot`. `actions.md` and `reference.md:44` group them the other way, so the
menu and the catalog list the same 46 actions in two orders.

**MINOR - `web/src/create/actions.md:628` - "Fades in top-left, holds, fades out" contradicts the authored corner three lines below.**
The summary predates the `corner` field; the table at `:635` documents `corner` as
required with four values and `CinematicTitleActionConfig::corner` has no serde
default.

**Verified in this session:** the dangling `second_shift` row, the missing
scenario file and the one-member campaign, plus `click_named`'s warn-and-continue
and the assert it reaches; that this is the only surviving code reference to
`second_shift`, by sweeping `crates/`, `examples/`, `web/`, `docs/`, `webmods/`
and `assets/mods/`; all four "Warn" claims against `LintIssue::error` at
`lint/scenario.rs:1525`, and that `CancelCinematic`'s Warn is correctly a Warn;
`leaf_config`/`leaf_config_mut` returning zero hits under `crates/` against
`guide-extend-scenarios.md:217`; `TIMER_KEY_FIELD_NAME` and
`CINEMATIC_KEY_FIELD_NAME` both being `"key"`; the intra-cycle skip-prompt entry
at `CHANGELOG.md:81` against the `skippable: false` entry two lines above; the
thumbnail row and orphan PNG; `stage.rs:12`'s two stale claims.

**CHECKED AND SOUND - the shipped-format break was handled correctly.** This was
the batch's most consequential judgement and it came back clean. `StoryMessage`
DID ship in v0.12.0 (`git show v0.12.0` finds it in `assets/mods/example/`, all
six `webmods/the-ledger/*.content.ron`, and five base scenarios), so the rename to
`NarrativeCue` plus a new required `channel` is a genuine shipped-format break -
and `CHANGELOG.md:251-254` marks it `**(breaking)**` with the migration. Both
hand-written mod sets were migrated in this range: all 46 cues in `webmods/` and
all 3 in `assets/mods/example/` carry `channel:`, verified per file, both READMEs
updated, and `content lint` is clean. `second_shift` never shipped, so it is owed
no changelog entry and no migration note, and the `**(breaking)**` campaign entry
correctly names only the five scenarios that did ship. The whole action vocabulary
was diffed against v0.12.0: exactly two removals since the release
(`StoryMessage`, `ForceTorpedoLaunch`), both marked; 22 additions, correctly
unmarked.

**Checked by the lanes:** the `second_shift` sweep across the whole tree including
`scripts/`, `.github/` and the probe catalog, and the `attack_*` -> `strike_*`
example renames; creator-doc coverage re-derived from the table rather than
grepped - all 46 action rows have a section in `actions.md`, an entry in
`reference.md` and a heading in `docs-manifest.js`, no deleted action still
documented, event count 26 and filter count 6 matching the code, and the RON
shape, required/optional status and defaults of every new payload verified
against the source, plus the prose claims (the 0.75 s hold-off, "finish fires on
every path out" through all four paths, "a skip cancels the cursor and never
replays the beats", "nested chains outlive the skip"); `cinematic.rs` end to end
against `world.rs` for double-skip, skip-before-arm, cancel-while-cancelled,
cancel-after-prune, deadline-stop, teardown and two scenes under one key; the
`Names::Cinematic` plumbing through to the inspector; the editor's
lift/lower/retype round trip for `Cinematic`; `ActionChoice = ActionTag` being
compiler-pinned to the table, and the editor mirror confirmed NOT to be a second
table - the 42-arm matches are deleted and only `stock` stays editor-side;
`step_chain` adoption in `wake.rs`, `action_groups`, the lint and
`sequence_gate_handlers`; the skip binding against the `scenario_advance`
follower and the `PauseStates::NovaOs` gate (no terminal cross-talk); camera-blend
arithmetic for zero, negative and NaN seconds; `mod_refs` coverage of the new
`AssetRef` fields; explicit `.chain()` ordering in `loader/clock.rs:96-113`; no
`#[allow]`, no banned `std::*` call, no new workspace lint, no new cross-crate
edge; all 135 `[Unreleased]` entries under 200 characters. Tests run:
`nova_scenario --lib` 386, `nova_editor --lib` 473, `content lint` clean
(0 errors, 0 warnings, 9 scenarios).

**Not checked:** no rendered example and no probe in either lane, so every perf
claim is reasoned from code and explicitly unmeasured, and the
`screenshot_scenario_picker` assert was established from the campaign RON and the
missing file rather than watched firing under Xvfb. The title card's live
appearance, the fade curve and the skip prompt's clearance above the dock are
unverified. `crates/nova_hud` (batch 5), `crates/nova_authoring` and `assets/`
(batch 7) were not judged. The `nova_assets` integration tests were not run (they
build the full asset stack). The web test suite was not built; doc anchors were
checked by parsing headings, not by rendering.

### Batch 5 - HUD presentation (`crates/nova_hud`, 997 lines)

Two lanes, both returned. No BLOCKER. The lanes agreed independently on three
findings, which are merged below and marked (both). Every load-bearing claim was
re-derived here.

**MAJOR - `crates/nova_hud/src/cinematic_prompt.rs:8` and `cinematic_title.rs:11` - both new widgets skip `HudTier` for a stated reason that is false in the shipped tree, and the choice has a player-visible cost.** (both)
The docstrings read "A scene almost always drops the HUD to its cinematic level"
and "a scene drops the HUD to its cinematic level". Nothing does that. Verified
every writer of `HudVisibility` in the workspace: the player's grave/tilde toggle
(`lib.rs:439`), the menu on `OnEnter(MainMenu)` (`nova_menu/src/ambience.rs:221`),
the debug harness (`nova_debug/src/harness.rs:687`), and a few screenshot
examples. No scenario action reaches it - `Cinematic`, `CinematicTitle`,
`CancelCinematic` and `HudReadout` all leave the level alone.
So the full contextual HUD - velocity sphere, speed chip, keybind dock, status
bar - stays over every cinematic shot. That is why the dock-overlap fix
(`02806185`) was needed at all: the counter-evidence is inside this same batch.
Then the player presses grave to clean the screen for a capture and gets the
opposite of what the toggle promises: every tagged widget clears, and the amber
title card and skip prompt stay. `apply_hud_visibility`'s `q_roots` is
`With<HudTier>, Without<ScreenIndicatorMarker>` (`lib.rs:475`), so an untagged
tree is simply not HUD-managed. These are the only two HUD surfaces in the crate
that refuse the player's own HUD toggle.
Fix: pick one. Either tag both and add the hook that drops the level for a scene
(the thing the docstrings assume exists), plus an exemption marker so the card
survives `Cinematic`; or keep them untagged and rewrite both docstrings to the
real reason - the player's manual toggle must not hide a scene's own
affordances - and fix `wiki/hud.md` with it.

**MAJOR - `web/src/wiki/hud.md:28, :46, :50` - the page promises Cinematic is a clean screen, and two widgets now survive it.**
Verified all three sentences: ":28 Cinematic clears every element", ":46
**Cinematic** - a clean screen for captures and quiet flying", ":50 Every widget
still declares its kind ... and all of them clear at Cinematic."
A player reads :46, presses grave during the campaign's opening shot for a clean
screenshot, and gets a title card in the corner. During any mod-authored
skippable scene they also keep the SKIP SCENE prompt. Neither widget declares a
kind, so :50 is false as written.
`web/src/widgets.ts:5101` says "every tier clears, Instrument, Chrome and Status
alike" - that one is still literally true and can stay.
`docs/keeping-docs-in-sync.md` puts the HUD row's obligation on this page and
nothing in it was updated for this change.
Fix: follows whichever way the finding above is resolved.

**MAJOR - `crates/nova_hud/src/comms_panel.rs:151, :403 - the card's body text stays comms-blue on every channel, so "the crew in phosphor" is only partly true.**
`comms_card` takes its border, speaker header and icon from `channel.tone()`, but
the 20 px body is `TextColor(COMMS_BODY...)` at `:403`, unconditional - verified
`COMMS_BODY` is one const with one use. And `ChipTone::fill()`
(`nova_ui/src/hud.rs:103`) returns the same `CHIP_FILL` for Phosphor, Amber and
Comms. So the largest element on the card, the line the player actually reads at
20 px against a 14 px header, is blue for a Crew line and for a Guard line.
`COMMS_BODY`'s own docstring is now stale: "legible against the blue chip" - two
of three channels no longer draw a blue chip.
Documented contracts this makes false: `CHANGELOG.md:109` "work traffic in
transmission blue, the crew in phosphor", and `web/src/create/actions.md:316`
"`Crew` | phosphor green".
Fix: derive the body colour from the tone, or narrow both texts to "the header
and frame are drawn in the channel".
Not higher: the border and header do carry the channel, so cards are
distinguishable. It is the dominant visual mass that was left behind.

**MAJOR (perf, unmeasured, PRE-EXISTING) - `crates/nova_hud/src/comms_panel.rs:342` - `sync_comms_cards` tears down and rebuilds the whole visible card stack every frame, idle frames included.**
Verified: no run condition, no change detection, and
`despawn_related::<Children>()` is queued unconditionally at `:342` BEFORE the
`queue.visible.is_empty()` early return; `:348` respawns from scratch.
Counts are exact from the code. `comms_card` builds a root plus
`children![icon, (Node, children![speaker Text, body Text])]` - 5 entities per
card - and `COMMS_VISIBLE_CAP` is 3 (`:134`). So while a conversation is up:
15 despawns and 15 spawns per frame, 15 taffy nodes deregistered and
re-registered, and 6 freshly built `Text` components per frame, which forces a
full text measure and shape pass over every glyph, body at font 20 wrapped to a
960 px column. On idle frames it still queues a command and writes
`Visibility::Hidden` unconditionally for the whole session.
This is exactly what the batch's data path lands on: three channels and a
campaign that opens on scripted dialogue mean the stack is up for most of a scene.
NOT introduced here - `git log -S despawn_related::<Children>` names `54ebcc2a`,
the nova_gameplay split. Recorded because this batch multiplies its traffic.
UNMEASURED and deliberately so: no range in `examples/systems/` stages the comms
panel, so the measurement slot had nothing to time. Stated as counts, not
milliseconds.
Fix: reconcile in place. The card set changes only when `CommsQueue.visible`
gains or loses an entry; the per-frame part is alpha. Key the cards by queue
slot, spawn and despawn only on a slot change, and write the colours each frame
on existing nodes - the shape `sync_cinematic_title` already uses.

**MINOR - `crates/nova_hud/src/cinematic_prompt.rs:118` - the prompt reads the keyboard column directly instead of the house source helper, so a pad player is told the wrong key or none.**
`.and_then(|action| action.keyboard.first()).map(|source| source.label())`.
The established idiom is `nova_ship/src/input/player/hints.rs:123` -
`action.sources().next()` then `glyph_label()` - and its own doc says why:
"Reading the rig's own `Bindings` matched KEYBOARD entries only, so a verb moved
onto a mouse button still fired and lost its chip with no way back except
rebinding to a key."
Verified `cinematic_skip` carries a gamepad default, `GamepadButton::DPadDown`
(`nova_scenario/src/loader/lifecycle.rs:405`). `InputBindings::refuse_spec`
accepts an EMPTY keyboard column where the default is non-empty, so a stored
override with `keyboard: []` applies cleanly; the label resolves to `None`, the
prompt hides entirely, and the skip still fires on the pad. Separately, a pad
player is always told "ENTER  SKIP SCENE" because the fallback is never reached.
Fix: `action.sources().next().map(|source| source.glyph_label())`. Identical for
keyboard and mouse, and it adds the pad.

**MINOR - `crates/nova_hud/src/comms_panel.rs:87 - `signal_strength` uses a wildcard arm on a closed enum while its two siblings enumerate.** (both)
`tone()` (`:62`) and `tag()` (`:76`) both list all three variants;
`signal_strength` is `Guard => 0.7, _ => 1.0`. A fourth channel breaks the build
in two places and silently draws at full strength in the third - the silent
default AGENTS.md bans. Spell out `Comms` and `Crew`.

**MINOR - `crates/nova_hud/src/comms_panel.rs:452` - the fallback icon tile went half channel-aware.** (both)
The authored branch's border became `tone.text()` (`:442`) and the fallback
branch's background became `tone.text()` at 0.18 (`:453`), but the fallback
border stayed `theme::PHOSPHOR_MUTED`. So a Guard line with no portrait draws a
phosphor-bordered tile over an amber wash, and a Comms line with no icon draws a
phosphor border on a blue card while the same speaker WITH an icon draws a blue
one. Fix: `tone.text()` in both branches, or say in a comment why the "no
portrait" frame is deliberately neutral.

**MINOR - `crates/nova_hud/src/cinematic_title.rs:211,214,217-233,262` and `cinematic_prompt.rs:121` - unconditional component writes where the house idiom is `set_if_neq`.**
`sync_cinematic_title` writes `*visibility` every frame unguarded, and while a
card is up rewrites `node.left/right/top/bottom/border/align_items` and the note's
`node.display` every frame from values that are CONSTANT for a given corner -
each `Node` deref marks the node dirty and re-enters `ui_layout_system`.
`sync_cinematic_prompt` writes `*visibility` unconditionally for the whole
session. Both are asymmetric with `keep_the_prompt_clear_of_the_dock:158`, which
does guard its `node.bottom` write, and with `apply_hud_visibility` (`lib.rs:500`),
which uses `set_if_neq`. Small absolute cost - five nodes - but it defeats
`Changed<Visibility>` and `Changed<Node>` downstream. The per-frame ALPHA writes
are legitimate and should stay.

**MINOR - `crates/nova_hud/src/cinematic_prompt.rs:283` - the dock-clearance fixture hand-builds `ComputedNode` instead of using the crate's live-layout rig.**
`spawn_dock` builds a bare `(KeybindDockMarker, ComputedNode { size,
inverse_scale_factor }, InheritedVisibility)`; production spawns
`keybind_dock_hud()` with a `Node`, seven chips and a real layout pass. The crate
already owns `chip_layout_rig.rs` ("Live-tree UI layout rig shared by the
world-anchored chip tests") for exactly this. This is the repo's own recorded
trap: a fixture must spawn every component production spawns. Concretely,
`an_empty_dock_gives_the_floor_back` asserts a measured height of zero, which is
true today only because `keybind_dock_hud()`'s row (`keybind_dock.rs:303`) has no
padding, border or `min_height`. A later padding change makes production wrong
while the test stays green.

**MINOR - `crates/nova_hud/src/cinematic_prompt.rs:82-84` - the prompt centres itself with a fixed width and a hand-computed half-width margin, and its text is not fixed-width.**
`left: 50%`, `margin-left: -90px`, `width: 180px`, against
`format!("{}  SKIP SCENE", label.to_uppercase())` where the label is
`format!("{key:?}")` with only `Key`/`Digit` stripped
(`nova_input/src/source.rs:114`). Default `Enter` fits; a player who rebinds
Advance to `NumpadEnter` or `BracketRight` gets "NUMPADENTER  SKIP SCENE" against
180 px minus 20 px padding at 12 px type. The keybind dock two modules over
centres itself the content-safe way - `left: 0, width: 100%, justify_content:
Center` (`keybind_dock.rs:304`) - verified. So this is also two ways to say one
thing, and the prompt picked the one that breaks. Use the dock's shape and drop
both magic numbers. Whether Bevy wraps or overflows here is unverified.

**MINOR - `CHANGELOG.md` Interface & HUD - nothing in `[Unreleased]` introduces the skip prompt, and once the intra-cycle line goes, nothing will.**
The only mention is `CHANGELOG.md:81`, already filed in batch 4 for deletion.
`Cinematic` under Modding says "a scene the player may leave" but never that a
prompt appears. Delete the fix line as filed, and add one entry that introduces
the prompt itself.
Two notes from the same block: `CinematicTitle` sits under Scenarios & Objectives
while every other new action this cycle (`Cinematic`, `SetCameraAnchor`,
`PlaySound`, `NarrativeCue`) is under Modding & Mod Portal; and verified only two
`skippable` values ship, both `false`
(`assets/base/scenarios/first_shift.content.ron:3699,3898`), so the prompt is a
mod-only surface today - which argues for wording it as a modding entry.

**MINOR - `web/src/create/actions.md:532-602` - the `Cinematic` page documents `skippable` at length and never says what the player sees.**
It covers the two events, the cursor cancel, the 0.75 s hold-off and nested
chains. An author who writes `skippable: true` is never told an amber SKIP SCENE
prompt appears bottom-centre, that it names the live `scenario_advance` key, or
that it rides above the keybind dock. That prompt is the feature's only feedback
and this page is the only place a creator would look.

**MINOR - `web/src/wiki/hud.md:97,:102` - the comms section never mentions channels.**
The page enumerates what a comms card is, and the campaign now ships lines on
three channels with three treatments and a `GUARD` tag. A player who sees
`MERIDIAN CONTROL / GUARD` in faint amber has nowhere in the wiki to learn what
it means. `docs/keeping-docs-in-sync.md` puts that on this page.

**MINOR - `crates/nova_hud/src/lib.rs:4-6` and `:168-170` - the crate docstring and the `HudTier` docstring were not updated by the change that added two untagged, ship-independent widgets.**
`:4-6` says "Each widget lives in its own submodule and is a [`HudTier`] layer
spawned and despawned with the player ship." Both new widgets spawn in `Startup`
(`cinematic_prompt.rs:63`, `cinematic_title.rs:125`), carry no tier and are never
despawned. `:168-170` is where the crate enumerates what is deliberately untagged
and still names only the juice gizmos. A contributor reads `:4-6` and believes
the tier is mandatory.

**MINOR - `crates/nova_hud/src/cinematic_title.rs:32,:124` - `Reflect` on `ScreenCorner` and `register_type::<ScreenCorner>()` reach nothing.**
Verified `ScreenCorner` is neither a `Component` nor a `Resource`, and `TitleCard`
and `CinematicTitle` derive neither `Reflect` nor registration - so nothing can
reach the type through reflection. Every other `register_type` in `nova_hud`
names a real component or resource; this is the one exception. The reflected
mirror actually needed is `ScreenCornerConfig`
(`nova_scenario/src/actions/cinematic.rs:107`), which the editor uses.

**MINOR - `crates/nova_hud/src/comms_panel.rs:45` and `cinematic_prompt.rs:283` - two small dead things.**
`NarrativeChannel` derives `Default` with `#[default] Comms` and verified ZERO
callers of `NarrativeChannel::default()` in the workspace; `StoryLine` derives no
`Default`. Since the creator docs insist the channel is required precisely so no
line is drawn in a guessed voice, the derive is a latent silent default with no
caller. Separately the test helper `spawn_dock` returns `Entity` and all three
call sites discard it.

**Reinforces a batch-4 finding, not re-filed:** Lane A independently reached the
missing harnessed range for the cinematic vocabulary and added evidence -
`grep -rln "NarrativeCue|CinematicTitleActionConfig|CinematicActionConfig"
examples/` returns nothing, `catalog_drift.rs` holds no cinematic or narrative
slug, and the batch-4 examples diff adds neither. The real chain
(`CinematicTitleActionConfig` -> `post_cinematic_title` -> `state_to_world_system`
in PostUpdate -> `CinematicTitle` -> `sync_cinematic_title` in Update, one frame
behind) is exercised by nothing; both unit suites drive their resource directly.
Lane A also independently reached the `CinematicTitleActionConfig.seconds` lint
gap, filed in batch 4.

**Verified in this session:** every `HudVisibility` writer in the workspace, and
`apply_hud_visibility`'s `With<HudTier>` filter that makes untagged trees
unmanaged; all three `wiki/hud.md` sentences; `COMMS_BODY` being one const with
one unconditional use, and `ChipTone::fill()` returning `CHIP_FILL` for all three
tones; `CHANGELOG.md:109` and `actions.md:316`; `sync_comms_cards`'s
unconditional `despawn_related` before the empty-return, with no run condition,
and `COMMS_VISIBLE_CAP = 3`; the prompt's `action.keyboard.first()` against
`hints.rs:123`'s `action.sources().next()`, and `cinematic_skip`'s
`GamepadButton::DPadDown` default; `signal_strength`'s wildcard against `tag()`'s
exhaustive match; the fallback tile's `PHOSPHOR_MUTED` border beside the
authored branch's `tone.text()`; `ScreenCorner`'s derives and the absence of
`Reflect` on `TitleCard`/`CinematicTitle`; zero callers of
`NarrativeChannel::default()`; the dock's `left: 0, width: 100%` centring against
the prompt's `-90px` margin; only two `skippable` values in shipped content, both
false.

**Checked and sound:** `nova_hud --lib` filters `cinematic` (14 passed) and
`comms` (13 passed) green at HEAD. Fade arithmetic divides only by non-zero
constants, so NaN hides the card and never panics; a just-spawned dock measures
zero and the prompt keeps its floor. Teardown leaks neither card nor prompt -
`NovaEventWorld::clear` resets both, `teardown_scenario_entities` takes
`ResMut`, which flags the resource, so the `resource_changed` chain propagates
`None`; unlike `StoryFeed`, neither needs the hand-reset, because the title sync
compares the whole `TitleCard` rather than a length. Reconciler ordering is the
uniform one-frame lag every pre-existing HUD mirror has, and no reconciler reads
a resource another HUD system writes the same frame. No `Local`-guarded
reconciler in the new code, so the `Added<Marker>` trap does not apply; neither
new system reads `Time`, so the `ManualDuration` zero-delta trap does not either.
The two new prompt systems are unordered but disjoint (one writes
`Visibility`/`Text`, the other `Node`). The title card's four-query split has the
required `Without<CinematicTitleMarker>`. Same-frame replacement is safe: a
second card reads strength 0, hides one frame, then fades in, and the old
placement is never shown under the new text. An open NOVA OS covers both widgets
rather than printing through (`DRAWER_EXEMPT_Z = 12` above HUD chrome at z 0),
and `hide_hud_chrome` is `OnEnter(MainMenu)` only, which tears the scenario down
first. No new `TextShadow`, so no ghosting. The `StoryMessage` -> `NarrativeCue`
rename is fully propagated; the only survivors are frozen historical records
(`web/src/news/0.7.0.md:77`, `CHANGELOG.md:1729`). `reference.md:46` lists 46
actions and `EventActionConfig` has 46 variants. The `NarrativeCue` and
`CinematicTitle` field tables match their configs field for field, including the
required `channel` with no serde default, the 8 s dwell, the cap of 3 and the
[3,30] dwell clamp. The prompt never offers a key the scene would refuse
(`is_skippable_at` gates on `CINEMATIC_SKIP_ARM_SECONDS`). Prelude discipline,
`#[expect(..., reason)]` throughout, inline `mod tests`, no banned `std` call, no
new workspace lint. All four new `[Unreleased]` entries are inside 200
characters. `ScreenCorner` and `HudReadoutFormat` both pass the batch-3
mirrored-enum test and are correctly NOT filed.

**Not checked:** no rendered frame and no probe run in either lane - the
measurement slot was deliberately not spent, because `examples/systems/` stages
no range for the comms panel or any cinematic action, so a probe had nothing
relevant to time. Every perf claim here is counts from code, explicitly
unmeasured. Unverified: whether Bevy wraps or overflows the prompt at a long key
label; the on-screen contrast of a 0.7-alpha Guard card; whether
`CARD_INSET_TOP_PX = 48` clears the status bar as `actions.md:662` claims; the
Escape pause overlay's z-order against the untagged card. The other 238
`nova_hud` lib tests were not run, and no Clippy. `keybind_dock.rs` was judged
only for the `DOCK_BOTTOM_PX` extraction. The comms queue/dwell/emphasis logic
this batch did not touch was not audited. Whether the campaign places cards in
corners that are actually free belongs to batch 7.

### Batch 6 - Radar lock line of sight (`9de7370a` `a6b73ef6` `f47c385a`, 1318 lines)

Two lanes, both returned. No new BLOCKER, but Lane A pinned the EXACT fix for the
batch 2 BLOCKER; see the update at the end of this block. Ground truth was
`tasks/20260905-114723/TASK.md`, and both lanes were told to verify its claims
rather than trust them. Two of its claims turned out to be overstated.

**MAJOR - `crates/nova_ship/src/input/targeting/contacts.rs:164,257` - the occlusion gate silently kills the TRAVEL lock, with no named branch and no player feedback.**
`collect_lockable` is shared, so the new ray removes a candidate for BOTH slots.
The combat slot gets the whole drop-reason model: `CombatLockDrop::Occluded`, the
`debug!` line, the `CombatLockDropped` message, the flight-log line. Verified the
travel slot gets `travel.0 = travel_now` and nothing else - no message, no log,
no toast. `LockClearedToast` is written only by the tap-clear gesture
(`gesture.rs:162,168`), so an occlusion-caused travel drop is completely silent,
and nothing re-picks.
Failure scenario in SHIPPED content: `first_shift` beat 6 has the player
designate `transit_mark_one` (`lock_signature: 400.0`, a 12 km gate). Beat 7
enables the GOTO verb and asks for `[G]`. The 60-rock belt sits between the ship
and that mark, and a rock at close range subtends a large angle. The moment one
eclipses the mark the white crosshair vanishes and `on_autopilot_goto_input`
(`flight_rig.rs:452`) takes the `travel.0 == None` early return - verified `[G]`
becomes a no-op whose only trace is `debug!("no travel lock, nothing to fly
to")`. The player sees a stuck objective and no reason.
Not a BLOCKER: the beat's `OnTravelLockStart` is `once: true` and already fired,
and an ENGAGED goto captures the target by value at press time, so a leg in
flight is not cancelled. The window is between designation and the `[G]` press,
and after any re-designation.
Fix: either give the travel slot its own drop report mirroring the combat one, or
exempt the travel slot from the occlusion gate - a nav designation is not the
same radio link as a weapons lock. The task's own principle ("Without a named
branch 'why did my lock let go?' becomes a guess") was written for the combat
lock only, and the travel slot now needs it too.

**MAJOR - `web/src/wiki/combat-weapons.md:69` - the documented hostile-behind-cover behaviour is now false.**
The page tells the player "Its attack orbit keeps it circling all the while, so
expect the pressure back the moment its motion clears the angle." Verified that
is no longer what happens. `update_ai_target` (`ai/acquisition.rs:232`) clears
`AITarget` when the ray is blocked, and `next_behavior_state`
(`ai/behavior.rs:233`) is `let Some(distance) = hostile_distance else { return
passive; }` - so it returns the passive routine UNCONDITIONALLY and the
`recently_damaged` override is never reached, because there is no target to
measure a distance to. A raider with no route and no orbit directive lands in
`Idle`, and `passive.rs` burns it to rest. It does not circle.
A player reads the page, ducks behind a rock expecting a hostile still orbiting,
and comes back out to a ship that stopped and went quiet. The next sentence
("buys you a pause in the pressure, not just a bullet sponge") now understates
it: it buys a full disengage.
Fix: rewrite `:67-72` to say the hostile loses the pick as well as the shot,
falls back to its patrol/orbit/idle routine with no grace, and re-acquires the
instant the line clears.

**MAJOR - `web/src/wiki/targeting-radar.md:3,:68` - the player wiki never says cover stops a lock, and its drop-reason list is now incomplete.**
Verified both: `:3` "a lock sticks until you clear it or the target is gone", and
`:68` "Locks also fall on their own when the target dies, leaves range, or turns
non-hostile" - an enumeration that now omits occlusion. Nothing on the page says
a body on the line refuses the pick outright.
This is worse than an ordinary doc gap because the game gives NO CUE for a
refused pick. `RadarDenied` fires only for the missing Lock capability
(`state.rs:263`, written only by `gesture.rs:43`), so holding CTRL on a hostile
you can plainly see behind a rock produces nothing - no hollow box, no buzz, no
message. The flight-log line only covers a lock that was already HELD. The wiki
is the only place the rule can be learned.
`docs/keeping-docs-in-sync.md` names this page as the owner for `nova_ship`
targeting.
Fix: add cover to the "Clearing locks" enumeration and a short line-of-sight note
under "Lock ranges". Worth one line too that the off-screen threat arrows go with
it, since `ThreatContacts` is built from the same `collect_lockable` pass.

**MAJOR - `web/src/create/objects.md:66,:156,:307` - an author placing a rock or a world now changes sensor behaviour, and the exhaustive object catalog does not say so.**
Verified all three rows. The Asteroid row documents `lock_signature` purely as
how far the rock can be locked FROM; nothing says the rock also hides everything
behind it. The Planet row is the same, with no mention that
`planet_scenario_object` puts `RadarOccluder` on its sphere collider
(`objects/planet.rs:131`). And `engage_range` at `:307` still reads "a passive
ship leaves its routine for a hostile inside this range" with no clause saying
detection is now line-of-sight gated - so an author who parks the documented
"long-watch emplacement that wakes for targets nothing else detects" behind a
planetoid gets a ship that never wakes.
Concrete: an author scatters a belt with `ScatterObjects` between a picket and
the player's approach lane, and the picket oscillates between engaging and
patrolling with nothing in the RON explaining it.

**MINOR - `crates/nova_ship/src/input/targeting/occlusion.rs:67` - the scanner's own hull is not exempt from its own ray, though the plan required it and the sibling gate does it.** (both)
The task's "Where" section says "The candidate itself must not occlude itself,
and neither may the player's own ship." Verified only the first is implemented:
`stops_radar_for` excludes colliders whose `ColliderOf.body` is the TARGET,
never the scanner. Vacuous today - grep confirms `asteroid.rs:257` and
`planet.rs:131` are the only two `RadarOccluder` insertion sites, so no ship hull
is ever an occluder.
It is a landmine, not a live bug. The origin is `live_structure_anchor`, inside
the scanner's own hull, and `solid: true` makes an origin-inside hit return
distance 0. The day a hull becomes an occluder - which the same task names as
the obvious next step ("a picket screening its wingman") - every ship, player and
AI, occludes itself instantly and locks nothing. The sibling gate already guards
this: `ai/guns.rs:253` predicates on `body_of(collider) != Ok(shooter)`.
The task's "What shipped" drops the promise silently rather than recording it as
deliberately deferred.
Fix: add `&& self.collider_of.get(collider).map(|of| of.body) != Ok(scanner)`,
threading the scanner entity through `is_occluded` - all three call sites already
have it.

**MINOR - `crates/nova_ship/src/input/targeting/contacts.rs:268` - `Occluded` outranks `OutOfRange`, so the drop-reason match applies a precedence rather than a discrimination.** (both)
Verified the order: `Err -> TargetGone`, then the occlusion guard, then
`Ok(_) -> OutOfRange`. The comment claims "the query with one more ray tells the
three apart"; it does not. The occlusion re-cast is not range-limited by the
incumbent gate, so a target that drifted past `max_range * range_hysteresis`
while any rock sits on the now very long line reports `Occluded`.
Failure scenario: a hostile burns away past the 2250 m `BarrenRock` in
`first_shift` and crosses the ship-class lock ceiling with the world on the line.
The flight log says "Combat lock lost: target behind cover."
(`flight_log.rs:132`); the player waits for the line to clear and the lock never
returns, because the real reason was range.
Fix: test the range gate before the occlusion guard, or report the reject
`collect_lockable` actually hit.

**MINOR (perf, reasoned-from-code, UNMEASURED) - `contacts.rs:164` and `radar.rs:87` - the same occlusion answers are recomputed within one frame, and most are discarded.**
Two wastes, both real, neither large at shipped content scale.
1. Duplicate pass. `update_contacts_and_locks` and `update_radar_search` are
`.chain()`ed in `Update` (`targeting/mod.rs:126`) and both call `collect_lockable`
with the same origin and no world mutation between them. While the radar is held
every candidate is ray-cast TWICE for an identical answer. A frame-local
candidate cache shared by the two would halve it AND make the "cannot disagree"
property structural rather than incidental.
2. Discarded answers. `collect_lockable` casts for every body passing the
component and range gates, but the caller reads the result only for two-entity
membership in `still()` and for the threat set, which filters to
`is_hostile && is_combat && !neutralized` (`contacts.rs:347`). Every neutral rock,
well body, salvage crate and non-hostile ship in range pays a ray nothing
consumes. On the radar path the 18-degree cone is applied AFTER collection in
`radar_pick`, so an off-cone body pays a ray it can never use - the cheapest
reject of all is the one not applied before the ray.
Scale in `first_shift`: 2 planets permanently inside their signature gates
(28.5 km and 67.5 km), 1-8 belt rocks, plus beacons and ships. Order 10-20
rays/frame, doubled while held. Not a frame problem in shipped content. The
scaling risk is uncapped: a mining scene with hundreds of rocks inside their lock
gate scales linearly with no budget, and the two planet rays traverse the full
BVH extent every frame with nothing to shrink avian's `max_distance`.

**MINOR - `tasks/20260905-114723/TASK.md`, "What shipped" - "the two gates now answer alike" is true of the collider-to-body mapping only, not of the blocking set.**
`ai/guns.rs:253` blocks on ANY non-sensor collider not belonging to the shooter;
`occlusion.rs:68` blocks only on `RadarOccluder`. So the narrative that follows
- "the AI held fire but kept the pick, chased, and waited. Now it loses the
target as well as the shot" - holds for rocks and planets and nothing else. With
a friendly ship, a salvage crate or any other tangible hull on the line, the old
behaviour is unchanged: fire blocked, pick retained, chase and wait. The "What
blocks" section does scope hulls out, so this is an overstatement in the record
rather than a code defect - but the record is what a later reader will trust.

**MINOR - `tasks/20260905-114723/TASK.md`, "Not done" - the strobing consequence is understated.**
The record says an AI ship skimming a rock edge "can drop its pick, fall to its
passive routine for a frame and take it back, and the FSM has no damping of its
own for that." There is a further effect it does not name: `update_behavior_state`
(`ai/behavior.rs:385`) fires `evade.cooldown.trigger()` on ANY exit from `Evade`,
and a target loss is now such an exit. A ship evading behind a rock edge burns
its evade refractory clock on every flicker, so it cannot evade again for the
cooldown even once the line clears. Read from code, unverified by a run.

**MINOR - `crates/nova_ship/src/input/ai/guns.rs:230` vs `targeting/occlusion.rs:41` - the two gates share a mapping, not a mechanism.**
The mapping claim is true - both resolve a collider through `ColliderOf`. The
rest is not: they remain two independent copies of the same ray-cast boilerplate
(`Dir3::new` guard, `cast_ray_predicate(origin, dir, reach.length(), true,
&SpatialQueryFilter::default(), &pred)`), differing only in the predicate.
`RadarScan`'s own docstring says it exists to avoid "a free function taking six
arguments"; `ai_line_of_fire_blocked` is a free function taking seven, and three
systems hand-thread the same three world reads into it (`guns.rs:281`,
`torpedo.rs:118`, `railgun.rs:131`). The change built the right shape and left the
identical shape unconverted next door.
Fix (not required to ship): give the fire gate the same `SystemParam` treatment,
or give `RadarScan` a second method over one shared private `cast_between`.

**MINOR - `crates/nova_ship/src/input/ai/acquisition.rs:17` - `RadarScan` is imported by deep path, bypassing the prelude.**
`use crate::{input::targeting::occlusion::RadarScan, prelude::*};` against
AGENTS.md "Import through preludes, including inside the same crate." `occlusion`
was widened to `pub(crate)` (`targeting/mod.rs:51`) solely so this one path
resolves, and `RadarScan` reaches no prelude. Every other cross-module targeting
identifier the AI uses arrives through `crate::prelude::*`.

**MINOR - four redundant `ColliderTrees` re-inits with comments that restate the helper.**
`ai_test_world()` (`ai/mod.rs:60`) already does `init_resource::<ColliderTrees>()`.
`guns.rs:374`, `torpedo.rs:291`, `behavior.rs:1144` and `behavior.rs:1330` call it
and then init the resource again, each with a comment explaining what the
helper's own docstring already explains. Separately the helper sits in the AI
plugin's assembly module rather than a sibling `test_support`, which is the
pattern this crate already uses (`input/player/test_support.rs`,
`sections/turret_section/test_support.rs`).

**MINOR - duplicated fixture and a cross-module test placement in `occlusion.rs`.**
`fn spawn_rock` is written twice, near-identically, at `occlusion.rs:101` and
`ai/acquisition.rs:1247`, both carrying the same "a rock the way the asteroid
spawner builds one" prose. And `occlusion.rs:82` reaches sideways with
`use super::{super::contacts::update_contacts_and_locks, ...}` so that
`a_rock_that_comes_between_drops_the_combat_lock_and_names_the_branch` can drive
a system that lives in `contacts.rs`. That test asserts `contacts.rs` behaviour
and belongs beside it.

**MINOR - `crates/nova_scenario/src/objects/asteroid.rs:902` - the spawner test reads the first child without proving it is the collider node.**
`children.iter().next()` then asserts `RadarOccluder`. The planet twin
(`planet.rs:295`) first asserts `get::<Collider>(hull).is_some()` on the same
node; the asteroid one does not. Fail-safe today, but the pair should read alike.

**MINOR - `crates/nova_ship/src/input/targeting/occlusion.rs:43` - the `Dir3::new` error comment describes one of the three cases it swallows.**
`Dir3::new` errors on zero, NaN and infinite. The comment says only "The scanner
is standing on the body." A candidate with a NaN `GlobalTransform` also reaches
here and is reported NOT occluded - it reaches here because the range gate
compares `distance_squared > max_range * max_range` and NaN comparisons are
false (pre-existing). Fix: say the ray is degenerate, or reject a non-finite
reach as occluded, failing closed the way `stops_radar_for` already does for an
unattributable collider.

**MINOR - `docs/development.md:242` - the new range is missing from the "what is on disk today" roster.**
The roster claims completeness and omits `system_lock_line_of_sight`. Largely
pre-existing drift: nine of the thirty-nine `examples/systems/` roots are absent
(`system_command_shell`, `system_turn_limit`, seven `system_headless_*`). Worth
one line while the section is being touched. `Cargo.toml:317` and the gated
roster in `catalog_drift.rs:411` are both correct.

**MINOR - two small slips at the engine-units boundary.**
`examples/systems/system_lock_line_of_sight.rs:53` documents a `Meters` constant
in world units - "Three world units is the size the carve ranges already mesh
cheaply" on `const ROCK_RADIUS: Meters = Meters(30.0)`. Numerically right (30 m =
3 wu) but stated in the wrong currency for the type it documents. And
`occlusion.rs:36` is the ray-cast boundary yet takes bare `Vec3` world-unit
positions without saying so locally; the parent module's blanket "ENGINE UNITS
throughout" (`targeting/mod.rs:40`) covers it by inheritance, but `acquisition.rs`
- a different subtree - now calls into it. No player- or creator-facing figure is
printed in world units anywhere in this change, and the flight-log line carries
no figure at all.

**MINOR - severed rock chunks do not occlude.**
`throw_severed_pieces` (`objects/asteroid_carve.rs:413`) routes through
`spawn_carved_chunk`, which does not insert `RadarOccluder`. The parent rock
correctly keeps its marker across a remesh (the marker rides the node whose
collider is replaced, `asteroid_carve.rs:812`), but a large island cut free
becomes transparent to radar the moment it separates. Consistent with the task's
stated scope; worth a line in the record or a follow-up rather than a fix here.

**BLOCKER UPDATE (batch 2) - the exact fix is now pinned.**
Re-ran `cargo test -p nova_probe_cli --test catalog_drift`: `catalog_matches_disk`
passes, `systems_ranges_assert_their_invariant_roster` FAILS at `:618`.
`system_ship_editor` emits 53 `outcome:` markers; its roster lists 50. The three
missing slugs, verified present in the example and absent from the roster, are:
- `a generated hull comes out bound` (`system_ship_editor.rs:2559`)
- `a named document saves without asking again` (`:2250`)
- `a named save writes a bundle of its own` (`:2588`)
The count test passes because `SYSTEMS_INVARIANTS: usize = 234` (`:567`) matches
the current INCOMPLETE table. So the fix is both halves: add the three roster
rows AND bump the const to 237, exactly as batch 2 recorded. Not caused by this
batch - `system_lock_line_of_sight` is correctly registered with four markers
matching four roster slugs.

**Verified in this session:** the travel slot's bare `travel.0 = travel_now` with
no report, and `on_autopilot_goto_input`'s `debug!`-only early return;
`combat-weapons.md:69`'s orbit sentence against `behavior.rs:233`'s unconditional
`return passive`; `targeting-radar.md:3` and `:68`; `objects.md:66`, `:156` and
`:307`; the drop-reason match order putting `Occluded` before `OutOfRange`;
`stops_radar_for` exempting only the target's body; the deep-path `RadarScan`
import; and the catalog_drift failure with its three named slugs.

**Checked and sound.** All three consumers really do agree: `contacts.rs:243`,
`radar.rs:87` and the drop branch all use the same `is_occluded` with the same
`live_structure_anchor` origin, chained in one `Update` block with nothing moving
between them. Avian semantics were read at source (0.7.0
`spatial_query/system_param.rs:173`): the predicate is applied per BVH proxy and a
rejected proxy returns `Scalar::MAX` so traversal continues past it, meaning the
target's own hull cannot abort the cast and the closest ACCEPTED hit wins; a
`LayerMask` filter would be tested at the same point and prune no earlier, so the
plan's "filter the spatial query to occluder colliders" is satisfied in cost by
the predicate. Despawned occluders are guarded twice (the live `occluders`
query, and avian's own collider lookup), and `Entity` generations stop a recycled
index false-positiving - the task's claim is TRUE. `ColliderOf` has
`ALLOW_SELF_REFERENTIAL: true`, so the self-exclusion covers both the nested
shape and a root-mounted collider, and a deeper nest still resolves to the body a
lock names; choosing it over walking `ChildOf` is right. `solid: true` is correct
for the stated intent. The marker is on the collider child in both spawners,
never the root, each pinned by a test, and those are the only two insertion sites.
The ray is genuinely cast LAST on both paths - player after the component and
squared-distance rejects, AI after the `AI_TARGET_MAX_RANGE` gate and the
allegiance/kind filter. Point defense is correctly ungated. Physics-tick
staleness is the house convention documented at `ai/mod.rs:166`. Edges: zero
occluders, a zero-length ray and an infinite position all resolve safely.
`RadarOccluder` living in `nova_ship` is right - `nova_scenario` already depends
on it and already inserts `LockSignature` from the same spawners, so no crate
gained an edge. The CHANGELOG entry is one line, correct section, 187 characters,
and does say the hostile side out loud as the task's Notes asked; no format break,
so no `**(breaking)**` is owed. Drop-reason plumbing is complete for the combat
slot and the log test's count moved 4 -> 5. The new range is registered with four
markers matching four roster slugs, built through `AppBuilder`, correctly
prefixed. Tests run: `nova_ship --lib --list` 860, confirming the record's count
exactly; `input::targeting::occlusion` 6 passed, confirming "six unit tests";
`input::ai::acquisition` line-of-sight and target-selection 11 passed, including
both new AI tests.

**Not checked:** NO TIMED MEASUREMENT WAS TAKEN. The host was not quiet - Lane A
recorded loadavg 5.61/4.93/5.20 at start and 4.27/4.78/5.10 at end, with two user
game sessions (`target/debug/nova-protocol --scenario first_shift`, PIDs 3937578
and 3939636) at ~215% CPU each and 4h44m elapsed, still running. Per the
measurement discipline the slot was not spent, and every perf number above is
arithmetic on authored content. `system_lock_line_of_sight` was read in full and
its four markers traced against the roster, but not compiled or run under Xvfb,
so the record's "Probe: OK, four markers in the timeline" is unverified, as is
whether writing `Transform` directly on the rock survives its
`TransformInterpolation`. The record's re-run list was not re-run. Whether
`first_shift` still plays is a live-run question: it carries 60 asteroids and two
planets, and both the player's locks and every AI pick in it are now
occlusion-gated. The `nova_scenario` spawner tests and the `flight_log` test were
read, not executed. Feel: whether a rock edge strobes a lock in real play, and
whether the total absence of a cue for a refused pick reads as a bug.

### Batch 7 - Campaign content, generated RON, creator docs, changelog, ranges (1458 lines)

Two lanes, both returned. ONE NEW BLOCKER, the most consequential of the run.
Story prose, dialogue and narrative choices were excluded per the owner's scope
limit; only mechanisms and documentation were judged.

**BLOCKER - `crates/nova_authoring/src/lint_walk.rs:806` - the `nova_authoring` lib-test target does not compile, so every test this batch added has never run.**
Reproduced: `cargo check -p nova_authoring --lib --profile test` ->
`error[E0063]: missing field `grammars` in initializer of `WalkedBundle``.
`WalkedBundle` gained `grammars: Vec<ShipGrammarConfig>` (`lint_walk.rs:47`) in
unpushed commit `aed4fab1` ("Added WFC generation to ships in editor"); the
`#[cfg(test)]` helper at `:806` still builds the old eight-field struct. Verified
`grammars` is ABSENT from `origin/master`, so this arrived with this range.
The consequence is larger than the fix. `cargo test -p nova_authoring --lib` has
been unrunnable since that commit, so batch 7's own new tests have never executed
once - including `every_scene_the_strike_plays_is_answered_by_a_handler`,
`the_cinematic_runs_its_shots_in_order_without_returning_attack_control`,
`every_preview_scene_passes_the_content_lint`,
`the_guard_channel_is_ignored_twice_before_it_matters`,
`a_line_buried_inside_a_scene_still_gets_a_face` and
`every_campaign_voice_has_its_own_portrait`. That is a SKIP being read as a PASS.
It also explains a gap in this review's own earlier coverage: batch 3 judged the
WFC commit and ran `nova_editor --lib` and `nova_scenario --lib`, both of which
compile. Nobody ran `nova_authoring`.
Fix: add `grammars: Vec::new(),` to the initializer at `:806`.

**MAJOR - `crates/nova_authoring/.../first_shift/mod.rs:300` and `:1245` - two comments claim a blown deadline gives the player back their ship. It does not.**
`:300`: "A blown deadline stops the chain, which ends the scene, which runs the
handler that gives the player their ship back - so the worst a mis-measured leg
can do is cut the set piece short." `:1245`: "What the scene owes the player back
is authored on the handler that answers it, never inside the chain, so the camera
and the controls come back on the one path out."
Verified against the runtime. `world.rs:848` handles a blown deadline by logging
an `error!`, setting `run.stopped = true`, then `endings.extend(
run.end_cinematic(false))` - so the ending fires with `skipped = false`,
indistinguishable from a normal finish. The handler that answers
`OnCinematicFinished` + `scene(SCENE_APPROACH)` + `BEAT_ATTACK` (`mod.rs:1282`)
calls ONLY `salvo_scene()`. Verified `release_camera()` appears exactly twice in
the chapter, `mod.rs:704` and `:796`, both BEFORE the strike, and the chapter's
own test asserts `released == 2`.
Failure scenario: if `APPROACH_LEG_DEADLINE` (100 s) or `ALIGN_DEADLINE` (30 s)
is ever blown - a slow host, a physics stall, a warship that cannot reach its
order point - the set piece is not "cut short". The salvo cinematic launches
immediately with the warship out of position and the player watches it from a
camera they do not control. The stated safety property does not exist.
Fix: correct both comments. The deadline hands off to the next scene; the real
degradation is a mis-staged salvo. Note the test that would have caught the shape
of this is one of the tests the BLOCKER above prevents from running, and its own
doc (`tests.rs:1062`) claims a stronger property than its body checks.

**MAJOR - `assets/base/thumbnails/first_shift.png` and `scripts/gen-scenario-thumbnails.py:66-67` - the shipped picker plate reads the old chapter name.**
Verified by re-running the script's OWN encoder against the committed bytes:
`encoded("first_shift", "First Shift") == disk` is **True**;
`encoded("first_shift", "An Ordinary Shift")` is **False**. So the plate renders
"FIRST SHIFT" while `assets/base/scenarios/first_shift.content.ron:4` names the
scenario "An Ordinary Shift", and `nova_menu/src/scenarios.rs:401` draws the name
and the thumbnail in the same details pane. A player selecting "An Ordinary
Shift" sees a plate captioned FIRST SHIFT. `--check` cannot catch it: the script
title and the PNG are stale together.
Line 67 additionally keeps the `second_shift` row, which is why the orphan
`assets/base/thumbnails/second_shift.png` is still tracked while
`base.bundle.ron` no longer lists it - nothing loads it, and the next generator
run recreates it. (The orphan itself was filed in batch 4; the stale CAPTION is
new, and is the reason the row cannot simply be deleted without regenerating.)
Fix: retitle line 66 to "An Ordinary Shift", delete line 67 and the orphan PNG,
regenerate the plate.
Arguably a BLOCKER since it ships and the player looks at it; kept MAJOR to stay
consistent with this run's bar, where BLOCKER has meant a build, test or probe
path that fails.

**MAJOR - `web/src/create/base-content.md:143`, `mod-files.md:170`, `reference.md:42` - the exhaustive creator catalog states a false list of content kinds, and `Grammar` has no creator documentation at all.**
Verified `nova_modding/src/lib.rs` has SEVEN `Content` variants: `Section`,
`Scenario`, `Campaign`, `Style`, `Ship`, `Grammar`, `Impact`. At v0.12.0 there
were five and the pages were right; this cycle added `Impact` and `Grammar`.
`base-content.md:143` still reads "There are no other content kinds - a content
file holds `Section`, `Scenario`, `Campaign`, `Ship`, and `Style` items only",
omitting both, and this batch edited the sentence two lines above it.
`mod-files.md:170` was updated to "The six content chapters" and lists `Impact`
but not `Grammar`. `reference.md:42` lists six, no Grammar.
Verified there is no `web/src/create/grammars.md` - the directory has an
`impacts.md` but no grammars page - and `base-content.md`'s id catalog does not
list the shipped grammar id `standard_hull`
(`assets/base/grammars/base.content.ron:3`). Meanwhile the CHANGELOG promises "a
mod ships its own procedural hull line".
So a mod author who reads the catalog is told the kind does not exist, and one
who believes the changelog has no field reference to write it from.
`docs/keeping-docs-in-sync.md` puts this in the "or every mod author reads a lie"
row.
Fix: correct all three counts to seven, add `Grammar` everywhere, and add a
`/create/grammars/` page.

**MAJOR - `web/src/create/base-content.md:226` - the base asset catalog omits the seven campaign portrait images.**
`base.bundle.ron` declares 12 PNGs: three textures, `banner.png`,
`thumbnails/first_shift.png`, and seven under `portraits/`. The page's Images
section lists five and never mentions `portraits/`. The Sounds section
immediately above names all 31 wavs exactly, with zero drift either way, which is
the standard this page holds itself to. A mod author who wants to reuse a comms
portrait, or just to know what `dep://base/` offers, concludes there are five
images. The already-filed "### Images (6)" heading mismatch is the symptom; the
missing seven is the defect.
Fix: add a `portraits/` bullet naming all seven and set the heading to 12.

**MAJOR - `web/src/wiki/getting-started.md:94` - "The camera takes eight authored shots" while the shipped chapter authors seven.**
Verified: `first_shift.content.ron` contains 7 `SetCameraAnchor((` and 0
`SetCamera((`, and `first_shift/tests.rs:895` pins exactly seven. History:
`d3a1946b` had 7 and the wiki said "seven"; `246c4c66` raised both to 8;
`40e5a550` dropped a shot back to 7 and left the wiki at "eight". A player
following the beat-by-beat walkthrough counts shots and is told to expect one
that is not there.

**MAJOR - `web/src/docs-manifest.js:37,:42-46` - the player wiki is indexed under a deleted scenario and five headings the page no longer has.**
The summary says "launch into the Shakedown Run"; `shakedown_run` is one of the
five scenarios the breaking changelog entry says are gone. The headings array
lists "The Shakedown Run, beat by beat" and Parts 1-4 by their old names, while
`getting-started.md` now has "An Ordinary Shift, beat by beat" and Parts 1-5 with
different names. The manifest drives the sidebar card, the page list and search,
so the card sells a scenario that no longer boots and searching "The detour" or
"The plate" returns nothing. A manifest-vs-page heading check across all 37 pages
found this to be the only genuine mismatch; every other is a legitimate search
keyword rather than a literal heading.

**MAJOR - `CHANGELOG.md:299` - an entry documents behaviour removed before it ever shipped, and names the chapter by its old title.**
"The wiki's first flight walks First Shift beat by beat, and the scenario list,
glossary and gravity figures read off the two new chapters." Second Shift was
created AND deleted inside this cycle. AGENTS.md: "Remove documentation for
behavior that was removed before it ever shipped." A 0.13.0 reader can never have
seen a second chapter. "First Shift" is also the pre-rename display name; the
same block at `:71` calls it An Ordinary Shift.
Distinct from the already-filed `:81` skip-prompt entry.

**MAJOR - `web/src/create/actions.md:69` - "the six kinds" for `SpawnScenarioObject` while there are seven.**
`nova_scenario/src/actions/spawn.rs:128` has `Anchor`, `Asteroid`, `Spaceship`,
`Beacon`, `SalvageCrate`, `Light`, `Planet`. `objects.md:6` was updated to "SEVEN
kinds" this cycle; `actions.md` was not, and both said six at v0.12.0 when six
was right. Same page `:10`: the at-a-glance row still reads "spawn one object:
asteroid, ship, beacon, crate, or light" - no Anchor, no Planet. An author
reading both pages in one sitting gets two different numbers.

**MAJOR - `docs/scenario-system.md:754` and `:783` - the dev book points at a file that does not exist and gives a recipe the action table invalidated.**
`:754` "Its `first_shift.rs` builds the New Game starter" - this batch split that
file into `nova_protocol/first_shift/{mod,marks,story,tests}.rs`, so
`first_shift.rs` is gone; "chapters" is also plural for one chapter. `:783` lists
the `actions/` submodules without `audio` or `cinematic`, both added this cycle,
and still says to add "an `EventActionConfig` variant in `actions/mod.rs`" - but
`actions/mod.rs:51` expands `registry::scenario_actions!`, so a contributor
following the book adds a variant by hand and collides with the macro.
Distinct from the already-filed `guide-extend-scenarios.md` recipes and the
`:228`/`:270` Sequence claims.

**MAJOR - `crates/nova_authoring/src/generation.rs:52`, `base_content/assets.rs:28`, `crates/nova_assets/src/collections.rs:213` - public rustdoc still describes the two-chapter campaign.**
`generation.rs:52` on `build_campaigns`: "listing its chapters in play order:
First Shift and Second Shift, both visible, both reachable for replay under the
campaign header." The campaign has one member. `assets.rs:28` and
`collections.rs:213` both document the `unknown_channel` portrait as "The cleanup
group's leader in chapter two" - that group was Second Shift's - while
`cast.rs:51` correctly documents the same portrait as the guard-channel voice. So
two crates' rustdoc attribute a shipped asset to a scenario that does not exist
and contradict the module that uses it.

**MINOR - `CHANGELOG.md:85` - `CinematicTitle` is filed under a different subsystem from every construct it shipped with.** (also raised in batch 5)
It sits under "Scenarios & Objectives"; `Cinematic` (`:248`), the `Cinematic`
filter (`:251`), `NarrativeCue` (`:253`) and `PlaySound` (`:256`) are all under
"Modding & Mod Portal", as is every other action entry. A mod author reading the
Modding section for this release's new scenario vocabulary gets four of the five.
The player-facing companion at `:83` correctly stays where it is.

**MINOR - `web/src/docs-manifest.js:503` - the manifest indexes "The five content chapters" while the page says six and the truth is seven.**
Fix with the Grammar finding, to seven.

**MINOR - a two-chapter comment sweep is outstanding across the authoring crate.** (both lanes)
Beyond the already-filed `stage.rs:12`: `stage.rs:1-9`, `:66`, `:159`, `:164`,
`:242`; `first_shift/marks.rs:7` and `:23`; `first_shift/story.rs:268`;
`first_shift/mod.rs:1318`; `first_shift/tests.rs:194`; `cast.rs:59`;
`pacing.rs:1` and `:95`; `nova_protocol/mod.rs:1` ("campaign chapters", plural);
`tests/campaign_membership.rs:50` ("Chapter one is the New Game entry and chapter
two is chained from it, so BOTH are visible", above an assertion that now loops
over a one-member vec); and `assets/base/base.bundle.ron:30` ("the campaign's two
chapters share one sky on purpose"). Also `Cargo.toml:108` ("The eight ships of
Nova Protocol's first two chapters") and `examples/systems/stress_hull_collapse.rs:41`,
which names the deleted `first_shift_08_attack_salvo.rs`. One sweep.

**MINOR - `first_shift/mod.rs:1888` and `pacing.rs:113` - a docstring contradicts the line under it, and `outro_sequence`'s `next_scenario` is now dead.**
`mod.rs:1888`: "The epilogue: the tease line, then the banner and the hand-off to
chapter two." Twelve lines later the call passes `None` with a comment saying
nothing follows. `outro_sequence` now has exactly one caller, which supplies
`None`, and `next_scenario` is documented nowhere in the function's doc block -
against AGENTS.md's rule that `Option` is reserved "for an override whose
documentation states what the absence means".

**MINOR - `first_shift/marks.rs:3` - the module doc undercounts the camera poses it owns and uses the pre-rename word.**
"the crates, and the two poses the attack is filmed from." The file holds five
strike poses (`:398`, `:404`, `:409`, `:437`, `:447`) plus `CINEMA_OPEN_OFFSET`
and `CINEMA_TRIM_OFFSET`.

**MINOR - `examples/playable/first_shift_08_strike_salvo.rs` - the rename stopped at the filename.**
`:34` `const SCENE_DONE: &str = "attack_salvo_scene_done";`, `:75`
`default_value = "attack"`, `:110` and `:157` `attack_plugin`, `:117`
`.step("load the attack salvo")`. Self-consistent, so nothing breaks - it is a
grep trap for the next person renaming these scenes.

**MINOR - `docs/development.md:243` - the playable catalog claims "What is on disk today, in reading order" and omits eleven playables.**
`first_shift_01_departure` through `first_shift_09_aftermath`, plus
`first_shift_map` and `first_shift_ships`, are all registered in `Cargo.toml:114`
and none appear. This batch renamed two of them without touching the list.
Sibling to the batch 6 finding about the same file's systems roster.

**MINOR - `first_shift/tests.rs:1062` - a test documents a stronger property than it checks.**
`every_scene_the_strike_plays_is_answered_by_a_handler` says the scene "gives them
back, on EVERY path out of it"; the body only asserts a handler exists for
`(scene, skipped=false)`. This is the MAJOR above in test form - and the test
cannot run at all until the BLOCKER is fixed.

**BLOCKER UPDATE (batch 4) - the picker capture is worse than recorded.**
Lane B independently reached `screenshot_scenario_picker.rs:54` and added two
facts. First, `scripts/gen-web-screenshots.py:116` sources the LIVING wiki figure
`wiki-scenarios-picker.png` from this example, so regenerating that figure fails
too - the batch 4 record named only the two news/wiki figures in
`docs/development.md:349`. Second, the constant's docstring at `:44-52` demands a
row that is "not the first chapter" so the campaign header reads as its parent;
with one chapter that requirement is UNSATISFIABLE. So retargeting the slug to
`first_shift` makes the example run but does not take the shot it documents. The
fix has to reframe the shot on the campaign header itself.

**Verified in this session:** the `nova_authoring` lib-test compile failure,
reproduced, and `grammars` confirmed absent from `origin/master`; the deadline
path in `world.rs:848` firing `end_cinematic(false)`, the `:1282` handler calling
only `salvo_scene()`, and `release_camera()` appearing exactly twice, both before
the strike; the thumbnail caption, by running the generator's own `encoded()`
against the committed bytes for both titles; seven `Content` variants against
`base-content.md:143`'s five, the absent `grammars.md`, and the shipped
`standard_hull` id; 7 `SetCameraAnchor` and 0 `SetCamera` against
`getting-started.md:94`'s "eight"; the scenario's "An Ordinary Shift" display
name; the generator's stale `first_shift` title row and surviving `second_shift`
row.

**Checked and sound.** Generated-content parity HOLDS and was answered
read-only: `cargo test -p nova_authoring --test content_ron_parity --test
campaign_membership` -> 3 passed, 0 failed. `content_ron_parity` rebuilds the
generator's own `content_files()` map and byte-compares every entry against disk,
and asserts the bundle ships exactly the generated set; it never mutates, and
`content gen` was NOT run. No hand-edit slipped into the RON.
`cargo run content lint` -> 0 errors, 0 warnings, 0 findings, 9 scenarios
balance-audited, 1 pre-existing ack, exit 0. `cargo check -p nova-protocol
--examples` and the same with `--features debug` -> both clean, exit 0, no
warnings. `catalog_matches_disk` passes: `Cargo.toml [[example]]` matches
`examples/` exactly, so no rename left a dangling entry and no deletion left a
stale block; `systems_ranges_assert_their_invariant_roster` fails with exactly
the three known `system_ship_editor` markers and no new drift. The `second_shift`
deletion sweep across `crates/`, `examples/`, `web/`, `docs/`, `scripts/`,
`assets/`, `Cargo.toml`, `webpack.config.js` and the capture scripts found no
campaign-advance, checkpoint, save-state or `NextScenario` path depending on a
second chapter - `campaigns.rs` is a one-element vec, `nova_protocol/mod.rs`
dropped the module and its re-exports, `outro()` passes `None`, and
`base.bundle.ron` dropped both manifest rows; everything else found is
documentation, reported above. Changelog rules: 135 `[Unreleased]` entries, zero
over 200 characters once wrapped lines are joined, no duplicate headings; the
`**(breaking)**` markers are correct in both directions, verified against
`git show v0.12.0:` - `StoryMessage` and the five deleted scenarios DID ship and
are marked, `second_shift` correctly is not, `SetCamera`'s optional `blend` needs
no marker, and the six new constructs correctly carry none; the two in-cycle
revisions (strike skippability, title-card wording) were each edited in place
rather than added beside, which is what the rule asks. Creator-reference counts
re-derived from code: 46 actions, 26 events, 6 filters, 7 objects, each matching
`reference.md`, its own page and the manifest. Every new construct's field table
matches its config, including the two `NarrativeCue` lint arms that really are
`warn`. The example-mod migration is complete and `webmods` sit at the versions
the changelog claims. `base-content.md`'s 31 wavs are byte-for-byte the bundle's
31. `hold_frames()` and the `backstop` headroom in the renamed salvo example are
sound, as is the `TRIM_RETURN_VERTICAL` gate fix. Craft basics in the new builder
files are clean: no `#[allow]`, prelude imports throughout, inline `mod tests`,
`campaign_membership.rs` correctly an integration test, and the
`story.rs`/`marks.rs` split the module docs promise is real.

**Not checked:** every `nova_authoring` unit test, blocked by the BLOCKER - this
is a SKIP, not a pass. No live or rendered run: no example, probe or scenario was
executed, so the deadline finding is grounded in reading `world.rs` and the
content rather than in a reproduction. No timing measurements; the host was not
verified quiet and the owner has game sessions running. `cd web && npm run ci`
was not run, so the manifest and markdown findings are not confirmed against a
real site build. Story prose, dialogue, tone and narrative choices were excluded
by the owner's scope limit.

## Verdict

Seven batches, fourteen lane runs, two lanes at a time. 3 BLOCKER, 45 MAJOR,
88 MINOR across roughly 17,000 lines of code in 47 unpushed commits. Story
commits were excluded by the owner's scope limit and are listed under "Not
reviewed".

The range is sound in its architecture and weak in its verification. Every
substantial mechanism this range adds - WFC hull generation, the scenario
vocabulary, the occlusion gate - is correctly shaped, and the one genuine
shipped-format break (`StoryMessage` -> `NarrativeCue` with a required `channel`)
was handled exactly right: marked `**(breaking)**`, migrated across all 49 cues
in both hand-written mod sets, and lint-clean. What is weak is the layer that
proves it: the probe roster drifted, a test target stopped compiling, a screenshot
range points at deleted content, and three of the four documentation surfaces
that describe the new vocabulary disagree with the code.

### The three BLOCKERs, in fix order

1. `crates/nova_authoring/src/lint_walk.rs:806` - one missing field breaks the
   whole crate's lib-test target. Fix FIRST: it is one line, and until it lands
   nobody can run the tests that would validate the rest of the fixes.
2. `crates/nova_probe_cli/tests/catalog_drift.rs:305,567` - add three
   `system_ship_editor` roster rows and bump `SYSTEMS_INVARIANTS` 234 -> 237.
3. `examples/screenshots/screenshot_scenario_picker.rs:54` - the dangling
   `second_shift` row. Needs a decision, not just a retarget: the shot's stated
   purpose is unsatisfiable with a one-chapter campaign, so it has to be reframed
   on the campaign header.

### The owner-raised item stands on its own

`NarrativeChannel` (`crates/nova_hud/src/comms_panel.rs:46`) is a closed enum
whose entire body is a lookup table, mirrored by `NarrativeChannelConfig` with a
hand-written `From`. It is the one mirrored enum in the range that fails the
test the other four pass: its variants dispatch to no distinct code, they only
return data. Channels should be authored content. The fix is cheap and owes
nothing to compatibility - the type has zero hits at `origin/master`, it arrived
in unpushed `246c4c66`, and v0.12.0 still used `StoryMessageActionConfig`. Per
AGENTS.md a format that never shipped needs neither a `**(breaking)**` marker nor
a migration note.

### Suggested fix grouping

- **A. Unblock** - the three BLOCKERs. Nothing else can be validated first.
- **B. Defects that reach a player** - the travel-lock silent drop, the
  `Occluded`-over-`OutOfRange` precedence, the pad-player skip prompt, the
  comms body colour, the stale thumbnail caption, the `PlaySound` volume bound.
- **C. The comms channel architecture** - the owner's item, self-contained.
- **D. False contracts** - the two untagged-widget docstrings and the three
  `wiki/hud.md` sentences that follow them; the two deadline comments; the four
  "Warn" claims; the two `wiki` pages that never mention cover; the seven-vs-five
  content kinds and the missing `grammars` page; the camera shot count; the
  manifest's dead headings.
- **E. Housekeeping** - the changelog entries that break the house rules, the
  two-chapter comment sweep, the rename leftovers, the roster omissions.

Take B and C before D: a false comment costs a reader, a silent travel-lock drop
costs the player mid-mission.

### What this review did not do

No timed measurement was taken anywhere in the run. The host was never quiet -
two `nova-protocol --scenario first_shift` sessions were live throughout at
roughly 215% CPU each - so every performance finding is arithmetic on authored
content and is labelled unmeasured. No example, probe or scenario was executed
under Xvfb, so nothing here is confirmed by a rendered frame. `npm run ci` was
never run. The `nova_authoring` unit tests could not run at all. Feel was not
judged: whether a rock edge strobes a lock in play, and whether the total absence
of a cue for a refused lock reads as a bug, are open questions a live session
would answer in minutes.

## Fix pass

Fixes are recorded here as they land. Nothing is staged or committed; the
working tree carries them for the owner to review. The owner's own `lore/` work
and its staged index entries were not touched.

### Group A - the three BLOCKERs. DONE, all verified green.

**A1. `crates/nova_authoring/src/lint_walk.rs` - the lib-test target compiles again.**
Added the missing `grammars` field to the `#[cfg(test)]` `walked()` helper.
Derived it from `content` with a `filter_map`, the way the helper already
derives `sections`, `ships`, `scenarios` and `campaigns`, and the way the real
`read_bundle` builds the same field - NOT `Vec::new()`. `Vec::new()` would have
compiled and silently made the grammar lint at `lint_walk.rs:255` unreachable
through the test helper, which is the same class of defect this review kept
finding.
Verified: `cargo test -p nova_authoring --lib` -> **112 passed, 0 failed**. All
six tests named in the batch 7 BLOCKER now run and pass, including
`every_scene_the_strike_plays_is_answered_by_a_handler` and
`the_cinematic_runs_its_shots_in_order_without_returning_attack_control`.
Worth stating plainly: the skip was masking a compile break, not hidden test
failures. Nothing was wrong underneath it.

**A2. `crates/nova_probe_cli/tests/catalog_drift.rs` - the roster matches the ranges again.**
Added the three missing `system_ship_editor` slugs in the example's own emission
order - `a named document saves without asking again` after `a seeded hull is
entered and inspected as a ship`, then `a generated hull comes out bound` and
`a named save writes a bundle of its own` after `the flown ship wears the skin` -
and bumped `SYSTEMS_INVARIANTS` 234 -> 237.
Verified: `cargo test -p nova_probe_cli --test catalog_drift` -> **2 passed, 0
failed**. Was 1 passed, 1 failed.

**A3. `examples/screenshots/screenshot_scenario_picker.rs` - the picker capture points at content that exists.**
Retargeted `CAMPAIGN_CHAPTER_ROW` to `first_shift` and rewrote the docstring.
The old rationale demanded a row that is "not the first chapter" so the campaign
header reads as a parent rather than as that row's own label; with one chapter
that is unsatisfiable, so the docstring now says the header reads as its parent
by the row's INDENT instead. The grouping is still the figure's subject: the
picker draws the `[-]` header with `first_shift` indented under it and
`example_arena` uncampaigned in the tail.
Verified: `cargo check --features debug --example screenshot_scenario_picker` ->
clean. Not run under Xvfb; the shot itself is unverified.

### Group B - defects that reach a player. Partly done.

**B1. `scripts/gen-scenario-thumbnails.py` + `assets/base/thumbnails/` - the picker plate reads the right name.**
Retitled the `first_shift` row to "An Ordinary Shift", deleted the `second_shift`
row, regenerated, and removed the orphan `assets/base/thumbnails/second_shift.png`
(referenced by nothing - `base.bundle.ron` had already dropped it).
Verified three ways: the script's `--check` reported `STALE ... first_shift.png`
BEFORE the regeneration, which confirms the finding independently of the review;
after regenerating, `--check` reports all 8 match byte for byte; and re-running
the generator's own `encoded()` against the committed bytes now returns True for
"An Ordinary Shift" and False for "First Shift", the exact reverse of the finding.
`git diff --stat` confirms only `first_shift.png` changed - the seven webmod
thumbnails round-tripped byte-identically, so no art churn.

**B2. `crates/nova_hud/src/cinematic_prompt.rs` - the skip prompt names the key the player actually has.**
Now reads `action.sources().next()` and `glyph_label()`, the house idiom from
`nova_ship/src/input/player/hints.rs:123`, instead of `action.keyboard.first()`
and `label()`. `cinematic_skip` ships a `DPadDown` default, so a pad player was
told to press ENTER; and a stored override with an empty keyboard column hid the
prompt entirely while the skip still fired. The comment now says why.

**B3. `crates/nova_hud/src/comms_panel.rs` - the card is drawn in its channel, body text included.**
`NarrativeChannel::body()` joins `tone()`/`tag()`/`signal_strength()` as one more
row of the same table, and the body `TextColor` reads it. Three constants replace
the single `COMMS_BODY`, each the channel's accent lifted about three quarters of
the way to white, which is how the original pale blue was derived from `BLUE`.
The guard channel is NOT darkened here - its faintness stays
`signal_strength()`, so one mechanism carries the frame, the header and the body
alike. This makes `CHANGELOG.md:109` ("the crew in phosphor") and
`actions.md:316` ("phosphor green") true rather than rewording them.
NOTE FOR THE OWNER: the two new tints are reasoned from the existing
construction, not eyeballed - I cannot render. They want a look before release.
Also enumerated `signal_strength`'s wildcard arm, and made the fallback icon
tile's border follow the tone like the authored branch already did (it was
drawing a phosphor border over an amber wash on a Guard line).

**B4. `crates/nova_ship/src/input/targeting/contacts.rs` - a range drop is no longer reported as cover.**
Extracted `max_lock_range()` and `incumbent_max_range()` so the collector and the
drop branch share ONE definition of the gate, then made the drop-reason match ask
in the collector's own order: range first, then the ray. A hostile that burns
past its gate with a rock on the line now reads "out of range" instead of sending
the player to wait for a line that was never the problem. No logic is duplicated;
the collector calls the same helper it did inline before.

**B5. `crates/nova_scenario/src/lint/scenario.rs` - `PlaySound` gain is bounded.**
Added the missing `check_action` arm: a non-finite gain, or one outside [0, 1],
is a lint error. Nothing downstream clamped it - the value rode `PlaySfx`
straight into the voice, so `volume: Some(40.0)` was a 40x gain in the player's
headphones. Every sibling action added in that batch already had a lint arm.
New test `a_sound_gain_outside_zero_to_one_errors_and_a_gain_inside_it_does_not`
covers 40x, negative, NaN, 0.4, 0.0, 1.0 and omitted.

**Verification for group B:** `cargo test -p nova_ship --lib input::targeting` ->
62 passed. `cargo test -p nova_hud --lib -- comms cinematic` -> 27 passed.
`cargo test -p nova_scenario --lib -- lint::scenario` -> 48 passed, plus the new
test green on its own. `cargo check -p nova_scenario -p nova_hud --lib` clean, no
warnings (an import left unused by B3 was removed).

**B6. `crates/nova_ship/src/input/targeting/` - the travel lock survives cover; acquiring anything does not.**
The owner's answer to the design fork: ACQUIRE needs sight, KEEPING does not.
`Lockable` is now a 5-tuple carrying `in_sight`, and the collector RECORDS the
ray instead of rejecting on it, because the two slots no longer agree about what
it means. Each slot applies its own policy: `radar_pick` refuses to offer a body
it cannot see (so nothing is ever acquired blind, and hysteresis cannot hold an
incumbent that went behind a rock), the combat lock requires membership AND
sight, the travel lock requires membership only, and the threat set filters on
sight like the pick does. A rock drifting across a destination no longer cancels
a burn the player is flying; a rock crossing a gunnery lock still drops it, which
is the fair trade for the hostile losing its own pick at the same instant.
This also subsumed B4's helper extraction, which was reverted: the drop-reason
branch now reads the collector's own pass (`Some(_)` -> `Occluded`, else the
candidate query -> `OutOfRange` / `TargetGone`) instead of re-deriving the gate,
so the log cannot disagree with the decision and one duplicated ray cast is gone.
A helper with one caller would have been the wrong shape.
New tests: `radar_pick_never_offers_a_body_it_cannot_see` and
`cover_drops_a_combat_lock_but_keeps_a_travel_designation` - both slots take the
same body over a clear sky, then one rock on one line in one pass makes them
answer differently.
Verified: `cargo test -p nova_ship --lib input::targeting` -> 64 passed (was 62).
Whole crate after widening a shared type: `cargo test -p nova_ship --lib` ->
**862 passed, 0 failed**, exactly the 860 of record plus the two new.

**B7. `crates/nova_ship/src/input/targeting/occlusion.rs` - the scanner is exempt from its own ray.**
The landmine filed as a MINOR, fixed while the file was open. `is_occluded` now
takes the scanner as well as the target and `stops_radar_between` excludes both
ends of the line. Vacuous today - the two `RadarOccluder` sites are the asteroid
and the planet - but `origin` is `live_structure_anchor`, INSIDE the scanner's
own hull, and `solid: true` makes an origin-inside hit return distance zero, so
the day a hull occludes (the plan's own next step: a picket screening its
wingman) every ship would blind itself instantly. The sibling line-of-FIRE gate
at `ai/guns.rs:253` already does this.
New test `a_scanner_that_stops_radar_still_sees_past_its_own_hull` drives a rock
AS the scanner - the same shape a screening picket would have - and asserts the
exemption is the scanner's alone.
Verified: `cargo test -p nova_ship --lib input::` -> 257 passed, 0 failed.

**A4 (NEW, found during the fix pass) - `crates/nova_assets/tests/*.rs` x4 - four integration-test targets do not compile.**
Same class as A1 and found the same way: the `Content::Grammar` variant added in
the unpushed range broke every exhaustive `match Content` nobody compiled.
`scenario_gate_course.rs:179`, `scenario_branch_choice.rs:190`,
`scenario_act_machine.rs:183` and `scenario_provocation.rs:218` each list the six
old variants; the compiler's own suggestion was
`nova_modding::Content::Grammar(_) => todo!()`.
Verified `Grammar` has zero hits in `origin/master:crates/nova_modding/src/lib.rs`,
so all four targets built before this range and none of them has been built since.
Fixed by adding `| Content::Grammar(_)` to each `=> None` arm - these helpers
pull the one `Scenario` out of a parsed content list, so a grammar is correctly
"not the item I asked for" rather than anything to handle.
This is the third distinct compile break in the range reachable only by building
a target the lanes did not build. Worth a CI target that builds `--all-targets`
across the workspace.

### Group D - false contracts. DONE.

**D1. The two untagged-widget docstrings.** `cinematic_prompt.rs:8` and
`cinematic_title.rs:11` both justified themselves with "a scene drops the HUD to
its cinematic level". Nothing drops the level; `HudVisibility` is the player's
grave/tilde toggle and the menu's. The reason is the other way round and now says
so: untagged means the PLAYER's toggle cannot delete the way out of a scene that
is still playing. The prompt's doc also names the cost - it and the title card
are the only two HUD surfaces that do not answer the toggle.

**D2. `web/src/wiki/hud.md:28,:46,:50`.** "Cinematic clears every element" and
"all of them clear at Cinematic" were false for exactly those two surfaces. The
page now says "the whole contextual HUD", and a new paragraph names the two
exceptions and why a capture taken during a cutscene keeps them.

**D3. `first_shift/mod.rs` - the two deadline/control comments.** The claim that
a blown deadline "runs the handler that gives the player their ship back" is
false in both halves: `end_cinematic(false)` fires an ending the next handler
cannot tell from a clean finish, and the handler it reaches launches the salvo.
The comment now says what actually degrades - a salvo staged with the warship
wherever the blown gate left it - and that the backstop bounds the WAIT, not the
damage. The second comment claimed the camera and controls "come back on the one
path out"; the strike is the one interval the chapter never hands back (3
suspends, 2 resumes, pinned by
`only_the_conversation_holds_return_control_before_teardown`), which is the point
of the chapter and now reads as deliberate.

**D4. The four "Warn" claims.** `filters.md:108`, `actions.md:583`,
`events.md:203` and `CHANGELOG.md:252` all promised a lint Warn for an unplayed
`Cinematic` filter key; `lint/scenario.rs:1541` pushes `LintIssue::error`, and a
lint error refuses the scenario at load. All four now say Error and two of them
say what that costs. The neighbouring `CancelCinematic` Warn (`actions.md:619`,
code `lint/scenario.rs:763`) was verified still a Warn and left alone.

**D5. The two wiki pages that never mentioned cover.**
`combat-weapons.md:69` promised an attack orbit that "keeps it circling all the
while"; a hostile that loses the line loses the PICK and goes passive with no
grace. Rewritten to say cover buys a full disengage.
`targeting-radar.md` got the rule it never had: `:3` no longer says a lock
"sticks until you clear it", `:68`'s drop enumeration gains cover, and a new
"Line of sight" section states the asymmetry the player has no cue for - a
refused pick is silent, so the wiki is the only place it can be learned - and
splits held combat locks (drop) from held travel locks (stay).

**D6. `create/objects.md:66,:156,:307` plus a new section.** The Asteroid and
Planet rows now say the body blocks radar, and `engage_range` says range is a
ceiling on top of sight, so the documented "long-watch emplacement" parked behind
a planetoid is no longer a silent trap. A new "Radar line of sight" section
carries the author-facing consequences (an oscillating picket, cover that can be
shot away, and the travel-lock exemption).

**D7. The seven-vs-five content kinds, and the missing page.** `Grammar` was a
shipped content kind with no page and no mention. Added
`web/src/create/grammars.md` - the `Grammar` item, the grid and its skin-driven
floor, the vacuum weights as the silhouette dial, the seeded keel, and parts with
their weights/aims/zones - and registered it in `docs-manifest.js`.
Corrected the counts and lists it was missing from: `mod-files.md:170`
(six -> seven, plus the bullet), `base-content.md:144` (the "no other content
kinds" sentence, which omitted both `Impact` and `Grammar`), `reference.md:42`
and its base-ids row, and `docs-manifest.js:503`. `base-content.md` also gained a
"Ship grammars" id table so `standard_hull` is in the id catalog.

**D8. `base-content.md:226` - the base asset catalog is complete.** The Images
section listed five under a "(6)" heading while the bundle declares 12. Added the
`portraits/` bullet naming all seven, heading now "(12)".

**D9. `wiki/getting-started.md:94` - eight -> seven authored shots**, matching
`first_shift.content.ron` and the test at `first_shift/tests.rs:895`.

**D10. `docs-manifest.js:37,:42-46` - the wiki card sells a scenario that exists.**
Summary retargeted from "the Shakedown Run" to "An Ordinary Shift", and the five
stale headings replaced with the page's real five parts, so the sidebar card and
search agree with the page.

**D11. `docs/scenario-system.md:754,:783`.** The book pointed at a
`first_shift.rs` this cycle split into `first_shift/{mod,marks,story,tests}.rs`,
and told a contributor to hand-write an `EventActionConfig` variant that
`registry::scenario_actions!` generates. Both corrected; the `actions/` submodule
list gains `audio` and `cinematic`.

**D12. Public rustdoc describing the deleted chapter.** `generation.rs:52` said
the campaign lists "First Shift and Second Shift"; `assets.rs:28` and
`collections.rs:213` both filed the `unknown_channel` portrait as "the cleanup
group's leader in chapter two", contradicting `cast.rs:51`, which correctly
documents it as the guard-channel voice. All three now match the shipped content.

### Group E - housekeeping. DONE.

**E1. `CHANGELOG.md`.** Dropped the skip-prompt dock-clearance entry (`:81`) -
the widget shipped in this same cycle, so there is no released version the fix
reads against. Rewrote the two-chapter wiki entry (`:299`) and dropped its
pre-rename title. Moved the `CinematicTitle` entry out of "Scenarios &
Objectives" into "Modding & Mod Portal", beside `Cinematic`, the `Cinematic`
filter, `NarrativeCue` and `PlaySound`. Added the `PlaySound` gain bound to the
entry that documents the field rather than filing B5 as its own line.

**E2. The two-chapter comment sweep.** `stage.rs` (module doc, the plate, the
body ids, the beacon ink, the planetoid docstring), `first_shift/marks.rs:7,:23`,
`first_shift/tests.rs:194`, `cast.rs:19`, `nova_protocol/mod.rs:1`, `pacing.rs:1`,
`campaign_membership.rs:50`, `assets/base/base.bundle.ron:30`, `Cargo.toml:108`.
`stage.rs` keeps its reason for existing: the map is meant to be revisited, and
authoring it once is what stops a later chapter's numbers from drifting - that is
now stated as intent rather than as a second chapter that exists.
Also `marks.rs:3` (two poses -> seven, named), `first_shift/mod.rs:1888` (the
outro no longer claims a hand-off its own call refuses), and `pacing.rs:113`
(`outro_sequence`'s `next_scenario` now documents what `None` means, per the
`Option` rule).

**E3. Rename leftovers.** `examples/playable/first_shift_08_strike_salvo.rs`:
`SCENE_DONE`, the `--label` default, `attack_plugin` and the step name all said
"attack". `examples/systems/stress_hull_collapse.rs:41` pointed at the deleted
`first_shift_08_attack_salvo.rs`.

**E4. `docs/development.md:243` - the playable catalog is complete.** It claimed
"what is on disk today, in reading order" while omitting thirteen: the nine
campaign scene fixtures, `first_shift_map`, `first_shift_ships`,
`asteroid_kinds` and `planet_types`.

### Group C - the comms channel architecture (the owner's item). DONE.

Channels are content now. `NarrativeChannel` - the closed enum in `nova_hud`
with a four-method lookup table hanging off it - is gone, and with it the
`NarrativeChannelConfig` mirror enum in `nova_scenario`.

**The type.** `nova_gameplay::narrative_channel` owns `NarrativeChannelConfig`
(`id`, `tone`, `tag`, `signal_strength`), the `GameChannels` catalog resource,
and the three base ids as constants. It sits in `nova_gameplay` because both
ends need it: the HUD reads a resolved channel off each line and the scenario
layer resolves the id a cue names - the same split `StoryFeed` already had.
`ChipTone` is re-exported from its prelude, because a crate that authors a
channel has to name its tone without depending on the widget crate.

**The wire.** `NarrativeCueActionConfig.channel` is a `String` id.
`world.rs`'s sync resolves it once against `GameChannels`, so the HUD never
sees an id. `Content::Channel` is the eighth content kind; the merge inserts
`GameChannels` beside the other seven registries.

**The gate.** A cue naming an id nothing authors is an Error at lint
(`lint_scenario` took a `known_channels` set) and again at load
(`start_errors`), never a fallback. `lint_channel_config` checks the row itself:
non-empty id, finite strength in (0, 1], no empty tag. The walk resolves a
mod's channels the way it resolves its sections - base, its declared
dependencies, its own - so a bundle cannot borrow a channel that merely happens
to be installed.

**The content.** `crates/nova_authoring/src/base_content/channels.rs` authors
the three rows; `assets/base/channels/base.content.ron` is generated from it and
listed in the base bundle. `ChipTone::body()` replaced the three hand-tuned
`COMMS_BODY_*` constants with one lift toward white (crew reproduced exactly,
work within 2/255, guard slightly paler - recorded, deliberate).

**Craft, taken along the way.** `Content::kind()` / `Content::id()` /
`Content::into_scenario()` replaced eight per-kind matches that each had to be
revisited for every content kind: the merge's duplicate check (seven
near-identical blocks, now one loop), its resource-ref gate, `lint_walk`'s
`file_of` and its resource-ref walk, and four integration tests that broke on
`Content::Channel`. `merge_content_item`'s own match stays - routing an item
into its bucket is what it is FOR. `merge_content_item` takes `&mut MergeOutcome` instead
of eight out-params. `on_load_scenario` was one parameter under Bevy's ceiling;
its five content reads are a `ContentGate` `SystemParam` now.

Per AGENTS.md, no `**(breaking)**` marker and no migration note: `channel:`
never shipped - it arrived in unpushed `246c4c66`, and v0.12.0 still used
`StoryMessageActionConfig`. The existing `StoryMessage` is `NarrativeCue` entry
carries the id spelling instead.

Verified: workspace `cargo check --all-targets` clean; `nova_scenario` 389,
`nova_hud` 252, `nova_authoring` 115, `nova_assets` 75 + all integration
targets, `nova_editor` 473, `nova_os_ui` 118, `nova_wfc` 13, `nova_gameplay`
280 - all green. `content lint`: 0 errors, 0 warnings over base and both
webmods. `content gen` regenerated the base RON. Live run of
`first_shift_01_departure` under Xvfb draws Demir's card in the resolved work
channel (screenshot inspected).

Not in scope, noted: the copy of The Ledger installed on this machine under
`mods://` is a pre-rename bundle and fails to parse `StoryMessage`. That is the
already-recorded `**(breaking)**` rename doing what the changelog says it does,
on a user-machine artifact - the `webmods/the-ledger` source in this repo is
current and lints clean.

### Group F - grammar as content, the load half. DONE.

The same authoring rule Group C closed for channels, closed for grammars: a
grammar is now refused where it is AUTHORED rather than when a generator is
asked to run it.

**The load half** (`merge.rs:373`). `register_bundles` ran the content gate over
ships, scenarios and campaigns and did nothing to a grammar but insert it, so
"an error at lint, then at load" had no load half - and `content lint` walks
`assets/` offline, where a cross-mod reference cannot be decided at all. A
`for grammar in &outcome.grammars` loop beside the ship loop now files findings
keyed on the grammar id. `tests/grammar_load_gate.rs` is the review's own
scenario: mod `parts` ships a drive, mod `hulls` ships a grammar seeding it,
`parts` goes off, and the grammar's dangling id is an Error. Confirmed to fail
without the loop (0 errors instead of 1).

The grammar stays REGISTERED. Dropping it would silently restore whatever it
overlaid, and a hidden fallback is the thing the rule exists to refuse.

**The seeds' own footprints** (`lint/ship.rs:175`). The gate floored each axis at
3 and stopped, while `nova_wfc::runnable` refused three further shapes it could
not see: a stern drive that does not fit (`half_width < drive.x + 1`,
`height < drive.y`, `length < drive.z + 1`), a `bow_gun` that is not 1x1 across,
and `length < bow.z + drive.z + 2`. The lint had the data all along -
`KnownSections` keeps `collider` - so it now resolves both seeded roles and runs
the same three bounds. The old `half_width >= 3 && grid.length < 3` arm went
with them: it could only ever fire alongside the axis floor that already had.

**The ceiling** (`grid.rs:55`). `Grid::cells()` was a `u32` multiply and every
bound was a lower one, so `2048x2048x1024` linted clean and multiplied to
exactly 2^32 - a panic in a dev build, a wrap to zero in a release one.
`GrammarGrid::cells()` is a `u64` product now, `MAX_GRAMMAR_CELLS` (65,536) is
the authored ceiling, and `runnable` refuses past it BEFORE anything else,
because every check under it is about a grid worth measuring. `Grid::cells()`
widens before multiplying rather than after. The shipped hull is 220 cells.

**The tests** (`lint/ship.rs:134`). 92 lines of gate had none, and that is how
the dead arm above survived landing. Seven now, one per failure mode, on a
`grammar()` / `grammar_sections()` fixture pair: the well-formed baseline, the
unknown id on each of the six roles it can be spelled in, the empty draw and the
four unpayable weights, the axis floor, the three seed-fit bounds (with the
"one more cell and it fits" case), the ceiling (2^32 and ceiling +/- 1), and the
vacuum prices - including that only `base` refuses a zero, since a taper of zero
is an evenly sparse hull, which is taste. `nova_wfc`'s own
`a_grammar_the_collapse_cannot_run_in_is_refused_rather_than_run` asserted only
`is_err()` across eight bent grammars; each row now carries the line it must be
refused WITH, and the ninth row is the oversized grid.

**Selection: NOT fixed, and now said so.** `get_grammar(id)`'s parameter is
answered with `STANDARD_HULL_GRAMMAR_ID` by every caller in the tree; there is
no picker, no scenario field and no flag. A mod's `Grammar` under a new id
merges and is reached by nothing. Building the picker is a feature, not a review
fix, so what changed is the claims: `ShipGrammarConfig` no longer says "a new id
is a new kind of ship", `name` no longer says a picker shows it,
`STANDARD_HULL_GRAMMAR_ID` drops "unless told otherwise", `get_grammar` states
the promise it does not keep, and the two CHANGELOG entries plus
`web/src/create/grammars.md` and `base-content.md` say plainly that
`standard_hull` is the only id anything asks for. `grammars.md` gained a
"Which grammar the generator reads" section ahead of everything else, and its
worked example declares `standard_hull` rather than teaching an id that cannot
be reached. The grid section documents the seed-fit bounds and the ceiling.

Still open from this theme: grammar SELECTION (`ship_grammar.rs:222`) - a
feature, unscheduled.

### Still open

The verdict's A-E grouping is done. It was a CURATED list, not the whole
findings set: 4 BLOCKER + 20 MAJOR are closed, and the rest of the batch blocks
were recorded but never scheduled. What stands, re-verified against HEAD on
2026-09-06:

**19 MAJOR, by theme.** (Group F closed four: the grammar load gate, the
seed-fit bounds, the grid ceiling, and the untested gate behind all three. The
previous count of 24 was one high - it read the three-file docs bullet as
three.)

Editor and WFC correctness (batches 1-3):
- `nova_wfc/src/collapse.rs:327,:683` - both documented refusal paths untested.
  Group F gave the SIBLING test
  (`a_grammar_the_collapse_cannot_run_in_is_refused_rather_than_run`) its message
  assertions; these two paths still have no test at all.
- `nova_editor/src/bundle.rs:90` - a save named "Sandbox" derives
  `editor_sandbox` and takes over the editor's own stage range.
- `nova_editor/src/bundle.rs:63,:488` - the `editor_` prefix is a listing
  convention, not a property the editor owns.
- `nova_editor/src/generate.rs:186,:379` - every Generate overwrites the ship's
  cladding and style; the cladding toggle the record claims never landed.
  Confirmed still `node.style = hull.style.clone();` at `:187`.
- `nova_editor/src/generate.rs:381` - the lint-refusal arm has no test.
- `nova_editor/src/node.rs:1310` (unmeasured) - per-frame document walks are
  O(sections) or O(sections^2); a generated hull multiplies the input.
- `nova_editor/src/scenario.rs:1201` - `retarget_retries` recurses into
  `Sequence` but not `Cinematic`, so a retry authored inside a scene keeps the
  wrong scenario id through a save. Confirmed: the match still has one arm.
- `nova_ui/src/screen/list.rs:93` - the `Hovered` fix has no failing-without-it
  test.
- `examples/playable/wfc_arena/stamps.rs:36` - the surviving stamp hardcodes the
  grid the grammar now authors.

Grammar as content (the rest closed in Group F):
- `nova_ship/src/sections/ship_grammar.rs:222` - a mod can only RETUNE
  `standard_hull`; a new grammar id is still unreachable. The claims that said
  otherwise are corrected; SELECTION is a feature and is unscheduled.

Coverage:
- `nova_scenario/src/actions/cinematic.rs` - the Cinematic and its skip path have
  no harnessed range, only unit tests.
- `nova_hud/src/comms_panel.rs:342` (perf, unmeasured, PRE-EXISTING) -
  `sync_comms_cards` rebuilds the whole visible stack every frame, idle included.

Documentation the range left behind:
- `docs/project-tour.md:30`, `docs/architecture.md:12`, `docs/concept-index.md:82`
  - `nova_wfc` is a workspace member in neither crate map, neither dependency
  graph, nor the concept index, whose WFC row points at a deleted file.
  Confirmed: zero `nova_wfc` hits in all three.
- `docs/ship-layout-sense.md:3` - written against the deleted file; every
  constant it cites is gone.
- `docs/guide-extend-scenarios.md:167` - Recipe 3 tells a contributor to
  hand-edit generated code and names symbols that no longer exist.
- `docs/scenario-system.md:228` - "`Sequence` is the one action whose state does
  not live in the action" is false now, and two neighbouring claims with it.
- `web/src/create/author-a-scenario.md:15` - still describes the single
  `editor_save` slot.
- `web/src/wiki/keybinds.md:255` - says torpedoes fire on the left mouse button;
  an editor-placed torpedo takes `F`.
- `CHANGELOG.md:373` - an unreleased entry describes the stamp this range
  deleted.

**~85 MINOR**, in the batch blocks above. Not triaged individually; each was
recorded where it was found.

Nothing here blocks the range: it builds, it lints, and it plays. The heaviest
left are `scenario.rs:1201` (a save silently rewrites a retry wrong),
`generate.rs:186` (every Generate overwrites the builder's cladding and style),
and `bundle.rs:90` (a save named "Sandbox" takes over the editor's own stage
range).
