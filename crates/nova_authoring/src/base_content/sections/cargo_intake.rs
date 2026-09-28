//! The cargo intake: a 2x2 hold mouth behind an accordion door.
//!
//! Fly slowly past a drifting canister with the door facing it, and the door
//! folds open and the intake takes the whole canister into the hold, if the
//! hold has room. The Inventory pane's Jettison drops a canister out through
//! the same door.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// The intake prototype id. Only the block fleet mounts it by name.
pub(crate) const CARGO_INTAKE_SECTION_ID: &str = "cargo_intake_section";

/// Hold machinery behind a door, as soft as a docking port: shooting the door
/// is a real way to stop a pickup.
const CARGO_INTAKE_BASE_HEALTH: f32 = 90.0;

/// The intake's box: two cells across, two high, one deep, with the door on
/// its local -Z face.
const INTAKE_CELLS: Vec3 = Vec3::new(2.0, 2.0, 1.0);

/// The slat box `intake_accordion_2x2x1.json` builds each `intake_slat_`
/// node from: its width across the door and its thickness. The Fold track
/// reads the same two numbers, so the art and the track move together.
const SLAT_WIDTH: f32 = 0.1458;
const SLAT_THICKNESS: f32 = 0.02;

/// The intake's sockets: one per cell on every closed face, none on the door.
///
/// A socket on the -Z face would invite a plate over the door, and the
/// volumes are measured from that face.
fn intake_link_points() -> Vec<LinkPoint> {
    let half = INTAKE_CELLS * 0.5;
    let mut points = Vec::new();
    for (face, normal) in [("x", Vec3::X), ("y", Vec3::Y)] {
        let across = if normal == Vec3::X { Vec3::Y } else { Vec3::X };
        for (sign, side) in [(1.0, "positive"), (-1.0, "negative")] {
            for (offset, cell) in [(-0.5, "a"), (0.5, "b")] {
                points.push(LinkPoint {
                    id: format!("{side}_{face}_{cell}"),
                    position: normal * sign * half.dot(normal) + across * offset,
                    normal: normal * sign,
                });
            }
        }
    }
    for (x, y) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
        points.push(LinkPoint {
            id: format!("positive_z_{}_{}", cell_name(x), cell_name(y)),
            position: Vec3::new(x, y, half.z),
            normal: Vec3::Z,
        });
    }
    points
}

fn cell_name(offset: f32) -> &'static str {
    if offset < 0.0 {
        "a"
    } else {
        "b"
    }
}

/// The door track: the twelve `intake_slat_` nodes of
/// `intake_accordion_2x2x1.glb` fold 80 degrees aside into two pockets.
fn intake_door_track() -> Vec<SectionAnimation> {
    vec![SectionAnimation {
        cue: SectionAnimationCue::IntakeDoor,
        node_prefix: "intake_slat_".to_string(),
        motion: SectionAnimationMotion::Fold {
            degrees: 80.0,
            slat_width: SLAT_WIDTH,
            slat_thickness: SLAT_THICKNESS,
        },
        open_seconds: 1.2,
        close_seconds: 1.2,
    }]
}

/// The intake prototype.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![SectionConfig {
        base: BaseSectionConfig {
            id: CARGO_INTAKE_SECTION_ID.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: "Cargo Intake Section".to_string(),
            description: "A hold mouth behind an accordion door. The door \
                          folds open for a canister within 40 m of it. The \
                          intake takes a canister moving under 5 m/s, and \
                          not away from the open door, whole, if it fits the \
                          opening and the hold has room. It never takes a \
                          canister that touches the ship. Jettison in the \
                          Inventory pane drops a canister out through the \
                          same door."
                .to_string(),
            health: CARGO_INTAKE_BASE_HEALTH,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: Some(SectionCollider::Cuboid { size: INTAKE_CELLS }),
            link_points: intake_link_points(),
            hide_in_editor: false,
            animations: intake_door_track(),
        },
        kind: SectionKind::CargoIntake(CargoIntakeSectionConfig {
            render_mesh: meshes.cargo_intake.clone(),
            render_mesh_transform: None,
            canister_mesh: meshes.cargo_canister.clone(),
            door_sound: meshes.torpedo_door_sound.clone(),
            eject_sound: meshes.cargo_eject_sound.clone(),
            take_sound: meshes.cargo_take_sound.clone(),
            detection_range: Meters(40.0),
            capture_gap: Meters(1.0),
            // The clear opening between the face ring and the folded slat
            // stacks of intake_accordion_2x2x1.json: 14.17 m by 16.0 m.
            aperture_width: Meters(14.1),
            aperture_height: Meters(16.0),
            maximum_capture_speed: MetersPerSecond(5.0),
            eject_speed: MetersPerSecond(3.0),
        }),
    }]
}
