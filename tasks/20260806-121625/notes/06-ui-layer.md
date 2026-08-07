# UI layer - nova_ui, nova_menu, nova_editor, nova_os

## Module maps

**nova_ui** (3,703): `theme` palette + metrics + `semantic` HUD accents; `skin`
UiSkin selector; `font` shared typeface handle; `units` distance/speed
formatting; `widget/{button,panel,list_row,segmented,slider,chrome,paint}`
skin-aware widget families + `register`; `hud` chip language; `status_bar`
metrics bar plugin; `tween` value tween plugin.

**nova_menu** (8,154): `lib` `NovaMenuPlugin`; `menu_ui` root menu + sub-screen
scaffolding; `settings`/`settings_store`; `mods` installed-mods list/details;
`portal` remote catalog Explore tab + install choreography; `scenarios`
picker/campaign tree/thumbnails; `pause` overlay + clock/cursor control;
`outcome` win/lose overlay; `ambience` backdrop scenario + camera; `widgets`
menu button wrappers + list scrolling; `tests/*` (2,800 LOC, 35% of the crate).

**nova_editor** (2,378): `lib` plugin + `ExampleStates`; `placement` ship
build/preview/delete observers; `keybind` section keybind chips and rebind
capture; `scenario` play-test scene; `config` build-state resources;
`ui/{mod,rail,drawer,card,tooltip}`.

**nova_os** (2,560): `terminal/{state,edit,view}` prompt resource, editing,
history, completion, row + prompt rendering; `shell` command matcher and typo
suggestions; `command` registry/arity/footer hints; `app` `NovaOsAppRuntime`
seam.

## Stated boundaries - mostly held

`nova_ui/Cargo.toml` has only bevy + serde. **No nova_os dependency.** Correct.

`nova_os` has zero UI symbols - grep for `Node|BackgroundColor|Text(|Camera`
returns nothing. The one bevy-UI leak is `crates/nova_os/src/app.rs:54`
`spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>)`, a
view-construction seam inside the "no UI" crate.

## Real violations

- **One-plugin-per-subsystem broken in nova_ui.** Two plugins
  (`status_bar.rs:147`, `tween.rs:198`) plus a third ad-hoc entry point
  `widget::register` (`widget/mod.rs`) guarded by a `WidgetObserversRegistered`
  resource - a first-caller-wins hack standing in for a plugin.
- **SystemSet grouping absent where it matters.** `nova_menu` registers ~25
  systems flat across `lib.rs:83-219` with no `SystemSet`. Only nova_ui uses
  `SystemSet` at all (`tween.rs:102`, `status_bar.rs:139`).
- **Direct cross-crate type coupling.** nova_menu reaches
  `nova_gameplay::prelude::NovaOsMonitorSettings` (`lib.rs:109`,
  `settings.rs:223`, `settings_store.rs:86`) and nova_assets portal commands
  (`portal.rs:11-14`). nova_editor imports `nova_gameplay::prelude::*` wholesale
  (`lib.rs:18`).
- **Prelude leaks.** `nova_ui::widget::button_on_setting` at
  `nova_menu/src/lib.rs:24` and `nova_editor/src/lib.rs:31`;
  `nova_ui::theme::AMBER_NOVA` and `nova_ui::hud::CHIP_RADIUS` in HUD files
  (`nova_gameplay/src/hud/ammo_readout.rs:480`, `target_inset.rs:301`).

nova_ui's prelude is effectively dead: 81 in-src deep-path uses against 3
prelude imports.

## Is nova_ui the shared layer?

**At the token level, yes.** Zero local `const _: Color` in nova_menu or
nova_editor; only 4 and 6 raw `Color::srgb` literals (`nova_editor/src/ui/card.rs:31-38`
kind tints). `nova_menu/src/widgets.rs:38` is a thin wrapper over
`nova_ui::widget::menu_button`.

**At the composition level, bypassed.** Three duplication sites:

| Duplication | Sites |
| --- | --- |
| Scroll viewport, ~~3x~~ | ~~`nova_menu/src/widgets.rs:66 max_menu_scroll_y`; `nova_gameplay/src/hud/nova_os/input.rs:430 max_nova_os_scroll_y`; `nova_editor/src/ui/mod.rs:61 scroll_editor_panel` (a third, unclamped variant). The menu doc comment literally says "Mirrors max_nova_os_scroll_y"~~ **Corrected 2026-08-07 - see below.** There are **two** `max_*_scroll_y`, not three |
| List+details screen, 3x inside nova_menu | `mods.rs:278/299/326/591`, `scenarios.rs:112/126/183/400`, `portal.rs:303` - each with its own `*_list_dirty` / `*_details_dirty` / `refresh_*_list` / `refresh_*_details` / `spawn_*_row`. None of it lives in nova_ui |
| Tooltip | only in `nova_editor/src/ui/tooltip.rs` though it is generic |
| Keybind chips, 2x | `nova_editor/src/keybind.rs` vs `nova_gameplay/src/hud/keybind_dock.rs` |

### Correction 2026-08-07 - the scroll count was wrong and the argument got stronger

Source: `10-review-hud-nova-os.md` and `12-review-ui-layer.md`. All four sites
re-verified against the tree 2026-08-07.

There are **two** `max_*_scroll_y` functions, not three, and they agree with
each other:

| Site | Verified | State |
| --- | --- | --- |
| `nova_gameplay/src/hud/nova_os/input.rs:430` `max_nova_os_scroll_y` | yes | clamped, **and buggy** |
| `nova_menu/src/widgets.rs:66` `max_menu_scroll_y` | yes | clamped, **same bug** |
| `nova_editor/src/ui/mod.rs:61` `scroll_editor_panel` | yes | **unclamped** - a different defect, not a third copy |

**Both copies carry a physical-vs-logical pixel unit bug.** They build the
bound from `ComputedNode::content_size` / `size` / `scrollbar_size`, which are
**physical** pixels, while `ScrollPosition` is **logical** - bevy converts with
`scroll_pos.y * inverse_target_scale_factor.recip()`
(`bevy_ui-0.19.0/src/layout/mod.rs:346-360`). On a 2x display the returned
maximum is twice the real one. The codebase already knows the rule:
`position_nova_os_block_caret` (`shell.rs:440`) multiplies by
`inverse_scale_factor()` and `screen_indicator.rs:418` carries the comment
"`ComputedNode::size` is PHYSICAL". These two sites just missed it.

A **fourth** scroll defect is in the same neighbourhood:
`nova_menu/src/widgets.rs:75 scroll_menu_lists` clamps only in the wheel
handler, so nothing re-clamps when content shrinks. Collapse a campaign header
with the Scenarios list scrolled to the bottom and the pane renders blank until
the player nudges the wheel.

**So the `nova_ui::screen` extraction below is no longer a duplication
argument.** It fixes four defects in one edit:

1. the unit bug in `max_nova_os_scroll_y`
2. the same unit bug in `max_menu_scroll_y`
3. the shrink-clamp gap in `scroll_menu_lists`
4. the unclamped editor variant

Deduplicating means fixing the unit bug once instead of twice, and gives the
third and fourth sites a correct implementation to adopt.

## NOVA OS ownership smear

Not a sensible split - the feature is spread across four crates:

| Where | What |
| --- | --- |
| `nova_os` | terminal model, shell, app runtime. No UI |
| `nova_gameplay/src/hud/nova_os/**` | ~5k: casing, crt, shell, spawn, lists, input, sound, content, style |
| `nova_gameplay/src/hud/nova_os_map/`, `nova_os_ship/` | ~4k sibling apps; plus `nova_os_pointer_rig.rs:396` |
| `nova_menu` | **state and settings**: `NovaOsMonitorSettings` at `lib.rs:109`; `OnEnter/OnExit(PauseStates::NovaOs)` clock+cursor hooks at `lib.rs:185-190`; persistence at `settings_store.rs:86`; special-case at `pause.rs:51-54` |

The "no UI in nova_os" rule is honored, but the feature has no owner, and the
crates are coupled by direct type imports rather than events.

## Size outliers

| File | Lines | Note |
| --- | --- | --- |
| `nova_menu/src/mods.rs` | 873 | list + details + deps + checkbox sync in one file |
| `nova_ui/src/widget/button.rs` | 863 | two full paint backends (`phosphor_paint:115`, `hardware_paint:178`), `ButtonSpec` builder, key chips, generic `button_on_setting:496` |
| `nova_menu/src/tests/portal.rs` | 727 | |
| `nova_gameplay/src/hud/nova_os/spawn.rs` | 715 | |
| `nova_gameplay/src/hud/nova_os/casing.rs` | 662 | no module doc |

Worst single function: **`setup_menu_ui` spans `menu_ui.rs:32-540`** - ~500
lines building every menu screen inline. Runner-up: `refresh_mod_details`
`mods.rs:591-760`.

## Comments

Doc lines / total: nova_ui 764/3,703 (21%), nova_os 458/2,560 (18%), nova_menu
842/8,154 (10%), nova_editor 182/2,378 (8%).

Quality is good. Module docs are why-docs - `nova_ui/src/theme.rs:10-18`
explains the retired palette and the drift test;
`nova_gameplay/src/hud/nova_os_ship/mod.rs:24-31` explains the blip-vs-mesh
pickability decision. Inline comments skew why, roughly 4:1.

**The noise is stale narrative, not what-comments**: docs referencing task
artifacts (`nova_gameplay/src/hud/nova_os/mod.rs:18` "see this task's
DECISION.md"; `nova_os_ship/mod.rs:33` "DECISION fork 4") and
self-congratulatory history (`nova_ui/src/theme.rs:130-138`).

## nova_editor was not on the list and should be - added 2026-08-07

`12-review-ui-layer.md` found **five defects in 2,378 LOC** against 13 tests
(both re-counted 2026-08-07: 13 `#[test]`, and the crate size matches
`02-workspace-map.md`). That is the worst defect density in the workspace, and
this note's ranked improvements did not mention the crate at all.

| Site | Defect | Verified |
| --- | --- | --- |
| `placement.rs:42,100` | `get_section("reinforced_hull_section").unwrap()` and `get_section("basic_controller_section").unwrap()`, plus `panic!` on kind mismatch at `:46,:104` and a third `panic!` at `:205`. A mod overlay redefining or dropping either id **panics the process** on "New Hull Ship". Every other catalog lookup in the codebase logs and skips | yes - five panic sites, one more than the review reported |
| `placement.rs:315` | Placement captures whatever key happens to be held as the new section's binding, and the editor camera uses those same keys. `ButtonInput::get_pressed()` iterates a HashSet, so W+D makes the bind nondeterministic | yes |
| `keybind.rs:60` | Keybind chips are root UI nodes with no `Pickable` override, so they block the picking ray to the sections they label. `card.rs:24` and `tooltip.rs:22` define an `IGNORE` Pickable for exactly this | yes |
| `keybind.rs:187` | Click-to-rebind accepts any key with no conflict check; an editor-built ship is never linted by `scenario_input_overlaps` | yes |
| `lib.rs:110` | Re-entering the Editor never resets or rebuilds `PlayerSpaceshipConfig`, so Sandbox -> build -> Play -> F1 leaves no preview, drops every click, and Play spawns the old ship | **citation corrected**: `lib.rs:110` is the `OnEnter(ExampleStates::Editor)` registration (where the reset would go). The `DespawnOnExit(ExampleStates::Editor)` the review quoted is at `placement.rs:32,90`, not `lib.rs:110`. The defect stands; the anchor was imprecise |

## Ranked improvements

1. **Extract a `nova_ui::screen` list+details+scroll composition module.**
   Collapses the mods/scenarios/portal triplication and the scroll clamps.
   Cost: 5 nova_menu files plus the nova_os drawer; behavior-identical, guarded
   by the existing menu test suite. **Amended 2026-08-07: this now fixes four
   defects, not just duplication** - see the scroll correction above.
2. **Give NOVA OS one owner** - a `nova_os_ui` crate, or a single `NovaOsPlugin`
   owning `NovaOsMonitorSettings`, the pause-axis hooks and persistence, with
   nova_menu reduced to emitting a settings event. Cost: ~10 files plus a
   settings-store field group; persisted-settings migration risk.
3. **Split nova_menu into per-screen plugins** under one `NovaMenuPlugin` with
   SystemSets (menu/settings/mods/scenarios/pause/outcome). Cost: mechanical,
   but the `lib.rs` ordering NOTEs must become explicit `.chain()`/set edges.
4. **Break up `setup_menu_ui`** into per-screen builders; split
   `widget/button.rs` into `spec.rs` + `paint_phosphor.rs` + `paint_hardware.rs`.
   Pure code motion, no test changes.
5. **Fix the plugin and prelude rules.** Fold `widget::register`, `TweenPlugin`
   and `StatusBarPlugin` into one `NovaUiPlugin` (dropping the
   `WidgetObserversRegistered` guard); export `theme`/`hud` consts through
   preludes so HUD files stop using deep paths. Cost: every consumer's plugin
   registration changes; guard removal needs double-registration checked in the
   menu and editor apps.
