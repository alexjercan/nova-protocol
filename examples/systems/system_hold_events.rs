//! system_hold_events: the six hold events, each fired once by the system that
//! owns the change, and the example mod's objectives completing on them.
//!
//! Boots the shipped app (via [`editor_app`]), enables the example mod, and
//! starts its `example_prospect` berth the way the Scenarios list does. Then
//! it performs each hold verb on its production path and reads what fired:
//!
//! - DOCK with the real key and Buy the tender's rations through the
//!   Inventory pane: `OnTraded` once, with the tender as `id`, and
//!   `OnCreditsChanged` once per party with the signed delta.
//! - Hurt the bridge and repair it through the Ship pane: `OnRepaired` once,
//!   naming the pane code and the plates spent.
//! - Undock and Jettison the rations: `OnJettisoned` once, when the intake
//!   drops the canister, not when the pane queued it.
//! - Stand a canister in the intake's trigger: `OnPickedUp` once as the
//!   intake takes it.
//! - Lock the rock and hold the mine key: `OnMined` once per paying pulse.
//!
//! ONE SUBJECT: that each verb raises exactly the events its payload promises
//! and that authored `once` handlers can complete an objective on each. The
//! flight, the pane and the rules behind each verb are other ranges' and
//! `mine_and_sell`'s.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_hold_events --features debug
//! # look for: `hold_events: PASS six events, six objectives, Victory`,
//! #           `autopilot: cycle complete, no panic`
//! ```

use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_hold_events")]
#[command(version = "1.0.0")]
#[command(
    about = "The six hold events on their production paths in the example mod's berth: trade, credits, repair, jettison, pickup and mining each fire once and complete their authored objective. Autopilot-only correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: the example mod starts disabled on a fresh
    // profile, and no player's enabled set or saved world may be in the way.
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

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds counts and payloads.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::HoldWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::LinearVelocity;
    use bevy::prelude::*;
    use nova_input::prelude::{dispatch, InputPhase};
    use nova_protocol::{
        nova_debug::harness::{hide_status_bar, AutopilotPlugin, Predicate},
        nova_events::engine::GameEvent,
        nova_interface::{pane::InterfacePaneType, ship::SectionCode},
    };

    use super::*;

    /// The mod and the berth under test, as the example mod ships them.
    const EXAMPLE_MOD: &str = "example";
    const SCENARIO: &str = "example_prospect";
    /// The scenario ids the berth spawns.
    const PLAYER_ID: &str = "player_spaceship";
    const TENDER_ID: &str = "example_tender";
    const ROCK_ID: &str = "example_ore_rock";
    /// The counter the berth's six handlers advance.
    const DONE_VAR: &str = "prospect_done";
    /// The objectives the berth authors, one per event.
    const OBJECTIVES: [&str; 6] = [
        "prospect_trade",
        "prospect_credits",
        "prospect_repair",
        "prospect_jettison",
        "prospect_pickup",
        "prospect_mine",
    ];
    /// What the berth authors: the player's credits and plates, the tender's
    /// credits and rations.
    const PLAYER_CREDITS: u32 = 500;
    const PLAYER_PLATES: u32 = 2;
    const TENDER_CREDITS: u32 = 1_000;
    const TENDER_RATIONS: u32 = 4;
    const RATIONS: &str = "Rations";
    /// What every section is knocked down by before the repair: one and a
    /// half plates, so an All draft spends both plates the berth stocks and
    /// the smallest section (60 HP) stays well alive.
    const HURT_HP: f32 = HULL_PLATE_HEALTH * 1.5;
    /// The ore in the canister the pickup takes.
    const PICKUP_ORE: u32 = 3;
    /// Seconds a load, a pane or a beam gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 120.0;

    /// Every hold event the run saw, by name, with its payload.
    #[derive(Resource, Default, Debug)]
    struct HoldWatch {
        events: Vec<(String, serde_json::Value)>,
        /// Corners each paying pulse flipped, from the mining observer.
        paid_pulses: Vec<u32>,
        /// Every player section's health after the hurt, before the repair.
        hurt: std::collections::BTreeMap<Entity, f32>,
    }

    impl HoldWatch {
        fn named(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct HoldWatchPlugin;

    impl Plugin for HoldWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<HoldWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<HoldWatch>| {
                let name = event.name();
                if !matches!(
                    name,
                    "onmined"
                        | "onpickedup"
                        | "onjettisoned"
                        | "ontraded"
                        | "oncreditschanged"
                        | "onrepaired"
                ) {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("hold_events: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
            app.add_observer(|pulse: On<MiningPulse>, mut watch: ResMut<HoldWatch>| {
                if let Ok(corners) = pulse.outcome {
                    if corners > 0 {
                        watch.paid_pulses.push(corners);
                    }
                }
            });
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn seen(name: &'static str, at_least: usize) -> Arc<Predicate> {
        when(move |world| world.resource::<HoldWatch>().named(name).len() >= at_least)
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
        })
    }

    fn world_has<T: Component>(world: &World) -> bool {
        world
            .try_query_filtered::<(), With<T>>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    }

    fn root(world: &World, id: &str) -> Entity {
        world
            .try_query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>()
            .and_then(|mut roots| {
                roots
                    .iter(world)
                    .find(|(_, scenario_id)| scenario_id.0 == id)
                    .map(|(entity, _)| entity)
            })
            .unwrap_or_else(|| panic!("hold_events: no ship {id}"))
    }

    fn credits(world: &World, id: &str) -> u32 {
        let ship = root(world, id);
        world
            .get::<ShipCredits>(ship)
            .unwrap_or_else(|| panic!("hold_events: {id} has no ShipCredits"))
            .0
    }

    fn held(world: &World, id: &str, item: &str) -> u32 {
        let ship = root(world, id);
        world
            .get::<ShipInventory>(ship)
            .unwrap_or_else(|| panic!("hold_events: {id} has no ShipInventory"))
            .count(&item.into())
    }

    fn objectives(world: &World) -> Vec<String> {
        world
            .resource::<GameObjectives>()
            .objectives
            .iter()
            .map(|objective| objective.id.clone())
            .collect()
    }

    fn done_var(world: &World) -> f64 {
        match world.resource::<NovaEventWorld>().get_variable(DONE_VAR) {
            Some(VariableLiteral::Number(value)) => *value,
            other => panic!("hold_events: '{DONE_VAR}' is {other:?}, not a number"),
        }
    }

    fn lock(world: &mut World, target: Entity) {
        let player = root(world, PLAYER_ID);
        world.entity_mut(player).insert(TravelLock(Some(target)));
    }

    /// Press or release whatever the warship's mining section is bound to.
    fn drive_mine_key(world: &mut World, phase: InputPhase) {
        let source = world
            .query::<&SpaceshipMiningInputBinding>()
            .iter(world)
            .next()
            .and_then(|binding| binding.0.first().copied())
            .expect("hold_events: the warship's mining section is bound to a key");
        dispatch::press_source(world, source, phase);
    }

    /// Knock every player section down by [`HURT_HP`], so whichever section
    /// the pane opens on (the first code) has a two-plate repair to draft.
    fn hurt_every_section(world: &mut World) {
        let player = root(world, PLAYER_ID);
        let mut sections =
            world.query_filtered::<(Entity, &mut Health, &ChildOf), With<SectionMarker>>();
        let mut hurt = std::collections::BTreeMap::new();
        for (entity, mut health, parent) in sections.iter_mut(world) {
            if parent.parent() == player {
                health.current = health.max - HURT_HP;
                hurt.insert(entity, health.current);
            }
        }
        assert!(!hurt.is_empty(), "hold_events: the warship has sections");
        world.resource_mut::<HoldWatch>().hurt = hurt;
    }

    /// Stand a canister of ore in the intake's trigger slab, at rest with
    /// the hull, so the intake takes it on its next pass.
    fn stand_a_canister_in_the_intake(world: &mut World) {
        let player = root(world, PLAYER_ID);
        let (face, normal, gap, mesh) = {
            let mut intakes = world.query_filtered::<(
                &ChildOf,
                &GlobalTransform,
                &SectionCollider,
                &CargoIntakeSectionConfigHelper,
            ), With<CargoIntakeSectionMarker>>();
            let (_, pose, collider, config) = intakes
                .iter(world)
                .find(|(parent, ..)| parent.parent() == player)
                .expect("hold_events: the warship has a cargo intake");
            let (face, normal) = cargo_intake_face(pose.translation(), pose.rotation(), *collider);
            (
                face,
                normal,
                config.capture_gap.to_engine(),
                config.canister_mesh.clone(),
            )
        };
        let velocity = world
            .get::<LinearVelocity>(player)
            .map_or(Vec3::ZERO, |velocity| velocity.0);
        let items = world.resource::<GameItems>().clone();
        let id = world.resource_mut::<CargoCanisterIdAllocator>().mint();
        world.spawn((
            cargo_canister(
                CargoCanister::new(&items, &ItemRoleType::StoneOre.into(), PICKUP_ORE),
                Transform::from_translation(face + normal * (gap * 0.5)),
                velocity,
                mesh,
            ),
            id,
        ));
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("hold_events: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("hold_events: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// The walk, one beat per verb.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("hold_events: reach the main menu")
            .until(state_is(GameStates::MainMenu))
            .deadline(SESSION_SECS)
            .add()
            // The example mod ships disabled; the merge re-runs on the enabled set.
            .step("hold_events: enable the example mod")
            .on_enter(|world: &mut World| {
                world
                    .resource_mut::<EnabledMods>()
                    .0
                    .insert(EXAMPLE_MOD.to_string());
            })
            .until(frames(SETTLE_FRAMES * 2))
            .add()
            .step("hold_events: start the berth the way the picker does")
            .on_enter(|world: &mut World| {
                let listed = world
                    .resource::<GameScenarios>()
                    .get(SCENARIO)
                    .unwrap_or_else(|| panic!("hold_events: '{SCENARIO}' is not registered"))
                    .picker_lists();
                assert!(listed, "the berth is a picker row");
                world.resource_mut::<NewGameScenario>().0 = Some(SCENARIO.to_string());
                *world.resource_mut::<GameMode>() = GameMode::NewGame;
                world
                    .resource_mut::<NextState<GameStates>>()
                    .set(GameStates::Playing);
            })
            .until(state_is(GameStates::Playing))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: the berth stands")
            .until(and(player_ship_present(), scenario_is_built()))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: settle and read the objectives")
            .on_enter(|world: &mut World| {
                hide_status_bar(world);
                assert_the_objectives_are_listed(world);
            })
            .until(frames(SETTLE_FRAMES))
            .add()
            // TRADE. Dock with the real key, Buy the rations through the pane.
            .step("hold_events: lock the tender")
            .on_enter(|world: &mut World| {
                let tender = root(world, TENDER_ID);
                lock(world, tender);
            })
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the dock key",
            "dock",
            any_entity::<(With<PlayerSpaceshipMarker>, With<DockedShip>)>(),
        );
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "hold_events: open the Inventory pane",
            "InterfaceTabInventory",
            ui_node_present("InventoryRowPartnerRations"),
            SESSION_SECS,
        )
        // A row on a live, un-lootable partner drafts a Buy.
        .click_named(
            "hold_events: buy: pick the tender's rations",
            "InventoryRowPartnerRations",
            ui_node_present("InventoryDraftConfirm"),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: buy: all",
            "InventoryDraftAll",
            pointer_released(),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: buy: confirm",
            "InventoryDraftConfirm",
            when(|world| held(world, PLAYER_ID, RATIONS) == TENDER_RATIONS),
            SESSION_SECS,
        );
        // The pane pushed its reports under the interface pause; the tracker
        // drains them on the first fixed tick after the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("hold_events: the trade reports")
            .until(and(seen("ontraded", 1), seen("oncreditschanged", 2)))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: one OnTraded, one OnCreditsChanged per party")
            .on_enter(assert_the_trade)
            .add()
            // REPAIR. Hurt every section, then All and Repair on the pane.
            .step("hold_events: hurt every section")
            .on_enter(hurt_every_section)
            .until(frames(1))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Inventory),
        )
        .click_named(
            "hold_events: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        // The schematic builds and settles over frames before its panel
        // enables Repair for the selected section.
        .step("hold_events: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "hold_events: repair: draft all",
            "ShipRepairAll",
            pointer_released(),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: repair: confirm",
            "ShipRepair",
            when(|world| held(world, PLAYER_ID, ItemRoleType::HullPlate.id()) == 0),
            SESSION_SECS,
        );
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("hold_events: the repair reports")
            .until(seen("onrepaired", 1))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: one OnRepaired naming the code and the plates")
            .on_enter(assert_the_repair)
            .add();
        // JETTISON. Undocked first: docked, the pane offers Give.
        let script = tap(
            script,
            "the dock key",
            "dock",
            when(|world| !world_has::<DockedShip>(world)),
        )
        .step("hold_events: let the sleeves come in")
        .until(elapsed(1.5))
        .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Ship),
        )
        .click_named(
            "hold_events: open the Inventory pane",
            "InterfaceTabInventory",
            ui_node_present("InventoryRowOwnRations"),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: jettison: pick the rations",
            "InventoryRowOwnRations",
            ui_node_present("InventoryDraftConfirm"),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: jettison: all",
            "InventoryDraftAll",
            pointer_released(),
            SESSION_SECS,
        )
        .click_named(
            "hold_events: jettison: confirm",
            "InventoryDraftConfirm",
            when(|world| held(world, PLAYER_ID, RATIONS) == 0),
            SESSION_SECS,
        )
        .step("hold_events: nothing fires while the canister only waits")
        .on_enter(|world: &mut World| {
            assert!(
                world
                    .resource::<HoldWatch>()
                    .named("onjettisoned")
                    .is_empty(),
                "a queued jettison is not a drop"
            );
        })
        .add();
        let script = tap(script, "the interface key", "interface_toggle", unpaused())
            .step("hold_events: the intake drops the canister")
            .until(seen("onjettisoned", 1))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: one OnJettisoned for the dropped stack")
            .on_enter(assert_the_jettison)
            .add()
            // PICKUP. A canister in the trigger slab, taken on the next pass.
            .step("hold_events: stand a canister in the intake")
            .on_enter(stand_a_canister_in_the_intake)
            .until(seen("onpickedup", 1))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: one OnPickedUp for the taken stack")
            .on_enter(assert_the_pickup)
            .add()
            // MINE. Lock the rock and hold the key until a pulse pays.
            .step("hold_events: lock the rock and hold the mine key")
            .on_enter(|world: &mut World| {
                let rock = {
                    let mut rocks =
                        world.query_filtered::<(Entity, &EntityId), With<AsteroidMarker>>();
                    rocks
                        .iter(world)
                        .find(|(_, id)| id.0 == ROCK_ID)
                        .map(|(entity, _)| entity)
                        .expect("hold_events: the rock spawned")
                };
                lock(world, rock);
                drive_mine_key(world, InputPhase::Press);
            })
            .until(seen("onmined", 1))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: let the mine key up")
            .on_enter(|world: &mut World| drive_mine_key(world, InputPhase::Release))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("hold_events: OnMined once per paying pulse")
            .on_enter(assert_the_mining)
            .add()
            .step("hold_events: the six handlers complete the tour")
            .until(scenario_variable_is(DONE_VAR, 6.0))
            .deadline(SESSION_SECS)
            .add()
            .step("hold_events: six objectives done, Victory up")
            .on_enter(assert_the_tour)
            .add();
        script
    }

    /// The berth lists its six objectives, one per event, and none is done.
    fn assert_the_objectives_are_listed(world: &mut World) {
        let listed = objectives(world);
        for objective in OBJECTIVES {
            assert!(
                listed.iter().any(|id| id == objective),
                "hold_events: '{objective}' is not listed in {listed:?}"
            );
        }
        assert_eq!(done_var(world), 0.0, "no handler ran before a verb");
        assert!(
            world.resource::<HoldWatch>().events.is_empty(),
            "no hold event fired at load"
        );
        nova_probe::probe_marker(
            world,
            "outcome: the berth lists one objective per hold event and none fires at load",
            serde_json::json!({ "objectives": listed.len() }),
        );
    }

    /// One `OnTraded` with the tender as subject and the player as the other
    /// party, and one `OnCreditsChanged` per party with the signed delta.
    fn assert_the_trade(world: &mut World) {
        let price = TENDER_RATIONS * world.resource::<GameItems>().design(&RATIONS.into()).ask_cr;
        let watch = world.resource::<HoldWatch>();
        let traded = watch.named("ontraded");
        assert_eq!(traded.len(), 1, "one OnTraded: {traded:?}");
        assert_eq!(
            traded[0],
            &serde_json::json!({
                "id": TENDER_ID,
                "other_id": PLAYER_ID,
                "other_type_name": SPACESHIP_TYPE_NAME,
                "trade": "Buy",
                "item": RATIONS,
                "count": TENDER_RATIONS,
                "price_cr": price,
            })
        );
        let mut changed: Vec<serde_json::Value> = watch
            .named("oncreditschanged")
            .into_iter()
            .cloned()
            .collect();
        changed.sort_by_key(|data| data["id"].as_str().unwrap_or_default().to_string());
        assert_eq!(
            changed,
            vec![
                serde_json::json!({
                    "id": TENDER_ID,
                    "type_name": SPACESHIP_TYPE_NAME,
                    "previous_cr": TENDER_CREDITS,
                    "current_cr": TENDER_CREDITS + price,
                    "delta_cr": i64::from(price),
                    "cause": "Trade",
                }),
                serde_json::json!({
                    "id": PLAYER_ID,
                    "type_name": SPACESHIP_TYPE_NAME,
                    "previous_cr": PLAYER_CREDITS,
                    "current_cr": PLAYER_CREDITS - price,
                    "delta_cr": -i64::from(price),
                    "cause": "Trade",
                }),
            ]
        );
        assert_eq!(credits(world, PLAYER_ID), PLAYER_CREDITS - price);
        assert_eq!(credits(world, TENDER_ID), TENDER_CREDITS + price);
        nova_probe::probe_marker(
            world,
            "outcome: a Buy fires OnTraded once with the trader as subject",
            serde_json::json!({ "count": TENDER_RATIONS, "price_cr": price }),
        );
        nova_probe::probe_marker(
            world,
            "outcome: a trade fires OnCreditsChanged once per party with the signed delta",
            serde_json::json!({ "delta_cr": price }),
        );
        info!("hold_events: traded {TENDER_RATIONS} rations for {price} cr");
    }

    /// One `OnRepaired` naming the pane code of the one section whose health
    /// rose, and both plates.
    fn assert_the_repair(world: &mut World) {
        let hurt = world.resource::<HoldWatch>().hurt.clone();
        let healed: Vec<(Entity, String)> = hurt
            .iter()
            .filter(|(&entity, &was)| {
                world
                    .get::<Health>(entity)
                    .is_some_and(|health| health.current > was)
            })
            .map(|(&entity, _)| {
                let code = world
                    .get::<SectionCode>(entity)
                    .unwrap_or_else(|| panic!("hold_events: {entity:?} has no pane code"))
                    .0
                    .clone();
                (entity, code)
            })
            .collect();
        assert_eq!(healed.len(), 1, "one section healed: {healed:?}");
        let code = healed[0].1.clone();
        let watch = world.resource::<HoldWatch>();
        let repaired = watch.named("onrepaired");
        assert_eq!(repaired.len(), 1, "one OnRepaired: {repaired:?}");
        assert_eq!(
            repaired[0],
            &serde_json::json!({
                "id": PLAYER_ID,
                "type_name": SPACESHIP_TYPE_NAME,
                "section": code,
                "item": ItemRoleType::HullPlate.id(),
                "count": PLAYER_PLATES,
            })
        );
        assert_eq!(held(world, PLAYER_ID, ItemRoleType::HullPlate.id()), 0);
        nova_probe::probe_marker(
            world,
            "outcome: a Ship pane repair fires OnRepaired once naming the section code and the plates",
            serde_json::json!({ "section": code, "plates": PLAYER_PLATES }),
        );
        info!("hold_events: repaired {code} with {PLAYER_PLATES} plates");
    }

    /// One `OnJettisoned` for the one stack the canister carried, and the
    /// canister is in space.
    fn assert_the_jettison(world: &mut World) {
        let watch = world.resource::<HoldWatch>();
        let dropped = watch.named("onjettisoned");
        assert_eq!(dropped.len(), 1, "one OnJettisoned: {dropped:?}");
        assert_eq!(dropped[0]["id"], PLAYER_ID);
        assert_eq!(dropped[0]["type_name"], SPACESHIP_TYPE_NAME);
        assert_eq!(dropped[0]["item"], RATIONS);
        assert_eq!(dropped[0]["count"], TENDER_RATIONS);
        let canister = dropped[0]["canister"]
            .as_u64()
            .expect("the payload names the canister");
        let live = world
            .query::<&CargoCanisterRuntimeId>()
            .iter(world)
            .any(|id| id.0 == canister);
        assert!(live, "the named canister is in space");
        nova_probe::probe_marker(
            world,
            "outcome: a jettison fires OnJettisoned once per stack when the intake drops the canister",
            serde_json::json!({ "canister": canister, "count": TENDER_RATIONS }),
        );
        info!("hold_events: canister {canister} dropped");
    }

    /// One `OnPickedUp` for the ore stack, and the ore is in the hold.
    fn assert_the_pickup(world: &mut World) {
        let watch = world.resource::<HoldWatch>();
        let taken = watch.named("onpickedup");
        assert_eq!(taken.len(), 1, "one OnPickedUp: {taken:?}");
        assert_eq!(taken[0]["id"], PLAYER_ID);
        assert_eq!(taken[0]["type_name"], SPACESHIP_TYPE_NAME);
        assert_eq!(taken[0]["item"], ItemRoleType::StoneOre.id());
        assert_eq!(taken[0]["count"], PICKUP_ORE);
        let canister = taken[0]["canister"]
            .as_u64()
            .expect("the payload names the canister");
        assert_ne!(
            Some(canister),
            watch.named("onjettisoned")[0]["canister"].as_u64(),
            "the taken canister is not the dropped one"
        );
        assert_eq!(
            held(world, PLAYER_ID, ItemRoleType::StoneOre.id()),
            PICKUP_ORE
        );
        nova_probe::probe_marker(
            world,
            "outcome: a canister the intake takes fires OnPickedUp once per stack",
            serde_json::json!({ "canister": canister, "count": PICKUP_ORE }),
        );
        info!("hold_events: canister {canister} taken");
    }

    /// One `OnMined` per paying pulse, each naming the rock, the miner and the
    /// corners that pulse flipped.
    fn assert_the_mining(world: &mut World) {
        let watch = world.resource::<HoldWatch>();
        let mined = watch.named("onmined");
        let paid = watch.paid_pulses.clone();
        assert!(!paid.is_empty(), "a pulse paid");
        assert_eq!(
            mined.len(),
            paid.len(),
            "one OnMined per paying pulse: {mined:?} against {paid:?}"
        );
        for (event, corners) in mined.iter().zip(&paid) {
            assert_eq!(
                *event,
                &serde_json::json!({
                    "id": ROCK_ID,
                    "other_id": PLAYER_ID,
                    "other_type_name": SPACESHIP_TYPE_NAME,
                    "item": ItemRoleType::StoneOre.id(),
                    "count": corners,
                })
            );
        }
        nova_probe::probe_marker(
            world,
            "outcome: a paying mining pulse fires OnMined once naming the rock and the miner",
            serde_json::json!({ "pulses": paid.len(), "corners": paid.iter().sum::<u32>() }),
        );
        info!("hold_events: {} paying pulse(s) reported", paid.len());
    }

    /// Every authored objective completed on its event, and the sixth ended
    /// the tour in Victory.
    fn assert_the_tour(world: &mut World) {
        let left = objectives(world);
        assert!(
            left.iter().all(|id| !OBJECTIVES.contains(&id.as_str())),
            "every hold objective completed, {left:?} remain"
        );
        assert_eq!(done_var(world), 6.0);
        let outcome = world
            .resource::<CurrentOutcome>()
            .0
            .clone()
            .expect("the authored outcome is up");
        assert_eq!(outcome.outcome, ScenarioOutcomeKind::Victory);
        nova_probe::probe_marker(
            world,
            "outcome: each hold event completes its authored objective and the tour ends in Victory",
            serde_json::json!({ "done": 6 }),
        );
        info!("hold_events: PASS six events, six objectives, Victory");
    }
}
