//! Screen-level composition: scrollable viewports and the list-beside-details
//! layout the menu screens and the NOVA OS drawer share.
//!
//! The scroll half exists because the unit conversion between
//! [`ComputedNode`](bevy::prelude::ComputedNode)'s physical pixels and
//! [`ScrollPosition`](bevy::prelude::ScrollPosition)'s logical ones has exactly
//! one correct spelling; two crates each carrying their own copy is how they
//! drifted apart. The float half is the same conversion on the other side of
//! the screen: a label the world says the position of.

mod float;
mod list;
pub mod prelude;
mod scroll;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
pub use float::*;
pub use list::*;
pub use scroll::*;

/// Wire the shared scroll driver and clamp.
pub(crate) fn build(app: &mut App) {
    scroll::build(app);
}
