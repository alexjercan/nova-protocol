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
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetSpeedCapActionConfig {
    /// The `EntityId` of the scoped ship to cap.
    #[reflect(@Names::Object)]
    pub id: String,
    /// `Some(cap)` installs/updates the cap; `None` removes it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cap: Option<MetersPerSecond>,
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
                        // Engine boundary: the flight code measures against an
                        // avian velocity, so the authored cap crosses here.
                        world
                            .entity_mut(ship)
                            .insert(FlightSpeedCap(cap.to_engine()));
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
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetAllegianceActionConfig {
    /// The `EntityId` of the scoped ship to re-align.
    #[reflect(@Names::Object)]
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
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetControllerVerbActionConfig {
    /// The `EntityId` of the scoped ship whose controller sections to edit.
    #[reflect(@Names::Object)]
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
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForceTorpedoLaunchActionConfig {
    /// The `EntityId` of the scoped ship whose bays launch.
    #[reflect(@Names::Object)]
    pub id: String,
    /// The `EntityId` of the scoped ship the ordnance homes on.
    #[reflect(@Names::Object)]
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

    /// A ship with two weapon sections and a bystander ship carrying the same
    /// section id, which is what every shipped scenario looks like: an id
    /// addresses a section OF a ship, never one in the field.
    fn two_armed_ships() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        let mut ship = |id: &str| {
            world
                .spawn((
                    ScenarioScopedMarker,
                    SpaceshipRootMarker,
                    EntityId::new(id.to_string()),
                ))
                .id()
        };
        let player = ship("player");
        let raider = ship("raider");
        let mut turret = |ship: Entity, id: &str, rounds: u32| {
            world
                .spawn((
                    ChildOf(ship),
                    SectionMarker,
                    EntityId::new(id.to_string()),
                    SectionAmmo {
                        rounds,
                        capacity: 10,
                    },
                ))
                .id()
        };
        let port = turret(player, "turret_port", 2);
        let dorsal = turret(player, "turret_dorsal", 3);
        turret(raider, "turret_port", 1);
        (world, port, dorsal, raider)
    }

    /// SetInfiniteAmmo strips the magazine of the addressed ship alone, and
    /// switching it back restores the authored capacity rather than what was
    /// left in it.
    #[test]
    fn set_infinite_ammo_suspends_and_restores_the_scoped_ships_magazines() {
        use nova_events::prelude::EventWorld;

        let (mut world, port, dorsal, raider) = two_armed_ships();
        let raider_turret = live_ship_sections(&mut world, raider)[0];

        let run = |world: &mut World, enabled| {
            let action = SetInfiniteAmmoActionConfig {
                id: "player".to_string(),
                enabled,
            };
            let mut event_world = world.resource_mut::<NovaEventWorld>();
            action.action(&mut event_world, &GameEventInfo::default());
            NovaEventWorld::state_to_world_system(world);
        };

        run(&mut world, true);
        assert!(
            world.get::<SectionAmmo>(port).is_none(),
            "port is unlimited"
        );
        assert!(
            world.get::<SectionAmmo>(dorsal).is_none(),
            "dorsal is unlimited"
        );
        assert!(
            world.get::<SectionAmmo>(raider_turret).is_some(),
            "the bystander ship keeps its magazine"
        );

        run(&mut world, false);
        let restored = world
            .get::<SectionAmmo>(port)
            .expect("the magazine is back");
        assert_eq!(
            (restored.rounds, restored.capacity),
            (10, 10),
            "a restored magazine is full, not what was left in it"
        );
    }

    /// RefillAmmo with a section id fills that section of that ship and leaves
    /// every other magazine where it was.
    #[test]
    fn refill_ammo_fills_one_named_section_of_the_scoped_ship() {
        use nova_events::prelude::EventWorld;

        let (mut world, port, dorsal, raider) = two_armed_ships();
        let raider_turret = live_ship_sections(&mut world, raider)[0];

        let action = RefillAmmoActionConfig {
            id: "player".to_string(),
            section: Some("turret_port".to_string()),
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);

        assert_eq!(world.get::<SectionAmmo>(port).unwrap().rounds, 10);
        assert_eq!(
            world.get::<SectionAmmo>(dorsal).unwrap().rounds,
            3,
            "the other section of the same ship is untouched"
        );
        assert_eq!(
            world.get::<SectionAmmo>(raider_turret).unwrap().rounds,
            1,
            "the same section id on another ship is untouched"
        );
    }
}

/// Switch unlimited ammunition on or off for a scenario ship's weapon sections
/// by id.
///
/// This replaces the old `infinite_ammo` flag on the player controller config,
/// which was authored once at spawn and honored only under the `debug` feature.
/// An action instead of a flag, for three reasons: it works on a LIVE ship, it
/// works in a shipped build (the command shell arms and marks the run, which is
/// what the `cfg` was standing in for), and content that wants to grant it -
/// a training scenario, a story beat - can now ask for it in the same
/// vocabulary as everything else.
///
/// Switching it ON strips [`SectionAmmo`] and [`SectionReload`], recording what
/// was there in [`SuspendedSectionAmmo`]; switching it OFF restores the
/// authored capacity, full. See [`SuspendedSectionAmmo`] for why full.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetInfiniteAmmoActionConfig {
    /// The `EntityId` of the scoped ship whose weapons to change.
    #[reflect(@Names::Object)]
    pub id: String,
    /// Whether the ship's weapons fire without limit.
    pub enabled: bool,
}

impl EventAction<NovaEventWorld> for SetInfiniteAmmoActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let enabled = self.enabled;
        debug!("SetInfiniteAmmo: '{}' -> {}", id, enabled);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scoped_ship(world, &id) else {
                    warn!("SetInfiniteAmmo: no scoped ship with id '{}'", id);
                    return;
                };
                for section in live_ship_sections(world, ship) {
                    apply_infinite_ammo(world, section, enabled);
                }
            });
        });
    }
}

/// Refill every finite magazine on a scenario ship by id, or one section of it.
///
/// The bounded, honest half of the ammunition cheat: it restores what a
/// magazine can hold rather than removing the magazine. Authored content can
/// use it for a resupply beat without the scenario granting unlimited fire.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RefillAmmoActionConfig {
    /// The `EntityId` of the scoped ship to resupply.
    #[reflect(@Names::Object)]
    pub id: String,
    /// One section's authored id, or `None` for every weapon on the ship.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub section: Option<String>,
}

impl EventAction<NovaEventWorld> for RefillAmmoActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.id.clone();
        let section_id = self.section.clone();
        debug!("RefillAmmo: '{}' section {:?}", id, section_id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scoped_ship(world, &id) else {
                    warn!("RefillAmmo: no scoped ship with id '{}'", id);
                    return;
                };
                let mut refilled = 0usize;
                for section in live_ship_sections(world, ship) {
                    let wanted_elsewhere = section_id.as_ref().is_some_and(|wanted| {
                        world
                            .entity(section)
                            .get::<EntityId>()
                            .is_none_or(|id| &id.0 != wanted)
                    });
                    if wanted_elsewhere {
                        continue;
                    }
                    if refill_section(world, section) {
                        refilled += 1;
                    }
                }
                if refilled == 0 {
                    warn!("RefillAmmo: '{}' has no finite magazine to refill", id);
                }
            });
        });
    }
}

/// The scoped scenario ship with this `EntityId`, if one is live.
fn scoped_ship(world: &mut World, id: &str) -> Option<Entity> {
    let mut query = world.query_filtered::<(Entity, &EntityId), (
        With<ScenarioScopedMarker>,
        With<SpaceshipRootMarker>,
    )>();
    query
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// Every section entity of one ship.
///
/// Public because the command shell's `ammo` cheats run the same operation
/// against a live ship and have to report what they actually touched, which a
/// deferred action cannot tell them.
pub fn live_ship_sections(world: &mut World, ship: Entity) -> Vec<Entity> {
    let mut query = world.query_filtered::<(Entity, &ChildOf), With<SectionMarker>>();
    query
        .iter(world)
        .filter(|(_, child_of)| child_of.parent() == ship)
        .map(|(entity, _)| entity)
        .collect()
}

/// Strip or restore one section's magazine. See [`SuspendedSectionAmmo`].
///
/// `false` when the section was already in the asked-for state, so a caller
/// that has to report what it touched does not have to diff the components
/// itself.
pub fn apply_infinite_ammo(world: &mut World, section: Entity, enabled: bool) -> bool {
    let mut entity = world.entity_mut(section);
    if enabled {
        let Some(ammo) = entity.get::<SectionAmmo>().copied() else {
            // Either already unlimited or not a weapon; both are already the
            // state the caller asked for.
            return false;
        };
        let reload = entity
            .get::<SectionReload>()
            .map(|reload| SectionReloadConfig {
                delay: reload.delay,
                amount: reload.amount,
            });
        entity.insert(SuspendedSectionAmmo {
            capacity: ammo.capacity,
            reload,
        });
        entity.remove::<SectionAmmo>();
        entity.remove::<SectionReload>();
    } else {
        let Some(suspended) = entity.get::<SuspendedSectionAmmo>().copied() else {
            return false;
        };
        entity.insert(SectionAmmo::new(suspended.capacity));
        if let Some(reload) = suspended.reload {
            entity.insert(SectionReload::from_config(reload));
        }
        entity.remove::<SuspendedSectionAmmo>();
    }
    true
}

/// Fill one section's magazine, if it has one. `false` when the section has no
/// finite magazine to fill.
pub fn refill_section(world: &mut World, section: Entity) -> bool {
    let mut entity = world.entity_mut(section);
    let Some(mut ammo) = entity.get_mut::<SectionAmmo>() else {
        return false;
    };
    ammo.rounds = ammo.capacity;
    // A magazine that was refilled is not mid-reload any more.
    if let Some(mut reload) = entity.get_mut::<SectionReload>() {
        reload.elapsed = 0.0;
    }
    true
}
