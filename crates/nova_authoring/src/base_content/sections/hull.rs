//! The plate families: the structural cubes a hull is built out of.
//!
//! Four are the same unit cube with sockets on every face. What separates
//! them is the health pool and the art. Reinforced plate is the structural
//! baseline every other section's durability is quoted against; light plate is
//! a third of it.
//!
//! Two are station structure with sockets on two opposite faces only, so the
//! derived skin, which clads only socketed faces, leaves their flanks bare:
//! the truss (sockets on +-Z) runs a spoke or a ring, and the solar panel
//! (sockets on +-X, a tenth of a cell thick) chains into a wing. Both sockets
//! are `Bare` dead ends, so the open end of a spoke or a wing stops clean
//! instead of wearing a cap plate.

use bevy::prelude::*;
use nova_ship::prelude::*;

use super::recipes;
use crate::base_content::assets::BaseContentAssets;

/// The freight cube: reinforced plate stats in caged-freight art.
pub(crate) const CARGO_HULL_SECTION_ID: &str = "cargo_hull_section";

/// The tank cube: reinforced plate stats around a pressure vessel.
pub(crate) const TANK_HULL_SECTION_ID: &str = "tank_hull_section";

/// The open station truss: a girder cell whose two sockets face +-Z.
pub(crate) const TRUSS_HULL_SECTION_ID: &str = "truss_hull_section";

/// The station solar panel: a thin cell whose two sockets face +-X.
pub(crate) const SOLAR_PANEL_SECTION_ID: &str = "solar_panel_section";

/// How thick the solar panel's collider is, as a share of a cell.
const SOLAR_PANEL_THICKNESS: f32 = 0.1;

/// The cube's sockets whose normals lie on `axis`, the only two a station
/// structure part offers. Both are dead ends: they mate, but the skin never
/// clads them.
fn axis_link_points(size: Vec3, axis: Vec3) -> Vec<LinkPoint> {
    box_link_points(size)
        .into_iter()
        .filter(|point| point.normal.dot(axis).abs() > 0.5)
        .map(|point| LinkPoint {
            skin: LinkPointSkinType::Bare,
            ..point
        })
        .collect()
}

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
                advancement: 0.3,
                draw_weight: 1.0,
                build: recipes::heavy_hull(),
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
                advancement: 0.0,
                draw_weight: 1.0,
                build: recipes::light_hull(),
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
                id: CARGO_HULL_SECTION_ID.to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Cargo Hull Section".to_string(),
                description: "A hull section packed with caged freight; every \
                              face reads the same."
                    .to_string(),
                health: 200.0,
                advancement: 0.3,
                draw_weight: 1.0,
                build: recipes::heavy_hull(),
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
                id: TANK_HULL_SECTION_ID.to_string(),
                damage_effects: DamageEffects(vec![DamageEffect::Cracks]),
                name: "Tank Hull Section".to_string(),
                description: "A hull section carrying a pressure vessel in \
                              open frame rails."
                    .to_string(),
                health: 200.0,
                advancement: 0.3,
                draw_weight: 1.0,
                build: recipes::heavy_hull(),
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
        SectionConfig {
            base: BaseSectionConfig {
                id: TRUSS_HULL_SECTION_ID.to_string(),
                damage_effects: DamageEffects::default(),
                name: "Truss Section".to_string(),
                description: "An open girder cell for station spokes and rings; it \
                              mates only at its two ends."
                    .to_string(),
                // Between light and reinforced plate: a spoke takes a burst
                // to cut, and cutting one is how a ring comes away.
                health: 120.0,
                advancement: 0.2,
                draw_weight: 1.0,
                build: recipes::truss_hull(),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                link_points: axis_link_points(Vec3::ONE, Vec3::Z),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_truss.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: SOLAR_PANEL_SECTION_ID.to_string(),
                damage_effects: DamageEffects::default(),
                name: "Solar Panel Section".to_string(),
                description: "A thin solar panel one cell wide; panels chain side \
                              to side into a wing."
                    .to_string(),
                health: 40.0,
                advancement: 0.5,
                draw_weight: 1.0,
                build: recipes::solar_panel(),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: Some(SectionCollider::Cuboid {
                    size: Vec3::new(1.0, SOLAR_PANEL_THICKNESS, 1.0),
                }),
                link_points: axis_link_points(Vec3::new(1.0, SOLAR_PANEL_THICKNESS, 1.0), Vec3::X),
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull_solar_panel.clone()),
                render_mesh_transform: None,
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The station structure parts offer exactly two sockets on one axis, so
    /// the skin leaves their other faces bare, and both are dead ends, so the
    /// skin leaves an open end bare too.
    #[test]
    fn station_structure_mates_only_along_its_axis_through_dead_ends() {
        let catalog = prototypes(&BaseContentAssets::from_paths());
        for (id, axis) in [
            (TRUSS_HULL_SECTION_ID, Vec3::Z),
            (SOLAR_PANEL_SECTION_ID, Vec3::X),
        ] {
            let part = catalog
                .iter()
                .find(|config| config.base.id == id)
                .expect("the part is in the catalog");
            let normals: Vec<Vec3> = part
                .base
                .link_points
                .iter()
                .map(|point| point.normal)
                .collect();
            assert_eq!(normals, vec![axis, -axis], "{id}");
            assert!(
                part.base
                    .link_points
                    .iter()
                    .all(|point| point.skin == LinkPointSkinType::Bare),
                "{id} clads an open end",
            );
        }
    }
}
