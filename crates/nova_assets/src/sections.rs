use bevy::prelude::*;
use nova_gameplay::prelude::*;

// Per-section-type durability baselines (task 20260525-133004).
//
// Section TYPE governs how much damage a section effectively takes. With the
// single (kinetic) damage model in play today, "takes more damage" is simply
// "has less health", so the variation lives here in the health numbers rather
// than in a damage-interception system (a real per-damage-type resistance -
// AP/EMP - is the next pass, task 20260708-162005, and lands nova-side).
//
// Thrusters are exposed propulsion and go down fast (take MORE); turrets are
// armored weapon mounts and shrug off MORE (take LESS); the controller core and
// the torpedo bay sit at the mid baseline. Direction follows the task title and
// is a playtest knob - flipping "fragile vs tough" is a one-line change here.
// Per-section variants (a reinforced hull, a scavenger-grade turret) may deviate
// from their type baseline on purpose; these are the values they start from.
const THRUSTER_BASE_HEALTH: f32 = 70.0;
const CONTROLLER_BASE_HEALTH: f32 = 100.0;
const TURRET_BASE_HEALTH: f32 = 130.0;
const TORPEDO_BASE_HEALTH: f32 = 100.0;

// Authored per-hit Kinetic damage of the player's PDC (`better_turret`), a
// playtest knob (task 20260712-172035). A point-defense profile: LOW per-hit,
// HIGH rate (100 rounds/s). At 4.0 the PDC does ~400 DPS - clearly the stronger
// gun than the scavenger light turret (3.825/hit @ 25 rps ~ 96 DPS) - while a
// 100-HP asteroid now takes ~25 rounds (~0.25s of fire) instead of ~5, so a
// burst visibly chips it down rather than popping it in a blink (playtest: "PDC
// destroys asteroids/objects with one bullet"). Was ~20.25 (the old emergent
// per-hit); the drop also slows ship TTK ~5x, consistent with a PDC and with the
// shakedown pirate still dying in a short burst (~0.15s on a 60-HP hull).
const BETTER_TURRET_BULLET_DAMAGE: f32 = 4.0;

/// Build the shipped turret's kinematic joint tree: the exact chain the flat
/// config used to author 1:1 (spike 20260717-214834). base(fixed, at
/// (0,-0.5,0)) -> yaw(Y, meshed) -> pitch(X, meshed, -30..90 deg) ->
/// barrel(fixed, meshed) -> muzzle(fixed, fire point). `fire_rate` is per-muzzle
/// now; every other numeric value is preserved from the old fields.
pub(crate) fn turret_joint_tree(
    yaw_mesh: &AssetRef<WorldAsset>,
    pitch_mesh: &AssetRef<WorldAsset>,
    barrel_mesh: &AssetRef<WorldAsset>,
    fire_rate: f32,
) -> TurretJoint {
    TurretJoint {
        offset: Vec3::new(0.0, -0.5, 0.0),
        axis: None,
        speed: std::f32::consts::PI,
        min: None,
        max: None,
        render_mesh: None,
        render_mesh_transform: None,
        muzzle: None,
        children: vec![TurretJoint {
            offset: Vec3::new(0.0, 0.1, 0.0),
            axis: Some(Vec3::Y),
            speed: std::f32::consts::PI, // 180 degrees per second
            min: None,
            max: None,
            render_mesh: Some(yaw_mesh.clone()),
            render_mesh_transform: None,
            muzzle: None,
            children: vec![TurretJoint {
                offset: Vec3::new(0.0, 0.332706, 0.303954),
                axis: Some(Vec3::X),
                speed: std::f32::consts::PI, // 180 degrees per second
                min: Some(-std::f32::consts::FRAC_PI_6),
                max: Some(std::f32::consts::FRAC_PI_2),
                render_mesh: Some(pitch_mesh.clone()),
                render_mesh_transform: None,
                muzzle: None,
                children: vec![TurretJoint {
                    offset: Vec3::new(0.0, 0.128437, -0.110729),
                    axis: None,
                    speed: std::f32::consts::PI,
                    min: None,
                    max: None,
                    render_mesh: Some(barrel_mesh.clone()),
                    render_mesh_transform: None,
                    muzzle: None,
                    children: vec![TurretJoint {
                        offset: Vec3::new(0.0, 0.0, -1.2),
                        axis: None,
                        speed: std::f32::consts::PI,
                        min: None,
                        max: None,
                        render_mesh: None,
                        render_mesh_transform: None,
                        muzzle: Some(MuzzleConfig {
                            fire_rate,
                            muzzle_effect: None,
                        }),
                        children: vec![],
                    }],
                }],
            }],
        }],
    }
}

/// The render-mesh asset references the section catalog needs, as `AssetRef`s.
///
/// The catalog itself (`build_sections`) is defined ONCE and is agnostic to how
/// these refs were sourced. Production no longer builds sections from `GameAssets`
/// handles - it loads the serialized catalog (`assets/base/sections/base.content.ron`)
/// via `nova_modding`; the only remaining source is the RON generator/parity test,
/// which builds them from asset PATHS (`from_paths`) so the serialized section
/// configs carry authorable paths instead of opaque handles.
pub struct SectionMeshRefs {
    pub hull: AssetRef<WorldAsset>,
    pub turret_yaw: AssetRef<WorldAsset>,
    pub turret_pitch: AssetRef<WorldAsset>,
    pub turret_barrel: AssetRef<WorldAsset>,
    pub torpedo_bay: AssetRef<WorldAsset>,
    /// The turret fire sound, authored the same `self://` way as the meshes
    /// (task 20260717-002228). Serialized into the section config's `fire_sound`
    /// field so base turrets ship + reference their weapon sound through the
    /// scheme pipeline; `base/sounds/turret_fire.wav` resolves to the same handle
    /// the global bank loads, so the audible result is unchanged.
    pub turret_fire_sound: AssetRef<AudioSource>,
    /// The turret dry-fire click, authored like the fire sound (task
    /// 20260717-101624).
    pub turret_dry_fire_sound: AssetRef<AudioSource>,
    /// The torpedo bay launch sound (task 20260717-101624).
    pub torpedo_launch_sound: AssetRef<AudioSource>,
    /// The controller's radar/lock/safety feedback cues (task 20260717-101633).
    pub controller_lock_on_sound: AssetRef<AudioSource>,
    pub controller_lock_off_sound: AssetRef<AudioSource>,
    pub controller_radar_deny_sound: AssetRef<AudioSource>,
    pub controller_radar_retarget_sound: AssetRef<AudioSource>,
    pub controller_safety_on_sound: AssetRef<AudioSource>,
    /// Per-target hit/destruction voices, shared by every catalog section
    /// (task 20260717-101641); asteroids author the same two in scenario
    /// content.
    pub section_impact_sound: AssetRef<AudioSource>,
    pub section_destroy_sound: AssetRef<AudioSource>,
    /// The thruster engine hum (task 20260717-101650).
    pub thruster_loop_sound: AssetRef<AudioSource>,
}

impl SectionMeshRefs {
    /// Generation source: the same asset paths `GameAssets` loads them from, so
    /// the serialized section configs carry authorable paths.
    pub fn from_paths() -> Self {
        Self {
            hull: AssetRef::from("self://gltf/hull-01.glb#Scene0".to_string()),
            turret_yaw: AssetRef::from("self://gltf/turret-yaw-01.glb#Scene0".to_string()),
            turret_pitch: AssetRef::from("self://gltf/turret-pitch-01.glb#Scene0".to_string()),
            turret_barrel: AssetRef::from("self://gltf/turret-barrel-01.glb#Scene0".to_string()),
            torpedo_bay: AssetRef::from("self://gltf/torpedo-bay-01.glb#Scene0".to_string()),
            turret_fire_sound: AssetRef::from("self://sounds/turret_fire.wav".to_string()),
            turret_dry_fire_sound: AssetRef::from("self://sounds/dry_fire.wav".to_string()),
            torpedo_launch_sound: AssetRef::from("self://sounds/torpedo_launch.wav".to_string()),
            controller_lock_on_sound: AssetRef::from("self://sounds/lock_on.wav".to_string()),
            controller_lock_off_sound: AssetRef::from("self://sounds/lock_off.wav".to_string()),
            controller_radar_deny_sound: AssetRef::from("self://sounds/radar_deny.wav".to_string()),
            controller_radar_retarget_sound: AssetRef::from(
                "self://sounds/radar_retarget.wav".to_string(),
            ),
            controller_safety_on_sound: AssetRef::from("self://sounds/safety_on.wav".to_string()),
            section_impact_sound: AssetRef::from("self://sounds/impact.wav".to_string()),
            section_destroy_sound: AssetRef::from("self://sounds/explosion.wav".to_string()),
            thruster_loop_sound: AssetRef::from("self://sounds/thruster_loop.wav".to_string()),
        }
    }
}

/// The section catalog, built against `meshes` for its render-mesh refs. The
/// single source of truth for the built-in sections; both the production
/// registry and the RON generator go through here.
pub fn build_sections(meshes: &SectionMeshRefs) -> Vec<SectionConfig> {
    let mut sections = vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: "reinforced_hull_section".to_string(),
                name: "Reinforced Hull Section".to_string(),
                description: "A reinforced hull section for spaceships.".to_string(),
                mass: 1.0,
                health: 200.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "basic_thruster_section".to_string(),
                name: "Basic Thruster Section".to_string(),
                description: "A basic thruster section for spaceships.".to_string(),
                mass: 1.0,
                // Exposed propulsion: fragile, takes more damage per hit than an
                // armored mount (task 20260525-133004).
                health: THRUSTER_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: None,
                render_mesh_transform: None,
                loop_sound: Some(meshes.thruster_loop_sound.clone()),
                exhaust: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "basic_controller_section".to_string(),
                name: "Basic Controller Section".to_string(),
                description: "A basic controller section for spaceships.".to_string(),
                mass: 1.0,
                // Command core: mid durability baseline (task 20260525-133004).
                health: CONTROLLER_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Controller(ControllerSectionConfig {
                frequency: 4.0,
                damping_ratio: 4.0,
                // Torque budget (task 20260709-095043): 40.0 keeps the
                // asteroid_field flagship (max principal inertia ~10.8) at
                // its familiar ~88 deg/s command rate while a hull+thruster
                // remnant hits the 240 deg/s ceiling - weight becomes legible
                // without regressing the baseline feel. 100.0 saturated
                // nothing (every build turned identically at the old fixed
                // slew). Flip-time optima per ship are tabled in
                // docs/2026-07-09-flight-feel-retune.md; playtest owns the
                // final number.
                max_torque: 40.0,
                // Full flight-verb loadout by default (no WithheldVerbs on the
                // built controller). Scenarios withhold a verb via a
                // `DisableVerb` section modification or the `SetControllerVerb`
                // action (the shakedown's GOTO-off intro) rather than baking it
                // into this shared catalog entry, which the pirate reuses too.
                render_mesh: None,
                render_mesh_transform: None,
                lock_on_sound: Some(meshes.controller_lock_on_sound.clone()),
                lock_off_sound: Some(meshes.controller_lock_off_sound.clone()),
                radar_deny_sound: Some(meshes.controller_radar_deny_sound.clone()),
                radar_retarget_sound: Some(meshes.controller_radar_retarget_sound.clone()),
                safety_on_sound: Some(meshes.controller_safety_on_sound.clone()),
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "better_turret_section".to_string(),
                name: "Better Turret Section".to_string(),
                description: "A better turret section for spaceships.".to_string(),
                mass: 1.0,
                // Armored weapon mount: tough, takes less damage per hit than an
                // exposed section (task 20260525-133004).
                health: TURRET_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                // base(fixed) -> yaw(Y) -> pitch(X) -> barrel(fixed) -> muzzle,
                // migrated 1:1 from the old flat fields (spike 20260717-214834).
                root: turret_joint_tree(
                    &meshes.turret_yaw,
                    &meshes.turret_pitch,
                    &meshes.turret_barrel,
                    100.0,
                ),
                muzzle_speed: 100.0,
                projectile_lifetime: 5.0,
                // Point-defense per-hit (task 20260712-172035): low damage, high
                // rate. See BETTER_TURRET_BULLET_DAMAGE. (Was the old emergent
                // per-hit representative_kinetic_damage(0.1, 100.0) ~= 20.25,
                // which vaporised asteroids in a blink.)
                bullet_damage: BETTER_TURRET_BULLET_DAMAGE,
                // Kinetic loadout (the slot's authored default; task
                // 20260712-133349).
                bullet_kind: DamageType::Kinetic,
                projectile_render_mesh: None,
                fire_sound: Some(meshes.turret_fire_sound.clone()),
                dry_fire_sound: Some(meshes.turret_dry_fire_sound.clone()),
                // ~5s of sustained fire at 100 rounds/s. Generous on purpose:
                // the player should feel the limit without running dry in a
                // normal engagement. Playtest knob.
                ammo_capacity: Some(500),
                // Discrete auto-reload: dump the magazine, then a ~3s cycle
                // refills it to full. Running dry is a brief cadence beat, never
                // a death, so finite ammo is safe to turn on (task 20260717-085640).
                reload: Some(SectionReloadConfig {
                    reload_time: 3.0,
                    rounds_per_cycle: 500,
                    only_when_empty: true,
                }),
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "light_hull_section".to_string(),
                name: "Light Hull Section".to_string(),
                description: "A thin-walled hull section; scavenger grade.".to_string(),
                mass: 1.0,
                // A third of reinforced: the shakedown pirate should die in
                // a short burst, not a slugging match (task 20260711-180506,
                // "gentle" is data).
                health: 60.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(meshes.hull.clone()),
                render_mesh_transform: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "light_turret_section".to_string(),
                name: "Light Turret Section".to_string(),
                description: "A low-caliber turret; scavenger grade.".to_string(),
                mass: 1.0,
                // Deliberately BELOW the turret baseline: scavenger grade, and
                // the shakedown pirate should die in a short burst (task
                // 20260711-180506). A per-section variant departing from its
                // type baseline on purpose - not the armored better_turret.
                health: 60.0,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                // Same joint tree as the better turret; scavenger grade differs
                // only in fire rate, muzzle speed, damage and ammo below.
                root: turret_joint_tree(
                    &meshes.turret_yaw,
                    &meshes.turret_pitch,
                    &meshes.turret_barrel,
                    // Scavenger grade: a quarter of the better turret's fire
                    // rate (per-muzzle now).
                    25.0,
                ),
                // Scavenger grade: slower rounds. Since the typed-damage pass
                // (task 20260712-133343) the per-hit damage is authored below
                // (bullet_damage) rather than emergent from mass x velocity.
                muzzle_speed: 60.0,
                projectile_lifetime: 5.0,
                // Authored Kinetic damage reproducing the old emergent per-hit
                // (mass 0.05 @ 60 u/s) - roughly a fifth of the better turret's,
                // matching the previous gentleness.
                bullet_damage: representative_kinetic_damage(0.05, 60.0),
                // Kinetic loadout (task 20260712-133349).
                bullet_kind: DamageType::Kinetic,
                projectile_render_mesh: None,
                fire_sound: Some(meshes.turret_fire_sound.clone()),
                dry_fire_sound: Some(meshes.turret_dry_fire_sound.clone()),
                // ~6s of fire at 25 rounds/s. Scavenger grade: a shorter fight
                // before the pirate's guns run dry. Playtest knob.
                ammo_capacity: Some(150),
                // Discrete auto-reload, ~2.5s to refill after running dry.
                reload: Some(SectionReloadConfig {
                    reload_time: 2.5,
                    rounds_per_cycle: 150,
                    only_when_empty: true,
                }),
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "torpedo_section".to_string(),
                name: "Torpedo Bay Section".to_string(),
                description: "A torpedo bay section for spaceships.".to_string(),
                mass: 1.0,
                // Torpedo bay: mid durability baseline (task 20260525-133004).
                health: TORPEDO_BASE_HEALTH,
                impact_sound: Some(meshes.section_impact_sound.clone()),
                destroy_sound: Some(meshes.section_destroy_sound.clone()),
                collider: None,
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                render_mesh: Some(meshes.torpedo_bay.clone()),
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
                launch_sound: Some(meshes.torpedo_launch_sound.clone()),
                // The blast IS the destruction voice: same wav as section
                // destruction (per-target authoring; playtest can diverge it).
                detonation_sound: Some(meshes.section_destroy_sound.clone()),
                // A small salvo of torpedoes before the bay is spent. Playtest
                // knob.
                ammo_capacity: Some(6),
                // Continuous rearm: the bay regrows one torpedo every ~4s up to
                // capacity, so a spent bay slowly comes back rather than
                // dumping-and-refilling all at once (task 20260717-085640).
                reload: Some(SectionReloadConfig {
                    reload_time: 4.0,
                    rounds_per_cycle: 1,
                    only_when_empty: false,
                }),
            }),
        },
    ];
    // The racer + cargob cut-cube ships: one prototype per cube, shared by the
    // base campaign AND downloaded mods (task craft-ships-into-base).
    sections.extend(crate::scenario::craft::racer_prototypes(meshes));
    sections.extend(crate::scenario::craft::cargob_prototypes(meshes));
    sections.extend(crate::scenario::craft::cargoa_prototypes(meshes));
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Variable damage by section type" (task 20260525-133004) as a checked
    /// invariant: section TYPE must drive durability, not sit at a uniform value.
    /// If someone flattens the baselines back to one number this fails, catching
    /// a silent regression of the feature.
    #[test]
    fn section_type_durability_ordering_holds() {
        // Thrusters take MORE damage than the baseline (fragile); turrets take
        // LESS (armored). The strict inequalities are the feature.
        assert!(
            THRUSTER_BASE_HEALTH < CONTROLLER_BASE_HEALTH,
            "a thruster must be more fragile than the mid baseline: {THRUSTER_BASE_HEALTH} vs {CONTROLLER_BASE_HEALTH}"
        );
        assert!(
            CONTROLLER_BASE_HEALTH < TURRET_BASE_HEALTH,
            "a turret must be tougher than the mid baseline: {CONTROLLER_BASE_HEALTH} vs {TURRET_BASE_HEALTH}"
        );
        // The controller core and the torpedo bay share the mid baseline.
        assert_eq!(CONTROLLER_BASE_HEALTH, TORPEDO_BASE_HEALTH);
    }

    /// Anti-regression guard for the PDC one-shot fix (task 20260712-172035): the
    /// player PDC's per-hit must stay low enough that a 100-HP asteroid takes a
    /// sustained burst, not a blink. At the old ~20.25 a 100-HP rock died in ~5
    /// rounds (~50 ms); the ceiling here keeps it at >= ~13 rounds. This fails if
    /// the PDC damage creeps back toward the old value; it is a loose guard, not a
    /// precise balance number (raise it consciously if playtest wants punchier).
    #[test]
    fn pdc_per_hit_stays_below_the_one_shot_ceiling() {
        // A representative environment object (field asteroid) is 100 HP.
        const ASTEROID_HP: f32 = 100.0;
        const MIN_ROUNDS_TO_KILL: f32 = 12.0;
        assert!(
            BETTER_TURRET_BULLET_DAMAGE <= ASTEROID_HP / MIN_ROUNDS_TO_KILL,
            "PDC per-hit {BETTER_TURRET_BULLET_DAMAGE} would kill a {ASTEROID_HP}-HP \
             object in under {MIN_ROUNDS_TO_KILL} rounds - too close to a one-shot pop"
        );
    }
}
