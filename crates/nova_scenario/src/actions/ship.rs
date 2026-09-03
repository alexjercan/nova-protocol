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

/// Fly a scripted ship to an authored mark with the real autopilot.
///
/// The whole point is that it FLIES: `GotoPos` plans the leg, the drives burn,
/// the hull swings, and the arrival is the same brake-and-settle every GOTO
/// runs. A cinematic that set the transform would slide a ship through a
/// planetoid and read as a cutscene rather than as something happening in the
/// world.
///
/// `arrival_standoff` exists because the global 500 m is far too coarse to
/// stage a shot with: a warship that has to sit under a carrier's bore needs
/// to stop where the author said. It is installed for the life of the order
/// and taken back off when the order retires, so it never silently retunes
/// every later GOTO the hull flies.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveShipToActionConfig {
    /// The key this order's completion is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped `None`-controller ship to fly.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The mark to fly to, world coordinates in meters.
    pub position: Meters3,
    /// How far short of the mark to come to rest. `None` flies the ship's own
    /// standoff (the global 500 m unless the spawn overrode it).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub arrival_standoff: Option<Meters>,
}

impl EventAction<NovaEventWorld> for MoveShipToActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let order = self.order.clone();
        let id = self.ship.clone();
        let position = self.position;
        let standoff = self.arrival_standoff;
        debug!("MoveShipTo: '{}' order '{}' -> {:?}", id, order, position);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "MoveShipTo") else {
                    return;
                };
                clear_helm_order(world, ship);

                let mut entity = world.entity_mut(ship);
                if let Some(standoff) = standoff {
                    // Engine boundary: the arrival rule measures against an
                    // avian position, so the authored meters cross here.
                    let previous = entity.get::<FlightArrivalStandoff>().map(|s| **s);
                    entity.insert((
                        SuspendedArrivalStandoff(previous),
                        FlightArrivalStandoff(standoff.to_engine()),
                    ));
                }
                entity.insert((
                    ScriptedHelmOrder::new(order, ShipOrderKind::Move),
                    Autopilot::engage(AutopilotAction::GotoPos {
                        position: position.to_engine(),
                    }),
                ));
            });
        });
    }
}

/// Turn a scripted ship's whole hull onto an authored bearing, without moving
/// it.
///
/// Rotation only - no autopilot, so no drive ever burns for translation. The
/// order reports complete once the aim is inside its tolerance and settled,
/// then HOLDS that facing until another helm order replaces it. The hold is
/// what makes a spinal weapon usable from a script: several railguns can run
/// their charges while the bore stays on the target, and the shot leaves down
/// the line the hull is still holding.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForceAlignActionConfig {
    /// The key this order's completion is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped `None`-controller ship to turn.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The world position to put under the bore, in meters.
    pub look_at: Meters3,
    /// How close the aim must come before the order reports complete, degrees.
    /// This is also what "settled" is measured against, so a tight tolerance
    /// asks for a genuinely steady hull and a coarse one accepts a drift.
    pub tolerance_degrees: f32,
}

impl EventAction<NovaEventWorld> for ForceAlignActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let order = self.order.clone();
        let id = self.ship.clone();
        let look_at = self.look_at;
        let tolerance_degrees = self.tolerance_degrees;
        debug!(
            "ForceAlign: '{}' order '{}' -> {:?} within {} deg",
            id, order, look_at, tolerance_degrees
        );
        if !(tolerance_degrees.is_finite() && tolerance_degrees >= 0.0) {
            error!(
                "ForceAlign: order '{}' has a nonsensical tolerance of {} degrees; \
                 the order would never complete",
                order, tolerance_degrees
            );
            return;
        }

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "ForceAlign") else {
                    return;
                };
                clear_helm_order(world, ship);
                world.entity_mut(ship).insert((
                    ScriptedHelmOrder::new(order, ShipOrderKind::Align),
                    ScriptedAlign {
                        // Engine boundary: the bearing is compared against an
                        // avian position every tick.
                        look_at: look_at.to_engine(),
                        tolerance: tolerance_degrees.to_radians(),
                    },
                ));
            });
        });
    }
}

/// Bring a scripted ship to rest with the real STOP maneuver.
///
/// The same flip-retrograde-and-burn the player's X key runs, so a ship that
/// arrives somewhere and stops does it by spending fuel and time, visibly.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopShipActionConfig {
    /// The key this order's completion is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped `None`-controller ship to stop.
    #[reflect(@Names::Object)]
    pub ship: String,
}

impl EventAction<NovaEventWorld> for StopShipActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let order = self.order.clone();
        let id = self.ship.clone();
        debug!("StopShip: '{}' order '{}'", id, order);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "StopShip") else {
                    return;
                };
                clear_helm_order(world, ship);
                world.entity_mut(ship).insert((
                    ScriptedHelmOrder::new(order, ShipOrderKind::Stop),
                    Autopilot::engage(AutopilotAction::Stop),
                ));
            });
        });
    }
}

/// Release a scripted ship's helm and let it drift.
///
/// The counterpart to the three orders above: whatever the ship was told, it is
/// no longer being told it. The hull keeps its velocity - this is space, and
/// the point of clearing an order is usually to let a ship coast out of frame.
/// Emits NO completion event: a cleared order did not finish, and a beat
/// waiting on it must not run.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClearShipOrderActionConfig {
    /// The `EntityId` of the scoped ship to release.
    #[reflect(@Names::Object)]
    pub ship: String,
}

impl EventAction<NovaEventWorld> for ClearShipOrderActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        debug!("ClearShipOrder: '{}'", id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "ClearShipOrder") else {
                    return;
                };
                clear_helm_order(world, ship);
            });
        });
    }
}

/// Fire one named railgun section of a scripted ship.
///
/// No target, and no steering: a railgun does not traverse, so the shot leaves
/// down whatever line the hull holds when the charge completes. Putting that
/// line on something is [`ForceAlignActionConfig`]'s job. Everything else is
/// the gun's own behavior - the authored charge, the magazine, the reload, the
/// recoil through the hull, the slug, the sound and the flash.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForceRailgunFireActionConfig {
    /// The `EntityId` of the scoped `None`-controller ship that fires.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The authored section id of the railgun to fire.
    #[reflect(@Names::Section)]
    pub section: String,
}

impl EventAction<NovaEventWorld> for ForceRailgunFireActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let section_id = self.section.clone();
        debug!("ForceRailgunFire: '{}' section '{}'", id, section_id);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "ForceRailgunFire") else {
                    return;
                };
                let Some(section) = ship_section::<RailgunSectionMarker>(
                    world,
                    ship,
                    &id,
                    &section_id,
                    "ForceRailgunFire",
                    "railgun",
                ) else {
                    return;
                };
                world.entity_mut(section).insert(ScriptedRailgunOrder);
            });
        });
    }
}

/// Launch one named torpedo bay of a scripted ship at one named target.
///
/// ONE bay, addressed by its section id. The broad all-bays action this
/// replaces could not stage a set piece: a warship with six bays down its
/// flanks emptied all six on one trigger, and an author who wanted a second
/// salvo later had nothing left to fire.
///
/// The bay's own gates still time the launch, and the scripted commit locks
/// the ordnance to the target the same way the AI and player commits do -
/// which is also what makes the torpedo visible to hostile point defense. A
/// missing target skips the launch rather than dumb-firing a dud.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ForceTorpedoFireActionConfig {
    /// The `EntityId` of the scoped `None`-controller ship that launches.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The authored section id of the bay to launch.
    #[reflect(@Names::Section)]
    pub section: String,
    /// The `EntityId` of the scoped ship the ordnance homes on.
    #[reflect(@Names::Object)]
    pub target: String,
}

impl EventAction<NovaEventWorld> for ForceTorpedoFireActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let section_id = self.section.clone();
        let target_id = self.target.clone();
        debug!(
            "ForceTorpedoFire: '{}' section '{}' -> '{}'",
            id, section_id, target_id
        );

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = scripted_ship(world, &id, "ForceTorpedoFire") else {
                    return;
                };
                let Some(target) = scoped_ship(world, &target_id) else {
                    warn!(
                        "ForceTorpedoFire: no scoped target ship with id '{}'; \
                         skipping the launch",
                        target_id
                    );
                    return;
                };
                let Some(section) = ship_section::<TorpedoSectionMarker>(
                    world,
                    ship,
                    &id,
                    &section_id,
                    "ForceTorpedoFire",
                    "torpedo bay",
                ) else {
                    return;
                };
                world
                    .entity_mut(section)
                    .insert(ScriptedTorpedoOrder { target });
            });
        });
    }
}

/// The scoped ship a SCRIPTED action may drive, or `None` with a logged refusal.
///
/// A scripted action owns its actor outright, so it accepts only a ship nobody
/// else drives. Stealing the helm from a player would fight the input layer,
/// which drops the autopilot on any flight input; stealing it from the AI would
/// lose, because the AI rewrites the seams every frame. Both used to happen
/// silently - the old all-bays torpedo action documented the race rather than
/// refusing it - so this is an ERROR, loud enough to find in a log.
///
/// The test is the DRIVER, never the allegiance: the chapter-one warship reads
/// Enemy and is still entirely scripted. It is also not the presence of
/// controller SECTIONS, which a scripted ship needs - the autopilot cannot turn
/// a hull with no live flight computer.
fn scripted_ship(world: &mut World, id: &str, what: &str) -> Option<Entity> {
    let ship = scoped_ship(world, id).or_else(|| {
        warn!("{what}: no scoped ship with id '{id}'");
        None
    })?;
    let entity = world.entity(ship);
    if entity.contains::<PlayerSpaceshipMarker>() {
        error!("{what}: ship '{id}' is player-driven; a scripted action cannot take its helm");
        return None;
    }
    if entity.contains::<AISpaceshipMarker>() {
        error!(
            "{what}: ship '{id}' is AI-driven; a scripted action cannot take its helm \
             (author `SpaceshipController::None`, or `non_combatant` for an armed \
             ship that flies itself and never shoots)"
        );
        return None;
    }
    Some(ship)
}

/// One named section of one ship, refusing a section of the wrong class.
///
/// The class check is the whole reason this is not a plain id lookup. Naming a
/// hull block where a railgun was meant is an authoring slip, and firing some
/// other mount instead - or nothing at all - is the kind of failure a set piece
/// hides until someone watches it play.
fn ship_section<M: Component>(
    world: &mut World,
    ship: Entity,
    ship_id: &str,
    section_id: &str,
    what: &str,
    class: &str,
) -> Option<Entity> {
    let sections = live_ship_sections(world, ship);
    let named: Vec<Entity> = sections
        .into_iter()
        .filter(|&section| {
            world
                .entity(section)
                .get::<EntityId>()
                .is_some_and(|id| id.0 == section_id)
        })
        .collect();
    if named.is_empty() {
        warn!("{what}: ship '{ship_id}' has no section with id '{section_id}'");
        return None;
    }
    let Some(&section) = named
        .iter()
        .find(|&&section| world.entity(section).contains::<M>())
    else {
        warn!(
            "{what}: section '{section_id}' of ship '{ship_id}' is not a {class}; \
             refusing rather than firing another mount"
        );
        return None;
    };
    Some(section)
}

/// Retire whatever scripted helm order a ship is holding, leaving it adrift.
///
/// Move, align and stop are one mutually exclusive family, so every install
/// runs this first. It reports nothing: a replaced or cleared order did not
/// complete, and the scenario layer's completion tracker only ever sees orders
/// that are still installed.
///
/// Also puts back the arrival standoff a scripted move displaced, so the
/// cinematic's tight staging tolerance does not outlive the cinematic.
fn clear_helm_order(world: &mut World, ship: Entity) {
    let mut entity = world.entity_mut(ship);
    if let Some(SuspendedArrivalStandoff(previous)) =
        entity.get::<SuspendedArrivalStandoff>().copied()
    {
        match previous {
            Some(standoff) => {
                entity.insert(FlightArrivalStandoff(standoff));
            }
            None => {
                entity.remove::<FlightArrivalStandoff>();
            }
        }
        entity.remove::<SuspendedArrivalStandoff>();
    }
    entity.remove::<ScriptedHelmOrder>();
    entity.remove::<ScriptedAlign>();
    entity.remove::<ScriptedAlignSettled>();
    entity.remove::<Autopilot>();
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

    /// A scripted ship, a bystander that carries the SAME section ids, and a
    /// target - the shape every set piece has. Both hulls are
    /// `None`-controller: neither driver marker is present, so both are
    /// scriptable and a refusal in a test below is about what that test added.
    fn a_scripted_ship_and_a_bystander() -> ScriptedFixture {
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
        let warship = ship("warship");
        let bystander = ship("bystander");
        let target = ship("prey");

        let mut section = |ship: Entity, id: &str, marker: SectionClassMarker| {
            let mut entity =
                world.spawn((ChildOf(ship), SectionMarker, EntityId::new(id.to_string())));
            match marker {
                SectionClassMarker::Railgun => entity.insert(RailgunSectionMarker),
                SectionClassMarker::Torpedo => entity.insert(TorpedoSectionMarker),
                SectionClassMarker::Hull => &mut entity,
            };
            entity.id()
        };
        let railgun = section(warship, "spinal", SectionClassMarker::Railgun);
        let bay = section(warship, "bay_port", SectionClassMarker::Torpedo);
        let block = section(warship, "nose_block", SectionClassMarker::Hull);
        section(bystander, "spinal", SectionClassMarker::Railgun);
        let bystander_bay = section(bystander, "bay_port", SectionClassMarker::Torpedo);

        ScriptedFixture {
            world,
            warship,
            target,
            railgun,
            bay,
            block,
            bystander_bay,
        }
    }

    /// Which mount a fixture section is, so the fixture reads as a section
    /// list rather than three near-identical spawn blocks.
    enum SectionClassMarker {
        Railgun,
        Torpedo,
        Hull,
    }

    /// The fixture [`a_scripted_ship_and_a_bystander`] builds.
    struct ScriptedFixture {
        world: World,
        warship: Entity,
        target: Entity,
        railgun: Entity,
        bay: Entity,
        block: Entity,
        bystander_bay: Entity,
    }

    /// Run one action against a fixture world and flush it into the world.
    fn run(world: &mut World, action: &dyn EventAction<NovaEventWorld>) {
        use nova_events::prelude::EventWorld;

        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(world);
    }

    /// ForceTorpedoFire orders EXACTLY the named bay of the named ship, locked
    /// to the resolved target entity. The same section id on a bystander hull
    /// is never touched - which is the whole difference from the all-bays
    /// action this replaced.
    #[test]
    fn force_torpedo_fire_orders_exactly_the_named_bay() {
        let ScriptedFixture {
            mut world,
            target,
            bay,
            bystander_bay,
            ..
        } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &ForceTorpedoFireActionConfig {
                ship: "warship".to_string(),
                section: "bay_port".to_string(),
                target: "prey".to_string(),
            },
        );

        assert_eq!(
            world.get::<ScriptedTorpedoOrder>(bay).map(|o| o.target),
            Some(target),
            "the named bay carries the order, locked to the resolved target"
        );
        assert!(
            world.get::<ScriptedTorpedoOrder>(bystander_bay).is_none(),
            "the same section id on another hull is never ordered"
        );
    }

    /// A target that is not in the scenario skips the launch instead of
    /// dumb-firing a dud, and a section id the hull does not carry orders
    /// nothing at all.
    #[test]
    fn a_missing_target_or_section_fires_nothing() {
        let ScriptedFixture { mut world, bay, .. } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &ForceTorpedoFireActionConfig {
                ship: "warship".to_string(),
                section: "bay_port".to_string(),
                target: "gone".to_string(),
            },
        );
        assert!(
            world.get::<ScriptedTorpedoOrder>(bay).is_none(),
            "no target, no order - the launch is skipped"
        );

        run(
            &mut world,
            &ForceTorpedoFireActionConfig {
                ship: "warship".to_string(),
                section: "bay_starboard".to_string(),
                target: "prey".to_string(),
            },
        );
        assert!(
            world.get::<ScriptedTorpedoOrder>(bay).is_none(),
            "an unknown section id does not fall back to another bay"
        );
    }

    /// A section id that names a hull block, not a mount, is refused: the
    /// action fires nothing rather than picking some other section.
    #[test]
    fn a_section_of_the_wrong_class_is_refused() {
        let ScriptedFixture {
            mut world,
            railgun,
            block,
            ..
        } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &ForceRailgunFireActionConfig {
                ship: "warship".to_string(),
                section: "nose_block".to_string(),
            },
        );

        assert!(
            world.get::<ScriptedRailgunOrder>(block).is_none(),
            "a hull block is not a railgun and takes no order"
        );
        assert!(
            world.get::<ScriptedRailgunOrder>(railgun).is_none(),
            "and the ship's actual railgun is not fired in its place"
        );
    }

    /// ForceRailgunFire puts a one-shot order on the named gun alone.
    #[test]
    fn force_railgun_fire_orders_exactly_the_named_gun() {
        let ScriptedFixture {
            mut world,
            railgun,
            bay,
            ..
        } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &ForceRailgunFireActionConfig {
                ship: "warship".to_string(),
                section: "spinal".to_string(),
            },
        );

        assert!(world.get::<ScriptedRailgunOrder>(railgun).is_some());
        assert!(
            world.get::<ScriptedRailgunOrder>(bay).is_none(),
            "the ship's other mounts stay cold"
        );
    }

    /// Every scripted action refuses a ship somebody else drives, whether the
    /// driver is the player or the AI. Taking the helm would either fight the
    /// input layer or lose to a controller that rewrites its seams every
    /// frame, so the action does nothing and says so.
    #[test]
    fn a_driven_ship_refuses_every_scripted_action() {
        for driven in [0, 1] {
            let ScriptedFixture {
                mut world,
                warship,
                railgun,
                bay,
                ..
            } = a_scripted_ship_and_a_bystander();
            if driven == 0 {
                world.entity_mut(warship).insert(PlayerSpaceshipMarker);
            } else {
                world.entity_mut(warship).insert(AISpaceshipMarker);
            }

            run(
                &mut world,
                &MoveShipToActionConfig {
                    order: "approach".to_string(),
                    ship: "warship".to_string(),
                    position: Meters3::new(0.0, 0.0, 1000.0),
                    arrival_standoff: None,
                },
            );
            run(
                &mut world,
                &ForceAlignActionConfig {
                    order: "aim".to_string(),
                    ship: "warship".to_string(),
                    look_at: Meters3::ZERO,
                    tolerance_degrees: 2.0,
                },
            );
            run(
                &mut world,
                &StopShipActionConfig {
                    order: "halt".to_string(),
                    ship: "warship".to_string(),
                },
            );
            run(
                &mut world,
                &ForceRailgunFireActionConfig {
                    ship: "warship".to_string(),
                    section: "spinal".to_string(),
                },
            );
            run(
                &mut world,
                &ForceTorpedoFireActionConfig {
                    ship: "warship".to_string(),
                    section: "bay_port".to_string(),
                    target: "prey".to_string(),
                },
            );

            let entity = world.entity(warship);
            assert!(!entity.contains::<ScriptedHelmOrder>(), "no helm order");
            assert!(!entity.contains::<Autopilot>(), "no autopilot");
            assert!(!entity.contains::<ScriptedAlign>(), "no held alignment");
            assert!(world.get::<ScriptedRailgunOrder>(railgun).is_none());
            assert!(world.get::<ScriptedTorpedoOrder>(bay).is_none());
        }
    }

    /// The three helm orders are one mutually exclusive family: installing one
    /// takes the last one off, component and autopilot together, so a ship
    /// under a new order carries no trace of the order before it.
    #[test]
    fn a_new_helm_order_replaces_the_one_before_it() {
        let ScriptedFixture {
            mut world, warship, ..
        } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &MoveShipToActionConfig {
                order: "approach".to_string(),
                ship: "warship".to_string(),
                position: Meters3::new(0.0, 0.0, 1000.0),
                arrival_standoff: Some(Meters(40.0)),
            },
        );
        assert!(matches!(
            world.get::<ScriptedHelmOrder>(warship).map(|o| o.kind),
            Some(ShipOrderKind::Move)
        ));
        assert!(world.get::<Autopilot>(warship).is_some());

        run(
            &mut world,
            &ForceAlignActionConfig {
                order: "aim".to_string(),
                ship: "warship".to_string(),
                look_at: Meters3::ZERO,
                tolerance_degrees: 2.0,
            },
        );

        let order = world
            .get::<ScriptedHelmOrder>(warship)
            .expect("the alignment order is installed");
        assert_eq!(order.key, "aim");
        assert_eq!(order.kind, ShipOrderKind::Align);
        assert!(
            world.get::<Autopilot>(warship).is_none(),
            "the move's autopilot is gone: an alignment does not translate"
        );
        assert!(
            world.get::<FlightArrivalStandoff>(warship).is_none(),
            "and the move's staging standoff does not outlive the move"
        );
    }

    /// ClearShipOrder takes the helm order off and puts the ship's own
    /// arrival standoff back, so a cinematic's tight staging tolerance does
    /// not retune every GOTO the hull flies afterwards.
    #[test]
    fn clearing_an_order_restores_the_ships_own_standoff() {
        let ScriptedFixture {
            mut world, warship, ..
        } = a_scripted_ship_and_a_bystander();
        // Engine boundary: the ship's authored override, already converted.
        world
            .entity_mut(warship)
            .insert(FlightArrivalStandoff(Meters(80.0).to_engine()));

        run(
            &mut world,
            &MoveShipToActionConfig {
                order: "approach".to_string(),
                ship: "warship".to_string(),
                position: Meters3::new(0.0, 0.0, 1000.0),
                arrival_standoff: Some(Meters(5.0)),
            },
        );
        assert_eq!(
            world.get::<FlightArrivalStandoff>(warship).map(|s| **s),
            Some(Meters(5.0).to_engine()),
            "the order flies the authored staging standoff"
        );

        run(
            &mut world,
            &ClearShipOrderActionConfig {
                ship: "warship".to_string(),
            },
        );

        let entity = world.entity(warship);
        assert!(
            !entity.contains::<ScriptedHelmOrder>(),
            "a cleared order is gone, so nothing reports it complete"
        );
        assert!(!entity.contains::<Autopilot>(), "and the helm is released");
        assert_eq!(
            world.get::<FlightArrivalStandoff>(warship).map(|s| **s),
            Some(Meters(80.0).to_engine()),
            "the ship's own standoff is back, not the cinematic's"
        );
    }

    /// A tolerance that cannot be met - negative, or NaN - is refused at the
    /// action rather than installed as an order that never completes and a
    /// sequence that never advances.
    #[test]
    fn an_impossible_alignment_tolerance_installs_no_order() {
        let ScriptedFixture {
            mut world, warship, ..
        } = a_scripted_ship_and_a_bystander();

        for tolerance in [-1.0, f32::NAN] {
            run(
                &mut world,
                &ForceAlignActionConfig {
                    order: "aim".to_string(),
                    ship: "warship".to_string(),
                    look_at: Meters3::ZERO,
                    tolerance_degrees: tolerance,
                },
            );
            assert!(
                !world.entity(warship).contains::<ScriptedHelmOrder>(),
                "a {tolerance} degree tolerance installs nothing"
            );
        }
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
