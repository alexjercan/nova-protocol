//! The browser strip under the stage: Assets, Problems, Ship and Log.
//!
//! One strip with a tab row, sized by [`EditorPanels`]. Each pane is built once
//! and shown by [`sync_browser`]. Problems rebuild when their findings move. The
//! Log tab is `log`. Assets are the grid in `assets`; sections are browsed in the
//! parts gallery. Log and Problems rows copy out through `row_copy`.

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_ui::{
    prelude::*,
    theme::{self, density, UiColor},
};

use super::{
    assets::{spawn_asset_body, spawn_asset_controls},
    layer,
    log::{spawn_log_controls, LogList},
    on_part_choice, on_skin_toggle, on_style_choice,
    rail::{part_row, skin_toggle_row, style_row},
    row_copy::{copy_row, spawn_copy_control, CopyListType, CopyRows, ShownRow},
    shell::EditorPanels,
    PartList, StyleList,
};
use crate::{
    config::{EditorViewType, HullPlanLine, SelectedNode},
    generate::{generate_ship, reroll_seed, GenerateButton, HullSeedField, UNAUTHORED_WEIGHT},
    node::{EditContext, NodeId, ObjectNode, ScenarioNode, SectionNode, ShipNode},
    problems::{Problem, ProblemSeverityType, Problems},
    project::ship::open_ship_node,
    ui::canvas::CanvasBoard,
};

/// Which tab of the browser strip is open.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTabType {
    /// What the document and the enabled bundles hold.
    #[default]
    Assets,
    /// What the lint says about the document.
    Problems,
    /// The settings of the entered ship. Listed only inside a ship.
    Ship,
    /// What the status line has said, and what the log warned of.
    Log,
}

impl BrowserTabType {
    /// Every tab, in strip order.
    const ALL: [Self; 4] = [Self::Assets, Self::Problems, Self::Ship, Self::Log];

    /// The word on the tab, and the end of its `Name`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Assets => "Assets",
            Self::Problems => "Problems",
            Self::Ship => "Ship",
            Self::Log => "Log",
        }
    }
}

/// A tab of the strip.
#[derive(Component, Clone, Copy)]
pub(crate) struct BrowserTabButton(BrowserTabType);

/// A pane of the strip, shown while its tab is open.
#[derive(Component, Clone, Copy)]
pub(crate) struct BrowserPane(pub(crate) BrowserTabType);

/// The body of the strip, under the tabs.
#[derive(Component)]
pub(crate) struct BrowserBody;

/// The strip itself.
#[derive(Component)]
pub(crate) struct EditorBrowser;

/// The grip on the strip's top edge.
#[derive(Component)]
pub(crate) struct BrowserResizeHandle;

/// The button that folds the strip.
#[derive(Component)]
pub(crate) struct BrowserFold;

/// The list the Problems pane fills.
#[derive(Component)]
pub(crate) struct ProblemList;

/// How tall the grip on the strip's top edge stands.
const RESIZE_GRIP_H: f32 = 4.0;

/// The node a browser row selects.
#[derive(Component, Clone, Copy)]
pub(crate) struct RowTarget(pub(crate) Entity);

/// One line of pane text.
pub(super) fn pane_text(text: &str, size: f32, colour: UiColor) -> impl Bundle {
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

/// Text that wraps inside a column.
fn pane_note(text: &str, size: f32, colour: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(colour),
    )
}

/// One row of a browser list: a mark, a label that takes the free width, and a
/// muted detail. The label clips rather than pushing the detail out.
fn browser_row(mark: &str, label: &str, detail: &str, tint: UiColor) -> impl Bundle {
    (
        ListRow,
        Button,
        Hovered::default(),
        browser_line(mark, label, detail, tint),
    )
}

/// A browser row's face: what a row that goes nowhere shows on its own.
fn browser_line(mark: &str, label: &str, detail: &str, tint: UiColor) -> impl Bundle {
    (
        Node {
            width: percent(100),
            min_height: px(20),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::axes(px(6), px(1)),
            border: UiRect::all(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        ThemedRadius::control(),
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
        children![
            pane_text(mark, 12.0, tint),
            (
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                children![pane_text(label, 12.0, UiColor::Primary)],
            ),
            pane_text(detail, 11.0, UiColor::Label),
        ],
    )
}

/// A pane that is one scrolling column named `name`.
fn spawn_scroll_list(
    pane: &mut RelatedSpawnerCommands<ChildOf>,
    name: &str,
    list: impl Bundle,
    rows: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
) {
    let layout = Node {
        align_items: AlignItems::Stretch,
        ..scroll_column()
    };
    spawn_scroll_body(pane, name, (list, layout), rows);
}

/// A scrolling list that lays `body`'s rows out as `body`'s `Node` says.
fn spawn_scroll_body(
    pane: &mut RelatedSpawnerCommands<ChildOf>,
    name: &str,
    body: impl Bundle,
    rows: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
) {
    pane.spawn((Name::new(format!("{name} Scroll")), scroll_row()))
        .with_children(|row| {
            row.spawn((Name::new(name.to_string()), body, scroll_viewport()))
                .with_children(rows);
            row.spawn((Name::new(format!("{name} Bar")), scroll_bar()));
        });
}

/// A pane node.
fn pane(tab: BrowserTabType) -> impl Bundle {
    (
        Name::new(format!("Browser Pane {}", tab.label())),
        BrowserPane(tab),
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            width: percent(100),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            ..default()
        },
    )
}

/// The filter row a list pane opens with.
fn filter_row() -> Node {
    Node {
        width: percent(100),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(8),
        ..default()
    }
}

/// What the Ship pane is built from.
pub(crate) struct ShipPaneData {
    /// Whether the entered ship wears the derived skin.
    pub(crate) skinned: bool,
    /// The styles on offer: id, name, swatch.
    pub(crate) styles: Vec<(String, String, Color)>,
    /// The parts a hull may draw: id, name, ticked, zone.
    pub(crate) parts: Vec<(String, String, bool, Option<nova_wfc::prelude::WfcZone>)>,
    /// The seed the field opens on.
    pub(crate) seed: u64,
}

fn spawn_ship_pane(strip: &mut RelatedSpawnerCommands<ChildOf>, ship: &ShipPaneData) {
    let column = |grow: f32| Node {
        flex_grow: grow,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(3),
        ..default()
    };
    strip
        .spawn(pane(BrowserTabType::Ship))
        .insert(Node {
            flex_grow: 1.0,
            min_height: px(0),
            width: percent(100),
            flex_direction: FlexDirection::Row,
            column_gap: px(16),
            ..default()
        })
        .with_children(|pane| {
            pane.spawn((Name::new("Ship Skin Column"), column(1.0)))
                .with_children(|skin| {
                    skin.spawn((
                        Name::new("Ship Skin Toggle"),
                        skin_toggle_row(ship.skinned),
                        observe(on_skin_toggle),
                    ));
                    skin.spawn((
                        Name::new("Ship Skin Note"),
                        pane_note("reflows around the part in hand", 11.0, UiColor::Label),
                    ));
                    // Two to a line: the strip is too short to stack the base
                    // styles, and a mod may add more.
                    let styles = Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        align_content: AlignContent::FlexStart,
                        column_gap: px(4),
                        ..scroll_column()
                    };
                    spawn_scroll_body(skin, "Ship Style List", (StyleList, styles), |list| {
                        for (index, (id, name, colour)) in ship.styles.iter().enumerate() {
                            let mut row = list.spawn((
                                Name::new(format!("Style: {name}")),
                                style_row(id, name, *colour),
                                observe(on_style_choice),
                            ));
                            // The first is what an unset style wears.
                            if index == 0 {
                                row.insert(Selected);
                            }
                        }
                    });
                });
            pane.spawn((Name::new("Generate Column"), column(1.0)))
                .with_children(|block| {
                    block.spawn((
                        Name::new("Generate Note"),
                        pane_note(
                            "collapses this ship out of the ticked sections, replacing what it holds",
                            11.0,
                            UiColor::Label,
                        ),
                    ));
                    block
                        .spawn((Name::new("Generate Seed Row"), filter_row()))
                        .with_children(|row| {
                            row.spawn((
                                Name::new("Hull Seed Field"),
                                HullSeedField,
                                text_field(
                                    TextFieldSpec::new(ship.seed.to_string())
                                        .max_chars(super::SEED_DIGITS)
                                        .tool(),
                                ),
                            ));
                            row.spawn((
                                Name::new("Reroll Seed Button"),
                                button(ButtonSpec::new("Reroll").ghost().dense()),
                                observe(reroll_seed),
                            ));
                            row.spawn((
                                Name::new("Generate Button"),
                                GenerateButton,
                                button(ButtonSpec::new("Generate").dense()),
                                ButtonCueType::Owned,
                                observe(generate_ship),
                            ));
                        });
                    block.spawn((
                        Name::new("Hull Plan Label"),
                        pane_text("HULL PLAN", 10.0, UiColor::Label),
                    ));
                    block.spawn((
                        Name::new("Hull Plan Line"),
                        HullPlanLine,
                        pane_note("stern -\nbow -", 10.0, UiColor::Secondary),
                    ));
                });
            pane.spawn((Name::new("Draw Column"), column(1.4)))
                .with_children(|block| {
                    block.spawn((
                        Name::new("Draw Label"),
                        pane_text("DRAW FROM", 10.0, UiColor::Label),
                    ));
                    block.spawn((
                        Name::new("Draw Note"),
                        pane_note(
                            &format!(
                                "unpriced parts join at {UNAUTHORED_WEIGHT}; the chip says where a part may stand",
                            ),
                            10.0,
                            UiColor::Label,
                        ),
                    ));
                    spawn_scroll_list(block, "Part List", PartList, |list| {
                        for (id, name, drawn, zone) in &ship.parts {
                            let mut row = list.spawn((
                                Name::new(format!("Part: {id}")),
                                part_row(id, name, *drawn, *zone),
                                observe(on_part_choice),
                            ));
                            // The mark is the setting; the paint is its picture.
                            if *drawn {
                                row.insert(Selected);
                            }
                        }
                    });
                });
        });
}

/// Spawn the strip.
pub(crate) fn spawn_browser(root: &mut RelatedSpawnerCommands<ChildOf>, ship: &ShipPaneData) {
    root.spawn((
        Name::new("Editor Browser"),
        EditorBrowser,
        GlobalZIndex(layer::CHROME_Z),
        Node {
            width: percent(100),
            height: Val::Auto,
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(density::GAP),
            padding: UiRect {
                bottom: px(2),
                ..UiRect::horizontal(px(density::PANEL_PAD))
            },
            border: UiRect::top(px(theme::BORDER_W)),
            overflow: Overflow::clip(),
            ..default()
        },
        panel(),
    ))
    .with_children(|strip| {
        strip.spawn((
            Name::new("Browser Resize Handle"),
            BrowserResizeHandle,
            Node {
                width: percent(100),
                height: px(RESIZE_GRIP_H),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Primary, 0.16),
            observe(on_browser_resize),
        ));
        strip
            .spawn((
                Name::new("Browser Tabs"),
                Node {
                    width: percent(100),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((Name::new("Browser Tab Row"), tab_strip()))
                    .with_children(|tabs| {
                        for browser_tab in BrowserTabType::ALL {
                            tabs.spawn((
                                Name::new(format!("Browser Tab {}", browser_tab.label())),
                                tab(browser_tab.label(), false),
                                BrowserTabButton(browser_tab),
                                observe(on_browser_tab),
                            ));
                        }
                    });
                row.spawn((
                    Name::new("Browser Tools"),
                    Node {
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        column_gap: px(density::GAP),
                        ..default()
                    },
                ))
                .with_children(|tools| {
                    spawn_asset_controls(tools);
                    spawn_log_controls(tools);
                    spawn_copy_control(tools);
                    tools.spawn((
                        Name::new("Browser Fold"),
                        BrowserFold,
                        button(ButtonSpec::new("Fold").ghost().dense().fit()),
                        observe(on_browser_fold),
                    ));
                });
            });
        strip
            .spawn((
                Name::new("Browser Body"),
                BrowserBody,
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ))
            .with_children(|body| {
                body.spawn(pane(BrowserTabType::Assets))
                    .with_children(|pane| {
                        spawn_asset_body(pane);
                    });
                // Hovered: the copy keys act on the list under the pointer.
                body.spawn((pane(BrowserTabType::Problems), Hovered::default()))
                    .with_children(|pane| {
                        spawn_scroll_list(pane, "Problem List", ProblemList, |_| {});
                    });
                spawn_ship_pane(body, ship);
                body.spawn((pane(BrowserTabType::Log), Hovered::default()))
                    .with_children(|pane| {
                        spawn_scroll_list(pane, "Log List", LogList, |_| {});
                    });
            });
    });
}

/// A tab press: open it, and open the strip with it.
fn on_browser_tab(
    activate: On<Activate>,
    tabs: Query<&BrowserTabButton>,
    mut open: ResMut<BrowserTabType>,
    mut panels: ResMut<EditorPanels>,
) {
    if let Ok(BrowserTabButton(picked)) = tabs.get(activate.entity) {
        *open = *picked;
        panels.browser = true;
    }
}

/// Drag the strip's top edge. Up makes the strip taller.
fn on_browser_resize(
    mut drag: On<Pointer<Drag>>,
    mut panels: ResMut<EditorPanels>,
    screen: Option<Single<&Window>>,
) {
    drag.propagate(false);
    if drag.button != PointerButton::Primary {
        return;
    }
    if let Some(screen) = screen {
        panels.resize_browser(drag.delta.y, screen.height());
    }
}

/// Fold or unfold the strip.
fn on_browser_fold(_activate: On<Activate>, mut panels: ResMut<EditorPanels>) {
    panels.browser = !panels.browser;
}

/// Show the open pane, fold the strip, and hide the Ship tab outside a ship.
pub(crate) fn sync_browser(
    mut commands: Commands,
    mut open: ResMut<BrowserTabType>,
    panels: Res<EditorPanels>,
    context: Res<EditContext>,
    problems: Res<Problems>,
    screen: Option<Single<&Window>>,
    mut strips: Query<
        &mut Node,
        (
            With<EditorBrowser>,
            Without<BrowserBody>,
            Without<BrowserResizeHandle>,
            Without<BrowserPane>,
            Without<BrowserTabButton>,
        ),
    >,
    mut bodies: Query<
        &mut Node,
        (
            Or<(With<BrowserBody>, With<BrowserResizeHandle>)>,
            Without<BrowserPane>,
            Without<BrowserTabButton>,
        ),
    >,
    mut panes: Query<(&BrowserPane, &mut Node), Without<BrowserTabButton>>,
    mut tabs: Query<(
        Entity,
        &BrowserTabButton,
        &mut Node,
        &Children,
        Has<Selected>,
    )>,
    mut labels: Query<&mut Text, With<TabLabel>>,
    folds: Query<&Children, With<BrowserFold>>,
    mut fold_labels: Query<&mut Text, (With<ButtonLabel>, Without<TabLabel>)>,
) {
    let inside = context.ship().is_some();
    if !inside && *open == BrowserTabType::Ship {
        *open = BrowserTabType::Assets;
    }
    let height = match (&screen, panels.browser) {
        (Some(screen), true) => px(panels.browser_height(screen.height())),
        _ => Val::Auto,
    };
    for mut node in &mut strips {
        if node.height != height {
            node.height = height;
        }
    }
    let shown = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut bodies {
        let display = shown(panels.browser);
        if node.display != display {
            node.display = display;
        }
    }
    for (pane, mut node) in &mut panes {
        let display = shown(pane.0 == *open);
        if node.display != display {
            node.display = display;
        }
    }
    let (errors, warnings) = problems.counts();
    for (entity, tab, mut node, children, marked) in &mut tabs {
        let display = shown(tab.0 != BrowserTabType::Ship || inside);
        if node.display != display {
            node.display = display;
        }
        match (tab.0 == *open, marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
        if tab.0 != BrowserTabType::Problems {
            continue;
        }
        let said = match errors + warnings {
            0 => "PROBLEMS".to_string(),
            count => format!("PROBLEMS {count}"),
        };
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child) {
                if text.0 != said {
                    text.0.clone_from(&said);
                }
            }
        }
    }
    let word = if panels.browser { "Fold" } else { "Unfold" };
    for children in &folds {
        for child in children {
            if let Ok(mut text) = fold_labels.get_mut(*child) {
                if text.0 != word {
                    text.0 = word.to_string();
                }
            }
        }
    }
}

/// The mark and tint of a problem row.
fn problem_paint(problem: &Problem) -> (&'static str, UiColor) {
    match problem.severity {
        ProblemSeverityType::Error => ("!", UiColor::Danger),
        ProblemSeverityType::Warning => ("?", UiColor::Accent),
    }
}

/// Where a finding stands: the ids from the scenario down to its node, as
/// `ship_1/hull_2`. Empty for a finding that names no node.
fn node_path(
    node: Entity,
    ids: &Query<&NodeId>,
    parents: &Query<&ChildOf>,
    roots: &Query<(), With<ScenarioNode>>,
) -> String {
    let mut path: Vec<&str> = std::iter::once(node)
        .chain(parents.iter_ancestors(node))
        .take_while(|step| !roots.contains(*step))
        .filter_map(|step| ids.get(step).ok().map(|id| id.0.as_str()))
        .collect();
    path.reverse();
    path.join("/")
}

/// A finding as one line of plain text, with where it stands.
pub(crate) fn problem_text(problem: &Problem, location: &str) -> String {
    let severity = match problem.severity {
        ProblemSeverityType::Error => "error",
        ProblemSeverityType::Warning => "warning",
    };
    if location.is_empty() {
        format!("{severity}: {}", problem.message)
    } else {
        format!("{severity}: {} (at {location})", problem.message)
    }
}

/// Rebuild the Problems rows when the findings change.
pub(crate) fn sync_problem_rows(
    mut commands: Commands,
    problems: Res<Problems>,
    ids: Query<&NodeId>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<ScenarioNode>>,
    lists: Query<Entity, With<ProblemList>>,
    fresh: Query<(), Added<ProblemList>>,
    mut copy: ResMut<CopyRows>,
    mut shown: Local<Option<Vec<Problem>>>,
) {
    if !fresh.is_empty() {
        *shown = None;
    }
    if shown.as_deref() == Some(problems.list.as_slice()) {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    let mut rows = Vec::with_capacity(problems.list.len());
    commands.entity(list).despawn_related::<Children>();
    commands.entity(list).with_children(|list| {
        for (index, problem) in problems.list.iter().enumerate() {
            let (mark, tint) = problem_paint(problem);
            let location = problem
                .node
                .map_or_else(String::new, |node| node_path(node, &ids, &parents, &roots));
            let key = index as u64;
            rows.push(ShownRow {
                key,
                text: problem_text(problem, &location),
            });
            let name = Name::new(format!("Problem Row {index}"));
            let copy = copy_row(CopyListType::Problems, key);
            match problem.node {
                Some(node) => {
                    list.spawn((
                        name,
                        browser_row(mark, &problem.message, &location, tint),
                        RowTarget(node),
                        observe(on_target_row),
                        copy,
                    ));
                }
                None => {
                    list.spawn((
                        name,
                        ListRow,
                        Hovered::default(),
                        browser_line(mark, &problem.message, &location, tint),
                        copy,
                    ));
                }
            }
        }
    });
    // A row's key is its place in the findings, which a new lint reorders.
    copy.set_rows(CopyListType::Problems, rows, false);
    *shown = Some(problems.list.clone());
}

/// Select the node a browser row names, going where the editor has to stand to
/// show it: the owning ship's own document for a section of another ship, the
/// Events tab with the block framed for a script node, and the Scene for
/// anything else. A Shift press only widens the rows marked for copying.
pub(crate) fn on_target_row(
    activate: On<Activate>,
    mut commands: Commands,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    rows: Query<&RowTarget>,
    owners: Query<&ChildOf>,
    ships: Query<(), With<ShipNode>>,
    sections: Query<(), With<SectionNode>>,
    objects: Query<(), With<ObjectNode>>,
    named: Query<(), With<NodeId>>,
    context: Res<EditContext>,
    mut tab: ResMut<EditorViewType>,
    mut selected: ResMut<SelectedNode>,
    mut board: ResMut<CanvasBoard>,
) {
    let Ok(RowTarget(node)) = rows.get(activate.entity) else {
        return;
    };
    if keys.is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])) {
        return;
    }
    let node = *node;
    if !named.contains(node) {
        return;
    }
    let in_ship_document = context.ship().is_some();
    if sections.contains(node) {
        let ship = owners
            .iter_ancestors(node)
            .find(|ancestor| ships.contains(*ancestor));
        if let Some(ship) = ship.filter(|ship| context.ship() != Some(*ship)) {
            commands.queue(move |world: &mut World| open_ship_node(world, ship, Some(node)));
            return;
        }
    } else if ships.contains(node) || objects.contains(node) {
        if !in_ship_document {
            *tab = EditorViewType::Scene;
        }
    } else if context.scenario().is_some_and(|scenario| scenario != node) && !in_ship_document {
        *tab = EditorViewType::Events;
        board.frame = Some(node);
    }
    selected.set(Some(node));
}

#[cfg(test)]
mod tests {
    use bevy::{
        prelude::*,
        ui_widgets::{observe, Activate},
    };

    use super::{on_target_row, sync_problem_rows, ProblemList, RowTarget};
    use crate::{
        config::{EditorViewType, SelectedNode},
        history::tests::{first_with, history_app},
        node::{EditContext, NodeId, SectionNode, ShipNode},
        problems::{Problem, ProblemSeverityType, Problems},
        ui::{
            canvas::CanvasBoard,
            row_copy::{CopyListType, CopyRows},
        },
    };

    /// Press a row that names `node`, from wherever the editor stands.
    fn press_row_of(app: &mut App, node: Entity) {
        let entity = app
            .world_mut()
            .spawn((RowTarget(node), observe(on_target_row)))
            .id();
        app.world_mut().trigger(Activate { entity });
        app.world_mut().flush();
    }

    #[test]
    fn a_problem_row_on_a_section_opens_its_ship_and_selects_it() {
        let mut app = history_app();
        app.init_resource::<EditorViewType>();
        app.init_resource::<CanvasBoard>();
        let section = first_with::<SectionNode>(&mut app);
        let ship = app
            .world()
            .get::<ChildOf>(section)
            .expect("a section hangs off its ship")
            .parent();
        assert!(app.world().get::<ShipNode>(ship).is_some());
        let ids = |app: &App, node: Entity| app.world().get::<NodeId>(node).cloned();
        let (ship_id, section_id) = (ids(&app, ship), ids(&app, section));

        press_row_of(&mut app, section);
        app.update();

        let open = app
            .world()
            .resource::<EditContext>()
            .ship()
            .expect("the ship is open");
        assert_eq!(ids(&app, open), ship_id);
        let marked = app
            .world()
            .resource::<SelectedNode>()
            .primary()
            .expect("a part is marked");
        assert_eq!(ids(&app, marked), section_id);
    }

    /// A row drawn before an asset switch outlives the node it names by a frame.
    #[test]
    fn a_row_on_a_dead_node_selects_nothing() {
        let mut app = history_app();
        app.init_resource::<EditorViewType>();
        app.init_resource::<CanvasBoard>();
        let gone = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(gone);

        press_row_of(&mut app, gone);

        assert_eq!(app.world().resource::<SelectedNode>().primary(), None);
        assert_eq!(
            *app.world().resource::<EditorViewType>(),
            EditorViewType::default()
        );
    }

    /// A finding that names no node goes nowhere, so it must not read as a
    /// button.
    #[test]
    fn a_problem_with_no_node_draws_a_row_that_cannot_be_pressed() {
        let mut app = history_app();
        let target = first_with::<ShipNode>(&mut app);
        let finding = |node| Problem {
            severity: ProblemSeverityType::Error,
            message: "a fault".to_string(),
            node,
        };
        let mut problems = Problems::default();
        problems.list = vec![finding(None), finding(Some(target))];
        app.world_mut().insert_resource(problems);
        app.init_resource::<CopyRows>();
        app.world_mut().spawn(ProblemList);

        app.world_mut()
            .run_system_cached(sync_problem_rows)
            .expect("the rows draw");
        app.world_mut().flush();

        let rows: Vec<bool> = app
            .world_mut()
            .query::<(&Name, Has<super::Button>)>()
            .iter(app.world())
            .filter(|(name, _)| name.as_str().starts_with("Problem Row"))
            .map(|(_, button)| button)
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.iter().filter(|button| **button).count(), 1);
    }

    /// A copied finding says where it stands: the ids from its ship down to
    /// the part, as its row shows them.
    #[test]
    fn a_copied_problem_line_carries_its_node_path() {
        let mut app = history_app();
        let section = first_with::<SectionNode>(&mut app);
        let ship = app
            .world()
            .get::<ChildOf>(section)
            .expect("a section hangs off its ship")
            .parent();
        let id = |node: Entity| app.world().get::<NodeId>(node).cloned().expect("an id");
        let path = format!("{}/{}", id(ship).0, id(section).0);
        let mut problems = Problems::default();
        problems.list = vec![
            Problem {
                severity: ProblemSeverityType::Error,
                message: "no thrust".to_string(),
                node: Some(section),
            },
            Problem {
                severity: ProblemSeverityType::Warning,
                message: "the mod holds no content".to_string(),
                node: None,
            },
        ];
        app.world_mut().insert_resource(problems);
        app.init_resource::<CopyRows>();
        app.world_mut().spawn(ProblemList);

        app.world_mut()
            .run_system_cached(sync_problem_rows)
            .expect("the rows draw");

        assert_eq!(
            app.world()
                .resource::<CopyRows>()
                .list(CopyListType::Problems)
                .lines(),
            [
                format!("error: no thrust (at {path})"),
                "warning: the mod holds no content".to_string(),
            ]
        );
    }
}
