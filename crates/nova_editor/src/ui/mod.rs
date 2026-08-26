//! The editor UI: a top bar of context actions over a left rail holding the
//! Scene tree, plus the placement readout. The theme + shared button widgets
//! live in `nova_ui`; `rail` holds the editor-specific rows and this module
//! assembles them into the scene.
//!
//! The layout is split by QUESTION: the top bar answers "where am I and what
//! can I do here" (breadcrumb + per-context actions), the rail answers "what
//! does the document hold" (the tree, and the edited ship's settings), and the
//! `inspector` on the right answers "what is THIS one". Parts are picked in the
//! `gallery`, which replaced the component drawer that used to sit beside this
//! rail.

pub(crate) mod callout;
pub(crate) mod inspector;
pub(crate) mod menu;
pub(crate) mod rail;
pub(crate) mod window;

use bevy::{
    ecs::{
        relationship::{RelatedSpawner, RelatedSpawnerCommands},
        spawn::SpawnWith,
    },
    picking::{hover::Hovered, mesh_picking::MeshPickingCamera},
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate},
};
use nova_assets::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{key_chip, panel, panel_header, separator, themed_button, ButtonLabel, UiSkin},
    theme,
    widget::{checkbox_colors, checkbox_glyph, Selected},
};

use crate::{
    bundle::ask_to_save,
    config::{
        ContextBreadcrumb, EditorKeyLegend, EditorOverlays, EditorStatus, LastClick,
        PlacementStatus, PlayButton, RebindButton, SceneList, SceneRow, SectionChoice,
        SelectedNode, ShipReadout, ShipReadoutNote, ShipSettings, SkinToggleCheckbox, StyleChoice,
        StyleList, StyleSwatch,
    },
    frame::{ask_for, on_frame_selection, FrameRequest, FrameSelectionItem},
    gallery::{EditorCamera, EditorChrome, GalleryAction, GalleryCategory},
    glyph::{
        category_mark, choice_mark, object_mark, section_mark, ship_mark, INSIDE, SCENARIO, SHIP_AI,
    },
    keybind::{on_rebind_action, EditorRebind},
    node::{
        id_order, objects_of, sections_of, split_ordinal, EditContext, NodeId, ObjectChoice,
        ObjectNode, ObjectNodes, ScenarioNode, SectionNode, SectionNodes, ShipNode, ShipNodes,
    },
    placement::{
        continue_to_simulation, create_blank_ship, create_scenario_object, cycle_armed_socket,
        deletable, delete_selected_node, put_armed_part_down, roll_armed_part,
    },
    ui::{
        callout::placement_callout,
        inspector::{inspector_panel, PANEL_W as INSPECTOR_W},
        menu::{
            menu_bar_slot, menu_dropdown_node, menu_item_row, menu_scrim, menu_z, on_menu_button,
            on_menu_scrim, toggle_key_legend, toggle_link_points, toggle_object_volumes,
            toggle_world_grid, ArmedMenuItem, MenuDeleteItem, MenuDropdown, MenuId, MenuLead,
            MenuTail, OpenMenu, ScenarioMenuItem, ShipMenuItem, ViewToggle,
        },
        rail::{scene_row, scene_tooltip, skin_toggle_row, style_row, SceneRowHint, SceneRowTrash},
        window::{window_layer, DestructiveVerb},
    },
    ExampleStates,
};

/// Fill one dropdown.
///
/// The whole menu bar in one place, so what the editor can do reads as a list
/// rather than as four `with_children` blocks buried in the bar's layout.
///
/// GREYED, NOT ABSENT, for the items that are not built: Save As needs a name
/// field nothing offers yet, and Undo and Redo are nobody's. A menu that only
/// lists what already works cannot say what the editor is going to be.
fn build_menu(items: &mut RelatedSpawnerCommands<ChildOf>, menu: MenuId, skin: UiSkin) {
    match menu {
        MenuId::File => {
            // The three rows that can lose work do not DO anything: they put
            // the question up (see `crate::ui::window::DestructiveVerb`), and
            // the window's own button carries the verb.
            items.spawn((
                Name::new("New Scenario Item"),
                DestructiveVerb::NewScenario,
                menu_item_row("New Scenario", MenuLead::None, MenuTail::None, skin),
            ));
            items.spawn((
                Name::new("Save Item"),
                menu_item_row("Save", MenuLead::None, MenuTail::Key("Ctrl+S"), skin),
                observe(ask_to_save),
            ));
            items.spawn((
                Name::new("Open Item"),
                DestructiveVerb::Open,
                menu_item_row("Open", MenuLead::None, MenuTail::None, skin),
            ));
            // Still greyed: Save As needs a name to save under and a place to
            // type it, and there is one save slot until it has both. It says
            // `soon` rather than nothing - a greyed row with a blank tail reads
            // as "you cannot save", which is the opposite of what it means.
            items.spawn((
                Name::new("Save As... Item"),
                menu_item_row("Save As...", MenuLead::None, MenuTail::Word("soon"), skin),
                InteractionDisabled,
            ));
            items.spawn(separator());
            items.spawn((
                Name::new("Back To Main Menu Item"),
                DestructiveVerb::MainMenu,
                menu_item_row("Back to Main Menu", MenuLead::None, MenuTail::None, skin),
            ));
        }
        MenuId::Edit => {
            for label in ["Undo", "Redo"] {
                items.spawn((
                    Name::new(format!("{label} Item")),
                    menu_item_row(label, MenuLead::None, MenuTail::Word("soon"), skin),
                    InteractionDisabled,
                ));
            }
            items.spawn(separator());
            items.spawn((
                Name::new("Delete Item"),
                MenuDeleteItem,
                menu_item_row("Delete", MenuLead::None, MenuTail::Key("Del"), skin),
                observe(delete_selected_node),
            ));
        }
        MenuId::View => {
            items.spawn((
                Name::new("Key Legend Item"),
                ViewToggle::KeyLegend,
                menu_item_row("Key Legend", MenuLead::Toggle, MenuTail::None, skin),
                observe(toggle_key_legend),
            ));
            items.spawn((
                Name::new("Link Points Item"),
                ViewToggle::LinkPoints,
                menu_item_row("Link Points", MenuLead::Toggle, MenuTail::None, skin),
                observe(toggle_link_points),
            ));
            items.spawn((
                Name::new("World Grid Item"),
                ViewToggle::WorldGrid,
                menu_item_row("World Grid", MenuLead::Toggle, MenuTail::None, skin),
                observe(toggle_world_grid),
            ));
            items.spawn((
                Name::new("Object Volumes Item"),
                ViewToggle::ObjectVolumes,
                menu_item_row("Object Volumes", MenuLead::Toggle, MenuTail::None, skin),
                observe(toggle_object_volumes),
            ));
            items.spawn(separator());
            items.spawn((
                Name::new("Frame Selection Item"),
                FrameSelectionItem,
                menu_item_row("Frame Selection", MenuLead::None, MenuTail::Key("F"), skin),
                observe(on_frame_selection),
            ));
        }
        MenuId::Ship => {
            // The three verbs that used to sit in the top right. They belong
            // to the ship you are inside, not to the screen, so they moved to
            // the one menu that says so.
            items.spawn((
                Name::new("Parts Item"),
                ShipMenuItem,
                GalleryAction::Open,
                menu_item_row("Parts...", MenuLead::None, MenuTail::Key("Tab"), skin),
            ));
            items.spawn(separator());
            // The pose verbs. They live only in a legend View can switch off,
            // and R/F/wheel are named nowhere else in the editor - so they get
            // rows, greyed until there is a part in hand for them to turn.
            items.spawn((
                Name::new("Roll The Part Item"),
                ArmedMenuItem,
                menu_item_row(
                    "Roll the Part (or wheel)",
                    MenuLead::None,
                    MenuTail::Key("R"),
                    skin,
                ),
                observe(roll_armed_part),
            ));
            items.spawn((
                Name::new("Cycle The Socket Item"),
                ArmedMenuItem,
                menu_item_row(
                    "Cycle the Socket (or Ctrl+wheel)",
                    MenuLead::None,
                    MenuTail::Key("F"),
                    skin,
                ),
                observe(cycle_armed_socket),
            ));
            items.spawn((
                Name::new("Put The Part Down Item"),
                ArmedMenuItem,
                menu_item_row(
                    "Put the Part Down",
                    MenuLead::None,
                    MenuTail::Key("Esc"),
                    skin,
                ),
                observe(put_armed_part_down),
            ));
            items.spawn(separator());
            items.spawn((
                Name::new("Rebind Key Item"),
                RebindButton,
                menu_item_row("Rebind Key...", MenuLead::None, MenuTail::None, skin),
                observe(on_rebind_action),
            ));
        }
        MenuId::Add => {
            // A ship and a rock are both "one more node under the scenario",
            // so they are one menu: the rail used to answer Add Ship on the
            // top bar and Add Object in a block halfway down the left, which
            // made two names for one question.
            //
            // Add means "one more node HERE", and here changes. The world
            // palette is live at the scenario node; the parts palette is live
            // inside a ship. Both blocks stay on show either way, greyed,
            // because the menu is also where a builder reads what entering a
            // ship changes.
            items.spawn((
                Name::new("Add Ship Button"),
                ScenarioMenuItem,
                // The OUTLINE ship: the row adds a design beside the one already
                // there. The first ship of an empty document becomes the
                // player's and the tree fills its mark in.
                menu_item_row("New Ship", MenuLead::Glyph(SHIP_AI), MenuTail::None, skin),
                observe(create_blank_ship),
            ));
            items.spawn(separator());
            for choice in ObjectChoice::ALL {
                items.spawn((
                    Name::new(format!("Add {}", choice.label())),
                    choice,
                    ScenarioMenuItem,
                    menu_item_row(
                        choice.label(),
                        MenuLead::Glyph(choice_mark(choice)),
                        MenuTail::None,
                        skin,
                    ),
                    observe(create_scenario_object),
                ));
            }
            items.spawn(separator());
            for category in GalleryCategory::ROW {
                if category == GalleryCategory::All {
                    continue;
                }
                items.spawn((
                    Name::new(format!("Add {} Item", category.label())),
                    ShipMenuItem,
                    GalleryAction::Browse(category),
                    menu_item_row(
                        &format!("{}...", category.label()),
                        MenuLead::Glyph(category_mark(category)),
                        MenuTail::None,
                        skin,
                    ),
                ));
            }
        }
    }
}

/// The ship the rail is reporting on, or `None` out in the scenario context.
fn edited_ship<'a>(context: &EditContext, ships: &'a Query<&ShipNode>) -> Option<&'a ShipNode> {
    ships.get(context.ship()?).ok()
}

/// What the Play button says when it can be pressed, and when it cannot.
///
/// The greyed form names the way out rather than only the refusal: Play
/// compiles the WHOLE document, and the one thing that stops it is standing
/// inside a ship.
const PLAY_LABEL: &str = "Play";
const PLAY_BLOCKED: &str = "Play (leave the ship)";

/// Left rail width (px). Kept narrow so the rail stays clear of screen centre
/// on the 1024-wide window, where the editor preview ship projects - a UI panel
/// over that point would block the placement raycast. A wider rail buys the
/// tree a few characters and costs the walk its aim at the ship: the rows buy
/// their width back from the type and the indent instead (see [`scene_row`]).
const RAIL_W: f32 = 150.0;

/// How much of its own colour a style row keeps while the skin is off. Enough
/// to read as the same list, not enough to be mistaken for the live one.
const GREYED_STYLE_ALPHA: f32 = 0.3;

/// Readout type size (px). The block lines its values up on a monospace column,
/// so it must not WRAP: at 11px the longest line (`Turn   5.23 rad/s2`) is
/// 119px inside a 130px rail, and a point more would fold it in half.
const READOUT_TEXT: f32 = 11.0;

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
    // checkbox starts bare, which is what a fresh ship is.
    let skinned = edited_ship(&context, &q_ships).is_some_and(|ship| ship.skin);
    let listed = listed_styles(&styles);
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
        // Opt in to mesh picking, which the transform gizmo rides on (see
        // `crate::gizmo`). The stage itself is picked through avian's
        // colliders, so this camera answers the pointer twice - once per
        // backend - and the nearer hit wins.
        MeshPickingCamera,
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
                // THREE COLUMNS, not a row with a spacer: Play sits in the
                // middle of the SCREEN the way it does in Godot and Unity, and
                // it only stays there if the two sides are equal-weight boxes.
                // A spacer would centre it between the menus and the actions
                // instead, which moves every time the breadcrumb grows a word.
                bar.spawn((
                    Name::new("Top Bar Left"),
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        // Without this a long breadcrumb wins the row and
                        // shoulders Play off centre.
                        min_width: px(0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        ..default()
                    },
                ))
                .with_children(|left| {
                    left.spawn((
                        Name::new("Editor Title"),
                        Text::new("EDITOR"),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                    ));
                    // The menu bar. Every entry drops a real list - the greyed
                    // File/Edit/View placeholders that used to sit here said
                    // the editor had menus while answering no press at all.
                    left.spawn((
                        Name::new("Editor Menu Bar"),
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(6),
                            ..default()
                        },
                    ))
                    .with_children(|menus| {
                        for menu in [
                            MenuId::File,
                            MenuId::Edit,
                            MenuId::View,
                            MenuId::Add,
                            MenuId::Ship,
                        ] {
                            menus
                                .spawn((
                                    Name::new(format!("{} Menu Slot", menu.label())),
                                    menu_bar_slot(),
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        Name::new(format!("{} Menu Button", menu.label())),
                                        menu,
                                        themed_button(menu.label()),
                                        observe(on_menu_button),
                                    ));
                                    slot.spawn((
                                        Name::new(format!("{} Menu", menu.label())),
                                        MenuDropdown,
                                        menu,
                                        menu_z(),
                                        menu_dropdown_node(),
                                        panel(skin),
                                    ))
                                    .with_children(|items| build_menu(items, menu, skin));
                                });
                        }
                    });
                });
                // Dead centre, in every context, greyed inside a ship (see
                // `sync_play_button`): a control that vanishes reads as a bug,
                // and the greyed button says where Play went. In its own slot
                // because `themed_button` is percent(100) wide - built for the
                // rail - and a bare one on the bar swallows the whole row.
                bar.spawn((
                    Name::new("Play Slot"),
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
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
                // The right column carries the context readout. The buttons
                // that used to sit here - Parts, Delete, Rebind - are verbs of
                // the ship you are inside and now hang under the Ship menu,
                // beside the other menus, where the pointer already goes for
                // File and Add. The crumb took their place rather than leaving
                // half the bar blank, and it reads better for it: on the left
                // it had to share a column with five menu buttons and was cut
                // to "[SHIP] scenar".
                bar.spawn((
                    Name::new("Top Bar Right"),
                    Node {
                        flex_basis: px(0),
                        flex_grow: 1.0,
                        min_width: px(0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexEnd,
                        ..default()
                    },
                ))
                .with_children(|right| {
                    // The breadcrumb doubles as the context readout: the tree
                    // marks the entered node, this says the same thing as a
                    // sentence - level, path, selection (see `sync_breadcrumb`).
                    // Phosphor rather than muted: it is the one line that says
                    // what a click will act on, so it must not read as a
                    // caption.
                    // A WRAPPER carries the clip and the text hangs inside it.
                    // `Overflow::clip` bounds a node's CHILDREN, and a text
                    // node draws its own glyphs - so the clip that used to sit
                    // on the text bounded nothing, and a long crumb ran on
                    // under the Play button.
                    //
                    // On the wrapper and not on the column: the column is a
                    // flex parent of exactly this, and a clip that high would
                    // also bound anything absolutely positioned inside it. The
                    // breadcrumb is the only thing on the bar that grows
                    // without bound, so it is the only thing that needs any of
                    // this.
                    right
                        .spawn((
                            Name::new("Context Breadcrumb Clip"),
                            Node {
                                min_width: px(0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ))
                        .with_child((
                            Name::new("Context Breadcrumb"),
                            ContextBreadcrumb,
                            Node::default(),
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
                });
            });

            // The catcher an open menu drops behind itself. A sibling of the
            // bar rather than a child of it, because "anywhere else" includes
            // the rail and the viewport.
            root.spawn((menu_scrim(), observe(on_menu_scrim)));

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
                    // Allowed to SHRINK below its content. Without this a tall
                    // child - an inspector on a turret's joint tree - grows the
                    // row past the bottom of the screen instead of being given
                    // a box to scroll inside.
                    min_height: px(0),
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
                        rail.spawn(panel_header("Scenario"));
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
                            settings.spawn(panel_header("Ship Settings"));
                            // The engineer readout: properties of the hull
                            // being built, and they move with every part
                            // placed. Without them a hull that is too big to
                            // turn reads as the game being broken rather than
                            // as a hull that wants another computer.
                            settings.spawn((
                                Name::new("Ship Readout"),
                                ShipReadout,
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(READOUT_TEXT),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR),
                                Node {
                                    margin: UiRect::top(px(4)),
                                    ..default()
                                },
                            ));
                            // The remedy, under the numbers it is about. Muted
                            // and a size down, because it is the sentence a
                            // builder reads ONCE per surprise and the block
                            // above is the one they watch.
                            settings.spawn((
                                Name::new("Ship Readout Note"),
                                ShipReadoutNote,
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR_MUTED),
                                Node {
                                    margin: UiRect::bottom(px(4)),
                                    ..default()
                                },
                            ));
                            // A SETTING rather than a tool: it arms nothing, it
                            // changes what the ship on the stage looks like -
                            // and what it looks like when it flies.
                            settings.spawn((
                                Name::new("Ship Skin Toggle"),
                                skin_toggle_row(skinned, skin),
                                observe(on_skin_toggle),
                            ));
                            // The sentence that used to hide in the key legend.
                            // It is a fact about THIS setting - the skin
                            // reflows around the part in hand - so it belongs
                            // under the row it is about, not in a line of keys
                            // View can switch off.
                            settings.spawn((
                                Name::new("Ship Skin Note"),
                                Text::new("reflows around the part in hand"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR_MUTED),
                                Node {
                                    margin: UiRect::bottom(px(4)),
                                    ..default()
                                },
                            ));
                            // Under the toggle, and shown only while it is on,
                            // because it answers the question the toggle
                            // raises: the skin is on, and this is which of the
                            // shipped styles it wears. One row per style out of
                            // the MERGED content, so a mod's style is listed
                            // beside the base ones without the editor knowing
                            // any id.
                            settings
                                .spawn((Name::new("Ship Style List"), StyleList, rail_list_node()))
                                .with_children(|list| {
                                    for (index, (id, name, colour)) in listed.iter().enumerate() {
                                        list.spawn((
                                            Name::new(format!("Style: {name}")),
                                            // The first is what an unset style
                                            // wears, so it starts marked.
                                            style_row(id, name, *colour, index == 0, skin),
                                            observe(on_style_choice),
                                        ));
                                    }
                                });
                        });
                    });
                // The Inspector, on the OTHER side of the stage from the tree:
                // the rail says what the document holds and this says what one
                // of those things is, and putting both in one column is what
                // ran the old all-in-one rail out of screen.
                content.spawn(inspector_panel(skin));
            });

            // The hint a tree row reveals on hover, and the layer the floating
            // windows stand on. Both hang off the root rather than off the
            // rail that raises them, because both are positioned against the
            // SCREEN: the windows stand above everything else the editor draws
            // - a window a panel could cover would be a window nobody opened -
            // and the hint stands above the windows.
            root.spawn(scene_tooltip(skin));
            root.spawn(placement_callout(skin));
            root.spawn(window_layer());

            // The foot of the screen: the placement verdict, then the key
            // legend under it. ONE bottom-anchored column rather than two
            // absolute rows, because the legend WRAPS - a status line pinned
            // at a fixed height above the bottom edge sat on top of the second
            // row of hints the moment the window was narrow enough to need one.
            //
            // BOUNDED, not full width: `left` and `right` pin the column
            // between the rail and the Inspector, which is also the span the
            // verdict is about.
            root.spawn((
                Name::new("Editor Foot"),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(8),
                    left: px(RAIL_W + 12.0),
                    right: px(INSPECTOR_W + 12.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: px(6),
                    ..default()
                },
                GlobalZIndex(10),
                children![
                    // A full-width row rather than an offset chip: the chip's
                    // width follows its text, so centring is the row's job.
                    (
                        Name::new("Placement Status Row"),
                        Node {
                            width: percent(100),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
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
                    ),
                    // Contextual (see `sync_key_legend`): a builder holding a
                    // part needs the pose keys, and one in select mode needs to
                    // be told the pipette exists.
                    (
                        Name::new("Editor Key Legend"),
                        EditorKeyLegend,
                        Pickable {
                            should_block_lower: false,
                            is_hoverable: false,
                        },
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            align_items: AlignItems::Center,
                            column_gap: px(10),
                            row_gap: px(4),
                            ..default()
                        },
                        Children::spawn(SpawnWith(move |cells: &mut RelatedSpawner<ChildOf>| {
                            cells.spawn(legend_mode_cell());
                            for index in 0..LEGEND_CELLS {
                                cells.spawn(legend_cell(index));
                            }
                        })),
                    ),
                ],
            ));
        });
}

/// Flip the skin on or off.
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

/// Styles a release build does not list. `placeholder` is the scaffolding the
/// base bundle ships to prove the pipeline dresses plates at all (it is built
/// by `nova_authoring::base_content::styles`), and it is not a look anybody
/// would choose - but a debug build is exactly where somebody wants to look at
/// it on a hull.
const DEBUG_ONLY_STYLES: &[&str] = &["placeholder"];

/// The style rows to build, out of the MERGED catalog - so a mod's style is
/// listed beside the base ones without the editor knowing any id.
///
/// The colour is the paint the style puts on a hull's TOP surface, which is the
/// face the build view mostly shows. A style that dresses no surface restates
/// the built-in plate colours, and its row carries no colour rather than a
/// guessed one.
fn listed_styles(styles: &GameStyles) -> Vec<(String, String, Color)> {
    styles
        .iter()
        .filter(|style| cfg!(feature = "debug") || !DEBUG_ONLY_STYLES.contains(&style.id.as_str()))
        .map(|style| {
            let colour = style
                .surface(ShellSurface::Top)
                .or_else(|| style.surfaces.first())
                .map_or(Color::NONE, |dress| dress.color);
            (style.id.clone(), style.name.clone(), colour)
        })
        .collect()
}

/// The style list's own column, so the rows read as one group under the toggle
/// rather than as four more tools.
fn rail_list_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// One row the Scene tree wants: the node it points at, and how it reads.
#[derive(Clone, PartialEq, Eq)]
struct WantedRow {
    node: Entity,
    /// How deep under the scenario root the node sits. The row spends it on
    /// left padding, which is what makes the list read as a tree.
    depth: usize,
    /// The glyph in front of the label, saying what the node is. `@` marks the
    /// node the editor is inside, `>` the ship Play hands to the player, `-` a
    /// design beside it, and a section wears its kind.
    lead: String,
    /// The node's id: what the row is CALLED, and what the driven walks find
    /// it by.
    id: String,
    /// What the row READS as. The id, shortened where the id repeats what the
    /// tree already says (see [`tree_text`]).
    label: String,
    /// The row's right-hand column: which one this is, when the id ends in an
    /// ordinal. Held apart from the label because it is the half that must
    /// survive a narrow rail (see [`tree_text`]).
    trail: String,
    /// What the row's icon MEANS, in one word. Read back by the hover hint,
    /// which is where a builder finds out what `%` was.
    kind: String,
}

/// What a node reads as in a 150px rail: (label, trail).
///
/// The AUTHORED name leads where there is one - a name nobody could see was a
/// name nobody could use. Where there is none the id stands in, minus the
/// ordinal: every shipped part is called `<something>_section_<n>`, so a
/// section's minted id spends a third of the row on the one word its glyph and
/// its place in the tree already say, and then clips off the number, which is
/// the only thing telling six reinforced hulls apart.
///
/// The ordinal goes to the row's own right-hand column, where the clip cannot
/// reach it.
///
/// Display only: the row is still named, selected and reported by its id.
pub(crate) fn tree_text(name: &str, id: &str) -> (String, String) {
    let (stem, ordinal) = split_ordinal(id);
    // An AUTHORED name stands alone: it is the thing that tells two nodes
    // apart, so an ordinal after it is one number nobody asked for. The
    // fallback keeps the ordinal, because there the id is all there is.
    if !name.is_empty() {
        return (name.to_string(), String::new());
    }
    (stem.to_string(), ordinal.to_string())
}

/// The one character that says "there is more of this name than the rail can
/// show". In the shipped face; the row's own clip is the backstop.
const ELLIPSIS: &str = "\u{2026}";

/// How many characters of label a row of `depth` can draw.
///
/// Character arithmetic rather than pixels: every face in the editor is
/// Iosevka, a monospace, so a column of the rail IS a column of text. The
/// indent takes a step out of the budget at every depth.
fn label_budget(depth: usize) -> usize {
    /// What a root row fits beside its mark and its trail.
    const AT_ROOT: usize = 15;
    /// What one step of indent costs, rounded to the character it eats.
    const PER_STEP: usize = 1;

    AT_ROOT.saturating_sub(depth * PER_STEP)
}

/// `label` shortened to fit, with the cut marked.
///
/// The cut is in the MIDDLE, because both ends carry: the head says what the
/// thing is and the tail is the half that differs between two rows of the same
/// family - `reinforced_hull` against `reinforced_hull_heavy`. Clipping at the
/// right edge, which is what the row did before, threw away the only half that
/// told them apart, and did it mid-glyph with nothing to say it had happened.
fn elide(label: &str, budget: usize) -> String {
    let letters: Vec<char> = label.chars().collect();
    if letters.len() <= budget {
        return label.to_string();
    }
    // Under three characters there is no room for a head, a tail and a mark,
    // so the mark alone is the honest answer.
    let Some(keep) = budget.checked_sub(1).filter(|keep| *keep >= 2) else {
        return ELLIPSIS.to_string();
    };
    let head: String = letters.iter().take(keep.div_ceil(2)).collect();
    let tail: String = letters[letters.len() - keep / 2..].iter().collect();
    // The space beside the mark goes with it: `Basic ...roller` spends a column
    // on a gap that the mark already is.
    format!("{}{ELLIPSIS}{}", head.trim_end(), tail.trim_start())
}

/// What a part is CALLED, from the config its row resolves to.
///
/// Minus a trailing `Section`: every part in the catalog is called one, the
/// tree is a tree of parts, and the word is a third of a 150px row spent
/// saying so. `None` where nothing named it - an inline part, or a prototype
/// the catalog has not got - and the row falls back to its id.
fn section_name(section: &SectionNode, catalog: Option<&GameSections>) -> Option<String> {
    let config = section.resolve(catalog)?;
    let name = config.base.name.trim();
    let name = name.strip_suffix("Section").unwrap_or(name).trim_end();
    (!name.is_empty()).then(|| name.to_string())
}

/// What the Scene tree is showing, so a frame that changed nothing costs one
/// comparison instead of a respawned list - and, more to the point, so hover
/// and selection survive a frame in which the document did not change.
#[derive(Default)]
pub(crate) struct ShownScene {
    rows: Vec<WantedRow>,
}

/// The document as a tree, for the context the editor is standing in.
///
/// At the scenario node that is the whole world: the root, every ship under it,
/// and then the objects. Ships first and objects after, rather than one
/// id-sorted list, because a builder came here to build a ship and the range it
/// stands on is context.
///
/// INSIDE A SHIP THE TREE ISOLATES IT: the root, that ship, and that ship's
/// sections - nothing else. The stage already takes the rest of the world away
/// when a ship is entered, and a row for a beacon you cannot see, whose
/// selection the Inspector would then report from inside a ship, is a click
/// that means nothing. The root row stays because it is the way back out.
///
/// The same nodes `context_nodes` reports, plus the two rungs of the path, so
/// the tree and the probe agree.
fn wanted_rows(
    context: &EditContext,
    q_scenarios: &Query<&NodeId, With<ScenarioNode>>,
    q_ships: &ShipNodes,
    q_objects: &ObjectNodes,
    nodes: &SectionNodes,
    catalog: Option<&GameSections>,
) -> Vec<WantedRow> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    let Ok(root_id) = q_scenarios.get(scenario) else {
        return Vec::new();
    };
    let (root_label, root_trail) = tree_text("", &root_id.0);
    let mut rows = vec![WantedRow {
        node: scenario,
        depth: 0,
        lead: SCENARIO.to_string(),
        id: root_id.0.clone(),
        label: elide(&root_label, label_budget(0)),
        trail: root_trail,
        kind: "SCENARIO".to_string(),
    }];

    let entered = context.ship();
    let mut ships: Vec<_> = q_ships
        .iter()
        .filter(|(_, owner, ..)| owner.parent() == scenario)
        .collect();
    ships.sort_unstable_by(|a, b| id_order(&a.2 .0).cmp(&id_order(&b.2 .0)));
    for (ship, _, id, node) in ships {
        // Isolation: a ship that is not the one being edited is not a rung of
        // the path and not a thing to act on from in here.
        if entered.is_some_and(|inside| inside != ship) {
            continue;
        }
        // WHO FLIES IT in the lead, WHERE YOU ARE in the trail. One column
        // for both meant entering the player's ship hid the fact that it was
        // the player's.
        let (glyph, kind) = ship_mark(node.driver);
        let (label, ordinal) = tree_text(&node.name, &id.0);
        let inside = entered == Some(ship);
        let parts = sections_of(ship, nodes).len();
        rows.push(WantedRow {
            node: ship,
            depth: 1,
            lead: glyph.to_string(),
            id: id.0.clone(),
            label: elide(&label, label_budget(1)),
            // ONE fact, chosen in this order: where the editor is standing,
            // then how much is folded up inside this row, then which ship it
            // is. A ship you are already in lists its parts one row below, and
            // a named ship - which every minted ship is - has no ordinal to
            // draw.
            trail: match (inside, parts) {
                (true, _) => INSIDE.to_string(),
                (false, 0) => ordinal,
                (false, parts) => parts.to_string(),
            },
            kind: match (inside, parts) {
                (true, _) => format!("{kind} - EDITING"),
                (false, 0) => format!("{kind} - EMPTY"),
                (false, 1) => format!("{kind} - 1 PART"),
                (false, parts) => format!("{kind} - {parts} PARTS"),
            },
        });
        if entered != Some(ship) {
            continue;
        }
        for (section, id, node, _) in sections_of(ship, nodes) {
            // The ORDINAL comes off the id even where the part is named,
            // unlike a ship: six reinforced hulls share one name, and the
            // number is the only thing telling them apart.
            let (stem, trail) = tree_text("", &id.0);
            let label = section_name(node, catalog).unwrap_or(stem);
            let (glyph, kind) = section_mark(node, catalog);
            rows.push(WantedRow {
                node: section,
                depth: 2,
                lead: glyph.to_string(),
                id: id.0.clone(),
                label: elide(&label, label_budget(2)),
                trail,
                kind: kind.to_string(),
            });
        }
    }
    // The world's objects belong to the scenario node, so they are listed
    // there and only there.
    let world = if entered.is_none() {
        objects_of(scenario, q_objects)
    } else {
        Vec::new()
    };
    for (object, id, node, _) in world {
        let (label, trail) = tree_text(&node.name, &id.0);
        let (glyph, kind) = object_mark(node);
        rows.push(WantedRow {
            node: object,
            depth: 1,
            lead: glyph.to_string(),
            id: id.0.clone(),
            label: elide(&label, label_budget(1)),
            trail,
            kind: kind.to_string(),
        });
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
    q_objects: ObjectNodes,
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
    let wanted = wanted_rows(
        &context,
        &q_scenarios,
        &q_ships,
        &q_objects,
        &nodes,
        catalog.as_deref(),
    );
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

    // The WHOLE row is the signature, names included: a rename changes nothing
    // about which nodes are listed, and a list that only compared ids would
    // keep drawing the old name until something else moved.
    if shown.rows != wanted {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|list| {
            for row in &wanted {
                // Painted marked from the start rather than waiting for the
                // pass below: these rows do not exist in `rows` until next
                // frame, and a highlight that lags a frame behind the click
                // that made it reads as a dropped input.
                let marked = Some(row.node) == selected.0;
                let mut entity = list.spawn((
                    // Named by ID, drawn by LABEL: the walks and the probe
                    // find a row by the node's own key, whatever the rail has
                    // room to print.
                    Name::new(format!("Scene Row {}", row.id)),
                    scene_row(row.depth, &row.lead, &row.label, &row.trail, marked, *skin),
                    SceneRow(row.node),
                    // What a hover reveals: the kind the icon stands for, and
                    // the id the 150px row had to clip.
                    SceneRowHint {
                        kind: row.kind.clone(),
                        id: row.id.clone(),
                    },
                    observe(on_scene_row),
                ));
                // The row's own delete, on the child that draws it: a press on
                // the trash must not read as a press on the row it sits in.
                let trash = entity.id();
                entity.commands().queue(move |world: &mut World| {
                    let Some(children) = world.get::<Children>(trash).map(|kids| kids.to_vec())
                    else {
                        return;
                    };
                    for child in children {
                        if world.get::<SceneRowTrash>(child).is_some() {
                            world.entity_mut(child).observe(on_row_trash);
                        }
                    }
                });
                if marked {
                    entity.insert(Selected);
                }
            }
        });
        shown.rows = wanted;
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

/// Show a row's delete on hover or on selection, and only where it would work.
///
/// Both, rather than hover alone: the pointer is not the only way a row gets
/// marked - the stage marks one too - and a marked row is the one the keyboard
/// Del is aimed at, so it should be showing the same affordance.
pub(crate) fn sync_row_trash(
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    nodes: Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    rows: Query<(&SceneRow, &Hovered, &Children)>,
    mut trash: Query<&mut Node, With<SceneRowTrash>>,
) {
    for (row, hovered, children) in &rows {
        let show =
            (hovered.get() || selected.0 == Some(row.0)) && deletable(row.0, &context, &nodes);
        let display = if show { Display::Flex } else { Display::None };
        for &child in children {
            let Ok(mut node) = trash.get_mut(child) else {
                continue;
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

/// A row's delete: remove the node that row stands for.
///
/// Marks it first, so the one place the editor says what it is about to
/// destroy - the selection - agrees with what it destroys.
pub(crate) fn on_row_trash(
    activate: On<Activate>,
    mut commands: Commands,
    mut selected: ResMut<SelectedNode>,
    context: Res<EditContext>,
    nodes: Query<(), Or<(With<ShipNode>, With<ObjectNode>, With<SectionNode>)>>,
    parents: Query<&ChildOf>,
    rows: Query<&SceneRow>,
) {
    let Some(row) = parents
        .get(activate.entity)
        .ok()
        .and_then(|parent| rows.get(parent.parent()).ok())
    else {
        return;
    };
    if !deletable(row.0, &context, &nodes) {
        return;
    }
    commands.entity(row.0).despawn();
    selected.0 = None;
}

/// One click SELECTS a row and puts the camera on it; two ENTER it.
///
/// The single click is the cheap, reversible one, so it is the one that
/// answers everywhere: every row selects, and the camera goes to whatever was
/// named. Only the second click changes the CONTEXT - into a ship, or out of
/// one from the root row - because that is the gesture that hides the rest of
/// the document behind a breadcrumb.
///
/// An earlier version entered on the first click, so that a container never
/// needed a double (owner: a double-click here had read as "the first click
/// did nothing"). It does something now: it frames.
pub(crate) fn on_scene_row(
    activate: On<Activate>,
    rows: Query<&SceneRow>,
    ships: Query<(), With<ShipNode>>,
    scenarios: Query<(), With<ScenarioNode>>,
    time: Res<Time<Real>>,
    mut last: ResMut<LastClick>,
    mut selected: ResMut<SelectedNode>,
    mut context: ResMut<EditContext>,
    mut request: ResMut<FrameRequest>,
) {
    let Ok(SceneRow(node)) = rows.get(activate.entity) else {
        return;
    };
    let double = last.press(*node, time.elapsed_secs());
    // A click that CHANGES the context hands the camera to
    // `crate::node::sync_camera_focus`, which frames whatever was entered - so
    // the request the first half of the double raised steps aside rather than
    // writing the camera a second time in the same frame.
    if scenarios.contains(*node) {
        if double {
            context.to_root();
            selected.0 = None;
            request.0 = None;
        } else {
            // The root is the whole stage: framing it is what "show me
            // everything" means, and leaving a ship is the second click.
            ask_for(&mut request, Some(*node));
        }
        return;
    }
    if double && ships.contains(*node) {
        context.enter(*node);
        selected.0 = None;
        request.0 = None;
        return;
    }
    selected.0 = Some(*node);
    ask_for(&mut request, Some(*node));
}

/// Paint whatever the editor is currently saying onto its one status line.
///
/// One WRITER, so the placement readout and a verb's answer cannot fight over
/// the node: they both write [`EditorStatus`], and this is the only thing that
/// touches the text. It also holds the clock: a verb's message is dropped here
/// once its hold is over, so every other reader sees the line as it reads.
pub(crate) fn sync_status_line(
    time: Res<Time>,
    mut status: ResMut<EditorStatus>,
    lines: Query<
        (&mut Text, &mut TextColor, &mut BorderColor, &mut Visibility),
        With<PlacementStatus>,
    >,
) {
    status.expire(time.elapsed_secs_f64());
    let line = status.line();
    for (mut text, mut colour, mut border, mut visibility) in lines {
        match line {
            Some((message, tint)) => {
                if text.0 != message {
                    text.0 = message.to_string();
                }
                if colour.0 != tint {
                    colour.0 = tint;
                    *border = BorderColor::all(tint);
                }
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            None => {
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
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
    buttons: Query<(Entity, Has<InteractionDisabled>, &Children), With<PlayButton>>,
    mut labels: Query<&mut Text, With<ButtonLabel>>,
) {
    let disabled = context.ship().is_some();
    for (entity, marked, children) in &buttons {
        // The button carries its own reason. Greying it says only that the
        // verb is gone; the sentence that said where it went lived in an
        // observer `InteractionDisabled` makes unreachable, so it was written
        // to a log nobody reading the screen can see.
        let wanted = if disabled { PLAY_BLOCKED } else { PLAY_LABEL };
        for &child in children {
            let Ok(mut text) = labels.get_mut(child) else {
                continue;
            };
            if text.0 != wanted {
                text.0 = wanted.to_string();
            }
        }
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

/// Show the rail's ship settings block only inside a ship.
///
/// Hidden rather than disabled, unlike Play: a greyed skin toggle at the
/// scenario node would say "this exists here and is refused", and it does not
/// exist there - a skin is a thing a SHIP has. The ship's VERBS answer the same
/// question in the Ship menu, where greyed rows say what entering a ship would
/// unlock (see `crate::ui::menu::sync_ship_menu`).
pub(crate) fn sync_context_panels(
    context: Res<EditContext>,
    mut panels: Query<&mut Node, With<ShipSettings>>,
) {
    let display = if context.ship().is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut panels {
        if node.display != display {
            node.display = display;
        }
    }
}

/// Write the top bar's context readout: WHAT is being edited (the level, in
/// capitals), the path to it in the names the tree shows, and the selection.
///
/// The level leads because it was the missing feedback: the bare path
/// "scenario / ship_1" never said whether a click would select, enter or
/// place. The same fact the tree's `@` mark shows, said as a sentence.
pub(crate) fn sync_breadcrumb(
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    ids: Query<&NodeId>,
    ships: Query<&ShipNode>,
    objects: Query<&ObjectNode>,
    mut crumbs: Query<&mut Text, With<ContextBreadcrumb>>,
) {
    let named = |node: Entity| {
        let id = ids
            .get(node)
            .map_or_else(|_| String::new(), |id| id.0.clone());
        let authored = ships
            .get(node)
            .map(|ship| ship.name.clone())
            .or_else(|_| objects.get(node).map(|object| object.name.clone()))
            .unwrap_or_default();
        let (label, ordinal) = tree_text(&authored, &id);
        if ordinal.is_empty() {
            label
        } else {
            format!("{label} {ordinal}")
        }
    };
    let path = context
        .path
        .iter()
        .map(|node| named(*node))
        .collect::<Vec<_>>()
        .join(" / ");
    let level = match (context.scenario(), context.ship()) {
        (None, _) => "",
        (Some(_), None) => "SCENARIO  ",
        (Some(_), Some(_)) => "SHIP  ",
    };
    let mut wanted = format!("{level}{path}");
    if let Some(node) = selected.0 {
        wanted.push_str(&format!("   selected {}", named(node)));
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

/// Pick the style this row names.
///
/// Writes an explicit id rather than a list index: the build state travels out
/// to the scenario and back, and an index into a catalog a mod can grow would
/// not survive that trip meaning the same thing.
/// Picking a style off the greyed list also turns the skin ON. The list is
/// visible while the skin is off so it can advertise what the toggle leads to,
/// and an advertisement that answers a press with nothing is worse than no
/// advertisement.
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
        ship.skin = true;
    }
}

/// Mark the row the build view is actually dressing plates in, and grey the
/// whole list while the ship wears no skin.
///
/// GREYED rather than hidden: a list that appears when a checkbox is ticked is
/// a list nobody knew the checkbox led to, and the styles are the reason to
/// tick it. The rows stay pressable - see [`on_style_choice`].
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
    rows: Query<(Entity, &StyleChoice, Has<Selected>, &Children)>,
    mut labels: Query<&mut TextColor>,
    mut swatches: Query<(&StyleSwatch, &mut BackgroundColor)>,
) {
    let ship = edited_ship(&context, &q_ships);
    let skinned = ship.is_some_and(|ship| ship.skin);
    let label = if skinned {
        theme::PHOSPHOR
    } else {
        theme::PHOSPHOR_MUTED
    };
    let paint = if skinned { 1.0 } else { GREYED_STYLE_ALPHA };

    // Nothing is marked while the skin is off: the ship is wearing no style,
    // and a mark on a greyed row says it is - in the one paint the greying has
    // to fight to be read through.
    let active = skinned
        .then(|| {
            match ship.and_then(|ship| ship.style.as_deref()) {
                Some(id) => styles.get_style(id),
                None => styles.first(),
            }
            .map(|style| style.id.as_str())
        })
        .flatten();
    for (entity, choice, selected, children) in &rows {
        match (active == Some(choice.0.as_str()), selected) {
            (true, false) => {
                commands.entity(entity).insert(Selected);
            }
            (false, true) => {
                commands.entity(entity).remove::<Selected>();
            }
            _ => {}
        }
        for &child in children {
            if let Ok(mut colour) = labels.get_mut(child) {
                if colour.0 != label {
                    colour.0 = label;
                }
            }
            if let Ok((swatch, mut background)) = swatches.get_mut(child) {
                let wanted = swatch.0.with_alpha(paint);
                if background.0 != wanted {
                    *background = wanted.into();
                }
            }
        }
    }
}

/// Repaint the skin checkbox for the state it reports, IN PLACE.
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

/// How many hint cells the legend carries.
///
/// Spawned once and written every frame rather than rebuilt per context: the
/// longest line needs this many, the shorter ones hide the rest, and a legend
/// that despawned and respawned its children would spend a frame empty every
/// time a part was picked up.
const LEGEND_CELLS: usize = 8;

/// The width a hint cell holds even when its words are shorter, so the hints
/// line up into columns rather than into a paragraph.
///
/// A FLOOR, not a fixed width: the line wraps inside the gap between the rail
/// and the Inspector, so a hint too long for its column widens its own cell
/// instead of pushing the line off the window.
const LEGEND_CELL_W: f32 = 100.0;

/// The legend's leading cell: which mode the keys below belong to.
///
/// Needed because one key means two things. `F` frames the selection in select
/// mode and cycles the socket with a part in hand, and the legend that named
/// only the key could not say which one this press would be.
#[derive(Component)]
pub(crate) struct LegendMode;

/// One hint cell, found by its place in the line.
#[derive(Component)]
pub(crate) struct LegendCell(usize);

/// The mode cell: an amber word, wider than a hint because it carries a phrase.
fn legend_mode_cell() -> impl Bundle {
    (
        Name::new("Legend Mode"),
        LegendMode,
        Node {
            margin: UiRect::right(px(4)),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(theme::AMBER_NOVA),
    )
}

/// One hint: the key as the chip every other surface draws a key as, and what
/// it does beside it.
fn legend_cell(index: usize) -> impl Bundle {
    (
        Name::new(format!("Legend Cell {index}")),
        LegendCell(index),
        Node {
            min_width: px(LEGEND_CELL_W),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![
            (LegendChip, key_chip("", 12.0)),
            (
                LegendLabel,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ),
        ],
    )
}

/// The chip half of a hint cell, and the words half.
#[derive(Component)]
pub(crate) struct LegendChip;
#[derive(Component)]
pub(crate) struct LegendLabel;

/// One hint: a key and what it does, in the one grammar the whole editor uses -
/// the key as it is typed, then a lowercase verb phrase.
type Hint = (&'static str, &'static str);

/// What the next Escape would take.
///
/// Escape has five rungs and the legend used to name one of them. It is a back
/// key, so the only useful thing it can say is what THIS press does - which is
/// a fact the ladder in `crate::escape_backs_out` already decides and nothing
/// else on screen reports.
fn escape_rung(rebinding: bool, menu_open: bool, armed: bool, inside_ship: bool) -> &'static str {
    match (rebinding, menu_open, armed, inside_ship) {
        (true, _, _, _) => "cancel the rebind",
        (_, true, _, _) => "close the menu",
        (_, _, true, _) => "put the part down",
        (_, _, _, true) => "leave the ship",
        _ => "pause",
    }
}

/// Keep the key legend in step with the armed tool AND the edit context.
///
/// Keyed on both because the same keys mean different things per level: at the
/// scenario node there are no parts to arm and Escape falls through to pause,
/// while inside a ship Tab browses parts and Escape backs out one rung.
///
/// WHAT IS LEFT HERE is what no other surface can carry: the pointer gestures
/// and the free-fly rig, which belong to no row and no menu, plus the rung the
/// next Escape takes. Every verb a menu row can name now lives on that row
/// with its key beside it.
///
/// Compared before writing rather than gated on a change: the legend is
/// spawned on entering the editor, which is not necessarily a frame the tool
/// or the context changed on.
///
/// View > Key Legend hides it. The line is still kept current while hidden -
/// it costs a few string compares, and turning the legend back on has to show
/// what the editor is doing NOW.
pub(crate) fn sync_key_legend(
    selection: Res<SectionChoice>,
    context: Res<EditContext>,
    overlays: Res<EditorOverlays>,
    rebind: Res<EditorRebind>,
    open_menu: Res<OpenMenu>,
    legend: Query<(&mut Node, &Children), With<EditorKeyLegend>>,
    mut modes: Query<&mut Text, With<LegendMode>>,
    cells: Query<(&LegendCell, &Children)>,
    chips: Query<&Children, With<LegendChip>>,
    mut nodes: Query<&mut Node, Without<EditorKeyLegend>>,
    mut texts: Query<&mut Text, Without<LegendMode>>,
) {
    let inside = context.ship().is_some();
    let escape = escape_rung(
        rebind.target.is_some(),
        open_menu.0.is_some(),
        *selection != SectionChoice::None,
        inside,
    );
    // The pointer and the rig, then the one key that changes meaning with the
    // rung it is on. Ordered so the gestures a builder is about to make come
    // first and the way out comes last.
    let (mode, hints): (&str, &[Hint]) = match (&*selection, inside) {
        (SectionChoice::None, false) => (
            "SELECT",
            &[
                ("LMB", "select"),
                ("LMB x2", "enter"),
                ("drag", "move it"),
                ("RMB+drag", "look"),
                ("WASD", "fly"),
                ("Space/Shift", "up and down"),
            ],
        ),
        (SectionChoice::None, true) => (
            "IN A SHIP",
            &[
                ("LMB", "select"),
                ("LMB x2", "leave"),
                ("Q", "pick a part"),
                ("RMB+drag", "look"),
                ("WASD", "fly"),
                ("Space/Shift", "up and down"),
            ],
        ),
        (SectionChoice::Section(_), _) => (
            "PART IN HAND",
            &[
                ("LMB", "place it"),
                ("Q", "pick a part"),
                ("RMB+drag", "look"),
                ("WASD", "fly"),
                ("Space/Shift", "up and down"),
            ],
        ),
    };
    for mut text in &mut modes {
        if text.0 != mode {
            text.0 = mode.to_string();
        }
    }
    let display = if overlays.key_legend {
        Display::Flex
    } else {
        Display::None
    };
    let Ok((mut root, children)) = legend.single_inner() else {
        return;
    };
    if root.display != display {
        root.display = display;
    }
    for &child in children {
        let Ok((cell, parts)) = cells.get(child) else {
            continue;
        };
        // Escape is always the LAST hint, whatever the mode: it is the way
        // back, and a way back that moved along the line as the context
        // changed would be a key you had to look for.
        let hint = if cell.0 == hints.len() {
            Some(("Esc", escape))
        } else {
            hints.get(cell.0).copied()
        };
        write_legend_cell(child, hint, parts, &chips, &mut nodes, &mut texts);
    }
}

/// Write one cell, or hide it when this mode has no hint for it.
fn write_legend_cell(
    cell: Entity,
    hint: Option<Hint>,
    parts: &Children,
    chips: &Query<&Children, With<LegendChip>>,
    nodes: &mut Query<&mut Node, Without<EditorKeyLegend>>,
    texts: &mut Query<&mut Text, Without<LegendMode>>,
) {
    for (place, part) in parts.iter().enumerate() {
        let wanted = match (place, hint) {
            (0, Some((key, _))) => key,
            (_, Some((_, label))) => label,
            (_, None) => "",
        };
        // A chip's word is one level down, inside the bordered box; the label
        // beside it is the text itself.
        let target = chips
            .get(part)
            .ok()
            .and_then(|kids| kids.first().copied())
            .unwrap_or(part);
        if let Ok(mut text) = texts.get_mut(target) {
            if text.0 != wanted {
                text.0 = wanted.to_string();
            }
        }
    }
    let display = if hint.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if let Ok(mut node) = nodes.get_mut(cell) {
        if node.display != display {
            node.display = display;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nova_scenario::prelude::SectionSource;
    use nova_ship::prelude::ShipStyleConfig;

    use super::*;
    use crate::{
        glyph::SHIP_PLAYER,
        node::{NextChildOrdinal, ShipDriver},
    };

    /// A rail with the Scene tree on it and the reconciler running, over an
    /// empty document. The tests below fill the document in.
    fn scene_app() -> App {
        let mut app = App::new();
        app.insert_resource(UiSkin::default());
        app.init_resource::<SelectedNode>();
        app.init_resource::<LastClick>();
        app.init_resource::<FrameRequest>();
        // No `TimePlugin`: the clock is driven by hand below, so a "click" and
        // a "double click" are a choice the test makes rather than a race with
        // how fast the suite runs.
        app.init_resource::<Time<Real>>();
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

    /// The TEXT of each row, which is not the same as its name: a section's
    /// row is drawn shorter than its id.
    fn row_columns(app: &mut App) -> Vec<(String, String)> {
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
                let children = app.world().get::<Children>(row)?;
                let (wrapper, trail) = (children.iter().nth(1)?, children.iter().nth(2)?);
                let label = app.world().get::<Children>(wrapper)?.iter().next()?;
                Some((
                    app.world().get::<Text>(label)?.0.clone(),
                    app.world().get::<Text>(trail)?.0.clone(),
                ))
            })
            .collect()
    }

    /// One click, a full second after whatever came before it.
    fn press(app: &mut App, row: Entity) {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        app.world_mut().trigger(Activate { entity: row });
        app.update();
    }

    /// Two clicks close enough together to read as one double click.
    fn double_press(app: &mut App, row: Entity) {
        press(app, row);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(80));
        app.world_mut().trigger(Activate { entity: row });
        app.update();
    }

    /// What the camera has been asked to look at, and clear the request.
    fn framed(app: &mut App) -> Option<Entity> {
        let asked = app.world().resource::<FrameRequest>().0;
        app.world_mut().resource_mut::<FrameRequest>().0 = None;
        asked
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

    /// The lead texts of the rows, in draw order - the kind glyphs the
    /// assertions read.
    /// What each row's hover hint would say about its kind.
    fn row_hints(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&SceneRowHint>()
            .iter(app.world())
            .map(|hint| hint.kind.clone())
            .collect()
    }

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

    /// Each row's left padding, in draw order - what the indentation assertion
    /// reads.
    fn row_indents(app: &mut App) -> Vec<f32> {
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
            .filter_map(|row| match app.world().get::<Node>(row)?.padding.left {
                Val::Px(left) => Some(left),
                _ => None,
            })
            .collect()
    }

    /// At the scenario node the tree is the whole DOCUMENT: the root and every
    /// ship, each branch collapsed. Entering one ISOLATES it - the root, that
    /// ship and its sections, and nothing a click in there could not mean.
    #[test]
    fn entering_a_ship_isolates_it_in_the_tree() {
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
            vec!["scenario", "ship_1", "hull_1"],
            "the entered ship opens, and its sibling is not in the tree at all"
        );

        app.world_mut().resource_mut::<EditContext>().enter(second);
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_2", "turret_1"],
            "entering the sibling moves the whole tree to it"
        );
    }

    /// The lead column is the tree's whole vocabulary: the root, the ship Play
    /// flies, a design beside it, and a section wearing its kind. WHERE THE
    /// EDITOR IS does not live here - see below.
    #[test]
    fn the_lead_glyphs_say_who_is_who() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        section_node(&mut app, first, "hull_1");

        app.update();
        assert_eq!(
            row_leads(&mut app),
            vec![SCENARIO, SHIP_PLAYER, SHIP_AI],
            "who flies which ship is readable without entering either"
        );

        app.world_mut().resource_mut::<EditContext>().enter(first);
        app.update();
        assert_eq!(
            row_leads(&mut app),
            vec![SCENARIO, SHIP_PLAYER, hull_mark(&mut app, first)],
            "entering keeps the driver mark and the hull section shows its kind"
        );
    }

    /// The mark the hull section under `ship` is drawn with, read from the same
    /// function the row is built by rather than restated as a literal.
    fn hull_mark(app: &mut App, ship: Entity) -> &'static str {
        let section = app
            .world_mut()
            .query::<(&ChildOf, &SectionNode)>()
            .iter(app.world())
            .find(|(parent, _)| parent.parent() == ship)
            .expect("the ship has a section")
            .1
            .clone();
        section_mark(&section, None).0
    }

    /// Two facts, two columns: entering the player's ship used to overwrite the
    /// mark that said it was the player's.
    #[test]
    fn the_entered_ship_is_marked_beside_its_driver() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);

        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        assert_eq!(row_leads(&mut app), vec![SCENARIO, SHIP_PLAYER]);
        assert!(
            row_columns(&mut app).contains(&("ship".to_string(), INSIDE.to_string())),
            "{:?}",
            row_columns(&mut app)
        );
    }

    /// A ship folded shut looked exactly like a ship with nothing in it. The
    /// trail says how much is inside, and the hover hint says what the number
    /// counts.
    #[test]
    fn a_ship_row_says_how_many_parts_are_folded_up_inside_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_section_1");
        section_node(&mut app, ship, "hull_section_2");
        app.update();

        assert!(
            row_columns(&mut app).contains(&("ship".to_string(), "2".to_string())),
            "{:?}",
            row_columns(&mut app)
        );
        assert!(
            row_hints(&mut app)
                .iter()
                .any(|hint| hint == "SHIP - PLAYER - 2 PARTS"),
            "{:?}",
            row_hints(&mut app)
        );
    }

    /// The one thing telling six reinforced hulls apart is the ordinal, and
    /// read as text `10` sorted between `1` and `2`.
    #[test]
    fn the_rows_of_one_family_climb_by_their_ordinals() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        for ordinal in [10, 2, 1] {
            section_node(&mut app, ship, &format!("hull_section_{ordinal}"));
        }
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let trails: Vec<String> = row_columns(&mut app)
            .into_iter()
            .map(|(_, trail)| trail)
            .collect();
        assert_eq!(
            trails,
            vec![
                String::new(),
                INSIDE.to_string(),
                "1".into(),
                "2".into(),
                "10".into()
            ],
        );
    }

    /// A name too long for the rail keeps BOTH ends: the head says what the
    /// thing is, the tail is the half that differs between two of a family.
    #[test]
    fn a_label_too_long_for_the_rail_is_cut_in_the_middle() {
        assert_eq!(elide("hull", 15), "hull", "a short name is left alone");
        assert_eq!(
            elide("reinforced_hull_heavy", 13),
            format!("reinfo{ELLIPSIS}_heavy"),
            "the cut is marked, and the tail survives it"
        );
        assert_eq!(
            elide("Basic Controller", 13),
            format!("Basic{ELLIPSIS}roller"),
            "a space beside the cut goes with it"
        );
        assert_eq!(
            elide("reinforced", 2),
            ELLIPSIS,
            "under three characters there is no room for a head and a tail"
        );
    }

    /// The tree reads as a tree because the rows step right, not because they
    /// draw connectors: a section sits one step further in than the ship that
    /// owns it, and the ship one step in from the root.
    #[test]
    fn nesting_steps_the_rows_right() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_1");
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let indents = row_indents(&mut app);
        assert_eq!(indents.len(), 3, "root, ship, section");
        assert!(
            indents[0] < indents[1] && indents[1] < indents[2],
            "each level steps further right, got {indents:?}"
        );
    }

    /// One click on a ship row marks it and puts the camera on it. The first
    /// click of a double is never wasted, which is what made the owner read an
    /// earlier double-click as a dropped input.
    #[test]
    fn one_click_on_a_ship_row_selects_it_and_frames_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_1");
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);

        assert_eq!(app.world().resource::<SelectedNode>().0, Some(ship));
        assert_eq!(framed(&mut app), Some(ship));
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            None,
            "one click does not hide the rest of the document"
        );
    }

    /// Two clicks ENTER: the gesture that changes what the tree is showing is
    /// the deliberate one.
    #[test]
    fn two_clicks_on_a_ship_row_enter_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_1");
        app.update();

        let row = row_for(&mut app, ship);
        double_press(&mut app, row);

        assert_eq!(app.world().resource::<EditContext>().ship(), Some(ship));
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            None,
            "a container is entered, not selected"
        );
        assert_eq!(
            framed(&mut app),
            None,
            "entering frames the new context itself; two systems writing the \
             camera in one frame is what `crate::frame` exists to avoid"
        );
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "hull_1"],
            "and its branch is open"
        );
    }

    /// Two clicks on DIFFERENT rows are two clicks: the count restarts, so a
    /// quick pass down the tree never falls into a ship.
    #[test]
    fn a_click_on_another_row_is_not_the_second_half_of_a_double() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let first = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        let second = spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Player);
        app.update();

        let row = row_for(&mut app, first);
        press(&mut app, row);
        let row = row_for(&mut app, second);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_millis(80));
        app.world_mut().trigger(Activate { entity: row });
        app.update();

        assert_eq!(app.world().resource::<EditContext>().ship(), None);
        assert_eq!(app.world().resource::<SelectedNode>().0, Some(second));
    }

    /// A slow second click is a second click, not the other half of a double.
    #[test]
    fn a_late_second_click_is_still_one_click() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        let row = row_for(&mut app, ship);
        press(&mut app, row);
        press(&mut app, row);

        assert_eq!(app.world().resource::<EditContext>().ship(), None);
        assert_eq!(app.world().resource::<SelectedNode>().0, Some(ship));
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
    /// than outside the document. One click on it frames the whole stage, which
    /// is what the scenario node's bounds ARE.
    #[test]
    fn the_root_row_leaves_the_ship_on_the_second_click() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();

        let root = row_for(&mut app, scenario);
        press(&mut app, root);
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            Some(ship),
            "one click stays where it is"
        );
        assert_eq!(framed(&mut app), Some(scenario), "and shows the stage");

        let root = row_for(&mut app, scenario);
        double_press(&mut app, root);

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
        // The REAL button: its label is half of what this system writes, and a
        // bare marker entity has no label to write.
        let button = app
            .world_mut()
            .spawn((
                PlayButton,
                Name::new("Play Button"),
                themed_button(PLAY_LABEL),
            ))
            .id();
        app.add_systems(Update, sync_play_button);
        app.init_resource::<EditorStatus>();
        app.init_resource::<Time>();
        app.add_observer(continue_to_simulation);

        app.update();
        assert!(
            !app.world().entity(button).contains::<InteractionDisabled>(),
            "at the scenario node Play is live"
        );
        assert_eq!(play_label(&mut app), PLAY_LABEL);

        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();
        assert!(
            app.world().entity(button).contains::<InteractionDisabled>(),
            "inside a ship it is greyed"
        );
        assert_eq!(
            play_label(&mut app),
            PLAY_BLOCKED,
            "a greyed verb has to say where it went - the reason used to live \
             in an observer the greying makes unreachable"
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
        assert!(
            app.world()
                .resource::<EditorStatus>()
                .line()
                .is_some_and(|(line, _)| line.contains("leave the ship")),
            "and it says so on the one line the editor speaks through, rather \
             than in a log nobody building a ship is reading"
        );
    }

    /// What the Play button reads.
    fn play_label(app: &mut App) -> String {
        let world = app.world_mut();
        let mut labels = world.query_filtered::<&Text, With<ButtonLabel>>();
        labels.single(world).expect("one label").0.clone()
    }

    /// The rail's ship settings are a ship's own: there is no skin to toggle
    /// at the scenario node, so the block is not there either.
    #[test]
    fn the_ship_settings_block_belongs_to_the_entered_ship() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<EditContext>();
        let scenario = world.spawn(ScenarioNode).id();
        let ship = world.spawn(ShipNode::default()).id();
        world.resource_mut::<EditContext>().path = vec![scenario];
        let settings = world.spawn((ShipSettings, Node::default())).id();

        let display = |world: &World, entity: Entity| world.get::<Node>(entity).unwrap().display;

        world.run_system_once(sync_context_panels).unwrap();
        assert_eq!(display(&world, settings), Display::None);

        world.resource_mut::<EditContext>().enter(ship);
        world.run_system_once(sync_context_panels).unwrap();
        assert_eq!(display(&world, settings), Display::Flex);
    }

    /// The readout says WHAT is being edited before it says where: the level
    /// in capitals, the path in the names the tree shows, and the selection.
    /// The bare path never answered "will this click select, enter or place".
    ///
    /// The level is a BARE word, the one treatment a kind tag gets: the
    /// inspector's title and the tree's hint print it that way too, and the
    /// crumb's brackets were a third way of drawing one thing.
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
            .spawn((
                ShipNode {
                    name: "Kestrel".to_string(),
                    ..default()
                },
                NodeId("ship_1".to_string()),
            ))
            .id();
        let crumb = world.spawn((ContextBreadcrumb, Text::new(""))).id();
        world.resource_mut::<EditContext>().path = vec![scenario];

        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(world.get::<Text>(crumb).unwrap().0, "SCENARIO  scenario");

        world.resource_mut::<EditContext>().enter(ship);
        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(
            world.get::<Text>(crumb).unwrap().0,
            "SHIP  scenario / Kestrel"
        );

        world.resource_mut::<SelectedNode>().0 = Some(ship);
        world.run_system_once(sync_breadcrumb).unwrap();
        assert_eq!(
            world.get::<Text>(crumb).unwrap().0,
            "SHIP  scenario / Kestrel   selected Kestrel"
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
            surfaces: vec![StyleSurfaceConfig {
                surface: ShellSurface::Top,
                color: Color::linear_rgb(0.2, 0.3, 0.4),
                roughness: 0.5,
                metallic: 0.0,
            }],
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
        let listed = listed_styles(&app.world().resource::<GameStyles>().clone());
        app.world_mut()
            .spawn((StyleList, rail_list_node()))
            .with_children(|list| {
                for (id, name, colour) in &listed {
                    list.spawn((
                        style_row(id, name, *colour, false, UiSkin::default()),
                        observe(on_style_choice),
                    ));
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
    /// style, which is not what it is showing.
    #[test]
    fn an_unset_style_marks_the_first_style() {
        let mut app = app(true);
        app.update();
        assert_eq!(marked(&mut app), vec!["first".to_string()]);
    }

    /// Pressing a row picks that style, and the mark follows it.
    #[test]
    fn picking_a_style_moves_the_mark_to_it() {
        let mut app = app(true);
        app.update();
        let second = row(&mut app, "second");
        app.world_mut().trigger(Activate { entity: second });
        app.update();

        assert_eq!(ship_node(&app).style, Some("second".to_string()),);
        assert_eq!(marked(&mut app), vec!["second".to_string()]);
    }

    /// The scaffolding style proves the plate pipeline dresses a hull at all.
    /// It is not a look anybody would choose, so a release build leaves it out
    /// of the list - and a debug build is exactly where somebody wants to put
    /// it on a ship and look at it.
    #[test]
    fn the_scaffolding_style_is_listed_only_in_a_debug_build() {
        let styles = GameStyles(vec![style("civilian"), style("placeholder")]);
        let ids: Vec<String> = listed_styles(&styles)
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        if cfg!(feature = "debug") {
            assert_eq!(ids, vec!["civilian", "placeholder"]);
        } else {
            assert_eq!(ids, vec!["civilian"]);
        }
    }

    /// A row shows the paint its style puts on a hull's top surface: five words
    /// for five looks made a builder open each in turn to find out what they
    /// meant.
    #[test]
    fn a_style_row_carries_the_colour_that_style_paints() {
        let listed = listed_styles(&GameStyles(vec![style("civilian")]));
        assert_eq!(
            listed.first().map(|(_, _, colour)| *colour),
            Some(Color::linear_rgb(0.2, 0.3, 0.4))
        );
    }

    /// The colours the rows of the list are wearing, one entry per row.
    fn swatch_alphas(app: &mut App) -> Vec<f32> {
        app.world_mut()
            .query_filtered::<&BackgroundColor, With<StyleSwatch>>()
            .iter(app.world())
            .map(|paint| paint.0.alpha())
            .collect()
    }

    /// With the skin off the list is GREYED rather than hidden: it is the only
    /// thing on the rail that says what ticking the box leads to, and a list
    /// that appears on the tick is a list nobody knew was there.
    #[test]
    fn the_style_list_greys_while_the_ship_is_bare() {
        let mut app = app(false);
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<&Node, With<StyleList>>()
                .single(app.world())
                .expect("the list exists")
                .display,
            Display::Flex,
            "the list stays on screen while the skin is off"
        );
        assert!(
            swatch_alphas(&mut app).iter().all(|alpha| *alpha < 1.0),
            "and wears its colours dimmed: {:?}",
            swatch_alphas(&mut app)
        );
        assert!(
            marked(&mut app).is_empty(),
            "a bare ship wears no style, so no row is marked as worn"
        );

        set_skin(&mut app, true);
        app.update();
        assert!(
            swatch_alphas(&mut app).iter().all(|alpha| *alpha == 1.0),
            "the skin gives the list its colour back: {:?}",
            swatch_alphas(&mut app)
        );
    }

    /// A greyed row is still a row: pressing one turns the skin on and dresses
    /// the ship in it. An advertisement that answers a press with nothing is
    /// worse than no advertisement.
    #[test]
    fn picking_a_style_off_the_greyed_list_turns_the_skin_on() {
        let mut app = app(false);
        app.update();
        let second = row(&mut app, "second");
        app.world_mut().trigger(Activate { entity: second });
        app.update();

        assert!(ship_node(&app).skin, "the press turned the skin on");
        assert_eq!(ship_node(&app).style, Some("second".to_string()));
        assert_eq!(marked(&mut app), vec!["second".to_string()]);
    }

    /// A 150px rail cannot hold `pdc_kinetic_turret_section_7`, and clipping it
    /// dropped the digit that says which turret. The part's NAME goes in the
    /// label, and the ordinal in the column the clip cannot reach.
    #[test]
    fn a_section_row_reads_its_name_with_the_ordinal_beside_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "pdc_kinetic_turret_section_7");
        app.world_mut().insert_resource(EditContext {
            path: vec![scenario, ship],
        });
        app.update();

        assert!(
            row_columns(&mut app).contains(&("hull".to_string(), "7".to_string())),
            "the row reads short, with the ordinal in its own column: {:?}",
            row_columns(&mut app)
        );
        assert!(
            row_names(&mut app).contains(&"pdc_kinetic_turret_section_7".to_string()),
            "and is still NAMED by the node's own id: {:?}",
            row_names(&mut app)
        );
    }

    /// A ship wears the name its builder gave it. The id it is keyed by is one
    /// hover away, and nowhere else: a node that carried two names and showed
    /// only the minted one was a rename nobody could see land.
    #[test]
    fn a_ship_row_reads_as_its_name() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.world_mut().entity_mut(ship).insert(ShipNode {
            name: "Kestrel".to_string(),
            ..default()
        });
        app.update();

        assert!(
            row_columns(&mut app).contains(&("Kestrel".to_string(), String::new())),
            "the authored name reads whole, with nothing in the trailing column: {:?}",
            row_columns(&mut app)
        );
        assert!(
            row_names(&mut app).contains(&"ship_1".to_string()),
            "and the row is still NAMED by the id the document keys on: {:?}",
            row_names(&mut app)
        );
    }

    /// Nothing named it, so the id stands in - minus the ordinal, which goes to
    /// the column a narrow rail cannot clip.
    #[test]
    fn an_unnamed_ship_row_falls_back_to_its_id() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        assert!(
            row_columns(&mut app).contains(&("ship".to_string(), "1".to_string())),
            "{:?}",
            row_columns(&mut app)
        );
    }

    /// A rename is a change to what the tree SAYS and to nothing else it lists,
    /// so a list that compared only ids would go on drawing the old name.
    #[test]
    fn a_renamed_ship_redraws_its_row() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        app.world_mut().entity_mut(ship).insert(ShipNode {
            name: "Kestrel".to_string(),
            ..default()
        });
        app.update();

        assert!(
            row_columns(&mut app).contains(&("Kestrel".to_string(), String::new())),
            "{:?}",
            row_columns(&mut app)
        );
    }

    /// Escape is a back key with five rungs, so the only useful thing it can
    /// say is which one this press takes.
    #[test]
    fn escape_names_the_rung_the_next_press_takes() {
        assert_eq!(escape_rung(true, true, true, true), "cancel the rebind");
        assert_eq!(escape_rung(false, true, true, true), "close the menu");
        assert_eq!(escape_rung(false, false, true, true), "put the part down");
        assert_eq!(escape_rung(false, false, false, true), "leave the ship");
        assert_eq!(escape_rung(false, false, false, false), "pause");
    }

    /// The legend spawns one cell per hint the longest mode needs, and every
    /// mode leaves room for Escape at the end of the line.
    #[test]
    fn every_mode_fits_in_the_cells_the_legend_has() {
        let mut app = legend_app();
        for (choice, inside) in [
            (SectionChoice::None, false),
            (SectionChoice::None, true),
            (SectionChoice::Section("hull".to_string()), true),
        ] {
            *app.world_mut().resource_mut::<SectionChoice>() = choice.clone();
            enter_ship(app.world_mut(), inside);
            app.update();
            let shown = shown_hints(&mut app);
            assert!(
                shown.len() <= LEGEND_CELLS,
                "{choice:?} inside={inside} wants {} cells, the legend has {LEGEND_CELLS}",
                shown.len()
            );
            assert_eq!(
                shown.last().map(|(key, _)| key.as_str()),
                Some("Esc"),
                "the way back is the last hint in every mode: {shown:?}"
            );
        }
    }

    /// The one key that means two things says which one it means now - not by
    /// naming the key twice, but by naming the mode the keys belong to.
    #[test]
    fn the_legend_names_the_mode_its_keys_belong_to() {
        let mut app = legend_app();
        enter_ship(app.world_mut(), true);
        app.update();
        assert_eq!(legend_mode(&mut app), "IN A SHIP");

        *app.world_mut().resource_mut::<SectionChoice>() = SectionChoice::Section("hull".into());
        app.update();
        assert_eq!(legend_mode(&mut app), "PART IN HAND");
    }

    /// A legend with the cells up and the sync running.
    fn legend_app() -> App {
        let mut app = App::new();
        app.init_resource::<SectionChoice>();
        app.init_resource::<EditContext>();
        app.init_resource::<EditorOverlays>();
        app.init_resource::<EditorRebind>();
        app.init_resource::<OpenMenu>();
        app.add_systems(Update, sync_key_legend);
        app.world_mut().spawn((
            EditorKeyLegend,
            Node::default(),
            Children::spawn(SpawnWith(move |cells: &mut RelatedSpawner<ChildOf>| {
                cells.spawn(legend_mode_cell());
                for index in 0..LEGEND_CELLS {
                    cells.spawn(legend_cell(index));
                }
            })),
        ));
        app
    }

    /// Put the context inside a ship, or out at the scenario node.
    fn enter_ship(world: &mut World, inside: bool) {
        let scenario = world.spawn(ScenarioNode).id();
        let mut context = world.resource_mut::<EditContext>();
        context.path = if inside {
            vec![scenario, scenario]
        } else {
            vec![scenario]
        };
    }

    /// Every hint the legend is showing, in the order the line reads.
    fn shown_hints(app: &mut App) -> Vec<(String, String)> {
        let world = app.world_mut();
        let mut cells = world.query::<(&LegendCell, &Node, &Children)>();
        let mut shown: Vec<(usize, (String, String))> = cells
            .iter(world)
            .filter(|(_, node, _)| node.display != Display::None)
            .map(|(cell, _, parts)| {
                let chip = world
                    .get::<Children>(parts[0])
                    .expect("the chip holds its word")[0];
                (
                    cell.0,
                    (
                        world.get::<Text>(chip).expect("the key").0.clone(),
                        world.get::<Text>(parts[1]).expect("the label").0.clone(),
                    ),
                )
            })
            .collect();
        shown.sort_by_key(|(index, _)| *index);
        shown.into_iter().map(|(_, hint)| hint).collect()
    }

    /// The word over the keys.
    fn legend_mode(app: &mut App) -> String {
        let world = app.world_mut();
        let mut modes = world.query_filtered::<&Text, With<LegendMode>>();
        modes.single(world).expect("one mode cell").0.clone()
    }
}
