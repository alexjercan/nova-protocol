//! The editor UI: a wiki-inspired left rail of categories plus a component
//! drawer of cards (task 20260714-204219). The theme + shared button widgets now
//! live in `nova_ui`; the submodules here hold the editor-specific rail, drawer,
//! cards and hover tooltip, and this module assembles them into the scene and
//! owns the panel scroll.

pub(crate) mod card;
pub(crate) mod drawer;
pub(crate) mod rail;
pub(crate) mod tooltip;

use bevy::{prelude::*, ui_widgets::observe};
use nova_assets::prelude::*;
use nova_gameplay::prelude::*;
use nova_ui::{
    prelude::{panel_header, separator, themed_button, ButtonValue},
    theme,
};

use crate::{
    config::SectionChoice,
    placement::{
        continue_to_simulation, create_new_spaceship, create_new_spaceship_with_controller,
    },
    ui::{
        card::component_card,
        drawer::DrawerPanel,
        rail::{coming_soon_category, components_category},
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
    nova_ui::widget::register(app);
    tooltip::register(app);
}

/// Marker for a scrollable panel (currently the drawer's card list). Task
/// 20260712-185527.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct EditorScrollPanel;

/// Pixels scrolled per line of wheel movement.
const SCROLL_LINE_HEIGHT: f32 = 20.0;

/// Scroll the editor's scrollable panel with the mouse wheel. Bevy does not
/// scroll `Overflow::Scroll` nodes on its own - a system must drive
/// `ScrollPosition` (bevy ui scroll example pattern). Editor-state only; the
/// WASD camera does not consume the wheel, so there is no zoom conflict.
pub(crate) fn scroll_editor_panel(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut q_panel: Query<&mut ScrollPosition, With<EditorScrollPanel>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let dy: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * SCROLL_LINE_HEIGHT,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if dy == 0.0 {
        return;
    }
    for mut scroll in &mut q_panel {
        // Wheel up (dy > 0) reveals content above -> smaller offset; clamp at the
        // top. Bevy clamps the bottom visually against the content height.
        scroll.0.y = (scroll.0.y - dy).max(0.0);
    }
}

pub(crate) fn setup_editor_scene(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
) {
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
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Direct SkyboxConfig insert (no PendingSkyboxSwap): safe because
        // `game_assets.cubemap` already has its Cube view. `prepare_cubemap_view`
        // (nova_assets) sets it at startup, before any camera spawns, so the bcs
        // SkyboxPlugin observer - which only sets the view on its single-layer
        // fallback branch - sees a ready 6-layer + Cube image and just attaches
        // Skybox. Pinned by prepare_cubemap_view_sets_cube_view_on_the_game_assets_cubemap
        // (task 20260717-133332, which confirmed there is no missing-view bug here).
        SkyboxConfig {
            cubemap: game_assets.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Root"),
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
            // -- Left rail: categories + tools + play --
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
                BorderColor::all(theme::BORDER),
                BackgroundColor(theme::PANEL),
            ))
            .with_children(|rail| {
                rail.spawn((
                    Name::new("Editor Title"),
                    Text::new("EDITOR"),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(theme::TEXT),
                    Node {
                        margin: UiRect::bottom(px(8)),
                        ..default()
                    },
                ));

                rail.spawn(panel_header("Categories"));
                rail.spawn(components_category());
                rail.spawn(coming_soon_category("Ships"));
                rail.spawn(coming_soon_category("Objects"));
                rail.spawn(coming_soon_category("Events"));
                rail.spawn(coming_soon_category("Objectives"));

                rail.spawn(separator());
                rail.spawn(panel_header("Ship"));
                // Names kept exact: the 09_editor / 12_menu_newgame autopilots find
                // these by Name and press them. Display text is free to change.
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
                // Deselect the build/delete tool -> select mode (SectionChoice::None),
                // where clicking a section rebinds its key (task 20260712-183725).
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

            // -- Component drawer: header + scrollable card list --
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
                BorderColor::all(theme::BORDER),
                BackgroundColor(theme::BG),
            ))
            .with_children(|drawer| {
                drawer.spawn(panel_header("Components"));
                drawer
                    .spawn((
                        Name::new("Component List"),
                        EditorScrollPanel,
                        ScrollPosition::default(),
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Stretch,
                            flex_grow: 1.0,
                            overflow: Overflow::scroll_y(),
                            ..default()
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn wheel_scrolls_the_editor_panel_and_clamps_at_the_top() {
        use bevy::input::{
            mouse::{MouseScrollUnit, MouseWheel},
            touch::TouchPhase,
        };

        // Fresh world per case: a re-run `MessageReader` reads the whole buffer,
        // so isolating avoids the first message leaking into the second run.
        fn run_wheel(y: f32, start_y: f32) -> f32 {
            let mut world = World::new();
            world.init_resource::<Messages<MouseWheel>>();
            let panel = world
                .spawn((EditorScrollPanel, ScrollPosition(Vec2::new(0.0, start_y))))
                .id();
            world.write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            world.run_system_once(scroll_editor_panel).unwrap();
            world.entity(panel).get::<ScrollPosition>().unwrap().0.y
        }

        // Wheel down from the top scrolls the panel down (offset grows).
        assert!(
            run_wheel(-3.0, 0.0) > 0.0,
            "wheel down must scroll the panel down"
        );
        // Wheel up past the top clamps the offset at 0.
        assert_eq!(
            run_wheel(100.0, 5.0),
            0.0,
            "scrolling up past the top clamps at 0"
        );
    }
}
