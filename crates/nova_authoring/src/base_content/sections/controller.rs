//! The command core: the box that turns a hull.
//!
//! What a hull DOES with one is `min(torque / inertia, structural load limit /
//! arm)`, so the authored torque is only half the answer and the hull's own
//! geometry is the other half.

use bevy::prelude::*;
use nova_gameplay::prelude::BuildRecipe;
use nova_ship::prelude::*;

use super::recipes;
use crate::base_content::assets::BaseContentAssets;

/// Command core: the mid durability baseline (`nova_gameplay::damage` scales a
/// hit by section class, and Kinetic is 1.0 against every class, so this is
/// what a generalist round meets).
pub(super) const CONTROLLER_BASE_HEALTH: f32 = 100.0;

/// The second-tier core: the same cell, armoured, slower to steer and
/// stronger in torque.
const ARMOURED_CONTROLLER_SECTION_ID: &str = "armoured_controller_section";

/// What one controller grade is: its tier and its attitude hardware.
/// Everything else - the cell, the sockets, the art - is the CONTROLLER,
/// shared by construction below so the two grades never drift apart.
struct ControllerSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    health: f32,
    advancement: f32,
    draw_weight: f32,
    build: BuildRecipe,
    steering_lag: f32,
    max_torque: f32,
}

/// One controller, named for the grade it is built at.
fn controller_prototype(meshes: &BaseContentAssets, spec: ControllerSpec<'_>) -> SectionConfig {
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
            collider: None,
            link_points: unit_cube_link_points(),
            hide_in_editor: false,
            animations: Vec::new(),
        },
        kind: SectionKind::Controller(ControllerSectionConfig {
            steering_lag: spec.steering_lag,
            max_torque: spec.max_torque,
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
    }
}

/// The controller prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        controller_prototype(
            meshes,
            ControllerSpec {
                id: BASIC_CONTROLLER_SECTION_ID,
                name: "Basic Controller Section",
                description: "A basic controller section for spaceships.",
                // Command core: mid durability baseline.
                health: CONTROLLER_BASE_HEALTH,
                advancement: 0.0,
                draw_weight: 1.0,
                build: recipes::controller(),
                steering_lag: 0.5,
                // What a hull DOES with this is `min(torque / inertia, load
                // limit / arm)`. Pinned on the largest hull measured: ten of
                // these put it about 10 percent over its own structural
                // ceiling, so an intact capital turns at the rate its metal
                // allows and a capital that has lost a computer does not.
                // Small hulls are structure-bound either way.
                max_torque: 9760.0,
            },
        ),
        controller_prototype(
            meshes,
            ControllerSpec {
                id: ARMOURED_CONTROLLER_SECTION_ID,
                name: "Armoured Controller Section",
                description: "The same core in an armoured cell: it takes \
                              more damage before it fails and turns the hull \
                              harder, but it answers the stick more slowly.",
                // Above the mid baseline the durability order is pinned on,
                // still under the turret baseline the order names next.
                health: 160.0,
                advancement: 0.3,
                draw_weight: 0.6,
                build: recipes::armoured_controller(),
                steering_lag: 0.7,
                max_torque: 11_000.0,
            },
        ),
    ]
}
