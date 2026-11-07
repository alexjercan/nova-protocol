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
/// Low in the stage and clear of the panels: the Hierarchy takes the left 240,
/// the Inspector the right 300, and the browser strip and the foot the bottom
/// 202 (`ui/shell.rs`), so a click at y 660 lands ON the strip and never reaches
/// the stage.
pub const EMPTY_SPACE: Vec2 = Vec2::new(460.0, 540.0);
