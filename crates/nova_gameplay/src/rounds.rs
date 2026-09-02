//! Gun-round flight: a round is integrated and swept by hand, never simulated
//! as a rigid body. The round must stay a straight-line-plus-gravity path that
//! interacts with nothing but what it hits, which is what lets one shape cast
//! replace a body. Change this module when a round gains a force, a shape, or a
//! rule about what it may pass through.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

/// [`RoundVelocity`], [`RoundBitten`], [`NovaRoundPlugin`] and [`NovaRoundSystems`].
pub mod prelude {
    pub use super::{NovaRoundPlugin, NovaRoundSystems, RoundBitten, RoundVelocity, PIERCE_SKIN};
}

/// A gun round's own velocity, in units/second.
///
/// Deliberately NOT avian's `LinearVelocity`: nothing integrates that on an
/// entity with no [`RigidBody`], so wearing it would claim a physics contract
/// this entity does not have. [`advance_rounds`] is the only writer.
#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq, Reflect)]
#[reflect(Component)]
#[require(RoundBitten)]
pub struct RoundVelocity(pub Vec3);

/// What this round has already bitten, so a section takes one bite per round
/// rather than one per step.
///
/// A sweep has no notion of a contact BEGINNING. Avian raised `CollisionStart`
/// once when a pair started overlapping; a cast fired fresh each step re-finds
/// any collider the round is still inside, and a section is routinely thicker
/// than one step's travel - a 4-unit plate at 100 u/s takes three steps to
/// cross. Without this a Pierce round rakes the same plate three times and
/// bites 60 where it authored 20 (which is exactly what happened).
///
/// A ring, not a list: the oldest entry is the one the round has travelled
/// furthest past, so it is the safe one to forget. Forgetting is safe because
/// the sweep only ever casts FORWARD from where the round now is - a wrapped
/// entry is a collider the round is already past and cannot meet again. A
/// railgun slug raking a whole hull inside one step wraps it routinely.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct RoundBitten {
    colliders: [Entity; BITE_MEMORY],
    next: usize,
}

impl Default for RoundBitten {
    fn default() -> Self {
        Self {
            colliders: [Entity::PLACEHOLDER; BITE_MEMORY],
            next: 0,
        }
    }
}

impl RoundBitten {
    fn contains(&self, collider: Entity) -> bool {
        self.colliders.contains(&collider)
    }

    fn remember(&mut self, collider: Entity) {
        self.colliders[self.next % BITE_MEMORY] = collider;
        self.next += 1;
    }
}

/// Ordering handle for [`advance_rounds`], so a scenario or a range can put
/// work either side of the sweep.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NovaRoundSystems;

/// The round's cast shape, matching the collider a body-backed round carries:
/// dropping the body changes what a round COSTS, not what it hits.
const ROUND_RADIUS: f32 = 0.05;

/// How many near misses one round may look past in a single step. Separate
/// from [`BITE_MEMORY`] on purpose: a round crossing a swarm is offered many
/// candidates it does not hit, and spending the pierce budget on those would
/// drop a real hit sitting behind them.
const REJECT_BUDGET: usize = 16;

/// How many bitten colliders a round remembers ACROSS steps, so a section
/// thicker than one step's travel is charged once rather than once per step.
///
/// Only the recent past needs remembering: the sweep restarts each step from
/// where the round now is and casts forward only, so anything already crossed
/// is behind it. Eight is comfortably more than the layers any round is still
/// overlapping at a step boundary - which is the one thing this size has to
/// clear, and what
/// `the_bite_ring_holds_every_layer_a_round_can_rest_inside` pins.
const BITE_MEMORY: usize = 8;

/// How many colliders one round may resolve WITHIN a single step.
///
/// Split from [`BITE_MEMORY`] because the railgun made the old shared value a
/// gameplay limit. A slug at lance speed crosses a whole hull inside one
/// 15.6 ms step, so a per-step cap of eight WAS the pierce layer cap for it,
/// silently, whatever the round authored. The ring can stay small (see above)
/// while this stays a runaway-geometry backstop, which is all it was ever
/// meant to be: a Pierce round's real bound is its power budget, and
/// [`ProjectileDamage::layers`] is the authored one.
const MAX_BITES_PER_STEP: usize = 32;

/// Nudge past a resolved hit before the next cast, so the sweep restarts
/// outside the surface it just crossed rather than inside it.
pub const PIERCE_SKIN: f32 = 1.0e-3;

/// Registers the round sweep after the physics step, in [`FixedPostUpdate`].
///
/// AFTER [`PhysicsSystems::Last`] so the round's step resolves against a world
/// that has finished moving. Which POSE that leaves a collider at is not
/// uniform, and [`rest_frame_impact`] is where that is dealt with.
pub struct NovaRoundPlugin;

impl Plugin for NovaRoundPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RoundVelocity>();
        app.register_type::<RoundBitten>();
        // The sweep integrates the well pull itself, so it needs the tunables
        // whether or not `NovaGravityPlugin` is in the app. Both plugins
        // `init_resource` the same defaulted settings, so this is
        // order-independent and never overwrites an authored one.
        app.init_resource::<GravitySettings>();
        app.add_systems(
            FixedPostUpdate,
            advance_rounds
                .after(PhysicsSystems::Last)
                .in_set(NovaRoundSystems),
        );
    }
}

/// Integrate, then cast: advance the round's velocity by whatever accelerates
/// it, advance its position by that velocity, and resolve what the segment
/// travelled crossed.
///
/// The two halves are kept separate on purpose. A cast hardcoded from `a` to
/// `a + v * dt` reads the same today and is a rewrite the day a round feels
/// anything but the wells; as written, a new force is a new term above the
/// sweep.
///
/// GRAVITY IS NOT OPTIONAL HERE. Rounds curve under wells, and a non-body
/// cannot ride `gravity_well_system` because that applies through `Forces`,
/// which only a [`RigidBody`] has. The pull is therefore recomputed here from
/// the
/// same pure [`well_accel`], so the two paths cannot drift apart in the maths -
/// only in the selection: with no `DominantWell` to carry an incumbent, a round
/// takes the strongest well outright, where a ship gets [`dominant_well`]'s
/// switch hysteresis. That is [`dominant_well`] with `current: None` by
/// construction, and at a round's couple of seconds of life inside overlapping
/// SOIs there is nothing to see.
///
/// THE TARGET MOVES TOO, and this is the whole reason the resolve is two
/// stages. `SpatialQuery` reads every collider at ONE instant, while the
/// round's segment spans a whole step - so a target that moves during that step
/// is tested where it started rather than anywhere it went. A torpedo capping
/// at 70 u/s covers 1.09 u per step, further than its own section is wide, and
/// a plain cast therefore misses it EVERY time. Point defence went from 8
/// torpedoes down to 0 on `stress_point_defense` before this was found.
///
/// No cast radius fixes it. Widening the round turns the miss into a hit only
/// once the radius exceeds a step of target motion, which is a cliff rather
/// than a curve (0 kills at 0.5 u, 47 at 1.0 against a baseline of 8) - a fat
/// round collecting every near miss, not an accurate one.
///
/// A rigid body does not have the problem: avian sweeps BOTH bodies and its
/// narrow phase is a two-body continuous test. [`rest_frame_impact`] is that
/// test without the bodies.
///
/// Each resolved collider is excluded from the rest of the step's casts. A
/// Pierce round restarting from the surface it just crossed would otherwise
/// re-hit the same section at distance zero and charge it twice.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owning the whole sweep beats splitting the round's step across two"
)]
fn advance_rounds(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<GravitySettings>,
    spatial: SpatialQuery,
    mut q_rounds: Query<
        (
            Entity,
            &mut Transform,
            &mut RoundVelocity,
            &mut RoundBitten,
            Option<&mut ProjectileDamage>,
            Option<&ProjectileOwner>,
        ),
        With<GunRoundMarker>,
    >,
    q_wells: Query<(&Position, &GravityWell)>,
    q_sensors: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
    q_health: Query<&Health>,
    q_velocity: Query<&LinearVelocity>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let round_shape = Collider::sphere(ROUND_RADIUS);
    // How far the fastest thing in the world travels in one step, which is the
    // most a single-instant pose can be wrong by. Widening the CANDIDATE cast
    // by it means the exact test below can never be denied a real hit; a false
    // positive only costs one more cast. Measured per step rather than fixed
    // because rounds are no longer bodies - this walks tens of ships, torpedoes
    // and rocks, not thousands of rounds.
    let fastest = q_velocity
        .iter()
        .fold(0.0f32, |widest, velocity| widest.max(velocity.length()));
    let reach = fastest * dt;

    for (entity, mut transform, mut velocity, mut bitten, damage, owner) in &mut q_rounds {
        let start = transform.translation;
        **velocity += well_pull(start, &q_wells, &settings) * dt;
        let step = **velocity * dt;
        transform.translation = start + step;

        let Ok(direction) = Dir3::new(step) else {
            // A round at rest crossed nothing this step.
            continue;
        };
        let speed = velocity.length();
        let Some(mut damage) = damage else {
            // A bare test spawn with no authored damage: it flies, and the
            // first thing it meets expends it, exactly as the body path did.
            resolve_undamaged(
                &mut commands,
                entity,
                &spatial,
                &round_shape,
                reach,
                start,
                direction,
                step.length(),
                owner,
                &q_sensors,
                &q_collider_of,
            );
            continue;
        };

        let mut origin = start;
        // TIME remaining in the step, not distance: the exact test works in a
        // target's rest frame, where the round covers a different distance.
        let mut remaining = dt;
        // Candidates the exact test rejected. They are near misses this step,
        // not bites, so they must not enter `bitten` and be ignored forever.
        let mut rejected = [Entity::PLACEHOLDER; REJECT_BUDGET];
        let mut rejects = 0usize;
        let mut bites = 0usize;
        // What the candidate pass has to look past: one step of target motion
        // (the pose sample) plus the widest acceptance margin the exact test
        // can allow, taken at the fastest closing speed in the world so one
        // shape serves every round.
        let candidate_margin = reach;

        while remaining > 0.0 && bites < MAX_BITES_PER_STEP && rejects < REJECT_BUDGET {
            // Copied, not borrowed: the predicate is handed to the cast while
            // `bitten` still has to be written after it returns.
            let already = *bitten;
            let near = &rejected[..rejects];
            let found = spatial.cast_shape_predicate(
                &round_shape,
                origin,
                Quat::IDENTITY,
                direction,
                &ShapeCastConfig::from_max_distance(speed * remaining)
                    .with_target_distance(candidate_margin),
                &SpatialQueryFilter::default(),
                &|collider| {
                    passable(collider, owner, &q_sensors, &q_collider_of)
                        && !already.contains(collider)
                        && !near.contains(&collider)
                },
            );
            let Some(candidate) = found else {
                break;
            };

            // Velocities live on the BODIES, not the colliders. A target with
            // no body of its own (or no velocity: a Static planetoid) counts as
            // at rest, which is what it is.
            let target_velocity = q_collider_of
                .get(candidate.entity)
                .ok()
                .and_then(|of| q_velocity.get(of.body).ok())
                .map_or(Vec3::ZERO, |velocity| **velocity);

            let Some(impact) = rest_frame_impact(
                &spatial,
                &round_shape,
                candidate.entity,
                origin,
                **velocity,
                target_velocity,
                remaining,
                dt,
            ) else {
                // Widened past it: close enough to be a candidate, not close
                // enough to be a hit. Skip it and look further along.
                rejected[rejects] = candidate.entity;
                rejects += 1;
                continue;
            };

            let at = origin + **velocity * impact;
            let closing = closing_speed(**velocity, target_velocity);
            // A collider with no Health is a wall to either round type.
            let health = q_health.get(candidate.entity).ok();
            trace!(
                "advance_rounds: round {:?} struck {:?} (health {}, closing {:.1}) at {:?}",
                entity,
                candidate.entity,
                health.is_some(),
                closing,
                at
            );
            match spend_piercing_damage(
                &mut commands,
                candidate.entity,
                Some(entity),
                health,
                *damage,
                closing,
                Some(at),
            ) {
                Some(remainder) => {
                    *damage = remainder;
                    bitten.remember(candidate.entity);
                    bites += 1;
                    let skin = if speed > 0.0 {
                        PIERCE_SKIN / speed
                    } else {
                        0.0
                    };
                    let advance = (impact + skin).min(remaining);
                    origin += **velocity * advance;
                    remaining -= advance;
                }
                None => {
                    // Expended on this layer: it stops where it struck, so the
                    // last rendered frame and any hit effect sit on the surface
                    // rather than a step past it.
                    transform.translation = at;
                    commands.entity(entity).try_despawn();
                    break;
                }
            }
        }
    }
}

/// When, within `remaining` seconds, the round actually reaches `target` -
/// solved in the TARGET's rest frame, which is the only frame where a single
/// pose sample is exact.
///
/// The round closes at `round_velocity - target_velocity` from where it is now,
/// with NO offset on the origin. That is right because of a detail of avian's
/// own bookkeeping: a collider parented to a body has its world `Position`
/// written once per step by `update_child_collider_position`, at the TOP of the
/// step, and nothing re-syncs it after the solver - so it still reads the pose
/// the round's own segment starts from.
///
/// It is a property of CHILD colliders, not of `SpatialQuery`. A collider
/// living on the body entity itself is written back post-solve and reads a
/// whole step ahead; nova spawns none (sections, torpedo sections and asteroid
/// nodes are all children), which is why this holds in production.
/// `a_round_intercepts_a_crossing_torpedo` is the guard, and it is the only
/// test built the way the game builds things.
///
/// Returns the time of impact, in seconds from the start of the segment, so the
/// caller can place the hit on the round's own path rather than in the rest
/// frame.
///
/// `None` when the two never converge - a target running exactly with the round
/// has no relative motion to close.
fn rest_frame_impact(
    spatial: &SpatialQuery,
    shape: &Collider,
    target: Entity,
    origin: Vec3,
    round_velocity: Vec3,
    target_velocity: Vec3,
    remaining: f32,
    dt: f32,
) -> Option<f32> {
    let relative = round_velocity - target_velocity;
    let direction = Dir3::new(relative).ok()?;
    let closing = relative.length();
    // The sampled pose is the target's at the START of the step. On a later
    // pierce the round has already advanced `elapsed` past that reference, so
    // the rest frame has to be pulled back by the target's travel over it -
    // otherwise every layer after the first is tested as if the round were
    // leading, by up to a full step of target motion.
    let elapsed = dt - remaining;
    let hit = spatial.cast_shape_predicate(
        shape,
        origin - target_velocity * elapsed,
        Quat::IDENTITY,
        direction,
        &ShapeCastConfig::from_max_distance(closing * remaining),
        &SpatialQueryFilter::default(),
        &|collider| collider == target,
    )?;
    Some(hit.distance / closing)
}

/// The first tangible, non-owner collider on the segment expends an
/// authorless round. Split out only to keep [`advance_rounds`] readable; a
/// production round always carries [`ProjectileDamage`].
#[expect(
    clippy::too_many_arguments,
    reason = "the cast needs its whole context and this is one call site"
)]
fn resolve_undamaged(
    commands: &mut Commands,
    entity: Entity,
    spatial: &SpatialQuery,
    shape: &Collider,
    margin: f32,
    origin: Vec3,
    direction: Dir3,
    distance: f32,
    owner: Option<&ProjectileOwner>,
    q_sensors: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
) {
    let hit = spatial.cast_shape_predicate(
        shape,
        origin,
        Quat::IDENTITY,
        direction,
        &ShapeCastConfig::from_max_distance(distance).with_target_distance(margin),
        &SpatialQueryFilter::default(),
        &|collider| passable(collider, owner, q_sensors, q_collider_of),
    );
    if hit.is_some() {
        commands.entity(entity).try_despawn();
    }
}

/// Whether the sweep may hit `collider`.
///
/// Sensors are transparent for the reason they always were: scenario trigger
/// areas, beacon spheres and blast shells are sensor colliders, and expending
/// rounds on a beacon's 70u trigger boundary once made a pirate un-hittable
/// while it patrolled near one. The firing body is transparent because the
/// muzzle sits on its own hull - the rule [`ProjectileOwner`] used to enforce
/// through avian's pair filter, which a non-body never reaches.
fn passable(
    collider: Entity,
    owner: Option<&ProjectileOwner>,
    q_sensors: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
) -> bool {
    if q_sensors.contains(collider) {
        return false;
    }
    let Some(&ProjectileOwner(owner)) = owner else {
        return true;
    };
    q_collider_of.get(collider).map(|of| of.body) != Ok(owner)
}

/// Acceleration on a round at `at` from the strongest well reaching it, or zero
/// in flat space. Mirrors `gravity_well_system`'s per-entity body minus the
/// [`DominantWell`] bookkeeping (see [`advance_rounds`]).
fn well_pull(
    at: Vec3,
    q_wells: &Query<(&Position, &GravityWell)>,
    settings: &GravitySettings,
) -> Vec3 {
    let mut strongest = 0.0;
    let mut pull = Vec3::ZERO;
    for (position, well) in q_wells {
        // Direction first: freshly spawned bodies sit at avian's
        // Position::PLACEHOLDER (Vector::MAX) until the first physics sync, so
        // a degenerate or non-finite offset is not a candidate.
        let offset = **position - at;
        let Some(toward_center) = offset.try_normalize() else {
            continue;
        };
        let accel = well_accel(
            well.mu,
            offset.length(),
            well.body_radius,
            well.soi_radius,
            settings.fade_fraction,
            settings.surface_margin,
        );
        if accel > strongest {
            strongest = accel;
            pull = toward_center * accel;
        }
    }
    pull
}

/// The round's hit contract under a real avian world: what a round damages,
/// what it passes through, what stops it, and what its budget may spend.
///
/// These moved here with the code from `nova_ship`'s turret firing, where they
/// drove the `CollisionStart` observer that the sweep replaced. Every assertion
/// is the one it made before - the mechanism changed, the contract did not.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{settle, unfinished_integrity_physics_app_with};

    /// A round world: real physics for the collider trees the sweep casts
    /// against, plus the sweep itself. No collision hooks - a round is not a
    /// body, so it never reaches avian's pair filter.
    fn round_app() -> App {
        let mut app = unfinished_integrity_physics_app_with(PhysicsPlugins::default());
        app.add_plugins(NovaRoundPlugin);
        app.finish();
        app
    }

    /// A free-floating slab of `hp` centred at `z`, [`PIERCE_PLATE_THICKNESS`]
    /// deep along the round's line of flight.
    ///
    /// Deliberately NO `ConnectedTo`: a slab that reaches zero health is
    /// disabled rather than destroyed, which keeps the render-facing explode
    /// observers (they cannot run headless) out of these tests. The pierce rule
    /// keys on the health pool, not on the destroy marker, so what is under test
    /// is unaffected.
    ///
    /// The collider is a CHILD of the body, as every collider nova spawns is
    /// (ship sections, torpedo sections, asteroid nodes). It matters: avian
    /// writes a child collider's world pose at the top of the step and a
    /// body-owned collider's after the solver, so the two are sampled a step
    /// apart. A fixture on the wrong convention hides that from every test
    /// built on it - and did, until a moving-target test was written.
    fn spawn_plate(app: &mut App, z: f32, hp: f32) -> Entity {
        let body = app
            .world_mut()
            .spawn((
                Name::new("plate"),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * z),
            ))
            .id();
        app.world_mut()
            .spawn((
                ChildOf(body),
                SectionMarker,
                Transform::default(),
                Collider::cuboid(8.0, 8.0, PIERCE_PLATE_THICKNESS),
                ColliderDensity(1.0),
                Health::new(hp),
            ))
            .id()
    }

    /// A production-shaped round flying -Z from `z` at `speed`: a transform, a
    /// velocity and an authored budget, which is the whole of one. The plates
    /// carry no `SectionClass`, so resistance is 1.0 everywhere and the
    /// arithmetic reads directly.
    fn spawn_round_at(app: &mut App, z: f32, amount: f32, kind: DamageType, speed: f32) -> Entity {
        app.world_mut()
            .spawn((
                Name::new("bullet"),
                TurretBulletProjectileMarker,
                Transform::from_translation(Vec3::Z * z),
                RoundVelocity(Vec3::NEG_Z * speed),
                ProjectileDamage::new(amount, kind),
            ))
            .id()
    }

    /// A Kinetic round at the anchor closing speed: the pre-speed round.
    fn spawn_round(app: &mut App, z: f32, amount: f32) -> Entity {
        spawn_round_at(app, z, amount, DamageType::Kinetic, PIERCE_TEST_SPEED)
    }

    fn plate_health(app: &App, plate: Entity) -> f32 {
        app.world()
            .get::<Health>(plate)
            .expect("plate still exists")
            .current
    }

    /// The pierce harness flies at the anchor, where both speed curves read 1.0,
    /// so every budget assertion is the pure pierce arithmetic. Tests that are
    /// ABOUT speed pass their own.
    const PIERCE_TEST_SPEED: f32 = REFERENCE_CLOSING_SPEED;

    /// Plate thickness and spacing along the line of flight. The pitch keeps
    /// each plate in its own step even at the 2x speed one test uses, so a
    /// multi-plate result is a rake and never one sweep resolving a stack.
    /// Tunnelling is no longer something a spacing has to prevent: the sweep
    /// resolves the whole segment travelled, at any speed.
    const PIERCE_PLATE_THICKNESS: f32 = 4.0;
    const PIERCE_PLATE_PITCH: f32 = 6.0;

    /// A round moves NOTHING it hits (playtest round 2 finding 2). A solid
    /// 0.1-mass round at 100 u/s used to shove a unit-cube target ~2.5+ u/s per
    /// hit - "1 bullet sends you off like crazy" - which the sensor collider
    /// fixed by removing the contact response. The sweep removes the contact
    /// itself, so this is now structural rather than configured, and the test
    /// stands as the guard on anyone reintroducing an impulse.
    ///
    /// Delivery guards: the health drop proves the hit landed (a missed round
    /// would also read zero knockback), and the despawn proves a round cannot
    /// sail on through everything behind its target.
    #[test]
    fn a_round_damages_its_target_without_moving_it() {
        let mut app = round_app();
        let target = app
            .world_mut()
            .spawn((
                Name::new("target"),
                RigidBody::Dynamic,
                Transform::default(),
                Collider::cuboid(2.0, 2.0, 2.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();
        settle(&mut app);
        let round = spawn_round_at(&mut app, 5.0, 20.0, DamageType::Kinetic, 100.0);

        for _ in 0..15 {
            app.update();
        }

        let health = plate_health(&app, target);
        assert!(
            health < 100.0,
            "delivery guard: the round must actually hit and damage, health {health}"
        );
        let speed = app
            .world()
            .get::<LinearVelocity>(target)
            .expect("target body")
            .length();
        assert!(
            speed < 0.05,
            "a round imparts no knockback (pre-fix: ~2.5+ u/s), got {speed}"
        );
        assert!(
            app.world().get_entity(round).is_err(),
            "the round is expended on its first hit"
        );
    }

    /// Damage is ONE number: what a round deals does not depend on which section
    /// it lands on, for either type. The round flies at exactly
    /// [`REFERENCE_CLOSING_SPEED`] into a target at rest, so the Kinetic curve
    /// reads 1.0 and the authored number is the expected one.
    #[test]
    fn a_round_deals_its_authored_damage_whatever_section_it_hits() {
        fn hit_drop(class: SectionClass, damage: ProjectileDamage) -> f32 {
            let mut app = round_app();
            let start_hp = 1000.0;
            let target = app
                .world_mut()
                .spawn((
                    Name::new("target"),
                    RigidBody::Dynamic,
                    Transform::default(),
                    Collider::cuboid(2.0, 2.0, 2.0),
                    ColliderDensity(1.0),
                    Health::new(start_hp),
                    class,
                ))
                .id();
            settle(&mut app);
            spawn_round_at(
                &mut app,
                5.0,
                damage.amount,
                damage.kind,
                REFERENCE_CLOSING_SPEED,
            );
            for _ in 0..15 {
                app.update();
            }
            start_hp - plate_health(&app, target)
        }

        let amount = 20.0;
        // Every section class, both round types, one expected number. This
        // fails the moment anything reintroduces a per-section multiplier.
        for class in [
            SectionClass::Hull,
            SectionClass::Thruster,
            SectionClass::Controller,
            SectionClass::Turret,
            SectionClass::Torpedo,
            SectionClass::Railgun,
        ] {
            for kind in [DamageType::Kinetic, DamageType::Pierce] {
                let dealt = hit_drop(class, ProjectileDamage::new(amount, kind));
                assert!(
                    (dealt - amount).abs() < 0.05,
                    "{kind:?} into {class:?} must deal its authored {amount}, dealt {dealt}"
                );
            }
        }
    }

    /// The railgun's contract, stated against the sweep: a Pierce round whose
    /// only bound is its POWER crosses a whole hull inside ONE step.
    ///
    /// This is the regression the per-step cap split exists for. While
    /// `MAX_BITES_PER_STEP` was `BITE_MEMORY` (8), a slug at lance speed
    /// crossed twelve plates of geometry and charged eight of them - the layer
    /// cap the railgun deliberately does not have, reimposed by a constant
    /// nothing authored and no test named.
    #[test]
    fn a_lance_speed_pierce_round_rakes_a_whole_hull_in_one_step() {
        const PLATES: usize = 12;
        const PLATE_HP: f32 = 60.0;

        let mut app = round_app();
        let plates: Vec<Entity> = (0..PLATES)
            .map(|layer| spawn_plate(&mut app, -(layer as f32) * PIERCE_PLATE_PITCH, PLATE_HP))
            .collect();
        settle(&mut app);

        // Fast enough to clear the whole stack in one tick, so every layer is
        // resolved inside a single `advance_rounds` pass - which is exactly
        // the case the old shared cap silently truncated.
        let speed = 8_000.0;
        app.world_mut().spawn((
            Name::new("slug"),
            RailgunSlugProjectileMarker,
            Transform::from_translation(Vec3::Z * 5.0),
            RoundVelocity(Vec3::NEG_Z * speed),
            ProjectileDamage {
                amount: 40.0,
                // Priced to outlast the stack: twelve plates at 60 max health
                // cost 720 at the reference multiplier.
                power: 4_000.0,
                // The owner's call: power is the only bound.
                layers: u32::MAX,
                kind: DamageType::Pierce,
            },
        ));
        app.update();

        for (layer, plate) in plates.iter().enumerate() {
            assert!(
                plate_health(&app, *plate) < PLATE_HP,
                "layer {layer} was not raked - the slug stopped short inside one step"
            );
        }
    }

    /// The other half of the [`BITE_MEMORY`] / [`MAX_BITES_PER_STEP`] split,
    /// which nothing pinned: the RING has to hold every layer a round can be
    /// resting inside at once.
    ///
    /// Forgetting is only safe for colliders the round has travelled PAST. A
    /// round at rest inside overlapping geometry - cladding over structure, a
    /// slow round stopped mid-stack - is re-found by every later cast, so a
    /// wrapped entry there is a layer charged twice. Eight overlapping layers
    /// is far past anything the game builds, and that headroom is the point:
    /// this fails the moment the ring is shrunk under it rather than when a
    /// slower round through thinner plating quietly starts double-biting.
    #[test]
    fn the_bite_ring_holds_every_layer_a_round_can_rest_inside() {
        /// Layers the round is inside at the same time. Fixed, not derived
        /// from [`BITE_MEMORY`] - a test that shrinks with the constant it
        /// guards proves nothing about it.
        const OVERLAPPING_LAYERS: usize = 8;
        const PLATE_HP: f32 = 1_000.0;
        const BITE: f32 = 10.0;
        /// Well under [`PIERCE_PLATE_THICKNESS`], so every layer contains the
        /// same span of the round's flight.
        const OVERLAP_PITCH: f32 = 0.3;
        /// Slow: the round has to sit inside the whole stack for several steps
        /// rather than clearing it inside one.
        const CRAWL_SPEED: f32 = 50.0;

        assert!(
            OVERLAPPING_LAYERS <= BITE_MEMORY,
            "the ring must remember every layer a round can be resting inside \
             at once; below {OVERLAPPING_LAYERS} the oldest is re-bitten"
        );
        assert!(
            BITE_MEMORY <= MAX_BITES_PER_STEP,
            "a round may resolve more layers in a step than it remembers - \
             the forgotten ones are behind it - but never fewer"
        );

        // STATIC, unlike `spawn_plate`'s free-floating slabs: eight dynamic
        // bodies born inside one another are eight bodies the solver throws
        // apart, and the ram model kills the whole stack before the round
        // arrives. Static pairs raise no contacts, and the sweep casts against
        // them exactly the same.
        let mut app = round_app();
        let plates: Vec<Entity> = (0..OVERLAPPING_LAYERS)
            .map(|layer| {
                let body = app
                    .world_mut()
                    .spawn((
                        Name::new("layer"),
                        RigidBody::Static,
                        Transform::from_translation(Vec3::NEG_Z * (layer as f32) * OVERLAP_PITCH),
                    ))
                    .id();
                app.world_mut()
                    .spawn((
                        ChildOf(body),
                        SectionMarker,
                        Transform::default(),
                        Collider::cuboid(8.0, 8.0, PIERCE_PLATE_THICKNESS),
                        ColliderDensity(1.0),
                        Health::new(PLATE_HP),
                    ))
                    .id()
            })
            .collect();
        settle(&mut app);

        app.world_mut().spawn((
            Name::new("crawler"),
            TurretBulletProjectileMarker,
            Transform::from_translation(Vec3::Z * 5.0),
            RoundVelocity(Vec3::NEG_Z * CRAWL_SPEED),
            ProjectileDamage {
                amount: BITE,
                // Priced to outlast the stack whatever the speed curve reads,
                // so what stops the round is the geometry running out.
                power: 1.0e6,
                layers: u32::MAX,
                kind: DamageType::Pierce,
            },
        ));
        // Long enough for the round to enter the stack, rest inside all of it
        // for several steps, and leave the far side.
        for _ in 0..40 {
            app.update();
        }

        for (layer, plate) in plates.iter().enumerate() {
            let dealt = PLATE_HP - plate_health(&app, *plate);
            assert!(
                (dealt - BITE).abs() < 0.01,
                "layer {layer} took {dealt} where one bite is {BITE} - the ring \
                 forgot a collider the round was still inside"
            );
        }
    }

    /// The two blind spots review R1.1/R1.2 caught when rounds became sensors,
    /// re-asserted against the sweep. A round crossing a pure trigger volume (a
    /// beacon sphere) must SURVIVE, or the pirate goes un-hittable while
    /// patrolling near a beacon; and a round into a health-less solid (an
    /// invulnerable planetoid) must still expend instead of passing through
    /// cover.
    ///
    /// The second half used to hinge on collision events: a solid with no
    /// Health never got `CollisionEventsEnabled`, so the round carried its own
    /// to make the pair report at all. A cast has no such asymmetry - it
    /// returns the collider whatever components it wears - so what this now
    /// pins is that [`passable`] keeps rejecting sensors and nothing else.
    #[test]
    fn a_round_crosses_a_trigger_volume_and_stops_at_a_health_less_solid() {
        let mut app = round_app();
        app.world_mut().spawn((
            Name::new("trigger"),
            RigidBody::Static,
            Transform::from_translation(Vec3::Z * 6.0),
            Collider::sphere(2.0),
            Sensor,
            CollisionEventsEnabled,
        ));
        app.world_mut().spawn((
            Name::new("health-less solid"),
            RigidBody::Static,
            Transform::default(),
            Collider::cuboid(3.0, 3.0, 1.0),
        ));
        settle(&mut app);

        let round = spawn_round_at(&mut app, 10.0, 20.0, DamageType::Kinetic, 100.0);

        // Past the trigger (4u of travel = 0.04s) but short of the solid.
        for _ in 0..4 {
            app.update();
        }
        assert!(
            app.world().get_entity(round).is_ok(),
            "a round crossing a trigger volume must fly on (review R1.1)"
        );

        for _ in 0..12 {
            app.update();
        }
        assert!(
            app.world().get_entity(round).is_err(),
            "a round must stop at a health-less solid instead of passing through \
             cover (review R1.2)"
        );
    }

    /// A Kinetic round that only dents its target is expended on it whatever
    /// budget arithmetic says - the pre-pierce behaviour, and the reason armour
    /// that HOLDS is still a wall.
    #[test]
    fn a_kinetic_round_stops_on_a_target_it_fails_to_destroy() {
        let mut app = round_app();
        let plate = spawn_plate(&mut app, 0.0, 100.0);
        settle(&mut app);
        let round = spawn_round(&mut app, 8.0, 20.0);

        for _ in 0..20 {
            app.update();
        }

        assert!(
            (plate_health(&app, plate) - 80.0).abs() < 0.05,
            "the whole 20-damage budget lands on the plate, got {}",
            plate_health(&app, plate)
        );
        assert!(
            app.world().get_entity(round).is_err(),
            "a round that fails to destroy its target must be expended on it"
        );
    }

    /// One crossing is charged ONCE.
    ///
    /// Two ways to get this wrong, and the sweep only has the second. A
    /// collision event raised per event-enabled collider arrives TWICE with the
    /// orderings swapped and pays its damage out twice (20 authored, 40 dealt);
    /// the sweep cannot double-report, but it CAN re-hit, since a round that
    /// pierces restarts its cast from the surface it just crossed - which is why
    /// [`advance_rounds`] excludes each resolved collider for the rest of the
    /// step.
    #[test]
    fn a_round_deals_its_authored_damage_once_per_crossing() {
        let mut app = round_app();
        let plate = spawn_plate(&mut app, 0.0, 1000.0);
        settle(&mut app);
        spawn_round(&mut app, 8.0, 20.0);

        for _ in 0..20 {
            app.update();
        }

        let dealt = 1000.0 - plate_health(&app, plate);
        assert!(
            (dealt - 20.0).abs() < 0.05,
            "a round authored at 20 must deal 20, not a doubled 40: dealt {dealt}"
        );
    }

    /// The point of the pierce rule: thin destructible cover costs a round part
    /// of its budget instead of stopping it, so the round reaches what the cover
    /// was protecting.
    #[test]
    fn a_round_that_destroys_a_thin_plate_damages_the_hull_behind_it() {
        let mut app = round_app();
        let plate = spawn_plate(&mut app, PIERCE_PLATE_PITCH, 20.0);
        let hull = spawn_plate(&mut app, 0.0, 500.0);
        settle(&mut app);
        let round = spawn_round(&mut app, 2.0 * PIERCE_PLATE_PITCH, 100.0);

        for _ in 0..40 {
            app.update();
        }

        assert_eq!(plate_health(&app, plate), 0.0, "the plate is destroyed");
        assert!(
            (plate_health(&app, hull) - 420.0).abs() < 0.05,
            "the surviving 80 of the budget must land on the hull behind, got a \
             drop of {}",
            500.0 - plate_health(&app, hull)
        );
        assert!(
            app.world().get_entity(round).is_err(),
            "the round is expended on the hull it could not destroy"
        );
    }

    /// The KINETIC budget is a hard cap: a slug crossing a stack of destructible
    /// plates deals its authored damage in total and no more, then stops - it
    /// does not re-deal its full amount to every plate on the line.
    #[test]
    fn a_kinetic_round_never_deals_more_than_the_budget_it_carries() {
        let mut app = round_app();
        let budget = 20.0;
        let plate_hp = 5.0;
        // Six 5 hp plates in a row: the 20-point budget is worth exactly four.
        let plates: Vec<Entity> = (0..6)
            .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
            .collect();
        settle(&mut app);
        let round = spawn_round(&mut app, 6.0 * PIERCE_PLATE_PITCH, budget);

        for _ in 0..60 {
            app.update();
        }

        let total_dealt: f32 = plates
            .iter()
            .map(|&plate| plate_hp - plate_health(&app, plate))
            .sum();
        assert!(
            (total_dealt - budget).abs() < 0.05,
            "a round may deal at most the {budget} it carries, dealt {total_dealt}"
        );
        // Direction guard: the damage really was spent nearest-first, on the
        // four plates the round reached, and the last two never saw it.
        for &plate in &plates[2..] {
            assert_eq!(
                plate_health(&app, plate),
                0.0,
                "the four plates nearest the muzzle are destroyed"
            );
        }
        for &plate in &plates[..2] {
            assert_eq!(
                plate_health(&app, plate),
                plate_hp,
                "a spent round must not reach the plates behind it"
            );
        }
        assert!(
            app.world().get_entity(round).is_err(),
            "the round dies when its budget runs out"
        );
    }

    /// Kinetic's identity under real physics: closing speed is DAMAGE. The same
    /// authored 20-point round hits for 30 on a charge, 20 at the anchor and 10
    /// in a stern chase, and the stern chase is driven by the TARGET's
    /// velocity - the half of the relative-velocity term a muzzle-speed-only
    /// reading would miss.
    #[test]
    fn a_kinetic_round_closing_faster_deals_more_damage_per_hit() {
        /// One hit on a plate far too tough to destroy, so the whole drop is the
        /// round's bite. `plate_speed` runs the plate away down the same line.
        fn hit_drop(round_speed: f32, plate_speed: f32) -> f32 {
            let mut app = round_app();
            let start_hp = 10_000.0;
            let plate = spawn_plate(&mut app, 0.0, start_hp);
            settle(&mut app);
            // The velocity belongs on the BODY; `spawn_plate` hands back the
            // collider, which is its child.
            let body = app.world().get::<ChildOf>(plate).expect("plate body").0;
            app.world_mut()
                .entity_mut(body)
                .insert(LinearVelocity(Vec3::NEG_Z * plate_speed));
            spawn_round_at(&mut app, 40.0, 20.0, DamageType::Kinetic, round_speed);
            for _ in 0..80 {
                app.update();
            }
            start_hp - plate_health(&app, plate)
        }

        let anchored = hit_drop(REFERENCE_CLOSING_SPEED, 0.0);
        let charging = hit_drop(1.5 * REFERENCE_CLOSING_SPEED, 0.0);
        let fleeing = hit_drop(REFERENCE_CLOSING_SPEED, 0.5 * REFERENCE_CLOSING_SPEED);

        assert!(
            (anchored - 20.0).abs() < 0.05,
            "at the reference closing speed the round deals exactly its authored \
             20, got {anchored}"
        );
        assert!(
            (charging - 30.0).abs() < 0.05,
            "closing 1.5x faster must deal 1.5x, got {charging}"
        );
        assert!(
            (fleeing - 10.0).abs() < 0.05,
            "a target running away at half the round's speed halves the hit, got \
             {fleeing}"
        );
    }

    /// The rake, end to end: a Pierce round is not stopped by a section being
    /// ALIVE. It pays that section's thickness out of its power and carries on
    /// into what the section was shielding, dealing its full bite there too.
    /// The A/B is a Kinetic round on the same rig, which stops dead on the front
    /// plate - the difference is the TYPE, not the speed.
    #[test]
    fn a_pierce_round_rakes_through_a_living_section_and_hits_what_is_behind() {
        /// `(front drop, back drop)` for a round of `kind` into two 100 hp
        /// plates, neither of which a 20-point bite can destroy.
        fn run(kind: DamageType) -> (f32, f32) {
            let mut app = round_app();
            let armour = 100.0;
            let front = spawn_plate(&mut app, 0.0, armour);
            let back = spawn_plate(&mut app, -2.0 * PIERCE_PLATE_PITCH, armour);
            settle(&mut app);
            spawn_round_at(
                &mut app,
                2.0 * PIERCE_PLATE_PITCH,
                20.0,
                kind,
                PIERCE_TEST_SPEED,
            );
            for _ in 0..40 {
                app.update();
            }
            (
                armour - plate_health(&app, front),
                armour - plate_health(&app, back),
            )
        }

        let (front, back) = run(DamageType::Pierce);
        assert!(
            (front - 20.0).abs() < 0.05,
            "the front section takes the round's authored bite, got {front}"
        );
        assert!(
            (back - 20.0).abs() < 0.05,
            "and so does what was behind it, UNDIMINISHED - the rake does not \
             decay with depth, got {back}"
        );

        let (front, back) = run(DamageType::Kinetic);
        assert!(
            (front - 20.0).abs() < 0.05,
            "the slug bites the same 20 on arrival, got {front}"
        );
        assert_eq!(
            back, 0.0,
            "but it is spent there: a slug travels only through what it destroys"
        );
    }

    /// The invariant the rake deliberately breaks: a Pierce round's TOTAL damage
    /// exceeds what it was fired with, because it pays for travel out of power
    /// and its damage never depletes. [`MAX_PIERCE_LAYERS`] is what ends it.
    #[test]
    fn a_pierce_round_deals_more_in_total_than_it_was_fired_with() {
        let mut app = round_app();
        let amount = 20.0;
        // 30 hp each: every plate SURVIVES its 20-point bite, so nothing here is
        // the old kill-to-continue rule wearing a different hat.
        let plate_hp = 30.0;
        let count = 8;
        let plates: Vec<Entity> = (0..count)
            .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
            .collect();
        settle(&mut app);
        spawn_round_at(
            &mut app,
            count as f32 * PIERCE_PLATE_PITCH,
            amount,
            DamageType::Pierce,
            PIERCE_TEST_SPEED,
        );

        for _ in 0..80 {
            app.update();
        }

        let raked = plates
            .iter()
            .filter(|&&plate| plate_health(&app, plate) < plate_hp)
            .count();
        let dealt: f32 = plates
            .iter()
            .map(|&plate| plate_hp - plate_health(&app, plate))
            .sum();
        assert_eq!(
            raked, MAX_PIERCE_LAYERS as usize,
            "the layer cap is what ends a rake through cheap plates"
        );
        assert!(
            dealt > amount,
            "a rake's total must EXCEED the amount it was fired with, got {dealt}"
        );
        assert!(
            (dealt - amount * MAX_PIERCE_LAYERS as f32).abs() < 0.05,
            "six layers x the authored 20, undiminished by depth, got {dealt}"
        );
    }

    /// Speed is POWER for a penetrator: the same round closing at 2x pays half
    /// as much per layer and rakes deeper, while its per-hit damage does not
    /// move at all. 100 hp plates cost 100 power each at the anchor (three of a
    /// 300 power budget) and 50 each at 2x (the layer cap binds first).
    #[test]
    fn a_fast_pierce_round_rakes_deeper_without_biting_harder() {
        /// `(layers raked, damage on the first layer)` for a Pierce round at
        /// `speed` down a stack of 100 hp plates.
        fn run(speed: f32) -> (usize, f32) {
            let mut app = round_app();
            let plate_hp = 100.0;
            let count = 8;
            let plates: Vec<Entity> = (0..count)
                .map(|index| spawn_plate(&mut app, index as f32 * PIERCE_PLATE_PITCH, plate_hp))
                .collect();
            settle(&mut app);
            spawn_round_at(
                &mut app,
                count as f32 * PIERCE_PLATE_PITCH,
                20.0,
                DamageType::Pierce,
                speed,
            );
            for _ in 0..120 {
                app.update();
            }
            let raked = plates
                .iter()
                .filter(|&&plate| plate_health(&app, plate) < plate_hp)
                .count();
            // The round enters from +Z, so the LAST plate is the first one hit.
            let first_hit = plate_hp - plate_health(&app, plates[count - 1]);
            (raked, first_hit)
        }

        let (anchored, anchored_bite) = run(PIERCE_TEST_SPEED);
        let (fast, fast_bite) = run(2.0 * PIERCE_TEST_SPEED);
        assert_eq!(
            anchored, 3,
            "300 power buys three 100 hp layers at the anchor"
        );
        assert!(
            fast > anchored,
            "closing at 2x must rake deeper: {fast} vs {anchored}"
        );
        assert!(
            (fast_bite - anchored_bite).abs() < 0.05,
            "and must NOT bite harder for it: {fast_bite} vs {anchored_bite}"
        );
    }

    /// A round intercepts a CLOSING torpedo: the point-defence case, whose
    /// collider is a child section of a body that is itself moving fast toward
    /// the round.
    #[test]
    fn a_round_intercepts_a_closing_torpedo() {
        let mut app = round_app();
        let torpedo = app
            .world_mut()
            .spawn((
                Name::new("torpedo"),
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::Z * -30.0),
                LinearVelocity(Vec3::Z * 60.0),
            ))
            .id();
        let warhead = app
            .world_mut()
            .spawn((
                ChildOf(torpedo),
                SectionMarker,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 3.0),
                ColliderDensity(1.0),
                Health::new(100.0),
                ActiveCollisionHooks::FILTER_PAIRS,
            ))
            .id();
        settle(&mut app);
        spawn_round_at(&mut app, 30.0, 20.0, DamageType::Kinetic, 400.0);

        for _ in 0..30 {
            app.update();
        }

        let health = app
            .world()
            .get::<Health>(warhead)
            .expect("warhead alive at 100 hp")
            .current;
        assert!(
            health < 100.0,
            "a round must connect with a closing torpedo; warhead untouched at {health}"
        );
    }

    /// A round intercepts a CROSSING torpedo, which is the case point defence
    /// actually shoots and the one a single-instant cast cannot do.
    ///
    /// A head-on closer (the test above) hides the defect: the target's
    /// per-step displacement lies ALONG the round's path, where being a step
    /// out changes only when it hits, not whether. Across the path the same
    /// displacement is a pure miss distance, and at 70 u/s it is 1.09 u per
    /// step - wider than the section. This is the geometry that took
    /// `stress_point_defense` from 8 torpedoes down to 0.
    ///
    /// Laid out so the round and the torpedo are AIMED to meet: the round
    /// covers 30 u at 400 u/s in 0.075 s, the torpedo covers 5.25 u across in
    /// the same time, so a round launched from x = 5.25 meets it at the origin.
    #[test]
    fn a_round_intercepts_a_crossing_torpedo() {
        let mut app = round_app();
        let torpedo = app
            .world_mut()
            .spawn((
                Name::new("torpedo"),
                RigidBody::Dynamic,
                Transform::default(),
                LinearVelocity(Vec3::X * 70.0),
            ))
            .id();
        let warhead = app
            .world_mut()
            .spawn((
                ChildOf(torpedo),
                SectionMarker,
                Transform::default(),
                Collider::cuboid(1.0, 1.0, 3.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();
        settle(&mut app);
        // settle() has already run the torpedo forward, so aim off where it IS.
        let lead = app
            .world()
            .get::<Position>(torpedo)
            .expect("torpedo body")
            .0
            + Vec3::X * (70.0 * 30.0 / 400.0);
        let round = spawn_round_at(&mut app, 30.0, 20.0, DamageType::Kinetic, 400.0);
        app.world_mut()
            .entity_mut(round)
            .insert(Transform::from_translation(Vec3::new(lead.x, 0.0, 30.0)));

        for _ in 0..30 {
            app.update();
        }

        let health = app
            .world()
            .get::<Health>(warhead)
            .expect("warhead alive at 100 hp")
            .current;
        assert!(
            health < 100.0,
            "a round must connect with a torpedo crossing its path; warhead \
             untouched at {health}"
        );
    }

    /// A round still CURVES under a well, and one out of reach flies straight.
    ///
    /// The regression this exists for: `gravity_well_system` reaches an entity
    /// through `Forces`, which only a [`RigidBody`] has, so the moment a round
    /// stopped being a body it silently left the affected set. The gravity
    /// module's own curve test could not catch that - its fixture builds a
    /// `RigidBody::Dynamic` and would keep passing while every round in the
    /// game flew dead straight. This one flies the real thing.
    ///
    /// Geometry mirrors that test so the two are comparable: well at the
    /// origin, the round entering at x = 40 deep inside the SOI and flying -Z
    /// straight past it, deflection measured off the entry lane.
    #[test]
    fn a_round_curves_under_a_well_and_flies_straight_without_one() {
        fn deflection(with_well: bool) -> f32 {
            let lane = 40.0;
            let mut app = round_app();
            if with_well {
                app.world_mut().spawn((
                    RigidBody::Static,
                    Transform::default(),
                    GravityWell {
                        mu: 1200.0,
                        body_radius: 20.0,
                        soi_radius: 160.0,
                    },
                ));
            }
            settle(&mut app);
            let round = spawn_round_at(&mut app, 60.0, 20.0, DamageType::Kinetic, 40.0);
            app.world_mut()
                .entity_mut(round)
                .insert(Transform::from_translation(Vec3::new(lane, 0.0, 60.0)));
            // ~4s: z sweeps +60 -> -60 through closest approach.
            for _ in 0..240 {
                app.update();
            }
            let at = app
                .world()
                .get::<Transform>(round)
                .expect("the round hit nothing")
                .translation;
            lane - at.x // toward the well is positive
        }

        let pulled = deflection(true);
        let control = deflection(false);
        assert!(
            control.abs() < 0.05,
            "with no well in the scene a round must fly straight; drifted {control}u"
        );
        assert!(
            pulled > 2.0,
            "a round must curve toward the well; deflection {pulled}u \
             (well-free control drifted {control}u)"
        );
    }

    /// A round never hits the ship that fired it. The muzzle sits ON the
    /// shooter's hull, so a round spawns overlapping it and leaves at muzzle
    /// speed; without the owner rule the first sweep would expend every round
    /// on its own ship. This is what [`ProjectileOwner`] used to buy through
    /// avian's pair filter, which a non-body never reaches.
    #[test]
    fn a_round_flies_out_of_the_hull_that_fired_it() {
        let mut app = round_app();
        let shooter = app
            .world_mut()
            .spawn((
                Name::new("shooter"),
                RigidBody::Dynamic,
                Transform::default(),
                Collider::cuboid(4.0, 4.0, 4.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();
        settle(&mut app);

        // Spawned INSIDE the shooter's own collider, as a muzzle on the hull is.
        let round = spawn_round(&mut app, 0.0, 20.0);
        app.world_mut()
            .entity_mut(round)
            .insert(ProjectileOwner(shooter));

        for _ in 0..10 {
            app.update();
        }

        assert!(
            app.world().get_entity(round).is_ok(),
            "the round must fly out of its own ship, not expend on it"
        );
        assert_eq!(
            plate_health(&app, shooter),
            100.0,
            "and must not damage it on the way"
        );
    }
}
