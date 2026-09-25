//! The docking port: a sealed hatch and the sleeve that reaches across it.
//!
//! Lock the ship to dock with, close to within a cell of its port with your own
//! port facing it, and DOCK clamps the two hulls together. The pair flies as
//! one body: the other ship flies it until HELM takes it, and DOCK again from
//! either side lets go.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// A hatch, a sleeve and the machinery that drives it, all behind hull plate.
/// Between the exposed drive and the command core: a port is soft enough that
/// shooting it is a real way to break a dock, and not so soft that bumping one
/// in traffic takes it off.
const DOCKING_BASE_HEALTH: f32 = 90.0;

/// The sleeve's fixed travel along the port face, in cell units. It is the
/// generator's `DOCK_EXTENSION`: the art is built to exactly this reach, and
/// the two numbers have to move together.
const DOCK_TUBE_TRAVEL: f32 = 0.5;

/// The port's sockets: every face but the one it docks through.
///
/// The port face (-Z) carries none, for the same reason a bay's muzzle does
/// not: a socket there is an invitation to bolt a plate over the hatch, and
/// the capture is measured from that face.
fn dock_link_points() -> Vec<LinkPoint> {
    unit_cube_link_points()
        .into_iter()
        .filter(|point| point.id != "negative_z")
        .collect()
}

/// The port's sleeve track: the `dock_tube` node modelled into
/// `dock_flush.glb`, slid 0.5 out along the port face.
///
/// The travel is FIXED, never fitted to the gap the ships met at, so two
/// ports always reach the same distance and at any smaller gap the sleeves
/// overlap on purpose. It is art: the mechanic is the joint, and the sleeve
/// never grows a collider (`gen-section-parts.py`, `_check_dock`).
///
/// It extends slower than it retracts: reaching across is a service motion,
/// and the release is not.
fn dock_tube_track() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::DockTube,
        node_prefix: "dock_tube".to_string(),
        motion: SectionAnimationMotion::Translate {
            offset: Vec3::NEG_Z * DOCK_TUBE_TRAVEL,
        },
        open_seconds: 1.2,
        close_seconds: 0.8,
    }]
}

/// The port prototype.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![SectionConfig {
        base: BaseSectionConfig {
            id: DOCKING_PORT_SECTION_ID.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: "Docking Port Section".to_string(),
            description: "A sealed-hatch docking port. Lock the ship you \
                              want to dock with, close to within a cell of its \
                              port with your own port facing it, and DOCK \
                              clamps the two hulls together; the sleeves reach \
                              across once the clamp holds. The pair flies as \
                              one body: the other ship flies it until HELM \
                              takes it, and DOCK again from either side lets \
                              go."
            .to_string(),
            // Light machinery behind a hatch: softer than the command core
            // it is bolted beside, tougher than exposed propulsion.
            health: DOCKING_BASE_HEALTH,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: None,
            link_points: dock_link_points(),
            hide_in_editor: false,
            animations: dock_tube_track(),
        },
        kind: SectionKind::Docking(DockingSectionConfig {
            render_mesh: Some(meshes.dock_flush.clone()),
            render_mesh_transform: None,
            // One cell of reach between the two RETRACTED faces, which is
            // exactly what two 0.5 sleeves bridge.
            capture_distance: Meters(10.0),
            // Roll is free (the port is round); this is the only angle
            // that matters, and 15 degrees is a hand-flown approach.
            capture_angle: 15.0,
            // Both hulls must be nearly station-keeping on each other. A
            // joint built at a real closing rate would snap the two ships
            // to a stop, and nothing here is allowed to do that.
            maximum_relative_speed: MetersPerSecond(5.0),
            maximum_relative_angular_speed: 5.0,
        }),
    }]
}
