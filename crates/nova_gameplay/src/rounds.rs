//! Gun-round flight: a round is integrated and swept by hand, never simulated
//! as a rigid body. The round must stay a straight-line-plus-gravity path that
//! interacts with nothing but what it hits, which is what lets one shape cast
//! replace a body. Change this module when a round gains a force, a shape, or a
//! rule about what it may pass through.

use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};

use crate::prelude::*;

/// [`RoundVelocity`], [`RoundBitten`], [`RoundRake`], [`NovaRoundPlugin`] and
/// [`NovaRoundSystems`].
pub mod prelude {
    pub use super::{
        NovaRoundPlugin, NovaRoundSystems, RoundBitten, RoundRake, RoundVelocity, PIERCE_SKIN,
    };
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

/// A round that cuts WIDER than its bore: the sphere a raking slug drags behind
/// its tip, and the bodies that tip has earned it against.
///
/// The narrow tip is unchanged and is still the whole of the round's aim. What
/// it now also does is ARM a body: only a body the tip struck directly is ever
/// raked, so a widened near miss stays a miss, each separate body has to be hit
/// on its own account, and one ship cannot be opened by a shot lined up on the
/// ship beside it. Once armed it stays armed for the round's whole life, across
/// the empty space inside a hull as much as across its plating.
///
/// Behind the tip the sphere trails by exactly its own radius, which puts its
/// front face tangent to the tip. The volume it sweeps is therefore a
/// continuous cylinder over ground the round has ALREADY crossed, and it can
/// never reach a section the round has not yet arrived at. It keeps sweeping
/// after the tip leaves the far side, which is what opens an exit the same
/// width as the corridor instead of a bore-sized one.
///
/// The rake spends the SAME [`ProjectileDamage`] the tip does. No second
/// budget, no falloff, no second damage type: a section caught beside the
/// corridor takes the same flat Pierce bite and costs the same thickness. A
/// dense hull therefore spends the shell sooner, and a fighter on the
/// centreline still presents almost nothing to spend it on - the width converts
/// depth the round was wasting into damage, rather than adding lethality of its
/// own.
///
/// The two sets are exact rather than a ring, and that is forced.
/// [`RoundBitten`] is safe to forget out of BECAUSE the tip only ever casts
/// forward; the rake's volume reaches BACKWARD by two radii, so a section
/// resolved near a step boundary is offered again on the next step and a
/// forgotten entry is a section charged twice. A raking round is a rare,
/// one-in-flight thing - a lance holds a single shell behind a twelve second
/// reload - so an exact set costs nothing that the ring was protecting a
/// thousand bullets a second from.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct RoundRake {
    /// The trailing sphere's radius, in units.
    radius: f32,
    /// Bodies the narrow tip has struck, and which may therefore be raked.
    armed: Vec<Entity>,
    /// Colliders this round has already charged, by either half of the sweep.
    charged: Vec<Entity>,
}

impl RoundRake {
    /// A rake of `radius` units, armed against nothing yet.
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            armed: Vec::new(),
            charged: Vec::new(),
        }
    }

    /// The trailing sphere's radius, in units.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    fn is_armed(&self, body: Entity) -> bool {
        self.armed.contains(&body)
    }

    fn arm(&mut self, body: Entity) {
        if !self.is_armed(body) {
            self.armed.push(body);
        }
    }

    fn has_charged(&self, collider: Entity) -> bool {
        self.charged.contains(&collider)
    }

    fn charge(&mut self, collider: Entity) {
        self.charged.push(collider);
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

/// How many times the lateral contact search alternates between the corridor
/// axis and the candidate's surface.
///
/// Two settles a convex shape: the first pass lands somewhere on it, the second
/// on the point of it nearest the corridor. Every section, rock node and plate
/// the sweep meets is convex.
const CORRIDOR_CONTACT_PASSES: usize = 2;

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
        app.register_type::<RoundRake>();
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
///
/// A round wearing [`RoundRake`] takes the wider path, which resolves the same
/// contacts the same way and then also pays for what its trailing sphere swept.
#[expect(
    clippy::too_many_arguments,
    reason = "one system owning the whole sweep beats splitting the round's step across two"
)]
fn advance_rounds(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<GravitySettings>,
    world: SweepWorld,
    mut q_rounds: Query<
        (
            Entity,
            &mut Transform,
            &mut RoundVelocity,
            &mut RoundBitten,
            Option<&mut ProjectileDamage>,
            Option<&ProjectileOwner>,
            Option<&mut RoundRake>,
        ),
        With<GunRoundMarker>,
    >,
    q_wells: Query<(&Position, &GravityWell)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    // How far the fastest thing in the world travels in one step, which is the
    // most a single-instant pose can be wrong by. Widening the CANDIDATE cast
    // by it means the exact test below can never be denied a real hit; a false
    // positive only costs one more cast. Measured per step rather than fixed
    // because rounds are no longer bodies - this walks tens of ships, torpedoes
    // and rocks, not thousands of rounds.
    let fastest = world
        .velocity
        .iter()
        .fold(0.0f32, |widest, velocity| widest.max(velocity.length()));
    let reach = fastest * dt;

    for (entity, mut transform, mut velocity, mut bitten, damage, owner, rake) in &mut q_rounds {
        let start = transform.translation;
        **velocity += well_pull(start, &q_wells, &settings) * dt;
        let step = **velocity * dt;
        transform.translation = start + step;

        let Ok(direction) = Dir3::new(step) else {
            // A round at rest crossed nothing this step.
            continue;
        };
        let sweep = Sweep {
            start,
            velocity: **velocity,
            speed: velocity.length(),
            direction,
            reach,
            dt,
        };
        let Some(mut damage) = damage else {
            // A bare test spawn with no authored damage: it flies, and the
            // first thing it meets expends it, exactly as the body path did.
            resolve_undamaged(&mut commands, entity, &world, sweep, step.length(), owner);
            continue;
        };

        match rake {
            Some(mut rake) => sweep_raking(
                &mut commands,
                &world,
                entity,
                &mut transform,
                &mut bitten,
                &mut damage,
                owner,
                &mut rake,
                sweep,
            ),
            None => sweep_narrow(
                &mut commands,
                &world,
                entity,
                &mut transform,
                &mut bitten,
                &mut damage,
                owner,
                sweep,
            ),
        }
    }
}

/// One round's segment for this step, and the two step-wide numbers the resolve
/// prices it against. The whole of what both halves need to know that is not a
/// query.
#[derive(Clone, Copy)]
struct Sweep {
    /// Where the round stood at the top of the step.
    start: Vec3,
    /// Its velocity for the whole segment, gravity already integrated.
    velocity: Vec3,
    /// The length of that velocity, cached because the walk divides by it.
    speed: f32,
    /// The line it travels, in world space.
    direction: Dir3,
    /// How far the fastest thing in the world moves in one step: the widest a
    /// single-instant pose sample can be wrong by.
    reach: f32,
    /// The step, in seconds.
    dt: f32,
}

/// The world one sweep reads, gathered into one parameter so each half of the
/// resolve can be its own function instead of carrying a dozen queries down
/// through it.
#[derive(SystemParam)]
struct SweepWorld<'w, 's> {
    spatial: SpatialQuery<'w, 's>,
    sensors: Query<'w, 's, (), With<Sensor>>,
    collider_of: Query<'w, 's, &'static ColliderOf>,
    body_colliders: Query<'w, 's, &'static RigidBodyColliders>,
    collider_local: Query<'w, 's, &'static ColliderTransform>,
    shape: Query<'w, 's, &'static Collider>,
    body_pose: Query<'w, 's, (&'static Position, &'static Rotation), With<RigidBody>>,
    health: Query<'w, 's, &'static Health>,
    velocity: Query<'w, 's, &'static LinearVelocity>,
}

impl SweepWorld<'_, '_> {
    /// The body a collider hangs off, or the collider itself when it hangs off
    /// nothing. The rake's notion of ONE TARGET: a hit arms the body, and only
    /// that body's own colliders are ever raked for it.
    fn body_of(&self, collider: Entity) -> Entity {
        self.collider_of
            .get(collider)
            .map_or(collider, |of| of.body)
    }

    /// How fast whatever `collider` is attached to is moving.
    ///
    /// Velocities live on the BODIES, not the colliders. A target with no body
    /// of its own (or no velocity: a Static planetoid) counts as at rest, which
    /// is what it is.
    fn body_velocity_of(&self, collider: Entity) -> Vec3 {
        self.collider_of
            .get(collider)
            .ok()
            .and_then(|of| self.velocity.get(of.body).ok())
            .map_or(Vec3::ZERO, |velocity| **velocity)
    }

    /// The pose the sweep tests a collider at: its OWN body's sampled pose
    /// composed with its local transform.
    ///
    /// Deliberately not the collider's own [`Position`]. Avian writes a child
    /// collider's world pose at the top of the step and a body-owned one after
    /// the solver, so on a moving ship the two are a step apart - and the rake
    /// resolves everything in the body's rest frame, where one consistent pose
    /// sample is the whole of its exactness.
    fn collider_pose(&self, collider: Entity) -> Option<(Vec3, Quat)> {
        let local = self.collider_local.get(collider).ok()?;
        let of = self.collider_of.get(collider).ok()?;
        let (position, rotation) = self.body_pose.get(of.body).ok()?;
        Some((
            position.0 + rotation.0 * local.translation,
            rotation.0 * local.rotation.0,
        ))
    }

    /// Where the corridor meets `collider`: the point on it nearest the line of
    /// flight, that point's depth along the line, and its distance off it.
    ///
    /// Alternating projection, seeded from the collider's own centre - project
    /// the axis point onto the surface, re-read the depth the result sits at,
    /// project again. Two passes settle a convex shape, and every section, rock
    /// node and plate this sweep meets is convex. The point matters as much as
    /// the health does: a lateral bite recorded on the central axis carves the
    /// corridor in the wrong place and puts the impact cue somewhere the player
    /// can see nothing happened.
    fn corridor_contact(&self, collider: Entity, from: Vec3, heading: Dir3) -> Option<Corridor> {
        let (centre, rotation) = self.collider_pose(collider)?;
        let shape = self.shape.get(collider).ok()?;
        let mut at = centre;
        let mut depth = (centre - from).dot(*heading);
        for _ in 0..CORRIDOR_CONTACT_PASSES {
            let axis = from + heading * depth;
            at = shape.project_point(centre, rotation, axis, true).0;
            depth = (at - from).dot(*heading);
        }
        Some(Corridor {
            at,
            depth,
            offset: (at - from - heading * depth).length(),
        })
    }
}

/// Where one candidate sits against the corridor the round is cutting.
struct Corridor {
    /// The point on the candidate nearest the line of flight, in the frame the
    /// sweep measured it in.
    at: Vec3,
    /// How far along the line of flight that point sits.
    depth: f32,
    /// How far off the line of flight it sits.
    offset: f32,
}

/// One contact the step found: what was struck, when the round reached it, how
/// far off the bore it sat, and where the bite lands.
struct Contact {
    collider: Entity,
    /// The body it hangs off, so a direct hit can arm the rake against it.
    body: Entity,
    /// Seconds from the start of the step. THE sort key, and it is a time
    /// rather than a distance because two targets moving at different speeds
    /// have no shared notion of depth.
    elapsed: f32,
    /// Distance off the line of flight. The tie-break, so an exhausted budget
    /// leaves a centred hole rather than an arbitrary half-cut.
    offset: f32,
    /// World-space point the bite is dealt at.
    at: Vec3,
    /// Closing speed against the body, which prices the crossing.
    closing: f32,
    /// Whether the narrow tip struck it, as opposed to the trailing sphere.
    direct: bool,
}

/// What one forward cast resolved.
struct TipHit {
    collider: Entity,
    /// Seconds from the START of the step, for ordering against a lateral.
    elapsed: f32,
    /// Seconds from where the tip stood when the cast went out, which is what
    /// the walk advances by.
    impact: f32,
    at: Vec3,
    closing: f32,
}

/// The tip's walk along one step: where it has got to, what is left of the
/// step, and the near misses it has already looked past.
struct TipWalk {
    /// The cast shape, matching the collider a body-backed round carried.
    shape: Collider,
    origin: Vec3,
    /// TIME crossed and time left, not distance: the exact test works in a
    /// target's rest frame, where the round covers a different distance.
    elapsed: f32,
    remaining: f32,
    /// Candidates the exact test rejected. They are near misses this step, not
    /// bites, so they must not enter `bitten` and be ignored forever.
    rejected: [Entity; REJECT_BUDGET],
    rejects: usize,
    bites: usize,
    /// What this step's casts have already resolved. [`RoundBitten`] covers
    /// this for a walk that charges as it goes; a walk that defers the charge
    /// (see [`sweep_raking`]) has nothing else to stop the next cast finding
    /// the same collider at distance zero.
    found: Vec<Entity>,
}

impl TipWalk {
    fn new(sweep: Sweep) -> Self {
        Self {
            shape: Collider::sphere(ROUND_RADIUS),
            origin: sweep.start,
            elapsed: 0.0,
            remaining: sweep.dt,
            rejected: [Entity::PLACEHOLDER; REJECT_BUDGET],
            rejects: 0,
            bites: 0,
            found: Vec::new(),
        }
    }

    /// The next collider the tip actually reaches, skipping what it has bitten,
    /// what it has already looked past, and anything in `skip`.
    fn next(
        &mut self,
        world: &SweepWorld,
        sweep: Sweep,
        owner: Option<&ProjectileOwner>,
        bitten: &RoundBitten,
        skip: &[Entity],
    ) -> Option<TipHit> {
        while self.remaining > 0.0
            && self.bites < MAX_BITES_PER_STEP
            && self.rejects < REJECT_BUDGET
        {
            // Copied, not borrowed: the predicate is handed to the cast while
            // `bitten` still has to be written after it returns.
            let already = *bitten;
            let near = &self.rejected[..self.rejects];
            let found = &self.found;
            let candidate = world.spatial.cast_shape_predicate(
                &self.shape,
                self.origin,
                Quat::IDENTITY,
                sweep.direction,
                &ShapeCastConfig::from_max_distance(sweep.speed * self.remaining)
                    // What the candidate pass has to look past: one step of
                    // target motion (the pose sample) plus the widest
                    // acceptance margin the exact test can allow, taken at the
                    // fastest closing speed in the world so one shape serves
                    // every round.
                    .with_target_distance(sweep.reach),
                &SpatialQueryFilter::default(),
                &|collider| {
                    passable(collider, owner, &world.sensors, &world.collider_of)
                        && !already.contains(collider)
                        && !near.contains(&collider)
                        && !found.contains(&collider)
                        && !skip.contains(&collider)
                },
            )?;

            let target_velocity = world.body_velocity_of(candidate.entity);
            let Some(impact) = rest_frame_impact(
                &world.spatial,
                &self.shape,
                candidate.entity,
                self.origin,
                sweep.velocity,
                target_velocity,
                self.remaining,
                sweep.dt,
            ) else {
                // Widened past it: close enough to be a candidate, not close
                // enough to be a hit. Skip it and look further along.
                self.rejected[self.rejects] = candidate.entity;
                self.rejects += 1;
                continue;
            };

            self.bites += 1;
            self.found.push(candidate.entity);
            return Some(TipHit {
                collider: candidate.entity,
                elapsed: self.elapsed + impact,
                impact,
                at: self.origin + sweep.velocity * impact,
                closing: closing_speed(sweep.velocity, target_velocity),
            });
        }
        None
    }

    /// Restart the walk just past a resolved hit, so the next cast leaves the
    /// surface it crossed rather than starting inside it.
    fn advance(&mut self, sweep: Sweep, impact: f32) {
        let skin = if sweep.speed > 0.0 {
            PIERCE_SKIN / sweep.speed
        } else {
            0.0
        };
        let advance = (impact + skin).min(self.remaining);
        self.origin += sweep.velocity * advance;
        self.elapsed += advance;
        self.remaining -= advance;
    }
}

/// The narrow sweep: cast forward, charge whatever the exact test confirms, and
/// restart from the surface just crossed. What every round that is not raking
/// does, and what a raking round's tip still does.
#[expect(
    clippy::too_many_arguments,
    reason = "the round's whole mutable state, at one call site"
)]
fn sweep_narrow(
    commands: &mut Commands,
    world: &SweepWorld,
    entity: Entity,
    transform: &mut Transform,
    bitten: &mut RoundBitten,
    damage: &mut ProjectileDamage,
    owner: Option<&ProjectileOwner>,
    sweep: Sweep,
) {
    let mut walk = TipWalk::new(sweep);
    while let Some(hit) = walk.next(world, sweep, owner, bitten, &[]) {
        // A collider with no Health is a wall to either round type.
        let health = world.health.get(hit.collider).ok();
        trace!(
            "advance_rounds: round {:?} struck {:?} (health {}, closing {:.1}) at {:?}",
            entity,
            hit.collider,
            health.is_some(),
            hit.closing,
            hit.at
        );
        match spend_piercing_damage(
            commands,
            hit.collider,
            Some(entity),
            health,
            *damage,
            hit.closing,
            Some(hit.at),
        ) {
            Some(remainder) => {
                *damage = remainder;
                bitten.remember(hit.collider);
                walk.advance(sweep, hit.impact);
            }
            None => {
                // Expended on this layer: it stops where it struck, so the
                // last rendered frame and any hit effect sit on the surface
                // rather than a step past it.
                transform.translation = hit.at;
                commands.entity(entity).try_despawn();
                return;
            }
        }
    }
}

/// The raking sweep: find where the tip goes and what it arms, sweep the
/// trailing sphere over the ground it crossed, then charge for BOTH in the
/// order the round reached them.
///
/// THREE passes and not one, because the two halves share one budget. What
/// stops the round is not knowable while the tip is still walking: a section
/// beside the corridor costs exactly what a section on it costs, so a shell can
/// run out on a lateral hit at a depth the tip has not reached yet. Pass one
/// therefore only DISCOVERS, against a provisional budget that can only ever
/// over-estimate how far the tip gets - laterals spend more, never less - so
/// its walk is an upper bound on the real one and nothing real is missed.
/// Nothing is charged until pass three walks the merged list in travel order
/// and stops where the power runs out.
///
/// Nothing here caps the merged list. [`MAX_BITES_PER_STEP`] still bounds how
/// many times the tip CASTS, which is what it was always for, but the trailing
/// sphere's candidates come out of one intersection test and are bounded by the
/// geometry that was really there. The authored power budget is the only thing
/// that decides how much of it gets paid for.
#[expect(
    clippy::too_many_arguments,
    reason = "the round's whole mutable state, at one call site"
)]
fn sweep_raking(
    commands: &mut Commands,
    world: &SweepWorld,
    entity: Entity,
    transform: &mut Transform,
    bitten: &mut RoundBitten,
    damage: &mut ProjectileDamage,
    owner: Option<&ProjectileOwner>,
    rake: &mut RoundRake,
    sweep: Sweep,
) {
    let mut contacts: Vec<Contact> = Vec::new();
    let mut armed_now: Vec<(Entity, f32)> = Vec::new();

    // PASS ONE: the tip. It alone decides what the rake is allowed to touch -
    // a body the narrow round only passed near is never armed, so a widened
    // near miss stays a miss and one ship cannot be opened by a shot lined up
    // on the ship beside it.
    let mut walk = TipWalk::new(sweep);
    let mut provisional = *damage;
    while let Some(hit) = walk.next(world, sweep, owner, bitten, &rake.charged) {
        let body = world.body_of(hit.collider);
        if !rake.is_armed(body) && !armed_now.iter().any(|&(known, _)| known == body) {
            armed_now.push((body, hit.elapsed));
        }
        contacts.push(Contact {
            collider: hit.collider,
            body,
            elapsed: hit.elapsed,
            offset: 0.0,
            at: hit.at,
            closing: hit.closing,
            direct: true,
        });
        let Some(left) = pierce_remainder(
            provisional,
            world.health.get(hit.collider).ok(),
            hit.closing,
        ) else {
            break;
        };
        provisional = left;
        walk.advance(sweep, hit.impact);
    }

    // PASS TWO: the trailing sphere, once per armed body so each is swept in
    // the rest frame its own motion defines. A body armed THIS step is only
    // raked from the instant the tip reached it: everything before that is a
    // sphere passing a target the round had not hit yet.
    let direct: Vec<Entity> = contacts.iter().map(|contact| contact.collider).collect();
    let armed_before = rake.armed.clone();
    for body in armed_before {
        collect_rake_contacts(world, rake, body, 0.0, owner, sweep, &direct, &mut contacts);
    }
    for &(body, armed_at) in &armed_now {
        collect_rake_contacts(
            world,
            rake,
            body,
            armed_at,
            owner,
            sweep,
            &direct,
            &mut contacts,
        );
    }

    // PASS THREE: charge, in the order the round reached them. Nearer wins;
    // at the same depth the axis is paid before the edge, so a budget that runs
    // out leaves a centred hole.
    contacts.sort_by(|left, right| {
        left.elapsed
            .total_cmp(&right.elapsed)
            .then(left.offset.total_cmp(&right.offset))
            .then(left.collider.to_bits().cmp(&right.collider.to_bits()))
    });
    for contact in contacts {
        let health = world.health.get(contact.collider).ok();
        trace!(
            "advance_rounds: raking round {:?} struck {:?} (health {}, direct {}, offset {:.2}) \
             at {:?}",
            entity,
            contact.collider,
            health.is_some(),
            contact.direct,
            contact.offset,
            contact.at
        );
        match spend_piercing_damage(
            commands,
            contact.collider,
            Some(entity),
            health,
            *damage,
            contact.closing,
            Some(contact.at),
        ) {
            Some(remainder) => {
                *damage = remainder;
                bitten.remember(contact.collider);
                rake.charge(contact.collider);
                if contact.direct {
                    rake.arm(contact.body);
                }
            }
            None => {
                // Expended here. The tip is what stops, and when a LATERAL
                // spends the last of the budget the tip is level with it - the
                // sphere's front face is tangent to the tip - so the round
                // stops on the axis at that depth.
                transform.translation = sweep.start + sweep.velocity * contact.elapsed;
                commands.entity(entity).try_despawn();
                return;
            }
        }
    }
}

/// Everything the trailing sphere swept through on one armed body this step.
///
/// The sphere's centre trails the tip by exactly its own radius, so what it
/// sweeps over a step is the capsule between the two centres: its front face is
/// tangent to the tip at every instant, which is what makes a section AHEAD of
/// the round unreachable, and its rear cap is flush with the last step's, which
/// is what makes the corridor continuous. It keeps sweeping after the tip has
/// left the far side of the body, and that is what opens an exit the same width
/// as the corridor rather than a bore-sized one.
///
/// Swept in the BODY's rest frame, for the reason [`rest_frame_impact`] gives:
/// one pose sample spans a whole step, so a body that moved during it is tested
/// where it started. A volume this fat would have survived the approximation
/// where the narrow tip could not - but the contact points are what the carve
/// and the impact cue are placed at, and a corridor drawn a step of ship motion
/// away from its own hole is visible.
///
/// The capsule is resolved ANALYTICALLY, from the corridor measurement, and not
/// by handing parry a capsule collider to intersection-test. At 1500 u/s a step
/// is 23 units long, and a shape that thin and that long is badly enough
/// conditioned for GJK that shallow overlaps come back as misses: swept down a
/// 5x5x4 lattice of unit cells it reported 31 of the 36 it covers, dropping the
/// four cells whose corners reach 0.29 into it while taking the identical four
/// one layer deeper. The corridor already measures the nearest point, its depth
/// and its offset exactly, and comparing that against the sphere's centre track
/// is both the same test and a cheaper one.
#[expect(
    clippy::too_many_arguments,
    reason = "one body's sweep needs the whole flight and both exclusion sets"
)]
fn collect_rake_contacts(
    world: &SweepWorld,
    rake: &RoundRake,
    body: Entity,
    armed_at: f32,
    owner: Option<&ProjectileOwner>,
    sweep: Sweep,
    direct: &[Entity],
    out: &mut Vec<Contact>,
) {
    let body_velocity = world
        .velocity
        .get(body)
        .map_or(Vec3::ZERO, |velocity| **velocity);
    let relative = sweep.velocity - body_velocity;
    let Ok(heading) = Dir3::new(relative) else {
        // A body running exactly with the round never meets it.
        return;
    };
    let pace = relative.length();
    let closing = closing_speed(sweep.velocity, body_velocity);
    // The sphere's centre, in depth from where the round started the step: it
    // begins one radius BEHIND the start, because it trails the tip by exactly
    // that, and ends one radius behind wherever the tip finished.
    let track = -rake.radius..=(pace * sweep.dt - rake.radius);

    // The BODY's own colliders are the whole candidate set - which is what
    // arming means - so there is no world query here to cap, and no layer
    // count either. What bounds the rake is the power budget, and only that.
    let Ok(colliders) = world.body_colliders.get(body) else {
        return;
    };
    for collider in colliders.iter() {
        if direct.contains(&collider)
            || rake.has_charged(collider)
            || !passable(collider, owner, &world.sensors, &world.collider_of)
        {
            continue;
        }
        let Some(corridor) = world.corridor_contact(collider, sweep.start, heading) else {
            continue;
        };
        // Beside the track this is the offset outright, which is the cylinder;
        // past either end it leans into the cap, which is what keeps a section
        // the tip has not reached yet out.
        let lead = corridor.depth - corridor.depth.clamp(*track.start(), *track.end());
        if corridor.offset.hypot(lead) > rake.radius {
            continue;
        }
        let elapsed = (corridor.depth / pace).clamp(0.0, sweep.dt);
        if elapsed < armed_at {
            continue;
        }
        out.push(Contact {
            collider,
            body,
            elapsed,
            offset: corridor.offset,
            // Back into world space: by the time the sphere reaches this point
            // the body has run on from the pose the sweep sampled it at.
            at: corridor.at + body_velocity * elapsed,
            closing,
            direct: false,
        });
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
fn resolve_undamaged(
    commands: &mut Commands,
    entity: Entity,
    world: &SweepWorld,
    sweep: Sweep,
    distance: f32,
    owner: Option<&ProjectileOwner>,
) {
    let hit = world.spatial.cast_shape_predicate(
        &Collider::sphere(ROUND_RADIUS),
        sweep.start,
        Quat::IDENTITY,
        sweep.direction,
        &ShapeCastConfig::from_max_distance(distance).with_target_distance(sweep.reach),
        &SpatialQueryFilter::default(),
        &|collider| passable(collider, owner, &world.sensors, &world.collider_of),
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

    // ---- The rake: what a slug that cuts wider than its bore may touch ----

    /// A hull cell. Every shipped hull section is a unit cube, so the rake
    /// tests are priced against the same geometry the catalog authors.
    const RAKE_CELL: f32 = 1.0;

    /// A reinforced hull cell's health, which is the thickest layer in the
    /// catalog and the one the lance is balanced against.
    const RAKE_CELL_HP: f32 = 200.0;

    /// The base lance's rake. One cell wide: it reaches every immediate
    /// neighbour of the column the tip crossed (a face at 0.5, a diagonal at
    /// 0.707) and nothing in the ring past it (1.5).
    const RAKE_TEST_RADIUS: f32 = 1.0;

    /// Where a raking slug sets off from, comfortably clear of every fixture.
    const RAKE_START_Z: f32 = 6.0;

    /// A hull of cells on ONE body, which is what a ship is: every section is a
    /// child collider of the same root, and that root is the rake's whole
    /// notion of a target. Returns the body and its cells, in the order given.
    fn spawn_hull(app: &mut App, centres: &[Vec3], hp: f32) -> (Entity, Vec<Entity>) {
        let body = app
            .world_mut()
            .spawn((Name::new("hull"), RigidBody::Dynamic, Transform::default()))
            .id();
        let cells = centres
            .iter()
            .map(|&centre| {
                app.world_mut()
                    .spawn((
                        ChildOf(body),
                        SectionMarker,
                        Transform::from_translation(centre),
                        Collider::cuboid(RAKE_CELL, RAKE_CELL, RAKE_CELL),
                        ColliderDensity(1.0),
                        Health::new(hp),
                    ))
                    .id()
            })
            .collect();
        (body, cells)
    }

    /// The shipped lance's muzzle speed, which puts a whole fixture inside ONE
    /// step: 1500 u/s is 23 units of travel per fixed tick.
    const RAKE_SHIPPED_SPEED: f32 = 1500.0;

    /// A lance slug flying -Z from [`RAKE_START_Z`] at the anchor closing speed,
    /// where the pierce multiplier reads exactly 1.0 and a crossing costs the
    /// layer's max health outright.
    fn spawn_lance_slug(app: &mut App, power: f32, rake: Option<f32>) -> Entity {
        spawn_lance_slug_at(app, power, rake, PIERCE_TEST_SPEED)
    }

    /// The same slug at a chosen speed, for the tests that are about how much of
    /// the flight one step covers.
    fn spawn_lance_slug_at(app: &mut App, power: f32, rake: Option<f32>, speed: f32) -> Entity {
        let mut slug = app.world_mut().spawn((
            Name::new("slug"),
            RailgunSlugProjectileMarker,
            Transform::from_translation(Vec3::Z * RAKE_START_Z),
            RoundVelocity(Vec3::NEG_Z * speed),
            ProjectileDamage {
                // Past a reinforced cell's whole pool, as the shipped lance's
                // is: what a cell LOSES is its health, and the arithmetic under
                // test is the power budget.
                amount: 300.0,
                power,
                // The owner's call, and the shipped lance's: power is the only
                // bound.
                layers: u32::MAX,
                kind: DamageType::Pierce,
            },
        ));
        if let Some(radius) = rake {
            slug.insert(RoundRake::new(radius));
        }
        slug.id()
    }

    /// Fly the slug far enough to clear every rake fixture.
    fn fly(app: &mut App) {
        for _ in 0..24 {
            app.update();
        }
    }

    fn hurt(app: &App, cell: Entity, hp: f32) -> bool {
        plate_health(app, cell) < hp
    }

    /// Every [`SurfaceImpact`] the sweep reported, in order.
    #[derive(Resource, Default)]
    struct Impacts(Vec<(Entity, Vec3)>);

    fn record_impacts(impact: On<SurfaceImpact>, mut log: ResMut<Impacts>) {
        log.0.push((impact.entity, impact.at));
    }

    /// Where the sweep said it struck `cell`.
    fn impact_on(app: &App, cell: Entity) -> Vec3 {
        app.world()
            .resource::<Impacts>()
            .0
            .iter()
            .find(|(entity, _)| *entity == cell)
            .expect("the cell reported no impact")
            .1
    }

    /// A 3x3 face of cells centred on the bore, one cell deep.
    fn cross_section(z: f32) -> Vec<Vec3> {
        let mut cells = vec![Vec3::new(0.0, 0.0, z)];
        for x in [-1.0f32, 0.0, 1.0] {
            for y in [-1.0f32, 0.0, 1.0] {
                if x != 0.0 || y != 0.0 {
                    cells.push(Vec3::new(x, y, z));
                }
            }
        }
        cells
    }

    /// The old gun, unchanged. A lance with no authored rake spawns a slug with
    /// no [`RoundRake`] at all, and that slug cuts exactly the column its bore
    /// crossed - which is the whole of the compatibility promise the optional
    /// field makes to content authored before it existed.
    #[test]
    fn a_slug_with_no_rake_cuts_only_the_cell_its_bore_crossed() {
        let mut app = round_app();
        let (_, cells) = spawn_hull(&mut app, &cross_section(0.0), RAKE_CELL_HP);
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, None);
        fly(&mut app);

        assert!(
            hurt(&app, cells[0], RAKE_CELL_HP),
            "the bore's own cell was not cut"
        );
        for (index, cell) in cells.iter().enumerate().skip(1) {
            assert!(
                !hurt(&app, *cell, RAKE_CELL_HP),
                "neighbour {index} was cut by a slug carrying no rake"
            );
        }
    }

    /// The rake proper: the tip's own cell arms the body, and the sphere
    /// trailing it takes every immediate neighbour of the column with it.
    #[test]
    fn a_raked_slug_opens_the_cells_around_the_one_its_tip_crossed() {
        let mut app = round_app();
        let (_, cells) = spawn_hull(&mut app, &cross_section(0.0), RAKE_CELL_HP);
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        for (index, cell) in cells.iter().enumerate() {
            assert!(
                hurt(&app, *cell, RAKE_CELL_HP),
                "cell {index} sat inside the corridor and was not cut"
            );
        }
    }

    /// THE WHOLE CORRIDOR IN ONE STEP. At the shipped 1500 u/s a fixed tick is
    /// 23 units of travel, so a hull this size is crossed entirely inside one
    /// sweep and the trailing capsule is 23 units long against a 1-unit cell.
    ///
    /// Handing a shape that long to parry's intersection test loses the cells it
    /// only shallowly overlaps - the four corners at 0.707 read as misses while
    /// the four faces at 0.5 read as hits, and the same four one layer deeper
    /// read as hits too. The sweep resolves the capsule analytically instead,
    /// and this pins that: a corridor that thins to a plus sign at the entry
    /// face and opens to a square one cell in is the shape of that bug.
    #[test]
    fn a_raked_slug_crossing_in_one_step_still_takes_the_shallow_corners() {
        let mut app = round_app();
        let (_, cells) = spawn_hull(&mut app, &cross_section(0.0), RAKE_CELL_HP);
        settle(&mut app);

        spawn_lance_slug_at(
            &mut app,
            5_000.0,
            Some(RAKE_TEST_RADIUS),
            RAKE_SHIPPED_SPEED,
        );
        fly(&mut app);

        for (index, cell) in cells.iter().enumerate() {
            assert!(
                hurt(&app, *cell, RAKE_CELL_HP),
                "cell {index} sat inside the corridor and was not cut"
            );
        }
    }

    /// ARMING IS PER BODY. A hull the narrow tip never touched takes nothing,
    /// however far inside the trailing sphere it sits - so a shot lined up on
    /// one ship cannot open the ship flying beside it, and a widened near miss
    /// is still a miss.
    #[test]
    fn a_body_the_tip_never_touched_takes_no_rake_damage() {
        let mut app = round_app();
        let (_, aimed) = spawn_hull(&mut app, &[Vec3::ZERO], RAKE_CELL_HP);
        // Half a unit off the bore: well inside a one-unit rake, and its own
        // body.
        let (_, beside) = spawn_hull(&mut app, &[Vec3::new(1.0, 0.0, 0.0)], RAKE_CELL_HP);
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        assert!(
            hurt(&app, aimed[0], RAKE_CELL_HP),
            "the hull the shot was lined up on was not cut"
        );
        assert!(
            !hurt(&app, beside[0], RAKE_CELL_HP),
            "a hull the tip never struck was raked through its neighbour's hit"
        );
    }

    /// The sphere TRAILS. Its front face is tangent to the tip, so at every
    /// instant of the flight nothing has been cut that the round has not yet
    /// arrived at - checked as the invariant it is, once per step, rather than
    /// as one reading at the end.
    #[test]
    fn the_rake_never_reaches_a_cell_the_tip_has_not_arrived_at() {
        /// Half a cell, plus room for the step the reading is taken a moment
        /// after.
        const AHEAD_EPSILON: f32 = 1.0e-3;

        let mut app = round_app();
        let mut centres = Vec::new();
        for layer in 0..6 {
            let z = -(layer as f32) * 2.0;
            centres.push(Vec3::new(0.0, 0.0, z));
            centres.push(Vec3::new(1.0, 0.0, z));
        }
        let (_, cells) = spawn_hull(&mut app, &centres, RAKE_CELL_HP);
        settle(&mut app);

        let slug = spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        for _ in 0..24 {
            app.update();
            let Some(transform) = app.world().get::<Transform>(slug) else {
                break;
            };
            let tip = transform.translation.z;
            for (index, cell) in cells.iter().enumerate() {
                if !hurt(&app, *cell, RAKE_CELL_HP) {
                    continue;
                }
                // The near face of the cell: the first of it the round can
                // possibly have reached.
                let face = centres[index].z + RAKE_CELL * 0.5;
                assert!(
                    tip <= face + AHEAD_EPSILON,
                    "cell {index} at z {face} was cut while the tip was still back at {tip}"
                );
            }
        }
    }

    /// A section lying at an angle across the corridor is caught by the swept
    /// CAPSULE, and it is caught on the part of itself that is really in the
    /// way. Its centre sits two and a half units off the bore - past any rake -
    /// while its near end reaches inside; a search that priced candidates by
    /// their centres would call this a miss.
    #[test]
    fn an_angled_section_beside_the_corridor_is_raked() {
        let mut app = round_app();
        app.init_resource::<Impacts>();
        app.add_observer(record_impacts);
        let body = app
            .world_mut()
            .spawn((Name::new("hull"), RigidBody::Dynamic, Transform::default()))
            .id();
        let column = app
            .world_mut()
            .spawn((
                ChildOf(body),
                SectionMarker,
                Transform::from_translation(Vec3::ZERO),
                Collider::cuboid(RAKE_CELL, RAKE_CELL, RAKE_CELL),
                ColliderDensity(1.0),
                Health::new(RAKE_CELL_HP),
            ))
            .id();
        let strut = app
            .world_mut()
            .spawn((
                ChildOf(body),
                SectionMarker,
                Transform::from_translation(Vec3::new(2.5, 0.0, -2.5))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
                Collider::cuboid(0.6, 1.0, 6.0),
                ColliderDensity(1.0),
                Health::new(RAKE_CELL_HP),
            ))
            .id();
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        assert!(
            hurt(&app, column, RAKE_CELL_HP),
            "the bore's own cell was not cut"
        );
        assert!(
            hurt(&app, strut, RAKE_CELL_HP),
            "the angled strut reached into the corridor and was not raked"
        );
        let landed = impact_on(&app, strut);
        assert!(
            landed.x < RAKE_TEST_RADIUS,
            "the strut's bite was recorded at {landed:?}, out at its centre rather than on \
             the end of it that was really in the corridor"
        );
    }

    /// ONCE PER ROUND, not once per step and not once per pass. The trailing
    /// volume reaches BACKWARD, so a section resolved near a step boundary is
    /// offered again on the next step; and the body stays armed across the
    /// empty space inside a hull, so the far compartment is raked without the
    /// near one being charged twice on the way.
    #[test]
    fn a_raked_cell_is_charged_once_across_steps_and_an_internal_gap() {
        /// Deeper than the slug's bite, so what a cell lost is readable rather
        /// than clamped at zero.
        const DEEP_CELL_HP: f32 = 900.0;
        const BITE: f32 = 300.0;

        let mut app = round_app();
        let (_, cells) = spawn_hull(
            &mut app,
            &[
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                // A compartment twenty units on, past many whole steps of
                // empty space.
                Vec3::new(0.0, 0.0, -20.0),
                Vec3::new(1.0, 0.0, -20.0),
            ],
            DEEP_CELL_HP,
        );
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        for (index, cell) in cells.iter().enumerate() {
            let left = plate_health(&app, *cell);
            assert!(
                (left - (DEEP_CELL_HP - BITE)).abs() < 0.05,
                "cell {index} kept {left} of {DEEP_CELL_HP}: it was charged {} times, not once",
                (DEEP_CELL_HP - left) / BITE
            );
        }
    }

    /// The width is paid for out of the DEPTH, not added beside it. The same
    /// hull, the same budget, the same shot: the raking slug spends its power
    /// on the cells beside the corridor and stops short of a layer the narrow
    /// one reaches.
    #[test]
    fn a_lateral_bite_spends_the_slugs_power_and_stops_it_sooner() {
        /// Three crossings of a reinforced cell at the anchor speed, and a
        /// hundred over.
        const THREE_LAYERS: f32 = 700.0;

        fn deep_cell_survived(rake: Option<f32>) -> bool {
            let mut app = round_app();
            let mut centres = Vec::new();
            for layer in 0..5 {
                centres.push(Vec3::new(0.0, 0.0, -(layer as f32) * 2.0));
            }
            centres.push(Vec3::new(1.0, 0.0, 0.0));
            centres.push(Vec3::new(1.0, 0.0, -2.0));
            let (_, cells) = spawn_hull(&mut app, &centres, RAKE_CELL_HP);
            settle(&mut app);

            spawn_lance_slug(&mut app, THREE_LAYERS, rake);
            fly(&mut app);

            // The third cell down the bore: inside a narrow slug's budget,
            // past a raking one's.
            !hurt(&app, cells[2], RAKE_CELL_HP)
        }

        assert!(
            !deep_cell_survived(None),
            "a narrow slug on this budget should still reach the third layer"
        );
        assert!(
            deep_cell_survived(Some(RAKE_TEST_RADIUS)),
            "the rake cut two neighbours and still reached as deep as the narrow slug: \
             the width is not being paid for out of the same power"
        );
    }

    /// An exhausted budget leaves a CENTRED hole. Candidates are paid nearest
    /// first and, at the same depth, from the axis outward, so a shell that
    /// runs out mid-layer stops at the edge of what it opened rather than
    /// cutting an arbitrary half of it. Deterministic, too: the same shot into
    /// the same hull twice leaves the same hole.
    #[test]
    fn an_exhausted_rake_is_paid_from_the_axis_outward() {
        /// Two crossings of a reinforced cell at the anchor speed, exactly.
        const TWO_LAYERS: f32 = 400.0;
        /// Wide enough to offer the cell in the second ring as a candidate
        /// (its near face is 1.5 out), so the budget is what excludes it.
        const WIDE_RAKE: f32 = 2.0;

        fn run() -> [bool; 3] {
            let mut app = round_app();
            let (_, cells) = spawn_hull(
                &mut app,
                &[
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(2.0, 0.0, 0.0),
                ],
                RAKE_CELL_HP,
            );
            settle(&mut app);

            spawn_lance_slug(&mut app, TWO_LAYERS, Some(WIDE_RAKE));
            fly(&mut app);

            [
                hurt(&app, cells[0], RAKE_CELL_HP),
                hurt(&app, cells[1], RAKE_CELL_HP),
                hurt(&app, cells[2], RAKE_CELL_HP),
            ]
        }

        assert_eq!(
            run(),
            [true, true, false],
            "the two crossings the budget bought were not the two nearest the bore"
        );
        assert_eq!(run(), run(), "the same shot cut two different holes");
    }

    /// The sweep does not stop when the tip leaves. The sphere is a radius
    /// BEHIND the tip, so the far side of a hull is opened after the round has
    /// already passed through it - which is what makes the exit the same width
    /// as the corridor instead of a bore-sized puncture.
    #[test]
    fn the_trailing_sphere_opens_the_far_side_after_the_tip_has_left() {
        let mut app = round_app();
        let (_, cells) = spawn_hull(
            &mut app,
            &[
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(0.0, 0.0, -2.0),
                // Beside the exit and PAST the last cell on the bore: nothing
                // here is on the line of fire, so only a sweep that keeps
                // running after the tip is clear can reach it.
                Vec3::new(1.0, 0.0, -3.5),
            ],
            RAKE_CELL_HP,
        );
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        assert!(
            hurt(&app, cells[2], RAKE_CELL_HP),
            "the last cell on the bore was not crossed"
        );
        assert!(
            hurt(&app, cells[3], RAKE_CELL_HP),
            "the exit was left bore-sized: the sphere stopped sweeping with the tip"
        );
    }

    /// A lateral bite lands ON THE SECTION IT BIT. Health alone cannot tell a
    /// corridor from a needle: put every mark on the bore and the right
    /// sections die while the hole is drawn in the wrong place.
    #[test]
    fn a_lateral_bite_is_recorded_where_the_corridor_met_the_cell() {
        let mut app = round_app();
        app.init_resource::<Impacts>();
        app.add_observer(record_impacts);
        let (_, cells) = spawn_hull(
            &mut app,
            &[Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)],
            RAKE_CELL_HP,
        );
        settle(&mut app);

        spawn_lance_slug(&mut app, 5_000.0, Some(RAKE_TEST_RADIUS));
        fly(&mut app);

        let bore = impact_on(&app, cells[0]);
        let lateral = impact_on(&app, cells[1]);
        assert!(
            bore.x.abs() < 0.1,
            "the direct hit was recorded off the bore, at {bore:?}"
        );
        assert!(
            (lateral.x - RAKE_CELL * 0.5).abs() < 0.1,
            "the lateral bite was recorded at {lateral:?} rather than on the inner face of \
             the cell it cut"
        );
    }
}
