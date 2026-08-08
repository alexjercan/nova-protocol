# KISS: nova_gameplay sections and integrity

- STATUS: CLOSED
- PRIORITY: 39
- TAGS: v0.9.0, refactor, chore, gameplay

## Story

As a maintainer I want ship sections and structural integrity to fit an agent context and carry only comments
that earn their place, so future work in this area starts from a smaller,
quieter surface.

Rubrics (comment + structure) live in the parent epic. Read the epic index
before starting; do not restate the rubric here.

Scope: crates/nova_gameplay/src/sections/ crates/nova_gameplay/src/integrity/
Current size: ~11.5k lines. Largest file: turret_section.rs at 3668 lines.

## Steps

- [x] Read the parent epic's comment and structure rubrics.
- [x] Inventory: per-file line counts and the concerns each file holds.
- [x] Split files that hold more than one concern; keep public paths and
      prelude exports stable.
- [x] Apply the comment rubric file by file: delete narration and provenance
      clauses, promote surviving constraints to NOTE/TODO/FIXME/BUG, keep
      rustdoc.
- [x] Open backlog tasks for any defect the pass uncovers; do not fix here.
      (None uncovered - the pass moved code and deleted comments only.)
- [x] Verify: check, fmt, and the existing tests for this area.

## Definition of Done

1. cmd: `nix develop --command cargo check --workspace --all-targets` - green.
2. cmd: `nix develop --command cargo fmt --check` - clean.
3. cmd: `grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_gameplay/src/sections/ crates/nova_gameplay/src/integrity/` - every hit is a
   deliberate NOTE/TODO/FIXME/BUG reference, listed in NOTES.md.
4. cmd: `wc -l crates/nova_gameplay/src/sections/ crates/nova_gameplay/src/integrity/` - no file over 1500 lines, or NOTES.md justifies the
   exception as one cohesive concern.
5. test: existing tests covering this area still pass.
6. manual: owner skims the diff and agrees no behavior changed.

## Notes

Moves, renames, deletions only. No new abstractions, no behavior change.

No NOTES.md: DoD 3 and DoD 4 both come back empty, so there is nothing to
justify - zero HUID comment references survive and the largest file is 1377
lines.

## Close-out

### What and why

Two oversized files held every concern of their section, and 98 comment lines
carried a task HUID as provenance rather than as a live reference.

Structure. `turret_section.rs` (3668) became a folder module of six concern
files plus a shared test rig; `torpedo_section/mod.rs` (1820) shed its launch
path into `bay.rs` and handed its in-flight tests to the `projectile`
submodule that owns those systems.

| New file | Holds |
| --- | --- |
| `turret_section/mod.rs` | prelude, bundle fn, components, plugin |
| `turret_section/config.rs` | joint tree + section config, serde tests |
| `turret_section/setup.rs` | spawn the joint tree, push live config edits |
| `turret_section/aim.rs` | aim system set, lead intercept, hinge CCD pass |
| `turret_section/firing.rs` | fixed-clock muzzle loop, bullet contact rule |
| `turret_section/render.rs` | joint/projectile meshes, muzzle + trail effects |
| `turret_section/test_support.rs` | the two rig helpers two files both need |
| `torpedo_section/bay.rs` | build the bay, tick its timer, launch, shoot-down |

Comments. Every provenance clause deleted, the surviving constraint promoted
to `NOTE:` where one existed (10 sites, `grep -rcE '//.*NOTE:' <scope>`),
rustdoc kept and re-wrapped by hand. Five section-separator comments that named
only a task ID were deleted outright (`comm -23` over the base/new comment
multisets). `turret_section/render.rs` also carried ~20 lines of narration
copied from a bevy_hanabi example ("the ball center", "the black background
box") plus one commented-out line from it; that went too.

### Alternatives

Splitting the turret tests into a single `tests.rs` was rejected: it would
still have been ~2050 lines, breaking DoD 4 on its own, and it separates every
test from the code it covers. Moving `muzzle_entity` / `joint_entities` into a
`test_support.rs` (the shape `integrity/` already uses) was cheaper than
duplicating them into the two files that need each.

### Difficulties and diagnosis

The `pub(super)` sweep is the one thing the compiler only half-proves - too
narrow fails to build, too wide compiles forever (lesson
`visibility-sweep-narrows-back`). It was driven off the actual E0603/E0425
list this time, not a blanket regex: 17 items widened (12 turret systems, 5
torpedo bay systems, plus `default_joint_speed`, `muzzle_entity` and
`joint_entities` for the cross-file test rigs - the enumerated list, matching
`grep -rc 'pub(super) fn'` at 18 in the new files against 16 pre-existing at
base). Each has a reference outside its defining file.

`update_turret_aim_point` is referenced by `hud/turret_lead.rs` through the
module path, so `mod.rs` re-exports it `pub(crate)` rather than leaving it
behind the private submodule - the `split-must-re-export-not-repoint` rule.

### Evidence

Commands re-run after the LAST edit of the round:

- `cargo check --workspace --all-targets` - green, no warnings in scope.
- `cargo fmt --check` - clean.
- `grep -rnE '//.*[0-9]{8}-[0-9]{6}' <scope>` - 98 hits before, 0 after.
- `wc -l` over the scope - largest file 1377 (`turret_section/firing.rs`),
  was 3668; 24 files, 11527 lines total.
- `cargo test -p nova_gameplay --lib sections::` - 100 passed, 0 failed.
- `cargo test -p nova_gameplay --lib integrity::` - 21 passed, 0 failed.
- Conservation: `#[test]` count 35 -> 35 (turret), 32 -> 32 (torpedo); the
  trimmed non-blank line multiset differs only by module scaffolding,
  visibility prefixes and the comment edits.
- Rustdoc-damage scan (the `doc-comment-rewrap-changes-the-render` lesson): no
  odd-backtick block and no new block construct after prose; the four list
  starts the scan reports all exist identically at base.
- `cargo doc -p nova_gameplay --no-deps` - zero warnings under `sections/` or
  `integrity/`, so no intra-doc link broke when items moved between modules.

### Reflection

The line-range concatenation method carried over from the two sibling tasks
unchanged and again reduced "did behavior change" to reading the import and
visibility diff. Doing the comment pass as explicit, asserted single-occurrence
replacements (each one failing loudly if its anchor moved) rather than a
regex sweep is what kept the rewrap-damage scan clean on the first run.
