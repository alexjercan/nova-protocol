//! The editor UI: a top bar of context actions over a left rail holding the
//! Scene tree, plus the placement readout. The theme + shared button widgets
//! live in `nova_ui`; `rail` holds the editor-specific rows and this module
//! assembles them into the scene.
//!
//! The layout is split by QUESTION: the top bar answers "where am I and what
//! can I do here" (breadcrumb + per-context actions), the rail answers "what
//! does the document hold" (the tree, and the edited ship's settings), and the
//! `inspector` on the right answers "what is THIS one". Parts are picked in the
//! `gallery`.

pub(crate) mod asset_files;
pub(crate) mod assets;
pub(crate) mod browser;
pub(crate) mod callout;
pub(crate) mod canvas;
pub(crate) mod files;
pub(crate) mod hierarchy;
pub(crate) mod hub;
pub(crate) mod inspector;
pub(crate) mod layer;
pub(crate) mod log;
pub(crate) mod menu;
pub(crate) mod menu_context;
pub(crate) mod pick;
pub(crate) mod picker;
pub(crate) mod plate;
pub(crate) mod rail;
pub(crate) mod settings;
pub(crate) mod shell;
pub(crate) mod stage_menu;
pub(crate) mod thumbnail;
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
use nova_scenario::prelude::ScenarioObjectKind;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{
        button, key_chip, panel, scroll_bar, scroll_column, scroll_row, scroll_viewport, separator,
        tab, tab_strip, text_field, tree_row, ButtonSpec, DoubleClick, InputMode, TextFieldSpec,
        TextFieldValue, Tooltip, TreeRowSpec, UiText,
    },
    theme,
    theme::{density, ActiveUiTheme, UiColor},
    widget::{checkbox_glyph, ButtonCueType, Selected, ThemedCheckbox, ThemedText},
};
use nova_wfc::prelude::{WfcPlan, WfcZone};

use crate::{
    asset::{item_name, AssetNode, AssetPartType, SocketNode},
    bundle::{ask_to_save, ask_to_save_copy},
    camera::EditorRig,
    clipboard::{copy_item, duplicate_item, paste_item},
    config::{
        EditorCamera, EditorKeyLegend, EditorOverlays, EditorRail, EditorSays, EditorStatus,
        EditorViewType, HullPlanLine, PartChoice, PartList, PartTick, PartZoneChip,
        PlacementStatus, PlayButton, SceneList, SceneRow, SelectedNode, ShipReadout,
        ShipReadoutNote, ShipSettings, SkinToggleCheckbox, StyleChoice, StyleList, StyleSwatch,
        ViewTabButton,
    },
    event::{Expanded, ScriptNodes},
    frame::{ask_for, on_frame_selection, on_view_preset, FrameRequest, ViewAngle, ViewPresetItem},
    gallery::{gallery_host, EditorChrome},
    generate::{bow_gun, main_engine, Drawn, HullSeed},
    glyph::{
        category_mark, choice_mark, object_mark, section_mark, ship_mark, INSIDE, OPEN, SCENARIO,
        SHIP_AI, SHUT,
    },
    history::{commit_edit, delete_node, redo_item, undo_item},
    keybind::{on_rebind_action, EditorRebind},
    node::{
        id_order, objects_of, sections_of, split_ordinal, EditContext, NodeId, ObjectChoice,
        ObjectNode, ObjectNodes, ScenarioNode, SectionNode, SectionNodes, ShipNode, ShipNodes,
    },
    placement::{
        create_blank_ship, create_scenario_object, cycle_armed_socket, deletable,
        delete_selected_node, put_armed_part_down, roll_armed_part, DeletableNode,
    },
    project::{
        package::on_package_item,
        verbs::{add_scenario, pressed},
        DocumentType, ModProject,
    },
    selection::{select_all_item, toggling},
    tool::StageTool,
    ui::{
        browser::{spawn_browser, ShipPaneData},
        callout::placement_callout,
        hierarchy::{filtered_rows, on_scene_row_menu, row_range, HierarchyFilter},
        inspector::{inspector_empty, inspector_panel, inspector_tooltip},
        menu::{
            menu_bar_slot, menu_dropdown_node, menu_item_row, menu_scrim, menu_z, on_menu_button,
            on_menu_scrim, palette_block, toggle_all_fields, toggle_handle_space, toggle_ids,
            toggle_key_legend, toggle_link_points, toggle_object_volumes, toggle_world_grid,
            MenuDropdown, MenuHistoryItem, MenuId, MenuLead, MenuTail, OpenMenu, ViewToggle,
        },
        menu_context::{spawn_section_palette, MenuSeparator, MenuVerbType},
        plate::plate_layer,
        rail::{next_zone, row_trash, scene_tooltip, zone_label, SceneRowHint, SceneRowTrash},
        settings::on_mod_settings_item,
        shell::{script_detail, script_mount, spawn_foot, spawn_top_bar, stage},
        window::{window_layer, DestructiveVerb},
    },
    ExampleStates,
};

/// The top bar's title word.
#[derive(Component)]
pub(crate) struct EditorTitle;

/// Say which mod is open, which asset of it, and which ship the editor stands
/// in.
pub(crate) fn sync_editor_title(
    project: Option<Res<ModProject>>,
    context: Res<EditContext>,
    ships: Query<(&ShipNode, &NodeId)>,
    mut titles: Query<&mut Text, With<EditorTitle>>,
) {
    let Some(project) = project else {
        return;
    };
    let name = project.data.meta.name.as_str();
    let name = if name.is_empty() { "New Mod" } else { name };
    let mut line = match project.data.open_name() {
        Some(open) => format!("{name} / {open}"),
        None => name.to_string(),
    };
    if let Some((ship, id)) = context.ship().and_then(|ship| ships.get(ship).ok()) {
        let (label, ordinal) = tree_text(&ship.name, &id.0);
        line = format!("{line} / {} {ordinal}", label)
            .trim_end()
            .to_string();
    }
    for mut title in &mut titles {
        if title.0 != line {
            title.0.clone_from(&line);
        }
    }
}

/// The menu bar. Every entry drops a real list: a greyed placeholder says the
/// editor has menus while answering no press at all.
pub(super) fn spawn_menu_bar(left: &mut RelatedSpawnerCommands<ChildOf>) {
    left.spawn((
        Name::new("Editor Menu Bar"),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
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
                    MenuVerbType::Menu(menu),
                    menu_bar_slot(),
                ))
                .with_children(|slot| {
                    slot.spawn((
                        Name::new(format!("{} Menu Button", menu.label())),
                        menu,
                        button(ButtonSpec::new(menu.label()).dense().ghost()),
                        observe(on_menu_button),
                    ));
                    slot.spawn((
                        Name::new(format!("{} Menu", menu.label())),
                        MenuDropdown,
                        menu,
                        menu_z(),
                        menu_dropdown_node(),
                        panel(),
                    ))
                    .with_children(|items| build_menu(items, menu));
                });
        }
    });
}

/// The view switch: a tab per view of the open document. Every view has its
/// tab here, and [`sync_view_tabs`] shows only those of the open document.
pub(super) fn spawn_view_tabs(bar: &mut RelatedSpawnerCommands<ChildOf>) {
    bar.spawn((Name::new("View Tabs"), tab_strip()))
        .with_children(|strip| {
            for view in EditorViewType::ALL {
                let mut entity = strip.spawn((
                    Name::new(format!("View Tab {}", view.label())),
                    tab(view.label(), false),
                    ViewTabButton(view),
                    ButtonCueType::Owned,
                    observe(on_view_tab),
                ));
                // The mark is the state the row reconciler paints from.
                if view == EditorViewType::default() {
                    entity.insert(Selected);
                }
            }
        });
}

/// File > New Scenario: a blank scenario joins the mod and opens. It loses
/// nothing, so it asks nothing; undo takes it back.
fn on_new_scenario_item(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, add_scenario));
}

/// Fill one dropdown.
///
/// The whole menu bar in one place, so what the editor can do reads as a list
/// rather than as four `with_children` blocks buried in the bar's layout.
///
/// Which rows show, grey or hide is not decided here: each row carries its
/// verb, and `menu_context::verb_state` decides it from the open document and
/// view.
fn build_menu(items: &mut RelatedSpawnerCommands<ChildOf>, menu: MenuId) {
    match menu {
        MenuId::File => {
            // The rows that can lose work do not DO anything: they put the
            // question up (see `crate::ui::window::DestructiveVerb`), and the
            // window's own button carries the verb.
            items.spawn((
                Name::new("New Mod Item"),
                DestructiveVerb::Hub,
                menu_item_row("New Mod", MenuLead::None, MenuTail::None),
            ));
            items.spawn((
                Name::new("Open Mod Item"),
                DestructiveVerb::Hub,
                menu_item_row("Open Mod", MenuLead::None, MenuTail::None),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("New Scenario Item"),
                menu_item_row("New Scenario", MenuLead::None, MenuTail::None),
                ButtonCueType::Owned,
                observe(on_new_scenario_item),
            ));
            items.spawn((
                Name::new("Save Item"),
                menu_item_row("Save", MenuLead::None, MenuTail::Key("Ctrl+S")),
                ButtonCueType::Owned,
                observe(ask_to_save),
            ));
            items.spawn((
                Name::new("Save As Copy Item"),
                menu_item_row("Save As Copy", MenuLead::None, MenuTail::None),
                observe(ask_to_save_copy),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Mod Settings Item"),
                menu_item_row("Mod Settings", MenuLead::None, MenuTail::None),
                observe(on_mod_settings_item),
            ));
            items.spawn((
                Name::new("Package Item"),
                menu_item_row("Package", MenuLead::None, MenuTail::None),
                ButtonCueType::Owned,
                observe(on_package_item),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Back To Main Menu Item"),
                DestructiveVerb::MainMenu,
                menu_item_row("Back to Main Menu", MenuLead::None, MenuTail::None),
            ));
        }
        MenuId::Edit => {
            items.spawn((
                Name::new("Undo Item"),
                MenuHistoryItem::Undo,
                menu_item_row("Undo", MenuLead::None, MenuTail::Key("Ctrl+Z")),
                InteractionDisabled,
                observe(undo_item),
            ));
            items.spawn((
                Name::new("Redo Item"),
                MenuHistoryItem::Redo,
                menu_item_row("Redo", MenuLead::None, MenuTail::Key("Ctrl+Y")),
                InteractionDisabled,
                observe(redo_item),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Copy Item"),
                MenuVerbType::Copy,
                menu_item_row("Copy", MenuLead::None, MenuTail::Key("Ctrl+C")),
                ButtonCueType::Owned,
                observe(copy_item),
            ));
            items.spawn((
                Name::new("Paste Item"),
                MenuVerbType::Paste,
                menu_item_row("Paste", MenuLead::None, MenuTail::Key("Ctrl+V")),
                ButtonCueType::Owned,
                observe(paste_item),
            ));
            items.spawn((
                Name::new("Duplicate Item"),
                MenuVerbType::Duplicate,
                menu_item_row("Duplicate", MenuLead::None, MenuTail::Key("Ctrl+D")),
                ButtonCueType::Owned,
                observe(duplicate_item),
            ));
            items.spawn((
                Name::new("Delete Item"),
                MenuVerbType::Delete,
                menu_item_row("Delete", MenuLead::None, MenuTail::Key("Del")),
                // A delete plays the editor's own removal cue.
                ButtonCueType::Owned,
                observe(delete_selected_node),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Select All Item"),
                MenuVerbType::SelectAll,
                menu_item_row("Select All", MenuLead::None, MenuTail::Key("Ctrl+A")),
                observe(select_all_item),
            ));
        }
        MenuId::View => {
            for (name, label, toggle, key, verb) in [
                ("Hierarchy", "Hierarchy", ViewToggle::Hierarchy, "Ctrl+1", 0),
                ("Inspector", "Inspector", ViewToggle::Inspector, "Ctrl+2", 1),
                ("Browser", "Browser", ViewToggle::Browser, "Ctrl+3", 2),
            ] {
                let mut row = items.spawn((
                    Name::new(format!("{name} Panel Item")),
                    toggle,
                    menu_item_row(label, MenuLead::Toggle, MenuTail::Key(key)),
                ));
                match verb {
                    0 => row.insert(observe(shell::toggle_hierarchy)),
                    1 => row.insert(observe(shell::toggle_inspector)),
                    _ => row.insert(observe(shell::toggle_browser)),
                };
            }
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Handle Axes Item"),
                MenuVerbType::HandleAxes,
                menu_item_row("Local Axes", MenuLead::Toggle, MenuTail::Key("X")),
                ViewToggle::LocalAxes,
                observe(toggle_handle_space),
            ));
            items.spawn((
                Name::new("Key Legend Item"),
                MenuVerbType::KeyLegend,
                ViewToggle::KeyLegend,
                menu_item_row("Key Legend", MenuLead::Toggle, MenuTail::None),
                observe(toggle_key_legend),
            ));
            items.spawn((
                Name::new("Link Points Item"),
                MenuVerbType::LinkPoints,
                ViewToggle::LinkPoints,
                menu_item_row("Link Points", MenuLead::Toggle, MenuTail::None),
                observe(toggle_link_points),
            ));
            items.spawn((
                Name::new("World Grid Item"),
                MenuVerbType::WorldGrid,
                ViewToggle::WorldGrid,
                menu_item_row("World Grid", MenuLead::Toggle, MenuTail::None),
                observe(toggle_world_grid),
            ));
            items.spawn((
                Name::new("Object Volumes Item"),
                MenuVerbType::ObjectVolumes,
                ViewToggle::ObjectVolumes,
                menu_item_row("Object Volumes", MenuLead::Toggle, MenuTail::None),
                observe(toggle_object_volumes),
            ));
            // The inspector's own view, in the menu that holds the stage's:
            // both answer "what am I being shown", and the panel had no other
            // place to say that it is showing a selection rather than
            // everything.
            items.spawn((
                Name::new("All Fields Item"),
                ViewToggle::AllFields,
                menu_item_row("All Fields", MenuLead::Toggle, MenuTail::None),
                observe(toggle_all_fields),
            ));
            // And the tree's: an event names the nodes it fires on by id, so
            // wiring one up is the one job where the names are in the way.
            items.spawn((
                Name::new("Ids Item"),
                MenuVerbType::Ids,
                ViewToggle::Ids,
                menu_item_row("Ids", MenuLead::Toggle, MenuTail::None),
                observe(toggle_ids),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Frame Selection Item"),
                MenuVerbType::Frame,
                menu_item_row("Frame Selection", MenuLead::None, MenuTail::Key("F")),
                observe(on_frame_selection),
            ));
            // The axis views, under the one that frames: they answer the same
            // question - put the camera on this - and differ only in where
            // they answer it from. Sockets are axis-aligned, so this is how a
            // builder checks a mate went where they meant it to.
            for angle in ViewAngle::PRESETS {
                items.spawn((
                    Name::new(format!("{} View Item", angle.label())),
                    MenuVerbType::Frame,
                    ViewPresetItem(angle),
                    menu_item_row(angle.label(), MenuLead::None, MenuTail::None),
                    observe(on_view_preset),
                ));
            }
        }
        MenuId::Ship => {
            // These verbs belong to the ship you are inside, not to the
            // screen, so they hang under the one menu that says so, and the
            // menu shows only in a ship.
            items.spawn((
                Name::new("Parts Item"),
                MenuVerbType::Parts,
                GalleryAction::Open,
                menu_item_row("Parts...", MenuLead::None, MenuTail::Key("Tab")),
            ));
            items.spawn((MenuSeparator, separator()));
            // The pose verbs. They live only in a legend View can switch off,
            // and R/F/wheel are named nowhere else in the editor - so they get
            // rows, greyed until there is a part in hand for them to turn.
            items.spawn((
                Name::new("Roll The Part Item"),
                MenuVerbType::PosePart,
                menu_item_row(
                    "Roll the Part (or wheel)",
                    MenuLead::None,
                    MenuTail::Key("R"),
                ),
                // A pose step plays the editor's own detent cue.
                ButtonCueType::Owned,
                observe(roll_armed_part),
            ));
            items.spawn((
                Name::new("Cycle The Socket Item"),
                MenuVerbType::PosePart,
                menu_item_row(
                    "Cycle the Socket (or Ctrl+wheel)",
                    MenuLead::None,
                    MenuTail::Key("F"),
                ),
                ButtonCueType::Owned,
                observe(cycle_armed_socket),
            ));
            items.spawn((
                Name::new("Put The Part Down Item"),
                MenuVerbType::PosePart,
                menu_item_row("Put the Part Down", MenuLead::None, MenuTail::Key("Esc")),
                observe(put_armed_part_down),
            ));
            items.spawn((MenuSeparator, separator()));
            items.spawn((
                Name::new("Rebind Key Item"),
                MenuVerbType::Rebind,
                menu_item_row("Rebind Key...", MenuLead::None, MenuTail::None),
                observe(on_rebind_action),
            ));
        }
        MenuId::Add => {
            // Add means "one more thing in THIS document", so each document
            // has its own block and `menu_context` shows only the open one's:
            // the scenario's world, its script, a ship's parts, a section's
            // mesh, collider and sockets. A form adds nothing, and the menu
            // hides.
            items
                .spawn((
                    Name::new("Add World Palette"),
                    MenuVerbType::AddWorld,
                    palette_block(),
                ))
                .with_children(|world| {
                    world.spawn((
                        Name::new("Add Ship Button"),
                        // The OUTLINE ship: the row adds a design beside the
                        // one already there. The first ship of an empty
                        // document becomes the player's and the tree fills its
                        // mark in.
                        menu_item_row("New Ship", MenuLead::Glyph(SHIP_AI), MenuTail::None),
                        ButtonCueType::Owned,
                        observe(create_blank_ship),
                    ));
                    world.spawn((MenuSeparator, separator()));
                    for choice in ObjectChoice::ALL {
                        world.spawn((
                            Name::new(format!("Add {}", choice.label())),
                            choice,
                            menu_item_row(
                                choice.label(),
                                MenuLead::Glyph(choice_mark(choice)),
                                MenuTail::None,
                            ),
                            ButtonCueType::Owned,
                            observe(create_scenario_object),
                        ));
                    }
                });
            items
                .spawn((
                    Name::new("Add Part Palette"),
                    MenuVerbType::AddPart,
                    palette_block(),
                ))
                .with_children(|parts| {
                    for category in GalleryCategory::ROW {
                        if category == GalleryCategory::All {
                            continue;
                        }
                        parts.spawn((
                            Name::new(format!("Add {} Item", category.label())),
                            GalleryAction::Browse(category),
                            menu_item_row(
                                &format!("{}...", category.label()),
                                MenuLead::Glyph(category_mark(category)),
                                MenuTail::None,
                            ),
                        ));
                    }
                });
            items
                .spawn((
                    Name::new("Add Script Palette"),
                    MenuVerbType::AddScript,
                    palette_block(),
                ))
                .with_children(|script| {
                    // Blocks are added by dragging them out of the palette;
                    // this row says where a builder used to the menu should go.
                    script.spawn((
                        Name::new("Add Script Hint"),
                        InteractionDisabled,
                        menu_item_row(
                            "Drag blocks from the palette",
                            MenuLead::Glyph(""),
                            MenuTail::Word(""),
                        ),
                    ));
                });
            spawn_section_palette(items);
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
pub(super) const PLAY_LABEL: &str = "Play";
const PLAY_BLOCKED: &str = "Leave the ship first: Play flies the whole scenario.";

/// Left rail width (px). Bounded by screen centre on the narrowest window,
/// where the editor preview ship projects: a UI panel over that point would
/// block the placement raycast. 240 here and 300 of Inspector keep the centre
/// inside the stage down to a 760px window.
pub(crate) const RAIL_W: f32 = 240.0;

/// How many digits the seed field takes: `u64::MAX` is twenty of them, and a
/// field that stops one short would refuse a seed the generator accepts.
pub(super) const SEED_DIGITS: usize = 20;

/// How much of its own colour a style row keeps while the skin is off. Enough
/// to read as the same list, not enough to be mistaken for the live one.
const GREYED_STYLE_ALPHA: f32 = 0.3;

/// Readout type size (px). The block lines its values up on a monospace column,
/// so it must not WRAP: at 11px the longest line (`Turn   5.23 rad/s2`) is
/// 119px, which fit even the old 130px rail and has room to spare in this one.
const READOUT_TEXT: f32 = 11.0;

/// Register the UI's observers (button colours, selection). The per-state
/// systems and the `HeldPart` setting observer are wired by the plugin,
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
    game_assets: Res<GameAssets>,
    styles: Res<GameStyles>,
    sections: Res<GameSections>,
    seed: Res<HullSeed>,
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
) {
    // The rail is built for the ship the editor opens on. With none entered the
    // checkbox starts bare, which is what a fresh ship is.
    let skinned = edited_ship(&context, &q_ships).is_some_and(|ship| ship.skin);
    let ship_pane = ShipPaneData {
        skinned,
        styles: listed_styles(&styles),
        parts: listed_parts(&sections),
        seed: seed.0,
    };
    // Key + rim, the same bearings the parts viewer lights its turntable with.
    // One light shining straight down puts every vertical face of every part in
    // flat shadow - fine for a ship seen from above, wrong for the gallery, where
    // the part IS the tile.
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

    let camera = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Camera"),
            Camera3d::default(),
            PostProcessingCamera,
            EditorRig::default(),
            // Opt in to mesh picking, which the transform gizmo rides on (see
            // `crate::gizmo`). The stage itself is picked through avian's
            // colliders, so this camera answers the pointer twice - once per
            // backend - and the nearer hit wins.
            MeshPickingCamera,
            // The gallery parks this camera on its own stage while it is open, so
            // it needs a handle that does not assume a single Camera3d.
            EditorCamera,
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            // Modelled parts are flat-Kd meshes; a touch of ambient keeps their
            // shadow side readable without washing out the key light's form.
            AmbientLight {
                color: Color::WHITE,
                brightness: 220.0,
                affects_lightmapped_meshes: true,
            },
            // Direct SkyboxConfig insert (no PendingSkyboxSwap) is safe
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
        ))
        .id();
    // The parts gallery lays itself out over the whole window and draws its
    // tiles through this camera, parked on the stage while it is up.
    commands.spawn(gallery_host(camera));

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
            spawn_top_bar(root);

            // The catcher an open menu drops behind itself. A sibling of the
            // bar rather than a child of it, because "anywhere else" includes
            // the rail and the viewport.
            root.spawn((menu_scrim(), observe(on_menu_scrim)));

            // Everything between the toolbar and the browser: the Hierarchy on
            // the left, the stage in the middle, the Inspector on the right.
            root.spawn((
                Name::new("Editor Content"),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
                // The panels claim their rung rather than inheriting one from
                // where they happen to sit in the tree: what the stage hangs
                // on a node is spawned from four other modules, and the rail
                // has to cover all of it. See `crate::ui::layer`.
                GlobalZIndex(layer::CHROME_Z),
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
                        EditorRail,
                        Node {
                            width: px(RAIL_W),
                            // The rail keeps its width: the panels beside it
                            // must not hand their pressure to the tree.
                            flex_shrink: 0.0,
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            row_gap: px(density::GAP),
                            padding: UiRect::all(px(density::PANEL_PAD)),
                            border: UiRect::right(px(theme::BORDER_W)),
                            ..default()
                        },
                        panel(),
                    ))
                    .with_children(|column| {
                        column.spawn((
                            Name::new("Hierarchy Filter"),
                            HierarchyFilter,
                            text_field(
                                TextFieldSpec::new("")
                                    .placeholder("filter the tree")
                                    .max_chars(40)
                                    .tool(),
                            ),
                        ));
                        // The rows are `sync_scene_list`'s, because what is
                        // listed depends on where the editor stands, on the tab
                        // and on the filter.
                        column
                            .spawn((Name::new("Editor Rail Scroll"), scroll_row()))
                            .with_children(|row| {
                                row.spawn((
                                    Name::new("Editor Rail Column"),
                                    Node {
                                        align_items: AlignItems::Stretch,
                                        ..scroll_column()
                                    },
                                    scroll_viewport(),
                                ))
                                .with_child((
                                    Name::new("Scene List"),
                                    SceneList,
                                    rail_list_node(),
                                ));
                                row.spawn((Name::new("Rail Scrollbar"), scroll_bar()));
                            });
                        // The engineer readout: properties of the hull being
                        // built. Pinned under the tree, because it moves with
                        // every part placed and is what a builder watches.
                        column
                            .spawn((
                                Name::new("Ship Settings"),
                                ShipSettings,
                                Node {
                                    width: percent(100),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Stretch,
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                            ))
                            .with_children(|settings| {
                                settings.spawn(separator());
                                settings.spawn((
                                    Name::new("Ship Readout"),
                                    ShipReadout,
                                    UiText,
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(READOUT_TEXT),
                                        ..default()
                                    },
                                    TextColor(Color::NONE),
                                    ThemedText::new(UiColor::Primary),
                                    Node {
                                        margin: UiRect::top(px(4)),
                                        ..default()
                                    },
                                ));
                                // The remedy, under the numbers it is about.
                                settings.spawn((
                                    Name::new("Ship Readout Note"),
                                    ShipReadoutNote,
                                    UiText,
                                    Text::new(""),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::NONE),
                                    ThemedText::new(UiColor::Label),
                                ));
                            });
                    });
                content.spawn(stage());
                content.spawn(script_mount()).with_children(|mount| {
                    canvas::spawn_canvas_area(mount);
                    mount.spawn(script_detail()).with_children(|detail| {
                        detail.spawn(inspector_panel());
                        detail.spawn(inspector_empty());
                    });
                });
            });

            spawn_browser(root, &ship_pane);

            // Contextual (see `sync_key_legend`): a builder holding a part needs
            // the pose keys, and one in select mode needs to be told the
            // pipette exists. One line: `fit_hints` drops what does not fit,
            // and the hints are ordered with the gestures a builder is about to
            // make first. The wrap and the clip are the backstop for a cell the
            // budget got wrong: it loses a whole cell, never half a word.
            spawn_foot(
                root,
                (
                    Name::new("Editor Key Legend"),
                    EditorKeyLegend,
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::FlexEnd,
                        align_items: AlignItems::Center,
                        align_content: AlignContent::FlexStart,
                        column_gap: px(LEGEND_GAP),
                        // Past the clip, so no sliver of the second line shows.
                        row_gap: px(LEGEND_MAX_H),
                        max_height: px(LEGEND_MAX_H),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Children::spawn(SpawnWith(move |cells: &mut RelatedSpawner<ChildOf>| {
                        cells.spawn(legend_mode_cell());
                        for index in 0..LEGEND_CELLS {
                            cells.spawn(legend_cell(index));
                        }
                    })),
                ),
            );

            // Positioned against the screen, so they hang off the root. The
            // windows stand above everything else the editor draws, and the
            // hints above the windows.
            root.spawn(scene_tooltip());
            root.spawn(inspector_tooltip());
            root.spawn(placement_callout());
            root.spawn(plate_layer());
            root.spawn(window_layer());
        });
}

/// Flip the skin on or off.
///
/// One bool on the build state and nothing else: the build view watches it (see
/// [`crate::skin`]) and Play reads it, so the ship on the stage and the ship
/// that flies cannot disagree about whether it is clad.
pub(crate) fn on_skin_toggle(
    _activate: On<Activate>,
    mut commands: Commands,
    context: Res<EditContext>,
    mut q_ships: Query<&mut ShipNode>,
) {
    let Some(ship) = context.ship() else {
        return;
    };
    if let Ok(mut ship) = q_ships.get_mut(ship) {
        ship.skin = !ship.skin;
        commit_edit(&mut commands, "Toggle Skin");
    }
}

/// The style rows to build, out of the MERGED catalog - so a mod's style is
/// listed beside the base ones without the editor knowing any id.
///
/// The colour is the paint the style puts on a hull's TOP surface, which is the
/// face the build view mostly shows. Every style has one - a palette dresses
/// both surfaces that are ever in shot - so the row is never a guessed colour.
fn listed_styles(styles: &GameStyles) -> Vec<(String, String, Color)> {
    styles
        .iter()
        .map(|style| {
            (
                style.id.clone(),
                style.name.clone(),
                style.palette.top.color,
            )
        })
        .collect()
}

/// The rows the Generate block's draw list is built from: every section in the
/// MERGED catalog, and whether the standard plan already draws it.
///
/// The whole catalog rather than the plan's own parts, because a plan that
/// names a part it cannot lay does not BUILD - so a list read off one could
/// never offer the parts a builder most wants to try. Ticking one of those is
/// an experiment the collapse either runs or refuses in a line; both answers
/// are better than a row that does not exist. A mod's section is on the list
/// for the same reason, and starts unticked because nothing prices it.
///
/// The standard plan's own draw starts ticked, so pressing Generate without
/// touching the list rolls the ship the base game rolls.
fn listed_parts(sections: &GameSections) -> Vec<(String, String, bool, Option<WfcZone>)> {
    let base = WfcPlan::standard_hull();
    let drawn = base.parts.as_slice();
    sections
        .iter()
        .map(|section| {
            let priced = drawn.iter().find(|part| part.prototype == section.base.id);
            (
                section.base.id.clone(),
                section.base.name.clone(),
                priced.is_some(),
                priced.and_then(|part| part.zone),
            )
        })
        .collect()
}

/// Tick or untick one part of the draw.
///
/// The row IS the setting - see [`PartChoice`] - so the press writes the mark
/// and nothing else has to be told.
pub(crate) fn on_part_choice(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<Has<Selected>, With<PartChoice>>,
) {
    let Ok(ticked) = rows.get(activate.entity) else {
        return;
    };
    if ticked {
        commands.entity(activate.entity).remove::<Selected>();
    } else {
        commands.entity(activate.entity).insert(Selected);
    }
}

/// Cycle one part's ZONE: where on the hull the collapse may stand it.
///
/// The chip is inside the row, and both are buttons. This is safe because
/// [`on_part_choice`] above rules on `activate.entity` - the button the press
/// names - rather than on the entity the observer happens to hang off, so a
/// press on the chip is not also a press on the tick.
pub(crate) fn on_part_zone(
    activate: On<Activate>,
    chips: Query<&ChildOf, With<PartZoneChip>>,
    mut rows: Query<&mut PartChoice>,
) {
    let Ok(parent) = chips.get(activate.entity) else {
        return;
    };
    let Ok(mut choice) = rows.get_mut(parent.parent()) else {
        return;
    };
    choice.zone = next_zone(choice.zone);
}

/// Write each part row's zone chip from its mark: the label, and whether the
/// chip is there at all.
///
/// Shown only while the row is ticked, for the reason [`zone_chip`] gives. Like
/// [`sync_part_ticks`] this compares before writing rather than gating on a
/// change, because the rows are spawned on entering the editor, which need not
/// be a frame anything was ticked on.
pub(crate) fn sync_part_zones(
    rows: Query<(&PartChoice, Has<Selected>, &Children)>,
    mut chips: Query<(&mut Node, &Children), With<PartZoneChip>>,
    mut labels: Query<&mut Text, Without<PartZoneChip>>,
) {
    for (choice, ticked, children) in &rows {
        for child in children {
            let Ok((mut node, chip)) = chips.get_mut(*child) else {
                continue;
            };
            let display = if ticked { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
            for label in chip {
                if let Ok(mut text) = labels.get_mut(*label) {
                    let wanted = zone_label(choice.zone);
                    if text.0 != wanted {
                        text.0 = wanted.to_string();
                    }
                }
            }
        }
    }
}

/// True while the GENERATE block is on screen.
///
/// The block is spawned with the rail and hidden with `Display::None`, so the
/// query behind it is never empty and only the context says whether anything
/// there would be read. See [`sync_context_panels`], which is what shows it.
pub(crate) fn a_ship_is_entered(context: Res<EditContext>) -> bool {
    context.ship().is_some()
}

/// Write the HULL PLAN line: which prototype fills each SEEDED role of the
/// hull the ticks add up to.
///
/// The seeded roles are not in the draw list and cannot be: a role is one part,
/// and the list is a set of many. They are DERIVED from the ticks - the biggest
/// ticked drive is the main engine, the biggest ticked spinal gun is the bow
/// gun - so a builder gets them without learning a second control, and this
/// line is what tells them it happened.
///
/// GATED on the block being shown, unlike the compare-and-write reconcilers
/// beside it: this walks every ticked row and formats a string, which is too
/// much to build each frame for a line that is not on screen.
pub(crate) fn sync_hull_plan(
    sections: Option<Res<GameSections>>,
    rows: Query<(&PartChoice, Has<Selected>)>,
    mut lines: Query<&mut Text, With<HullPlanLine>>,
) {
    let Some(sections) = sections.as_deref() else {
        return;
    };
    let drawn: Vec<Drawn> = rows
        .iter()
        .filter(|(_, ticked)| *ticked)
        .map(|(choice, _)| Drawn {
            prototype: choice.prototype.clone(),
            zone: choice.zone,
        })
        .collect();
    let named = |id: Option<&str>| {
        id.and_then(|id| sections.get_section(id))
            .map_or_else(|| "-".to_string(), |config| config.base.name.clone())
    };
    // The two roles the TICKS decide. The keel and the bridge are the standard
    // plan's and are the same on every hull, so naming them here would cost two
    // lines of a narrow rail to say nothing.
    let stern = WfcPlan::standard_hull().keel.stern_drive;
    let plan = format!(
        "stern {}\nbow {}",
        named(main_engine(sections, &drawn).as_deref().or(Some(&stern))),
        named(bow_gun(sections, &drawn).as_deref()),
    );
    for mut text in &mut lines {
        if text.0 != plan {
            text.0 = plan.clone();
        }
    }
}

/// Write each part row's tick glyph from its mark.
///
/// The shared reconciler paints the row's fill and border off `Selected`; the
/// glyph is this list's own, because a highlight alone reads as "the one
/// selected" and this list is a set of many.
///
/// Compared before writing rather than gated on a change, for the same reason
/// as [`sync_style_list`]: the rows are spawned on entering the editor, which
/// need not be a frame anything was ticked on.
pub(crate) fn sync_part_ticks(
    rows: Query<(Has<Selected>, &Children), With<PartChoice>>,
    mut ticks: Query<&mut Text, With<PartTick>>,
) {
    for (ticked, children) in &rows {
        for child in children {
            if let Ok(mut text) = ticks.get_mut(*child) {
                let glyph = checkbox_glyph(ticked);
                if text.0 != glyph {
                    text.0 = glyph.to_string();
                }
            }
        }
    }
}

/// The style list's own column, so the rows read as one group under the toggle
/// rather than as four more tools.
fn rail_list_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Column,
        // The rail SCROLLS. A child that may shrink hands the column a second
        // way to fit its content, and flexbox takes it: the draw list made the
        // whole rail overflow, and every block above it - the tree included -
        // was squeezed until its rows were unclickable rather than scrolled
        // past.
        flex_shrink: 0.0,
        ..default()
    }
}

/// One row the Scene tree wants: the node it points at, and how it reads.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WantedRow {
    pub(crate) node: Entity,
    /// How deep under the scenario root the node sits. The row spends it on
    /// left padding, which is what makes the list read as a tree.
    pub(crate) depth: usize,
    /// Which way the caret points. `None` on a row with nothing under it.
    pub(crate) open: Option<bool>,
    /// How many rows hang directly under this one, open or shut.
    pub(crate) count: usize,
    /// The glyph in front of the label, saying what the node is. `@` marks the
    /// node the editor is inside, `>` the ship Play hands to the player, `-` a
    /// design beside it, and a section wears its kind.
    pub(crate) lead: String,
    /// The node's id: what the row is CALLED, and what the driven walks find
    /// it by.
    pub(crate) id: String,
    /// The label before elision, which is what a filter reads.
    pub(crate) full: String,
    /// What the row READS as. The id, shortened where the id repeats what the
    /// tree already says (see [`tree_text`]).
    pub(crate) label: String,
    /// The row's right-hand column: which one this is, when the id ends in an
    /// ordinal. Held apart from the label because it is the half that must
    /// survive a narrow rail (see [`tree_text`]).
    pub(crate) trail: String,
    /// What the row's icon MEANS, in one word. Read back by the hover hint,
    /// which is where a builder finds out what `%` was.
    pub(crate) kind: String,
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
    /// What a root row fits beside its caret, its mark and its count.
    const AT_ROOT: usize = 24;
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

/// Show the view the pressed tab stands for.
///
/// The SELECTION goes with it: a node marked in one view is not in the other's
/// list, and a mark the tree cannot draw is one the Inspector would still be
/// reporting from a panel nothing on screen points at.
pub(crate) fn on_view_tab(
    activate: On<Activate>,
    tabs: Query<&ViewTabButton>,
    mut view: ResMut<EditorViewType>,
    mut selected: ResMut<SelectedNode>,
    mut says: EditorSays,
) {
    let Ok(ViewTabButton(pressed)) = tabs.get(activate.entity) else {
        return;
    };
    if *view == *pressed {
        return;
    }
    *view = *pressed;
    selected.set(None);
    says.accept();
}

/// Show the tabs of the open document's views, and mark the one on screen.
///
/// Compared before writing rather than gated on a change, for the same reason
/// as [`sync_style_list`]: the tabs are spawned on entering the editor, which
/// need not be a frame the view changed on.
pub(crate) fn sync_view_tabs(
    mut commands: Commands,
    view: Res<EditorViewType>,
    project: Option<Res<ModProject>>,
    context: Res<EditContext>,
    mut tabs: Query<(Entity, &ViewTabButton, &mut Node, Has<Selected>)>,
) {
    let open = project
        .as_ref()
        .and_then(|project| project.data.open.as_ref());
    let views = DocumentType::of(open, context.ship().is_some()).views();
    for (entity, ViewTabButton(this), mut node, marked) in &mut tabs {
        let display = if views.contains(this) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        match (*this == *view, marked) {
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
    script: &ScriptNodes,
    bench: &BenchTree,
    ids: bool,
    all: bool,
) -> Vec<WantedRow> {
    let Some(scenario) = context.scenario() else {
        return Vec::new();
    };
    if let Some(rows) = bench.rows(scenario) {
        return rows;
    }
    let Ok(root_id) = q_scenarios.get(scenario) else {
        return Vec::new();
    };
    let mut rows = vec![scenario_row(scenario, &root_id.0)];

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
        let (glyph, kind) = ship_mark(node);
        let (label, ordinal) = tree_text(&node.name, &id.0);
        let inside = entered == Some(ship);
        // ONE walk for both: this row's count and the rows under it are the
        // same list, and the walk sorts every section node in the document.
        let sections = sections_of(ship, nodes);
        let parts = sections.len();
        let open = inside || all || script.expanded(ship);
        rows.push(WantedRow {
            node: ship,
            depth: 1,
            open: (parts > 0).then_some(open),
            count: parts,
            lead: glyph.to_string(),
            id: id.0.clone(),
            label: elide(&label, label_budget(1)),
            full: label,
            trail: if inside { INSIDE.to_string() } else { ordinal },
            kind: match (inside, parts) {
                (true, _) => format!("{kind} - EDITING"),
                (false, 0) => format!("{kind} - EMPTY"),
                (false, 1) => format!("{kind} - 1 PART"),
                (false, parts) => format!("{kind} - {parts} PARTS"),
            },
        });
        if !open {
            continue;
        }
        for (section, id, node, _) in sections {
            // The ORDINAL comes off the id even where the part is named,
            // unlike a ship: six reinforced hulls share one name, and the
            // number is the only thing telling them apart.
            let (stem, trail) = tree_text("", &id.0);
            let label = section_name(node, catalog).unwrap_or(stem);
            let (glyph, kind) = section_mark(node, catalog);
            rows.push(WantedRow {
                node: section,
                depth: 2,
                open: None,
                count: 0,
                lead: glyph.to_string(),
                id: id.0.clone(),
                label: elide(&label, label_budget(2)),
                full: label,
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
    // HULLS FIRST, with the built ships they stand beside. A seeded spacecraft
    // is a ship that a scenario stores as an object, and filing it between the
    // rocks and the beacons made the range read as scenery. Each half keeps its
    // id order.
    let hulled = |node: &ObjectNode| matches!(node.kind, ScenarioObjectKind::Spaceship(_));
    let world = world
        .iter()
        .filter(|(_, _, node, _)| hulled(node))
        .chain(world.iter().filter(|(_, _, node, _)| !hulled(node)));
    for &(object, id, node, _) in world {
        let (label, trail) = tree_text(&node.name, &id.0);
        let (glyph, kind) = object_mark(node);
        rows.push(WantedRow {
            node: object,
            depth: 1,
            open: None,
            count: 0,
            lead: glyph.to_string(),
            id: id.0.clone(),
            label: elide(&label, label_budget(1)),
            full: label,
            trail,
            kind: kind.to_string(),
        });
    }
    // The root counts what hangs under it, which the walk above just listed.
    rows[0].count = rows.iter().filter(|row| row.depth == 1).count();
    show_ids(&mut rows, ids, label_budget);
    rows
}

/// The document root, which both halves of the tree are hung under: it is the
/// row that says which range this is, and the way back out of a ship.
/// What the tree reads of the bench.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct BenchTree<'w, 's> {
    assets: Query<'w, 's, &'static AssetNode>,
    parts: Query<'w, 's, (Entity, &'static AssetPartType, &'static ChildOf)>,
    sockets: Query<
        'w,
        's,
        (
            Entity,
            &'static NodeId,
            &'static SocketNode,
            &'static ChildOf,
        ),
    >,
}

impl BenchTree<'_, '_> {
    /// The rows of the item under `root`: the item, then a section's collider,
    /// mesh and sockets. `None` when `root` holds a scenario.
    fn rows(&self, root: Entity) -> Option<Vec<WantedRow>> {
        let item = &self.assets.get(root).ok()?.0;
        let leaf = |node: Entity, lead: &str, id: &str, label: &str, kind: &str| WantedRow {
            node,
            depth: 1,
            open: None,
            count: 0,
            lead: lead.to_string(),
            id: id.to_string(),
            label: elide(label, label_budget(1)),
            full: label.to_string(),
            trail: String::new(),
            kind: kind.to_string(),
        };
        let mut rows = Vec::new();
        let mut parts: Vec<_> = self
            .parts
            .iter()
            .filter(|(_, _, owner)| owner.parent() == root)
            .collect();
        parts.sort_by_key(|(_, part, _)| AssetPartType::ALL.iter().position(|all| all == *part));
        for (node, part, _) in parts {
            rows.push(leaf(node, "#", part.id(), part.label(), "PART"));
        }
        let mut sockets: Vec<_> = self
            .sockets
            .iter()
            .filter(|(.., owner)| owner.parent() == root)
            .collect();
        sockets.sort_by_key(|(_, _, socket, _)| socket.order);
        for (node, id, ..) in sockets {
            rows.push(leaf(node, "o", &id.0, &id.0, "SOCKET"));
        }
        let name = item_name(item);
        let head = WantedRow {
            node: root,
            depth: 0,
            open: (!rows.is_empty()).then_some(true),
            count: rows.len(),
            lead: SCENARIO.to_string(),
            id: item.id().to_string(),
            label: elide(&name, label_budget(0)),
            full: name,
            trail: String::new(),
            kind: item.kind().to_uppercase(),
        };
        rows.insert(0, head);
        Some(rows)
    }
}

fn scenario_row(scenario: Entity, id: &str) -> WantedRow {
    let (label, trail) = tree_text("", id);
    WantedRow {
        node: scenario,
        depth: 0,
        open: None,
        count: 0,
        lead: SCENARIO.to_string(),
        id: id.to_string(),
        label: elide(&label, label_budget(0)),
        full: label,
        trail,
        kind: "SCENARIO".to_string(),
    }
}

/// View > Ids, applied to the finished list rather than threaded through every
/// arm that built one: the id is the same string whatever made the row.
///
/// The trail goes with the name - the ordinal it holds is the tail of the id
/// now in the label - except the mark that says which ship you are inside,
/// which no id carries.
fn show_ids(rows: &mut [WantedRow], ids: bool, budget: fn(usize) -> usize) {
    if !ids {
        return;
    }
    for row in rows {
        row.label = elide(&row.id, budget(row.depth));
        if row.trail != INSIDE {
            row.trail = String::new();
        }
    }
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
    context: Res<EditContext>,
    catalog: Option<Res<GameSections>>,
    overlays: Res<EditorOverlays>,
    mut selected: ResMut<SelectedNode>,
    q_scenarios: Query<&NodeId, With<ScenarioNode>>,
    q_ships: ShipNodes,
    q_objects: ObjectNodes,
    nodes: SectionNodes,
    (script, bench): (ScriptNodes, BenchTree),
    lists: Query<Entity, With<SceneList>>,
    fresh: Query<(), Added<SceneList>>,
    rows: Query<(Entity, &SceneRow, Has<Selected>)>,
    filters: Query<&TextFieldValue, With<HierarchyFilter>>,
    mut shown: Local<ShownScene>,
) {
    // A fresh list holds no rows whatever this Local remembers: the list dies
    // with the editor scene (DespawnOnExit) while a `Local` survives the
    // state round-trip, so a Play-and-return handed an unchanged document to
    // an empty list the signature compare would never refill.
    if !fresh.is_empty() {
        shown.rows.clear();
    }
    let filter = filters
        .iter()
        .next()
        .map(|value| value.0.trim().to_lowercase())
        .unwrap_or_default();
    // A filter reads through every fold: a match under a shut row is still a
    // match, and it has to be found.
    let all = !filter.is_empty();
    let wanted = wanted_rows(
        &context,
        &q_scenarios,
        &q_ships,
        &q_objects,
        &nodes,
        catalog.as_deref(),
        &script,
        &bench,
        overlays.ids,
        all,
    );
    // A selection cannot outlive its row: a section of a ship that was left is
    // not in the tree, so there is nothing left to carry the mark.
    // A block on the canvas has no row of its own.
    let listed = |node: &Entity| wanted.iter().any(|row| row.node == *node) || script.holds(*node);
    if !selected.iter().all(|node| listed(&node)) {
        selected.retain(listed);
    }
    let Ok(list) = lists.single() else {
        return;
    };
    // After the selection pass: a filter hides a row, not the mark on it.
    let wanted = if all {
        filtered_rows(wanted, &filter)
    } else {
        wanted
    };

    // The WHOLE row is the signature, names included: a rename changes nothing
    // about which nodes are listed, and a list that only compared ids would
    // keep drawing the old name until something else moved.
    if shown.rows != wanted {
        commands.entity(list).despawn_related::<Children>();
        commands.entity(list).with_children(|list| {
            for row in &wanted {
                // Marked from the start rather than waiting for the pass
                // below: these rows do not exist in `rows` until next frame,
                // and a highlight that lags a frame behind the click that made
                // it reads as a dropped input.
                let marked = selected.contains(row.node);
                let spec = TreeRowSpec {
                    depth: row.depth,
                    caret: match row.open {
                        Some(true) => OPEN.to_string(),
                        Some(false) => SHUT.to_string(),
                        None => String::new(),
                    },
                    lead: row.lead.clone(),
                    label: row.label.clone(),
                    count: match row.count {
                        0 => String::new(),
                        count => count.to_string(),
                    },
                    trail: row.trail.clone(),
                };
                let mut entity = list.spawn((
                    // Named by ID, drawn by LABEL: the walks and the probe
                    // find a row by the node's own key, whatever the rail has
                    // room to print.
                    Name::new(format!("Scene Row {}", row.id)),
                    tree_row(&spec),
                    SceneRow(row.node),
                    // What a hover reveals: the kind the icon stands for, and
                    // the id the row had to clip.
                    SceneRowHint {
                        kind: row.kind.clone(),
                        id: row.id.clone(),
                        open: row.open,
                        lead: row.lead.clone(),
                    },
                    observe(on_scene_row),
                    observe(on_scene_row_menu),
                ));
                if marked {
                    entity.insert(Selected);
                }
                // The row's own delete, on a child: a press on the trash must
                // not read as a press on the row it sits in.
                entity.with_child((row_trash(), observe(on_row_trash)));
            }
        });
        shown.rows = wanted;
        // The rows just queued are not in `rows` yet, and the ones that are
        // have been queued for despawn. Marking either is a write to an entity
        // that is about to stop existing.
        return;
    }

    for (entity, row, marked) in &rows {
        match (selected.contains(row.0), marked) {
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
    nodes: Query<(), DeletableNode>,
    rows: Query<(&SceneRow, &Hovered, &Children)>,
    mut trash: Query<&mut Node, With<SceneRowTrash>>,
) {
    for (row, hovered, children) in &rows {
        let show =
            (hovered.get() || selected.contains(row.0)) && deletable(row.0, &context, &nodes);
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
    nodes: Query<(), DeletableNode>,
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
    delete_node(&mut commands, row.0);
    selected.set(None);
}

/// One click SELECTS a row and puts the camera on it; two ENTER it; one on a
/// row you are already INSIDE comes back out to it.
///
/// The single click is the cheap, reversible one, so it is the one that
/// answers everywhere: every row selects, and the camera goes to whatever was
/// named. Only the second click goes IN - because entering is the gesture that
/// hides the rest of the document behind a breadcrumb.
///
/// Coming OUT is not that gesture and never was. A row for a node the editor
/// is inside cannot mean "go there", so one click means "come back out to
/// there", at whatever depth. It used to frame instead, which read as the
/// click having done nothing.
///
/// An earlier version entered on the first click, so that a container never
/// needed a double (owner: a double-click here had read as "the first click
/// did nothing"). It does something now: it frames.
pub(crate) fn on_scene_row(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&SceneRow>,
    ships: Query<(), With<ShipNode>>,
    script: ScriptNodes,
    scenarios: Query<(), With<ScenarioNode>>,
    placed: Query<(), With<Transform>>,
    time: Res<Time<Real>>,
    mut last: ResMut<DoubleClick<Entity>>,
    mut selected: ResMut<SelectedNode>,
    mut context: ResMut<EditContext>,
    mut request: ResMut<FrameRequest>,
    input: (
        Option<Res<ButtonInput<KeyCode>>>,
        Option<Res<ButtonInput<MouseButton>>>,
    ),
    tree: (
        Query<&Children, With<SceneList>>,
        Query<&ChildOf>,
        Query<(), With<SectionNode>>,
    ),
) {
    let Ok(SceneRow(node)) = rows.get(activate.entity) else {
        return;
    };
    let (keys, mouse) = input;
    // `Button` activates on any button. The right one is the context menu's.
    if mouse.is_some_and(|mouse| mouse.just_released(MouseButton::Right)) {
        return;
    }
    let (list, owners, sections) = tree;
    // A section of a ship the editor is not in: the row is the way in, whatever
    // the modifiers say, because a part can only be marked from inside.
    if sections.contains(*node) {
        let owner = owners
            .iter_ancestors(*node)
            .find(|ancestor| ships.contains(*ancestor));
        if let Some(ship) = owner.filter(|ship| context.ship() != Some(*ship)) {
            context.enter(ship);
            selected.set(Some(*node));
            request.node = None;
            return;
        }
    }
    // Shift marks the run from the last mark to this row, in the order shown.
    // A row on the path the editor stands on is a way out, which has no second
    // meaning, so a run steps over it.
    let shift = keys
        .as_deref()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    if let Some(from) = selected.primary().filter(|_| shift) {
        if !context.path.contains(node) {
            let order: Vec<Entity> = list
                .iter()
                .flatten()
                .filter_map(|row| rows.get(*row).ok().map(|row| row.0))
                .collect();
            let run = row_range(&order, from, *node);
            selected.set_all(run.into_iter().filter(|row| !context.path.contains(row)));
            return;
        }
    }
    // Ctrl or Shift on a row marks it and goes nowhere.
    if toggling(keys.as_deref()) && !context.path.contains(node) {
        selected.toggle(*node);
        return;
    }
    let double = last.press(*node, time.elapsed_secs());
    // A click that CHANGES the context hands the camera to
    // `crate::node::sync_camera_focus`, which frames whatever was entered - so
    // the request the first half of the double raised steps aside rather than
    // writing the camera a second time in the same frame.
    if context.leave_to(*node) {
        selected.set(None);
        request.node = None;
        return;
    }
    if scenarios.contains(*node) {
        // The root, and the editor is already standing on it: framing it is
        // what "show me everything" means, and the Inspector falls back to it.
        ask_for(&mut request, Some(*node));
        selected.set(None);
        return;
    }
    if double && ships.contains(*node) {
        context.enter(*node);
        selected.set(None);
        request.node = None;
        return;
    }
    // The script's own "go in": a second click OPENS a container rather than
    // entering it. A handler is not a place the editor can stand - there is no
    // camera to put on it and no breadcrumb to come back by - so what the
    // gesture means here is showing what is under it.
    if double && script.holds(*node) {
        if script.expanded(*node) {
            commands.entity(*node).remove::<Expanded>();
        } else {
            commands.entity(*node).insert(Expanded);
        }
    }
    selected.set(Some(*node));
    // A handler is not SOMEWHERE. The script's nodes carry no pose, so their
    // rows select and leave the camera where the builder put it, rather than
    // raising a request the framing pass can only throw away.
    if placed.contains(*node) {
        ask_for(&mut request, Some(*node));
    }
}

/// Paint whatever the editor is currently saying onto its one status line.
///
/// One WRITER, so the placement readout and a verb's answer cannot fight over
/// the node: they both write [`EditorStatus`], and this is the only thing that
/// touches the text. It also holds the clock: a verb's message is dropped here
/// once its hold is over, so every other reader sees the line as it reads.
pub(crate) fn sync_status_line(
    time: Res<Time>,
    theme: Res<ActiveUiTheme>,
    mut status: ResMut<EditorStatus>,
    lines: Query<(&mut Text, &mut TextColor, &mut Visibility), With<PlacementStatus>>,
) {
    status.expire(time.elapsed_secs_f64());
    let line = status.line();
    for (mut text, mut colour, mut visibility) in lines {
        match line {
            Some((message, tint)) => {
                let tint = theme.color(tint);
                if text.0 != message {
                    text.0 = message.to_string();
                }
                if colour.0 != tint {
                    colour.0 = tint;
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
    mut buttons: Query<(Entity, Has<InteractionDisabled>, &mut Tooltip), With<PlayButton>>,
) {
    let disabled = context.ship().is_some();
    for (entity, marked, mut tooltip) in &mut buttons {
        let wanted = if disabled { PLAY_BLOCKED } else { "" };
        if tooltip.0 != wanted {
            tooltip.0 = wanted.to_string();
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

/// Show the rail's ship blocks - the settings and the generator - only inside
/// a ship.
///
/// Hidden rather than disabled, unlike Play: a greyed skin toggle at the
/// scenario node would say "this exists here and is refused", and it does not
/// exist there - a skin is a thing a SHIP has. The Ship menu follows the same
/// rule (see `crate::ui::menu_context`).
///
/// Generate obeys the same rule for the same reason: what it rolls IS a hull,
/// and a hull is what a ship is made of.
pub(crate) fn sync_context_panels(
    context: Res<EditContext>,
    mut panels: Query<&mut Node, With<ShipSettings>>,
) {
    let shown = if context.ship().is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut panels {
        if node.display != shown {
            node.display = shown;
        }
    }
}

/// Pick the style this row names.
///
/// Writes an explicit id rather than a list index: the build state travels out
/// to the scenario and back, and an index into a catalog a mod can grow would
/// not survive that trip meaning the same thing.
///
/// Picking a style off the greyed list also turns the skin ON. The list is
/// visible while the skin is off so it can advertise what the toggle leads to,
/// and an advertisement that answers a press with nothing is worse than no
/// advertisement.
pub(crate) fn on_style_choice(
    activate: On<Activate>,
    mut commands: Commands,
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
        commit_edit(&mut commands, "Pick Style");
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
    theme: Res<ActiveUiTheme>,
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
    styles: Res<GameStyles>,
    rows: Query<(Entity, &StyleChoice, Has<Selected>, &Children)>,
    mut labels: Query<&mut TextColor>,
    mut swatches: Query<(&StyleSwatch, &mut BackgroundColor)>,
) {
    let ship = edited_ship(&context, &q_ships);
    let skinned = ship.is_some_and(|ship| ship.skin);
    let label = theme.color(if skinned {
        UiColor::Primary
    } else {
        UiColor::Label
    });
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

/// Write the skin checkbox's STATE from the ship it reports on.
///
/// Writes the state rather than the paint: `ThemedCheckbox` is the one input
/// nova_ui's checkbox reconciler paints from, so the box cannot drift from the
/// `checkbox()` factory the rail built it with, and it follows a theme flip
/// with every other control.
///
/// Compared before writing rather than gated on a change, for the same reason
/// as [`sync_key_legend`]: the row is spawned on entering the editor, which
/// need not be a frame the toggle changed on. The compare also keeps the
/// reconciler's `Changed` gate honest.
pub(crate) fn sync_skin_toggle(
    context: Res<EditContext>,
    q_ships: Query<&ShipNode>,
    mut boxes: Query<&mut ThemedCheckbox, With<SkinToggleCheckbox>>,
) {
    let on = ThemedCheckbox(edited_ship(&context, &q_ships).is_some_and(|ship| ship.skin));
    for mut state in &mut boxes {
        if *state != on {
            *state = on;
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

/// How tall the legend may grow, in logical pixels: three rows of cells and the
/// gaps between them, which is what the stock 1024x768 shape already draws.
const LEGEND_MAX_H: f32 = 22.0;

/// The width a hint cell holds even when its words are shorter, so the hints
/// line up into columns rather than into a paragraph.
///
/// A FLOOR, not a fixed width: the line wraps inside the gap between the rail
/// and the Inspector, so a hint too long for its column widens its own cell
/// instead of pushing the line off the window.
const LEGEND_CELL_W: f32 = 100.0;

/// The gap between legend cells.
const LEGEND_GAP: f32 = 10.0;

/// The legend's 12 px word advance. The UI font is monospace.
const LEGEND_WORD_ADVANCE: f32 = 6.0;

/// The advance of a key on its chip, which is 2 px smaller than the words.
const LEGEND_KEY_ADVANCE: f32 = 5.0;

/// What a hint spends besides its characters: the chip's margin, padding and
/// border, and the gap to its words.
const LEGEND_CHIP_W: f32 = 8.0 + 2.0 * (5.0 + theme::BORDER_W) + 6.0;

/// What the mode word's own margin adds to its characters.
const LEGEND_MODE_MARGIN: f32 = 4.0;

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
            margin: UiRect::right(px(LEGEND_MODE_MARGIN)),
            ..default()
        },
        UiText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Accent),
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
                UiText,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
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

/// The width a hint's cell takes on the line, its gap included.
fn hint_width((key, label): Hint) -> f32 {
    let words = key.chars().count() as f32 * LEGEND_KEY_ADVANCE
        + label.chars().count() as f32 * LEGEND_WORD_ADVANCE;
    (LEGEND_CHIP_W + words).max(LEGEND_CELL_W) + LEGEND_GAP
}

/// Keep the hints that fit `room` beside the mode word and the Escape cell.
///
/// The hints are in priority order, so a narrow foot loses the last ones. Escape
/// is budgeted first: it is the way back. A `room` of zero is a line not laid out
/// yet, and keeps them all.
fn fit_hints(hints: &mut Vec<Hint>, escape: Hint, mode: &str, room: f32) {
    if room <= 0.0 {
        return;
    }
    let mut used =
        mode.chars().count() as f32 * LEGEND_WORD_ADVANCE + LEGEND_MODE_MARGIN + hint_width(escape);
    let kept = hints
        .iter()
        .take_while(|hint| {
            used += hint_width(**hint);
            used <= room
        })
        .count();
    hints.truncate(kept);
}

/// What the next Escape would take.
///
/// Escape has several rungs and the legend used to name one of them. It is a
/// back key, so the only useful thing it can say is what THIS press does - which
/// is a fact the keyboard's owner and the ladder in `crate::escape_backs_out`
/// already decide and nothing else on screen reports.
fn escape_rung(
    keyboard: InputMode,
    popup: bool,
    menu_open: bool,
    armed: bool,
    inside_ship: bool,
) -> &'static str {
    match (keyboard, popup, menu_open, armed, inside_ship) {
        (InputMode::Bind, ..) => "cancel the rebind",
        (InputMode::Insert, ..) => "leave the field",
        (_, true, ..) => "cancel the pop-up",
        (_, _, true, ..) => "close the menu",
        (_, _, _, true, _) => "put the part down",
        (.., true) => "leave the ship",
        _ => "pause",
    }
}

/// The Events line for this keyboard owner.
///
/// `Del` and `F` go with the same [`InputMode`] that gates their systems, so the
/// line cannot offer a verb the gate holds back. A pop-up's catcher also takes
/// every press outside it, so none of the canvas gestures reach the canvas.
fn events_line(keyboard: InputMode, popup: bool) -> (&'static str, Vec<Hint>) {
    if popup {
        return ("POP-UP", vec![("click away", "keep it")]);
    }
    let verbs = keyboard == InputMode::Normal;
    let hints = [
        (true, ("drag", "move block")),
        (true, ("LMB", "select")),
        (verbs, ("Del", "delete")),
        (true, ("MMB", "pan")),
        (true, ("Ctrl+Wheel", "zoom")),
        (verbs, ("F", "frame")),
    ];
    let shown = hints.into_iter().filter(|(live, _)| *live);
    ("EVENTS", shown.map(|(_, hint)| hint).collect())
}

/// Keep the key legend in step with the armed tool AND the edit context.
///
/// Keyed on both because the same keys mean different things per level: at the
/// scenario node there are no parts to arm and Escape falls through to pause,
/// while inside a ship Tab browses parts and Escape backs out one rung.
///
/// WHAT IS HERE is what no other surface can carry: the pointer gestures and
/// the viewport camera, which belong to no row and no menu, plus the rung the next
/// Escape takes. Every verb a menu row can name lives on that row with its key
/// beside it.
///
/// Compared before writing rather than gated on a change: the legend is
/// spawned on entering the editor, which is not necessarily a frame the tool
/// or the context changed on.
///
/// View > Key Legend hides it. The line is still kept current while hidden -
/// it costs a few string compares, and turning the legend back on has to show
/// what the editor is doing NOW.
pub(crate) fn sync_key_legend(
    selection: Res<HeldPart>,
    context: Res<EditContext>,
    overlays: Res<EditorOverlays>,
    rebind: Res<EditorRebind>,
    open_menu: Res<OpenMenu>,
    tool: Res<StageTool>,
    rail: Res<EditorViewType>,
    keyboard: Res<InputMode>,
    block_popup: Res<canvas::BlockPopup>,
    legend: Query<(&mut Node, &Children, &ComputedNode), With<EditorKeyLegend>>,
    mut modes: Query<&mut Text, With<LegendMode>>,
    cells: Query<(&LegendCell, &Children)>,
    chips: Query<&Children, With<LegendChip>>,
    mut nodes: Query<&mut Node, Without<EditorKeyLegend>>,
    mut texts: Query<&mut Text, Without<LegendMode>>,
) {
    let inside = context.ship().is_some();
    let popup = block_popup.is_open();
    let escape = escape_rung(
        *keyboard,
        popup,
        open_menu.0.is_some(),
        *selection != HeldPart(None),
        inside,
    );
    // The pointer and the rig, then the one key that changes meaning with the
    // rung it is on. Ordered so the gestures a builder is about to make come
    // first and the way out comes last.
    //
    // Bind FIRST, because it is a mode and the rest are tools. It takes every
    // key except Escape, so a legend that still read `Bksp leave` would be
    // advertising a verb that now binds itself to a thruster.
    let tool = tool.tool.hint();
    let (mode, mut hints): (&str, Vec<Hint>) = if rebind.target.is_some() {
        ("REBINDING", vec![("any key", "bind it")])
    } else if rail.is_events() {
        events_line(*keyboard, popup)
    } else if *rail == EditorViewType::Form {
        (
            "FORM",
            vec![
                ("LMB", "edit a field"),
                ("Enter", "keep it"),
                ("Ctrl+Z", "undo"),
            ],
        )
    } else {
        match (&*selection, inside) {
            (HeldPart(None), false) => (
                "SELECT",
                vec![
                    ("LMB", "select"),
                    ("LMB x2", "enter"),
                    ("drag", "move it"),
                    ("QWER", tool),
                    ("MMB", "orbit"),
                    ("Wheel", "dolly"),
                ],
            ),
            (HeldPart(None), true) => (
                "IN A SHIP",
                vec![
                    ("LMB", "select"),
                    // The key, not the gesture: leaving is a single click on the
                    // scenario row now, which the tree shows and does not need a
                    // legend for. What the legend owes is the key that does it
                    // without the rail - see `backspace_steps_out`.
                    ("Bksp", "leave"),
                    ("P", "pick a part"),
                    ("MMB", "orbit"),
                    ("Wheel", "dolly"),
                ],
            ),
            (HeldPart(Some(_)), _) => (
                "PART IN HAND",
                vec![
                    ("LMB", "place it"),
                    ("P", "pick a part"),
                    ("Wheel", "roll it"),
                    ("MMB", "orbit"),
                ],
            ),
        }
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
    let Ok((mut root, children, computed)) = legend.single_inner() else {
        return;
    };
    // The line's own width: the foot shares the row with the status line, and
    // the UI scale is already in it. It does not follow the hints, so a drop
    // here cannot feed back.
    let room = computed.size().x * computed.inverse_scale_factor();
    fit_hints(&mut hints, ("Esc", escape), mode, room);
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

    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::{
        AIControllerConfig, DebugMessageActionConfig, EntityFilterConfig, EventActionConfig,
        EventConfig, EventFilterConfig, ScenarioEventConfig, SectionSource, SpaceshipConfig,
        SpaceshipController,
    };
    use nova_ship::prelude::ShipStyleConfig;
    use nova_ui::prelude::TreeCaret;

    use super::*;
    use crate::{
        generate::UNAUTHORED_WEIGHT,
        glyph::{SHIP_AI, SHIP_PLAYER},
        node::{NextChildOrdinal, ObjectChoice, ShipDriver},
        placement::continue_to_simulation,
        ui::{inspector::PANEL_W as INSPECTOR_W, rail::style_row},
    };

    /// The narrowest window the editor is built for. The driven walks run at
    /// this size, and the placement raycast they aim with goes at its centre.
    const NARROW: f32 = 1024.0;

    /// Both docked panels are free to grow, but not through the middle of the
    /// screen: a panel over the centre eats the placement ray and the walk
    /// stops being able to put a part on the hull.
    #[test]
    fn the_docked_panels_leave_the_stage_its_centre() {
        let centre = NARROW / 2.0;
        assert!(
            RAIL_W < centre,
            "the rail reaches {RAIL_W}px, past the centre at {centre}px"
        );
        assert!(
            NARROW - INSPECTOR_W > centre,
            "the Inspector starts at {}px, before the centre at {centre}px",
            NARROW - INSPECTOR_W
        );
    }

    /// A rail with the Scene tree on it and the reconciler running, over an
    /// empty document. The tests below fill the document in.
    fn scene_app() -> App {
        let mut app = App::new();
        app.init_resource::<ActiveUiTheme>();
        app.init_resource::<SelectedNode>();
        app.init_resource::<EditorOverlays>();
        app.init_resource::<EditorViewType>();
        app.init_resource::<DoubleClick<Entity>>();
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
                ScenarioNode::default(),
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
                // A ship node stands somewhere - the lowering reads its pose -
                // so the fixture gives it one: a row's framing asks whether the
                // node it names is placed at all.
                Transform::default(),
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
                let (wrapper, trail) = (children.iter().nth(2)?, children.iter().nth(4)?);
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
        let asked = app.world().resource::<FrameRequest>().node;
        app.world_mut().resource_mut::<FrameRequest>().node = None;
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
                let lead = app.world().get::<Children>(row)?.iter().nth(1)?;
                Some(app.world().get::<Text>(lead)?.0.clone())
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

    /// Lift `events` under `scenario` as the document's script.
    fn script(app: &mut App, scenario: Entity, events: Vec<ScenarioEventConfig>) {
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                crate::event::lift(&mut commands, scenario, events.clone());
            })
            .expect("the lift runs");
    }

    /// A handler with one filter and one action.
    fn handler(name: EventConfig, id: &str) -> ScenarioEventConfig {
        ScenarioEventConfig {
            label: None,
            position: None,
            name,
            once: false,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some(id.to_string()),
                ..default()
            })],
            actions: vec![EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: id.to_string(),
            })],
        }
    }

    /// A ship opens from the scenario level without going in: the caret is a
    /// look at what it holds, not a change of where the editor stands.
    #[test]
    fn a_caret_press_opens_a_ship_row_and_a_second_shuts_it() {
        let mut app = scene_app();
        app.add_observer(hierarchy::on_tree_caret);
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Ai);
        section_node(&mut app, ship, "hull_1");
        app.update();
        assert_eq!(row_names(&mut app), vec!["scenario", "ship_1"]);

        press_caret(&mut app, ship);
        assert_eq!(row_names(&mut app), vec!["scenario", "ship_1", "hull_1"]);
        assert_eq!(app.world().resource::<EditContext>().ship(), None);

        press_caret(&mut app, ship);
        assert_eq!(row_names(&mut app), vec!["scenario", "ship_1"]);
    }

    /// The node of each row, in draw order.
    fn row_nodes(app: &mut App) -> Vec<Entity> {
        let list = app
            .world_mut()
            .query_filtered::<Entity, With<SceneList>>()
            .single(app.world())
            .expect("one scene list");
        app.world()
            .get::<Children>(list)
            .map(|children| {
                children
                    .iter()
                    .filter_map(|row| app.world().get::<SceneRow>(row).map(|row| row.0))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A press on the caret of `node`'s row.
    fn press_caret(app: &mut App, node: Entity) {
        let row = row_for(app, node);
        let caret = app
            .world()
            .get::<Children>(row)
            .expect("a row has columns")
            .iter()
            .find(|child| app.world().get::<TreeCaret>(*child).is_some())
            .expect("the row has a caret");
        app.world_mut().trigger(Activate { entity: caret });
        app.world_mut().flush();
        app.update();
    }

    /// The filter finds a part under a shut ship, and takes its parents with it.
    #[test]
    fn a_filter_reads_through_a_shut_row() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Ai);
        section_node(&mut app, ship, "hull_1");
        spawn_ship(&mut app, scenario, "ship_2", ShipDriver::Ai);
        app.world_mut()
            .spawn((HierarchyFilter, TextFieldValue("hull_1".to_string())));
        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "hull_1"],
            "the other ship has no match, and the shut one opens for its part"
        );
    }

    /// `Button` activates on every mouse button, and the right one means the
    /// row's menu: a selection or a way into the ship on top of it would be a
    /// second verb from one click.
    #[test]
    fn a_right_click_is_not_a_press_on_the_row() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Ai);
        app.update();
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Right);
        mouse.release(MouseButton::Right);
        app.world_mut().insert_resource(mouse);

        let row = row_for(&mut app, ship);
        app.world_mut().trigger(Activate { entity: row });
        app.world_mut().flush();

        assert!(app.world().resource::<SelectedNode>().is_empty());
    }

    /// Switching tabs drops the mark: a node marked in one half is not in the
    /// other's list, and the Inspector would otherwise be reporting a node
    /// nothing on screen points at.
    #[test]
    fn switching_tabs_drops_a_selection_the_other_half_cannot_show() {
        let mut app = scene_app();
        app.init_resource::<EditorStatus>().init_resource::<Time>();
        app.add_systems(Update, sync_view_tabs);
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        script(
            &mut app,
            scenario,
            vec![handler(EventConfig::OnDestroyed, "ship_1")],
        );
        let tab = app
            .world_mut()
            .spawn((
                ViewTabButton(EditorViewType::Events),
                Name::new("View Tab Events"),
                Node::default(),
                observe(on_view_tab),
            ))
            .id();
        app.update();

        app.world_mut()
            .resource_mut::<SelectedNode>()
            .set(Some(ship));
        app.world_mut().trigger(Activate { entity: tab });
        app.update();

        assert_eq!(
            *app.world().resource::<EditorViewType>(),
            EditorViewType::Events
        );
        assert_eq!(app.world().resource::<SelectedNode>().primary(), None);
        assert!(
            app.world().get::<Selected>(tab).is_some(),
            "and the tab that was pressed is the one marked"
        );
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

    /// The tree draws what a builder CALLED a thing, and an event fires on
    /// what the document calls it. View > Ids is the one switch between them.
    #[test]
    fn the_tree_can_be_read_as_the_ids_an_event_would_name() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.world_mut()
            .entity_mut(ship)
            .get_mut::<ShipNode>()
            .expect("the ship node")
            .name = "Warden".to_string();

        app.update();
        assert!(
            row_columns(&mut app).contains(&("Warden".to_string(), String::new())),
            "named, by default: {:?}",
            row_columns(&mut app)
        );

        app.world_mut().resource_mut::<EditorOverlays>().ids = true;
        app.update();
        assert!(
            row_columns(&mut app).contains(&("ship_1".to_string(), String::new())),
            "and by id on request: {:?}",
            row_columns(&mut app)
        );
    }

    /// One world object, owned by the scenario.
    fn spawn_object(app: &mut App, scenario: Entity, id: &str, kind: ScenarioObjectKind) -> Entity {
        app.world_mut()
            .spawn((
                ObjectNode {
                    name: String::new(),
                    kind,
                },
                NodeId(id.to_string()),
                Transform::default(),
                ChildOf(scenario),
            ))
            .id()
    }

    /// A picket is a ship a scenario happens to store as an object. Filed
    /// between the rocks and the beacons, wearing a generic object mark, the
    /// range read as scenery next to the thing being built.
    #[test]
    fn a_seeded_hull_stands_with_the_ships_and_not_with_the_rocks() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        spawn_object(
            &mut app,
            scenario,
            "asteroid_1",
            ObjectChoice::Asteroid.stock().kind,
        );
        spawn_object(
            &mut app,
            scenario,
            "spaceship_1",
            ScenarioObjectKind::Spaceship(SpaceshipConfig {
                controller: SpaceshipController::AI(AIControllerConfig::default()),
                ..default()
            }),
        );

        app.update();
        assert_eq!(
            row_names(&mut app),
            vec!["scenario", "ship_1", "spaceship_1", "asteroid_1"],
            "the picket belongs above the rock, whatever id order says"
        );
        assert_eq!(
            row_leads(&mut app),
            vec![
                SCENARIO,
                SHIP_PLAYER,
                SHIP_AI,
                object_mark_of(&mut app, "asteroid_1")
            ],
            "a picket wears the AI ship's mark, not an object's"
        );
        assert!(
            row_hints(&mut app).contains(&"SHIP - AI".to_string()),
            "and the hover says ship too: {:?}",
            row_hints(&mut app)
        );
    }

    /// The mark the object with `id` is drawn with, read from the function the
    /// row is built by rather than restated as a literal.
    fn object_mark_of(app: &mut App, id: &str) -> &'static str {
        let node = app
            .world_mut()
            .query::<(&NodeId, &ObjectNode)>()
            .iter(app.world())
            .find(|(node_id, _)| node_id.0 == id)
            .expect("that object")
            .1
            .clone();
        object_mark(&node).0
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
    /// count says how much is inside, and the hover hint says what it counts.
    #[test]
    fn a_ship_row_says_how_many_parts_are_folded_up_inside_it() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        let ship = spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        section_node(&mut app, ship, "hull_section_1");
        section_node(&mut app, ship, "hull_section_2");
        app.update();

        let counts: Vec<String> = row_nodes(&mut app)
            .into_iter()
            .filter_map(|node| {
                let row = row_for(&mut app, node);
                let count = app.world().get::<Children>(row)?.iter().nth(3)?;
                Some(app.world().get::<Text>(count)?.0.clone())
            })
            .collect();
        assert_eq!(
            counts,
            ["1", "2"],
            "the root counts its ship, the ship its parts"
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

        assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(ship));
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
            app.world().resource::<SelectedNode>().primary(),
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
        assert_eq!(
            app.world().resource::<SelectedNode>().primary(),
            Some(second)
        );
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
        assert_eq!(app.world().resource::<SelectedNode>().primary(), Some(ship));
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

        assert_eq!(
            app.world().resource::<SelectedNode>().primary(),
            Some(section)
        );
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            Some(ship),
            "selecting a section does not move the context"
        );
    }

    /// The root row is the way back, on ONE click, and it lands at the scenario
    /// node rather than outside the document.
    ///
    /// A row for something you are already inside cannot mean "go there". It
    /// used to frame instead, and a builder who pressed the thing they wanted
    /// to get back to watched the camera pull out without leaving.
    #[test]
    fn the_root_row_leaves_the_ship_on_one_click() {
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

    /// Standing ON the scenario node, its own row still frames the whole stage:
    /// there is nothing to come out of, and "show me everything" is what the
    /// root's bounds mean.
    #[test]
    fn the_root_row_frames_the_stage_from_the_root() {
        let mut app = scene_app();
        let scenario = document(&mut app);
        spawn_ship(&mut app, scenario, "ship_1", ShipDriver::Player);
        app.update();

        let root = row_for(&mut app, scenario);
        press(&mut app, root);

        assert_eq!(framed(&mut app), Some(scenario));
        assert_eq!(app.world().resource::<EditContext>().ship(), None);
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
        assert_eq!(
            app.world().resource::<SelectedNode>().primary(),
            Some(section)
        );

        app.world_mut().resource_mut::<EditContext>().exit();
        app.update();
        assert_eq!(app.world().resource::<SelectedNode>().primary(), None);
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
            .spawn((PlayButton, Name::new("Play Button"), Tooltip::default()))
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

        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.update();
        assert!(
            app.world().entity(button).contains::<InteractionDisabled>(),
            "inside a ship it is greyed"
        );
        assert_eq!(
            app.world().get::<Tooltip>(button).unwrap().0,
            PLAY_BLOCKED,
            "a greyed verb says why in its tooltip; the label stays Play"
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

    /// The rail's ship settings are a ship's own: there is no skin to toggle
    /// at the scenario node, so the block is not there either.
    #[test]
    fn the_ship_settings_block_belongs_to_the_entered_ship() {
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<EditContext>();
        let scenario = world.spawn(ScenarioNode::default()).id();
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

    fn style(id: &str) -> ShipStyleConfig {
        ShipStyleConfig {
            id: id.to_string(),
            name: id.to_string(),
            palette: StylePalette {
                top: SurfaceFinish {
                    color: Color::linear_rgb(0.2, 0.3, 0.4),
                    roughness: 0.5,
                    metallic: 0.0,
                },
                wall: default(),
            },
            fixtures: vec![],
            greeble_rules: default(),
        }
    }

    /// The rail as `sync_style_list` sees it: the list node and one row per
    /// style, wired to the real observer so a press goes the way a click does.
    fn app(clad: bool) -> App {
        let mut app = App::new();
        // `sync_style_list` inks its rows from the live theme; `NovaUiPlugin`
        // owns this resource in production and this harness adds no plugins.
        app.init_resource::<ActiveUiTheme>();
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
                    list.spawn((style_row(id, name, *colour), observe(on_style_choice)));
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

    /// Escape is a back key with several rungs, so the only useful thing it
    /// can say is which one this press takes.
    #[test]
    fn escape_names_the_rung_the_next_press_takes() {
        let rung = |keyboard, popup, menu, armed, inside| {
            escape_rung(keyboard, popup, menu, armed, inside)
        };
        assert_eq!(
            rung(InputMode::Bind, true, true, true, true),
            "cancel the rebind"
        );
        assert_eq!(
            rung(InputMode::Insert, true, true, true, true),
            "leave the field"
        );
        assert_eq!(
            rung(InputMode::Browse, true, true, true, true),
            "cancel the pop-up"
        );
        assert_eq!(
            rung(InputMode::Normal, false, true, true, true),
            "close the menu"
        );
        assert_eq!(
            rung(InputMode::Normal, false, false, true, true),
            "put the part down"
        );
        assert_eq!(
            rung(InputMode::Normal, false, false, false, true),
            "leave the ship"
        );
        assert_eq!(rung(InputMode::Normal, false, false, false, false), "pause");
    }

    /// The legend under a pop-up offers only what its keys and clicks do.
    #[test]
    fn a_block_popup_takes_the_canvas_hints_off_the_legend() {
        let keys = |line: (&str, Vec<Hint>)| -> Vec<&str> {
            line.1.into_iter().map(|(key, _)| key).collect()
        };
        assert_eq!(
            keys(events_line(InputMode::Normal, false)),
            ["drag", "LMB", "Del", "MMB", "Ctrl+Wheel", "F"]
        );
        assert_eq!(
            keys(events_line(InputMode::Insert, false)),
            ["drag", "LMB", "MMB", "Ctrl+Wheel"],
            "a field holds the keys, so the verbs go with it"
        );
        let (mode, hints) = events_line(InputMode::Browse, true);
        assert_eq!(mode, "POP-UP");
        assert_eq!(hints, [("click away", "keep it")]);
    }

    /// The legend spawns one cell per hint the longest mode needs, and every
    /// mode leaves room for Escape at the end of the line.
    #[test]
    fn every_mode_fits_in_the_cells_the_legend_has() {
        let mut app = legend_app();
        for (choice, inside) in [
            (HeldPart(None), false),
            (HeldPart(None), true),
            (HeldPart(Some("hull".to_string())), true),
        ] {
            *app.world_mut().resource_mut::<HeldPart>() = choice.clone();
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

        *app.world_mut().resource_mut::<HeldPart>() = HeldPart(Some("hull".into()));
        app.update();
        assert_eq!(legend_mode(&mut app), "PART IN HAND");
    }

    /// Bind mode takes every key except Escape, so the legend stops advertising
    /// the ones it took. `Bksp leave` was the worst of them: pressing it bound
    /// Backspace to the thruster instead of leaving the ship, with nothing on
    /// screen saying either thing had happened.
    #[test]
    fn the_legend_stops_advertising_keys_a_rebind_has_taken() {
        let mut app = legend_app();
        enter_ship(app.world_mut(), true);
        app.update();
        assert!(
            shown_hints(&mut app).iter().any(|(key, _)| key == "Bksp"),
            "the ship legend offers Backspace to begin with"
        );

        let section = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<EditorRebind>().target = Some(section);
        *app.world_mut().resource_mut::<InputMode>() = InputMode::Bind;
        app.update();

        assert_eq!(legend_mode(&mut app), "REBINDING");
        assert_eq!(
            shown_hints(&mut app),
            vec![
                ("any key".to_string(), "bind it".to_string()),
                ("Esc".to_string(), "cancel the rebind".to_string()),
            ],
            "only the two keys that still mean what they say"
        );
    }

    /// The foot has room for less than the line wants. The last hints give way
    /// and Escape stays: at 1280 and at 1920 the wrap used to drop Escape first,
    /// because it is the last cell.
    #[test]
    fn a_narrow_foot_loses_the_last_hints_and_keeps_the_way_back() {
        let line: Vec<Hint> = vec![
            ("LMB", "select"),
            ("LMB x2", "enter"),
            ("drag", "move it"),
            ("QWER", "tool: move"),
            ("MMB", "orbit"),
        ];
        let escape = ("Esc", "pause");
        let budget = |room: f32| {
            let mut hints = line.clone();
            fit_hints(&mut hints, escape, "SELECT", room);
            hints.iter().map(|hint| hint.0).collect::<Vec<_>>()
        };

        assert_eq!(budget(486.0), ["LMB", "LMB x2", "drag"]);
        assert_eq!(budget(140.0), Vec::<&str>::new());
        assert_eq!(budget(2000.0).len(), line.len());
        assert_eq!(budget(0.0).len(), line.len(), "not laid out yet");
    }

    /// A legend with the cells up and the sync running.
    fn legend_app() -> App {
        let mut app = App::new();
        app.init_resource::<HeldPart>();
        app.init_resource::<EditContext>();
        app.init_resource::<EditorOverlays>();
        app.init_resource::<EditorRebind>();
        app.init_resource::<OpenMenu>();
        app.init_resource::<EditorViewType>();
        app.init_resource::<StageTool>();
        app.init_resource::<InputMode>();
        app.init_resource::<canvas::BlockPopup>();
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
        let scenario = world.spawn(ScenarioNode::default()).id();
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

    /// The list a builder is handed: the whole merged catalog, with the
    /// standard plan's own draw already ticked - so pressing Generate without
    /// touching it rolls the ship the base game rolls.
    #[test]
    fn the_draw_list_offers_the_catalog_and_ticks_the_standard_plans_own() {
        let sections = GameSections(nova_authoring::generation::build_section_catalog());
        let listed = listed_parts(&sections);

        assert_eq!(
            listed.len(),
            sections.len(),
            "every section in the merged catalog gets a row"
        );
        let ticked: Vec<&str> = listed
            .iter()
            .filter(|(_, _, drawn, _)| *drawn)
            .map(|(id, _, _, _)| id.as_str())
            .collect();
        for part in &WfcPlan::standard_hull().parts {
            assert!(
                ticked.contains(&part.prototype.as_str()),
                "'{}' is in the standard draw, so its row starts ticked",
                part.prototype
            );
        }
        assert!(
            listed
                .iter()
                .any(|(id, _, drawn, _)| id == "railgun_lance_section" && !drawn),
            "the lance is on the list and starts untouched, which is the row a \
             builder came for"
        );
        assert!(
            listed.iter().all(|(_, name, _, _)| !name.is_empty()),
            "a row nobody can read is a row nobody ticks"
        );
    }

    /// A mod's own section is on the draw list, unticked, and ticking it puts
    /// it into the plan the next Generate collapses.
    ///
    /// The whole point of the list surviving the grammar catalog's removal: the
    /// plan is code now, so nothing prices a section the base game never
    /// shipped - and a row a builder cannot reach would make a mod's part
    /// undrawable rather than merely untuned.
    #[test]
    fn a_mod_section_joins_the_draw_list_unticked_and_can_still_be_drawn() {
        let mut catalog = nova_authoring::generation::build_section_catalog();
        let mut modded = catalog[0].clone();
        modded.base.id = "mod_spar_section".to_string();
        modded.base.name = "Mod Spar".to_string();
        catalog.push(modded);
        let sections = GameSections(catalog);

        let listed = listed_parts(&sections);
        let row = listed
            .iter()
            .find(|(id, ..)| id == "mod_spar_section")
            .expect("a section the merge loaded is a row");
        assert!(
            !row.2,
            "nothing in the standard plan prices it, so it starts unticked"
        );

        let plan = crate::generate::drawn_plan(
            &sections,
            &[Drawn {
                prototype: "mod_spar_section".to_string(),
                zone: None,
            }],
        )
        .expect("one ticked row is a draw");
        assert_eq!(
            plan.parts
                .iter()
                .map(|part| (part.prototype.as_str(), part.weight))
                .collect::<Vec<_>>(),
            vec![("mod_spar_section", UNAUTHORED_WEIGHT)],
            "ticking it draws it, at the weight the block's note states"
        );
    }

    /// Pressing a row ticks it, pressing it again unticks it, and the glyph
    /// follows. A set is not a selection: ticking one does not clear the rest.
    #[test]
    fn a_part_row_ticks_and_unticks_without_disturbing_the_others() {
        let mut app = App::new();
        app.add_systems(Update, sync_part_ticks);
        app.add_observer(on_part_choice);
        let rows: Vec<Entity> = ["one", "two"]
            .iter()
            .map(|id| {
                let row = app
                    .world_mut()
                    .spawn((
                        PartChoice {
                            prototype: (*id).to_string(),
                            zone: None,
                        },
                        Children::default(),
                    ))
                    .id();
                app.world_mut()
                    .spawn((PartTick, Text::new(""), ChildOf(row)));
                row
            })
            .collect();
        // The first starts ticked, as a shipped part's row does.
        app.world_mut().entity_mut(rows[0]).insert(Selected);
        app.update();

        let glyph = |app: &mut App, row: Entity| -> String {
            let child = app.world().get::<Children>(row).expect("a tick child")[0];
            app.world().get::<Text>(child).expect("the glyph").0.clone()
        };
        assert_eq!(glyph(&mut app, rows[0]), checkbox_glyph(true));
        assert_eq!(glyph(&mut app, rows[1]), checkbox_glyph(false));

        app.world_mut().trigger(Activate { entity: rows[1] });
        app.update();
        assert_eq!(glyph(&mut app, rows[1]), checkbox_glyph(true));
        assert_eq!(
            glyph(&mut app, rows[0]),
            checkbox_glyph(true),
            "ticking one part did not untick the other: this is a set, not a choice"
        );

        app.world_mut().trigger(Activate { entity: rows[0] });
        app.update();
        assert_eq!(glyph(&mut app, rows[0]), checkbox_glyph(false));
        assert_eq!(glyph(&mut app, rows[1]), checkbox_glyph(true));
    }

    /// The chip cycles the ZONE without flipping the tick under it, and it is
    /// only there while the row is ticked.
    ///
    /// The chip is a button inside a button, which is the whole reason this
    /// test exists: `on_part_choice` rules on the entity the activation NAMES,
    /// so a press on the chip is not also a press on the row.
    #[test]
    fn a_zone_chip_cycles_its_row_without_ticking_it() {
        let mut app = App::new();
        app.add_systems(Update, sync_part_zones);
        app.add_observer(on_part_choice);
        app.add_observer(on_part_zone);
        let row = app
            .world_mut()
            .spawn((
                PartChoice {
                    prototype: "one".to_string(),
                    zone: None,
                },
                Selected,
                Children::default(),
            ))
            .id();
        let chip = app
            .world_mut()
            .spawn((PartZoneChip, Node::default(), ChildOf(row)))
            .id();
        let label = app.world_mut().spawn((Text::new(""), ChildOf(chip))).id();
        app.update();

        let read = |app: &App| app.world().get::<Text>(label).expect("the label").0.clone();
        assert_eq!(read(&app), zone_label(None));
        assert!(app.world().get::<Selected>(row).is_some());

        app.world_mut().trigger(Activate { entity: chip });
        app.update();
        assert_eq!(read(&app), zone_label(Some(WfcZone::Bow)));
        assert!(
            app.world().get::<Selected>(row).is_some(),
            "the press that cycled the zone did not untick the row under it"
        );

        // All the way round and back to unzoned.
        let mut zone = Some(WfcZone::Bow);
        for _ in 0..6 {
            app.world_mut().trigger(Activate { entity: chip });
            app.update();
            zone = next_zone(zone);
            assert_eq!(read(&app), zone_label(zone));
        }
        assert_eq!(zone, None, "the cycle comes back round to anywhere");

        // Untick the row and the chip goes away: a zone on a part the collapse
        // may not draw says nothing.
        app.world_mut().trigger(Activate { entity: row });
        app.update();
        assert!(app.world().get::<Selected>(row).is_none());
        assert_eq!(
            app.world().get::<Node>(chip).expect("the chip").display,
            Display::None
        );
    }
}
