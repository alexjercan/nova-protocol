//! Live-tree tests for the TAB interface card: a pane switch keeps the card,
//! its title row and its tabs and replaces only the pane body.

use bevy::ui::{ComputedNode, UiGlobalTransform};
use nova_input::prelude::{BindingSpec, InputSource, RegisterInputActions};
use nova_ship::prelude::{GameSections, PartsGalleryPlugin};
use nova_sound::prelude::{PlaySound, SoundAt};
use nova_ui::{prelude::metrics, widget::button_on_setting};

use super::*;
use crate::pointer_rig::{
    click_at, hear_ui_cues, pane_pointer_rig, resize_window, settle, take_churn, take_cues,
    track_node_churn, NodeChurn, PanePointerRig,
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
    app.init_resource::<BalanceCountUp>();
    app.init_resource::<crate::inventory::InventoryRuntime>();
    app.add_observer(button_on_setting::<InterfacePaneType>);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(PauseStates::Interface);
    app.add_systems(
        Update,
        (
            leave_station_pane,
            rebuild_interface_body,
            fit_pane_footer,
            fit_card_head,
            refresh_card_head_keys,
            show_station_tab,
            refresh_pane_input_hints,
            show_pane_input_hints,
        )
            .chain(),
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

    // A click on the Ship tab: the tick of the pointer arriving, one click cue,
    // the same card, a new body.
    let ship = tab_centre(rig.app.world_mut(), InterfacePaneType::Ship);
    click_at(&mut rig, ship);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Ship
    );
    assert_eq!(
        take_cues(&mut rig.app),
        [UiSfx::MenuFocus, UiSfx::MenuSelect]
    );
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
        // The split is the body child that grows to fill it; the Map's
        // notice row above it does not.
        let split = world
            .get::<Children>(body_id)
            .unwrap()
            .iter()
            .find(|child| {
                world
                    .get::<Node>(*child)
                    .is_some_and(|node| node.flex_grow > 0.0)
            })
            .unwrap_or_else(|| panic!("{pane:?} has a growing split"));
        let [view, panel] = world.get::<Children>(split).unwrap()[..] else {
            panic!("{pane:?} splits into a view and a panel");
        };
        let (view, panel) = (rect_of(world, view), rect_of(world, panel));
        assert!(
            view.max.y <= foot.min.y && panel.max.y <= foot.min.y,
            "{pane:?} view {view:?} and panel {panel:?} sit above the footer {foot:?}"
        );
        assert!(
            (panel.width() - metrics::SIDE_PANEL_W).abs() <= 0.5,
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

/// The rect of every footer hint that is shown.
fn shown_hint_rects(world: &mut World) -> Vec<Rect> {
    let shown: Vec<Entity> = world
        .query_filtered::<(Entity, &Node), Or<(With<PaneInputHint>, With<PanePointerHint>)>>()
        .iter(world)
        .filter(|(_, node)| node.display == Display::Flex)
        .map(|(entity, _)| entity)
        .collect();
    shown.into_iter().map(|hint| rect_of(world, hint)).collect()
}

/// The rect of the node named `name`.
fn rect_named(world: &mut World, name: &str) -> Rect {
    let entity = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a node named {name}"));
    rect_of(world, entity)
}

/// The footer zone of `kind`.
fn zone(world: &mut World, kind: PaneFooterZoneType) -> Entity {
    world
        .query::<(Entity, &PaneFooterZoneType)>()
        .iter(world)
        .find(|(_, z)| **z == kind)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a {kind:?} footer zone"))
}

/// The rect of every text label under `root`.
fn label_rects(world: &mut World, root: Entity) -> Vec<Rect> {
    let mut labels = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<Text>(entity).is_some() {
            labels.push(rect_of(world, entity));
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    labels
}

/// Whether `inner` lies in `outer`, to half a pixel.
fn inside(inner: Rect, outer: Rect) -> bool {
    inner.min.x >= outer.min.x - 0.5
        && inner.min.y >= outer.min.y - 0.5
        && inner.max.x <= outer.max.x + 0.5
        && inner.max.y <= outer.max.y + 0.5
}

/// Whether two rects share any area.
fn overlap(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 0.5 && shared.height() > 0.5
}

#[test]
fn a_narrow_window_puts_the_footer_hints_under_the_legend_and_nothing_overlaps() {
    let mut rig = card_rig();
    // Ship pane, 640x600, a part in hand at a refit dock with a repairable,
    // rebindable, deletable selection: the widest hint set the footer shows.
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
    rig.app
        .insert_resource(HeldPart(Some("pdc_pierce_turret_section".into())));
    rig.app.insert_resource(ShipRuntime {
        refits: true,
        panel_repairable: true,
        panel_rebind_enabled: true,
        delete_verdict: Some(Ok(std::collections::BTreeMap::new())),
        ..default()
    });
    resize_window(&mut rig.app, UVec2::new(640, 600));
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    let footer_id = footer(world).expect("Ship has a footer");
    let foot = rect_of(world, footer_id);
    assert_eq!(foot.height(), PANE_FOOTER_NARROW_PX, "narrow footer height");
    let legend_id = zone(world, PaneFooterZoneType::Legend);
    let legend = rect_of(world, legend_id);
    let hints = shown_hint_rects(world);
    let placing = SHIP_KEY_HINTS
        .iter()
        .filter(|(.., when)| *when != HintWhenType::Selecting)
        .count()
        + POINTER_HINTS
            .iter()
            .filter(|(_, when)| *when != HintWhenType::Selecting)
            .count();
    assert_eq!(
        hints.len(),
        placing,
        "every hint but the Selecting ones shows"
    );
    for hint in &hints {
        assert!(inside(*hint, foot), "hint {hint:?} in the footer {foot:?}");
        assert!(
            hint.min.y >= legend.max.y - 0.5,
            "hint {hint:?} under the legend {legend:?}"
        );
    }
    let labels = label_rects(world, legend_id);
    assert_eq!(
        labels.len(),
        SectionIconType::ALL.len(),
        "one word per legend icon"
    );
    for label in &labels {
        assert!(
            inside(*label, legend),
            "legend word {label:?} in its zone {legend:?}"
        );
    }
    let buttons: Vec<Rect> = ["ShipCatalogue", "ShipFit", "ShipReset"]
        .into_iter()
        .map(|name| rect_named(world, name))
        .collect();
    for button in &buttons {
        assert!(
            inside(*button, foot),
            "button {button:?} in the footer {foot:?}"
        );
    }
    let rects: Vec<Rect> = hints
        .iter()
        .chain(&labels)
        .chain(&buttons)
        .copied()
        .collect();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            assert!(!overlap(*a, *b), "{a:?} overlaps {b:?}");
        }
    }
    let body_id = card_ids(world).body;
    let split = world.get::<Children>(body_id).unwrap()[0];
    let [view, panel] = world.get::<Children>(split).unwrap()[..] else {
        panic!("the Ship pane splits into a view and a panel");
    };
    let (view, panel) = (rect_of(world, view), rect_of(world, panel));
    assert!(
        view.max.y <= foot.min.y + 0.5 && panel.max.y <= foot.min.y + 0.5,
        "view {view:?} and panel {panel:?} sit above the footer {foot:?}"
    );
    assert!(
        (panel.width() - metrics::SIDE_PANEL_W).abs() <= 0.5,
        "panel is {} wide",
        panel.width()
    );

    // Back to a wide window: one row, and the legend's longest word whole.
    resize_window(&mut rig.app, UVec2::new(1024, 768));
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    let foot = rect_of(world, footer_id);
    assert_eq!(foot.height(), PANE_FOOTER_PX, "wide footer height");
    let legend_id = zone(world, PaneFooterZoneType::Legend);
    let legend = rect_of(world, legend_id);
    let controller = world
        .query::<(Entity, &Text)>()
        .iter(world)
        .find(|(_, text)| text.0 == "Controller")
        .map(|(entity, _)| entity)
        .expect("the legend names the Controller");
    let controller = rect_of(world, controller);
    assert!(
        inside(controller, legend),
        "Controller {controller:?} whole in the legend {legend:?}"
    );
}

/// A click on the Ship pane's Catalogue button opens the parts gallery and
/// plays one interface click, as Fit and Reset do, after the pointer's tick.
#[test]
fn the_catalogue_button_opens_the_gallery_and_clicks_once() {
    #[derive(Resource, Default)]
    struct HeardPlaces(Vec<SoundAt>);

    let mut rig = card_rig();
    rig.app.add_plugins(PartsGalleryPlugin);
    rig.app.init_resource::<GameSections>();
    rig.app.init_resource::<HeardPlaces>();
    rig.app
        .add_observer(|sfx: On<PlaySound>, mut heard: ResMut<HeardPlaces>| {
            heard.0.push(sfx.at);
        });
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
    rig.app.insert_resource(ShipRuntime {
        refits: true,
        ..default()
    });
    settle(&mut rig.app);
    take_cues(&mut rig.app);
    rig.app.world_mut().resource_mut::<HeardPlaces>().0.clear();

    let catalogue = rect_named(rig.app.world_mut(), "ShipCatalogue").center();
    click_at(&mut rig, catalogue);

    assert!(
        rig.app.world().resource::<GalleryState>().open,
        "the Catalogue button opens the gallery"
    );
    assert_eq!(
        take_cues(&mut rig.app),
        [UiSfx::MenuFocus, UiSfx::MenuSelect]
    );
    assert_eq!(
        rig.app.world().resource::<HeardPlaces>().0,
        [SoundAt::Ui, SoundAt::Ui]
    );
}

/// Whether the Craft view's host node is in the tree.
fn craft_view_shown(world: &mut World) -> bool {
    world
        .query::<&Name>()
        .iter(world)
        .any(|name| name.as_str() == crate::craft::CRAFT_VIEW)
}

/// A click on the Inventory pane's Craft button opens the Craft view over the
/// pane body, and a tab switch closes it: the Inventory shows again with no
/// view over it.
#[test]
fn the_craft_button_opens_the_craft_view_and_a_tab_switch_closes_it() {
    let mut rig = card_rig();
    rig.app
        .init_resource::<crate::craft::CraftRuntime>()
        .add_systems(
            Update,
            crate::craft::manage_craft_view.after(rebuild_interface_body),
        );
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    settle(&mut rig.app);
    assert!(!craft_view_shown(rig.app.world_mut()));

    let craft = rect_named(rig.app.world_mut(), crate::craft::INVENTORY_CRAFT).center();
    click_at(&mut rig, craft);
    assert!(
        rig.app
            .world()
            .resource::<crate::craft::CraftRuntime>()
            .is_open(),
        "the Craft button opens the view"
    );
    settle(&mut rig.app);
    assert!(
        craft_view_shown(rig.app.world_mut()),
        "the view covers the Inventory pane body"
    );

    let jobs = tab_centre(rig.app.world_mut(), InterfacePaneType::Jobs);
    click_at(&mut rig, jobs);
    settle(&mut rig.app);
    assert!(
        !rig.app
            .world()
            .resource::<crate::craft::CraftRuntime>()
            .is_open(),
        "a tab switch closes the view"
    );
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    settle(&mut rig.app);
    assert!(
        !craft_view_shown(rig.app.world_mut()),
        "the Inventory comes back without the view"
    );
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

/// The text of the key chip inside the node named `name`.
fn chip_text(world: &mut World, name: &str) -> String {
    let root = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("a node named {name}"));
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if world.get::<KeyChipLabel>(entity).is_some() {
            return world
                .get::<Text>(entity)
                .expect("a chip label is text")
                .0
                .clone();
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    panic!("{name} holds a key chip");
}

#[test]
fn the_close_button_leaves_the_interface() {
    let mut rig = card_rig();
    let close = rect_named(rig.app.world_mut(), "InterfaceClose").center();
    click_at(&mut rig, close);
    assert_eq!(
        *rig.app.world().resource::<State<PauseStates>>().get(),
        PauseStates::Unpaused
    );
}

#[test]
fn the_card_head_names_the_live_close_key() {
    let mut rig = card_rig();
    assert_eq!(chip_text(rig.app.world_mut(), "InterfaceClose"), "Tab");
    assert_eq!(chip_text(rig.app.world_mut(), "InterfaceNextHint"), "M");
    rig.app.world_mut().resource_mut::<InputBindings>().rebind(
        "interface_toggle",
        BindingSpec {
            keyboard: vec![InputSource::Keyboard(KeyCode::KeyJ)],
            gamepad: Vec::new(),
        },
    );
    settle(&mut rig.app);
    assert_eq!(chip_text(rig.app.world_mut(), "InterfaceClose"), "J");
}

#[test]
fn the_card_head_keeps_every_control_inside_it_at_853_logical_px() {
    let mut rig = card_rig();
    resize_window(&mut rig.app, UVec2::new(853, 480));
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    let head = card_ids(world).head;
    let head = rect_of(world, head);
    for name in ["InterfaceClose", "InterfaceTabMap", "InterfaceTabStation"] {
        let rect = rect_named(world, name);
        if rect.width() > 0.0 {
            assert!(inside(rect, head), "{name} {rect:?} in the head {head:?}");
        }
    }
}

#[test]
fn a_larger_ui_scale_narrows_the_layout_width() {
    let window = Window {
        resolution: bevy::window::WindowResolution::new(1280, 720),
        ..default()
    };
    assert_eq!(ui_width(&window, &UiScale(1.0)), 1280.0);
    assert!((ui_width(&window, &UiScale(1.5)) - 853.333).abs() < 0.01);
}

/// Press and release `M` through the keyboard messages a window writes, one
/// frame each.
fn press_m(app: &mut App) {
    use bevy::input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    };

    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyM,
            logical_key: Key::Character("m".into()),
            state,
            text: (state == ButtonState::Pressed).then(|| "m".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    }
}

#[test]
fn a_focused_text_field_types_m_instead_of_switching_panes() {
    use bevy::{input::InputPlugin, state::app::StatesPlugin};
    use nova_gameplay::PauseStates;
    use nova_input::prelude::RegisterInputActions;
    use nova_ui::{
        prelude::{in_input_mode, InputMode, TextFieldFocused},
        NovaUiPlugin,
    };

    // nova_ui's real arbiter and the pane switch under the plugin's own gate.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, NovaUiPlugin));
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(InterfacePaneType::Inventory);
    app.add_systems(
        Update,
        next_interface_pane.run_if(in_input_mode(InputMode::Normal)),
    );
    let field = app.world_mut().spawn(TextFieldFocused::at_end("1")).id();
    app.update();

    press_m(&mut app);
    assert_eq!(
        *app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Inventory,
        "M typed into the focused quantity field must not switch panes"
    );

    app.world_mut()
        .entity_mut(field)
        .remove::<TextFieldFocused>();
    app.update();
    press_m(&mut app);
    assert_eq!(
        *app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Inventory.next(false),
        "with the field let go, M switches panes again"
    );
}

/// The Nova Debug window claims `InputMode::Insert` while its prompt has the
/// keyboard. Before the toggle took the plugin's Normal gate, TAB typed there
/// for completion also closed the Interface under the window.
#[test]
fn tab_typed_into_a_focused_field_does_not_close_the_interface() {
    use bevy::{
        input::{
            keyboard::{Key, KeyboardInput},
            ButtonState, InputPlugin,
        },
        state::app::StatesPlugin,
    };
    use nova_gameplay::{GameStates, PauseStates};
    use nova_ui::{
        prelude::{in_input_mode, ClaimKeyboard, InputMode, InputModeSystems},
        NovaUiPlugin,
    };

    /// Whether a field holds the keyboard, as its owner claims each frame.
    #[derive(Resource)]
    struct FieldHasKeyboard(bool);

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, NovaUiPlugin));
    app.insert_state(GameStates::Playing);
    app.insert_state(PauseStates::Interface);
    app.register_input_actions(crate::bindings::interface_bindings());
    app.insert_resource(FieldHasKeyboard(true));
    app.add_systems(
        PreUpdate,
        (|field: Res<FieldHasKeyboard>, mut claims: MessageWriter<ClaimKeyboard>| {
            if field.0 {
                claims.write(ClaimKeyboard(InputMode::Insert));
            }
        })
        .before(InputModeSystems),
    );
    app.add_systems(
        Update,
        toggle_interface
            .run_if(in_state(GameStates::Playing))
            .run_if(in_input_mode(InputMode::Normal)),
    );
    app.update();

    let press_tab = |app: &mut App| {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::Tab,
                logical_key: Key::Tab,
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            app.update();
        }
    };
    press_tab(&mut app);
    assert_eq!(
        *app.world().resource::<State<PauseStates>>().get(),
        PauseStates::Interface,
        "TAB into the focused field must not close the Interface"
    );

    app.insert_resource(FieldHasKeyboard(false));
    app.update();
    press_tab(&mut app);
    assert_eq!(
        *app.world().resource::<State<PauseStates>>().get(),
        PauseStates::Unpaused,
        "with the field let go, TAB closes the Interface again"
    );
}

/// The Ship pane's side panel scrolled so its nested rows run up past the
/// panel's top edge, under the tabs. bevy_ui 0.19.0 clips a node only by its
/// direct parent, so a row nested in a non-clipping wrapper stays pickable
/// where the panel hides it. Before the head took a `ZIndex`, a tab click
/// there landed on the hidden row and the pane did not switch, the bug the
/// owner saw after a placement until the HUD was reopened.
#[test]
fn a_tab_click_lands_while_the_ship_panel_is_scrolled_under_the_head() {
    let mut rig = card_rig();
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Ship;
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    let panel = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == "ShipPanel")
        .map(|(entity, _)| entity)
        .expect("the Ship pane has its side panel");
    let row = world
        .spawn((
            Node {
                flex_shrink: 0.0,
                width: px(4000),
                margin: UiRect::left(px(-2000)),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    let hidden = world
        .spawn((
            Node {
                width: percent(100),
                height: px(4000),
                ..default()
            },
            bevy::ui_widgets::Button,
            ChildOf(row),
        ))
        .id();
    world
        .entity_mut(panel)
        .insert(ScrollPosition(Vec2::new(0.0, 1500.0)));
    settle(&mut rig.app);

    let world = rig.app.world_mut();
    let map = tab_centre(world, InterfacePaneType::Map);
    let shown_panel = rect_of(world, panel);
    let under = rect_of(world, hidden);
    assert!(
        map.y < shown_panel.min.y && under.contains(map),
        "the hidden row {under:?} must lie under the Map tab {map:?}, above the panel {shown_panel:?}"
    );

    click_at(&mut rig, map);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Map,
        "a click on the Map tab must switch panes over the hidden row"
    );
}

/// The Station tab's display, and whether a click there would land.
fn station_tab_display(world: &mut World) -> Display {
    world
        .query::<(&ButtonValue<InterfacePaneType>, &Node)>()
        .iter(world)
        .find(|(value, _)| value.0 == InterfacePaneType::Station)
        .map(|(_, node)| node.display)
        .expect("the card has a Station tab")
}

/// Dock a player ship with a new partner, a station when `station`, and
/// return the partner.
fn dock_the_player(world: &mut World, station: bool) -> Entity {
    use nova_gameplay::prelude::{SpaceshipRootMarker, StationMarker};
    use nova_ship::prelude::{DockedHelmType, DockedShip, DockingConnection};

    let player = world
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
        .id();
    let partner = world.spawn(SpaceshipRootMarker).id();
    if station {
        world.entity_mut(partner).insert(StationMarker);
    }
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
    world.entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    partner
}

#[test]
fn the_station_tab_shows_only_while_docked_at_a_living_station() {
    use nova_gameplay::prelude::{DerelictShipMarker, StationMarker};

    let mut rig = card_rig();
    assert_eq!(station_tab_display(rig.app.world_mut()), Display::None);

    // Not docked: the Station pane falls back to Ship before it is built.
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Station;
    settle(&mut rig.app);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Ship
    );

    // Docked with a ship, not a station: still hidden.
    let ship = dock_the_player(rig.app.world_mut(), false);
    settle(&mut rig.app);
    assert_eq!(station_tab_display(rig.app.world_mut()), Display::None);
    rig.app.world_mut().entity_mut(ship).insert(StationMarker);

    // Docked with a living station: the tab shows and a click opens it.
    settle(&mut rig.app);
    assert_eq!(station_tab_display(rig.app.world_mut()), Display::Flex);
    let station = tab_centre(rig.app.world_mut(), InterfacePaneType::Station);
    click_at(&mut rig, station);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Station
    );
    assert_eq!(
        shown(rig.app.world_mut()),
        ("STATION".to_string(), vec![InterfacePaneType::Station])
    );

    // The station dies: the tab hides and the pane falls back to Ship.
    rig.app
        .world_mut()
        .entity_mut(ship)
        .insert(DerelictShipMarker);
    settle(&mut rig.app);
    assert_eq!(station_tab_display(rig.app.world_mut()), Display::None);
    assert_eq!(
        *rig.app.world().resource::<InterfacePaneType>(),
        InterfacePaneType::Ship
    );
}

#[test]
fn the_next_tab_key_steps_through_jobs_always_and_station_only_while_docked() {
    assert_eq!(
        InterfacePaneType::Inventory.next(false),
        InterfacePaneType::Jobs
    );
    assert_eq!(InterfacePaneType::Jobs.next(false), InterfacePaneType::Map);
    assert_eq!(
        InterfacePaneType::Jobs.next(true),
        InterfacePaneType::Station
    );
    assert_eq!(
        InterfacePaneType::Station.next(true),
        InterfacePaneType::Map
    );
}

#[test]
fn the_title_row_clock_reads_the_play_time_only_while_a_world_is_armed() {
    use bevy::ecs::system::RunSystemOnce;
    use nova_events::prelude::SectorCoord;
    use nova_gameplay::prelude::{Chart, ClusterType, WorldClock};

    let mut rig = card_rig();
    // The clock's slot stands always, so the head never shifts; off, it
    // reads nothing.
    let clock_of = |world: &mut World| {
        world
            .query_filtered::<&Text, With<InterfacePaneClock>>()
            .single(world)
            .map(|text| text.0.clone())
            .expect("the title row has one clock")
    };

    // No clock and no chart: empty.
    rig.app
        .world_mut()
        .run_system_once(show_pane_clock)
        .expect("the clock system runs");
    assert_eq!(clock_of(rig.app.world_mut()), "");

    // A clock on an unarmed world (empty chart) stays empty.
    rig.app.insert_resource(WorldClock {
        play_seconds: 3_661.0,
    });
    rig.app.init_resource::<Chart>();
    rig.app
        .world_mut()
        .run_system_once(show_pane_clock)
        .expect("the clock system runs");
    assert_eq!(clock_of(rig.app.world_mut()), "");

    // The start cell on the chart arms it: the clock shows the play time.
    rig.app.world_mut().resource_mut::<Chart>().visit(
        SectorCoord::ORIGIN,
        vec![ClusterType::AsteroidRich],
        0.0,
    );
    rig.app
        .world_mut()
        .run_system_once(show_pane_clock)
        .expect("the clock system runs");
    assert_eq!(clock_of(rig.app.world_mut()), "1:01:01");
}

#[test]
fn the_title_row_shows_the_player_balance_grouped_and_hides_it_without_a_player_ship() {
    use bevy::ecs::system::RunSystemOnce;

    let mut rig = card_rig();
    let balance_of = |world: &mut World| {
        let (text, row) = world
            .query_filtered::<(&Text, &ChildOf), With<InterfacePaneBalance>>()
            .single(world)
            .map(|(text, child_of)| (text.0.clone(), child_of.parent()))
            .expect("the title row has one balance");
        let shown = world
            .get::<Node>(row)
            .expect("the balance digits sit in a figure row")
            .display
            != Display::None;
        (text, shown)
    };
    let run = |app: &mut App| {
        app.world_mut()
            .run_system_once(show_pane_balance)
            .expect("the balance system runs");
    };

    // No player ship: hidden.
    run(&mut rig.app);
    assert!(!balance_of(rig.app.world_mut()).1);

    // A player ship: its balance shows, grouped by thousands.
    let player = rig
        .app
        .world_mut()
        .spawn((PlayerSpaceshipMarker, ShipCredits(22_980)))
        .id();
    run(&mut rig.app);
    assert_eq!(
        balance_of(rig.app.world_mut()),
        ("22,980".to_string(), true)
    );

    // The player ship gone: hidden again.
    rig.app.world_mut().despawn(player);
    run(&mut rig.app);
    assert!(!balance_of(rig.app.world_mut()).1);
}

/// The head's slots are fixed: a balance that grows from one digit to the
/// widest figure moves no tab, the mode tags or Close, and every head control
/// is one control high.
#[test]
fn the_tabs_hold_their_place_as_the_balance_grows_and_the_pane_switches() {
    let mut rig = card_rig();
    rig.app.add_systems(Update, show_pane_balance);
    let player = rig
        .app
        .world_mut()
        .spawn((PlayerSpaceshipMarker, ShipCredits(5)))
        .id();
    settle(&mut rig.app);

    let head_layout = |world: &mut World| {
        let tabs: Vec<Vec2> = InterfacePaneType::ALL
            .iter()
            .filter(|(pane, _)| *pane != InterfacePaneType::Station)
            .map(|(pane, _)| tab_centre(world, *pane))
            .collect();
        (
            tabs,
            rect_named(world, INTERFACE_MODE_TAGS),
            rect_named(world, "InterfaceClose"),
        )
    };
    let before = head_layout(rig.app.world_mut());

    rig.app
        .world_mut()
        .get_mut::<ShipCredits>(player)
        .unwrap()
        .0 = 999_999_999;
    settle(&mut rig.app);
    let after = head_layout(rig.app.world_mut());
    assert_eq!(before, after, "a growing balance moves nothing in the head");

    // INVENTORY is the longest pane title and MAP the shortest.
    *rig.app.world_mut().resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    settle(&mut rig.app);
    let switched = head_layout(rig.app.world_mut());
    assert_eq!(before, switched, "a pane switch moves nothing in the head");

    let close = after.2;
    assert!(
        (close.height() - metrics::CONTROL_H).abs() < 0.5,
        "Close is {} high, not {}",
        close.height(),
        metrics::CONTROL_H
    );
    let world = rig.app.world_mut();
    let tab = world
        .query::<(&ButtonValue<InterfacePaneType>, &ComputedNode)>()
        .iter(world)
        .find(|(value, _)| value.0 == InterfacePaneType::Map)
        .map(|(_, node)| node.size().y * node.inverse_scale_factor())
        .expect("the Map tab is laid out");
    assert!(
        (tab - metrics::CONTROL_H).abs() < 0.5,
        "a tab is {tab} high, not {}",
        metrics::CONTROL_H
    );
}

/// Docked at a station that offers every service, with the widest balance
/// and a gain beside it, the head fits a 1024 px and a 1280 px window: Close
/// and every tab lie whole inside it, the mode tags give way before the
/// balance, the gain fits its slot and the station's name still shows.
#[test]
fn close_and_the_tabs_fit_the_head_from_1024_px_while_docked() {
    use nova_gameplay::prelude::{StationServiceType, StationServices};

    let mut rig = card_rig();
    rig.app
        .add_systems(Update, (show_pane_balance, show_pane_mode_tags));
    let station = dock_the_player(rig.app.world_mut(), true);
    rig.app.world_mut().entity_mut(station).insert((
        Name::new("Kestrel Exchange"),
        StationServices(vec![
            StationServiceType::Refit,
            StationServiceType::Craft,
            StationServiceType::Trade,
            StationServiceType::Jobs,
        ]),
    ));
    let world = rig.app.world_mut();
    let player = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(world)
        .expect("one player ship");
    world.entity_mut(player).insert(ShipCredits(999_900_000));
    settle(&mut rig.app);
    // The widest balance, and the widest gain that fits the slot in this
    // rig's fallback font, whose digits are a fifth wider than the UI font's.
    pay(rig.app.world_mut(), 99_999);

    for width in [1024, 1280] {
        resize_window(&mut rig.app, UVec2::new(width, 768));
        settle(&mut rig.app);
        let world = rig.app.world_mut();
        assert_eq!(gain_of(world), "+99,999", "the gain shows at {width} px");
        let head = card_ids(world).head;
        let head = rect_of(world, head);
        for name in [
            "InterfaceClose",
            "InterfaceTabMap",
            "InterfaceTabShip",
            "InterfaceTabInventory",
            "InterfaceTabJobs",
            "InterfaceTabStation",
        ] {
            let rect = rect_named(world, name);
            assert!(rect.width() > 0.0, "{name} is shown at {width} px");
            assert!(
                inside(rect, head),
                "{name} {rect:?} in the head {head:?} at {width} px"
            );
        }
        let tags = rect_named(world, INTERFACE_MODE_TAGS);
        let balance = rect_named(world, INTERFACE_PANE_BALANCE);
        assert!(
            tags.max.x <= balance.min.x + 0.5,
            "the mode tags {tags:?} stop left of the balance {balance:?} at {width} px"
        );
        let gain = rect_named(world, INTERFACE_PANE_GAIN);
        let close = rect_named(world, "InterfaceClose");
        assert!(
            gain.width() > 0.0 && balance.max.x <= gain.min.x + 0.5 && gain.max.x <= close.min.x,
            "the gain {gain:?} stands between the balance {balance:?} and Close {close:?} at \
             {width} px"
        );
        assert!(
            gain.width() <= GAIN_SLOT_PX + 0.5,
            "the gain {gain:?} fits its {GAIN_SLOT_PX} px slot at {width} px"
        );
        let name = world
            .query::<(Entity, &Text)>()
            .iter(world)
            .find(|(_, text)| text.0 == "KESTREL EXCHANGE")
            .map(|(entity, _)| entity)
            .expect("the head names the station");
        let name = rect_of(world, name);
        assert!(
            inside(name, tags),
            "the station name {name:?} shows whole in the tags {tags:?} at {width} px"
        );
    }
}

/// Pay the one player ship `gain_cr` and raise the gain on the title row, as
/// a job hand-in does.
fn pay(world: &mut World, gain_cr: u32) {
    let mut credits = world
        .query_filtered::<&mut ShipCredits, With<PlayerSpaceshipMarker>>()
        .single_mut(world)
        .expect("one player ship");
    credits.0 += gain_cr;
    world.resource_mut::<BalanceCountUp>().raise();
}

/// The title row's balance text.
fn balance_of(world: &mut World) -> String {
    world
        .query_filtered::<&Text, With<InterfacePaneBalance>>()
        .single(world)
        .map(|text| text.0.clone())
        .expect("the title row has one balance")
}

/// The title row's gain text.
fn gain_of(world: &mut World) -> String {
    world
        .query_filtered::<&Text, With<InterfacePaneGain>>()
        .single(world)
        .map(|text| text.0.clone())
        .expect("the title row has one gain slot")
}

/// One frame of the count, in seconds: 60 a second.
const COUNT_FRAME_SECS: f32 = 1.0 / 60.0;

/// A raised gain counts the balance from the figure shown to the true
/// balance in [`BALANCE_COUNT_SECS`], never down and never past it, with the
/// gain beside it until [`BALANCE_GAIN_SECS`]; then the gain goes and the
/// true balance stays.
#[test]
fn a_count_up_ends_on_the_true_balance_and_never_passes_it() {
    let mut count_up = BalanceCountUp::default();
    let quiet = BalanceFigures {
        balance_cr: 2_000,
        gain_cr: None,
    };
    assert_eq!(count_up.step(2_000, COUNT_FRAME_SECS), quiet);

    count_up.raise();
    let start = count_up.step(3_460, COUNT_FRAME_SECS);
    assert_eq!(
        start,
        BalanceFigures {
            balance_cr: 2_000,
            gain_cr: Some(1_460),
        },
        "the count starts on the figure shown"
    );
    assert!(count_up.is_counting());

    let mut last = start.balance_cr;
    let mut age = 0.0;
    let mut landed_at = None;
    let mut gain_gone_at = None;
    while gain_gone_at.is_none() {
        age += COUNT_FRAME_SECS;
        let figures = count_up.step(3_460, COUNT_FRAME_SECS);
        assert!(
            (last..=3_460).contains(&figures.balance_cr),
            "the figure {} at {age:.2} s rises from {last} and stays at or under 3,460",
            figures.balance_cr
        );
        last = figures.balance_cr;
        if figures.balance_cr == 3_460 && landed_at.is_none() {
            landed_at = Some(age);
            assert!(!count_up.is_counting());
        }
        if figures.gain_cr.is_none() {
            gain_gone_at = Some(age);
        }
        assert!(age < 10.0, "the count ends");
    }
    let landed_at = landed_at.expect("the count lands on the true balance");
    assert!(
        (landed_at - BALANCE_COUNT_SECS).abs() < 2.0 * COUNT_FRAME_SECS,
        "the count lands at {landed_at:.2} s"
    );
    let gain_gone_at = gain_gone_at.expect("the gain goes");
    assert!(
        (gain_gone_at - BALANCE_GAIN_SECS).abs() < 2.0 * COUNT_FRAME_SECS,
        "the gain goes at {gain_gone_at:.2} s"
    );
    assert_eq!(
        count_up.step(3_460, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: 3_460,
            gain_cr: None,
        }
    );
}

/// A frame longer than the whole count steps it by
/// [`BALANCE_COUNT_STEP_MAX_SECS`] only, so the count still shows.
#[test]
fn a_long_frame_does_not_skip_the_count() {
    let mut count_up = BalanceCountUp::default();
    count_up.step(2_000, COUNT_FRAME_SECS);
    count_up.raise();
    count_up.step(3_460, 5.0);
    let figures = count_up.step(3_460, 5.0);
    assert!(
        figures.balance_cr > 2_000 && figures.balance_cr < 3_460,
        "one long frame leaves the figure {} between the two balances",
        figures.balance_cr
    );
    assert_eq!(figures.gain_cr, Some(1_460));
}

/// A balance change with no gain raised, such as a purchase or a fee, ends a
/// running count on that frame: the head shows the true balance and no gain.
#[test]
fn a_credit_change_that_is_no_job_reward_ends_a_running_count() {
    let mut count_up = BalanceCountUp::default();
    count_up.step(2_000, COUNT_FRAME_SECS);
    count_up.raise();
    count_up.step(3_460, COUNT_FRAME_SECS);
    let counting = count_up.step(3_460, COUNT_FRAME_SECS);
    assert!(counting.balance_cr < 3_460 && counting.gain_cr == Some(1_460));

    // A 100 cr fee.
    assert_eq!(
        count_up.step(3_360, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: 3_360,
            gain_cr: None,
        }
    );
    assert!(!count_up.is_counting());

    // Through the gain's dwell too: the count has landed, the gain still shows.
    count_up.raise();
    count_up.step(3_400, COUNT_FRAME_SECS);
    for _ in 0..60 {
        count_up.step(3_400, COUNT_FRAME_SECS);
    }
    assert_eq!(count_up.gain_cr(), Some(40));
    assert_eq!(
        count_up.step(3_300, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: 3_300,
            gain_cr: None,
        }
    );

    // A gain spent in the frame that paid it has nothing to count up to.
    count_up.raise();
    assert_eq!(
        count_up.step(3_250, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: 3_250,
            gain_cr: None,
        }
    );
}

/// A credit grant that lands in the frame of a job reward is part of the
/// gain shown: the figure beside the balance is what the head counts up by.
#[test]
fn a_credit_grant_in_the_frame_of_a_job_reward_is_part_of_the_shown_gain() {
    let mut count_up = BalanceCountUp::default();
    count_up.step(2_000, COUNT_FRAME_SECS);

    // A 1,460 cr reward and a 50 cr scenario bonus reach the player before
    // the head next reads the balance.
    count_up.raise();
    assert_eq!(
        count_up.step(3_510, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: 2_000,
            gain_cr: Some(1_510),
        }
    );
    let mut figures = count_up.step(3_510, COUNT_FRAME_SECS);
    while count_up.is_counting() {
        assert_eq!(figures.gain_cr, Some(1_510));
        figures = count_up.step(3_510, COUNT_FRAME_SECS);
    }
    assert_eq!(
        figures,
        BalanceFigures {
            balance_cr: 3_510,
            gain_cr: Some(1_510),
        },
        "the head lands on the start figure plus the gain"
    );
}

/// A second gain raised during a count starts a new count from the figure
/// shown, to the new true balance; the gain shown is all the head still has
/// to rise by.
#[test]
fn a_second_gain_during_a_count_restarts_from_the_figure_shown() {
    let mut count_up = BalanceCountUp::default();
    count_up.step(2_000, COUNT_FRAME_SECS);
    count_up.raise();
    count_up.step(3_000, COUNT_FRAME_SECS);
    let mut shown = 2_000;
    for _ in 0..10 {
        shown = count_up.step(3_000, COUNT_FRAME_SECS).balance_cr;
    }
    assert!(shown > 2_000 && shown < 3_000);

    count_up.raise();
    assert_eq!(
        count_up.step(3_500, COUNT_FRAME_SECS),
        BalanceFigures {
            balance_cr: shown,
            gain_cr: Some(3_500 - shown),
        }
    );
    let mut last = shown;
    for _ in 0..60 {
        let figures = count_up.step(3_500, COUNT_FRAME_SECS);
        assert!((last..=3_500).contains(&figures.balance_cr));
        last = figures.balance_cr;
    }
    assert_eq!(last, 3_500);
}

/// On the live card a raised gain writes `+N` into its slot and counts the
/// balance up, and nothing in the head moves when the gain shows or goes.
/// The count ends with the title row: a card opened again shows the true
/// balance and no gain, and a gain raised with the card shut shows nothing.
#[test]
fn a_gain_shows_beside_the_balance_moves_nothing_and_ends_with_the_title_row() {
    use bevy::time::TimeUpdateStrategy;

    let mut rig = card_rig();
    rig.app.insert_resource(TimeUpdateStrategy::ManualDuration(
        core::time::Duration::from_secs_f32(COUNT_FRAME_SECS),
    ));
    rig.app.add_systems(Update, show_pane_balance);
    rig.app
        .world_mut()
        .spawn((PlayerSpaceshipMarker, ShipCredits(2_000)));
    settle(&mut rig.app);
    assert_eq!(balance_of(rig.app.world_mut()), "2,000");
    assert_eq!(gain_of(rig.app.world_mut()), "");

    let head_layout = |world: &mut World| {
        let tabs: Vec<Vec2> = InterfacePaneType::ALL
            .iter()
            .filter(|(pane, _)| *pane != InterfacePaneType::Station)
            .map(|(pane, _)| tab_centre(world, *pane))
            .collect();
        (
            tabs,
            rect_named(world, INTERFACE_MODE_TAGS),
            rect_named(world, INTERFACE_PANE_CLOCK),
            rect_named(world, "InterfaceClose"),
        )
    };
    let before = head_layout(rig.app.world_mut());
    let balance_right = rect_named(rig.app.world_mut(), INTERFACE_PANE_BALANCE)
        .max
        .x;

    pay(rig.app.world_mut(), 1_460);
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    assert_eq!(gain_of(world), "+1,460");
    let counting: u64 = balance_of(world).replace(',', "").parse().unwrap();
    assert!(
        counting > 2_000 && counting < 3_460,
        "the balance reads {counting} during the count"
    );
    assert_eq!(head_layout(world), before, "a shown gain moves nothing");
    let balance = rect_named(world, INTERFACE_PANE_BALANCE);
    assert!(
        (balance.max.x - balance_right).abs() < 0.5,
        "the balance keeps its right edge"
    );
    let gain = rect_named(world, INTERFACE_PANE_GAIN);
    let clock = world
        .query_filtered::<&ChildOf, With<InterfacePaneClock>>()
        .single(world)
        .map(ChildOf::parent)
        .expect("the clock sits in its slot");
    let clock = rect_of(world, clock);
    assert!(
        gain.width() > 0.0 && gain.min.x >= balance.max.x && gain.max.x <= clock.min.x,
        "the gain {gain:?} stands between the balance {balance:?} and the clock slot {clock:?}"
    );

    // Through the count and the dwell.
    for _ in 0..(BALANCE_GAIN_SECS / COUNT_FRAME_SECS) as usize + 2 {
        rig.app.update();
    }
    let world = rig.app.world_mut();
    assert_eq!(balance_of(world), "3,460");
    assert_eq!(gain_of(world), "");
    assert_eq!(head_layout(world), before, "a gain that went moves nothing");

    // The card shut during a count, then open again.
    pay(rig.app.world_mut(), 540);
    settle(&mut rig.app);
    assert_eq!(gain_of(rig.app.world_mut()), "+540");
    let world = rig.app.world_mut();
    let root = world
        .query_filtered::<Entity, With<InterfaceRootMarker>>()
        .single(world)
        .expect("one interface root");
    world.despawn(root);
    rig.app.update();
    assert_eq!(
        *rig.app.world().resource::<BalanceCountUp>(),
        BalanceCountUp::default(),
        "the count ends with the title row"
    );
    // Paid with the card shut.
    pay(rig.app.world_mut(), 1_000);
    rig.app.update();
    rig.app.world_mut().spawn((
        InterfaceRootMarker,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    settle(&mut rig.app);
    let world = rig.app.world_mut();
    assert_eq!(balance_of(world), "5,000");
    assert_eq!(gain_of(world), "");
}
