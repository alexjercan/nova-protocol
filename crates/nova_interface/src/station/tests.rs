//! The Station pane's buy results: the one cue a result or a refusal plays,
//! with no pane body spawned.

use core::time::Duration;

use bevy::{asset::AssetPlugin, prelude::*, time::TimeUpdateStrategy};
use nova_events::prelude::{EntityId, GeneratedBodyId, SectorCoord};
use nova_gameplay::{prelude::*, test_support::test_items};
use nova_ship::prelude::*;
use nova_sound::prelude::*;

use super::*;
use crate::pointer_rig::{hear_ui_cues, take_cues};

/// The buy handler and the cue player on a manual real clock, with the UI
/// sound bank captured and no UI tree: the cue must not need the slot.
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

/// A refused buy buzzes once; the frames after it stay silent. A refusal
/// raised before the sound bank lands waits for it and buzzes once then, not
/// never.
#[test]
fn a_refused_station_buy_buzzes_once() {
    let mut app = station_app();
    let player = dock_at_station(app.world_mut(), Allegiance::Neutral);

    app.world_mut().write_message(StationBuyCommand);
    app.update();
    assert_eq!(
        app.world().get::<ShipCredits>(player),
        Some(&ShipCredits(100)),
        "a refusal spends nothing"
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

/// A buy that goes through pays the price and clicks once; the frame after
/// is silent.
#[test]
fn a_station_buy_pays_and_clicks_once() {
    let mut app = station_app();
    let player = dock_at_station(app.world_mut(), Allegiance::Player);

    app.world_mut().write_message(StationBuyCommand);
    app.update();
    assert_eq!(
        app.world().get::<ShipCredits>(player),
        Some(&ShipCredits(0))
    );
    assert_eq!(take_cues(&mut app), [UiSfx::MenuSelect]);
    app.update();
    assert!(take_cues(&mut app).is_empty(), "the cue plays once");
}

/// Every text under the entity named `name`, depth first.
fn texts_under(world: &mut World, name: &str) -> Vec<String> {
    let root = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a node named {name}"));
    let mut texts = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    texts
}

/// The first press on Buy station arms it and spends nothing: the button
/// reads Confirm and a line asks `Buy station?`. The second press buys.
#[test]
fn buying_a_station_needs_a_second_press() {
    use bevy::ui_widgets::Activate;

    let mut app = station_app();
    app.insert_resource(crate::icons::InterfaceIcons::blank());
    app.add_systems(
        Update,
        update_station_sale_block.before(apply_station_buy_commands),
    );
    app.world_mut()
        .commands()
        .spawn(Node::default())
        .with_children(spawn_station_sale_block);
    app.world_mut().flush();
    let player = dock_at_station(app.world_mut(), Allegiance::Player);
    app.update();
    app.update();

    let press = |app: &mut App| {
        let button = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == STATION_BUY_BUTTON)
            .map(|(entity, _)| entity)
            .expect("the Buy button");
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        app.update();
    };

    press(&mut app);
    assert_eq!(
        app.world().get::<ShipCredits>(player),
        Some(&ShipCredits(100)),
        "the first press spends nothing"
    );
    let texts = texts_under(app.world_mut(), STATION_BUY_BUTTON);
    assert!(texts.iter().any(|text| text == "Confirm"), "{texts:?}");
    let asked = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == "Buy station?");
    assert!(asked, "the line asks to confirm the buy");

    press(&mut app);
    assert_eq!(
        app.world().get::<ShipCredits>(player),
        Some(&ShipCredits(0)),
        "the second press buys"
    );
}

/// Offer cards fill their grid row evenly: at 1000 px three 280 px cards fit,
/// so each takes a third of the width less the gaps, and every verb on a
/// card's foot is one control high.
#[test]
fn offer_cards_share_the_width_evenly_with_control_high_verbs() {
    use nova_ui::prelude::{metrics, ButtonSpec, ThemedButton};

    use crate::pointer_rig::{pane_pointer_rig, settle};

    let mut rig = pane_pointer_rig();
    rig.app.add_plugins(nova_ui::NovaUiPlugin);
    let icons = crate::icons::InterfaceIcons::blank();
    let content = rig.content_root;
    let width = 1000.0;
    rig.app
        .world_mut()
        .commands()
        .entity(content)
        .with_children(|content| {
            content
                .spawn(Node {
                    width: px(width),
                    ..offer_rows()
                })
                .with_children(|grid| {
                    for (index, price) in [Some((120, false)), Some((90_000, true)), None]
                        .into_iter()
                        .enumerate()
                    {
                        grid.spawn((Name::new(format!("Card-{index}")), offer_cell()))
                            .with_children(|cell| {
                                cell.spawn(text("A hull", TextRole::Value));
                                spawn_card_foot(cell, &icons, price, |verbs| {
                                    verbs.spawn(nova_ui::prelude::button(ButtonSpec::control(
                                        "Info",
                                    )));
                                    verbs.spawn(nova_ui::prelude::button(ButtonSpec::control(
                                        "Buy",
                                    )));
                                });
                            });
                    }
                });
        });
    settle(&mut rig.app);
    settle(&mut rig.app);

    let world = rig.app.world_mut();
    let widths: Vec<f32> = (0..3)
        .map(|index| {
            let name = format!("Card-{index}");
            world
                .query::<(&Name, &ComputedNode)>()
                .iter(world)
                .find(|(each, _)| each.as_str() == name)
                .map(|(_, node)| node.size().x)
                .unwrap_or_else(|| panic!("a laid-out {name}"))
        })
        .collect();
    let even = (width - 2.0 * metrics::GAP_L) / 3.0;
    for card in &widths {
        assert!(
            (card - even).abs() < 1.0,
            "every card takes an even share {even}: {widths:?}"
        );
    }
    let heights: Vec<f32> = world
        .query_filtered::<&ComputedNode, With<ThemedButton>>()
        .iter(world)
        .map(|node| node.size().y)
        .collect();
    assert_eq!(heights.len(), 6, "two verbs per card: {heights:?}");
    for height in &heights {
        assert!(
            (height - metrics::CONTROL_H).abs() < 0.5,
            "a card verb is one control high: {heights:?}"
        );
    }
}
