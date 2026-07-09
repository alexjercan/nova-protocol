use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{AIBehaviorState, AISpaceshipMarker, SpaceshipAIInputPlugin};
}

pub struct SpaceshipAIInputPlugin;

impl Plugin for SpaceshipAIInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipAIInputPlugin: build");

        app.register_type::<AIBehaviorState>();

        app.add_systems(
            Update,
            (
                update_behavior_state,
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_turret_target_input,
                on_projectile_input,
            )
                .chain()
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Marker component to identify the ai's spaceship.
///
/// This should be added to the root entity of the ai's spaceship.
/// Carries [`Allegiance::Enemy`] and an [`AIBehaviorState`] by requirement,
/// so every AI-marked root participates in the relation model and the
/// behavior state machine without extra spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Enemy, AIBehaviorState)]
pub struct AISpaceshipMarker;

/// What an AI ship is currently doing - the state skeleton of the AI combat
/// arc (docs/spikes/20260709-225508-ai-combat-behaviors.md). One state per
/// ship root, driven by [`update_behavior_state`]; every AI system gates its
/// behavior on it.
///
/// Only `Engage` and `Idle` have real behavior today. The others exist so
/// their tasks slot into a stable enum instead of reshaping it:
/// - `Patrol`: waypoint flight, task 20260709-225730 (behaves as `Idle`).
/// - `Evade`: under-fire jinking, task 20260709-225731 (stubs to `Engage`).
/// - `Retreat`: low-integrity disengage, task 20260709-225734 (stubs to
///   `Engage`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum AIBehaviorState {
    /// Station-keeping: no thrust, no fire, frozen helm.
    Idle,
    /// Waypoint flight (20260709-225730); behaves as `Idle` until then.
    Patrol,
    /// Chase and shoot the hostile - today's whole AI, and the default so
    /// an AI ship dropped into a fight behaves exactly as before the state
    /// machine existed.
    #[default]
    Engage,
    /// Under-fire evasion (20260709-225731); stubs to `Engage` until then.
    Evade,
    /// Low-integrity disengage (20260709-225734); stubs to `Engage` until
    /// then.
    Retreat,
}

impl AIBehaviorState {
    /// Whether this state runs the engage-style chase/aim/fire pipeline.
    /// `Evade` and `Retreat` deliberately stub to Engage behavior until
    /// their tasks land (see the variant docs).
    fn engages(&self) -> bool {
        matches!(self, Self::Engage | Self::Evade | Self::Retreat)
    }
}

/// The skeleton's one real transition: combat states need a hostile to
/// fight - with none in the world every state falls back to `Idle`, and a
/// hostile appearing pulls the passive states into `Engage`. Detection
/// RANGE (engage only when close enough) is the patrol task's scope
/// (20260709-225730); presence-based engagement matches today's
/// always-chase behavior. Pure for unit testing.
fn next_behavior_state(current: AIBehaviorState, hostile_present: bool) -> AIBehaviorState {
    if !hostile_present {
        return AIBehaviorState::Idle;
    }
    match current {
        // A hostile appeared: the passive states pick the fight up.
        AIBehaviorState::Idle | AIBehaviorState::Patrol => AIBehaviorState::Engage,
        // Combat states hold; their exit triggers are their tasks' scope.
        state => state,
    }
}

/// Drive each AI ship's [`AIBehaviorState`] from the world. Runs before the
/// behavior systems in the same frame so a transition takes effect
/// immediately (no one-frame stale-state window).
fn update_behavior_state(
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    mut q_spaceship: Query<&mut AIBehaviorState, With<AISpaceshipMarker>>,
) {
    // Minimal hostility: the player is the AI's only hostile until target
    // selection over the relation model lands (20260709-225727).
    let hostile_present = !q_player.is_empty();

    for mut state in &mut q_spaceship {
        let next = next_behavior_state(*state, hostile_present);
        // Change-detection hygiene: only write on a real transition.
        if *state != next {
            *state = next;
        }
    }
}

// AI "brain" tuning constants. The AI chases the player at a speed that scales with
// distance (so it slows as it closes in) and brakes when it overshoots.
/// Target chase speed per unit of distance to the player.
const AI_CHASE_SPEED_GAIN: f32 = 0.2;
/// Lower/upper clamp on the distance-scaled chase speed.
const AI_MIN_CHASE_SPEED: f32 = 2.0;
const AI_MAX_CHASE_SPEED: f32 = 20.0;
/// The ship brakes once its speed exceeds the target chase speed by this margin.
const AI_BRAKE_SPEED_MARGIN: f32 = 1.0;
/// Only thrust when the ship's forward vector aligns with the desired direction at least
/// this much (dot product, 1.0 == perfectly aligned).
const AI_THRUST_ALIGNMENT: f32 = 0.95;
/// Only fire when the muzzle aligns with the player at least this much.
const AI_FIRE_ALIGNMENT: f32 = 0.95;

/// The direction an AI ship should face: toward the player while it is slower than its
/// distance-scaled target speed, or opposite its velocity when overshooting (braking).
/// Falls back to facing the player if the computed direction degenerates to zero.
fn ai_desired_direction(to_player: Vec3, velocity: Vec3) -> Vec3 {
    let target_speed =
        (to_player.length() * AI_CHASE_SPEED_GAIN).clamp(AI_MIN_CHASE_SPEED, AI_MAX_CHASE_SPEED);
    let too_fast = velocity.length() > target_speed + AI_BRAKE_SPEED_MARGIN;

    let desired = if too_fast {
        // Brake: point opposite the current velocity.
        -velocity.normalize_or_zero()
    } else {
        // Chase: point toward the player.
        to_player.normalize()
    };

    if desired.length_squared() == 0.0 {
        to_player.normalize_or_zero()
    } else {
        desired
    }
}

fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            &ComputedAngularInertia,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    // Chase the player's live structure, not the root origin: the origin is
    // the build spot of the first sections and floats in empty space once
    // they are destroyed (task 20260709-150711). With no player at all the
    // command freezes, same as a non-engaging state.
    let Some(player) = player else {
        return;
    };
    let (player_transform, player_com) = player.into_inner();
    let player_anchor = live_structure_anchor(player_transform, player_com);

    for (entity, transform, velocity, inertia, com, state) in &q_spaceship {
        // A non-engaging state (Idle/Patrol) holds its helm: the command
        // freezes exactly like a dead helm, so re-engaging resumes from
        // where the hull actually points.
        if !state.engages() {
            continue;
        }
        // Both ends of the chase vector track live structure: the AI's own
        // root origin goes as stale as the player's once sections die.
        let own_anchor = live_structure_anchor(transform, com);
        let to_player = player_anchor - own_anchor;
        let desired_direction = ai_desired_direction(to_player, **velocity);

        // Slew the command at the hull's torque-budget turn rate instead of
        // rewriting it every frame: a distant setpoint drives the PD into
        // torque saturation where its damping is swamped and the hull
        // limit-cycles - the regime the player path was fixed for in the
        // flight-feel retune (20260709-095043). Same derivation as the
        // player path and the autopilot (flight::ship_turn_rate). With no
        // live computer the command FREEZES, matching the player path:
        // nothing consumes it, and slewing a dead helm would drift it so a
        // later re-activation snaps the hull.
        let Some(turn_rate) = crate::flight::ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == entity)
                .map(|(pd, _)| pd.max_torque),
            inertia,
            &settings,
        ) else {
            continue;
        };
        let max_step = turn_rate * time.delta_secs();

        for (mut controller, _) in q_controller
            .iter_mut()
            .filter(|(_, ChildOf(parent))| *parent == entity)
        {
            // The input is an ABSOLUTE world rotation - every other writer
            // treats it that way; the old code wrote a delta arc (the bug
            // this task fixes). The goal carries the command's own forward
            // onto the desired direction, and the command evolves from ITS
            // OWN previous state, never from the hull: a command rebuilt
            // from the hull each tick inherits the hull's roll, the PD then
            // sees zero roll error, and roll picked up during a swing spins
            // the ship forever (see the autopilot's rotation step).
            let command = **controller;
            let command_forward = command * Vec3::NEG_Z;
            let goal = Quat::from_rotation_arc(command_forward, desired_direction) * command;
            **controller = crate::flight::slew_rotation(command, goal, max_step);
        }
    }
}

fn on_thruster_input(
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &GlobalTransform, &ChildOf),
        With<ThrusterSectionMarker>,
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    let player_anchor = player.map(|player| {
        let (player_transform, player_com) = player.into_inner();
        live_structure_anchor(player_transform, player_com)
    });

    for (entity, transform, velocity, com, state) in &q_spaceship {
        // A non-engaging state (or no player left to chase) cuts the burn -
        // written as an explicit 0.0, not a skip, so a ship that was
        // thrusting when the state flipped actually stops.
        let thrust_level = match player_anchor {
            Some(player_anchor) if state.engages() => {
                // Same live-structure vector as the rotation system, so the
                // thrust gate and the rotation command agree on where
                // "toward the player" is.
                let to_player = player_anchor - live_structure_anchor(transform, com);
                let desired_direction = ai_desired_direction(to_player, **velocity);

                // Thrust only when the ship is pointing roughly toward the
                // desired direction.
                let forward = transform.forward();
                let alignment = forward.dot(desired_direction);
                if alignment > AI_THRUST_ALIGNMENT {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        for (mut thruster_input, _, _) in q_thruster
            .iter_mut()
            .filter(|(_, _, ChildOf(parent))| *parent == entity)
        {
            **thruster_input = thrust_level;
        }
    }
}

fn update_turret_target_input(
    mut q_turret: Query<(&mut TurretSectionTargetInput, &ChildOf), With<TurretSectionMarker>>,
    q_spaceship: Query<
        (Entity, &AIBehaviorState),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    // Aim at the live structure: fire converging on the root origin lands in
    // empty space once the player's front sections die (task 20260709-150711).
    let player_anchor = player.map(|player| {
        let (transform, com) = player.into_inner();
        live_structure_anchor(transform, com)
    });

    for (entity, state) in &q_spaceship {
        // A non-engaging state (or no player) clears the aim: turrets slew
        // back to rest instead of tracking a fight that is over.
        let target = if state.engages() { player_anchor } else { None };
        for (mut turret_input, _) in q_turret
            .iter_mut()
            .filter(|(_, ChildOf(c_parent))| *c_parent == entity)
        {
            **turret_input = target;
        }
    }
}

fn on_projectile_input(
    mut q_turret: Query<
        (
            &TurretSectionMuzzleEntity,
            &mut TurretSectionInput,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_spaceship: Query<
        (Entity, &AIBehaviorState),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Option<
        Single<
            (&Transform, Option<&ComputedCenterOfMass>),
            (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
        >,
    >,
) {
    let player_anchor = player.map(|player| {
        let (player_transform, player_com) = player.into_inner();
        live_structure_anchor(player_transform, player_com)
    });

    for (entity, state) in &q_spaceship {
        for (muzzle, mut input, _) in q_turret
            .iter_mut()
            .filter(|(_, _, ChildOf(c_parent))| *c_parent == entity)
        {
            // Hold fire outside the engaging states (or with no player) -
            // written as an explicit false so a firing turret stops.
            let (Some(player_anchor), true) = (player_anchor, state.engages()) else {
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

            let direction_to_player = (player_anchor - muzzle_transform.translation()).normalize();
            let forward = muzzle_transform.forward();

            let alignment = forward.dot(direction_to_player);
            **input = alignment > AI_FIRE_ALIGNMENT;
        }
    }
}

#[cfg(test)]
mod behavior_state_tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn transitions_need_a_hostile_to_fight() {
        use AIBehaviorState::*;

        // No hostile: every state falls back to Idle.
        for state in [Idle, Patrol, Engage, Evade, Retreat] {
            assert_eq!(next_behavior_state(state, false), Idle, "from {state:?}");
        }
        // Hostile present: passive states engage, combat states hold (their
        // exit triggers belong to their own tasks).
        assert_eq!(next_behavior_state(Idle, true), Engage);
        assert_eq!(next_behavior_state(Patrol, true), Engage);
        assert_eq!(next_behavior_state(Engage, true), Engage);
        assert_eq!(next_behavior_state(Evade, true), Evade);
        assert_eq!(next_behavior_state(Retreat, true), Retreat);
    }

    #[test]
    fn an_ai_ship_spawns_engaged_by_requirement() {
        // The default state preserves pre-state-machine behavior: an AI
        // ship dropped into a fight chases and shoots immediately.
        let mut world = World::new();
        let ship = world.spawn(AISpaceshipMarker).id();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage
        );
    }

    #[test]
    fn the_state_idles_without_a_player_and_reengages_with_one() {
        let mut world = World::new();
        let ship = world.spawn(AISpaceshipMarker).id();

        world.run_system_once(update_behavior_state).unwrap();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Idle,
            "no hostile in the world: nothing to engage"
        );

        world.spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        world.run_system_once(update_behavior_state).unwrap();
        assert_eq!(
            *world.entity(ship).get::<AIBehaviorState>().unwrap(),
            AIBehaviorState::Engage,
            "a hostile appearing pulls Idle back into the fight"
        );
    }

    #[test]
    fn idle_cuts_thrust_fire_and_aim() {
        // Flip a fully lit ship to Idle with the player still present: every
        // actuator must be explicitly zeroed, not left at its last value.
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
        ));
        let ship = world
            .spawn((
                AISpaceshipMarker,
                AIBehaviorState::Idle,
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        let thruster = world
            .spawn((
                ThrusterSectionMarker,
                ThrusterSectionInput(1.0),
                GlobalTransform::IDENTITY,
                ChildOf(ship),
            ))
            .id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(Some(Vec3::X)),
                TurretSectionInput(true),
                TurretSectionMuzzleEntity(Entity::PLACEHOLDER),
                ChildOf(ship),
            ))
            .id();

        world.run_system_once(on_thruster_input).unwrap();
        world.run_system_once(update_turret_target_input).unwrap();
        world.run_system_once(on_projectile_input).unwrap();

        assert_eq!(
            **world
                .entity(thruster)
                .get::<ThrusterSectionInput>()
                .unwrap(),
            0.0,
            "Idle cuts the burn"
        );
        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            None,
            "Idle clears the turret aim"
        );
        assert!(
            !**world.entity(turret).get::<TurretSectionInput>().unwrap(),
            "Idle holds fire"
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn ai_turrets_target_the_live_structure_anchor() {
        // AI fire must converge on the player's surviving structure, not the
        // root origin build-spot (task 20260709-150711).
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(0.0, 0.0, 3.0)),
        ));
        let ai_ship = world.spawn(AISpaceshipMarker).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(10.0, 0.0, 3.0)),
            "AI turret input = the player's live-structure anchor"
        );
    }

    #[test]
    fn ai_turrets_fall_back_to_the_origin_without_a_com() {
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        ));
        let ai_ship = world.spawn(AISpaceshipMarker).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(1.0, 2.0, 3.0))
        );
    }
}

#[cfg(test)]
mod rotation_tests {
    // Command-level harness with manual time, mirroring the player path's
    // command_lag_tests: the AI rotation command must be an ABSOLUTE world
    // rotation slewed at the hull's derived turn rate (task 20260709-155921).
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// An AI ship + controller facing -Z with the player dead astern (+Z),
    /// so the desired direction is a 180 flip from the initial command.
    fn flip_world() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        app.add_systems(Update, update_controller_target_rotation_torque);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        // The stock ship's numbers: inertia ~2.3, computer torque 10.
        let ship = app
            .world_mut()
            .spawn((
                AISpaceshipMarker,
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
                ComputedAngularInertia::new(Vec3::splat(2.3)),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_torque: 10.0,
                },
                ControllerSectionRotationInput::default(),
            ))
            .id();
        (app, controller)
    }

    #[test]
    fn an_ai_flip_reaches_the_command_over_many_frames() {
        // The old code rewrote the command every frame with no slew - the
        // exact PD-saturation regime the player path was fixed for.
        let (mut app, controller) = flip_world();

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let expected = crate::flight::hull_turn_rate(
            10.0,
            2.3,
            &app.world().resource::<FlightSettings>().clone(),
        ) / 60.0;
        // One frame advances exactly one slew step of the DERIVED rate -
        // this pins hull_turn_rate's wiring, not just "some" slew.
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one torque-budget slew step \
             (moved {moved}, expected {expected})"
        );
        let flip = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::Z);
        assert!(
            command.angle_between(flip) > 2.0,
            "a 180 flip must not reach the command in one frame"
        );
    }

    #[test]
    fn the_command_converges_to_the_absolute_look_at_rotation() {
        // The input is an absolute world rotation; the old code wrote a
        // DELTA (`from_rotation_arc(forward, desired)`), which for a
        // constant bearing never points the commanded forward at the
        // player. Slewed long enough, the command's forward must land on
        // the player bearing exactly.
        let (mut app, controller) = flip_world();

        for _ in 0..600 {
            app.update();
        }

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let commanded_forward = command * Vec3::NEG_Z;
        let to_player = Vec3::Z; // player at +Z, ship at the origin
        assert!(
            commanded_forward.dot(to_player) > 0.999,
            "the commanded forward must converge on the player bearing, \
             got {commanded_forward:?}"
        );
    }

    #[test]
    fn a_dead_helm_freezes_the_command() {
        // With no live computer the command must not drift (matches the
        // player path): slewing a dead helm would snap the hull on a later
        // re-activation.
        let (mut app, controller) = flip_world();
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);

        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        assert_eq!(command, Quat::IDENTITY, "dead helm: the command freezes");
    }
}

#[cfg(test)]
mod physics_tests {
    // A real avian world with the real PD, mirroring flight.rs's
    // physics-level harness: AI rotation command -> PD torque -> hull
    // swings. Covers the task's acceptance: the AI swings to the target
    // attitude and settles without limit-cycling (task 20260709-155921).
    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
    };

    #[test]
    fn the_ai_swings_onto_the_player_and_settles() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                super::super::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_controller_target_rotation_torque,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(super::super::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // Player abeam at +X: a 90-degree swing from the AI's initial -Z.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(200.0, 0.0, 0.0)),
        ));
        let ship = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default(), AISpaceshipMarker))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_torque: 10.0,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);
        // 10 simulated seconds: ample for the swing plus settling.
        for _ in 0..600 {
            app.update();
        }

        // No limit cycle on the aim: the nose must be ON the player and STAY
        // there for a further simulated second. The old delta-command code
        // fails this two ways: the delta setpoint never points the hull at
        // the player at all, and the unslewed rewrite saturates the PD into
        // an attitude limit cycle.
        let mut min_aim = f32::INFINITY;
        let mut max_spin = 0.0f32;
        for _ in 0..60 {
            app.update();
            let forward: Vec3 = app.world().get::<Transform>(ship).unwrap().forward().into();
            min_aim = min_aim.min(forward.dot(Vec3::X));
            let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
            max_spin = max_spin.max(spin);
        }
        assert!(
            min_aim > 0.996,
            "the hull must hold its nose on the player (within ~5 degrees) \
             for a full second, worst aim cos {min_aim}"
        );
        // The aim axes are quiet; what residual spin remains is pure ROLL
        // about the nose, which the bcs PD cannot damp (open bug
        // 20260709-125640, amplitude ~0.23 rad/s in this rig). Bound it so a
        // regression in THIS path still trips, and tighten toward ~0 when
        // the bcs fix lands.
        assert!(
            max_spin < 0.5,
            "residual spin must stay within the known roll-damping bound \
             (20260709-125640), got {max_spin} rad/s"
        );
    }
}
