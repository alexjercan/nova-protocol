//! Racer semantic parts, prototype catalog entries, and assembly.
//!
//! The racer flies UNARMED: a fast, expensive civilian hull (the yacht the
//! chapters protect). Its turret prototypes stay in the catalog - the ledger
//! and gauntlet webmods mount them - but the base assembly skips them.

use nova_scenario::prelude::*;
use nova_ship::prelude::SectionConfig;

use super::shared::*;
use crate::base_content::assets::BaseContentAssets;

pub(super) const RACER_PARTS: [PartSpec; 9] = [
    part(
        "engine_starboard",
        "racer_engine_starboard",
        "racer/engine_starboard.glb",
        v(0.5, 0.5, 1.5),
        v(-0.09, -0.3, -0.3),
        v(0.4, 0.44189, 0.32567),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "racer_engine_port",
        "racer/engine_port.glb",
        v(-0.5, 0.5, 1.5),
        v(-0.4, -0.3, -0.3),
        v(0.09, 0.44189, 0.32567),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "wing_starboard",
        "racer_wing_starboard",
        "racer/wing_starboard.glb",
        v(1.0, 0.5, 0.0),
        v(-0.59, -0.5, -0.964329),
        v(0.2, 0.5, 1.2),
        180.0,
        PartRole::Hull,
    ),
    part(
        "wing_port",
        "racer_wing_port",
        "racer/wing_port.glb",
        v(-1.0, 0.5, 0.0),
        v(-0.2, -0.5, -0.964329),
        v(0.59, 0.5, 1.2),
        180.0,
        PartRole::Hull,
    ),
    part(
        "nose",
        "racer_nose",
        "racer/nose.glb",
        v(0.0, 0.5, -1.5),
        v(-0.4, -0.5, -0.52567),
        v(0.4, 0.72265, 0.5),
        120.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "racer_tail",
        "racer/tail.glb",
        v(0.0, 1.0, 1.5),
        v(-0.41, -0.8, -0.3),
        v(0.41, 0.5, 0.52567),
        120.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "racer_fuselage",
        "racer/fuselage.glb",
        v(0.0, 0.5, 0.0),
        v(-0.41, -0.5, -1.0),
        v(0.41, 0.9, 1.2),
        240.0,
        PartRole::Controller,
    ),
    module(
        "turret_starboard",
        "racer_turret_starboard",
        v(1.35, 0.4, -0.8),
        130.0,
        PartRole::Turret,
        PartSide::Starboard,
    ),
    module(
        "turret_port",
        "racer_turret_port",
        v(-1.35, 0.4, -0.8),
        130.0,
        PartRole::Turret,
        PartSide::Port,
    ),
];

pub(super) const RACER_EDGES: [(usize, usize); 10] = [
    (6, 4),
    (6, 5),
    (5, 0),
    (5, 1),
    (6, 2),
    (6, 3),
    (2, 0),
    (3, 1),
    (2, 7),
    (3, 8),
];

pub(super) fn prototypes_for(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    prototypes(&RACER_PARTS, &RACER_EDGES, "Racer", assets, true)
}

pub(super) fn sections() -> Vec<SpaceshipSectionConfig> {
    // The meshed seven only: the turret modules (indices 7-8) are
    // catalog-only prototypes, not part of the unarmed assembly.
    ship_sections(&RACER_PARTS[..7], ShipGrade::Player)
}
