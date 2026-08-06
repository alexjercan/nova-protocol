# Prototype 01 - the event engine -> `nova_events` (+ `nova_events_macros`)

**Do this first.** `nova_events` is a leaf (`bevy` + `serde`), and every other
BCS consumer sits downstream of it. Nothing else can drop the dep until this
lands.

## Scope

| From (BCS @ 6f09461) | LOC | To |
|---|---|---|
| `src/modding/events.rs` | 561 | `crates/nova_events/src/engine.rs` |
| `bevy_common_systems_macros/src/lib.rs` | 44 | new crate `crates/nova_events_macros/src/lib.rs` |

Exports that must survive by name (nova code is written in them):
`GameEvent`, `GameEventInfo`, `GameEventQueue`, `EventHandler`, `EventAction`,
`EventFilter`, `EventWorld`, `GameEventsPlugin`, `CommandsGameEventExt`,
`EventKind`, `EventHandlerIndex`, and the `EventKind` derive.

## Explicitly dropped

`src/modding/registry.rs` (494 L). `EventHandlerRegistry`, `HandlerSpec`,
`parse_specs`, `RegistryError` - zero references under `crates/` or
`examples/`. Verified.

## Manifest work

### New crate `crates/nova_events_macros`

Copy `bevy_common_systems_macros/Cargo.toml` and change the name. It is
`[lib] proc-macro = true` with `quote` / `syn` (features `["full"]`) /
`proc-macro2`. Add `"crates/nova_events_macros"` to `[workspace] members` in
the root `Cargo.toml`, in alphabetical position next to `crates/nova_events`.

`nova_events` is the derive's only user - do not make it a workspace-wide dep.

### `crates/nova_events/Cargo.toml`

```diff
-bevy_common_systems = { git = "...", tag = "v0.19.5" }
+serde_json = { version = "1" }
+nova_events_macros = { path = "../nova_events_macros" }
```

**`serde_json` is required.** `modding/events.rs:156` declares
`pub data: Option<serde_json::Value>` and `:162` calls `serde_json::to_value`.
NOTES.md's claim that serde_json is "persist + registry only" is wrong.
`serde` is already there.

The `debug` feature at `Cargo.toml:18` forwards to `bevy_common_systems/debug`.
The engine code has no `debug` gating of its own, so after the copy the feature
reduces to `debug = ["bevy/track_location"]`. **Check whether anything still
selects `nova_events/debug`** - if nothing does, delete the feature and its
forwards. Do not leave a feature that forwards to nothing.

## Module wiring

`crates/nova_events/src/lib.rs:12` currently does
`use bevy_common_systems::prelude::*;` - the event derives in this file resolve
`EventKind` through it. After the copy that becomes `use crate::engine::*;`
(or an explicit list).

Add to `lib.rs`:

```rust
pub mod engine;
```

and extend the existing `prelude` (`lib.rs:17-26`) with the engine names.
Note the current prelude is an explicit list of nova's own event types; append
the engine names to it rather than adding a second glob.

`engine.rs` needs `pub use nova_events_macros::EventKind;` re-exported at the
crate root or in the prelude, because the derive expands to
`impl EventKind for #name` and requires the **trait** `EventKind` in scope at
the derive site - which is how BCS's `prelude` worked (`bevy_common_systems_macros::*`
plus `modding::events::EventKind` both landed in one glob). Preserve that
pairing or every downstream `#[derive(EventKind)]` breaks.

## Callsites to repoint

Ordered by crate, all verified present.

### `nova_scenario` (23 files/sites)

The bulk. Two import shapes:

- `use bevy_common_systems::prelude::*;` at
  `src/actions/{flow,mission,mod,ship,spawn,view}.rs`, `src/events.rs`,
  `src/lib.rs:41`, `src/loader/{clock,lifecycle,trackers}.rs`
- `use bevy_common_systems::modding::prelude::*;` at `src/filters.rs:2` -
  this one is the `modding` sub-prelude specifically, and after the drop of
  `registry.rs` it is exactly the engine names. Point it at
  `nova_events::prelude::*`.
- Fully-qualified sites: `src/loader/lifecycle.rs:791,795`,
  `src/world.rs:411,485`, `src/objects/asteroid.rs:3`, `src/objects/area.rs:3`
- Test-local `use` inside `#[cfg(test)]` mods: `src/world.rs:438,492,517,544,564`,
  `src/actions/mission.rs:573,628,660`, `src/actions/ship.rs:191,268,335`,
  `src/actions/spawn.rs:546,587`, `src/actions/view.rs:570,594`,
  `src/loader/clock.rs:122,175,272,344,433`,
  `src/loader/trackers.rs:248,374,526`, `src/loader/lifecycle.rs:716`,
  `src/objects/{area.rs:170,asteroid.rs:762,salvage.rs:343}`,
  `src/filters.rs:202`
- `benches/scenario_dispatch.rs:26`:
  `use bevy_common_systems::{modding::events::GameEventQueue, prelude::*};`
  -> `nova_events::engine::GameEventQueue` + `nova_events::prelude::*`

Careful: some of those `prelude::*` globs are ALSO pulling non-engine BCS names
(`GameObjectives`, `WASDCameraController`, `EventWorld`). `GameObjectives` does
not arrive until prototype 08 and `WASDCameraController` until 03. Expect
`nova_scenario` to still need the BCS dep after this step - that is fine, this
prototype only guarantees `nova_events` itself is clean.

Concretely: after step 01, only `nova_events` drops the dep. `nova_scenario`
and `nova_assets` drop it in prototype 10, once every name they glob has a nova
home. NOTES.md's suggested order ("drop from nova_events, then nova_scenario,
then nova_assets") is optimistic about `nova_scenario`; do not force it.

### `nova_assets`

`bevy_common_systems` is a **dev-dependency** (`Cargo.toml:72`), tests only:

- `src/scenario/shakedown/tests/walk.rs:5` - `CommandsGameEventExt`,
  `EventHandler`, `GameEventsPlugin`
- `tests/{broadside_assault,final_tally_claim,gauntlet_course,ledger_ch2_encounter,ledger_ch3_channel,ledger_ch4_ending,ledger_ch5_raid,lifeline_convoy,neutralized_ships}.rs`
  - all `use bevy_common_systems::prelude::{...}`

These are pure engine names, so `nova_assets` can drop the dev-dep in this
step. `nova_events` is already a dev-dep (`Cargo.toml:73`). Confirm the braced
import lists contain nothing outside the engine surface before deleting line 72.

### `nova_gameplay`

- `src/integrity/neutralize.rs:18` - `CommandsGameEventExt`
- `src/integrity/neutralize.rs:158`, `src/integrity/glue.rs:610` - test-local
  `GameEvent`

`nova_gameplay` already depends on `nova_events`.

## Compile hazards

- `EventKind::Info` is bounded on `serde::Serialize` - the copied trait needs
  `serde` in scope at `nova_events`, which it already has.
- The macro's guard comment about `()` being the default payload (and the test
  `attribute_less_derive_defaults_to_no_payload` in `modding::events`) must
  survive the copy. That test is in `events.rs` and comes with it.
- `events.rs` has 2 occurrences of the string `bevy_common_systems` - rustdoc.
- `#![warn(missing_docs)]` is on in `nova_events`: `pub mod prelude { ... }` in
  the copied file has no doc comment, and `GameEventInfo`'s pub fields may not
  either.

## Verification

```
nix develop --command cargo check -p nova_events --all-targets
nix develop --command cargo check --workspace --all-targets
nix develop --command cargo test -p nova_events --lib
nix develop --command cargo test -p nova_scenario --lib filters::
nix develop --command cargo fmt --check
```

`nova_scenario`'s bench must still compile: `cargo check -p nova_scenario --benches`.

## Done when

- `crates/nova_events/Cargo.toml` has no `bevy_common_systems`.
- `crates/nova_assets/Cargo.toml:72` (dev-dep) is gone.
- `crates/nova_events_macros` is a workspace member and the only consumer of
  `syn`/`quote`.
- `nova_events/debug` either forwards only to `bevy/track_location` or is
  deleted along with every forward that selected it.
- Workspace gained no new crate-graph edges except
  `nova_events -> nova_events_macros`.
