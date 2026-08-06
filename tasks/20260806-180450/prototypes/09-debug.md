# Prototype 09 - inspector + wireframe -> `nova_debug`

Small, and `nova_debug` is BCS's sole consumer here. The only subtlety is
**three distinct types called `DebugEnabled`**.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/debug/inspector.rs` | 313 | `crates/nova_debug/src/inspector.rs` |
| `src/debug/wireframe.rs` | 66 | `crates/nova_debug/src/wireframe.rs` |

Do **not** copy `src/debug/harness/` - nova has `nova_autopilot`
(tasks `20260802-183403`, `20260802-183406`), and BCS's harness twins are the
exact plugins that used to boot every example inert. Do not copy
`src/debug/mod.rs` either; its prelude re-exports `harness::prelude::*`.

## The three `DebugEnabled` resources

`nova_debug/src/lib.rs:17-22` currently does:

```rust
use bevy_common_systems::{
    debug::{
        inspector::DebugEnabled as InspectorEnabled, wireframe::DebugEnabled as WireframeEnabled,
    },
    prelude::*,
};
```

and `nova_debug/src/lib.rs:82` defines nova's own third:

```rust
pub struct DebugEnabled(pub bool);
```

nova's is the master toggle (F11, `DEBUG_TOGGLE_KEYCODE`,
`DEBUG_LAYER_STARTS_ON`); the other two are the per-plugin gates it drives.
`harness.rs:426` and `:431` reach the BCS pair by **full path**, and
`harness.rs:412` documents them.

**Keep all three distinct.** After the copy:

- `crate::inspector::DebugEnabled` and `crate::wireframe::DebugEnabled`
- the `as InspectorEnabled` / `as WireframeEnabled` aliases at `lib.rs:17-22`
  stay, now aliasing crate-local paths
- `harness.rs:426,431` change from
  `bevy_common_systems::debug::inspector::DebugEnabled` to
  `crate::inspector::DebugEnabled` (same for wireframe)
- `harness.rs:412`'s doc comment stops saying "bevy_common_systems"

Renaming them to `InspectorEnabled` / `WireframeEnabled` at the definition site
is tempting and is **out of scope** - it is a rename of copied code, and the
alias already does the job at the one place that needs it.

## Dependency change: `bevy-inspector-egui`

`inspector.rs` needs `bevy_inspector_egui` and `avian3d`:

```diff
 # crates/nova_debug/Cargo.toml
-bevy_common_systems = { git = "...", tag = "v0.19.5", features = ["debug"] }
+bevy-inspector-egui = { version = "0.37" }
```

`avian3d` is already there with `features = ["diagnostic_ui"]`
(`Cargo.toml:10`) - which is exactly what BCS's `debug` feature was forwarding
(`debug = ["avian3d/diagnostic_ui", "bevy/track_location", "bevy-inspector-egui"]`).
`bevy` already has `track_location` (`Cargo.toml:11`). So the whole of BCS's
`debug` feature is already satisfied by `nova_debug`'s own manifest except the
egui dep.

`nova_debug` has no `[features]` block of its own - the crate is compiled only
under the game's `debug` feature. Make `bevy-inspector-egui` a plain
(non-optional) dep; adding a feature to gate it would be a knob with no caller.

Version: pin `0.37` to match what BCS resolved. Check `Cargo.lock` after the
change - `bevy-inspector-egui` was already in the tree transitively, so this
should be a lock-graph edge move, not a new download.

## Callsites to repoint

| File | Line | What |
|---|---|---|
| `nova_debug/src/lib.rs` | 17-22 | the aliased import + `prelude::*` |
| `nova_debug/src/lib.rs` | ~99, ~100 | `app.add_plugins(InspectorDebugPlugin)` / `WireframeDebugPlugin` |
| `nova_debug/src/harness.rs` | 412 | doc comment |
| `nova_debug/src/harness.rs` | 426, 431 | full-path `DebugEnabled` reads |
| `nova_debug/src/harness.rs` | 78 | `WASDCameraController` - **prototype 03's**, not this one |

Note `lib.rs:17-22`'s `prelude::*` is pulling more than the two debug types
(that is where `WASDCameraController` and friends come from). This step removes
the `debug::{...}` half; the `prelude::*` half survives until prototypes 03 and
08 land. `nova_debug` cannot drop the BCS dep until then - it drops in
prototype 10.

## Module wiring

`crates/nova_debug/src/lib.rs`:

```rust
pub mod inspector;
pub mod wireframe;
```

alongside the existing `gravity`, `harness`, `screenshot`, `sections`.

`nova_debug`'s prelude (`lib.rs:37-67`) deliberately does **not** export the
raw driver types, to avoid clashing with Bevy's own `ScreenshotPlugin`. Follow
that precedent: do not add `InspectorDebugPlugin` / `WireframeDebugPlugin` to
the prelude. `DebugPlugin` adds them; nothing else needs them.

Update the crate docstring (`lib.rs:1-9`), which describes the inspector as
external.

## Compile hazards

- `inspector.rs` has **zero** `bevy_common_systems` strings; `wireframe.rs` has
  one (a doctest `use bevy_common_systems::debug::prelude::*;`) - that whole
  doctest belongs to BCS's `debug/mod.rs` idiom and should be rewritten to
  `nova_debug`'s.
- `inspector.rs` is `#[cfg(feature = "debug")]`-gated at the BCS crate root
  (`bcs lib.rs:11`). `nova_debug` has no such gate - the crate itself is the
  gate. Strip any inner `#[cfg(feature = "debug")]` the copied code carries and
  verify none is load-bearing.
- The egui crate name is `bevy-inspector-egui` in Cargo and
  `bevy_inspector_egui` in Rust.
- `#![warn(missing_docs)]` on `nova_debug`.

## Verification

```
nix develop --command cargo check -p nova_debug --all-targets
nix develop --command cargo check --workspace --all-targets --features debug
nix develop --command cargo test -p nova_debug --lib
nix develop --command cargo clippy --workspace --all-targets --features debug
nix develop --command cargo fmt --check
```

CI's one clippy pass runs with `--features debug` (`.github/workflows/ci.yaml:70`),
so this is the step most likely to surface a lint CI catches and a bare check
does not. Run that clippy line locally for this step even though the project
rule is to skip local clippy - it is the only crate the debug feature gates
in, and a break here is invisible otherwise.

**Run** a debug-featured example under Xvfb `:99` and press F11: the master
toggle must still raise the inspector panel, the avian gizmos and the wireframe
pass as one layer. Three `DebugEnabled` resources getting crossed is a
behavior bug no test here covers.

## Done when

- `nova_debug` owns `inspector.rs` and `wireframe.rs`.
- Three `DebugEnabled` types still exist and stay distinct; the F11 layer
  raises all of it together.
- `bevy-inspector-egui = "0.37"` is a direct `nova_debug` dep, non-optional.
- No harness code came across.
- `cargo clippy --workspace --all-targets --features debug` is clean.
