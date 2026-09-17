//! The command core: the box that turns a hull.
//!
//! What a hull DOES with one is `min(torque / inertia, structural load limit /
//! arm)`, so the authored torque is only half the answer and the hull's own
//! geometry is the other half.

use bevy::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// Command core: the mid durability baseline (`nova_gameplay::damage` scales a
/// hit by section class, and Kinetic is 1.0 against every class, so this is
/// what a generalist round meets).
pub(super) const CONTROLLER_BASE_HEALTH: f32 = 100.0;

/// The controller prototype.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![SectionConfig {
        base: BaseSectionConfig {
            id: BASIC_CONTROLLER_SECTION_ID.to_string(),
            damage_effects: DamageEffects(vec![DamageEffect::Cracks, DamageEffect::Sparks]),
            name: "Basic Controller Section".to_string(),
            description: "A basic controller section for spaceships.".to_string(),
            // Command core: mid durability baseline.
            health: CONTROLLER_BASE_HEALTH,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: None,
            link_points: unit_cube_link_points(),
            hide_in_editor: false,
            animations: Vec::new(),
        },
        kind: SectionKind::Controller(ControllerSectionConfig {
            steering_lag: 0.5,
            // What a hull DOES with this is `min(torque / inertia, load
            // limit / arm)`. Pinned on the largest hull measured: ten of
            // these put it about 10 percent over its own structural
            // ceiling, so an intact capital turns at the rate
            // its metal allows and a capital that has lost a computer does
            // not. Small hulls are structure-bound either way.
            max_torque: 9760.0,
            // Attitude hardware only. Spawn capabilities control permitted
            // actions, and presentation config controls labels and effects;
            // neither belongs to this section prototype.
            //
            // The cable-wrapped computer cell: the first controller with a
            // body of its own instead of an invisible cube. Every face
            // carries the same signal pattern, so its rotation never shows.
            render_mesh: Some(meshes.controller_core.clone()),
            render_mesh_transform: None,
        }),
    }]
}
