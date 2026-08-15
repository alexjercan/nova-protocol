//! Built-in SKIN STYLE content: the looks a ship's derived cladding can wear.
//!
//! A style is content like a section is content, so this file is a builder and
//! not a table of constants - `content gen` serializes it into
//! `assets/base/styles/base.content.ron` and a mod overlays it by id.
//!
//! What is here today is SCAFFOLDING, deliberately. `placeholder` wires the four
//! placeholder greebles to four rules chosen to exercise the whole plate
//! vocabulary rather than to look like anything, and the authored kits replace
//! it (task `20260815-225748`, Phase B).

use bevy::prelude::{Color, Vec3};
use nova_ship::prelude::{
    PlateFacing, PlateRelief, ScatterRule, ShellSurface, ShipStyleConfig, StyleFixtureConfig,
    StyleSurfaceConfig,
};

use super::assets::BaseContentAssets;

/// The id the placeholder style is named by. The editor and the generator
/// reference it, so a rename is one constant.
pub const PLACEHOLDER_STYLE_ID: &str = "placeholder";

/// Every built-in style, in stable generated-content order.
pub(crate) fn style_catalog(assets: &BaseContentAssets) -> Vec<ShipStyleConfig> {
    vec![placeholder_style(assets)]
}

/// The scaffolding style: four placeholder greebles on four rules, one per
/// question the plate vocabulary can answer.
///
/// Each rule is here to DEMONSTRATE a reading, and they are in priority order
/// because a plate takes at most one piece:
///
/// 1. the mast reads RELIEF, FACING, DEPTH and HEIGHT together - only on the
///    high ground of a ship's upper surface, and only where there is ship under
///    it to bolt to;
/// 2. the vent reads the RUN and aligns to it, on a stride, which is the
///    grid-occupancy claim the research settled on instead of blue noise;
/// 3. the block reads the BORDER - trim, only ever at the end of a run, of
///    whatever kind the run is;
/// 4. the blister reads the POCKET distance - beside the mouth of a fitting,
///    which is the "weight decoration toward link points" finding. It goes LAST
///    because on a hull as full of fittings as the generator draws, "near a
///    fitting" is nearly everywhere: first in the order it carpeted 45% of every
///    ship and the other three rules never got a plate.
fn placeholder_style(assets: &BaseContentAssets) -> ShipStyleConfig {
    ShipStyleConfig {
        id: PLACEHOLDER_STYLE_ID.to_string(),
        name: "Placeholder".to_string(),
        // A restatement of the built-in plate colours with the top lifted and
        // warmed a little: enough to prove a style really does dress the
        // derived plates, and not so much that the greebles stop reading
        // against them.
        surfaces: vec![
            StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.125, 0.135, 0.160),
                roughness: 0.6,
                metallic: 0.2,
            },
            StyleSurfaceConfig {
                surface: ShellSurface::Wall,
                color: Color::linear_rgb(0.070, 0.082, 0.110),
                roughness: 0.7,
                metallic: 0.15,
            },
        ],
        // In PRIORITY order, most specific first - a plate takes one piece.
        // Tuned against what a real hull actually offers, measured on the wfc
        // row: about four fifths of every ship comes out RIM, a seventh STEP
        // and under a seventh flat, with ridges rare and studs absent. A rule
        // written for flat panels alone lands on almost nothing.
        fixtures: vec![
            StyleFixtureConfig {
                id: "placeholder_mast".to_string(),
                model: assets.greeble_mast.clone(),
                health: 8.0,
                density: 0.05,
                collider: Vec3::new(0.12, 0.38, 0.12),
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Ridge, PlateRelief::Peak, PlateRelief::Step],
                    facing: PlateFacing::Up,
                    min_depth: 2,
                    min_height: 2,
                    chance: 0.3,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_vent".to_string(),
                model: assets.greeble_vent.clone(),
                health: 12.0,
                density: 0.15,
                collider: Vec3::new(0.32, 0.04, 0.2),
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat],
                    min_run: 2,
                    stride: 2,
                    chance: 0.8,
                    align: true,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_block".to_string(),
                model: assets.greeble_block.clone(),
                health: 20.0,
                density: 0.2,
                collider: Vec3::new(0.22, 0.1, 0.22),
                // No stride. A lattice is for making a ROW, which is what the
                // vent above wants; the END of a run is already sparse and
                // already structured, and striding it as well left a small
                // build with a single greeble on it.
                scatter: ScatterRule {
                    max_border: Some(0),
                    min_height: 2,
                    chance: 0.6,
                    align: true,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_blister".to_string(),
                model: assets.greeble_blister.clone(),
                health: 16.0,
                density: 0.2,
                collider: Vec3::new(0.18, 0.07, 0.18),
                scatter: ScatterRule {
                    near_fitting: Some(1),
                    stride: 2,
                    chance: 0.5,
                    ..Default::default()
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every model a style names is one of the greebles the base bundle ships,
    /// authored through the scheme like every other base asset ref.
    ///
    /// The runtime gate and the repo lint both check the ref against the
    /// manifest; this catches the same mistake in the BUILDER, where the fix
    /// belongs.
    #[test]
    fn every_placeholder_fixture_names_a_shipped_greeble() {
        let assets = BaseContentAssets::from_paths();
        for style in style_catalog(&assets) {
            assert!(!style.fixtures.is_empty(), "{} scatters nothing", style.id);
            for fixture in &style.fixtures {
                let path = fixture.model.path().unwrap_or_default().to_string();
                assert!(
                    path.starts_with("self://gltf/greebles/"),
                    "{} names '{path}', which is not a base greeble",
                    fixture.id,
                );
                assert!(
                    fixture.collider.min_element() > 0.0,
                    "{} carries no collider, so a round would pass through it",
                    fixture.id,
                );
                assert!(fixture.health > 0.0, "{} cannot be shot off", fixture.id);
            }
        }
    }

    /// The placeholder style's rules cover the whole vocabulary, which is the
    /// only reason it exists: the checkpoint render has to be able to show
    /// whether the readings can carry a look.
    #[test]
    fn the_placeholder_style_exercises_every_reading() {
        let style = placeholder_style(&BaseContentAssets::from_paths());
        let rules: Vec<&ScatterRule> = style.fixtures.iter().map(|f| &f.scatter).collect();
        assert!(
            rules.iter().any(|rule| rule.near_fitting.is_some()),
            "nothing reads the pocket distance",
        );
        assert!(
            rules.iter().any(|rule| !rule.relief.is_empty()),
            "nothing reads the relief",
        );
        assert!(
            rules.iter().any(|rule| !rule.facing.is_any()),
            "nothing reads the facing",
        );
        assert!(
            rules.iter().any(|rule| rule.min_depth > 0),
            "nothing reads the support depth",
        );
        assert!(
            rules.iter().any(|rule| rule.min_height > 0),
            "nothing reads how much of its cell a plate fills",
        );
        assert!(
            rules.iter().any(|rule| rule.min_run > 0 && rule.align),
            "nothing reads the run or aligns to it",
        );
        assert!(
            rules.iter().any(|rule| rule.max_border == Some(0)),
            "nothing reads the border, so nothing is trim",
        );
        assert!(
            rules.iter().any(|rule| rule.stride > 1),
            "nothing claims a lattice, which is what keeps a look off confetti",
        );
    }
}
