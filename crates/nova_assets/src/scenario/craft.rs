//! The two Kenney "cut cube" ships - the racer and the cargob - as reusable
//! inline section builders, moved out of the former example mods into the base
//! game (task craft-ships-into-base). The racer is the campaign player ship AND
//! the scavenger enemy; the cargob is the Rust Tally boss.
//!
//! Each ship's cube layout lives here in ONE place (the `*_CUBES` tables, cut by
//! `scripts/cut-obj-into-hulls.py`), and `ShipGrade` drives per-section HP and
//! turret strength directly - so a player racer and a weaker AI racer come from
//! the same builder without section-modification overlays. Meshes live under
//! `self://gltf/{racer,cargob}/`; sounds reuse the shared base-section refs.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use crate::sections::{turret_joint_tree, SectionMeshRefs};

// Cube grid coordinates (i, j, k) of each ship, one per cut `.glb` mesh.
const RACER_CUBES: &[(i32, i32, i32)] = &[
    (0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, -1), (0, 0, -2), (0, 1, 0), (0, 1, 1), (0, 1, 2),
    (0, 1, -1), (0, 1, -2), (1, 0, 0), (1, 0, 1), (1, 0, 2), (1, 0, -1), (-1, 0, 0),
    (-1, 0, 1), (-1, 0, 2), (-1, 0, -1),
];
const CARGOB_CUBES: &[(i32, i32, i32)] = &[
    (0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, -1), (0, 1, 2), (0, 1, -1), (0, 1, -2), (0, 2, 0),
    (0, 2, 1), (0, 2, 2), (0, 2, -1), (0, 2, -2), (1, 0, 0), (1, 0, 1), (1, 0, 2), (1, 0, -1),
    (1, 0, -2), (1, 1, 0), (1, 1, 1), (1, 1, 2), (1, 1, -1), (1, 1, -2), (1, 2, 0), (1, 2, 1),
    (1, 2, 2), (1, 2, -1), (1, 2, -2), (-1, 0, 0), (-1, 0, 1), (-1, 0, 2), (-1, 0, -1),
    (-1, 0, -2), (-1, 1, 0), (-1, 1, 1), (-1, 1, 2), (-1, 1, -1), (-1, 1, -2), (-1, 2, 0),
    (-1, 2, 1), (-1, 2, 2), (-1, 2, -1), (-1, 2, -2),
];

/// The two racer turret cubes, in section-id form - the player binds both to its
/// fire input.
pub(crate) const RACER_TURRET_IDS: [&str; 2] = ["cube_i1_j0_km1", "cube_im1_j0_km1"];

/// Who flies a racer. Drives HP and turret power: the player ship is sturdy with
/// full-power PDCs; an AI scavenger is squishier and shoots a light turret.
#[derive(Clone, Copy)]
pub(crate) enum ShipGrade {
    Player,
    Enemy,
}

fn enc(n: i32) -> String {
    if n < 0 {
        format!("m{}", -n)
    } else {
        n.to_string()
    }
}

fn stem(i: i32, j: i32, k: i32) -> String {
    format!("cube_i{}_j{}_k{}", enc(i), enc(j), enc(k))
}

fn cube_mesh(sub: &str, i: i32, j: i32, k: i32) -> AssetRef<WorldAsset> {
    AssetRef::from(format!("self://gltf/{sub}/{}.glb#Scene0", stem(i, j, k)))
}

fn cube_base(
    id: String,
    name: String,
    description: &str,
    health: f32,
    m: &SectionMeshRefs,
    collider: Option<SectionCollider>,
) -> BaseSectionConfig {
    BaseSectionConfig {
        id,
        name,
        description: description.to_string(),
        mass: 1.0,
        health,
        impact_sound: Some(m.section_impact_sound.clone()),
        destroy_sound: Some(m.section_destroy_sound.clone()),
        collider,
    }
}

fn hull_kind(mesh: AssetRef<WorldAsset>) -> SectionKind {
    SectionKind::Hull(HullSectionConfig {
        render_mesh: Some(mesh),
        render_mesh_transform: None,
    })
}

/// A racer/cargob mount cube carries the turret's fixed base mesh: the turret
/// kinematics ride on top of the cut cube. The section is rolled 90deg so the
/// mount seats on the hull; the mesh is counter-rolled so the cube renders
/// upright. `i > 0` is the starboard side (mirror the port side).
fn turret_side(i: i32) -> (Quat, Quat) {
    let h = std::f32::consts::FRAC_PI_2;
    if i > 0 {
        (Quat::from_rotation_z(-h), Quat::from_rotation_z(h))
    } else {
        (Quat::from_rotation_z(h), Quat::from_rotation_z(-h))
    }
}

fn mounted_turret_root(
    m: &SectionMeshRefs,
    cube: AssetRef<WorldAsset>,
    mesh_rot: Quat,
    fire_rate: f32,
) -> TurretJoint {
    let mut root = turret_joint_tree(&m.turret_yaw, &m.turret_pitch, &m.turret_barrel, fire_rate);
    // The shared tree's fixed base sits at (0,-0.5,0); mount the cube on it and
    // counter-roll so the cube renders upright despite the section roll.
    root.render_mesh = Some(cube);
    root.render_mesh_transform = Some(RenderMeshTransform {
        position: Vec3::new(0.0, 0.5, 0.0),
        rotation: mesh_rot,
    });
    root
}

fn turret_kind(m: &SectionMeshRefs, root: TurretJoint, grade: ShipGrade) -> SectionKind {
    let cfg = match grade {
        // Full-power PDC (matches better_turret_section).
        ShipGrade::Player => TurretSectionConfig {
            root,
            muzzle_speed: 100.0,
            projectile_lifetime: 5.0,
            bullet_damage: 4.0,
            bullet_kind: DamageType::Kinetic,
            projectile_render_mesh: None,
            fire_sound: Some(m.turret_fire_sound.clone()),
            dry_fire_sound: Some(m.turret_dry_fire_sound.clone()),
            ammo_capacity: Some(500),
            reload: Some(SectionReloadConfig {
                reload_time: 3.0,
                rounds_per_cycle: 500,
                only_when_empty: true,
            }),
        },
        // Scavenger grade (matches light_turret_section): quarter fire rate,
        // slower rounds, a fifth of the damage.
        ShipGrade::Enemy => TurretSectionConfig {
            root,
            muzzle_speed: 60.0,
            projectile_lifetime: 5.0,
            bullet_damage: representative_kinetic_damage(0.05, 60.0),
            bullet_kind: DamageType::Kinetic,
            projectile_render_mesh: None,
            fire_sound: Some(m.turret_fire_sound.clone()),
            dry_fire_sound: Some(m.turret_dry_fire_sound.clone()),
            ammo_capacity: Some(150),
            reload: Some(SectionReloadConfig {
                reload_time: 2.5,
                rounds_per_cycle: 150,
                only_when_empty: true,
            }),
        },
    };
    SectionKind::Turret(cfg)
}

fn turret_fire_rate(grade: ShipGrade) -> f32 {
    match grade {
        ShipGrade::Player => 100.0,
        ShipGrade::Enemy => 25.0,
    }
}

fn controller_kind(
    m: &SectionMeshRefs,
    render_mesh: Option<AssetRef<WorldAsset>>,
    max_torque: f32,
) -> SectionKind {
    SectionKind::Controller(ControllerSectionConfig {
        frequency: 4.0,
        damping_ratio: 4.0,
        max_torque,
        render_mesh,
        render_mesh_transform: None,
        lock_on_sound: Some(m.controller_lock_on_sound.clone()),
        lock_off_sound: Some(m.controller_lock_off_sound.clone()),
        radar_deny_sound: Some(m.controller_radar_deny_sound.clone()),
        radar_retarget_sound: Some(m.controller_radar_retarget_sound.clone()),
        safety_on_sound: Some(m.controller_safety_on_sound.clone()),
    })
}

fn rect_exhaust(m: &SectionMeshRefs, cube: AssetRef<WorldAsset>, off: Vec3, w: f32, h: f32) -> SectionKind {
    SectionKind::Thruster(ThrusterSectionConfig {
        magnitude: 1.0,
        render_mesh: Some(cube),
        render_mesh_transform: None,
        loop_sound: Some(m.thruster_loop_sound.clone()),
        exhaust: Some(ThrusterExhaust {
            offset: off,
            rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            shape: ThrusterExhaustConfig {
                geometry: ThrusterExhaustShape::Rect,
                width: w,
                height: h,
                ..default()
            },
        }),
    })
}

fn inline(
    id: String,
    position: Vec3,
    rotation: Quat,
    base: BaseSectionConfig,
    kind: SectionKind,
) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id,
        position,
        rotation,
        source: SectionSource::Inline(SectionConfig { base, kind }),
        modifications: vec![],
    }
}

/// Build the racer's 18 sections at the given grade. `controller_mods` are
/// attached to the controller cube (the shakedown tutorial withholds GOTO/LOCK/
/// ORBIT there).
pub(crate) fn racer_sections(
    grade: ShipGrade,
    controller_mods: Vec<SectionModification>,
) -> Vec<SpaceshipSectionConfig> {
    let m = SectionMeshRefs::from_paths();
    let (hull_hp, thruster_hp, controller_hp, turret_hp) = match grade {
        ShipGrade::Player => (60.0, 70.0, 100.0, 130.0),
        ShipGrade::Enemy => (35.0, 25.0, 45.0, 60.0),
    };
    let desc = "A cut hull cube of the Kenney craft_racer.";
    let mut out = Vec::new();
    for &(i, j, k) in RACER_CUBES {
        let id = stem(i, j, k);
        let pos = Vec3::new(i as f32, j as f32, k as f32);
        let mesh = cube_mesh("racer", i, j, k);
        let mut section = match (i, j, k) {
            (1, 0, -1) | (-1, 0, -1) => {
                let (sec_rot, mesh_rot) = turret_side(i);
                let root = mounted_turret_root(&m, mesh, mesh_rot, turret_fire_rate(grade));
                let base = cube_base(
                    id.clone(),
                    format!("Racer Turret ({i},{j},{k})"),
                    desc,
                    turret_hp,
                    &m,
                    None,
                );
                inline(id.clone(), pos, sec_rot, base, turret_kind(&m, root, grade))
            }
            (1, 0, 2) | (-1, 0, 2) => {
                let off_x = if i > 0 { -0.35 } else { 0.35 };
                let base = cube_base(
                    id.clone(),
                    format!("Racer Thruster ({i},{j},{k})"),
                    desc,
                    thruster_hp,
                    &m,
                    None,
                );
                let kind = rect_exhaust(&m, mesh, Vec3::new(off_x, 0.0, -0.2), 0.3, 0.5);
                inline(id.clone(), pos, Quat::IDENTITY, base, kind)
            }
            (0, 1, 0) => {
                let base = cube_base(id.clone(), "Racer Controller".to_string(), desc, controller_hp, &m, None);
                inline(id.clone(), pos, Quat::IDENTITY, base, controller_kind(&m, Some(mesh), 800.0))
            }
            _ => {
                let base = cube_base(id.clone(), format!("Racer Cube ({i},{j},{k})"), desc, hull_hp, &m, None);
                inline(id.clone(), pos, Quat::IDENTITY, base, hull_kind(mesh))
            }
        };
        if (i, j, k) == (0, 1, 0) {
            section.modifications = controller_mods.clone();
        }
        out.push(section);
    }
    out
}

fn cargob_torpedo_kind(m: &SectionMeshRefs, cube: AssetRef<WorldAsset>) -> SectionKind {
    SectionKind::Torpedo(TorpedoSectionConfig {
        render_mesh: Some(cube),
        render_mesh_transform: None,
        projectile_render_mesh: None,
        spawn_offset: Vec3::NEG_Z * 2.0,
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
        launch_sound: Some(m.torpedo_launch_sound.clone()),
        detonation_sound: Some(m.section_destroy_sound.clone()),
        ammo_capacity: Some(6),
        reload: Some(SectionReloadConfig {
            reload_time: 4.0,
            rounds_per_cycle: 1,
            only_when_empty: false,
        }),
    })
}

/// The Rust Tally boss: the cargob's 42 cut cubes plus a core controller in the
/// hollow centre. Strong (boss-grade) turrets and two torpedo tubes; kept
/// sturdier than a racer.
pub(crate) fn cargob_sections() -> Vec<SpaceshipSectionConfig> {
    let m = SectionMeshRefs::from_paths();
    let desc = "A cut hull cube of the Kenney craft_cargoB.";
    let mut out = Vec::new();
    for &(i, j, k) in CARGOB_CUBES {
        let id = stem(i, j, k);
        let pos = Vec3::new(i as f32, j as f32, k as f32);
        let mesh = cube_mesh("cargob", i, j, k);
        let section = match (i, j, k) {
            (1, 2, 0) | (-1, 2, 0) => {
                let (sec_rot, mesh_rot) = turret_side(i);
                let root = mounted_turret_root(&m, mesh, mesh_rot, turret_fire_rate(ShipGrade::Player));
                let base = cube_base(id.clone(), format!("Cargo Turret ({i},{j},{k})"), desc, 130.0, &m, None);
                inline(id.clone(), pos, sec_rot, base, turret_kind(&m, root, ShipGrade::Player))
            }
            (1, 1, 2) | (-1, 1, 2) => {
                let base = cube_base(id.clone(), format!("Cargo Thruster ({i},{j},{k})"), desc, 70.0, &m, None);
                let kind = rect_exhaust(&m, mesh, Vec3::new(0.0, 0.0, 0.5), 0.4, 0.6);
                inline(id.clone(), pos, Quat::IDENTITY, base, kind)
            }
            (1, 1, -2) | (-1, 1, -2) => {
                let base = cube_base(id.clone(), format!("Cargo Torpedo Bay ({i},{j},{k})"), desc, 100.0, &m, None);
                inline(id.clone(), pos, Quat::IDENTITY, base, cargob_torpedo_kind(&m, mesh))
            }
            // Beveled top-front corners: the cut hull does not fill the cell, so
            // tighten the collider to the mesh (mass + hitbox only).
            (1, 2, -2) | (-1, 2, -2) => {
                let base = cube_base(
                    id.clone(),
                    format!("Cargo Cube ({i},{j},{k})"),
                    desc,
                    70.0,
                    &m,
                    Some(SectionCollider::Cuboid { size: Vec3::splat(0.8) }),
                );
                inline(id.clone(), pos, Quat::IDENTITY, base, hull_kind(mesh))
            }
            _ => {
                let base = cube_base(id.clone(), format!("Cargo Cube ({i},{j},{k})"), desc, 70.0, &m, None);
                inline(id.clone(), pos, Quat::IDENTITY, base, hull_kind(mesh))
            }
        };
        out.push(section);
    }
    // Core controller in the hollow (0,1,0) cell (no mesh - it sits inside).
    let base = cube_base(
        "core_controller".to_string(),
        "Core Controller".to_string(),
        "The ship's controller, in the hollow core cell.",
        100.0,
        &m,
        None,
    );
    out.push(inline(
        "core_controller".to_string(),
        Vec3::new(0.0, 1.0, 0.0),
        Quat::IDENTITY,
        base,
        controller_kind(&m, None, 800.0),
    ));
    out
}
