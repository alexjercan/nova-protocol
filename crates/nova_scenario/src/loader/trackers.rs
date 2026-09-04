//! Orbit-lifecycle, weapon-lock and ship-helm-order events derived from
//! live ship state.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

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

/// Last reported target for one lock slot.
#[derive(Clone, Debug, Reflect)]
struct LockTargetEcho {
    entity: Entity,
    id: String,
}

/// Last reported player lock state.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub(super) struct LockEcho {
    travel: Option<LockTargetEcho>,
    combat: Option<LockTargetEcho>,
}

fn lock_transition(
    echo: &mut Option<LockTargetEcho>,
    current: Option<Entity>,
    q_ids: &Query<&EntityId>,
) -> (Option<String>, Option<String>) {
    if echo.as_ref().map(|target| target.entity) == current {
        return (None, None);
    }

    let ended = echo.take().map(|target| target.id);
    let started = current
        .and_then(|entity| q_ids.get(entity).ok().map(|id| (entity, id.0.clone())))
        .map(|(entity, id)| {
            *echo = Some(LockTargetEcho {
                entity,
                id: id.clone(),
            });
            id
        });
    (ended, started)
}

/// Turn physical player-autopilot completions into authored GOTO and STOP
/// events. The flight layer reports only successful terminal conditions, so a
/// canceled maneuver, lost target, or disabled ship cannot satisfy a scenario
/// continuation.
pub(super) fn track_player_autopilot_completions(
    q_ships: Query<
        (
            Entity,
            &PlayerAutopilotCompleted,
            &EntityId,
            &EntityTypeName,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_ids: Query<&EntityId>,
    mut commands: Commands,
) {
    for (ship, completion, ship_id, ship_type_name) in &q_ships {
        match completion.action {
            AutopilotAction::Stop => {
                commands.fire::<OnStopCompleteEvent>(OnStopCompleteEventInfo {
                    id: ship_id.0.clone(),
                    type_name: ship_type_name.0.clone(),
                });
            }
            AutopilotAction::Goto { target } => {
                if let Ok(target_id) = q_ids.get(target) {
                    commands.fire::<OnGotoCompleteEvent>(OnGotoCompleteEventInfo {
                        id: target_id.0.clone(),
                        other_id: ship_id.0.clone(),
                        other_type_name: ship_type_name.0.clone(),
                    });
                }
            }
            AutopilotAction::GotoPos { .. } | AutopilotAction::Orbit { .. } => {}
        }
        commands.entity(ship).remove::<PlayerAutopilotCompleted>();
    }
}

fn lock_info(
    target_id: String,
    ship_id: &EntityId,
    ship_type_name: &EntityTypeName,
) -> LockEventInfo {
    LockEventInfo {
        id: target_id,
        other_id: ship_id.0.clone(),
        other_type_name: ship_type_name.0.clone(),
    }
}

/// Emit edge-triggered lock lifecycle events for the player ship. Target
/// switches emit end for the old target before start for the new target. AI
/// locks remain gameplay-internal and do not fire scenario events.
pub(super) fn track_player_locks(
    mut commands: Commands,
    mut q_ships: Query<
        (
            Entity,
            &TravelLock,
            &CombatLock,
            Option<&mut LockEcho>,
            &EntityId,
            &EntityTypeName,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_ids: Query<&EntityId>,
) {
    for (ship, travel, combat, echo, ship_id, ship_type_name) in &mut q_ships {
        let Some(mut echo) = echo else {
            commands.entity(ship).insert(LockEcho::default());
            continue;
        };

        let (travel_end, travel_start) = lock_transition(&mut echo.travel, travel.0, &q_ids);
        if let Some(target_id) = travel_end {
            commands.fire::<OnTravelLockEndEvent>(lock_info(target_id, ship_id, ship_type_name));
        }
        if let Some(target_id) = travel_start {
            commands.fire::<OnTravelLockStartEvent>(lock_info(target_id, ship_id, ship_type_name));
        }

        let (combat_end, combat_start) = lock_transition(&mut echo.combat, combat.0, &q_ids);
        if let Some(target_id) = combat_end {
            commands.fire::<OnCombatLockEndEvent>(lock_info(target_id, ship_id, ship_type_name));
        }
        if let Some(target_id) = combat_start {
            commands.fire::<OnCombatLockStartEvent>(lock_info(target_id, ship_id, ship_type_name));
        }
    }
}

/// Turn every unreported ship-order outcome into its scenario event.
///
/// The seam between the two layers. The flight layer decides WHAT happened -
/// an arrival, an AI breaking off, a well that stopped existing - and queues
/// it on the ship; this drains the queue and names each outcome in the
/// authored vocabulary. Nothing in `nova_ship` knows what a scenario event is,
/// and nothing here re-derives physics.
///
/// A queue rather than a per-frame comparison because outcomes are EDGES, and
/// several can land in one tick: a replacement order cancels the old one and
/// the new one can fail on the same tick it was installed. Draining preserves
/// the order they happened in, which is the order a chained beat expects.
pub(super) fn track_ship_order_reports(
    mut q_ships: Query<
        (&mut ShipOrderReports, &EntityId, &EntityTypeName),
        With<SpaceshipRootMarker>,
    >,
    mut commands: Commands,
) {
    for (mut reports, ship_id, ship_type_name) in &mut q_ships {
        if reports.0.is_empty() {
            continue;
        }
        for report in reports.0.drain(..) {
            debug!(
                "track_ship_order_reports: ship '{}' {:?} {:?} order '{}'",
                ship_id.0, report.outcome, report.kind, report.key
            );
            // One payload shape, five event kinds. The fields are named
            // identically across all five so a single `ShipOrder` filter
            // matches whichever a handler listens for, and the ordinary entity
            // filter still finds the ship.
            let order = report.key;
            let id = ship_id.0.clone();
            let type_name = ship_type_name.0.clone();
            let kind = report.kind;
            match report.outcome {
                ShipOrderOutcome::Complete => {
                    commands.fire::<OnShipOrderCompleteEvent>(OnShipOrderCompleteEventInfo {
                        order,
                        id,
                        type_name,
                        kind,
                    });
                }
                ShipOrderOutcome::Interrupted => {
                    commands.fire::<OnShipOrderInterruptedEvent>(OnShipOrderInterruptedEventInfo {
                        order,
                        id,
                        type_name,
                        kind,
                    });
                }
                ShipOrderOutcome::Resumed => {
                    commands.fire::<OnShipOrderResumedEvent>(OnShipOrderResumedEventInfo {
                        order,
                        id,
                        type_name,
                        kind,
                    });
                }
                ShipOrderOutcome::Canceled => {
                    commands.fire::<OnShipOrderCanceledEvent>(OnShipOrderCanceledEventInfo {
                        order,
                        id,
                        type_name,
                        kind,
                    });
                }
                ShipOrderOutcome::Failed => {
                    commands.fire::<OnShipOrderFailedEvent>(OnShipOrderFailedEventInfo {
                        order,
                        id,
                        type_name,
                        kind,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    /// Successful player completions preserve both sides of a GOTO and name a
    /// STOP by its ship. Scenario handlers can therefore retire a target only
    /// after the physical maneuver has stopped using it.
    #[test]
    fn player_autopilot_completions_fire_targeted_goto_and_stop_events() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_player_autopilot_completions);

        let set_seen = |key: &str| {
            EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            })
        };
        let mut goto = EventHandler::<NovaEventWorld>::from(EventConfig::OnGotoComplete);
        goto.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("mark".to_string()),
            other_id: Some("cutter".to_string()),
            ..default()
        }));
        goto.add_action(set_seen("goto_seen"));
        app.world_mut().spawn(goto);

        let mut stop = EventHandler::<NovaEventWorld>::from(EventConfig::OnStopComplete);
        stop.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("cutter".to_string()),
            ..default()
        }));
        stop.add_action(set_seen("stop_seen"));
        app.world_mut().spawn(stop);

        for key in ["goto_seen", "stop_seen"] {
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }

        let cutter = app
            .world_mut()
            .spawn((
                PlayerSpaceshipMarker,
                EntityId::new("cutter"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ))
            .id();
        let mark = app.world_mut().spawn(EntityId::new("mark")).id();
        app.update();

        app.world_mut()
            .entity_mut(cutter)
            .insert(PlayerAutopilotCompleted {
                action: AutopilotAction::Goto { target: mark },
            });
        app.update();
        app.update();
        app.world_mut()
            .entity_mut(cutter)
            .insert(PlayerAutopilotCompleted {
                action: AutopilotAction::Stop,
            });
        app.update();
        app.update();

        let value = |key| {
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable(key)
                .cloned()
        };
        assert_eq!(value("goto_seen"), Some(VariableLiteral::Number(1.0)));
        assert_eq!(value("stop_seen"), Some(VariableLiteral::Number(1.0)));
    }

    /// Each queued outcome fires ONCE, under its own key, as its own event
    /// kind. The `ShipOrder` filter is what a waiting beat matches it with, so
    /// the test gates on one - and the filter must be able to tell a
    /// completion from a cancellation of the same order.
    #[test]
    fn every_order_outcome_fires_its_own_event_once_under_its_key() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{ShipOrderOutcome, ShipOrderReport, ShipOrderReports};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_ship_order_reports);

        // One counter per lifecycle event, each behind the `ShipOrder` filter
        // a beat waiting on that order would use.
        let counters = [
            (EventConfig::OnShipOrderComplete, "complete"),
            (EventConfig::OnShipOrderInterrupted, "interrupted"),
            (EventConfig::OnShipOrderResumed, "resumed"),
            (EventConfig::OnShipOrderCanceled, "canceled"),
            (EventConfig::OnShipOrderFailed, "failed"),
        ];
        for (event, key) in counters {
            let mut handler = EventHandler::<NovaEventWorld>::from(event);
            handler.add_filter(EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some("approach".to_string()),
                ship: Some("warship".to_string()),
                kind: Some(ShipOrderKind::Move),
            }));
            handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name(key)),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            }));
            app.world_mut().spawn(handler);
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        let count =
            |app: &App, key: &str| match app.world().resource::<NovaEventWorld>().get_variable(key)
            {
                Some(VariableLiteral::Number(value)) => *value,
                other => panic!("{key} count missing: {other:?}"),
            };
        let settle = |app: &mut App| {
            app.update();
            app.update();
        };

        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("warship"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                ShipOrderReports::default(),
            ))
            .id();

        settle(&mut app);
        for (_, key) in counters {
            assert_eq!(
                count(&app, key),
                0.0,
                "an empty queue fires nothing ({key})"
            );
        }

        let queue = |app: &mut App, outcome: ShipOrderOutcome| {
            app.world_mut()
                .get_mut::<ShipOrderReports>(ship)
                .expect("the ship carries a report queue")
                .0
                .push(ShipOrderReport {
                    key: "approach".to_string(),
                    kind: ShipOrderKind::Move,
                    outcome,
                });
        };

        // The whole interruptible life of one order, in the order it happens.
        queue(&mut app, ShipOrderOutcome::Interrupted);
        queue(&mut app, ShipOrderOutcome::Resumed);
        settle(&mut app);
        assert_eq!(count(&app, "interrupted"), 1.0);
        assert_eq!(count(&app, "resumed"), 1.0);
        assert_eq!(
            count(&app, "complete"),
            0.0,
            "an interruption is not a completion"
        );

        queue(&mut app, ShipOrderOutcome::Complete);
        settle(&mut app);
        assert_eq!(count(&app, "complete"), 1.0);

        settle(&mut app);
        assert_eq!(
            count(&app, "complete"),
            1.0,
            "a drained queue does not re-report"
        );

        queue(&mut app, ShipOrderOutcome::Canceled);
        queue(&mut app, ShipOrderOutcome::Failed);
        settle(&mut app);
        assert_eq!(count(&app, "canceled"), 1.0);
        assert_eq!(count(&app, "failed"), 1.0);
        assert_eq!(
            count(&app, "complete"),
            1.0,
            "and none of them woke the completion beat"
        );
    }

    /// The `ShipOrder` filter separates two orders on the SAME ship by key: a
    /// beat waiting for the approach must not fire on the bore's completion.
    #[test]
    fn an_outcome_only_wakes_the_beat_waiting_on_its_own_key() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{ShipOrderOutcome, ShipOrderReport, ShipOrderReports};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_ship_order_reports);

        for key in ["approach", "bore"] {
            let mut handler =
                EventHandler::<NovaEventWorld>::from(EventConfig::OnShipOrderComplete);
            handler.add_filter(EventFilterConfig::ShipOrder(ShipOrderFilterConfig {
                order: Some(key.to_string()),
                ..default()
            }));
            handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
                key: key.to_string(),
                expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            }));
            app.world_mut().spawn(handler);
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        let count =
            |app: &App, key: &str| match app.world().resource::<NovaEventWorld>().get_variable(key)
            {
                Some(VariableLiteral::Number(value)) => *value,
                other => panic!("{key} count missing: {other:?}"),
            };

        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("warship"),
            EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ShipOrderReports(vec![ShipOrderReport {
                key: "bore".to_string(),
                kind: ShipOrderKind::Align,
                outcome: ShipOrderOutcome::Complete,
            }]),
        ));

        app.update();
        app.update();
        assert_eq!(count(&app, "bore"), 1.0, "the bore's beat hears it");
        assert_eq!(
            count(&app, "approach"),
            0.0,
            "and the approach's beat does not"
        );
    }

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
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
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
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
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

    /// Lock lifecycle events fire once per edge. A target switch queues end-old
    /// before start-new, held locks stay quiet, and AI locks remain internal.
    #[test]
    fn player_lock_lifecycle_events_are_edge_triggered() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, PlayerSpaceshipMarker, SpaceshipRootMarker};
        use nova_ship::prelude::{CombatLock, TravelLock};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_player_locks);

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
            (EventConfig::OnTravelLockStart, "travel_start"),
            (EventConfig::OnTravelLockEnd, "travel_end"),
            (EventConfig::OnCombatLockStart, "combat_start"),
            (EventConfig::OnCombatLockEnd, "combat_end"),
        ] {
            app.world_mut().spawn(count_handler(event, key));
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable(key.to_string(), VariableLiteral::Number(0.0));
        }
        for (event, edge) in [
            (EventConfig::OnTravelLockStart, "S"),
            (EventConfig::OnTravelLockEnd, "E"),
        ] {
            let mut handler = EventHandler::<NovaEventWorld>::from(event);
            handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "travel_order".to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name("travel_order")),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::String(edge.to_string())),
                    )),
                ),
            }));
            app.world_mut().spawn(handler);
        }
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable(
                "travel_order".to_string(),
                VariableLiteral::String(String::new()),
            );

        let count =
            |app: &App, key: &str| match app.world().resource::<NovaEventWorld>().get_variable(key)
            {
                Some(VariableLiteral::Number(value)) => *value,
                other => panic!("{key} count missing: {other:?}"),
            };
        let settle = |app: &mut App| {
            app.update();
            app.update();
        };

        let old_target = app.world_mut().spawn(EntityId::new("old_target")).id();
        let new_target = app.world_mut().spawn(EntityId::new("new_target")).id();
        let unnamed_target = app.world_mut().spawn_empty().id();
        let player = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                EntityId::new("player"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                TravelLock(None),
                CombatLock(None),
            ))
            .id();
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("ai"),
            EntityTypeName::new(SPACESHIP_TYPE_NAME),
            TravelLock(None),
            CombatLock(Some(old_target)),
        ));
        settle(&mut app);

        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(old_target);
        settle(&mut app);
        assert_eq!(count(&app, "travel_start"), 1.0);
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(count(&app, "travel_start"), 1.0, "held lock stays quiet");

        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(new_target);
        settle(&mut app);
        assert_eq!(count(&app, "travel_start"), 2.0);
        assert_eq!(count(&app, "travel_end"), 1.0);
        assert_eq!(
            app.world()
                .resource::<NovaEventWorld>()
                .get_variable("travel_order"),
            Some(&VariableLiteral::String("SES".to_string())),
            "target switch queues end-old before start-new"
        );

        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = None;
        app.world_mut().get_mut::<CombatLock>(player).unwrap().0 = Some(old_target);
        settle(&mut app);
        assert_eq!(count(&app, "travel_end"), 2.0);
        assert_eq!(count(&app, "combat_start"), 1.0);

        app.world_mut().get_mut::<CombatLock>(player).unwrap().0 = None;
        settle(&mut app);
        assert_eq!(count(&app, "combat_end"), 1.0);

        app.world_mut().get_mut::<TravelLock>(player).unwrap().0 = Some(unnamed_target);
        settle(&mut app);
        assert_eq!(
            count(&app, "travel_start"),
            2.0,
            "id-less targets stay quiet"
        );
        assert_eq!(count(&app, "combat_start"), 1.0, "AI locks never fire");
    }
}
