# Audit the codebase for duplicated algorithms and logic

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog, refactor, quality

Audit the workspace for duplicated algorithms and logic that must be changed
in more than one place. Identify and record first; change nothing yet.

## Scope

- In scope: `crates/*/src/**`, the real codebase.
- Out of scope: `examples/`, `crates/*/examples/`, `benches/`, fixtures, and
  test scaffolding. Duplication there is acceptable by design.
- Target: the same algorithm or complex decision implemented twice. Not one
  or two repeated lines, not boilerplate a derive already settles.

## Question the audit answers

If I change this behavior in one place, must I change the same behavior
somewhere else for the game to stay consistent?

Priority order:
1. UI interaction logic. One example: does closing a window with `Esc` run
   the same code as closing it with a click, or two separate paths?
2. Two features that are supposed to behave the same, implemented apart.
3. Shared utilities re-written per call site: file and RON reading, id
   lookup, unit conversion, formatting, timers, spatial queries, math.

## Method

Four reviewers in two cross-checking pairs, two waves. Each pair reads the
same chunk by two different methods, structural and behavioral, so that each
finding is either confirmed or contradicted by a second reader.

## Deliverable

`FINDINGS.md` in this folder: each duplicate family with `path:line` for every
copy, a short excerpt, what breaks if only one copy changes, and a proposed
single home. No code change until the owner selects findings.

## Step 1 complete: findings recorded

`REPORT.md` is the summary. `FINDINGS.md` holds every family with `path:line`
evidence, excerpts, proposed homes, refutations and coverage.

Ten reviewer-passes in five pairs over `crates/*/src/**`. Counts: 4 BLOCKER,
24 MAJOR, 24 MINOR, 16 families confirmed NOT duplicated, 11 claims rejected.

BLOCKERs, each already inconsistent in the tree:

1. `rebind-policy-four-surfaces` - four rebind surfaces, four conflict
   policies. A console `bind` is persisted then silently reverted at next
   launch; the in-ship panel drops a section's gamepad binding while the
   editor asserts the opposite.
2. `scroll-driver-fork` - `nova_os_ui` re-implements the `nova_ui` scroll stack
   it imports; wheel step 20.0 against the shared 60.0.
3. `terminal-command-error-vocabulary` - the same typo in the same CRT gets two
   different answers depending on which shell is open.
4. `scenario-name-vocabulary-two-tables` - lint match and editor reflect
   attributes are two tables for one question; a dangling id in
   `SetInfiniteAmmo` passes lint and dies silently at runtime.

Two findings were corrected DOWNWARD after the owner challenged them, and are
recorded as such: the weapon-class fire sequence (shared code is two statements
calling already-shared helpers; the "railgun drift" claim is withdrawn, the
railgun renders as a streak and needs no easing seed) and the menu panel close
(four ~13-line handlers; the missing Escape is a UX gap, not duplication).

Nothing was changed. Next step is the owner's selection of which families to
fix; each has a proposed home and signature in `FINDINGS.md`.

## Step 2 complete: all 22 selected families fixed

On the `dedup-fixes` sprout, two commits. Each family got ONE home; every
deliberate difference was kept as a named parameter or variant carrying its
reason, never flattened.

Commit 1, eight families: `scroll-driver-fork` (2),
`terminal-command-error-vocabulary` (3), `turret-reach-formula` (37),
`torpedo-launch-commit-three-paths` (26), `sfx-juice-throttle` (23),
`hud-anchored-chip-module-cloned` (12), `probe-check-capability-gate` (20),
`portal-store-bypasses-storage` (35).

Commit 2, fourteen families: `rebind-policy-four-surfaces` (1),
`section-aabb-overlap-rule` (6), `enabled-bundle-set-three-ways` (7),
`scenario-id-to-entity` (8), `render-target-image-recipe` (9),
`novaos-viewer-twin` (10), `verb-grant-predicate` (11),
`section-source-resolution` (13), `authored-object-walk` (14),
`mod-ref-scope-build` (16), `deterministic-hash-hand-rolled` (18),
`scenario-name-vocabulary-two-tables` (29),
`object-reference-prefix-guard` (30), `subtree-collider-aabb-walk` (33).

New shared homes: `nova_gameplay::{bounds, hash, render_target}`,
`nova_os_ui::viewer`, `nova_ship::flight::capability`,
`nova_hud::anchored_chip`, `nova_input`'s `RebindSurface`/`RebindVerdict`,
`nova_os::commands::CommandError`, `nova_scenario::names::walk_names`,
`nova_gameplay::juice::CueThrottle`.

### Findings corrected while implementing

- #24 withdrew its claimed `base_velocity` site in `nova_authoring`; that span
  is a `SectionSource` match. Real count: 10 production sites.
- #20's premise was wrong. The five probe capability tables are NOT identical:
  `capture_simulated` defers to `fps_within_baseline` on purpose. Preserved as
  `SilentGap::Defers` rather than flattened.
- #13 overcounted at 6 sites; 5 are real. `nova_wfc/src/check.rs:50-62` is a
  rejection, not a resolution, and was left alone.
- #8's stated mechanism was wrong. Severing does NOT make each section a free
  body: `integrity.rs:444-472` spawns one dynamic wreck per component and
  re-parents its sections.
- #18's two spark sites disagreed on which hash bits aim them. `damage_sparks`
  drew part of its height from FNV-1a's least-mixed low byte; the shared rule
  takes the well-behaved slice, so a damaged section's spark aim moves.

### Verification

`cargo check --workspace --all-targets --features debug` clean, no warnings.
`cargo test --workspace --features debug` 3759 passed, 0 failed.
`content gen` produces no diff under `assets/base/**`.
`content lint` 0 errors, 0 warnings, 0 findings.
`cargo fmt --all --check` clean.

Not run: Clippy, and any live/rendered play. Behavior-visible changes are the
spark aim above, the NOVA OS drawer wheel step, and the four rebind surfaces
now sharing one conflict policy.

### Deliberately NOT fixed, for the owner to decide

The `"normal"` hanabi property at 4 sites; `item_highlights.rs` and
`allegiance_markers.rs` as 3rd/4th copies of the chip lifecycle;
`flight/order.rs:427-433`'s `can_turn` omitting the PD requirement;
`EventActionConfig::collect_injections` as a 7th traversal copy;
`RefillAmmoActionConfig.section` missing `#[reflect(@Names::Section)]`;
`nova-os-palette-second-copy`'s 4 divergent tokens;
`nova_editor/src/skin.rs:273` as a 4th `DefaultHasher` fingerprint.
