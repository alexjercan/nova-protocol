//! The world's side of an SOS rescue call: which station answers, and the
//! answer itself.
//!
//! The Ship pane asks [`find_rescue_station`] at the click: the nearest
//! living station, of the civilizations in the 3x3x3 lattice block round the
//! ship, whose civilization is Ally or Neutral to the player now. A hostile
//! station never answers. `nova_ship` runs the call ([`ShipRescueCall`]);
//! when it is due, [`answer_rescue_calls`] checks the station would still
//! admit the ship, takes the bill, rebuilds the chain, moves the ship and its
//! fleet beside the station at rest and gives the ship a [`StationBerth`],
//! which docks it there. A refusal takes nothing.

use bevy::prelude::*;
use nova_events::prelude::{GeneratedBodyId, Meters3};
use nova_gameplay::prelude::{
    CivilizationId, CivilizationStances, DerelictShipMarker, NeutralizedMarker, StanceType,
    WorldClock,
};
use nova_world::prelude::{FrozenBodyType, FrozenSectors, SectorFault, WorldConfig};

use crate::{
    civilizations::{CivilizationStatusType, CIVILIZATION_LATTICE},
    layered::NovaLayeredWorld,
    stance::seeded_stance,
    start::block_stations,
};

/// The station that answers a rescue call.
#[derive(Clone, Debug, PartialEq)]
pub struct RescueStation {
    /// The station.
    pub id: GeneratedBodyId,
    /// Its civilization.
    pub civilization: CivilizationId,
    /// Where the rescued ship waits for its berth: past the station on its
    /// open side, clear of its cluster.
    pub spawn: Meters3,
}

/// The lattice node of the civilization whose centroid cell holds `at`.
fn lattice_node(at: Meters3) -> [i32; 3] {
    at.get()
        .to_array()
        .map(|axis| (axis / CIVILIZATION_LATTICE.0).round() as i32)
}

/// Whether `civilization` answers the player now: any stance but Enemy.
fn answers(world: &World, civilization: &CivilizationId) -> bool {
    let now = world
        .get_resource::<WorldClock>()
        .map_or(0.0, |clock| clock.play_seconds);
    let stance = world.get_resource::<CivilizationStances>().map_or_else(
        || seeded_stance(civilization),
        |stances| stances.effective(civilization, seeded_stance(civilization), now),
    );
    stance != StanceType::Enemy
}

/// Whether the generated station `id` still stands: live and not
/// neutralized or derelict, frozen and not derelict, held back, or in a cell
/// never visited, where its living civilization keeps it whole.
fn station_alive(world: &mut World, id: &GeneratedBodyId) -> bool {
    let live = world
        .query::<(
            &GeneratedBodyId,
            Has<NeutralizedMarker>,
            Has<DerelictShipMarker>,
        )>()
        .iter(world)
        .find(|(body, ..)| *body == id)
        .map(|(_, neutralized, derelict)| !neutralized && !derelict);
    if let Some(alive) = live {
        return alive;
    }
    let Some(sector) = world
        .get_resource::<FrozenSectors>()
        .and_then(|frozen| frozen.get(id.origin))
    else {
        return true;
    };
    if !sector.is_visited() {
        return true;
    }
    let name = id.to_string();
    sector.bodies().iter().any(|body| match body.body() {
        FrozenBodyType::Station(station, _) => {
            body.id() == Some(name.as_str()) && !station.ship().is_derelict()
        }
        FrozenBodyType::PendingStation { id } => *id == name,
        _ => false,
    })
}

/// The station that answers a rescue call placed at `from`, or `None` when
/// none does or no open world is armed.
///
/// # Errors
///
/// Whatever the station claims or the clusters refuse for a civilization of
/// the block.
pub fn find_rescue_station(
    world: &mut World,
    from: Meters3,
) -> Result<Option<RescueStation>, SectorFault> {
    let Some(config) = world.get_resource::<WorldConfig<NovaLayeredWorld>>() else {
        return Ok(None);
    };
    let mut stations = block_stations(
        config.seed,
        &config.generator,
        lattice_node(from),
        |civilization| {
            civilization.status == CivilizationStatusType::Living
                && answers(world, &civilization.id)
        },
    )?;
    stations.sort_by(|(a, _), (b, _)| {
        a.position
            .distance(from)
            .get()
            .total_cmp(&b.position.distance(from).get())
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(stations
        .into_iter()
        .find(|(station, _)| station_alive(world, &station.id))
        .map(|(station, civilization)| RescueStation {
            id: station.id,
            civilization,
            spawn: station.spawn,
        }))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use answer::answer_rescue_calls;

#[cfg(not(target_arch = "wasm32"))]
mod answer {
    use std::collections::BTreeSet;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position};
    use bevy::{ecs::system::SystemState, prelude::*};
    use nova_events::prelude::{CreditChangeCauseType, EntityId, GeneratedBodyId};
    use nova_gameplay::prelude::{
        cr_text, PlayerFleetMarker, SectionMarker, ShipCredits, ShipInventory, ShipInventoryReport,
        ShipInventoryReports, StationMarker,
    };
    use nova_scenario::prelude::{StationBerth, StationBerthType};
    use nova_ship::prelude::{
        rescue_bill, shift_chase_cameras, AIFollow, DockAdmission, DockedShip, HullEnvelopeRadius,
        RepairJobCancelRequest, RepairJobKey, RescueCallEnded, RescueEndType,
        SectionRebuildRequest, ShipRepairJobs, ShipRescueCall, AI_FOLLOW_GAP,
    };

    use super::{answers, station_alive};

    /// Frames in which the game clock runs that a rescued ship waits for
    /// its station to stream in and take it before it gives up undocked.
    /// Frames, not seconds: a slow frame streams no less per frame.
    pub(crate) const RESCUE_BERTH_FRAMES: u32 = 1_200;

    /// Steps outward a fleet slot tries, [`AI_FOLLOW_GAP`] apart, before an
    /// escort takes the last one.
    const FLEET_SLOT_STEPS: usize = 8;

    /// Answer every due rescue call.
    ///
    /// A docked ship, or one already berthing, ends its call as
    /// [`RescueEndType::Docked`]. A station that would not admit the ship
    /// refuses: it is dead, its civilization is Enemy to the player (a
    /// provocation included), or, while it is in play, it is hostile to the
    /// ship ([`DockAdmission::turns_away`]). A refusal takes nothing and the
    /// ship stays where it is. Otherwise the answer, in this order:
    ///
    /// 1. takes the bill ([`rescue_bill`]) for the chain sections still
    ///    lost, after it cancels a field rebuild running on one: the hold
    ///    parts, then the credits as one `Rescue` credit change;
    /// 2. rebuilds each such section through [`SectionRebuildRequest`] and
    ///    reports it `Rebuilt` with the parts taken from the hold for it and
    ///    its share of the credits charged
    ///    ([`RescueBill::section_charges_cr`](nova_gameplay::prelude::RescueBill::section_charges_cr));
    /// 3. moves the ship to the station's spawn point at rest, and each fleet
    ///    escort that is not docked to a clear fleet slot round it;
    /// 4. gives the ship a [`StationBerth`], which docks it. A berth that
    ///    gives up leaves the ship paid for and fixed beside the station.
    ///
    /// Runs after the repair jobs and their command flush, so a field
    /// rebuild that ended this frame is a live section here and is neither
    /// billed nor rebuilt twice.
    ///
    /// # Panics
    ///
    /// As [`rescue_bill`], and when the ship has no `Position`, hold,
    /// balance or report queue: the call is placed only on a player ship in
    /// play.
    pub(crate) fn answer_rescue_calls(world: &mut World) {
        let due: Vec<(Entity, ShipRescueCall)> = world
            .query::<(Entity, &ShipRescueCall)>()
            .iter(world)
            .filter(|(_, call)| call.is_due())
            .map(|(ship, call)| (ship, call.clone()))
            .collect();
        for (ship, call) in due {
            world.entity_mut(ship).remove::<ShipRescueCall>();
            if world.get::<DockedShip>(ship).is_some() || world.get::<StationBerth>(ship).is_some()
            {
                info!("answer_rescue_calls: ship {ship:?} is docked; its SOS call ends");
                world.write_message(RescueCallEnded {
                    ship,
                    end: RescueEndType::Docked,
                });
                continue;
            }
            if let Some(why) = refusal(world, ship, &call) {
                info!(
                    "answer_rescue_calls: station {} refuses: {why}; nothing is taken",
                    call.station
                );
                world.write_message(RescueCallEnded {
                    ship,
                    end: RescueEndType::Refused,
                });
                continue;
            }
            let paid_cr = take_the_bill_and_rebuild(world, ship, &call);
            move_beside_the_station(world, ship, &call);
            info!(
                "answer_rescue_calls: station {} answered; paid {}",
                call.station,
                cr_text(paid_cr)
            );
            world.entity_mut(ship).insert(StationBerth {
                station: call.station.clone(),
                purpose: StationBerthType::Rescue {
                    frames_left: RESCUE_BERTH_FRAMES,
                },
            });
            world.write_message(RescueCallEnded {
                ship,
                end: RescueEndType::Answered { paid_cr },
            });
        }
    }

    /// Why `call`'s station would not admit `ship` now, or `None` when it
    /// would. A station out of play keeps no fire to answer, so only its
    /// life and its civilization's stance judge it.
    fn refusal(world: &mut World, ship: Entity, call: &ShipRescueCall) -> Option<&'static str> {
        if !station_alive(world, &call.station) {
            return Some("it is dead");
        }
        if !answers(world, &call.civilization) {
            return Some("its civilization is Enemy to the player");
        }
        let live = world
            .query_filtered::<(Entity, &GeneratedBodyId), With<StationMarker>>()
            .iter(world)
            .find(|(_, id)| **id == call.station)
            .map(|(station, _)| station);
        let station = live?;
        let mut admission = SystemState::<DockAdmission>::new(world);
        admission
            .get(world)
            .expect("DockAdmission holds only queries, which always validate")
            .turns_away(ship, station)
            .then_some("it is hostile to the ship")
    }

    /// Take the bill for the chain sections of `call` still lost on `ship`,
    /// then rebuild them. Returns the credits paid.
    fn take_the_bill_and_rebuild(world: &mut World, ship: Entity, call: &ShipRescueCall) -> u32 {
        let live: BTreeSet<String> = world
            .get::<Children>(ship)
            .into_iter()
            .flatten()
            .filter(|child| world.get::<SectionMarker>(**child).is_some())
            .filter_map(|child| world.get::<EntityId>(*child))
            .map(|id| id.0.clone())
            .collect();
        let chain: Vec<_> = call
            .chain
            .iter()
            .filter(|rebuild| !live.contains(&rebuild.section))
            .cloned()
            .collect();
        let ids: Vec<String> = chain
            .iter()
            .map(|rebuild| rebuild.section.clone())
            .collect();
        let running: Vec<String> = world
            .get::<ShipRepairJobs>(ship)
            .into_iter()
            .flat_map(|jobs| &jobs.0)
            .filter_map(|job| match job.key() {
                RepairJobKey::Lost(section) if ids.contains(&section) => Some(section),
                _ => None,
            })
            .collect();
        for section in running {
            world.trigger(RepairJobCancelRequest {
                entity: ship,
                key: RepairJobKey::Lost(section),
            });
        }
        world.flush();
        let bill = rescue_bill(world, ship, &ids);
        let mut entity = world.entity_mut(ship);
        {
            let mut own = entity
                .get_mut::<ShipInventory>()
                .unwrap_or_else(|| panic!("answer_rescue_calls: ship {ship:?} has no hold"));
            // `rescue_bill` planned these from the free hold.
            for (item, &count) in &bill.from_hold {
                own.remove(item, count);
            }
        }
        let mut credits = *entity
            .get::<ShipCredits>()
            .unwrap_or_else(|| panic!("answer_rescue_calls: ship {ship:?} has no balance"));
        let left_cr = credits.0 - bill.charged_cr;
        let mut reports = entity
            .get_mut::<ShipInventoryReports>()
            .unwrap_or_else(|| panic!("answer_rescue_calls: ship {ship:?} has no report queue"));
        reports.change_credits(&mut credits, left_cr, CreditChangeCauseType::Rescue);
        for ((rebuild, section), price_cr) in chain
            .iter()
            .zip(&bill.sections)
            .zip(bill.section_charges_cr())
        {
            reports.0.push(ShipInventoryReport::Rebuilt {
                section: rebuild.code.clone(),
                parts: section.from_hold.clone(),
                price_cr,
            });
        }
        entity.insert(credits);
        for section in ids {
            world.trigger(SectionRebuildRequest {
                entity: ship,
                section,
            });
        }
        world.flush();
        bill.charged_cr
    }

    /// Move `ship` to the spawn point of `call`'s station at rest, the chase
    /// camera with it, and every fleet escort that is not docked to a clear
    /// fleet slot round it, at rest. A docked escort stays at its dock. The
    /// berth turns the ship and points the rigs.
    fn move_beside_the_station(world: &mut World, ship: Entity, call: &ShipRescueCall) {
        let from = world
            .get::<Position>(ship)
            .unwrap_or_else(|| panic!("answer_rescue_calls: ship {ship:?} has no Position"))
            .0;
        let spawn = call.spawn.to_engine();
        world.entity_mut(ship).insert((
            Position(spawn),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
        ));
        shift_chase_cameras(world, spawn - from);
        let escorts: Vec<(Entity, Vec3)> = world
            .query_filtered::<(Entity, &AIFollow), (With<PlayerFleetMarker>, Without<DockedShip>)>()
            .iter(world)
            .filter(|(escort, _)| *escort != ship)
            .map(|(escort, follow)| (escort, follow.side))
            .collect();
        let radius = |world: &World, body: Entity| {
            world
                .get::<HullEnvelopeRadius>(body)
                .map_or(0.0, |radius| radius.0)
        };
        let ship_radius = radius(world, ship);
        for (escort, side) in escorts {
            let escort_radius = radius(world, escort);
            let bodies: Vec<(Vec3, f32)> = world
                .query::<(Entity, &Position, &HullEnvelopeRadius)>()
                .iter(world)
                .filter(|(body, ..)| *body != escort)
                .map(|(_, at, radius)| (at.0, radius.0))
                .collect();
            let base = ship_radius + escort_radius + AI_FOLLOW_GAP;
            let slots = (0..FLEET_SLOT_STEPS)
                .map(|step| spawn + side * (base + step as f32 * AI_FOLLOW_GAP));
            let clear = |at: &Vec3| {
                bodies.iter().all(|(other, other_radius)| {
                    at.distance(*other) > other_radius + escort_radius + AI_FOLLOW_GAP
                })
            };
            let at = slots.clone().find(clear).unwrap_or_else(|| {
                let last = slots.last().expect("FLEET_SLOT_STEPS is not zero");
                warn!(
                    "answer_rescue_calls: no clear fleet slot for escort {escort:?}; it takes \
                     the outermost"
                );
                last
            });
            world.entity_mut(escort).insert((
                Position(at),
                LinearVelocity::ZERO,
                AngularVelocity::ZERO,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::fixture_world, OPEN_WORLD_SECTOR_EDGE};

    const SEED: u32 = 20_260_923;

    /// The nearest station answers; once its civilization turns Enemy none
    /// of its stations answers, and a neutralized station never does.
    #[test]
    fn a_rescue_is_answered_by_the_nearest_living_station_that_is_not_hostile() {
        let mut world = World::new();
        world.insert_resource(WorldConfig {
            seed: SEED,
            sector_edge: OPEN_WORLD_SECTOR_EDGE,
            active_radius: 1,
            generator: fixture_world(),
        });
        world.init_resource::<CivilizationStances>();
        world.init_resource::<WorldClock>();
        let find = |world: &mut World| {
            find_rescue_station(world, Meters3::ZERO)
                .expect("the fixture block generates")
                .expect("a station answers")
        };

        let nearest = find(&mut world);
        let hostile = nearest.civilization;
        world.resource_mut::<CivilizationStances>().set(
            hostile,
            seeded_stance(&hostile),
            StanceType::Enemy,
            0.0,
        );
        let next = find(&mut world);
        assert_ne!(next.civilization, hostile, "a hostile station answered");

        world.spawn((next.id.clone(), NeutralizedMarker));
        let after = find(&mut world);
        assert_ne!(after.id, next.id, "a neutralized station answered");
        assert_ne!(after.civilization, hostile);
    }

    /// A refused answer leaves the credits and the hold as they were, an
    /// angered Neutral station in play refuses like an Enemy one, and an
    /// admitted answer takes the bill once: the hold parts, then the fee
    /// and the section's credits, with a `Rebuilt` report of what it took.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_refused_rescue_takes_nothing_and_an_answer_takes_the_bill_once() {
        use avian3d::prelude::Position;
        use bevy::ecs::message::Messages;
        use nova_events::prelude::SectorCoord;
        use nova_gameplay::prelude::{
            Allegiance, BuildRecipe, ItemDesignId, RetaliationTarget, ShipCredits, ShipInventory,
            ShipInventoryReport, ShipInventoryReports, SpaceshipRootMarker, StationMarker,
            RESCUE_FEE_CR, RESCUE_SECS,
        };
        use nova_scenario::prelude::StationBerth;
        use nova_ship::prelude::{
            BaseSectionConfig, DesignSection, HullSectionConfig, RescueCallEnded, RescueEndType,
            RescueRebuild, SectionConfig, SectionKind, ShipDesignSections, ShipRescueCall,
        };

        let items = nova_gameplay::test_support::test_items();
        let panel = ItemDesignId::from("SectionPanel");
        let mut world = World::new();
        world.insert_resource(items.clone());
        world.init_resource::<CivilizationStances>();
        world.init_resource::<WorldClock>();
        world.init_resource::<Messages<RescueCallEnded>>();
        let station = GeneratedBodyId::new(SectorCoord::new(9, 9, 9), "cluster_station", 0);
        let civilization = CivilizationId {
            world_seed: SEED,
            node: [0, 0, 0],
        };
        let stance = |world: &mut World, stance| {
            world.resource_mut::<CivilizationStances>().set(
                civilization,
                seeded_stance(&civilization),
                stance,
                0.0,
            );
        };
        let spawn = Meters3::from_engine(Vec3::new(100.0, 0.0, 0.0));
        let plate = DesignSection {
            id: "plate".to_string(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            config: SectionConfig {
                base: BaseSectionConfig {
                    id: "plate".to_string(),
                    build: BuildRecipe {
                        credits: 50,
                        parts: [(panel.clone(), 2)].into(),
                        strip_yield: 0.5,
                    },
                    ..default()
                },
                kind: SectionKind::Hull(HullSectionConfig::default()),
            },
        };
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                ShipDesignSections(vec![plate]),
                ShipCredits(1_000),
                ShipInventory::new(&items, 1_000_000, [(panel.clone(), 3)]),
                ShipInventoryReports::default(),
            ))
            .id();
        let call = |world: &mut World| {
            let mut call = ShipRescueCall::new(
                station.clone(),
                civilization,
                spawn,
                vec![RescueRebuild {
                    section: "plate".to_string(),
                    code: "HULL-1".to_string(),
                }],
            );
            call.elapsed_secs = RESCUE_SECS;
            world.entity_mut(ship).insert(call);
            answer_rescue_calls(world);
            world
                .resource_mut::<Messages<RescueCallEnded>>()
                .drain()
                .map(|ended| ended.end)
                .collect::<Vec<_>>()
        };
        let state = |world: &World| {
            (
                world.get::<ShipCredits>(ship).unwrap().0,
                world.get::<ShipInventory>(ship).unwrap().count(&panel),
                world.get::<StationBerth>(ship).is_some(),
            )
        };

        stance(&mut world, StanceType::Enemy);
        assert_eq!(call(&mut world), vec![RescueEndType::Refused]);
        assert_eq!(state(&world), (1_000, 3, false));

        // A Neutral station in play that answers the player's fire.
        stance(&mut world, StanceType::Neutral);
        let angered = world
            .spawn((
                station.clone(),
                SpaceshipRootMarker,
                StationMarker,
                Allegiance::Neutral,
                RetaliationTarget(Some(ship)),
            ))
            .id();
        assert_eq!(call(&mut world), vec![RescueEndType::Refused]);
        assert_eq!(state(&world), (1_000, 3, false));

        world.entity_mut(angered).insert(RetaliationTarget(None));
        let paid_cr = RESCUE_FEE_CR + 50;
        assert_eq!(call(&mut world), vec![RescueEndType::Answered { paid_cr }]);
        assert_eq!(state(&world), (1_000 - paid_cr, 1, true));
        assert_eq!(
            world.get::<Position>(ship).unwrap().0,
            spawn.to_engine(),
            "the ship waits at the station's spawn point"
        );
        let rebuilt: Vec<_> = world
            .get::<ShipInventoryReports>(ship)
            .unwrap()
            .0
            .iter()
            .filter_map(|report| match report {
                ShipInventoryReport::Rebuilt {
                    section,
                    parts,
                    price_cr,
                } => Some((section.clone(), parts.clone(), *price_cr)),
                _ => None,
            })
            .collect();
        assert_eq!(
            rebuilt,
            vec![("HULL-1".to_string(), [(panel.clone(), 2)].into(), 50)]
        );

        // The call is gone: a second answer takes nothing.
        answer_rescue_calls(&mut world);
        assert_eq!(state(&world), (1_000 - paid_cr, 1, true));
    }
}
