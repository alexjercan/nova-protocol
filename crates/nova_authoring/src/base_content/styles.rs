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
    PlateFacing, PlateRelief, ScatterAlign, ScatterRule, ShellSurface, ShipStyleConfig,
    StyleFixtureConfig, StyleSurfaceConfig,
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
///    grid-occupancy claim the research settled on instead of blue noise. It
///    also carries the density FLOOR, so a hand-built hull that the stride and
///    the share between them would have thinned to nothing still wears a row;
/// 3. the block reads the BORDER and the FALL - trim at the end of a run, and
///    on the straight edge of a hull it is turned OUTBOARD, which the run alone
///    could not say;
/// 4. the blister reads the POCKET distance - beside the mouth of a fitting,
///    which is the "weight decoration toward link points" finding. It goes LAST
///    because "near a fitting" is broad even now that the distance is counted in
///    face steps: first in the order, on the ring it used to be measured over,
///    it carpeted 45% of every ship and the other three rules never got a plate.
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
        // row: about four fifths of every ship FALLS AWAY somewhere, a seventh
        // is a STEP and under a seventh is flat, with ridges rare and studs
        // absent. A rule written for flat panels alone lands on almost nothing,
        // which is why the trim below reads the border of any relief and the
        // fairing reads the falling plate rather than the flat.
        fixtures: vec![
            StyleFixtureConfig {
                id: "placeholder_mast".to_string(),
                model: assets.greeble_mast.clone(),
                health: 8.0,
                density: 0.05,
                collider: Vec3::new(0.12, 0.38, 0.12),
                // The HIGH GROUND, which now includes the SPUR - the tips and
                // outer corners a hull ends at. Before the falling plate split
                // three ways there was no way to say that: a tip and the middle
                // of a flank were one relief.
                //
                // `min_height` is 1 and not 2, measured: a ridge fills an eighth
                // of its cell and a spur less, so a mast asking for half a cell
                // of plate under it was asking for something no pointy plate
                // is - the rule read as narrow and landed on NOTHING.
                scatter: ScatterRule {
                    relief: vec![
                        PlateRelief::Ridge,
                        PlateRelief::Peak,
                        PlateRelief::Step,
                        PlateRelief::Spur,
                    ],
                    facing: PlateFacing::Up,
                    min_depth: 2,
                    min_height: 1,
                    chance: 0.25,
                    ..Default::default()
                },
            },
            StyleFixtureConfig {
                id: "placeholder_vent".to_string(),
                model: assets.greeble_vent.clone(),
                health: 12.0,
                density: 0.15,
                collider: Vec3::new(0.32, 0.04, 0.2),
                // FLAT AND BEVEL, not flat alone. A bevel is a panel with one
                // corner taken off - the same place to a piece this size, and
                // there are more of them on a generated hull than there are
                // flat plates. `patch` is the floor under the stride and the
                // share: without it this rule is a field of vents on a big hull
                // and nothing at all on a small one.
                scatter: ScatterRule {
                    relief: vec![PlateRelief::Flat, PlateRelief::Bevel],
                    min_run: 2,
                    stride: 2,
                    chance: 0.8,
                    patch: 3,
                    align: ScatterAlign::Run,
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
                //
                // Turned OUTBOARD rather than down the run: at the end of a run
                // is exactly where a hull turns a corner, and the fall is the
                // only reading that says which side of it is off the ship. A
                // plate that does not fall one way is left square, which is the
                // rest of the hull.
                //
                // The PURE per-patch form: no share at all, and one piece per
                // block of hull. This is the rule that has to carry a small
                // build, because it is the only one a hand-built hull passes -
                // the vent wants flat plate and there is none - and a share
                // tuned on the row put ONE piece on a 20-plate ship. Stated as a
                // density instead, it reads the same at both sizes.
                scatter: ScatterRule {
                    // At the end of a RUN, and a run is two cells or more. The
                    // border alone reads as specific and is not: on a hull as
                    // broken up as the generator draws, nearly every plate is at
                    // the end of its own one-cell run, so the rule admitted 126
                    // of 132 plates - the `near_fitting` trap wearing another
                    // field's name.
                    min_run: 2,
                    max_border: Some(0),
                    // A quarter cell, not half. Measured on a hand-built hull:
                    // it is nearly all ridges and spurs, which fill an eighth
                    // of their cell, so a trim piece asking for half a cell of
                    // plate is the vent's mistake again in another rule.
                    min_height: 1,
                    chance: 0.0,
                    patch: 3,
                    align: ScatterAlign::Outward,
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
            rules
                .iter()
                .any(|rule| rule.min_run > 0 && rule.align == ScatterAlign::Run),
            "nothing reads the run or aligns to it",
        );
        assert!(
            rules.iter().any(|rule| rule.align == ScatterAlign::Outward),
            "nothing turns a piece off the ship, so nothing reads the fall",
        );
        assert!(
            rules
                .iter()
                .any(|rule| rule.relief.iter().any(|relief| matches!(
                    relief,
                    PlateRelief::Bevel | PlateRelief::Brink | PlateRelief::Spur
                ))),
            "nothing stands on the falling plate, which is four fifths of a hull",
        );
        assert!(
            rules.iter().any(|rule| rule.max_border == Some(0)),
            "nothing reads the border, so nothing is trim",
        );
        assert!(
            rules.iter().any(|rule| rule.stride > 1),
            "nothing claims a lattice, which is what keeps a look off confetti",
        );
        assert!(
            rules.iter().any(|rule| rule.patch > 0),
            "nothing normalises its density, so a small build wears almost none",
        );
    }
}
