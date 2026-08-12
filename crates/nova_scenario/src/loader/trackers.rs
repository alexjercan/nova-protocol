//! Orbit-lifecycle and weapon-lock events derived from live ship state.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

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
                EntityTypeName::new("spaceship"),
                TravelLock(None),
                CombatLock(None),
            ))
            .id();
        app.world_mut().spawn((
            SpaceshipRootMarker,
            EntityId::new("ai"),
            EntityTypeName::new("spaceship"),
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
