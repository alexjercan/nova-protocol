//! Build recipes for the base section prototypes: the parts and credits a
//! station charges to rebuild one lost section.
//!
//! One function per recipe row, so the whole table reads in one place and an
//! inline section in a base scenario takes the row of the prototype it copies.
//! Prices are provisional values, not a balance decision. `strip_yield` is
//! the share of each part a strip gives back: 0.5 for most sections, 0.3 for
//! the large drives, whose bells and spars do not survive a teardown whole.
//! The recipe lint holds every row below its own price at strip.

use std::collections::BTreeMap;

use nova_gameplay::prelude::{BuildRecipe, ItemDesignId};

use crate::base_content::items::{
    ACTUATOR_PACK, CAPACITOR_BANK, DOCKING_COLLAR, DRIVE_BELL, FLIGHT_CORE, FRAME_SPAR,
    SECTION_PANEL, TORPEDO_TUBE, TURRET_BARREL, TURRET_BODY,
};

/// The strip share of most recipes.
const STRIP_YIELD: f32 = 0.5;
/// The strip share of the large drives.
const LARGE_DRIVE_STRIP_YIELD: f32 = 0.3;

fn recipe(credits: u32, parts: &[(&str, u32)], strip_yield: f32) -> BuildRecipe {
    BuildRecipe {
        credits,
        parts: parts
            .iter()
            .map(|&(id, count)| (ItemDesignId::from(id), count))
            .collect::<BTreeMap<_, _>>(),
        strip_yield,
    }
}

/// Reinforced, cargo and tank hull.
pub(crate) fn heavy_hull() -> BuildRecipe {
    recipe(160, &[(SECTION_PANEL, 6), (FRAME_SPAR, 2)], STRIP_YIELD)
}

/// Light hull.
pub(crate) fn light_hull() -> BuildRecipe {
    recipe(80, &[(SECTION_PANEL, 2)], STRIP_YIELD)
}

/// Basic thruster.
pub(crate) fn basic_thruster() -> BuildRecipe {
    recipe(60, &[(DRIVE_BELL, 1), (SECTION_PANEL, 2)], STRIP_YIELD)
}

/// Vector thruster.
pub(crate) fn vector_thruster() -> BuildRecipe {
    recipe(
        400,
        &[(DRIVE_BELL, 4), (SECTION_PANEL, 8), (FRAME_SPAR, 4)],
        LARGE_DRIVE_STRIP_YIELD,
    )
}

/// Capital thruster.
pub(crate) fn capital_thruster() -> BuildRecipe {
    recipe(
        1200,
        &[(DRIVE_BELL, 10), (SECTION_PANEL, 20), (FRAME_SPAR, 10)],
        LARGE_DRIVE_STRIP_YIELD,
    )
}

/// Basic controller.
pub(crate) fn controller() -> BuildRecipe {
    recipe(100, &[(FLIGHT_CORE, 1), (SECTION_PANEL, 2)], STRIP_YIELD)
}

/// A PDC mount with one barrel per muzzle, kinetic or pierce alike.
pub(crate) fn pdc_turret(barrels: u32) -> BuildRecipe {
    recipe(
        100,
        &[
            (TURRET_BODY, 1),
            (TURRET_BARREL, barrels),
            (ACTUATOR_PACK, 1),
            (SECTION_PANEL, 2),
        ],
        STRIP_YIELD,
    )
}

/// Either torpedo bay: the tube is the same, only the load differs.
pub(crate) fn torpedo_bay() -> BuildRecipe {
    recipe(
        150,
        &[
            (TORPEDO_TUBE, 1),
            (ACTUATOR_PACK, 1),
            (SECTION_PANEL, 4),
            (FRAME_SPAR, 1),
        ],
        STRIP_YIELD,
    )
}

/// Railgun lance.
pub(crate) fn railgun_lance() -> BuildRecipe {
    recipe(
        300,
        &[(CAPACITOR_BANK, 3), (FRAME_SPAR, 3), (SECTION_PANEL, 6)],
        STRIP_YIELD,
    )
}

/// Docking port.
pub(crate) fn docking_port() -> BuildRecipe {
    recipe(
        80,
        &[(DOCKING_COLLAR, 1), (ACTUATOR_PACK, 1), (SECTION_PANEL, 2)],
        STRIP_YIELD,
    )
}

/// Cargo intake.
pub(crate) fn cargo_intake() -> BuildRecipe {
    recipe(
        60,
        &[(ACTUATOR_PACK, 2), (SECTION_PANEL, 4), (FRAME_SPAR, 1)],
        STRIP_YIELD,
    )
}

/// Mining beam.
pub(crate) fn mining_beam() -> BuildRecipe {
    recipe(
        100,
        &[(CAPACITOR_BANK, 1), (ACTUATOR_PACK, 1), (SECTION_PANEL, 2)],
        STRIP_YIELD,
    )
}
