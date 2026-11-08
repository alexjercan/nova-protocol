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
//!
//! The script canvas's block colours live here for the same reason: a block's
//! colour is what role it plays, and a theme that moved one would change what
//! a builder has learned to look for.

use bevy::prelude::*;

use crate::event::BlockRoleType;

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

/// What a block of `role` is painted with, on the canvas and on its palette
/// heading alike. White text reads on every one.
pub(crate) fn block_colour(role: BlockRoleType) -> Color {
    match role {
        BlockRoleType::Event => Color::srgb_u8(0xb0, 0x78, 0x12),
        BlockRoleType::Condition => Color::srgb_u8(0x17, 0x85, 0x9c),
        BlockRoleType::Action => Color::srgb_u8(0x2f, 0x6f, 0xc9),
        BlockRoleType::Flow => Color::srgb_u8(0xb4, 0x4e, 0x2e),
        BlockRoleType::Value => Color::srgb_u8(0x4e, 0x8c, 0x28),
    }
}

/// `colour`, lifted toward white by `amount`: a block nested in its own kind
/// has to stand off the block holding it.
pub(crate) fn block_lifted(colour: Color, amount: f32) -> Color {
    colour.mix(&Color::WHITE, amount)
}

/// The bare canvas under the blocks.
pub(crate) const BLOCK_GROUND: Color = Color::srgb_u8(0x14, 0x18, 0x1f);

/// An empty socket: the dark hole a block fits into.
pub(crate) const BLOCK_SOCKET: Color = Color::srgb_u8(0x1c, 0x22, 0x2c);

/// The inset an inline field sits in on a coloured block.
pub(crate) const BLOCK_CHIP: Color = Color::srgba(0.0, 0.0, 0.0, 0.32);

/// A typed literal: the socket's own field.
pub(crate) const BLOCK_LITERAL: Color = Color::srgb_u8(0xf1, 0xf3, 0xf6);

/// The text on a typed literal.
pub(crate) const BLOCK_LITERAL_TEXT: Color = Color::srgb_u8(0x1b, 0x24, 0x30);

/// Text on a block.
pub(crate) const BLOCK_TEXT: Color = Color::WHITE;

/// The small word naming a field on a block.
pub(crate) const BLOCK_TEXT_DIM: Color = Color::srgba(1.0, 1.0, 1.0, 0.72);

/// A slot that will take the block in hand.
pub(crate) const BLOCK_SLOT_LIT: Color = Color::srgb_u8(0xff, 0xd8, 0x6a);

/// The gap the nearest slot opens: where the block will land.
pub(crate) const BLOCK_SLOT_GAP: Color = Color::srgba(1.0, 0.85, 0.42, 0.42);

/// The marked block's outline.
pub(crate) const BLOCK_MARKED: Color = Color::WHITE;

/// A block the lint or the gap check has something to say about.
pub(crate) const BLOCK_FAULT: Color = Color::srgb_u8(0xff, 0x5a, 0x5a);
