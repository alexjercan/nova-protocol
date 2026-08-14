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

use bevy::{prelude::*, ui_widgets::observe};
use nova_assets::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{panel, panel_header, separator, themed_button, ButtonValue, UiSkin},
    theme,
};

use crate::{
    config::{EditorKeyLegend, PlacementStatus, SectionChoice},
    gallery::{EditorCamera, EditorChrome, GalleryAction},
    placement::{
        continue_to_simulation, create_new_spaceship, create_new_spaceship_with_controller,
    },
    ui::rail::{category_row, coming_soon_category},
    ExampleStates,
};

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
) {
    let skin = *skin;
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

                rail.spawn(separator());
                rail.spawn((
                    Name::new("Play Button"),
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
            "LMB rebind a section   Q pick its part   RMB+drag look   WASD/Space/Shift fly   \
             Esc pause"
        }
        SectionChoice::Section(_) => {
            "LMB place   wheel roll   Shift+wheel socket   R roll   F socket   Q pick   \
             Esc put down"
        }
        SectionChoice::Delete => "LMB delete   Q pick a part   Esc put down",
    };
    for mut text in &mut legend {
        if text.0 != line {
            text.0 = line.to_string();
        }
    }
}
