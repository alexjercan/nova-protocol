//! Semantic Kenney ship parts and the shipped Racer, CargoB, and CargoA assemblies.
//!
//! Part meshes are centered on tight primitive colliders. Structural edges come only from
//! authored link-point mates shared by the catalog prototypes and ship builders.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::sections::{turret_joint_tree, SectionMeshRefs};

pub(crate) const RACER_TURRET_IDS: [&str; 2] = ["turret_port", "turret_starboard"];

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ShipGrade {
    Player,
    Enemy,
}

#[derive(Clone, Copy)]
enum PartRole {
    Hull,
    Thruster,
    Controller,
    Torpedo,
    Turret,
}

#[derive(Clone, Copy)]
enum PartSide {
    None,
    Port,
    Starboard,
}

#[derive(Clone, Copy)]
struct PartSpec {
    id: &'static str,
    prototype: &'static str,
    mesh: Option<&'static str>,
    origin: Vec3,
    bbox_min: Vec3,
    bbox_max: Vec3,
    health: f32,
    role: PartRole,
    side: PartSide,
}

impl PartSpec {
    fn center(self) -> Vec3 {
        self.origin + (self.bbox_min + self.bbox_max) * 0.5
    }

    fn size(self) -> Vec3 {
        self.bbox_max - self.bbox_min
    }

    fn mesh_offset(self) -> Vec3 {
        self.origin - self.center()
    }

    fn rotation(self) -> Quat {
        let quarter = std::f32::consts::FRAC_PI_2;
        match self.side {
            PartSide::None => Quat::IDENTITY,
            PartSide::Port => Quat::from_rotation_z(quarter),
            PartSide::Starboard => Quat::from_rotation_z(-quarter),
        }
    }
}

const RACER_PARTS: [PartSpec; 9] = [
    part(
        "engine_starboard",
        "racer_engine_starboard",
        "racer/engine_starboard.glb",
        v(0.5, 0.5, 1.5),
        v(-0.09, -0.3, -0.3),
        v(0.4, 0.44189, 0.32567),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "racer_engine_port",
        "racer/engine_port.glb",
        v(-0.5, 0.5, 1.5),
        v(-0.4, -0.3, -0.3),
        v(0.09, 0.44189, 0.32567),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "wing_starboard",
        "racer_wing_starboard",
        "racer/wing_starboard.glb",
        v(1.0, 0.5, 0.0),
        v(-0.59, -0.5, -0.964329),
        v(0.2, 0.5, 1.2),
        180.0,
        PartRole::Hull,
    ),
    part(
        "wing_port",
        "racer_wing_port",
        "racer/wing_port.glb",
        v(-1.0, 0.5, 0.0),
        v(-0.2, -0.5, -0.964329),
        v(0.59, 0.5, 1.2),
        180.0,
        PartRole::Hull,
    ),
    part(
        "nose",
        "racer_nose",
        "racer/nose.glb",
        v(0.0, 0.5, -1.5),
        v(-0.4, -0.5, -0.52567),
        v(0.4, 0.72265, 0.5),
        120.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "racer_tail",
        "racer/tail.glb",
        v(0.0, 1.0, 1.5),
        v(-0.41, -0.8, -0.3),
        v(0.41, 0.5, 0.52567),
        120.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "racer_fuselage",
        "racer/fuselage.glb",
        v(0.0, 0.5, 0.0),
        v(-0.41, -0.5, -1.0),
        v(0.41, 0.9, 1.2),
        240.0,
        PartRole::Controller,
    ),
    module(
        "turret_starboard",
        "racer_turret_starboard",
        v(1.35, 0.4, -0.8),
        130.0,
        PartRole::Turret,
        PartSide::Starboard,
    ),
    module(
        "turret_port",
        "racer_turret_port",
        v(-1.35, 0.4, -0.8),
        130.0,
        PartRole::Turret,
        PartSide::Port,
    ),
];

const RACER_EDGES: [(usize, usize); 10] = [
    (6, 4),
    (6, 5),
    (5, 0),
    (5, 1),
    (6, 2),
    (6, 3),
    (2, 0),
    (3, 1),
    (2, 7),
    (3, 8),
];

const CARGOB_PARTS: [PartSpec; 9] = [
    part(
        "engine_starboard",
        "cargob_engine_starboard",
        "cargob/engine_starboard.glb",
        v(1.0, 0.5, 2.0),
        v(-0.39, -0.3, -0.5),
        v(0.4, 0.7, 0.5),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "cargob_engine_port",
        "cargob/engine_port.glb",
        v(-1.0, 0.5, 2.0),
        v(-0.4, -0.3, -0.5),
        v(0.39, 0.7, 0.5),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "pod_starboard",
        "cargob_pod_starboard",
        "cargob/pod_starboard.glb",
        v(1.0, 0.5, -0.5),
        v(-0.39, -0.3, -2.0),
        v(0.5, 0.7, 2.0),
        350.0,
        PartRole::Torpedo,
    ),
    part(
        "pod_port",
        "cargob_pod_port",
        "cargob/pod_port.glb",
        v(-1.0, 0.5, -0.5),
        v(-0.5, -0.3, -2.0),
        v(0.39, 0.7, 2.0),
        350.0,
        PartRole::Torpedo,
    ),
    part(
        "nose",
        "cargob_nose",
        "cargob/nose.glb",
        v(0.0, 1.0, -2.0),
        v(-0.61, -0.8, -0.5),
        v(0.61, 0.8, 1.0),
        180.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "cargob_tail",
        "cargob/tail.glb",
        v(0.0, 0.5, 2.0),
        v(-0.61, -0.5, -0.5),
        v(0.61, 0.8, 0.5),
        150.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "cargob_fuselage",
        "cargob/fuselage.glb",
        v(0.0, 1.0, 0.5),
        v(-0.61, -1.0, -1.5),
        v(0.61, 0.8, 1.0),
        300.0,
        PartRole::Controller,
    ),
    module(
        "turret_starboard",
        "cargob_turret_starboard",
        v(1.55, 1.2, 0.0),
        130.0,
        PartRole::Turret,
        PartSide::Starboard,
    ),
    module(
        "turret_port",
        "cargob_turret_port",
        v(-1.55, 1.2, 0.0),
        130.0,
        PartRole::Turret,
        PartSide::Port,
    ),
];

const CARGOB_EDGES: [(usize, usize); 8] = [
    (6, 4),
    (6, 5),
    (6, 2),
    (6, 3),
    (2, 0),
    (3, 1),
    (2, 7),
    (3, 8),
];

const CARGOA_PARTS: [PartSpec; 7] = [
    part(
        "engine_starboard",
        "cargoa_engine_starboard",
        "cargoa/engine_starboard.glb",
        v(1.0, 0.5, 2.0),
        v(-0.19, -0.2975, -0.5),
        v(0.6, 0.4975, 0.45),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "engine_port",
        "cargoa_engine_port",
        "cargoa/engine_port.glb",
        v(-1.0, 0.5, 2.0),
        v(-0.6, -0.2975, -0.5),
        v(0.19, 0.4975, 0.45),
        70.0,
        PartRole::Thruster,
    ),
    part(
        "pod_starboard",
        "cargoa_pod_starboard",
        "cargoa/pod_starboard.glb",
        v(1.0, 0.5, 0.5),
        v(-0.19, -0.3, -1.05),
        v(0.6, 0.7, 1.0),
        350.0,
        PartRole::Hull,
    ),
    part(
        "pod_port",
        "cargoa_pod_port",
        "cargoa/pod_port.glb",
        v(-1.0, 0.5, 0.5),
        v(-0.6, -0.3, -1.05),
        v(0.19, 0.7, 1.0),
        350.0,
        PartRole::Hull,
    ),
    part(
        "nose",
        "cargoa_nose",
        "cargoa/nose.glb",
        v(0.0, 1.0, -2.0),
        v(-0.8, -0.8, -0.45),
        v(0.8, 0.4, 0.85),
        180.0,
        PartRole::Hull,
    ),
    part(
        "tail",
        "cargoa_tail",
        "cargoa/tail.glb",
        v(0.0, 0.5, 2.0),
        v(-0.81, -0.5, -0.5),
        v(0.81, 0.675, 0.45),
        150.0,
        PartRole::Hull,
    ),
    part(
        "fuselage",
        "cargoa_fuselage",
        "cargoa/fuselage.glb",
        v(0.0, 1.0, 0.0),
        v(-0.81, -1.0, -1.15),
        v(0.81, 0.6, 1.5),
        350.0,
        PartRole::Controller,
    ),
];

const CARGOA_EDGES: [(usize, usize); 6] = [(6, 4), (6, 5), (6, 2), (6, 3), (2, 0), (3, 1)];

const fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

const fn part(
    id: &'static str,
    prototype: &'static str,
    mesh: &'static str,
    origin: Vec3,
    bbox_min: Vec3,
    bbox_max: Vec3,
    health: f32,
    role: PartRole,
) -> PartSpec {
    PartSpec {
        id,
        prototype,
        mesh: Some(mesh),
        origin,
        bbox_min,
        bbox_max,
        health,
        role,
        side: PartSide::None,
    }
}

const fn module(
    id: &'static str,
    prototype: &'static str,
    center: Vec3,
    health: f32,
    role: PartRole,
    side: PartSide,
) -> PartSpec {
    PartSpec {
        id,
        prototype,
        mesh: None,
        origin: center,
        bbox_min: v(-0.15, -0.15, -0.15),
        bbox_max: v(0.15, 0.15, 0.15),
        health,
        role,
        side,
    }
}

fn mesh_ref(file: &str) -> AssetRef<WorldAsset> {
    AssetRef::from(format!("self://gltf/parts/{file}#Scene0"))
}

fn render_transform(spec: PartSpec) -> Option<RenderMeshTransform> {
    Some(RenderMeshTransform {
        position: spec.mesh_offset(),
        rotation: Quat::IDENTITY,
    })
}

fn link_points(specs: &[PartSpec], edges: &[(usize, usize)], index: usize) -> Vec<LinkPoint> {
    let spec = specs[index];
    let center = spec.center();
    let rotation = spec.rotation();
    edges
        .iter()
        .filter_map(|&(a, b)| {
            let other = if a == index {
                b
            } else if b == index {
                a
            } else {
                return None;
            };
            let other_center = specs[other].center();
            let direction = (other_center - center).normalize();
            let world_position = (center + other_center) * 0.5;
            Some(LinkPoint {
                id: format!("to_{}", specs[other].id),
                position: rotation.inverse() * (world_position - center),
                normal: rotation.inverse() * direction,
            })
        })
        .collect()
}

fn base_config(
    spec: PartSpec,
    links: Vec<LinkPoint>,
    meshes: &SectionMeshRefs,
) -> BaseSectionConfig {
    BaseSectionConfig {
        id: spec.prototype.to_string(),
        name: spec
            .id
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
        description: "A semantic ship part from a Kenney spacecraft.".to_string(),
        mass: 1.0,
        health: spec.health,
        impact_sound: Some(meshes.section_impact_sound.clone()),
        destroy_sound: Some(meshes.section_destroy_sound.clone()),
        collider: Some(SectionCollider::Cuboid { size: spec.size() }),
        link_points: links,
        // Placement remains hidden until link-point snapping lands in the editor.
        hide_in_editor: true,
    }
}

fn hull_kind(spec: PartSpec) -> SectionKind {
    SectionKind::Hull(HullSectionConfig {
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
    })
}

fn thruster_kind(spec: PartSpec, meshes: &SectionMeshRefs) -> SectionKind {
    SectionKind::Thruster(ThrusterSectionConfig {
        magnitude: 1.0,
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        loop_sound: Some(meshes.thruster_loop_sound.clone()),
        exhaust: Some(ThrusterExhaust {
            offset: Vec3::new(0.0, 0.0, spec.size().z * 0.5),
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            shape: ThrusterExhaustConfig {
                geometry: ThrusterExhaustShape::Rect,
                width: spec.size().x * 0.7,
                height: spec.size().y * 0.6,
                ..default()
            },
        }),
    })
}

fn controller_kind(spec: PartSpec, meshes: &SectionMeshRefs) -> SectionKind {
    SectionKind::Controller(ControllerSectionConfig {
        frequency: 4.0,
        damping_ratio: 4.0,
        max_torque: 800.0,
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        lock_on_sound: Some(meshes.controller_lock_on_sound.clone()),
        lock_off_sound: Some(meshes.controller_lock_off_sound.clone()),
        radar_deny_sound: Some(meshes.controller_radar_deny_sound.clone()),
        radar_retarget_sound: Some(meshes.controller_radar_retarget_sound.clone()),
        safety_on_sound: Some(meshes.controller_safety_on_sound.clone()),
        rcs_loop_sound: Some(meshes.controller_rcs_loop_sound.clone()),
    })
}

fn torpedo_kind(spec: PartSpec, meshes: &SectionMeshRefs) -> SectionKind {
    SectionKind::Torpedo(TorpedoSectionConfig {
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
        projectile_render_mesh: None,
        spawn_offset: Vec3::new(0.0, 0.0, -spec.size().z * 0.5 - 0.5),
        spawn_rotation: Quat::IDENTITY,
        fire_rate: 1.0,
        spawner_speed: 1.0,
        projectile_lifetime: 100.0,
        arm_time: 0.5,
        arm_distance: 5.0,
        nav_constant: 3.0,
        max_speed: 35.0,
        linear_damping: 0.8,
        blast_radius: 30.0,
        blast_damage: 100.0,
        blast_effect: None,
        launch_effect: None,
        launch_sound: Some(meshes.torpedo_launch_sound.clone()),
        detonation_sound: Some(meshes.section_destroy_sound.clone()),
        ammo_capacity: Some(6),
        reload: Some(SectionReloadConfig {
            reload_time: 4.0,
            rounds_per_cycle: 1,
            only_when_empty: false,
        }),
    })
}

fn turret_kind(meshes: &SectionMeshRefs, enemy: bool) -> SectionKind {
    let fire_rate = if enemy { 25.0 } else { 100.0 };
    let root = turret_joint_tree(
        &meshes.turret_yaw,
        &meshes.turret_pitch,
        &meshes.turret_barrel,
        fire_rate,
    );
    SectionKind::Turret(TurretSectionConfig {
        root,
        muzzle_speed: if enemy { 60.0 } else { 100.0 },
        projectile_lifetime: 5.0,
        bullet_damage: if enemy {
            representative_kinetic_damage(0.05, 60.0)
        } else {
            4.0
        },
        bullet_kind: DamageType::Kinetic,
        projectile_render_mesh: None,
        fire_sound: Some(meshes.turret_fire_sound.clone()),
        dry_fire_sound: Some(meshes.turret_dry_fire_sound.clone()),
        ammo_capacity: Some(if enemy { 150 } else { 500 }),
        reload: Some(SectionReloadConfig {
            reload_time: if enemy { 2.5 } else { 3.0 },
            rounds_per_cycle: if enemy { 150 } else { 500 },
            only_when_empty: true,
        }),
    })
}

fn prototypes(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    meshes: &SectionMeshRefs,
    light_turrets: bool,
) -> Vec<SectionConfig> {
    let mut output = Vec::new();
    for (index, &spec) in specs.iter().enumerate() {
        let links = link_points(specs, edges, index);
        let kind = match spec.role {
            PartRole::Hull => hull_kind(spec),
            PartRole::Thruster => thruster_kind(spec, meshes),
            PartRole::Controller => controller_kind(spec, meshes),
            PartRole::Torpedo => torpedo_kind(spec, meshes),
            PartRole::Turret => turret_kind(meshes, false),
        };
        output.push(SectionConfig {
            base: base_config(spec, links.clone(), meshes),
            kind,
        });
        if light_turrets && matches!(spec.role, PartRole::Turret) {
            let mut base = base_config(spec, links, meshes);
            base.id = format!("{}_light", spec.prototype);
            base.name = format!("{} Light", base.name);
            base.health = 60.0;
            output.push(SectionConfig {
                base,
                kind: turret_kind(meshes, true),
            });
        }
    }
    output
}

pub(crate) fn racer_prototypes(meshes: &SectionMeshRefs) -> Vec<SectionConfig> {
    prototypes(&RACER_PARTS, &RACER_EDGES, meshes, true)
}

pub(crate) fn cargob_prototypes(meshes: &SectionMeshRefs) -> Vec<SectionConfig> {
    prototypes(&CARGOB_PARTS, &CARGOB_EDGES, meshes, false)
}

pub(crate) fn cargoa_prototypes(meshes: &SectionMeshRefs) -> Vec<SectionConfig> {
    prototypes(&CARGOA_PARTS, &CARGOA_EDGES, meshes, false)
}

fn ship_sections(
    specs: &[PartSpec],
    grade: ShipGrade,
    controller_modifications: &[SectionModification],
) -> Vec<SpaceshipSectionConfig> {
    specs
        .iter()
        .map(|&spec| {
            let mut modifications = Vec::new();
            let mut prototype = spec.prototype.to_string();
            if grade == ShipGrade::Enemy {
                match spec.role {
                    PartRole::Hull => modifications.push(SectionModification::SetHealth(
                        (spec.health * 0.58).max(35.0),
                    )),
                    PartRole::Thruster => {
                        modifications.push(SectionModification::SetHealth(25.0));
                    }
                    PartRole::Controller => {
                        modifications.push(SectionModification::SetHealth(140.0));
                    }
                    PartRole::Turret => prototype.push_str("_light"),
                    PartRole::Torpedo => {}
                }
            }
            if matches!(spec.role, PartRole::Controller) {
                modifications.extend_from_slice(controller_modifications);
            }
            SpaceshipSectionConfig {
                id: spec.id.to_string(),
                position: spec.center(),
                rotation: spec.rotation(),
                source: SectionSource::Prototype(prototype),
                modifications,
            }
        })
        .collect()
}

pub(crate) fn racer_sections(
    grade: ShipGrade,
    controller_modifications: Vec<SectionModification>,
) -> Vec<SpaceshipSectionConfig> {
    ship_sections(&RACER_PARTS, grade, &controller_modifications)
}

pub(crate) fn cargob_sections() -> Vec<SpaceshipSectionConfig> {
    ship_sections(&CARGOB_PARTS, ShipGrade::Player, &[])
}

pub(crate) fn cargoa_sections() -> Vec<SpaceshipSectionConfig> {
    ship_sections(&CARGOA_PARTS, ShipGrade::Player, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parts_ship_has_one_connected_mate_graph() {
        for (specs, edges) in [
            (RACER_PARTS.as_slice(), RACER_EDGES.as_slice()),
            (CARGOB_PARTS.as_slice(), CARGOB_EDGES.as_slice()),
            (CARGOA_PARTS.as_slice(), CARGOA_EDGES.as_slice()),
        ] {
            let points: Vec<_> = specs
                .iter()
                .enumerate()
                .map(|(index, _)| SectionLinkPoints(link_points(specs, edges, index)))
                .collect();
            let placed: Vec<_> = specs
                .iter()
                .zip(&points)
                .map(|(spec, points)| PlacedSectionLinkPoints {
                    position: spec.center(),
                    rotation: spec.rotation(),
                    link_points: points,
                })
                .collect();
            let mates = derive_link_point_graph(&placed).unwrap();
            assert_eq!(mates.len(), edges.len());
        }
    }

    #[test]
    fn part_mesh_offsets_preserve_recipe_assembly_bounds() {
        for specs in [&RACER_PARTS[..7], &CARGOB_PARTS[..7], &CARGOA_PARTS] {
            for spec in specs {
                let rendered_min = spec.center() + spec.mesh_offset() + spec.bbox_min;
                let rendered_max = spec.center() + spec.mesh_offset() + spec.bbox_max;
                assert_eq!(rendered_min, spec.origin + spec.bbox_min);
                assert_eq!(rendered_max, spec.origin + spec.bbox_max);
            }
        }
    }
}
