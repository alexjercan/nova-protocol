//! `nova_authoring` is the OFFLINE half of the content pipeline: the Rust
//! builders that define every built-in scenario and section, the serializer
//! that writes them to the committed `assets/base/**/*.content.ron`, and the
//! lint/balance walk that validates a content tree. It was carved out of
//! `nova_assets` because keeping it there hid the runtime asset stack behind
//! twice its own volume.
//!
//! Touch this crate to change what built-in content IS (a builder under
//! `base_content`, then `content gen`), or to change what the `content lint`
//! gate accepts.
//!
//! The CLI over all of it is [`cli`], reached as the game binary's `content`
//! subcommand:
//!
//! ```text
//! cargo run content gen
//! cargo run content lint [--target <mod>]
//! ```
#![warn(missing_docs)]

mod base_content;

pub mod balance;
/// Narrow runtime-neutral access to reusable built-in scenario scenes.
pub mod built_in_scenarios {
    pub use crate::base_content::scenarios::nova_protocol::{first_shift_scene, FirstShiftScene};
}
/// Generic constructors for Rust-authored scenario configuration.
pub mod scenario_helpers;
// The CLI driver is native-only: `gen` writes through
// `nova_assets::storage::write_atomic`, which does not exist on wasm - and the
// wasm game bundle never exposes the subcommand anyway (the root package
// target-gates this whole crate off wasm).
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
pub mod content_report;
/// Deterministic serialization of the private built-in content inventory.
pub mod generation;
pub mod lint_walk;

/// Glob-import surface: `use nova_authoring::prelude::*` brings the content
/// report model, the lint/balance walk entry points and the RON generation
/// surface into scope.
pub mod prelude {
    pub use super::{
        balance::prelude::*, built_in_scenarios::*, content_report::prelude::*,
        generation::prelude::*, lint_walk::prelude::*, scenario_helpers::prelude::*,
    };
}
