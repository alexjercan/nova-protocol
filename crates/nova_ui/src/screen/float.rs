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
///
/// `None` when the ANCHOR itself is outside the viewport. Clamping that would
/// pin a label to the border for something that is not on screen, and
/// [`Camera::world_to_viewport`] does not answer it: it errors behind the eye
/// and past the far plane, and returns out-of-range coordinates for a point in
/// front of the camera but beside the frame. A caller that hides only on `Err`
/// is a caller that has never been told.
pub fn hang_at(anchor: Vec2, hang: Hang, node: &ComputedNode, viewport: Vec2) -> Option<Vec2> {
    if anchor.cmplt(Vec2::ZERO).any() || anchor.cmpgt(viewport).any() {
        return None;
    }
    let size = node.size() * node.inverse_scale_factor();
    let corner = anchor - size * hang.align + hang.gap;
    Some(corner.clamp(Vec2::ZERO, (viewport - size).max(Vec2::ZERO)))
}

/// `anchor`, moved until it is clear of every spot already `standing`.
///
/// Several things a hand's width apart on a hull project to the same few
/// pixels, and a pile of labels names nothing. Beside [`hang_at`] because both
/// callers that place a label over the world need both halves: one of them had
/// this and the other piled up.
///
/// UP first, because a label hangs above its anchor either way and the ship is
/// usually below: a pile pushed downward walks over the hull it labels. DOWN
/// once the column runs out of viewport, because a lift that leaves the screen
/// is a label [`hang_at`] then puts on the top row - every one of them on the
/// same row, which is the single outcome this exists to prevent.
///
/// With both directions full it gives up and overlaps, which is the honest
/// answer when there is nowhere left to stand.
pub fn clear_of(anchor: Vec2, clearance: Vec2, viewport: Vec2, standing: &mut Vec<Vec2>) -> Vec2 {
    let collides = |spot: Vec2, standing: &[Vec2]| {
        standing.iter().any(|held| {
            (held.x - spot.x).abs() < clearance.x && (held.y - spot.y).abs() < clearance.y
        })
    };
    let mut spot = anchor;
    while collides(spot, standing) && spot.y - clearance.y >= 0.0 {
        spot.y -= clearance.y;
    }
    if collides(spot, standing) {
        spot = anchor;
        while collides(spot, standing) && spot.y + clearance.y <= viewport.y {
            spot.y += clearance.y;
        }
    }
    standing.push(spot);
    spot
}
