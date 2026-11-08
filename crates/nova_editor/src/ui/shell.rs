//! The fixed frame of the editor: one top bar over the panels, the stage
//! between them, and the foot under the browser strip.
//!
//! ```text
//! Editor Top Bar    menus, Scene/Events, tools, Parts | Play | title
//! Editor Content    Editor Rail | Editor Stage | Script Mount
//! Editor Browser    tabs and the pane of the open tab
//! Editor Foot       status, selection count, key hints
//! ```
//!
//! Nothing here docks. A panel is shown or hidden, and the strip's top edge
//! drags its height. [`EditorPanels`] holds both; [`sync_panels`] shows the
//! panels and `browser::sync_browser` sizes the strip.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_ship::prelude::{GalleryAction, HeldPart};
use nova_ui::{
    prelude::*,
    theme::{self, density, ActiveUiTheme, UiColor},
};

use super::{
    canvas::{BlockPopup, CanvasArea, POPUP_W},
    inspector::PANEL_W as INSPECTOR_W,
    layer,
};
use crate::{
    camera::ViewportPointer,
    config::{EditorFoot, EditorRail, EditorViewType, PlayButton, SelectedNode},
    history::{redo_item, undo_item},
    node::{EditContext, ObjectNode, ShipNode},
    placement::continue_to_simulation,
    project::{
        ship::{open_ship_node, placed_design},
        trail::DocumentTrail,
        verbs::go_back,
        ModProject,
    },
    tool::{pick_tool, swap_handle_space, GizmoSpaceType, StageTool, StageToolType},
    ui::{
        menu::{MenuHistoryItem, UnsavedMark},
        menu_context::{MenuVerbType, VerbHint},
    },
};

/// The open browser strip's opening height, as a share of the window: 220 px
/// at 1080, 147 at 720.
const BROWSER_SHARE: f32 = 220.0 / 1080.0;

/// The shortest open strip: its tabs and three rows.
const BROWSER_MIN_H: f32 = 100.0;

/// The tallest open strip, as a share of the window. The stage keeps the rest.
const BROWSER_MAX_SHARE: f32 = 0.6;

/// How much of the window a folded strip takes: its tab row.
const BROWSER_FOLDED_H: f32 = 32.0;

/// The height of the foot row.
pub(crate) const FOOT_H: f32 = 22.0;

/// The height of the top bar.
pub(crate) const TOP_BAR_H: f32 = 36.0;

/// Under this logical width the top bar's words drop to glyphs, so the left
/// group fits beside Play. A bar at 1280 px holds the glyphs and not the words.
const BAR_WORDS_W: f32 = 1500.0;

/// What a pass-through node gives the stage: no hover, and no block on the
/// node under it.
const PASS_THROUGH: Pickable = Pickable {
    should_block_lower: false,
    is_hoverable: false,
};

/// Which panels are open. Lives for the session; nothing is persisted.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct EditorPanels {
    /// The left panel.
    pub(crate) hierarchy: bool,
    /// The right panel. The Events tab shows it whatever this says, because
    /// there the panel is the editor.
    pub(crate) inspector: bool,
    /// The body of the browser strip. Its tab row always stays.
    pub(crate) browser: bool,
    /// The open strip's height as a share of the window, so it keeps its
    /// proportion when the window changes.
    browser_share: f32,
}

impl Default for EditorPanels {
    fn default() -> Self {
        Self {
            hierarchy: true,
            inspector: true,
            browser: true,
            browser_share: BROWSER_SHARE,
        }
    }
}

/// `height`, inside the strip's limits. On a window too short for both, the
/// floor wins: a strip with no rows is no strip.
fn held_browser_height(height: f32, window_h: f32) -> f32 {
    height.min(window_h * BROWSER_MAX_SHARE).max(BROWSER_MIN_H)
}

impl EditorPanels {
    /// How tall the open strip stands in a window `window_h` tall.
    pub(crate) fn browser_height(&self, window_h: f32) -> f32 {
        held_browser_height(self.browser_share * window_h, window_h)
    }

    /// How much of the window's height the strip takes, open or folded.
    pub(crate) fn browser_taken(&self, window_h: f32) -> f32 {
        if self.browser {
            self.browser_height(window_h)
        } else {
            BROWSER_FOLDED_H
        }
    }

    /// Move the strip's top edge down by `pull` pixels.
    pub(crate) fn resize_browser(&mut self, pull: f32, window_h: f32) {
        let height = self.browser_height(window_h) - pull;
        self.browser_share = held_browser_height(height, window_h) / window_h;
    }
}

/// The node that holds the script surface, right of the stage.
///
/// Slice-owned seam: the block canvas spawns as the first child of this node,
/// and [`sync_panels`] already gives the node the whole row on the Events tab.
#[derive(Component)]
pub(crate) struct ScriptMount;

/// The column the Inspector stands in, inside [`ScriptMount`].
#[derive(Component)]
pub(crate) struct ScriptDetail;

/// The Inspector's width beside the canvas: the page is laid out for it.
pub(crate) const DETAIL_W: f32 = INSPECTOR_W;

/// The form's column width in the Form view: a field row reads at it, and the
/// row does not stretch across a wide window.
pub(crate) const FORM_W: f32 = 640.0;

/// The one bar along the top of the editor.
#[derive(Component)]
pub(crate) struct EditorTopBar;

/// The stage: the gap between the panels the viewport shows through.
#[derive(Component)]
pub(crate) struct EditorStage;

/// The summary drawn on the stage: the tool, and what is marked.
#[derive(Component)]
pub(crate) struct StageOverlay;

/// A toolbar button that picks a tool.
#[derive(Component, Clone, Copy)]
pub(crate) struct ToolButton(pub(crate) StageToolType);

/// The toolbar button that swaps the handle axes.
#[derive(Component)]
pub(crate) struct SpaceButton;

/// The toolbar button that toggles the snap.
#[derive(Component)]
pub(crate) struct SnapButton;

/// The bar's stage tools: the tool buttons, the axes, Snap and Parts. Events
/// and Form have no stage, so [`sync_panels`] hides them there. Hidden, not
/// taken out of the layout, so nothing beside them moves on a tab change.
#[derive(Component)]
pub(crate) struct StageTools;

/// What a top bar button says with room, and with little.
#[derive(Component, Clone, Copy)]
pub(crate) struct BarWord {
    wide: &'static str,
    narrow: &'static str,
}

/// Back: the document this one was opened over, named. Absent from the bar
/// when there is none.
#[derive(Component)]
pub(crate) struct BackButton;

/// Open Ship: the selected ship of the scenario, on its own. Absent from the
/// bar when no ship of a scenario is selected.
#[derive(Component)]
pub(crate) struct OpenShipButton;

/// The foot's count of marked nodes.
#[derive(Component)]
pub(crate) struct FootSelection;

/// One line of chrome text, on a single line.
fn line(text: &str, size: f32, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_string()),
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
    )
}

/// A compact bar button.
fn bar_button(label: &str) -> impl Bundle {
    button(bar_spec(label))
}

/// The one size every bar word is set in.
fn bar_spec(label: &str) -> ButtonSpec {
    ButtonSpec::new(label).ghost().dense()
}

/// A bar button whose word drops to `narrow` in a small window.
fn worded_button(wide: &'static str, narrow: &'static str, hint: &str) -> impl Bundle {
    (
        BarWord { wide, narrow },
        Tooltip(hint.to_string()),
        bar_button(wide),
    )
}

/// A thin upright rule between bar groups.
fn bar_rule() -> impl Bundle {
    (
        Node {
            width: px(theme::BORDER_W),
            height: px(18),
            flex_shrink: 0.0,
            margin: UiRect::axes(px(2), px(0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Primary, 0.16),
    )
}

/// Spawn the one top bar. Three columns keep Play in the middle of the window
/// however wide the left group is.
pub(crate) fn spawn_top_bar(root: &mut RelatedSpawnerCommands<ChildOf>) {
    root.spawn((
        Name::new("Editor Top Bar"),
        EditorTopBar,
        GlobalZIndex(layer::CHROME_Z),
        Node {
            width: percent(100),
            height: px(TOP_BAR_H),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            flex_shrink: 0.0,
            padding: UiRect::axes(px(density::PANEL_PAD), px(0)),
            border: UiRect::bottom(px(theme::BORDER_W)),
            column_gap: px(8),
            ..default()
        },
        panel(),
    ))
    .with_children(|bar| {
        bar.spawn((
            Name::new("Toolbar Left"),
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                // NO `min_width: px(0)`, unlike the right column. An item
                // allowed below its own content does not clip, it overflows,
                // and a menu button that is not there to press is a missing
                // verb. The right column's title is the part that gives way.
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(density::GAP),
                ..default()
            },
        ))
        .with_children(|left| {
            super::spawn_menu_bar(left);
            left.spawn(bar_rule());
            super::spawn_view_tabs(left);
            left.spawn(bar_rule());
            spawn_tool_group(left);
        });
        bar.spawn((
            Name::new("Play Slot"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|slot| {
            slot.spawn((
                Name::new("Play Button"),
                PlayButton,
                Tooltip::default(),
                button(ButtonSpec::new(super::PLAY_LABEL).dense().primary()),
                ButtonCueType::Owned,
                observe(continue_to_simulation),
            ));
        });
        bar.spawn((
            Name::new("Toolbar Right"),
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                min_width: px(0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                column_gap: px(density::GAP),
                ..default()
            },
        ))
        .with_children(|right| {
            // A wrapper carries the clip. `Overflow::clip` bounds a node's
            // children, and a text node draws its own glyphs.
            right
                .spawn((
                    Name::new("Editor Title Clip"),
                    Node {
                        min_width: px(0),
                        flex_shrink: 1.0,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_child((
                    Name::new("Editor Title"),
                    super::EditorTitle,
                    line("EDITOR", density::TEXT, UiColor::Body),
                ));
            // Beside the title and never in it: the title clips, and this is
            // the one word that must not be cut off.
            right.spawn((
                Name::new("Unsaved Mark"),
                UnsavedMark,
                line("UNSAVED", density::TEXT_SMALL, UiColor::Danger),
                Node {
                    display: Display::None,
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
        });
    });
}

/// The left group's tools: select, move, turn, scale, then axes, snap, history
/// and the parts gallery.
fn spawn_tool_group(group: &mut RelatedSpawnerCommands<ChildOf>) {
    group
        .spawn((
            Name::new("Stage Tools"),
            StageTools,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(density::GAP),
                ..default()
            },
        ))
        .with_children(|stage| {
            stage
                .spawn((Name::new("Tool Buttons"), segmented_container()))
                .with_children(|tools| {
                    for (tool, label, hint) in [
                        (StageToolType::Select, "Q", "Select (Q)"),
                        (StageToolType::Move, "W", "Move (W)"),
                        (StageToolType::Turn, "E", "Turn (E)"),
                        (StageToolType::Scale, "R", "Scale (R)"),
                    ] {
                        tools.spawn((
                            Name::new(format!("Tool Button {tool:?}")),
                            ToolButton(tool),
                            Tooltip(hint.to_string()),
                            button(bar_spec(label)),
                            observe(on_tool_button),
                        ));
                    }
                });
            stage.spawn((
                Name::new("Axes Button"),
                SpaceButton,
                Tooltip("Handle axes (X)".to_string()),
                bar_button("World"),
                observe(on_space_button),
            ));
            stage.spawn((
                Name::new("Snap Button"),
                SnapButton,
                worded_button("Snap", "#", "Snap handles (Ctrl flips it)"),
                observe(on_snap_button),
            ));
            stage.spawn(bar_rule());
        });
    group.spawn((
        Name::new("Undo Button"),
        MenuHistoryItem::Undo,
        worded_button("Undo", "<", "Undo (Ctrl+Z)"),
        observe(undo_item),
    ));
    group.spawn((
        Name::new("Redo Button"),
        MenuHistoryItem::Redo,
        worded_button("Redo", ">", "Redo (Ctrl+Y)"),
        observe(redo_item),
    ));
    group.spawn((StageTools, bar_rule()));
    group.spawn((
        Name::new("Parts Button"),
        StageTools,
        MenuVerbType::Parts,
        VerbHint("Section catalog (Tab)"),
        GalleryAction::Open,
        bar_button("Parts"),
    ));
    group.spawn((
        Name::new("Open Ship Button"),
        OpenShipButton,
        Tooltip("Open this ship on its own (Enter)".to_string()),
        bar_button("Open Ship"),
        observe(on_open_ship),
    ));
    group.spawn((
        Name::new("Back Button"),
        BackButton,
        Tooltip::default(),
        bar_button("Back"),
        observe(on_back),
    ));
}

/// The stage: empty, and open to the pointer, with its overlay in one corner.
///
/// Every node here is `Pickable::IGNORE` or pass-through, text included. A
/// node that took the hit would read as `ViewportPointer::over_ui` and stop
/// every camera gesture that started over it.
pub(crate) fn stage() -> impl Bundle {
    (
        Name::new("Editor Stage"),
        EditorStage,
        PASS_THROUGH,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            padding: UiRect::all(px(8)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            ..default()
        },
        children![(
            Name::new("Stage Overlay"),
            StageOverlay,
            Pickable::IGNORE,
            line("", 12.0, UiColor::Primary),
            Node {
                padding: UiRect::axes(px(6), px(2)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Void, 0.6),
        )],
    )
}

/// The column right of the stage. See [`ScriptMount`].
pub(crate) fn script_mount() -> impl Bundle {
    (
        Name::new("Script Mount"),
        ScriptMount,
        Node {
            width: px(INSPECTOR_W),
            flex_shrink: 0.0,
            min_height: px(0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            ..default()
        },
        panel(),
    )
}

/// The Inspector's column in the mount. In Events it floats as a block's
/// pop-up, so it wears a panel of its own.
pub(crate) fn script_detail() -> impl Bundle {
    (
        Name::new("Script Detail"),
        ScriptDetail,
        Node {
            width: px(DETAIL_W),
            flex_shrink: 0.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        panel(),
    )
}

/// The foot: the status line, the count, then the key hints.
pub(crate) fn spawn_foot(root: &mut RelatedSpawnerCommands<ChildOf>, legend: impl Bundle) {
    root.spawn((
        Name::new("Editor Foot"),
        EditorFoot,
        GlobalZIndex(layer::FOOT_Z),
        Node {
            width: percent(100),
            height: px(FOOT_H),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(14),
            padding: UiRect::axes(px(density::PANEL_PAD + 3.0), px(0)),
            border: UiRect::top(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        panel(),
    ))
    .with_children(|foot| {
        foot.spawn((
            Name::new("Placement Status Row"),
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_child((
            Name::new("Placement Status"),
            super::PlacementStatus,
            Visibility::Hidden,
            line("", 12.0, UiColor::Danger),
        ));
        foot.spawn((
            Name::new("Foot Selection"),
            FootSelection,
            line("", 11.0, UiColor::Secondary),
        ));
        foot.spawn(legend);
    });
}

/// A toolbar press on a tool: the same verb as its key.
fn on_tool_button(
    activate: On<Activate>,
    buttons: Query<&ToolButton>,
    mut tool: ResMut<StageTool>,
) {
    if let Ok(ToolButton(picked)) = buttons.get(activate.entity) {
        pick_tool(&mut tool, *picked);
    }
}

/// A toolbar press on the axes: the same verb as X.
fn on_space_button(
    _activate: On<Activate>,
    mut tool: ResMut<StageTool>,
    mut says: crate::config::EditorSays,
) {
    swap_handle_space(&mut tool, &mut says);
}

/// A toolbar press on Snap. Ctrl inverts whatever this says for one gesture.
fn on_snap_button(_activate: On<Activate>, mut tool: ResMut<StageTool>) {
    tool.snap = !tool.snap;
}

/// The nodes that open as a document of their own: a ship, and a placed
/// design, which opens the design.
type Openable<'w, 's> = Query<'w, 's, (Has<ShipNode>, Option<&'static ObjectNode>)>;

/// Whether `node` opens on its own.
fn opens_alone(nodes: &Openable, node: Entity) -> bool {
    nodes
        .get(node)
        .is_ok_and(|(ship, object)| ship || object.and_then(placed_design).is_some())
}

/// Open Ship: the selected ship, on its own.
fn on_open_ship(
    _activate: On<Activate>,
    mut commands: Commands,
    ships: Openable,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
) {
    if let Some(ship) = selected
        .primary()
        .filter(|node| opens_alone(&ships, *node) && context.ship().is_none())
    {
        commands.queue(move |world: &mut World| open_ship_node(world, ship, None));
    }
}

/// Back: the document this one was opened over.
fn on_back(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(go_back);
}

/// Enter over the stage opens the selected ship on its own. Over a panel the
/// key is the panel's.
pub(crate) fn stage_enter_opens_ship(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    pointer: Res<ViewportPointer>,
    ships: Openable,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
) {
    if !keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter])
        || pointer.over_ui
        || pointer.at.is_none()
        || context.ship().is_some()
    {
        return;
    }
    if let Some(ship) = selected.primary().filter(|node| opens_alone(&ships, *node)) {
        commands.queue(move |world: &mut World| open_ship_node(world, ship, None));
    }
}

/// Name Back's target, and show Open Ship over a selected ship of a scenario.
/// Each button leaves the bar when it could not act.
pub(crate) fn sync_ship_step(
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    ships: Openable,
    trail: Res<DocumentTrail>,
    project: Option<Res<ModProject>>,
    mut backs: Query<
        (&mut Node, &Children, &mut Tooltip),
        (With<BackButton>, Without<OpenShipButton>),
    >,
    mut opens: Query<&mut Node, (With<OpenShipButton>, Without<BackButton>)>,
    mut labels: Query<&mut Text, With<ButtonLabel>>,
) {
    let over = context.ship().is_none()
        && selected
            .primary()
            .is_some_and(|node| opens_alone(&ships, node));
    let shown = if over { Display::Flex } else { Display::None };
    for mut node in &mut opens {
        if node.display != shown {
            node.display = shown;
        }
    }
    let target = trail.last().map(|stop| {
        stop.open
            .as_ref()
            .and_then(|open| project.as_ref()?.data.name_of(open))
            .unwrap_or_else(|| "the last document".to_string())
    });
    let shown = if target.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let word = target
        .as_ref()
        .map_or_else(String::new, |name| format!("Back to {name}"));
    for (mut node, children, mut tooltip) in &mut backs {
        if node.display != shown {
            node.display = shown;
        }
        let hint = format!("{word} (Esc)");
        if tooltip.0 != hint {
            tooltip.0 = hint;
        }
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                if text.0 != word {
                    text.0.clone_from(&word);
                }
            }
        }
    }
}

/// Show the panels [`EditorPanels`] and the rail tab ask for.
///
/// Compared before writing rather than gated on `is_changed`: the panels are
/// spawned on entering the editor, which need not be a frame this changed on.
pub(crate) fn sync_panels(
    mut commands: Commands,
    panels: Res<EditorPanels>,
    tab: Res<EditorViewType>,
    popup: Res<BlockPopup>,
    mut rails: Query<&mut Node, (With<EditorRail>, Without<ScriptMount>, Without<EditorStage>)>,
    mut mounts: Query<&mut Node, (With<ScriptMount>, Without<EditorStage>)>,
    mut stages: Query<&mut Node, With<EditorStage>>,
    mut stage_tools: Query<&mut Visibility, With<StageTools>>,
    mut areas: Query<
        &mut Node,
        (
            With<CanvasArea>,
            Without<ScriptMount>,
            Without<EditorStage>,
            Without<EditorRail>,
        ),
    >,
    mut details: Query<
        (Entity, &mut Node),
        (
            With<ScriptDetail>,
            Without<CanvasArea>,
            Without<ScriptMount>,
            Without<EditorStage>,
            Without<EditorRail>,
        ),
    >,
) {
    let events = tab.is_events();
    // A form has no stage: its fields take the row, at a width a line of them
    // reads at, and the tree of its one item would say nothing.
    let form = *tab == EditorViewType::Form;
    let shown = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut rails {
        let display = shown(panels.hierarchy && !events && !form);
        if node.display != display {
            node.display = display;
        }
    }
    for mut node in &mut areas {
        let display = shown(events);
        if node.display != display {
            node.display = display;
        }
    }
    // In Events this column is a block's pop-up, floating over the canvas. The
    // pop-up's system places it; this says only what kind of node it is.
    let floating = events && popup.is_open();
    let detail_display = shown(if events {
        floating
    } else {
        panels.inspector || form
    });
    for (entity, mut node) in &mut details {
        if node.display != detail_display {
            node.display = detail_display;
        }
        let (position, width, border) = if floating {
            (
                PositionType::Absolute,
                POPUP_W,
                UiRect::all(px(theme::BORDER_W)),
            )
        } else if form {
            (
                PositionType::Relative,
                FORM_W,
                UiRect::horizontal(px(theme::BORDER_W)),
            )
        } else {
            (PositionType::Relative, DETAIL_W, UiRect::ZERO)
        };
        if node.position_type == position && !floating && node.width != px(width) {
            node.width = px(width);
            node.border = border;
        }
        if node.position_type != position {
            node.position_type = position;
            node.width = px(width);
            node.border = border;
            if floating {
                commands.entity(entity).insert(GlobalZIndex(layer::MENU_Z));
            } else {
                node.left = Val::Auto;
                node.top = Val::Auto;
                node.height = Val::Auto;
                commands.entity(entity).remove::<GlobalZIndex>();
            }
        }
    }
    // Events has no stage to keep clear: the mount takes the row, and the
    // stage's own labels sit below the chrome rung so nothing has to hide them.
    let (mount_width, mount_grow) = if events || form {
        (Val::Auto, 1.0)
    } else {
        (px(INSPECTOR_W), 0.0)
    };
    let justify = if form {
        JustifyContent::Center
    } else {
        JustifyContent::FlexStart
    };
    for mut node in &mut mounts {
        let display = shown(panels.inspector || events || form);
        if node.justify_content != justify {
            node.justify_content = justify;
        }
        if node.display != display {
            node.display = display;
        }
        if node.width != mount_width {
            node.width = mount_width;
        }
        if node.flex_grow != mount_grow {
            node.flex_grow = mount_grow;
        }
    }
    for mut node in &mut stages {
        let display = shown(!events && !form);
        if node.display != display {
            node.display = display;
        }
    }
    // The Ship editor has no stage tools: a part mates where it goes.
    let tools = if events || form || *tab == EditorViewType::Ship {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut stage_tools {
        if *visibility != tools {
            *visibility = tools;
        }
    }
}

/// Mark the active tool, name the axes, and light Snap.
pub(crate) fn sync_toolbar(
    mut commands: Commands,
    tool: Res<StageTool>,
    tools: Query<(Entity, &ToolButton, Has<Selected>)>,
    snaps: Query<(Entity, Has<Selected>), With<SnapButton>>,
    spaces: Query<&Children, With<SpaceButton>>,
    words: Query<(&BarWord, &Children)>,
    bars: Query<&ComputedNode, With<EditorTopBar>>,
    mut labels: Query<&mut Text, With<ButtonLabel>>,
) {
    for (entity, button, marked) in &tools {
        match (button.0 == tool.tool, marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
    for (entity, marked) in &snaps {
        match (tool.snap, marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
    let narrow = bars
        .iter()
        .next()
        .is_some_and(|bar| bar.size().x * bar.inverse_scale_factor() < BAR_WORDS_W);
    let axes = match (tool.space, narrow) {
        (GizmoSpaceType::World, false) => "World",
        (GizmoSpaceType::Local, false) => "Local",
        (GizmoSpaceType::World, true) => "Wld",
        (GizmoSpaceType::Local, true) => "Loc",
    };
    for children in &spaces {
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                if text.0 != axes {
                    text.0 = axes.to_string();
                }
            }
        }
    }
    for (word, children) in &words {
        let wanted = if narrow { word.narrow } else { word.wide };
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                if text.0 != wanted {
                    text.0 = wanted.to_string();
                }
            }
        }
    }
}

/// What the stage overlay says: the tool, then the part or the marks.
pub(crate) fn overlay_line(
    tool: &StageTool,
    held: &HeldPart,
    marked: usize,
    primary: Option<&str>,
) -> String {
    let hand = match held {
        HeldPart(Some(part)) => format!("PLACE {part}"),
        HeldPart(None) => {
            let axes = match tool.space {
                GizmoSpaceType::World => "WORLD",
                GizmoSpaceType::Local => "LOCAL",
            };
            let snap = if tool.snap { " SNAP" } else { "" };
            format!("{} {axes}{snap}", tool.tool.label())
        }
    };
    match (marked, primary) {
        (0, _) => hand,
        (1, Some(name)) => format!("{hand}  |  {name}"),
        (count, _) => format!("{hand}  |  {count} SELECTED"),
    }
}

/// Write the stage overlay and the foot's count.
pub(crate) fn sync_stage_overlay(
    tool: Res<StageTool>,
    held: Res<HeldPart>,
    selected: Res<SelectedNode>,
    hints: Query<(&super::SceneRow, &super::rail::SceneRowHint)>,
    mut overlays: Query<&mut Text, (With<StageOverlay>, Without<FootSelection>)>,
    mut counts: Query<&mut Text, With<FootSelection>>,
) {
    let primary = selected.primary().and_then(|node| {
        hints
            .iter()
            .find(|(row, _)| row.0 == node)
            .map(|(_, hint)| hint.id.as_str())
    });
    let said = overlay_line(&tool, &held, selected.len(), primary);
    for mut text in &mut overlays {
        if text.0 != said {
            text.0.clone_from(&said);
        }
    }
    let count = match selected.len() {
        0 => "NOTHING SELECTED".to_string(),
        count => format!("{count} SELECTED"),
    };
    for mut text in &mut counts {
        if text.0 != count {
            text.0.clone_from(&count);
        }
    }
}

/// View > Hierarchy, Inspector and Browser, and their keys.
///
/// Gated on `InputMode::Normal` where it is scheduled, so a typed digit never
/// reaches it.
pub(crate) fn panel_keys(keys: Res<ButtonInput<KeyCode>>, mut panels: ResMut<EditorPanels>) {
    if !crate::tool::snapping(Some(&keys)) {
        return;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        panels.hierarchy = !panels.hierarchy;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        panels.inspector = !panels.inspector;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        panels.browser = !panels.browser;
    }
}

/// View > Hierarchy.
pub(crate) fn toggle_hierarchy(_activate: On<Activate>, mut panels: ResMut<EditorPanels>) {
    panels.hierarchy = !panels.hierarchy;
}

/// View > Inspector.
pub(crate) fn toggle_inspector(_activate: On<Activate>, mut panels: ResMut<EditorPanels>) {
    panels.inspector = !panels.inspector;
}

/// View > Browser.
pub(crate) fn toggle_browser(_activate: On<Activate>, mut panels: ResMut<EditorPanels>) {
    panels.browser = !panels.browser;
}

/// The colour the validity word of a node takes.
pub(crate) fn validity_tint(theme: &ActiveUiTheme, errors: usize, warnings: usize) -> Color {
    theme.color(match (errors, warnings) {
        (0, 0) => UiColor::Nominal,
        (0, _) => UiColor::Accent,
        _ => UiColor::Danger,
    })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::{
        config::EditorStatus,
        tool::{tool_keys, StageTool},
    };

    fn toolbar_app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<StageTool>()
            .init_resource::<EditorViewType>()
            .init_resource::<EditorStatus>()
            .init_resource::<Time>()
            .insert_resource(HeldPart(None));
        app
    }

    fn press_key(app: &mut App, key: KeyCode) {
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(key);
        app.world_mut().insert_resource(input);
        app.world_mut()
            .run_system_once(tool_keys)
            .expect("the keys are read");
    }

    fn press_button(app: &mut App, button: impl Bundle) {
        let entity = app.world_mut().spawn(button).id();
        app.world_mut().trigger(Activate { entity });
        app.world_mut().flush();
    }

    #[test]
    fn a_tool_button_picks_what_its_key_picks() {
        for (key, tool) in [
            (KeyCode::KeyQ, StageToolType::Select),
            (KeyCode::KeyW, StageToolType::Move),
            (KeyCode::KeyE, StageToolType::Turn),
            (KeyCode::KeyR, StageToolType::Scale),
        ] {
            let mut by_key = toolbar_app();
            press_key(&mut by_key, key);
            let mut by_button = toolbar_app();
            press_button(&mut by_button, (ToolButton(tool), observe(on_tool_button)));
            assert_eq!(
                by_key.world().resource::<StageTool>().tool,
                by_button.world().resource::<StageTool>().tool,
            );
            assert_eq!(by_button.world().resource::<StageTool>().tool, tool);
        }
    }

    #[test]
    fn the_axes_button_swaps_what_x_swaps() {
        let mut by_key = toolbar_app();
        press_key(&mut by_key, KeyCode::KeyX);
        let mut by_button = toolbar_app();
        press_button(&mut by_button, observe(on_space_button));
        assert_eq!(
            by_key.world().resource::<StageTool>().space,
            by_button.world().resource::<StageTool>().space,
        );
        assert_eq!(
            by_button.world().resource::<StageTool>().space,
            GizmoSpaceType::Local
        );
    }

    /// The rail, the mount and the stage under `sync_panels`.
    fn panel_app() -> (App, Entity, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<EditorViewType>()
            .init_resource::<EditorPanels>()
            .init_resource::<BlockPopup>()
            .add_systems(Update, sync_panels);
        let rail = app.world_mut().spawn((EditorRail, Node::default())).id();
        let mount = app.world_mut().spawn((ScriptMount, Node::default())).id();
        let stage = app.world_mut().spawn((EditorStage, Node::default())).id();
        (app, rail, mount, stage)
    }

    /// Events is a mode, not a filter on a list: the stage is not placed on, so
    /// the mount takes its width.
    #[test]
    fn the_events_tab_gives_the_mount_the_stage() {
        let (mut app, _, mount, stage) = panel_app();
        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
        app.update();

        let node = |entity| app.world().get::<Node>(entity).expect("a node");
        assert_eq!(node(mount).flex_grow, 1.0);
        assert_eq!(node(stage).display, Display::None);
    }

    /// And back: the placement ray goes through the window's centre, so Scene
    /// has to give the stage its width again.
    #[test]
    fn the_scene_tab_hands_the_stage_back() {
        let (mut app, _, mount, stage) = panel_app();
        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
        app.update();
        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Scene;
        app.update();

        let node = |entity| app.world().get::<Node>(entity).expect("a node");
        assert_eq!(node(mount).width, px(INSPECTOR_W));
        assert_eq!(node(mount).flex_grow, 0.0);
        assert_eq!(node(stage).display, Display::Flex);
    }

    /// Events shows the palette and the canvas: no Hierarchy and no Inspector,
    /// whatever the panel switches say.
    #[test]
    fn events_shows_no_hierarchy_or_inspector() {
        let (mut app, rail, mount, _) = panel_app();
        let detail = app.world_mut().spawn((ScriptDetail, Node::default())).id();
        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
        app.update();

        let display = |entity| app.world().get::<Node>(entity).expect("a node").display;
        assert_eq!(display(rail), Display::None);
        assert_eq!(display(detail), Display::None);
        assert_eq!(display(mount), Display::Flex);
    }

    /// The stage tools have nothing to act on in Events. They hide without
    /// leaving the layout, so Undo and Play hold their place across the tabs.
    #[test]
    fn events_hides_the_stage_tools_and_keeps_their_place() {
        let (mut app, ..) = panel_app();
        let tools = app
            .world_mut()
            .spawn((StageTools, Node::default(), Visibility::default()))
            .id();
        let seen = |app: &App| {
            let world = app.world();
            let visibility = *world.get::<Visibility>(tools).expect("a visibility");
            (
                visibility,
                world.get::<Node>(tools).expect("a node").display,
            )
        };

        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Events;
        app.update();
        assert_eq!(seen(&app), (Visibility::Hidden, Display::Flex));

        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Scene;
        app.update();
        assert_eq!(seen(&app), (Visibility::Inherited, Display::Flex));

        *app.world_mut().resource_mut::<EditorViewType>() = EditorViewType::Ship;
        app.update();
        assert_eq!(
            seen(&app),
            (Visibility::Hidden, Display::Flex),
            "nor in a ship"
        );
    }

    /// `ViewportPointer::over_ui` is true for any `Node` that takes the hit, and
    /// a true there stops every camera gesture that starts on it.
    #[test]
    fn nothing_on_the_stage_takes_the_pointer() {
        let mut world = World::new();
        let stage = world.spawn(stage()).id();
        let overlay = world
            .get::<Children>(stage)
            .expect("the overlay hangs off the stage")
            .to_vec();
        assert!(!overlay.is_empty());
        for node in std::iter::once(stage).chain(overlay) {
            let pickable = world.get::<Pickable>(node).expect("each node says so");
            assert!(!pickable.is_hoverable, "{node:?} would take the hover");
            assert!(
                !pickable.should_block_lower,
                "{node:?} would block the stage"
            );
        }
    }

    #[test]
    fn snap_inverts_under_ctrl() {
        let mut tool = StageTool::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        assert!(!crate::tool::snap_active(&tool, Some(&keys)));
        keys.press(KeyCode::ControlLeft);
        assert!(crate::tool::snap_active(&tool, Some(&keys)));
        tool.snap = true;
        assert!(!crate::tool::snap_active(&tool, Some(&keys)));
        keys.release(KeyCode::ControlLeft);
        assert!(crate::tool::snap_active(&tool, Some(&keys)));
    }

    #[test]
    fn the_strip_opens_in_proportion_to_the_window() {
        let panels = EditorPanels::default();

        assert_eq!(panels.browser_height(1080.0).round(), 220.0);
        assert_eq!(panels.browser_height(720.0).round(), 147.0);
    }

    #[test]
    fn a_resize_stops_at_the_limits() {
        let mut panels = EditorPanels::default();

        panels.resize_browser(-5000.0, 720.0);
        assert_eq!(panels.browser_height(720.0).round(), 432.0);

        panels.resize_browser(5000.0, 720.0);
        assert_eq!(panels.browser_height(720.0).round(), 100.0);
    }
}
