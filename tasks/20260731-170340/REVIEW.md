# Review: KISS: nova_gameplay input layer - player, AI, targeting

- TASK: 20260731-170340
- BRANCH: refactor/kiss-gameplay-input

## Round 1

- REVIEWER: out-of-context
- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_gameplay/src/input/targeting/mod.rs:35 - the
  three new `mod.rs` files declare `pub mod <concern>;` but never re-export
  the moved items, so previously-reachable paths no longer resolve:
  `input::targeting::{RadarState, CombatLock, ComponentLock, ...}`,
  `input::player::{binding_label, FlightVerbHints, ...}` and
  `input::ai::{AITarget, AIThreat, ...}` now live only at
  `input::<mod>::<file>::Name`. The epic's structure rubric requires "a folder
  module with `mod.rs` re-exporting. Public paths must not change", and the
  landed sibling split does exactly that (`hud/nova_os_ship/mod.rs:50` -
  `pub(crate) use self::{app::*, scene::*, sections::*};`). Concrete
  breakage: `cargo doc -p nova_gameplay --no-deps` emits `unresolved link to
  'super::player::binding_label'` at `input/reference.rs:6`, which resolved on
  master. Fix: add `pub use self::{...}::*;` in each of `ai/mod.rs`,
  `player/mod.rs` and `targeting/mod.rs` restoring the previously-visible
  names at the parent path, then revert the three repointed call sites
  (`hud/key_glyphs.rs:275,287,291` and the two
  `crate::input::targeting::safety::` / `player::flight_rig::` test paths).
  - Response: fixed in the round-2 commit. Each `mod.rs` now re-exports every
    name its pre-split file made visible at the parent path - `pub use
    self::{...}` for the public API plus `pub(crate) use self::{...}` for the
    input-action types, `on_component_cycle_next`, `FlightInputMarker`,
    `flight_input_rig`, `keyboard_label` and
    `update_weapons_safety_for_tests` (the last three `#[cfg(test)]`, matching
    their only consumers). All three repointed call sites are reverted to
    their master spelling. Explicit name lists rather than the precedent's
    glob, because `pub(crate) use self::x::*;` is what produces the
    `ambiguous import visibility` warnings `nova_os_*` already carries.
    `cargo doc` now resolves `super::player::binding_label`.
- [x] R1.2 (MAJOR) tasks/20260731-170340/NOTES.md:24 - "Public paths are
  unchanged" and the close-out's "the crate's external API is byte-identical"
  are both false given R1.1: the prelude paths survived, the direct module
  paths did not, three out-of-module call sites had to be repointed, and one
  rustdoc link was left broken. Fix: land R1.1 and keep the sentence, or
  restate it honestly and record the new `cargo doc` warning.
  - Response: fixed in the round-2 commit. R1.1 landed, so the claim is now
    true and stated precisely: NOTES' structure section names the re-export
    mechanism, and the evidence list records the one remaining unresolved
    intra-doc link (`nova_assets` in the untouched `hud/key_glyphs.rs`).
- [x] R1.3 (MINOR) crates/nova_gameplay/src/input/player/mod.rs:59 - the
  `SpaceshipPlayerInputPlugin` rustdoc first line is duplicated verbatim on
  lines 59 and 60; master has it once. Delete the duplicate line.
  - Response: fixed in the round-2 commit - duplicate line deleted.
- [x] R1.4 (MINOR) crates/nova_gameplay/src/input/player/mod.rs:104 -
  stripper damage that survived the repair pass: master reads "(previously a
  `.chain()` when they shared a module)", the branch reads "(previously
  a.chain when they shared a module)". Restore the `.chain()` spelling.
  - Response: fixed in the round-2 commit - restored to "(previously a
    `.chain()` when they shared a module)".
- [x] R1.5 (MINOR) crates/nova_gameplay/src/input/ai/torpedo.rs:181 - rustdoc
  points at pre-split file names this commit deleted: here `(input/player.rs)`
  and `player/flight_rig.rs:96` `(input/targeting.rs dispatch)`. Eight more
  stale pointers now exist outside `input/` and outside the exempt `tasks/`
  tree: `hud/component_lock.rs:4`, `hud/lock_dwell_ring.rs:3`,
  `hud/target_inset.rs:8`, `audio.rs:1739`, `targeting/state.rs:174`,
  `nova_scenario/src/loader.rs:679,948`,
  `nova_scenario/src/objects/beacon.rs:87`,
  `nova_assets/src/balance.rs:47,51`, and `web/src/wiki/dev/project-tour.md`
  names a file that no longer exists. Repoint them or record a follow-up task.
  - Response: fixed in the round-2 commit - all repointed to the new module,
    including three the round did not list (`flight.rs:34,1210,2263`) and the
    wiki's project-tour table. The `input/{player,targeting,ai}.rs` grep over
    `crates/` and `web/src` now returns nothing; NOTES has a Doc-surface
    sweep section listing every site.
- [x] R1.6 (MINOR) crates/nova_gameplay/src/input/reference.rs:9 - the epic's
  "deferred work: keep as TODO/FIXME with the tatr ID if one exists" row was
  applied backwards: "Full remapping + key icons stay backlog (task
  20260710-231927)" lost its ID, leaving unactionable prose. Restore it as a
  `TODO:` carrying the ID, or drop the sentence.
  - Response: fixed in the round-2 commit - `reference.rs:10` now reads "full
    remapping + key icons stay backlog (TODO: 20260710-231927)". It is the
    only DoD 3 grep hit and NOTES lists it in the required table.
- [x] R1.7 (MINOR) tasks/20260731-170340/NOTES.md:110 - "Eleven `// -- section
  --` separators were deleted" is off by one: master's three files carry
  twelve (`grep -nE '^\s*//\s*-{2,}'` returns 4 + 1 + 7) and zero remain.
  Correct the number.
  - Response: fixed in the round-2 commit - NOTES now says twelve and shows
    the 4 + 1 + 7 breakdown.

Verified in-session (re-derived independently of the out-of-context round):

- `cargo check --workspace --all-targets` green; the two `nova_os_*`
  ambiguous-import warnings are pre-existing in untouched `hud/`.
- `cargo fmt --check` clean.
- `cargo test -p nova_gameplay --lib input::` - 180 passed, 0 failed, matching
  the recorded number. `#[test]` count conserved: 97 + 28 + 54 = 179 before,
  179 after, plus `reference.rs`'s 1.
- Every `Plugin::build` body is multiset-identical to master, so registration
  order, observer wiring and set membership are unchanged.
- Whole-area non-comment line multiset vs master: every difference is a
  visibility widening, a moved import, or a module-path repoint. Nothing
  became `pub`.
- DoD 3 grep returns zero hits; DoD 4 max file is 1076 lines; NOTES' per-file
  table matches `wc -l`; the eleven `NOTE:` markers sit where NOTES says.
- R1.3, R1.4, R1.6 and R1.7 re-derived directly from the working tree; the
  R1.1 precedent confirmed by reading `hud/nova_os_ship/mod.rs:49-50`.

Process signal: the mechanical comment stripper produced three classes of
silent text damage (`a.chain`, a duplicated doc line, a dropped backlog ID)
that a lint sweep over the RESULT did not catch. The check that would have
caught them is a comment-text diff base-vs-branch, not a lint.

Pending user checks:

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed.

## Round 2

- REVIEWER: out-of-context
- VERDICT: APPROVE

All seven round-1 findings are confirmed fixed and ticked above. The round-2
reader re-derived R1.1 independently: master's `pub`/`pub(crate)` item lists
for the three pre-split files were diffed item by item against the new
re-export lists, the preludes hash-match master, and all three previously
repointed call sites are back to their master spelling. Three record
inaccuracies were found, fixed and re-confirmed by the same reader:

- [x] R2.1 (MINOR) tasks/20260731-170340/NOTES.md:48 - the round-2 commit
  changed five files, so five rows of the "After" line-count table went stale
  (`ai/mod.rs` 159 -> 164, `targeting/mod.rs` 127 -> 140, `player/mod.rs` 121
  -> 127, `player/hints.rs` 650 -> 646, `player/flight_rig.rs` 1002 -> 1001),
  as did the line-68 prose. Re-measure and update.
  - Response: fixed. All 23 rows re-measured against `wc -l`; the reviewer
    re-checked every row, not just the five, and confirms Prod + Tests sums
    to Lines on each.
- [x] R2.2 (MINOR) tasks/20260731-170340/TASK.md:58 - R1.7's correction landed
  in NOTES only; the close-out still said "eleven stale section separators".
  Change to twelve.
  - Response: fixed. Close-out now reads twelve; "eleven constraint comments
    promoted to `NOTE:`" is unchanged and was re-counted as correct.
- [x] R2.3 (NIT) crates/nova_gameplay/src/input/player/mod.rs:44 - NOTES
  claimed each `mod.rs` re-exports "every name the pre-split file made visible
  at the parent path", but `keyboard_label` and `flight_input_rig` were
  unconditionally `pub(crate)` on master and are now `#[cfg(test)]`-gated. A
  narrowing, not a break (both callers are test-only). Qualify the sentence or
  drop the gate.
  - Response: fixed by qualifying NOTES - the gate stays, because an ungated
    re-export of a test-only helper is an unused-import warning in the lib
    target. The sentence now names all three gated helpers.

Verified in-session:

- `cargo check --workspace --all-targets` green; the same two `nova_os_*`
  ambiguous-import warnings, pre-existing in untouched `hud/`.
- `cargo fmt --check` clean; `cargo test -p nova_gameplay --lib input::` 180
  passed, 0 failed.
- R1.1 re-derived a third way: a throwaway `#[cfg(test)]` module importing
  `input::ai::{AIBehaviorState, AIFireCadence, AITarget, AIThreat,
  AITorpedoBay}`, `input::player::{binding_label, flight_rig_reserved_sources,
  FlightVerbHints, VerbHint}` and `input::targeting::{CombatLock,
  ComponentLock, RadarState, TravelLock, COMBAT_DECAY_SECS}` compiles clean;
  removed after the run.
- Every row of NOTES' line table re-checked against `wc -l` in-session before
  the round was written.
- `cargo doc -p nova_gameplay --no-deps` leaves one unresolved intra-doc link
  (`nova_assets`, in untouched `hud/key_glyphs.rs`); the
  `super::player::binding_label` link round 1 broke now resolves.

Pending user checks:

- DoD 6 (`manual:`) - owner skims the diff and agrees no behavior changed.
