//! Debug gizmos for ship sections (nova_ship::sections): turret barrel
//! directions, bullet spawners and projectiles, thruster and torpedo spawner
//! markers, plus position logging. Gated behind the F11 debug toggle via the
//! [`DebugSystems`](super::DebugSystems) set.
//!
//! Every line is a measured quantity of the thing it is drawn on: a barrel
//! reaches as far as the gun shoots, a round and a launch stub run one
//! [`ROUND_TRACE_SECONDS`] of flight, and a drive runs one
//! [`BURN_TRACE_SECONDS`] of its own burn. A gizmo that cannot read its figure
//! is not drawn, so a missing one reads as missing rather than as a default.

use avian3d::prelude::*;
use bevy::{color::palettes::tailwind, prelude::*};
use itertools::Itertools;
use nova_events::prelude::Meters;
use nova_gameplay::prelude::*;
use nova_ship::{prelude::*, sections::turret_section::TurretSectionBarrelFireState};
/// Debug overlay plugin for ship-section gizmos.
///
/// Adds the turret/thruster/torpedo gizmo systems plus `log_position` to
/// `PostUpdate` (after transform propagation) under the
/// [`DebugSystems`](super::DebugSystems) set.
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

/// How far ahead a round is traced: its line is where it will be this long from
/// now, and a launch stub is where the next one will be.
///
/// A tenth of a second reads as a tracer at gun speeds - a stock 1000 m/s round
/// draws 100 m - and still shows a 80 m/s torpedo easing out of its tube. The
/// two share one interval so a stub and the round that leaves it are the same
/// length.
const ROUND_TRACE_SECONDS: f32 = 0.1;

/// How long a drive is traced: its line is how far the section's own thrust
/// moves the hull it is bolted to in this long, from rest.
///
/// A whole second, where a round gets a tenth, because drives move hulls in
/// tens of metres per second squared while rounds cross hundreds of metres per
/// second. Calibrated on the shipped salvage skiff, whose basic drives keep
/// about the 20 m stub the old fixed 2 u line drew them.
const BURN_TRACE_SECONDS: f32 = 1.0;

/// Radius of a spawner or projectile marker dot.
///
/// An authored visual clearance and not a measurement: a round carries no
/// collider (`nova_gameplay::rounds` sweeps it instead) and a launch point is a
/// point, so there is no body here to size a dot from.
const MARKER_RADIUS: Meters = Meters(2.0);

/// The engine figures of the turret section the barrel at `muzzle` belongs to.
///
/// A muzzle is a leaf of the turret's joint chain and only the section root
/// carries the authored reach and muzzle speed, so the walk goes up the chain.
/// `None` for a muzzle spawned outside a turret - an aim-solver rig - which has
/// no authored gun behind it to draw.
fn turret_figures(
    muzzle: Entity,
    q_parent: &Query<&ChildOf>,
    q_figures: &Query<&TurretEngineFigures>,
) -> Option<TurretEngineFigures> {
    std::iter::successors(Some(muzzle), |&entity| {
        q_parent.get(entity).ok().map(|&ChildOf(parent)| parent)
    })
    .find_map(|entity| q_figures.get(entity).ok().copied())
}

/// How far this drive's burn moves its hull over [`BURN_TRACE_SECONDS`], from
/// rest.
///
/// A [`ThrusterSectionMagnitude`] is an impulse per FIXED tick, so the
/// magnitude over the hull mass over the tick length is the acceleration the
/// drive is authored to deliver, and the live input is the throttle on it. A
/// hull physics has not weighed yet reads as unit mass rather than as an
/// infinite acceleration; a stopped clock draws no line at all.
fn burn_travel(magnitude: f32, mass: f32, tick: f32, input: f32) -> f32 {
    if tick <= 0.0 {
        return 0.0;
    }
    let acceleration = magnitude / mass.max(f32::EPSILON) / tick;
    0.5 * acceleration * input.clamp(0.0, 1.0) * BURN_TRACE_SECONDS * BURN_TRACE_SECONDS
}

fn draw_turret_barrel_direction(
    q_muzzle: Query<(Entity, &GlobalTransform), With<TurretSectionBarrelMuzzleMarker>>,
    q_parent: Query<&ChildOf>,
    q_figures: Query<&TurretEngineFigures>,
    mut gizmos: Gizmos,
) {
    for (muzzle, muzzle_transform) in &q_muzzle {
        let Some(figures) = turret_figures(muzzle, &q_parent, &q_figures) else {
            continue;
        };

        let barrel_pos = muzzle_transform.translation();
        let barrel_dir = muzzle_transform.forward();

        let line_end = barrel_pos + barrel_dir * figures.reach;

        let color = tailwind::RED_500;
        gizmos.line(barrel_pos, line_end, color);
    }
}

fn draw_turret_bullet_spawner(
    mut gizmos: Gizmos,
    q_muzzle: Query<
        (Entity, &GlobalTransform, &TurretSectionBarrelFireState),
        With<TurretSectionBarrelMuzzleMarker>,
    >,
    q_parent: Query<&ChildOf>,
    q_figures: Query<&TurretEngineFigures>,
) {
    for (muzzle, transform, fire_state) in &q_muzzle {
        let Some(figures) = turret_figures(muzzle, &q_parent, &q_figures) else {
            continue;
        };

        let origin = transform.translation();
        let dir = transform.forward() * (figures.muzzle_speed * ROUND_TRACE_SECONDS);

        let color = if fire_state.is_finished() {
            tailwind::GREEN_500
        } else {
            tailwind::YELLOW_500
        };

        gizmos.sphere(transform.to_isometry(), MARKER_RADIUS.to_engine(), color);
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_turret_bullet_projectile(
    mut gizmos: Gizmos,
    q_bullet: Query<(&Transform, &RoundVelocity), With<TurretBulletProjectileMarker>>,
) {
    for (transform, velocity) in &q_bullet {
        let origin = transform.translation;
        let dir = **velocity * ROUND_TRACE_SECONDS;
        let color = tailwind::BLUE_500;

        gizmos.sphere(
            Isometry3d::from_translation(origin),
            MARKER_RADIUS.to_engine(),
            color,
        );
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_thruster(
    fixed_time: Res<Time<Fixed>>,
    mut gizmos: Gizmos,
    q_thruster: Query<
        (
            &GlobalTransform,
            &ThrusterSectionInput,
            &ThrusterSectionMagnitude,
            &ChildOf,
        ),
        With<ThrusterSectionMarker>,
    >,
    q_hull: Query<&ComputedMass>,
) {
    let tick = fixed_time.timestep().as_secs_f32();
    for (transform, input, magnitude, &ChildOf(hull)) in &q_thruster {
        let mass = q_hull.get(hull).map_or(1.0, |mass| mass.value());
        let origin = transform.translation();
        let dir = transform.back() * burn_travel(**magnitude, mass, tick, **input);

        let color = tailwind::TEAL_500;

        gizmos.sphere(
            Isometry3d::from_translation(origin),
            MARKER_RADIUS.to_engine(),
            color,
        );
        gizmos.line(origin, origin + dir, color);
    }
}

fn draw_torpedo_spawner(
    mut gizmos: Gizmos,
    q_torpedo: Query<
        (
            &GlobalTransform,
            &TorpedoSectionSpawnerFireState,
            &TorpedoSectionPartOf,
        ),
        With<TorpedoSectionSpawnerMarker>,
    >,
    q_bay: Query<&TorpedoSectionConfigHelper>,
) {
    for (transform, input, &TorpedoSectionPartOf(bay)) in &q_torpedo {
        let Ok(config) = q_bay.get(bay) else {
            continue;
        };

        let origin = transform.translation();
        let dir = transform.forward() * (config.spawner_speed.to_engine() * ROUND_TRACE_SECONDS);

        let color = if input.ready() {
            tailwind::GREEN_500
        } else {
            tailwind::YELLOW_500
        };

        gizmos.sphere(
            Isometry3d::from_translation(origin),
            MARKER_RADIUS.to_engine(),
            color,
        );
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

/// `SectionsDebugPlugin`.
pub mod prelude {
    pub use super::SectionsDebugPlugin;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// Walk a spawned muzzle up to its turret, through the production helper.
    fn figures_of(world: &mut World, muzzle: Entity) -> Option<TurretEngineFigures> {
        world
            .run_system_once(
                move |q_parent: Query<&ChildOf>, q_figures: Query<&TurretEngineFigures>| {
                    turret_figures(muzzle, &q_parent, &q_figures)
                },
            )
            .expect("the probe system runs")
    }

    #[test]
    fn a_barrel_reads_the_reach_of_the_turret_it_hangs_from() {
        let mut world = World::new();
        let figures = TurretEngineFigures {
            muzzle_speed: 100.0,
            reach: 200.0,
        };
        let turret = world.spawn(figures).id();
        let joint = world.spawn(ChildOf(turret)).id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, ChildOf(joint)))
            .id();

        assert_eq!(figures_of(&mut world, muzzle), Some(figures));
    }

    #[test]
    fn a_muzzle_with_no_turret_above_it_has_no_figures() {
        let mut world = World::new();
        let muzzle = world.spawn(TurretSectionBarrelMuzzleMarker).id();

        assert_eq!(figures_of(&mut world, muzzle), None);
    }

    #[test]
    fn a_burn_line_grows_with_authored_thrust_and_live_input() {
        let tick = 1.0 / 64.0;
        let half = burn_travel(9.0, 3.0, tick, 0.5);
        let full = burn_travel(9.0, 3.0, tick, 1.0);
        let stronger = burn_travel(18.0, 3.0, tick, 1.0);

        assert!((full - 2.0 * half).abs() < 1e-3, "input scales the line");
        assert!(
            (stronger - 2.0 * full).abs() < 1e-3,
            "the authored magnitude scales the line"
        );
    }

    #[test]
    fn a_heavier_hull_shortens_the_same_drives_burn_line() {
        let tick = 1.0 / 64.0;

        assert!(burn_travel(9.0, 6.0, tick, 1.0) < burn_travel(9.0, 3.0, tick, 1.0));
    }

    #[test]
    fn a_stopped_clock_draws_no_burn_line() {
        assert_eq!(burn_travel(9.0, 3.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn a_burn_line_ignores_input_outside_the_throttle_range() {
        let tick = 1.0 / 64.0;

        assert_eq!(
            burn_travel(9.0, 3.0, tick, 4.0),
            burn_travel(9.0, 3.0, tick, 1.0)
        );
        assert_eq!(burn_travel(9.0, 3.0, tick, -1.0), 0.0);
    }
}
