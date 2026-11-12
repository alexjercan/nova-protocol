//! The mining beam: a one-cell emitter that cuts ore out of the rock ahead.
//!
//! Hold an emitter's own key, and that emitter parts its doors, runs its tip
//! out and pulses a beam down its -Z face. No travel lock is needed. A pulse
//! cuts the nearest asteroid the beam meets if that rock holds ore, and the
//! ore it frees leaves the rock as canisters.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::BuildRecipe;
use nova_ship::prelude::*;

use super::recipes;
use crate::base_content::assets::BaseContentAssets;

/// The emitter prototype id. Only the block fleet mounts it by name.
pub(crate) const MINING_BEAM_SECTION_ID: &str = "mining_beam_section";

/// The second-tier emitter: the same box with a longer, faster beam.
const LONG_MINING_BEAM_SECTION_ID: &str = "long_mining_beam_section";

/// Provisional: an exposed emitter as soft as the cargo intake.
const MINING_BEAM_BASE_HEALTH: f32 = 90.0;

/// One track leg in each direction. The doors open fully before the tip
/// moves, so a full deploy takes twice this.
const TRACK_SECONDS: f32 = 0.3;

/// The emitter's sockets: one on every face but the -Z emitter face.
///
/// A socket on the -Z face would invite a plate over the doors, and the beam
/// leaves through that face.
fn mining_beam_link_points() -> Vec<LinkPoint> {
    [
        ("positive_x", Vec3::X),
        ("negative_x", Vec3::NEG_X),
        ("positive_y", Vec3::Y),
        ("negative_y", Vec3::NEG_Y),
        ("positive_z", Vec3::Z),
    ]
    .into_iter()
    .map(|(id, normal)| LinkPoint {
        id: id.to_string(),
        position: normal * 0.5,
        normal,
        skin: LinkPointSkinType::Clad,
    })
    .collect()
}

/// The door and tip tracks of `mining_beam_compact.glb`. The art rests
/// deployed; progress 1 is stowed. The `stow_lid_` doors slide 0.22 inward to
/// shut (the left door is turned 180 degrees about Z, so one signed offset
/// closes both), and `beam_tip` slides 0.16 back into the collar behind them.
fn mining_beam_tracks() -> Vec<SectionAnimation> {
    vec![
        SectionAnimation {
            cue: SectionAnimationCue::StowDoors,
            node_prefix: "stow_lid_".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(-0.22, 0.0, 0.0),
            },
            open_seconds: TRACK_SECONDS,
            close_seconds: TRACK_SECONDS,
        },
        SectionAnimation {
            cue: SectionAnimationCue::StowLift,
            node_prefix: "beam_tip".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(0.0, 0.0, 0.16),
            },
            open_seconds: TRACK_SECONDS,
            close_seconds: TRACK_SECONDS,
        },
    ]
}

/// What one emitter grade is: its tier and its cut. Everything else - the
/// one-cell box, the sockets, the door and tip tracks, the art and the
/// sounds - is the EMITTER, shared by construction below so the two grades
/// never drift apart.
struct MiningBeamSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    health: f32,
    advancement: f32,
    draw_weight: f32,
    build: BuildRecipe,
    /// How far down the -Z face a pulse looks for rock.
    reach: Meters,
    /// Seconds between pulses while the key is held.
    pulse_interval_seconds: f32,
    /// The carve sphere, in cells of the rock's own field.
    carve_radius_cells: f32,
}

/// One emitter, named for the grade it is built at.
fn mining_beam_prototype(meshes: &BaseContentAssets, spec: MiningBeamSpec<'_>) -> SectionConfig {
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
            collider: Some(SectionCollider::Cuboid { size: Vec3::ONE }),
            link_points: mining_beam_link_points(),
            hide_in_editor: false,
            animations: mining_beam_tracks(),
        },
        kind: SectionKind::Mining(MiningSectionConfig {
            render_mesh: meshes.mining_beam.clone(),
            render_mesh_transform: None,
            pulse_sound: meshes.mining_pulse_sound.clone(),
            refusal_sound: meshes.mining_refusal_sound.clone(),
            door_open_sound: meshes.mining_door_open_sound.clone(),
            door_close_sound: meshes.mining_door_close_sound.clone(),
            reach: spec.reach,
            pulse_interval_seconds: spec.pulse_interval_seconds,
            carve_radius_cells: spec.carve_radius_cells,
        }),
    }
}

/// The emitter prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        mining_beam_prototype(
            meshes,
            MiningBeamSpec {
                id: MINING_BEAM_SECTION_ID,
                name: "Mining Beam Section",
                description: "A compact ore emitter behind sliding doors. Hold \
                              its key: the doors part, the tip runs out, and once \
                              a second the beam cuts the nearest rock straight \
                              ahead of the emitter face within 100 m, with no \
                              lock needed. A barren rock in front blocks ore \
                              behind it. The ore it cuts free leaves the rock as \
                              canisters. Release the key and the tip retracts \
                              before the doors shut.",
                health: MINING_BEAM_BASE_HEALTH,
                advancement: 0.0,
                draw_weight: 1.0,
                build: recipes::mining_beam(),
                // Provisional stats: 100 m of reach, one pulse a second, and
                // a carve sphere of 1.5 cells of the rock's own field.
                reach: Meters(100.0),
                pulse_interval_seconds: 1.0,
                carve_radius_cells: 1.5,
            },
        ),
        mining_beam_prototype(
            meshes,
            MiningBeamSpec {
                id: LONG_MINING_BEAM_SECTION_ID,
                name: "Long Mining Beam Section",
                description: "The same emitter with a longer, faster beam: it \
                              cuts rock within 160 m, pulses every 0.8 s, and \
                              its casing takes more damage before it fails.",
                health: 110.0,
                advancement: 0.3,
                draw_weight: 0.6,
                build: recipes::long_mining_beam(),
                // The same cut, reached from further out and more often: the
                // grade buys standoff and tempo, never a bigger sphere.
                reach: Meters(160.0),
                pulse_interval_seconds: 0.8,
                carve_radius_cells: 1.5,
            },
        ),
    ]
}
