//! The editor UI: a top bar of context actions over a left rail holding the
//! Scene tree, plus the placement readout. The theme + shared button widgets
//! live in `nova_ui`; `rail` holds the editor-specific rows and this module
//! assembles them into the scene.
//!
//! The layout is split by QUESTION: the top bar answers "where am I and what
//! can I do here" (breadcrumb + per-context actions), the rail answers "what
//! does the document hold" (the tree, and the edited ship's settings). Parts
//! are picked in the `gallery`, which replaced the component drawer that used
//! to sit beside this rail.

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
        AttitudeReadout, ContextBreadcrumb, EditorKeyLegend, PlacementStatus, PlayButton,
        RebindButton, ScenarioActions, SceneList, SceneRow, SectionChoice, SelectedNode,
        ShipActions, ShipSettings, SkinToggleCheckbox, StyleChoice, StyleList,
    },
    gallery::{EditorCamera, EditorChrome, GalleryAction},
    keybind::on_rebind_action,
    node::{
        sections_of, EditContext, NodeId, ScenarioNode, SectionNode, SectionNodes, ShipDriver,
        ShipNode, ShipNodes,
    },
    placement::{continue_to_simulation, create_blank_ship},
    ui::rail::{scene_row, skin_toggle_row, style_row},
    ExampleStates,
};

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
            // The gallery hides the whole top bar + rail while it is up.
            EditorChrome,
            // Pass pointer events through the empty area to the 3D scene, so
            // building is not blocked; the top bar and rail panels still block.
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                ..default()
            },
        ))
        .with_children(|root| {
            // The top bar: where you are, and what you can DO there. Actions
            // live up here rather than in the rail because they are verbs of
            // the current context, and the rail's vertical budget belongs to
            // the tree - the old all-in-one rail ran out of screen the moment
            // the tree grew two rows.
            root.spawn((
                Name::new("Editor Top Bar"),
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(10), px(6)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    column_gap: px(10),
                    ..default()
                },
                panel(skin),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Name::new("Editor Title"),
                    Text::new("EDITOR"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(theme::SCREEN_TEXT),
                ));
                // The menu bar every editor grows into: placeholders for now,
                // greyed rather than absent so the layout does not reflow when
                // they gain their menus. Slots for the same reason Play has
                // one - `themed_button` is percent(100) wide, built for the
                // rail, and a bare one on the bar swallows the row.
                bar.spawn((
                    Name::new("Editor Menu Bar"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|menus| {
                    for menu in ["File", "Edit", "View"] {
                        menus
                            .spawn((Name::new(format!("{menu} Menu Slot")), Node::default()))
                            .with_children(|slot| {
                                slot.spawn((
                                    Name::new(format!("{menu} Menu Button")),
                                    themed_button(menu),
                                    InteractionDisabled,
                                ));
                            });
                    }
                });
                // The breadcrumb doubles as the context readout: the tree marks
                // the entered node, this says the same thing as a sentence -
                // level, path, selection (see `sync_breadcrumb`). Phosphor
                // rather than muted: it is the one line that says what a click
                // will act on, so it must not read as a caption.
                bar.spawn((
                    Name::new("Context Breadcrumb"),
                    ContextBreadcrumb,
                    Text::new(""),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));
                bar.spawn((
                    Name::new("Top Bar Spacer"),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ));
                // NOTE: button names kept stable - the driven walks find these
                // by Name and press them. Display text is free to change.
                bar.spawn((
                    Name::new("Scenario Actions"),
                    ScenarioActions,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        ..default()
                    },
                ))
                .with_children(|actions| {
                    actions.spawn((
                        Name::new("Add Ship Button"),
                        themed_button("Add Ship"),
                        observe(create_blank_ship),
                    ));
                });
                bar.spawn((
                    Name::new("Ship Actions"),
                    ShipActions,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(8),
                        ..default()
                    },
                ))
                .with_children(|actions| {
                    actions.spawn((
                        Name::new("Parts Gallery Category"),
                        themed_button("Parts"),
                        GalleryAction::Open,
                    ));
                    actions.spawn((
                        Name::new("Delete Section Button"),
                        themed_button("Delete"),
                        ButtonValue(SectionChoice::Delete),
                    ));
                    // Acts on the SELECTED section; `sync_rebind_button` greys
                    // it while the selection cannot take a binding.
                    actions.spawn((
                        Name::new("Rebind Section Button"),
                        RebindButton,
                        themed_button("Rebind"),
                        observe(on_rebind_action),
                    ));
                });
                // On the bar in EVERY context, greyed inside a ship (see
                // `sync_play_button`): a control that vanishes reads as a bug,
                // and the greyed button says where Play went. In its own slot
                // because `themed_button` is percent(100) wide - built for the
                // rail - and a bare one on the bar swallows the whole row.
                bar.spawn((
                    Name::new("Play Slot"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|slot| {
                    slot.spawn((
                        Name::new("Play Button"),
                        PlayButton,
                        themed_button("Play"),
                        observe(continue_to_simulation),
                    ));
                });
            });

            // Everything under the bar: the rail on the left, the 3D viewport
            // behind the rest.
            root.spawn((
                Name::new("Editor Content"),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
            ))
            .with_children(|content| {
                content
                    .spawn((
                        Name::new("Editor Rail"),
                        Node {
                            width: px(RAIL_W),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            padding: UiRect::all(px(10)),
                            border: UiRect::right(px(theme::BORDER_W)),
                            ..default()
                        },
                        panel(skin),
                    ))
                    .with_children(|rail| {
                        // The document, as a tree. The rows are built by
                        // `sync_scene_list`, because what is expanded depends
                        // on which node the editor is inside.
                        rail.spawn(panel_header("Scene"));
                        rail.spawn((Name::new("Scene List"), SceneList, rail_list_node()));

                        // Ship settings: properties of the ship being edited,
                        // so the whole block is hidden at the scenario node by
                        // `sync_context_panels`.
                        rail.spawn((
                            Name::new("Ship Settings"),
                            ShipSettings,
                            Node {
                                width: percent(100),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                ..default()
                            },
                        ))
                        .with_children(|settings| {
                            settings.spawn(separator());
                            settings.spawn(panel_header("Ship"));
                            // The attitude readout: a property of the hull
                            // being built, and it moves with every part placed.
                            // Without it a hull that is too big to turn reads
                            // as the game being broken rather than as a hull
                            // that wants another computer.
                            settings.spawn((
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
                            // A SETTING rather than a tool: it arms nothing, it
                            // changes what the ship on the stage looks like -
                            // and what it looks like when it flies.
                            settings.spawn((
                                Name::new("Ship Skin Toggle"),
                                skin_toggle_row(clad, skin),
                                observe(on_skin_toggle),
                            ));
                            // Under the toggle, and shown only while it is on,
                            // because it answers the question the toggle
                            // raises: the skin is on, and this is which of the
                            // shipped looks it wears. One row per style out of
                            // the MERGED content, so a mod's look is listed
                            // beside the base ones without the editor knowing
                            // any id.
                            settings
                                .spawn((Name::new("Ship Look List"), StyleList, rail_list_node()))
                                .with_children(|list| {
                                    for (index, (id, name)) in looks.iter().enumerate() {
                                        list.spawn((
                                            Name::new(format!("Look: {name}")),
                                            // The first is what an unset style
                                            // wears, so it starts marked.
                                            style_row(id, name, index == 0, skin),
                                            observe(on_style_choice),
                                        ));
                                    }
                                });
                        });
                    });
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

/// One row the Scene tree wants: the node it points at, and how it reads.
struct WantedRow {
    node: Entity,
    /// The tree furniture in front of the label: ASCII connectors for the
    /// depth, then a glyph for what the node is. `@` marks the node the editor
    /// is inside, `>` the ship Play hands to the player, `-` a design beside
    /// it, and a section wears its kind.
    lead: String,
    label: String,
}

/// What the Scene tree is showing, so a frame that changed nothing costs one
/// comparison instead of a respawned list - and, more to the point, so hover
/// and selection survive a frame in which the document did not change.
#[derive(Default)]
pub(crate) struct ShownScene {
    rows: Vec<(Entity, String, String)>,
}

/// The glyph a section row wears: its kind, so the tree says what a ship is
/// made of without a second column.
fn section_glyph(section: &SectionNode, catalog: Option<&GameSections>) -> &'static str {
    match section.resolve(catalog).map(|config| &config.kind) {
        Some(SectionKind::Hull(_)) => "=",
        Some(SectionKind::Controller(_)) => "o",
        Some(SectionKind::Thruster(_)) => "^",
        Some(SectionKind::Turret(_)) => "+",
        Some(SectionKind::Torpedo(_)) => "!",
        None => "?",
    }
}

/// The whole document as a tree: the scenario root, every ship under it, and
/// the ENTERED ship's sections nested under that ship. Sibling ships stay
/// collapsed - their sections are not what the builder is working on, and a
/// 150px rail cannot hold three ships' worth of rows.
fn wanted_rows(
    context: &EditContext,
    q_scenarios: &Query<&NodeId, With<ScenarioNode>>,
    q_ships: &ShipNodes,
    nodes: &SectionNodes,
    catalog: Option<&GameSections>,
) -> Vec<WantedRow> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    let Ok(root_id) = q_scenarios.get(scenario) else {
        return Vec::new();
    };
    let mut rows = vec![WantedRow {
        node: scenario,
        lead: "*".to_string(),
        label: root_id.0.clone(),
    }];

    let entered = context.ship();
    let mut ships: Vec<_> = q_ships
        .iter()
        .filter(|(_, owner, ..)| owner.parent() == scenario)
        .collect();
    ships.sort_unstable_by(|a, b| a.2.cmp(b.2));
    for (ship, _, id, node) in ships {
        let glyph = if entered == Some(ship) {
            "@"
        } else {
            match node.driver {
                ShipDriver::Player => ">",
                ShipDriver::Ai => "-",
            }
        };
        rows.push(WantedRow {
            node: ship,
            lead: format!("|- {glyph}"),
            label: id.0.clone(),
        });
        if entered != Some(ship) {
            continue;
        }
        for (section, id, node, _) in sections_of(ship, nodes) {
            rows.push(WantedRow {
                node: section,
                lead: format!("|  |- {}", section_glyph(node, catalog)),
                label: id.0.clone(),
            });
        }
    }
    rows
}

/// Rebuild the Scene tree when the document or the context changes, and mark
/// the selected row.
///
/// A whole-list rebuild rather than a per-row reconcile: the tree is short, it
/// changes only when the builder does something, and the ROW ORDER changes with
/// it - a reconciler that matched rows to nodes would still have to reorder.
/// The compare against [`ShownScene`] is what keeps a static tree from
/// respawning every frame and eating its own hover.
pub(crate) fn sync_scene_list(
    mut commands: Commands,
    skin: Res<UiSkin>,
    context: Res<EditContext>,
    catalog: Option<Res<GameSections>>,
    mut selected: ResMut<SelectedNode>,
    q_scenarios: Query<&NodeId, With<ScenarioNode>>,
    q_ships: ShipNodes,
    nodes: SectionNodes,
    lists: Query<Entity, With<SceneList>>,
    fresh: Query<(), Added<SceneList>>,
    rows: Query<(Entity, &SceneRow, Has<Selected>)>,
    mut shown: Local<ShownScene>,
) {
    // A fresh list holds no rows whatever this Local remembers: the list dies
    // with the editor scene (DespawnOnExit) while a `Local` survives the
    // state round-trip, so a Play-and-return handed an unchanged document to
    // an empty list the signature compare would never refill.
    if !fresh.is_empty() {
        shown.rows.clear();
    }
    let wanted = wanted_rows(&context, &q_scenarios, &q_ships, &nodes, catalog.as_deref());
    // A selection cannot outlive its row: a section of a ship that was left is
    // not in the tree, so there is nothing left to carry the mark.
    if selected
        .0
        .is_some_and(|node| !wanted.iter().any(|row| row.node == node))
    {
        selected.0 = None;
    }
    let Ok(list) = lists.single() else {
        return;
    };

    let signature: Vec<(Entity, String, String)> = wanted
        .iter()
        .map(|row| (row.node, row.lead.clone(), row.label.clone()))
        .collect();
    if shown.rows != signature {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|list| {
            for row in &wanted {
                // Painted marked from the start rather than waiting for the
                // pass below: these rows do not exist in `rows` until next
                // frame, and a highlight that lags a frame behind the click
                // that made it reads as a dropped input.
                let marked = Some(row.node) == selected.0;
                let mut entity = list.spawn((
                    Name::new(format!("Scene Row {}", row.label)),
                    scene_row(&row.lead, &row.label, marked, *skin),
                    SceneRow(row.node),
                    observe(on_scene_row),
                ));
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
        match (Some(row.0) == selected.0, marked) {
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

/// One click, and the row's kind says what it means: a ship row ENTERS the
/// ship, the scenario root leaves it, and a section row SELECTS - the thing an
/// inspector and the Rebind action hang off.
///
/// A container is entered and a leaf is selected, so one gesture covers both
/// questions without a double-click - which the owner tried and read as "the
/// first click did nothing".
pub(crate) fn on_scene_row(
    activate: On<Activate>,
    rows: Query<&SceneRow>,
    ships: Query<(), With<ShipNode>>,
    scenarios: Query<(), With<ScenarioNode>>,
    mut selected: ResMut<SelectedNode>,
    mut context: ResMut<EditContext>,
) {
    let Ok(SceneRow(node)) = rows.get(activate.entity) else {
        return;
    };
    if scenarios.contains(*node) {
        context.to_root();
        selected.0 = None;
        return;
    }
    if ships.contains(*node) {
        context.enter(*node);
        selected.0 = None;
        return;
    }
    selected.0 = Some(*node);
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

/// Show each context its own verbs: the scenario node's action group at the
/// scenario node, the ship's action group and settings block inside a ship.
///
/// Hidden rather than disabled, unlike Play: a greyed Add Ship inside a ship
/// would say "this exists here and is refused", and it does not exist there -
/// adding a ship is a thing the SCENARIO does.
pub(crate) fn sync_context_panels(
    context: Res<EditContext>,
    mut panels: Query<
        (&mut Node, Has<ScenarioActions>),
        Or<(With<ScenarioActions>, With<ShipActions>, With<ShipSettings>)>,
    >,
) {
    let inside = context.ship().is_some();
    for (mut node, scenario_only) in &mut panels {
        let display = if scenario_only != inside {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// Write the top bar's context readout: WHAT is being edited (the level, in
/// capitals), the path to it in the document's own ids, and the selection.
///
/// The level leads because it was the missing feedback: the bare path
/// "scenario / ship_1" never said whether a click would select, enter or
/// place. The same fact the tree's `@` mark shows, said as a sentence.
pub(crate) fn sync_breadcrumb(
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    ids: Query<&NodeId>,
    mut crumbs: Query<&mut Text, With<ContextBreadcrumb>>,
) {
    let path = context
        .path
        .iter()
        .filter_map(|node| ids.get(*node).ok())
        .map(|id| id.0.as_str())
        .collect::<Vec<_>>()
        .join(" / ");
    let level = match (context.scenario(), context.ship()) {
        (None, _) => "",
        (Some(_), None) => "[SCENARIO] ",
        (Some(_), Some(_)) => "[SHIP] ",
    };
    let mut wanted = format!("{level}{path}");
    if let Some(id) = selected.0.and_then(|node| ids.get(node).ok()) {
        wanted.push_str(&format!("   selected {}", id.0));
    }
    for mut text in &mut crumbs {
        if text.0 != wanted {
            text.0 = wanted.clone();
        }
    }
}

/// Grey the Rebind action unless the selection can take a binding: a bindable
/// section of the edited ship. The same guards `on_rebind_action` enforces,
/// painted, so the button never invites a press that does nothing.
pub(crate) fn sync_rebind_button(
    mut commands: Commands,
    catalog: Option<Res<GameSections>>,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    q_sections: Query<(&SectionNode, &ChildOf)>,
    buttons: Query<(Entity, Has<InteractionDisabled>), With<RebindButton>>,
) {
    let armable = selected.0.is_some_and(|node| {
        q_sections.get(node).is_ok_and(|(section, owner)| {
            context.ship() == Some(owner.parent()) && section.bindable(catalog.as_deref())
        })
    });
    for (entity, marked) in &buttons {
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

/// Keep the key legend in step with the armed tool AND the edit context.
///
/// Keyed on both because the same keys mean different things per level: at the
/// scenario node there are no parts to arm and Escape falls through to pause,
/// while inside a ship Tab browses parts and Escape backs out one rung. The
/// old single line told a builder at the scenario node about a pipette that
/// disarms instantly, and told one inside a ship that Escape would pause.
///
/// Compared before writing rather than gated on a change: the legend is
/// spawned on entering the editor, which is not necessarily a frame the tool
/// or the context changed on.
pub(crate) fn sync_key_legend(
    selection: Res<SectionChoice>,
    context: Res<EditContext>,
    mut legend: Query<&mut Text, With<EditorKeyLegend>>,
) {
    let line = match (&*selection, context.ship().is_some()) {
        (SectionChoice::None, false) => {
            "LMB select a ship   LMB+drag move it   RMB+drag look   \
             WASD/Space/Shift fly   Play flies the scenario   Esc pause"
        }
        (SectionChoice::None, true) => {
            "Tab parts   LMB select   Q pick its part   RMB+drag look   \
             WASD/Space/Shift fly   Rebind acts on the selection   Esc leave the ship"
        }
        (SectionChoice::Section(_), _) => {
            "LMB place   wheel roll   Ctrl+wheel socket   R roll   F socket   Q pick   \
             Tab parts   Ship Skin reflows as you aim   Esc put down"
        }
        (SectionChoice::Delete, _) => "LMB delete   Q pick a part   Tab parts   Esc put down",
    };
    for mut text in &mut legend {
        if text.0 != line {
            text.0 = line.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::ShipStyleConfig;

    use super::*;
    use crate::node::NextChildOrdinal;

    /// A rail with the Scene tree on it and the reconciler running, over an
    /// empty document. The tests below fill the document in.
    fn scene_app() -> App {
        let mut app = App::new();
        app.insert_resource(UiSkin::default());
        app.init_resource::<SelectedNode>();
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

    /// The lead texts of the rows, in draw order - the tree furniture the
    /// glyph assertions read.
    fn row_leads(app: &mut App) -> Vec<String> {
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<SceneList>>()
            .single(app.world())
            .expect("one scene list");
        let rows: Vec<Entity> = app
            .world()
            .get::<Children>(list)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        rows.into_iter()
            .filter_map(|row| {
                let first = app.world().get::<Children>(row)?.iter().next()?;
                Some(app.world().get::<Text>(first)?.0.clone())
            })
            .collect()
    }

    /// The tree is the DOCUMENT: the scenario root, every ship, and the
    /// entered ship's sections nested under it. Sibling ships stay collapsed.
    #[test]
    fn the_scene_tree_opens_the_entered_branch_only() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let second = spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        section_node(&mut app, first, "hull_1");
        section_node(&mut app, second, "turret_1");

        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "ship_2"],
            "at the scenario node every branch is collapsed"
        );

        app.world_mut().resource_mut::<EditContext>().enter(first);
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "hull_1", "ship_2"],
            "the entered ship opens; its sibling does not"
        );

        app.world_mut().resource_mut::<EditContext>().enter(second);
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "ship_2", "turret_1"],
            "entering the sibling moves the open branch"
        );
    }

    /// The lead column is the tree's whole vocabulary: `*` the root, `>` the
    /// ship Play flies, `-` a design beside it, `@` where the editor is, and a
    /// section wears its kind.
    #[test]
    fn the_lead_glyphs_say_who_is_who() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        section_node(&mut app, first, "hull_1");

        app.update();
        assert_eq!(row_leads(&mut app), vec!["*", "|- >", "|- -"]);

        app.world_mut().resource_mut::<EditContext>().enter(first);
        app.update();
        assert_eq!(
            row_leads(&mut app),
            vec!["*", "|- @", "|  |- =", "|- -"],
            "the entered ship is marked, and its hull section shows its kind"
        );
    }

    /// One click on a ship row ENTERS it - the owner tried double-click and
    /// read the first click as a dropped input.
    #[test]
    fn a_single_click_on_a_ship_row_enters_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_1");
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);

        assert_eq!(app.world().resource::<EditContext>().ship(), Some(ship));
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            None,
            "a container is entered, not selected"
        );
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "hull_1"],
            "and its branch is open"
        );
    }

    /// A section row SELECTS: a leaf has nothing to enter yet, and the mark is
    /// what an inspector and the Rebind action act on.
    #[test]
    fn a_section_row_selects_without_moving_the_context() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let section = section_node(&mut app, ship, "hull_1");
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let row = row_for(&mut app, section);
        press(&mut app, row);

        assert_eq!(app.world().resource::<SelectedNode>().0, Some(section));
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            Some(ship),
            "selecting a section does not move the context"
        );
    }

    /// The root row is the way back, and it lands at the scenario node rather
    /// than outside the document.
    #[test]
    fn the_root_row_leaves_the_ship() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let root = row_for(&mut app, scenario);
        press(&mut app, root);

        assert_eq!(app.world().resource::<EditContext>().ship(), None);
        assert_eq!(
            app.world().resource::<EditContext>().scenario(),
            Some(scenario)
        );
        assert_eq!(row_names(&mut app), vec!["scenario", "ship_1"]);
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

    /// A RESPAWNED list is refilled for the same document. The list dies with
    /// the editor scene while both the document and the reconciler's `Local`
    /// survive the Play round-trip, so without the `Added` override the
    /// signature compare saw "unchanged" and the fresh list stayed empty -
    /// and with world-click-enter gone, an empty tree left no door into any
    /// ship.
    #[test]
    fn a_respawned_scene_list_is_refilled_for_the_same_document() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();
        assert_eq!(row_names(&mut app), vec!["scenario", "ship_1"]);

        // What leaving for Play and coming back does to the UI: the old list
        // (rows included) is despawned and a fresh empty one is spawned, while
        // the document and the system's Local both persist.
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<SceneList>>()
            .single(app.world())
            .expect("one scene list");
        app.world_mut().entity_mut(list).despawn();
        app.world_mut()
            .spawn((Name::new("Scene List"), SceneList, rail_list_node()));

        app.update();
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1"],
            "the unchanged document must refill the fresh list"
        );
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

    /// Each context shows its own verbs: Add Ship at the scenario node, the
    /// ship actions and settings inside a ship - never both.
    #[test]
    fn each_context_shows_its_own_actions() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<EditContext>();
        let scenario = world.spawn(ScenarioNode).id();
        let ship = world.spawn(ShipNode::default()).id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        let add = world.spawn((ScenarioActions, Node::default())).id();
        let tools = world.spawn((ShipActions, Node::default())).id();
        let settings = world.spawn((ShipSettings, Node::default())).id();

        let display = |world: &World, entity: Entity| world.get::<Node>(entity).unwrap().display;

        world.run_system_once(sync_context_panels).unwrap();
        assert_eq!(display(&world, add), Display::Flex);
        assert_eq!(display(&world, tools), Display::None);
        assert_eq!(display(&world, settings), Display::None);

        world.resource_mut::<EditContext>().enter(ship);
        world.run_system_once(sync_context_panels).unwrap();
        assert_eq!(display(&world, add), Display::None);
        assert_eq!(display(&world, tools), Display::Flex);
        assert_eq!(display(&world, settings), Display::Flex);
    }

    /// The readout says WHAT is being edited before it says where: the level
    /// in capitals, the path in the document's own ids, and the selection.
    /// The bare path never answered "will this click select, enter or place".
    #[test]
    fn the_breadcrumb_names_the_level_the_path_and_the_selection() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<EditContext>();
        world.init_resource::<SelectedNode>();
        let scenario = world
            .spawn((ScenarioNode, NodeId("scenario".to_string())))
            .id();
        let ship = world
            .spawn((ShipNode::default(), NodeId("ship_1".to_string())))
            .id();
        let crumb = world.spawn((ContextBreadcrumb, Text::new(""))).id();
        world.resource_mut::<EditContext>().path = vec![scenario];

        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(world.get::<Text>(crumb).unwrap().0, "[SCENARIO] scenario");

        world.resource_mut::<EditContext>().enter(ship);
        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(
            world.get::<Text>(crumb).unwrap().0,
            "[SHIP] scenario / ship_1"
        );

        world.resource_mut::<SelectedNode>().0 = Some(ship);
        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(
            world.get::<Text>(crumb).unwrap().0,
            "[SHIP] scenario / ship_1   selected ship_1"
        );
    }

    /// Rebind is greyed until the selection can actually take a binding - the
    /// same guards the action enforces, painted.
    #[test]
    fn rebind_is_greyed_until_a_bindable_section_is_selected() {
        use bevy::ecs::system::RunSystemOnce;
        use nova_ship::prelude::{SectionConfig, SectionKind, TurretSectionConfig};

        let section = |kind: SectionKind| SectionNode {
            source: SectionSource::Inline(SectionConfig {
                base: BaseSectionConfig {
                    id: "part".to_string(),
                    name: "part".to_string(),
                    ..default()
                },
                kind,
            }),
            modifications: vec![],
            binds: vec![],
        };

        let mut world = World::new();
        world.init_resource::<SelectedNode>();
        let ship = world.spawn(ShipNode::default()).id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        let hull = world
            .spawn((section(SectionKind::Hull(default())), ChildOf(ship)))
            .id();
        let turret = world
            .spawn((
                section(SectionKind::Turret(TurretSectionConfig::default())),
                ChildOf(ship),
            ))
            .id();
        let button = world.spawn(RebindButton).id();

        let disabled = |world: &World| world.entity(button).contains::<InteractionDisabled>();

        world.run_system_once(sync_rebind_button).unwrap();
        assert!(disabled(&world), "nothing selected, nothing to rebind");

        world.resource_mut::<SelectedNode>().0 = Some(hull);
        world.run_system_once(sync_rebind_button).unwrap();
        assert!(disabled(&world), "a hull takes no binding");

        world.resource_mut::<SelectedNode>().0 = Some(turret);
        world.run_system_once(sync_rebind_button).unwrap();
        assert!(!disabled(&world), "a turret of the edited ship can rebind");
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
