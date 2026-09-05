//! AI gunnery: turret aim, the burst [`AIFireCadence`], the line-of-fire
//! check that keeps friendlies out of the beam, and the trigger itself.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::maneuver::ai_target_anchor;
#[cfg(test)]
use super::torpedo::update_torpedo_section_input;
use crate::{input::point_defense::mount_may_shoot, prelude::*};

/// Fraction of a turret's maximum bullet travel (muzzle_speed * lifetime)
/// inside which the AI considers a shot worth taking: a margin below 1.0 so
/// bullets arrive with the target still catchable, not at their despawn
/// range.
///
/// # The engagement-range chain - every constant below moves TOGETHER
///
/// A turret has no `range` field. Its reach is
/// `muzzle_speed * projectile_lifetime`, and this factor turns that reach
/// into the gate below. Muzzle speed is pinned by
/// [`REFERENCE_CLOSING_SPEED`](nova_gameplay::prelude::REFERENCE_CLOSING_SPEED)
/// (both damage curves read 1.0 at the muzzle speed a creator authors as
/// 1 000 m/s), so LIFETIME is the only knob that moves reach, and everything
/// here is tuned against the gate it produces. The reach source is AUTHORED
/// and quoted in meters; the reach and the gate are compared against avian
/// positions, so they are world units, one of which is 10 m:
///
/// | reach source | reach | fire gate (x0.9) |
/// |---|---|---|
/// | every shipped PDC, 1 000 m/s x 2.0 s | 200 u (2.0 km) | 180 u (1.8 km) |
///
/// The shipped catalog is one gun in four dressings, so 180 u is the weakest
/// gate as well as the only one. A mod authoring a slower round or a shorter
/// lifetime moves its own gate down under every constant below.
///
/// Change a lifetime and these must be re-derived in the SAME commit:
///
/// - `AI_STANDOFF_RANGE` + `AI_STANDOFF_BAND` (`maneuver.rs`) - where a fight
///   actually settles. `standoff + band` (125 u) must sit well inside the
///   WEAKEST shipped gun's gate, or AI ships orbit outside their own reach
///   and never fire. There is no error when that happens; the guns simply
///   stay quiet.
/// - `AI_POINT_DEFENSE_RANGE` (`acquisition.rs`) - at or under the gate, or
///   the opening rounds of every intercept are wasted.
/// - `AI_ENGAGE_RANGE` (`behavior.rs`) - above the gate; the difference is
///   how long a committed ship flies before it can shoot.
/// - `AI_THREAT_AIM_RANGE` (`threat.rs`) - tracks the gate: a nose held on
///   me from beyond weapon reach is not a threat yet.
/// - `EFFECTIVE_RANGE_MARGIN` (`nova_authoring::balance`) - mirrors THIS
///   constant to derive each ship's audited threat envelope. A lifetime
///   change is a balance-audit change; re-run `balance_audit_gate`.
///
/// The factor stays at 0.9 rather than tightening to the ~0.85 that would be
/// strictly safe against a target fleeing at the player's authored 150 m/s
/// speed cap.
/// The gate is computed in the SHOOTER's frame while true reach is
/// `closing_speed * lifetime`, so it over-reads against a runner and
/// under-reads against a charger. That error scales with reach, and the
/// lifetime cut shrank it 2.5x on its own (750 m of overshoot at the old
/// 5.0 s, 300 m now); buying the rest would cost a quarter of the envelope in
/// the head-on case a fight is actually decided in. The gate itself and every
/// constant listed above stay engine-side, in world units.
pub const AI_FIRE_RANGE_FACTOR: f32 = 0.9;
/// Burst cadence (s): guns fire for the window, then hold, cyclically.
pub(super) const AI_BURST_FIRE_SECS: f32 = 1.5;
const AI_BURST_HOLD_SECS: f32 = 0.8;

/// What ONE turret has its guns on this frame, and whether that is a
/// point-defense engagement.
///
/// The turret's own [`TurretDefenseTarget`] decides first, because it is the
/// only reading of point defense that knows what THIS mount can reach. A
/// turret carrying that component with `None` has been told there is nothing in
/// its arc, and drops back to the ship's primary target - it must NOT fall back
/// on the ship-wide [`AIPointDefenseTarget`], because that fallback is exactly
/// the dogpile the per-turret pass exists to break, and it is what leaves a
/// mount slewed onto something under its own hull.
///
/// The ship-wide pick is the fallback only for a turret with no assignment at
/// all: its first frame, a player-flown hull, or a rig that runs the gun
/// systems without the assignment pass.
///
/// Point defense applies in EVERY behavior state - a patrolling or idle ship
/// still defends itself - while the engaging states otherwise track the primary
/// target and non-engaging states clear the aim so turrets slew back to rest.
fn ai_turret_gun_target(
    turret_defense: Option<&TurretDefenseTarget>,
    ship_defense: &AIPointDefenseTarget,
    state: &AIBehaviorState,
    target: &AITarget,
) -> (Option<Entity>, bool) {
    let defending = match turret_defense {
        Some(assignment) => **assignment,
        None => **ship_defense,
    };
    match defending {
        Some(torpedo) => (Some(torpedo), true),
        None => (if state.engages() { **target } else { None }, false),
    }
}

pub(super) fn update_turret_target_input(
    mut q_turret: Query<
        (
            &mut TurretSectionTargetInput,
            &mut TurretSectionTargetVelocity,
            &mut TurretSectionTargetEntity,
            Option<&TurretDefenseTarget>,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_spaceship: Query<
        (&AIBehaviorState, &AITarget, &AIPointDefenseTarget),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    q_target_velocity: Query<&LinearVelocity>,
) {
    // Iterated turret-first, not ship-first: every turret now resolves its own
    // gun target, so there is nothing left to hoist out of the inner loop.
    for (
        mut turret_input,
        mut turret_velocity,
        mut turret_tracked,
        turret_defense,
        ChildOf(ship),
    ) in &mut q_turret
    {
        let Ok((state, target, ship_defense)) = q_spaceship.get(*ship) else {
            continue;
        };
        let (gun_target, _) = ai_turret_gun_target(turret_defense, ship_defense, state, target);
        // Aim at the live structure: fire converging on the root origin lands
        // in empty space once the front sections die.
        let aim = ai_target_anchor(gun_target, &q_target);
        // Feed the target root's velocity alongside the position so
        // lead_intercept_point computes a real lead for AI turrets - the AI-
        // side sibling of the player lock feed. The solve is shooter-frame-
        // correct on its own.
        let velocity = aim
            .and_then(|_| gun_target.and_then(|entity| q_target_velocity.get(entity).ok()))
            .map(|velocity| **velocity)
            .unwrap_or(Vec3::ZERO);
        **turret_input = aim;
        **turret_velocity = velocity;
        // The body the velocity belongs to, so the mount's velocity track
        // restarts when the guns swing to a different target rather than
        // leading the new one along the old one's course.
        **turret_tracked = aim.and(gun_target);
    }
}

/// The free-running burst cycle of an AI ship's guns: fire for
/// `AI_BURST_FIRE_SECS`, hold for `AI_BURST_HOLD_SECS`, repeat. A ship
/// fires only while the window is open (and every other gate passes), so AI
/// fire reads as deliberate bursts instead of a continuous hose. Required by
/// [`AISpaceshipMarker`]; ticked by `update_fire_cadence`.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIFireCadence {
    /// Time left in the current phase.
    timer: Cooldown,
    /// Whether the current phase is a fire window (else a hold).
    pub(crate) firing: bool,
}

impl Default for AIFireCadence {
    fn default() -> Self {
        // Starts in a fire window so an AI ship dropped into a fight shoots
        // immediately, matching pre-cadence behavior at spawn.
        Self {
            timer: Cooldown::started(AI_BURST_FIRE_SECS),
            firing: true,
        }
    }
}

impl AIFireCadence {
    /// Advance the cycle, flipping between fire and hold phases.
    pub(crate) fn tick(&mut self, delta: f32) {
        self.timer.tick(delta);
        if self.timer.ready() {
            self.firing = !self.firing;
            let phase = if self.firing {
                AI_BURST_FIRE_SECS
            } else {
                AI_BURST_HOLD_SECS
            };
            // trigger_for, not trigger: the two phases have different lengths,
            // so the wait is set per phase rather than from one duration.
            self.timer.trigger_for(phase);
        }
    }
}

/// Tick every AI ship's burst cycle. Free-running (it does not reset on
/// state or target changes): the phase offset between ships also staggers
/// their volleys for free.
pub(super) fn update_fire_cadence(
    time: Res<Time>,
    mut q_spaceship: Query<&mut AIFireCadence, With<AISpaceshipMarker>>,
) {
    for mut cadence in &mut q_spaceship {
        cadence.tick(time.delta_secs());
    }
}

/// Whether the straight line from `origin` to `aim` is blocked by a tangible
/// collider belonging to neither the shooter nor the target.
///
/// The ray IS the bullet's path: turrets steer to the LEADED aim point, so
/// occlusion is judged against that line, not the target's current position.
/// A hit that resolves to the TARGET's own body (a slow or near target whose
/// collider straddles the line before the lead point) is not occlusion.
/// Sensor colliders are transparent to the ray for the same reason they are
/// transparent to rounds (`resolve_bullet_hit` skips sensors): a beacon's
/// trigger sphere or a blast shell must not read as cover. The shooter's own
/// colliders are transparent too - the muzzle sits on its hull.
///
/// An unattributable tangible hit (no [`ColliderOf`]) counts as blocked:
/// failing closed holds one burst, failing open shoots through a wall.
///
/// The collider trees are maintained by the physics schedule, so this
/// Update-schedule reader sees poses at most one physics tick stale; for a
/// fire gate the cost is a one-frame-late hold or release at a cover edge,
/// never a wrong sustained decision.
pub(super) fn ai_line_of_fire_blocked(
    spatial: &SpatialQuery,
    q_sensor: &Query<(), With<Sensor>>,
    q_collider_of: &Query<&ColliderOf>,
    shooter: Entity,
    target: Entity,
    origin: Vec3,
    aim: Vec3,
) -> bool {
    let to_aim = aim - origin;
    let Ok(direction) = Dir3::new(to_aim) else {
        // Degenerate line (aim on the muzzle): nothing to occlude.
        return false;
    };
    let body_of = |collider: Entity| q_collider_of.get(collider).map(|of| of.body);
    spatial
        .cast_ray_predicate(
            origin,
            direction,
            to_aim.length(),
            true,
            &SpatialQueryFilter::default(),
            // Predicate false = the collider is transparent to the ray.
            &|collider| !q_sensor.contains(collider) && body_of(collider) != Ok(shooter),
        )
        .is_some_and(|hit| body_of(hit.entity) != Ok(target))
}

pub(super) fn on_projectile_input(
    mut q_turret: Query<
        (
            &TurretSectionMuzzleEntity,
            &TurretSectionAimPoint,
            &TurretEngineFigures,
            &mut TurretSectionInput,
            Option<&TurretDefenseTarget>,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_spaceship: Query<
        (
            &AIBehaviorState,
            &AITarget,
            &AIPointDefenseTarget,
            &AIFireCadence,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    spatial: SpatialQuery,
    q_sensor: Query<(), With<Sensor>>,
    q_collider_of: Query<&ColliderOf>,
) {
    // Turret-first, like the aim system: the gun target, and therefore whether
    // the burst cadence and the line-of-fire gate apply at all, is a per-MOUNT
    // answer. One turret on a hull can be defending while its neighbour keeps
    // working the primary target.
    for (muzzle, aim_point, figures, mut input, turret_defense, ChildOf(ship)) in &mut q_turret {
        let Ok((state, target, ship_defense, cadence)) = q_spaceship.get(*ship) else {
            continue;
        };
        // While defending, the burst cadence is BYPASSED - point defense fires
        // continuously; bursts are a discipline for shooting at ships, not at
        // inbound ordnance.
        let (gun_target, defending) =
            ai_turret_gun_target(turret_defense, ship_defense, state, target);
        let target_anchor = ai_target_anchor(gun_target, &q_target);
        let firing_allowed = defending || (state.engages() && cadence.firing);
        // Hold fire with no gun target or outside the burst window -
        // written as an explicit false so a firing turret stops.
        let (Some(target_anchor), true) = (target_anchor, firing_allowed) else {
            **input = false;
            continue;
        };

        let Ok(muzzle_transform) = q_muzzle.get(**muzzle) else {
            error!(
                "on_projectile_input: muzzle entity {:?} not found in q_muzzle",
                **muzzle
            );
            continue;
        };

        // The RANGE gate (past the distance its bullets can actually live, a
        // shot is noise, not pressure) and the BEARING gate, both out of
        // `mount_may_shoot` - the one place the two triggers on this path
        // share, so the player's Flight Computer cannot drift onto a looser
        // number of its own.
        //
        // Alignment is judged against the LEADED aim point the turret actually
        // steers to, falling back to the anchor before the lead resolves: a
        // turret correctly leading a crossing target never aligns with the raw
        // anchor, and would otherwise hold fire forever.
        //
        // The SECTION fire path applies `muzzle_on_target` too, per muzzle and
        // on the raw physics pose, so this is not the enforcement - it is what
        // keeps an AI trigger meaning "this mount is shooting" for the readers
        // of `TurretSectionInput`.
        let muzzle_position = muzzle_transform.translation();
        let aim = aim_point.unwrap_or(target_anchor);
        if !mount_may_shoot(muzzle_transform, figures, target_anchor, aim) {
            **input = false;
            continue;
        }

        // Line-of-fire gate, last so only a shot that would otherwise
        // fire pays for the ray. Point defense is exempt: inbound
        // ordnance is hunting THIS ship, so its line is short, closing,
        // and the one case where a wasted round beats a held trigger.
        // `target_anchor` came through `ai_target_anchor`, so gun_target is
        // Some here; the else arm is unreachable belt-and-braces.
        let Some(gun_target) = gun_target else {
            **input = false;
            continue;
        };
        **input = defending
            || !ai_line_of_fire_blocked(
                &spatial,
                &q_sensor,
                &q_collider_of,
                *ship,
                gun_target,
                muzzle_position,
                aim,
            );
    }
}

#[cfg(test)]
mod fire_discipline_tests {
    use avian3d::collider_tree::ColliderTrees;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// An AI ship engaged on a hand-set target, with one turret whose
    /// muzzle sits at the origin facing -Z. Returns (world, turret, muzzle).
    fn firing_world(target_position: Vec3, target_velocity: Vec3) -> (World, Entity, Entity) {
        let mut world = World::new();
        // Empty collider trees for the fire gate's SpatialQuery: no
        // colliders means no occluders. Occlusion itself is covered by the
        // physics-app tests in `line_of_fire_tests`.
        world.init_resource::<ColliderTrees>();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(target_position),
                LinearVelocity(target_velocity),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let muzzle = world
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(false),
                TurretSectionMuzzleEntity(muzzle),
                ChildOf(ship),
            ))
            .id();
        (world, turret, muzzle)
    }

    #[test]
    fn the_turret_feed_carries_the_target_velocity() {
        // The AI-side sibling of the player lock feed: without it,
        // lead_intercept_point degenerates to aim-at-the-target and AI
        // turrets shoot behind every mover.
        let velocity = Vec3::new(30.0, 0.0, 0.0);
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -100.0), velocity);

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetVelocity>()
                .unwrap(),
            velocity
        );
    }

    #[test]
    fn fire_aligns_with_the_leaded_aim_point_not_the_anchor() {
        // A crossing target: the aim point leads it well off the raw
        // anchor bearing. The muzzle (facing -Z) is ON the lead point and
        // ~22 degrees OFF the anchor, far outside the on-target cone -
        // discipline must fire anyway, because the turret is exactly where
        // the lead solution wants it.
        let (mut world, turret, _) = firing_world(Vec3::new(40.0, 0.0, -100.0), Vec3::ZERO);
        **world
            .entity_mut(turret)
            .get_mut::<TurretSectionAimPoint>()
            .unwrap() = Some(Vec3::new(0.0, 0.0, -100.0));

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "aligned with the LEADED point: fire, even though the raw \
             anchor is off-axis"
        );
    }

    #[test]
    fn no_fire_beyond_the_effective_range() {
        // Dead ahead and perfectly aligned, but past muzzle_speed *
        // lifetime * margin (the authored 1 000 m/s is 100 u/s, so
        // 100 * 2 * 0.9 = 180 u): the bullet
        // dies in flight, so discipline holds.
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -200.0), Vec3::ZERO);

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "beyond effective range: hold fire"
        );

        // The same shot inside the envelope fires.
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -160.0), Vec3::ZERO);
        world.run_system_once(on_projectile_input).unwrap();
        assert!(
            **world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "inside effective range and aligned: fire"
        );
    }

    #[test]
    fn the_default_turret_reaches_past_the_standoff_band() {
        // The silent failure this range pass exists to prevent: a ship
        // orbiting where its own rounds expire holds fire forever, with no
        // error to notice. Graded at the band's OUTER edge, where the orbit
        // takes a ship furthest from its target. The AUTHORED catalog is
        // graded the same way, one grade at a time, in nova_authoring's
        // base_content tests - this pins the engine default.
        let config = TurretSectionConfig::default();
        let gate = config
            .muzzle_speed
            .over(config.projectile_lifetime)
            .to_engine()
            * AI_FIRE_RANGE_FACTOR;
        assert!(
            gate > AI_STANDOFF_OUTER_EDGE,
            "default turret fire gate {gate:.0}u must cover the standoff \
             band edge at {:.0}u, or a ship flying the envelope never shoots",
            AI_STANDOFF_OUTER_EDGE
        );
    }

    #[test]
    fn the_burst_cadence_alternates_fire_and_hold() {
        let mut cadence = AIFireCadence::default();
        assert!(cadence.firing, "spawns in a fire window");

        // Tick past the fire window: the hold begins.
        cadence.tick(AI_BURST_FIRE_SECS + 0.01);
        assert!(!cadence.firing, "fire window over: hold");

        // Tick past the hold: firing resumes.
        cadence.tick(AI_BURST_HOLD_SECS + 0.01);
        assert!(cadence.firing, "hold over: next burst");
    }

    #[test]
    fn a_closed_burst_window_holds_fire_even_when_aligned() {
        let (mut world, turret, _) = firing_world(Vec3::new(0.0, 0.0, -100.0), Vec3::ZERO);
        // Force the ship's cadence into a hold phase.
        let ship = world
            .query_filtered::<Entity, With<AISpaceshipMarker>>()
            .iter(&world)
            .next()
            .unwrap();
        let mut cadence = world.entity_mut(ship);
        let mut cadence = cadence.get_mut::<AIFireCadence>().unwrap();
        cadence.tick(AI_BURST_FIRE_SECS + 0.01);
        assert!(!cadence.firing);

        world.run_system_once(on_projectile_input).unwrap();

        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "hold phase: no fire, alignment notwithstanding"
        );
    }
}

#[cfg(test)]
mod line_of_fire_tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_gameplay::test_support::{integrity_physics_app, settle};

    use super::*;

    /// A physics app (real avian collider trees, zero gravity, manual
    /// clock) with an engaged AI ship at the origin whose muzzle faces -Z,
    /// and a hostile target body dead ahead. The gate reads the SAME
    /// spatial state production reads; the bare-World rigs in
    /// `fire_discipline_tests` cover the other gates with empty trees.
    fn los_app(target_position: Vec3, target_radius: f32) -> (App, Entity, Entity, Entity) {
        let mut app = integrity_physics_app();
        let target = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Collider::sphere(target_radius),
                Transform::from_translation(target_position),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                AISpaceshipMarker,
                AITarget(Some(target)),
                RigidBody::Dynamic,
                Collider::sphere(1.0),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let muzzle = app
            .world_mut()
            .spawn((TurretSectionBarrelMuzzleMarker, GlobalTransform::IDENTITY))
            .id();
        let turret = app
            .world_mut()
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                TurretSectionAimPoint(None),
                TurretSectionConfigHelper(TurretSectionConfig::default()),
                TurretSectionInput(false),
                TurretSectionMuzzleEntity(muzzle),
                ChildOf(ship),
            ))
            .id();
        settle(&mut app);
        (app, ship, turret, target)
    }

    /// A tangible static rock: the production shape of authored cover
    /// (an invulnerable asteroid is a Static body with a plain collider).
    fn spawn_rock(app: &mut App, position: Vec3, radius: f32) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Static,
                Collider::sphere(radius),
                Transform::from_translation(position),
            ))
            .id()
    }

    fn fire_decision(app: &mut App, turret: Entity) -> bool {
        app.world_mut()
            .run_system_once(on_projectile_input)
            .unwrap();
        **app
            .world()
            .entity(turret)
            .get::<TurretSectionInput>()
            .unwrap()
    }

    #[test]
    fn cover_between_muzzle_and_target_holds_fire() {
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -50.0), 5.0);
        settle(&mut app);

        assert!(
            !fire_decision(&mut app, turret),
            "a tangible rock on the line of fire must hold the trigger"
        );

        // Delivery guard, same test: with the rock gone the identical shot
        // fires - so the hold above was the rock, not a broken rig.
        app.world_mut().despawn(rock);
        settle(&mut app);
        assert!(
            fire_decision(&mut app, turret),
            "same geometry without the rock must fire"
        );
    }

    #[test]
    fn sensor_volumes_are_not_cover() {
        // A scenario trigger area / blast shell: rounds fly through
        // sensors (resolve_bullet_hit skips them), so the gate must
        // not read one as cover - the R1.1 beacon lesson, ray edition.
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        app.world_mut().spawn((
            RigidBody::Static,
            Collider::sphere(20.0),
            Sensor,
            Transform::from_translation(Vec3::new(0.0, 0.0, -50.0)),
        ));
        settle(&mut app);

        assert!(
            fire_decision(&mut app, turret),
            "a sensor volume on the line is transparent: fire"
        );
    }

    #[test]
    fn the_targets_own_hull_is_not_cover() {
        // A big, close target: the ray meets the target's own collider
        // well before the aim anchor. That is the shot LANDING, not
        // occlusion.
        let (mut app, _, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 30.0);

        assert!(
            fire_decision(&mut app, turret),
            "hitting the target's own collider first is a hit, not cover"
        );
    }

    #[test]
    fn point_defense_fires_through_cover() {
        // Inbound ordnance behind a rock: PD is exempt from the gate -
        // ordnance hunting THIS ship closes the line itself, and a held
        // trigger is worse than a wasted round.
        let (mut app, ship, turret, _) = los_app(Vec3::new(0.0, 0.0, -100.0), 1.0);
        let torpedo = app
            .world_mut()
            .spawn((Transform::from_translation(Vec3::new(0.0, 0.0, -100.0)),))
            .id();
        app.world_mut()
            .entity_mut(ship)
            .insert(AIPointDefenseTarget(Some(torpedo)));
        spawn_rock(&mut app, Vec3::new(0.0, 0.0, -50.0), 5.0);
        settle(&mut app);

        assert!(
            fire_decision(&mut app, turret),
            "point defense bypasses the line-of-fire gate"
        );
    }

    #[test]
    fn cover_holds_the_torpedo_launch() {
        // Same geometry as the trigger tests, torpedo side: in-envelope
        // target (default blast 30 -> min range 90; 300 m dead ahead),
        // rock on the bearing - the bay must not spend ordnance on cover.
        let (mut app, ship, _, _) = los_app(Vec3::new(0.0, 0.0, -300.0), 1.0);
        let bay = app
            .world_mut()
            .spawn((
                torpedo_section(TorpedoSectionConfig::default()),
                AITorpedoBay::default(),
                ChildOf(ship),
            ))
            .id();
        let rock = spawn_rock(&mut app, Vec3::new(0.0, 0.0, -150.0), 10.0);
        settle(&mut app);

        app.world_mut()
            .run_system_once(update_torpedo_section_input)
            .unwrap();
        assert!(
            !**app
                .world()
                .entity(bay)
                .get::<TorpedoSectionInput>()
                .unwrap(),
            "rock on the launch bearing: hold the torpedo"
        );

        // Delivery guard, same test: the identical launch with the rock
        // gone goes out (the cadence only resets on an actual launch, so
        // it is still elapsed here).
        app.world_mut().despawn(rock);
        settle(&mut app);
        app.world_mut()
            .run_system_once(update_torpedo_section_input)
            .unwrap();
        assert!(
            **app
                .world()
                .entity(bay)
                .get::<TorpedoSectionInput>()
                .unwrap(),
            "same envelope without the rock: launch"
        );
    }
}

/// The per-turret point-defense assignment has to reach the GUNS, not just sit
/// on the turret: these drive the real aim system and read back where each
/// mount is pointed.
#[cfg(test)]
mod per_turret_defense_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::input::point_defense::update_turret_point_defense;

    /// An engaged AI ship with `count` turrets and a hostile ship to fight.
    fn engaged_ship(world: &mut World, count: usize) -> (Entity, Vec<Entity>) {
        let enemy = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -300.0)),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                SpaceshipRootMarker,
                Allegiance::Enemy,
                AITarget(Some(enemy)),
                AIBehaviorState::Engage,
                AIPointDefenseTarget::default(),
                Transform::default(),
            ))
            .id();
        let arc = TurretSectionArc::from_tree(&TurretSectionConfig::default().root).unwrap();
        let turrets = (0..count)
            .map(|_| {
                world
                    .spawn((
                        TurretSectionMarker,
                        arc,
                        GlobalTransform::IDENTITY,
                        TurretSectionTargetInput(None),
                        TurretSectionTargetVelocity(Vec3::ZERO),
                        TurretDefenseTarget::default(),
                        ChildOf(ship),
                    ))
                    .id()
            })
            .collect();
        (ship, turrets)
    }

    fn torpedo(world: &mut World, ship: Entity, position: Vec3) -> Entity {
        world
            .spawn((
                TorpedoProjectileMarker,
                TorpedoTargetChosen,
                TorpedoTargetEntity(ship),
                Allegiance::Player,
                Transform::from_translation(position),
                LinearVelocity((Vec3::ZERO - position).normalize_or_zero() * 30.0),
            ))
            .id()
    }

    fn aim(world: &World, turret: Entity) -> Option<Vec3> {
        **world.get::<TurretSectionTargetInput>(turret).unwrap()
    }

    #[test]
    fn two_turrets_aim_at_two_different_torpedoes() {
        // End to end: assignment -> aim. Before the per-turret pass BOTH
        // barrels were fed the ship-wide pick, so the second torpedo flew in
        // with nothing pointed at it.
        let mut world = World::new();
        let (ship, turrets) = engaged_ship(&mut world, 2);
        let near = Vec3::new(0.0, 10.0, -60.0);
        let far = Vec3::new(0.0, 10.0, -120.0);
        torpedo(&mut world, ship, near);
        torpedo(&mut world, ship, far);

        world.run_system_once(update_turret_point_defense).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        let aims: Vec<Option<Vec3>> = turrets.iter().map(|&t| aim(&world, t)).collect();
        assert!(
            aims.contains(&Some(near)) && aims.contains(&Some(far)),
            "each barrel must be on its own torpedo, got {aims:?}"
        );
    }

    #[test]
    fn a_turret_with_no_reachable_torpedo_keeps_working_the_primary_target() {
        // The anti-idleness rule, at the gun. A mount that cannot depress onto
        // the inbound must not be parked on it - it goes back to the ship the
        // hull is fighting, which is the only useful thing left to shoot.
        let mut world = World::new();
        let (ship, turrets) = engaged_ship(&mut world, 1);
        torpedo(&mut world, ship, Vec3::new(0.0, -60.0, -30.0));

        world.run_system_once(update_turret_point_defense).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            aim(&world, turrets[0]),
            Some(Vec3::new(0.0, 0.0, -300.0)),
            "an idle mount returns to the primary target, not to a torpedo \
             under its own hull"
        );
    }

    #[test]
    fn a_turret_with_no_assignment_component_still_uses_the_ship_wide_pick() {
        // The fallback path: a mount that the assignment pass has never seen
        // (its first frame, a rig without the pass) behaves exactly as it did
        // before per-turret assignment existed.
        let mut world = World::new();
        let (ship, _) = engaged_ship(&mut world, 0);
        let inbound = torpedo(&mut world, ship, Vec3::new(0.0, 10.0, -60.0));
        world.get_mut::<AIPointDefenseTarget>(ship).unwrap().0 = Some(inbound);
        let turret = world
            .spawn((
                TurretSectionMarker,
                GlobalTransform::IDENTITY,
                TurretSectionTargetInput(None),
                TurretSectionTargetVelocity(Vec3::ZERO),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(aim(&world, turret), Some(Vec3::new(0.0, 10.0, -60.0)));
    }
}
