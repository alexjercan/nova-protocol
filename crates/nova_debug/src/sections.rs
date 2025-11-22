use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use itertools::Itertools;
use nova_gameplay::{prelude::*, sections::turret_section::TurretSectionBarrelFireState};

pub struct SectionsDebugPlugin;

impl Plugin for SectionsDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                draw_turret_barrel_direction,
                draw_turret_bullet_spawner,
                draw_turret_bullet_projectile,
                draw_thruster,
                draw_torpedo_spawner,
                log_position,
            )
                .after(TransformSystems::Propagate)
                .in_set(super::DebugSystems),
        );
    }
}

const DEBUG_LINE_LENGTH: f32 = 100.0;

fn draw_turret_barrel_direction(
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    mut gizmos: Gizmos,
) {
    for muzzle_transform in &q_muzzle {
        let barrel_pos = muzzle_transform.translation();
        let barrel_dir = muzzle_transform.forward();

        let line_length = DEBUG_LINE_LENGTH;
        let line_end = barrel_pos + barrel_dir * line_length;

        let color = tailwind::RED_500;
        gizmos.line(barrel_pos, line_end, color);
    }
}

fn draw_turret_bullet_spawner(
    mut gizmos: Gizmos,
    q_muzzle: Query<
        (&GlobalTransform, &TurretSectionBarrelFireState),
        With<TurretSectionBarrelMuzzleMarker>,
    >,
) {
    for (transform, fire_state) in &q_muzzle {
        let origin = transform.translation();
        let dir = transform.forward() * 2.0;

        let color = if fire_state.is_finished() {
            tailwind::GREEN_500
        } else {
            tailwind::YELLOW_500
        };

        gizmos.sphere(transform.to_isometry(), 0.2, color);
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_turret_bullet_projectile(
    mut gizmos: Gizmos,
    q_bullet: Query<(&Position, &LinearVelocity), With<TurretBulletProjectileMarker>>,
) {
    for (position, velocity) in &q_bullet {
        let origin = **position;
        let dir = velocity.normalize_or_zero();
        let color = tailwind::BLUE_500;

        gizmos.sphere(Isometry3d::from_translation(origin), 0.2, color);
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_thruster(
    mut gizmos: Gizmos,
    q_thruster: Query<(&GlobalTransform, &ThrusterSectionInput), With<ThrusterSectionMarker>>,
) {
    for (transform, input) in &q_thruster {
        let origin = transform.translation();
        let dir = transform.back() * (**input) * 2.0;

        let color = tailwind::TEAL_500;

        gizmos.sphere(Isometry3d::from_translation(origin), 0.2, color);
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_torpedo_spawner(
    mut gizmos: Gizmos,
    q_torpedo: Query<
        (&GlobalTransform, &TorpedoSectionSpawnerFireState),
        With<TorpedoSectionSpawnerMarker>,
    >,
) {
    for (transform, input) in &q_torpedo {
        let origin = transform.translation();
        let dir = transform.forward() * 2.0;

        let color = if input.is_finished() {
            tailwind::GREEN_500
        } else {
            tailwind::YELLOW_500
        };

        gizmos.sphere(Isometry3d::from_translation(origin), 0.2, color);
        gizmos.line(origin, origin + dir, color);
    }
}

fn log_position(
    q_spaceship: Query<(&Name, &Position, &Transform, &GlobalTransform), With<SpaceshipRootMarker>>,
    q_sections: Query<
        (&Name, &Position, &Transform, &GlobalTransform, &ChildOf),
        With<SectionMarker>,
    >,
) {
    for (parent, chunk) in &q_sections
        .iter()
        .chunk_by(|(_, _, _, _, &ChildOf(parent))| parent)
    {
        let Ok((
            spaceship_name,
            spaceship_position,
            spaceship_transform,
            spaceship_global_transform,
        )) = q_spaceship.get(parent)
        else {
            continue;
        };

        trace!(
            "Spaceship: {} | Position: {:?} | Local Transform: {:?} | Global Transform: {:?}",
            spaceship_name.as_str(),
            **spaceship_position,
            spaceship_transform.translation,
            spaceship_global_transform.translation()
        );
        for (section_name, section_position, section_transform, section_global_transform, _) in
            chunk
        {
            trace!(
                "  Section: {} | Position: {:?} | Local Transform: {:?} | Global Transform: {:?}",
                section_name.as_str(),
                **section_position,
                section_transform.translation,
                section_global_transform.translation()
            );
        }
    }
}
