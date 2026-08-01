# NOTES - KISS: small crates and root binary

## Inventory (base, `wc -l`)

| File | Lines |
| --- | --- |
| crates/nova_modding/src/lib.rs | 446 |
| crates/nova_core/src/lib.rs | 337 |
| crates/nova_mod_format/src/lib.rs | 314 |
| crates/nova_core/src/loading_screen.rs | 285 |
| crates/nova_events/src/lib.rs | 238 |
| crates/nova_mod_format/src/deps.rs | 231 |
| crates/nova_core/tests/cubemap_meta_app_config.rs | 108 |
| src/main.rs | 37 |
| crates/nova_info/build.rs | 20 |
| crates/nova_info/src/lib.rs | 19 |
| src/lib.rs | 1 |
| **total** | **2036** |

The task header quoted "largest file: lib.rs at 622 lines". No file in scope
is anywhere near that; the real maximum is 446. The structure axis of the epic
rubric is therefore a NO-OP here - nothing exceeds 1500, and every file holds
one concern:

- `nova_core/src/lib.rs` - the `AppBuilder` composition root plus the three
  plugin-config helpers it feeds (`window_plugin`/`log_plugin`/`assets_plugin`).
  One concern; splitting the helpers out would be line-count-driven, which the
  epic rubric explicitly forbids.
- `nova_modding/src/lib.rs` - the three bevy `AssetLoader`s and their asset
  types. One concern (the engine-facing loader layer); the engine-free serde
  half is already a separate crate.
- `nova_mod_format` - already split into `lib.rs` (wire types) + `deps.rs`
  (resolution).
- `nova_core/src/loading_screen.rs`, `nova_events/src/lib.rs` - single concern.

So this pass is comments only.

## Proof that no behavior changed

Comment-stripped line multiset over every touched `.rs` file, base vs branch
(blank lines and any line whose first non-space chars are `//` dropped, then
counted as a multiset):

```
BASE-ONLY:
 - let g = graph(&[]); // no edges      1
NEW-ONLY:
 + let g = graph(&[]);                  1
```

The single residue is the removal of a TRAILING comment (a comment itself, so
invisible to a line-leading strip). No statement, literal, signature, import,
`mod` line or visibility keyword changed anywhere in scope. This is a stronger
result than the sibling tasks got: there is no move component at all.

## Comment rubric application

Deleted outright (narration or provenance, nothing load-bearing):

- `nova_core/src/lib.rs` - the 16-line commented-out `new_headless_app()`
  corpse; the loading-screen, editor-default and main-menu-default narrations
  in `build()` (each restated the line under it).
- `loading_screen.rs` - four test section markers whose assert messages already
  say the same thing.
- `deps.rs` - the `indegree`/`dependents_of` declaration narration, the
  "anything left is in a cycle" narration, and six test comments that restated
  the `graph(&[...])` literal or the assert message directly below.
- `src/main.rs` - a comment about `editor_app` sitting above the `render` cfg
  block, duplicating `editor_app`'s own rustdoc.
- `nova_mod_format/src/lib.rs` - two test section markers.

Promoted to `NOTE:` (constraint still binds):

| Where | Constraint |
| --- | --- |
| `nova_core/src/lib.rs:86` | `mods://` must register BEFORE `AssetPlugin` lands |
| `nova_core/src/lib.rs` (already NOTE) | do not re-add `UiWidgetsPlugins` |
| `nova_core/src/lib.rs` handoff | only advance out of `Loading` (BCS_SHOT race) |
| `nova_core/src/lib.rs` window | canvas selector; wasm key capture |
| `nova_core/src/lib.rs` status bar | bar is deliberately NOT `HudNovaOsExempt` |
| `nova_modding/src/lib.rs` | why the format types live in `nova_mod_format` |
| `nova_modding/src/lib.rs` | content paths are bundle-DIR-relative |
| `nova_modding/src/lib.rs` | the owned `to_string` is load-bearing (lifetime) |
| `nova_modding/src/lib.rs` | `resource_base` must follow the content map |
| `nova_modding/src/lib.rs` | catalog bundle paths are asset-root-relative |
| `deps.rs` | seeding `seen` with `id` is what kills the self-edge |
| `deps.rs` | input-order re-scan is what makes the tiebreak stable |
| `deps.rs` (x3, tests) | why three non-obvious expected values hold |
| `cubemap_meta_app_config.rs` | cwd/asset-root; the deadline bounds a hang only |

Rustdoc kept everywhere; the four blocks that carried task HUIDs
(`assets_plugin`, the `NAMING:` guards on both loaders, the cubemap test's
module doc) were compacted so the surviving guard reads without the history.
Two rustdoc fixes: `nova_events`'s module doc omitted `OnNeutralizedEvent`, and
`nova_info`'s carried a stale "this crate is the `missing_docs` exemplar"
paragraph from a 2026-05 rollout that has since finished.

## DoD 3 - HUID grep

```
grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_core/ crates/nova_events/ \
  crates/nova_info/ crates/nova_modding/ crates/nova_mod_format/ src/
```

Returns NOTHING. No deliberate HUID reference survives in scope, so there is no
exception list to justify here. This matches the crates the epic's earlier
children already landed (`nova_ui`, `nova_os`, `nova_probe`, `nova_scenario`
all return zero hits on the same grep).

## DoD 4 - file sizes

Largest file in scope after the pass: `nova_modding/src/lib.rs` at 439 lines.
No exception needed.

## Defects uncovered

One, already tracked: `cargo doc`/`cargo check` emit four
`ambiguous import visibility` warnings from `nova_gameplay`'s `hud/nova_os_map`
and `hud/nova_os_ship` modules - fallout from an earlier child's split, filed
as 20260801-005057. Nothing new was found in this scope.

## Verification

| Proof | Result |
| --- | --- |
| `cargo check --workspace --all-targets` | green (only the pre-existing `nova_gameplay` warnings) |
| `cargo fmt --check` | clean |
| `cargo test -p nova_core -p nova_events -p nova_info -p nova_modding -p nova_mod_format --lib` | 12 passed, 0 failed |
| `cargo test -p nova_core --test cubemap_meta_app_config` | 3 passed |
| `cargo doc --no-deps` over the five crates | no warning from any crate in scope |

Every test in scope ran to completion here - this scope is small enough to fit
this box's RAM, unlike the earlier children (20260731-210651).
