//! The cargo intake: a 3x2 hold mouth behind an accordion door.
//!
//! Fly slowly past a drifting canister with the door facing it, and the door
//! folds open and the intake takes every whole item the hold has room for.
//! The rest stays in the same canister. The Inventory pane's Jettison drops a
//! canister out through the same door.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::BuildRecipe;
use nova_ship::prelude::*;

use super::recipes;
use crate::base_content::assets::BaseContentAssets;

/// The intake prototype id. Only the block fleet mounts it by name.
pub(crate) const CARGO_INTAKE_SECTION_ID: &str = "cargo_intake_section";

/// The second-tier intake: the same box with a wider, longer trigger.
const WIDE_INTAKE_SECTION_ID: &str = "wide_intake_section";

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
                    skin: LinkPointSkinType::Clad,
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
                skin: LinkPointSkinType::Clad,
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

/// What one intake grade is: its tier, its door and its trigger. Everything
/// else - the 3x2x1 box, the sockets, the accordion track, the art and the
/// sounds - is the INTAKE, shared by construction below so the two grades
/// never drift apart.
struct IntakeSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    health: f32,
    advancement: f32,
    draw_weight: f32,
    build: BuildRecipe,
    /// How far out a fitting canister folds the door open.
    detection_range: Meters,
    /// How deep the take trigger stands in front of the opening.
    capture_gap: Meters,
    /// The clear opening, inside the 30 x 20 m door face.
    aperture_width: Meters,
    aperture_height: Meters,
}

/// One intake, named for the grade it is built at.
fn intake_prototype(meshes: &BaseContentAssets, spec: IntakeSpec<'_>) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: spec.id.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            health: spec.health,
            advancement: spec.advancement,
            draw_weight: spec.draw_weight,
            build: spec.build,
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
            detection_range: spec.detection_range,
            capture_gap: spec.capture_gap,
            aperture_width: spec.aperture_width,
            aperture_height: spec.aperture_height,
            eject_speed: MetersPerSecond(3.0),
        }),
    }
}

/// The intake prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        intake_prototype(
            meshes,
            IntakeSpec {
                id: CARGO_INTAKE_SECTION_ID,
                name: "Cargo Intake Section",
                description: "A hold mouth behind an accordion door. The door \
                              folds open for a canister within 40 m of it. When \
                              a canister reaches into the 1 m in front of its \
                              opening, the intake takes, at any speed, every \
                              whole item the hold has room for. The rest stays \
                              in the same canister. Jettison in the Inventory \
                              pane drops a canister out through the same door.",
                health: CARGO_INTAKE_BASE_HEALTH,
                advancement: 0.0,
                draw_weight: 1.0,
                build: recipes::cargo_intake(),
                detection_range: Meters(40.0),
                capture_gap: Meters(1.0),
                // The clear opening centred on the face, inside the folded
                // slat stacks and the face ring of intake_accordion_3x2x1.glb.
                // Neither is centred: the nearer slat stack is 11.13 m out
                // and the ring top 7.70 m up, so the opening is 22.2 m by
                // 15.3 m.
                aperture_width: Meters(22.2),
                aperture_height: Meters(15.3),
            },
        ),
        intake_prototype(
            meshes,
            IntakeSpec {
                id: WIDE_INTAKE_SECTION_ID,
                name: "Wide Intake Section",
                description: "The same door and hold mouth with a wider \
                              trigger: the door opens for a canister within \
                              60 m, the take reaches 1.5 m out over a 28 x \
                              18 m opening, and the machinery takes a third \
                              more damage before it fails.",
                health: 120.0,
                advancement: 0.3,
                draw_weight: 0.6,
                build: recipes::wide_intake(),
                detection_range: Meters(60.0),
                capture_gap: Meters(1.5),
                // Past the slat stacks of the same art, so a canister the
                // base opening would miss by a cell still goes in. The lint
                // holds it inside the 30 x 20 m collider face.
                aperture_width: Meters(28.0),
                aperture_height: Meters(18.0),
            },
        ),
    ]
}
