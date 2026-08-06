# Prototype 02 - status bar + tween -> `nova_ui`

`nova_ui` is a leaf (`bevy` only, plus optional `serde`). `nova_gameplay` and
`nova_core` already depend on it, so this adds no graph edge.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/ui/status.rs` | 328 | `crates/nova_ui/src/status_bar.rs` |
| `src/tween/mod.rs` | 419 | `crates/nova_ui/src/tween.rs` |

Neither file has any intra-BCS import beyond rustdoc links, and neither needs a
dep `nova_ui` does not already have. This is the cleanest step in the
migration - do it second, right after the events engine.

## Exports that must survive

**status_bar** (BCS `ui::status::prelude`): `status_bar`, `status_bar_item`,
`status_bar_with_fps`, `status_fps_color_fn`, `status_fps_value_fn`,
`status_version_color_fn`, `status_version_value_fn`, `StatusBarItemConfig`,
`StatusBarItemMarker`, `StatusBarPlugin`, `StatusBarPluginSystems`,
`StatusBarRootConfig`, `StatusBarRootMarker`, `StatusValue`.

**tween** (BCS `tween::prelude`): `Tween`, `TweenFinished`, `TweenOnComplete`,
`TweenPlugin`, `TweenSystems`, `TweenValue`.

## Module wiring

`crates/nova_ui/src/lib.rs`:

```rust
pub mod status_bar;
pub mod tween;
```

Then extend the `prelude` block (`lib.rs:26-42`). That prelude is currently an
explicit per-item list; keep that shape. Add the status-bar names nova actually
calls and the tween names `hud/` uses, not the whole surface - the crate-level
`nova_ui::status_bar::` path stays available for the rest.

Update the crate docstring (`lib.rs:1-10`), which enumerates the modules.

## Callsites to repoint

| File | Line | Name |
|---|---|---|
| `crates/nova_core/src/lib.rs` | 283, 286, 293 | `status_bar`, `status_bar_item`, `StatusBarRootConfig`, `StatusBarItemConfig` |
| `crates/nova_gameplay/src/plugin.rs` | 105 | `app.add_plugins(bevy_common_systems::prelude::StatusBarPlugin)` |
| `crates/nova_gameplay/src/hud/mod.rs` | 301 | `app.add_plugins(bevy_common_systems::prelude::TweenPlugin)` |

`nova_core` reaches the status-bar names through
`nova_gameplay::prelude` (the hand-written re-export at
`nova_gameplay/src/lib.rs:77`), not by naming BCS - so `nova_core` needs no
import change **if** you keep re-exporting those names from
`nova_gameplay::prelude` in the interim. Prefer the clean version: point
`nova_core` at `nova_ui::prelude` directly (it already depends on `nova_ui`)
and drop the five status names from the `nova_gameplay` prelude list in this
step rather than in prototype 10.

Tween consumers are `nova_gameplay/src/hud/` only. Grep for `Tween` there and
repoint; they resolve through `nova_gameplay::prelude::*` today, which does
NOT list the tween names - so they must be reaching them another way. Check
this before assuming: `grep -rn 'Tween' crates/nova_gameplay/src/hud/`.

## Plugin-registration move

`StatusBarPlugin` moves from `nova_gameplay/src/plugin.rs:105` and `TweenPlugin`
from `hud/mod.rs:301`. Both stay registered from exactly one place - do not
have `nova_ui` register them itself, `nova_ui` ships no composition root.

Because this moves plugin registration, the examples must be **run** under Xvfb
`:99`, not just checked (project rule: `cargo check` misses duplicate-component
panics).

## Compile hazards

- `status.rs` doc header warns the `value_fn` closures run in an **exclusive
  system** once per frame. Keep that comment.
- `status.rs` uses `bevy::platform::collections::HashMap` and `std::any::Any` -
  no new deps.
- `tween.rs` rustdoc links `crate::meth::lerp::LerpSnap` and `crate::transform`
  (lines 3, 12). Those are **rustdoc intra-doc links to modules that will not
  exist in `nova_ui`** - they become broken-link warnings. Rewrite the two
  sentences to prose, or point them at the nova homes (`meth` lands in
  `nova_gameplay` in prototype 03; a cross-crate intra-doc link from `nova_ui`
  to `nova_gameplay` would be a **new graph edge in the wrong direction** -
  do not add it, just de-link the prose).
- 1 `bevy_common_systems` string in each file (a doctest `use` line).
- `#![warn(missing_docs)]`: `status.rs` has undocumented pub items -
  `StatusBarRootConfig` (line ~29) has no doc, and `pub mod prelude` in both
  files is undocumented.

## Verification

```
nix develop --command cargo check -p nova_ui --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_ui --lib
nix develop --command cargo fmt --check
```

Run one UI example under Xvfb `:99` (the `ui/` category) and one that shows the
status bar, to confirm the bar still draws and the tweens still animate.

## Done when

- `nova_ui` owns both modules and depends only on `bevy` (+ optional `serde`).
- `nova_core` reads the status bar from `nova_ui`, not through
  `nova_gameplay::prelude`.
- `StatusBarPlugin` / `TweenPlugin` are each registered exactly once, from the
  same place as before.
- No new crate-graph edge.
