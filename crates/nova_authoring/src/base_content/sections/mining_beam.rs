//! The mining beam: a one-cell emitter that cuts ore out of a locked rock.
//!
//! Hold an emitter's own key with a rock travel-locked, and that emitter parts
//! its doors, runs its tip out and pulses a beam down its -Z face. A pulse that
//! lands cuts the rock, and the ore it frees leaves the rock as canisters.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// The emitter prototype id. Only the block fleet mounts it by name.
pub(crate) const MINING_BEAM_SECTION_ID: &str = "mining_beam_section";

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

/// The emitter prototype.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![SectionConfig {
        base: BaseSectionConfig {
            id: MINING_BEAM_SECTION_ID.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: "Mining Beam Section".to_string(),
            description: "A compact ore emitter behind sliding doors. Hold \
                          its key with a rock travel-locked: the doors \
                          part, the tip runs out, and once a second the beam \
                          cuts the rock where it hits, if the rock is within \
                          100 m of the emitter face. The ore it cuts free \
                          leaves the rock as canisters. Release the key and \
                          the tip retracts before the doors shut."
                .to_string(),
            health: MINING_BEAM_BASE_HEALTH,
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
            door_open_sound: meshes.mining_door_open_sound.clone(),
            door_close_sound: meshes.mining_door_close_sound.clone(),
            // Provisional stats: 100 m of reach, one pulse a second, and a
            // carve sphere of 1.5 cells of the rock's own field.
            reach: Meters(100.0),
            pulse_interval_seconds: 1.0,
            carve_radius_cells: 1.5,
        }),
    }]
}
