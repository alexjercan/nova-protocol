//! Orbit-hold and weapon-lock trackers: the state-derived events
//! (`OnOrbit`, `OnTravelLock`, `OnCombatLock`) a scenario can react to.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::clock::scenario_elapsed;
use crate::prelude::*;

/// How long (seconds) a ship must hold an engaged ORBIT around one well before
/// [`OnOrbitEvent`] fires - and the RE-FIRE period while the hold continues.
/// Recurring, not once-per-engagement: a single-shot event consumed while a
/// handler's beat guard rejects it would be gone for good, soft-locking any
/// scenario whose beat can advance during a held orbit. Beat-gated handlers
/// make the repeats no-ops.
const ORBIT_HOLD_SECS: f64 = 5.0;

/// Resolve an author-supplied event-window override against the engine default.
/// A non-finite or non-positive override is rejected (content_lint errors on
/// it), so at runtime we fail closed to `default` rather than ever produce a
/// zero/negative window that would fire every frame.
fn resolve_window_secs(override_secs: Option<f64>, default: f64) -> f64 {
    match override_secs {
        Some(secs) if secs.is_finite() && secs > 0.0 => secs,
        _ => default,
    }
}

/// Bookkeeping for the orbit-hold tracker, on the orbiting ship: which well
/// and the scenario-clock reading (`scenario_elapsed`) when the current
/// window opened - engagement, well switch, or the last fire. The window has
/// elapsed once `now - started_at >= window`, where `window` is the ship's
/// [`OrbitHoldSecs`] override or the `ORBIT_HOLD_SECS` default. Disengaging (or
/// switching wells) removes it, restarting the window; the component also dies
/// with its entity on teardown, so a retry re-arms against a fresh clock.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub(super) struct OrbitHold {
    /// The well entity the ship is currently orbiting.
    pub well: Entity,
    /// The scenario-clock time (seconds) the current hold began.
    pub started_at: f64,
}

/// Fire [`OnOrbitEvent`] once a ship has HELD an engaged `Autopilot { action:
/// Orbit { well } }` for [`ORBIT_HOLD_SECS`] continuously. Ships are identified
/// by their scenario `EntityId` (ships without one - editor previews - are
/// invisible to the tracker); the event's `id` is the WELL's scenario id,
/// mirroring OnEnter's (area, other) shape so filters compose identically. The
/// hold window is measured against the engine scenario clock
/// ([`scenario_elapsed`]) rather than an accumulated `Time` delta, so it
/// freezes under pause and resets on teardown/retry with the clock itself.
pub(super) fn track_orbit_holds(
    world: Res<NovaEventWorld>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &Autopilot,
            Option<&mut OrbitHold>,
            Option<&OrbitHoldSecs>,
            &EntityId,
            &EntityTypeName,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_disengaged: Query<Entity, (With<OrbitHold>, Without<Autopilot>)>,
    q_ids: Query<&EntityId>,
) {
    // Disengaged ships re-arm: the hold dies with the autopilot.
    for ship in &q_disengaged {
        commands.entity(ship).remove::<OrbitHold>();
    }

    let now = scenario_elapsed(&world);

    for (ship, autopilot, hold, hold_override, ship_id, ship_type_name) in &mut q_ships {
        let AutopilotAction::Orbit { well, .. } = autopilot.action else {
            // Engaged, but not an orbit (GOTO/STOP): no hold.
            if hold.is_some() {
                commands.entity(ship).remove::<OrbitHold>();
            }
            continue;
        };

        // Per-ship override (AIControllerConfig::orbit_hold_secs), else the
        // engine default.
        let window = resolve_window_secs(hold_override.map(|o| o.0), ORBIT_HOLD_SECS);

        match hold {
            Some(mut hold) if hold.well == well => {
                if now - hold.started_at >= window {
                    // Restart the window whether or not the event can be
                    // addressed.
                    hold.started_at = now;
                    let Ok(well_id) = q_ids.get(well) else {
                        // A well without a scenario id (despawned or
                        // non-scenario body) has no address to fire under.
                        continue;
                    };
                    debug!(
                        "track_orbit_holds: ship '{}' held orbit around '{}' for {}s",
                        ship_id.0, well_id.0, window
                    );
                    commands.fire::<OnOrbitEvent>(OnOrbitEventInfo {
                        id: well_id.0.clone(),
                        other_id: ship_id.0.clone(),
                        other_type_name: ship_type_name.0.clone(),
                    });
                }
            }
            // New engagement, or the directive switched wells: open a fresh
            // window on the current well, anchored at the current clock.
            _ => {
                commands.entity(ship).insert(OrbitHold {
                    well,
                    started_at: now,
                });
            }
        }
    }
}

/// Re-fire period (seconds) for a HELD lock. Acquisition fires immediately;
/// while the lock stays on the same target the event RECURS on this period -
/// the orbit-hold rationale: a one-shot event consumed under a rejecting beat
/// guard is gone for good, and a scenario whose beat advances while the lock is
/// already held would soft-lock. Beat-gated handlers make the repeats no-ops.
const LOCK_REFIRE_SECS: f64 = 5.0;

/// Bookkeeping for the player-lock bridge: per slot, the last target the
/// bridge saw and the scenario-clock reading (`scenario_elapsed`) when it
/// last fired for that target. The re-fire window has elapsed once
/// `now - last_fired_at >= refire`, where `refire` is the player's
/// [`LockRefireSecs`] override or the `LOCK_REFIRE_SECS` default.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub(super) struct LockEcho {
    travel: Option<(Entity, f64)>,
    combat: Option<(Entity, f64)>,
}

/// One lock slot's tick: returns `Some(target)` when the bridge should
/// fire this frame - on ACQUISITION (the slot's value changed onto a
/// target; the slot writers are equality-skipped, so a held live-radar
/// lock does not churn this) and again every `refire_secs` seconds while
/// the same target stays held. `now` is the engine scenario clock
/// ([`scenario_elapsed`]); the window is `now - last_fired_at`, so it freezes
/// under pause and resets on teardown with the clock. `refire_secs` is the
/// per-player override ([`LockRefireSecs`]) or the [`LOCK_REFIRE_SECS`] default,
/// resolved by the caller. Pure for the unit tests.
fn tick_lock_slot(
    state: &mut Option<(Entity, f64)>,
    current: Option<Entity>,
    now: f64,
    refire_secs: f64,
) -> Option<Entity> {
    match (current, state.as_mut()) {
        (None, _) => {
            *state = None;
            None
        }
        (Some(target), Some((held, last_fired_at))) if *held == target => {
            if now - *last_fired_at >= refire_secs {
                *last_fired_at = now;
                Some(target)
            } else {
                None
            }
        }
        (Some(target), _) => {
            *state = Some((target, now));
            Some(target)
        }
    }
}

/// Fire [`OnTravelLockEvent`]/[`OnCombatLockEvent`] when the PLAYER's lock
/// slots land on scenario objects. Player-scoped on purpose: the AI combat
/// mirror (nova_gameplay input/ai/acquisition.rs) writes `CombatLock` on every
/// engaged AI ship, and an unscoped bridge would fire for all of them. The
/// event's `id` is the locked TARGET's scenario id, `other` is the player ship
/// - OnEnter's (area, other) shape, so filters compose identically.
pub(super) fn track_player_locks(
    world: Res<NovaEventWorld>,
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &TravelLock,
            &CombatLock,
            Option<&mut LockEcho>,
            Option<&LockRefireSecs>,
            &EntityId,
            &EntityTypeName,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_ids: Query<&EntityId>,
) {
    let now = scenario_elapsed(&world);
    for (ship, travel, combat, echo, refire_override, ship_id, ship_type_name) in &mut q_ships {
        let Some(mut echo) = echo else {
            // First sight of this player ship: arm the bookkeeping; the
            // next frame ticks it (an already-held lock then reads as an
            // acquisition, which is the honest interpretation on spawn).
            commands.entity(ship).insert(LockEcho::default());
            continue;
        };
        // Per-player override (PlayerControllerConfig::lock_refire_secs), else
        // the engine default.
        let refire = resolve_window_secs(refire_override.map(|o| o.0), LOCK_REFIRE_SECS);
        let fired_travel = tick_lock_slot(&mut echo.travel, travel.0, now, refire);
        let fired_combat = tick_lock_slot(&mut echo.combat, combat.0, now, refire);
        if let Some(target_id) = fired_travel.and_then(|target| q_ids.get(target).ok()) {
            commands.fire::<OnTravelLockEvent>(OnTravelLockEventInfo {
                id: target_id.0.clone(),
                other_id: ship_id.0.clone(),
                other_type_name: ship_type_name.0.clone(),
            });
        }
        if let Some(target_id) = fired_combat.and_then(|target| q_ids.get(target).ok()) {
            commands.fire::<OnCombatLockEvent>(OnCombatLockEventInfo {
                id: target_id.0.clone(),
                other_id: ship_id.0.clone(),
                other_type_name: ship_type_name.0.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::{clock::tick_scenario_clock, fixtures::*};

    /// The orbit-hold tracker: an engaged ORBIT fires OnOrbit once per HOLD
    /// WINDOW - never before the window, never per frame, and the window recurs
    /// while the hold continues. Driven through the real event pipeline into a
    /// real handler counting into a scenario variable.
    #[test]
    fn orbit_hold_fires_once_per_window_and_recurs() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{Autopilot, AutopilotAction};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // 0.2s steps: Time<Virtual> clamps any single delta at its
        // default max_delta of 0.25s, so bigger manual steps silently
        // accumulate slower than wall time.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        // The tracker now measures its window against `scenario_elapsed`, so
        // the clock has to advance under the same gate production uses. Chain
        // the tick ahead of the tracker so it reads THIS frame's clock, exactly
        // like the plugin's `.after(tick_scenario_clock)` ordering.
        app.add_systems(
            Update,
            (tick_scenario_clock, track_orbit_holds)
                .chain()
                .run_if(scenario_is_live),
        );

        // The counting handler: orbits = orbits + 1 on every OnOrbit
        // under the well's id.
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbit);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("planetoid".to_string()),
            other_id: Some("player_spaceship".to_string()),
            ..default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "orbits".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("orbits".to_string())),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("orbits".to_string(), VariableLiteral::Number(0.0));

        let orbits = |app: &App| -> f64 {
            match app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable("orbits")
            {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("orbits variable missing: {:?}", other),
            }
        };

        let well = app
            .world_mut()
            .spawn(EntityId::new("planetoid".to_string()))
            .id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("player_spaceship".to_string()),
                EntityTypeName::new("spaceship".to_string()),
                Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            ))
            .id();

        // ~2 seconds of hold (10 frames at 0.2s): under the 5s window.
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(orbits(&app), 0.0, "no fire before the hold window");

        // Push just past the window: exactly one fire, not one per frame.
        // (~1.8s held so far; 18 frames = 3.6s more puts the total at
        // ~5.4s, 0.4s into the next window.)
        for _ in 0..18 {
            app.update();
        }
        assert_eq!(orbits(&app), 1.0, "one fire per window, not per frame");

        // Keep holding through a second full window: the event RECURS during
        // one continuous engagement - this is what saves a beat that advances
        // mid-orbit from a consumed one-shot.
        for _ in 0..25 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            2.0,
            "a continued hold fires again next window"
        );

        // Disengage, re-engage: the clock restarts from zero and the next
        // window fires again.
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        app.update();
        app.world_mut()
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::Orbit {
                well,
                plan: None,
            }));
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            3.0,
            "a fresh engagement fires on a fresh clock"
        );
    }

    /// A per-ship `OrbitHoldSecs` override shortens the hold window: a 1s
    /// override fires within ~1.2s of hold, long before the 5s default would.
    #[test]
    fn orbit_hold_honors_a_per_ship_override() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{Autopilot, AutopilotAction};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        app.add_systems(
            Update,
            (tick_scenario_clock, track_orbit_holds)
                .chain()
                .run_if(scenario_is_live),
        );

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbit);
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("planetoid".to_string()),
            other_id: Some("player_spaceship".to_string()),
            ..default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "orbits".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("orbits".to_string())),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("orbits".to_string(), VariableLiteral::Number(0.0));

        let orbits = |app: &App| -> f64 {
            match app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable("orbits")
            {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("orbits variable missing: {:?}", other),
            }
        };

        let well = app
            .world_mut()
            .spawn(EntityId::new("planetoid".to_string()))
            .id();
        // Override: a 1s hold window on this ship.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("player_spaceship".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            OrbitHoldSecs(1.0),
        ));

        // 3 frames (~0.6s): under the 1s window - no fire yet.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(orbits(&app), 0.0, "no fire before the 1s override window");

        // 5 more frames (total ~1.6s): past the 1s window - exactly one fire,
        // where the 5s default would still be silent.
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            orbits(&app),
            1.0,
            "the 1s override fires well before the 5s default"
        );
    }

    #[test]
    fn a_lock_slot_fires_on_acquisition_then_echoes_per_window() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let mut state = None;

        // `now` is the absolute scenario clock; advance it before each tick
        // exactly as the clock does per frame. The window is `now - last_fire`.
        let mut now = 0.0_f64;
        let mut at = |dt: f64| {
            now += dt;
            now
        };

        // The default engine window; the slot takes it per call.
        let w = LOCK_REFIRE_SECS;

        // Acquisition fires immediately.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(0.1), w), Some(a));
        // Held: quiet until the echo window elapses, then one re-fire.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        assert_eq!(
            tick_lock_slot(&mut state, Some(a), at(2.0), w),
            Some(a),
            "a held lock echoes once per window (the anti-soft-lock recurrence)"
        );
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(2.0), w), None);
        // A live-radar retarget is a fresh acquisition on a fresh clock.
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(0.1), w), Some(b));
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(2.0), w), None);
        // Clearing re-arms: the next lock is an acquisition again.
        assert_eq!(tick_lock_slot(&mut state, None, at(0.1), w), None);
        assert_eq!(tick_lock_slot(&mut state, Some(b), at(0.1), w), Some(b));
    }

    /// A per-player `refire_secs` override changes the echo cadence: a 2s
    /// window re-fires after 2s of hold, where the 5s default would still be
    /// quiet.
    #[test]
    fn a_lock_slot_honors_a_custom_refire_window() {
        let a = Entity::from_raw_u32(1).unwrap();
        let mut state = None;
        let mut now = 0.0_f64;
        let mut at = |dt: f64| {
            now += dt;
            now
        };

        // Acquisition fires immediately regardless of window.
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(0.1), 2.0), Some(a));
        // 1.5s held under the 2s window: quiet (and would be quiet at 5s too).
        assert_eq!(tick_lock_slot(&mut state, Some(a), at(1.5), 2.0), None);
        // Crossing 2s of hold: re-fires on the SHORT window, where the 5s
        // default would not have yet.
        assert_eq!(
            tick_lock_slot(&mut state, Some(a), at(1.0), 2.0),
            Some(a),
            "a 2s override echoes at 2s of hold"
        );
    }

    /// The bridge end to end through the real event pipeline: a travel
    /// lock ticks a travel handler, a combat lock a combat handler, an AI
    /// ship's combat lock ticks NOTHING, and a target without a scenario
    /// id is quiet (delivery-guarded by the fires before it).
    #[test]
    fn player_locks_fire_their_events_and_ai_locks_never_do() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, PlayerSpaceshipMarker, SpaceshipRootMarker};
        use nova_ship::prelude::{CombatLock, TravelLock};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.2,
        )));
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.insert_resource(CurrentScenario(Some(scenario_with("live", vec![]))));
        // The bridge measures its echo window against `scenario_elapsed`, so
        // tick the clock ahead of the tracker (mirrors the plugin's
        // `.after(tick_scenario_clock)` ordering) - otherwise `now` never moves
        // and "held is quiet" would pass for the wrong reason.
        app.add_systems(
            Update,
            (tick_scenario_clock, track_player_locks)
                .chain()
                .run_if(scenario_is_live),
        );

        // Counting handlers: one per slot, filtered on the beacon's id.
        let count_into = |key: &str| -> EventActionConfig {
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(key.to_string())),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            })
        };
        for (config, key) in [
            (EventConfig::OnTravelLock, "travel_locks"),
            (EventConfig::OnCombatLock, "combat_locks"),
        ] {
            let mut handler = EventHandler::<NovaEventWorld>::from(config);
            handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("beacon_3".to_string()),
                other_id: Some("player_spaceship".to_string()),
                ..default()
            }));
            handler.add_action(count_into(key));
            app.world_mut().spawn(handler);
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        let count = |app: &App, key: &str| -> f64 {
            match app.world().resource::<NovaEventWorld>().get_variable(key) {
                Some(VariableLiteral::Number(n)) => *n,
                other => panic!("{key} variable missing: {:?}", other),
            }
        };

        let beacon = app
            .world_mut()
            .spawn(EntityId::new("beacon_3".to_string()))
            .id();
        let unnamed = app.world_mut().spawn_empty().id();
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId::new("player_spaceship".to_string()),
                EntityTypeName::new("spaceship".to_string()),
                TravelLock(None),
                CombatLock(None),
            ))
            .id();
        // An AI ship with a combat lock on the SAME beacon (the combat
        // mirror writes these constantly): must never fire.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("scavenger".to_string()),
            EntityTypeName::new("spaceship".to_string()),
            TravelLock(None),
            CombatLock(Some(beacon)),
        ));
        // Arm the echo bookkeeping (first frame inserts it).
        app.update();
        app.update();

        // Travel acquisition: one travel fire, no combat fire.
        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(beacon);
        app.update();
        app.update();
        assert_eq!(count(&app, "travel_locks"), 1.0, "travel lock ticks");
        assert_eq!(count(&app, "combat_locks"), 0.0);

        // Combat acquisition on the same target: the combat handler ticks.
        app.world_mut().get_mut::<CombatLock>(player).unwrap().0 = Some(beacon);
        app.update();
        app.update();
        assert_eq!(count(&app, "combat_locks"), 1.0, "combat lock ticks");

        // Holding both under the echo window: quiet (once per acquisition).
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(count(&app, "travel_locks"), 1.0);
        assert_eq!(count(&app, "combat_locks"), 1.0);

        // A target with no scenario id: quiet (the fires above are the
        // delivery guard that the pipeline works).
        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(unnamed);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            count(&app, "travel_locks"),
            1.0,
            "an id-less target fires nothing"
        );

        // The AI ship's lock sat on beacon_3 the whole test: still zero
        // fires beyond the player's own (the player-scope pin).
        assert_eq!(count(&app, "combat_locks"), 1.0, "AI locks never fire");
    }
}
