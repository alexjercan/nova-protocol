//! CargoA semantic parts, prototype catalog entries, and assembly.
//!
//! The cargoa is the campaign's light fighter - the corvette. Its wide pods
//! carry the two PDC turrets on their forward shoulders (the cargob's mount
//! grammar), so player and raider corvettes read as the armed-hauler lineage
//! the gunship caps.

use nova_scenario::prelude::*;
use nova_ship::prelude::SectionConfig;

use super::shared::*;
use crate::base_content::assets::BaseContentAssets;

pub(crate) const CARGOA_TURRET_IDS: [&str; 2] = ["turret_port", "turret_starboard"];

pub(super) const CARGOA_PARTS: [PartSpec; 9] = [
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
    // Nose cheeks: centred on the nose's port/starboard faces (the nose spans
    // x +-0.8, y 0.2..1.4, z -2.45..-1.15) and sitting FLUSH against them -
    // the 0.15 module half-width puts the inner face exactly on the hull at
    // +-0.95, so the guns read bolted on with their bases clear of the
    // bodywork. Forward of the fuselage and the pods, so the front arcs are
    // clear; the shared joint tree's -10 degree depression floor keeps a
    // barrel from swinging back into the nose it stands on.
    module(
        "turret_starboard",
        "cargoa_turret_starboard",
        v(0.95, 0.8, -1.8),
        130.0,
        PartRole::Turret,
        PartSide::Starboard,
    ),
    module(
        "turret_port",
        "cargoa_turret_port",
        v(-0.95, 0.8, -1.8),
        130.0,
        PartRole::Turret,
        PartSide::Port,
    ),
];

pub(super) const CARGOA_EDGES: [(usize, usize); 8] = [
    (6, 4),
    (6, 5),
    (6, 2),
    (6, 3),
    (2, 0),
    (3, 1),
    // The turrets mate to the NOSE (4), not the pods: they sit on its cheeks.
    (4, 7),
    (4, 8),
];

pub(super) fn prototypes_for(assets: &BaseContentAssets) -> Vec<SectionConfig> {
    prototypes(&CARGOA_PARTS, &CARGOA_EDGES, "CargoA", assets, true)
}

pub(crate) fn sections(
    grade: ShipGrade,
    controller_modifications: Vec<SectionModification>,
) -> Vec<SpaceshipSectionConfig> {
    ship_sections(&CARGOA_PARTS, grade, &controller_modifications)
}
