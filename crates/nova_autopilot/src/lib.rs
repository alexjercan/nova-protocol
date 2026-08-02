//! `nova_autopilot` is the standalone home for Nova Protocol's automation
//! drivers and the completion protocol they share. It depends on `bevy` only:
//! no `nova_*` crate, no `bevy_common_systems`, no `avian3d`.
//!
//! ## Ownership boundary
//!
//! This crate owns the drivers (scripted autopilot, settled-frame screenshot,
//! screenshot reel) and the completion protocol that reports when a run has
//! finished. It does not own anything Nova-specific: the adapters - scenario
//! presets, camera posing, rigid-body freezing, overlay hiding - stay in
//! `nova_debug` and reach in through caller hooks.
//!
//! Nova-SHAPED choices (env var names, defaults, API vocabulary) do live here.
//! Nova TYPES do not. The drivers are generic over the app's state type,
//! `S: States + FreelyMutableState`, and that generic is what keeps
//! `nova_gameplay::GameStates` - and with it the whole game dependency tree -
//! out of this crate.

#![warn(missing_docs)]

/// The scripted autopilot driver.
pub mod autopilot;
// No outer doc here: `completion.rs` carries its own `//!` module docs, and an
// outer `///` would concatenate ahead of them and re-resolve their intra-doc
// links (`AppExit`, `DEADLINE_ENV`) in THIS module's scope, where they do not
// exist. See REVIEW.md R1.3.
pub mod completion;
/// The screenshot reel driver, driven through caller hooks.
pub mod reel;
/// The settled-frame screenshot driver.
pub mod screenshot;

/// Glob-import surface: `use nova_autopilot::prelude::*`. Empty until the
/// drivers and the completion protocol land.
pub mod prelude {}
