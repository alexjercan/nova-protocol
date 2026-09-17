//! The editor VIEWPORT's colours: the gizmos drawn over the 3D scene.
//!
//! Not chrome, and deliberately off the UI theme. A socket marker, a grid
//! decade line and a gizmo axis are MEANING - green is "this will land", red is
//! "this is refused", amber is "this is the one" - and a builder reads them
//! against the hull under the pointer, not against the panel beside it. A
//! theme that moved them would make a refusal green on one look and amber on
//! another, which is the one thing a placement overlay must never do.
//!
//! The same exemption [`nova_ui::theme::semantic`] documents for the HUD's
//! reticles, in the crate that owns these overlays.
//!
//! Values are the phosphor palette's, carried over verbatim from when they came
//! from `nova_ui::theme` - the restyle changed nothing in the viewport.

use bevy::prelude::*;

/// A solve that will land, a free socket, an axis that reads as "up".
pub(crate) const GO: Color = Color::srgb_u8(0x36, 0xff, 0x79);

/// The dimmer form of [`GO`]: a grid decade line, a hover mark, a part in hand
/// over empty space.
pub(crate) const GO_DIM: Color = Color::srgb_u8(0x19, 0xa6, 0x4f);

/// The quietest form: a hint nothing is waiting on.
pub(crate) const GO_MUTED: Color = Color::srgb_u8(0x0d, 0x6e, 0x35);

/// "This is the one": the socket the ghost would take, the plumb line, the
/// nose arrow.
pub(crate) const AIMED: Color = Color::srgb_u8(0xff, 0xb8, 0x4a);

/// A refusal, and the X axis.
pub(crate) const NO: Color = Color::srgb_u8(0xff, 0x4e, 0x42);

/// A trigger volume, and the Z axis.
pub(crate) const TRIGGER: Color = Color::srgb_u8(0x36, 0xa3, 0xff);
