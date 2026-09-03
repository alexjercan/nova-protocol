//! Actions that drive or retune a live scenario ship: the helm-order family,
//! the two force-fire verbs, the AI constraints, and the older speed cap,
//! allegiance and per-verb controller flags.

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

/// Fly an ordered ship to an authored mark with the real autopilot.
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
    /// The key this order's lifecycle is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped ship to fly. Any ship the player is not
    /// flying: a `None`-controller actor or an AI ship on a mission.
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
                install_ship_order(
                    world,
                    &id,
                    order,
                    // Engine boundary: the leg and the arrival rule are both
                    // measured against an avian position every tick.
                    ShipOrderDirective::Move {
                        position: position.to_engine(),
                        arrival_standoff: standoff.map(|standoff| standoff.to_engine()),
                    },
                    "MoveShipTo",
                );
            });
        });
    }
}

/// Turn an ordered ship's whole hull onto an authored bearing, without moving
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
    /// The key this order's lifecycle is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped ship to turn.
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
                install_ship_order(
                    world,
                    &id,
                    order,
                    ShipOrderDirective::Align {
                        // Engine boundary: the bearing is compared against an
                        // avian position every tick.
                        look_at: look_at.to_engine(),
                        tolerance: tolerance_degrees.to_radians(),
                    },
                    "ForceAlign",
                );
            });
        });
    }
}

/// Bring an ordered ship to rest with the real STOP maneuver.
///
/// The same flip-retrograde-and-burn the player's X key runs, so a ship that
/// arrives somewhere and stops does it by spending fuel and time, visibly.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopShipActionConfig {
    /// The key this order's lifecycle is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped ship to stop.
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
                install_ship_order(world, &id, order, ShipOrderDirective::Stop, "StopShip");
            });
        });
    }
}

/// Fly an ordered ship ONE loop of an authored route.
///
/// One loop, always: the ship visits every waypoint in order and then returns
/// to the first, and that arrival is the completion. A standing patrol is a
/// scenario that answers its own `OnShipOrderComplete` by issuing the same
/// action again - which is what gives the author a beat at every lap instead
/// of a route that can never be counted.
///
/// This is not the passive `AIControllerConfig::patrol` routine. That one is
/// a ship's own idle behavior and loops forever; this is a MISSION, and while
/// it runs the ship flies it instead of whatever it would do on its own.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PatrolShipActionConfig {
    /// The key this order's lifecycle is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped ship to send round.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The route's marks, world coordinates in meters, in the order they are
    /// flown. One mark is a legal route: the ship flies to it and the loop is
    /// done. An empty one is refused.
    pub waypoints: Vec<Meters3>,
}

impl EventAction<NovaEventWorld> for PatrolShipActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let order = self.order.clone();
        let id = self.ship.clone();
        let waypoints = self.waypoints.clone();
        debug!(
            "PatrolShip: '{}' order '{}' over {} waypoints",
            id,
            order,
            waypoints.len()
        );
        if waypoints.is_empty() {
            error!(
                "PatrolShip: order '{}' has no waypoints; there is no loop to fly",
                order
            );
            return;
        }

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                install_ship_order(
                    world,
                    &id,
                    order,
                    ShipOrderDirective::Patrol {
                        // Engine boundary: the legs are flown against avian
                        // positions.
                        waypoints: waypoints.iter().map(|point| point.to_engine()).collect(),
                        leg: 0,
                    },
                    "PatrolShip",
                );
            });
        });
    }
}

/// Put an ordered ship into a station-keeping orbit around a gravity well.
///
/// The real ORBIT verb: the computer circularizes into the stable band and
/// then holds the ring with micro-burns. Completion reports that the orbit
/// was ESTABLISHED, not that it ended - the ship keeps station afterwards,
/// the same way an alignment keeps its bearing, until a scenario says
/// otherwise.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OrbitShipActionConfig {
    /// The key this order's lifecycle is reported under.
    #[reflect(@Names::Order)]
    pub order: String,
    /// The `EntityId` of the scoped ship to put in orbit.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The `EntityId` of the gravity well to orbit - a planetoid, not a rock.
    #[reflect(@Names::Object)]
    pub well: String,
}

impl EventAction<NovaEventWorld> for OrbitShipActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let order = self.order.clone();
        let id = self.ship.clone();
        let well = self.well.clone();
        debug!("OrbitShip: '{}' order '{}' around '{}'", id, order, well);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                install_ship_order(
                    world,
                    &id,
                    order,
                    ShipOrderDirective::Orbit { well },
                    "OrbitShip",
                );
            });
        });
    }
}

/// Release an ordered ship's helm.
///
/// The counterpart to the five orders above: whatever the ship was told, it is
/// no longer being told it. The hull keeps its velocity - this is space, and
/// the point of clearing an order is usually to let a ship coast out of frame.
/// An AI ship goes back to flying itself.
///
/// Reports `OnShipOrderCanceled`, never `OnShipOrderComplete`: a cleared order
/// did not finish, and a beat waiting on its completion must not run. An order
/// that had ALREADY completed or failed reports nothing here - it is already
/// terminal, and one order fires one terminal event.
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
                let Some(ship) = orderable_ship(world, &id, "ClearShipOrder") else {
                    return;
                };
                cancel_ship_order(world, ship);
            });
        });
    }
}

/// A territorial tether for one AI ship, or its removal.
///
/// A CONSTRAINT, not a mission: it says where the ship may fight, and it
/// coexists with whatever helm order the ship is under. A mission outranks it
/// - a scenario that orders an AI ship across the map gets the move it asked
/// for, and the tether applies again once the order is interrupted or done.
///
/// `Some` installs or replaces the tether; `None` removes it and lets the
/// ship chase freely, the same `Option` shape `SetSpeedCap` uses.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetAILeashActionConfig {
    /// The `EntityId` of the scoped AI ship to tether.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The tether itself; `None` removes it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub leash: Option<AILeashConfig>,
}

/// Where an AI ship's territory is and how big it is.
#[derive(Clone, Copy, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AILeashConfig {
    /// The territory's anchor, world coordinates in meters. Unlike the spawn
    /// config's leash - which anchors on the patrol centroid - this is stated
    /// outright, because a scenario moving a ship's ground has a new one in
    /// mind.
    pub center: Meters3,
    /// Distance from `center` beyond which combat breaks off, in meters.
    pub radius: Meters,
}

impl EventAction<NovaEventWorld> for SetAILeashActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let leash = self.leash;
        debug!("SetAILeash: '{}' -> {:?}", id, leash);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = ai_ship(world, &id, "SetAILeash") else {
                    return;
                };
                let mut entity = world.entity_mut(ship);
                match leash {
                    // Engine boundary: the tether is measured against an avian
                    // position every tick.
                    Some(leash) => {
                        entity.insert(AILeash {
                            center: leash.center.to_engine(),
                            radius: leash.radius.to_engine(),
                        });
                    }
                    None => {
                        entity.remove::<AILeash>();
                    }
                }
            });
        });
    }
}

/// How far one AI ship will leave its routine to fight, or the removal of the
/// override.
///
/// A constraint like the leash, and independent of it: clearing one leaves the
/// other standing. `None` restores the engine's own detection range.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetAIEngageRangeActionConfig {
    /// The `EntityId` of the scoped AI ship to retune.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The hostile-detection range in meters; `None` restores the default.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub range: Option<Meters>,
}

impl EventAction<NovaEventWorld> for SetAIEngageRangeActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let range = self.range;
        debug!("SetAIEngageRange: '{}' -> {:?}", id, range);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = ai_ship(world, &id, "SetAIEngageRange") else {
                    return;
                };
                let mut entity = world.entity_mut(ship);
                match range {
                    // Engine boundary: the range is compared against an avian
                    // distance every tick.
                    Some(range) => {
                        entity.insert(AIEngageRange(range.to_engine()));
                    }
                    None => {
                        entity.remove::<AIEngageRange>();
                    }
                }
            });
        });
    }
}

/// How close an inbound torpedo must come before one AI ship's guns answer
/// it, or the removal of the override.
///
/// The third independent constraint. Point defense runs regardless of what
/// the ship's behavior state or helm order is doing, so this keeps working
/// through a mission.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetAIPointDefenseRangeActionConfig {
    /// The `EntityId` of the scoped AI ship to retune.
    #[reflect(@Names::Object)]
    pub ship: String,
    /// The point-defense range in meters; `None` restores the default.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub range: Option<Meters>,
}

impl EventAction<NovaEventWorld> for SetAIPointDefenseRangeActionConfig {
    fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
        let id = self.ship.clone();
        let range = self.range;
        debug!("SetAIPointDefenseRange: '{}' -> {:?}", id, range);

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| {
                let Some(ship) = ai_ship(world, &id, "SetAIPointDefenseRange") else {
                    return;
                };
                let mut entity = world.entity_mut(ship);
                match range {
                    // Engine boundary: the envelope is compared against an
                    // avian distance every tick.
                    Some(range) => {
                        entity.insert(AIPointDefenseRange(range.to_engine()));
                    }
                    None => {
                        entity.remove::<AIPointDefenseRange>();
                    }
                }
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

/// The scoped ship a scripted WEAPON action may fire, or `None` with a logged
/// refusal.
///
/// Stricter than [`orderable_ship`], and deliberately: a forced shot leaves
/// down whatever line the hull is holding, so the action is only meaningful on
/// a ship whose facing the scenario also owns. An AI ship rewrites its own aim
/// every frame, and a player's ship is not the scenario's to fire. Both used
/// to happen silently - the old all-bays torpedo action documented the race
/// rather than refusing it - so this is an ERROR, loud enough to find in a log.
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

/// Install one helm order on a scoped ship, retiring whatever it was under.
///
/// The five helm actions differ only in their directive, so they share this
/// whole path: resolve the actor, refuse a player's ship, cancel the previous
/// order (which is what reports the cancellation), then install the durable
/// order and hand it the helm. The transient maneuver is NOT built here - the
/// flight layer's driver builds it from the directive, which is the same code
/// that rebuilds it after an AI interruption.
fn install_ship_order(
    world: &mut World,
    id: &str,
    key: String,
    directive: ShipOrderDirective,
    what: &str,
) {
    let Some(ship) = orderable_ship(world, id, what) else {
        return;
    };
    cancel_ship_order(world, ship);

    let mut entity = world.entity_mut(ship);
    // Insert-if-absent, never overwrite: the cancellation above may have just
    // queued a report into it, and a fresh queue would swallow the event that
    // tells a waiting beat its old order is gone.
    if !entity.contains::<ShipOrderReports>() {
        entity.insert(ShipOrderReports::default());
    }
    entity.insert((ShipHelmOrder::new(key, directive), ShipOrderHelmAuthority));
}

/// The scoped ship a HELM order may be given, or `None` with a logged refusal.
///
/// Player ships only. Everything else is fair game: a `SpaceshipController::None`
/// actor has nobody else driving it, and an AI ship simply stops flying itself
/// while the order holds the helm (see `ShipOrderHelmAuthority`) and picks its
/// routine back up when the order ends.
///
/// A player's helm is the one that cannot be shared. The input layer drops the
/// autopilot on any flight input, so an order there would be a tug of war the
/// scenario loses silently every time the player nudges the stick. A scenario
/// that genuinely must fly the player's ship has to take the Player controller
/// off it first, which is a decision worth having to make out loud.
///
/// The test is the DRIVER, never the allegiance: the chapter-one warship reads
/// Enemy and is still entirely scripted.
fn orderable_ship(world: &mut World, id: &str, what: &str) -> Option<Entity> {
    let ship = scoped_ship(world, id).or_else(|| {
        warn!("{what}: no scoped ship with id '{id}'");
        None
    })?;
    if world.entity(ship).contains::<PlayerSpaceshipMarker>() {
        error!(
            "{what}: ship '{id}' is player-driven; a helm order cannot share a helm with \
             live input (replace the Player controller with `None` first)"
        );
        return None;
    }
    Some(ship)
}

/// The scoped AI ship an AI-only constraint may retune, or `None` with a
/// logged refusal.
///
/// The mirror of [`orderable_ship`]: a leash, an engage range and a
/// point-defense envelope are all statements about how a ship's own judgement
/// behaves, and a hull with no judgement has nothing to constrain. Setting one
/// on a `None`-controller actor would install a component nothing reads.
fn ai_ship(world: &mut World, id: &str, what: &str) -> Option<Entity> {
    let ship = scoped_ship(world, id).or_else(|| {
        warn!("{what}: no scoped ship with id '{id}'");
        None
    })?;
    if !world.entity(ship).contains::<AISpaceshipMarker>() {
        error!(
            "{what}: ship '{id}' is not AI-driven; an AI constraint only means something \
             on a ship that flies itself"
        );
        return None;
    }
    Some(ship)
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

    /// A PLAYER's ship refuses every one of these actions. Taking the helm
    /// would fight the input layer, which drops the autopilot on any flight
    /// input, and a forced shot is not the scenario's to fire; both used to
    /// happen silently. A scenario that must fly the player's hull has to take
    /// the Player controller off it first.
    #[test]
    fn a_player_ship_refuses_every_ship_action() {
        let ScriptedFixture {
            mut world,
            warship,
            railgun,
            bay,
            ..
        } = a_scripted_ship_and_a_bystander();
        world.entity_mut(warship).insert(PlayerSpaceshipMarker);

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
            &PatrolShipActionConfig {
                order: "sweep".to_string(),
                ship: "warship".to_string(),
                waypoints: vec![Meters3::new(0.0, 0.0, 500.0)],
            },
        );
        run(
            &mut world,
            &OrbitShipActionConfig {
                order: "hold".to_string(),
                ship: "warship".to_string(),
                well: "planetoid".to_string(),
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
        assert!(!entity.contains::<ShipHelmOrder>(), "no helm order");
        assert!(
            !entity.contains::<ShipOrderHelmAuthority>(),
            "and no helm authority to fight the stick with"
        );
        assert!(world.get::<ScriptedRailgunOrder>(railgun).is_none());
        assert!(world.get::<ScriptedTorpedoOrder>(bay).is_none());
    }

    /// An AI ship TAKES a helm order - that is the whole point of the shared
    /// mission layer - but still refuses a forced shot. A forced shot leaves
    /// down whatever line the hull holds, and an AI hull rewrites its own aim
    /// every frame, so the two are not the same question.
    #[test]
    fn an_ai_ship_takes_a_helm_order_but_not_a_forced_shot() {
        let ScriptedFixture {
            mut world,
            warship,
            railgun,
            bay,
            ..
        } = a_scripted_ship_and_a_bystander();
        world.entity_mut(warship).insert(AISpaceshipMarker);

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
        assert!(
            entity.contains::<ShipHelmOrder>(),
            "the mission is installed on an AI hull"
        );
        assert!(
            entity.contains::<ShipOrderHelmAuthority>(),
            "and it owns the helm, which is what silences the AI's flight writers"
        );
        assert!(
            world.get::<ScriptedRailgunOrder>(railgun).is_none(),
            "but its guns are still its own"
        );
        assert!(world.get::<ScriptedTorpedoOrder>(bay).is_none());
    }

    /// The five helm orders are one mutually exclusive family: installing one
    /// takes the last one off and reports it CANCELED, so a beat waiting for
    /// the replaced order's completion is told rather than left waiting.
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
            world.get::<ShipHelmOrder>(warship).map(ShipHelmOrder::kind),
            Some(ShipOrderKind::Move)
        ));

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
            .get::<ShipHelmOrder>(warship)
            .expect("the alignment order is installed");
        assert_eq!(order.key, "aim");
        assert_eq!(order.kind(), ShipOrderKind::Align);
        let reports = world
            .get::<ShipOrderReports>(warship)
            .expect("the ship carries a report queue");
        assert_eq!(
            reports
                .0
                .iter()
                .map(|report| (report.key.as_str(), report.outcome))
                .collect::<Vec<_>>(),
            vec![("approach", ShipOrderOutcome::Canceled)],
            "the replaced move is reported canceled, and the new order is not \
             reported at all until it happens"
        );
    }

    /// ClearShipOrder retires the order and reports a CANCELLATION, never a
    /// completion: a cleared order did not finish, and a beat chained off its
    /// completion must not run.
    #[test]
    fn clearing_an_order_reports_a_cancellation_not_a_completion() {
        let ScriptedFixture {
            mut world, warship, ..
        } = a_scripted_ship_and_a_bystander();

        run(
            &mut world,
            &StopShipActionConfig {
                order: "halt".to_string(),
                ship: "warship".to_string(),
            },
        );
        run(
            &mut world,
            &ClearShipOrderActionConfig {
                ship: "warship".to_string(),
            },
        );

        let entity = world.entity(warship);
        assert!(
            !entity.contains::<ShipHelmOrder>(),
            "a cleared order is gone"
        );
        assert!(
            !entity.contains::<ShipOrderHelmAuthority>(),
            "and the helm is handed back"
        );
        let reports = world
            .get::<ShipOrderReports>(warship)
            .expect("the ship carries a report queue");
        assert_eq!(
            reports
                .0
                .iter()
                .map(|report| report.outcome)
                .collect::<Vec<_>>(),
            vec![ShipOrderOutcome::Canceled]
        );
    }

    /// A tolerance that cannot be met - negative, or NaN - is refused at the
    /// action rather than installed as an order that never completes and a
    /// sequence that never advances. An empty patrol route is refused for the
    /// same reason: there is no loop to fly and so no completion to wait for.
    #[test]
    fn an_unflyable_order_is_refused_rather_than_installed() {
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
                !world.entity(warship).contains::<ShipHelmOrder>(),
                "a {tolerance} degree tolerance installs nothing"
            );
        }

        run(
            &mut world,
            &PatrolShipActionConfig {
                order: "sweep".to_string(),
                ship: "warship".to_string(),
                waypoints: Vec::new(),
            },
        );
        assert!(
            !world.entity(warship).contains::<ShipHelmOrder>(),
            "an empty route installs nothing"
        );
        assert!(
            world
                .get::<ShipOrderReports>(warship)
                .is_none_or(|reports| reports.0.is_empty()),
            "and a REFUSED order reports nothing at all - it was never accepted"
        );
    }

    /// An AI constraint only means something on a ship that flies itself, and
    /// the three of them are independent: setting one and clearing it leaves
    /// the others standing.
    #[test]
    fn ai_constraints_apply_only_to_ai_ships_and_clear_independently() {
        let ScriptedFixture {
            mut world, warship, ..
        } = a_scripted_ship_and_a_bystander();

        let leash = |center: Meters3, radius: f32| SetAILeashActionConfig {
            ship: "warship".to_string(),
            leash: Some(AILeashConfig {
                center,
                radius: Meters(radius),
            }),
        };

        run(&mut world, &leash(Meters3::ZERO, 3_000.0));
        assert!(
            !world.entity(warship).contains::<AILeash>(),
            "a hull with no judgement has nothing to constrain"
        );

        world.entity_mut(warship).insert(AISpaceshipMarker);
        run(&mut world, &leash(Meters3::new(0.0, 0.0, 100.0), 3_000.0));
        run(
            &mut world,
            &SetAIEngageRangeActionConfig {
                ship: "warship".to_string(),
                range: Some(Meters(1_200.0)),
            },
        );
        run(
            &mut world,
            &SetAIPointDefenseRangeActionConfig {
                ship: "warship".to_string(),
                range: Some(Meters(900.0)),
            },
        );

        assert_eq!(
            world.get::<AILeash>(warship).map(|leash| leash.radius),
            Some(Meters(3_000.0).to_engine())
        );
        assert_eq!(
            world.get::<AIEngageRange>(warship).map(|range| range.0),
            Some(Meters(1_200.0).to_engine())
        );
        assert_eq!(
            world
                .get::<AIPointDefenseRange>(warship)
                .map(|range| range.0),
            Some(Meters(900.0).to_engine())
        );

        run(
            &mut world,
            &SetAIEngageRangeActionConfig {
                ship: "warship".to_string(),
                range: None,
            },
        );
        let entity = world.entity(warship);
        assert!(
            !entity.contains::<AIEngageRange>(),
            "the cleared constraint is gone"
        );
        assert!(
            entity.contains::<AILeash>() && entity.contains::<AIPointDefenseRange>(),
            "and the other two are untouched"
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
