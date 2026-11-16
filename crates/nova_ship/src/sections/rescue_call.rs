//! The SOS rescue call a stranded player ship places, its clock, cancel and
//! end.
//!
//! The Ship pane places the call: it picks the answering station at the
//! click and the lost sections the rescue will rebuild, and inserts
//! [`ShipRescueCall`] on the ship root. Nothing is spent at the click.
//! [`advance_ship_rescue_call`] runs the call on the game clock, so the TAB
//! interface and the pause menu, which stop `Time<Virtual>`, hold it. When
//! [`RESCUE_SECS`] have run the call is due, and `nova_world_base` answers it:
//! when the station still admits the ship it takes the bill, rebuilds the
//! chain, moves the ship beside the station at rest and docks it there. A
//! ship that docks anywhere before the answer ends its call
//! ([`end_rescue_call_on_dock`]). Every end writes one [`RescueCallEnded`] for
//! the HUD notice.
//!
//! A save keeps the call with its station, chain and elapsed time, so a Load
//! resumes it.
//!
//! [`RESCUE_SECS`]: nova_gameplay::prelude::RESCUE_SECS

use bevy::prelude::*;
use nova_events::prelude::{EntityId, GeneratedBodyId, Meters3};
use nova_gameplay::prelude::{
    plan_rescue_bill, stranded_reasons, CivilizationId, ControllerSectionMarker, GameItems,
    RescueBill, SectionClass, SectionInactiveMarker, SectionMarker, ShipCredits, ShipInventory,
    SpaceshipRootMarker, StrandedFacts, StrandedType, ThrusterSectionMarker, RESCUE_SECS,
};

use super::prelude::{
    rescue_rebuild_chain, DockedPort, DockedShip, DockingSectionMarker, PlacedSectionLinkPoints,
    RepairWorkType, SectionLinkPoints, ShipDesignSections, ShipRepairJobs,
};

/// The rescue call, its tick, cancel and end.
pub mod prelude {
    pub use super::{
        advance_ship_rescue_call, end_rescue_call_on_dock, rescue_bill, rescue_quote,
        stranded_facts, RescueCallCancelRequest, RescueCallEnded, RescueEndType, RescueQuote,
        RescueRebuild, RescueSystems, ShipRescueCall,
    };
}

/// A running rescue call on a ship root.
#[derive(Component, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipRescueCall {
    /// The station that answers, picked at the click.
    pub station: GeneratedBodyId,
    /// The station's civilization. A station whose civilization turned
    /// Enemy before the answer refuses it.
    pub civilization: CivilizationId,
    /// Where the ship waits for the station's berth: in the station's open
    /// space, clear of its cluster.
    pub spawn: Meters3,
    /// The lost design sections the rescue rebuilds, in rebuild order, as
    /// planned at the click. The answer rebuilds those still lost, so the
    /// bill is never above the one the button showed.
    pub chain: Vec<RescueRebuild>,
    /// Game seconds the call has run, at most [`RESCUE_SECS`].
    pub elapsed_secs: f32,
}

/// One lost section a rescue rebuilds.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RescueRebuild {
    /// The design section id.
    pub section: String,
    /// Its pane code (`DOCK-1`), for the reports and the notice.
    pub code: String,
}

impl ShipRescueCall {
    /// A call to `station` of `civilization`, waiting at `spawn`, that
    /// rebuilds `chain`.
    pub fn new(
        station: GeneratedBodyId,
        civilization: CivilizationId,
        spawn: Meters3,
        chain: Vec<RescueRebuild>,
    ) -> Self {
        Self {
            station,
            civilization,
            spawn,
            chain,
            elapsed_secs: 0.0,
        }
    }

    /// The share of the call's time run, in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        (self.elapsed_secs / RESCUE_SECS).clamp(0.0, 1.0)
    }

    /// Game seconds left before the answer, never below zero.
    pub fn remaining_secs(&self) -> f32 {
        (RESCUE_SECS - self.elapsed_secs).max(0.0)
    }

    /// Whether the call's time has run: the world answers it.
    pub fn is_due(&self) -> bool {
        self.elapsed_secs >= RESCUE_SECS
    }
}

/// What the Ship pane offers a stranded ship: why it is stranded, what a
/// rescue would rebuild and the bill it would take now.
#[derive(Clone, Debug, PartialEq)]
pub struct RescueQuote {
    /// Why the ship is stranded; never empty.
    pub reasons: Vec<StrandedType>,
    /// The reasons the chain cannot fix: the ship has no lost section of
    /// that kind its live hull reaches, only live ones that do not work. A
    /// rescue that leaves one of these would dock a ship that still cannot
    /// fly, so the pane refuses the call while this is not empty.
    pub unfixed: Vec<StrandedType>,
    /// The lost design sections the rescue rebuilds, in rebuild order.
    pub chain: Vec<String>,
    /// The bill, at the hold and balance the ship holds now.
    pub bill: RescueBill,
}

/// The stranded facts of ship root `ship`: whether one of its sections is a
/// working thruster, a working flight computer and a free, working docking
/// port.
pub fn stranded_facts(world: &World, ship: Entity) -> StrandedFacts {
    let mut facts = StrandedFacts::default();
    for child in world.get::<Children>(ship).into_iter().flatten() {
        let section = world.entity(*child);
        if !section.contains::<SectionMarker>() || section.contains::<SectionInactiveMarker>() {
            continue;
        }
        facts.working_thruster |= section.contains::<ThrusterSectionMarker>();
        facts.working_computer |= section.contains::<ControllerSectionMarker>();
        facts.free_port |=
            section.contains::<DockingSectionMarker>() && !section.contains::<DockedPort>();
    }
    facts
}

/// The live sections of `ship` as the rebuild rules read them: each design
/// id with its placement in the ship root's frame.
fn live_sections(world: &World, ship: Entity) -> Vec<(&str, PlacedSectionLinkPoints<'_>)> {
    world
        .get::<Children>(ship)
        .into_iter()
        .flatten()
        .filter_map(|child| {
            let section = world.entity(*child);
            section.get::<SectionMarker>()?;
            let (id, transform, points) = (
                section.get::<EntityId>()?,
                section.get::<Transform>()?,
                section.get::<SectionLinkPoints>()?,
            );
            Some((
                id.0.as_str(),
                PlacedSectionLinkPoints {
                    position: transform.translation,
                    rotation: transform.rotation,
                    link_points: &points.0,
                },
            ))
        })
        .collect()
}

/// The bill of a rescue of `ship` that rebuilds the lost design sections
/// `chain`, at its hold and balance now. Parts running repair jobs hold
/// back stay out of it, except those of a field rebuild of a `chain`
/// section, which the rescue cancels.
///
/// # Panics
///
/// When `ship` has no [`ShipDesignSections`], [`ShipInventory`] or
/// [`ShipCredits`], when `chain` names a section its design lacks, or before
/// the item catalog loaded: a rescue is offered only to a designed player
/// ship in play.
pub fn rescue_bill(world: &World, ship: Entity, chain: &[String]) -> RescueBill {
    let design = world
        .get::<ShipDesignSections>(ship)
        .unwrap_or_else(|| panic!("rescue_bill: ship {ship:?} has no ShipDesignSections"));
    let own = world
        .get::<ShipInventory>(ship)
        .unwrap_or_else(|| panic!("rescue_bill: ship {ship:?} has no ShipInventory"));
    let credits = world
        .get::<ShipCredits>(ship)
        .unwrap_or_else(|| panic!("rescue_bill: ship {ship:?} has no ShipCredits"));
    let free = match world.get::<ShipRepairJobs>(ship) {
        Some(jobs) => ShipRepairJobs(
            jobs.0
                .iter()
                .filter(|job| match &job.work {
                    RepairWorkType::FieldRebuild { section, .. } => !chain.contains(section),
                    RepairWorkType::Plates { .. } => true,
                })
                .cloned()
                .collect(),
        )
        .free_hold(own),
        None => own.clone(),
    };
    let recipes: Vec<_> = chain
        .iter()
        .map(|id| {
            &design
                .get(id)
                .unwrap_or_else(|| {
                    panic!("rescue_bill: ship {ship:?} has no design section '{id}'")
                })
                .config
                .base
                .build
        })
        .collect();
    plan_rescue_bill(world.resource::<GameItems>(), &recipes, &free, credits.0)
}

/// The rescue the Ship pane offers ship root `ship`, or `None` while it is
/// docked or not stranded. The chain reaches a lost port, thruster and
/// flight computer, each only when the ship has no working one.
///
/// # Panics
///
/// As [`rescue_bill`].
pub fn rescue_quote(world: &World, ship: Entity) -> Option<RescueQuote> {
    if world.get::<DockedShip>(ship).is_some() {
        return None;
    }
    let facts = stranded_facts(world, ship);
    let reasons = stranded_reasons(facts);
    if reasons.is_empty() {
        return None;
    }
    let design = world.get::<ShipDesignSections>(ship)?;
    let needs: Vec<(StrandedType, SectionClass)> = [
        (
            !facts.free_port,
            StrandedType::NoDockingPort,
            SectionClass::Docking,
        ),
        (
            !facts.working_thruster,
            StrandedType::NoThruster,
            SectionClass::Thruster,
        ),
        (
            !facts.working_computer,
            StrandedType::NoFlightComputer,
            SectionClass::Controller,
        ),
    ]
    .into_iter()
    .filter_map(|(need, reason, class)| need.then_some((reason, class)))
    .collect();
    let classes: Vec<SectionClass> = needs.iter().map(|(_, class)| *class).collect();
    let chain = rescue_rebuild_chain(design, &live_sections(world, ship), &classes);
    let unfixed = needs
        .iter()
        .filter(|(_, class)| {
            !chain.iter().any(|id| {
                design
                    .get(id)
                    .is_some_and(|section| section.config.kind.class() == *class)
            })
        })
        .map(|(reason, _)| *reason)
        .collect();
    let bill = rescue_bill(world, ship, &chain);
    Some(RescueQuote {
        reasons,
        unfixed,
        chain,
        bill,
    })
}

/// Asks to cancel the rescue call of ship `entity`. Nothing was spent, so a
/// cancel is free. A ship with no call does nothing.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct RescueCallCancelRequest {
    /// The ship root.
    pub entity: Entity,
}

/// A rescue call of ship `ship` reached an end.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct RescueCallEnded {
    /// The ship root.
    pub ship: Entity,
    /// How the call ended.
    pub end: RescueEndType,
}

/// How a [`ShipRescueCall`] ended.
#[derive(Clone, Debug, PartialEq)]
pub enum RescueEndType {
    /// The call was answered: the bill was taken, the chain rebuilt and the
    /// ship moved beside the station. The berth that docks it follows.
    Answered {
        /// Credits the ship paid.
        paid_cr: u32,
    },
    /// The ship docked before the answer. Nothing was taken.
    Docked,
    /// At the answer the station would not admit the ship: it is dead, its
    /// civilization turned Enemy, or it is hostile to the ship. Nothing was
    /// taken and the ship stays where it is.
    Refused,
    /// After a paid answer the station did not take the ship in time: no
    /// free berth, or the dock was refused. The rebuilt sections stay, so
    /// the ship, fixed beside the station, can fly and dock by itself.
    NoBerth,
    /// The player cancelled the call. Nothing was taken.
    Cancelled,
}

/// System set for the rescue call tick, in `Update`. Runs on the game clock.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RescueSystems;

/// Run every ship's rescue call for this frame's game time, up to
/// [`RESCUE_SECS`]. The world answers a due call.
pub fn advance_ship_rescue_call(
    time: Res<Time>,
    mut q_call: Query<&mut ShipRescueCall, With<SpaceshipRootMarker>>,
) {
    let dt = time.delta_secs();
    for mut call in &mut q_call {
        if call.is_due() {
            continue;
        }
        call.elapsed_secs = (call.elapsed_secs + dt).min(RESCUE_SECS);
    }
}

/// End the call of every ship that docked before its answer, as
/// [`RescueEndType::Docked`]: nothing was taken, and a docked ship is not
/// stranded.
pub fn end_rescue_call_on_dock(
    mut commands: Commands,
    q_docked: Query<Entity, (With<ShipRescueCall>, With<DockedShip>)>,
    mut ended: MessageWriter<RescueCallEnded>,
) {
    for ship in &q_docked {
        info!("end_rescue_call_on_dock: ship {ship:?} docked; its SOS call ends");
        commands.entity(ship).remove::<ShipRescueCall>();
        ended.write(RescueCallEnded {
            ship,
            end: RescueEndType::Docked,
        });
    }
}

/// Cancel the call a [`RescueCallCancelRequest`] names.
pub(crate) fn cancel_rescue_call(request: On<RescueCallCancelRequest>, mut commands: Commands) {
    let ship = request.entity;
    // Take the call and write the end in one command, so two cancels before
    // a flush end the call once.
    commands.queue(move |world: &mut World| {
        let taken = world
            .get_entity_mut(ship)
            .ok()
            .and_then(|mut entity| entity.take::<ShipRescueCall>());
        if taken.is_some() {
            world.write_message(RescueCallEnded {
                ship,
                end: RescueEndType::Cancelled,
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{ecs::message::Messages, time::TimeUpdateStrategy};
    use nova_events::prelude::SectorCoord;
    use nova_gameplay::prelude::CivilizationId;

    use super::*;

    /// A ship with a fresh call, its clock stepping 10 s a frame after the
    /// warm-up frame whose delta is zero.
    fn call_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(10)));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(60));
        app.add_message::<RescueCallEnded>();
        app.add_observer(cancel_rescue_call);
        app.add_systems(
            Update,
            (end_rescue_call_on_dock, advance_ship_rescue_call).chain(),
        );
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipRescueCall::new(
                    GeneratedBodyId::new(SectorCoord::new(1, 0, 0), "cluster_station", 0),
                    CivilizationId {
                        world_seed: 1,
                        node: [0, 0, 0],
                    },
                    Meters3::ZERO,
                    vec![RescueRebuild {
                        section: "dock".to_string(),
                        code: "DOCK-1".to_string(),
                    }],
                ),
            ))
            .id();
        app.update();
        (app, ship)
    }

    fn call(app: &App, ship: Entity) -> Option<&ShipRescueCall> {
        app.world().get::<ShipRescueCall>(ship)
    }

    /// The TAB interface and the pause menu stop `Time<Virtual>`; the call
    /// holds while they do, runs on the game clock, and is due after
    /// `RESCUE_SECS` of it.
    #[test]
    fn a_rescue_call_runs_on_the_game_clock_and_holds_while_it_is_frozen() {
        let (mut app, ship) = call_app();
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(call(&app, ship).unwrap().elapsed_secs, 0.0);
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        let frames = (RESCUE_SECS / 10.0).ceil() as usize;
        for _ in 0..frames - 1 {
            app.update();
            assert!(!call(&app, ship).unwrap().is_due());
        }
        app.update();
        let due = call(&app, ship).unwrap();
        assert!(due.is_due());
        assert_eq!(due.elapsed_secs, RESCUE_SECS);
        assert_eq!(due.remaining_secs(), 0.0);
    }

    /// A cancel ends the call once, as cancelled; a second cancel does
    /// nothing.
    #[test]
    fn a_cancelled_rescue_call_ends_once() {
        let (mut app, ship) = call_app();
        app.update();
        app.world_mut()
            .trigger(RescueCallCancelRequest { entity: ship });
        app.world_mut()
            .trigger(RescueCallCancelRequest { entity: ship });
        app.update();
        assert!(call(&app, ship).is_none());
        let ends: Vec<RescueEndType> = app
            .world_mut()
            .resource_mut::<Messages<RescueCallEnded>>()
            .drain()
            .map(|ended| ended.end)
            .collect();
        assert_eq!(ends, vec![RescueEndType::Cancelled]);
    }

    /// A ship that docks while its call runs ends the call once, as docked,
    /// and nothing answers it later.
    #[test]
    fn a_rescue_call_ends_when_the_ship_docks() {
        let (mut app, ship) = call_app();
        app.update();
        let connection = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(ship).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        app.update();
        app.update();
        assert!(call(&app, ship).is_none());
        let ends: Vec<RescueEndType> = app
            .world_mut()
            .resource_mut::<Messages<RescueCallEnded>>()
            .drain()
            .map(|ended| ended.end)
            .collect();
        assert_eq!(ends, vec![RescueEndType::Docked]);
    }
}
