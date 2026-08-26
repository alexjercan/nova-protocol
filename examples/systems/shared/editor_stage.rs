//! The two literals every editor walk aims at.
//!
//! Four ranges drive the editor, and each carried its own copy of these. The
//! copies drifted: the soak's founding click was `(760, 640)` against the
//! others' `(460, 660)` until a commit in this range corrected it, which is a
//! whole run spent proving that a literal was wrong rather than that the
//! editor works.

use bevy::prelude::*;

/// The top-bar menu carrying Add Ship and the object rows.
pub const ADD_MENU: &str = "Add Menu Button";

/// A viewport point (logical px) with nothing under it on the 1024x768 window
/// the app opens - where the founding click lands, and where a beat points to
/// put a ghost away without disarming the part it is holding.
///
/// Below the stage's own middle and clear of all three panels: the rail takes
/// the left 210 and the inspector the right 300 (`ui/mod.rs`,
/// `ui/inspector.rs`), so a click out at 760 lands ON the inspector and never
/// reaches the stage.
pub const EMPTY_SPACE: Vec2 = Vec2::new(460.0, 660.0);
