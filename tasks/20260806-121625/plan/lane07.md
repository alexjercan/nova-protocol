# L7 - `nova_ui::screen` extraction

**Baseline: BLOCKS - lands AFTER it.** Creates a module and moves code.

Findings: **F17, F28**.

**Depends on:** L2.

**This stopped being a duplication argument.** The extraction fixes **four**
defects in one edit. Fixing the two unit bugs separately means writing the
physical-to-logical conversion twice, which is how they diverged in the first
place.

## The four defects one module closes

| Site | Defect |
| --- | --- |
| `nova_gameplay/src/hud/nova_os/input.rs:430` `max_nova_os_scroll_y` | physical-vs-logical unit bug (F17) |
| `nova_menu/src/widgets.rs:66` `max_menu_scroll_y` | the **same** unit bug, byte-identical body (F17) |
| `nova_menu/src/widgets.rs:75` `scroll_menu_lists` | clamps only in the wheel handler, so nothing re-clamps when content shrinks (F28) |
| `nova_editor` scroll variant | unclamped entirely - gets a correct implementation to adopt |

```rust
// crates/nova_menu/src/widgets.rs:66  and
// crates/nova_gameplay/src/hud/nova_os/input.rs:430   - IDENTICAL today
pub(crate) fn max_menu_scroll_y(computed_node: Option<&ComputedNode>) -> f32 {
    computed_node
        .map(|node| (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0))
        .unwrap_or(f32::MAX)
}
//   ComputedNode is PHYSICAL px; ScrollPosition is LOGICAL
//   (bevy_ui-0.19.0/src/layout/mod.rs:346-360). On a 2x display the maximum is
//   twice the real one. input.rs:257's `page = size.y * 0.8` is physical too,
//   so one PageUp jumps 1.6 viewports.
//   THE CODEBASE KNOWS THE RULE: shell.rs:440 and screen_indicator.rs:418 both
//   multiply by inverse_scale_factor().
```

## The new module

```rust
// NEW  crates/nova_ui/src/screen/mod.rs
//! List + details + scroll composition: the shape `nova_menu` repeats three
//! times (mods, scenarios, portal) and `nova_gameplay`'s NOVA OS repeats once.
pub mod prelude;
mod scroll;
mod list;

// NEW  crates/nova_ui/src/screen/scroll.rs
/// How far a viewport's content overflows its box, in the LOGICAL pixels
/// `ScrollPosition` uses. `ComputedNode` reports physical px, so the
/// conversion happens here and nowhere else.
pub fn max_scroll_y(node: Option<&ComputedNode>) -> f32;

/// One page step for keyboard paging, logical px. Was `size.y * 0.8` in
/// physical px at nova_os/input.rs:257.
pub fn page_step(node: Option<&ComputedNode>) -> f32;

/// Marks a viewport whose stored offset is clamped every frame, not only on
/// wheel input - so collapsing a section cannot leave the pane blank (F28).
#[derive(Component)]
pub struct ScrollViewport;

/// Wheel input for every `ScrollViewport`, clamped at both ends.
pub fn scroll_viewports(
    wheel: MessageReader<MouseWheel>,
    panels: Query<(&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
                  With<ScrollViewport>>,
);

/// Re-clamp every viewport after layout, so a SHRINKING content size pulls the
/// offset back instead of leaving the pane scrolled past its end. This is the
/// system `scroll_menu_lists` never had.
pub fn clamp_viewports(
    panels: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollViewport>>,
);
```

`clamp_viewports` must be ordered **after** `ui_layout_system`, or it clamps
against last frame's `ComputedNode`.

## Call-site changes

```rust
DELETE  crates/nova_menu/src/widgets.rs:66   max_menu_scroll_y
DELETE  crates/nova_menu/src/widgets.rs:75   scroll_menu_lists
DELETE  crates/nova_menu/src/widgets.rs      ScrollableList  -> screen::ScrollViewport
DELETE  crates/nova_gameplay/src/hud/nova_os/input.rs:430  max_nova_os_scroll_y
CHANGE  crates/nova_gameplay/src/hud/nova_os/input.rs:255   PageUp/PageDown ->
        screen::page_step + screen::max_scroll_y
CHANGE  crates/nova_gameplay/src/hud/nova_os/input.rs:426   wheel handler ->
        keeps its `any_hovered` precedence, calls screen::max_scroll_y
CHANGE  crates/nova_editor/...  the unclamped variant adopts ScrollViewport
```

**Interaction with L4's F18.** The `f32::MAX` pin-to-bottom sentinel at
`shell.rs:379` is removed in L4 and replaced with a real clamp. That clamp
should call `screen::max_scroll_y` once this lane lands - if L4 runs first, it
calls the local function and this lane rewrites the call. Expected, cheap.

## The list + details composition

The `mods` / `scenarios` / `portal` triplication inside `nova_menu` collapses
into the same module:

```rust
// NEW  crates/nova_ui/src/screen/list.rs
/// A scrollable list beside a details pane: the shape nova_menu builds three
/// times. Callers supply rows and the details bundle; the module owns the
/// viewport, the selection marker and the scroll wiring.
pub fn list_detail_screen(...) -> impl Bundle;
```

Exact signature falls out of reading the three call sites together - that
reading is the first task of the lane, not a decision to make in advance.

## CONVENTIONS.md rules 3 and 4 - `nova_ui`'s share

`nova_ui` is **the crate that made rule 3 a question**: its root prelude names
40-odd items by hand (`lib.rs:32-51`) while `font.rs` and 5 siblings have no
prelude, so every new public item is a two-file edit. It also has **zero**
`use crate::prelude` anywhere - its own prelude is exercised only by downstream
crates.

```rust
NEW   6 module preludes (font.rs and 5 siblings), plus screen/prelude.rs
CHANGE crates/nova_ui/src/lib.rs:32-51
-  pub use crate::widget::{...40-odd items by hand...};
+  pub use crate::{font::prelude::*, screen::prelude::*, widget::prelude::*, ...};
```

Creating a `screen` module here means creating its prelude anyway - **do the
whole crate in one pass.**

## Escape hatch, if L2 stretches

F17 and F28 are player-visible bugs waiting on owner review time only because
their fix happens to be an extraction. If the benchmark ratification drags,
land the unit conversion and the shrink clamp **in place** during L4's window
(NEUTRAL, ~10 lines, no file moved) and let this lane delete the duplicated
bodies afterwards. Cost: writing the conversion twice. Owner's call.

## Verified by

The existing `nova_menu` test suite - 2,800 LOC, 35% of the crate. **The
best-covered structural change in the epic.** F17 needs a scale-factor test
specifically, since the defect is invisible at scale 1.0.
