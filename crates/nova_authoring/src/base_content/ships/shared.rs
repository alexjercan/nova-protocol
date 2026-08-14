//! Semantic Kenney ship parts and the shipped Racer, CargoB, and CargoA assemblies.
//!
//! Part meshes are centered on tight primitive colliders. Structural edges come only from
//! authored link-point mates shared by the catalog prototypes and ship builders.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::base_content::{
    assets::BaseContentAssets,
    sections::{turret_joint_tree, UNIT_TURRET_MOUNT, UNIT_TURRET_SCALE},
};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ShipGrade {
    Player,
    Enemy,
}

#[derive(Clone, Copy)]
pub(super) enum PartRole {
    Hull,
    Thruster,
    Controller,
    Torpedo,
    Turret,
}

#[derive(Clone, Copy)]
pub(super) enum PartSide {
    None,
    Port,
    Starboard,
}

#[derive(Clone, Copy)]
pub(super) struct PartSpec {
    id: &'static str,
    prototype: &'static str,
    mesh: Option<&'static str>,
    pub(super) origin: Vec3,
    pub(super) bbox_min: Vec3,
    pub(super) bbox_max: Vec3,
    health: f32,
    role: PartRole,
    side: PartSide,
}

impl PartSpec {
    pub(super) fn center(self) -> Vec3 {
        self.origin + (self.bbox_min + self.bbox_max) * 0.5
    }

    fn size(self) -> Vec3 {
        self.bbox_max - self.bbox_min
    }

    pub(super) fn mesh_offset(self) -> Vec3 {
        self.origin - self.center()
    }

    pub(super) fn rotation(self) -> Quat {
        let quarter = std::f32::consts::FRAC_PI_2;
        match self.side {
            PartSide::None => Quat::IDENTITY,
            PartSide::Port => Quat::from_rotation_z(quarter),
            PartSide::Starboard => Quat::from_rotation_z(-quarter),
        }
    }
}

pub(super) const fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub(super) const fn part(
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

pub(super) const fn module(
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
        ..default()
    })
}

/// The sockets one authored adjacency gives a part: one per edge it takes part
/// in, sitting at the midpoint between the two part centres.
///
/// The normal is SNAPPED to a cardinal axis rather than left pointing at the
/// neighbour's centre. Cut parts sit wherever the craft's art put them, so a
/// raw centre-to-centre direction is oblique - the cargob's pod faces its
/// fuselage 36 degrees off -X - and anything mated onto that socket arrived
/// tilted by exactly that much, which is what made parts look like they only
/// fit the craft they were cut from. The snap is antisymmetric (see
/// [`cardinal_axis`]), so both ends of an edge stay exactly opposed and every
/// shipped mate survives.
pub(super) fn link_points(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    index: usize,
) -> Vec<LinkPoint> {
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
            // Snapped in SHIP space, not in the part's: both ends then read the
            // same axis off the same direction, whatever each part is rotated by.
            let direction = cardinal_axis(other_center - center);
            let world_position = (center + other_center) * 0.5;
            Some(LinkPoint {
                id: format!("to_{}", specs[other].id),
                position: rotation.inverse() * (world_position - center),
                normal: rotation.inverse() * direction,
            })
        })
        .collect()
}

/// Title-case one snake_case part id: `pod_starboard` -> `Pod Starboard`.
fn title_case(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn base_config(
    spec: PartSpec,
    family: &str,
    links: Vec<LinkPoint>,
    meshes: &BaseContentAssets,
) -> BaseSectionConfig {
    BaseSectionConfig {
        id: spec.prototype.to_string(),
        // Named for the craft it was cut off, not for its role alone: three
        // craft each contribute a "Nose" and a "Tail", and a bare role name
        // leaves a browser no way to tell them apart or to find the rest of a
        // set.
        name: format!("{family} // {}", title_case(spec.id)),
        description: format!("The {family}'s {} section.", spec.id.replace('_', " ")),
        mass: 1.0,
        health: spec.health,
        impact_sound: Some(meshes.section_impact_sound.clone()),
        destroy_sound: Some(meshes.section_destroy_sound.clone()),
        collider: Some(SectionCollider::Cuboid { size: spec.size() }),
        link_points: links,
        // Placeable now that editor placement MATES link points: a semantic
        // part only goes where its authored sockets say it does, which is what
        // hiding it was waiting for. The one exception is the turret modules -
        // see `prototypes`.
        hide_in_editor: matches!(spec.role, PartRole::Turret),
    }
}

fn hull_kind(spec: PartSpec) -> SectionKind {
    SectionKind::Hull(HullSectionConfig {
        render_mesh: spec.mesh.map(mesh_ref),
        render_mesh_transform: render_transform(spec),
    })
}

fn thruster_kind(spec: PartSpec, meshes: &BaseContentAssets) -> SectionKind {
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

fn controller_kind(spec: PartSpec, meshes: &BaseContentAssets) -> SectionKind {
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

fn torpedo_kind(spec: PartSpec, meshes: &BaseContentAssets) -> SectionKind {
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
        // The standard torpedo's damage (see `sections::standard`): a
        // connecting torpedo all but decides a small-craft fight, and the
        // counter is point defense, not armor. The cargob's tubes are the
        // campaign's torpedoes, so they hit like the catalog's.
        blast_damage: 750.0,
        blast_effect: None,
        launch_effect: None,
        launch_sound: Some(meshes.torpedo_launch_sound.clone()),
        detonation_sound: Some(meshes.section_destroy_sound.clone()),
        projectile_health: 1.0,
        ammo_capacity: Some(6),
        reload: Some(SectionReloadConfig {
            reload_time: 4.0,
            rounds_per_cycle: 1,
            only_when_empty: false,
        }),
    })
}

fn turret_kind(meshes: &BaseContentAssets, enemy: bool) -> SectionKind {
    let fire_rate = if enemy { 25.0 } else { 100.0 };
    let root = turret_joint_tree(
        &meshes.turret_yaw,
        &meshes.turret_pitch,
        &meshes.turret_barrel,
        fire_rate,
        // The shipped modules keep the unit-cube mount and size their art was
        // placed against, whatever their own collider says: changing either
        // MOVES or RESIZES the turret on every shipped ship.
        UNIT_TURRET_MOUNT,
        UNIT_TURRET_SCALE,
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

/// Every catalog prototype one craft's parts contribute, named for `family`.
///
/// The turret modules are catalog-only (`hide_in_editor`): they carry no mesh
/// of their own, so all six of them - port and starboard, across three craft,
/// doubled again by `light_turrets` - are the SAME PDC on the same joint tree.
/// Offering ten identical turrets buried the two that actually differ, and now
/// that a part mates onto any socket the same way up (see [`link_points`]),
/// there is nothing a per-craft copy could do that the standard turret cannot.
/// The ships still build from them, and mods can still name them.
pub(super) fn prototypes(
    specs: &[PartSpec],
    edges: &[(usize, usize)],
    family: &str,
    meshes: &BaseContentAssets,
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
            base: base_config(spec, family, links.clone(), meshes),
            kind,
        });
        if light_turrets && matches!(spec.role, PartRole::Turret) {
            let mut base = base_config(spec, family, links, meshes);
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

pub(super) fn ship_sections(
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
