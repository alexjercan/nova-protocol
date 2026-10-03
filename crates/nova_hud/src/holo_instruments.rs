//! World-space holo instruments: the expansion of the language the ORBIT ring
//! piloted - thin unlit NAV_CYAN geometry the flight computer "projects" into
//! space.
//!
//! - **Trajectory ribbon**: the part of the leg's [`FlightPrediction`] the
//!   ship has not flown yet, as thin cylinder segments from the rendered
//!   centre of mass. It stops where the prediction stops: at the end of the
//!   leg or at the prediction horizon, never on to the goal or a park point.
//!   Without a prediction there is no ribbon; a straight line would promise a
//!   path the autopilot does not fly.
//! - **Flip gate**: a ring at the predicted flip point, perpendicular to the
//!   predicted path, sized to fly through, until the predicted brake starts.

use std::time::Duration;

use avian3d::prelude::ComputedCenterOfMass;
use bevy::{light::NotShadowCaster, prelude::*};
use nova_events::units::prelude::*;
use nova_gameplay::markers::prelude::*;
use nova_ship::{
    flight::prelude::*,
    prelude::{live_structure_anchor, HullEnvelopeRadius},
};

use super::NAV_CYAN;

/// `HoloInstrumentsPlugin` with the flip-gate and trajectory-ribbon components.
pub mod prelude {
    pub use super::{FlipGateMarker, HoloInstrumentsPlugin, TrajectoryRibbonSegment};
}

/// Ribbon segment tube radius, world units.
const RIBBON_RADIUS: f32 = 0.06;

/// Most segments the ribbon draws. A 30 s prediction has about 240 points;
/// the ribbon keeps its sharpest bends and drops the rest, so the entity count
/// stays bounded.
const RIBBON_SEGMENTS: usize = 60;

/// How far outside the hull's own physical envelope the flip gate's mouth
/// stands.
///
/// The gate is a promise the ship can fly through it, and a flip SWEEPS the
/// hull: the ship turns end for end about its centre of mass, so the volume it
/// needs is the containment sphere, not its cross-section. The ring was a fixed
/// 40 m, which the shipped salvage skiff does not fit through and the carrier
/// swallows whole. The same visual clearance the rest of the presentation layer
/// keeps outside a hull.
const GATE_CLEARANCE: Meters = Meters(5.0);

/// Flip gate tube thickness, world units. An INDICATOR size, not a hull size:
/// the ring reads as a drawn line at every hull scale, so it is authored here
/// and the mesh is rebuilt rather than scaled when the mouth changes.
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
    /// The flip gate's torus, with the mouth radius it was built at. Rebuilt
    /// rather than scaled: a scaled torus thickens its tube with its mouth, and
    /// the tube is an indicator width.
    gate_mesh: Option<(f32, Handle<Mesh>)>,
    material: Option<Handle<StandardMaterial>>,
}

impl HoloAssets {
    pub(crate) fn segment_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.segment_mesh
            .get_or_insert_with(|| meshes.add(Cylinder::new(RIBBON_RADIUS, 1.0)))
            .clone()
    }

    /// The gate torus with a `major` mouth radius, rebuilt when the hull it is
    /// sized for changes. One hull flies a leg at a time, so one cached mesh is
    /// the whole working set; a hull shedding sections rebuilds it as rarely as
    /// its envelope actually moves.
    fn gate_mesh(&mut self, meshes: &mut Assets<Mesh>, major: f32) -> Handle<Mesh> {
        if let Some((built, handle)) = &self.gate_mesh {
            if *built == major {
                return handle.clone();
            }
        }
        let handle = meshes.add(Torus::new(
            major - GATE_MINOR_RADIUS,
            major + GATE_MINOR_RADIUS,
        ));
        self.gate_mesh = Some((major, handle.clone()));
        handle
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

/// Draws the world-space holo instruments: the trajectory ribbon along the
/// predicted leg and the flip gate ring at its predicted flip point.
/// Inits `HoloAssets`, registers [`TrajectoryRibbonSegment`]/[`FlipGateMarker`],
/// and runs `sync_trajectory_ribbon` and `sync_flip_gate` in Update within
/// [`super::NovaHudSystems`].
#[derive(Default)]
pub struct HoloInstrumentsPlugin;

impl Plugin for HoloInstrumentsPlugin {
    fn build(&self, app: &mut App) {
        trace!("HoloInstrumentsPlugin: build");

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

/// Whether point `index` of `prediction` is still ahead of the ship at the
/// `Time<Fixed>` elapsed time `now`. Point `i` is the state `i` sample
/// intervals after the seed, except the last point, which is the state at
/// [`FlightPrediction::final_point_time`]; a point the ship has reached is
/// behind it.
fn is_ahead(prediction: &FlightPrediction, now: Duration, index: usize) -> bool {
    let flown = now.saturating_sub(prediction.seed_time).as_secs_f32();
    let point_time = if index + 1 == prediction.points.len() {
        prediction.final_point_time
    } else {
        index as f32 * prediction.sample_interval
    };
    point_time > flown
}

/// Reduce a polyline to at most `max` vertices, keeping its ends and its
/// sharpest bends: starting from the two ends, repeatedly keep the vertex that
/// lies furthest from the chord between its kept neighbours (top-N
/// Ramer-Douglas-Peucker). The pick depends only on the points, so one path
/// always gives one ribbon. A polyline already within `max` is returned whole.
fn decimate_path(points: &[Vec3], max: usize) -> Vec<Vec3> {
    if points.len() <= max.max(2) {
        return points.to_vec();
    }
    let deviation = |i: usize, a: usize, b: usize| {
        let chord = points[b] - points[a];
        let offset = points[i] - points[a];
        let length_squared = chord.length_squared();
        if length_squared <= f32::EPSILON {
            offset.length()
        } else {
            offset
                .reject_from_normalized(chord / length_squared.sqrt())
                .length()
        }
    };
    let mut kept = vec![0, points.len() - 1];
    while kept.len() < max {
        let Some((at, index, _)) = kept
            .windows(2)
            .enumerate()
            .filter_map(|(at, pair)| {
                (pair[0] + 1..pair[1])
                    .map(|i| (at + 1, i, deviation(i, pair[0], pair[1])))
                    .max_by(|a, b| a.2.total_cmp(&b.2))
            })
            .max_by(|a, b| a.2.total_cmp(&b.2))
        else {
            break;
        };
        kept.insert(at, index);
    }
    kept.into_iter().map(|i| points[i]).collect()
}

/// Own the ribbon: one thin segment per span of the path from the rendered
/// ship through the prediction points still ahead of it, decimated to at most
/// [`RIBBON_SEGMENTS`] pooled segments, updated every frame (the ship end
/// moves every tick), despawned when the ship has no prediction.
fn sync_trajectory_ribbon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    time: Res<Time<Fixed>>,
    // The ribbon's ship end must meet the RENDERED hull: the eased root
    // Transform, not raw avian Position, lifted to the centre of mass because
    // every prediction point is one.
    q_ship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &FlightPrediction,
        ),
        (
            With<PlayerSpaceshipMarker>,
            Without<TrajectoryRibbonSegment>,
        ),
    >,
    mut q_segment: Query<(Entity, &TrajectoryRibbonSegment, &mut Transform)>,
) {
    let path = q_ship
        .iter()
        .next()
        .map(|(ship, transform, center_of_mass, prediction)| {
            let ahead = prediction
                .points
                .iter()
                .enumerate()
                .filter(|&(index, _)| is_ahead(prediction, time.elapsed(), index))
                .map(|(_, point)| *point);
            let points: Vec<Vec3> =
                std::iter::once(live_structure_anchor(transform, center_of_mass))
                    .chain(ahead)
                    .collect();
            (ship, decimate_path(&points, RIBBON_SEGMENTS + 1))
        });

    let Some((ship, points)) = path else {
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
            crate::HudTier::Instrument,
            TrajectoryRibbonSegment { ship, index },
            Mesh3d(assets.segment_mesh(&mut meshes)),
            NotShadowCaster,
            MeshMaterial3d(assets.material(&mut materials)),
            segment_transform(points[index], points[index + 1]),
            Visibility::Visible,
        ));
    }
}

/// Own the flip gate: a fly-through ring at the prediction's flip point,
/// facing along the predicted path there. Gone once the ship reaches that
/// point, when the prediction does not brake, and when the ship has no
/// prediction.
fn sync_flip_gate(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: ResMut<HoloAssets>,
    time: Res<Time<Fixed>>,
    q_ship: Query<
        (Entity, &FlightPrediction, Option<&HullEnvelopeRadius>),
        With<PlayerSpaceshipMarker>,
    >,
    mut q_gate: Query<(Entity, &FlipGateMarker, &mut Transform, &mut Mesh3d)>,
) {
    let flip = q_ship
        .iter()
        .next()
        .and_then(|(ship, prediction, envelope)| {
            let index = prediction
                .flip_index
                .filter(|&index| is_ahead(prediction, time.elapsed(), index))?;
            // The path's direction at the flip point: the chord between its
            // neighbours, one-sided at either end of the prediction.
            let points = &prediction.points;
            let along = (points[(index + 1).min(points.len() - 1)]
                - points[index.saturating_sub(1)])
            .try_normalize()?;
            // A hull that has not been measured yet gets no gate. The ring is a
            // fly-through promise, and one drawn at a guessed mouth is a
            // promise about a hull nothing has looked at.
            let envelope = envelope?;
            Some((
                ship,
                points[index],
                along,
                **envelope + GATE_CLEARANCE.to_engine(),
            ))
        });

    let Some((ship, flip, along, major)) = flip else {
        for (entity, _, _, _) in &q_gate {
            commands.entity(entity).despawn();
        }
        return;
    };

    // The torus lies in the XZ plane (normal Y); face it down the path so
    // the ship flies through it.
    let rotation = Quat::from_rotation_arc(Vec3::Y, along);
    let mesh = assets.gate_mesh(&mut meshes, major);
    let mut found = false;
    for (entity, gate, mut transform, mut gate_mesh) in &mut q_gate {
        if gate.ship != ship {
            commands.entity(entity).despawn();
            continue;
        }
        found = true;
        if transform.translation != flip || transform.rotation != rotation {
            transform.translation = flip;
            transform.rotation = rotation;
        }
        // A hull that lost the section holding its envelope re-sizes its gate
        // mid-leg; without this the live ring keeps the old mouth forever.
        if gate_mesh.0 != mesh {
            gate_mesh.0 = mesh.clone();
        }
    }
    if !found {
        commands.spawn((
            Name::new("FlipGateHolo"),
            crate::HudTier::Instrument,
            FlipGateMarker { ship },
            Mesh3d(mesh),
            NotShadowCaster,
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

    fn holo_world() -> World {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<HoloAssets>();
        world.init_resource::<Time<Fixed>>();
        world
    }

    const SAMPLE_INTERVAL: f32 = 0.125;

    /// A predicted leg that bends off -Z toward +X, seeded at time zero. The
    /// points are centres of mass, as the predictor publishes them.
    const BENDING_PATH: [Vec3; 5] = [
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -20.0),
        Vec3::new(4.0, 0.0, -40.0),
        Vec3::new(12.0, 0.0, -58.0),
        Vec3::new(24.0, 0.0, -72.0),
    ];

    fn prediction(points: Vec<Vec3>, flip_index: Option<usize>) -> FlightPrediction {
        FlightPrediction {
            final_point_time: (points.len() - 1) as f32 * SAMPLE_INTERVAL,
            points,
            flip_index,
            seed_time: Duration::ZERO,
            sample_interval: SAMPLE_INTERVAL,
            end: FlightPredictionEndType::Completed,
        }
    }

    /// The shipped salvage skiff's measured containment radius, world units -
    /// the envelope its live sections publish.
    const SKIFF_ENVELOPE: f32 = 4.89;

    fn spawn_ship(world: &mut World, prediction: FlightPrediction) -> Entity {
        world
            .spawn((
                PlayerSpaceshipMarker,
                SpaceshipRootMarker,
                Transform::default(),
                // Published every fixed tick in production; the gate is sized
                // from it, so the fixture carries it too.
                HullEnvelopeRadius(SKIFF_ENVELOPE),
                prediction,
            ))
            .id()
    }

    /// Both ends of every ribbon segment, in path order. The far end of a
    /// segment is the top of its Y-up unit cylinder.
    fn ribbon(world: &mut World) -> Vec<(Vec3, Vec3)> {
        let mut segments: Vec<(usize, Transform)> = world
            .query::<(&TrajectoryRibbonSegment, &Transform)>()
            .iter(world)
            .map(|(segment, transform)| (segment.index, *transform))
            .collect();
        segments.sort_by_key(|(index, _)| *index);
        segments
            .into_iter()
            .map(|(_, transform)| {
                let half = transform.rotation.mul_vec3(Vec3::Y) * transform.scale.y * 0.5;
                (transform.translation - half, transform.translation + half)
            })
            .collect()
    }

    fn assert_ribbon(world: &mut World, vertices: &[Vec3]) {
        let segments = ribbon(world);
        assert_eq!(segments.len(), vertices.len() - 1, "segments {segments:?}");
        for ((from, to), pair) in segments.iter().zip(vertices.windows(2)) {
            assert!(
                from.distance(pair[0]) < 1e-4 && to.distance(pair[1]) < 1e-4,
                "segment {from} -> {to}, want {} -> {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn the_holo_geometry_never_casts_a_shadow() {
        // The instruments are a projection, not a thing in the world - see the
        // crate doc. Bevy casts from every Mesh3d by default, so a ribbon
        // segment or gate ring left to default drops its own shadow across
        // whatever it is drawn over.
        let mut world = holo_world();
        spawn_ship(&mut world, prediction(BENDING_PATH.to_vec(), Some(2)));
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        world.run_system_once(sync_flip_gate).unwrap();

        let meshed: Vec<Entity> = world
            .query_filtered::<Entity, With<Mesh3d>>()
            .iter(&world)
            .collect();
        assert!(!meshed.is_empty(), "the ribbon and gate must have spawned");
        for entity in meshed {
            assert!(
                world.entity(entity).contains::<NotShadowCaster>(),
                "holo mesh {entity:?} must not cast a shadow"
            );
        }
    }

    /// The ribbon starts at the rendered centre of mass, because every
    /// prediction point is a centre of mass, then follows the points the ship
    /// has not reached, ends at the last one, and goes with the prediction.
    #[test]
    fn the_ribbon_draws_the_unflown_prediction_from_the_rendered_centre_of_mass() {
        let mut world = holo_world();
        let ship = spawn_ship(&mut world, prediction(BENDING_PATH.to_vec(), None));
        // A quarter turn carries the local +Z centre of mass to world +X.
        world.entity_mut(ship).insert((
            Transform::from_translation(Vec3::new(0.5, 0.0, 0.5))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            ComputedCenterOfMass(Vec3::Z),
        ));
        let rendered = Vec3::new(1.5, 0.0, 0.5);

        world.run_system_once(sync_trajectory_ribbon).unwrap();
        // The seed point is where the ship already is.
        let mut vertices = vec![rendered];
        vertices.extend_from_slice(&BENDING_PATH[1..]);
        assert_ribbon(&mut world, &vertices);

        // Two and a half samples on, the ship has flown past points 1 and 2.
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f32(2.5 * SAMPLE_INTERVAL));
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        assert_ribbon(&mut world, &[rendered, BENDING_PATH[3], BENDING_PATH[4]]);

        // No prediction, no ribbon: nothing falls back to a straight line.
        world.entity_mut(ship).remove::<FlightPrediction>();
        world.run_system_once(sync_trajectory_ribbon).unwrap();
        assert!(ribbon(&mut world).is_empty());
    }

    /// A full 30 s prediction has hundreds of points; the ribbon draws at most
    /// [`RIBBON_SEGMENTS`] of them and keeps its ends and its corner.
    #[test]
    fn the_ribbon_keeps_its_ends_and_bends_within_its_segment_cap() {
        let mut world = holo_world();
        let corner = Vec3::new(0.0, 0.0, -199.0);
        let points: Vec<Vec3> = (0..400)
            .map(|i| match i {
                0..200 => Vec3::new(0.0, 0.0, -(i as f32)),
                _ => corner + Vec3::X * (i - 199) as f32,
            })
            .collect();
        let end = *points.last().unwrap();
        spawn_ship(&mut world, prediction(points, None));

        world.run_system_once(sync_trajectory_ribbon).unwrap();
        let segments = ribbon(&mut world);
        assert_eq!(segments.len(), RIBBON_SEGMENTS);
        assert!(segments[0].0.distance(Vec3::ZERO) < 1e-4);
        assert!(segments[RIBBON_SEGMENTS - 1].1.distance(end) < 1e-4);
        assert!(
            segments.iter().any(|(_, to)| to.distance(corner) < 1e-4),
            "the corner {corner} must stay a vertex"
        );
    }

    /// The gate mouth is the hull's own containment sphere plus the visual
    /// clearance, because a flip sweeps the whole hull about its centre of
    /// mass. A fixed 40 m ring is one the shipped skiff does not fit through
    /// and one the carrier swallows; the tube stays an indicator width at
    /// either size, so the ring is rebuilt rather than scaled.
    #[test]
    fn the_flip_gate_mouth_follows_the_hull() {
        const CARRIER_ENVELOPE: f32 = 19.53;

        let mouth_for = |envelope: Option<f32>| -> Option<f32> {
            let mut world = holo_world();
            let ship = spawn_ship(&mut world, prediction(BENDING_PATH.to_vec(), Some(2)));
            match envelope {
                Some(envelope) => {
                    world.entity_mut(ship).insert(HullEnvelopeRadius(envelope));
                }
                None => {
                    world.entity_mut(ship).remove::<HullEnvelopeRadius>();
                }
            }
            world.run_system_once(sync_flip_gate).unwrap();
            if world.query::<&FlipGateMarker>().iter(&world).count() == 0 {
                return None;
            }
            Some(world.resource::<HoloAssets>().gate_mesh.as_ref().unwrap().0)
        };

        let clearance = GATE_CLEARANCE.to_engine();
        assert_eq!(
            mouth_for(Some(SKIFF_ENVELOPE)),
            Some(SKIFF_ENVELOPE + clearance)
        );
        assert_eq!(
            mouth_for(Some(CARRIER_ENVELOPE)),
            Some(CARRIER_ENVELOPE + clearance)
        );
        // An unmeasured hull gets no ring at all rather than one drawn at a
        // guessed mouth.
        assert_eq!(mouth_for(None), None);
    }

    /// A hull that sheds the section holding its envelope re-sizes its gate
    /// mid-leg: the LIVE ring's mesh is swapped, not just the cache.
    #[test]
    fn the_live_gate_resizes_when_the_hull_does() {
        let mut world = holo_world();
        let ship = spawn_ship(&mut world, prediction(BENDING_PATH.to_vec(), Some(2)));
        world.entity_mut(ship).insert(HullEnvelopeRadius(19.53));
        world.run_system_once(sync_flip_gate).unwrap();
        let (gate, big) = world
            .query_filtered::<(Entity, &Mesh3d), With<FlipGateMarker>>()
            .single(&world)
            .map(|(entity, mesh)| (entity, mesh.0.clone()))
            .expect("one gate");

        world
            .entity_mut(ship)
            .insert(HullEnvelopeRadius(SKIFF_ENVELOPE));
        world.run_system_once(sync_flip_gate).unwrap();

        let small = world.entity(gate).get::<Mesh3d>().expect("the gate lives");
        assert_ne!(small.0, big, "the live ring kept its old mouth");
        assert_eq!(
            world.resource::<HoloAssets>().gate_mesh.as_ref().unwrap().0,
            SKIFF_ENVELOPE + GATE_CLEARANCE.to_engine()
        );
    }

    /// The gate sits on the predicted flip point and faces along the predicted
    /// path through it, so the ship flies through it on the curve. A leg that
    /// does not brake has none, and the gate goes once the ship reaches it,
    /// also when the flip is the leg's off-grid last point.
    #[test]
    fn the_flip_gate_sits_on_the_predicted_path_until_the_brake() {
        let mut world = holo_world();
        let ship = spawn_ship(&mut world, prediction(BENDING_PATH.to_vec(), None));
        world.run_system_once(sync_flip_gate).unwrap();
        assert_eq!(world.query::<&FlipGateMarker>().iter(&world).count(), 0);

        world
            .entity_mut(ship)
            .insert(prediction(BENDING_PATH.to_vec(), Some(2)));
        world.run_system_once(sync_flip_gate).unwrap();
        let (transform, _) = world
            .query::<(&Transform, &FlipGateMarker)>()
            .single(&world)
            .expect("one gate");
        assert_eq!(transform.translation, BENDING_PATH[2]);
        let facing = transform.rotation.mul_vec3(Vec3::Y);
        let along = (BENDING_PATH[3] - BENDING_PATH[1]).normalize();
        assert!(
            facing.distance(along) < 1e-5,
            "the gate faces along the path {along}, got {facing}"
        );

        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f32(2.0 * SAMPLE_INTERVAL));
        world.run_system_once(sync_flip_gate).unwrap();
        assert_eq!(
            world.query::<&FlipGateMarker>().iter(&world).count(),
            0,
            "the brake retires the gate"
        );

        // The leg ends between samples 3 and 4 and flips at its last point.
        world.entity_mut(ship).insert(FlightPrediction {
            final_point_time: 3.5 * SAMPLE_INTERVAL,
            ..prediction(BENDING_PATH.to_vec(), Some(4))
        });
        world.run_system_once(sync_flip_gate).unwrap();
        let (transform, _) = world
            .query::<(&Transform, &FlipGateMarker)>()
            .single(&world)
            .expect("one gate");
        assert_eq!(transform.translation, BENDING_PATH[4]);

        // Past the last point's time but before the sample grid's: the
        // ship has reached the off-grid flip.
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f32(1.75 * SAMPLE_INTERVAL));
        world.run_system_once(sync_flip_gate).unwrap();
        assert_eq!(
            world.query::<&FlipGateMarker>().iter(&world).count(),
            0,
            "the off-grid flip retires the gate at its own time"
        );
    }
}
