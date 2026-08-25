//! The top bar's menu bar: File, Edit, View and Add, each a button that drops
//! a list of items under it.
//!
//! Editor-specific rather than a `nova_ui` widget, because a menu is the one
//! piece of chrome the rest of the game has no use for - the menu screen and
//! the HUD are both full-screen compositions, and neither wants a dropdown.
//!
//! WHAT AN ITEM DOES IS THE ITEM'S OWN BUSINESS. Each row is spawned with its
//! own observer, exactly the way the rail's rows are, so there is no central
//! action enum to keep in step with the menus - a new item is one `spawn` with
//! one `observe`. Items that are not built yet carry `InteractionDisabled` and
//! no observer: greyed says "this belongs here and is coming", absent says
//! nothing at all.

use bevy::{
    ecs::{hierarchy::ChildOf, relationship::RelatedSpawner, spawn::SpawnWith},
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::GameStates;
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{key_chip, list_row_colors, ListRow},
};

use crate::config::{EditorOverlays, SelectedNode};

/// Which menu a bar button opens and a dropdown belongs to.
///
/// A component on both halves, so the sync is one comparison rather than an
/// entity handle that has to be threaded from the bar to the panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuId {
    /// The document itself: new, save, load, leave.
    File,
    /// What can be done to the selection.
    Edit,
    /// What the stage draws.
    View,
    /// Everything that puts a new node in the document.
    Add,
    /// The verbs of the ship you are inside: arm a part, arm the delete tool,
    /// rebind a key. Greyed out at the scenario node, where there is no ship
    /// for any of them to act on.
    Ship,
}

impl MenuId {
    /// The bar button's label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            MenuId::File => "File",
            MenuId::Edit => "Edit",
            MenuId::View => "View",
            MenuId::Add => "Add",
            MenuId::Ship => "Ship",
        }
    }
}

/// The menu that is open, or `None`.
///
/// One at a time: a bar where two lists hang open at once has no way to say
/// which one the next click belongs to.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
pub(crate) struct OpenMenu(pub(crate) Option<MenuId>);

/// One dropped list, hanging under its bar button.
#[derive(Component)]
pub(crate) struct MenuDropdown;

/// One row in a dropdown. The marker is what closes the menu after a press -
/// the row's own observer does the work and knows nothing about the menu it
/// was in.
#[derive(Component)]
pub(crate) struct MenuItem;

/// The full-screen catcher behind an open menu.
///
/// A click anywhere off the menu closes it, and this is what "anywhere" is: an
/// invisible node that blocks the pointer, so the click that dismisses the
/// menu does not also land on the stage behind it and move something.
#[derive(Component)]
pub(crate) struct MenuScrim;

/// A dropdown draws over the stage and over the rail; the scrim sits under the
/// dropdowns and over everything else. Explicit because UI stacking follows the
/// tree, and the menu bar is one of the first things in it.
const SCRIM_Z: i32 = 20;
/// The dropdowns' layer, above [`SCRIM_Z`].
const MENU_Z: i32 = 21;

/// One bar button. Wrapped in a slot by the caller for the same reason Play is:
/// `themed_button` is `percent(100)` wide.
pub(crate) fn menu_bar_slot() -> Node {
    Node {
        // The dropdown hangs off this, so it has to be the positioned ancestor.
        position_type: PositionType::Relative,
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// The panel a menu drops: absolutely positioned under its button, hidden
/// until [`sync_menus`] shows it.
pub(crate) fn menu_dropdown_node() -> Node {
    Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        top: percent(100),
        left: px(0),
        min_width: px(180),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        padding: UiRect::all(px(4)),
        border: UiRect::all(px(theme::BORDER_W)),
        ..default()
    }
}

/// What a row's right-hand column carries.
///
/// One column, three things it could be, and until now no way to tell them
/// apart: a key looked exactly like a toggle's state. A KEY is drawn as the
/// chip every other surface in the game draws a key as; a WORD is the row's
/// own state, in the muted tone a label wears.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum MenuTail<'a> {
    /// Nothing to say.
    #[default]
    None,
    /// The key that runs this row.
    Key(&'a str),
    /// A word about the row itself - a toggle's `on`/`off`, an unbuilt row's
    /// `soon`.
    Word(&'a str),
}

/// The chip half of a row's tail, so the paint pass can find it.
#[derive(Component)]
pub(crate) struct MenuKeyChip;

/// The font size a row's tail is drawn at.
const TAIL_FONT: f32 = 11.0;

/// One item row: the same `ListRow` shape the rail's rows wear, so the shared
/// reconciler paints the hover and this module owns no colour.
///
/// Every row has exactly two children - the label and the tail - so
/// [`sync_menu_item_paint`] and [`sync_view_menu_marks`] can reach either by
/// position rather than by search.
pub(crate) fn menu_item_row(label: &str, tail: MenuTail, skin: UiSkin) -> impl Bundle {
    let (background, border) = list_row_colors(false, false, skin);
    (
        ListRow,
        MenuItem,
        Button,
        Hovered::default(),
        Node {
            width: percent(100),
            min_height: px(24),
            margin: UiRect::bottom(px(2)),
            padding: UiRect::axes(px(10), px(3)),
            border: UiRect::all(px(theme::BORDER_W)),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            column_gap: px(16),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(background),
        children![
            (
                Text::new(label.to_string()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(ITEM_LABEL),
            ),
            menu_item_tail(tail),
        ],
    )
}

/// The row's right-hand column.
///
/// Always spawned, even when there is nothing to say: the paint pass and the
/// toggle marks both reach the tail as the row's second child, and a row that
/// sometimes has one child would make that a search.
fn menu_item_tail(tail: MenuTail) -> impl Bundle {
    let tail = tail.owned();
    (
        Node::default(),
        Children::spawn(SpawnWith(
            move |parent: &mut RelatedSpawner<ChildOf>| match &tail {
                OwnedTail::Key(key) => {
                    parent.spawn((MenuKeyChip, key_chip(key, TAIL_FONT + 2.0)));
                }
                OwnedTail::Word(word) => {
                    parent.spawn((
                        Text::new(word.clone()),
                        TextFont {
                            font_size: FontSize::Px(TAIL_FONT),
                            ..default()
                        },
                        TextColor(ITEM_MARK),
                    ));
                }
            },
        )),
    )
}

/// [`MenuTail`] with the string it borrowed, so the spawn closure can own it.
enum OwnedTail {
    Key(String),
    Word(String),
}

impl MenuTail<'_> {
    fn owned(self) -> OwnedTail {
        match self {
            MenuTail::Key(key) => OwnedTail::Key(key.to_string()),
            MenuTail::Word(word) => OwnedTail::Word(word.to_string()),
            MenuTail::None => OwnedTail::Word(String::new()),
        }
    }
}

/// Press a bar button: open its menu, or close it if it was already the open
/// one. The second press closing is what makes the button a toggle rather than
/// a trap.
pub(crate) fn on_menu_button(
    activate: On<Activate>,
    menus: Query<&MenuId>,
    mut open: ResMut<OpenMenu>,
) {
    let Ok(menu) = menus.get(activate.entity) else {
        return;
    };
    open.0 = if open.0 == Some(*menu) {
        None
    } else {
        Some(*menu)
    };
}

/// Any menu item press closes the menu it was in.
///
/// A global observer beside the item's OWN observer rather than something every
/// item has to remember to do: an item that did the work and left its menu
/// hanging open over the thing it just changed is the bug this prevents, and it
/// would be one forgotten line away every time.
pub(crate) fn close_menu_on_item(
    activate: On<Activate>,
    items: Query<(), With<MenuItem>>,
    mut open: ResMut<OpenMenu>,
) {
    if items.contains(activate.entity) && open.0.is_some() {
        open.0 = None;
    }
}

/// A click on the scrim - anywhere off the menu - closes it.
pub(crate) fn on_menu_scrim(_activate: On<Activate>, mut open: ResMut<OpenMenu>) {
    open.0 = None;
}

/// Show the open menu's dropdown and the scrim under it, hide the rest.
pub(crate) fn sync_menus(
    open: Res<OpenMenu>,
    mut dropdowns: Query<(&MenuId, &mut Node), With<MenuDropdown>>,
    mut scrims: Query<&mut Node, (With<MenuScrim>, Without<MenuDropdown>)>,
) {
    for (menu, mut node) in &mut dropdowns {
        let display = if open.0 == Some(*menu) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let display = if open.0.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut scrims {
        if node.display != display {
            node.display = display;
        }
    }
}

/// Close the open menu, and say whether there was one.
///
/// A helper rather than a system because it is the TOP RUNG of the editor's
/// Escape ladder (`crate::escape_backs_out`), and that ladder is one press one
/// rung - a separate system would have to agree with the rest of it about
/// whether the press was already spent. A menu is the frontmost thing on screen
/// while it is open, so it is what Escape is aimed at first.
pub(crate) fn close_open_menu(open: &mut OpenMenu) -> bool {
    if open.0.is_none() {
        return false;
    }
    open.0 = None;
    true
}

/// Leaving the editor takes the open menu with it: the bar it hangs off is
/// `DespawnOnExit(Editor)`, and a menu still "open" on the way back in would
/// show a dropdown with nothing under it.
pub(crate) fn close_menus(mut open: ResMut<OpenMenu>) {
    open.0 = None;
}

/// The scrim, spawned once beside the editor's chrome.
pub(crate) fn menu_scrim() -> impl Bundle {
    (
        Name::new("Menu Scrim"),
        MenuScrim,
        Button,
        Hovered::default(),
        GlobalZIndex(SCRIM_Z),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
    )
}

/// The z-layer a dropdown draws on.
pub(crate) fn menu_z() -> GlobalZIndex {
    GlobalZIndex(MENU_Z)
}

/// View > Key Legend: show or hide the footer's key line.
pub(crate) fn toggle_key_legend(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.key_legend = !overlays.key_legend;
}

/// View > Link Points: show or hide the socket gizmos a ship draws while a
/// part is armed.
pub(crate) fn toggle_link_points(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.link_points = !overlays.link_points;
}

/// View > World Grid: show or hide the ground plane the range is laid out on.
pub(crate) fn toggle_world_grid(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.world_grid = !overlays.world_grid;
}

/// View > Object Volumes: show or hide the trigger spheres, lamp ranges and sun
/// directions the objects on the stage have no body to show.
pub(crate) fn toggle_object_volumes(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.object_volumes = !overlays.object_volumes;
}

/// Repaint the View toggles' labels, so the menu says what is on rather than
/// only what can be turned on.
pub(crate) fn sync_view_menu_marks(
    overlays: Res<EditorOverlays>,
    marks: Query<(&ViewToggle, &Children)>,
    tails: Query<&Children>,
    mut texts: Query<&mut Text>,
) {
    for (toggle, children) in &marks {
        let on = match toggle {
            ViewToggle::KeyLegend => overlays.key_legend,
            ViewToggle::LinkPoints => overlays.link_points,
            ViewToggle::WorldGrid => overlays.world_grid,
            ViewToggle::ObjectVolumes => overlays.object_volumes,
        };
        let Some(word) = tail_word(children, &tails) else {
            continue;
        };
        let Ok(mut text) = texts.get_mut(word) else {
            continue;
        };
        let wanted = if on { "on" } else { "off" };
        if text.0 != wanted {
            text.0 = wanted.to_string();
        }
    }
}

/// Which overlay a View row toggles, so its right-hand column can read it back.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) enum ViewToggle {
    /// The footer's key line.
    KeyLegend,
    /// The socket gizmos.
    LinkPoints,
    /// The stage's ground plane.
    WorldGrid,
    /// The volumes and aims an object has no body to show.
    ObjectVolumes,
}

/// A disabled row's text, and an enabled one's, for the label column and the
/// shortcut column. Painted by a system rather than baked in at spawn because
/// Edit > Delete greys and ungreys with the selection (see [`sync_menu_delete`])
/// - and a row that only says "disabled" in its border is a row a builder keeps
/// pressing.
const ITEM_LABEL: Color = theme::PHOSPHOR;
/// The right-hand column: the shortcut, or a toggle's on/off mark.
const ITEM_MARK: Color = theme::PHOSPHOR_MUTED;
/// How much of its colour a greyed row keeps.
const DISABLED_ALPHA: f32 = 0.35;

/// Paint every menu row from whether it can be pressed.
///
/// The tail is reached through its wrapper rather than painted directly: a key
/// chip is a bordered box around its own text, so a greyed row has to take the
/// border down with the letter or the row reads as live from the one thing on
/// it that is coloured.
pub(crate) fn sync_menu_item_paint(
    items: Query<(Has<InteractionDisabled>, &Children), With<MenuItem>>,
    tails: Query<&Children>,
    chips: Query<Has<MenuKeyChip>>,
    mut texts: Query<&mut TextColor>,
    mut borders: Query<&mut BorderColor>,
) {
    for (disabled, children) in &items {
        let mut children = children.iter();
        if let Some(label) = children.next() {
            paint_text(&mut texts, label, ITEM_LABEL, disabled);
        }
        let Some(tail) = children.next() else {
            continue;
        };
        let Some(&tail) = tails.get(tail).ok().and_then(|kids| kids.first()) else {
            continue;
        };
        if chips.get(tail).unwrap_or(false) {
            for &text in tails.get(tail).map(|kids| &kids[..]).unwrap_or_default() {
                paint_text(&mut texts, text, theme::AMBER_NOVA, disabled);
            }
            let edge = alpha_if(theme::AMBER_NOVA.with_alpha(0.5), disabled);
            if let Ok(mut border) = borders.get_mut(tail) {
                if border.left != edge {
                    border.set_all(edge);
                }
            }
        } else {
            paint_text(&mut texts, tail, ITEM_MARK, disabled);
        }
    }
}

/// One text, at its colour or at the greyed fraction of it.
fn paint_text(texts: &mut Query<&mut TextColor>, entity: Entity, base: Color, disabled: bool) {
    let wanted = alpha_if(base, disabled);
    let Ok(mut colour) = texts.get_mut(entity) else {
        return;
    };
    if colour.0 != wanted {
        colour.0 = wanted;
    }
}

/// `colour`, faded to [`DISABLED_ALPHA`] of what it already had when greyed.
fn alpha_if(colour: Color, disabled: bool) -> Color {
    if disabled {
        colour.with_alpha(colour.alpha() * DISABLED_ALPHA)
    } else {
        colour
    }
}

/// Grey Edit > Delete unless the selection is something that can be deleted.
///
/// A PART counts. The row used to grey the moment a section was selected and
/// say nothing about where the verb had gone - it had gone to a brush in
/// another menu, under another name.
pub(crate) fn sync_menu_delete(
    mut commands: Commands,
    selected: Res<SelectedNode>,
    context: Res<crate::node::EditContext>,
    nodes: Query<
        (),
        Or<(
            With<crate::node::ShipNode>,
            With<crate::node::ObjectNode>,
            With<crate::node::SectionNode>,
        )>,
    >,
    items: Query<(Entity, Has<InteractionDisabled>), With<MenuDeleteItem>>,
) {
    let armable = selected
        .0
        .is_some_and(|node| crate::placement::deletable(node, &context, &nodes));
    for (entity, marked) in &items {
        match (armable, marked) {
            (false, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (true, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

/// Edit > Delete's row, so [`sync_menu_delete`] can find it.
#[derive(Component)]
pub(crate) struct MenuDeleteItem;

/// A row of the Ship menu that needs a ship to act on, so [`sync_ship_menu`]
/// can grey it at the scenario node.
///
/// Rebind is NOT one of these: it needs a bindable section as well, and
/// `crate::ui::sync_rebind_button` already paints that stricter rule.
#[derive(Component)]
pub(crate) struct ShipMenuItem;

/// Grey the Ship menu's rows at the scenario node.
///
/// Greyed rather than absent, the same as File > Save: the menu says what the
/// editor can do INSIDE a ship even while you are standing outside one, which
/// is also how a builder finds out that entering a ship is what unlocks it.
pub(crate) fn sync_ship_menu(
    mut commands: Commands,
    context: Res<crate::node::EditContext>,
    items: Query<(Entity, Has<InteractionDisabled>), With<ShipMenuItem>>,
) {
    let inside = context.ship().is_some();
    for (entity, marked) in &items {
        match (inside, marked) {
            (false, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (true, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

/// A Ship menu row that needs a part IN HAND, not just a ship - the pose verbs.
#[derive(Component)]
pub(crate) struct ArmedMenuItem;

/// Grey the pose verbs unless there is a part in hand for them to turn.
///
/// A stricter rule than [`sync_ship_menu`]'s, and its own system rather than a
/// branch inside it: these rows are live only in the one state R, F and the
/// wheel do anything in, so the menu is also where that state is reported.
pub(crate) fn sync_armed_menu(
    mut commands: Commands,
    choice: Res<crate::config::SectionChoice>,
    items: Query<(Entity, Has<InteractionDisabled>), With<ArmedMenuItem>>,
) {
    let armed = matches!(*choice, crate::config::SectionChoice::Section(_));
    for (entity, marked) in &items {
        match (armed, marked) {
            (false, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (true, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

/// The text a row's tail carries, reached through the tail's wrapper.
fn tail_word(row: &Children, tails: &Query<&Children>) -> Option<Entity> {
    let tail = row.iter().nth(1)?;
    tails.get(tail).ok()?.first().copied()
}

/// File > Back to Main Menu: end the session.
///
/// The document dies with it (`teardown_document` on leaving `Playing`), which
/// is the whole meaning of the item - it is the one exit that does not keep
/// what was built.
pub(crate) fn back_to_main_menu(
    _activate: On<Activate>,
    mut game_state: ResMut<NextState<GameStates>>,
) {
    game_state.set(GameStates::MainMenu);
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A bar with two dropdowns and a scrim on it, and nothing else - the
    /// display sync is the only thing under test.
    fn bar(world: &mut World) -> (Entity, Entity, Entity) {
        world.init_resource::<OpenMenu>();
        let file = world
            .spawn((MenuDropdown, MenuId::File, menu_dropdown_node()))
            .id();
        let add = world
            .spawn((MenuDropdown, MenuId::Add, menu_dropdown_node()))
            .id();
        let scrim = world
            .spawn((
                MenuScrim,
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        (file, add, scrim)
    }

    fn display(world: &World, entity: Entity) -> Display {
        world.get::<Node>(entity).expect("a node").display
    }

    /// One menu hangs open at a time, and the scrim goes up with it: a bar
    /// where two lists were down at once has no way to say which one the next
    /// click belongs to.
    #[test]
    fn the_open_menu_is_the_only_one_showing() {
        let mut world = World::new();
        let (file, add, scrim) = bar(&mut world);

        world.run_system_once(sync_menus).expect("the sync runs");
        assert_eq!(display(&world, file), Display::None);
        assert_eq!(display(&world, add), Display::None);
        assert_eq!(display(&world, scrim), Display::None, "no menu, no catcher");

        world.resource_mut::<OpenMenu>().0 = Some(MenuId::Add);
        world.run_system_once(sync_menus).expect("the sync runs");
        assert_eq!(display(&world, file), Display::None);
        assert_eq!(display(&world, add), Display::Flex);
        assert_eq!(display(&world, scrim), Display::Flex);
    }

    /// A bar button is a TOGGLE: the press that opened a menu closes it again,
    /// so a builder who opened the wrong one is not trapped in it.
    #[test]
    fn a_bar_button_toggles_its_own_menu() {
        let mut world = World::new();
        world.init_resource::<OpenMenu>();
        world.add_observer(on_menu_button);
        let file = world.spawn(MenuId::File).id();
        let view = world.spawn(MenuId::View).id();

        world.trigger(Activate { entity: file });
        world.flush();
        assert_eq!(world.resource::<OpenMenu>().0, Some(MenuId::File));

        world.trigger(Activate { entity: view });
        world.flush();
        assert_eq!(
            world.resource::<OpenMenu>().0,
            Some(MenuId::View),
            "a different button moves the open menu rather than closing it"
        );

        world.trigger(Activate { entity: view });
        world.flush();
        assert_eq!(world.resource::<OpenMenu>().0, None);
    }

    /// Pressing an item closes the menu it was in, whatever the item does.
    /// Central, so an item's own observer cannot forget - a menu left hanging
    /// over the thing it just changed is the bug this prevents.
    #[test]
    fn any_item_press_closes_the_menu_it_was_in() {
        let mut world = World::new();
        world.init_resource::<OpenMenu>();
        world.add_observer(close_menu_on_item);
        let item = world.spawn(MenuItem).id();
        let bystander = world.spawn_empty().id();

        world.resource_mut::<OpenMenu>().0 = Some(MenuId::Edit);
        world.trigger(Activate { entity: bystander });
        world.flush();
        assert_eq!(
            world.resource::<OpenMenu>().0,
            Some(MenuId::Edit),
            "a button that is not in a menu leaves it alone"
        );

        world.trigger(Activate { entity: item });
        world.flush();
        assert_eq!(world.resource::<OpenMenu>().0, None);
    }

    /// The View rows say what is ON, not only what can be turned on: a toggle
    /// that reads the same in both states tells a builder nothing.
    #[test]
    fn the_view_rows_report_what_they_toggle() {
        let mut world = World::new();
        world.init_resource::<EditorOverlays>();
        world.add_observer(toggle_link_points);
        // The REAL row, so the test reads the mark where the widget puts it
        // rather than where it once put it.
        let row = world
            .spawn((
                ViewToggle::LinkPoints,
                menu_item_row("Link Points", MenuTail::Word("on"), UiSkin::default()),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_view_menu_marks)
            .expect("the sync runs");
        assert_eq!(toggle_mark(&world, row), "on");

        world.trigger(Activate { entity: row });
        world.flush();
        world
            .run_system_once(sync_view_menu_marks)
            .expect("the sync runs");
        assert_eq!(toggle_mark(&world, row), "off");
        assert!(!world.resource::<EditorOverlays>().link_points);
    }

    /// What a row's tail says, read the way the sync writes it: the row's
    /// second child holds the tail, and the tail holds the word.
    fn toggle_mark(world: &World, row: Entity) -> String {
        let tail = world.get::<Children>(row).expect("the row has children")[1];
        let word = world.get::<Children>(tail).expect("the tail has a child")[0];
        world.get::<Text>(word).expect("the word").0.clone()
    }

    /// The Ship menu's rows need a ship. At the scenario node they are greyed
    /// rather than gone: the menu is where a builder reads what entering a
    /// ship would unlock.
    #[test]
    fn the_ship_rows_are_greyed_outside_a_ship() {
        use crate::node::{ScenarioNode, ShipNode};

        let mut world = World::new();
        world.init_resource::<crate::node::EditContext>();
        let scenario = world.spawn(ScenarioNode).id();
        let ship = world.spawn(ShipNode::default()).id();
        world.resource_mut::<crate::node::EditContext>().path = vec![scenario];
        let row = world.spawn(ShipMenuItem).id();

        world
            .run_system_once(sync_ship_menu)
            .expect("the sync runs");
        assert!(world.entity(row).contains::<InteractionDisabled>());

        world.resource_mut::<crate::node::EditContext>().enter(ship);
        world
            .run_system_once(sync_ship_menu)
            .expect("the sync runs");
        assert!(!world.entity(row).contains::<InteractionDisabled>());
    }
}
