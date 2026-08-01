//! `nova_info` exposes build-time metadata. Today that is a single item,
//! [`APP_VERSION`], injected by this crate's `build.rs` at compile time so the
//! menu and about screens can show the running version without every crate
//! taking a dependency on the build script.
#![warn(missing_docs)]

/// The running application version, set by `build.rs` from the Cargo package
/// version at compile time. Displayed on the menu and about screens.
pub const APP_VERSION: &str = env!("APP_VERSION");

/// Glob-import surface: `use nova_info::prelude::*` brings [`APP_VERSION`] into
/// scope.
pub mod prelude {
    pub use super::APP_VERSION;
}
