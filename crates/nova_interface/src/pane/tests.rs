//! Live-tree tests for the TAB interface card: a pane switch keeps the card,
//! its title row and its tabs and replaces only the pane body.

use bevy::ui::{ComputedNode, UiGlobalTransform};
use nova_input::prelude::{BindingSpec, InputSource, RegisterInputActions};
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
    let card = world
        .get::<ChildOf>(head)
        .expect("the title row is in the card")
        .parent();
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
    tabs.sort_by_key(|(_, value)| {
        InterfacePaneType::ALL
            .iter()
            .position(|(each, _)| each == value)
    });
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
    let title = world
        .get::<Text>(ids.title)
        .expect("the title is text")
        .0
        .clone();
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
        .query::<(
            &ButtonValue<InterfacePaneType>,
            &ComputedNode,
            &UiGlobalTransform,
        )>()
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
    app.register_input_actions(crate::bindings::interface_bindings());
    app.add_systems(
        Update,
        (rebuild_interface_body, refresh_pane_input_hints).chain(),
    );
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
    assert_eq!(
        shown(world),
        ("MAP".to_string(), vec![InterfacePaneType::Map])
    );
    let map_body = body_children(world);

    // Idle: nothing is spawned, despawned or rewritten.
    settle(&mut rig.app);
    assert_eq!(take_churn(&mut rig.app), NodeChurn::default());

    // A click on the Ship tab: one click cue, the same card, a new body.
    let ship = tab_centre(rig.app.world_mut(), InterfacePaneType::Ship);
    click_at(&mut rig, ship);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Ship
    );
    assert_eq!(take_cues(&mut rig.app), [UiSfx::MenuSelect]);
    let world = rig.app.world_mut();
    assert_eq!(card_ids(world), before);
    assert_eq!(
        shown(world),
        ("SHIP".to_string(), vec![InterfacePaneType::Ship])
    );
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

/// The window-space rect of a laid-out node.
fn rect_of(world: &World, entity: Entity) -> Rect {
    let node = world.get::<ComputedNode>(entity).expect("a laid-out node");
    let centre = world
        .get::<UiGlobalTransform>(entity)
        .expect("a laid-out node")
        .translation;
    Rect::from_center_size(centre, node.size())
}

/// The one footer, if the shown pane has one.
fn footer(world: &mut World) -> Option<Entity> {
    let footers: Vec<Entity> = world
        .query_filtered::<Entity, With<InterfacePaneFooter>>()
        .iter(world)
        .collect();
    assert!(footers.len() <= 1, "one footer at most: {footers:?}");
    footers.first().copied()
}

/// The footer's key hints, in spawn order.
fn key_hints(world: &mut World) -> Vec<String> {
    world
        .query_filtered::<&Text, With<PaneInputHint>>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect()
}

#[test]
fn map_and_ship_share_a_fixed_footer_under_the_view_and_inventory_has_none() {
    let mut rig = card_rig();
    for pane in [InterfacePaneType::Map, InterfacePaneType::Ship] {
        *rig.app.world_mut().resource_mut::<InterfacePaneType>() = pane;
        settle(&mut rig.app);
        let world = rig.app.world_mut();
        let body_id = card_ids(world).body;
        let body = rect_of(world, body_id);
        let footer = footer(world).unwrap_or_else(|| panic!("{pane:?} has a footer"));
        let foot = rect_of(world, footer);
        assert_eq!(foot.height(), PANE_FOOTER_PX, "{pane:?} footer height");
        assert!(
            foot.max.y <= body.max.y && foot.width() >= body.width() - 24.0 - 0.5,
            "{pane:?} footer {foot:?} spans the bottom of the body {body:?}"
        );
        let zones = world.get::<Children>(footer).map_or(0, |c| c.len());
        assert_eq!(zones, 3, "{pane:?} footer has legend, controls and summary");
        let split = world.get::<Children>(body_id).unwrap()[0];
        let [view, panel] = world.get::<Children>(split).unwrap()[..] else {
            panic!("{pane:?} splits into a view and a panel");
        };
        let (view, panel) = (rect_of(world, view), rect_of(world, panel));
        assert!(
            view.max.y <= foot.min.y && panel.max.y <= foot.min.y,
            "{pane:?} view {view:?} and panel {panel:?} sit above the footer {foot:?}"
        );
        assert!(
            panel.width() >= SIDE_PANEL_MIN_PX,
            "{pane:?} panel is {} wide",
            panel.width()
        );
        let hints = key_hints(world);
        assert!(
            !hints.is_empty() && hints.iter().all(|hint| !hint.is_empty()),
            "{pane:?} names its keys: {hints:?}"
        );
    }
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    settle(&mut rig.app);
    assert_eq!(footer(rig.app.world_mut()), None, "Inventory has no footer");
}

#[test]
fn a_rebound_key_moves_its_footer_hint() {
    let mut rig = card_rig();
    let hints = key_hints(rig.app.world_mut());
    assert!(hints.iter().any(|hint| hint == "G GOTO"), "{hints:?}");
    rig.app.world_mut().resource_mut::<InputBindings>().rebind(
        "map_goto",
        BindingSpec {
            keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
            gamepad: Vec::new(),
        },
    );
    settle(&mut rig.app);
    let hints = key_hints(rig.app.world_mut());
    assert!(
        hints.iter().any(|hint| hint == "J GOTO") && !hints.iter().any(|hint| hint == "G GOTO"),
        "{hints:?}"
    );
}
