//! The sparks a hit throws, seen at the moment of impact.
//!
//! This is the burst, not the leak. `nova_ship`'s damage sparks are a STATE
//! effect - a section hurt past a threshold throws sparks continuously, faster
//! the worse it is - and they say nothing about the instant a round landed.
//! What used to say that was an expanding gizmo ring, which is the most-seen
//! effect in a fight and reads as a diagram of a hit rather than as one.
//!
//! # Why meshes and not a particle graph
//!
//! A hanabi effect is an instance with a GPU buffer of its own, and an impact
//! happens tens of times a second under sustained PDC fire. A handful of
//! kinematic quads costs a transform each and nothing to allocate. The blast,
//! which happens once and wants hundreds of particles, is the opposite case and
//! is a graph for the same reason.
//!
//! # Why a separate event
//!
//! `juice` decides WHETHER a hit is worth a cue and how strong - it owns the
//! throttle, the distance falloff and the settings toggle - and then says so.
//! This module decides what that looks like. A mod replacing the observer
//! changes the look and inherits every gate for free, the same split
//! [`CarveSpew`](crate::integrity::spew) already uses.

use avian3d::prelude::{LinearVelocity, RigidBody};
use bevy::prelude::*;

use crate::prelude::TempEntity;

/// `ImpactSparks`, `ImpactSparkMarker` and `ImpactSparkPlugin`.
pub mod prelude {
    pub use super::{ImpactSparkMarker, ImpactSparkPlugin, ImpactSparks};
}

/// Asks for a spark burst at a world position.
///
/// `count` is already resolved: the caller has applied its own distance falloff
/// and settings, so a burst asked for is a burst thrown.
#[derive(Event, Clone, Copy, Debug)]
pub struct ImpactSparks {
    /// Where the hit landed, in world space.
    pub at: Vec3,
    /// How many sparks to throw. Zero is a legal ask and does nothing.
    pub count: u32,
    /// Scales how far they fly, `0..1`, so a glancing hit does not spray like a
    /// killing one.
    pub force: f32,
}

/// A spark in flight. A range can count these to assert a burst happened.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ImpactSparkMarker;

/// Edge length of a spark across its short axes, world units.
const SPARK_THICKNESS: f32 = 0.02;

/// Length of a spark along its travel, world units.
///
/// Eight times the thickness, because a spark is seen as a STREAK and never as
/// a point: at the speeds below it crosses several times its own length in a
/// frame, and a cube at that speed reads as a flickering dot.
const SPARK_LENGTH: f32 = 0.16;

/// Seconds a spark lives.
///
/// Short. This is the flash half of an impact - the shards
/// ([`spew`](crate::integrity::spew)) are the half that lasts long enough to
/// sell the direction, and a spark that outlives them stops reading as heat.
const SPARK_LIFETIME_SECS: f32 = 0.28;

/// Slowest and fastest a spark leaves the hit, world units per second, before
/// [`ImpactSparks::force`] scales it.
const SPARK_SPEED_MIN: f32 = 5.0;
const SPARK_SPEED_MAX: f32 = 20.0;

/// How hot a spark glows. Well past 1.0 so it blooms.
const SPARK_EMISSIVE: LinearRgba = LinearRgba::new(9.0, 4.4, 1.1, 1.0);

/// Which way and how fast the `nth` spark of a burst at `at` is thrown.
///
/// DETERMINISTIC, like the damage-spark spread and the carve shards, and for
/// the same reasons: the same fight throws the same sparks twice, which is what
/// a re-run capture and a replay both want, and it keeps this off the global
/// RNG.
///
/// Over a whole sphere and not a cone. At this seam the incoming direction is
/// not known - the damage event names the target, not the shooter - and a hit
/// throws heat every way in any case. The shards carry the directional cue.
fn spark_throw(at: Vec3, nth: u32) -> (Vec3, f32) {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in
        at.x.to_bits()
            .to_le_bytes()
            .iter()
            .chain(at.y.to_bits().to_le_bytes().iter())
            .chain(at.z.to_bits().to_le_bytes().iter())
            .chain(nth.to_le_bytes().iter())
    {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    // Three independent fractions out of the one hash: the top half turns, the
    // next byte picks the height, the last byte picks the speed.
    let turn = (hash >> 16) as f32 / 65_536.0 * std::f32::consts::TAU;
    let z = ((hash >> 8) & 0xff) as f32 / 256.0 * 2.0 - 1.0;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    let speed_t = (hash & 0xff) as f32 / 256.0;

    let direction = Vec3::new(ring * turn.cos(), ring * turn.sin(), z).normalize_or(Vec3::Y);
    let speed = SPARK_SPEED_MIN + (SPARK_SPEED_MAX - SPARK_SPEED_MIN) * speed_t;
    (direction, speed)
}

/// The one mesh and one material every spark is drawn with.
///
/// Lazy rather than [`FromWorld`], because juice runs in headless apps that
/// have no asset stores at all: a test rig added the plugin for its throttle
/// and its trauma, not to draw anything.
#[derive(Resource, Default, Debug)]
struct ImpactSparkAssets(Option<(Handle<Mesh>, Handle<StandardMaterial>)>);

impl ImpactSparkAssets {
    fn get(
        &mut self,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        self.0
            .get_or_insert_with(|| {
                // Long on Z, because `Transform::looking_to` points -Z along
                // the throw and the streak has to lie along travel.
                let mesh = meshes.add(Cuboid::new(SPARK_THICKNESS, SPARK_THICKNESS, SPARK_LENGTH));
                let material = materials.add(StandardMaterial {
                    base_color: Color::BLACK,
                    emissive: SPARK_EMISSIVE,
                    ..default()
                });
                (mesh, material)
            })
            .clone()
    }
}

/// Throws a spark burst wherever an [`ImpactSparks`] asks for one.
pub struct ImpactSparkPlugin;

impl Plugin for ImpactSparkPlugin {
    fn build(&self, app: &mut App) {
        trace!("ImpactSparkPlugin: build");

        app.register_type::<ImpactSparkMarker>();
        app.init_resource::<ImpactSparkAssets>();
        app.add_observer(throw_impact_sparks);
    }
}

fn throw_impact_sparks(
    burst: On<ImpactSparks>,
    mut commands: Commands,
    mut cached: ResMut<ImpactSparkAssets>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    if burst.count == 0 {
        return;
    }

    // A world with no asset stores has nothing to draw with and nothing that
    // could see the result: a headless server, or a test app that took the
    // juice plugin for its throttle alone.
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let (mesh, material) = cached.get(&mut meshes, &mut materials);

    trace!(
        "throw_impact_sparks: {} spark(s) at {}",
        burst.count,
        burst.at
    );

    for nth in 0..burst.count {
        let (direction, speed) = spark_throw(burst.at, nth);
        commands.spawn((
            Name::new("Impact Spark"),
            ImpactSparkMarker,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            // Kinematic and WITHOUT a collider, exactly as damage sparks and
            // carve shards are: a spark is born inside the hull it came off,
            // and a dynamic collider there would shove the ship every time it
            // was shot.
            Transform::from_translation(burst.at).looking_to(direction, Vec3::Y),
            RigidBody::Kinematic,
            LinearVelocity(direction * speed * burst.force),
            TempEntity(SPARK_LIFETIME_SECS),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rig with the plugin and the asset stores a spark needs.
    fn spark_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(ImpactSparkPlugin);
        app
    }

    fn spark_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<ImpactSparkMarker>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn a_burst_throws_the_count_it_asked_for() {
        let mut app = spark_app();
        app.world_mut().trigger(ImpactSparks {
            at: Vec3::ZERO,
            count: 6,
            force: 1.0,
        });
        app.update();
        assert_eq!(spark_count(&mut app), 6);
    }

    #[test]
    fn a_zero_count_burst_throws_nothing() {
        let mut app = spark_app();
        app.world_mut().trigger(ImpactSparks {
            at: Vec3::ZERO,
            count: 0,
            force: 1.0,
        });
        app.update();
        assert_eq!(spark_count(&mut app), 0);
    }

    #[test]
    fn the_throw_is_deterministic_and_spread_over_a_sphere() {
        let at = Vec3::new(3.0, -1.0, 7.0);
        assert_eq!(
            spark_throw(at, 4),
            spark_throw(at, 4),
            "same ask, same spark"
        );

        let directions: Vec<Vec3> = (0..24).map(|nth| spark_throw(at, nth).0).collect();
        for direction in &directions {
            assert!(
                (direction.length() - 1.0).abs() < 1e-4,
                "every throw is a unit direction"
            );
        }
        // The burst must not all go one way: the mean of a sphere's worth of
        // directions is near zero, while a cone's is not.
        let mean = directions.iter().sum::<Vec3>() / directions.len() as f32;
        assert!(
            mean.length() < 0.4,
            "a burst sprays every way, not one way (mean {mean})"
        );
    }

    #[test]
    fn a_spark_flies_at_a_speed_inside_the_declared_range() {
        let at = Vec3::new(-2.0, 5.0, 0.5);
        for nth in 0..32 {
            let (_, speed) = spark_throw(at, nth);
            assert!(
                (SPARK_SPEED_MIN..=SPARK_SPEED_MAX).contains(&speed),
                "spark {nth} left at {speed}"
            );
        }
    }
}
