//! The editor UI: a wiki-inspired left rail of categories, the ship tools and
//! the placement readout. The theme + shared button widgets live in `nova_ui`;
//! `rail` holds the editor-specific category rows and this module assembles
//! them into the scene.
//!
//! Parts are picked in the `gallery`, which replaced the component drawer that
//! used to sit beside this rail. The drawer listed every prototype as a text
//! card, which cannot say what a part LOOKS like - the one thing a builder
//! needs from a parts list.

pub(crate) mod rail;

use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use nova_assets::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{panel, panel_header, separator, themed_button, ButtonValue, UiSkin},
    theme,
    widget::{checkbox_colors, checkbox_glyph, Selected},
};

use crate::{
    config::{
        AttitudeReadout, EditorKeyLegend, PlacementStatus, PlayButton, SceneList, SceneRow,
        SceneRowPress, SceneUpRow, SectionChoice, SelectedNode, SkinToggleCheckbox, StyleChoice,
        StyleList,
    },
    gallery::{EditorCamera, EditorChrome, GalleryAction},
    node::{context_nodes, EditContext, NodeKind, SectionNodes, ShipNode, ShipNodes},
    placement::{
        continue_to_simulation, create_new_spaceship, create_new_spaceship_with_controller,
    },
    ui::rail::{category_row, coming_soon_category, scene_row, skin_toggle_row, style_row},
    ExampleStates,
};

/// How long after a Scene row is pressed a second press on the same row still
/// reads as a double-click. The OS figure is usually 500ms; 400 is inside every
/// platform default, so a deliberate double never misses.
pub(crate) const DOUBLE_CLICK_WINDOW: f32 = 0.4;

/// The ship the rail is reporting on, or `None` out in the scenario context.
fn edited_ship<'a>(context: &EditContext, ships: &'a Query<&ShipNode>) -> Option<&'a ShipNode> {
    ships.get(context.ship()?).ok()
}

/// Left rail width (px). Kept narrow so the rail stays clear of screen centre
/// on the 1024-wide window, where the editor preview ship projects - a UI panel
/// over that point would block the placement raycast.
const RAIL_W: f32 = 150.0;

/// Register the UI's observers (button colours, selection). The per-state
/// systems and the `SectionChoice` setting observer are wired by the plugin,
/// which owns those types.
pub(crate) fn register(app: &mut App) {
    // The menu and gameplay want the same app-global UI wiring; whoever gets
    // there first adds it.
    if !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
        app.add_plugins(nova_ui::NovaUiPlugin);
    }
}

pub(crate) fn setup_editor_scene(
    mut commands: Commands,
    skin: Res<UiSkin>,
    game_assets: Res<GameAssets>,
    styles: Res<GameStyles>,
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
) {
    let skin = *skin;
    // The rail is built for the ship the editor opens on. With none entered the
    // checkbox starts unclad, which is what a fresh ship is.
    let clad = edited_ship(&context, &q_ships).is_some_and(|ship| ship.skin);
    let looks: Vec<(String, String)> = styles
        .iter()
        .map(|style| (style.id.clone(), style.name.clone()))
        .collect();
    // Key + rim, the same bearings the parts viewer lights its turntable with.
    // The editor used to carry one light shining straight down, which put every
    // vertical face of every part in flat shadow - fine for a ship seen from
    // above, wrong for the gallery, where the part IS the tile.
    commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        Name::new("Editor Key Light"),
        DirectionalLight {
            illuminance: 9_000.0,
            ..default()
        },
        Transform::from_xyz(-6.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        Name::new("Editor Rim Light"),
        DirectionalLight {
            illuminance: 2_500.0,
            ..default()
        },
        Transform::from_xyz(5.0, -2.0, -7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        Name::new("WASD Camera"),
        Camera3d::default(),
        PostProcessingCamera,
        WASDCameraController,
        // The gallery parks this camera on its own stage while it is open, so
        // it needs a handle that does not assume a single Camera3d.
        EditorCamera,
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        // The Kenney parts are flat-Kd meshes; a touch of ambient keeps their
        // shadow side readable without washing out the key light's form.
        AmbientLight {
            color: Color::WHITE,
            brightness: 220.0,
            affects_lightmapped_meshes: true,
        },
        // NOTE: direct SkyboxConfig insert (no PendingSkyboxSwap) is safe
        // because `game_assets.cubemap` already has its Cube view.
        // `prepare_cubemap_view` (nova_assets) sets it at startup, before any
        // camera spawns, so the SkyboxPlugin observer - which only sets the
        // view on its single-layer fallback branch - sees a ready 6-layer + Cube
        // image and just attaches Skybox. Pinned by
        // prepare_cubemap_view_sets_cube_view_on_the_game_assets_cubemap.
        SkyboxConfig {
            cubemap: game_assets.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Root"),
            // The gallery hides the whole rail + drawer while it is up.
            EditorChrome,
            // Pass pointer events through the empty (right) area to the 3D scene,
            // so building is not blocked; the rail/drawer panels still block.
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("Editor Rail"),
                Node {
                    width: px(RAIL_W),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(px(10)),
                    border: UiRect::right(px(theme::BORDER_W)),
                    ..default()
                },
                panel(skin),
            ))
            .with_children(|rail| {
                rail.spawn((
                    Name::new("Editor Title"),
                    Text::new("EDITOR"),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(theme::SCREEN_TEXT),
                    Node {
                        margin: UiRect::bottom(px(8)),
                        ..default()
                    },
                ));

                rail.spawn(panel_header("Categories"));
                rail.spawn((
                    Name::new("Parts Gallery Category"),
                    category_row("Parts"),
                    GalleryAction::Open,
                ));
                rail.spawn(coming_soon_category("Ships", skin));
                rail.spawn(coming_soon_category("Objects", skin));
                rail.spawn(coming_soon_category("Events", skin));
                rail.spawn(coming_soon_category("Objectives", skin));

                // The document, as a list. WIP furniture: it exists so the node
                // tree can be driven and tested by hand until the real
                // hierarchy panel lands. The rows are built by
                // `sync_scene_list`, because what is in the list depends on
                // which node the editor is inside.
                rail.spawn(separator());
                rail.spawn(panel_header("Scene"));
                rail.spawn((Name::new("Scene List"), SceneList, rail_list_node()));

                rail.spawn(separator());
                rail.spawn(panel_header("Ship"));
                // NOTE: names kept exact - the editor / menu_newgame autopilots
                // find these by Name and press them. Display text is free to
                // change.
                rail.spawn((
                    Name::new("Create New Spaceship Button V2"),
                    themed_button("New Ship"),
                    observe(create_new_spaceship_with_controller),
                ));
                rail.spawn((
                    Name::new("Create New Spaceship Button V1"),
                    themed_button("New Hull Ship"),
                    observe(create_new_spaceship),
                ));
                // The attitude readout, under the ship buttons and above the
                // tools: it is a property of the hull being built, and it moves
                // with every part placed. Without it a hull that is too big to
                // turn reads as the game being broken rather than as a hull
                // that wants another computer.
                rail.spawn((
                    Name::new("Attitude Readout"),
                    AttitudeReadout,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                    Node {
                        margin: UiRect::vertical(px(4)),
                        ..default()
                    },
                ));

                rail.spawn(separator());
                rail.spawn(panel_header("Tools"));
                // Deselect the build/delete tool -> select mode
                // (SectionChoice::None), where clicking a section rebinds its
                // key.
                rail.spawn((
                    Name::new("Select Section Button"),
                    themed_button("Select / Rebind"),
                    ButtonValue(SectionChoice::None),
                ));
                rail.spawn((
                    Name::new("Delete Section Button"),
                    themed_button("Delete Section"),
                    ButtonValue(SectionChoice::Delete),
                ));
                // A SETTING among the modes: it arms nothing, it changes what
                // the ship on the stage looks like - and what it looks like
                // when it flies.
                rail.spawn((
                    Name::new("Ship Skin Toggle"),
                    skin_toggle_row(clad, skin),
                    observe(on_skin_toggle),
                ));
                // Under the toggle, and shown only while it is on, because it
                // answers the question the toggle raises: the skin is on, and
                // this is which of the shipped looks it wears. One row per
                // style out of the MERGED content, so a mod's look is listed
                // beside the base ones without the editor knowing any id.
                rail.spawn((Name::new("Ship Look List"), StyleList, rail_list_node()))
                    .with_children(|list| {
                        for (index, (id, name)) in looks.iter().enumerate() {
                            list.spawn((
                                Name::new(format!("Look: {name}")),
                                // The first is what an unset style wears, so it
                                // is the row that starts marked.
                                style_row(id, name, index == 0, skin),
                                observe(on_style_choice),
                            ));
                        }
                    });

                rail.spawn(separator());
                rail.spawn((
                    Name::new("Play Button"),
                    PlayButton,
                    themed_button("Play"),
                    observe(continue_to_simulation),
                ));
            });

            // The placement verdict, along the bottom rather than in the rail:
            // it is about the part under the pointer, so it belongs where the
            // builder is looking. Hidden until there is a placement to report.
            // A full-width row rather than an offset chip: the chip's width
            // follows its text, so centring is the row's job.
            root.spawn((
                Name::new("Placement Status Row"),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(28),
                    left: px(0),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                GlobalZIndex(10),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
                children![(
                    Name::new("Placement Status"),
                    PlacementStatus,
                    Visibility::Hidden,
                    Pickable {
                        should_block_lower: false,
                        is_hoverable: false,
                    },
                    Node {
                        padding: UiRect::axes(px(10), px(4)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    BorderColor::all(theme::RED),
                    BackgroundColor(theme::SPACE),
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme::RED),
                )],
            ));

            // The key legend, bottom-left and out of the build area. Contextual
            // (see `sync_key_legend`): a builder holding a part needs the pose
            // keys, and one in select mode needs to be told the pipette exists.
            root.spawn((
                Name::new("Editor Key Legend"),
                EditorKeyLegend,
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(8),
                    left: px(RAIL_W + 12.0),
                    ..default()
                },
                GlobalZIndex(10),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ));
        });
}

/// Flip the cladding on or off.
///
/// One bool on the build state and nothing else: the build view watches it (see
/// [`crate::skin`]) and Play reads it, so the ship on the stage and the ship
/// that flies cannot disagree about whether it is clad.
pub(crate) fn on_skin_toggle(
    _activate: On<Activate>,
    context: Res<EditContext>,
    mut q_ships: Query<&mut ShipNode>,
) {
    let Some(ship) = context.ship() else {
        return;
    };
    if let Ok(mut ship) = q_ships.get_mut(ship) {
        ship.skin = !ship.skin;
    }
}

/// The look list's own column, so the rows read as one group under the toggle
/// rather than as four more tools.
fn rail_list_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// One row the Scene list wants: the node it points at, and how it reads.
struct WantedRow {
    /// `None` is the ".." row, which is a gesture rather than a node.
    node: Option<Entity>,
    /// A leading glyph, so the kinds are told apart without a second column.
    kind: &'static str,
    label: String,
}

/// What the Scene list is showing, so a frame that changed nothing costs one
/// comparison instead of a respawned list - and, more to the point, so hover
/// and selection survive a frame in which the document did not change.
#[derive(Default)]
pub(crate) struct ShownScene {
    rows: Vec<(Option<Entity>, String)>,
}

/// The rows the current edit context calls for: whatever it contains, with a
/// ".." on top while there is somewhere to go back to.
fn wanted_rows(context: &EditContext, q_ships: &ShipNodes, nodes: &SectionNodes) -> Vec<WantedRow> {
    let mut rows = Vec::new();
    if context.ship().is_some() {
        rows.push(WantedRow {
            node: None,
            kind: "..",
            label: "scenario".to_string(),
        });
    }
    rows.extend(
        context_nodes(context, q_ships, nodes)
            .into_iter()
            .map(|node| WantedRow {
                node: Some(node.entity),
                // The ship the player flies is the one Play hands over, and
                // nothing else on the row says so.
                kind: match node.kind {
                    NodeKind::PlayerShip => ">",
                    NodeKind::AiShip | NodeKind::Section => "-",
                },
                label: node.id.0.clone(),
            }),
    );
    rows
}

/// Rebuild the Scene list when the document or the context changes, and mark
/// the selected row.
///
/// A whole-list rebuild rather than a per-row reconcile: the list is short, it
/// changes only when the builder does something, and the ROW ORDER changes with
/// it - a reconciler that matched rows to nodes would still have to reorder.
/// The compare against [`ShownScene`] is what keeps a static list from
/// respawning every frame and eating its own hover.
pub(crate) fn sync_scene_list(
    mut commands: Commands,
    skin: Res<UiSkin>,
    context: Res<EditContext>,
    mut selected: ResMut<SelectedNode>,
    q_ships: ShipNodes,
    nodes: SectionNodes,
    lists: Query<Entity, With<SceneList>>,
    rows: Query<(Entity, Option<&SceneRow>, Has<Selected>), Or<(With<SceneRow>, With<SceneUpRow>)>>,
    mut shown: Local<ShownScene>,
) {
    let wanted = wanted_rows(&context, &q_ships, &nodes);
    // A selection cannot outlive the context it was made in: the node it names
    // is not in this list, so there is no row left to carry the mark.
    if selected
        .0
        .is_some_and(|node| !wanted.iter().any(|row| row.node == Some(node)))
    {
        selected.0 = None;
    }
    let Ok(list) = lists.single() else {
        return;
    };

    let signature: Vec<(Option<Entity>, String)> = wanted
        .iter()
        .map(|row| (row.node, row.label.clone()))
        .collect();
    if shown.rows != signature {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|list| {
            for row in &wanted {
                // Painted marked from the start rather than waiting for the
                // pass below: these rows do not exist in `rows` until next
                // frame, and a highlight that lags a frame behind the click
                // that made it reads as a dropped input.
                let marked = row.node.is_some() && row.node == selected.0;
                let mut entity = list.spawn((
                    Name::new(format!("Scene Row {}", row.label)),
                    scene_row(row.kind, &row.label, marked, *skin),
                ));
                match row.node {
                    Some(node) => {
                        entity.insert((SceneRow(node), observe(on_scene_row)));
                    }
                    None => {
                        entity.insert((SceneUpRow, observe(on_scene_up)));
                    }
                }
                if marked {
                    entity.insert(Selected);
                }
            }
        });
        shown.rows = signature;
        // The rows just queued are not in `rows` yet, and the ones that are
        // have been queued for despawn. Marking either is a write to an entity
        // that is about to stop existing.
        return;
    }

    for (entity, row, marked) in &rows {
        match (row.is_some_and(|row| Some(row.0) == selected.0), marked) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// Select the node a Scene row names, and ENTER it on a double-click.
///
/// Two gestures on one row because selecting and entering are two different
/// questions - "tell me about this" and "work on this" - and a list that
/// entered on every click could not answer the first at all. Godot's scene tree
/// draws the same line.
pub(crate) fn on_scene_row(
    activate: On<Activate>,
    time: Res<Time>,
    rows: Query<&SceneRow>,
    ships: Query<(), With<ShipNode>>,
    mut last: ResMut<SceneRowPress>,
    mut selected: ResMut<SelectedNode>,
    mut context: ResMut<EditContext>,
) {
    let Ok(SceneRow(node)) = rows.get(activate.entity) else {
        return;
    };
    let now = time.elapsed_secs();
    let doubled = last.row == Some(activate.entity) && now - last.at <= DOUBLE_CLICK_WINDOW;
    *last = SceneRowPress {
        row: Some(activate.entity),
        at: now,
    };

    // Only a ship can be entered today. A double-click on a section is still a
    // select, which is what the section inspector will hang off.
    if doubled && ships.contains(*node) {
        context.enter(*node);
        // The list is about to be rebuilt for the ship's own contents, where
        // the ship itself is not a row.
        selected.0 = None;
        return;
    }
    selected.0 = Some(*node);
}

/// Leave the node the editor is inside.
pub(crate) fn on_scene_up(
    _activate: On<Activate>,
    mut selected: ResMut<SelectedNode>,
    mut context: ResMut<EditContext>,
) {
    context.exit();
    selected.0 = None;
}

/// Disable Play anywhere but the scenario node.
///
/// Play COMPILES the document, which is the scenario node's whole job. Inside a
/// ship "play what" has no answer the builder asked for: the obvious readings -
/// fly this ship alone, or fly the document you cannot currently see - are
/// different scenarios, and picking one silently is worse than refusing.
///
/// Disabled rather than hidden: a control that vanishes reads as a bug, and the
/// greyed button plus the ".." row above it says where Play went.
pub(crate) fn sync_play_button(
    mut commands: Commands,
    context: Res<EditContext>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<PlayButton>>,
) {
    let disabled = context.ship().is_some();
    for (entity, marked) in &buttons {
        match (disabled, marked) {
            (true, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (false, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

/// Pick the look this row names.
///
/// Writes an explicit id rather than a list index: the build state travels out
/// to the scenario and back, and an index into a catalog a mod can grow would
/// not survive that trip meaning the same thing.
pub(crate) fn on_style_choice(
    activate: On<Activate>,
    choices: Query<&StyleChoice>,
    context: Res<EditContext>,
    mut q_ships: Query<&mut ShipNode>,
) {
    let (Ok(choice), Some(ship)) = (choices.get(activate.entity), context.ship()) else {
        return;
    };
    if let Ok(mut ship) = q_ships.get_mut(ship) {
        ship.style = Some(choice.0.clone());
    }
}

/// Show the look list only while the ship is clad, and mark the row the build
/// view is actually dressing plates in.
///
/// The FALLBACK is spelled here as well as in `crate::skin`, because the rail
/// has to mark what is on screen: a ship that has picked no style wears the
/// FIRST one, so that row is the one to highlight.
///
/// Compared before writing rather than gated on a change, for the same reason
/// as [`sync_skin_toggle`]: the rows are spawned on entering the editor, which
/// need not be a frame the style changed on.
pub(crate) fn sync_style_list(
    mut commands: Commands,
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
    styles: Res<GameStyles>,
    mut lists: Query<&mut Node, With<StyleList>>,
    rows: Query<(Entity, &StyleChoice, Has<Selected>)>,
) {
    let ship = edited_ship(&context, &q_ships);
    let display = if ship.is_some_and(|ship| ship.skin) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut lists {
        if node.display != display {
            node.display = display;
        }
    }

    let active = match ship.and_then(|ship| ship.style.as_deref()) {
        Some(id) => styles.get_style(id),
        None => styles.first(),
    }
    .map(|style| style.id.as_str());
    for (entity, choice, selected) in &rows {
        match (active == Some(choice.0.as_str()), selected) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
    }
}

/// Repaint the cladding checkbox for the state it reports, IN PLACE.
///
/// Painted from nova_ui's `checkbox_colors`/`checkbox_glyph` rather than by
/// respawning the widget, so it cannot drift from the `checkbox()` factory the
/// rail built it with - and so the row's hover state survives a toggle.
///
/// Compared before writing rather than gated on a change, for the same reason
/// as [`sync_key_legend`]: the row is spawned on entering the editor, which
/// need not be a frame the toggle changed on.
pub(crate) fn sync_skin_toggle(
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
    skin: Res<UiSkin>,
    boxes: Query<(&Children, &mut BackgroundColor, &mut BorderColor), With<SkinToggleCheckbox>>,
    mut glyphs: Query<(&mut Text, &mut TextColor)>,
) {
    let on = edited_ship(&context, &q_ships).is_some_and(|ship| ship.skin);
    let (fill, edge, glyph_colour) = checkbox_colors(on, *skin);
    let mark = checkbox_glyph(on);
    for (children, mut background, mut border) in boxes {
        if background.0 != fill {
            *background = fill.into();
            border.set_all(edge);
        }
        for &child in children {
            let Ok((mut text, mut colour)) = glyphs.get_mut(child) else {
                continue;
            };
            if text.0 != mark {
                text.0 = mark.to_string();
            }
            if colour.0 != glyph_colour {
                colour.0 = glyph_colour;
            }
        }
    }
}

/// Keep the key legend in step with the armed tool.
///
/// Compared before writing rather than gated on `SectionChoice` changing: the
/// legend is spawned on entering the editor, which is not necessarily a frame
/// the tool changed on, and an empty legend is worse than a redundant compare
/// across three text nodes.
pub(crate) fn sync_key_legend(
    selection: Res<SectionChoice>,
    mut legend: Query<&mut Text, With<EditorKeyLegend>>,
) {
    let line = match *selection {
        SectionChoice::None => {
            "Tab parts   LMB rebind a section   Q pick its part   RMB+drag look   \
             WASD/Space/Shift fly   Ship Skin clads the build, Look dresses it   Esc pause"
        }
        SectionChoice::Section(_) => {
            "LMB place   wheel roll   Ctrl+wheel socket   R roll   F socket   Q pick   \
             Tab parts   Ship Skin reflows as you aim   Esc put down"
        }
        SectionChoice::Delete => "LMB delete   Q pick a part   Tab parts   Esc put down",
    };
    for mut text in &mut legend {
        if text.0 != line {
            text.0 = line.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::ShipStyleConfig;

    use super::*;
    use crate::node::{NextChildOrdinal, NodeId, ScenarioNode, SectionNode, ShipDriver};

    /// A rail with the Scene list on it and the reconciler running, over an
    /// empty document. The tests below fill the document in.
    fn scene_app() -> App {
        let mut app = App::new();
        app.insert_resource(UiSkin::default());
        app.init_resource::<Time>();
        app.init_resource::<SelectedNode>();
        app.init_resource::<SceneRowPress>();
        app.world_mut()
            .spawn((Name::new("Scene List"), SceneList, rail_list_node()));
        app.add_systems(Update, sync_scene_list);
        app
    }

    /// One scenario node, entered.
    fn document(app: &mut App) -> Entity {
        let scenario = app
            .world_mut()
            .spawn((
                ScenarioNode,
                NodeId("scenario".to_string()),
                NextChildOrdinal::default(),
            ))
            .id();
        app.world_mut().insert_resource(EditContext {
            path: vec![scenario],
        });
        scenario
    }

    fn spawn_ship(app: &mut App, scenario: Entity, id: &str, driver: ShipDriver) -> Entity {
        app.world_mut()
            .spawn((
                ShipNode {
                    driver,
                    ..default()
                },
                NodeId(id.to_string()),
                NextChildOrdinal::default(),
                ChildOf(scenario),
            ))
            .id()
    }

    fn section_node(app: &mut App, ship: Entity, id: &str) -> Entity {
        app.world_mut()
            .spawn((
                SectionNode {
                    source: SectionSource::Inline(SectionConfig {
                        base: BaseSectionConfig {
                            id: "hull".to_string(),
                            name: "hull".to_string(),
                            ..default()
                        },
                        kind: SectionKind::Hull(HullSectionConfig::default()),
                    }),
                    modifications: vec![],
                    binds: vec![],
                },
                NodeId(id.to_string()),
                Transform::default(),
                ChildOf(ship),
            ))
            .id()
    }

    /// The Scene rows, in the order the list draws them.
    fn row_names(app: &mut App) -> Vec<String> {
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<SceneList>>()
            .single(app.world())
            .expect("one scene list");
        let children: Vec<Entity> = app
            .world()
            .get::<Children>(list)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        children
            .into_iter()
            .filter_map(|child| app.world().get::<Name>(child))
            .map(|name| name.as_str().replace("Scene Row ", ""))
            .collect()
    }

    fn press(app: &mut App, row: Entity) {
        app.world_mut().trigger(Activate { entity: row });
        app.update();
    }

    /// The row that points at `node`.
    fn row_for(app: &mut App, node: Entity) -> Entity {
        app.world_mut()
            .query::<(Entity, &SceneRow)>()
            .iter(app.world())
            .find(|(_, row)| row.0 == node)
            .map(|(entity, _)| entity)
            .expect("a row for that node")
    }

    fn up_row(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<SceneUpRow>>()
            .single(app.world())
            .expect("an up row")
    }

    /// The list is the CONTEXT's contents: ships at the scenario node, and the
    /// way back out plus that ship's sections once you are inside one.
    #[test]
    fn the_scene_list_follows_the_edit_context() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        section_node(&mut app, first, "hull_1");

        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["ship_1", "ship_2"],
            "the scenario node lists its ships, and no sections"
        );

        app.world_mut().resource_mut::<EditContext>().enter(first);
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "hull_1"],
            "inside a ship: the way out, then that ship's sections"
        );
    }

    /// A ship you are not inside contributes nothing to the list of the ship
    /// you ARE inside - the whole reason the context exists.
    #[test]
    fn a_sibling_ships_sections_are_not_listed() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let second = spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        section_node(&mut app, first, "hull_1");
        section_node(&mut app, second, "turret_1");

        app.world_mut().resource_mut::<EditContext>().enter(second);
        app.update();

        assert_eq!(row_names(&mut app), vec!["scenario", "turret_1"]);
    }

    /// Click SELECTS. Entering is a second gesture, so a builder can ask about
    /// a node without descending into it.
    #[test]
    fn a_single_click_selects_without_entering() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);

        assert_eq!(app.world().resource::<SelectedNode>().0, Some(ship));
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            None,
            "a select is not an enter"
        );
    }

    /// Double-click ENTERS, Godot-style. The second press lands inside
    /// `DOUBLE_CLICK_WINDOW` because the test clock has not moved.
    #[test]
    fn a_double_click_enters_the_ship() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_1");
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);
        press(&mut app, row);

        assert_eq!(app.world().resource::<EditContext>().ship(), Some(ship));
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            None,
            "the ship is not a row of its own contents, so the mark is dropped"
        );
        assert_eq!(row_names(&mut app), vec!["scenario", "hull_1"]);
    }

    /// A slow second click is two selections, not a double-click. Without the
    /// window, any two clicks on one row would eventually descend.
    #[test]
    fn a_slow_second_click_is_not_a_double_click() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(DOUBLE_CLICK_WINDOW + 0.1));
        press(&mut app, row);

        assert_eq!(app.world().resource::<EditContext>().ship(), None);
        assert_eq!(app.world().resource::<SelectedNode>().0, Some(ship));
    }

    /// The ".." row is the way back, and it lands at the scenario node rather
    /// than outside the document.
    #[test]
    fn the_up_row_leaves_the_ship() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let up = up_row(&mut app);
        press(&mut app, up);

        assert_eq!(app.world().resource::<EditContext>().ship(), None);
        assert_eq!(
            app.world().resource::<EditContext>().scenario(),
            Some(scenario)
        );
        assert_eq!(row_names(&mut app), vec!["ship_1"]);
    }

    /// A selection cannot outlive the context it was made in: there is no row
    /// left to carry the mark, and a stale one would point the inspector at a
    /// node the list is not showing.
    #[test]
    fn leaving_a_context_drops_the_selection() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let section = section_node(&mut app, ship, "hull_1");
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let row = row_for(&mut app, section);
        press(&mut app, row);
        assert_eq!(app.world().resource::<SelectedNode>().0, Some(section));

        app.world_mut().resource_mut::<EditContext>().exit();
        app.update();
        assert_eq!(app.world().resource::<SelectedNode>().0, None);
    }

    /// An unchanged document must not respawn its rows: a row that is despawned
    /// and rebuilt every frame loses its hover, and its click never lands.
    #[test]
    fn an_unchanged_document_keeps_the_rows_it_had() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();
        let before: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<SceneRow>>()
            .iter(app.world())
            .collect();

        app.update();
        app.update();

        let after: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<SceneRow>>()
            .iter(app.world())
            .collect();
        assert_eq!(before, after, "the same row entities, not new ones");
    }

    /// Play is the scenario node's gesture. Inside a ship the button is greyed
    /// and the observer refuses, so the keyboard and the autopilot hit the same
    /// rule the pointer does.
    #[test]
    fn play_is_disabled_inside_a_ship() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<ExampleStates>();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let button = app
            .world_mut()
            .spawn((PlayButton, Name::new("Play Button")))
            .id();
        app.add_systems(Update, sync_play_button);
        app.add_observer(continue_to_simulation);

        app.update();
        assert!(
            !app.world().entity(button).contains::<InteractionDisabled>(),
            "at the scenario node Play is live"
        );

        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();
        assert!(
            app.world().entity(button).contains::<InteractionDisabled>(),
            "inside a ship it is greyed"
        );

        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert!(
            !matches!(
                app.world().resource::<NextState<ExampleStates>>(),
                NextState::Pending(ExampleStates::Scenario)
            ),
            "and pressing it anyway does not hand off"
        );
    }

    fn style(id: &str) -> ShipStyleConfig {
        ShipStyleConfig {
            id: id.to_string(),
            name: id.to_string(),
            surfaces: vec![],
            fixtures: vec![],
        }
    }

    /// The rail as `sync_style_list` sees it: the list node and one row per
    /// style, wired to the real observer so a press goes the way a click does.
    fn app(clad: bool) -> App {
        let mut app = App::new();
        app.insert_resource(GameStyles(vec![style("first"), style("second")]));
        let ship = app
            .world_mut()
            .spawn(ShipNode {
                skin: clad,
                ..default()
            })
            .id();
        app.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        app.world_mut()
            .spawn((StyleList, rail_list_node()))
            .with_children(|list| {
                for id in ["first", "second"] {
                    list.spawn((StyleChoice(id.to_string()), observe(on_style_choice)));
                }
            });
        app.add_systems(Update, sync_style_list);
        app
    }

    /// The ship node the rail is reporting on.
    fn ship_node(app: &App) -> &ShipNode {
        let ship = app
            .world()
            .resource::<EditContext>()
            .ship()
            .expect("the test app enters its ship");
        app.world().get::<ShipNode>(ship).expect("the ship node")
    }

    fn set_skin(app: &mut App, on: bool) {
        let ship = app
            .world()
            .resource::<EditContext>()
            .ship()
            .expect("the test app enters its ship");
        app.world_mut()
            .get_mut::<ShipNode>(ship)
            .expect("the ship node")
            .skin = on;
    }

    fn row(app: &mut App, id: &str) -> Entity {
        app.world_mut()
            .query::<(Entity, &StyleChoice)>()
            .iter(app.world())
            .find(|(_, choice)| choice.0 == id)
            .map(|(entity, _)| entity)
            .expect("the row exists")
    }

    fn marked(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query_filtered::<&StyleChoice, With<Selected>>()
            .iter(app.world())
            .map(|choice| choice.0.clone())
            .collect()
    }

    /// A ship that has picked no style wears the FIRST one, so that is the row
    /// the rail marks. Marking nothing would say the build view is showing no
    /// look, which is not what it is showing.
    #[test]
    fn an_unset_style_marks_the_first_look() {
        let mut app = app(true);
        app.update();
        assert_eq!(marked(&mut app), vec!["first".to_string()]);
    }

    /// Pressing a row picks that look, and the mark follows it.
    #[test]
    fn picking_a_look_moves_the_mark_to_it() {
        let mut app = app(true);
        app.update();
        let second = row(&mut app, "second");
        app.world_mut().trigger(Activate { entity: second });
        app.update();

        assert_eq!(ship_node(&app).style, Some("second".to_string()),);
        assert_eq!(marked(&mut app), vec!["second".to_string()]);
    }

    /// A look is a property of a skin that is on. With the cladding off the
    /// list is not a control the builder can act on, so it is not on screen.
    #[test]
    fn the_look_list_is_hidden_while_the_ship_is_bare() {
        let mut app = app(false);
        app.update();
        let display = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Node, With<StyleList>>()
                .single(app.world())
                .expect("the list exists")
                .display
        };
        assert_eq!(display(&mut app), Display::None);

        set_skin(&mut app, true);
        app.update();
        assert_eq!(display(&mut app), Display::Flex);
    }
}
