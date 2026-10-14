//! The scaffolding skin style: four placeholder greebles under a small greeble
//! rule set.
//!
//! It is not a look anybody would choose, so it is not base content. It exists
//! to prove the plate pipeline dresses a hull at all.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// The id the scaffolding style is named by.
pub const PLACEHOLDER_STYLE_ID: &str = "placeholder";

/// The scaffolding style.
///
/// The mast touches only empty space, the vent runs as a line, and the block
/// and the blister fill between.
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
            fixture("placeholder_mast", 8.0, Vec3::new(0.12, 0.38, 0.12)),
            fixture("placeholder_vent", 12.0, Vec3::new(0.32, 0.04, 0.2)),
            fixture("placeholder_block", 20.0, Vec3::new(0.22, 0.1, 0.22)),
            fixture("placeholder_blister", 16.0, Vec3::new(0.18, 0.07, 0.18)),
        ],
        greeble_rules: GreebleRules {
            link_types: vec![
                link("any", &["any", "empty"]),
                link("empty", &["any", "empty", "spaced"]),
                link("spaced", &["empty"]),
                link("line", &["line"]),
            ],
            greebles: vec![
                greeble("empty", None, 4.0, ["empty"; 4]),
                greeble(
                    "placeholder_mast",
                    Some("placeholder_mast"),
                    0.5,
                    ["spaced"; 4],
                ),
                greeble(
                    "placeholder_vent",
                    Some("placeholder_vent"),
                    1.0,
                    ["line", "any", "line", "any"],
                ),
                GreebleConfig {
                    rotate: GreebleRotateType::All,
                    ..greeble(
                        "placeholder_vent_end",
                        Some("placeholder_vent"),
                        0.4,
                        ["line", "any", "any", "any"],
                    )
                },
                greeble(
                    "placeholder_block",
                    Some("placeholder_block"),
                    0.6,
                    ["any"; 4],
                ),
                greeble(
                    "placeholder_blister",
                    Some("placeholder_blister"),
                    0.8,
                    ["any"; 4],
                ),
            ],
        },
    }
}

/// A link type facing `connects`.
fn link(id: &str, connects: &[&str]) -> GreebleLinkConfig {
    GreebleLinkConfig {
        id: id.into(),
        connects: connects.iter().map(|other| (*other).into()).collect(),
    }
}

/// A greeble with link types `[up, right, down, left]`, placed at one turn.
fn greeble(
    id: &str,
    fixture: Option<&str>,
    weight: f32,
    [up, right, down, left]: [&str; 4],
) -> GreebleConfig {
    GreebleConfig {
        id: id.into(),
        fixture: fixture.map(str::to_string),
        weight,
        rotate: GreebleRotateType::None,
        link_points: GreebleLinkPoints {
            up: up.into(),
            right: right.into(),
            down: down.into(),
            left: left.into(),
        },
        near: Vec::new(),
        max_count: None,
    }
}

/// One fixture over the greeble model of that id. The models are base bundle
/// resources, named by the path the merge resolves them to.
fn fixture(id: &str, health: f32, collider: Vec3) -> StyleFixtureConfig {
    StyleFixtureConfig {
        id: id.to_string(),
        model: AssetRef::from(format!("base/gltf/greebles/{id}.glb#Scene0")),
        health,
        collider,
    }
}
