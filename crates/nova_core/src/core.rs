use avian3d::prelude::*;
use bevy::{
    picking::{hover::Hovered, pointer::PointerInteraction},
    platform::collections::HashMap,
    prelude::*,
    reflect::Is,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::{observe, Activate, Button},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_enhanced_input::prelude::Binding;
use nova_scenario::prelude::*;
use rand::prelude::*;

use crate::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum ExampleStates {
    #[default]
    Loading,
    Editor,
    Scenario,
}

pub(crate) fn core_plugin(app: &mut App) {
    app.init_state::<ExampleStates>();
    app.insert_resource(SectionChoice::None);
    app.insert_resource(PlayerSpaceshipConfig::default());

    app.add_systems(
        OnEnter(GameStates::Playing),
        (|mut game_state: ResMut<NextState<ExampleStates>>| {
            game_state.set(ExampleStates::Editor);
        },),
    );

    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            setup_grab_cursor_scenario,
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        (
            setup_editor_scene,
            setup_grab_cursor_editor,
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (setup_scenario, |mut selection: ResMut<SectionChoice>| {
            *selection = SectionChoice::None;
        }),
    );

    app.add_observer(button_on_interaction::<Add, Pressed>)
        .add_observer(button_on_interaction::<Remove, Pressed>)
        .add_observer(button_on_interaction::<Add, InteractionDisabled>)
        .add_observer(button_on_interaction::<Remove, InteractionDisabled>)
        .add_observer(button_on_interaction::<Insert, Hovered>);

    app.add_observer(on_add_selected)
        .add_observer(on_remove_selected);
    app.add_observer(button_on_setting::<SectionChoice>);

    app.add_observer(on_click_spaceship_section)
        .add_observer(on_hover_spaceship_section)
        .add_observer(on_move_spaceship_section)
        .add_observer(on_out_spaceship_section);

    app.add_systems(
        Update,
        lock_on_left_click.run_if(in_state(ExampleStates::Editor)),
    );
    app.add_systems(
        Update,
        switch_scene_editor.run_if(in_state(ExampleStates::Scenario)),
    );

    app.configure_sets(
        Update,
        SpaceshipInputSystems.run_if(in_state(ExampleStates::Scenario)),
    );
    app.configure_sets(
        FixedUpdate,
        SpaceshipSectionSystems.run_if(in_state(ExampleStates::Scenario)),
    );
    app.configure_sets(
        Update,
        SpaceshipSectionSystems.run_if(in_state(ExampleStates::Scenario)),
    );
}

fn setup_scenario(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    player_config: Res<PlayerSpaceshipConfig>,
    sections: Res<GameSections>,
) {
    commands.trigger(LoadScenario(test_scenario(
        &game_assets,
        player_config,
        sections,
    )));
}

fn switch_scene_editor(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<NextState<ExampleStates>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::F1) {
        debug!("switch_scene_editor: F1 pressed, switching to Editor state.");
        state.set(ExampleStates::Editor);
        commands.trigger(UnloadScenario);
    }
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
struct PlayerSpaceshipConfig {
    sections: HashMap<Entity, SpaceshipSectionConfig>,
    inputs: HashMap<Entity, Vec<Binding>>,
}

fn test_scenario(
    game_assets: &GameAssets,
    player_config: Res<PlayerSpaceshipConfig>,
    sections: Res<GameSections>,
) -> ScenarioConfig {
    let mut rng = rand::rng();

    let mut objects = Vec::new();
    for i in 0..20 {
        let pos = Vec3::new(
            rng.random_range(-100.0..100.0),
            rng.random_range(-20.0..20.0),
            rng.random_range(-100.0..100.0),
        );
        let radius = rng.random_range(1.0..3.0);
        let texture = game_assets.asteroid_texture.clone();

        objects.push(ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: format!("asteroid_{}", i),
                name: format!("Asteroid {}", i),
                position: pos,
                rotation: Quat::IDENTITY,
                health: 100.0,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig { radius, texture }),
        });
    }

    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::AI(AIControllerConfig {}),
        sections: vec![
            SpaceshipSectionConfig {
                id: "controller".to_string(),
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_controller_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_front".to_string(),
                position: Vec3::new(0.0, 0.0, 1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "hull_back".to_string(),
                position: Vec3::new(0.0, 0.0, -1.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("reinforced_hull_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "thruster".to_string(),
                position: Vec3::new(0.0, 0.0, 2.0),
                rotation: Quat::IDENTITY,
                config: sections
                    .get_section("basic_thruster_section")
                    .unwrap()
                    .clone(),
            },
            SpaceshipSectionConfig {
                id: "turret".to_string(),
                position: Vec3::new(0.0, 0.0, -2.0),
                rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                config: sections
                    .get_section("better_turret_section")
                    .unwrap()
                    .clone(),
            },
        ],
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "other_spaceship".to_string(),
            name: "Other Spaceship".to_string(),
            position: Vec3::new(
                rng.random_range(-100.0..100.0),
                rng.random_range(-10.0..10.0),
                rng.random_range(-200.0..-100.0),
            ),
            rotation: Quat::IDENTITY,
            health: 100.0,
        },
        kind: ScenarioObjectKind::Spaceship(spaceship),
    });

    let player_spaceship = SpaceshipConfig {
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: player_config
                .inputs
                .iter()
                .map(|(entity, key)| (entity.to_string(), key.clone()))
                .collect(),
        }),
        sections: player_config.sections.values().cloned().collect(),
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player's Spaceship".to_string(),
            position: Vec3::new(0.0, 0.0, 50.0),
            rotation: Quat::IDENTITY,
            health: 100.0,
        },
        kind: ScenarioObjectKind::Spaceship(player_spaceship),
    });

    let events = vec![
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: objects
                .into_iter()
                .map(EventActionConfig::SpawnScenarioObject)
                .collect::<_>(),
        },
        ScenarioEventConfig {
            name: EventConfig::OnStart,
            filters: vec![],
            actions: vec![EventActionConfig::Objective(ObjectiveActionConfig::new(
                "destroy_spaceship",
                "Objective: Destroy the other spaceship.",
            ))],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("player_spaceship".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![EventActionConfig::DebugMessage(DebugMessageActionConfig {
                message: "The player's spaceship was destroyed!".to_string(),
            })],
        },
        ScenarioEventConfig {
            name: EventConfig::OnDestroyed,
            filters: vec![EventFilterConfig::Entity(EntityFilterConfig {
                id: Some("other_spaceship".to_string()),
                type_name: None,
                ..default()
            })],
            actions: vec![
                EventActionConfig::DebugMessage(DebugMessageActionConfig {
                    message: "Objective Complete: Destroyed the other spaceship!".to_string(),
                }),
                EventActionConfig::ObjectiveComplete(ObjectiveCompleteActionConfig {
                    id: "destroy_spaceship".to_string(),
                }),
            ],
        },
    ];

    ScenarioConfig {
        id: "test_scenario".to_string(),
        name: "Test Scenario".to_string(),
        description: "A test scenario.".to_string(),
        cubemap: game_assets.cubemap.clone(),
        events,
    }
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_PRESSED_BUTTON: Color = Color::srgb(0.25, 0.65, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);
const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);

const BACKGROUND_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

fn setup_editor_scene(
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
        WASDCameraController,
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        SkyboxConfig {
            cubemap: game_assets.cubemap.clone(),
            brightness: 1000.0,
        },
    ));

    commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Editor Main Menu"),
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Menu Container"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        height: percent(80),
                        width: px(400),
                        margin: UiRect::all(px(50)),
                        padding: UiRect::all(px(0)).with_top(px(20)).with_bottom(px(20)),
                        ..default()
                    },
                    BackgroundColor(BACKGROUND_COLOR),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Title"),
                        Text::new("Spaceship Editor"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(TEXT_COLOR),
                        Node { ..default() },
                    ));
                    parent.spawn((
                        Name::new("Separator 1"),
                        Node {
                            width: percent(80),
                            height: px(2),
                            margin: UiRect::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                    parent.spawn((
                        Name::new("Create New Spaceship Button V1"),
                        button("Create New Spaceship V1"),
                        observe(create_new_spaceship),
                    ));
                    parent.spawn((
                        Name::new("Create New Spaceship Button V2"),
                        button("Create New Spaceship V2"),
                        observe(create_new_spaceship_with_controller),
                    ));
                    parent.spawn((
                        Name::new("Separator 2"),
                        Node {
                            width: percent(80),
                            height: px(2),
                            margin: UiRect::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                    parent
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::FlexStart,
                            width: percent(100),
                            ..default()
                        },))
                        .with_children(|parent| {
                            for section in sections.iter() {
                                parent.spawn((
                                    Name::new(section.base.name.clone()),
                                    button(&section.base.name),
                                    SectionChoice::Section(section.base.id.clone()),
                                ));
                            }

                            parent.spawn((
                                Name::new("Delete Section Button"),
                                button("Delete Section"),
                                SectionChoice::Delete,
                            ));
                        });
                    parent.spawn((
                        Name::new("Separator 3"),
                        Node {
                            width: percent(80),
                            height: px(2),
                            margin: UiRect::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                    parent.spawn((
                        Name::new("Play Button"),
                        button("Play"),
                        observe(continue_to_simulation),
                    ));
                });
        });
}

#[derive(Resource, Default, Debug, Component, PartialEq, Eq, Clone, Reflect)]
enum SectionChoice {
    #[default]
    None,
    Section(String),
    Delete,
}

fn create_new_spaceship(
    _activate: On<Activate>,
    mut commands: Commands,
    q_spaceship: Query<Entity, With<SpaceshipRootMarker>>,
    sections: Res<GameSections>,
) {
    for entity in &q_spaceship {
        commands.entity(entity).despawn();
    }

    let entity = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipRootMarker,
            Name::new("Spaceship Prefab"),
            SpaceshipSectionsConfig::default(),
            SpaceshipController::None,
            Transform::default(),
            Visibility::Visible,
            RigidBody::Dynamic,
        ))
        .id();

    let position = Vec3::ZERO;
    let rotation = Quat::IDENTITY;
    let section = sections.get_section("reinforced_hull_section").unwrap();
    let base = section.base.clone();
    let hull = match &section.kind {
        SectionKind::Hull(h) => h.clone(),
        _ => panic!("create_new_spaceship: Section is not a hull."),
    };

    let mut hull_entity = Entity::PLACEHOLDER;
    commands.entity(entity).with_children(|parent| {
        hull_entity = parent
            .spawn((
                base_section(base.clone()),
                Transform::from_translation(position).with_rotation(rotation),
                hull_section(hull.clone()),
            ))
            .id();
    });

    commands.insert_resource(PlayerSpaceshipConfig {
        sections: HashMap::from([(
            hull_entity,
            SpaceshipSectionConfig {
                id: "initial_hull".to_string(),
                position,
                rotation,
                config: SectionConfig {
                    base,
                    kind: SectionKind::Hull(hull),
                },
            },
        )]),
        ..default()
    });
}

fn create_new_spaceship_with_controller(
    _activate: On<Activate>,
    mut commands: Commands,
    q_spaceship: Query<Entity, With<SpaceshipRootMarker>>,
    sections: Res<GameSections>,
) {
    for entity in &q_spaceship {
        commands.entity(entity).despawn();
    }

    let entity = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipRootMarker,
            Name::new("Spaceship Prefab with Controller"),
            SpaceshipSectionsConfig::default(),
            SpaceshipController::None,
            Transform::default(),
            Visibility::Visible,
            RigidBody::Dynamic,
        ))
        .id();

    let position = Vec3::ZERO;
    let rotation = Quat::IDENTITY;
    let section = sections.get_section("basic_controller_section").unwrap();
    let base = section.base.clone();
    let controller = match &section.kind {
        SectionKind::Controller(c) => c.clone(),
        _ => panic!("create_new_spaceship_with_controller: Section is not a controller."),
    };

    let mut controller_entity = Entity::PLACEHOLDER;
    commands.entity(entity).with_children(|parent| {
        controller_entity = parent
            .spawn((
                base_section(base.clone()),
                Transform::from_translation(position).with_rotation(rotation),
                controller_section(controller.clone()),
            ))
            .id();
    });

    commands.insert_resource(PlayerSpaceshipConfig {
        sections: HashMap::from([(
            controller_entity,
            SpaceshipSectionConfig {
                id: "initial_controller".to_string(),
                position,
                rotation,
                config: SectionConfig {
                    base,
                    kind: SectionKind::Controller(controller),
                },
            },
        )]),
        ..default()
    });
}

fn continue_to_simulation(
    _activate: On<Activate>,
    mut game_state: ResMut<NextState<ExampleStates>>,
) {
    game_state.set(ExampleStates::Scenario);
}

#[derive(Component)]
struct SectionPreviewMarker;

fn on_click_spaceship_section(
    click: On<Pointer<Press>>,
    mut commands: Commands,
    spaceship: Single<Entity, With<SpaceshipRootMarker>>,
    q_pointer: Query<&PointerInteraction>,
    q_section: Query<&Transform, With<SectionMarker>>,
    selection: Res<SectionChoice>,
    q_preview: Query<Entity, With<SectionPreviewMarker>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    sections: Res<GameSections>,
    mut player_config: ResMut<PlayerSpaceshipConfig>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let entity = click.entity;

    let Some(normal) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(e, hit)| if *e == entity { hit.normal } else { None })
    else {
        return;
    };

    let Ok(transform) = q_section.get(entity) else {
        return;
    };

    let spaceship = spaceship.into_inner();
    let position = transform.translation + normal * 1.0;

    match *selection {
        SectionChoice::None => {}
        SectionChoice::Section(ref id) => {
            let Some(section) = sections.get_section(id) else {
                panic!("on_click_spaceship_section: Section '{}' not found.", id);
            };

            match &section.kind {
                SectionKind::Hull(hull) => {
                    let rotation = Quat::IDENTITY;

                    let mut hull_entity = Entity::PLACEHOLDER;
                    commands.entity(spaceship).with_children(|parent| {
                        hull_entity = parent
                            .spawn((
                                base_section(section.base.clone()),
                                hull_section(hull.clone()),
                                Transform {
                                    translation: position,
                                    ..default()
                                },
                            ))
                            .id();
                    });

                    player_config.sections.insert(
                        hull_entity,
                        SpaceshipSectionConfig {
                            id: hull_entity.to_string(),
                            position,
                            rotation,
                            config: section.clone(),
                        },
                    );
                }
                SectionKind::Thruster(thruster) => {
                    let rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize());

                    let key_bind = keyboard.map(|k| {
                        k.get_pressed()
                            .next()
                            .map_or(KeyCode::Space.into(), |k| Binding::from(*k))
                    });
                    let pad_bind = gamepad.map(|b| {
                        b.get_pressed()
                            .next()
                            .map_or(GamepadButton::RightTrigger.into(), |b| Binding::from(*b))
                    });
                    let binds = vec![key_bind, pad_bind]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<Binding>>();

                    let mut thruster_entity = Entity::PLACEHOLDER;
                    commands.entity(spaceship).with_children(|parent| {
                        thruster_entity = parent
                            .spawn((
                                base_section(section.base.clone()),
                                thruster_section(thruster.clone()),
                                SpaceshipThrusterInputBinding(binds.clone()),
                                Transform {
                                    translation: position,
                                    rotation,
                                    ..default()
                                },
                            ))
                            .id();
                    });

                    player_config.sections.insert(
                        thruster_entity,
                        SpaceshipSectionConfig {
                            id: thruster_entity.to_string(),
                            position,
                            rotation,
                            config: section.clone(),
                        },
                    );
                    player_config.inputs.insert(thruster_entity, binds);
                }
                SectionKind::Controller(controller) => {
                    let rotation = Quat::IDENTITY;

                    let mut controller_entity = Entity::PLACEHOLDER;
                    commands.entity(spaceship).with_children(|parent| {
                        controller_entity = parent
                            .spawn((
                                base_section(section.base.clone()),
                                controller_section(controller.clone()),
                                Transform {
                                    translation: position,
                                    rotation,
                                    ..default()
                                },
                            ))
                            .id();
                    });

                    player_config.sections.insert(
                        controller_entity,
                        SpaceshipSectionConfig {
                            id: controller_entity.to_string(),
                            position,
                            rotation,
                            config: section.clone(),
                        },
                    );
                }
                SectionKind::Turret(turret) => {
                    let rotation = Quat::from_rotation_arc(Vec3::Y, normal.normalize());

                    let key_bind = keyboard.map(|k| {
                        k.get_pressed()
                            .next()
                            .map_or(MouseButton::Left.into(), |k| Binding::from(*k))
                    });
                    let pad_bind = gamepad.map(|b| {
                        b.get_pressed()
                            .next()
                            .map_or(GamepadButton::RightTrigger2.into(), |b| Binding::from(*b))
                    });
                    let binds = vec![key_bind, pad_bind]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<Binding>>();

                    let mut turret_entity = Entity::PLACEHOLDER;
                    commands.entity(spaceship).with_children(|parent| {
                        turret_entity = parent
                            .spawn((
                                base_section(section.base.clone()),
                                turret_section(turret.clone()),
                                SpaceshipTurretInputBinding(binds.clone()),
                                Transform {
                                    translation: position,
                                    rotation,
                                    ..default()
                                },
                            ))
                            .id();
                    });

                    player_config.sections.insert(
                        turret_entity,
                        SpaceshipSectionConfig {
                            id: turret_entity.to_string(),
                            position,
                            rotation,
                            config: section.clone(),
                        },
                    );
                    player_config.inputs.insert(turret_entity, binds);
                }
                SectionKind::Torpedo(torpedo) => {
                    let rotation = Quat::from_rotation_arc(Vec3::Y, normal.normalize());

                    let key_bind = keyboard.map(|k| {
                        k.get_pressed()
                            .next()
                            .map_or(MouseButton::Left.into(), |k| Binding::from(*k))
                    });
                    let pad_bind = gamepad.map(|b| {
                        b.get_pressed()
                            .next()
                            .map_or(GamepadButton::RightTrigger2.into(), |b| Binding::from(*b))
                    });
                    let binds = vec![key_bind, pad_bind]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<Binding>>();

                    let mut torpedo_entity = Entity::PLACEHOLDER;
                    commands.entity(spaceship).with_children(|parent| {
                        torpedo_entity = parent
                            .spawn((
                                base_section(section.base.clone()),
                                torpedo_section(torpedo.clone()),
                                SpaceshipTorpedoInputBinding(binds.clone()),
                                Transform {
                                    translation: position,
                                    rotation,
                                    ..default()
                                },
                            ))
                            .id();
                    });

                    player_config.sections.insert(
                        torpedo_entity,
                        SpaceshipSectionConfig {
                            id: torpedo_entity.to_string(),
                            position,
                            rotation,
                            config: section.clone(),
                        },
                    );
                    player_config.inputs.insert(torpedo_entity, binds);
                }
            }
        }
        SectionChoice::Delete => {
            commands.entity(entity).despawn();
            player_config.sections.remove(&entity);

            for preview in &q_preview {
                commands.entity(preview).despawn();
            }
        }
    }
}

fn on_hover_spaceship_section(
    hover: On<Pointer<Over>>,
    mut commands: Commands,
    q_pointer: Query<&PointerInteraction>,
    q_section: Query<&GlobalTransform, With<SectionMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selection: Res<SectionChoice>,
) {
    let entity = hover.entity;

    let Some(normal) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(e, hit)| if *e == entity { hit.normal } else { None })
    else {
        return;
    };

    let Ok(transform) = q_section.get(entity) else {
        return;
    };

    match *selection {
        SectionChoice::None => {}
        SectionChoice::Delete => {
            let position = transform.translation();

            commands.spawn((
                SectionPreviewMarker,
                Mesh3d(meshes.add(Cuboid::new(1.01, 1.01, 1.01))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
                Transform {
                    translation: position,
                    ..default()
                },
            ));
        }
        _ => {
            let position = transform.translation() + normal * 1.0;
            let rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize());

            commands.spawn((
                SectionPreviewMarker,
                Mesh3d(meshes.add(Cuboid::new(1.01, 1.01, 1.01))),
                MeshMaterial3d(materials.add(Color::srgb(0.2, 0.8, 0.2))),
                Transform {
                    translation: position,
                    rotation,
                    ..default()
                },
            ));
        }
    }
}

fn on_move_spaceship_section(
    move_: On<Pointer<Move>>,
    q_pointer: Query<&PointerInteraction>,
    q_section: Query<&GlobalTransform, With<SectionMarker>>,
    preview: Single<&mut Transform, With<SectionPreviewMarker>>,
    selection: Res<SectionChoice>,
) {
    if matches!(*selection, SectionChoice::Delete | SectionChoice::None) {
        return;
    }

    let entity = move_.entity;

    let Some(normal) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(e, hit)| if *e == entity { hit.normal } else { None })
    else {
        return;
    };

    let Ok(transform) = q_section.get(entity) else {
        return;
    };

    let position = transform.translation() + normal * 1.0;
    let rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize());

    let mut preview_transform = preview.into_inner();
    preview_transform.translation = position;
    preview_transform.rotation = rotation;
}

fn on_out_spaceship_section(
    out: On<Pointer<Out>>,
    q_section: Query<&Transform, With<SectionMarker>>,
    mut commands: Commands,
    preview: Single<Entity, With<SectionPreviewMarker>>,
) {
    let Ok(_) = q_section.get(out.entity) else {
        return;
    };

    commands.entity(preview.into_inner()).despawn();
}

fn setup_grab_cursor_scenario(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if cfg!(not(feature = "debug")) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_cursor_options.visible = false;
    }
}

fn setup_grab_cursor_editor(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut primary_cursor_options = primary_cursor_options.into_inner();
    primary_cursor_options.grab_mode = CursorGrabMode::None;
    primary_cursor_options.visible = true;
}

fn lock_on_left_click(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_cursor_options.visible = false;
    } else if mouse.just_released(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::None;
        primary_cursor_options.visible = true;
    }
}

#[derive(Component)]
struct SelectedOption;

#[derive(Component)]
struct EditorButton;

fn button_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    mut q_button: Query<
        (
            &Hovered,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<SelectedOption>,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        With<EditorButton>,
    >,
) {
    if let Ok((hovered, disabled, pressed, selected, mut color, mut border_color, children)) =
        q_button.get_mut(event.event_target())
    {
        if children.is_empty() {
            return;
        }
        if selected {
            *color = HOVERED_PRESSED_BUTTON.into();
            border_color.set_all(BORDER_COLOR_ACTIVE);
            return;
        }

        let hovered = hovered.get();
        let pressed = pressed && !(E::is::<Remove>() && C::is::<Pressed>());
        let disabled = disabled && !(E::is::<Remove>() && C::is::<InteractionDisabled>());
        match (disabled, hovered, pressed) {
            (true, _, _) => {
                *color = NORMAL_BUTTON.into();
                *border_color = BORDER_COLOR_INACTIVE.into();
            }

            (false, true, true) => {
                *color = HOVERED_PRESSED_BUTTON.into();
                border_color.set_all(BORDER_COLOR_ACTIVE);
            }

            (false, true, false) => {
                *color = HOVERED_BUTTON.into();
                border_color.set_all(BORDER_COLOR_ACTIVE);
            }

            (false, false, _) => {
                *color = NORMAL_BUTTON.into();
                *border_color = BORDER_COLOR_INACTIVE.into();
            }
        }
    }
}

fn button_on_setting<T: Resource + Component + PartialEq + Clone>(
    event: On<Add, Pressed>,
    mut commands: Commands,
    selected: Option<Single<Entity, (With<T>, With<SelectedOption>)>>,
    q_t: Query<(Entity, &T), (Without<SelectedOption>, With<EditorButton>)>,
    mut setting: ResMut<T>,
) {
    let Ok((entity, t)) = q_t.get(event.event_target()) else {
        return;
    };

    if *setting != *t {
        if let Some(previous) = selected {
            commands
                .entity(previous.into_inner())
                .remove::<SelectedOption>();
        }
        commands.entity(entity).insert(SelectedOption);
        *setting = t.clone();
    }
}

fn on_add_selected(
    add: On<Add, SelectedOption>,
    mut q_color: Query<&mut BackgroundColor, (With<SelectedOption>, With<EditorButton>)>,
) {
    if let Ok(mut color) = q_color.get_mut(add.event_target()) {
        *color = PRESSED_BUTTON.into();
    }
}

fn on_remove_selected(
    remove: On<Remove, SelectedOption>,
    mut q_color: Query<&mut BackgroundColor, With<EditorButton>>,
) {
    if let Ok(mut color) = q_color.get_mut(remove.event_target()) {
        *color = NORMAL_BUTTON.into();
    }
}

fn button(text: &str) -> impl Bundle {
    (
        Node {
            width: percent(80),
            min_height: px(40),
            margin: UiRect::all(px(20)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        EditorButton,
        Button,
        Hovered::default(),
        BorderColor::all(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new(text),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}
