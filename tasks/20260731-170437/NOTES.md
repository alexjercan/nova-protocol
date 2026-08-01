# KISS pass on nova_ui / nova_os / nova_editor / nova_debug - design record

## Structure

Two files were over 1500 lines; both held several concerns. The other 25 files
in the four crates are single-concern and were left where they are (largest
untouched file: `nova_debug/src/harness.rs` at 650 lines - one concern, the
autopilot/smoke harness).

### `nova_ui/src/widget.rs` (2265) -> `nova_ui/src/widget/`

One file held six widget families plus the shared paint helpers and one test
module covering all of them. Split by family, since each family is a
marker + factory + reconciler triple that only touches its own components.

| Module | Concern |
| --- | --- |
| `mod.rs` | module doc, `pub use` re-exports, the shared `Selected` / `UiText` markers, and `register()` - the one call that wires every family's observers and reconcilers |
| `paint.rs` | the gradient/shadow builders shared by buttons and panels |
| `button.rs` | `ThemedButton`: paint model, interaction observers, skin reconciler, `ButtonSpec` and the button factories |
| `chrome.rs` | stateless chrome: `panel_header`, `separator`, `badge`, `checkbox`, `toggle` |
| `panel.rs` | `panel` paint, `PanelSkin` reconciler, `panel_node`, `panel_head` |
| `list_row.rs` | `list_row` + the `ListRow` observers/reconciler |
| `slider.rs` | the two track skins, `sync_slider_tracks`, the rebuild reconciler |
| `segmented.rs` | `segmented_container` / `segmented_option` + its reconciler |
| `fixtures.rs` | `cfg(test)` `skin_app` / `bg` / `has_gradient`, shared by 4 test modules |

`register()` stays in `mod.rs` rather than becoming per-module `register` fns:
the ordering edge between `reconcile_slider_track_skins` and
`sync_slider_tracks` is cross-family, so one schedule site is the simpler
design. The systems it names became `pub(super)`; nothing else changed
visibility.

### `nova_os/src/terminal.rs` (1579) -> `nova_os/src/terminal/`

| Module | Concern |
| --- | --- |
| `mod.rs` | doc + `pub use` (public paths unchanged) |
| `state.rs` | `NovaOsTerminal` and its row/mode types, the accessors, and the session lifecycle (boot reveal, app enter/exit, reset) |
| `edit.rs` | prompt editing, `submit`, completion, history, parse refresh, `TerminalSubmitOutcome` |
| `view.rs` | what the UI renders: welcome/boot/help/version rows and the three prompt strings |
| `fixtures.rs` | `cfg(test)` spec builders + `type_text`, shared by both test modules |

`NovaOsTerminal`'s fields became `pub(super)` so `edit.rs` can drive the state
it owns; the type's public surface is unchanged. `NOVA_OS_PROMPT_PREFIX` was
`pub(crate)` with exactly one use site, so it moved into `edit.rs` as a private
const.

## Public paths

Unchanged in both splits, checked mechanically rather than by eye: the sorted
set of `pub fn` / `pub struct` / `pub enum` / `pub const` names in the pre-split
file and in the post-split folder diff empty, for both `widget` and `terminal`.
Both crate `prelude`s and both `lib.rs` files are untouched. `cargo doc -p
nova_ui -p nova_os --no-deps` went from 4 warnings to 2 (both pre-existing:
`button` -> `reconcile_button_skins` and `UiText` -> `apply_ui_font`, each a doc
link to a private item that predates this task).

## Comments

Applied the epic's rubric across all four crates. Every task-HUID provenance
clause is gone - `grep -rnE '//.*[0-9]{8}-[0-9]{6}'` over the four crates
returns nothing, so DoD 3 needs no exception list (the one `#` comment in
`nova_ui/Cargo.toml` carrying a HUID was cleaned too, though the DoD grep does
not reach it). Surviving constraints were promoted to `NOTE:`: the `try_insert`
despawn race, the missing `TextShadow`, the rebuild/value ordering edge, the
Bevy-immutable `RigidBody` swap, the exact `Name` strings the autopilots press,
the skybox-view insert, the F11 shared-default block, and the prompt hint set.
Narration that restated the next line was deleted, as were the
`// ==== section ====` banners the splits made redundant. Rustdoc was kept and
edited only where a deleted clause left the sentence ragged.

## Evidence that behavior did not change

The `#[test]` count per crate is identical before and after (nova_os 20,
nova_ui 21), no test was renamed or weakened, and `cargo test -p nova_ui -p
nova_os -p nova_editor -p nova_debug --lib` runs 65 tests green. The tests moved
with the code they cover; their shared fixtures were hoisted into a
`#[cfg(test)] mod fixtures` per split rather than copied per child.

The branch's one non-comment deletion is the widget test helper `fn only_button`
(`master:crates/nova_ui/src/widget.rs:1775`) and its `let _ = only_button;`
keep-alive (`:2216`) - dead, speculative scaffolding no test called. Nothing
else outside comments was removed, so a reader reconciling the line multiset
should expect exactly that pair as residue.

## Defects found

None. The pass uncovered no defect worth a backlog task.
