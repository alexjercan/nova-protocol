//! Scenario events bridged from the gameplay events their owners raise.
//!
//! A system outside this crate that moves player state raises a gameplay
//! event where it does it and names no scenario event. One observer here
//! per gameplay event is the only path from it to its scenario event, so
//! the payload is built in one place. A Load restores the state in place
//! and raises none of the gameplay events, so it fires none of these.
//!
//! The job events: the Jobs pane and the open world raise [`JobAccepted`],
//! [`JobDone`], [`JobHandedIn`] and [`JobDropped`], which fire
//! `OnJobAccepted`, `OnJobDone`, `OnJobHandedIn` and `OnJobDropped` through
//! one payload builder, so the four payloads cannot drift apart.
//!
//! The stance event: every site that steps a civilization's table stance
//! raises [`StanceChanged`], which fires `OnStanceChanged`.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

/// Register every bridge.
pub(super) fn register_bridged_events(app: &mut App) {
    app.add_observer(fire_job_accepted);
    app.add_observer(fire_job_done);
    app.add_observer(fire_job_handed_in);
    app.add_observer(fire_job_dropped);
    app.add_observer(fire_stance_changed);
}

/// The payload every job event carries for `job`. `post_key` is the key a
/// scenario's `PostJob` posted the job under, and empty for a job a board
/// slot or a visit posted.
fn job_event_info(job: &Job) -> JobEventInfo {
    JobEventInfo {
        id: job.id.to_string(),
        kind: job.kind.name().to_string(),
        station: job.id.station.to_string(),
        reward_cr: job.reward_cr,
        post_key: job.id.post_key().unwrap_or_default().to_string(),
    }
}

fn fire_job_accepted(accepted: On<JobAccepted>, mut commands: Commands) {
    commands.fire::<OnJobAcceptedEvent>(job_event_info(&accepted.job));
}

fn fire_job_done(done: On<JobDone>, mut commands: Commands) {
    commands.fire::<OnJobDoneEvent>(job_event_info(&done.job));
}

fn fire_job_handed_in(handed_in: On<JobHandedIn>, mut commands: Commands) {
    commands.fire::<OnJobHandedInEvent>(job_event_info(&handed_in.job));
}

fn fire_job_dropped(dropped: On<JobDropped>, mut commands: Commands) {
    commands.fire::<OnJobDroppedEvent>(JobDroppedEventInfo::new(
        job_event_info(&dropped.job),
        dropped.cause,
    ));
}

fn fire_stance_changed(changed: On<StanceChanged>, mut commands: Commands) {
    let StanceChanged(step) = *changed;
    commands.fire::<OnStanceChangedEvent>(StanceChangedEventInfo {
        civilization: step.civilization.to_string(),
        from: step.from.name().to_string(),
        to: step.to.name().to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every scenario event that passed, as its name and the scalar fields
    /// of its payload.
    #[derive(Resource, Default)]
    struct Fired(Vec<(&'static str, Vec<(String, String)>)>);

    fn bridge_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Fired>();
        register_bridged_events(&mut app);
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<Fired>| {
            let info = event.info();
            let fields = info
                .field_names()
                .into_iter()
                .map(|name| {
                    let value = match info.field(name).expect("a listed field reads") {
                        GameEventField::Text(text) => text.to_string(),
                        GameEventField::Number(number) => number.to_string(),
                        GameEventField::Boolean(flag) => flag.to_string(),
                    };
                    (name.to_string(), value)
                })
                .collect();
            fired.0.push((event.name(), fields));
        });
        app
    }

    /// A recovery station `cluster_0_0_0_station_0` posted on slot 1,
    /// posting 2, for 420 cr.
    fn recovery() -> Job {
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        Job {
            id: JobId::slot(station.clone(), 1, 2),
            kind: JobType::Recover {
                wreck: GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_wreck", 0),
                wreck_name: "Wreck".to_string(),
                item: ItemDesignId::from(ItemRoleType::IronOre.id()),
                count: 4,
            },
            target: station,
            reward_cr: 420,
        }
    }

    fn fields(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn each_job_step_fires_its_event_once_with_the_job_s_fields() {
        let job = recovery();
        let station = job.id.station.to_string();
        let id = format!("{station}#1.2");
        let job_fields = [
            (ENTITY_ID_COMPONENT_NAME, id.as_str()),
            (JOB_KIND_FIELD_NAME, "Recover"),
            (JOB_POST_KEY_FIELD_NAME, ""),
            (JOB_REWARD_FIELD_NAME, "420"),
            (JOB_STATION_FIELD_NAME, station.as_str()),
        ];
        let mut app = bridge_app();

        app.world_mut().trigger(JobAccepted { job: job.clone() });
        app.world_mut().trigger(JobDone { job: job.clone() });
        app.world_mut().trigger(JobHandedIn { job: job.clone() });
        app.world_mut().flush();

        assert_eq!(
            app.world().resource::<Fired>().0,
            [
                (OnJobAcceptedEvent::name(), fields(&job_fields)),
                (OnJobDoneEvent::name(), fields(&job_fields)),
                (OnJobHandedInEvent::name(), fields(&job_fields)),
            ]
        );
    }

    #[test]
    fn a_scenario_s_job_carries_its_key_at_every_step() {
        let station = GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0);
        let job = Job {
            id: JobId::scenario(station.clone(), "walk"),
            kind: JobType::Procure {
                item: ItemDesignId::from(ItemRoleType::IronOre.id()),
                count: 4,
            },
            target: station.clone(),
            reward_cr: 90,
        };
        let station = station.to_string();
        let id = format!("{station}#walk");
        let job_fields = [
            (ENTITY_ID_COMPONENT_NAME, id.as_str()),
            (JOB_KIND_FIELD_NAME, "Procure"),
            (JOB_POST_KEY_FIELD_NAME, "walk"),
            (JOB_REWARD_FIELD_NAME, "90"),
            (JOB_STATION_FIELD_NAME, station.as_str()),
        ];
        let mut app = bridge_app();

        app.world_mut().trigger(JobAccepted { job: job.clone() });
        app.world_mut().trigger(JobHandedIn { job: job.clone() });
        app.world_mut().trigger(JobDropped {
            job: job.clone(),
            cause: JobDropCauseType::Abandoned,
        });
        app.world_mut().flush();

        let mut dropped = vec![(JOB_DROP_CAUSE_FIELD_NAME, "Abandoned")];
        dropped.extend(job_fields);
        assert_eq!(
            app.world().resource::<Fired>().0,
            [
                (OnJobAcceptedEvent::name(), fields(&job_fields)),
                (OnJobHandedInEvent::name(), fields(&job_fields)),
                (OnJobDroppedEvent::name(), fields(&dropped)),
            ]
        );
    }

    #[test]
    fn a_dropped_job_fires_one_event_that_names_its_cause() {
        let job = recovery();
        let station = job.id.station.to_string();
        let id = job.id.to_string();
        let mut app = bridge_app();

        for cause in [JobDropCauseType::Lost, JobDropCauseType::Abandoned] {
            app.world_mut().trigger(JobDropped {
                job: job.clone(),
                cause,
            });
        }
        app.world_mut().flush();

        let dropped = |cause: &str| {
            (
                OnJobDroppedEvent::name(),
                fields(&[
                    (JOB_DROP_CAUSE_FIELD_NAME, cause),
                    (ENTITY_ID_COMPONENT_NAME, id.as_str()),
                    (JOB_KIND_FIELD_NAME, "Recover"),
                    (JOB_POST_KEY_FIELD_NAME, ""),
                    (JOB_REWARD_FIELD_NAME, "420"),
                    (JOB_STATION_FIELD_NAME, station.as_str()),
                ]),
            )
        };
        assert_eq!(
            app.world().resource::<Fired>().0,
            [dropped("Lost"), dropped("Abandoned")]
        );
    }

    #[test]
    fn a_stance_step_fires_one_event_with_the_civilization_and_both_stances() {
        let mut app = bridge_app();

        app.world_mut().trigger(StanceChanged(StanceStep {
            civilization: CivilizationId {
                world_seed: 7,
                node: [1, -2, 0],
            },
            from: StanceType::Neutral,
            to: StanceType::Enemy,
        }));
        app.world_mut().flush();

        assert_eq!(
            app.world().resource::<Fired>().0,
            [(
                OnStanceChangedEvent::name(),
                fields(&[
                    (STANCE_CIVILIZATION_FIELD_NAME, "civ_1_n2_0@7"),
                    (STANCE_FROM_FIELD_NAME, "Neutral"),
                    (STANCE_TO_FIELD_NAME, "Enemy"),
                ]),
            )]
        );
    }
}
