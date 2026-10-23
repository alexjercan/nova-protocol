//! The Station pane's notice line: the outcome, the dwell and the one cue a
//! result or a refusal plays, with no pane body spawned.

use core::time::Duration;

use bevy::{asset::AssetPlugin, prelude::*, time::TimeUpdateStrategy};
use nova_events::prelude::{EntityId, GeneratedBodyId, SectorCoord};
use nova_gameplay::{prelude::*, test_support::test_items};
use nova_ship::prelude::*;

use super::*;
use crate::{
    pane::{ActionOutcomeType, NOTICE_SECS},
    pointer_rig::{hear_ui_cues, take_cues},
};

/// Real seconds each `app.update()` counts off the notice: the strategy asks
/// for 0.5 s and `update_station_panel` clamps a frame to 0.25 s.
const FRAME_SECS: f32 = 0.25;

/// The buy handler and the notice painter on a manual real clock, with the
/// UI sound bank captured and no UI tree: the cue must not need the slot.
fn station_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        0.5,
    )));
    app.insert_resource(test_items());
    app.init_resource::<StationRuntime>();
    app.add_message::<StationBuyCommand>();
    app.add_systems(
        Update,
        (apply_station_buy_commands, update_station_panel).chain(),
    );
    hear_ui_cues(&mut app);
    // The first frame advances the clock by nothing.
    app.update();
    app
}

/// A player ship with 100 cr docked at a generated station named STATION of
/// `side`, holding 50 cr and nothing else: it is worth 100 cr at the
/// ownership factor.
fn dock_at_station(world: &mut World, side: Allegiance) -> Entity {
    let items = test_items();
    let player = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            ShipCredits(100),
            ShipInventoryReports::default(),
        ))
        .id();
    let station = world
        .spawn((
            StationMarker,
            Name::new("STATION"),
            ShipInventory::new(&items, 1_000, Vec::<(ItemDesignId, u32)>::new()),
            ShipCredits(50),
            ShipInventoryReports::default(),
            side,
            GeneratedBodyId::new(SectorCoord::ORIGIN, "cluster_0_0_0_station", 0),
            EntityId("cluster_0_0_0_station".to_string()),
        ))
        .id();
    let connection = world
        .spawn(DockingConnection {
            first_ship: player,
            first_section: Entity::PLACEHOLDER,
            second_ship: station,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    world.entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    player
}

fn notice(app: &App) -> Option<(ActionOutcomeType, String)> {
    app.world()
        .resource::<StationRuntime>()
        .notice
        .shown()
        .map(|(outcome, note)| (outcome, note.to_string()))
}

/// A refused buy posts one Refused notice with its reason and buzzes once;
/// the frames after it stay silent. A refusal raised before the sound bank
/// lands waits for it and buzzes once then, not never.
#[test]
fn a_refused_station_buy_posts_a_refusal_notice_and_buzzes_once() {
    let mut app = station_app();
    dock_at_station(app.world_mut(), Allegiance::Neutral);

    app.world_mut().write_message(StationBuyCommand);
    app.update();
    assert_eq!(
        notice(&app),
        Some((
            ActionOutcomeType::Refused,
            "buy: STATION's civilization sells to its allies only".to_string()
        ))
    );
    assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
    app.update();
    assert!(take_cues(&mut app).is_empty(), "the cue plays once");

    let bank = app
        .world_mut()
        .remove_resource::<SoundBank<UiSfx>>()
        .expect("the rig loaded the UI bank");
    app.world_mut().write_message(StationBuyCommand);
    app.update();
    assert!(
        take_cues(&mut app).is_empty(),
        "no bank: nothing plays, the cue waits"
    );
    app.insert_resource(bank);
    app.update();
    assert_eq!(take_cues(&mut app), [UiSfx::EditorDeny]);
    app.update();
    assert!(take_cues(&mut app).is_empty(), "the waited cue plays once");
}

/// A buy that goes through posts one Done notice naming the price, clicks
/// once, and the notice clears after [`NOTICE_SECS`] of real time.
#[test]
fn a_station_buy_posts_a_done_notice_that_clicks_once_and_clears_after_its_dwell() {
    let mut app = station_app();
    let player = dock_at_station(app.world_mut(), Allegiance::Player);

    app.world_mut().write_message(StationBuyCommand);
    app.update();
    assert_eq!(
        notice(&app),
        Some((
            ActionOutcomeType::Done,
            "bought STATION for 100 cr".to_string()
        ))
    );
    assert_eq!(
        app.world().get::<ShipCredits>(player),
        Some(&ShipCredits(0))
    );
    assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);

    let mut shown_secs = FRAME_SECS;
    while notice(&app).is_some() {
        app.update();
        shown_secs += FRAME_SECS;
        assert!(shown_secs <= NOTICE_SECS + FRAME_SECS, "the notice clears");
    }
    assert!(
        (shown_secs - NOTICE_SECS).abs() < f32::EPSILON,
        "the notice showed for {shown_secs} s, not {NOTICE_SECS} s"
    );
    assert!(
        take_cues(&mut app).is_empty(),
        "the dwell plays nothing more"
    );
}
