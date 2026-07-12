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
use nova_assets::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use rand::prelude::*;

pub mod prelude {
    pub use super::NovaEditorPlugin;
}

/// The spaceship editor: a scene where you build a ship out of sections and then hand
/// it off to a scenario simulation.
///
/// `nova_core` adds this as its default "game" plugin when no custom game plugins are
/// supplied (see `AppBuilder`). Examples that provide their own scenario opt out of it.
pub struct NovaEditorPlugin;

impl Plugin for NovaEditorPlugin {
    fn build(&self, app: &mut App) {
        editor_plugin(app);
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
enum ExampleStates {
    #[default]
    Loading,
    Editor,
    Scenario,
}

fn editor_plugin(app: &mut App) {
    app.init_state::<ExampleStates>();
    app.insert_resource(SectionChoice::None);
    app.insert_resource(PlayerSpaceshipConfig::default());
    app.init_resource::<EditorRebind>();

    // The editor is the Sandbox game. When the main menu fronts the app it hands
    // off to Playing with GameMode set: Sandbox enters the editor, NewGame goes
    // straight to the Scenario state. The menu owns the NewGame scenario load;
    // setup_scenario below stays Sandbox-only so the two do not both fire.
    // GameMode defaults to Sandbox (NovaGameplayPlugin), so menu-less apps
    // behave as before. (The spaceship input/section sets are gated on
    // scenario-liveness by nova_scenario, not on these states - see the note
    // at the end of this function.)
    app.add_systems(
        OnEnter(GameStates::Playing),
        (
            |mode: Res<GameMode>, mut game_state: ResMut<NextState<ExampleStates>>| {
                game_state.set(match *mode {
                    GameMode::Sandbox => ExampleStates::Editor,
                    GameMode::NewGame => ExampleStates::Scenario,
                });
            },
        ),
    );

    // Leaving Playing (the pause menu's Back to Main Menu) must tear the
    // editor scene down: DespawnOnExit(ExampleStates::...) entities only
    // despawn when the inner state actually changes, and a later Sandbox
    // entry must start fresh in Editor, not resume a stale Scenario.
    app.add_systems(
        OnExit(GameStates::Playing),
        |mut game_state: ResMut<NextState<ExampleStates>>| {
            game_state.set(ExampleStates::Loading);
        },
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
        (
            // Sandbox-only: in NewGame the menu already loaded its scenario and a
            // second LoadScenario here would tear it straight back down.
            setup_scenario.run_if(resource_equals(GameMode::Sandbox)),
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
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

    // Editor section keybind labels + click-to-rebind (task 20260712-163912).
    // A stale rebind must not survive a scene change, so clear it on every
    // state entry (like SectionChoice).
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut rebind: ResMut<EditorRebind>| rebind.target = None,
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        |mut rebind: ResMut<EditorRebind>| rebind.target = None,
    );
    app.add_systems(
        Update,
        (
            sync_section_keybind_labels,
            apply_section_rebind,
            position_section_keybind_labels,
            scroll_editor_panel,
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_systems(
        Update,
        lock_on_left_click
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused))),
    );
    app.add_systems(
        Update,
        // F1-to-editor is demo/sandbox furniture: campaigns (NewGame) must
        // not offer an editor escape (task 20260711-203805); the pause menu
        // is the sanctioned way out.
        switch_scene_editor
            .run_if(in_state(ExampleStates::Scenario).and_then(resource_equals(GameMode::Sandbox))),
    );

    // The spaceship input/section system sets are deliberately NOT gated
    // here anymore: nova_scenario's ScenarioLoaderPlugin gates them on
    // scenario-liveness (task 20260711-212519). The editor's build-mode
    // preview stays inert because the Editor state never has a scenario
    // loaded - initial entry loads nothing and F1 triggers UnloadScenario.
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
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                radius,
                texture,
                health: 100.0,
                surface_gravity: None,
                invulnerable: false,
            }),
        });
    }

    // Sandbox is for building and flying, not fighting: the other ship is a passive
    // target, per the main-menu spike's sandbox scope (docs/spikes/20260711-180500).
    let spaceship = SpaceshipConfig {
        controller: SpaceshipController::None,
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

            speed_cap: None,
            // The editor sandbox keeps normal finite magazines.
            infinite_ammo: false,
        }),
        sections: player_config.sections.values().cloned().collect(),
    };
    objects.push(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: "player_spaceship".to_string(),
            name: "Player's Spaceship".to_string(),
            position: Vec3::new(0.0, 0.0, 50.0),
            rotation: Quat::IDENTITY,
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
        PostProcessingCamera,
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
                    // Scrollable (task 20260712-185527): the palette grew past
                    // the fixed height, so the panel scrolls vertically with the
                    // wheel (`scroll_editor_panel` drives ScrollPosition).
                    EditorScrollPanel,
                    ScrollPosition::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        height: percent(80),
                        width: px(400),
                        margin: UiRect::all(px(50)),
                        padding: UiRect::all(px(0)).with_top(px(20)).with_bottom(px(20)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(BACKGROUND_COLOR),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Title"),
                        Text::new("Spaceship Editor"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
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
                            // Deselect the build/delete tool -> select mode
                            // (SectionChoice::None), where clicking a section
                            // rebinds its key. Without this there is no way back
                            // to None once a tool is picked, so the rebind flow
                            // (task 20260712-163912) was unreachable (task
                            // 20260712-183725).
                            parent.spawn((
                                Name::new("Select Section Button"),
                                button("Select / Rebind"),
                                ButtonValue(SectionChoice::None),
                            ));

                            for section in sections.iter() {
                                parent.spawn((
                                    Name::new(section.base.name.clone()),
                                    button(&section.base.name),
                                    ButtonValue(SectionChoice::Section(section.base.id.clone())),
                                ));
                            }

                            parent.spawn((
                                Name::new("Delete Section Button"),
                                button("Delete Section"),
                                ButtonValue(SectionChoice::Delete),
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

#[derive(Resource, Default, Debug, PartialEq, Eq, Clone, Reflect)]
enum SectionChoice {
    #[default]
    None,
    Section(String),
    Delete,
}

fn create_new_spaceship(
    _activate: On<Activate>,
    mut commands: Commands,
    q_spaceship: Query<Entity, With<SpaceshipPreviewMarker>>,
    sections: Res<GameSections>,
) {
    for entity in &q_spaceship {
        commands.entity(entity).despawn();
    }

    let entity = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipPreviewMarker,
            Name::new("Spaceship Preview"),
            Transform::default(),
            Visibility::Visible,
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
                preview_section(base.clone()),
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
    q_spaceship: Query<Entity, With<SpaceshipPreviewMarker>>,
    sections: Res<GameSections>,
) {
    for entity in &q_spaceship {
        commands.entity(entity).despawn();
    }

    let entity = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipPreviewMarker,
            Name::new("Spaceship Preview with Controller"),
            Transform::default(),
            Visibility::Visible,
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
                preview_section(base.clone()),
                Transform::from_translation(position).with_rotation(rotation),
                preview_controller_section(controller.clone()),
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

/// The root of the editor's preview ship. Deliberately distinct from the gameplay
/// `SpaceshipRootMarker`: the preview is a static, pickable visual used only to build a
/// `PlayerSpaceshipConfig`, so it must not trigger `insert_spaceship_sections` or any of the
/// integrity/health systems that key on `SpaceshipRootMarker`. The real ship is built from
/// the config when entering the scenario.
#[derive(Component)]
struct SpaceshipPreviewMarker;

#[derive(Component)]
struct SectionPreviewMarker;

fn on_click_spaceship_section(
    click: On<Pointer<Press>>,
    mut commands: Commands,
    spaceship: Single<Entity, With<SpaceshipPreviewMarker>>,
    q_pointer: Query<&PointerInteraction>,
    q_section: Query<&Transform, With<SectionMarker>>,
    selection: Res<SectionChoice>,
    q_preview: Query<Entity, With<SectionPreviewMarker>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    sections: Res<GameSections>,
    mut player_config: ResMut<PlayerSpaceshipConfig>,
    mut rebind: ResMut<EditorRebind>,
    q_bindable: Query<
        (),
        Or<(
            With<SpaceshipThrusterInputBinding>,
            With<SpaceshipTurretInputBinding>,
            With<SpaceshipTorpedoInputBinding>,
        )>,
    >,
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
        SectionChoice::None => {
            // No placement tool selected = select/edit mode: clicking a bindable
            // section arms a rebind (task 20260712-163912). `apply_section_rebind`
            // captures the next key or mouse-button press. Non-bindable sections
            // (hull, controller) and empty space do nothing.
            //
            // Only arm when nothing is armed yet: while a rebind is pending, the
            // next click is the user PICKING a mouse-button binding (e.g. LMB), so
            // it must not re-arm on whatever is under the cursor (task 20260712-191604).
            if rebind.target.is_none() && q_bindable.get(entity).is_ok() {
                rebind.target = Some(entity);
                // Wait for this arming click to release before capturing.
                rebind.awaiting_release = true;
            }
        }
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
                                preview_section(section.base.clone()),
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
                                preview_section(section.base.clone()),
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
                                preview_section(section.base.clone()),
                                preview_controller_section(controller.clone()),
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
                                preview_section(section.base.clone()),
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
                                preview_section(section.base.clone()),
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

// -- Section keybind labels + rebind (task 20260712-163912) --

/// The section currently awaiting a new keybind. Armed by clicking a bindable
/// section in select mode (`SectionChoice::None`); `apply_section_rebind`
/// consumes the next key or mouse-button press. Reset to `None` on every state
/// entry.
#[derive(Resource, Debug, Clone, Default)]
struct EditorRebind {
    target: Option<Entity>,
    /// Set true when armed by a mouse click: the capture waits until that click
    /// is released before reading a press, so the arming LMB is not itself bound
    /// (task 20260712-191604). False = ready to capture (e.g. armed in a test).
    awaiting_release: bool,
}

/// A screen-space UI chip showing `section`'s current keybind, positioned each
/// frame over the section by projecting its world position with the editor
/// camera. One per bindable (thruster/turret/torpedo) section.
#[derive(Component, Debug, Clone, Copy)]
struct SectionKeybindLabel {
    section: Entity,
}

/// The chip text of the currently-armed section (see [`EditorRebind`]).
const REBIND_PROMPT: &str = "press key";

/// True set of currently-bindable sections (carry one of the three input
/// binding components).
type BindableFilter = Or<(
    With<SpaceshipThrusterInputBinding>,
    With<SpaceshipTurretInputBinding>,
    With<SpaceshipTorpedoInputBinding>,
)>;

/// Keep exactly one [`SectionKeybindLabel`] per bindable section: spawn for new
/// ones, despawn labels whose section is gone or lost its binding. Reconcile
/// shape mirrors the ammo readout's `sync_ammo_readouts`.
fn sync_section_keybind_labels(
    mut commands: Commands,
    q_bindable: Query<Entity, BindableFilter>,
    q_labels: Query<(Entity, &SectionKeybindLabel)>,
) {
    // Despawn stale labels.
    for (label, SectionKeybindLabel { section }) in &q_labels {
        if q_bindable.get(*section).is_err() {
            commands.entity(label).despawn();
        }
    }
    // Spawn missing labels.
    let has_label = |section: Entity| q_labels.iter().any(|(_, l)| l.section == section);
    for section in &q_bindable {
        if !has_label(section) {
            commands.spawn((
                DespawnOnExit(ExampleStates::Editor),
                SectionKeybindLabel { section },
                Name::new("Section Keybind Label"),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.35)),
                TextShadow::default(),
                Node {
                    position_type: PositionType::Absolute,
                    // Pill padding + rounded corners so the background reads as a
                    // chip (BorderRadius is a Node field, not a component).
                    padding: UiRect::axes(px(6), px(2)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                // Dark semi-transparent pill so the gold text stays legible over
                // the 3D scene (task 20260712-183725).
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                // Hidden until the positioner projects it this frame.
                Visibility::Hidden,
            ));
        }
    }
}

/// Position each keybind label over its section (project with the editor
/// camera) and set its text to the section's current binding - or the rebind
/// prompt while that section is armed. Hidden when the section projects
/// off-screen or behind the camera.
///
/// Runs in `Update`, so it reads the previous frame's `GlobalTransform` - a
/// one-frame lag that is invisible for a near-static editor scene (only the
/// WASD camera moves). If labels ever need to track fast motion exactly, move
/// this to `PostUpdate` after transform propagation (and mind bevy_ui layout
/// ordering, as `screen_indicator` does).
#[allow(clippy::type_complexity)]
fn position_section_keybind_labels(
    rebind: Res<EditorRebind>,
    camera: Single<(&Camera, &GlobalTransform), With<WASDCameraController>>,
    q_section: Query<(
        &GlobalTransform,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
    )>,
    mut q_labels: Query<(&SectionKeybindLabel, &mut Node, &mut Text, &mut Visibility)>,
) {
    let (cam, cam_transform) = *camera;
    for (SectionKeybindLabel { section }, mut node, mut text, mut visibility) in &mut q_labels {
        let Ok((section_transform, thruster, turret, torpedo)) = q_section.get(*section) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        match cam.world_to_viewport(cam_transform, section_transform.translation()) {
            Ok(screen) => {
                node.left = Val::Px(screen.x);
                node.top = Val::Px(screen.y);
                *visibility = Visibility::Visible;
            }
            Err(_) => {
                // Behind the camera / off-viewport: do not draw.
                *visibility = Visibility::Hidden;
                continue;
            }
        }
        let wanted = if rebind.target == Some(*section) {
            REBIND_PROMPT.to_string()
        } else {
            let binds = thruster
                .map(|b| b.0.as_slice())
                .or(turret.map(|b| b.0.as_slice()))
                .or(torpedo.map(|b| b.0.as_slice()))
                .unwrap_or(&[]);
            binding_label(binds)
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

/// Consume the next key or mouse-button press to rebind the armed section (see
/// [`EditorRebind`]). Escape cancels. The new binding replaces the section's
/// previous PRIMARY input (keyboard or mouse button; any gamepad binding is
/// preserved) on both the live component and `PlayerSpaceshipConfig::inputs`
/// (what the scenario reads).
fn apply_section_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut rebind: ResMut<EditorRebind>,
    mut player_config: ResMut<PlayerSpaceshipConfig>,
    mut q_thruster: Query<&mut SpaceshipThrusterInputBinding>,
    mut q_turret: Query<&mut SpaceshipTurretInputBinding>,
    mut q_torpedo: Query<&mut SpaceshipTorpedoInputBinding>,
) {
    let Some(section) = rebind.target else {
        return;
    };
    // The section vanished (deleted while armed): drop the rebind.
    let still_bindable =
        q_thruster.contains(section) || q_turret.contains(section) || q_torpedo.contains(section);
    if !still_bindable {
        rebind.target = None;
        rebind.awaiting_release = false;
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        rebind.target = None;
        rebind.awaiting_release = false;
        return;
    }
    // Armed by a mouse click: wait for that click to release before reading a
    // press, so the arming LMB is not captured as the new binding.
    if rebind.awaiting_release {
        if mouse.get_pressed().next().is_none() {
            rebind.awaiting_release = false;
        }
        return;
    }

    // The next key or mouse button pressed becomes the binding (keyboard wins a
    // same-frame tie, arbitrary but stable).
    let new_binding = keys
        .get_just_pressed()
        .find(|k| **k != KeyCode::Escape)
        .map(|k| Binding::from(*k))
        .or_else(|| mouse.get_just_pressed().next().map(|b| Binding::from(*b)));
    let Some(new_binding) = new_binding else {
        return;
    };

    // Replace the PRIMARY input (keyboard OR mouse button), keep gamepad binds.
    let rebind_binds = |current: &[Binding]| -> Vec<Binding> {
        let mut binds: Vec<Binding> = current
            .iter()
            .filter(|b| !matches!(b, Binding::Keyboard { .. } | Binding::MouseButton { .. }))
            .cloned()
            .collect();
        binds.insert(0, new_binding);
        binds
    };

    let new_binds = if let Ok(mut b) = q_thruster.get_mut(section) {
        let binds = rebind_binds(&b.0);
        b.0 = binds.clone();
        binds
    } else if let Ok(mut b) = q_turret.get_mut(section) {
        let binds = rebind_binds(&b.0);
        b.0 = binds.clone();
        binds
    } else if let Ok(mut b) = q_torpedo.get_mut(section) {
        let binds = rebind_binds(&b.0);
        b.0 = binds.clone();
        binds
    } else {
        rebind.target = None;
        return;
    };

    player_config.inputs.insert(section, new_binds);
    rebind.target = None;
}

/// Marker for the scrollable editor palette panel (the "Menu Container" in
/// `setup_editor_scene`). Task 20260712-185527.
#[derive(Component, Debug, Clone, Copy)]
struct EditorScrollPanel;

/// Pixels scrolled per line of wheel movement.
const SCROLL_LINE_HEIGHT: f32 = 20.0;

/// Scroll the editor palette panel with the mouse wheel. Bevy does not scroll
/// `Overflow::Scroll` nodes on its own - a system must drive `ScrollPosition`
/// (bevy ui scroll example pattern). Editor-state only; the WASD camera does not
/// consume the wheel, so there is no zoom conflict.
fn scroll_editor_panel(
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

/// The value a settings button represents. Kept distinct from the `T` resource so a
/// button can carry a choice without being interpreted as - and clobbering - the resource
/// itself: on Bevy 0.19 a `#[derive(Resource)]` type is component-backed, so putting it on
/// a button entity is treated as a resource insert.
#[derive(Component, Debug, Clone)]
struct ButtonValue<T>(T);

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

fn button_on_setting<
    T: Resource + Component<Mutability = bevy::ecs::component::Mutable> + PartialEq + Clone,
>(
    event: On<Add, Pressed>,
    mut commands: Commands,
    // Each button carries its value as a `ButtonValue<T>` component (distinct from the T
    // resource, so a button never clobbers the resource), and clicking copies that value
    // into the `ResMut<T>` resource.
    selected: Option<Single<Entity, (With<ButtonValue<T>>, With<SelectedOption>)>>,
    q_t: Query<(Entity, &ButtonValue<T>), (Without<SelectedOption>, With<EditorButton>)>,
    mut setting: ResMut<T>,
) {
    let Ok((entity, value)) = q_t.get(event.event_target()) else {
        return;
    };

    if *setting != value.0 {
        if let Some(previous) = selected {
            commands
                .entity(previous.into_inner())
                .remove::<SelectedOption>();
        }
        commands.entity(entity).insert(SelectedOption);
        *setting = value.0.clone();
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
            border_radius: BorderRadius::MAX,
            ..default()
        },
        EditorButton,
        Button,
        Hovered::default(),
        BorderColor::all(Color::BLACK),
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )],
    )
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, state::app::StatesPlugin};

    use super::*;

    /// Counts LoadScenario triggers so the NewGame test can prove the editor
    /// stayed out of the menu's scenario load (review R1.1).
    #[derive(Resource, Default)]
    struct EditorScenarioLoads(usize);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_resource::<GameMode>();
        // switch_scene_editor polls the keyboard while in the Scenario state.
        app.init_resource::<ButtonInput<KeyCode>>();
        editor_plugin(&mut app);
        app.init_resource::<EditorScenarioLoads>();
        app.add_observer(
            |_: On<LoadScenario>, mut loads: ResMut<EditorScenarioLoads>| {
                loads.0 += 1;
            },
        );
        app
    }

    /// Regression for review R1.1: in NewGame mode the editor must still enter
    /// its Scenario state (cursor grab and the F1/despawn furniture key on it),
    /// while leaving the scenario load itself to the menu. (Flyability itself
    /// is no longer tied to this state: the spaceship sets are gated on
    /// scenario-liveness by nova_scenario, task 20260711-212519.)
    #[test]
    fn new_game_enters_scenario_state_without_loading_the_editor_scenario() {
        let mut app = app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();

        // Delivery guard: the handoff actually reached the Scenario state.
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        // The editor did not fire its own sandbox scenario on top of the menu's.
        assert_eq!(app.world().resource::<EditorScenarioLoads>().0, 0);
    }

    /// Leaving Playing (the pause menu's Back to Main Menu) resets the
    /// editor's inner state so DespawnOnExit scene entities are torn down
    /// and the next Sandbox entry starts fresh (task 20260711-185156).
    #[test]
    fn leaving_playing_resets_the_inner_state() {
        let mut app = app();
        // NewGame routes to Scenario, which applies safely headless (the
        // editor's own scenario load is Sandbox-gated).
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Loading,
            "inner state must reset when Playing is left"
        );
    }

    /// F1 back-to-editor is Sandbox-only (task 20260711-203805): in NewGame
    /// the same press must do nothing. Delivery guard: the identical press in
    /// Sandbox mode queues the Editor state and unloads the scenario, proving
    /// the stimulus path works.
    #[test]
    fn f1_returns_to_editor_only_in_sandbox_mode() {
        let make_app = app;
        // NewGame: F1 must be inert.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario,
            "F1 must not leave the scenario in NewGame"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "no editor scenario churn in NewGame"
        );

        // Sandbox: the same press flips to Editor. Enter Playing via NewGame
        // (going through Editor would run setup_editor_scene, which needs
        // GameAssets headless), then flip the mode - the gate reads the
        // resource at press time. Assert the queued target without applying
        // it, for the same reason.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "the same press must work in Sandbox (delivery guard)"
        );
    }

    /// The scenario-liveness gate (nova_scenario, task 20260711-212519)
    /// keeps the editor's build-mode preview inert only if the Editor state
    /// never has a live scenario. This exercises the one route that enters
    /// Editor FROM a live scenario - F1 - and asserts the same press
    /// unloads it, with the editor firing no scenario load of its own
    /// anywhere on the route. (Initial Sandbox entry loading nothing is
    /// covered by sandbox_heads_to_editor_state plus setup_scenario's
    /// Sandbox-and-Scenario-only wiring.)
    #[test]
    fn editor_state_never_keeps_a_scenario_live() {
        #[derive(Resource, Default)]
        struct Unloads(usize);

        let mut app = app();
        app.init_resource::<Unloads>();
        app.add_observer(|_: On<UnloadScenario>, mut unloads: ResMut<Unloads>| {
            unloads.0 += 1;
        });

        // Enter Playing via NewGame (Editor's OnEnter scene setup needs
        // GameAssets headless), then flip to Sandbox so F1 is armed - the
        // gate reads the resource at press time.
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        assert_eq!(app.world().resource::<Unloads>().0, 0);

        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "delivery guard: the press was seen and Editor is queued"
        );
        assert_eq!(
            app.world().resource::<Unloads>().0,
            1,
            "the same press must unload the scenario"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "the editor fired no scenario load of its own on this route"
        );
    }

    /// Sandbox mode heads for the editor scene, exactly as before the menu. The
    /// full editor path (scene setup needs GameAssets) is covered end to end by
    /// the 09_editor smoke run; this pins just the state routing.
    #[test]
    fn sandbox_heads_to_editor_state() {
        let mut app = app();
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        // A single transition step: entering Editor would run setup_editor_scene,
        // which needs GameAssets, so only assert the queued target.
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, None, "nothing queued before Playing is applied");
        app.world_mut()
            .run_schedule(bevy::state::state::StateTransition);
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, Some(ExampleStates::Editor));
    }

    // -- section keybind labels + rebind (task 20260712-163912) --

    #[test]
    fn keybind_labels_reconcile_to_one_per_bound_section() {
        let mut world = World::new();
        let section = world
            .spawn(SpaceshipThrusterInputBinding(vec![Binding::from(
                KeyCode::KeyW,
            )]))
            .id();
        // A non-bindable section (hull/controller have no binding) gets no label.
        let _unbound = world.spawn(SectionMarker).id();

        world.run_system_once(sync_section_keybind_labels).unwrap();
        let labels: Vec<Entity> = world
            .query::<&SectionKeybindLabel>()
            .iter(&world)
            .map(|l| l.section)
            .collect();
        assert_eq!(
            labels,
            vec![section],
            "one label, for the bound section only"
        );

        // Idempotent: a second pass adds no duplicate.
        world.run_system_once(sync_section_keybind_labels).unwrap();
        assert_eq!(
            world.query::<&SectionKeybindLabel>().iter(&world).count(),
            1
        );

        // Section gone -> its label is despawned.
        world.despawn(section);
        world.run_system_once(sync_section_keybind_labels).unwrap();
        assert_eq!(
            world.query::<&SectionKeybindLabel>().iter(&world).count(),
            0
        );
    }

    #[test]
    fn rebind_replaces_the_keyboard_bind_on_component_and_config() {
        let mut world = World::new();
        world.init_resource::<EditorRebind>();
        world.init_resource::<PlayerSpaceshipConfig>();
        let section = world
            .spawn(SpaceshipThrusterInputBinding(vec![
                Binding::from(KeyCode::Space),
                Binding::from(GamepadButton::RightTrigger),
            ]))
            .id();
        world.resource_mut::<EditorRebind>().target = Some(section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        let binds = &world
            .entity(section)
            .get::<SpaceshipThrusterInputBinding>()
            .unwrap()
            .0;
        assert!(
            binds
                .iter()
                .any(|b| matches!(b, Binding::Keyboard { key, .. } if *key == KeyCode::KeyR)),
            "the new key is bound"
        );
        assert!(
            !binds
                .iter()
                .any(|b| matches!(b, Binding::Keyboard { key, .. } if *key == KeyCode::Space)),
            "the old key is replaced"
        );
        assert!(
            binds.iter().any(|b| matches!(b, Binding::GamepadButton(_))),
            "a non-keyboard bind is preserved"
        );
        // The scenario reads player_config.inputs, so it must update too.
        assert!(world
            .resource::<PlayerSpaceshipConfig>()
            .inputs
            .get(&section)
            .is_some_and(|b| b
                .iter()
                .any(|b| matches!(b, Binding::Keyboard { key, .. } if *key == KeyCode::KeyR))));
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "the rebind is consumed"
        );
    }

    #[test]
    fn rebind_escape_cancels_without_changing_the_bind() {
        let mut world = World::new();
        world.init_resource::<EditorRebind>();
        world.init_resource::<PlayerSpaceshipConfig>();
        let section = world
            .spawn(SpaceshipTurretInputBinding(vec![Binding::from(
                KeyCode::Space,
            )]))
            .id();
        world.resource_mut::<EditorRebind>().target = Some(section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Escape);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        let binds = &world
            .entity(section)
            .get::<SpaceshipTurretInputBinding>()
            .unwrap()
            .0;
        assert_eq!(binds, &vec![Binding::from(KeyCode::Space)], "unchanged");
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "Escape still consumes the arm"
        );
    }

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

    #[test]
    fn rebind_binds_a_mouse_button_after_the_arming_click_releases() {
        let mut world = World::new();
        world.init_resource::<EditorRebind>();
        world.init_resource::<PlayerSpaceshipConfig>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        // Turret with a KEYBOARD primary + a gamepad bind; we'll swap the primary
        // to LMB.
        let section = world
            .spawn(SpaceshipTurretInputBinding(vec![
                Binding::from(KeyCode::Space),
                Binding::from(GamepadButton::RightTrigger2),
            ]))
            .id();
        {
            let mut r = world.resource_mut::<EditorRebind>();
            r.target = Some(section);
            r.awaiting_release = true; // armed by a click
        }
        // The arming LMB is still held.
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        // Click still down -> capture nothing, keep waiting (must not bind the
        // arming click).
        world.run_system_once(apply_section_rebind).unwrap();
        assert!(world.resource::<EditorRebind>().awaiting_release);
        assert_eq!(world.resource::<EditorRebind>().target, Some(section));

        // Release the arming click -> ready, still armed, nothing bound yet.
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        world.run_system_once(apply_section_rebind).unwrap();
        assert!(!world.resource::<EditorRebind>().awaiting_release);
        assert_eq!(world.resource::<EditorRebind>().target, Some(section));

        // A fresh LMB press now binds it.
        {
            let mut m = world.resource_mut::<ButtonInput<MouseButton>>();
            m.clear();
            m.press(MouseButton::Left);
        }
        world.run_system_once(apply_section_rebind).unwrap();

        let binds = &world
            .entity(section)
            .get::<SpaceshipTurretInputBinding>()
            .unwrap()
            .0;
        assert!(
            binds.iter().any(
                |b| matches!(b, Binding::MouseButton { button, .. } if *button == MouseButton::Left)
            ),
            "LMB is now bound"
        );
        assert!(
            !binds.iter().any(|b| matches!(b, Binding::Keyboard { .. })),
            "the old keyboard primary is replaced"
        );
        assert!(
            binds.iter().any(|b| matches!(b, Binding::GamepadButton(_))),
            "the gamepad bind is preserved"
        );
        assert!(
            world
                .resource::<PlayerSpaceshipConfig>()
                .inputs
                .get(&section)
                .is_some_and(|b| b.iter().any(|b| matches!(b, Binding::MouseButton { .. }))),
            "config (read on hand-off) updated"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "rebind consumed"
        );
    }
}
