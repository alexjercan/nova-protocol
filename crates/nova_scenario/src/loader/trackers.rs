//! Orbit-lifecycle and weapon-lock events derived from live ship state.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use super::clock::scenario_elapsed;
use crate::prelude::*;

/// Last reported ORBIT state for one ship.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub(super) struct OrbitEcho {
    /// Well named by the active ORBIT maneuver.
    pub well: Entity,
    /// Scenario id retained so a despawned well can still produce an end edge.
    pub well_id: String,
    /// Whether stable station-keeping was last reported.
    pub stable: bool,
}

fn orbit_info(
    well_id: &str,
    ship_id: &EntityId,
    ship_type_name: &EntityTypeName,
) -> OrbitEventInfo {
    OrbitEventInfo {
        id: well_id.to_string(),
        other_id: ship_id.0.clone(),
        other_type_name: ship_type_name.0.clone(),
    }
}

/// Emit edge-triggered ORBIT lifecycle events. Entering `Hold` is stable;
/// leaving `Hold` while ORBIT remains engaged is unstable. A surviving ship
/// that leaves ORBIT emits only end, not unstable followed by end. Despawned
/// ships use `OnDestroyed` and intentionally emit no orbit-end event.
#[expect(
    clippy::type_complexity,
    reason = "one query snapshots the complete orbit transition"
)]
pub(super) fn track_orbit_transitions(
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &Autopilot,
            Option<&mut OrbitEcho>,
            &EntityId,
            &EntityTypeName,
        ),
        With<SpaceshipRootMarker>,
    >,
    q_ended: Query<
        (Entity, &OrbitEcho, &EntityId, &EntityTypeName),
        (With<SpaceshipRootMarker>, Without<Autopilot>),
    >,
    q_ids: Query<&EntityId>,
) {
    for (ship, echo, ship_id, ship_type_name) in &q_ended {
        commands.fire::<OnOrbitEndEvent>(orbit_info(&echo.well_id, ship_id, ship_type_name));
        commands.entity(ship).remove::<OrbitEcho>();
    }

    for (ship, autopilot, echo, ship_id, ship_type_name) in &mut q_ships {
        let AutopilotAction::Orbit { well, .. } = autopilot.action else {
            if let Some(echo) = echo {
                commands.fire::<OnOrbitEndEvent>(orbit_info(
                    &echo.well_id,
                    ship_id,
                    ship_type_name,
                ));
                commands.entity(ship).remove::<OrbitEcho>();
            }
            continue;
        };
        let stable = autopilot.phase == AutopilotPhase::Hold;

        match echo {
            None => {
                let Ok(well_id) = q_ids.get(well) else {
                    continue;
                };
                let info = orbit_info(&well_id.0, ship_id, ship_type_name);
                commands.fire::<OnOrbitStartEvent>(info.clone());
                if stable {
                    commands.fire::<OnOrbitStableEvent>(info);
                }
                commands.entity(ship).insert(OrbitEcho {
                    well,
                    well_id: well_id.0.clone(),
                    stable,
                });
            }
            Some(mut echo) if echo.well != well => {
                commands.fire::<OnOrbitEndEvent>(orbit_info(
                    &echo.well_id,
                    ship_id,
                    ship_type_name,
                ));
                let Ok(well_id) = q_ids.get(well) else {
                    commands.entity(ship).remove::<OrbitEcho>();
                    continue;
                };
                let info = orbit_info(&well_id.0, ship_id, ship_type_name);
                commands.fire::<OnOrbitStartEvent>(info.clone());
                if stable {
                    commands.fire::<OnOrbitStableEvent>(info);
                }
                echo.well = well;
                echo.well_id = well_id.0.clone();
                echo.stable = stable;
            }
            Some(mut echo) if echo.stable != stable => {
                let info = orbit_info(&echo.well_id, ship_id, ship_type_name);
                if stable {
                    commands.fire::<OnOrbitStableEvent>(info);
                } else {
                    commands.fire::<OnOrbitUnstableEvent>(info);
                }
                echo.stable = stable;
            }
            Some(_) => {}
        }
    }
}

fn resolve_window_secs(override_secs: Option<f64>, default: f64) -> f64 {
    match override_secs {
        Some(secs) if secs.is_finite() && secs > 0.0 => secs,
        _ => default,
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

    /// ORBIT events report each state edge once, including ordered well switches.
    #[test]
    fn orbit_lifecycle_events_are_edge_triggered() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{Autopilot, AutopilotAction, AutopilotPhase};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_orbit_transitions);

        fn count_handler(event: EventConfig, key: &str) -> EventHandler<NovaEventWorld> {
            let mut handler = EventHandler::<NovaEventWorld>::from(event);
            handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(key)),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            }));
            handler
        }
        for (event, key) in [
            (EventConfig::OnOrbitStart, "start"),
            (EventConfig::OnOrbitStable, "stable"),
            (EventConfig::OnOrbitUnstable, "unstable"),
            (EventConfig::OnOrbitEnd, "end"),
        ] {
            app.world_mut().spawn(count_handler(event, key));
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        let mut order_start = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbitStart);
        order_start.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "order".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("order")),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::String("S".to_string())),
                )),
            ),
        }));
        app.world_mut().spawn(order_start);
        let mut order_end = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbitEnd);
        order_end.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "order".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("order")),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::String("E".to_string())),
                )),
            ),
        }));
        app.world_mut().spawn(order_end);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("order".to_string(), VariableLiteral::String(String::new()));
        let counts = |app: &App| {
            let world = app.world().resource::<NovaEventWorld>();
            let n = |key| match world.get_variable(key) {
                Some(VariableLiteral::Number(value)) => *value,
                other => panic!("{key} count missing: {other:?}"),
            };
            (n("start"), n("stable"), n("unstable"), n("end"))
        };
        let settle = |app: &mut App| {
            app.update();
            app.update();
        };

        let old_well = app.world_mut().spawn(EntityId::new("old_well")).id();
        let new_well = app.world_mut().spawn(EntityId::new("new_well")).id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("player"),
                EntityTypeName::new("spaceship"),
                Autopilot::engage(AutopilotAction::Orbit {
                    well: old_well,
                    plan: None,
                }),
            ))
            .id();
        settle(&mut app);
        assert_eq!(counts(&app), (1.0, 0.0, 0.0, 0.0));

        app.world_mut().get_mut::<Autopilot>(ship).unwrap().phase = AutopilotPhase::Hold;
        settle(&mut app);
        assert_eq!(counts(&app), (1.0, 1.0, 0.0, 0.0));

        app.world_mut().get_mut::<Autopilot>(ship).unwrap().phase = AutopilotPhase::Burn;
        settle(&mut app);
        assert_eq!(counts(&app), (1.0, 1.0, 1.0, 0.0));

        app.world_mut().get_mut::<Autopilot>(ship).unwrap().phase = AutopilotPhase::Hold;
        settle(&mut app);
        assert_eq!(counts(&app), (1.0, 2.0, 1.0, 0.0));

        app.world_mut().get_mut::<Autopilot>(ship).unwrap().action = AutopilotAction::Orbit {
            well: new_well,
            plan: None,
        };
        settle(&mut app);
        assert_eq!(
            counts(&app),
            (2.0, 3.0, 1.0, 1.0),
            "switch emits end-old, start-new, stable-new"
        );
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("order"),
            Some(&VariableLiteral::String("SES".to_string())),
            "well switch queues end-old before start-new"
        );

        app.world_mut().entity_mut(new_well).despawn();
        app.world_mut().entity_mut(ship).remove::<Autopilot>();
        settle(&mut app);
        assert_eq!(
            counts(&app),
            (2.0, 3.0, 1.0, 2.0),
            "losing a well ends a stable orbit without an unstable edge"
        );
    }

    /// Destruction has its own event and does not synthesize an orbit end.
    #[test]
    fn destroyed_orbiting_ship_does_not_emit_orbit_end() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{Autopilot, AutopilotAction};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_orbit_transitions);
        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbitEnd);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "ended".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);
        let well = app.world_mut().spawn(EntityId::new("well")).id();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("ship"),
                EntityTypeName::new("spaceship"),
                Autopilot::engage(AutopilotAction::Orbit { well, plan: None }),
            ))
            .id();
        app.update();
        app.update();
        app.world_mut().entity_mut(ship).despawn();
        app.update();
        app.update();
        assert!(app
            .world()
            .resource::<NovaEventWorld>()
            .get_variable("ended")
            .is_none());
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
