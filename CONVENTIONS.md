# Conventions

Rust style for all Nova Protocol crates. `AGENTS.md` owns non-style rules.

## Modules and public API

1. Start each module with a `//!` document of at most three sentences:
   ownership, the key constraint, and when to change it. Use intra-doc links
   for types and wiki links for concepts.
2. Document information that declarations do not show. Do not restate code.
3. Give each module that exports items a `prelude`. Re-export items by name
   when a glob can include an engine prelude.
4. Import through preludes, including inside the same crate. Do not import from
   another module's internal path.
5. Export each module prelude from the crate root.

```rust
//! Ship command selection shared by player, AI, and targeting systems.
//!
//! Change this module when adding a way to command a ship.

/// The asteroid config, spawner, and plugin.
pub mod prelude {
    pub use super::{AsteroidConfig, AsteroidPlugin};
}

use crate::prelude::*;
```

## Comments and lints

1. Explain constraints and non-obvious choices. Do not record history or
   restate behavior.
2. Do not cite task artifacts in docs. `TODO(<task-id>)` is allowed for live
   tracker work.
3. Explain why a manual trait implementation cannot use `derive`.
4. Use `#[expect(<lint>, reason = "...")]`, not bare `#[allow]`.
   `#[allow(missing_docs)]` in `nova_assets/src/portal/mod.rs` is the exception.

## Bevy systems

1. Name plugin types `<Subsystem>Plugin`.
2. Name system sets `<Subsystem>Systems`.
3. State each scheduling dependency with `.before(...)` or `.after(...)`.
4. Create a `SystemSet` only when another plugin needs an ordering handle.

```rust
app.add_systems(Update, draw_juice_flashes.after(TransformSystems::Propagate));

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;
```

## Tests and formatting

1. Put unit tests in inline `#[cfg(test)] mod tests`. Move a large test module
   to a sibling `src/**/tests/` directory. Keep `crates/*/tests/` for integration
   tests.
2. Name tests as sentences that state the behavior.
3. Use the pinned nightly toolchain and `rustfmt.toml`. Stable Rust ignores the
   nightly `imports_granularity` and `group_imports` settings.
4. Do not enable `clippy::pedantic`, `clippy::nursery`,
   `clippy::wildcard_imports`, `clippy::redundant_pub_crate`,
   `clippy::needless_pass_by_value`, or `clippy::missing_docs_in_private_items`
   across the workspace.
