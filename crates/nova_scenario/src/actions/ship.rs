//! Actions that retune a live scenario ship: speed cap, allegiance, and
//! per-verb controller flags.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::prelude::*;

/// Set or clear the manual [`FlightSpeedCap`] on a scenario ship by id
/// (the shakedown training governor releases at beacon 1; playtest round
/// 2 finding 3). Scoped-only lookup, same rule as DespawnScenarioObject.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetSpeedCapActionConfig {
    /// The `EntityId` of the scoped ship to cap.
    pub id: String,
    /// `Some(cap)` installs/updates the cap (u/s); `None` removes it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cap: Option<f32>,
}

impl EventAction<NovaEventWorld> for SetSpeedCapActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let cap = self.cap;
        debug!("SetSpeedCap: '{}' -> {:?}", id, cap);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = query
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetSpeedCap: no scoped ship with id '{}'", id);
                    return;
                };
                match cap {
                    Some(cap) => {
                        world.entity_mut(ship).insert(FlightSpeedCap(cap));
                    }
                    None => {
                        world.entity_mut(ship).remove::<FlightSpeedCap>();
                    }
                }
            });
        });
    }
}

/// Overwrite a scenario ship's [`Allegiance`] by id at runtime, flipping it
/// between Player/Enemy/Neutral. Allegiance is otherwise written only at spawn
/// and never changed; this is the missing primitive for "neutral until
/// provoked" encounters (a Neutral ship stays a bystander until a trigger fires
/// this action to make it Enemy). Scoped-only lookup, same rule as SetSpeedCap.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetAllegianceActionConfig {
    /// The `EntityId` of the scoped ship to re-align.
    pub id: String,
    /// The allegiance to overwrite the ship's `Allegiance` component with.
    pub allegiance: Allegiance,
}

impl EventAction<NovaEventWorld> for SetAllegianceActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let allegiance = self.allegiance;
        debug!("SetAllegiance: '{}' -> {:?}", id, allegiance);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut query = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = query
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetAllegiance: no scoped ship with id '{}'", id);
                    return;
                };
                world.entity_mut(ship).insert(allegiance);
            });
        });
    }
}

/// Enable or disable one flight verb on a scenario ship's controller section(s)
/// by id. Flight verbs (STOP/GOTO/ORBIT) are a capability the controller
/// grants; this flips a single verb at runtime - the shakedown withholds GOTO
/// until the first objective is complete. Scoped-only lookup, same rule as
/// SetSpeedCap; writes every controller section on the ship so the union the
/// input layer reads matches.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetControllerVerbActionConfig {
    /// The `EntityId` of the scoped ship whose controller sections to edit.
    pub id: String,
    /// The flight verb (STOP/GOTO/ORBIT/LOCK/RCS/POINT DEFENSE) to toggle.
    pub verb: FlightVerb,
    /// Whether the verb is enabled (true) or disabled (false).
    pub enabled: bool,
}

impl EventAction<NovaEventWorld> for SetControllerVerbActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let verb = self.verb;
        let enabled = self.enabled;
        debug!("SetControllerVerb: '{}' {:?} -> {}", id, verb, enabled);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut ships = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let Some(ship) = ships
                    .iter(world)
                    .find(|(_, entity_id)| entity_id.0 == id)
                    .map(|(entity, _)| entity)
                else {
                    warn!("SetControllerVerb: no scoped ship with id '{}'", id);
                    return;
                };

                // Every controller section on this ship (active or not - the
                // flag persists across (de)activation), so the union the hint
                // pass and observers read reflects the change.
                let mut controllers =
                    world.query_filtered::<(Entity, &ChildOf), With<ControllerSectionMarker>>();
                let targets: Vec<Entity> = controllers
                    .iter(world)
                    .filter(|(_, &ChildOf(parent))| parent == ship)
                    .map(|(entity, _)| entity)
                    .collect();
                if targets.is_empty() {
                    warn!("SetControllerVerb: ship '{}' has no controller section", id);
                    return;
                }
                for controller in targets {
                    // `WithheldVerbs` is absent on a fresh controller (all
                    // granted); a disable must materialize it first. An enable
                    // on an absent component is already a no-op (nothing is
                    // withheld), so only insert-if-absent when disabling.
                    if world.get::<WithheldVerbs>(controller).is_none() {
                        if !enabled {
                            world
                                .entity_mut(controller)
                                .insert(WithheldVerbs::default());
                        } else {
                            continue;
                        }
                    }
                    let mut withheld = world
                        .get_mut::<WithheldVerbs>(controller)
                        .expect("WithheldVerbs present: it was just inserted or already existed");
                    if enabled {
                        withheld.grant(verb);
                    } else {
                        withheld.withhold(verb);
                    }
                }
            });
        });
    }
}

/// Force a scenario ship's torpedo bays to launch at a named target - the
/// scripted counterpart of the AI's launch decision, for controller-less
/// emplacements fired by timers ("a battery shoots every N seconds"). Puts a
/// one-shot [`ScriptedTorpedoOrder`] on every torpedo bay of the ship; the
/// bay's own cooldown/ammo gates still time the actual launch, and the
/// scripted commit locks the ordnance so hostile point defense can engage
/// it. A missing target skips the launch entirely (no dumb-fire duds from a
/// mid-respawn window). Scoped-only lookup, same rule as SetSpeedCap.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForceTorpedoLaunchActionConfig {
    /// The `EntityId` of the scoped ship whose bays launch.
    pub id: String,
    /// The `EntityId` of the scoped ship the ordnance homes on.
    pub target: String,
}

impl EventAction<NovaEventWorld> for ForceTorpedoLaunchActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let target_id = self.target.clone();
        debug!("ForceTorpedoLaunch: '{}' -> '{}'", id, target_id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let mut ships = world.query_filtered::<(Entity, &EntityId), (
                    With<ScenarioScopedMarker>,
                    With<SpaceshipRootMarker>,
                )>();
                let mut find = |id: &str| {
                    ships
                        .iter(world)
                        .find(|(_, entity_id)| entity_id.0 == id)
                        .map(|(entity, _)| entity)
                };
                let Some(ship) = find(&id) else {
                    warn!("ForceTorpedoLaunch: no scoped ship with id '{}'", id);
                    return;
                };
                let Some(target) = find(&target_id) else {
                    warn!(
                        "ForceTorpedoLaunch: no scoped target ship with id '{}'; \
                         skipping the launch",
                        target_id
                    );
                    return;
                };

                let mut bays =
                    world.query_filtered::<(Entity, &ChildOf), With<TorpedoSectionMarker>>();
                let targets: Vec<Entity> = bays
                    .iter(world)
                    .filter(|(_, &ChildOf(parent))| parent == ship)
                    .map(|(entity, _)| entity)
                    .collect();
                if targets.is_empty() {
                    warn!("ForceTorpedoLaunch: ship '{}' has no torpedo bay", id);
                    return;
                }
                for bay in targets {
                    world
                        .entity_mut(bay)
                        .insert(ScriptedTorpedoOrder { target });
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SetControllerVerb flips exactly the addressed ship's controller verb,
    /// leaving other verbs on that controller and other ships untouched; and
    /// re-enabling restores it. If the action did not scope by ship id, the
    /// bystander ship's controller would flip too and this test would fail.
    #[test]
    fn set_controller_verb_flips_only_the_scoped_ship() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        // The target ship and a bystander ship, each a scoped root with a
        // controller section carrying no WithheldVerbs (all granted, the
        // production default - disabling must materialize the component).
        let player = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("player".to_string()),
            ))
            .id();
        let player_ctrl = world.spawn((ChildOf(player), ControllerSectionMarker)).id();
        let bystander = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("bystander".to_string()),
            ))
            .id();
        let bystander_ctrl = world
            .spawn((ChildOf(bystander), ControllerSectionMarker))
            .id();

        // Disable GOTO on the player only.
        let disable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        let pv = world.get::<WithheldVerbs>(player_ctrl).unwrap();
        assert!(
            !pv.granted(FlightVerb::Goto),
            "GOTO disabled on the addressed ship"
        );
        assert!(
            pv.granted(FlightVerb::Stop) && pv.granted(FlightVerb::Orbit),
            "other verbs on that controller untouched"
        );
        assert!(
            world
                .get::<WithheldVerbs>(bystander_ctrl)
                .is_none_or(|w| w.granted(FlightVerb::Goto)),
            "the bystander ship's controller is untouched (still grants GOTO)"
        );

        // Re-enable restores it.
        let enable = SetControllerVerbActionConfig {
            id: "player".to_string(),
            verb: FlightVerb::Goto,
            enabled: true,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        enable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        assert!(
            world
                .get::<WithheldVerbs>(player_ctrl)
                .unwrap()
                .granted(FlightVerb::Goto),
            "GOTO re-enabled on the addressed ship"
        );
    }

    /// SetControllerVerb writes EVERY controller section on the ship, so the
    /// union the input layer reads (verb available if ANY live controller
    /// grants it) reflects the change no matter which controller it samples.
    #[test]
    fn set_controller_verb_writes_all_controllers_on_the_ship() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let ship = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("twin".to_string()),
            ))
            .id();
        let ctrl_a = world.spawn((ChildOf(ship), ControllerSectionMarker)).id();
        let ctrl_b = world.spawn((ChildOf(ship), ControllerSectionMarker)).id();

        let disable = SetControllerVerbActionConfig {
            id: "twin".to_string(),
            verb: FlightVerb::Stop,
            enabled: false,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        disable.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            !world
                .get::<WithheldVerbs>(ctrl_a)
                .unwrap()
                .granted(FlightVerb::Stop),
            "first controller written"
        );
        assert!(
            !world
                .get::<WithheldVerbs>(ctrl_b)
                .unwrap()
                .granted(FlightVerb::Stop),
            "second controller written too"
        );
    }

    /// A scenario authors `SetAllegiance` in RON to wake a neutral ship, so the
    /// whole action must round-trip through serde with its id and allegiance.
    #[cfg(feature = "serde")]
    #[test]
    fn set_allegiance_action_round_trips_through_ron() {
        let action = EventActionConfig::SetAllegiance(SetAllegianceActionConfig {
            id: "x".into(),
            allegiance: Allegiance::Enemy,
        });
        let ron = ron::to_string(&action).expect("serialize");
        let back: EventActionConfig = ron::from_str(&ron).expect("deserialize");
        match back {
            EventActionConfig::SetAllegiance(config) => {
                assert_eq!(config.id, "x");
                assert_eq!(config.allegiance, Allegiance::Enemy);
            }
            other => panic!("expected SetAllegiance, got {other:?}"),
        }
    }

    /// SetAllegiance overwrites the addressed ship's `Allegiance` at runtime:
    /// a spawned-Neutral ship becomes Enemy (neutral-until-provoked). Without
    /// the apply path the component would stay Neutral, failing this test. An
    /// unknown id warns and does not panic.
    #[test]
    fn set_allegiance_flips_the_scoped_ship() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let ship = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("spaceship_1".to_string()),
                Allegiance::Neutral,
            ))
            .id();

        // Provoke: flip the neutral ship to Enemy.
        let flip = SetAllegianceActionConfig {
            id: "spaceship_1".to_string(),
            allegiance: Allegiance::Enemy,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        flip.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world.get::<Allegiance>(ship).copied(),
            Some(Allegiance::Enemy),
            "the scoped ship's allegiance is now Enemy"
        );

        // A bad id warns and is a no-op (no panic, ship unchanged).
        let miss = SetAllegianceActionConfig {
            id: "nope".to_string(),
            allegiance: Allegiance::Player,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        miss.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world.get::<Allegiance>(ship).copied(),
            Some(Allegiance::Enemy),
            "an unknown id does not touch any ship"
        );
    }

    /// ForceTorpedoLaunch puts a one-shot order on exactly the addressed
    /// ship's bays, locked to the resolved target entity; a missing target
    /// skips the launch entirely (no dumb-fire duds while the target is
    /// mid-respawn), and a bystander's bay is never touched.
    #[test]
    fn force_torpedo_launch_orders_only_the_scoped_bays() {
        use nova_events::prelude::EventWorld;

        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();

        let battery = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("battery".to_string()),
            ))
            .id();
        let bay = world.spawn((ChildOf(battery), TorpedoSectionMarker)).id();
        let bystander = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("bystander".to_string()),
            ))
            .id();
        let bystander_bay = world.spawn((ChildOf(bystander), TorpedoSectionMarker)).id();
        let target = world
            .spawn((
                ScenarioScopedMarker,
                SpaceshipRootMarker,
                EntityId::new("prey".to_string()),
            ))
            .id();

        let fire = ForceTorpedoLaunchActionConfig {
            id: "battery".to_string(),
            target: "prey".to_string(),
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        fire.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(
            world
                .get::<ScriptedTorpedoOrder>(bay)
                .map(|order| order.target),
            Some(target),
            "the battery's bay carries the order, locked to the resolved target"
        );
        assert!(
            world.get::<ScriptedTorpedoOrder>(bystander_bay).is_none(),
            "a bystander's bay is never ordered"
        );

        // A missing target skips the launch instead of ordering a dud.
        world.entity_mut(bay).remove::<ScriptedTorpedoOrder>();
        let miss = ForceTorpedoLaunchActionConfig {
            id: "battery".to_string(),
            target: "gone".to_string(),
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        miss.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert!(
            world.get::<ScriptedTorpedoOrder>(bay).is_none(),
            "no target, no order - the launch is skipped"
        );
    }
}
