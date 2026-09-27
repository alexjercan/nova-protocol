//! Live-tree tests for the TAB interface card: a pane switch keeps the card,
//! its title row and its tabs and replaces only the pane body.

use bevy::ui::{ComputedNode, UiGlobalTransform};
use nova_ui::widget::button_on_setting;

use super::*;
use crate::pointer_rig::{
    click_at, hear_ui_cues, pane_pointer_rig, settle, take_churn, take_cues, track_node_churn,
    NodeChurn, PanePointerRig,
};

/// The card's lasting entities: the card, the title row, the title text, and
/// the tabs in title-row order with whether each is selected.
#[derive(Debug, PartialEq, Eq)]
struct CardIds {
    card: Entity,
    head: Entity,
    title: Entity,
    body: Entity,
    tabs: Vec<Entity>,
}

fn card_ids(world: &mut World) -> CardIds {
    let head = world
        .query_filtered::<Entity, With<InterfacePaneHead>>()
        .single(world)
        .expect("the card has one title row");
    let body = world
        .query_filtered::<Entity, With<InterfacePaneBody>>()
        .single(world)
        .expect("the card has one pane body");
    let card = world.get::<ChildOf>(head).expect("the title row is in the card").parent();
    let title = world
        .query_filtered::<(Entity, &ChildOf), With<PanelHeadTitle>>()
        .iter(world)
        .find(|(_, parent)| parent.parent() == head)
        .map(|(entity, _)| entity)
        .expect("the title row has a title");
    let mut tabs: Vec<(Entity, InterfacePaneType)> = world
        .query::<(Entity, &ButtonValue<InterfacePaneType>)>()
        .iter(world)
        .map(|(entity, value)| (entity, value.0))
        .collect();
    tabs.sort_by_key(|(_, value)| InterfacePaneType::ALL.iter().position(|(each, _)| each == value));
    CardIds {
        card,
        head,
        title,
        body,
        tabs: tabs.into_iter().map(|(entity, _)| entity).collect(),
    }
}

/// The title text and the one pane whose tab is selected.
fn shown(world: &mut World) -> (String, Vec<InterfacePaneType>) {
    let ids = card_ids(world);
    let title = world.get::<Text>(ids.title).expect("the title is text").0.clone();
    let selected = world
        .query_filtered::<&ButtonValue<InterfacePaneType>, With<Selected>>()
        .iter(world)
        .map(|value| value.0)
        .collect();
    (title, selected)
}

/// The body's children.
fn body_children(world: &mut World) -> Vec<Entity> {
    let body = card_ids(world).body;
    world
        .get::<Children>(body)
        .map(|children| children.to_vec())
        .unwrap_or_default()
}

/// The window-space centre of the tab for `pane`.
fn tab_centre(world: &mut World, pane: InterfacePaneType) -> Vec2 {
    world
        .query::<(&ButtonValue<InterfacePaneType>, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .find(|(value, node, _)| value.0 == pane && node.size().x > 0.0)
        .map(|(_, _, xf)| xf.translation)
        .expect("the tab is laid out")
}

/// The interface card over the pointer rig, with the tab observer, cue capture
/// and churn counter, showing Map.
fn card_rig() -> PanePointerRig {
    let mut rig = pane_pointer_rig();
    let app = &mut rig.app;
    app.insert_resource(InterfaceIcons::blank());
    app.init_resource::<InterfacePaneType>();
    app.init_resource::<crate::inventory::InventoryRuntime>();
    app.add_observer(button_on_setting::<InterfacePaneType>);
    app.add_systems(Update, rebuild_interface_body);
    hear_ui_cues(app);
    track_node_churn(app);
    app.world_mut().spawn((
        InterfaceRootMarker,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    settle(app);
    take_cues(app);
    take_churn(app);
    rig
}

#[test]
fn a_pane_switch_keeps_the_card_title_row_and_tabs_and_replaces_only_the_body() {
    let mut rig = card_rig();
    let world = rig.app.world_mut();
    let before = card_ids(world);
    assert_eq!(shown(world), ("MAP".to_string(), vec![InterfacePaneType::Map]));
    let map_body = body_children(world);

    // Idle: nothing is spawned, despawned or rewritten.
    settle(&mut rig.app);
    assert_eq!(take_churn(&mut rig.app), NodeChurn::default());

    // A click on the Ship tab: one click cue, the same card, a new body.
    let ship = tab_centre(rig.app.world_mut(), InterfacePaneType::Ship);
    click_at(&mut rig, ship);
    assert_eq!(*rig.app.world().resource::<InterfacePaneType>(), InterfacePaneType::Ship);
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    let world = rig.app.world_mut();
    assert_eq!(card_ids(world), before);
    assert_eq!(shown(world), ("SHIP".to_string(), vec![InterfacePaneType::Ship]));
    let ship_body = body_children(world);
    assert!(
        ship_body.iter().all(|child| !map_body.contains(child)),
        "the Ship body must replace the Map body"
    );
    take_churn(&mut rig.app);

    // A click on the shown tab changes nothing and stays silent.
    click_at(&mut rig, ship);
    assert!(take_cues(&mut rig.app).is_empty());
    let churn = take_churn(&mut rig.app);
    assert_eq!((churn.spawned, churn.despawned), (0, 0), "{churn:?}");
    assert_eq!(body_children(rig.app.world_mut()), ship_body);

    // A keyboard or pad switch writes the resource: the tab and title follow
    // in place, and no click plays.
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    settle(&mut rig.app);
    assert!(take_cues(&mut rig.app).is_empty());
    let world = rig.app.world_mut();
    assert_eq!(card_ids(world), before);
    assert_eq!(
        shown(world),
        ("INVENTORY".to_string(), vec![InterfacePaneType::Inventory])
    );
    take_churn(&mut rig.app);
    settle(&mut rig.app);
    assert_eq!(take_churn(&mut rig.app), NodeChurn::default());
}
