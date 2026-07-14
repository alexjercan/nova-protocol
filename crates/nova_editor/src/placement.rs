//! Building the preview ship: creating a fresh ship, and the pointer observers
//! that place / preview / delete sections by raycasting the hovered section and
//! offsetting along its surface normal. Nothing here spawns live physics - it
//! only edits `PlayerSpaceshipConfig` and the pickable preview entities.

use bevy::{
    picking::pointer::PointerInteraction, platform::collections::HashMap, prelude::*,
    ui_widgets::Activate,
};
use bevy_enhanced_input::prelude::Binding;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use crate::{
    config::{PlayerSpaceshipConfig, SectionChoice, SectionPreviewMarker, SpaceshipPreviewMarker},
    keybind::EditorRebind,
    ExampleStates,
};

pub(crate) fn create_new_spaceship(
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
                source: SectionSource::Inline(SectionConfig {
                    base,
                    kind: SectionKind::Hull(hull),
                }),
                modifications: vec![],
            },
        )]),
        ..default()
    });
}

pub(crate) fn create_new_spaceship_with_controller(
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
                source: SectionSource::Inline(SectionConfig {
                    base,
                    kind: SectionKind::Controller(controller),
                }),
                modifications: vec![],
            },
        )]),
        ..default()
    });
}

pub(crate) fn continue_to_simulation(
    _activate: On<Activate>,
    mut game_state: ResMut<NextState<ExampleStates>>,
) {
    game_state.set(ExampleStates::Scenario);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn on_click_spaceship_section(
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
                            source: SectionSource::Inline(section.clone()),
                            modifications: vec![],
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
                            source: SectionSource::Inline(section.clone()),
                            modifications: vec![],
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
                            source: SectionSource::Inline(section.clone()),
                            modifications: vec![],
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
                            source: SectionSource::Inline(section.clone()),
                            modifications: vec![],
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
                            source: SectionSource::Inline(section.clone()),
                            modifications: vec![],
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

pub(crate) fn on_hover_spaceship_section(
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

pub(crate) fn on_move_spaceship_section(
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

pub(crate) fn on_out_spaceship_section(
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
