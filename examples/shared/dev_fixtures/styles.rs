//! The scaffolding skin style: four placeholder greebles on four placement
//! rules, one per question the plate vocabulary can answer.
//!
//! It is not a look anybody would choose, so it is not base content. It exists
//! to prove the plate pipeline dresses a hull at all, and to stand every rung
//! of the placement vocabulary next to its neighbour on one wall.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The id the scaffolding style is named by.
pub const PLACEHOLDER_STYLE_ID: &str = "placeholder";

/// The scaffolding style.
///
/// The rules are in PRIORITY order because a plate takes at most one piece:
///
/// 1. the mast reads the HIGH GROUND - the crests, tips and studs of a ship's
///    upper surface, the one region that takes a creased plate;
/// 2. the vent reads the PANEL and aligns down it, on the lattice. Its rung
///    carries a density FLOOR, so a hand-built hull the share would have
///    thinned to nothing still wears a row;
/// 3. the block reads ANYWHERE at the rarest rung - one per block of hull -
///    turned OUTBOARD, the only reading a region with no relief can express;
/// 4. the blister reads the POCKET distance, beside the mouth of a fitting. It
///    goes LAST because "near a fitting" is broad: first in the order it
///    carpets 45% of every ship and the other three rules never get a plate.
pub fn placeholder_style() -> ShipStyleConfig {
    ShipStyleConfig {
        id: PLACEHOLDER_STYLE_ID.to_string(),
        name: "Placeholder".to_string(),
        // The built-in plate colours with the top lifted and warmed a little:
        // enough to prove a style dresses the derived plates, and not so much
        // that the greebles stop reading against them.
        palette: StylePalette {
            top: SurfaceFinish {
                color: Color::linear_rgb(0.125, 0.135, 0.160),
                roughness: 0.6,
                metallic: 0.2,
            },
            wall: SurfaceFinish {
                color: Color::linear_rgb(0.070, 0.082, 0.110),
                roughness: 0.7,
                metallic: 0.15,
            },
        },
        fixtures: vec![
            fixture(
                "placeholder_mast",
                8.0,
                Vec3::new(0.12, 0.38, 0.12),
                // HIGH GROUND carries every reading a mast needs - the pointy
                // reliefs, the upper surface, a cell of ship to bolt to, and
                // no seat gate. Spelled out as height filters it landed on
                // nothing: a crest fills an eighth of its cell and a spur tip
                // less.
                FixtureRegion::HighGround,
                FixtureDensity::Regular,
                FixtureOrientation::Free,
            ),
            fixture(
                "placeholder_vent",
                12.0,
                Vec3::new(0.32, 0.04, 0.2),
                // The region takes the corner-cut panel with the flat one, and
                // the seat gate keeps a vent off the ones that actually
                // crease.
                FixtureRegion::Panel,
                FixtureDensity::Dense,
                FixtureOrientation::Along,
            ),
            fixture(
                "placeholder_block",
                20.0,
                Vec3::new(0.22, 0.1, 0.22),
                // `Rare` is the pure per-block form: no share at all, one
                // piece per block of hull, so it reads the same on a 20-plate
                // ship and a 2,000-plate one.
                FixtureRegion::Anywhere,
                FixtureDensity::Rare,
                FixtureOrientation::Outward,
            ),
            fixture(
                "placeholder_blister",
                16.0,
                Vec3::new(0.18, 0.07, 0.18),
                FixtureRegion::NearFitting,
                FixtureDensity::Dense,
                FixtureOrientation::Free,
            ),
        ],
    }
}

/// One fixture rule over the greeble of that id. The models are base bundle
/// resources, named by the path the merge resolves them to.
fn fixture(
    id: &str,
    health: f32,
    collider: Vec3,
    region: FixtureRegion,
    density: FixtureDensity,
    orientation: FixtureOrientation,
) -> StyleFixtureConfig {
    StyleFixtureConfig {
        id: id.to_string(),
        model: AssetRef::from(format!("base/gltf/greebles/{id}.glb#Scene0")),
        health,
        collider,
        placement: FixturePlacement {
            region,
            density,
            orientation,
        },
    }
}
