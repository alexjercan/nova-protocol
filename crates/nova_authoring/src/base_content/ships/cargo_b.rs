//! CargoB semantic parts, prototype catalog entries, and assembly.

use nova_scenario::prelude::*;
use nova_ship::prelude::SectionConfig;

use super::shared::*;
use crate::base_content::assets::BaseContentAssets;

pub(super) const CARGOB_PARTS: [PartSpec; 9] = [
    part(
        "engine_starboard",
        "cargob_engine_starboard",
        "cargob/engine_starboard.glb",
        v(1.0, 0.5, 2.0),
        v(-0.39, -0.3, -0.5),
        v(0.4, 0.7, 0.5),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "cargob_engine_port",
        "cargob/engine_port.glb",
        v(-1.0, 0.5, 2.0),
        v(-0.4, -0.3, -0.5),
        v(0.39, 0.7, 0.5),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "pod_starboard",
        "cargob_pod_starboard",
        "cargob/pod_starboard.glb",
        v(1.0, 0.5, -0.5),
        v(-0.39, -0.3, -2.0),
        v(0.5, 0.7, 2.0),
        350.0,
        PartRole::Torpedo,
    ),
    part(
        "pod_port",
        "cargob_pod_port",
        "cargob/pod_port.glb",
        v(-1.0, 0.5, -0.5),
        v(-0.5, -0.3, -2.0),
        v(0.39, 0.7, 2.0),
        350.0,
        PartRole::Torpedo,
    ),
    part(
        "nose",
        "cargob_nose",
        "cargob/nose.glb",
        v(0.0, 1.0, -2.0),
        v(-0.61, -0.8, -0.5),
        v(0.61, 0.8, 1.0),
        180.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "cargob_tail",
        "cargob/tail.glb",
        v(0.0, 0.5, 2.0),
        v(-0.61, -0.5, -0.5),
        v(0.61, 0.8, 0.5),
        150.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "cargob_fuselage",
        "cargob/fuselage.glb",
        v(0.0, 1.0, 0.5),
        v(-0.61, -1.0, -1.5),
        v(0.61, 0.8, 1.0),
        300.0,
        PartRole::Controller,
    ),
    module(
        "turret_starboard",
        "cargob_turret_starboard",
        v(1.55, 1.2, 0.0),
        130.0,
        PartRole::Turret,
        PartSide::Starboard,
    ),
    module(
        "turret_port",
        "cargob_turret_port",
        v(-1.55, 1.2, 0.0),
        130.0,
        PartRole::Turret,
        PartSide::Port,
    ),
];

pub(super) const CARGOB_EDGES: [(usize, usize); 8] = [
    (6, 4),
    (6, 5),
    (6, 2),
    (6, 3),
    (2, 0),
    (3, 1),
    (2, 7),
    (3, 8),
];

pub(super) fn prototypes_for(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    prototypes(&CARGOB_PARTS, &CARGOB_EDGES, assets, false)
}

pub(crate) fn sections() -> Vec<SpaceshipSectionConfig> {
    ship_sections(&CARGOB_PARTS, ShipGrade::Player, &[])
}
