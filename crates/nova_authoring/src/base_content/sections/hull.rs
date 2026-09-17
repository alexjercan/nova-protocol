//! The plate families: the structural cubes a hull is built out of.
//!
//! All four are the same unit cube with sockets on every face. What separates
//! them is the health pool and the art. Reinforced plate is the structural
//! baseline every other section's durability is quoted against; light plate is
//! a third of it.

use bevy::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// The plate prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: REINFORCED_HULL_SECTION_ID.to_string(),
                // Material and nothing else, so the damage reads in the
                // surface: it cracks, and its cladding leaves plate by plate.
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Reinforced Hull Section".to_string(),
                description: "A reinforced hull section for spaceships.".to_string(),
                health: 200.0,
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: LIGHT_HULL_SECTION_ID.to_string(),
                damage_effects: DamageEffects::default(),
                name: "Light Hull Section".to_string(),
                description: "A thin-walled hull section; scavenger grade.".to_string(),
                // A third of reinforced, so a hull plated with it comes apart
                // in a short burst rather than in a slugging match.
                health: 60.0,
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        // The cargo and tank hulls are the reinforced hull in different
        // clothes: same stats on purpose, so today they are a visual choice.
        // The models are the investment - when real hull TYPES arrive
        // (resources, cargo capacity), these two are where the stats land.
        SectionConfig {
            base: BaseSectionConfig {
                id: "cargo_hull_section".to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Cargo Hull Section".to_string(),
                description: "A hull section packed with caged freight; every \
                              face reads the same."
                    .to_string(),
                health: 200.0,
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_cargo.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "tank_hull_section".to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Tank Hull Section".to_string(),
                description: "A hull section carrying a pressure vessel in \
                              open frame rails."
                    .to_string(),
                health: 200.0,
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: unit_cube_link_points(),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_tank.clone()),
                render_mesh_transform: None,
            }),
        },
    ]
}
