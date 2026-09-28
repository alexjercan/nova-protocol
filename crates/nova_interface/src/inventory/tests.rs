//! Live-tree tests for the Inventory pane: the rows drawn from the player's
//! and the docked partner's real inventories, the blank partner slot, and
//! filter and row selection through window picking.

use bevy::{
    ecs::system::RunSystemOnce,
    ui::{ComputedNode, UiGlobalTransform},
    ui_widgets::ValueChange,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{DockedHelmType, DockedShip, DockingConnection};
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

/// A player ship carrying `plates` hull plates.
fn spawn_player(world: &mut World, plates: u32) -> Entity {
    world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Name::new("NOVA"),
            [(ItemType::HullPlate, plates)]
                .into_iter()
                .collect::<ShipInventory>(),
        ))
        .id()
}

/// Dock `player` with a new ship named `name` carrying `plates` hull plates.
fn dock_partner(world: &mut World, player: Entity, name: &str, plates: u32) -> Entity {
    let partner = world
        .spawn((
            SpaceshipRootMarker,
            Name::new(name.to_string()),
            [(ItemType::HullPlate, plates)]
                .into_iter()
                .collect::<ShipInventory>(),
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

    // Undocked: the player's own stack, and no partner to read.
    app.update();
    let world = app.world_mut();
    assert_eq!(column_title(world, InventorySideType::Own), "NOVA");
    assert_eq!(
        column_texts(world, InventorySideType::Own),
        ["Hull plate", "x12"]
    );
    assert!(column_texts(world, InventorySideType::Partner).is_empty());

    // Docked: the partner column reads the partner's own inventory.
    dock_partner(app.world_mut(), player, "Picket", 40);
    app.update();
    let world = app.world_mut();
    assert_eq!(column_title(world, InventorySideType::Partner), "Picket");
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
    dock_partner(rig.app.world_mut(), player, "Picket", 40);
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
    let stock = rig
        .app
        .world_mut()
        .query::<(&InventoryInspectorField, &Text)>()
        .iter(rig.app.world())
        .find(|(field, _)| **field == InventoryInspectorField::Stock)
        .map(|(_, text)| text.0.clone());
    assert_eq!(stock.as_deref(), Some("x40 in Picket"));

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
    assert!(row_entities(rig.app.world_mut()).is_empty());
    take_churn(&mut rig.app);

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
    use ItemTransferType::{Give, Take};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_resource::<InventoryRuntime>();
    app.add_message::<InventoryTransferCommand>();
    app.add_systems(Update, apply_inventory_transfer_commands);
    hear_ui_cues(&mut app);
    let player = spawn_player(app.world_mut(), 12);
    let partner = dock_partner(app.world_mut(), player, "Derelict", 8);
    let total = |app: &App| plates(app.world(), player) + plates(app.world(), partner);

    // Write one confirmed command with its draft open, and run it.
    let confirm = |app: &mut App, transfer, quantity: Option<u32>| {
        let command = InventoryTransferCommand {
            transfer,
            item: ItemType::HullPlate,
            quantity,
        };
        app.world_mut().resource_mut::<InventoryRuntime>().draft = Some(InventoryDraft {
            transfer,
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

    // A target at the count's limit refuses rather than wrap.
    app.world_mut().entity_mut(partner).insert(
        [(ItemType::HullPlate, u32::MAX)]
            .into_iter()
            .collect::<ShipInventory>(),
    );
    assert_eq!(
        confirm(&mut app, Give, Some(1)),
        refused("Refused: Derelict cannot hold more Hull plate")
    );
    assert_eq!(plates(app.world(), player), 20);

    // A command that arrives after the undock moves nothing.
    app.world_mut().entity_mut(player).remove::<DockedShip>();
    assert_eq!(
        confirm(&mut app, Give, Some(1)),
        refused("Refused: not docked")
    );
    assert_eq!(plates(app.world(), player), 20);
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
            transfer: ItemTransferType::Take,
            item: ItemType::HullPlate,
            quantity: Some(1),
        })
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
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
    assert_eq!(take_cues(&mut rig.app), [UiSfx::UiTick]);
    rig.app
        .world_mut()
        .get_mut::<TextFieldValue>(field)
        .unwrap()
        .0 = "4x".to_string();
    settle(&mut rig.app);
    assert_eq!(draft_quantity(&rig.app), None);
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
    rig.app.world_mut().entity_mut(player).insert(
        [(ItemType::HullPlate, 1)]
            .into_iter()
            .collect::<ShipInventory>(),
    );
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
            .map(|draft| draft.transfer),
        Some(ItemTransferType::Give)
    );
    assert_eq!(
        rig.app.world().get::<Node>(slider).unwrap().display,
        Display::None
    );
}
