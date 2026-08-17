//! SPARKS: a section failing without losing any of itself.
//!
//! The damage effect for anything whose geometry has to stay whole. A turret
//! that has taken a beating still has to be a turret - the barrel has to point,
//! the mount has to turn, and a player has to be able to tell at a glance that
//! it is the thing shooting at them. Eroding it would answer the question "how
//! is that still shooting?" with "it is not, really", which is the wrong
//! answer.
//!
//! So nothing is removed. The section throws sparks instead, faster the worse
//! it is, and its geometry is untouched at every level. Same for a thruster:
//! the bell has to stay a bell.
//!
//! This is the rule the whole effect vocabulary rests on - ONLY NON-FUNCTIONAL
//! MATERIAL IS REMOVED - seen from the other side. Where a hull cell has
//! nothing but material to lose and can erode freely, a turret is all function
//! and loses nothing at all.
//!
//! Sparks are their own entities rather than a change to the section's
//! material, deliberately: [`SectionDamageTint`](super::damage_tint) already
//! owns that material, and two effects writing one material would fight over it
//! every frame.

use avian3d::prelude::{LinearVelocity, RigidBody};
use bevy::prelude::*;
use nova_gameplay::prelude::{DamageLevel, TempEntity};

/// `DamageSparks` and `DamageSparksPlugin`.
pub mod prelude {
    pub use super::{DamageSparks, DamageSparksPlugin};
}

/// The level below which a section throws nothing at all.
///
/// A section that sparks the moment it is scratched reads as broken rather than
/// as hit, and every fight starts with a scratch. The effect should say "this
/// one is failing", so it starts once the section is properly hurt.
const SPARK_THRESHOLD: f32 = 0.35;

/// Seconds between sparks at [`SPARK_THRESHOLD`], falling to
/// [`SPARK_INTERVAL_MIN`] as the level reaches 1.
const SPARK_INTERVAL_MAX: f32 = 0.9;
/// Seconds between sparks at the worst level a living section can reach.
const SPARK_INTERVAL_MIN: f32 = 0.08;

/// How long one spark lives. Short: a spark is a flash, and a long-lived one
/// reads as debris the player might think they can shoot.
const SPARK_LIFETIME_SECS: f32 = 0.35;

/// How far a spark is thrown, in units per second.
const SPARK_SPEED: f32 = 2.5;

/// How big one spark is drawn. Small enough to read as a flash rather than as a
/// piece of the ship coming off - that is erosion's job, not this one.
const SPARK_SIZE: f32 = 0.06;

/// The colour a spark is emitted at. Bright and hot, well above 1.0 so it
/// blooms.
const SPARK_EMISSIVE: LinearRgba = LinearRgba::new(6.0, 2.4, 0.5, 1.0);

/// Makes a section throw sparks as its damage level rises, WITHOUT losing any
/// geometry.
///
/// Carried by sections whose shape has to stay whole to keep working: turrets,
/// thrusters, controllers. Compose it with other effects freely - a thruster
/// that sparks and runs a ragged plume is two effects on one section, which is
/// the point of effects being components.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct DamageSparks {
    /// Seconds until the next spark. Counted down by [`throw_damage_sparks`];
    /// authored as `0.0`, which fires one on the first frame past the
    /// threshold.
    pub next: f32,
    /// How many this section has thrown, which is what varies the direction of
    /// the next one. See [`spark_direction`] for why a counter and not an RNG.
    pub thrown: u32,
}

/// Which way the `nth` spark off `section` is thrown: a point on the unit
/// sphere, spread by a hash of the two.
///
/// DETERMINISTIC, and deliberately so - the same fight sparks the same way
/// twice, which is what a re-run capture and a replay both want, and it is the
/// rule the skin's decoration already follows. It also keeps this module off
/// the global RNG, which nothing here needs to touch.
///
/// The hash is FNV-1a, the same one asteroid seeds use, and the point is placed
/// by the cylindrical-equal-area map (`z` uniform, angle uniform) so the spread
/// is even rather than bunched at the poles.
fn spark_direction(section: Entity, nth: u32) -> Vec3 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in section
        .to_bits()
        .to_le_bytes()
        .iter()
        .chain(nth.to_le_bytes().iter())
    {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }

    // Two independent fractions out of the one hash: the high half turns, the
    // low half picks the height.
    let turn = (hash >> 16) as f32 / 65_536.0 * std::f32::consts::TAU;
    let z = ((hash & 0xffff) as f32 / 65_536.0) * 2.0 - 1.0;
    let ring = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(ring * turn.cos(), ring * turn.sin(), z).normalize_or(Vec3::Y)
}

/// The one mesh and one material every spark is drawn with.
///
/// Built once and shared. A spark is spawned and thrown away several times a
/// second per failing section, and a capital in its last minute has dozens
/// failing at once - minting an asset per spark would pile up mesh and material
/// assets for the rest of the scenario, none of them ever freed.
#[derive(Resource, Debug)]
struct SparkAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for SparkAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(SPARK_SIZE, SPARK_SIZE, SPARK_SIZE));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: SPARK_EMISSIVE,
                ..default()
            });
        Self { mesh, material }
    }
}

/// Throws sparks from damaged sections that carry [`DamageSparks`].
pub struct DamageSparksPlugin;

impl Plugin for DamageSparksPlugin {
    fn build(&self, app: &mut App) {
        debug!("DamageSparksPlugin: build");

        app.register_type::<DamageSparks>();
        app.init_resource::<SparkAssets>();
        app.add_systems(Update, throw_damage_sparks);
    }
}

/// Seconds between sparks at `level`, or `None` when the section is not hurt
/// enough to throw any.
///
/// Pure, so the curve can be read without a running app.
fn spark_interval(level: f32) -> Option<f32> {
    if level < SPARK_THRESHOLD {
        return None;
    }
    // How far into the sparking range this level sits, 0 at the threshold and
    // 1 at destruction.
    let into = ((level - SPARK_THRESHOLD) / (1.0 - SPARK_THRESHOLD)).clamp(0.0, 1.0);
    Some(SPARK_INTERVAL_MAX + (SPARK_INTERVAL_MIN - SPARK_INTERVAL_MAX) * into)
}

fn throw_damage_sparks(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<SparkAssets>,
    mut q_sparking: Query<(Entity, &mut DamageSparks, &DamageLevel, &GlobalTransform)>,
) {
    let delta = time.delta_secs();
    for (section, mut sparks, level, transform) in &mut q_sparking {
        let Some(interval) = spark_interval(level.0) else {
            // Healed back above the threshold, or never below it. Reset so a
            // section that gets hurt again sparks immediately rather than
            // waiting out a timer set when it was worse off.
            sparks.next = 0.0;
            continue;
        };

        sparks.next -= delta;
        if sparks.next > 0.0 {
            continue;
        }
        sparks.next = interval;
        sparks.thrown = sparks.thrown.wrapping_add(1);

        let origin = transform.translation();
        let direction = spark_direction(section, sparks.thrown);

        commands.spawn((
            Name::new("Damage Spark"),
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(origin + direction * 0.4),
            // Kinematic and deliberately WITHOUT a collider: a spark is a
            // flash, not a thing in the world. It must not shove the ship it
            // came off, must not be shootable, and must not cost the broad
            // phase anything on a capital throwing hundreds of them.
            RigidBody::Kinematic,
            LinearVelocity(direction * SPARK_SPEED),
            TempEntity(SPARK_LIFETIME_SECS),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lightly scratched section is not a failing one, and every fight starts
    /// with a scratch.
    #[test]
    fn a_lightly_damaged_section_throws_nothing() {
        assert_eq!(spark_interval(0.0), None);
        assert_eq!(spark_interval(SPARK_THRESHOLD - 0.01), None);
        assert!(spark_interval(SPARK_THRESHOLD).is_some());
    }

    /// The worse it gets the faster it goes, all the way down.
    #[test]
    fn sparks_come_faster_the_worse_the_section_is() {
        let at = |level: f32| spark_interval(level).expect("past the threshold");

        let mut previous = at(SPARK_THRESHOLD);
        for step in 1..=10 {
            let level = SPARK_THRESHOLD + (1.0 - SPARK_THRESHOLD) * (step as f32 / 10.0);
            let interval = at(level);
            assert!(
                interval <= previous,
                "level {level} sparked slower than the level before it"
            );
            previous = interval;
        }

        assert!(
            (at(1.0) - SPARK_INTERVAL_MIN).abs() < 1e-5,
            "a section at death sparks at the fastest rate"
        );
    }

    fn spark_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(DamageSparksPlugin);
        // A manual clock's first tick is dt 0, so nothing that counts down has
        // moved until one has passed.
        app.update();
        app
    }

    fn spark_count(app: &mut App) -> usize {
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "Damage Spark")
            .count()
    }

    fn spawn_section(app: &mut App, level: f32) -> Entity {
        app.world_mut()
            .spawn((
                DamageSparks::default(),
                DamageLevel(level),
                GlobalTransform::IDENTITY,
            ))
            .id()
    }

    /// The claim the whole effect exists for: a badly damaged section throws
    /// sparks and KEEPS every part of itself, so it can still do its job.
    #[test]
    fn a_failing_section_sparks_without_losing_anything() {
        let mut app = spark_app();
        let turret = spawn_section(&mut app, 0.9);
        let children_before = app.world().get::<Children>(turret).map(|c| c.len());

        app.update();

        assert!(spark_count(&mut app) > 0, "a failing section sparks");
        assert_eq!(
            app.world().get::<Children>(turret).map(|c| c.len()),
            children_before,
            "and loses none of its own geometry doing it"
        );
        assert!(
            app.world().entities().contains(turret),
            "the section itself is untouched"
        );
    }

    /// A healthy section is silent, which is what makes a sparking one read as
    /// information rather than as decoration.
    #[test]
    fn a_healthy_section_stays_quiet() {
        let mut app = spark_app();
        spawn_section(&mut app, 0.1);

        for _ in 0..5 {
            app.update();
        }

        assert_eq!(spark_count(&mut app), 0);
    }

    /// Sparks are flashes, not debris: they clear themselves, so a long fight
    /// cannot leave a cloud of them hanging over the field.
    #[test]
    fn every_spark_clears_itself() {
        let mut app = spark_app();
        spawn_section(&mut app, 1.0);
        app.update();

        assert!(spark_count(&mut app) > 0, "delivery guard: it sparked");
        let mut q_sparks = app.world_mut().query::<(&Name, Option<&TempEntity>)>();
        for (name, temp) in q_sparks.iter(app.world()) {
            if name.as_str() == "Damage Spark" {
                assert!(temp.is_some(), "a spark must despawn itself");
            }
        }
    }
}
