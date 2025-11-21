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
            },
            kind: SectionKind::Controller(ControllerSectionConfig {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 100.0,
                render_mesh: None,
            }),
        },
        SectionConfig {
            base: BaseSectionConfig {
                id: "better_turret_section".to_string(),
                name: "Better Turret Section".to_string(),
                description: "A better turret section for spaceships.".to_string(),
                mass: 1.0,
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
            },
            kind: SectionKind::Torpedo(TorpedoSectionConfig {
                render_mesh: None,
                projectile_render_mesh: None,
                spawn_offset: Vec3::Y * 2.0,
                fire_rate: 1.0,
                spawner_speed: 1.0,
                projectile_lifetime: 10.0,
            }),
        },
    ]));
}
