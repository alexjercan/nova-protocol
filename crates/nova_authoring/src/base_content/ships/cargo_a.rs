//! CargoA semantic parts, prototype catalog entries, and assembly.

use nova_scenario::prelude::*;
use nova_ship::prelude::SectionConfig;

use super::shared::*;
use crate::base_content::assets::BaseContentAssets;

pub(super) const CARGOA_PARTS: [PartSpec; 7] = [
    part(
        "engine_starboard",
        "cargoa_engine_starboard",
        "cargoa/engine_starboard.glb",
        v(1.0, 0.5, 2.0),
        v(-0.19, -0.2975, -0.5),
        v(0.6, 0.4975, 0.45),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "cargoa_engine_port",
        "cargoa/engine_port.glb",
        v(-1.0, 0.5, 2.0),
        v(-0.6, -0.2975, -0.5),
        v(0.19, 0.4975, 0.45),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "pod_starboard",
        "cargoa_pod_starboard",
        "cargoa/pod_starboard.glb",
        v(1.0, 0.5, 0.5),
        v(-0.19, -0.3, -1.05),
        v(0.6, 0.7, 1.0),
        350.0,
        PartRole::Hull,
    ),
    part(
        "pod_port",
        "cargoa_pod_port",
        "cargoa/pod_port.glb",
        v(-1.0, 0.5, 0.5),
        v(-0.6, -0.3, -1.05),
        v(0.19, 0.7, 1.0),
        350.0,
        PartRole::Hull,
    ),
    part(
        "nose",
        "cargoa_nose",
        "cargoa/nose.glb",
        v(0.0, 1.0, -2.0),
        v(-0.8, -0.8, -0.45),
        v(0.8, 0.4, 0.85),
        180.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "cargoa_tail",
        "cargoa/tail.glb",
        v(0.0, 0.5, 2.0),
        v(-0.81, -0.5, -0.5),
        v(0.81, 0.675, 0.45),
        150.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "cargoa_fuselage",
        "cargoa/fuselage.glb",
        v(0.0, 1.0, 0.0),
        v(-0.81, -1.0, -1.15),
        v(0.81, 0.6, 1.5),
        350.0,
        PartRole::Controller,
    ),
];

pub(super) const CARGOA_EDGES: [(usize, usize); 6] =
    [(6, 4), (6, 5), (6, 2), (6, 3), (2, 0), (3, 1)];

pub(super) fn prototypes_for(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    prototypes(&CARGOA_PARTS, &CARGOA_EDGES, assets, false)
}

pub(crate) fn sections() -> Vec<SpaceshipSectionConfig> {
    ship_sections(&CARGOA_PARTS, ShipGrade::Player, &[])
}
