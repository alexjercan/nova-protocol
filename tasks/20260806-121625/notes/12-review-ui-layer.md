# Code review - nova_ui, nova_menu, nova_editor, nova_os

Source: dedicated reviewer, 2026-08-07. Spot-verified.

## Two entire features are dead - VERIFIED

This is the strongest evidence yet for the owner's third deletion target
("dead and lying surface").

### `Tween` - the whole subsystem has no consumers

`crates/nova_ui/src/tween.rs` (421 lines, 11 tests) is registered into the real
app at `crates/nova_gameplay/src/hud/mod.rs:301`.

```
grep -rn "Tween<|Tween::new|TweenFinished" crates/ src/ examples/ \
  | grep -v nova_ui/src/tween.rs
-> (no output)
```

**Zero hits.** No `Tween<T>` component is ever spawned outside the module's own
tests. `TweenPlugin` runs four empty queries every frame, and none of the
completion-policy code has ever executed against real game state. The
docstring advertises guarding "a killed enemy's flash tween"; no such call
exists.

VERIFIED. This corrects the implication in `08-tests-ci-risk.md` that
`tween.rs` was untested vendored code - it has 11 tests. The problem is the
opposite: it is well-tested code that nothing uses.

Related latent defect if it ever does get a consumer: `tween.rs:243` inserts
`TweenFinished` on completion and nothing removes it, so it is a latch rather
than an event. A second tween on the same entity makes `try_insert` a replace,
not an add, so the `On<Add, TweenFinished>` observer never fires again.

### `StatusBarStore` - declared, initialised, never touched

```
grep -rn "StatusBarStore" crates/ src/
-> status_bar.rs:133  pub struct StatusBarStore {
-> status_bar.rs:153  app.init_resource::<StatusBarStore>();
```

The per-entity staging it documents is actually done by the
`StatusBarItemValue` component. VERIFIED dead.

### Confirming the zero-test claim

`08-tests-ci-risk.md` said `status_bar.rs`, `camera/chase.rs` and
`camera/wasd_controller.rs` were vendored with no tests. Re-measured:

| File | `#[test]` |
| --- | --- |
| `nova_ui/src/status_bar.rs` | 0 |
| `nova_gameplay/src/camera/chase.rs` | 0 |
| `nova_gameplay/src/camera/wasd_controller.rs` | 0 |
| `nova_ui/src/tween.rs` | 11 |

Confirmed for the first three. And `status_bar.rs` is where the reviewer found
three defects - the untested-vendored risk in the register was well aimed.

## status_bar.rs - three defects in untested vendored code

| Site | Defect |
| --- | --- |
| `status_bar.rs:196` | `update_status_bar_item_ui` writes `Text` and `TextColor` **unconditionally every frame**. The version item's value is a `&'static str`, yet `**text = v.to_string()` allocates and marks `Text` changed, so `measure_text_system` + `text_system` re-measure and re-lay-out both status items every frame, forever |
| `status_bar.rs:238` | The entity the caller spawns with `status_bar_item` is **never parented and never rendered**. The observer copies its data into a brand-new child of the root, leaving the caller's entity a permanent orphan with no `Node`. `nova_core/src/lib.rs:290,297` spawns two. Any future "remove this metric" code operating on the returned handle is a silent no-op |
| `status_bar.rs:118,256` | `config.icon.unwrap_or_default()` gives an icon-less item an `ImageNode` on the null handle, and the icon node spawns unconditionally at 16x16. The version item (`nova_core/src/lib.rs:297`, `icon: None`) renders indented ~20px behind an empty slot |

## Persistence

**`nova_menu/src/settings.rs:247`** - the settings save is debounced 15 idle
frames with **no flush on shutdown**, and `save_settings` has exactly one
caller.

Failure: open Settings, drag the volume slider, click Exit (`menu_ui.rs:564`
writes `AppExit` immediately) within ~250ms. `idle_frames` is still
`Some(n<15)`; the write never happens; the setting is silently lost. Same for
a skin or quality click followed immediately by quit.

Severity: bug. Note this is a *third* distinct persistence defect, alongside
the two in `11-review-assets-scenario.md`. Three independent stores, three
different ways to lose data - see the cross-cutting note at the end.

**`nova_menu/src/settings.rs:228`** - the load path clamps `master_volume` but
writes `nova_os_bright_detent` / `nova_os_scan_detent` straight through.
`components.rs:156` clamps on read so the screen looks right, but `advance`
(`components.rs:178`) computes `(99+1) % 4 == 0`, so the next BRIGHT knob click
jumps from brightest to dimmest instead of wrapping from what is displayed.

## nova_editor - five input and lifecycle bugs

The editor is 2,378 LOC and the least-tested UI crate (13 tests). It shows.

| Site | Defect |
| --- | --- |
| `keybind.rs:60` | Keybind chips are root UI nodes with **no `Pickable` override**, so they block the picking ray to the sections they label. `card.rs:24` and `tooltip.rs:22` define an `IGNORE` Pickable for exactly this. Clicking a thruster's chip quadrant does nothing - reads as "clicking randomly does nothing" |
| `placement.rs:315,240,361` | Placement captures **whatever key happens to be held** as the new section's binding, and the editor camera is driven by those same keys. Hold Space or W while clicking to place a turret, and the turret fires on every burn in flight. `ButtonInput::get_pressed()` iterates a HashSet, so holding W+D makes the bind nondeterministic |
| `keybind.rs:187` | Click-to-rebind accepts any key with **no conflict check**. Authored content with that mapping is rejected by `scenario_input_overlaps`, but an editor-built ship is constructed at runtime and never linted |
| `lib.rs:110` | The preview ship is `DespawnOnExit(ExampleStates::Editor)` but `PlayerSpaceshipConfig` is never reset or rebuilt on re-entry. Sandbox -> build -> Play -> F1 back to Editor: no preview exists, every click is dropped, yet Play spawns the old ship from the surviving config |
| `placement.rs:42,100,104` | `get_section("reinforced_hull_section").unwrap()` plus a `panic!` on kind mismatch. A mod overlay redefining or dropping that id **panics the process** on "New Hull Ship". Every other catalog lookup in the codebase logs and skips |

The last one is the same site the cross-cutting sweep independently flagged as
one of only three genuinely-bad `unwrap` sites in the workspace.

## Widget and skin divergence

**`nova_ui/src/widget/button.rs:496`** - `button_on_setting` fires on
`On<Add, Pressed>` (mouse-DOWN) while every other button commits on `Activate`
(release-over). Press and hold a UI-skin option, drag off, release: the skin
has already changed, with no cancel. Severity: bug.

**`nova_menu/src/settings.rs:95`** - the Settings panel spawns raw `Text` spans
with no `nova_ui::widget::UiText` marker, so `apply_ui_font` never routes them
through `UiFont`. The "Volume" label, the `NN%` readout, the Controls headers
and both columns of every keybind row render in **Bevy's default face** while
their siblings render in Iosevka Term. Same at `pause.rs:203,286`.
`settings.rs` and `pause.rs` are the only menu files that never import
`UiText`. Severity: bug, and visible in any screenshot.

**`nova_ui/src/widget/panel.rs:112`** - `panel_head` takes a `UiSkin` and
discards it (`_skin`). Switching to Hardware repaints the panel to grey but
leaves the header a green CRT band. The `skin` parameter makes every call site
believe otherwise.

Three more paint divergences between the two skins, all nits but all of the
same class - `button.rs:244` (hardware Danger collapses hover and press into
one paint, so Exit has no press feedback on one skin), `slider.rs:26` (`round()`
instead of floor, so the phosphor meter shows 98% as full and 2% as empty while
the hardware fill shows both correctly), `slider.rs:78` (phosphor track padding
is not accounted for in bevy's hit math, so clicks land ~3px off).

**`nova_menu/src/widgets.rs:75`** - `ScrollPosition` is only clamped in the
wheel handler, so nothing re-clamps when content shrinks. Scroll the Scenarios
list to the bottom, collapse a campaign header: the pane renders blank until
the player nudges the wheel. **Distinct from the known clamp duplication** -
and note this makes three separate scroll defects across two files, all fixed
at once by the `nova_ui::screen` extraction in `06-ui-layer.md`.

## nova_os terminal

- `edit.rs:293` `completion_matches` iterates a `std::collections::HashMap`
  and appends without dedup, so Tab-cycle order for argument candidates depends
  on hash iteration order and varies between processes.
- `edit.rs:109` history is unbounded and never deduped; only `reset_session`
  clears it. 200 submits of `log` means 200 Up presses to reach anything else.
- `view.rs:222` the completion ghost strips the prefix from the raw prompt
  while `refresh_parse` built the hint from the trimmed one, so a leading space
  turns the prompt green with no ghost. **Independently found by the HUD
  reviewer too** - see `10-review-hud-nova-os.md`.

## Came back clean

- **`tween.rs` easing math**: `fraction()` guards `duration <= 0`, `advance`
  clamps at duration, `sample_clamped` bounds the ease, and the despawn race is
  covered by two tests using `auto_insert_apply_deferred: false`.
- **`settings_store.rs`**: every field has a serde default, `load` returns
  `None` (not a reset-to-default write) on parse failure, and `save_to` creates
  the parent dir. Four tests pin exactly this. **The "one bad field wipes all
  settings" hypothesis is dead.**
- `persist_settings_on_change`'s `is_added` guard is correct - a launch that
  changes nothing never rewrites the store.
- **Pause `OnEnter`/`OnExit` symmetry is correct.** `pause_clocks`/
  `unpause_clocks` and `release_cursor`/`restore_cursor` pair on both the
  `Paused` and `NovaOs` axes; `force_unpause` on `OnExit(Playing)` covers the
  Back path. No stranded cursor, no double-spawn.
- Byte-vs-char indexing in the terminal: no UTF-8 panic reachable. **Third
  independent confirmation.**
- `reconcile_segmented_skins` / `reconcile_slider_track_skins` missing an
  `Added` override is NOT a defect - unlike `button()`, both factories take
  `skin` as an argument, so a fresh spawn is already correct.

## Bearing on the epic

- The two dead features (`Tween`, `StatusBarStore`) are pure deletion, zero
  risk, and they are exactly the target the owner picked. `Tween` alone is 421
  lines plus a plugin registration.
- The `nova_ui::screen` extraction proposed in `06-ui-layer.md` now fixes
  **four** defects rather than just removing duplication: the two unit-mismatch
  clamps, the shrink-clamp, and the third unclamped editor variant.
- `nova_editor` is the weakest crate in the tree by defect density (5 bugs in
  2,378 LOC). It was not on the epic's list. It should be.
