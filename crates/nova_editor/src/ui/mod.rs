//! The editor UI: a wiki-inspired left rail of categories plus a component
//! drawer of cards. The theme + shared button widgets live in `nova_ui`; the
//! submodules here hold the editor-specific rail, drawer,
//! cards and hover tooltip, and this module assembles them into the scene.

pub(crate) mod card;
pub(crate) mod drawer;
pub(crate) mod rail;
pub(crate) mod tooltip;

use bevy::{prelude::*, ui_widgets::observe};
use nova_assets::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{panel, panel_header, separator, themed_button, ButtonValue, UiSkin},
    screen::{scroll_column, scroll_viewport},
    theme,
};

use crate::{
    config::SectionChoice,
    gallery::{EditorCamera, EditorChrome, GalleryAction},
    placement::{
        continue_to_simulation, create_new_spaceship, create_new_spaceship_with_controller,
    },
    ui::{
        card::component_card,
        drawer::DrawerPanel,
        rail::{category_row, coming_soon_category, components_category},
    },
    ExampleStates,
};

/// Left rail width (px). Kept narrow so the rail + drawer stay clear of screen
/// centre on the 1024-wide window, where the editor preview ship projects - a
/// UI panel over that point would block the placement raycast.
const RAIL_W: f32 = 150.0;
/// Component drawer width (px). RAIL_W + DRAWER_W = 430 < 512 (half of 1024),
/// so the centred build area stays pickable.
const DRAWER_W: f32 = 280.0;

/// Register the UI's observers (button colours, selection, tooltips). The
/// per-state systems and the `SectionChoice` setting observer are wired by the
/// plugin, which owns those types.
pub(crate) fn register(app: &mut App) {
    // The menu and gameplay want the same app-global UI wiring; whoever gets
    // there first adds it.
    if !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
        app.add_plugins(nova_ui::NovaUiPlugin);
    }
    tooltip::register(app);
}

pub(crate) fn setup_editor_scene(
    mut commands: Commands,
    skin: Res<UiSkin>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
    let skin = *skin;
    commands.spawn((
        DespawnOnExit(ExampleStates::Editor),
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_2,
            0.0,
            0.0,
        )),
        GlobalTransform::default(),
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
                rail.spawn(components_category());
                rail.spawn((
                    Name::new("Parts Gallery Category"),
                    category_row("Parts Gallery"),
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

            root.spawn((
                Name::new("Component Drawer"),
                DrawerPanel,
                Node {
                    width: px(DRAWER_W),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    padding: UiRect::all(px(12)),
                    border: UiRect::right(px(theme::BORDER_W)),
                    ..default()
                },
                BorderColor::all(theme::PHOSPHOR_MUTED),
                BackgroundColor(theme::SPACE),
            ))
            .with_children(|drawer| {
                drawer.spawn(panel_header("Components"));
                drawer
                    .spawn((
                        Name::new("Component List"),
                        scroll_viewport(),
                        Node {
                            align_items: AlignItems::Stretch,
                            ..scroll_column()
                        },
                    ))
                    .with_children(|list| {
                        // Skip sections flagged `hide_in_editor` (the cut-cube
                        // spaceship prototypes) - they only make sense assembled
                        // into a ship, not placed one tile at a time.
                        for section in sections.iter().filter(|s| !s.base.hide_in_editor) {
                            list.spawn(component_card(section));
                        }
                    });
            });
        });
}
