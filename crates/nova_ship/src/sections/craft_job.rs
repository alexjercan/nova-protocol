//! The craft job a ship runs over game time: N batches of one recipe.
//!
//! The Craft view checks a start by [`plan_craft_start`] and inserts
//! [`ShipCraftJob`] on the ship root; a ship runs at most one. Nothing is
//! taken at the start. [`advance_ship_craft_jobs`] runs the job on the game
//! clock, so the TAB interface and the pause menu, which stop
//! `Time<Virtual>`, hold it. A ship flies and fights while it crafts.
//!
//! At the end of each batch the tick plans it again by [`plan_craft_batch`]:
//! a pass takes the inputs from the hold (less what running repair jobs hold
//! back), adds the output and, for a station recipe, moves the fee from the
//! ship to the station, reported on both as a
//! [`CreditChangeCauseType::Craft`] change. A refusal stops the job with the
//! reason; finished batches stay. A station recipe runs only while the ship
//! is docked at a living station that offers Craft ([`CraftStations`]); at
//! any other time its batch holds. A batch end at such a station below the
//! recipe's advancement floor stops the job: a start there is refused and the
//! save check refuses a Load that would dock one there, so only a broken
//! world reaches it. The undock itself (the ship's
//! [`DockedShip`] going away while the ship stays) stops it. A Load spawns
//! the ship before it rebuilds the dock, so the job holds over those frames
//! and runs on once the dock is back; a poll for the dock would stop it
//! there. A docked station that no longer offers Craft (it turned derelict
//! when its last turret died; the dock holds) stops the job at once, as the
//! undock does: [`CraftStations::docked_without_craft`] reads that from the
//! station itself, so a dock not rebuilt yet still holds. A cancel
//! ([`CraftJobCancelRequest`]) keeps finished batches and takes nothing more.
//! Nothing is ever held by a running job, so nothing is refunded.
//!
//! Every end writes one [`CraftJobEnded`] for the HUD notice and the pane
//! note. A save keeps the job as it is: it names its recipe by id and holds
//! no entity.
//!
//! [`plan_craft_start`]: nova_gameplay::prelude::plan_craft_start
//! [`plan_craft_batch`]: nova_gameplay::prelude::plan_craft_batch
//! [`CreditChangeCauseType::Craft`]: nova_gameplay::prelude::CreditChangeCauseType::Craft

use bevy::{ecs::system::SystemParam, prelude::*};
use nova_gameplay::prelude::{
    plan_craft_batch, station_offers, CraftRefusalType, CraftSiteType, CraftStationSite,
    CreditChangeCauseType, DerelictShipMarker, GameItems, GameRecipes, RecipeDesign,
    RecipeDesignId, ShipCredits, ShipInventory, ShipInventoryReports, SpaceshipRootMarker,
    StationAdvancement, StationMarker, StationServiceType, StationServices,
};

use super::prelude::{DockedShip, DockingConnection, ShipRepairJobs};

/// The craft job, its tick, cancel and end.
pub mod prelude {
    pub use super::{
        advance_ship_craft_jobs, stop_station_craft_on_undock, CraftJobCancelRequest,
        CraftJobEndType, CraftJobEnded, CraftJobSystems, CraftStations, ShipCraftJob,
    };
}

/// A running craft job on a ship root: `queued` batches of `recipe`.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShipCraftJob {
    /// The recipe.
    pub recipe: RecipeDesignId,
    /// Batches asked for, above zero.
    pub queued: u32,
    /// Batches finished, below `queued` while the job runs.
    pub done: u32,
    /// Game seconds run in the current batch, below the recipe's
    /// `duration_secs`.
    pub elapsed_secs: f32,
}

impl ShipCraftJob {
    /// A fresh job of `batches` batches of `recipe`.
    pub fn new(recipe: RecipeDesignId, batches: u32) -> Self {
        Self {
            recipe,
            queued: batches,
            done: 0,
            elapsed_secs: 0.0,
        }
    }

    /// The share of the whole job done, in `[0, 1]`, with `recipe` its
    /// recipe.
    pub fn progress(&self, recipe: &RecipeDesign) -> f32 {
        let done = self.done as f32 + self.elapsed_secs / recipe.duration_secs;
        (done / self.queued.max(1) as f32).clamp(0.0, 1.0)
    }

    /// Game seconds left if nothing stops the job, never below zero.
    pub fn remaining_secs(&self, recipe: &RecipeDesign) -> f32 {
        (self.queued.saturating_sub(self.done) as f32 * recipe.duration_secs - self.elapsed_secs)
            .max(0.0)
    }
}

/// The living stations that offer Craft, as a docked ship sees them.
#[derive(SystemParam)]
pub struct CraftStations<'w, 's> {
    connections: Query<'w, 's, &'static DockingConnection>,
    stations: Query<
        'w,
        's,
        (
            &'static StationServices,
            Option<&'static StationAdvancement>,
            Has<DerelictShipMarker>,
        ),
        With<StationMarker>,
    >,
}

impl CraftStations<'_, '_> {
    /// The station `ship` is docked at, with its advancement, when it is a
    /// living station that offers Craft. `docked` is the ship's
    /// [`DockedShip`].
    ///
    /// # Panics
    ///
    /// When that station carries no [`StationAdvancement`]: every station
    /// root spawns and thaws with one.
    pub fn docked_at(&self, ship: Entity, docked: Option<&DockedShip>) -> Option<(Entity, f32)> {
        let partner = self.partner(ship, docked?)?;
        let (services, advancement, derelict) = self.stations.get(partner).ok()?;
        if !station_offers(Some(services), derelict, StationServiceType::Craft) {
            return None;
        }
        let Some(advancement) = advancement else {
            panic!("station {partner:?} offers Craft but carries no StationAdvancement");
        };
        Some((partner, advancement.0))
    }

    /// Whether `ship` is docked at a station that does not offer Craft now:
    /// a derelict one, or one without the service. A dock not rebuilt yet
    /// after a Load (no [`DockedShip`], no connection, no station) is not
    /// such a dock.
    pub fn docked_without_craft(&self, ship: Entity, docked: Option<&DockedShip>) -> bool {
        docked
            .and_then(|docked| self.partner(ship, docked))
            .and_then(|partner| self.stations.get(partner).ok())
            .is_some_and(|(services, _, derelict)| {
                !station_offers(Some(services), derelict, StationServiceType::Craft)
            })
    }

    /// The other root of `ship`'s dock, while its connection exists.
    fn partner(&self, ship: Entity, docked: &DockedShip) -> Option<Entity> {
        let connection = self.connections.get(docked.connection).ok()?;
        Some(if connection.first_ship == ship {
            connection.second_ship
        } else {
            connection.first_ship
        })
    }
}

/// Asks to cancel the craft job of ship `entity`. Finished batches stay;
/// nothing more is taken. A ship with no job does nothing.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct CraftJobCancelRequest {
    /// The ship root.
    pub entity: Entity,
}

/// A craft job of ship `ship` ended.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct CraftJobEnded {
    /// The ship root.
    pub ship: Entity,
    /// The recipe it ran.
    pub recipe: RecipeDesignId,
    /// How it ended.
    pub end: CraftJobEndType,
}

/// How a [`ShipCraftJob`] ended.
#[derive(Clone, Debug, PartialEq)]
pub enum CraftJobEndType {
    /// Every queued batch finished.
    Crafted {
        /// Batches finished.
        batches: u32,
    },
    /// A batch end refused; the batches finished before it stay.
    Stopped {
        /// Batches finished.
        batches: u32,
        /// Why it stopped.
        reason: CraftRefusalType,
    },
    /// The player cancelled the job; the batches finished before it stay.
    Cancelled {
        /// Batches finished.
        batches: u32,
    },
}

/// System set for the craft job tick, in `Update`, after
/// `RepairJobSystems`: both write the hold, and a craft batch reads what the
/// repair jobs left and hold back.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CraftJobSystems;

/// Run every ship's craft job for this frame's game time. See the module
/// docs for what a batch end and a stop do.
///
/// # Panics
///
/// When a job names a recipe the loaded catalog lacks, or runs before the
/// recipe and item catalogs loaded: the pane starts a job only from a loaded
/// recipe, and the save check refuses an unknown one.
#[expect(
    clippy::type_complexity,
    reason = "the ship's job, hold, balance, reports, repair jobs and dock"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "the catalogs, the ships, the stations and the end message"
)]
pub fn advance_ship_craft_jobs(
    time: Res<Time>,
    items: Option<Res<GameItems>>,
    recipes: Option<Res<GameRecipes>>,
    mut commands: Commands,
    mut q_ship: Query<
        (
            Entity,
            &mut ShipCraftJob,
            &mut ShipInventory,
            &mut ShipCredits,
            &mut ShipInventoryReports,
            Option<&ShipRepairJobs>,
            Option<&DockedShip>,
        ),
        (With<SpaceshipRootMarker>, Without<StationMarker>),
    >,
    sites: CraftStations,
    mut q_station: Query<
        (&mut ShipCredits, &mut ShipInventoryReports),
        (With<StationMarker>, Without<ShipCraftJob>),
    >,
    mut ended: MessageWriter<CraftJobEnded>,
) {
    let dt = time.delta_secs();
    for (ship, mut job, mut hold, mut credits, mut reports, repairs, docked) in &mut q_ship {
        let recipes = recipes
            .as_deref()
            .expect("craft job: a job runs only after the recipe catalog loads");
        let items = items
            .as_deref()
            .expect("craft job: a job runs only after the item catalog loads");
        let recipe = recipes.design(&job.recipe);
        let station = sites.docked_at(ship, docked);
        let mut end = None;
        if matches!(recipe.site, CraftSiteType::Station { .. }) && station.is_none() {
            if !sites.docked_without_craft(ship, docked) {
                continue;
            }
            end = Some(CraftJobEndType::Stopped {
                batches: job.done,
                reason: CraftRefusalType::NotAtCraftStation,
            });
        } else {
            job.elapsed_secs += dt;
        }
        while end.is_none() && job.elapsed_secs >= recipe.duration_secs {
            job.elapsed_secs -= recipe.duration_secs;
            let free = match repairs {
                Some(repairs) => repairs.free_hold(&hold),
                None => hold.clone(),
            };
            let site = station.and_then(|(station, advancement)| {
                q_station.get(station).ok().map(|(cr, _)| CraftStationSite {
                    advancement,
                    credits: cr.0,
                })
            });
            match plan_craft_batch(recipe, site, &free, credits.0) {
                Ok(batch) => {
                    for (item, count) in &batch.inputs {
                        hold.remove(item, *count);
                    }
                    hold.add(items, &batch.output, batch.count);
                    reports.change_credits(
                        &mut credits,
                        batch.own_cr,
                        CreditChangeCauseType::Craft,
                    );
                    if let (Some((station, _)), Some(station_cr)) = (station, batch.station_cr) {
                        let (mut station_credits, mut station_reports) = q_station
                            .get_mut(station)
                            .expect("craft job: the Craft station was read this batch");
                        station_reports.change_credits(
                            &mut station_credits,
                            station_cr,
                            CreditChangeCauseType::Craft,
                        );
                    }
                    job.done += 1;
                    if job.done >= job.queued {
                        end = Some(CraftJobEndType::Crafted { batches: job.done });
                    }
                }
                Err(reason) => {
                    end = Some(CraftJobEndType::Stopped {
                        batches: job.done,
                        reason,
                    });
                }
            }
        }
        let Some(end) = end else {
            continue;
        };
        debug!("craft job: {} on {ship:?} ended: {end:?}", job.recipe);
        commands.entity(ship).remove::<ShipCraftJob>();
        ended.write(CraftJobEnded {
            ship,
            recipe: job.recipe.clone(),
            end,
        });
    }
}

/// Stop the station craft job of a ship that undocked, with its finished
/// batches. Checked in a command, so a ship despawning (which also drops its
/// [`DockedShip`]) ends nothing, and a dock replaced in the same flush stops
/// nothing.
pub fn stop_station_craft_on_undock(undock: On<Remove<DockedShip>>, mut commands: Commands) {
    let ship = undock.entity;
    commands.queue(move |world: &mut World| {
        let Ok(entity) = world.get_entity(ship) else {
            return;
        };
        if entity.contains::<DockedShip>() {
            return;
        }
        let Some(job) = entity.get::<ShipCraftJob>() else {
            return;
        };
        let at_station = world
            .get_resource::<GameRecipes>()
            .and_then(|recipes| recipes.get(&job.recipe))
            .is_some_and(|recipe| matches!(recipe.site, CraftSiteType::Station { .. }));
        if !at_station {
            return;
        }
        let Some(job) = world.entity_mut(ship).take::<ShipCraftJob>() else {
            return;
        };
        debug!(
            "craft job: {} on {ship:?} stopped by the undock",
            job.recipe
        );
        world.write_message(CraftJobEnded {
            ship,
            recipe: job.recipe,
            end: CraftJobEndType::Stopped {
                batches: job.done,
                reason: CraftRefusalType::NotAtCraftStation,
            },
        });
    });
}

/// Cancel the job a [`CraftJobCancelRequest`] names: end it as cancelled
/// with its finished batches.
pub(crate) fn cancel_craft_job(request: On<CraftJobCancelRequest>, mut commands: Commands) {
    let ship = request.entity;
    // Take the job and write the end in one command, so two cancels before a
    // flush end the job once.
    commands.queue(move |world: &mut World| {
        let taken = world
            .get_entity_mut(ship)
            .ok()
            .and_then(|mut entity| entity.take::<ShipCraftJob>());
        if let Some(job) = taken {
            world.write_message(CraftJobEnded {
                ship,
                recipe: job.recipe,
                end: CraftJobEndType::Cancelled { batches: job.done },
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{ecs::message::Messages, time::TimeUpdateStrategy};
    use nova_gameplay::{prelude::ItemDesignId, test_support::test_items};

    use super::*;
    use crate::sections::prelude::DockedHelmType;

    /// The base recipes the tests run: PDC rounds on the ship, 8 s a batch,
    /// and the alloy plate at a station for 30 cr, 10 s a batch.
    fn recipes() -> GameRecipes {
        let recipe =
            |id: &str, inputs: &[(&str, u32)], output: (&str, u32), secs, site| RecipeDesign {
                id: RecipeDesignId::from(id),
                inputs: inputs
                    .iter()
                    .map(|(item, count)| (ItemDesignId::from(*item), *count))
                    .collect(),
                output: (ItemDesignId::from(output.0), output.1),
                duration_secs: secs,
                site,
            };
        GameRecipes::new([
            recipe(
                "base/pdc_round",
                &[("IronOre", 1), ("StoneOre", 1)],
                ("PdcRound", 100),
                8.0,
                CraftSiteType::Ship,
            ),
            recipe(
                "base/alloy_plate",
                &[("IronOre", 2), ("CarbonOre", 1)],
                ("AlloyPlate", 1),
                10.0,
                CraftSiteType::Station {
                    fee_cr: 30,
                    advancement: 0.25,
                },
            ),
        ])
    }

    /// A ship with `stacks` in its hold and 100 cr, its clock stepping
    /// `step` game seconds a frame after the warm-up frame whose delta is
    /// zero.
    fn craft_app(step: f32, stacks: &[(&str, u32)]) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            step,
        )));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(60));
        app.insert_resource(test_items());
        app.insert_resource(recipes());
        app.add_message::<CraftJobEnded>();
        app.add_observer(cancel_craft_job);
        app.add_observer(stop_station_craft_on_undock);
        app.add_systems(Update, advance_ship_craft_jobs);
        let items = test_items();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipInventory::new(
                    &items,
                    1_000_000,
                    stacks
                        .iter()
                        .map(|(item, count)| (ItemDesignId::from(*item), *count)),
                ),
                ShipCredits(100),
            ))
            .id();
        app.update();
        (app, ship)
    }

    fn count(app: &App, ship: Entity, item: &str) -> u32 {
        app.world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(&ItemDesignId::from(item))
    }

    fn ends(app: &mut App) -> Vec<CraftJobEndType> {
        app.world_mut()
            .resource_mut::<Messages<CraftJobEnded>>()
            .drain()
            .map(|ended| ended.end)
            .collect()
    }

    fn start(app: &mut App, ship: Entity, recipe: &str, batches: u32) {
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipCraftJob::new(RecipeDesignId::from(recipe), batches));
    }

    /// Each batch end takes one batch of inputs and adds the output; the
    /// start takes nothing, and the job ends once every batch is done.
    #[test]
    fn a_ship_recipe_takes_its_inputs_per_batch_on_the_game_clock() {
        let (mut app, ship) = craft_app(8.0, &[("IronOre", 5), ("StoneOre", 5)]);
        start(&mut app, ship, "base/pdc_round", 3);
        for batch in 1..=3u32 {
            app.update();
            assert_eq!(count(&app, ship, "IronOre"), 5 - batch);
            assert_eq!(count(&app, ship, "PdcRound"), 100 * batch);
        }
        assert!(app.world().get::<ShipCraftJob>(ship).is_none());
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Crafted { batches: 3 }]
        );
        assert_eq!(
            app.world().get::<ShipCredits>(ship),
            Some(&ShipCredits(100))
        );
    }

    /// The TAB interface and the pause menu stop `Time<Virtual>`; the job
    /// holds while they do.
    #[test]
    fn a_craft_job_does_not_progress_while_the_clock_is_frozen() {
        let (mut app, ship) = craft_app(8.0, &[("IronOre", 5), ("StoneOre", 5)]);
        start(&mut app, ship, "base/pdc_round", 2);
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(count(&app, ship, "PdcRound"), 0);
        assert_eq!(
            app.world().get::<ShipCraftJob>(ship).unwrap().elapsed_secs,
            0.0
        );
        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.update();
        assert_eq!(count(&app, ship, "PdcRound"), 100);
    }

    /// A hold that runs out of an input stops the job at the next batch end
    /// with the finished batches kept; a cancel keeps them and takes no more.
    #[test]
    fn a_craft_job_stops_on_a_short_input_and_a_cancel_keeps_finished_batches() {
        let (mut app, ship) = craft_app(8.0, &[("IronOre", 5), ("StoneOre", 1)]);
        start(&mut app, ship, "base/pdc_round", 3);
        app.update();
        app.update();
        assert_eq!(count(&app, ship, "PdcRound"), 100);
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Stopped {
                batches: 1,
                reason: CraftRefusalType::ShortInput {
                    item: ItemDesignId::from("StoneOre"),
                    need: 1,
                    have: 0,
                },
            }]
        );

        let (mut app, ship) = craft_app(5.0, &[("IronOre", 5), ("StoneOre", 5)]);
        start(&mut app, ship, "base/pdc_round", 3);
        app.update();
        app.update();
        app.world_mut()
            .trigger(CraftJobCancelRequest { entity: ship });
        app.world_mut()
            .trigger(CraftJobCancelRequest { entity: ship });
        app.update();
        assert_eq!(count(&app, ship, "PdcRound"), 100);
        assert_eq!(count(&app, ship, "IronOre"), 4);
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Cancelled { batches: 1 }]
        );
    }

    /// Docks `ship` at a new living Craft station of `advancement` with
    /// 1000 cr; returns the station.
    fn dock_at_craft_station(app: &mut App, ship: Entity, advancement: f32) -> Entity {
        let station = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                StationMarker,
                StationServices(vec![StationServiceType::Craft]),
                StationAdvancement(advancement),
                ShipCredits(1_000),
            ))
            .id();
        let connection = app
            .world_mut()
            .spawn(DockingConnection {
                first_ship: ship,
                first_section: Entity::PLACEHOLDER,
                second_ship: station,
                second_section: Entity::PLACEHOLDER,
                helm: DockedHelmType::Station(station),
                measurement_fault: false,
            })
            .id();
        app.world_mut().entity_mut(ship).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives: false,
        });
        station
    }

    /// A station recipe crafts while docked at a living Craft station and
    /// pays it the fee; the undock stops it with the finished batches kept.
    #[test]
    fn a_station_recipe_pays_the_station_and_stops_on_undock() {
        let (mut app, ship) = craft_app(10.0, &[("IronOre", 6), ("CarbonOre", 3)]);
        let station = dock_at_craft_station(&mut app, ship, 0.25);
        start(&mut app, ship, "base/alloy_plate", 3);
        app.update();
        assert_eq!(count(&app, ship, "AlloyPlate"), 1);
        assert_eq!(app.world().get::<ShipCredits>(ship), Some(&ShipCredits(70)));
        assert_eq!(
            app.world().get::<ShipCredits>(station),
            Some(&ShipCredits(1_030))
        );

        app.world_mut().entity_mut(ship).remove::<DockedShip>();
        app.update();
        assert_eq!(count(&app, ship, "AlloyPlate"), 1);
        assert_eq!(count(&app, ship, "IronOre"), 4);
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Stopped {
                batches: 1,
                reason: CraftRefusalType::NotAtCraftStation,
            }]
        );
    }

    /// A Load spawns the ship with its job before it rebuilds the dock: the
    /// station batch holds while the ship has no dock and runs on, from
    /// where it was saved, once the dock is back.
    #[test]
    fn a_station_job_holds_until_the_dock_is_rebuilt() {
        let (mut app, ship) = craft_app(4.0, &[("IronOre", 6), ("CarbonOre", 3)]);
        app.world_mut().entity_mut(ship).insert(ShipCraftJob {
            elapsed_secs: 7.0,
            ..ShipCraftJob::new(RecipeDesignId::from("base/alloy_plate"), 1)
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<ShipCraftJob>(ship)
                .map(|job| job.elapsed_secs),
            Some(7.0)
        );
        assert!(ends(&mut app).is_empty());
        dock_at_craft_station(&mut app, ship, 0.25);
        app.update();
        assert_eq!(count(&app, ship, "AlloyPlate"), 1);
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Crafted { batches: 1 }]
        );
    }

    /// A docked station that turns derelict mid job (its last turret died;
    /// the dock holds) stops the job at once, as an undock does: the
    /// finished batch stays and the unfinished one takes nothing.
    #[test]
    fn a_station_job_stops_when_its_docked_station_turns_derelict() {
        let (mut app, ship) = craft_app(10.0, &[("IronOre", 6), ("CarbonOre", 3)]);
        let station = dock_at_craft_station(&mut app, ship, 0.25);
        start(&mut app, ship, "base/alloy_plate", 3);
        app.update();
        assert_eq!(count(&app, ship, "AlloyPlate"), 1);
        assert!(ends(&mut app).is_empty());

        app.world_mut()
            .entity_mut(station)
            .insert(DerelictShipMarker);
        app.update();
        assert!(app.world().get::<DockedShip>(ship).is_some());
        assert!(app.world().get::<ShipCraftJob>(ship).is_none());
        assert_eq!(count(&app, ship, "AlloyPlate"), 1);
        assert_eq!(count(&app, ship, "IronOre"), 4);
        assert_eq!(app.world().get::<ShipCredits>(ship), Some(&ShipCredits(70)));
        assert_eq!(
            app.world().get::<ShipCredits>(station),
            Some(&ShipCredits(1_030))
        );
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Stopped {
                batches: 1,
                reason: CraftRefusalType::NotAtCraftStation,
            }]
        );
    }

    /// A station job docked at a station below the recipe's advancement
    /// stops at its batch end and takes nothing: no input, no fee.
    #[test]
    fn a_station_job_at_a_station_below_its_advancement_stops_and_takes_nothing() {
        let (mut app, ship) = craft_app(10.0, &[("IronOre", 6), ("CarbonOre", 3)]);
        let station = dock_at_craft_station(&mut app, ship, 0.2);
        start(&mut app, ship, "base/alloy_plate", 2);
        app.update();
        assert_eq!(count(&app, ship, "AlloyPlate"), 0);
        assert_eq!(count(&app, ship, "IronOre"), 6);
        assert_eq!(
            app.world().get::<ShipCredits>(ship),
            Some(&ShipCredits(100))
        );
        assert_eq!(
            app.world().get::<ShipCredits>(station),
            Some(&ShipCredits(1_000))
        );
        assert_eq!(
            ends(&mut app),
            vec![CraftJobEndType::Stopped {
                batches: 0,
                reason: CraftRefusalType::NotAtCraftStation,
            }]
        );
    }
}
