//! The scenario write to a station's job board.
//!
//! A scenario posts a procurement beside the jobs a board draws. The posting
//! is a [`ScenarioPosting`] in [`JobPostings`], so the open world puts it on
//! the station's board, the Jobs pane lists it and a save carries it, with
//! no path of its own. The four job events report it with its key as
//! `post_key`.

use bevy::prelude::*;
use nova_events::prelude::*;
use nova_gameplay::prelude::*;

use crate::prelude::*;

/// Post one procurement on the job board of a generated station.
///
/// The job asks for `count` of `item`, hands in at the posting station and
/// pays `reward_cr` out of that station's credits. It has no window: it
/// stands on the board until the player hands it in or drops it. `key` names
/// it: `OnJobAccepted`, `OnJobDone`, `OnJobHandedIn` and `OnJobDropped`
/// carry it as `post_key`, and a station carries a key once for the whole
/// run.
///
/// No author can write a generated station's id, so `station` is `$id` or
/// `$other_id`, bound from the firing event: `$id` under `OnDocked` is the
/// station docked with.
///
/// Refused with `unknown_station` when the event carries no such field or no
/// live body wears the id, with `no_job_board` when the body is no living
/// station that offers jobs, and with `duplicate_job` when the station
/// offers the key or the player holds, handed in or dropped the job under
/// it. A refused action posts nothing.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PostJobActionConfig {
    /// `$id` or `$other_id`: the firing event's subject or other party, the
    /// station that posts the job.
    #[reflect(@Names::Object)]
    pub station: String,
    /// The key the job events carry as `post_key`: a lowercase letter, then
    /// lowercase letters, digits and `_` ([`is_job_post_key`]). Any other
    /// key is a lint error.
    pub key: String,
    /// The item wanted. An id no loaded bundle defines is a lint error.
    pub item: ItemDesignId,
    /// Units wanted. Zero is a lint error.
    pub count: u32,
    /// What the station pays on the hand-in, in credits. Zero is a lint
    /// error.
    pub reward_cr: u32,
}

impl EventAction<NovaEventWorld> for PostJobActionConfig {
    fn action(&self, world: &mut NovaEventWorld, info: &GameEventInfo) {
        let config = self.clone();
        let bound = bind_object_reference(&self.station, info);
        debug!(
            "PostJob: '{}' ({bound:?}) posts '{}': {} {} for {} cr",
            config.station, config.key, config.count, config.item, config.reward_cr
        );

        world.push_command(move |commands| {
            commands.queue(move |world: &mut World| post_job(world, &config, bound));
        });
    }
}

/// Post the job on the station `bound` names, or refuse and change nothing.
fn post_job(world: &mut World, config: &PostJobActionConfig, bound: Option<String>) {
    const ACTION: &str = "PostJob";

    let Some(id) = bound else {
        refuse_action(
            world,
            ACTION,
            &config.station,
            ActionRefusalType::UnknownStation,
            "the firing event carries no such field",
        );
        return;
    };
    let Ok(station_id) = id.parse::<GeneratedBodyId>() else {
        // An authored body: only a generated station posts a board.
        let refusal = if scoped_entity(world, &id).is_some() {
            ActionRefusalType::NoJobBoard
        } else {
            ActionRefusalType::UnknownStation
        };
        refuse_action(
            world,
            ACTION,
            &id,
            refusal,
            "only a live generated station posts a job board",
        );
        return;
    };
    let Some(station) = generated_entity(world, &station_id) else {
        refuse_action(
            world,
            ACTION,
            &id,
            ActionRefusalType::UnknownStation,
            "no live body wears the id",
        );
        return;
    };
    let body = world.entity(station);
    let posts = body.contains::<StationMarker>()
        && station_offers(
            body.get::<StationServices>(),
            body.contains::<DerelictShipMarker>(),
            StationServiceType::Jobs,
        );
    if !posts {
        refuse_action(
            world,
            ACTION,
            &id,
            ActionRefusalType::NoJobBoard,
            "the body is no living station that offers jobs",
        );
        return;
    }

    let job_id = JobId::scenario(station_id.clone(), config.key.clone());
    // The open world keeps the three beside every generated station.
    let jobs = world.resource::<Jobs>();
    let state = if world
        .resource::<JobPostings>()
        .scenario_posting(&job_id)
        .is_some()
    {
        Some("the station offers it")
    } else if jobs.holds(&job_id) {
        Some("the player holds it")
    } else if jobs.is_completed(&job_id) {
        Some("the player handed it in")
    } else if jobs.is_dropped(&job_id) {
        Some("the player dropped it")
    } else {
        None
    };
    if let Some(state) = state {
        refuse_action(
            world,
            ACTION,
            &id,
            ActionRefusalType::DuplicateJob,
            format_args!("the key '{}' is taken: {state}", config.key),
        );
        return;
    }

    // The load gate lints the scenario, so the key, the item, the count and
    // the reward make a job `Job::validate` accepts here.
    let job = Job {
        id: job_id,
        kind: JobType::Procure {
            item: config.item.clone(),
            count: config.count,
        },
        target: station_id,
        reward_cr: config.reward_cr,
    };
    let now = world.resource::<WorldClock>().play_seconds;
    info!(
        "PostJob: '{}' goes on its board at {now:.1} s of play",
        job.id
    );
    world.resource_mut::<JobPostings>().post_scenario(job, now);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every refusal raised, in order, as the id and the reason.
    #[derive(Resource, Default)]
    struct Refused(Vec<(String, String)>);

    fn station_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "cluster_0_0_0_station", 0)
    }

    fn outpost_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "cluster_0_0_0_station", 1)
    }

    fn husk_id() -> GeneratedBodyId {
        GeneratedBodyId::new(SectorCoord::new(0, 0, 0), "cluster_0_0_0_station", 2)
    }

    fn job_id() -> JobId {
        JobId::scenario(station_id(), "walk")
    }

    /// A world that records refusals, at 40 s of play, with a generated
    /// station that offers jobs, a generated outpost that offers none, a
    /// derelict generated station and an authored station `dock`.
    fn world_with_stations() -> World {
        let mut world = World::new();
        world.init_resource::<NovaEventWorld>();
        world.init_resource::<GameObjectives>();
        world.init_resource::<Refused>();
        world.init_resource::<Jobs>();
        world.init_resource::<JobPostings>();
        world.insert_resource(WorldClock { play_seconds: 40.0 });
        world.add_observer(|event: On<GameEvent>, mut refused: ResMut<Refused>| {
            if event.name() != OnActionRefusedEvent::name() {
                return;
            }
            let text = |field: &str| match event.info().field(field) {
                Some(GameEventField::Text(text)) => text.to_string(),
                other => panic!("the refusal carries the text field '{field}': {other:?}"),
            };
            refused.0.push((
                text(ENTITY_ID_COMPONENT_NAME),
                text(REFUSAL_REASON_FIELD_NAME),
            ));
        });
        let jobs = || StationServices(vec![StationServiceType::Jobs]);
        world.spawn((ScenarioScopedMarker, station_id(), StationMarker, jobs()));
        world.spawn((
            ScenarioScopedMarker,
            outpost_id(),
            StationMarker,
            StationServices(vec![StationServiceType::Trade]),
        ));
        world.spawn((
            ScenarioScopedMarker,
            husk_id(),
            StationMarker,
            jobs(),
            DerelictShipMarker,
        ));
        world.spawn((
            ScenarioScopedMarker,
            ScenarioAddressableMarker,
            EntityId::new("dock".to_string()),
            StationMarker,
            jobs(),
        ));
        world
    }

    /// Run a `PostJob` under `key` on `$id` under an `OnDocked` of the
    /// player with the hull `docked_with`: 4 iron ore for 90 cr.
    fn post(world: &mut World, docked_with: &str, key: &str) {
        let info = GameEventInfo::from_data(DockingEventInfo {
            id: docked_with.to_string(),
            other_id: "player".to_string(),
            ..default()
        });
        let action = PostJobActionConfig {
            station: "$id".to_string(),
            key: key.to_string(),
            item: ItemDesignId::from(ItemRoleType::IronOre.id()),
            count: 4,
            reward_cr: 90,
        };
        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &info);
        NovaEventWorld::state_to_world_system(world);
        world.flush();
    }

    #[test]
    fn a_job_posted_on_a_bound_station_stands_on_its_postings_under_the_key() {
        let mut world = world_with_stations();

        post(&mut world, &station_id().to_string(), "walk");

        assert!(world.resource::<Refused>().0.is_empty());
        let postings = world.resource::<JobPostings>();
        assert_eq!(
            postings.scenario_postings(&station_id()),
            [ScenarioPosting {
                posted_at: 40.0,
                offer: Job {
                    id: job_id(),
                    kind: JobType::Procure {
                        item: ItemDesignId::from(ItemRoleType::IronOre.id()),
                        count: 4,
                    },
                    target: station_id(),
                    reward_cr: 90,
                },
            }]
        );
        postings.validate(40.0).unwrap();
    }

    /// The key is spent on its station for the run: offered, held, handed in
    /// or dropped, a second post is refused and posts nothing. Another key
    /// posts beside it.
    #[test]
    fn a_second_post_under_a_key_is_refused_while_offered_held_handed_in_or_dropped() {
        let station = station_id().to_string();
        let duplicate = [(station.clone(), "duplicate_job".to_string())];
        let posted = |world: &World| {
            world
                .resource::<JobPostings>()
                .scenario_postings(&station_id())
                .len()
        };
        let offer = |world: &World| {
            world
                .resource::<JobPostings>()
                .scenario_posting(&job_id())
                .expect("the first post stands")
                .offer
                .clone()
        };

        // Offered.
        let mut world = world_with_stations();
        post(&mut world, &station, "walk");
        post(&mut world, &station, "walk");
        assert_eq!(world.resource::<Refused>().0, duplicate);
        assert_eq!(posted(&world), 1);
        post(&mut world, &station, "second");
        assert_eq!(posted(&world), 2, "another key posts beside it");

        // Held.
        let mut world = world_with_stations();
        post(&mut world, &station, "walk");
        let job = offer(&world);
        world.resource_mut::<Jobs>().accept(job);
        post(&mut world, &station, "walk");
        assert_eq!(world.resource::<Refused>().0, duplicate);
        assert_eq!(posted(&world), 1);

        // Handed in, then dropped: the board pass took the posting off.
        for end in [Jobs::complete as fn(&mut Jobs, &JobId) -> Job, Jobs::drop] {
            let mut world = world_with_stations();
            post(&mut world, &station, "walk");
            let job = offer(&world);
            world.resource_mut::<Jobs>().accept(job);
            end(&mut world.resource_mut::<Jobs>(), &job_id());
            let jobs = world.resource::<Jobs>().clone();
            world
                .resource_mut::<JobPostings>()
                .retire_scenario_postings(&jobs);
            post(&mut world, &station, "walk");
            assert_eq!(world.resource::<Refused>().0, duplicate);
            assert_eq!(posted(&world), 0);
        }
    }

    #[test]
    fn a_body_nobody_wears_or_that_posts_no_board_is_refused_and_posts_nothing() {
        let retired = GeneratedBodyId::new(SectorCoord::new(4, 0, 0), "cluster_0_0_0_station", 0);
        for (docked_with, reason) in [
            (retired.to_string(), "unknown_station"),
            ("ghost".to_string(), "unknown_station"),
            (outpost_id().to_string(), "no_job_board"),
            (husk_id().to_string(), "no_job_board"),
            ("dock".to_string(), "no_job_board"),
        ] {
            let mut world = world_with_stations();

            post(&mut world, &docked_with, "walk");

            assert_eq!(
                world.resource::<Refused>().0,
                [(docked_with, reason.to_string())]
            );
            assert!(world.resource::<JobPostings>().is_empty(), "{reason}");
        }
    }

    /// A binding the event does not carry is refused under the reference as
    /// written: there is no id to name.
    #[test]
    fn a_binding_the_event_does_not_carry_is_refused_as_an_unknown_station() {
        let mut world = world_with_stations();
        let action = PostJobActionConfig {
            station: "$other_id".to_string(),
            key: "walk".to_string(),
            item: ItemDesignId::from(ItemRoleType::IronOre.id()),
            count: 4,
            reward_cr: 90,
        };

        let mut event_world = world.resource_mut::<NovaEventWorld>();
        action.action(&mut event_world, &GameEventInfo::default());
        NovaEventWorld::state_to_world_system(&mut world);
        world.flush();

        assert_eq!(
            world.resource::<Refused>().0,
            [("$other_id".to_string(), "unknown_station".to_string())]
        );
        assert!(world.resource::<JobPostings>().is_empty());
    }
}
