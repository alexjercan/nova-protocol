//! Live-tree tests for the Inventory pane: the rows drawn from the player's
//! and the docked partner's real inventories, the blank partner slot, and
//! filter and row selection through window picking.

use std::collections::VecDeque;

use bevy::{
    ecs::system::RunSystemOnce,
    ui::{ComputedNode, InteractionDisabled, UiGlobalTransform},
    ui_widgets::{Activate, ValueChange},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{
    CargoIntakeEjectionQueue, CargoIntakeSectionMarker, DockedHelmType, DockedShip,
    DockingConnection,
};
use nova_ui::widget::{TextFieldError, TextFieldValue};

use super::*;
use crate::{
    icons::InterfaceIcons,
    pointer_rig::{
        click_at, hear_ui_cues, move_cursor_to, pane_pointer_rig, settle, take_churn, take_cues,
        track_node_churn, PanePointerRig,
    },
};

/// Build the pane body under `parent` through the production builder.
fn spawn_inventory_body(app: &mut App, parent: Entity) {
    app.world_mut()
        .run_system_once_with(
            |parent: In<Entity>, mut commands: Commands, icons: Res<InterfaceIcons>| {
                commands
                    .entity(*parent)
                    .with_children(|body| inventory_body(body, &icons));
            },
            parent,
        )
        .expect("building the Inventory pane body");
}

/// A 400 kg hold carrying `plates` hull plates of 10 kg.
fn hold(plates: u32) -> ShipInventory {
    ShipInventory::new(400_000, [(ItemType::HullPlate, plates)])
}

/// A player ship carrying `plates` hull plates.
fn spawn_player(world: &mut World, plates: u32) -> Entity {
    world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Name::new("NOVA"),
            hold(plates),
        ))
        .id()
}

/// Dock `player` with a new ship named `name` carrying `plates` hull plates.
fn dock_partner(world: &mut World, player: Entity, name: &str, plates: u32) -> Entity {
    let partner = world
        .spawn((
            SpaceshipRootMarker,
            Name::new(name.to_string()),
            hold(plates),
        ))
        .id();
    let connection = world
        .spawn(DockingConnection {
            first_ship: player,
            first_section: Entity::PLACEHOLDER,
            second_ship: partner,
            second_section: Entity::PLACEHOLDER,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    for (ship, drives) in [(player, false), (partner, true)] {
        world.entity_mut(ship).insert(DockedShip {
            connection,
            helm: Quat::IDENTITY,
            drives,
        });
    }
    partner
}

/// Every text one side's column shows, in tree order.
fn column_texts(world: &mut World, side: InventorySideType) -> Vec<String> {
    let column = world
        .query::<(Entity, &InventoryColumn)>()
        .iter(world)
        .find(|(_, column)| column.side == side)
        .map(|(entity, _)| entity)
        .expect("the pane has a column per side");
    let mut texts = Vec::new();
    let mut stack = vec![column];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter().rev());
        }
    }
    texts
}

/// The text of one side's column title.
fn column_title(world: &mut World, side: InventorySideType) -> String {
    world
        .query::<(&InventoryColumnTitle, &Text)>()
        .iter(world)
        .find(|(title, _)| title.0 == side)
        .map(|(_, text)| text.0.clone())
        .expect("the pane has a title per side")
}

/// The window-space centre of the first node matching `pick`.
fn centre_of<C: Component>(world: &mut World, pick: impl Fn(&C) -> bool) -> Vec2 {
    let (node, xf) = world
        .query::<(&C, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .find(|(each, ..)| pick(each))
        .map(|(_, node, xf)| (node.size(), xf.translation))
        .expect("the node is laid out");
    assert!(node.x > 0.0 && node.y > 0.0, "the node has no area");
    xf
}

#[test]
fn inventory_columns_show_the_player_and_docked_partner_stacks() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<InventoryRuntime>();
    app.add_systems(Update, update_inventory_panel);
    let root = app.world_mut().spawn(Node::default()).id();
    spawn_inventory_body(&mut app, root);
    let player = spawn_player(app.world_mut(), 12);
    app.world_mut()
        .entity_mut(player)
        .insert(ShipCredits(2_000));

    // Undocked: the player's own stack, and no partner to read.
    app.update();
    let world = app.world_mut();
    // The player's title carries its load against its capacity and its
    // credits, grouped by thousands.
    assert_eq!(
        column_title(world, InventorySideType::Own),
        "NOVA 120 kg / 400 kg  2,000 cr"
    );
    assert_eq!(
        column_texts(world, InventorySideType::Own),
        ["Hull plate", "x12"]
    );
    assert!(column_texts(world, InventorySideType::Partner).is_empty());

    // Docked: the partner column reads the partner's own inventory.
    dock_partner(app.world_mut(), player, "Picket", 40);
    app.update();
    let world = app.world_mut();
    assert_eq!(
        column_title(world, InventorySideType::Partner),
        "Picket  0 cr"
    );
    assert_eq!(
        column_texts(world, InventorySideType::Partner),
        ["Hull plate", "x40"]
    );

    // A ship with no stacks says so rather than drawing nothing.
    app.world_mut()
        .entity_mut(player)
        .insert(ShipInventory::default());
    app.update();
    assert_eq!(
        column_texts(app.world_mut(), InventorySideType::Own),
        ["Inventory empty."]
    );
}

/// Every row entity, in no order.
fn row_entities(world: &mut World) -> Vec<Entity> {
    let mut rows: Vec<Entity> = world
        .query_filtered::<Entity, With<InventoryRow>>()
        .iter(world)
        .collect();
    rows.sort();
    rows
}

/// The window-space x and width of one side's panel, and whether it is drawn.
fn panel_of(world: &mut World, side: InventorySideType) -> (f32, f32, bool) {
    world
        .query::<(
            &InventoryColumnPanel,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        )>()
        .iter(world)
        .find(|(panel, ..)| panel.0 == side)
        .map(|(_, node, xf, visible)| (xf.translation.x, node.size().x, visible.get()))
        .expect("the pane has a panel per side")
}

/// The pane body in the pointer rig with a player carrying 12 hull plates,
/// the cue capture and the churn counter.
fn inventory_rig() -> (PanePointerRig, Entity) {
    let mut rig = pane_pointer_rig();
    rig.app.insert_resource(InterfaceIcons::blank());
    rig.app.add_plugins(InventoryPanePlugin);
    hear_ui_cues(&mut rig.app);
    track_node_churn(&mut rig.app);
    let content_root = rig.content_root;
    spawn_inventory_body(&mut rig.app, content_root);
    let player = spawn_player(rig.app.world_mut(), 12);
    settle(&mut rig.app);
    (rig, player)
}

#[test]
fn undocked_the_partner_slot_is_blank_and_both_columns_keep_equal_width() {
    let (mut rig, player) = inventory_rig();

    let (own_x, own_w, own_drawn) = panel_of(rig.app.world_mut(), InventorySideType::Own);
    let (partner_x, partner_w, partner_drawn) =
        panel_of(rig.app.world_mut(), InventorySideType::Partner);
    assert!(
        own_drawn && !partner_drawn,
        "undocked, only the player panel draws"
    );
    assert!(partner_x > own_x, "the blank slot stays on the right");
    assert!(own_w > 0.0 && own_w == partner_w, "{own_w} != {partner_w}");
    assert!(column_texts(rig.app.world_mut(), InventorySideType::Partner).is_empty());

    // The inspector takes 20% of the row beside the stores.
    let world = rig.app.world_mut();
    let (inspector_w, row) = world
        .query::<(&Name, &ComputedNode, &ChildOf)>()
        .iter(world)
        .find(|(name, ..)| name.as_str() == "InventoryInspector")
        .map(|(_, node, child_of)| (node.size().x, child_of.parent()))
        .expect("the pane has an inspector");
    let row_w = world
        .get::<ComputedNode>(row)
        .expect("a laid-out row")
        .size()
        .x;
    assert!(
        (inspector_w - row_w * 0.2).abs() <= 1.0,
        "inspector {inspector_w} is not 20% of row {row_w}"
    );

    // Idle: nothing is spawned, despawned or rewritten.
    take_churn(&mut rig.app);
    settle(&mut rig.app);
    assert_eq!(take_churn(&mut rig.app), Default::default());

    dock_partner(rig.app.world_mut(), player, "Picket", 40);
    settle(&mut rig.app);
    let (_, docked_own_w, _) = panel_of(rig.app.world_mut(), InventorySideType::Own);
    let (_, docked_partner_w, partner_drawn) =
        panel_of(rig.app.world_mut(), InventorySideType::Partner);
    assert!(partner_drawn, "docked, the partner panel draws");
    assert_eq!((docked_own_w, docked_partner_w), (own_w, partner_w));
}

/// A dock that ends with a trade form open, as the player's hit on a docked
/// Neutral ends it, hides the partner column and closes its Buy form.
#[test]
fn a_dock_ending_mid_trade_hides_the_partner_column_and_closes_its_form() {
    let (mut rig, player) = inventory_rig();
    let partner = dock_partner(rig.app.world_mut(), player, "Picket", 40);
    settle(&mut rig.app);
    let partner_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        *row == InventoryRow {
            side: InventorySideType::Partner,
            item: ItemType::HullPlate,
        }
    });
    click_at(&mut rig, partner_row);
    let runtime = rig.app.world().resource::<InventoryRuntime>();
    assert_eq!(
        runtime.draft.map(|draft| draft.action),
        Some(InventoryActionType::Buy),
        "the trading partner's row opens Buy"
    );

    // The release despawns the connection and drops both roots' DockedShip.
    let connection = rig
        .app
        .world()
        .get::<DockedShip>(player)
        .expect("docked")
        .connection;
    let world = rig.app.world_mut();
    world.entity_mut(connection).despawn();
    for ship in [player, partner] {
        world.entity_mut(ship).remove::<DockedShip>();
    }
    settle(&mut rig.app);

    let (_, _, partner_drawn) = panel_of(rig.app.world_mut(), InventorySideType::Partner);
    assert!(!partner_drawn, "the partner column is hidden");
    assert!(column_texts(rig.app.world_mut(), InventorySideType::Partner).is_empty());
    let runtime = rig.app.world().resource::<InventoryRuntime>();
    assert_eq!((runtime.selected, runtime.draft), (None, None));
}

#[test]
fn the_filters_read_all_food_ammo_repair_raw_parts_left_to_right() {
    let (mut rig, _) = inventory_rig();
    let world = rig.app.world_mut();
    let mut chips: Vec<(f32, Option<ItemCategoryType>)> = world
        .query::<(&InventoryFilterChip, &UiGlobalTransform)>()
        .iter(world)
        .map(|(chip, xf)| (xf.translation.x, chip.0))
        .collect();
    chips.sort_by(|a, b| a.0.total_cmp(&b.0));
    let order: Vec<Option<ItemCategoryType>> = chips.into_iter().map(|(_, chip)| chip).collect();
    assert_eq!(
        order,
        [
            None,
            Some(ItemCategoryType::Food),
            Some(ItemCategoryType::Ammo),
            Some(ItemCategoryType::Repair),
            Some(ItemCategoryType::Raw),
            Some(ItemCategoryType::Parts),
        ]
    );
}

#[test]
fn clicking_a_row_inspects_it_and_a_filter_chip_hides_other_categories() {
    let (mut rig, player) = inventory_rig();
    let partner = dock_partner(rig.app.world_mut(), player, "Picket", 39);
    rig.app
        .world_mut()
        .get_mut::<ShipInventory>(partner)
        .expect("the partner has a hold")
        .add(ItemType::PdcRound, 7);
    settle(&mut rig.app);
    take_churn(&mut rig.app);
    let rows = row_entities(rig.app.world_mut());

    // The partner's row selects the partner's stack, not the player's.
    let partner_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        *row == InventoryRow {
            side: InventorySideType::Partner,
            item: ItemType::HullPlate,
        }
    });
    click_at(&mut rig, partner_row);
    assert_eq!(
        rig.app.world().resource::<InventoryRuntime>().selected,
        Some((InventorySideType::Partner, ItemType::HullPlate))
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    // Selecting repaints the rows in place: none is spawned or despawned.
    let churn = take_churn(&mut rig.app);
    assert_eq!((churn.spawned, churn.despawned), (0, 0), "{churn:?}");
    assert_eq!(row_entities(rig.app.world_mut()), rows);

    // The selected row again changes nothing and stays silent.
    click_at(&mut rig, partner_row);
    assert!(take_cues(&mut rig.app).is_empty());
    assert_eq!(take_churn(&mut rig.app), Default::default());
    let inspector = |world: &mut World, wanted: InventoryInspectorField| {
        world
            .query::<(&InventoryInspectorField, &Text)>()
            .iter(world)
            .find(|(field, _)| **field == wanted)
            .map(|(_, text)| text.0.clone())
    };
    let total_weight_shown = |world: &mut World| {
        world
            .query::<(&InspectorPart, &Node)>()
            .iter(world)
            .find(|(part, _)| **part == InspectorPart::TotalWeight)
            .map(|(_, node)| node.display)
            .expect("the inspector has a total weight row")
            != Display::None
    };
    let world = rig.app.world_mut();
    assert_eq!(
        inspector(world, InventoryInspectorField::Stock).as_deref(),
        Some("x39 in Picket")
    );
    assert_eq!(
        inspector(world, InventoryInspectorField::Weight).as_deref(),
        Some("10 kg")
    );
    // Picket is not lootable, so it trades: its row opens a one-unit Buy.
    assert!(total_weight_shown(world));
    assert_eq!(
        world
            .resource::<InventoryRuntime>()
            .draft
            .map(|draft| draft.action),
        Some(InventoryActionType::Buy)
    );

    // Ammo hides the hull plates and drops the selection with them.
    let ammo = centre_of::<InventoryFilterChip>(rig.app.world_mut(), |chip| {
        chip.0 == Some(ItemCategoryType::Ammo)
    });
    click_at(&mut rig, ammo);
    let runtime = rig.app.world().resource::<InventoryRuntime>().clone();
    assert_eq!(runtime.filter, Some(ItemCategoryType::Ammo));
    assert_eq!(runtime.selected, None);
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    assert_eq!(
        column_texts(rig.app.world_mut(), InventorySideType::Own),
        ["Nothing in this category."]
    );
    take_churn(&mut rig.app);

    // A round weighs a fraction of a kilogram.
    let rounds_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        *row == InventoryRow {
            side: InventorySideType::Partner,
            item: ItemType::PdcRound,
        }
    });
    click_at(&mut rig, rounds_row);
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    take_churn(&mut rig.app);
    let world = rig.app.world_mut();
    assert_eq!(
        inspector(world, InventoryInspectorField::Weight).as_deref(),
        Some("0.2 kg")
    );
    assert!(total_weight_shown(world));

    // The current filter again rebuilds nothing and stays silent.
    click_at(&mut rig, ammo);
    assert!(take_cues(&mut rig.app).is_empty());
    let churn = take_churn(&mut rig.app);
    assert_eq!((churn.spawned, churn.despawned), (0, 0), "{churn:?}");

    // Repair shows them again.
    let repair = centre_of::<InventoryFilterChip>(rig.app.world_mut(), |chip| {
        chip.0 == Some(ItemCategoryType::Repair)
    });
    click_at(&mut rig, repair);
    assert_eq!(
        rig.app.world().resource::<InventoryRuntime>().filter,
        Some(ItemCategoryType::Repair)
    );
    assert_eq!(
        column_texts(rig.app.world_mut(), InventorySideType::Own),
        ["Hull plate", "x12"]
    );
}

#[test]
#[should_panic(expected = "has no ShipInventory")]
fn a_player_ship_without_an_inventory_panics_the_panel() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<InventoryRuntime>();
    app.add_systems(Update, update_inventory_panel);
    let root = app.world_mut().spawn(Node::default()).id();
    spawn_inventory_body(&mut app, root);
    let player = app
        .world_mut()
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Name::new("NOVA"),
        ))
        .id();
    app.world_mut().entity_mut(player).remove::<ShipInventory>();

    app.update();
}

/// Every hull plate `ship` carries.
fn plates(world: &World, ship: Entity) -> u32 {
    world
        .get::<ShipInventory>(ship)
        .expect("a ship root carries a ShipInventory")
        .count(ItemType::HullPlate)
}

/// The note line's text, if a result is showing.
fn note(app: &App) -> Option<String> {
    app.world()
        .resource::<InventoryRuntime>()
        .note
        .as_ref()
        .map(|(note, _)| note.clone())
}

#[test]
fn confirm_moves_items_between_docked_ships_and_a_refusal_changes_nothing() {
    use InventoryActionType::{Give, Take};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_resource::<InventoryRuntime>();
    app.add_message::<InventoryActionCommand>();
    app.add_systems(Update, apply_inventory_action_commands);
    hear_ui_cues(&mut app);
    let player = spawn_player(app.world_mut(), 12);
    let partner = dock_partner(app.world_mut(), player, "Derelict", 8);
    let total = |app: &App| plates(app.world(), player) + plates(app.world(), partner);

    // Write one confirmed command with its draft open, and run it.
    let confirm = |app: &mut App, action, quantity: Option<u32>| {
        let command = InventoryActionCommand {
            action,
            item: ItemType::HullPlate,
            quantity,
        };
        app.world_mut().resource_mut::<InventoryRuntime>().draft = Some(InventoryDraft {
            action,
            item: ItemType::HullPlate,
            quantity,
        });
        app.world_mut().write_message(command);
        app.update();
        let draft_open = app.world().resource::<InventoryRuntime>().draft.is_some();
        (note(app), take_cues(app), draft_open)
    };
    let moved = |text: &str| (Some(text.to_string()), vec![UiSfx::MenuSelect], false);
    let refused = |text: &str| (Some(text.to_string()), vec![UiSfx::EditorDeny], true);

    // A live ship that was never neutralized: Take is stealing, Give is fine.
    assert_eq!(
        confirm(&mut app, Take, Some(3)),
        refused("Refused: Derelict is not neutralized or lootable")
    );
    assert_eq!(
        (plates(app.world(), player), plates(app.world(), partner)),
        (12, 8)
    );
    assert_eq!(
        confirm(&mut app, Give, Some(2)),
        moved("Gave 2 Hull plate to Derelict")
    );
    assert_eq!(
        (plates(app.world(), player), plates(app.world(), partner)),
        (10, 10)
    );

    // Lootable: Take moves; each bad quantity refuses with nothing moved.
    app.world_mut()
        .entity_mut(partner)
        .insert(LootableShipMarker);
    assert_eq!(
        confirm(&mut app, Take, Some(3)),
        moved("Took 3 Hull plate from Derelict")
    );
    assert_eq!(
        (plates(app.world(), player), plates(app.world(), partner)),
        (13, 7)
    );
    assert_eq!(
        confirm(&mut app, Take, None),
        refused("Refused: enter a quantity")
    );
    assert_eq!(
        confirm(&mut app, Take, Some(0)),
        refused("Refused: quantity is zero")
    );
    assert_eq!(
        confirm(&mut app, Take, Some(8)),
        refused("Refused: only 7 Hull plate in Derelict")
    );
    assert_eq!(
        confirm(&mut app, Give, Some(14)),
        refused("Refused: only 13 Hull plate in NOVA")
    );
    assert_eq!(
        (plates(app.world(), player), plates(app.world(), partner)),
        (13, 7)
    );

    // Neutralized, not lootable: Take moves the whole stack and drops it.
    app.world_mut()
        .entity_mut(partner)
        .remove::<LootableShipMarker>()
        .insert(NeutralizedMarker);
    assert_eq!(
        confirm(&mut app, Take, Some(7)),
        moved("Took 7 Hull plate from Derelict")
    );
    assert!(app
        .world()
        .get::<ShipInventory>(partner)
        .unwrap()
        .is_empty());
    assert_eq!(total(&app), 20);

    // A target without the room refuses the whole move.
    app.world_mut().entity_mut(partner).insert(hold(39));
    assert_eq!(
        confirm(&mut app, Give, Some(2)),
        refused("Refused: Derelict has room for 10 kg more")
    );
    assert_eq!(
        (plates(app.world(), player), plates(app.world(), partner)),
        (20, 39)
    );
    app.world_mut()
        .entity_mut(partner)
        .insert(ShipInventory::default());

    // A command that arrives after the undock moves nothing.
    app.world_mut().entity_mut(player).remove::<DockedShip>();
    assert_eq!(
        confirm(&mut app, Give, Some(1)),
        refused("Refused: not docked")
    );
    assert_eq!(plates(app.world(), player), 20);
}

/// Buy and Sell move items one way and the exact price the other way, in
/// one run; every refusal moves neither. Items and credits both conserve.
#[test]
fn confirm_trades_items_for_credits_and_a_refusal_changes_nothing() {
    use InventoryActionType::{Buy, Sell};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_resource::<InventoryRuntime>();
    app.add_message::<InventoryActionCommand>();
    app.add_systems(Update, apply_inventory_action_commands);
    hear_ui_cues(&mut app);
    let player = spawn_player(app.world_mut(), 12);
    let partner = dock_partner(app.world_mut(), player, "Trader", 8);
    app.world_mut().entity_mut(player).insert(ShipCredits(100));
    app.world_mut()
        .entity_mut(partner)
        .insert(ShipCredits(1_000));
    let state = |app: &App| {
        let credits = |ship| app.world().get::<ShipCredits>(ship).unwrap().0;
        (
            plates(app.world(), player),
            credits(player),
            plates(app.world(), partner),
            credits(partner),
        )
    };
    let confirm = |app: &mut App, action, quantity: Option<u32>| {
        app.world_mut().resource_mut::<InventoryRuntime>().draft = Some(InventoryDraft {
            action,
            item: ItemType::HullPlate,
            quantity,
        });
        app.world_mut().write_message(InventoryActionCommand {
            action,
            item: ItemType::HullPlate,
            quantity,
        });
        app.update();
        let draft_open = app.world().resource::<InventoryRuntime>().draft.is_some();
        let (items, player_cr, partner_items, partner_cr) = state(app);
        assert_eq!(items + partner_items, 20, "items conserve");
        assert_eq!(player_cr + partner_cr, 1_100, "credits conserve");
        (note(app), take_cues(app), draft_open)
    };
    let traded = |text: &str| (Some(text.to_string()), vec![UiSfx::MenuSelect], false);
    let refused = |text: &str| (Some(text.to_string()), vec![UiSfx::EditorDeny], true);

    // A hull plate asks 40 cr and bids 30 cr.
    assert_eq!(
        confirm(&mut app, Buy, Some(2)),
        traded("Bought 2 Hull plate from Trader for 80 cr")
    );
    assert_eq!(state(&app), (14, 20, 6, 1_080));
    assert_eq!(
        confirm(&mut app, Buy, Some(1)),
        refused("Refused: NOVA has only 20 cr")
    );
    assert_eq!(
        confirm(&mut app, Sell, Some(3)),
        traded("Sold 3 Hull plate to Trader for 90 cr")
    );
    assert_eq!(state(&app), (11, 110, 9, 990));
    assert_eq!(
        confirm(&mut app, Sell, Some(12)),
        refused("Refused: only 11 Hull plate in NOVA")
    );
    assert_eq!(
        confirm(&mut app, Buy, Some(10)),
        refused("Refused: only 9 Hull plate in Trader")
    );
    assert_eq!(
        confirm(&mut app, Sell, Some(0)),
        refused("Refused: quantity is zero")
    );

    // The buyer's hold binds before its credits.
    app.world_mut()
        .entity_mut(partner)
        .insert(ShipInventory::new(100_000, [(ItemType::HullPlate, 9)]));
    assert_eq!(
        confirm(&mut app, Sell, Some(2)),
        refused("Refused: Trader has room for 10 kg more")
    );

    // A lootable ship is a wreck, not a market.
    app.world_mut()
        .entity_mut(partner)
        .insert(LootableShipMarker);
    assert_eq!(
        confirm(&mut app, Buy, Some(1)),
        refused("Refused: Trader does not trade")
    );
    assert_eq!(state(&app), (11, 110, 9, 990));
}

/// A click moves the docked partner's whole credit balance in one
/// confirmation and zeros it; an intact, non-neutralized partner is never
/// robbed; repeating the action against a zeroed balance refuses again; and
/// an overflowing balance refuses without moving anything.
#[test]
fn take_credits_moves_the_whole_balance_once_and_a_refusal_changes_nothing() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_resource::<InventoryRuntime>();
    app.add_message::<CreditTakeCommand>();
    app.add_systems(Update, apply_credit_take_commands);
    hear_ui_cues(&mut app);
    let player = spawn_player(app.world_mut(), 12);
    let partner = dock_partner(app.world_mut(), player, "Derelict", 8);
    app.world_mut().entity_mut(player).insert(ShipCredits(100));
    app.world_mut().entity_mut(partner).insert(ShipCredits(500));
    let credits = |app: &App, ship| app.world().get::<ShipCredits>(ship).unwrap().0;
    let take = |app: &mut App| {
        app.world_mut().write_message(CreditTakeCommand);
        app.update();
        (note(app), take_cues(app))
    };

    // A live ship that was never neutralized is not robbed.
    assert_eq!(
        take(&mut app),
        (
            Some("Refused: Derelict is not neutralized or lootable".to_string()),
            vec![UiSfx::EditorDeny]
        )
    );
    assert_eq!((credits(&app, player), credits(&app, partner)), (100, 500));

    // Lootable: one click takes the whole balance and zeros the partner.
    app.world_mut()
        .entity_mut(partner)
        .insert(LootableShipMarker);
    assert_eq!(
        take(&mut app),
        (
            Some("Took 500 cr from Derelict".to_string()),
            vec![UiSfx::MenuSelect]
        )
    );
    assert_eq!((credits(&app, player), credits(&app, partner)), (600, 0));

    // Repeating the action against the zeroed balance refuses again.
    assert_eq!(
        take(&mut app),
        (
            Some("Refused: Derelict holds no credits".to_string()),
            vec![UiSfx::EditorDeny]
        )
    );
    assert_eq!((credits(&app, player), credits(&app, partner)), (600, 0));

    // An overflowing take refuses without moving anything.
    app.world_mut()
        .entity_mut(player)
        .insert(ShipCredits(u32::MAX - 2));
    app.world_mut().entity_mut(partner).insert(ShipCredits(3));
    assert_eq!(
        take(&mut app),
        (
            Some("Refused: your ship cannot hold more credits".to_string()),
            vec![UiSfx::EditorDeny]
        )
    );
    assert_eq!(
        (credits(&app, player), credits(&app, partner)),
        (u32::MAX - 2, 3)
    );
}

/// The partner header's Take credits button shows only while the partner is
/// eligible (neutralized or lootable) and holds a credit above zero, even
/// with an empty hold, and never for an intact trading partner.
#[test]
fn take_credits_button_shows_only_for_an_eligible_nonzero_balance() {
    let (mut rig, player) = inventory_rig();
    let partner = dock_partner(rig.app.world_mut(), player, "Derelict", 8);
    rig.app
        .world_mut()
        .entity_mut(partner)
        .insert((ShipCredits(500), ShipInventory::default()));
    settle(&mut rig.app);
    let shown = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Node, With<InventoryTakeCreditsButton>>()
            .single(app.world())
            .expect("the pane spawns one Take credits button")
            .display
            != Display::None
    };

    // An intact, non-neutralized partner with credits: hidden.
    assert!(!shown(&mut rig.app));

    // Lootable with a nonzero balance and an empty hold: shown.
    rig.app
        .world_mut()
        .entity_mut(partner)
        .insert(LootableShipMarker);
    settle(&mut rig.app);
    assert!(shown(&mut rig.app));

    // A zero balance hides it again, even while still lootable.
    rig.app
        .world_mut()
        .entity_mut(partner)
        .insert(ShipCredits(0));
    settle(&mut rig.app);
    assert!(!shown(&mut rig.app));
}

/// Confirm follows the live plan: disabled on each refusal the summary shows,
/// so neither a click nor a triggered `Activate` sends it, and enabled again
/// once the quantity, room or credits allow the trade. A Buy that fills the
/// hold and spends every credit exactly is allowed.
#[test]
fn confirm_is_disabled_while_the_draft_is_refused_and_enabled_at_exact_room_and_credits() {
    let (mut rig, player) = inventory_rig();
    // 380 of 400 kg: room for two 10 kg plates; 80 cr buys two at 40 cr.
    rig.app
        .world_mut()
        .entity_mut(player)
        .insert((hold(38), ShipCredits(80)));
    let partner = dock_partner(rig.app.world_mut(), player, "Trader", 8);
    rig.app
        .world_mut()
        .entity_mut(partner)
        .insert(ShipCredits(1_000));
    settle(&mut rig.app);
    let state = |app: &App| {
        let credits = |ship| app.world().get::<ShipCredits>(ship).unwrap().0;
        (
            plates(app.world(), player),
            credits(player),
            plates(app.world(), partner),
            credits(partner),
        )
    };
    let confirm = only::<InventoryDraftConfirm>(rig.app.world_mut());
    let field = only::<InventoryDraftField>(rig.app.world_mut());
    let form = |app: &mut App| {
        let world = app.world_mut();
        let summary = world
            .query::<(&InventoryInspectorField, &Text)>()
            .iter(world)
            .find(|(each, _)| **each == InventoryInspectorField::DraftSummary)
            .map(|(_, text)| text.0.clone())
            .expect("the form has a summary");
        (summary, world.get::<InteractionDisabled>(confirm).is_none())
    };
    let set_field = |app: &mut App, text: &str| {
        app.world_mut().get_mut::<TextFieldValue>(field).unwrap().0 = text.to_string();
        settle(app);
    };

    let partner_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        row.side == InventorySideType::Partner
    });
    click_at(&mut rig, partner_row);
    assert_eq!(
        form(&mut rig.app),
        ("Price 40 cr, you after: 40 cr".to_string(), true)
    );

    // Three plates overfill the hold: disabled, and neither a click nor a
    // triggered Activate moves anything or says anything.
    set_field(&mut rig.app, "3");
    assert_eq!(
        form(&mut rig.app),
        ("Refused: NOVA has room for 20 kg more".to_string(), false)
    );
    take_cues(&mut rig.app);
    let confirm_at = centre_of::<InventoryDraftConfirm>(rig.app.world_mut(), |_| true);
    click_at(&mut rig, confirm_at);
    rig.app.world_mut().trigger(Activate { entity: confirm });
    settle(&mut rig.app);
    assert_eq!(state(&rig.app), (38, 80, 8, 1_000));
    assert_eq!(note(&rig.app), None);
    assert!(take_cues(&mut rig.app).is_empty());
    assert_eq!(draft_quantity(&rig.app), Some(3));

    set_field(&mut rig.app, "2x");
    assert_eq!(
        form(&mut rig.app),
        ("Type a whole number".to_string(), false)
    );

    // Two plates fill the hold and spend every credit exactly: allowed.
    set_field(&mut rig.app, "2");
    assert_eq!(
        form(&mut rig.app),
        ("Price 80 cr, you after: 0 cr".to_string(), true)
    );

    // A credit short disables it; the credit back enables it.
    rig.app
        .world_mut()
        .entity_mut(player)
        .insert(ShipCredits(79));
    settle(&mut rig.app);
    assert_eq!(
        form(&mut rig.app),
        ("Refused: NOVA has only 79 cr".to_string(), false)
    );
    rig.app
        .world_mut()
        .entity_mut(player)
        .insert(ShipCredits(80));
    settle(&mut rig.app);
    assert!(form(&mut rig.app).1);

    // The shorter summary reflows the form, so aim at Confirm again.
    let confirm_at = centre_of::<InventoryDraftConfirm>(rig.app.world_mut(), |_| true);
    click_at(&mut rig, confirm_at);
    assert_eq!(state(&rig.app), (40, 0, 6, 1_080));
    assert_eq!(rig.app.world().resource::<InventoryRuntime>().draft, None);
    assert_eq!(
        note(&rig.app).as_deref(),
        Some("Bought 2 Hull plate from Trader for 80 cr")
    );
}

#[test]
fn confirm_jettisons_through_the_intake_and_a_refusal_changes_nothing() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_resource::<InventoryRuntime>();
    app.add_message::<InventoryActionCommand>();
    app.add_systems(Update, apply_inventory_action_commands);
    hear_ui_cues(&mut app);
    let player = spawn_player(app.world_mut(), 12);
    let command = |quantity: Option<u32>| InventoryActionCommand {
        action: InventoryActionType::Jettison,
        item: ItemType::HullPlate,
        quantity,
    };
    // Write confirmed commands with their draft open, and run them.
    let confirm = |app: &mut App, quantities: &[Option<u32>]| {
        for &quantity in quantities {
            app.world_mut().resource_mut::<InventoryRuntime>().draft = Some(InventoryDraft {
                action: InventoryActionType::Jettison,
                item: ItemType::HullPlate,
                quantity,
            });
            app.world_mut().write_message(command(quantity));
        }
        app.update();
        let draft_open = app.world().resource::<InventoryRuntime>().draft.is_some();
        (note(app), take_cues(app), draft_open)
    };
    let dropped = |text: &str| (Some(text.to_string()), vec![UiSfx::MenuSelect], false);
    let refused = |text: &str| (Some(text.to_string()), vec![UiSfx::EditorDeny], true);
    let queue = |app: &App, intake: Entity| {
        app.world()
            .get::<CargoIntakeEjectionQueue>(intake)
            .map(|queue| queue.0.iter().map(CargoCanister::total_mass_g).collect())
    };

    assert_eq!(
        confirm(&mut app, &[Some(4)]),
        refused("Refused: no working cargo intake")
    );
    assert_eq!(plates(app.world(), player), 12);

    let intake = app
        .world_mut()
        .spawn((ChildOf(player), CargoIntakeSectionMarker))
        .id();
    assert_eq!(
        confirm(&mut app, &[None]),
        refused("Refused: enter a quantity")
    );
    assert_eq!(
        confirm(&mut app, &[Some(0)]),
        refused("Refused: quantity is zero")
    );
    assert_eq!(
        confirm(&mut app, &[Some(13)]),
        refused("Refused: only 12 Hull plate in NOVA")
    );
    assert_eq!(plates(app.world(), player), 12);
    assert_eq!(queue(&app, intake), None);

    // The stack leaves the hold and waits on the intake in the same run; a
    // second jettison in that run merges into the waiting canister.
    assert_eq!(
        confirm(&mut app, &[Some(4), Some(1)]),
        (
            Some("Jettisoned 1 Hull plate: 1 canister queued".to_string()),
            vec![UiSfx::MenuSelect, UiSfx::MenuSelect],
            false
        )
    );
    assert_eq!(plates(app.world(), player), 7);
    assert_eq!(queue(&app, intake), Some(vec![50_000]));
    assert_eq!(
        confirm(&mut app, &[Some(1)]),
        dropped("Jettisoned 1 Hull plate: 1 canister queued")
    );
    assert_eq!(plates(app.world(), player), 6);
    assert_eq!(queue(&app, intake), Some(vec![60_000]));

    // Past one canister, Confirm removes the whole quantity at once: the
    // waiting 190 kg canister takes one plate and new canisters the rest.
    app.world_mut().entity_mut(player).insert(hold(39));
    app.world_mut()
        .entity_mut(intake)
        .insert(CargoIntakeEjectionQueue(VecDeque::from([
            CargoCanister::new(ItemType::HullPlate, 19),
        ])));
    assert_eq!(
        confirm(&mut app, &[Some(40)]),
        refused("Refused: only 39 Hull plate in NOVA")
    );
    assert_eq!(plates(app.world(), player), 39);
    assert_eq!(queue(&app, intake), Some(vec![190_000]));
    assert_eq!(
        confirm(&mut app, &[Some(39)]),
        dropped("Jettisoned 39 Hull plate: 3 canisters queued")
    );
    assert_eq!(plates(app.world(), player), 0);
    assert_eq!(queue(&app, intake), Some(vec![200_000, 200_000, 180_000]));
    app.world_mut()
        .entity_mut(intake)
        .remove::<CargoIntakeEjectionQueue>();
    app.world_mut().entity_mut(player).insert(hold(12));

    // A disabled intake is no intake, and a docked ship drops nothing.
    app.world_mut()
        .entity_mut(intake)
        .insert(SectionInactiveMarker);
    assert_eq!(
        confirm(&mut app, &[Some(1)]),
        refused("Refused: no working cargo intake")
    );
    app.world_mut()
        .entity_mut(intake)
        .remove::<SectionInactiveMarker>();
    dock_partner(app.world_mut(), player, "Derelict", 8);
    assert_eq!(
        confirm(&mut app, &[Some(1)]),
        refused("Refused: undock to jettison")
    );
    assert_eq!(plates(app.world(), player), 12);
    assert_eq!(queue(&app, intake), None);
}

/// The one entity carrying `C`.
fn only<C: Component>(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<C>>()
        .single(world)
        .expect("the form has one of each control")
}

/// The open draft's quantity.
fn draft_quantity(app: &App) -> Option<u32> {
    app.world()
        .resource::<InventoryRuntime>()
        .draft
        .expect("a draft is open")
        .quantity
}

#[test]
fn a_selected_row_opens_a_one_unit_draft_that_every_quantity_control_sets() {
    let (mut rig, player) = inventory_rig();
    let partner = dock_partner(rig.app.world_mut(), player, "Derelict", 8);
    rig.app
        .world_mut()
        .entity_mut(partner)
        .insert(LootableShipMarker);
    settle(&mut rig.app);
    take_cues(&mut rig.app);

    // The derelict's row opens a Take of one.
    let partner_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        row.side == InventorySideType::Partner
    });
    click_at(&mut rig, partner_row);
    assert_eq!(
        rig.app.world().resource::<InventoryRuntime>().draft,
        Some(InventoryDraft {
            action: InventoryActionType::Take,
            item: ItemType::HullPlate,
            quantity: Some(1),
        })
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    // The total weighs the draft's quantity, never the source stack of 8.
    let total_weight = |world: &mut World| {
        let shown = world
            .query::<(&InspectorPart, &Node)>()
            .iter(world)
            .any(|(part, node)| {
                *part == InspectorPart::TotalWeight && node.display != Display::None
            });
        let text = world
            .query::<(&InventoryInspectorField, &Text)>()
            .iter(world)
            .find(|(each, _)| **each == InventoryInspectorField::TotalWeight)
            .map(|(_, text)| text.0.clone());
        shown.then(|| text.expect("the total weight row has a value"))
    };
    assert_eq!(total_weight(rig.app.world_mut()).as_deref(), Some("10 kg"));
    let world = rig.app.world_mut();
    let (field, slider, wheel) = (
        only::<InventoryDraftField>(world),
        only::<InventoryDraftSlider>(world),
        only::<InventoryDraftWheel>(world),
    );
    assert_eq!(
        world.get::<Node>(slider).unwrap().display,
        Display::Flex,
        "a stock of 8 shows the slider"
    );
    take_churn(&mut rig.app);

    // The wheel over the quantity row steps by one, one tick each.
    let over_wheel = centre_of::<InventoryDraftWheel>(rig.app.world_mut(), |_| true);
    move_cursor_to(&mut rig, over_wheel);
    for _ in 0..2 {
        let window = rig
            .app
            .world_mut()
            .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
            .single(rig.app.world())
            .unwrap();
        rig.app
            .world_mut()
            .write_message(bevy::window::WindowEvent::MouseWheel(
                bevy::input::mouse::MouseWheel {
                    unit: bevy::input::mouse::MouseScrollUnit::Line,
                    x: 0.0,
                    y: 1.0,
                    window,
                    phase: bevy::input::touch::TouchPhase::Moved,
                },
            ));
        settle(&mut rig.app);
    }
    assert_eq!(draft_quantity(&rig.app), Some(3));
    assert_eq!(total_weight(rig.app.world_mut()).as_deref(), Some("30 kg"));
    assert_eq!(take_cues(&mut rig.app), [UiSfx::UiTick, UiSfx::UiTick]);
    assert_eq!(rig.app.world().get::<TextFieldValue>(field).unwrap().0, "3");

    // The slider sets it; the same value again is silent.
    for _ in 0..2 {
        rig.app.world_mut().trigger(ValueChange {
            source: slider,
            value: 6.0_f32,
            is_final: true,
        });
        settle(&mut rig.app);
    }
    assert_eq!(draft_quantity(&rig.app), Some(6));
    assert_eq!(total_weight(rig.app.world_mut()).as_deref(), Some("60 kg"));
    assert_eq!(take_cues(&mut rig.app), [UiSfx::UiTick]);

    // Typed text sets it; text that is not a number stays as typed, marks
    // the field and says so, with no tick.
    rig.app
        .world_mut()
        .get_mut::<TextFieldValue>(field)
        .unwrap()
        .0 = "4".to_string();
    settle(&mut rig.app);
    assert_eq!(draft_quantity(&rig.app), Some(4));
    assert_eq!(total_weight(rig.app.world_mut()).as_deref(), Some("40 kg"));
    assert_eq!(take_cues(&mut rig.app), [UiSfx::UiTick]);
    rig.app
        .world_mut()
        .get_mut::<TextFieldValue>(field)
        .unwrap()
        .0 = "4x".to_string();
    settle(&mut rig.app);
    assert_eq!(draft_quantity(&rig.app), None);
    assert_eq!(
        total_weight(rig.app.world_mut()),
        None,
        "no quantity, no total row"
    );
    assert!(take_cues(&mut rig.app).is_empty());
    let world = rig.app.world_mut();
    assert_eq!(world.get::<TextFieldValue>(field).unwrap().0, "4x");
    assert!(world.get::<TextFieldError>(field).is_some());
    let summary = world
        .query::<(&InventoryInspectorField, &Text)>()
        .iter(world)
        .find(|(each, _)| **each == InventoryInspectorField::DraftSummary)
        .map(|(_, text)| text.0.clone());
    assert_eq!(summary.as_deref(), Some("Type a whole number"));

    // All takes the whole source stack, clears the error and rewrites the field.
    let all = centre_of::<Name>(rig.app.world_mut(), |name| {
        name.as_str() == "InventoryDraftAll"
    });
    click_at(&mut rig, all);
    assert_eq!(draft_quantity(&rig.app), Some(8));
    assert_eq!(total_weight(rig.app.world_mut()).as_deref(), Some("80 kg"));
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    let world = rig.app.world_mut();
    assert_eq!(world.get::<TextFieldValue>(field).unwrap().0, "8");
    assert!(world.get::<TextFieldError>(field).is_none());

    // Every change rewrote the form in place: no node came or went.
    let churn = take_churn(&mut rig.app);
    assert_eq!((churn.spawned, churn.despawned), (0, 0), "{churn:?}");
    let world = rig.app.world_mut();
    assert_eq!(
        (
            only::<InventoryDraftField>(world),
            only::<InventoryDraftSlider>(world),
            only::<InventoryDraftWheel>(world),
        ),
        (field, slider, wheel)
    );

    // Confirm moves the stack, closes the form and leaves the note.
    let confirm = centre_of::<Name>(rig.app.world_mut(), |name| {
        name.as_str() == "InventoryDraftConfirm"
    });
    click_at(&mut rig, confirm);
    assert_eq!(
        (
            plates(rig.app.world(), player),
            plates(rig.app.world(), partner)
        ),
        (20, 0)
    );
    assert_eq!(rig.app.world().resource::<InventoryRuntime>().draft, None);
    assert_eq!(
        note(&rig.app).as_deref(),
        Some("Took 8 Hull plate from Derelict")
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);

    // A source of one hides the slider; the field and All still set it.
    rig.app.world_mut().entity_mut(player).insert(hold(1));
    settle(&mut rig.app);
    let own_row = centre_of::<InventoryRow>(rig.app.world_mut(), |row| {
        row.side == InventorySideType::Own
    });
    click_at(&mut rig, own_row);
    assert_eq!(
        rig.app
            .world()
            .resource::<InventoryRuntime>()
            .draft
            .map(|draft| draft.action),
        Some(InventoryActionType::Give)
    );
    assert_eq!(
        rig.app.world().get::<Node>(slider).unwrap().display,
        Display::None
    );
}

/// A generated wreck as the open world spawns it, docked through the real
/// docking systems and looted and restocked through the pane's own transfer
/// system.
mod generated_wreck {
    use nova_events::prelude::{Meters, Meters3, MetersPerSecond3};
    use nova_gameplay::test_support::{settle as settle_physics, unfinished_integrity_physics_app};
    use nova_scenario::prelude::{
        resolve_ship_design, spaceship_scenario_object, GameShipDesigns, ShipDesignPrototype,
        ShipDesignSource, SpaceshipConfig, SpaceshipController, SpaceshipPlugin,
    };
    use nova_ship::{
        flight::prelude::NovaFlightPlugin,
        prelude::{
            DockingConnectionRequest, GameSections, PDControllerPlugin, SectionConfig,
            SpaceshipSectionPlugin,
        },
    };
    use nova_world::{materialize_sector, prelude::*, ObserverBody};
    use nova_world_base::prelude::{
        generate_wreck, ship_stock, ShipLayoutRequest, ShipPartFamilyType, ShipPartPack,
        ShipPartSnapshot,
    };
    use serde::Deserialize;

    use super::*;

    /// The open world's player hull and the collar it docks on, facing -X.
    const WARSHIP: &str = "block_line_warship";
    const PLAYER_COLLAR: &str = "port_collar";
    /// Face-to-face gap at capture, cells: inside the one-cell capture
    /// distance.
    const GAP: f32 = 0.5;
    /// What the player carries before the dock.
    const PLAYER_PLATES: u32 = 12;

    #[derive(Deserialize)]
    #[expect(
        clippy::large_enum_variant,
        reason = "each catalog entry is parsed once and moved out"
    )]
    enum Entry {
        Section(SectionConfig),
        Ship(ShipDesignPrototype),
    }

    /// The shipped section and ship catalogs.
    fn catalog() -> (Vec<SectionConfig>, Vec<ShipDesignPrototype>) {
        let load = |path: &str| -> Vec<Entry> {
            let file = format!("{}/../../assets/base/{path}", env!("CARGO_MANIFEST_DIR"));
            ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
        };
        let (mut sections, mut ships) = (Vec::new(), Vec::new());
        for entry in load("sections/base.content.ron")
            .into_iter()
            .chain(load("ships/base.content.ron"))
        {
            match entry {
                Entry::Section(section) => sections.push(section),
                Entry::Ship(ship) => ships.push(ship),
            }
        }
        (sections, ships)
    }

    /// A generator whose every cell holds one ship.
    #[derive(Clone, Debug)]
    struct OneShip(SectorShip);

    impl SectorGenerator for OneShip {
        fn validate(&self, _geometry: WorldGeometry) -> Result<(), SectorFault> {
            Ok(())
        }

        fn generate(&self, input: SectorGenerationInput) -> Result<SectorManifest, SectorFault> {
            Ok(SectorManifest {
                coord: input.coord,
                asteroids: Vec::new(),
                planets: Vec::new(),
                ships: vec![self.0.clone()],
            })
        }
    }

    /// How many of `item` `ship` carries.
    fn count(world: &World, ship: Entity, item: ItemType) -> u32 {
        world
            .get::<ShipInventory>(ship)
            .expect("a ship root carries a ShipInventory")
            .count(item)
    }

    /// Write one confirmed command with its draft open, run it, and return
    /// the note line.
    fn confirm(
        app: &mut App,
        action: InventoryActionType,
        item: ItemType,
        quantity: u32,
    ) -> Option<String> {
        let command = InventoryActionCommand {
            action,
            item,
            quantity: Some(quantity),
        };
        app.world_mut().resource_mut::<InventoryRuntime>().draft = Some(InventoryDraft {
            action,
            item,
            quantity: Some(quantity),
        });
        app.world_mut().write_message(command);
        app.update();
        note(app)
    }

    #[test]
    fn a_docked_generated_wreck_gives_its_stock_by_take_and_takes_it_back_by_give() {
        let (sections, ships) = catalog();
        let snapshot = ShipPartSnapshot::build(&[ShipPartPack {
            id: "base".to_string(),
            dependencies: Vec::new(),
            sections: sections.clone(),
        }])
        .unwrap_or_else(|faults| panic!("the base snapshot builds: {faults:?}"));

        // An armored wreck: its weapons spawn inactive, and it must still
        // not read as neutralized.
        let request = ShipLayoutRequest {
            seed: 3,
            civilization: CivilizationId {
                world_seed: 7,
                node: [0, 0, 0],
            },
            role: ShipRoleType::Armored,
            advancement: 0.5,
        };
        let wreck =
            generate_wreck(&snapshot, request).unwrap_or_else(|failure| panic!("{failure}"));
        let stock = ship_stock(
            &snapshot,
            &wreck.design,
            request.role,
            SectorShipConditionType::Derelict,
            request.advancement,
            5,
        )
        .expect("a wreck's hull holds its role's lightest item");
        let (drawn_item, drawn) = stock
            .stacks()
            .next()
            .expect("ship_stock never returns empty stock");
        let dock = wreck
            .design
            .sections
            .iter()
            .filter(|section| {
                snapshot
                    .parts()
                    .iter()
                    .find(|part| part.id() == section.source.prototype_id())
                    .is_some_and(|part| part.family == ShipPartFamilyType::Docking)
            })
            .max_by(|a, b| a.position.x.total_cmp(&b.position.x))
            .expect("every generated ship docks")
            .position;
        assert!(dock.x > 0.0, "the wreck's starboard dock faces +X");

        let edge = Meters(32_000.0);
        let centre = SectorCoord::ORIGIN.centre(edge);
        let config = WorldConfig {
            seed: 7,
            sector_edge: edge,
            active_radius: 1,
            generator: OneShip(SectorShip {
                id: sector_id(SectorCoord::ORIGIN, "ship", 0),
                position: centre,
                rotation: Quat::IDENTITY,
                initial_velocity: MetersPerSecond3::ZERO,
                clearance: wreck.clearance,
                design: wreck.design.clone(),
                condition: SectorShipConditionType::Derelict,
                crew: None,
                civilization: request.civilization,
                role: request.role,
                stock,
                credits: 0,
            }),
        };
        let prepared =
            prepare_sector(config, SectorCoord::ORIGIN).expect("one valid wreck prepares");

        let mut app = unfinished_integrity_physics_app();
        app.add_plugins((
            PDControllerPlugin,
            SpaceshipSectionPlugin { render: false },
            SpaceshipPlugin,
            NovaFlightPlugin,
        ));
        app.insert_resource(GameSections(sections));
        app.insert_resource(GameShipDesigns(ships));
        app.init_resource::<InventoryRuntime>();
        app.add_message::<InventoryActionCommand>();
        app.add_systems(Update, apply_inventory_action_commands);
        hear_ui_cues(&mut app);
        app.finish();

        // The world spawns the wreck, with the observer far from it.
        let loaded = app.world().resource::<GameSections>().clone();
        let world = app.world_mut();
        materialize_sector(
            &mut world.commands(),
            prepared,
            &AssetRef::default(),
            &loaded,
            ObserverBody {
                position: centre + Meters3::new(0.0, 0.0, 10_000.0),
                reach: Meters::ZERO,
            },
        );
        world.flush();
        let wreck = world
            .query_filtered::<Entity, With<DerelictShipMarker>>()
            .single(world)
            .expect("the world spawned one wreck");

        // The player's port collar square to the wreck's dock, GAP apart.
        let (warship, errors) = resolve_ship_design(
            &ShipDesignSource::prototype(WARSHIP),
            app.world().resource::<GameShipDesigns>(),
            app.world().resource::<GameSections>(),
        );
        assert!(errors.is_empty(), "{errors:?}");
        let collar = warship
            .sections
            .iter()
            .find(|section| section.id == PLAYER_COLLAR)
            .expect("the warship has a port collar")
            .position;
        let player = app
            .world_mut()
            .spawn((
                Transform::from_translation(
                    centre.to_engine() + dock + Vec3::X * (1.0 + GAP) - collar,
                ),
                spaceship_scenario_object(SpaceshipConfig {
                    design: ShipDesignSource::prototype(WARSHIP),
                    controller: SpaceshipController::None,
                    initial_velocity: MetersPerSecond3::ZERO,
                    allegiance: None,
                    capabilities: default(),
                    inventory: ShipInventoryStock::new([(ItemType::HullPlate, PLAYER_PLATES)]),
                    lootable: false,
                    credits: 0,
                }),
            ))
            .id();
        settle_physics(&mut app);
        settle_physics(&mut app);
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerSpaceshipMarker);
        app.world_mut().trigger(DockingConnectionRequest {
            entity: player,
            target: wreck,
        });
        app.update();
        assert!(
            app.world().get::<DockedShip>(player).is_some(),
            "the real dock with the wreck was refused"
        );
        settle_physics(&mut app);

        let entity = app.world().entity(wreck);
        assert!(entity.contains::<LootableShipMarker>());
        assert!(!entity.contains::<NeutralizedMarker>());
        assert_eq!(entity.get::<Allegiance>(), Some(&Allegiance::Neutral));
        assert_eq!(
            count(app.world(), wreck, drawn_item),
            drawn,
            "the wreck spawns its stock"
        );
        let name = entity
            .get::<Name>()
            .expect("the wreck is named")
            .to_string();
        let label = drawn_item.label();
        let held = |app: &App| {
            (
                count(app.world(), player, drawn_item),
                count(app.world(), wreck, drawn_item),
            )
        };
        let before_take = held(&app).0;

        assert_eq!(
            confirm(&mut app, InventoryActionType::Take, drawn_item, drawn),
            Some(format!("Took {drawn} {label} from {name}"))
        );
        assert_eq!(held(&app), (before_take + drawn, 0));
        assert_eq!(
            confirm(&mut app, InventoryActionType::Give, drawn_item, 1),
            Some(format!("Gave 1 {label} to {name}"))
        );
        assert_eq!(held(&app), (before_take + drawn - 1, 1));
    }
}
