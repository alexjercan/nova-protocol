use bevy::prelude::*;
use nova_gameplay::prelude::*;

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
                health: 100.0,
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
                health: 100.0,
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
                render_mesh: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "better_turret_section".to_string(),
                name: "Better Turret Section".to_string(),
                description: "A better turret section for spaceships.".to_string(),
                mass: 1.0,
                health: 100.0,
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
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "torpedo_section".to_string(),
                name: "Torpedo Bay Section".to_string(),
                description: "A torpedo bay section for spaceships.".to_string(),
                mass: 1.0,
                health: 100.0,
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
            }),
        },
    ]));
}
