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
//! one `observe`. A row that cannot act right now carries `InteractionDisabled`:
//! greyed says "this belongs here", absent says nothing at all.

use bevy::{
    ecs::{hierarchy::ChildOf, relationship::RelatedSpawner, spawn::SpawnWith},
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_gameplay::prelude::{GameStates, UnsavedWork};
use nova_ui::{
    theme,
    theme::{ActiveUiTheme, UiColor},
    widget::{checkbox_glyph, key_chip, ListRow, ThemedRadius, UiText},
};

use super::shell::EditorPanels;
use crate::{
    config::{EditorOverlays, EditorSays},
    history::EditHistory,
    tool::{swap_handle_space, GizmoSpaceType, StageTool},
    ui::layer,
};

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
    /// The verbs of the ship you are inside: arm a part, pose it, rebind a
    /// key. Shown only in a ship, where they have something to act on.
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
/// menu does not also land on the stage behind it and move something. A
/// surface and not a `Button`: it takes no click cue and no hover tick.
#[derive(Component)]
pub(crate) struct MenuScrim;

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
/// A KEY is drawn as the chip every other surface in the game draws a key as.
///
/// A toggle's state is NOT here: it sits in the lead column, drawn with the
/// checkbox's own glyph (see [`MenuLead::Toggle`]).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum MenuTail<'a> {
    /// Nothing to say.
    #[default]
    None,
    /// The key that runs this row.
    Key(&'a str),
}

/// What a row's LEAD column carries: a toggle's state, in its own column, so
/// it never shares one with the keys.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum MenuLead {
    /// Nothing in the column - which the column keeps anyway, so every label in
    /// a menu starts at the same x.
    #[default]
    None,
    /// A checkbox, ticked from the row's own state by the pass that owns it.
    Toggle,
}

/// The lead column of a row, so the toggle marks and the paint pass can find
/// it.
#[derive(Component)]
pub(crate) struct MenuLeadMark;

/// The tail column of a row, so the paint pass can find it.
#[derive(Component)]
pub(crate) struct MenuTailMark;

/// The chip half of a row's tail, so the paint pass can find it.
#[derive(Component)]
pub(crate) struct MenuKeyChip;

/// The font size a row's tail is drawn at.
const TAIL_FONT: f32 = 11.0;

/// One item row: the same `ListRow` shape the rail's rows wear, so the shared
/// reconciler paints the hover and this module owns no colour.
///
/// Every row has a lead, a label and a tail, each with its own marker, so
/// [`sync_menu_item_paint`] and [`sync_view_menu_marks`] find them by marker. A
/// row may hold other children: a [`MenuVerbType`](super::menu_context::MenuVerbType)
/// row's `Tooltip` spawns its panel as the FIRST child.
pub(crate) fn menu_item_row(label: &str, lead: MenuLead, tail: MenuTail) -> impl Bundle {
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
            column_gap: px(8),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            menu_item_lead(lead),
            (
                MenuItemLabel,
                UiText,
                Text::new(label.to_string()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                // One line. A wrappable label let the dropdown's auto height
                // be measured at a narrower width: a long Undo name left an
                // empty row's height under the last row.
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                // GROWS, which is what right-aligns the tail: three children in
                // a row cannot be spaced apart without pushing the label to the
                // middle of the menu.
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ),
            menu_item_tail(tail),
        ],
    )
}

/// The row's left-hand column.
///
/// Always spawned and always the same width, even where there is no mark to
/// draw: a column that appeared only on the rows that had one would step every
/// other label sideways.
fn menu_item_lead(lead: MenuLead) -> impl Bundle {
    let glyph = match lead {
        MenuLead::None => "",
        // Ticked by `sync_view_menu_marks` from the state the row toggles.
        MenuLead::Toggle => checkbox_glyph(false),
    };
    (
        MenuLeadMark,
        // `UiText`: the marks are line art the engine's built-in font does not
        // carry, and a missing glyph is drawn as an empty box.
        UiText,
        Text::new(glyph.to_string()),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::NONE),
        Node {
            width: px(12),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

/// The row's right-hand column.
///
/// Always spawned, even when there is nothing to say, so every row has the
/// same columns.
fn menu_item_tail(tail: MenuTail) -> impl Bundle {
    let tail = tail.owned();
    (
        MenuTailMark,
        Node::default(),
        Children::spawn(SpawnWith(
            move |parent: &mut RelatedSpawner<ChildOf>| match &tail {
                Some(key) => {
                    parent.spawn((MenuKeyChip, key_chip(key, TAIL_FONT + 2.0)));
                }
                None => {
                    parent.spawn((
                        UiText,
                        Text::new(String::new()),
                        TextFont {
                            font_size: FontSize::Px(TAIL_FONT),
                            ..default()
                        },
                        TextColor(Color::NONE),
                    ));
                }
            },
        )),
    )
}

impl MenuTail<'_> {
    /// The key it names, owned, so the spawn closure can hold it.
    fn owned(self) -> Option<String> {
        match self {
            MenuTail::Key(key) => Some(key.to_string()),
            MenuTail::None => None,
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
pub(crate) fn on_menu_scrim(_click: On<PointerClick>, mut open: ResMut<OpenMenu>) {
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
        GlobalZIndex(layer::SCRIM_Z),
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
    GlobalZIndex(layer::MENU_Z)
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

/// View > All Fields: show every field a config declares, instead of the ones
/// the kind is authored through.
pub(crate) fn toggle_all_fields(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.all_fields = !overlays.all_fields;
}

/// View > Ids: draw each tree row's id instead of the name it was given.
pub(crate) fn toggle_ids(_activate: On<Activate>, mut overlays: ResMut<EditorOverlays>) {
    overlays.ids = !overlays.ids;
}

/// View > Local Axes: the same verb as X.
pub(crate) fn toggle_handle_space(
    _activate: On<Activate>,
    mut tool: ResMut<StageTool>,
    mut says: EditorSays,
) {
    swap_handle_space(&mut tool, &mut says);
}

/// Tick the View toggles, so the menu says what is ON rather than only what
/// can be turned on.
///
/// The CHECKBOX's own glyph, in the lead column. The word `on` in the column
/// that also holds keys would make three vocabularies of one column, and the
/// editor two drawings of a tick.
pub(crate) fn sync_view_menu_marks(
    overlays: Res<EditorOverlays>,
    panels: Res<EditorPanels>,
    tool: Res<StageTool>,
    marks: Query<(&ViewToggle, &Children)>,
    leads: Query<(), With<MenuLeadMark>>,
    mut texts: Query<&mut Text>,
) {
    for (toggle, children) in &marks {
        let on = match toggle {
            ViewToggle::KeyLegend => overlays.key_legend,
            ViewToggle::LinkPoints => overlays.link_points,
            ViewToggle::WorldGrid => overlays.world_grid,
            ViewToggle::ObjectVolumes => overlays.object_volumes,
            ViewToggle::AllFields => overlays.all_fields,
            ViewToggle::Ids => overlays.ids,
            ViewToggle::Hierarchy => panels.hierarchy,
            ViewToggle::Inspector => panels.inspector,
            ViewToggle::Browser => panels.browser,
            ViewToggle::LocalAxes => tool.space == GizmoSpaceType::Local,
        };
        let Some(lead) = children.iter().find(|child| leads.contains(*child)) else {
            continue;
        };
        let Ok(mut text) = texts.get_mut(lead) else {
            continue;
        };
        let wanted = checkbox_glyph(on);
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
    /// Every field of a config, instead of the ones its kind is authored
    /// through.
    AllFields,
    /// The tree's ids, instead of the names the nodes were given.
    Ids,
    /// The left panel.
    Hierarchy,
    /// The right panel.
    Inspector,
    /// The body of the browser strip.
    Browser,
    /// The handles' local axes, instead of the world's.
    LocalAxes,
}

/// A disabled row's text, and an enabled one's, for the label column and the
/// shortcut column. Rows spawn UNPAINTED and [`sync_menu_item_paint`] is their
/// only paint path - one writer for one component, and a theme flip reaches an
/// open menu. Painted by a system rather than baked in at spawn because
/// Edit > Delete greys and ungreys with the selection (see
/// `crate::ui::menu_context`) - and a row that only says "disabled" in its border
/// is a row a builder keeps pressing.
const ITEM_LABEL: UiColor = UiColor::Primary;
/// The right-hand column: the shortcut, or a toggle's on/off mark.
const ITEM_MARK: UiColor = UiColor::Label;
/// How much of its colour a greyed row keeps.
const DISABLED_ALPHA: f32 = 0.35;

/// Paint every menu row from whether it can be pressed.
///
/// The tail is reached through its wrapper rather than painted directly: a key
/// chip is a bordered box around its own text, so a greyed row has to take the
/// border down with the letter or the row reads as live from the one thing on
/// it that is coloured.
pub(crate) fn sync_menu_item_paint(
    theme: Res<ActiveUiTheme>,
    items: Query<(Has<InteractionDisabled>, &Children), With<MenuItem>>,
    parts: Query<(Has<MenuLeadMark>, Has<MenuItemLabel>, Has<MenuTailMark>)>,
    tails: Query<&Children>,
    chips: Query<Has<MenuKeyChip>>,
    mut texts: Query<&mut TextColor>,
    mut borders: Query<&mut BorderColor>,
) {
    let item_label = theme.color(ITEM_LABEL);
    let item_mark = theme.color(ITEM_MARK);
    let chip_ink = theme.color(UiColor::Accent);
    for (disabled, children) in &items {
        for &part in children {
            match parts.get(part) {
                Ok((true, _, _)) => paint_text(&mut texts, part, item_mark, disabled),
                Ok((_, true, _)) => paint_text(&mut texts, part, item_label, disabled),
                _ => {}
            }
        }
        let Some(tail) = children
            .iter()
            .find(|&child| parts.get(child).is_ok_and(|(_, _, tail)| tail))
        else {
            continue;
        };
        let Some(&tail) = tails.get(tail).ok().and_then(|kids| kids.first()) else {
            continue;
        };
        if chips.get(tail).unwrap_or(false) {
            for &text in tails.get(tail).map(|kids| &kids[..]).unwrap_or_default() {
                paint_text(&mut texts, text, chip_ink, disabled);
            }
            let edge = alpha_if(theme.color_alpha(UiColor::Accent, 0.5), disabled);
            if let Ok(mut border) = borders.get_mut(tail) {
                if border.left != edge {
                    border.set_all(edge);
                }
            }
        } else {
            paint_text(&mut texts, tail, item_mark, disabled);
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

/// The label of a menu row, for a row whose wording changes.
#[derive(Component)]
pub(crate) struct MenuItemLabel;

/// Edit > Undo and Edit > Redo.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuHistoryItem {
    /// Takes the last step back.
    Undo,
    /// Puts the last undone step back.
    Redo,
}

/// Name the step each row would take, and grey the row with none to take.
pub(crate) fn sync_history_menu(
    mut commands: Commands,
    history: Res<EditHistory>,
    items: Query<(
        Entity,
        &MenuHistoryItem,
        Has<InteractionDisabled>,
        &Children,
    )>,
    mut labels: Query<&mut Text, With<MenuItemLabel>>,
) {
    for (entity, item, disabled, children) in &items {
        let (word, step) = match item {
            MenuHistoryItem::Undo => ("Undo", history.undo_label()),
            MenuHistoryItem::Redo => ("Redo", history.redo_label()),
        };
        match (step.is_some(), disabled) {
            (true, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            (false, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            _ => {}
        }
        let wanted = step.map_or_else(|| word.to_string(), |step| format!("{word} {step}"));
        for child in children.iter() {
            if let Ok(mut label) = labels.get_mut(child) {
                if label.0 != wanted {
                    label.0.clone_from(&wanted);
                }
            }
        }
    }
}

/// The top bar's word for a document that differs from its file.
#[derive(Component)]
pub(crate) struct UnsavedMark;

/// Show the mark while the document holds unsaved work, and tell the ways out
/// of the session the same thing ([`UnsavedWork`]).
pub(crate) fn sync_unsaved_mark(
    history: Res<EditHistory>,
    mut unsaved: ResMut<UnsavedWork>,
    mut marks: Query<&mut Node, With<UnsavedMark>>,
) {
    let dirty = history.dirty();
    unsaved.hold(dirty);
    let display = if dirty { Display::Flex } else { Display::None };
    for mut mark in &mut marks {
        if mark.display != display {
            mark.display = display;
        }
    }
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
    /// that reads the same in both states tells a builder nothing. The state is
    /// the CHECKBOX's mark, in the lead column, so it cannot be read as the key
    /// that runs the row.
    #[test]
    fn the_view_rows_report_what_they_toggle() {
        let mut world = World::new();
        world.init_resource::<EditorOverlays>();
        world.init_resource::<EditorPanels>();
        world.init_resource::<StageTool>();
        world.add_observer(toggle_link_points);
        // The REAL row, so the test reads the mark where the widget puts it
        // rather than where it once put it.
        let row = world
            .spawn((
                ViewToggle::LinkPoints,
                menu_item_row("Link Points", MenuLead::Toggle, MenuTail::None),
            ))
            .id();
        world.flush();

        world
            .run_system_once(sync_view_menu_marks)
            .expect("the sync runs");
        assert_eq!(toggle_mark(&world, row), checkbox_glyph(true));

        world.trigger(Activate { entity: row });
        world.flush();
        world
            .run_system_once(sync_view_menu_marks)
            .expect("the sync runs");
        assert_eq!(toggle_mark(&world, row), checkbox_glyph(false));
        assert!(!world.resource::<EditorOverlays>().link_points);
    }

    /// What a row's LEAD says, read the way the sync writes it: the mark is the
    /// row's [`MenuLeadMark`] child.
    fn toggle_mark(world: &World, row: Entity) -> String {
        let lead = world
            .get::<Children>(row)
            .expect("the row has children")
            .iter()
            .find(|child| world.get::<MenuLeadMark>(*child).is_some())
            .expect("the row has a lead");
        world.get::<Text>(lead).expect("the mark").0.clone()
    }

    /// A row whose first child is not its lead still has its label painted. A
    /// verb row's `Tooltip` spawns its panel as the first child, and the Add >
    /// Section rows drew with no label when the paint read children by place.
    #[test]
    fn a_row_with_a_tooltip_panel_first_still_paints_its_label() {
        let mut world = World::new();
        world.init_resource::<ActiveUiTheme>();
        let row = world
            .spawn(menu_item_row("Socket", MenuLead::None, MenuTail::None))
            .id();
        let panel = world.spawn(Node::default()).id();
        world.entity_mut(row).insert_children(0, &[panel]);

        world
            .run_system_once(sync_menu_item_paint)
            .expect("the paint runs");

        let label = world
            .get::<Children>(row)
            .expect("the row has children")
            .iter()
            .find(|child| world.get::<MenuItemLabel>(*child).is_some())
            .expect("the row has a label");
        let wanted = world.resource::<ActiveUiTheme>().color(ITEM_LABEL);
        assert_eq!(
            world.get::<TextColor>(label).map(|colour| colour.0),
            Some(wanted)
        );
    }
}
