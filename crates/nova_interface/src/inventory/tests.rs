//! Live-tree tests for the Inventory pane: the rows drawn from the player's
//! and the docked partner's real inventories, the blank partner slot, and
//! filter and row selection through window picking.

use bevy::{
    ecs::system::RunSystemOnce,
    ui::{ComputedNode, UiGlobalTransform},
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{DockedHelmType, DockedShip, DockingConnection};

use super::*;
use crate::{
    icons::InterfaceIcons,
    pointer_rig::{
        click_at, hear_ui_cues, pane_pointer_rig, settle, take_churn, take_cues, track_node_churn,
        PanePointerRig,
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
    rig.app.init_resource::<InventoryRuntime>();
    rig.app.add_systems(Update, update_inventory_panel);
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
    let runtime = *rig.app.world().resource::<InventoryRuntime>();
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
