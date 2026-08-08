//! `nova_authoring` is the OFFLINE half of the content pipeline: the Rust
//! builders that define every built-in scenario and section, the serializer
//! that writes them to the committed `assets/base/**/*.content.ron`, and the
//! lint/balance walk that validates a content tree. It exists because none of
//! this ships - the game binary loads the generated RON and never links a
//! linter - so keeping it in `nova_assets` hid the runtime asset stack behind
//! twice its own volume.
//!
//! Touch this crate to change what a built-in scenario IS (a builder under
//! `scenario`, then `content -- gen`), or to change what the `content -- lint`
//! gate accepts.
//!
//! The crate's binary is that CLI:
//!
//! ```text
//! cargo run -p nova_authoring --bin content -- gen
//! cargo run -p nova_authoring --bin content -- lint [--target <mod>]
//! ```
#![warn(missing_docs)]

mod scenario;
mod sections;

pub mod balance;
pub mod content_report;
pub mod lint_walk;
pub mod scenario_generation;

/// Glob-import surface: `use nova_authoring::prelude::*` brings the content
/// report model, the lint/balance walk entry points and the RON generation
/// surface into scope.
pub mod prelude {
    pub use super::{
        balance::prelude::*, content_report::prelude::*, lint_walk::prelude::*,
        scenario_generation::prelude::*,
    };
}
