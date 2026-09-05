//! Orbit-lifecycle, weapon-lock and ship-helm-order events derived from
//! live ship state.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

/// How long a ship may be off the ring before its partial lap is written off.
///
/// ORBIT is a maneuver the autopilot FLIES, not a pose the ship holds. Leaving
/// `Hold` only means the velocity error grew past the hold band, and `Align`
/// and `Burn` are the correction that puts the ship back on the ring - so
/// dropping lap progress the moment `Hold` ends drops it for the autopilot
/// doing its job. Three quarters of a lap could go to one nudge, with nothing
/// on screen to say why, and the player would fly the same ring again and
/// again wondering what they were doing wrong.
///
/// The honest line between "correcting" and "gone" is TIME, not phase: long
/// enough to cover any correction the autopilot makes on the ring, short
/// enough that a ship knocked off it, or flown off it deliberately, starts its
/// lap again.
pub const ORBIT_LAP_GRACE_SECS: f32 = 5.0;

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
    /// Whether the ship has reached its ring and is banking lap progress.
    ///
    /// Raised by the first `Hold` and cleared only by a departure longer than
    /// [`ORBIT_LAP_GRACE_SECS`]. Counting has to start at the RING rather than
    /// at the verb: the insertion approach curves around the well and can
    /// sweep most of a revolution before the ship is on any ring at all.
    pub inserted: bool,
    /// Seconds the ship has been continuously outside `Hold`, zero while it
    /// holds. Measured against [`ORBIT_LAP_GRACE_SECS`].
    pub off_ring: f32,
    /// Previous unit direction from the well to the ship.
    pub previous_radial: Option<Vec3>,
    /// Net angular travel since the ring was reached or the last reported lap.
    pub angular_travel: f32,
}

fn orbit_radial(
    ship: Entity,
    well: Entity,
    q_transforms: &Query<&GlobalTransform>,
) -> Option<Vec3> {
    let ship_position = q_transforms.get(ship).ok()?.translation();
    let well_position = q_transforms.get(well).ok()?.translation();
    (ship_position - well_position).try_normalize()
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
///
/// Lap progress is tracked on a longer fuse than the stability label: it
/// starts at the ring, accumulates through every phase the autopilot flies
/// there, and is written off only by a departure outlasting
/// [`ORBIT_LAP_GRACE_SECS`].
#[expect(
    clippy::type_complexity,
    reason = "one query snapshots the complete orbit transition"
)]
pub(super) fn track_orbit_transitions(
    mut commands: Commands,
    time: Res<Time>,
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
    q_transforms: Query<&GlobalTransform>,
) {
    for (ship, echo, ship_id, ship_type_name) in &q_ended {
        commands.fire::<OnOrbitEndEvent>(orbit_info(&echo.well_id, ship_id, ship_type_name));
        commands.entity(ship).remove::<OrbitEcho>();
    }

    for (ship, autopilot, echo, ship_id, ship_type_name) in &mut q_ships {
        let AutopilotAction::Orbit { well, plan } = autopilot.action else {
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
        // Mission progress is gated on this one phase: every orbit event, and
        // so the whole First Shift orbit beat, hangs off it. A flight retune
        // that leaves the ring flown but never flips the phase stalls a chapter
        // with nothing on screen to say why, so nova_ship pins the phase as
        // reachable in planetoid-strength gravity
        // (`a_strong_well_orbit_reaches_the_hold_phase_the_scenario_layer_reads`).
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
                    inserted: stable,
                    off_ring: 0.0,
                    previous_radial: orbit_radial(ship, well, &q_transforms),
                    angular_travel: 0.0,
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
                echo.inserted = stable;
                echo.off_ring = 0.0;
                echo.previous_radial = orbit_radial(ship, well, &q_transforms);
                echo.angular_travel = 0.0;
            }
            Some(mut echo) => {
                if echo.stable != stable {
                    let info = orbit_info(&echo.well_id, ship_id, ship_type_name);
                    if stable {
                        commands.fire::<OnOrbitStableEvent>(info);
                    } else {
                        commands.fire::<OnOrbitUnstableEvent>(info);
                    }
                    echo.stable = stable;
                }

                // The stability LABEL is an edge; lap progress is not. Holding
                // puts the ship on the ring and keeps it there, and any other
                // phase is the autopilot flying it back, so the clock - not the
                // phase - decides when the ring was abandoned.
                if stable {
                    echo.off_ring = 0.0;
                    echo.inserted = true;
                } else {
                    echo.off_ring += time.delta_secs();
                    if echo.off_ring > ORBIT_LAP_GRACE_SECS {
                        echo.inserted = false;
                        echo.angular_travel = 0.0;
                    }
                }

                let radial = orbit_radial(ship, well, &q_transforms);
                if echo.inserted {
                    if let (Some(previous), Some(current), Some(plan)) =
                        (echo.previous_radial, radial, plan)
                    {
                        let signed_step = plan
                            .normal
                            .dot(previous.cross(current))
                            .atan2(previous.dot(current));
                        echo.angular_travel += signed_step;
                        if echo.angular_travel >= std::f32::consts::TAU {
                            commands.fire::<OnOrbitLapEvent>(orbit_info(
                                &echo.well_id,
                                ship_id,
                                ship_type_name,
                            ));
                            echo.angular_travel -= std::f32::consts::TAU;
                        }
                    }
                }
                echo.previous_radial = radial;
            }
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

    /// A lap is net angular travel in the planned direction, not elapsed time.
    #[test]
    fn orbit_lap_fires_after_one_stable_revolution() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
        use nova_ship::prelude::{Autopilot, AutopilotAction, AutopilotPhase, OrbitPlan};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_systems(Update, track_orbit_transitions);

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbitLap);
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "lap".to_string(),
            expression: VariableExpressionNode::new_add(
                VariableTermNode::new_factor(VariableFactorNode::new_name("lap")),
                VariableExpressionNode::new_term(VariableTermNode::new_factor(
                    VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                )),
            ),
        }));
        app.world_mut().spawn(handler);
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("lap".to_string(), VariableLiteral::Number(0.0));

        let well = app
            .world_mut()
            .spawn((
                EntityId::new("well"),
                GlobalTransform::from_translation(Vec3::ZERO),
            ))
            .id();
        let mut autopilot = Autopilot::engage(AutopilotAction::Orbit {
            well,
            plan: Some(OrbitPlan {
                radius: 1.0,
                normal: Vec3::Z,
            }),
        });
        autopilot.phase = AutopilotPhase::Hold;
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("ship"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
                GlobalTransform::from_translation(Vec3::X),
                autopilot,
            ))
            .id();
        app.update();
        app.update();

        for radial in [Vec3::NEG_Y, Vec3::X] {
            app.world_mut()
                .entity_mut(ship)
                .insert(GlobalTransform::from_translation(radial));
            app.update();
            app.update();
        }
        assert_eq!(
            app.world().resource::<NovaEventWorld>().get_variable("lap"),
            Some(&VariableLiteral::Number(0.0)),
            "backtracking and retracing an arc are zero net travel"
        );

        for radial in [Vec3::Y, Vec3::NEG_X, Vec3::NEG_Y] {
            app.world_mut()
                .entity_mut(ship)
                .insert(GlobalTransform::from_translation(radial));
            app.update();
            app.update();
        }
        assert_eq!(
            app.world().resource::<NovaEventWorld>().get_variable("lap"),
            Some(&VariableLiteral::Number(0.0)),
            "three quarters of a real orbit do not complete the lap"
        );

        app.world_mut()
            .entity_mut(ship)
            .insert(GlobalTransform::from_translation(Vec3::X));
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<NovaEventWorld>().get_variable("lap"),
            Some(&VariableLiteral::Number(1.0))
        );
    }

    /// Seconds of tracker clock one [`LapRig`] tick advances.
    const TICK_SECS: f32 = 0.1;
    /// Ticks per revolution. A lap therefore takes 36 s, so
    /// [`ORBIT_LAP_GRACE_SECS`] is a seventh of one - roughly the proportion a
    /// real orbit has, and the reason the grace can be generous about a
    /// correction without ever covering a whole lap.
    const TICKS_PER_LAP: u32 = 360;
    /// Enough ticks to pass TAU from a standing start. One more than
    /// [`TICKS_PER_LAP`] because the tick that creates the echo has no
    /// previous radial to measure an arc against, plus slack that is nowhere
    /// near a second lap.
    const TICKS_TO_CLOSE_A_LAP: u32 = TICKS_PER_LAP + 10;

    /// One ship flying one ring around one well, on a clock the TEST owns:
    /// every tick advances the tracker by exactly [`TICK_SECS`] and the ship
    /// by one step of arc, whatever the wall clock did.
    struct LapRig {
        app: App,
        ship: Entity,
        flown: u32,
    }

    impl LapRig {
        fn new() -> Self {
            use nova_events::prelude::{EventHandler, GameEventsPlugin};
            use nova_gameplay::prelude::{GameObjectives, SpaceshipRootMarker};
            use nova_ship::prelude::{Autopilot, AutopilotAction, AutopilotPhase, OrbitPlan};

            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
            app.init_resource::<NovaEventWorld>();
            app.init_resource::<GameObjectives>();
            // A test measured in seconds off the ring cannot read the wall
            // clock. `ManualDuration` is the only lever that survives the
            // frame: writing the generic `Time` directly is undone when
            // `RunFixedMainLoop` restores it between `PreUpdate` and `Update`.
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(TICK_SECS),
            ));
            app.add_systems(Update, track_orbit_transitions);

            let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnOrbitLap);
            handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
                key: "lap".to_string(),
                expression: VariableExpressionNode::new_add(
                    VariableTermNode::new_factor(VariableFactorNode::new_name("lap")),
                    VariableExpressionNode::new_term(VariableTermNode::new_factor(
                        VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
                    )),
                ),
            }));
            app.world_mut().spawn(handler);
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable("lap".to_string(), VariableLiteral::Number(0.0));

            let well = app
                .world_mut()
                .spawn((
                    EntityId::new("well"),
                    GlobalTransform::from_translation(Vec3::ZERO),
                ))
                .id();
            let ship = app
                .world_mut()
                .spawn((
                    SpaceshipRootMarker,
                    EntityId::new("ship"),
                    EntityTypeName::new(SPACESHIP_TYPE_NAME),
                    GlobalTransform::from_translation(Vec3::X),
                    Autopilot {
                        action: AutopilotAction::Orbit {
                            well,
                            plan: Some(OrbitPlan {
                                radius: 1.0,
                                normal: Vec3::Y,
                            }),
                        },
                        phase: AutopilotPhase::Align,
                    },
                ))
                .id();
            // The first update carries a zero delta and only opens the echo;
            // the ring is flown from the second onward.
            app.update();
            Self {
                app,
                ship,
                flown: 0,
            }
        }

        /// Fly `ticks` more of the ring in `phase`, in the plan's travel
        /// direction. The ship keeps going round whatever the phase says -
        /// that is the point: `Align` and `Burn` here are the autopilot
        /// correcting ON the ring, not the ship leaving it.
        fn fly(&mut self, phase: nova_ship::prelude::AutopilotPhase, ticks: u32) {
            use nova_ship::prelude::Autopilot;

            for _ in 0..ticks {
                self.app
                    .world_mut()
                    .get_mut::<Autopilot>(self.ship)
                    .unwrap()
                    .phase = phase;
                self.flown += 1;
                let theta = std::f32::consts::TAU * f32::from(u16::try_from(self.flown).unwrap())
                    / f32::from(u16::try_from(TICKS_PER_LAP).unwrap());
                self.app.world_mut().entity_mut(self.ship).insert(
                    GlobalTransform::from_translation(Vec3::new(theta.cos(), 0.0, -theta.sin())),
                );
                self.app.update();
            }
        }

        /// Laps reported so far. Settles the two frames the event queue needs
        /// to hand the last tick's fire to its handler.
        fn laps(&mut self) -> f64 {
            self.app.update();
            self.app.update();
            match self
                .app
                .world()
                .resource::<NovaEventWorld>()
                .get_variable("lap")
            {
                Some(VariableLiteral::Number(laps)) => *laps,
                other => panic!("lap count missing: {other:?}"),
            }
        }
    }

    /// A correction does not cost the lap. Leaving `Hold` only means the
    /// velocity error grew past the hold band; the autopilot is still flying
    /// the ring, and it is back inside the band well within
    /// [`ORBIT_LAP_GRACE_SECS`]. Zeroing progress there loses three quarters of
    /// a revolution to one nudge, with nothing on screen to say why, and the
    /// player flies the same ring again wondering what they did wrong.
    #[test]
    fn a_correction_burn_on_the_ring_does_not_erase_the_lap() {
        use nova_ship::prelude::AutopilotPhase;

        let mut rig = LapRig::new();
        rig.fly(AutopilotPhase::Hold, 270);
        // Two seconds off the band, comfortably inside the grace.
        rig.fly(AutopilotPhase::Burn, 20);
        rig.fly(AutopilotPhase::Hold, TICKS_TO_CLOSE_A_LAP - 290);
        assert_eq!(
            rig.laps(),
            1.0,
            "the ring was flown once; a mid-lap correction is not a new lap"
        );
    }

    /// A ship that actually leaves the ring starts again. The grace is a
    /// correction budget, not an amnesty: past [`ORBIT_LAP_GRACE_SECS`] the
    /// partial lap is written off and only a return to `Hold` starts banking
    /// again, so `OnOrbitLap` keeps meaning a lap OF THE ORBIT.
    ///
    /// Reads as a real assertion only because
    /// `a_correction_burn_on_the_ring_does_not_erase_the_lap` proves this rig
    /// can report a lap at all.
    #[test]
    fn a_ship_that_leaves_the_ring_for_longer_than_the_grace_starts_its_lap_again() {
        use nova_ship::prelude::AutopilotPhase;

        let mut rig = LapRig::new();
        rig.fly(AutopilotPhase::Hold, 270);
        // Ten seconds away: twice the grace, so the ring counts as abandoned.
        rig.fly(AutopilotPhase::Align, 100);
        assert_eq!(rig.laps(), 0.0, "the departure landed inside the lap");
        rig.fly(AutopilotPhase::Hold, 300);
        assert_eq!(
            rig.laps(),
            0.0,
            "progress restarted at the ring, so five sixths of a lap is not one"
        );
    }

    /// Counting starts at the RING, not at the verb. An insertion curves
    /// around the well and can sweep a whole revolution before the ship is on
    /// any ring at all; banking that would complete a lap objective for flying
    /// TO the planetoid.
    #[test]
    fn the_approach_to_the_ring_banks_no_lap_progress() {
        use nova_ship::prelude::AutopilotPhase;

        let mut rig = LapRig::new();
        rig.fly(AutopilotPhase::Align, TICKS_TO_CLOSE_A_LAP);
        assert_eq!(
            rig.laps(),
            0.0,
            "a full revolution flown while still reaching for the ring is not a lap"
        );
        rig.fly(AutopilotPhase::Hold, TICKS_TO_CLOSE_A_LAP);
        assert_eq!(
            rig.laps(),
            1.0,
            "the first lap is the first one on the ring"
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
