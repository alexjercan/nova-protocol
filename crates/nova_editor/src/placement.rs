//! Building the preview ship: creating a fresh ship, rebuilding it from the
//! surviving config on re-entry, and the pointer observers that place / preview
//! / delete sections by raycasting the hovered section and offsetting along its
//! surface normal. Nothing here spawns live physics - it only edits
//! `PlayerSpaceshipConfig` and the pickable preview entities.

use bevy::{picking::pointer::PointerInteraction, prelude::*, ui_widgets::Activate};
use bevy_enhanced_input::prelude::Binding;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use crate::{
    config::{PlayerSpaceshipConfig, SectionChoice, SectionPreviewMarker, SpaceshipPreviewMarker},
    keybind::EditorRebind,
    ExampleStates,
};

/// Keys the editor's own WASD camera drives (`wasd_controller`). Binding one to
/// a section makes that section fire on every camera move once the ship flies,
/// so placement never captures them.
const EDITOR_CAMERA_KEYS: [KeyCode; 6] = [
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::Space,
    KeyCode::ShiftLeft,
];

/// The catalog section for `id`, or `None` (logged) when a mod overlay dropped
/// or renamed it. Every catalog lookup in the editor goes through this: an
/// overlay must degrade the editor to "no preview", never panic the process.
fn required_section<'a>(sections: &'a GameSections, id: &str) -> Option<&'a SectionConfig> {
    let section = sections.get_section(id);
    if section.is_none() {
        warn!("editor: section '{id}' is not in the section catalog - skipping");
    }
    section
}

/// The button to bind to a section placed this frame: the lowest button *just*
/// pressed and not `reserved`. `just_pressed` rather than `pressed` so a held
/// camera key cannot be captured, and lowest rather than first because
/// `ButtonInput` iterates a `HashSet` - W+D would otherwise bind either one.
fn capture_binding<T>(input: &ButtonInput<T>, reserved: &[T]) -> Option<T>
where
    T: Copy + Ord + core::hash::Hash + Send + Sync + 'static,
{
    input
        .get_just_pressed()
        .copied()
        .filter(|button| !reserved.contains(button))
        .min()
}

/// The bindings a freshly placed section gets: the button the player pressed as
/// they clicked, else the kind's default. A missing input resource (headless)
/// contributes no binding at all.
fn placement_binds(
    keyboard: Option<&ButtonInput<KeyCode>>,
    gamepad: Option<&ButtonInput<GamepadButton>>,
    default_key: Binding,
    default_pad: Binding,
) -> Vec<Binding> {
    let mut binds = Vec::new();
    if let Some(keyboard) = keyboard {
        binds.push(
            capture_binding(keyboard, &EDITOR_CAMERA_KEYS).map_or(default_key, Binding::from),
        );
    }
    if let Some(gamepad) = gamepad {
        binds.push(capture_binding(gamepad, &[]).map_or(default_pad, Binding::from));
    }
    binds
}

/// The bindings a section of this kind takes when placed. Hull and controller
/// sections are not bindable and take none.
fn default_binds_for(
    kind: &SectionKind,
    keyboard: Option<&ButtonInput<KeyCode>>,
    gamepad: Option<&ButtonInput<GamepadButton>>,
) -> Vec<Binding> {
    match kind {
        SectionKind::Hull(_) | SectionKind::Controller(_) => vec![],
        SectionKind::Thruster(_) => placement_binds(
            keyboard,
            gamepad,
            KeyCode::Space.into(),
            GamepadButton::RightTrigger.into(),
        ),
        SectionKind::Turret(_) | SectionKind::Torpedo(_) => placement_binds(
            keyboard,
            gamepad,
            MouseButton::Left.into(),
            GamepadButton::RightTrigger2.into(),
        ),
    }
}

/// How a section of this kind sits against the surface it was placed on.
fn placement_rotation(kind: &SectionKind, normal: Vec3) -> Quat {
    match kind {
        SectionKind::Hull(_) | SectionKind::Controller(_) => Quat::IDENTITY,
        SectionKind::Thruster(_) => Quat::from_rotation_arc(Vec3::Z, normal.normalize()),
        SectionKind::Turret(_) | SectionKind::Torpedo(_) => {
            Quat::from_rotation_arc(Vec3::Y, normal.normalize())
        }
    }
}

/// Spawn one preview section under the preview ship. The single place that
/// knows how a [`SectionConfig`] becomes preview entities, so click-placement
/// and the on-enter rebuild cannot drift apart.
fn spawn_preview_section(
    parent: &mut ChildSpawnerCommands,
    section: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let base = preview_section(section.base.clone());
    match &section.kind {
        SectionKind::Hull(hull) => parent
            .spawn((base, transform, hull_section(hull.clone())))
            .id(),
        SectionKind::Controller(controller) => parent
            .spawn((
                base,
                transform,
                preview_controller_section(controller.clone()),
            ))
            .id(),
        SectionKind::Thruster(thruster) => parent
            .spawn((
                base,
                transform,
                thruster_section(thruster.clone()),
                SpaceshipThrusterInputBinding(binds),
            ))
            .id(),
        SectionKind::Turret(turret) => parent
            .spawn((
                base,
                transform,
                turret_section(turret.clone()),
                SpaceshipTurretInputBinding(binds),
            ))
            .id(),
        SectionKind::Torpedo(torpedo) => parent
            .spawn((
                base,
                transform,
                torpedo_section(torpedo.clone()),
                SpaceshipTorpedoInputBinding(binds),
            ))
            .id(),
    }
}

/// Record a freshly spawned preview section in the build state. The config id
/// IS the preview entity: `sandbox_objects` keys the scenario's `input_mapping`
/// by that same entity, so the two must not diverge.
fn register_preview_section(
    config: &mut PlayerSpaceshipConfig,
    entity: Entity,
    section: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) {
    config.sections.insert(
        entity,
        SpaceshipSectionConfig {
            id: entity.to_string(),
            position: transform.translation,
            rotation: transform.rotation,
            source: SectionSource::Inline(section.clone()),
            modifications: vec![],
        },
    );
    if !binds.is_empty() {
        config.inputs.insert(entity, binds);
    }
}

/// Replace the preview ship with a fresh one seeded from a single catalog
/// section, and reset the build state to match.
fn reset_preview_to_seed(
    commands: &mut Commands,
    q_spaceship: &Query<Entity, With<SpaceshipPreviewMarker>>,
    section: &SectionConfig,
    name: &'static str,
) {
    for entity in q_spaceship {
        commands.entity(entity).despawn();
    }

    let root = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipPreviewMarker,
            Name::new(name),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    let transform = Transform::default();
    let mut seed = Entity::PLACEHOLDER;
    commands.entity(root).with_children(|parent| {
        seed = spawn_preview_section(parent, section, transform, vec![]);
    });

    let mut config = PlayerSpaceshipConfig::default();
    register_preview_section(&mut config, seed, section, transform, vec![]);
    commands.insert_resource(config);
}

pub(crate) fn create_new_spaceship(
    _activate: On<Activate>,
    mut commands: Commands,
    q_spaceship: Query<Entity, With<SpaceshipPreviewMarker>>,
    sections: Res<GameSections>,
) {
    let Some(section) = required_section(&sections, "reinforced_hull_section") else {
        return;
    };
    if !matches!(section.kind, SectionKind::Hull(_)) {
        warn!("editor: 'reinforced_hull_section' is not a hull section - skipping");
        return;
    }
    reset_preview_to_seed(&mut commands, &q_spaceship, section, "Spaceship Preview");
}

pub(crate) fn create_new_spaceship_with_controller(
    _activate: On<Activate>,
    mut commands: Commands,
    q_spaceship: Query<Entity, With<SpaceshipPreviewMarker>>,
    sections: Res<GameSections>,
) {
    let Some(section) = required_section(&sections, "basic_controller_section") else {
        return;
    };
    if !matches!(section.kind, SectionKind::Controller(_)) {
        warn!("editor: 'basic_controller_section' is not a controller section - skipping");
        return;
    }
    reset_preview_to_seed(
        &mut commands,
        &q_spaceship,
        section,
        "Spaceship Preview with Controller",
    );
}

/// Rebuild the preview ship from [`PlayerSpaceshipConfig`] on every entry into
/// the editor. The preview entities are `DespawnOnExit(Editor)` but the config
/// resource survives, so a second visit used to show nothing - every click
/// dropped - while Play still spawned the ship the first visit built. The
/// config is keyed by live preview entity, so both maps are re-keyed onto the
/// entities spawned here.
pub(crate) fn rebuild_editor_preview_on_enter(
    mut commands: Commands,
    mut player_config: ResMut<PlayerSpaceshipConfig>,
) {
    if player_config.sections.is_empty() {
        return;
    }

    let previous: Vec<(SpaceshipSectionConfig, Vec<Binding>)> = player_config
        .sections
        .iter()
        .map(|(entity, section)| {
            (
                section.clone(),
                player_config
                    .inputs
                    .get(entity)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect();

    let root = commands
        .spawn((
            DespawnOnExit(ExampleStates::Editor),
            SpaceshipPreviewMarker,
            Name::new("Spaceship Preview"),
            Transform::default(),
            Visibility::Visible,
        ))
        .id();

    let mut rebuilt = PlayerSpaceshipConfig::default();
    commands.entity(root).with_children(|parent| {
        for (section, binds) in &previous {
            // A prototype source would need the catalog, which a mod overlay may
            // have changed since; the editor only ever writes Inline.
            let SectionSource::Inline(config) = &section.source else {
                warn!(
                    "editor: preview section '{}' is not inline - dropping it from the rebuild",
                    section.id
                );
                continue;
            };
            let transform =
                Transform::from_translation(section.position).with_rotation(section.rotation);
            let entity = spawn_preview_section(parent, config, transform, binds.clone());
            register_preview_section(&mut rebuilt, entity, config, transform, binds.clone());
        }
    });

    *player_config = rebuilt;
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
            // NOTE: no placement tool selected = select/edit mode - clicking a
            // bindable section arms a rebind, and `apply_section_rebind`
            // captures the next key or mouse-button press. Non-bindable sections
            // (hull, controller) and empty space do nothing.
            //
            // Only arm when nothing is armed yet: while a rebind is pending, the
            // next click is the user PICKING a mouse-button binding (e.g. LMB),
            // so it must not re-arm on whatever is under the cursor.
            if rebind.target.is_none() && q_bindable.get(entity).is_ok() {
                rebind.target = Some(entity);
                // Wait for this arming click to release before capturing.
                rebind.awaiting_release = true;
            }
        }
        SectionChoice::Section(ref id) => {
            let Some(section) = required_section(&sections, id) else {
                return;
            };

            let transform = Transform::from_translation(position)
                .with_rotation(placement_rotation(&section.kind, normal));
            let binds = default_binds_for(&section.kind, keyboard.as_deref(), gamepad.as_deref());

            let mut placed = Entity::PLACEHOLDER;
            commands.entity(spaceship).with_children(|parent| {
                placed = spawn_preview_section(parent, section, transform, binds.clone());
            });
            register_preview_section(&mut player_config, placed, section, transform, binds);
        }
        SectionChoice::Delete => {
            commands.entity(entity).despawn();
            player_config.sections.remove(&entity);
            // The scenario's input_mapping is built from `inputs`; a leftover
            // entry would map a key to a section that no longer exists.
            player_config.inputs.remove(&entity);

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

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, platform::collections::HashMap};

    use super::*;

    fn hull_config(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                ..default()
            },
            kind: SectionKind::Hull(HullSectionConfig::default()),
        }
    }

    fn turret_config(id: &str) -> SectionConfig {
        SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                name: id.to_string(),
                ..default()
            },
            kind: SectionKind::Turret(TurretSectionConfig::default()),
        }
    }

    /// F11: a mod overlay that drops the seeded id must log and skip, not panic
    /// the process on "New Hull Ship".
    #[test]
    fn a_missing_seed_section_skips_instead_of_panicking() {
        let mut app = App::new();
        app.insert_resource(GameSections(vec![]));
        app.insert_resource(PlayerSpaceshipConfig::default());
        app.add_observer(create_new_spaceship);
        app.add_observer(create_new_spaceship_with_controller);
        let button = app.world_mut().spawn_empty().id();

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&SpaceshipPreviewMarker>()
                .iter(app.world())
                .count(),
            0,
            "no preview ship is built from a missing catalog id"
        );
        assert!(
            app.world()
                .resource::<PlayerSpaceshipConfig>()
                .sections
                .is_empty(),
            "and the build state is untouched"
        );
    }

    /// F11: the seeded id resolving to the WRONG kind is the other overlay
    /// failure - it used to `panic!` outright.
    #[test]
    fn a_retyped_seed_section_skips_instead_of_panicking() {
        let mut app = App::new();
        app.insert_resource(GameSections(vec![turret_config("reinforced_hull_section")]));
        app.insert_resource(PlayerSpaceshipConfig::default());
        app.add_observer(create_new_spaceship);
        let button = app.world_mut().spawn_empty().id();

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&SpaceshipPreviewMarker>()
                .iter(app.world())
                .count(),
            0,
            "a retyped seed builds no ship"
        );
    }

    /// The happy path, so the two skip tests above cannot pass vacuously.
    #[test]
    fn a_present_seed_section_builds_the_preview_ship() {
        let mut app = App::new();
        app.insert_resource(GameSections(vec![hull_config("reinforced_hull_section")]));
        app.insert_resource(PlayerSpaceshipConfig::default());
        app.add_observer(create_new_spaceship);
        let button = app.world_mut().spawn_empty().id();

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&SpaceshipPreviewMarker>()
                .iter(app.world())
                .count(),
            1
        );
        let config = app.world().resource::<PlayerSpaceshipConfig>();
        assert_eq!(config.sections.len(), 1, "the seed section is registered");
        let (entity, section) = config.sections.iter().next().unwrap();
        assert_eq!(
            section.id,
            entity.to_string(),
            "a section's config id is its preview entity"
        );
    }

    /// F29: a held camera key must not become the new section's binding, and a
    /// tie must resolve the same way every run.
    #[test]
    fn capture_skips_held_and_reserved_keys() {
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyW);
        assert_eq!(
            capture_binding(&input, &EDITOR_CAMERA_KEYS),
            None,
            "a camera key is never captured"
        );

        // A held (not just-pressed) key is not captured either: clearing the
        // just-pressed set is what a second frame of holding does.
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        input.clear();
        assert_eq!(
            capture_binding(&input, &[]),
            None,
            "held keys are not captured"
        );

        // W+R: W is reserved, so R wins - deterministically.
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyW);
        input.press(KeyCode::KeyR);
        assert_eq!(
            capture_binding(&input, &EDITOR_CAMERA_KEYS),
            Some(KeyCode::KeyR)
        );

        // Two free keys: the lowest, every time.
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        input.press(KeyCode::KeyT);
        let first = capture_binding(&input, &EDITOR_CAMERA_KEYS);
        assert!(first.is_some());
        for _ in 0..16 {
            assert_eq!(capture_binding(&input, &EDITOR_CAMERA_KEYS), first);
        }
    }

    /// F29: placing a turret while a camera key is held falls back to the
    /// kind's default instead of binding that key.
    #[test]
    fn placement_falls_back_to_the_default_when_only_camera_keys_are_held() {
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::KeyW);
        let binds = default_binds_for(
            &SectionKind::Turret(TurretSectionConfig::default()),
            Some(&keyboard),
            None,
        );
        assert_eq!(
            binds,
            vec![Binding::from(MouseButton::Left)],
            "W drives the camera, so the turret keeps its default"
        );
    }

    /// F31: re-entering the editor rebuilds the preview from the surviving
    /// config, and re-keys the config onto the new entities so Play still finds
    /// each section's bindings.
    #[test]
    fn re_entering_the_editor_rebuilds_the_preview_from_the_config() {
        let mut world = World::new();
        // A key left over from the previous visit: its entity is gone, exactly
        // as DespawnOnExit(Editor) leaves it.
        let stale = world.spawn_empty().id();
        world.despawn(stale);
        let turret = turret_config("turret");
        let binds = vec![Binding::from(KeyCode::KeyR)];
        world.insert_resource(PlayerSpaceshipConfig {
            sections: HashMap::from([(
                stale,
                SpaceshipSectionConfig {
                    id: stale.to_string(),
                    position: Vec3::new(1.0, 2.0, 3.0),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::Inline(turret.clone()),
                    modifications: vec![],
                },
            )]),
            inputs: HashMap::from([(stale, binds.clone())]),
        });

        world
            .run_system_once(rebuild_editor_preview_on_enter)
            .unwrap();

        assert_eq!(
            world
                .query::<&SpaceshipPreviewMarker>()
                .iter(&world)
                .count(),
            1,
            "the preview ship is back"
        );
        let live: Vec<Entity> = world
            .query_filtered::<Entity, With<SpaceshipTurretInputBinding>>()
            .iter(&world)
            .collect();
        assert_eq!(live.len(), 1, "the turret section is back");

        let config = world.resource::<PlayerSpaceshipConfig>();
        assert!(
            !config.sections.contains_key(&stale),
            "the dead entity key is gone"
        );
        assert_eq!(
            config.sections.keys().copied().collect::<Vec<_>>(),
            live,
            "the config is keyed by the live preview entity"
        );
        assert_eq!(config.sections[&live[0]].id, live[0].to_string());
        assert_eq!(
            config.inputs.get(&live[0]),
            Some(&binds),
            "the section keeps its bindings across the rebuild"
        );
        assert_eq!(
            world
                .entity(live[0])
                .get::<SpaceshipTurretInputBinding>()
                .unwrap()
                .0,
            binds,
            "and so does the live component"
        );
    }

    /// A first entry (nothing built yet) must not spawn an empty preview ship -
    /// the "New Ship" buttons still own creation.
    #[test]
    fn an_empty_config_rebuilds_nothing() {
        let mut world = World::new();
        world.init_resource::<PlayerSpaceshipConfig>();

        world
            .run_system_once(rebuild_editor_preview_on_enter)
            .unwrap();

        assert_eq!(
            world
                .query::<&SpaceshipPreviewMarker>()
                .iter(&world)
                .count(),
            0
        );
    }
}
