//! system_scenario_jobs: a scenario follows a job from the accept to the
//! hand-in, posts a job of its own, and writes the hold and a civilization's
//! stance.
//!
//! Boots the shipped app (via [`editor_app`]) and starts New Game on a pinned
//! seed as the `scavenger` start role. New Game plays `scenario_jobs_fixture`:
//! the shipped open world under its own id, with the handlers of this range
//! appended. The handlers write variables, and the walk reads the variables
//! beside the real world state. The walk is the shared located-job range in
//! [`located_jobs`] for a recover job, with four beats of its own round the
//! hand-in:
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the scenario walk accepts a recover job` | the shared range's accept claim, under the fixture scenario |
//! | 2 | `outcome: the scenario walk reads the hand-in refused until done` | the shared range's refusal claim |
//! | 3 | `outcome: the scenario walk sees the recover job done` | the shared range's done claim |
//! | 4 | `outcome: the scenario walk is paid the reward and the scenario grants` | the hand-in pays the reward, the open world's first-delivery bonus and the fixture's `GrantCredits` of 25 cr, to the credit |
//! | 5 | `outcome: a scenario counts the steps of a job in order` | `OnJobAccepted`, `OnJobDone` and `OnJobHandedIn` each fired once and in that order: the order variable reads 3 |
//! | 6 | `outcome: a scenario posts a job that is accepted and handed in under its key` | on the dock `PostJob` puts a procurement under the key `walk` on the station's board, `GiveItems` puts the goods aboard, a second `PostJob` of the key is refused as `duplicate_job`; the Jobs tab accepts and hands in the job and `OnJobHandedIn` reads `event.post_key == "walk"` |
//! | 7 | `outcome: a scenario taking more than the hold carries is refused` | `TakeItems` above the held count records `short_items` and the hold reads as before; the handler-written `entity("player").credits` equals the real balance |
//! | 8 | `outcome: a scenario sets the stance of a docked station's civilization` | `SetStance` on the docked station records `event.to == "Enemy"` and `CivilizationStances` reads Enemy; on the undock a second `SetStance` puts the first stance back |
//!
//! ONE SUBJECT: that the job events, `PostJob`, `GiveItems`, `TakeItems`,
//! `GrantCredits` and `SetStance` do in the live open world what their
//! payloads and refusals promise. The board, the Jobs pane and the located
//! kinds are other ranges'.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=900 \
//!   cargo run --example system_scenario_jobs --features debug
//! # look for: `scenario jobs: PASS beat 1` to `PASS beat 4`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "shared/located_jobs.rs"]
pub mod located_jobs;

#[derive(Parser)]
#[command(name = "system_scenario_jobs")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a fixture of the open world whose handlers count a job's steps, post a \
             job, write the hold and set a stance; the walk reads each variable beside the real \
             world state. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        use located_jobs::{
            hand_in, walk_to_done, HandInGrant, LocatedJobMarkers, LocatedJobWalkPlugin,
            LocatedKindType, FIRST_DELIVERY_BONUS_CR,
        };

        // The roster test reads the markers off this file, beside the walk.
        const MARKERS: LocatedJobMarkers = LocatedJobMarkers {
            accepted: "outcome: the scenario walk accepts a recover job",
            refused: "outcome: the scenario walk reads the hand-in refused until done",
            done: "outcome: the scenario walk sees the recover job done",
            paid: "outcome: the scenario walk is paid the reward and the scenario grants",
            escort: None,
            no_medkit: None,
        };

        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds what the scenario wrote.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.insert_resource(HandInGrant(
            FIRST_DELIVERY_BONUS_CR + chain::HAND_IN_GRANT_CR,
        ));
        app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Recover, MARKERS));
        app.add_plugins(chain::ScenarioJobsPlugin);
        let script = chain::before_the_hand_in(walk_to_done(LocatedKindType::Recover));
        app.add_plugins(chain::after_the_hand_in(hand_in(
            script,
            LocatedKindType::Recover,
        )));
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}

#[cfg(feature = "debug")]
mod chain {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate},
        prelude::*,
    };

    use super::{
        located_jobs::{
            activate_named, berth_and_dock_at_the_station, park_beside_the_station, player,
            player_docked_at_the_station, player_free, posted_row, station, tap, the_job, unpaused,
            when, BEAT_SECS, FIRST_DELIVERY_BONUS_CR, SESSION_SECS, SETTLE_FRAMES,
        },
        *,
    };

    /// The fixture New Game plays: the open world plus this range's handlers.
    const FIXTURE_SCENARIO_ID: &str = "scenario_jobs_fixture";
    /// The player's ship, by the id the open world spawns it under.
    const PLAYER_ID: &str = "player";
    /// What the fixture's once `OnJobHandedIn` handler grants, in credits.
    pub(super) const HAND_IN_GRANT_CR: u32 = 25;
    /// The key the fixture posts its job under.
    const POST_KEY: &str = "walk";
    /// The goods the posted job asks for: light, so the hold takes them
    /// beside the recovered stack.
    const GOODS: &str = "Rations";
    /// How many of [`GOODS`] the posted job asks for.
    const GOODS_ASKED: u32 = 3;
    /// How many of [`GOODS`] `GiveItems` puts aboard: the job leaves the rest,
    /// so the short `TakeItems` reads a hold that carries some.
    const GOODS_GIVEN: u32 = 5;
    /// What the posted job pays, in credits.
    const POST_REWARD_CR: u32 = 40;
    /// What the short `TakeItems` asks for: more than any hold carries.
    const GOODS_TAKEN: u32 = 1_000;

    /// The walk raises `beat` to arm one docking handler; the handler moves
    /// it on, so it runs once.
    const BEAT: &str = "beat";
    /// 0, then 1 at the accept, 2 at the done and 3 at the hand-in; each step
    /// is taken only from the one before.
    const ORDER: &str = "order";
    const ACCEPTED: &str = "accepted";
    const DONE: &str = "done";
    const HANDED_IN: &str = "handed_in";
    const LAST_POST_KEY: &str = "last_post_key";
    const KEYED_HAND_INS: &str = "keyed_hand_ins";
    const POST_REFUSALS: &str = "post_refusals";
    const POST_REFUSAL: &str = "post_refusal";
    const TAKE_REFUSALS: &str = "take_refusals";
    const TAKE_REFUSAL: &str = "take_refusal";
    const STANCE_CHANGES: &str = "stance_changes";
    const STANCE_TO: &str = "stance_to";
    const PLAYER_CR: &str = "player_cr";

    /// The value of `beat` that arms the posting dock.
    const BEAT_POST: f64 = 2.0;
    /// Arms the undock that takes too much.
    const BEAT_TAKE: f64 = 3.0;
    /// While `beat` reads this the fixture copies the player's balance.
    const BEAT_TAKEN: f64 = 30.0;
    /// Arms the dock that turns the civilization Enemy.
    const BEAT_ENEMY: f64 = 4.0;

    /// The value of `beat` that arms the undock that sets `stance` back.
    fn beat_back_to(stance: StanceType) -> f64 {
        match stance {
            StanceType::Ally => 5.0,
            StanceType::Neutral => 6.0,
            StanceType::Enemy => panic!("scenario jobs: no dock is made with an Enemy"),
        }
    }

    /// What the walk recorded between beats.
    #[derive(Resource, Debug, Default)]
    struct ChainWatch {
        /// The player's balance before the located job's hand-in dock.
        cr_before: Option<u32>,
        /// The player's count of [`GOODS`] before that dock.
        goods_before: Option<u32>,
        /// The posted job's row on the Jobs tab.
        row: Option<usize>,
        /// The player's and the station's balances before the posted job's
        /// accept.
        post_cr_before: Option<(u32, u32)>,
        /// The player's count of [`GOODS`] before the short take.
        goods_before_take: Option<u32>,
        /// The station's civilization, its seed stance, its table stance
        /// before the walk moved it, and the stance changes counted by then.
        stance: Option<(CivilizationId, StanceType, StanceType, f64)>,
    }

    /// The fixture scenario, the New Game pin and the walk's record.
    pub(super) struct ScenarioJobsPlugin;

    impl Plugin for ScenarioJobsPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ChainWatch>();
            app.add_systems(
                OnEnter(GameAssetsStates::Loaded),
                register_fixture_scenario.in_set(RuntimeScenarioSystems),
            );
            app.add_systems(Update, pin_new_game_start);
        }
    }

    /// Keep the declared New Game start on the fixture: the bundle merge
    /// writes the base bundle's start by command at content load.
    fn pin_new_game_start(mut start: ResMut<NewGameStart>) {
        if start.0.as_deref() != Some(FIXTURE_SCENARIO_ID) {
            start.0 = Some(FIXTURE_SCENARIO_ID.to_string());
        }
    }

    fn expression(text: &str) -> VariableExpressionNode {
        text.parse()
            .unwrap_or_else(|error| panic!("scenario jobs: '{text}' is no expression: {error}"))
    }

    /// `key = <text>`.
    fn set(key: &str, text: &str) -> EventActionConfig {
        EventActionConfig::VariableSet(VariableSetActionConfig {
            key: key.to_string(),
            expression: expression(text),
        })
    }

    /// `key = key + 1`.
    fn count(key: &str) -> EventActionConfig {
        set(key, &format!("{key} + 1"))
    }

    fn only_if(text: &str) -> EventFilterConfig {
        EventFilterConfig::Expression(ExpressionFilterConfig(
            text.parse()
                .unwrap_or_else(|error| panic!("scenario jobs: '{text}' is no condition: {error}")),
        ))
    }

    fn handler(
        label: &str,
        name: EventConfig,
        once: bool,
        filters: Vec<EventFilterConfig>,
        actions: Vec<EventActionConfig>,
    ) -> ScenarioEventConfig {
        ScenarioEventConfig {
            label: Some(label.to_string()),
            position: None,
            name,
            once,
            filters,
            actions,
        }
    }

    /// The filters of a docking handler the walk arms with `beat`: the beat,
    /// and the player as the asking ship, since a generated trader docks too.
    fn player_docking_at(beat: f64) -> Vec<EventFilterConfig> {
        vec![
            only_if(&format!("{BEAT} == {beat}")),
            only_if(&format!("event.other_id == \"{PLAYER_ID}\"")),
        ]
    }

    fn post_the_job() -> EventActionConfig {
        EventActionConfig::PostJob(PostJobActionConfig {
            // Under `OnDocked` the hull docked with is `id`.
            station: "$id".to_string(),
            key: POST_KEY.to_string(),
            item: ItemDesignId::from(GOODS),
            count: GOODS_ASKED,
            reward_cr: POST_REWARD_CR,
        })
    }

    fn set_stance(stance: StanceType) -> EventActionConfig {
        EventActionConfig::SetStance(SetStanceActionConfig {
            body: "$id".to_string(),
            stance,
        })
    }

    /// Register the fixture: the shipped open world under its own id, its
    /// `OnStart` seeding the variables, and this range's handlers at the end.
    fn register_fixture_scenario(mut scenarios: ResMut<GameScenarios>) {
        let mut scenario = scenarios
            .get(OPEN_WORLD_SCENARIO_ID)
            .cloned()
            .expect("scenario jobs: the base bundle registers the open world");
        scenario.id = FIXTURE_SCENARIO_ID.to_string();
        scenario.name = "Scenario Jobs Fixture".to_string();
        let start = scenario
            .events
            .iter_mut()
            .find(|event| event.name == EventConfig::OnStart)
            .expect("scenario jobs: the open world opens with OnStart");
        for key in [
            BEAT,
            ORDER,
            ACCEPTED,
            DONE,
            HANDED_IN,
            KEYED_HAND_INS,
            POST_REFUSALS,
            TAKE_REFUSALS,
            STANCE_CHANGES,
        ] {
            start.actions.push(set(key, "0"));
        }
        let give = EventActionConfig::GiveItems(GiveItemsActionConfig {
            ship: PLAYER_ID.to_string(),
            item: ItemDesignId::from(GOODS),
            count: GOODS_GIVEN,
        });
        let take = EventActionConfig::TakeItems(TakeItemsActionConfig {
            ship: PLAYER_ID.to_string(),
            item: ItemDesignId::from(GOODS),
            count: GOODS_TAKEN,
        });
        scenario.events.extend([
            handler(
                "count the accepts",
                EventConfig::OnJobAccepted,
                false,
                vec![],
                vec![count(ACCEPTED)],
            ),
            handler(
                "the accept comes first",
                EventConfig::OnJobAccepted,
                false,
                vec![only_if(&format!("{ORDER} == 0"))],
                vec![set(ORDER, "1")],
            ),
            handler(
                "count the dones",
                EventConfig::OnJobDone,
                false,
                vec![],
                vec![count(DONE)],
            ),
            handler(
                "the done follows the accept",
                EventConfig::OnJobDone,
                false,
                vec![only_if(&format!("{ORDER} == 1"))],
                vec![set(ORDER, "2")],
            ),
            handler(
                "count the hand-ins and keep the key",
                EventConfig::OnJobHandedIn,
                false,
                vec![],
                vec![count(HANDED_IN), set(LAST_POST_KEY, "event.post_key")],
            ),
            handler(
                "the hand-in follows the done",
                EventConfig::OnJobHandedIn,
                false,
                vec![only_if(&format!("{ORDER} == 2"))],
                vec![set(ORDER, "3")],
            ),
            handler(
                "grant on the first hand-in",
                EventConfig::OnJobHandedIn,
                true,
                vec![],
                vec![EventActionConfig::GrantCredits(GrantCreditsActionConfig {
                    ship: PLAYER_ID.to_string(),
                    amount: i32::try_from(HAND_IN_GRANT_CR).expect("the grant fits an i32"),
                })],
            ),
            handler(
                "count the hand-ins under the key",
                EventConfig::OnJobHandedIn,
                false,
                vec![only_if(&format!("event.post_key == \"{POST_KEY}\""))],
                vec![count(KEYED_HAND_INS)],
            ),
            handler(
                "keep why a post was refused",
                EventConfig::OnActionRefused,
                false,
                vec![only_if("event.action == \"PostJob\"")],
                vec![count(POST_REFUSALS), set(POST_REFUSAL, "event.reason")],
            ),
            handler(
                "keep why a take was refused",
                EventConfig::OnActionRefused,
                false,
                vec![only_if("event.action == \"TakeItems\"")],
                vec![count(TAKE_REFUSALS), set(TAKE_REFUSAL, "event.reason")],
            ),
            handler(
                "keep where a stance went",
                EventConfig::OnStanceChanged,
                false,
                vec![],
                vec![count(STANCE_CHANGES), set(STANCE_TO, "event.to")],
            ),
            handler(
                "post the job, give the goods, post again",
                EventConfig::OnDocked,
                false,
                player_docking_at(BEAT_POST),
                vec![set(BEAT, "20"), post_the_job(), give, post_the_job()],
            ),
            handler(
                "take more than the hold carries",
                EventConfig::OnUndocked,
                false,
                player_docking_at(BEAT_TAKE),
                vec![set(BEAT, &BEAT_TAKEN.to_string()), take],
            ),
            handler(
                "copy the player's balance",
                EventConfig::OnUpdate,
                false,
                vec![only_if(&format!("{BEAT} == {BEAT_TAKEN}"))],
                vec![set(PLAYER_CR, &format!("entity(\"{PLAYER_ID}\").credits"))],
            ),
            handler(
                "turn the docked station's civilization Enemy",
                EventConfig::OnDocked,
                false,
                player_docking_at(BEAT_ENEMY),
                vec![set(BEAT, "40"), set_stance(StanceType::Enemy)],
            ),
            handler(
                "set the stance back to Ally",
                EventConfig::OnUndocked,
                false,
                player_docking_at(beat_back_to(StanceType::Ally)),
                vec![set(BEAT, "50"), set_stance(StanceType::Ally)],
            ),
            handler(
                "set the stance back to Neutral",
                EventConfig::OnUndocked,
                false,
                player_docking_at(beat_back_to(StanceType::Neutral)),
                vec![set(BEAT, "60"), set_stance(StanceType::Neutral)],
            ),
        ]);
        scenarios.insert(FIXTURE_SCENARIO_ID.to_string(), scenario);
    }

    fn variable(world: &World, key: &str) -> Option<VariableLiteral> {
        world
            .resource::<NovaEventWorld>()
            .get_variable(key)
            .cloned()
    }

    fn number(world: &World, key: &str) -> f64 {
        match variable(world, key) {
            Some(VariableLiteral::Number(value)) => value,
            other => panic!("scenario jobs: '{key}' is a number, read {other:?}"),
        }
    }

    fn text(world: &World, key: &str) -> Option<String> {
        match variable(world, key) {
            Some(VariableLiteral::String(value)) => Some(value),
            None => None,
            other => panic!("scenario jobs: '{key}' is text, read {other:?}"),
        }
    }

    fn text_is(key: &'static str, value: &'static str) -> Arc<Predicate> {
        when(move |world| text(world, key).as_deref() == Some(value))
    }

    /// Every variable the fixture writes, for a failed wait.
    fn describe_the_variables(world: &World) -> String {
        [
            BEAT,
            ORDER,
            ACCEPTED,
            DONE,
            HANDED_IN,
            LAST_POST_KEY,
            KEYED_HAND_INS,
            POST_REFUSALS,
            POST_REFUSAL,
            TAKE_REFUSALS,
            TAKE_REFUSAL,
            STANCE_CHANGES,
            STANCE_TO,
            PLAYER_CR,
        ]
        .iter()
        .map(|key| format!("{key}={:?}", variable(world, key)))
        .collect::<Vec<_>>()
        .join(" ")
    }

    /// Raise `beat`, as a handler's own `VariableSet` would.
    fn raise_the_beat(world: &mut World, beat: f64) {
        let mut events = world.resource_mut::<NovaEventWorld>();
        set(BEAT, &beat.to_string()).action(&mut events, &GameEventInfo::default());
        info!("scenario jobs: {BEAT} raised to {beat}");
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .unwrap_or_else(|| panic!("scenario jobs: {root:?} holds no credits"))
            .0
    }

    fn goods_aboard(world: &World) -> u32 {
        world
            .get::<ShipInventory>(player(world))
            .expect("scenario jobs: the player has a hold")
            .count(&ItemDesignId::from(GOODS))
    }

    fn station_id(world: &World) -> GeneratedBodyId {
        the_job(world).1.id.station
    }

    fn posted_id(world: &World) -> JobId {
        JobId::scenario(station_id(world), POST_KEY)
    }

    /// The posted job as the station's board carries it.
    fn posted_job(world: &World) -> Option<Job> {
        world
            .get::<JobBoard>(station(world))?
            .job(&posted_id(world))
            .cloned()
    }

    fn release_the_dock(world: &mut World) {
        let player = player(world);
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// The station's civilization, its seed stance and its table stance.
    fn the_stance(world: &World) -> (CivilizationId, StanceType, StanceType) {
        let civilization = world
            .get::<TraderMarket>(station(world))
            .expect("scenario jobs: a living generated station has a market")
            .civilization;
        let seeded = seeded_stance(&civilization);
        let stance = world
            .resource::<CivilizationStances>()
            .stance(&civilization, seeded);
        (civilization, seeded, stance)
    }

    /// Arm the posting dock and note what the hand-in dock will move.
    fn arm_the_post(world: &mut World) {
        assert_eq!(number(world, BEAT), 0.0, "scenario jobs: no beat is armed");
        let cr_before = credits(world, player(world));
        let goods_before = goods_aboard(world);
        let mut watch = world.resource_mut::<ChainWatch>();
        watch.cr_before = Some(cr_before);
        watch.goods_before = Some(goods_before);
        raise_the_beat(world, BEAT_POST);
    }

    /// Claim 5: the three job events fired once each and in order, and the
    /// hand-in moved the balance by the reward and both grants.
    fn assert_the_chain(world: &mut World) {
        let (_, job) = the_job(world);
        let counted = [ACCEPTED, DONE, HANDED_IN].map(|key| number(world, key));
        assert_eq!(
            counted,
            [1.0, 1.0, 1.0],
            "scenario jobs: one accept, one done and one hand-in were counted"
        );
        assert_eq!(
            number(world, ORDER),
            3.0,
            "scenario jobs: the accept, the done and the hand-in came in order"
        );
        assert_eq!(
            text(world, LAST_POST_KEY).as_deref(),
            Some(""),
            "scenario jobs: a board job carries an empty post_key"
        );
        assert_eq!(
            number(world, KEYED_HAND_INS),
            0.0,
            "scenario jobs: no hand-in was under the key yet"
        );
        let before = world.resource::<ChainWatch>().cr_before.expect("noted");
        let own_cr = credits(world, player(world));
        let expected = before + job.reward_cr + FIRST_DELIVERY_BONUS_CR + HAND_IN_GRANT_CR;
        assert_eq!(
            own_cr, expected,
            "scenario jobs: the balance rose by the {} cr reward, the open world's \
             {FIRST_DELIVERY_BONUS_CR} cr and the fixture's {HAND_IN_GRANT_CR} cr",
            job.reward_cr
        );
        nova_probe::probe_marker(
            world,
            "outcome: a scenario counts the steps of a job in order",
            serde_json::json!({
                "job": job.id.to_string(),
                "order": 3,
                "reward_cr": job.reward_cr,
                "player_cr_before": before,
                "player_cr": own_cr,
            }),
        );
        info!(
            "scenario jobs: PASS beat 1 order 3, balance {before} -> {own_cr} cr (reward {} + \
             {FIRST_DELIVERY_BONUS_CR} + {HAND_IN_GRANT_CR})",
            job.reward_cr
        );
    }

    /// Find the posted job's row on the open Jobs tab.
    fn find_the_posted_row(world: &mut World) {
        let Some(job) = posted_job(world) else {
            return;
        };
        let title = job.title(world.resource::<GameItems>());
        if let Some((row, _)) = posted_row(world, &title) {
            world.resource_mut::<ChainWatch>().row = Some(row);
        }
    }

    fn describe_the_board(world: &World) -> String {
        format!(
            "board {:?}; postings {:?}; {}",
            world.get::<JobBoard>(station(world)),
            world
                .resource::<JobPostings>()
                .scenario_postings(&station_id(world)),
            describe_the_variables(world)
        )
    }

    /// The first half of claim 6: the dock posted one job, gave the goods and
    /// had its second post refused.
    fn assert_the_post(world: &mut World) {
        let id = posted_id(world);
        let job = posted_job(world).expect("scenario jobs: the board carries the posted job");
        assert_eq!(
            job.kind,
            JobType::Procure {
                item: ItemDesignId::from(GOODS),
                count: GOODS_ASKED
            },
            "scenario jobs: the posted job is the authored procurement"
        );
        assert_eq!(job.reward_cr, POST_REWARD_CR, "scenario jobs: its reward");
        assert_eq!(
            id.to_string(),
            format!("{}#{POST_KEY}", station_id(world)),
            "scenario jobs: the job's id is the station and the key"
        );
        let postings = world.resource::<JobPostings>();
        assert_eq!(
            postings.scenario_postings(&station_id(world)).len(),
            1,
            "scenario jobs: the station carries one scenario posting"
        );
        assert!(
            postings.scenario_posting(&id).is_some(),
            "scenario jobs: the posting stands under its id"
        );
        assert_eq!(
            (
                number(world, POST_REFUSALS),
                text(world, POST_REFUSAL).as_deref()
            ),
            (1.0, Some("duplicate_job")),
            "scenario jobs: the second PostJob of the key was refused once as a duplicate"
        );
        let before = world.resource::<ChainWatch>().goods_before.expect("noted");
        assert_eq!(
            goods_aboard(world),
            before + GOODS_GIVEN,
            "scenario jobs: GiveItems put {GOODS_GIVEN} {GOODS} aboard"
        );
        let balances = (
            credits(world, player(world)),
            credits(world, station(world)),
        );
        world.resource_mut::<ChainWatch>().post_cr_before = Some(balances);
        info!(
            "scenario jobs: '{id}' is posted, the second post was refused as duplicate_job, \
             the hold carries {} {GOODS}",
            before + GOODS_GIVEN
        );
    }

    fn the_posted_job_is(check: fn(&Jobs, &JobId) -> bool) -> Arc<Predicate> {
        when(move |world| check(world.resource::<Jobs>(), &posted_id(world)))
    }

    /// Claim 6: the posted job was accepted and handed in under its key.
    fn assert_the_keyed_hand_in(world: &mut World) {
        let id = posted_id(world);
        assert_eq!(
            (
                number(world, KEYED_HAND_INS),
                text(world, LAST_POST_KEY).as_deref()
            ),
            (1.0, Some(POST_KEY)),
            "scenario jobs: OnJobHandedIn carried the key once"
        );
        let counted = [ACCEPTED, DONE, HANDED_IN, ORDER].map(|key| number(world, key));
        assert_eq!(
            counted,
            [2.0, 1.0, 2.0, 3.0],
            "scenario jobs: a second accept and hand-in were counted, no done for a \
             procurement, and the order held"
        );
        let (own_before, station_before) = world
            .resource::<ChainWatch>()
            .post_cr_before
            .expect("noted");
        assert_eq!(
            (
                credits(world, player(world)),
                credits(world, station(world))
            ),
            (own_before + POST_REWARD_CR, station_before - POST_REWARD_CR),
            "scenario jobs: the station paid the posted reward and no grant fired twice"
        );
        let goods_before = world.resource::<ChainWatch>().goods_before.expect("noted");
        assert_eq!(
            goods_aboard(world),
            goods_before + GOODS_GIVEN - GOODS_ASKED,
            "scenario jobs: the hand-in took the {GOODS_ASKED} {GOODS} asked for"
        );
        assert_eq!(
            number(world, POST_REFUSALS),
            1.0,
            "scenario jobs: nothing else was refused"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a scenario posts a job that is accepted and handed in under its key",
            serde_json::json!({
                "job": id.to_string(),
                "post_key": POST_KEY,
                "reward_cr": POST_REWARD_CR,
                "second_post": "duplicate_job",
            }),
        );
        info!(
            "scenario jobs: PASS beat 2 '{id}' posted, accepted and handed in with post_key \
             '{POST_KEY}'; the second post was refused as duplicate_job"
        );
    }

    fn arm_the_take(world: &mut World) {
        let goods = goods_aboard(world);
        assert!(
            (1..GOODS_TAKEN).contains(&goods),
            "scenario jobs: the hold carries some {GOODS}, fewer than the take: {goods}"
        );
        world.resource_mut::<ChainWatch>().goods_before_take = Some(goods);
        raise_the_beat(world, BEAT_TAKE);
    }

    /// The fixture's copy of the player's balance, once it equals the real
    /// one.
    fn the_balance_is_copied() -> Arc<Predicate> {
        when(|world| {
            variable(world, PLAYER_CR)
                == Some(VariableLiteral::Number(f64::from(credits(
                    world,
                    player(world),
                ))))
        })
    }

    /// Claim 7: the short take was refused and took nothing, and the
    /// handler-written balance is the real one.
    fn assert_the_take(world: &mut World) {
        assert_eq!(
            (
                number(world, TAKE_REFUSALS),
                text(world, TAKE_REFUSAL).as_deref()
            ),
            (1.0, Some("short_items")),
            "scenario jobs: the take was refused once as short"
        );
        let goods = world
            .resource::<ChainWatch>()
            .goods_before_take
            .expect("noted");
        assert_eq!(
            goods_aboard(world),
            goods,
            "scenario jobs: a refused take leaves the hold as it was"
        );
        let (_, job) = the_job(world);
        let before = world.resource::<ChainWatch>().cr_before.expect("noted");
        let own_cr = credits(world, player(world));
        assert_eq!(
            own_cr,
            before + job.reward_cr + FIRST_DELIVERY_BONUS_CR + HAND_IN_GRANT_CR + POST_REWARD_CR,
            "scenario jobs: the balance holds both rewards and each grant once"
        );
        assert_eq!(
            number(world, PLAYER_CR),
            f64::from(own_cr),
            "scenario jobs: entity(\"{PLAYER_ID}\").credits reads the real balance"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a scenario taking more than the hold carries is refused",
            serde_json::json!({
                "item": GOODS,
                "held": goods,
                "asked": GOODS_TAKEN,
                "reason": "short_items",
                "player_cr": own_cr,
            }),
        );
        info!(
            "scenario jobs: PASS beat 3 TakeItems of {GOODS_TAKEN} {GOODS} refused as \
             short_items, the hold still carries {goods}; entity credits read {own_cr} cr"
        );
    }

    fn arm_the_enemy_dock(world: &mut World) {
        let (civilization, seeded, stance) = the_stance(world);
        assert_ne!(
            stance,
            StanceType::Enemy,
            "scenario jobs: the station that paid is no Enemy"
        );
        let changes = number(world, STANCE_CHANGES);
        world.resource_mut::<ChainWatch>().stance = Some((civilization, seeded, stance, changes));
        info!(
            "scenario jobs: {} stands {stance:?} (seed {seeded:?}) after {changes} changes",
            civilization.name()
        );
        raise_the_beat(world, BEAT_ENEMY);
    }

    fn the_noted_stance(world: &World) -> (CivilizationId, StanceType, StanceType, f64) {
        world.resource::<ChainWatch>().stance.expect("noted")
    }

    fn the_table_reads(stance: fn(&World) -> StanceType) -> Arc<Predicate> {
        when(move |world| the_stance(world).2 == stance(world))
    }

    /// The first half of claim 8: the dock turned the civilization Enemy in
    /// the variable and in the table. Then arm the way back and undock.
    fn assert_the_enemy_and_undock(world: &mut World) {
        let (civilization, seeded, before, changes) = the_noted_stance(world);
        assert_eq!(
            (
                number(world, STANCE_CHANGES),
                text(world, STANCE_TO).as_deref()
            ),
            (changes + 1.0, Some("Enemy")),
            "scenario jobs: OnStanceChanged fired once with to == Enemy"
        );
        let stances = world.resource::<CivilizationStances>();
        assert_eq!(
            stances.stance(&civilization, seeded),
            StanceType::Enemy,
            "scenario jobs: the table reads Enemy"
        );
        info!(
            "scenario jobs: {} reads Enemy in the table, record {:?}",
            civilization.name(),
            stances.record(&civilization)
        );
        raise_the_beat(world, beat_back_to(before));
        release_the_dock(world);
    }

    /// Claim 8: the undock put the first stance back.
    fn assert_the_stance_is_back(world: &mut World) {
        let (civilization, seeded, before, changes) = the_noted_stance(world);
        assert_eq!(
            (
                number(world, STANCE_CHANGES),
                text(world, STANCE_TO).as_deref()
            ),
            (changes + 2.0, Some(before.name())),
            "scenario jobs: OnStanceChanged fired a second time, back to the first stance"
        );
        assert_eq!(
            world
                .resource::<CivilizationStances>()
                .stance(&civilization, seeded),
            before,
            "scenario jobs: the table reads the first stance again"
        );
        nova_probe::probe_marker(
            world,
            "outcome: a scenario sets the stance of a docked station's civilization",
            serde_json::json!({
                "civilization": civilization.to_string(),
                "from": before.name(),
                "to": "Enemy",
                "back_to": before.name(),
            }),
        );
        info!(
            "scenario jobs: PASS beat 4 {} went {} -> Enemy -> {}",
            civilization.name(),
            before.name(),
            before.name()
        );
    }

    /// Append the step that arms the posting dock to a walk that ends with
    /// its job done: the shared hand-in docks next, and that dock posts.
    pub(super) fn before_the_hand_in(
        script: AutopilotPlugin<GameStates>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step("scenario jobs: arm the posting dock")
            .on_enter(arm_the_post)
            .add()
    }

    /// Append this range's beats to a walk that ends docked at the poster
    /// with the located job handed in and the Jobs tab open.
    pub(super) fn after_the_hand_in(
        script: AutopilotPlugin<GameStates>,
    ) -> AutopilotPlugin<GameStates> {
        let script = script
            .step("scenario jobs: assert the located job's chain")
            .on_enter(assert_the_chain)
            .add()
            .step("scenario jobs: the Jobs tab lists the posted job")
            .each(|world, _, _| find_the_posted_row(world))
            .until(and(
                when(|world| world.resource::<ChainWatch>().row.is_some()),
                frames(SETTLE_FRAMES),
            ))
            .diagnose(describe_the_board)
            .deadline(SESSION_SECS)
            .add()
            .step("scenario jobs: assert the post")
            .on_enter(assert_the_post)
            .add()
            .step("scenario jobs: click Accept on the posted job")
            .on_enter(|world: &mut World| {
                let row = world.resource::<ChainWatch>().row.expect("found");
                activate_named(world, &format!("JobAccept-{row}"));
            })
            .until(and(the_posted_job_is(Jobs::holds), frames(SETTLE_FRAMES)))
            .diagnose(describe_the_board)
            .deadline(BEAT_SECS)
            .add()
            .step("scenario jobs: the Hand in button shows")
            .until(ui_node_present("JobHandIn-0"))
            .deadline(BEAT_SECS)
            .add()
            .step("scenario jobs: click Hand in on the posted job")
            .on_enter(|world: &mut World| activate_named(world, "JobHandIn-0"))
            .until(and(
                the_posted_job_is(Jobs::is_completed),
                frames(SETTLE_FRAMES),
            ))
            .diagnose(describe_the_board)
            .deadline(BEAT_SECS)
            .add()
            .step("scenario jobs: assert the keyed hand-in")
            .on_enter(assert_the_keyed_hand_in)
            .add()
            .step("scenario jobs: arm the short take")
            .on_enter(arm_the_take)
            .add();
        tap(script, "the interface key", "interface_toggle", unpaused())
            .step("scenario jobs: undock into the short take")
            .on_enter(release_the_dock)
            .until(and(
                and(player_free(), text_is(TAKE_REFUSAL, "short_items")),
                and(the_balance_is_copied(), frames(SETTLE_FRAMES)),
            ))
            .diagnose(describe_the_variables)
            .deadline(SESSION_SECS)
            .add()
            .step("scenario jobs: assert the short take")
            .on_enter(assert_the_take)
            .add()
            .step("scenario jobs: arm the Enemy dock")
            .on_enter(arm_the_enemy_dock)
            .add()
            .step("scenario jobs: park beside the station")
            .on_enter(park_beside_the_station)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("scenario jobs: berth and dock a third time")
            .each(|world, _, _| berth_and_dock_at_the_station(world))
            .until(player_docked_at_the_station())
            .deadline(SESSION_SECS)
            .add()
            .step("scenario jobs: the dock turns the civilization Enemy")
            .until(and(
                text_is(STANCE_TO, "Enemy"),
                the_table_reads(|_| StanceType::Enemy),
            ))
            .diagnose(describe_the_variables)
            .deadline(BEAT_SECS)
            .add()
            .step("scenario jobs: assert the Enemy stance and undock")
            .on_enter(assert_the_enemy_and_undock)
            .until(and(
                player_free(),
                the_table_reads(|world| the_noted_stance(world).2),
            ))
            .diagnose(describe_the_variables)
            .deadline(SESSION_SECS)
            .add()
            .step("scenario jobs: assert the stance is back")
            .on_enter(assert_the_stance_is_back)
            .add()
    }
}
