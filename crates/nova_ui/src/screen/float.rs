//! Where a floating UI node goes when something in the world says where.

use bevy::prelude::*;

/// How a floating node hangs off its anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hang {
    /// Which point of the node lands on the anchor, as a fraction of the
    /// node's own size: `(0.5, 1.0)` is its bottom edge's middle, which puts
    /// the node ABOVE the anchor and centred on it.
    pub align: Vec2,
    /// Clearance between that point and the anchor, in logical pixels.
    pub gap: Vec2,
}

impl Hang {
    /// Centred on the anchor and `gap` above it.
    pub fn above(gap: f32) -> Self {
        Self {
            align: Vec2::new(0.5, 1.0),
            gap: Vec2::new(0.0, -gap),
        }
    }

    /// Centred on the anchor and `gap` below it.
    pub fn below(gap: f32) -> Self {
        Self {
            align: Vec2::new(0.5, 0.0),
            gap: Vec2::new(0.0, gap),
        }
    }
}

/// The top-left corner a node hanging off `anchor` is written at, in the units
/// [`Node`] is written in.
///
/// The one place the conversion happens. `anchor` is a viewport point, which
/// [`Camera::world_to_viewport`] answers in LOGICAL pixels, and so is `Node`'s
/// `left`/`top`; [`ComputedNode::size`] is PHYSICAL. They are the same number
/// at scale factor 1, which is why three copies of this arithmetic read fine on
/// the machine they were written on and put every label half its own size out
/// of place on a HiDPI screen.
///
/// The answer is kept inside `viewport`, so a node anchored near an edge slides
/// along it rather than hanging off it. A node too big for the viewport pins to
/// the top-left, which shows its start rather than its middle.
pub fn hang_at(anchor: Vec2, hang: Hang, node: &ComputedNode, viewport: Vec2) -> Vec2 {
    let size = node.size() * node.inverse_scale_factor();
    let corner = anchor - size * hang.align + hang.gap;
    corner.clamp(Vec2::ZERO, (viewport - size).max(Vec2::ZERO))
}
