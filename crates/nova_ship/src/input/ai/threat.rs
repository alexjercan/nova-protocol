//! Under-fire memory and the evade clocks: who recently hit this ship
//! ([`AIThreat`]) and where the evade cycle is ([`AIEvade`]). The behavior
//! state machine reads both to decide when to jink.

#[cfg(test)]
use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::acquisition::update_ai_target;
#[cfg(test)]
use super::behavior::update_behavior_state;
#[cfg(test)]
use super::maneuver::{ai_evade_direction, on_thruster_input};
use crate::prelude::*;

/// How long (s) a hostile hit stays "recent" for the threat model: within
/// this window the ship counts as under fire and the attacker biases target
/// selection.
const AI_THREAT_DAMAGE_MEMORY_SECS: f32 = 3.0;
/// Range (u) inside which a hostile holding its nose on me counts as a
/// threat even before a shot lands. Kept just past the guns' fire gate
/// (180 u): a nose on me further out cannot hurt me yet. Tracks the gate, so
/// it moves with any `projectile_lifetime` change - see AI_FIRE_RANGE_FACTOR
/// in `guns.rs`.
pub(super) const AI_THREAT_AIM_RANGE: f32 = 200.0;
/// Aim cone (cos) for the aiming-at-me signal: the hostile's hull forward
/// against the bearing to my anchor. A cheap proxy - turrets can aim off
/// the hull axis - accepted per the spike; true incoming-projectile
/// detection is the follow-up if evasion feels blind.
pub(super) const AI_THREAT_AIM_COS: f32 = 0.95;
/// How long (s) one evade cycle lasts before decaying back to Engage.
/// Three jink legs at [`AI_JINK_INTERVAL_SECS`]. Playtest note: Evade has
/// no speed budget (the jink bypasses the standoff envelope's brake
/// regime), so back-to-back cycles can build speed that Engage re-entry
/// then brakes off; if evasion reads as careening, cap the cycle count or
/// shorten this.
const AI_EVADE_SECS: f32 = 3.6;
/// Refractory period (s) after an evade cycle before a threat can trigger
/// the next one. Without it a hostile that keeps its nose on the ship would
/// re-trigger Evade every frame and the standoff orbit would never be seen;
/// with it a fight reads as jink bursts with engage windows between them.
const AI_EVADE_COOLDOWN_SECS: f32 = 1.5;
/// Length (s) of one jink leg: long enough for the hull to swing onto the
/// leg's heading (torque-budget slew) and burn, short enough to read as
/// jinking. Playtest knob, paired with AI_EVADE_SECS.
const AI_JINK_INTERVAL_SECS: f32 = 1.2;
/// Thrust gate (dot) while evading: looser than [`AI_THRUST_ALIGNMENT`] so
/// lateral bursts fire while the hull is still swinging onto the jink leg -
/// waiting for a tight alignment would spend most of each leg coasting.
pub(super) const AI_EVADE_THRUST_ALIGNMENT: f32 = 0.75;
/// Distance discount for the ship that recently damaged me: whoever is
/// shooting me steals the pick from comparably distant hostiles.
pub(super) const AI_THREAT_ATTACKER_DISCOUNT: f32 = 0.5;

/// The ship's under-fire memory: how recently a hostile hit landed and who
/// fired it. Written by `on_damage_track_threat`, ticked by
/// `update_behavior_state`; drives the Engage -> Evade transition and the
/// attacker bias in `pick_ai_target`. Required by [`AISpaceshipMarker`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIThreat {
    /// Time left in the recent-damage memory; ready = not under fire.
    pub(crate) damage_memory: Cooldown,
    /// The ship root behind the remembered damage (a hit's source resolved
    /// through [`ProjectileOwner`]). May be despawned by read time; the
    /// picker simply no longer finds it, while the memory still evades.
    pub(crate) attacker: Option<Entity>,
}

impl Default for AIThreat {
    fn default() -> Self {
        Self {
            // A fresh Cooldown is ready: a freshly spawned ship has not been shot yet.
            damage_memory: Cooldown::new(AI_THREAT_DAMAGE_MEMORY_SECS),
            attacker: None,
        }
    }
}

impl AIThreat {
    /// Remember a hostile hit: restart the memory window and note the
    /// attacker. An unattributed hit (no resolvable owner) keeps the
    /// previous attacker - the shooter most likely has not changed.
    pub(crate) fn record(&mut self, attacker: Option<Entity>) {
        self.damage_memory.trigger();
        self.attacker = attacker.or(self.attacker);
    }

    /// Whether a hostile hit landed within the memory window.
    pub(crate) fn recently_damaged(&self) -> bool {
        !self.damage_memory.ready()
    }

    /// The remembered attacker, while the memory window is open.
    pub(crate) fn recent_attacker(&self) -> Option<Entity> {
        self.recently_damaged().then_some(self.attacker).flatten()
    }
}

/// The evade cycle's clocks: how long the current cycle has left, the
/// refractory period before the next one, and the jink-leg cadence within a
/// cycle. Managed by `update_behavior_state`; the rotation and thrust systems
/// read `Self::leg` to fly the current jink. Required by
/// [`AISpaceshipMarker`].
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIEvade {
    /// Time left in the current evade cycle. Triggered on entering Evade;
    /// ticks only while evading; expiry decays the state back to Engage.
    pub(crate) duration: Cooldown,
    /// Refractory period after an evade cycle. Triggered on leaving Evade;
    /// starts ready so a fresh ship's first threat evades immediately.
    pub(crate) cooldown: Cooldown,
    /// Cadence of the jink pattern: each completion turns onto the next leg.
    pub(crate) jink: Timer,
    /// The jink pattern leg currently being flown (see
    /// [`ai_evade_direction`]). Advances monotonically, wrapping.
    pub(crate) leg: u32,
}

impl Default for AIEvade {
    fn default() -> Self {
        Self {
            duration: Cooldown::started(AI_EVADE_SECS),
            // A fresh Cooldown is ready, so the first threat evades immediately.
            cooldown: Cooldown::new(AI_EVADE_COOLDOWN_SECS),
            // Repeating, and therefore NOT a Cooldown: the jink cadence rolls
            // over on its own to turn onto the next leg.
            jink: Timer::from_seconds(AI_JINK_INTERVAL_SECS, TimerMode::Repeating),
            leg: 0,
        }
    }
}

/// Resolve a damage source (the hitting collider the impact path puts in
/// `HealthApplyDamage.source`) to the attacking ship root and the
/// allegiance governing the hit, walking `ChildOf` ancestors: a turret
/// bullet carries [`ProjectileOwner`] on the collider itself, a torpedo
/// warhead on its projectile root, a detonation blast on the blast entity.
/// With no owner anywhere (a ram), the attacker is the source's own ship
/// root, if it has one.
fn resolve_damage_attacker(
    source: Entity,
    q_owner: &Query<&ProjectileOwner>,
    q_allegiance: &Query<&Allegiance>,
    q_parent: &Query<&ChildOf>,
    q_ship_root: &Query<(), With<SpaceshipRootMarker>>,
) -> (Option<Entity>, Option<Allegiance>) {
    let mut allegiance = None;
    let mut entity = source;
    loop {
        if let Ok(&ProjectileOwner(owner)) = q_owner.get(entity) {
            // The projectile copies the shooter's allegiance at launch, so
            // the hit stays classifiable even if the owner died mid-flight.
            let allegiance = allegiance.or_else(|| q_allegiance.get(entity).ok().copied());
            return (Some(owner), allegiance);
        }
        if allegiance.is_none() {
            allegiance = q_allegiance.get(entity).ok().copied();
        }
        let Ok(&ChildOf(parent)) = q_parent.get(entity) else {
            // Topmost ancestor, no owner anywhere: a direct body-to-body
            // hit. A ship root is its own attacker; anything else
            // (asteroid, debris) has nobody to blame.
            let attacker = q_ship_root.get(entity).is_ok().then_some(entity);
            return (attacker, allegiance);
        };
        entity = parent;
    }
}

/// Record hostile hits into the damaged ship's [`AIThreat`].
///
/// `HealthApplyDamage` propagates from the hit section up through `ChildOf`
/// to the ship root, so this fires once the event reaches an entity
/// carrying `AIThreat` - the AI root. Only hits whose resolved allegiance is
/// hostile count: the ship's own torpedo blast catching it (blast damage
/// deliberately affects the owner) must not spook it into evading itself.
pub(super) fn on_damage_track_threat(
    damage: On<HealthApplyDamage>,
    mut q_ship: Query<(&Allegiance, &mut AIThreat), With<AISpaceshipMarker>>,
    q_owner: Query<&ProjectileOwner>,
    q_allegiance: Query<&Allegiance>,
    q_parent: Query<&ChildOf>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
) {
    // A zero amount is a hit on a corpse (`on_damage` zeroes absorbed damage), not
    // fire worth reacting to.
    if damage.amount <= 0.0 {
        return;
    }
    let Ok((own_allegiance, mut threat)) = q_ship.get_mut(damage.entity) else {
        return;
    };
    let Some(source) = damage.source else {
        return;
    };
    let (attacker, attacker_allegiance) =
        resolve_damage_attacker(source, &q_owner, &q_allegiance, &q_parent, &q_ship_root);
    if relation(Some(own_allegiance), attacker_allegiance.as_ref()) != Relation::Hostile {
        return;
    }
    threat.record(attacker);
}

#[cfg(test)]
mod threat_tests {
    // Damage attribution into the threat memory: the weapon populates
    // HealthApplyDamage.source with the hitting collider, and the observer
    // resolves it to the firing ship root through ProjectileOwner.
    use super::*;

    fn threat_world() -> (World, Entity) {
        let mut world = World::new();
        world.add_observer(on_damage_track_threat);
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        (world, ship)
    }

    fn hit(world: &mut World, ship: Entity, source: Entity) {
        world.trigger(HealthApplyDamage {
            entity: ship,
            source: Some(source),
            amount: 10.0,
        });
    }

    fn recent_attacker(world: &World, ship: Entity) -> Option<Entity> {
        world
            .entity(ship)
            .get::<AIThreat>()
            .unwrap()
            .recent_attacker()
    }

    #[test]
    fn a_bullet_hit_records_its_owner_as_the_attacker() {
        // Turret bullet: root and collider are one entity, the owner sits
        // right on the source.
        let (mut world, ship) = threat_world();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let bullet = world
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();

        hit(&mut world, ship, bullet);

        assert_eq!(recent_attacker(&world, ship), Some(player));
    }

    #[test]
    fn a_warhead_section_resolves_through_its_projectile_root() {
        // Torpedo contact damage: the source is the warhead child section;
        // the owner (and copied allegiance) live on the projectile root.
        let (mut world, ship) = threat_world();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let torpedo = world
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(player),
                Allegiance::Player,
            ))
            .id();
        let warhead = world.spawn(ChildOf(torpedo)).id();

        hit(&mut world, ship, warhead);

        assert_eq!(recent_attacker(&world, ship), Some(player));
    }

    #[test]
    fn a_hit_on_a_section_propagates_to_the_root_threat() {
        // The production path: the weapon triggers the event on the HIT SECTION and
        // it propagates through ChildOf to the root. The observer must catch
        // it at the root hop.
        let (mut world, ship) = threat_world();
        let section = world.spawn(ChildOf(ship)).id();
        let player = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let bullet = world
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();

        hit(&mut world, section, bullet);

        assert_eq!(
            recent_attacker(&world, ship),
            Some(player),
            "a section hit must reach the root's threat memory"
        );
    }

    #[test]
    fn a_self_blast_is_not_a_threat() {
        // Blast damage deliberately reaches the owner (see ProjectileHooks):
        // a ship caught in its own torpedo's blast must not spook itself.
        let (mut world, ship) = threat_world();
        let blast = world.spawn((ProjectileOwner(ship), Allegiance::Enemy)).id();

        hit(&mut world, ship, blast);

        assert_eq!(recent_attacker(&world, ship), None);
        assert!(
            !world
                .entity(ship)
                .get::<AIThreat>()
                .unwrap()
                .recently_damaged(),
            "an own-relation hit must not open the threat window"
        );
    }

    #[test]
    fn a_hostile_ram_blames_the_rammer() {
        // No ProjectileOwner anywhere up the chain: a body-to-body hit. The
        // source's own ship root is the attacker.
        let (mut world, ship) = threat_world();
        let rammer = world
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let rammer_section = world.spawn(ChildOf(rammer)).id();

        hit(&mut world, ship, rammer_section);

        assert_eq!(recent_attacker(&world, ship), Some(rammer));
    }

    #[test]
    fn the_memory_decays_and_unattributed_hits_keep_the_shooter() {
        let attacker = Entity::from_raw_u32(7).unwrap();
        let mut threat = AIThreat::default();
        assert!(!threat.recently_damaged(), "spawns with no memory");

        threat.record(Some(attacker));
        assert_eq!(threat.recent_attacker(), Some(attacker));

        // A later hit that could not be attributed keeps the known shooter:
        // the most likely source has not changed.
        threat.record(None);
        assert_eq!(threat.recent_attacker(), Some(attacker));

        threat
            .damage_memory
            .tick(AI_THREAT_DAMAGE_MEMORY_SECS + 0.01);
        assert_eq!(threat.recent_attacker(), None, "the window closed");
    }
}

#[cfg(test)]
mod evade_tests {
    // The evade cycle through the real acquisition -> transition pipeline,
    // with real time (the manual-duration harness the rotation tests use).
    use core::time::Duration;

    use bevy::{ecs::system::RunSystemOnce, time::TimeUpdateStrategy};

    use super::*;

    fn evade_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.add_observer(on_damage_track_threat);
        app.add_systems(Update, (update_ai_target, update_behavior_state).chain());

        // Inside engage range, NOT aiming at the ship (default forward -Z,
        // the ship is at -X of the player): only the damage signal fires.
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ))
            .id();
        let ship = app
            .world_mut()
            .spawn((AISpaceshipMarker, Transform::default()))
            .id();
        (app, ship, player)
    }

    fn state_of(app: &App, ship: Entity) -> AIBehaviorState {
        *app.world().get::<AIBehaviorState>(ship).unwrap()
    }

    #[test]
    fn a_hit_breaks_engage_into_evade_and_the_cycle_decays_back() {
        let (mut app, ship, player) = evade_app();

        // Settle into Engage on the acquired hostile.
        app.update();
        app.update();
        assert_eq!(state_of(&app, ship), AIBehaviorState::Engage);

        // A hostile bullet lands.
        let bullet = app
            .world_mut()
            .spawn((ProjectileOwner(player), Allegiance::Player))
            .id();
        app.world_mut().trigger(HealthApplyDamage {
            entity: ship,
            source: Some(bullet),
            amount: 10.0,
        });

        app.update();
        assert_eq!(
            state_of(&app, ship),
            AIBehaviorState::Evade,
            "getting shot breaks Engage into Evade"
        );

        // Mid-cycle the state holds...
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(state_of(&app, ship), AIBehaviorState::Evade);

        // ...and past AI_EVADE_SECS the cycle decays back to Engage (the damage
        // memory is shorter than the cycle, and the player is not aiming, so
        // nothing re-triggers).
        for _ in 0..((AI_EVADE_SECS * 60.0) as usize + 30) {
            app.update();
        }
        assert_eq!(
            state_of(&app, ship),
            AIBehaviorState::Engage,
            "the jink is timed: it decays back to Engage"
        );
    }

    #[test]
    fn a_hostile_holding_its_nose_on_me_triggers_evade_without_a_hit() {
        // The second cheap signal: inside aim range with the hostile's hull
        // forward on my anchor. Driven through the real systems with a
        // zero-delta Time - entry does not need elapsed time.
        let mut world = World::new();
        world.init_resource::<Time>();
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            // Well inside the aim range, expressed against the constant: the
            // range moves whenever turret reach does.
            Transform::from_translation(Vec3::new(AI_THREAT_AIM_RANGE * 0.5, 0.0, 0.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
        ));

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Evade,
            "a hostile's guns on me inside aim range is a threat"
        );
    }

    #[test]
    fn beyond_aim_range_a_pointed_nose_is_not_a_threat() {
        // Same geometry outside AI_THREAT_AIM_RANGE (still inside engage
        // range): the nose cannot hurt me yet, so the ship keeps engaging.
        let mut world = World::new();
        world.init_resource::<Time>();
        let ship = world.spawn((AISpaceshipMarker, Transform::default())).id();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(AI_THREAT_AIM_RANGE + 100.0, 0.0, 0.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
        ));

        world.run_system_once(update_ai_target).unwrap();
        world.run_system_once(update_behavior_state).unwrap();

        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
    }

    #[test]
    fn an_evading_ship_burns_along_the_jink_not_the_pursuit_vector() {
        // Target dead ahead at -Z, far outside the standoff band: Engage
        // would burn straight at it. Evade must not - the jink leg points
        // well off the line of sight.
        let mut world = World::new();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::from_translation(Vec3::new(0.0, 0.0, -1000.0)),
            ))
            .id();
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Evade,
                AITarget(Some(target)),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(0.0),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();

        // Facing the target (the pursuit vector): an engaging ship would
        // burn, an evading one must hold - the jink points elsewhere.
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "facing the target, the jink gate must not open"
        );

        // Swing the hull onto the jink leg: the lateral burst fires.
        let jink = ai_evade_direction(Vec3::new(0.0, 0.0, -1000.0), 0);
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .look_to(jink, Vec3::Y);
        world.run_system_once(on_thruster_input).unwrap();
        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            1.0,
            "aligned with the jink leg, the burst fires"
        );
    }
}
