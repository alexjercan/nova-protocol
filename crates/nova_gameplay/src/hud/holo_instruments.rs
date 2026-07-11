//! World-space holo instruments (task 20260710-174629, spike
//! docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md):
//! the expansion of the language the ORBIT ring piloted - thin unlit
//! NAV_CYAN geometry the flight computer "projects" into space.
//!
//! - **Trajectory ribbon**: the engaged leg's path (ship -> flip -> park
//!   point, or ship -> park point once braking) as thin cylinder segments,
//!   for GOTO and STOP alike via [`ManeuverTelemetry`]. The ribbon ends at
//!   the park point, not the goal center - on a big body the center sits
//!   radius + standoff past where the ship actually stops (task
//!   20260710-214316). Deliberately the straight-line
//!   plan the computer actually flies today; when the arrival solve
//!   becomes gravity-aware (task 20260710-193500) a curved prediction can
//!   replace it - the instrument must not out-promise the autopilot.
//! - **Flip gate**: a ring at the flip point, perpendicular to the path,
//!   sized to fly through.

use bevy::prelude::*;

use super::NAV_CYAN;
use crate::{flight::prelude::*, input::prelude::*};

pub mod prelude {
    pub use super::{FlipGateMarker, HoloInstrumentsPlugin, TrajectoryRibbonSegment};
}

/// Ribbon segment tube radius, world units.
const RIBBON_RADIUS: f32 = 0.06;

/// Flip gate ring radius, world units - sized to fly through.
const GATE_RADIUS: f32 = 4.0;

/// Flip gate tube thickness, world units.
const GATE_MINOR_RADIUS: f32 = 0.12;

/// One segment of the trajectory ribbon. Public for tests and future
/// consumers.
#[derive(Component, Debug, Clone, Reflect)]
pub struct TrajectoryRibbonSegment {
    /// The ship whose leg this segment renders.
    pub ship: Entity,
    /// Segment index along the path (0 = from the ship).
    pub index: usize,
}

/// The flip gate of an engaged leg.
#[derive(Component, Debug, Clone, Reflect)]
pub struct FlipGateMarker {
    /// The ship whose flip this gate marks.
    pub ship: Entity,
}

/// Shared meshes/material for every holo element (the ribbon, the gate,
/// and the orbit ring in maneuver_instruments), created lazily
/// so the systems stay plain `Assets<_>` consumers and run headless in
/// tests. A Resource, not a per-system Local: one material keeps the
/// family batchable.
#[derive(Resource, Default)]
pub(crate) struct HoloAssets {
    /// Unit cylinder (radius RIBBON_RADIUS, height 1) for ribbon segments.
    segment_mesh: Option<Handle<Mesh>>,
    /// The flip gate's torus (constant radius).
    gate_mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
}

impl HoloAssets {
    pub(crate) fn segment_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.segment_mesh
            .get_or_insert_with(|| meshes.add(Cylinder::new(RIBBON_RADIUS, 1.0)))
            .clone()
    }

    fn gate_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.gate_mesh
            .get_or_insert_with(|| {
                meshes.add(Torus::new(
                    GATE_RADIUS - GATE_MINOR_RADIUS,
                    GATE_RADIUS + GATE_MINOR_RADIUS,
                ))
            })
            .clone()
    }

    pub(crate) fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        self.material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: NAV_CYAN,
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                })
            })
            .clone()
    }
}

#[derive(Default)]
pub struct HoloInstrumentsPlugin;

impl Plugin for HoloInstrumentsPlugin {
    fn build(&self, app: &mut App) {
        debug!("HoloInstrumentsPlugin: build");

        app.init_resource::<HoloAssets>();

        app.register_type::<TrajectoryRibbonSegment>()
            .register_type::<FlipGateMarker>();

        app.add_systems(
            Update,
            (sync_trajectory_ribbon, sync_flip_gate).in_set(super::NovaHudSystems),
        );
    }
}

/// The Y-up unit cylinder stretched onto a world segment. Shared with the
/// radius spoke in maneuver_instruments.
pub(crate) fn segment_transform(from: Vec3, to: Vec3) -> Transform {
    let axis = to - from;
    let length = axis.length().max(f32::EPSILON);
    Transform {
        translation: (from + to) * 0.5,
        rotation: Quat::from_rotation_arc(Vec3::Y, axis / length),
        scale: Vec3::new(1.0, length, 1.0),
    }
}

/// Own the ribbon: one thin segment per leg of the engaged path, updated
/// every frame (the ship end moves every tick), despawned with the leg.
fn sync_trajectory_ribbon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    // The ribbon's ship end must meet the RENDERED hull: eased root
    // Transform, not raw avian Position (task 20260710-231928). The other
    // points are plan geometry, not rendered bodies.
    q_ship: Query<
        (Entity, &Transform, &ManeuverTelemetry),
        (
            With<PlayerSpaceshipMarker>,
            Without<TrajectoryRibbonSegment>,
        ),
    >,
    mut q_segment: Query<(Entity, &TrajectoryRibbonSegment, &mut Transform)>,
) {
    let leg = q_ship.iter().next().map(|(ship, transform, telemetry)| {
        let mut points = vec![transform.translation];
        if let Some(flip) = telemetry.flip_point {
            points.push(flip);
        }
        // The leg ends where the ship will rest, not at the target
        // center - on a big body the center sits radius + standoff past
        // the park point (task 20260710-214316).
        points.push(telemetry.park_point);
        (ship, points)
    });

    let Some((ship, points)) = leg else {
        for (entity, _, _) in &q_segment {
            commands.entity(entity).despawn();
        }
        return;
    };

    let wanted = points.len() - 1;
    let mut present = vec![false; wanted];
    for (entity, segment, mut transform) in &mut q_segment {
        if segment.ship != ship || segment.index >= wanted {
            commands.entity(entity).despawn();
            continue;
        }
        present[segment.index] = true;
        *transform = segment_transform(points[segment.index], points[segment.index + 1]);
    }
    for (index, _) in present
        .iter()
        .enumerate()
        .filter(|(_, in_place)| !**in_place)
    {
        commands.spawn((
            Name::new("TrajectoryRibbonSegment"),
            crate::hud::HudTier::Instrument,
            TrajectoryRibbonSegment { ship, index },
            Mesh3d(assets.segment_mesh(&mut meshes)),
            MeshMaterial3d(assets.material(&mut materials)),
            segment_transform(points[index], points[index + 1]),
            Visibility::Visible,
        ));
    }
}

/// Own the flip gate: a fly-through ring at the predicted flip point,
/// facing along the path; gone once braking (no flip prediction) or when
/// the leg ends.
fn sync_flip_gate(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    // Same render-clock ship read as the ribbon (direction only, but the
    // uniform pose family keeps the instruments coherent).
    q_ship: Query<
        (Entity, &Transform, &ManeuverTelemetry),
        (With<PlayerSpaceshipMarker>, Without<FlipGateMarker>),
    >,
    mut q_gate: Query<(Entity, &FlipGateMarker, &mut Transform)>,
) {
    let flip = q_ship
        .iter()
        .next()
        .and_then(|(ship, transform, telemetry)| {
            let flip = telemetry.flip_point?;
            let along = (telemetry.goal - transform.translation).try_normalize()?;
            Some((ship, flip, along))
        });

    let Some((ship, flip, along)) = flip else {
        for (entity, _, _) in &q_gate {
            commands.entity(entity).despawn();
        }
        return;
    };

    // The torus lies in the XZ plane (normal Y); face it down the path so
    // the ship flies through it.
    let rotation = Quat::from_rotation_arc(Vec3::Y, along);
    let mut found = false;
    for (entity, gate, mut transform) in &mut q_gate {
        if gate.ship != ship {
            commands.entity(entity).despawn();
            continue;
        }
        found = true;
        if transform.translation != flip || transform.rotation != rotation {
            transform.translation = flip;
            transform.rotation = rotation;
        }
    }
    if !found {
        commands.spawn((
            Name::new("FlipGateHolo"),
            crate::hud::HudTier::Instrument,
            FlipGateMarker { ship },
            Mesh3d(assets.gate_mesh(&mut meshes)),
            MeshMaterial3d(assets.material(&mut materials)),
            Transform::from_translation(flip).with_rotation(rotation),
            Visibility::Visible,
        ));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::sections::prelude::*;

    fn holo_world() -> World {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<HoloAssets>();
        world
    }

    /// A 50u standoff on the ship->goal line (the ship sits at the
    /// origin), the same shape the autopilot publishes.
    fn park_point(goal: Vec3) -> Vec3 {
        goal - goal.normalize_or_zero() * 50.0
    }

    fn telemetry(goal: Vec3, flip: Option<Vec3>) -> ManeuverTelemetry {
        ManeuverTelemetry {
            goal,
            goal_entity: None,
            park_point: park_point(goal),
            distance: goal.length(),
            closing_speed: 10.0,
            brake_accel: 10.0,
            flip_point: flip,
            seconds_to_flip: flip.map(|_| 5.0),
            eta: Some(10.0),
        }
    }

    fn spawn_ship(world: &mut World, telemetry_value: ManeuverTelemetry) -> Entity {
        world
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Transform::default(),
                telemetry_value,
            ))
            .id()
    }

    #[test]
    fn ribbon_tracks_the_leg_and_dies_with_it() {
        let mut world = holo_world();
        let goal = Vec3::new(0.0, 0.0, -300.0);
        let flip = Vec3::new(0.0, 0.0, -240.0);
        let ship = spawn_ship(&mut world, telemetry(goal, Some(flip)));

        world.run_system_once(sync_trajectory_ribbon).unwrap();
        let count = world
            .query::<&TrajectoryRibbonSegment>()
            .iter(&world)
            .count();
        assert_eq!(count, 2, "ship -> flip -> park point is two segments");

        // Braking: the flip vanishes, the ribbon collapses to one segment.
        world.entity_mut(ship).insert(telemetry(goal, None));
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        let count = world
            .query::<&TrajectoryRibbonSegment>()
            .iter(&world)
            .count();
        assert_eq!(count, 1, "no flip, one segment");

        // The leg ends: the ribbon goes with it.
        world.entity_mut(ship).remove::<ManeuverTelemetry>();
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        assert_eq!(
            world
                .query::<&TrajectoryRibbonSegment>()
                .iter(&world)
                .count(),
            0
        );
    }

    #[test]
    fn ribbon_terminates_at_the_park_point_not_the_goal() {
        // Task 20260710-214316: on a big body the goal center sits
        // radius + standoff past where the ship actually parks; the last
        // segment must stop at the published park point.
        let mut world = holo_world();
        let goal = Vec3::new(0.0, 0.0, -300.0);
        let flip = Vec3::new(0.0, 0.0, -240.0);
        spawn_ship(&mut world, telemetry(goal, Some(flip)));

        world.run_system_once(sync_trajectory_ribbon).unwrap();
        let mut last: Option<(usize, Transform)> = None;
        for (segment, transform) in world
            .query::<(&TrajectoryRibbonSegment, &Transform)>()
            .iter(&world)
        {
            if last.is_none_or(|(index, _)| segment.index > index) {
                last = Some((segment.index, *transform));
            }
        }
        let (_, transform) = last.expect("the ribbon has segments");
        // The far end of the Y-up unit cylinder segment.
        let far_end =
            transform.translation + transform.rotation.mul_vec3(Vec3::Y) * transform.scale.y * 0.5;
        let park = park_point(goal);
        assert!(
            far_end.distance(park) < 1e-4,
            "the ribbon ends at the park point {park}, got {far_end}"
        );
        assert!(
            far_end.distance(goal) > 49.0,
            "the ribbon must stop a standoff short of the target center"
        );
    }

    #[test]
    fn flip_gate_faces_the_path_and_dies_when_braking() {
        let mut world = holo_world();
        let goal = Vec3::new(0.0, 0.0, -300.0);
        let flip = Vec3::new(0.0, 0.0, -240.0);
        let ship = spawn_ship(&mut world, telemetry(goal, Some(flip)));

        world.run_system_once(sync_flip_gate).unwrap();
        let (transform, _) = world
            .query::<(&Transform, &FlipGateMarker)>()
            .single(&world)
            .expect("one gate");
        assert_eq!(transform.translation, flip);
        let facing = transform.rotation.mul_vec3(Vec3::Y);
        assert!(
            (facing - Vec3::NEG_Z).length() < 1e-5,
            "the gate faces down the path, got {facing}"
        );

        world.entity_mut(ship).insert(telemetry(goal, None));
        world.run_system_once(sync_flip_gate).unwrap();
        assert_eq!(
            world.query::<&FlipGateMarker>().iter(&world).count(),
            0,
            "braking retires the gate"
        );
    }
}
