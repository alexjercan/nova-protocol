//! The cargo intake: a 3x2 hold mouth behind an accordion door.
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

/// The intake's box: three cells across, two high, one deep, with the door on
/// its local -Z face.
const INTAKE_CELLS: Vec3 = Vec3::new(3.0, 2.0, 1.0);

/// The slat box `intake_accordion_3x2x1.json` builds each `intake_slat_`
/// node from: its width across the door and its thickness. The Fold track
/// reads the same two numbers, so the art and the track move together.
const SLAT_WIDTH: f32 = 0.2292;
const SLAT_THICKNESS: f32 = 0.02;

/// The intake's sockets: one per cell on every closed face, none on the door.
///
/// A socket on the -Z face would invite a plate over the door, and the
/// volumes are measured from that face.
fn intake_link_points() -> Vec<LinkPoint> {
    let half = INTAKE_CELLS * 0.5;
    // Cell centres, lettered from the negative end: three across, two up.
    let across = [("a", -1.0), ("b", 0.0), ("c", 1.0)];
    let up = [("a", -0.5), ("b", 0.5)];
    let mut points = Vec::new();
    for (face, normal, along, cells) in [
        ("x", Vec3::X, Vec3::Y, &up[..]),
        ("y", Vec3::Y, Vec3::X, &across[..]),
    ] {
        for (sign, side) in [(1.0, "positive"), (-1.0, "negative")] {
            for &(cell, offset) in cells {
                points.push(LinkPoint {
                    id: format!("{side}_{face}_{cell}"),
                    position: normal * sign * half.dot(normal) + along * offset,
                    normal: normal * sign,
                });
            }
        }
    }
    for (x_cell, x) in across {
        for (y_cell, y) in up {
            points.push(LinkPoint {
                id: format!("positive_z_{x_cell}_{y_cell}"),
                position: Vec3::new(x, y, half.z),
                normal: Vec3::Z,
            });
        }
    }
    points
}

/// The door track: the twelve `intake_slat_` nodes of
/// `intake_accordion_3x2x1.glb` fold 80 degrees aside into two pockets.
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
            // The clear opening centred on the face, inside the folded slat
            // stacks and the face ring of intake_accordion_3x2x1.glb. Neither
            // is centred: the nearer slat stack is 11.13 m out and the ring
            // top 7.70 m up, so the opening is 22.2 m by 15.3 m.
            aperture_width: Meters(22.2),
            aperture_height: Meters(15.3),
            maximum_capture_speed: MetersPerSecond(5.0),
            eject_speed: MetersPerSecond(3.0),
        }),
    }]
}
