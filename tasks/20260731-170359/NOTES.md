# NOTES - 20260731-170359

KISS pass on `crates/nova_menu`. Both axes: `lib.rs` (7705 lines, one file)
became a plugin shell plus nine concern modules and a nine-file test tree, and
the comment rubric ran over every file.

## Structure axis

`lib.rs` held the whole crate: the plugin wiring, the pause overlay, the two
in-play modal overlays, the settings body, the mods screen's two tabs, the
scenarios picker, the main-menu panel, the backdrop staging, the shared button
helpers, and 3490 lines of tests. Every one of those is a separate concern with
its own entry point, so the split is by cohesion, not by line count.

`lib.rs` now holds only the crate doc, the prelude, the module declarations and
`impl Plugin for NovaMenuPlugin` - the wiring index. Its `build` body is
byte-identical to the base except for the comments.

| File | Lines | Concern |
|-|-|-|
| mods.rs | 873 | Mods screen, Installed tab: panel, list, details, toggles |
| menu_ui.rs | 637 | the main-menu panel + the `OnEnter(Playing)` scenario start |
| scenarios.rs | 632 | Scenarios picker: rows, campaign headers, details, Play |
| portal.rs | 562 | Mods screen, Explore tab: remote catalog + action buttons |
| outcome.rs | 414 | the win/lose frame and the FAILED TO START report |
| pause.rs | 364 | the pause overlay, its clocks and its cursor |
| settings.rs | 341 | the shared Settings body and its persistence systems |
| settings_store.rs | 331 | (unchanged) cross-platform settings persistence |
| lib.rs | 219 | crate doc, prelude, module list, plugin wiring |
| ambience.rs | 172 | backdrop pick, camera staging, HUD chrome |
| widgets.rs | 93 | `button`/`button_variant`, the click cue, list scrolling |

Tests moved with the code they cover, as `src/tests/<concern>.rs` under a
`#[cfg(test)] mod tests`. Fixtures shared by more than one concern went to
`tests/support.rs`; fixtures used by one stayed local to it.

| Test file | Lines |
|-|-|
| tests/portal.rs | 727 |
| tests/outcome.rs | 571 |
| tests/scenarios.rs | 536 |
| tests/mods.rs | 461 |
| tests/pause.rs | 333 |
| tests/support.rs | 320 |
| tests/menu.rs | 226 |
| tests/settings.rs | 228 |
| tests/ambience.rs | 193 |

Largest file in the crate: 873 lines. DoD 4 passes with no exception needed.

### What the split did NOT do

No new abstractions and no renames. Every moved item became `pub(crate)` so
sibling modules can reach it; nothing became `pub`, so the crate's public
surface is unchanged (`NovaMenuPlugin` and `prelude` only).

`mods.rs` and `portal.rs` reference each other: the Installed-tab list dispatches
to the Explore renderer on a tab switch, and Explore rows reuse the installed
row's select observer and version/author line. That is the shape the code
already had; breaking it would need a new abstraction, which is out of scope.

## Comment axis

Provenance and narration went; guards stayed. Following the precedent the
landed siblings set (20260731-170322, -170329), a surviving comment that
explains a non-obvious constraint stays a plain `//` - only a comment guarding
a specific literal value was promoted to `NOTE:`.

Cut:

- **Task-HUID provenance clauses, 67 of the base's 68 HUID occurrences**,
  spread over 67 comment lines of `lib.rs` (63) and `settings_store.rs` (4):
  `(task 20260716-214919)`, `Task 20260715-142931.`, `updated for task ...` and
  friends. The constraint they carried was kept; the pointer was the rot.
- **A dead `docs/spikes/` pointer** in the crate doc. `docs/` holds only
  `README.md`; the spike survives as `tasks/20260711-180500/SPIKE.md`. Same
  call, same reason, as 20260731-170329.
- **Wiring narration in `Plugin::build`**, ~60 lines. Comments that restated
  the `add_systems` call under them ("Persistence: load the saved settings once
  at startup..."), or recorded the history of a system that no longer exists
  ("the menu's old per-frame colour-polling system was folded away").
- **`(demo 1)` layout narration** in `setup_menu_ui`.

Kept and promoted to `NOTE:`:

| Site | Guards |
|-|-|
| lib.rs, resource inits | why the menu re-inits resources other plugins own |
| lib.rs, `drive_update_choreography` | why it is ungated by menu state |
| lib.rs, `UiSkin` init | why the transitive init is repeated |
| lib.rs, `OnExit(MainMenu)` | the OnExit-before-OnEnter ordering the teardown relies on |
| lib.rs, the two `.chain()` blocks | why the refresh pair must land in one frame |
| lib.rs, `scroll_menu_lists` | why the wheel-buffer `run_if` exists |
| lib.rs, `toggle_pause` | that pausing `Time<Virtual>` does not stop schedules |
| lib.rs, `PauseStates::NovaOs` | why it skips `setup_pause_ui` |
| lib.rs, outcome + start-failure | the `resource_exists` and Playing-only gates |
| menu_ui.rs, the list pane | `flex_shrink: 0`, with the measured 141..331 px swing |

### DoD 3: the HUID grep

`grep -rnE '//.*[0-9]{8}-[0-9]{6}' crates/nova_menu/` returns exactly one hit:

- `menu_ui.rs` - `// NOTE (20260729-211150): keep `flex_shrink` at 0.` The
  deliberate reference: the ID points at the task that measured the failure the
  literal prevents.

## Defects the pass uncovered

Four pre-existing orphan-docstring defects, all fixed in place because they are
comment-only. The first two were found during the pass; the last two by review
round 1, which is why the class is worth naming: a docstring that describes the
item ABOVE it survives every compile and every test.

- `tests/support.rs` - `app()`'s docstring sat above the `TEST_START_ID`
  constants instead of above `fn app`, so `app()` was undocumented and the
  fixture-id constants carried the wrong description.
- `mods.rs` - `mod_dep_graph`'s docstring opened with a stray copy of
  `on_mod_toggle`'s first paragraph. Removed; `on_mod_toggle` keeps its own.
- `mods.rs` - `DepStatus`'s docstring opened with
  `spawn_mod_details_header`'s paragraph. Removed.
- `widgets.rs` - `button()`'s docstring opened with two sentences describing a
  per-frame colour-polling system that no longer exists (this same pass deleted
  the matching reference in `lib.rs`). Removed.

One defect left alone, out of this crate's scope:

- `nova_gameplay` emits four `ambiguous import visibility` warnings from
  `hud/nova_os_map/mod.rs:45` and `hud/nova_os_ship/mod.rs:55`, landed by
  20260731-170322. Not touched here; worth a backlog task against that crate.

## Evidence

- `cargo check -p nova_menu --all-targets`: clean, zero warnings from this
  crate. Workspace check below.
- `cargo fmt --check`: clean.
- **Test-name multiset identical.** 76 `#[test]` names before, 76 after, same
  names. The module path changes (`tests::foo` -> `tests::pause::foo`); the
  test set does not.
- **Non-comment source text is a pure move.** Stripping comments, blank lines
  and the `pub(crate) ` prefix from the base `lib.rs` and from all eleven new
  files leaves 28 lines present only in the base: import-block fragments,
  `mod tests {`, `use super::*;`, eight function signatures rustfmt re-wrapped
  because `pub(crate) ` pushed them past the margin, and two statements rustfmt
  re-joined. No behavior-carrying line was added, removed or altered.
- **Comment text word-multiset diff.** The same idea one level down, for the
  comment axis: every word in every comment in the crate, HUIDs elided, counted
  against the base. Nothing vanished that was not a deliberate deletion, and
  nothing was invented. This is the check that catches a regex gluing two
  ordinary words together ("untilwires"), which no character-class heuristic
  can see; review round 1 found that site before this check existed.
- `cargo test -p nova_menu --lib`: 76 passed, 0 failed. Re-run after the round-1
  fixes, along with the workspace check and `fmt --check`.
