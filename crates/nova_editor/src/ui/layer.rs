//! The editor's z ladder, in one place.
//!
//! UI stacking follows the tree unless a node claims a [`GlobalZIndex`], and
//! the editor's surfaces are spawned from six modules that cannot see each
//! other's spawn order. Every one of them reads its rung from here instead of
//! picking a number that looked free.
//!
//! The rule the ladder encodes: the STAGE's own text - a label hung off a node
//! out in the world - is part of the scene, and the docked chrome is the window
//! you look at the scene through. A window is never behind what it frames, so
//! anything anchored to the world sits below the panels.

/// Names hung on nodes out in the world: the hull nameplates, the section
/// keybind chips and the leaders that tie them to their parts.
///
/// The BOTTOM rung, so the rail and the Inspector cover it: a chip that
/// crossed the panel drew phosphor text over a phosphor list and neither could
/// be read. Every rung is positive - a UI node below the camera's own rung
/// does not draw at all.
pub(crate) const STAGE_LABEL_Z: i32 = 1;

/// The placement callout, over the labels: a verdict about the part in hand
/// outranks a name on a hull behind it. Still under the panels.
pub(crate) const STAGE_VERDICT_Z: i32 = 2;

/// The docked chrome: the rail, the Inspector, the top bar.
pub(crate) const CHROME_Z: i32 = 10;

/// The foot: the status line and the key legend, between the two rails.
///
/// Above the chrome so a line that runs long is readable over the panel edges,
/// and it is the one thing on screen that is always allowed to interrupt.
pub(crate) const FOOT_Z: i32 = 15;

/// The parts gallery: a full-screen browse over the whole editor.
pub(crate) const GALLERY_Z: i32 = 20;

/// The scrim that swallows a click aimed past an open dropdown.
pub(crate) const SCRIM_Z: i32 = 25;

/// The dropdowns themselves, over their own scrim.
pub(crate) const MENU_Z: i32 = 26;

/// Floating windows - the colour picker, a confirm. A window a panel could
/// hide would be a window nobody opened.
pub(crate) const WINDOW_Z: i32 = 30;

/// The hover hint, frontmost: it lives for as long as the pointer rests, and
/// it is the only surface that is allowed over an open window.
pub(crate) const TOOLTIP_Z: i32 = 40;
