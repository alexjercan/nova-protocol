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

pub(crate) fn register_sections(mut commands: Commands, game_assets: Res<super::GameAssets>) {
    // This should be loaded from a JSON file, but for now it is fine.

    commands.insert_resource(GameSections(vec![
        SectionConfig {
            base: BaseSectionConfig {
                id: "reinforced_hull_section".to_string(),
                name: "Reinforced Hull Section".to_string(),
                description: "A reinforced hull section for spaceships.".to_string(),
                mass: 1.0,
                health: 200.0,
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(game_assets.hull_01.clone()),
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
            },
            kind: SectionKind::Thruster(ThrusterSectionConfig {
                magnitude: 1.0,
                render_mesh: None,
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
                // Full flight-verb loadout by default; scenarios withhold a
                // verb at runtime via `SetControllerVerb` (the shakedown's
                // GOTO-off intro) rather than baking it into this shared
                // catalog entry, which the pirate reuses too.
                verbs: ControllerVerbs::default(),
                render_mesh: None,
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
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                yaw_speed: std::f32::consts::PI,   // 180 degrees per second
                pitch_speed: std::f32::consts::PI, // 180 degrees per second
                min_pitch: Some(-std::f32::consts::FRAC_PI_6),
                max_pitch: Some(std::f32::consts::FRAC_PI_2),
                render_mesh_base: None,
                base_offset: Vec3::new(0.0, -0.5, 0.0),
                render_mesh_yaw: Some(game_assets.turret_yaw_01.clone()),
                yaw_offset: Vec3::new(0.0, 0.1, 0.0),
                render_mesh_pitch: Some(game_assets.turret_pitch_01.clone()),
                pitch_offset: Vec3::new(0.0, 0.332706, 0.303954),
                render_mesh_barrel: Some(game_assets.turret_barrel_01.clone()),
                barrel_offset: Vec3::new(0.0, 0.128437, -0.110729),
                muzzle_offset: Vec3::new(0.0, 0.0, -1.2),
                fire_rate: 100.0,
                muzzle_speed: 100.0,
                projectile_lifetime: 5.0,
                projectile_mass: 0.1,
                projectile_render_mesh: None,
                muzzle_effect: None,
                // ~5s of sustained fire at 100 rounds/s. Generous on purpose:
                // finite ammo lands before its reload (task 20260708-162005),
                // so the player should feel the limit without running dry in a
                // normal engagement. Playtest knob.
                ammo_capacity: Some(500),
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
            },
            kind: SectionKind::Hull(HullSectionConfig {
                render_mesh: Some(game_assets.hull_01.clone()),
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
            },
            kind: SectionKind::Turret(TurretSectionConfig {
                yaw_speed: std::f32::consts::PI,
                pitch_speed: std::f32::consts::PI,
                min_pitch: Some(-std::f32::consts::FRAC_PI_6),
                max_pitch: Some(std::f32::consts::FRAC_PI_2),
                render_mesh_base: None,
                base_offset: Vec3::new(0.0, -0.5, 0.0),
                render_mesh_yaw: Some(game_assets.turret_yaw_01.clone()),
                yaw_offset: Vec3::new(0.0, 0.1, 0.0),
                render_mesh_pitch: Some(game_assets.turret_pitch_01.clone()),
                pitch_offset: Vec3::new(0.0, 0.332706, 0.303954),
                render_mesh_barrel: Some(game_assets.turret_barrel_01.clone()),
                barrel_offset: Vec3::new(0.0, 0.128437, -0.110729),
                muzzle_offset: Vec3::new(0.0, 0.0, -1.2),
                // Bullet damage is kinetic (impulse/energy modifiers in the
                // bcs integrity pipeline), so gentleness is tuned here: a
                // quarter of the better turret's fire rate, slower and
                // lighter rounds - roughly a fifth of the per-hit energy.
                fire_rate: 25.0,
                muzzle_speed: 60.0,
                projectile_lifetime: 5.0,
                projectile_mass: 0.05,
                projectile_render_mesh: None,
                muzzle_effect: None,
                // ~6s of fire at 25 rounds/s. Scavenger grade: a shorter fight
                // before the pirate's guns run dry. Playtest knob.
                ammo_capacity: Some(150),
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
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                render_mesh: Some(game_assets.torpedo_bay_01.clone()),
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
                // A small salvo of torpedoes before the bay is spent. Playtest
                // knob; reloading is task 20260708-162005.
                ammo_capacity: Some(6),
            }),
        },
    ]));
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
}
