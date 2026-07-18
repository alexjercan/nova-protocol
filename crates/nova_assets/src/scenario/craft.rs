//! The two Kenney "cut cube" ships - the racer and the cargob - as reusable base
//! SECTION PROTOTYPES plus the ship builders that reference them (task
//! craft-ships-into-base). The racer is the campaign player ship AND the
//! scavenger enemy; the cargob is the Rust Tally boss.
//!
//! Each cut cube is a prototype in the base catalog (`racer_cube_*`,
//! `cargob_cube_*`, plus weak `racer_light_cube_*` turret variants for AI
//! enemies), so BOTH the base campaign AND downloaded mods build these ships by
//! referencing prototype ids - the ship geometry lives in ONE place. The cube
//! layout is the `*_CUBES` tables (cut by `scripts/cut-obj-into-hulls.py`).
//! Meshes are `self://gltf/{racer,cargob}/`; sounds reuse the shared base refs.

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

// The cargoa - a wider, unarmed cargo hauler (no turrets, no torpedoes): hull
// cubes plus two rear thrusters and a hollow-core controller. Used for NEUTRAL
// ships (the campaign hauler, the ledger scout/escort, the waystation haulers).
const CARGOA_CUBES: &[(i32, i32, i32)] = &[
    (0, 0, 0), (0, 0, 1), (0, 0, 2), (0, 0, -1), (0, 1, 1), (0, 1, 2), (0, 1, -1), (0, 1, -2),
    (0, 2, 0), (0, 2, 1), (0, 2, -1), (0, 2, -2), (1, 0, 0), (1, 0, 1), (1, 0, 2), (1, 0, -1),
    (1, 1, 0), (1, 1, 1), (1, 1, 2), (1, 1, -1), (1, 1, -2), (1, 2, 0), (1, 2, 1), (1, 2, -1),
    (1, 2, -2), (2, 0, 0), (2, 0, 1), (2, 0, -1), (2, 1, 0), (2, 1, 1), (2, 1, 2), (2, 1, -1),
    (-1, 0, 0), (-1, 0, 1), (-1, 0, 2), (-1, 0, -1), (-1, 1, 0), (-1, 1, 1), (-1, 1, 2),
    (-1, 1, -1), (-1, 1, -2), (-1, 2, 0), (-1, 2, 1), (-1, 2, -1), (-1, 2, -2), (-2, 0, 0),
    (-2, 0, 1), (-2, 0, -1), (-2, 1, 0), (-2, 1, 1), (-2, 1, 2), (-2, 1, -1),
];

/// The two racer turret cubes, in section-id form - the player binds both to its
/// fire input.
pub(crate) const RACER_TURRET_IDS: [&str; 2] = ["cube_i1_j0_km1", "cube_im1_j0_km1"];

// Player-grade per-section health (the prototype baseline). Enemy ships override
// hull/thruster/controller down via SetHealth and swap to the light turret.
const RACER_HULL_HP: f32 = 60.0;
const RACER_THRUSTER_HP: f32 = 70.0;
const RACER_CONTROLLER_HP: f32 = 100.0;
const RACER_TURRET_HP: f32 = 130.0;
const RACER_LIGHT_TURRET_HP: f32 = 60.0;
const CARGOB_HULL_HP: f32 = 70.0;
const CARGOA_HULL_HP: f32 = 70.0;
// AI-scavenger health overrides on the shared player-grade prototypes.
const ENEMY_HULL_HP: f32 = 35.0;
const ENEMY_THRUSTER_HP: f32 = 25.0;
const ENEMY_CONTROLLER_HP: f32 = 45.0;

/// Who flies a racer. Drives HP and turret power: the player ship is sturdy with
/// full-power PDCs; an AI scavenger is squishier (SetHealth overrides) and shoots
/// the weak `racer_light_*` turret.
#[derive(Clone, Copy, PartialEq)]
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

/// The per-cube section id used ON A SHIP (also the input-mapping key).
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
        // The racer/cargob/cargoa cut-cube prototypes are dozens of near-
        // identical hull tiles that only make sense assembled into a ship - keep
        // them out of the editor sandbox's section palette.
        hide_in_editor: true,
    }
}

fn hull_kind(mesh: AssetRef<WorldAsset>) -> SectionKind {
    SectionKind::Hull(HullSectionConfig {
        render_mesh: Some(mesh),
        render_mesh_transform: None,
    })
}

/// A racer/cargob mount cube carries the turret's fixed base mesh: the turret
/// kinematics ride on top of the cut cube. The ship section is rolled 90deg so
/// the mount seats on the hull; the mesh is counter-rolled so the cube renders
/// upright. Returns (section roll, mesh counter-roll). `i > 0` is starboard.
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
        rcs_loop_sound: Some(m.controller_rcs_loop_sound.clone()),
    })
}

fn rect_exhaust(
    m: &SectionMeshRefs,
    cube: AssetRef<WorldAsset>,
    off: Vec3,
    w: f32,
    h: f32,
) -> SectionKind {
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

fn proto(id: String, name: String, desc: &str, hp: f32, m: &SectionMeshRefs, kind: SectionKind, collider: Option<SectionCollider>) -> SectionConfig {
    SectionConfig {
        base: cube_base(id, name, desc, hp, m, collider),
        kind,
    }
}

// ---------------------------------------------------------------------------
// PROTOTYPES (added to the base section catalog by `sections::build_sections`).
// ---------------------------------------------------------------------------

/// The racer's section prototypes: one player-grade prototype per cut cube
/// (`racer_cube_*`), plus a weak `racer_light_cube_*` variant for each turret
/// cube (the AI-enemy gun).
pub(crate) fn racer_prototypes(m: &SectionMeshRefs) -> Vec<SectionConfig> {
    let desc = "A cut hull cube of the Kenney craft_racer.";
    let mut out = Vec::new();
    for &(i, j, k) in RACER_CUBES {
        let s = stem(i, j, k);
        let id = format!("racer_{s}");
        let mesh = cube_mesh("racer", i, j, k);
        match (i, j, k) {
            (1, 0, -1) | (-1, 0, -1) => {
                let (_, mesh_rot) = turret_side(i);
                let root = mounted_turret_root(m, mesh.clone(), mesh_rot, turret_fire_rate(ShipGrade::Player));
                out.push(proto(id, format!("Racer Turret ({i},{j},{k})"), desc, RACER_TURRET_HP, m, turret_kind(m, root, ShipGrade::Player), None));
                let lroot = mounted_turret_root(m, mesh, mesh_rot, turret_fire_rate(ShipGrade::Enemy));
                out.push(proto(format!("racer_light_{s}"), format!("Racer Light Turret ({i},{j},{k})"), desc, RACER_LIGHT_TURRET_HP, m, turret_kind(m, lroot, ShipGrade::Enemy), None));
            }
            (1, 0, 2) | (-1, 0, 2) => {
                let off_x = if i > 0 { -0.35 } else { 0.35 };
                out.push(proto(id, format!("Racer Thruster ({i},{j},{k})"), desc, RACER_THRUSTER_HP, m, rect_exhaust(m, mesh, Vec3::new(off_x, 0.0, -0.2), 0.3, 0.5), None));
            }
            (0, 1, 0) => {
                out.push(proto(id, "Racer Controller".to_string(), desc, RACER_CONTROLLER_HP, m, controller_kind(m, Some(mesh), 800.0), None));
            }
            _ => {
                out.push(proto(id, format!("Racer Cube ({i},{j},{k})"), desc, RACER_HULL_HP, m, hull_kind(mesh), None));
            }
        }
    }
    out
}

/// The cargob boss's section prototypes: one prototype per cut cube
/// (`cargob_cube_*`) plus the hollow-core controller (`cargob_core_controller`).
pub(crate) fn cargob_prototypes(m: &SectionMeshRefs) -> Vec<SectionConfig> {
    let desc = "A cut hull cube of the Kenney craft_cargoB.";
    let mut out = Vec::new();
    for &(i, j, k) in CARGOB_CUBES {
        let s = stem(i, j, k);
        let id = format!("cargob_{s}");
        let mesh = cube_mesh("cargob", i, j, k);
        match (i, j, k) {
            (1, 2, 0) | (-1, 2, 0) => {
                let (_, mesh_rot) = turret_side(i);
                let root = mounted_turret_root(m, mesh, mesh_rot, turret_fire_rate(ShipGrade::Player));
                out.push(proto(id, format!("Cargo Turret ({i},{j},{k})"), desc, RACER_TURRET_HP, m, turret_kind(m, root, ShipGrade::Player), None));
            }
            (1, 1, 2) | (-1, 1, 2) => {
                out.push(proto(id, format!("Cargo Thruster ({i},{j},{k})"), desc, RACER_THRUSTER_HP, m, rect_exhaust(m, mesh, Vec3::new(0.0, 0.0, 0.5), 0.4, 0.6), None));
            }
            (1, 1, -2) | (-1, 1, -2) => {
                out.push(proto(id, format!("Cargo Torpedo Bay ({i},{j},{k})"), desc, 100.0, m, cargob_torpedo_kind(m, mesh), None));
            }
            // Beveled top-front corners: the cut hull does not fill the cell, so
            // tighten the collider to the mesh (mass + hitbox only).
            (1, 2, -2) | (-1, 2, -2) => {
                out.push(proto(id, format!("Cargo Cube ({i},{j},{k})"), desc, CARGOB_HULL_HP, m, hull_kind(mesh), Some(SectionCollider::Cuboid { size: Vec3::splat(0.8) })));
            }
            _ => {
                out.push(proto(id, format!("Cargo Cube ({i},{j},{k})"), desc, CARGOB_HULL_HP, m, hull_kind(mesh), None));
            }
        }
    }
    out.push(proto(
        "cargob_core_controller".to_string(),
        "Core Controller".to_string(),
        "The ship's controller, in the hollow core cell.",
        RACER_CONTROLLER_HP,
        m,
        controller_kind(m, None, 800.0),
        None,
    ));
    out
}

/// The cargoa hauler's section prototypes: one prototype per cut cube
/// (`cargoa_cube_*`) plus the hollow-core controller (`cargoa_core_controller`).
/// Unarmed - hull cubes and two rear thrusters only - so it fits neutral,
/// no-guns roles (campaign hauler, ledger scout/escort, waystation traffic).
pub(crate) fn cargoa_prototypes(m: &SectionMeshRefs) -> Vec<SectionConfig> {
    let desc = "A cut hull cube of the Kenney craft_cargoA.";
    let mut out = Vec::new();
    for &(i, j, k) in CARGOA_CUBES {
        let s = stem(i, j, k);
        let id = format!("cargoa_{s}");
        let mesh = cube_mesh("cargoa", i, j, k);
        match (i, j, k) {
            (1, 1, 2) | (-1, 1, 2) => {
                let off_x = if i > 0 { 0.2 } else { -0.2 };
                out.push(proto(id, format!("Cargo Thruster ({i},{j},{k})"), desc, RACER_THRUSTER_HP, m, rect_exhaust(m, mesh, Vec3::new(off_x, -0.1, 0.4), 0.56, 0.4), None));
            }
            _ => {
                out.push(proto(id, format!("Cargo Cube ({i},{j},{k})"), desc, CARGOA_HULL_HP, m, hull_kind(mesh), None));
            }
        }
    }
    out.push(proto(
        "cargoa_core_controller".to_string(),
        "Core Controller".to_string(),
        "The ship's controller, in the hollow core cell.",
        RACER_CONTROLLER_HP,
        m,
        controller_kind(m, None, 800.0),
        None,
    ));
    out
}

// ---------------------------------------------------------------------------
// SHIP BUILDERS (compact prototype references; mods author the same shape).
// ---------------------------------------------------------------------------

fn ship_section(
    id: String,
    position: Vec3,
    rotation: Quat,
    proto_id: String,
    modifications: Vec<SectionModification>,
) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id,
        position,
        rotation,
        source: SectionSource::Prototype(proto_id),
        modifications,
    }
}

/// The racer's 18 sections as prototype references. Player grade uses the full
/// prototypes as-is; Enemy grade swaps the turrets to `racer_light_*` and
/// SetHealth-nerfs hull/thruster/controller. `controller_mods` (the shakedown
/// tutorial's DisableVerbs) ride on the controller cube.
pub(crate) fn racer_sections(
    grade: ShipGrade,
    controller_mods: Vec<SectionModification>,
) -> Vec<SpaceshipSectionConfig> {
    let enemy = grade == ShipGrade::Enemy;
    let mut out = Vec::new();
    for &(i, j, k) in RACER_CUBES {
        let s = stem(i, j, k);
        let pos = Vec3::new(i as f32, j as f32, k as f32);
        let (rotation, proto_id, enemy_hp) = match (i, j, k) {
            (1, 0, -1) | (-1, 0, -1) => {
                let (sec_rot, _) = turret_side(i);
                // Enemy turrets are the weak `racer_light_*` prototype (health
                // baked in), so they take no SetHealth override.
                let id = if enemy { format!("racer_light_{s}") } else { format!("racer_{s}") };
                (sec_rot, id, None)
            }
            (1, 0, 2) | (-1, 0, 2) => (Quat::IDENTITY, format!("racer_{s}"), Some(ENEMY_THRUSTER_HP)),
            (0, 1, 0) => (Quat::IDENTITY, format!("racer_{s}"), Some(ENEMY_CONTROLLER_HP)),
            _ => (Quat::IDENTITY, format!("racer_{s}"), Some(ENEMY_HULL_HP)),
        };
        let mut mods = Vec::new();
        if enemy {
            if let Some(hp) = enemy_hp {
                mods.push(SectionModification::SetHealth(hp));
            }
        }
        if (i, j, k) == (0, 1, 0) {
            mods.extend(controller_mods.clone());
        }
        out.push(ship_section(s, pos, rotation, proto_id, mods));
    }
    out
}

/// The cargob boss's sections as prototype references (always boss grade).
pub(crate) fn cargob_sections() -> Vec<SpaceshipSectionConfig> {
    let mut out = Vec::new();
    for &(i, j, k) in CARGOB_CUBES {
        let s = stem(i, j, k);
        let pos = Vec3::new(i as f32, j as f32, k as f32);
        let rotation = match (i, j, k) {
            (1, 2, 0) | (-1, 2, 0) => turret_side(i).0,
            _ => Quat::IDENTITY,
        };
        out.push(ship_section(s.clone(), pos, rotation, format!("cargob_{s}"), vec![]));
    }
    out.push(ship_section(
        "core_controller".to_string(),
        Vec3::new(0.0, 1.0, 0.0),
        Quat::IDENTITY,
        "cargob_core_controller".to_string(),
        vec![],
    ));
    out
}

/// The cargoa hauler's sections as prototype references (a single neutral grade;
/// the ship carries no weapons). All cubes sit at identity rotation; the hollow
/// core (0,1,0) gets the `cargoa_core_controller`.
pub(crate) fn cargoa_sections() -> Vec<SpaceshipSectionConfig> {
    let mut out = Vec::new();
    for &(i, j, k) in CARGOA_CUBES {
        let s = stem(i, j, k);
        let pos = Vec3::new(i as f32, j as f32, k as f32);
        out.push(ship_section(s.clone(), pos, Quat::IDENTITY, format!("cargoa_{s}"), vec![]));
    }
    out.push(ship_section(
        "core_controller".to_string(),
        Vec3::new(0.0, 1.0, 0.0),
        Quat::IDENTITY,
        "cargoa_core_controller".to_string(),
        vec![],
    ));
    out
}
