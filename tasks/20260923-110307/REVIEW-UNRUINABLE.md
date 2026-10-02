# Review: Unruinable intact-layout check (regional-ship-runtime, uncommitted)

Scope: the `Unruinable` variant, `check` gate, `wreck_floor` clamp removal,
and their tests/docs, per the owner's change description. Reviewed the
working tree (uncommitted) at
`/home/alex/.cache/sprouts/nova-protocol/regional-ship-runtime`. Read-only;
no edits, no cargo runs.

## Verdict

No BLOCKER or MAJOR findings. The gate is sound: `check` computes cells,
cubes and dock backing identically to `generate_wreck`, so an accepted
intact layout always has a wreck floor of at least 1. One MINOR
documentation-only finding below (a task note, not shipped docs).

## (1) `check` vs `generate_wreck`: same cells/cubes/backing, floor >= 1 guaranteed

CONFIRMED CLEAN. `check` (ship_layout.rs:1870-1927) and `generate_wreck`
(ship_layout.rs:446-489) both compute:
- `filled_cells(snapshot, design)` on the same `design`
- `dock_backing(snapshot, design)` on the same `design`
- `loose`/`cubes` = the filled cubes minus dock backing
- `wreck_floor(&cells, &loose, target)`

`check` calls `wreck_floor(&cells, &loose, 1)` and rejects on `== 0`
(ship_layout.rs:1919-1925). `generate_wreck` calls
`wreck_floor(&cells, &cubes, target)` with `target = wreck_omissions(...)`,
always >= `MIN_WRECK_OMISSIONS` (3) (ship_layout.rs:112, 452, 455, 494).

`wreck_floor`'s per-cube pair test (cell_bounds unchanged + `joined`) does
not depend on `target`; `target` only bounds how many cubes the scan visits
before it stops early (`if pairs.len() >= target { break; }`,
ship_layout.rs:571-573) and caps the returned count
(`pairs.len().min(target)`, ship_layout.rs:588). So if `check`'s
target-1 scan finds >= 1 valid pair, `generate_wreck`'s target->=3 scan over
the same cube set is guaranteed to find that same pair (or more), so its
floor is also >= 1. Since `generate_ship`'s attempt loop only returns a
design that passed `check` (ship_layout.rs:420-421), every `intact.design`
handed to `generate_wreck` has floor >= 1. Only Cause 2 (cumulative
ruin exceeding the per-cube-alone floor) can still produce `Unruined`, as
intended.

## (2) `wreck_floor` returning 0: every caller checked

CONFIRMED CLEAN. Three call sites, all accounted for:
- `check` (ship_layout.rs:1923): explicitly tests `== 0` and returns
  `Constraint::Unruinable`. This is the only place 0 is a live, expected
  value.
- `generate_wreck` (ship_layout.rs:455): per (1), can only see a `cells`
  design that already passed `check`, so its `floor` is never 0 in
  practice; no clamp is needed here to avoid a bad value.
- test `a_wreck_keeps_its_fittings_size_and_connection_and_breaks_its_mirror`
  (ship_layout.rs:2553): same reasoning, `needed` is computed from an
  `intact` design that already passed `check` via `generate_ship`.

No other caller exists (grepped `wreck_floor(` across `crates/`). The
removed `.clamp(1, target)` is dead weight now, not a latent bug: nothing
observes a wreck-side 0 floor since `check` intercepts it first, one layer
up, on the intact design.

## (3) No stale "never below 1" / "Unruined (0 of 1)" claims left in shipped docs

CONFIRMED CLEAN in code/doc comments and `docs/architecture.md`. Grepped the
whole tree for "never below 1", "0 of 1", "cannot lose even one",
"cannot lose one off-centre", "fails as unruined": the only source-adjacent
hit is `nova_interface/src/terminal/crt.rs:378`, an unrelated PID-controller
comment ("a derivative never below 1"), not ship layout.

`docs/architecture.md`'s `nova_world_base` row was updated in this diff to
read "...capped at that target. Every generated hull has at least one. A
wreck whose plans all fall short of that floor fails with `Unruined`;
nothing rerolls or skips it," replacing the old "...and never below 1. A
hull that cannot lose one such cube fails with `Unruined` (0 of 1)...".
Matches the new behavior: `Unruinable` (not `Unruined`) is now the failure
for a hull with zero removable cubes, and it fails the *intact* layout
attempt (rerolled up to 8 times), not the wreck request.

`ship_layout.rs`'s module doc (top of file) and `wreck_floor`'s doc comment
were both updated consistently (diff hunks at lines ~25-53 and ~555-561).

MINOR, CONFIRMED, informational only (not a doc defect):
`tasks/20260923-110307/TASK.md:67` still describes the *old*, pre-this-step
behavior ("`generate_wreck` fails loudly with `Unruined` (0 of 1)...") as a
"PR A limitation." This is a historical task note recording a decision
point, not a durable doc under AGENTS.md's changelog/docs policy, and
`tasks/20260923-110307/UNRUINED-CAUSE.md`'s "Outcome" section (same folder,
written by the author) already documents that this limitation was resolved
by `Unruinable`. Flagging only so the TASK.md line isn't misread later as
still-current behavior; not scored as a defect.

## (4) Updated tests assert something real, no fallback/quiet-accept of a loud failure

CONFIRMED CLEAN.
- `a_layout_with_no_removable_outer_cube_is_refused_as_unruinable`
  (ship_layout.rs:2599-2633, replaces
  `a_hull_with_no_removable_outer_cube_fails_as_unruined`): finds the
  specific attempt index that `check` refuses as `Unruinable` via a direct
  `Draw::new(...).layout().and_then(check)` call (no reliance on
  `generate_ship`'s internal retry alone), then asserts `generate_ship`
  still succeeds on a *later* attempt (`intact.attempt > refused`) and that
  `generate_wreck` then succeeds. This is a strong, concrete assertion: it
  proves both that the rejection fires and that the reroll recovers, rather
  than accepting any `Err`/`Ok` outcome.
- `every_base_wreck_at_advancement_zero_breaks_its_mirror` (renamed from
  `a_base_wreck_at_advancement_zero_breaks_its_mirror_or_fails_as_unruinable`,
  generated_wrecks.rs:9-39): dropped the `Ok(..) => .. / Err(..) => ..`
  branch that used to tolerate a failure; now `generate_wreck` is
  unconditionally `.unwrap_or_else(|failure| panic!(...))` for every
  seed/role at advancement 0 on the real base content catalog. This is a
  strictly stronger assertion than before, not a weaker one.
- `a_wreck_keeps_its_fittings_size_and_connection_and_breaks_its_mirror`
  (ship_layout.rs:2474-2597): dropped the advancement==0.0 "thin hull may
  fail as Unruined (0,1)" escape hatch; `generate_wreck` is now
  unconditionally unwrapped. Its `needed` computation was also fixed to
  subtract `dock_backing` before calling `wreck_floor`
  (ship_layout.rs:2546-2553), matching what `check`/`generate_wreck`
  actually do in production (previously the test computed the floor over
  *all* cubes including dock backing, a real test/production mismatch per
  the author's own `UNRUINED-CAUSE.md` notes, lines 91-95 of that file).
  This is a bug fix in the test, not a weakened assertion.

None of the three tests added a `Result`-swallowing branch, a default, or an
`if let Ok` that silently skips the failure case.

## (5) `ClusterBody::Hull` vs `Ship` matches: `is_hull()` usage complete within scope

CONFIRMED CLEAN, within `crates/nova_world_base/src/clusters.rs` (the file
named in the change description). Grepped every `ClusterBody::` reference:
- `resolve()`'s companion filter (clusters.rs:1386) and the companion test
  helper (clusters.rs:1825) both now use `!other.body.is_hull()` /
  `!other.body.is_hull()`, replacing the old
  `!matches!(other.body, ClusterBody::Hull { .. })` that missed `Ship`.
- Every remaining `ClusterBody::Hull` reference in this file is a
  construction (`hull()`, a test fixture) or a destructure
  (`plan_sector`'s per-hull loop, which converts it to `ClusterBody::Ship`
  immediately), or the `unreachable!()` arm in the manifest match
  (`ClusterBody::Hull { .. } => unreachable!(...)`) - none of these are
  membership tests that need `is_hull()`.
- No other `matches!(_, ClusterBody::Hull` or direct `ClusterBody::Hull`
  equality check remains in clusters.rs.

Out of scope, not reviewed as part of this gate (not named in the change
description, and part of the larger uncommitted regional-ship-runtime work):
`examples/shared/world_fixture/clustered.rs` defines its own, separate
`ClusterBody` enum (its own `enum ClusterBody` at clustered.rs:238, not a
reuse of the production one) and does not have an equivalent
`is_hull()`/companion-filter construct at all in the diff I read, so (5)'s
"anything else" check does not extend to it under this review's boundary.
Noting this only as a skipped-check boundary, not a finding.

## Skipped checks / uncertainties

- Did not run `cargo check`/`cargo test` (not required; correctness was
  verified by direct code reading plus the author's own recorded mutation
  test and multi-seed sector sweep in `UNRUINED-CAUSE.md`'s "Outcome"
  section: 343 cells x 5 seeds, 0 sector faults where seed 20260922 had 2
  before; `nova_world_base` lib 51/51; `world_clusters` example 15/15;
  `generated_wrecks` 1/1; and a disable-the-check mutation test that fails
  the new unit test as expected).
- Did not independently re-run or re-derive the author's mutation/sweep
  numbers; treated them as reported, since they are concrete and specific
  (counts, seeds, test names) rather than prose claims.
- Did not review `examples/shared/world_fixture/clustered.rs`,
  `crates/nova_world_base/src/sector_ships.rs`, or the rest of the
  uncommitted `regional-ship-runtime` diff (civilization selection, pending
  ship streaming, `clearance`/`centred_clearance` rework, etc.) - out of
  this change's stated scope; a prior review
  (`tasks/20260923-110307/REVIEW-91.md`) already covers that broader slice
  and explicitly deferred `ship_layout.rs` generation internals to "the PR A
  gate," i.e., this review.
- Did not run a game/GPU capture or any workspace-wide test/Clippy pass, per
  instructions.
- Performance: `check`'s new `wreck_floor(&cells, &loose, 1)` call adds one
  more O(outer cubes) scan (each cube test is an O(sections) `joined` BFS)
  per layout attempt, on top of the existing `connected_and_clear` and
  `derive_link_point_graph` work in the same attempt. Because `target = 1`
  here, the scan breaks after the first qualifying cube in the common
  (accepted) case, so the added cost is negligible for ships that pass; only
  the rare `Unruinable`-rejected attempt scans every outer cube, and that
  attempt is discarded and retried anyway. Not flagged as a finding - judged
  negligible relative to existing per-attempt work, consistent with world
  generation being an off-frame, not per-frame, cost.
