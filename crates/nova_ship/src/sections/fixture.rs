//! What is bolted TO a ship as opposed to what a ship is MADE of.
//!
//! A [`SectionFixture`] hangs off a section, takes damage and comes off, and
//! never speaks for the structure it is attached to. Skin plates and the
//! greebles standing on them are the two kinds; change this module when the
//! line between structure and dressing moves.
//!
//! # Coming off is the whole of a fixture's death
//!
//! [`shed_dead_fixtures`] is the other half of the marker. Structure dies
//! through the integrity graph and a fixture is deliberately not in it, so
//! without this a spent plate would sit at zero health still stopping rounds.
//! There is no fireball and no wreck: a plate leaving IS the damage read, and
//! what it uncovers is the hull behind it.

use std::ops::Range;

use avian3d::prelude::{
    AngularVelocity, CenterOfMass, Collider, ComputeMassProperties3d, LinearVelocity,
    NoAutoCenterOfMass, RigidBody,
};
use bevy::prelude::*;
use bevy_rand::prelude::{GlobalRng, WyRand};
use nova_gameplay::prelude::{
    inherited_motion, random_unit_vector, HealthIsolated, HealthZeroMarker, TempEntity,
};
use rand::RngExt;

/// The fixture marker and the marker a shed one carries.
pub mod prelude {
    pub use super::{SectionFixture, ShedFixtureMarker};
}

/// How long a shed fixture drifts before it despawns.
///
/// Shorter than the 30 seconds a wreck gets, because a hull wears hundreds of
/// fixtures against a handful of sections: a raked ship sheds more cladding in
/// one pass than it has parts to lose in its whole life. Long enough to watch a
/// plate leave and lose it against the stars.
const SHED_LIFETIME_SECS: f32 = 12.0;

/// How fast a fixture is pushed off the hull, in world units per second, on top
/// of whatever the ship was already doing.
///
/// Under a section's kick. A plate is a sheet coming away from the frame it was
/// bolted to, not a compartment letting go, so it drifts off the hull rather
/// than being thrown clear of it.
const SHED_KICK: Range<f32> = 1.5..4.0;

/// How fast a shed fixture tumbles as it leaves, in radians per second.
///
/// Above a section's, and the difference is the read: a flat, light thing spins
/// up faster than the block of ship it was screwed to, which is what tells a
/// stripped patch of skin from a hull coming apart.
const SHED_SPIN: Range<f32> = 2.0..6.0;

/// Something attached to a section that is NOT part of the ship's structure.
///
/// A fixture has health, mass, a collider and a look, and that is the whole of
/// it. It is never a node in the integrity graph, never counted in a ship's
/// aggregate health, never in the editor palette, and never a
/// [`SectionMarker`](nova_gameplay::prelude::SectionMarker). The test for which
/// side of the line a thing falls on is CAPABILITY: if shooting it off should
/// cost the ship something it can do, it is a section, not a fixture. Cladding
/// costs the ship nothing but the cladding, so it is a fixture.
///
/// The marker earns its place because three systems must distinguish a fixture
/// from structure:
///
/// - `damage_cracks::owning_section` stops its ancestor walk at a fixture, so
///   cladding keeps its authored material instead of wearing the hull's damage.
///   A fixture's damage feedback is that it COMES OFF, and cracked cladding says
///   the ship is hurt when only its skin is.
/// - `build_ship_integrity_graph` derives connectivity from the link points of
///   everything marked a section, so a run of cladding would bridge two halves
///   of a ship that combat had already cut apart.
/// - a ship's aggregate health is the sum of its sections, and a clad ship
///   carries hundreds of plates against a handful of parts worth counting. The
///   hull bar would read the skin, not the ship.
///
/// A fixture requires [`HealthIsolated`] because it stands IN FRONT of the
/// section it hangs off rather than being part of it. Without that, a plate's
/// damage bubbles into the hull it is bolted to and cladding makes a ship take
/// more damage than going bare - and a piercing round that spends budget on the
/// plate and then hits the hull charges it twice over.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(HealthIsolated)]
pub struct SectionFixture;

/// Tags a fixture that has come off and is now drifting on its own, NAMING the
/// section it was bolted to.
///
/// Also the guard that keeps [`shed_dead_fixtures`] from shedding one twice:
/// the same absence it reads for that - a fixture with no parent left - is what
/// this records the answer to, so an observer can still ask which part of the
/// hull a piece of debris came off after the fact.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ShedFixtureMarker(pub Entity);

/// Take a spent fixture off the ship and let it tumble away.
///
/// The same finale a destroyed section gets from
/// [`explode`](nova_gameplay::integrity::explode): off the parent, out along
/// the hull normal, spinning, gone on a timer. It inherits
/// the ship's motion through [`inherited_motion`] for the reason that module
/// gives - a plate that kept only its position hangs in space while the ship
/// flies out from under it, which reads as debris being spawned rather than
/// shed.
///
/// # It is the SAME ENTITY, not a wreck built to look like it
///
/// A section detaches by spawning a body and moving its art onto it, because a
/// section's art hangs off descendants and the gameplay entity is despawned. A
/// fixture is the opposite shape: a plate carries its meshes as children and a
/// greeble carries its model on itself, so the cheapest and most faithful
/// detach is to cut the entity loose where it stands - drop `ChildOf`, promote
/// its world transform to a local one, and give it a body. Nothing is spawned,
/// copied or re-dressed, and its greebles ride it out still bolted on.
///
/// # It leaves its collider behind, but keeps where the collider stood
///
/// Shed cladding is DEBRIS, not material: kinematic, colliderless, and
/// untouchable for the seconds it lives - the same claim
/// [`spew`](nova_gameplay::integrity::spew) makes for its shards, and the
/// opposite of the one a carved chunk makes. A hull wears hundreds of
/// plates and every plate wears greebles, so the alternative is a dynamic body
/// per plate carrying a compound per greeble, which is the cost that already
/// forced a dying section to strip the colliders off everything it takes with
/// it. Flying through a sheet of tumbling cladding costs the read nothing.
///
/// The shape is still needed for one number: the PIVOT. Avian turns a body
/// about its centre of mass, and taking the collider away does not take the
/// centre with it: `ColliderMassProperties` and `ColliderTransform` OUTLIVE a
/// removed `Collider`, and that transform is the pose the plate held in the
/// SHIP's frame. So a piece cut loose from a hull inherits a centre of mass
/// tens of meters away, out where the hull's origin used to be, and swings
/// around it on a wire.
///
/// The fix is to state the centre and to say that it is the whole answer: the
/// collider's own centre, read before it goes, plus [`NoAutoCenterOfMass`] so
/// the stale props and the greebles still riding the plate are not averaged
/// back in. A fixture is authored around the face it MOUNTS ON rather than its
/// middle - a plate hangs off the floor of its cell, a greeble stands with its
/// foot at the origin - so that centre is not the entity origin either.
pub(crate) fn shed_dead_fixtures(
    mut commands: Commands,
    q_dead: Query<
        (Entity, &GlobalTransform, &ChildOf, &Collider),
        (With<SectionFixture>, With<HealthZeroMarker>),
    >,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_motion: Query<(&GlobalTransform, &LinearVelocity, Option<&AngularVelocity>)>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    for (fixture, frame, ChildOf(section), collider) in &q_dead {
        let transform = frame.compute_transform();
        // Outward from the middle of the ship, which for cladding is the way it
        // already faces: a plate stands on the hull's outer surface, so this is
        // its own normal without anything having to read one.
        let (centre, drift) =
            inherited_motion(fixture, transform.translation, &q_parents, &q_motion)
                .unwrap_or((transform.translation, Vec3::ZERO));
        let toss = random_unit_vector(&mut rng);
        let away =
            Dir3::new(transform.translation - centre).unwrap_or(Dir3::new(toss).unwrap_or(Dir3::Y));

        commands
            .entity(fixture)
            // Dropping `ChildOf` is what sheds it, and it is also the guard:
            // a fixture already off the ship has no parent to leave and so
            // never matches this query again.
            .remove::<ChildOf>()
            .remove::<Collider>()
            .insert((
                ShedFixtureMarker(*section),
                // The world pose it was standing at, now that there is no
                // parent frame to be local to.
                transform,
                RigidBody::Kinematic,
                CenterOfMass(collider.center_of_mass()),
                NoAutoCenterOfMass,
                LinearVelocity(drift + away * rng.random_range(SHED_KICK)),
                AngularVelocity(random_unit_vector(&mut rng) * rng.random_range(SHED_SPIN)),
                TempEntity(SHED_LIFETIME_SECS),
            ));

        // The greebles on a plate come with it, and their colliders do not -
        // avian attaches every collider under a body to that body, and a plate
        // that kept its dressing's shapes would be a debris body carrying a
        // compound per piece.
        let mut stack: Vec<Entity> = q_children
            .get(fixture)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        while let Some(node) = stack.pop() {
            commands.entity(node).try_remove::<Collider>();
            if let Ok(grandchildren) = q_children.get(node) {
                stack.extend(grandchildren.iter());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_rand::prelude::EntropyPlugin;
    use nova_gameplay::prelude::{Health, HealthApplyDamage, NovaHealthPlugin};

    use super::*;

    /// A ship-shaped rig: a moving rigid body, a section under it, and a
    /// fixture bolted to the section a metre out along `offset`.
    fn shed_app(offset: Vec3) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, NovaHealthPlugin));
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(7u64.to_ne_bytes()));
        app.add_systems(Update, shed_dead_fixtures);

        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                LinearVelocity(Vec3::X * 10.0),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((ChildOf(ship), Transform::default()))
            .id();
        let fixture = app
            .world_mut()
            .spawn((
                ChildOf(section),
                SectionFixture,
                Health::new(10.0),
                Collider::cuboid(1.0, 1.0, 1.0),
                Transform::from_translation(offset),
            ))
            .id();
        app.update();
        (app, section, fixture)
    }

    fn kill(app: &mut App, entity: Entity) {
        app.world_mut().trigger(HealthApplyDamage {
            entity,
            source: None,
            amount: 1000.0,
        });
        app.update();
    }

    /// A spent plate leaves the hull as a body of its own rather than being
    /// deleted where it stands. It is the SAME entity, so the meshes drawing it
    /// come along without anything having to rebuild them.
    #[test]
    fn a_spent_fixture_comes_off_the_ship_still_wearing_its_own_art() {
        let (mut app, section, fixture) = shed_app(Vec3::Y * 2.0);
        let art = app
            .world_mut()
            .spawn((ChildOf(fixture), Transform::default()))
            .id();

        kill(&mut app, fixture);

        let world = app.world();
        assert!(
            world.get::<ChildOf>(fixture).is_none(),
            "a dead plate is still bolted to the hull",
        );
        assert_eq!(
            world.get::<ShedFixtureMarker>(fixture).map(|shed| shed.0),
            Some(section),
            "shed cladding cannot say which part it came off",
        );
        assert_eq!(
            world.get::<ChildOf>(art).map(|parent| parent.0),
            Some(fixture),
            "the meshes drawing the plate did not come off with it",
        );
        assert!(
            world.get_entity(section).is_ok(),
            "the section behind the cladding died with it",
        );
    }

    /// The pose survives the cut: a plate two units up the hull is still two
    /// units up the hull once it has no parent to be local to.
    #[test]
    fn a_shed_fixture_keeps_the_place_it_was_standing() {
        let (mut app, _, fixture) = shed_app(Vec3::Y * 2.0);
        kill(&mut app, fixture);

        assert_eq!(
            app.world().get::<Transform>(fixture).unwrap().translation,
            Vec3::Y * 2.0,
        );
    }

    /// A plate leaves with the ship, not with the frame it was hit in. Without
    /// the inheritance the hull flies out from under its own cladding.
    #[test]
    fn a_shed_fixture_leaves_carrying_the_ships_motion() {
        let (mut app, _, fixture) = shed_app(Vec3::Y * 2.0);
        kill(&mut app, fixture);

        let world = app.world();
        let velocity = world.get::<LinearVelocity>(fixture).unwrap().0;
        assert!(
            velocity.x >= 10.0,
            "shed cladding lost the ship's 10 u/s of drift: {velocity}",
        );
        // Outward: the plate stands up the +Y face, so that is the way it goes.
        assert!(velocity.y > 0.0, "cladding shed INTO the hull: {velocity}");
        assert!(
            world.get::<AngularVelocity>(fixture).unwrap().0.length() >= SHED_SPIN.start,
            "shed cladding slid off instead of tumbling",
        );
    }

    /// Debris, not material. A body that kept its shapes would be a plate per
    /// collider and a greeble per compound, times every plate a burst strips.
    #[test]
    fn shed_cladding_takes_no_colliders_with_it() {
        let (mut app, _, fixture) = shed_app(Vec3::Y * 2.0);
        let greeble = app
            .world_mut()
            .spawn((
                ChildOf(fixture),
                SectionFixture,
                Collider::cuboid(0.2, 0.2, 0.2),
                Transform::default(),
            ))
            .id();

        kill(&mut app, fixture);

        let world = app.world();
        assert!(world.get::<Collider>(fixture).is_none());
        assert!(
            world.get::<Collider>(greeble).is_none(),
            "a greeble riding the wreck kept a shape on the debris body",
        );
        assert!(world.get::<TempEntity>(fixture).is_some(), "debris forever");
    }

    /// A shed piece tumbles about ITSELF. A fixture is authored around the face
    /// it mounts on, so a body left to avian's default centre swings about a
    /// point clear of the piece instead of spinning where it stands.
    #[test]
    fn shed_cladding_tumbles_about_the_shape_it_was_wearing() {
        let (mut app, section, _) = shed_app(Vec3::Y * 2.0);
        // A plate's collider in miniature: a thin box hanging off the floor of
        // its cell, which is nowhere near the entity carrying it.
        let seat = Vec3::Y * -0.4;
        let plate = app
            .world_mut()
            .spawn((
                ChildOf(section),
                SectionFixture,
                Health::new(10.0),
                Collider::compound(vec![(
                    seat,
                    Quat::IDENTITY,
                    Collider::cuboid(1.0, 0.2, 1.0),
                )]),
                Transform::default(),
            ))
            .id();
        app.update();

        kill(&mut app, plate);

        let pivot = app
            .world()
            .get::<CenterOfMass>(plate)
            .expect("shed cladding was given no centre of mass")
            .0;
        assert!(
            pivot.distance(seat) < 1.0e-4,
            "shed cladding spins about {pivot} instead of the plate at {seat}",
        );
    }

    /// A shed plate turns about its own seat, not about the hull it came off.
    ///
    /// The live solver, because this cannot be seen without it. Taking a
    /// `Collider` away leaves `ColliderMassProperties` and `ColliderTransform`
    /// behind, still holding the pose the plate had in the SHIP's frame, and
    /// avian averages those into the body's centre of mass. Unchecked, a plate
    /// two units up the hull pivots about a point two units further up again -
    /// and on a real hull that is tens of meters, which reads as debris
    /// swinging on a wire.
    #[test]
    fn a_shed_plate_turns_about_its_seat_and_not_about_the_hull_it_left() {
        use avian3d::prelude::ComputedCenterOfMass;
        use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(7u64.to_ne_bytes()));
        app.add_systems(Update, shed_dead_fixtures);
        app.finish();

        let ship = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Collider::cuboid(1.0, 1.0, 1.0),
                LinearVelocity(Vec3::X * 10.0),
                Transform::default(),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((ChildOf(ship), Transform::default()))
            .id();
        // A plate standing two units up the hull, clad the way the skin lays
        // one: the box hangs off the floor of its cell, not on the entity.
        let seat = Vec3::Y * -0.4;
        let plate = app
            .world_mut()
            .spawn((
                ChildOf(section),
                SectionFixture,
                Health::new(10.0),
                Collider::compound(vec![(
                    seat,
                    Quat::IDENTITY,
                    Collider::cuboid(1.0, 0.2, 1.0),
                )]),
                Transform::from_translation(Vec3::Y * 2.0),
            ))
            .id();
        // A greeble rides it out. Its own stale collider props would otherwise
        // be averaged into the plate's centre alongside the plate's.
        app.world_mut().spawn((
            ChildOf(plate),
            SectionFixture,
            Health::new(5.0),
            Collider::cuboid(0.2, 0.2, 0.2),
            Transform::from_translation(Vec3::Y * 0.1),
        ));
        settle(&mut app);

        kill(&mut app, plate);

        assert!(
            app.world()
                .get::<ComputedCenterOfMass>(plate)
                .expect("a shed plate is not a solver body")
                .0
                .distance(seat)
                < 1.0e-3,
            "the plate turns about the hull's frame, not its own seat",
        );

        let pose = |app: &App| {
            app.world()
                .get::<GlobalTransform>(plate)
                .expect("the shed plate went away")
                .compute_transform()
        };

        // How far a path bends away from the straight line between its own
        // ends. Measured this way rather than against `velocity * elapsed`
        // because physics runs on its own fixed step: a kick sampled per app
        // frame does not predict where the solver actually put the body, and
        // the question here is only whether the path is STRAIGHT.
        let bend = |path: &[Vec3]| {
            let (Some(first), Some(last)) = (path.first(), path.last()) else {
                return 0.0;
            };
            let Ok(along) = Dir3::new(*last - *first) else {
                return 0.0;
            };
            path.iter()
                .map(|at| (*at - *first).reject_from(*along).length())
                .fold(0.0, f32::max)
        };

        let mut seat_path = vec![pose(&app).transform_point(seat)];
        let mut origin_path = vec![pose(&app).translation];
        for _ in 0..30 {
            app.update();
            seat_path.push(pose(&app).transform_point(seat));
            origin_path.push(pose(&app).translation);
        }

        // Asserted as a PAIR. The seat runs straight because the plate turns
        // around it; the origin corkscrews because it is the thing being swung
        // through a radius of 0.4. Testing only the first half would pass a
        // plate that never turned at all.
        assert!(
            bend(&seat_path) < 1.0e-4,
            "the plate's seat bends {} off the line it was kicked along",
            bend(&seat_path),
        );
        assert!(
            bend(&origin_path) > 0.05,
            "the plate did not turn: its origin ran straight, bending only {}",
            bend(&origin_path),
        );
    }

    /// The shed runs ONCE. A fixture keeps its health pool and its marker after
    /// it comes off, so a query that only asked those two would re-kick the
    /// same piece of debris every frame.
    #[test]
    fn a_fixture_is_shed_once_and_then_left_alone() {
        let (mut app, _, fixture) = shed_app(Vec3::Y * 2.0);
        kill(&mut app, fixture);
        let velocity = app.world().get::<LinearVelocity>(fixture).unwrap().0;

        app.update();

        assert_eq!(
            app.world().get::<LinearVelocity>(fixture).unwrap().0,
            velocity,
            "shed cladding was kicked a second time",
        );
    }
}
