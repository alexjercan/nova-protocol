//! Building the preview ship: creating a fresh ship, rebuilding it from the
//! surviving config on re-entry, and the pointer path that places, previews and
//! deletes sections.
//!
//! A part is placed by MATING link points (see [`crate::snap`]), never by
//! offsetting along a hit normal, so the editor can only build ships the
//! runtime's own integrity graph accepts. Nothing here spawns live physics - it
//! only edits `PlayerSpaceshipConfig` and the pickable preview entities.

use bevy::{
    input::mouse::MouseWheel, picking::pointer::PointerInteraction, prelude::*,
    ui_widgets::Activate,
};
use bevy_enhanced_input::prelude::Binding;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{ButtonValue, Selected},
    theme,
};

use crate::{
    config::{
        Placement, PlacementPose, PlacementPreview, PlacementStatus, PlayerSpaceshipConfig,
        SectionChoice, SectionGhost, SectionPreviewMarker, SpaceshipPreviewMarker,
    },
    keybind::EditorRebind,
    preview::{insert_preview_section, PreviewRole},
    snap::{self, PlacedSection},
    ExampleStates,
};

/// Rolls the ghost a quarter turn about the mating axis.
const PLACEMENT_ROLL_KEY: KeyCode = KeyCode::KeyR;
/// Cycles which socket of the armed part does the mating.
const PLACEMENT_SOCKET_KEY: KeyCode = KeyCode::KeyF;
/// Arms whatever section is under the pointer, Factorio-pipette style.
const PLACEMENT_PICK_KEY: KeyCode = KeyCode::KeyQ;

/// How far a socket marker is drawn, and how long its normal stub runs.
const SOCKET_MARKER_RADIUS: f32 = 0.07;
/// Length of the stub that shows which way a socket faces.
const SOCKET_NORMAL_LENGTH: f32 = 0.4;

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

/// Spawn one preview section under the preview ship, through the shared
/// [`insert_preview_section`] spawner - so click-placement, the on-enter
/// rebuild and the gallery tiles cannot drift apart.
fn spawn_preview_section(
    parent: &mut ChildSpawnerCommands,
    section: &SectionConfig,
    transform: Transform,
    binds: Vec<Binding>,
) -> Entity {
    let mut entity = parent.spawn(transform);
    insert_preview_section(&mut entity, section, PreviewRole::Section, binds);
    entity.id()
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

/// Keep the rail and drawer selection in step with the armed tool, whoever set
/// it.
///
/// A button moves its own `Selected` marker when IT is pressed, but the gallery
/// arms a part by writing the resource - and a rail chip still lit for the
/// previous tool shows the player a tool they are not holding.
pub(crate) fn sync_tool_selection(
    mut commands: Commands,
    choice: Res<SectionChoice>,
    buttons: Query<(Entity, &ButtonValue<SectionChoice>, Has<Selected>)>,
) {
    if !choice.is_changed() {
        return;
    }
    for (entity, value, selected) in &buttons {
        match (value.0 == *choice, selected) {
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

pub(crate) fn continue_to_simulation(
    _activate: On<Activate>,
    mut game_state: ResMut<NextState<ExampleStates>>,
) {
    game_state.set(ExampleStates::Scenario);
}

/// One step around a cycle of `len`, forwards or `back`.
fn step_cycle(current: usize, len: usize, back: bool) -> usize {
    if len == 0 {
        return 0;
    }
    if back {
        (current + len - 1) % len
    } else {
        (current + 1) % len
    }
}

/// The number of sockets the armed part carries, or `None` when nothing is
/// armed.
fn armed_socket_count(selection: &SectionChoice, sections: &GameSections) -> Option<usize> {
    let SectionChoice::Section(id) = selection else {
        return None;
    };
    Some(sections.get_section(id)?.base.link_points.len())
}

/// Cycle the placement pose: which of the part's sockets does the mating, and
/// how far it is rolled about the mating axis.
///
/// Both are the builder's choice and neither can be derived - a mate leaves the
/// roll free by definition, and which face of a part points at the ship is a
/// design decision, not a geometry one. Inert unless a part is armed, so the
/// keys stay free for the select/rebind tool.
///
/// The keys step FORWARD only; the wheel (see [`wheel_placement_pose`]) is the
/// reversible half of the same control, because overshooting the socket you
/// wanted on a six-socket part should cost one gesture back, not five more
/// forward.
pub(crate) fn cycle_placement_pose(
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    mut pose: ResMut<PlacementPose>,
) {
    let Some(sockets) = armed_socket_count(&selection, &sections) else {
        return;
    };
    if keys.just_pressed(PLACEMENT_ROLL_KEY) {
        pose.roll = step_cycle(pose.roll as usize, 4, false) as u32;
    }
    if keys.just_pressed(PLACEMENT_SOCKET_KEY) && sockets > 0 {
        pose.source = step_cycle(pose.source % sockets, sockets, false);
    }
}

/// Roll the ghost with the wheel, and cycle its socket with Shift+wheel - the
/// reversible half of [`cycle_placement_pose`].
///
/// The wheel is free in the editor (the free-fly rig drives off WASD), and it
/// is the one gesture that reads as "turn this a bit either way".
pub(crate) fn wheel_placement_pose(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    mut pose: ResMut<PlacementPose>,
) {
    let Some(sockets) = armed_socket_count(&selection, &sections) else {
        // Drained, so a scroll made before a part was armed is not replayed
        // into the pose on the frame it arms.
        wheel.clear();
        return;
    };
    let socket_modifier = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    for event in wheel.read() {
        // Line and pixel units both arrive here; only the SIGN is a gesture.
        let back = match event.y.partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => false,
            Some(std::cmp::Ordering::Less) => true,
            _ => continue,
        };
        if socket_modifier {
            if sockets > 0 {
                pose.source = step_cycle(pose.source % sockets, sockets, back);
            }
        } else {
            pose.roll = step_cycle(pose.roll as usize, 4, back) as u32;
        }
    }
}

/// Arm whatever prototype the section under the pointer was built from -
/// Factorio's pipette.
///
/// Finding a part you can SEE on the ship meant opening the gallery and
/// recognising it in a grid; this is the shortcut for "another one of those".
/// Reads the build state rather than the catalog because that is where a placed
/// section's prototype is recorded.
pub(crate) fn pick_section_under_pointer(
    keys: Res<ButtonInput<KeyCode>>,
    q_pointer: Query<&PointerInteraction>,
    player_config: Res<PlayerSpaceshipConfig>,
    q_section: Query<(), With<SectionMarker>>,
    mut selection: ResMut<SectionChoice>,
) {
    if !keys.just_pressed(PLACEMENT_PICK_KEY) {
        return;
    }
    let Some(entity) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .map(|(entity, _)| *entity)
        .find(|entity| q_section.get(*entity).is_ok())
    else {
        return;
    };
    let Some(SectionSource::Inline(config)) = player_config
        .sections
        .get(&entity)
        .map(|section| &section.source)
    else {
        return;
    };
    let picked = SectionChoice::Section(config.base.id.clone());
    if *selection != picked {
        *selection = picked;
    }
}

/// Recompute what a click would build, from the section under the pointer.
///
/// One solve per frame feeds both the ghost and the click, so the builder
/// cannot commit something other than what is on screen.
pub(crate) fn update_placement_preview(
    q_pointer: Query<&PointerInteraction>,
    spaceship: Option<Single<&Children, With<SpaceshipPreviewMarker>>>,
    q_section: Query<(&Transform, &SectionLinkPoints, &SectionCollider), With<SectionMarker>>,
    selection: Res<SectionChoice>,
    pose: Res<PlacementPose>,
    sections: Res<GameSections>,
    mut preview: ResMut<PlacementPreview>,
) {
    preview.placement = None;

    let SectionChoice::Section(id) = &*selection else {
        return;
    };
    let (Some(spaceship), Some(part)) = (spaceship, sections.get_section(id)) else {
        return;
    };

    // The ship as the solver sees it, in the order its indices refer to.
    let mut entities = Vec::new();
    let mut ship = Vec::new();
    for child in spaceship.iter() {
        let Ok((transform, link_points, collider)) = q_section.get(child) else {
            continue;
        };
        entities.push(child);
        ship.push(PlacedSection {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: link_points.to_vec(),
            collider: *collider,
        });
    }

    let Some((hovered, hit)) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(entity, hit)| {
            let index = entities.iter().position(|section| section == entity)?;
            Some((index, hit.position?))
        })
    else {
        return;
    };

    preview.placement = Some(Placement {
        prototype: id.clone(),
        target_section: entities[hovered],
        solve: snap::solve(
            &ship,
            hovered,
            hit,
            &part.base.link_points,
            part.base.collider.unwrap_or_default(),
            pose.source,
            pose.roll,
        ),
    });
}

/// Show the solved placement: the part's real mesh at the pose a click would
/// build, a bounds box in the colour of its verdict, and the refusal in words.
pub(crate) fn sync_placement_ghost(
    mut commands: Commands,
    preview: Res<PlacementPreview>,
    sections: Res<GameSections>,
    mut gizmos: Gizmos,
    ghosts: Query<(Entity, &SectionGhost, &mut Transform)>,
    q_link_points: Query<&SectionLinkPoints>,
    status: StatusQuery,
) {
    let wanted = preview.placement.as_ref().map(|placement| SectionGhost {
        prototype: placement.prototype.clone(),
        source: placement.solve.source,
    });

    // The mesh is rebuilt only when the PART or its socket choice changes; a
    // pose change is a transform write, so dragging the pointer across a hull
    // does not respawn a scene every frame.
    let mut kept = false;
    for (entity, ghost, mut transform) in ghosts {
        match (&wanted, preview.placement.as_ref()) {
            (Some(wanted), Some(placement))
                if ghost.prototype == wanted.prototype && ghost.source == wanted.source =>
            {
                *transform = placement.solve.transform;
                kept = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }

    let Some(placement) = preview.placement.as_ref() else {
        set_status(status, None);
        return;
    };
    set_status(
        status,
        Some(match placement.solve.refusal {
            Some(refusal) => (refusal.message().to_string(), theme::RED),
            // Naming the mate is the readout: which socket of the ship the part
            // is about to take, and which of its own it takes it with. The keys
            // that change that answer live in the legend now, so the line does
            // not repeat them under the pointer every frame.
            None => (
                format!(
                    "{} <- {}",
                    socket_id(
                        q_link_points.get(placement.target_section).ok(),
                        placement.solve.target
                    ),
                    sections.get_section(&placement.prototype).map_or_else(
                        String::new,
                        |section| {
                            section
                                .base
                                .link_points
                                .get(placement.solve.source)
                                .map_or_else(String::new, |point| point.id.clone())
                        }
                    ),
                ),
                theme::PHOSPHOR_MUTED,
            ),
        }),
    );

    if let (false, Some(section)) = (kept, sections.get_section(&placement.prototype)) {
        let mut entity = commands.spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Placement Ghost"),
            SectionGhost {
                prototype: placement.prototype.clone(),
                source: placement.solve.source,
            },
            SectionPreviewMarker,
            placement.solve.transform,
            // The ghost sits between the pointer and the ship it is being
            // placed on: it must never take the ray that positions it.
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
        ));
        insert_preview_section(&mut entity, section, PreviewRole::Display, vec![]);
    }

    // The bounds box is the verdict: the mesh alone cannot say "refused".
    if let Some(section) = sections.get_section(&placement.prototype) {
        let half = section
            .base
            .collider
            .unwrap_or_default()
            .aabb_half_extents();
        let colour = match placement.solve.refusal {
            None => theme::PHOSPHOR,
            Some(_) => theme::RED,
        };
        gizmos.cube(placement.solve.transform.with_scale(half * 2.0), colour);
    }
}

/// The diagnostic id of one socket on a live section, for the readout.
fn socket_id(link_points: Option<&SectionLinkPoints>, index: usize) -> String {
    link_points
        .and_then(|points| points.get(index))
        .map_or_else(String::new, |point| point.id.clone())
}

/// Write the placement readout, or hide it when nothing is being placed.
type StatusQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text,
        &'static mut TextColor,
        &'static mut BorderColor,
        &'static mut Visibility,
    ),
    With<PlacementStatus>,
>;

fn set_status(status: StatusQuery, line: Option<(String, Color)>) {
    for (mut text, mut colour, mut border, mut visibility) in status {
        match &line {
            Some((message, tint)) => {
                if text.0 != *message {
                    text.0 = message.clone();
                }
                if colour.0 != *tint {
                    colour.0 = *tint;
                    *border = BorderColor::all(*tint);
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

/// Place, delete or arm a rebind, depending on the armed tool.
///
/// Placement itself commits [`PlacementPreview`] - the pose the ghost is
/// already showing - rather than re-deriving anything from this click.
#[allow(clippy::too_many_arguments)]
pub(crate) fn on_click_spaceship_section(
    click: On<Pointer<Press>>,
    mut commands: Commands,
    spaceship: Single<Entity, With<SpaceshipPreviewMarker>>,
    selection: Res<SectionChoice>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    sections: Res<GameSections>,
    preview: Res<PlacementPreview>,
    mut player_config: ResMut<PlayerSpaceshipConfig>,
    mut rebind: ResMut<EditorRebind>,
    q_section: Query<(), With<SectionMarker>>,
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
    if q_section.get(entity).is_err() {
        return;
    }

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
        SectionChoice::Section(_) => {
            let Some(placement) = preview.placement.as_ref() else {
                return;
            };
            // A refused placement is shown red and stays unbuilt: the ship it
            // would make is one the runtime graph rejects.
            if let Some(refusal) = placement.solve.refusal {
                debug!(
                    "editor: placement refused ({}) for '{}'",
                    refusal.message(),
                    placement.prototype
                );
                return;
            }
            let Some(section) = required_section(&sections, &placement.prototype) else {
                return;
            };

            let transform = placement.solve.transform;
            let binds = default_binds_for(&section.kind, keyboard.as_deref(), gamepad.as_deref());
            let mut placed = Entity::PLACEHOLDER;
            commands
                .entity(spaceship.into_inner())
                .with_children(|parent| {
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
        }
    }
}

/// Draw the ship's sockets while a part is armed, and the socket on the armed
/// part that is about to mate.
///
/// Link points are the only thing placement snaps to, and until now they were
/// invisible: the builder swept the pointer across a hull and read the status
/// line to find out where the part would go. A free socket draws as a ring with
/// a stub along its normal, the socket under the pointer draws bright, and the
/// armed part's own mating socket draws on the ghost - so "which link point"
/// becomes something you point at rather than something you count presses to.
pub(crate) fn draw_link_points(
    preview: Res<PlacementPreview>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    spaceship: Option<Single<&Children, With<SpaceshipPreviewMarker>>>,
    q_section: Query<(&Transform, &SectionLinkPoints), With<SectionMarker>>,
    mut gizmos: Gizmos,
) {
    let (SectionChoice::Section(_), Some(spaceship)) = (&*selection, spaceship) else {
        return;
    };

    let mut entities = Vec::new();
    let mut ship = Vec::new();
    for child in spaceship.iter() {
        let Ok((transform, link_points)) = q_section.get(child) else {
            continue;
        };
        entities.push(child);
        ship.push(PlacedSectionLinkPoints {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: link_points.as_slice(),
        });
    }

    // A socket that already carries a neighbour is not somewhere a part can go,
    // so it is not drawn: the markers on screen are exactly the free ones.
    let taken: std::collections::BTreeSet<(usize, usize)> = candidate_link_point_mates(&ship)
        .iter()
        .flat_map(|mate| [mate.a, mate.b])
        .map(|reference| (reference.section_index, reference.link_point_index))
        .collect();

    let aimed = preview.placement.as_ref().and_then(|placement| {
        let section = entities
            .iter()
            .position(|e| *e == placement.target_section)?;
        Some((section, placement.solve.target))
    });

    for (section_index, section) in ship.iter().enumerate() {
        for (point_index, point) in section.link_points.iter().enumerate() {
            if taken.contains(&(section_index, point_index)) {
                continue;
            }
            let position = section.position + section.rotation * point.position;
            let normal = (section.rotation * point.normal).normalize_or(Vec3::Z);
            let colour = if aimed == Some((section_index, point_index)) {
                theme::PHOSPHOR
            } else {
                theme::PHOSPHOR_MUTED
            };
            draw_socket(&mut gizmos, position, normal, colour);
        }
    }

    // The part's own mating socket, drawn where the ghost puts it, in the
    // colour of the verdict - which is what says "this end goes here".
    let Some(placement) = preview.placement.as_ref() else {
        return;
    };
    let Some(source) = sections
        .get_section(&placement.prototype)
        .and_then(|section| section.base.link_points.get(placement.solve.source))
    else {
        return;
    };
    let transform = placement.solve.transform;
    draw_socket(
        &mut gizmos,
        transform.translation + transform.rotation * source.position,
        (transform.rotation * source.normal).normalize_or(Vec3::Z),
        match placement.solve.refusal {
            None => theme::PHOSPHOR,
            Some(_) => theme::RED,
        },
    );
}

/// One socket marker: a ring on the socket's plane plus a stub along its
/// normal, so both WHERE it is and which way it faces read at a glance.
fn draw_socket(gizmos: &mut Gizmos, position: Vec3, normal: Vec3, colour: Color) {
    gizmos.circle(
        Isometry3d::new(position, Quat::from_rotation_arc(Vec3::Z, normal)),
        SOCKET_MARKER_RADIUS,
        colour,
    );
    gizmos.line(position, position + normal * SOCKET_NORMAL_LENGTH, colour);
}

/// Draw the preview ship's forward direction.
///
/// A ship under assembly is a pile of boxes with no obvious front, and forward
/// is -Z here as it is everywhere else in the game - a fact the editor never
/// showed, so a builder could only find out which way their thrusters pointed
/// by flying it.
pub(crate) fn draw_ship_heading(
    spaceship: Option<Single<&Transform, With<SpaceshipPreviewMarker>>>,
    q_section: Query<(&Transform, &SectionCollider), With<SectionMarker>>,
    mut gizmos: Gizmos,
) {
    let Some(spaceship) = spaceship else {
        return;
    };
    // Reach past the nose of the ship, so the arrow is never buried inside it.
    let reach = q_section
        .iter()
        .map(|(transform, collider)| {
            (transform.translation.z - collider.aabb_half_extents().z).abs()
        })
        .fold(1.0f32, f32::max)
        + 1.0;
    let origin = spaceship.translation;
    let nose = origin + spaceship.rotation * (Vec3::NEG_Z * reach);
    gizmos.arrow(origin, nose, theme::AMBER_NOVA);
}

/// Outline the section a delete click would remove.
pub(crate) fn draw_delete_target(
    q_pointer: Query<&PointerInteraction>,
    q_section: Query<(&Transform, &SectionCollider), With<SectionMarker>>,
    selection: Res<SectionChoice>,
    mut gizmos: Gizmos,
) {
    if !matches!(*selection, SectionChoice::Delete) {
        return;
    }
    let Some((entity, _)) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .next()
    else {
        return;
    };
    let Ok((transform, collider)) = q_section.get(*entity) else {
        return;
    };
    gizmos.cube(
        transform.with_scale(collider.aabb_half_extents() * 2.0),
        theme::RED,
    );
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
