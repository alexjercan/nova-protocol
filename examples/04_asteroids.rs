use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Slider, SliderRange, SliderThumb, SliderValue, ValueChange},
};
use clap::Parser;
use nova_core::nova_scenario::objects::asteroid::PlanetHeight;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "04_asteroids")]
#[command(version = "1.0.0")]
#[command(about = "A simple example showing how to create a basic asteroid in nova_protocol", long_about = None)]
struct Cli;

fn main() {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    app.run();
}

fn custom_plugin(app: &mut App) {
    app.insert_resource(DemoWidgetStates {
        zoom_scale: 0.1,
        seed: 0.0,
    });
    app.insert_resource(PlanetHeight::default());

    app.add_systems(
        OnEnter(GameStates::Playing),
        (setup_scenario, setup_ui_slider),
    );
    app.add_systems(
        Update,
        (
            update_asteroid.run_if(resource_changed::<PlanetHeight>),
            update_planet.run_if(resource_changed::<DemoWidgetStates>),
        ),
    );

    app.add_observer(slider_on_interaction::<Insert, Hovered>)
        .add_observer(slider_on_change_value::<SliderValue>)
        .add_observer(slider_on_change_value::<SliderRange>);
}

#[derive(Component)]
struct ExampleAsteroidMarker;

fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        ExampleAsteroidMarker,
        Transform::IDENTITY,
        Visibility::Visible,
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.5, 0.5))),
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        WASDCameraController,
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        SkyboxConfig {
            cubemap: game_assets.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    commands.spawn((
        ScenarioScopedMarker,
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
}

fn update_asteroid(
    mut commands: Commands,
    asteroid: Single<Entity, With<ExampleAsteroidMarker>>,
    planet: Res<PlanetHeight>,
    game_assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let asteroid_entity = asteroid.into_inner();

    let mesh = TriangleMeshBuilder::new_octahedron(3)
        .apply_noise(&*planet)
        .build();
    let material = StandardMaterial {
        base_color_texture: Some(game_assets.asteroid_texture.clone()),
        ..default()
    };

    commands.entity(asteroid_entity).insert((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
    ));
}

fn update_planet(mut planet: ResMut<PlanetHeight>, widget_states: Res<DemoWidgetStates>) {
    planet.zoom_scale = widget_states.zoom_scale as f64;
    planet.seed = widget_states.seed as u32;
}

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Component, Default)]
struct DemoSlider;

#[derive(Component, Default)]
struct DemoSliderThumb;

#[derive(Resource)]
struct DemoWidgetStates {
    zoom_scale: f32,
    seed: f32,
}

fn setup_ui_slider(mut commands: Commands) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                Children::spawn((
                    Spawn((
                        Text::new("Seed"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    )),
                    Spawn((
                        Node {
                            width: percent(30),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        children![(
                    slider(0.0, 1000.0, 0.0),
                    observe(
                        |value_change: On<ValueChange<f32>>,
                         mut commands: Commands,
                         mut widget_states: ResMut<DemoWidgetStates>| {
                            widget_states.seed = value_change.value;
                            commands
                                .entity(value_change.event_target())
                                .insert(SliderValue(value_change.value));
                        },
                    ),
                )],
                    )),
                )),
            ),
            (
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                Children::spawn((
                    Spawn((
                        Text::new("Zoom Scale"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    )),
                    Spawn((
                        Node {
                            width: percent(30),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        children![(
                    slider(0.0, 1.0, 0.1),
                    observe(
                        |value_change: On<ValueChange<f32>>,
                         mut commands: Commands,
                         mut widget_states: ResMut<DemoWidgetStates>| {
                            widget_states.zoom_scale = value_change.value;
                            commands
                                .entity(value_change.event_target())
                                .insert(SliderValue(value_change.value));
                        },
                    ),
                )],
                    )),
                )),
            )
        ],
    ));
}

/// Create a demo slider
fn slider(min: f32, max: f32, value: f32) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            justify_items: JustifyItems::Center,
            column_gap: px(4),
            height: px(12),
            width: percent(100),
            ..default()
        },
        Name::new("Slider"),
        Hovered::default(),
        DemoSlider,
        Slider::default(),
        SliderValue(value),
        SliderRange::new(min, max),
        // TabIndex(0),
        Children::spawn((
            // Slider background rail
            Spawn((
                Node {
                    height: px(6),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK), // Border color for the checkbox
                BorderRadius::all(px(3)),
            )),
            // Invisible track to allow absolute placement of thumb entity. This is narrower than
            // the actual slider, which allows us to position the thumb entity using simple
            // percentages, without having to measure the actual width of the slider thumb.
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: px(0),
                    // Track is short by 12px to accommodate the thumb.
                    right: px(12),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                children![(
                    // Thumb
                    DemoSliderThumb,
                    SliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        left: percent(0), // This will be updated by the slider's value
                        ..default()
                    },
                    BorderRadius::MAX,
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

fn slider_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    sliders: Query<(Entity, &Hovered), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut BackgroundColor, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, hovered)) = sliders.get(event.event_target()) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_bg, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_bg.0 = if hovered.0 {
                        SLIDER_THUMB.lighter(0.3)
                    } else {
                        SLIDER_THUMB
                    }
                }
            }
        }
    }
}

fn slider_on_change_value<C: Component>(
    insert: On<Insert, C>,
    sliders: Query<(Entity, &SliderValue, &SliderRange), With<DemoSlider>>,
    children: Query<&Children>,
    mut thumbs: Query<(&mut Node, Has<DemoSliderThumb>), Without<DemoSlider>>,
) {
    if let Ok((slider_ent, value, range)) = sliders.get(insert.entity) {
        for child in children.iter_descendants(slider_ent) {
            if let Ok((mut thumb_node, is_thumb)) = thumbs.get_mut(child) {
                if is_thumb {
                    thumb_node.left = percent(range.thumb_position(value.0) * 100.0);
                }
            }
        }
    }
}
