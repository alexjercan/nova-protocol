//! Repair work that runs on a ship over game time: plate repair jobs on
//! damaged sections and field rebuilds of lost ones.
//!
//! [`ShipRepairJobs`] sits on a ship root and holds every running job. The
//! Ship pane plans a job and pushes it; [`advance_ship_repair_jobs`] runs the
//! jobs on the game clock, so the TAB interface and the pause menu, which
//! stop `Time<Virtual>`, hold them. A ship flies and fights while its jobs
//! run.
//!
//! A plate job fits one hull plate at the end of each [`PLATE_REPAIR_SECS`]
//! interval by the [`plan_plate_step`] rule: it spends one plate and adds
//! [`HULL_PLATE_HEALTH`]. Nothing is spent when the job starts, so a cancel,
//! a reload or a lost section never loses an unspent plate. The job stops
//! with a reason when the section is destroyed, full or the hold has no
//! plate. Jobs on different sections run side by side; a section has at
//! most one job.
//!
//! A field rebuild builds a lost, anchored design section away from any
//! station. When its [`field_rebuild_duration_secs`] end, it checks the
//! section again by [`plan_section_rebuild`] at [`RebuildSiteType::Field`],
//! takes the recipe's parts from the hold and raises the
//! [`SectionRebuildRequest`] a station rebuild raises. A refusal at the end
//! takes nothing.
//!
//! Every end writes one [`RepairJobEnded`] for the HUD notice and the pane
//! note. A plate job that spent a plate queues one `Repaired` report; a
//! field rebuild queues one `Rebuilt` report at no price. Jobs are not
//! saved: a reload drops them, and nothing was spent early.
//!
//! [`PLATE_REPAIR_SECS`]: nova_gameplay::prelude::PLATE_REPAIR_SECS
//! [`plan_plate_step`]: nova_gameplay::prelude::plan_plate_step
//! [`HULL_PLATE_HEALTH`]: nova_gameplay::prelude::HULL_PLATE_HEALTH
//! [`field_rebuild_duration_secs`]: nova_gameplay::prelude::field_rebuild_duration_secs
//! [`plan_section_rebuild`]: nova_gameplay::prelude::plan_section_rebuild
//! [`RebuildSiteType::Field`]: nova_gameplay::prelude::RebuildSiteType::Field

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{
    field_rebuild_duration_secs, plan_plate_step, plan_section_rebuild, BuildRecipe, GameItems,
    Health, IntegrityDisabledMarker, ItemDesignId, ItemRoleType, PlateRepairRefusalType,
    RebuildSiteType, SectionMarker, SectionRebuildRefusalType, ShipInventory, ShipInventoryReport,
    ShipInventoryReports, SpaceshipRootMarker, PLATE_REPAIR_SECS,
};

use super::prelude::{
    lost_section_facts, PlacedSectionLinkPoints, SectionLinkPoints, SectionRebuildRequest,
    ShipDesignSections,
};

/// The repair jobs, their tick, cancel and end.
pub mod prelude {
    pub use super::{
        advance_ship_repair_jobs, RepairJob, RepairJobCancelRequest, RepairJobEndType,
        RepairJobEnded, RepairJobKey, RepairJobSystems, RepairWorkType, ShipRepairJobs,
    };
}

/// The running repair jobs of a ship, in start order, on its root.
///
/// Inserted by the Ship pane with the first job. A section has at most one
/// job: [`ShipRepairJobs::push`] refuses a second.
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ShipRepairJobs(pub Vec<RepairJob>);

/// One running repair job.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct RepairJob {
    /// Pane code of the section (`HULL-3`), for the pane, the HUD and the
    /// reports.
    pub code: String,
    /// What the job does.
    pub work: RepairWorkType,
    /// Game seconds run in the current interval of a plate job, or in the
    /// whole of a field rebuild.
    pub elapsed_secs: f32,
}

/// What a [`RepairJob`] does.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum RepairWorkType {
    /// Fit `queued` hull plates to the live section `section`, one per
    /// [`PLATE_REPAIR_SECS`].
    Plates {
        /// The section, a child of the ship root.
        section: Entity,
        /// Plates the player queued; above zero.
        queued: u32,
        /// Plates fitted so far.
        spent: u32,
    },
    /// Build the lost design section `section` from `parts` in the hold.
    FieldRebuild {
        /// The design section id.
        section: String,
        /// The recipe's parts, held back from other jobs until the end.
        parts: BTreeMap<ItemDesignId, u32>,
        /// Game seconds the rebuild takes.
        duration_secs: f32,
    },
}

/// Which section a job works on: the identity a cancel names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepairJobKey {
    /// A plate job on this live section.
    Section(Entity),
    /// A field rebuild of this lost design section id.
    Lost(String),
}

impl RepairJob {
    /// A plate job fitting `queued` plates to `section`, coded `code`.
    pub fn plates(section: Entity, code: String, queued: u32) -> Self {
        Self {
            code,
            work: RepairWorkType::Plates {
                section,
                queued,
                spent: 0,
            },
            elapsed_secs: 0.0,
        }
    }

    /// A field rebuild of design section `section`, coded `code`, built by
    /// `recipe`: its parts, over [`field_rebuild_duration_secs`].
    pub fn field_rebuild(section: String, code: String, recipe: &BuildRecipe) -> Self {
        Self {
            code,
            work: RepairWorkType::FieldRebuild {
                section,
                parts: recipe.parts.clone(),
                duration_secs: field_rebuild_duration_secs(recipe),
            },
            elapsed_secs: 0.0,
        }
    }

    /// The section this job works on.
    pub fn key(&self) -> RepairJobKey {
        match &self.work {
            RepairWorkType::Plates { section, .. } => RepairJobKey::Section(*section),
            RepairWorkType::FieldRebuild { section, .. } => RepairJobKey::Lost(section.clone()),
        }
    }

    /// The share of the whole job done, in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        match &self.work {
            RepairWorkType::Plates { queued, spent, .. } => {
                let done = *spent as f32 + self.elapsed_secs / PLATE_REPAIR_SECS;
                (done / (*queued).max(1) as f32).clamp(0.0, 1.0)
            }
            RepairWorkType::FieldRebuild { duration_secs, .. } => {
                (self.elapsed_secs / duration_secs.max(f32::EPSILON)).clamp(0.0, 1.0)
            }
        }
    }

    /// Game seconds left if nothing stops the job, never below zero.
    pub fn remaining_secs(&self) -> f32 {
        match &self.work {
            RepairWorkType::Plates { queued, spent, .. } => {
                (queued.saturating_sub(*spent) as f32 * PLATE_REPAIR_SECS - self.elapsed_secs)
                    .max(0.0)
            }
            RepairWorkType::FieldRebuild { duration_secs, .. } => {
                (duration_secs - self.elapsed_secs).max(0.0)
            }
        }
    }
}

impl ShipRepairJobs {
    /// The job on `key`'s section, if one runs.
    pub fn get(&self, key: &RepairJobKey) -> Option<&RepairJob> {
        self.0.iter().find(|job| job.key() == *key)
    }

    /// Start `job`.
    ///
    /// # Panics
    ///
    /// When a job already runs on the same section: the pane plans a job
    /// only for a section with none.
    pub fn push(&mut self, job: RepairJob) {
        assert!(
            self.get(&job.key()).is_none(),
            "ShipRepairJobs: {} already has a job",
            job.code
        );
        self.0.push(job);
    }

    /// What the hold `own` has left for a new job: `own` less the plates the
    /// plate jobs have still to fit and the parts the field rebuilds hold
    /// back, each at most what the hold carries.
    pub fn free_hold(&self, own: &ShipInventory) -> ShipInventory {
        let mut held_back: BTreeMap<ItemDesignId, u32> = BTreeMap::new();
        for job in &self.0 {
            match &job.work {
                RepairWorkType::Plates { queued, spent, .. } => {
                    *held_back
                        .entry(ItemDesignId::from(ItemRoleType::HullPlate))
                        .or_default() += queued.saturating_sub(*spent);
                }
                RepairWorkType::FieldRebuild { parts, .. } => {
                    for (item, &count) in parts {
                        *held_back.entry(item.clone()).or_default() += count;
                    }
                }
            }
        }
        let mut free = own.clone();
        for (item, count) in held_back {
            let taken = count.min(free.count(&item));
            if taken > 0 {
                free.remove(&item, taken);
            }
        }
        free
    }
}

/// Asks to cancel the job on `key`'s section of ship `entity`. Plates the
/// job already fitted stay fitted and are reported; nothing else is spent.
/// A key with no job does nothing.
#[derive(EntityEvent, Clone, Debug)]
pub struct RepairJobCancelRequest {
    /// The ship root.
    pub entity: Entity,
    /// The section whose job to cancel.
    pub key: RepairJobKey,
}

/// One repair job ended on ship `ship`.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct RepairJobEnded {
    /// The ship root.
    pub ship: Entity,
    /// Pane code of the section (`HULL-3`).
    pub code: String,
    /// How the job ended.
    pub end: RepairJobEndType,
}

/// How a [`RepairJob`] ended.
#[derive(Clone, Debug, PartialEq)]
pub enum RepairJobEndType {
    /// A plate job fitted every queued plate.
    Repaired {
        /// Plates fitted.
        plates: u32,
    },
    /// A field rebuild took its parts and the section spawns again.
    Rebuilt,
    /// A plate job stopped early.
    RepairStopped {
        /// Plates fitted before the stop.
        plates: u32,
        /// Why it stopped.
        reason: PlateRepairRefusalType,
    },
    /// A field rebuild refused at its end; nothing was taken.
    RebuildStopped(SectionRebuildRefusalType),
    /// The player cancelled the job.
    Cancelled {
        /// Plates fitted before the cancel; zero for a field rebuild.
        plates: u32,
    },
}

/// System set for the repair job tick, in `Update`. Runs on the game clock;
/// a pane that starts a job may run before or after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepairJobSystems;

/// The plates a plate job has fitted, or zero for a field rebuild.
fn plates_spent(job: &RepairJob) -> u32 {
    match job.work {
        RepairWorkType::Plates { spent, .. } => spent,
        RepairWorkType::FieldRebuild { .. } => 0,
    }
}

/// Queue the `Repaired` report of a plate job that fitted `plates` plates
/// to section `code`; nothing for none.
fn report_plates(reports: &mut ShipInventoryReports, code: &str, plates: u32) {
    if plates == 0 {
        return;
    }
    reports.0.push(ShipInventoryReport::Repaired {
        section: code.to_string(),
        item: ItemDesignId::from(ItemRoleType::HullPlate),
        count: plates,
    });
}

/// Run every ship's repair jobs for this frame's game time.
///
/// A plate job ends as destroyed as soon as its section is gone, is no
/// longer a section of the ship, is disabled or is at zero Health. At the end
/// of each interval it takes one [`plan_plate_step`] step. A field rebuild
/// ends when its time is up. See the module docs for what each end does.
///
/// # Panics
///
/// When a field rebuild ends on a ship with no [`ShipDesignSections`] or a
/// design without its section, or before the item catalog loaded: the pane
/// starts a field rebuild only for a lost section of a designed ship, after
/// content loaded.
#[expect(
    clippy::type_complexity,
    reason = "the ship's jobs, hold, reports, design and sections"
)]
pub fn advance_ship_repair_jobs(
    time: Res<Time>,
    items: Option<Res<GameItems>>,
    mut commands: Commands,
    mut q_ship: Query<
        (
            Entity,
            &mut ShipRepairJobs,
            &mut ShipInventory,
            &mut ShipInventoryReports,
            Option<&ShipDesignSections>,
            Option<&Children>,
        ),
        With<SpaceshipRootMarker>,
    >,
    mut q_health: Query<(&ChildOf, &mut Health, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
    q_live: Query<(&EntityId, &Transform, &SectionLinkPoints), With<SectionMarker>>,
    mut ended: MessageWriter<RepairJobEnded>,
) {
    let dt = time.delta_secs();
    let plate = ItemDesignId::from(ItemRoleType::HullPlate);
    for (ship, mut jobs, mut own, mut reports, design, children) in &mut q_ship {
        if jobs.0.is_empty() {
            continue;
        }
        let mut index = 0;
        while index < jobs.0.len() {
            let job = &mut jobs.0[index];
            let end = match &mut job.work {
                RepairWorkType::Plates {
                    section,
                    queued,
                    spent,
                } => match q_health.get_mut(*section) {
                    Ok((child, mut health, disabled))
                        if child.parent() == ship && !disabled && health.current > 0.0 =>
                    {
                        job.elapsed_secs += dt;
                        if job.elapsed_secs < PLATE_REPAIR_SECS {
                            None
                        } else {
                            job.elapsed_secs -= PLATE_REPAIR_SECS;
                            match plan_plate_step(Some(&health), disabled, own.count(&plate)) {
                                Ok(current) => {
                                    own.remove(&plate, 1);
                                    health.current = current;
                                    *spent += 1;
                                    (*spent >= *queued)
                                        .then_some(RepairJobEndType::Repaired { plates: *spent })
                                }
                                Err(reason) => Some(RepairJobEndType::RepairStopped {
                                    plates: *spent,
                                    reason,
                                }),
                            }
                        }
                    }
                    _ => Some(RepairJobEndType::RepairStopped {
                        plates: *spent,
                        reason: PlateRepairRefusalType::Destroyed,
                    }),
                },
                RepairWorkType::FieldRebuild {
                    section,
                    duration_secs,
                    ..
                } => {
                    job.elapsed_secs = (job.elapsed_secs + dt).min(*duration_secs);
                    if job.elapsed_secs < *duration_secs {
                        None
                    } else {
                        let design = design.unwrap_or_else(|| {
                            panic!("field rebuild: ship {ship:?} has no ShipDesignSections")
                        });
                        let lost = design.get(section).unwrap_or_else(|| {
                            panic!("field rebuild: ship {ship:?} has no design section '{section}'")
                        });
                        let live: Vec<(&str, PlacedSectionLinkPoints<'_>)> = children
                            .into_iter()
                            .flatten()
                            .filter_map(|child| q_live.get(*child).ok())
                            .map(|(id, transform, points)| {
                                (
                                    id.0.as_str(),
                                    PlacedSectionLinkPoints {
                                        position: transform.translation,
                                        rotation: transform.rotation,
                                        link_points: &points.0,
                                    },
                                )
                            })
                            .collect();
                        let facts = lost_section_facts(lost, &live);
                        let items = items
                            .as_deref()
                            .expect("field rebuild: a job ends only after the item catalog loads");
                        match plan_section_rebuild(
                            items,
                            &lost.config.base.id,
                            &lost.config.base.build,
                            RebuildSiteType::Field,
                            facts.lost,
                            facts.anchored,
                            &own,
                        ) {
                            Ok(payment) => {
                                for (item, &count) in &payment.from_hold {
                                    own.remove(item, count);
                                }
                                reports.0.push(ShipInventoryReport::Rebuilt {
                                    section: job.code.clone(),
                                    parts: payment.from_hold,
                                    price_cr: 0,
                                });
                                commands.trigger(SectionRebuildRequest {
                                    entity: ship,
                                    section: section.clone(),
                                });
                                Some(RepairJobEndType::Rebuilt)
                            }
                            Err(reason) => Some(RepairJobEndType::RebuildStopped(reason)),
                        }
                    }
                }
            };
            let Some(end) = end else {
                index += 1;
                continue;
            };
            let job = jobs.0.remove(index);
            if let RepairJobEndType::Repaired { plates }
            | RepairJobEndType::RepairStopped { plates, .. } = end
            {
                report_plates(&mut reports, &job.code, plates);
            }
            debug!("repair job: {} on {ship:?} ended: {end:?}", job.code);
            ended.write(RepairJobEnded {
                ship,
                code: job.code,
                end,
            });
        }
    }
}

/// Cancel the job a [`RepairJobCancelRequest`] names: report the plates it
/// fitted and end it as cancelled.
pub(crate) fn cancel_repair_job(
    request: On<RepairJobCancelRequest>,
    mut q_ship: Query<(&mut ShipRepairJobs, &mut ShipInventoryReports)>,
    mut ended: MessageWriter<RepairJobEnded>,
) {
    let Ok((mut jobs, mut reports)) = q_ship.get_mut(request.entity) else {
        return;
    };
    let Some(index) = jobs.0.iter().position(|job| job.key() == request.key) else {
        return;
    };
    let job = jobs.0.remove(index);
    let plates = plates_spent(&job);
    report_plates(&mut reports, &job.code, plates);
    ended.write(RepairJobEnded {
        ship: request.entity,
        code: job.code,
        end: RepairJobEndType::Cancelled { plates },
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{ecs::message::Messages, time::TimeUpdateStrategy};
    use nova_gameplay::{prelude::HULL_PLATE_HEALTH, test_support::test_items};

    use super::*;

    /// A ship with one hull section at `current` of 100 HP and `plates` in
    /// its hold, its clock stepping one plate interval per frame, after the
    /// warm-up frame whose delta is zero.
    fn plate_app(current: f32, plates: u32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            PLATE_REPAIR_SECS,
        )));
        // The virtual clock clamps a frame to 0.25 s by default.
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(60));
        app.add_message::<RepairJobEnded>();
        app.add_observer(cancel_repair_job);
        app.add_systems(Update, advance_ship_repair_jobs);
        let items = test_items();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipInventory::new(
                    &items,
                    1_000_000,
                    [(ItemDesignId::from(ItemRoleType::HullPlate), plates)],
                ),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current,
                    max: 100.0,
                },
                ChildOf(ship),
            ))
            .id();
        app.update();
        (app, ship, section)
    }

    fn health(app: &App, section: Entity) -> f32 {
        app.world().get::<Health>(section).unwrap().current
    }

    fn plates(app: &App, ship: Entity) -> u32 {
        app.world()
            .get::<ShipInventory>(ship)
            .unwrap()
            .count(&ItemDesignId::from(ItemRoleType::HullPlate))
    }

    fn ends(app: &mut App) -> Vec<RepairJobEndType> {
        app.world_mut()
            .resource_mut::<Messages<RepairJobEnded>>()
            .drain()
            .map(|ended| ended.end)
            .collect()
    }

    fn repaired_reports(app: &App, ship: Entity) -> Vec<u32> {
        app.world()
            .get::<ShipInventoryReports>(ship)
            .unwrap()
            .0
            .iter()
            .filter_map(|report| match report {
                ShipInventoryReport::Repaired { count, .. } => Some(*count),
                _ => None,
            })
            .collect()
    }

    /// Each interval fits exactly one plate: one plate leaves the hold and
    /// the section gains one plate's Health. Nothing is spent at the start,
    /// and the job ends with one report of every plate it fitted.
    #[test]
    fn a_plate_job_fits_one_plate_per_interval_and_reports_them_at_the_end() {
        let (mut app, ship, section) = plate_app(30.0, 5);
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipRepairJobs(vec![RepairJob::plates(
                section,
                "HULL-1".into(),
                3,
            )]));
        assert_eq!(plates(&app, ship), 5, "a queued job spends nothing");

        for step in 1..=3u32 {
            app.update();
            assert_eq!(
                health(&app, section),
                30.0 + HULL_PLATE_HEALTH * step as f32
            );
            assert_eq!(plates(&app, ship), 5 - step);
        }
        assert!(app
            .world()
            .get::<ShipRepairJobs>(ship)
            .unwrap()
            .0
            .is_empty());
        assert_eq!(
            ends(&mut app),
            vec![RepairJobEndType::Repaired { plates: 3 }]
        );
        assert_eq!(repaired_reports(&app, ship), vec![3]);

        app.update();
        assert_eq!(
            health(&app, section),
            90.0,
            "an ended job fits nothing more"
        );
    }

    /// The TAB interface and the pause menu stop `Time<Virtual>`; a job
    /// holds while they do and goes on when the clock runs again.
    #[test]
    fn a_plate_job_does_not_progress_while_the_clocks_are_frozen() {
        let (mut app, ship, section) = plate_app(30.0, 5);
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipRepairJobs(vec![RepairJob::plates(
                section,
                "HULL-1".into(),
                2,
            )]));
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(health(&app, section), 30.0);
        assert_eq!(plates(&app, ship), 5);
        assert_eq!(
            app.world().get::<ShipRepairJobs>(ship).unwrap().0[0].elapsed_secs,
            0.0
        );

        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.update();
        assert_eq!(health(&app, section), 50.0);
        assert_eq!(plates(&app, ship), 4);
    }

    /// A hold that runs dry stops the job at the next interval with the plates
    /// it fitted reported; a destroyed section stops it at once.
    #[test]
    fn a_plate_job_stops_on_an_empty_hold_or_a_destroyed_section() {
        let (mut app, ship, section) = plate_app(30.0, 1);
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipRepairJobs(vec![RepairJob::plates(
                section,
                "HULL-1".into(),
                3,
            )]));
        app.update();
        app.update();
        assert_eq!(health(&app, section), 50.0);
        assert_eq!(
            ends(&mut app),
            vec![RepairJobEndType::RepairStopped {
                plates: 1,
                reason: PlateRepairRefusalType::NoPlates,
            }]
        );
        assert_eq!(repaired_reports(&app, ship), vec![1]);

        let (mut app, ship, section) = plate_app(30.0, 5);
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipRepairJobs(vec![RepairJob::plates(
                section,
                "HULL-1".into(),
                3,
            )]));
        app.world_mut().despawn(section);
        app.update();
        assert_eq!(
            ends(&mut app),
            vec![RepairJobEndType::RepairStopped {
                plates: 0,
                reason: PlateRepairRefusalType::Destroyed,
            }]
        );
        assert_eq!(plates(&app, ship), 5);
        assert!(repaired_reports(&app, ship).is_empty());
    }

    /// A cancel ends the job; the plates it fitted stay fitted and are
    /// reported, the rest stay in the hold.
    #[test]
    fn a_cancelled_plate_job_keeps_the_fitted_plates_and_spends_no_more() {
        let (mut app, ship, section) = plate_app(30.0, 5);
        app.world_mut()
            .entity_mut(ship)
            .insert(ShipRepairJobs(vec![RepairJob::plates(
                section,
                "HULL-1".into(),
                3,
            )]));
        app.update();
        app.world_mut().trigger(RepairJobCancelRequest {
            entity: ship,
            key: RepairJobKey::Section(section),
        });
        app.update();
        assert_eq!(health(&app, section), 50.0);
        assert_eq!(plates(&app, ship), 4);
        assert_eq!(
            ends(&mut app),
            vec![RepairJobEndType::Cancelled { plates: 1 }]
        );
        assert_eq!(repaired_reports(&app, ship), vec![1]);
    }

    /// Plates a plate job has still to fit and parts a field rebuild holds
    /// are not free for a new job.
    #[test]
    fn running_jobs_hold_back_their_plates_and_parts_from_a_new_job() {
        let items = test_items();
        let own = ShipInventory::new(
            &items,
            1_000_000,
            [
                (ItemDesignId::from(ItemRoleType::HullPlate), 4),
                (ItemDesignId::from("SectionPanel"), 3),
            ],
        );
        let recipe = BuildRecipe {
            credits: 10,
            parts: [(ItemDesignId::from("SectionPanel"), 2)].into(),
            strip_yield: 0.5,
        };
        let mut plate_job = RepairJob::plates(Entity::PLACEHOLDER, "HULL-1".into(), 3);
        if let RepairWorkType::Plates { spent, .. } = &mut plate_job.work {
            *spent = 1;
        }
        let jobs = ShipRepairJobs(vec![
            plate_job,
            RepairJob::field_rebuild("dock".into(), "DOCK-1".into(), &recipe),
        ]);
        let free = jobs.free_hold(&own);
        assert_eq!(free.count(&ItemDesignId::from(ItemRoleType::HullPlate)), 2);
        assert_eq!(free.count(&ItemDesignId::from("SectionPanel")), 1);
    }
}
