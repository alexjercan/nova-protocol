//! The drives: the basic bell and the two large vectoring blocks.
//!
//! Large-drive mass stays physical collider volume, but thrust follows
//! exhaust-face area and health follows a compressed, surface-like curve.
//! Linear volume scaling made the arena capital too fast and too durable.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::assets::BaseContentAssets;

/// Exposed propulsion: fragile, takes more damage per hit than an armoured
/// mount. The HEALTH half of "variable damage by section type"; the other half
/// is `nova_gameplay::damage`'s resistance table.
pub(super) const THRUSTER_BASE_HEALTH: f32 = 70.0;

/// The two authoring-owned drive ids. The basic bell's id is shared with the
/// engine, so it comes from `nova_ship` instead.
///
/// The vectoring block is re-exported by `sections`, because block hulls mount
/// one by name.
pub(crate) const VECTOR_THRUSTER_SECTION_ID: &str = "vector_thruster_section";
const CAPITAL_THRUSTER_SECTION_ID: &str = "capital_thruster_section";

fn drive_mount_points(cells: UVec3) -> Vec<LinkPoint> {
    let half = (cells.as_vec3() - Vec3::ONE) * 0.5;
    let mut points: Vec<LinkPoint> = (0..cells.x)
        .flat_map(|x| (0..cells.y).map(move |y| (x, y)))
        .map(|(x, y)| LinkPoint {
            id: format!("base_{x}_{y}"),
            position: Vec3::new(
                x as f32 - half.x,
                y as f32 - half.y,
                -(cells.z as f32) * 0.5,
            ),
            normal: Vec3::NEG_Z,
        })
        .collect();
    points.sort_by_key(|point| {
        let radial = point.position.x * point.position.x + point.position.y * point.position.y;
        ((radial * 4.0).round() as i32, point.id.clone())
    });
    points
}

struct LargeDriveSpec<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
    cells: UVec3,
    health: f32,
    magnitude: f32,
    mesh: &'a AssetRef<WorldAsset>,
    /// The drive's own hum. The three sizes run 34 / 52 / 78 Hz and are
    /// separated by pitch alone.
    loop_sound: &'a AssetRef<AudioSource>,
    exhaust_offset: f32,
    exhaust_radius: f32,
    exhaust_inner_radius: f32,
    exhaust_height: f32,
}

fn large_thruster_prototype(meshes: &BaseContentAssets, spec: LargeDriveSpec<'_>) -> SectionConfig {
    SectionConfig {
        base: BaseSectionConfig {
            id: spec.id.to_string(),
            name: spec.name.to_string(),
            description: spec.description.to_string(),
            health: spec.health,
            destroy_sound: Some(meshes.section_destroy_sound.clone()),
            collider: Some(SectionCollider::Cuboid {
                size: spec.cells.as_vec3(),
            }),
            link_points: drive_mount_points(spec.cells),
            hide_in_editor: false,
            damage_effects: DamageEffects(vec![
                DamageEffect::Cracks,
                DamageEffect::Sparks,
                DamageEffect::Plume,
            ]),
            animations: Vec::new(),
        },
        kind: SectionKind::Thruster(ThrusterSectionConfig {
            magnitude: spec.magnitude,
            render_mesh: Some(spec.mesh.clone()),
            render_mesh_transform: None,
            loop_sound: Some(spec.loop_sound.clone()),
            exhaust: Some(ThrusterExhaust {
                offset: Vec3::new(0.0, 0.0, spec.exhaust_offset),
                shape: ThrusterExhaustConfig {
                    exhaust_height: spec.exhaust_height,
                    exhaust_radius: spec.exhaust_radius,
                    exhaust_inner_height: spec.exhaust_height * 0.5,
                    exhaust_inner_radius: spec.exhaust_inner_radius,
                    ..default()
                },
                ..default()
            }),
        }),
    }
}

/// The drive prototypes, in catalog order.
pub(super) fn prototypes(meshes: &BaseContentAssets) -> Vec<SectionConfig> {
    vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: BASIC_THRUSTER_SECTION_ID.to_string(),
                // A drive is machinery and a bell: it sparks and its
                // plume guts, and it never loses a piece of itself.
                damage_effects: DamageEffects(vec![
                    DamageEffect::Cracks,
                    DamageEffect::Sparks,
                    DamageEffect::Plume,
                ]),
                name: "Basic Thruster Section".to_string(),
                description: "A basic thruster section for spaceships.".to_string(),
                // Exposed propulsion: fragile, takes more damage per hit than
                // an armored mount.
                health: THRUSTER_BASE_HEALTH,
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
                // ONE socket, on the mounting face. The authored bell opens
                // toward +Z and the plume fires out of it. So the flat forward
                // end is the only structure on the part - the other five faces
                // are drive and exhaust. Six sockets offered them
                // all as mating surfaces, and a builder would bolt a hull slab
                // onto the barrel or plate one across the nozzle.
                link_points: vec![LinkPoint {
                    id: "base".to_string(),
                    position: Vec3::NEG_Z * 0.5,
                    normal: Vec3::NEG_Z,
                }],
                hide_in_editor: false,
                animations: Vec::new(),
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: Some(meshes.thruster_bell.clone()),
                render_mesh_transform: None,
                loop_sound: Some(meshes.thruster_loop_sound.clone()),
                exhaust: Some(ThrusterExhaust {
                    offset: Vec3::new(0.0, 0.0, 0.51),
                    shape: ThrusterExhaustConfig {
                        exhaust_radius: 0.24,
                        exhaust_inner_radius: 0.07,
                        ..default()
                    },
                    ..default()
                }),
            }),
        },
        // Large-drive mass stays physical collider volume, but thrust follows
        // exhaust-face area and health follows a compressed, surface-like curve.
        // Linear volume scaling made the arena capital too fast and too durable.
        large_thruster_prototype(
            meshes,
            LargeDriveSpec {
                id: VECTOR_THRUSTER_SECTION_ID,
                name: "Vector Thruster Section",
                description: "A 3x3x2 vectoring drive for larger ships.",
                cells: UVec3::new(3, 3, 2),
                health: 480.0,
                magnitude: 9.0,
                mesh: &meshes.thruster_vector,
                loop_sound: &meshes.thruster_vector_loop_sound,
                exhaust_offset: 0.886,
                exhaust_radius: 0.58,
                exhaust_inner_radius: 0.18,
                exhaust_height: 0.3,
            },
        ),
        large_thruster_prototype(
            meshes,
            LargeDriveSpec {
                id: CAPITAL_THRUSTER_SECTION_ID,
                name: "Capital Thruster Section",
                description: "A 5x5x3 capital drive for the largest ships.",
                cells: UVec3::new(5, 5, 3),
                health: 1250.0,
                magnitude: 25.0,
                mesh: &meshes.thruster_capital,
                loop_sound: &meshes.thruster_capital_loop_sound,
                exhaust_offset: 1.51,
                exhaust_radius: 1.1,
                exhaust_inner_radius: 0.35,
                exhaust_height: 0.5,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_drives_use_face_thrust_and_surface_scaled_health() {
        let catalog = crate::generation::build_section_catalog();
        for (id, cells, health, magnitude) in [
            (VECTOR_THRUSTER_SECTION_ID, UVec3::new(3, 3, 2), 480.0, 9.0),
            (
                CAPITAL_THRUSTER_SECTION_ID,
                UVec3::new(5, 5, 3),
                1250.0,
                25.0,
            ),
        ] {
            let drive = catalog
                .iter()
                .find(|section| section.base.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in the catalog"));
            assert_eq!(drive.base.health, health);
            assert_eq!(drive.base.link_points.len(), (cells.x * cells.y) as usize);
            assert_eq!(
                drive.base.collider,
                Some(SectionCollider::Cuboid {
                    size: cells.as_vec3(),
                })
            );
            let SectionKind::Thruster(config) = &drive.kind else {
                panic!("`{id}` has the wrong section kind");
            };
            assert_eq!(config.magnitude, magnitude);
            assert!(config.render_mesh.is_some());
        }
    }

    /// The drive bolts on by its forward end and by nothing else.
    ///
    /// The authored bell lies on the Z axis and opens toward +Z, so the
    /// mounting end is -Z - NOT the -Y a turret stands on.
    /// Six sockets made the barrel and the open nozzle mating surfaces too, and
    /// a generator would plate a hull slab across the exhaust.
    #[test]
    fn the_thruster_sockets_only_the_face_it_bolts_on_by() {
        let drive = crate::generation::build_section_catalog()
            .into_iter()
            .find(|section| section.base.id == BASIC_THRUSTER_SECTION_ID)
            .expect("the basic thruster is in the catalog");
        let [base] = drive.base.link_points.as_slice() else {
            panic!(
                "the thruster carries {} sockets, not one",
                drive.base.link_points.len()
            );
        };
        assert_eq!(base.normal, Vec3::NEG_Z, "the mounting face points forward");
        assert_eq!(
            base.position,
            Vec3::NEG_Z * 0.5,
            "the socket sits on that face, not inside the part"
        );
    }

    #[test]
    fn the_basic_thruster_exhaust_fits_its_bell() {
        let drive = crate::generation::build_section_catalog()
            .into_iter()
            .find(|section| section.base.id == BASIC_THRUSTER_SECTION_ID)
            .expect("the basic thruster is in the catalog");
        let SectionKind::Thruster(config) = drive.kind else {
            panic!("the basic thruster has the wrong section kind");
        };
        let exhaust = config
            .exhaust
            .expect("the basic thruster authors its exhaust");
        assert_eq!(exhaust.offset, Vec3::new(0.0, 0.0, 0.51));
        assert_eq!(exhaust.shape.geometry, ThrusterExhaustShape::Cone);
        assert_eq!(exhaust.shape.exhaust_radius, 0.24);
        assert_eq!(exhaust.shape.exhaust_inner_radius, 0.07);
    }
}
