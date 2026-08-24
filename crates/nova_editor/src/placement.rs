//! Building a ship: adding one to the document, and the pointer path that
//! places, previews and deletes its sections.
//!
//! A part is placed by MATING link points (see [`crate::snap`]), never by
//! offsetting along a hit normal, so the editor can only build ships the
//! runtime's own integrity graph accepts. Nothing here spawns live physics - it
//! edits the node tree ([`crate::node`]) and the views hung off it.
//!
//! Everything here works on the ship in the EDIT CONTEXT and no other. A click
//! on a ship you are not inside is an "enter", never a placement.

use bevy::{
    input::mouse::MouseWheel, picking::pointer::PointerInteraction, prelude::*,
    ui_widgets::Activate,
};
use bevy_enhanced_input::prelude::Binding;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{ButtonValue, Selected},
    theme,
};

use crate::{
    config::{
        Placement, PlacementPose, PlacementPreview, PlacementStatus, SectionChoice, SectionGhost,
        SectionPreviewMarker, SelectedNode,
    },
    keybind::EditorRebind,
    node::{
        node_of_view, sections_of, spawn_section_node, spawn_ship_node, EditContext,
        NextChildOrdinal, NodeView, SectionNode, SectionNodes, ShipDriver, ShipNode,
    },
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

/// Add a BLANK ship to the document and go inside it - the scenario context's
/// "Add Ship" action.
///
/// ADDITIVE, where this used to despawn whatever was on the stage and reset the
/// build state: the document holds ships, so a second "Add Ship" is one more
/// subtree standing beside the first rather than a reset. The new ship becomes
/// the edit context, so the builder lands inside the thing they just made.
/// Blank rather than seeded: which part a ship starts from is the builder's
/// first decision, and [`found_empty_ship`] is where they make it.
pub(crate) fn create_blank_ship(
    _activate: On<Activate>,
    mut commands: Commands,
    mut ordinals: Query<&mut NextChildOrdinal>,
    q_ships: Query<(), With<ShipNode>>,
    mut context: ResMut<EditContext>,
) {
    let ships = q_ships.iter().count();
    // The first ship of a document is the one the player flies; anything built
    // beside it is scenery until something says otherwise. A second Player ship
    // would make "which one do I fly" ambiguous, and the answer belongs to the
    // ship rather than to the order the buttons were pressed.
    let driver = if ships == 0 {
        ShipDriver::Player
    } else {
        ShipDriver::Ai
    };
    if spawn_ship_node(&mut commands, &mut ordinals, &mut context, ships, driver).is_none() {
        warn!("editor: no document to add a ship to - skipping");
    }
}

/// FOUND an empty ship: with a part armed and nothing under the pointer, a
/// click drops the first section at the ship's own origin.
///
/// A blank ship has no view for the placement ray to hit, so the mate solver
/// can never say where the first part goes - somebody has to pick a pose out
/// of nothing, and the ship origin is the one pose that means the same thing
/// in every saved file. The empty-space test doubles as the UI guard: a click
/// on any widget or any view lands a hit and is not a founding.
pub(crate) fn found_empty_ship(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    q_pointer: Query<&PointerInteraction>,
    q_windows: Query<(), With<bevy::window::Window>>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    context: Res<EditContext>,
    nodes: SectionNodes,
    mut commands: Commands,
    mut ordinals: Query<&mut NextChildOrdinal>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let SectionChoice::Section(id) = &*selection else {
        return;
    };
    let Some(ship) = context.ship() else {
        return;
    };
    if !sections_of(ship, &nodes).is_empty() {
        return;
    }
    // "Empty space" means the nearest hit is the WINDOW itself: bevy_picking
    // targets the window when no entity is under the pointer, so any other
    // nearest hit - a UI panel, a view - is a click on something.
    if q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .any(|(entity, _)| !q_windows.contains(*entity))
    {
        return;
    }
    let Some(config) = required_section(&sections, id) else {
        return;
    };
    let binds = default_binds_for(&config.kind, keyboard.as_deref(), gamepad.as_deref());
    spawn_section_node(
        &mut commands,
        &mut ordinals,
        ship,
        config,
        Transform::default(),
        binds,
    );
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

/// Compile the document and fly it.
///
/// Only from the scenario node. `crate::ui::sync_play_button` greys the button
/// out inside a ship, and this is the same rule stated where the transition
/// actually happens - a disabled button is a paint job, and the keyboard, the
/// autopilot and any future shortcut all arrive here instead.
pub(crate) fn continue_to_simulation(
    _activate: On<Activate>,
    context: Res<EditContext>,
    mut game_state: ResMut<NextState<ExampleStates>>,
) {
    if context.ship().is_some() {
        warn!("editor: Play compiles the whole scenario - leave the ship first");
        return;
    }
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

/// Roll the ghost with the wheel, and cycle its socket with Ctrl+wheel - the
/// reversible half of [`cycle_placement_pose`].
///
/// The wheel is free in the editor (the free-fly rig drives off WASD), and it
/// is the one gesture that reads as "turn this a bit either way". CTRL and not
/// Shift: Shift is the free-fly rig's descend key, so a builder holding it to
/// cycle a socket also sank the camera.
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
    let socket_modifier = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

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
    q_views: Query<&ChildOf, With<NodeView>>,
    q_nodes: Query<&SectionNode>,
    mut selection: ResMut<SectionChoice>,
) {
    if !keys.just_pressed(PLACEMENT_PICK_KEY) {
        return;
    }
    let Some(section) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(entity, _)| node_of_view(*entity, &q_views))
        .and_then(|node| q_nodes.get(node).ok())
    else {
        return;
    };
    let picked = SectionChoice::Section(section.prototype().to_string());
    if *selection != picked {
        *selection = picked;
    }
}

/// Put last frame's answer away before this frame's has a chance to be solved.
///
/// UNGATED, unlike [`update_placement_preview`], and that is the whole point.
/// The solver is gated on the gallery being CLOSED and ordered before the
/// gallery's own keyboard, so the frame a keystroke closes the overlay carries
/// no solve at all - while the pointer, the camera and the armed part have all
/// moved since the last one. Without this, that frame's answer is the build
/// view's from before the gallery went up, and anything reading the preview
/// (the public [`EditorProbe`](crate::EditorProbe) most of all) is told a click
/// would build something it would not.
///
/// So the invariant is: the answer is REBUILT every frame, and a frame with no
/// solve has no answer.
pub(crate) fn clear_placement_preview(mut preview: ResMut<PlacementPreview>) {
    if preview.placement.is_some() {
        preview.placement = None;
    }
}

/// Recompute what a click would build, from the section under the pointer.
///
/// One solve per frame feeds both the ghost and the click, so the builder
/// cannot commit something other than what is on screen.
pub(crate) fn update_placement_preview(
    q_pointer: Query<&PointerInteraction>,
    q_views: Query<&ChildOf, With<NodeView>>,
    // The ship being edited, and only it: a section on a ship you are not
    // inside is not something a click can build on.
    context: Res<EditContext>,
    nodes: SectionNodes,
    selection: Res<SectionChoice>,
    pose: Res<PlacementPose>,
    sections: Res<GameSections>,
    mut preview: ResMut<PlacementPreview>,
) {
    preview.placement = None;

    let SectionChoice::Section(id) = &*selection else {
        return;
    };
    let (Some(edited), Some(part)) = (context.ship(), sections.get_section(id)) else {
        return;
    };

    // The ship as the solver sees it, in the order its indices refer to. Read
    // out of the DOCUMENT rather than off the scene: the sockets, the footprint
    // and the exit are all properties of the section's config, and the config is
    // what the node carries.
    let mut entities = Vec::new();
    let mut ship = Vec::new();
    for (entity, _, section, transform) in sections_of(edited, &nodes) {
        let Some(config) = section.resolve(Some(&sections)) else {
            continue;
        };
        entities.push(entity);
        ship.push(PlacedSection {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: config.base.link_points.clone(),
            collider: config.base.collider.unwrap_or_default(),
            exit: exit_normal(&config.kind),
        });
    }

    let Some((hovered, hit)) = q_pointer
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .find_map(|(entity, hit)| {
            let node = node_of_view(*entity, &q_views)?;
            let index = entities.iter().position(|section| *section == node)?;
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
            exit_normal(&part.kind),
            pose.source,
            pose.roll,
        ),
    });
}

/// Show the solved placement: the part's real mesh at the pose a click would
/// build, a bounds box in the colour of its verdict, and the refusal in words.
///
/// The solve is in SHIP-LOCAL space, because that is the space the section node
/// it would become is posed in. The ghost is therefore parented to the ship, and
/// the bounds gizmo - which is drawn in world space - is put through the ship's
/// pose on the way out.
#[expect(
    clippy::too_many_arguments,
    reason = "the ghost reconciles a mesh, a gizmo and a status line from one solve"
)]
pub(crate) fn sync_placement_ghost(
    mut commands: Commands,
    preview: Res<PlacementPreview>,
    sections: Res<GameSections>,
    context: Res<EditContext>,
    selection: Res<SectionChoice>,
    // `&ChildOf` only: the ghost query below writes `Transform`, so reading the
    // section nodes through `SectionNodes` here would be a B0001 conflict.
    q_owners: Query<&ChildOf, With<SectionNode>>,
    mut gizmos: Gizmos,
    ghosts: Query<(Entity, &SectionGhost, &mut Transform)>,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    q_nodes: Query<&SectionNode>,
    status: StatusQuery,
) {
    let edited = context.ship();
    let wanted = preview
        .placement
        .as_ref()
        .zip(edited)
        .map(|(placement, ship)| SectionGhost {
            prototype: placement.prototype.clone(),
            source: placement.solve.source,
            ship,
        });

    // The mesh is rebuilt only when the PART, its socket choice or the ship it
    // is being placed on changes; a pose change is a transform write, so
    // dragging the pointer across a hull does not respawn a scene every frame.
    let mut kept = false;
    for (entity, ghost, mut transform) in ghosts {
        match (&wanted, preview.placement.as_ref()) {
            (Some(wanted), Some(placement))
                if ghost.prototype == wanted.prototype
                    && ghost.source == wanted.source
                    && ghost.ship == wanted.ship =>
            {
                *transform = placement.solve.transform;
                kept = true;
            }
            _ => commands.entity(entity).despawn(),
        }
    }

    let Some(placement) = preview.placement.as_ref() else {
        // A part in hand over an EMPTY ship has no view to solve against, so
        // the status line carries the founding rule instead of going dark -
        // without it, a blank Add Ship reads as an editor that stopped placing.
        let founding = matches!(*selection, SectionChoice::Section(_))
            && edited.is_some_and(|ship| !q_owners.iter().any(|owner| owner.parent() == ship));
        set_status(
            status,
            founding.then(|| {
                (
                    "click empty space - the first part founds the ship".to_string(),
                    theme::PHOSPHOR_MUTED,
                )
            }),
        );
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
                        q_nodes
                            .get(placement.target_section)
                            .ok()
                            .and_then(|section| section.resolve(Some(&sections))),
                        placement.solve.target
                    ),
                    socket_id(
                        sections.get_section(&placement.prototype),
                        placement.solve.source
                    ),
                ),
                theme::PHOSPHOR_MUTED,
            ),
        }),
    );

    let Some(ship) = edited else {
        return;
    };
    if let (false, Some(section)) = (kept, sections.get_section(&placement.prototype)) {
        let mut entity = commands.spawn((
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Placement Ghost"),
            SectionGhost {
                prototype: placement.prototype.clone(),
                source: placement.solve.source,
                ship,
            },
            SectionPreviewMarker,
            placement.solve.transform,
            // Parented to the ship it is being placed on, because the solve is
            // in that ship's space - a ghost hung off the world would sit at the
            // right pose on the wrong ship.
            ChildOf(ship),
            // The ghost sits between the pointer and the ship it is being
            // placed on: it must never take the ray that positions it.
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
        ));
        insert_preview_section(&mut entity, section, PreviewRole::Display);
    }

    // The bounds box is the verdict: the mesh alone cannot say "refused".
    if let (Ok(ship_pose), Some(section)) = (
        q_ships.get(ship),
        sections.get_section(&placement.prototype),
    ) {
        let half = section
            .base
            .collider
            .unwrap_or_default()
            .aabb_half_extents();
        let colour = match placement.solve.refusal {
            None => theme::PHOSPHOR,
            Some(_) => theme::RED,
        };
        // Gizmos are drawn in WORLD space while the solve is ship-local.
        let pose = ship_pose
            .mul_transform(placement.solve.transform)
            .compute_transform();
        gizmos.cube(pose.with_scale(half * 2.0), colour);
    }
}

/// The diagnostic id of one socket on a section, for the readout.
fn socket_id(config: Option<&SectionConfig>, index: usize) -> String {
    config
        .and_then(|config| config.base.link_points.get(index))
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

/// Place, delete, or SELECT - depending on the armed tool and on which ship
/// was clicked.
///
/// Placement itself commits [`PlacementPreview`] - the pose the ghost is
/// already showing - rather than re-deriving anything from this click.
///
/// Clicks land on a [`NodeView`], never on a node, so the first thing this does
/// is find the document entity behind the collider that was hit.
#[expect(
    clippy::too_many_arguments,
    reason = "one click routes to select, place or delete"
)]
pub(crate) fn on_click_spaceship_section(
    click: On<Pointer<Press>>,
    mut commands: Commands,
    context: Res<EditContext>,
    selection: Res<SectionChoice>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    sections: Res<GameSections>,
    preview: Res<PlacementPreview>,
    mut ordinals: Query<&mut NextChildOrdinal>,
    rebind: Res<EditorRebind>,
    mut selected: ResMut<SelectedNode>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_nodes: Query<(&SectionNode, &ChildOf)>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Some(node) = node_of_view(click.entity, &q_views) else {
        return;
    };
    let Ok((_, owner)) = q_nodes.get(node) else {
        return;
    };

    // While a rebind is pending, the next click is the user PICKING a
    // mouse-button binding (e.g. LMB), so it must not also move the selection
    // out from under the chip that is prompting.
    if rebind.target.is_some() {
        return;
    }

    // A section of a ship you are NOT inside selects the SHIP: the world and
    // the tree answer a click the same way, and the tree is the door - the
    // solver only ever scans the edited ship, so such a click could never have
    // been a placement anyway.
    let owner = owner.parent();
    if context.ship() != Some(owner) {
        selected.0 = Some(owner);
        return;
    }

    match *selection {
        SectionChoice::None => {
            // No tool in hand = select mode: the clicked section takes the
            // mark, exactly as its tree row would. Rebinding is the top bar's
            // Rebind action on this selection.
            selected.0 = Some(node);
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
            let Some(config) = required_section(&sections, &placement.prototype) else {
                return;
            };

            let binds = default_binds_for(&config.kind, keyboard.as_deref(), gamepad.as_deref());
            spawn_section_node(
                &mut commands,
                &mut ordinals,
                owner,
                config,
                placement.solve.transform,
                binds,
            );
        }
        SectionChoice::Delete => {
            // The node goes and takes its view with it. Its binds are part of
            // the node, so there is no second map left holding a key bound to a
            // section that no longer exists.
            commands.entity(node).despawn();
        }
    }
}

/// Put the armed tool down on the way OUT of a ship.
///
/// Placing and deleting are ship-context verbs - the solver only ever scans
/// the edited ship, and the Parts and Delete buttons live in the ship's action
/// group. Out at the scenario node the pointer selects and drags, and a part
/// silently still in hand would refuse both while showing no tool anywhere on
/// screen.
pub(crate) fn disarm_outside_ship(context: Res<EditContext>, mut choice: ResMut<SectionChoice>) {
    if context.ship().is_none() && *choice != SectionChoice::None {
        *choice = SectionChoice::None;
    }
}

/// The ship being dragged across the stage, and how it was grabbed.
///
/// A resource rather than drag-event state because the grab OFFSET has to
/// survive from `DragStart` to every `Drag`: without it the ship would jump to
/// put its origin under the pointer the moment the drag began.
#[derive(Resource, Default, Debug)]
pub(crate) struct ShipDrag {
    pub(crate) ship: Option<Entity>,
    /// Ship translation minus the grab point on the ground plane.
    pub(crate) offset: Vec3,
}

/// Where `ray` meets the horizontal plane at `height`, or `None` when it runs
/// parallel to it or the plane is behind the camera.
fn ray_to_ground(ray: Ray3d, height: f32) -> Option<Vec3> {
    let direction = ray.direction.as_vec3();
    if direction.y.abs() < 1e-4 {
        return None;
    }
    let t = (height - ray.origin.y) / direction.y;
    (t > 0.0).then(|| ray.origin + direction * t)
}

/// The ground-plane point under a viewport position.
fn pointer_ground_hit(
    camera: &Camera,
    camera_pose: &GlobalTransform,
    viewport: Vec2,
    height: f32,
) -> Option<Vec3> {
    let ray = camera.viewport_to_world(camera_pose, viewport).ok()?;
    ray_to_ground(ray, height)
}

/// Grab a ship at the scenario node: dragging its body slides it on the ground
/// plane. The first transform gesture, and deliberately the only one for now.
///
/// Scenario-node only, and only in select mode: inside a ship the pointer
/// belongs to the build tools, and mating is the rule in there - free-dragging
/// parts would build hulls the runtime's integrity graph rejects.
pub(crate) fn on_ship_drag_start(
    drag: On<Pointer<DragStart>>,
    context: Res<EditContext>,
    selection: Res<SectionChoice>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<crate::gallery::EditorCamera>>>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_sections: Query<&ChildOf, With<SectionNode>>,
    q_ships: Query<&Transform, With<ShipNode>>,
    mut selected: ResMut<SelectedNode>,
    mut state: ResMut<ShipDrag>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if context.ship().is_some() || *selection != SectionChoice::None {
        return;
    }
    let Some(section) = node_of_view(drag.entity, &q_views) else {
        return;
    };
    let Ok(owner) = q_sections.get(section) else {
        return;
    };
    let ship = owner.parent();
    let (Ok(transform), Some(camera)) = (q_ships.get(ship), camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(grab) = pointer_ground_hit(
        camera,
        camera_pose,
        drag.pointer_location.position,
        transform.translation.y,
    ) else {
        return;
    };
    state.ship = Some(ship);
    state.offset = transform.translation - grab;
    // Grabbing is also pointing at: the tree marks the ship being moved.
    selected.0 = Some(ship);
}

/// Slide the grabbed ship to keep its grab point under the pointer.
///
/// The plane height is the ship's own, so the drag never changes altitude:
/// position on the stage is a layout choice, the Y axis is not.
pub(crate) fn on_ship_drag(
    drag: On<Pointer<Drag>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<crate::gallery::EditorCamera>>>,
    state: Res<ShipDrag>,
    mut q_ships: Query<&mut Transform, With<ShipNode>>,
) {
    let Some(ship) = state.ship else {
        return;
    };
    let (Ok(mut transform), Some(camera)) = (q_ships.get_mut(ship), camera) else {
        return;
    };
    let (camera, camera_pose) = *camera;
    let Some(hit) = pointer_ground_hit(
        camera,
        camera_pose,
        drag.pointer_location.position,
        transform.translation.y,
    ) else {
        return;
    };
    let wanted = hit + state.offset;
    if transform.translation != wanted {
        transform.translation = wanted;
    }
}

/// Let go.
pub(crate) fn on_ship_drag_end(_drag: On<Pointer<DragEnd>>, mut state: ResMut<ShipDrag>) {
    if state.ship.is_some() {
        state.ship = None;
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
    context: Res<EditContext>,
    nodes: SectionNodes,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    mut gizmos: Gizmos,
) {
    let (SectionChoice::Section(_), Some(edited)) = (&*selection, context.ship()) else {
        return;
    };
    // Sockets are solved in ship-local space and drawn in world space.
    let Ok(ship_pose) = q_ships.get(edited) else {
        return;
    };

    let mut entities = Vec::new();
    let mut ship = Vec::new();
    for (entity, _, section, transform) in sections_of(edited, &nodes) {
        let Some(config) = section.resolve(Some(&sections)) else {
            continue;
        };
        entities.push(entity);
        ship.push(PlacedSectionLinkPoints {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: config.base.link_points.as_slice(),
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
            let position =
                ship_pose.transform_point(section.position + section.rotation * point.position);
            let normal =
                (ship_pose.rotation() * section.rotation * point.normal).normalize_or(Vec3::Z);
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
        ship_pose.transform_point(transform.translation + transform.rotation * source.position),
        (ship_pose.rotation() * transform.rotation * source.normal).normalize_or(Vec3::Z),
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
    context: Res<EditContext>,
    sections: Res<GameSections>,
    nodes: SectionNodes,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    mut gizmos: Gizmos,
) {
    let Some(edited) = context.ship() else {
        return;
    };
    let Ok(ship_pose) = q_ships.get(edited) else {
        return;
    };
    // Reach past the nose of the ship, so the arrow is never buried inside it.
    let reach = sections_of(edited, &nodes)
        .iter()
        .filter_map(|(_, _, section, transform)| {
            let config = section.resolve(Some(&sections))?;
            let half = config.base.collider.unwrap_or_default().aabb_half_extents();
            Some((transform.translation.z - half.z).abs())
        })
        .fold(1.0f32, f32::max)
        + 1.0;
    let pose = ship_pose.compute_transform();
    let origin = pose.translation;
    let nose = origin + pose.rotation * (Vec3::NEG_Z * reach);
    gizmos.arrow(origin, nose, theme::AMBER_NOVA);
}

/// Outline the section a delete click would remove.
pub(crate) fn draw_delete_target(
    q_pointer: Query<&PointerInteraction>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_nodes: Query<(&SectionNode, &GlobalTransform)>,
    sections: Res<GameSections>,
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
    let Some((section, pose)) = node_of_view(*entity, &q_views)
        .and_then(|node| q_nodes.get(node).ok())
        .map(|(section, pose)| (section, pose.compute_transform()))
    else {
        return;
    };
    let Some(config) = section.resolve(Some(&sections)) else {
        return;
    };
    gizmos.cube(
        pose.with_scale(config.base.collider.unwrap_or_default().aabb_half_extents() * 2.0),
        theme::RED,
    );
}
#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::node::{ensure_document, NodeId, ScenarioNode};

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

    /// An app with a document and the "Add Ship" observer armed.
    fn document_app(catalog: Vec<SectionConfig>) -> App {
        let mut app = App::new();
        app.insert_resource(GameSections(catalog));
        app.init_resource::<EditContext>();
        app.add_observer(create_blank_ship);
        app.world_mut()
            .run_system_once(ensure_document)
            .expect("the document is created");
        app
    }

    fn press_new_ship(app: &mut App) {
        let button = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(Activate { entity: button });
        app.update();
    }

    fn ship_nodes(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<ShipNode>>()
            .iter(app.world())
            .collect()
    }

    fn section_nodes(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<SectionNode>>()
            .iter(app.world())
            .collect()
    }

    /// The document is one scenario node, created once, and the editor opens
    /// OUTSIDE any ship - the "Add Ship" action still owns creation, exactly as
    /// the buttons did when an empty build state rebuilt nothing.
    #[test]
    fn a_fresh_document_is_one_empty_scenario_node() {
        let mut app = document_app(vec![]);

        assert_eq!(
            app.world_mut()
                .query::<&ScenarioNode>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(ship_nodes(&mut app).is_empty());
        let context = app.world().resource::<EditContext>();
        assert!(context.scenario().is_some());
        assert_eq!(context.ship(), None);

        // Idempotent: a second entry into the editor finds the document it left.
        let scenario = context.scenario();
        app.world_mut()
            .run_system_once(ensure_document)
            .expect("the document check runs");
        assert_eq!(app.world().resource::<EditContext>().scenario(), scenario);
    }

    /// Add Ship starts BLANK and entered: which part a ship begins from is the
    /// builder's first decision, not the button's.
    #[test]
    fn add_ship_starts_blank_and_entered() {
        let mut app = document_app(vec![]);

        press_new_ship(&mut app);

        let ships = ship_nodes(&mut app);
        assert_eq!(ships.len(), 1);
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            Some(ships[0]),
            "the new ship is the one the founding click builds"
        );
        assert_eq!(
            app.world()
                .get::<ShipNode>(ships[0])
                .map(|ship| ship.driver),
            Some(ShipDriver::Player),
            "the first ship of a document is the one the player flies"
        );
        assert!(section_nodes(&mut app).is_empty(), "and it starts empty");
    }

    /// Two ships in one session, which is what the whole model exists for. A
    /// second "Add Ship" once despawned the first and reset the build state.
    #[test]
    fn a_second_new_ship_leaves_the_first_standing() {
        let mut app = document_app(vec![]);

        press_new_ship(&mut app);
        let first = ship_nodes(&mut app);
        assert_eq!(first.len(), 1);

        press_new_ship(&mut app);
        let ships = ship_nodes(&mut app);
        assert_eq!(ships.len(), 2, "the first ship is still there");

        let second = *ships
            .iter()
            .find(|ship| **ship != first[0])
            .expect("a second ship");
        assert_eq!(
            app.world().resource::<EditContext>().ship(),
            Some(second),
            "the new ship is entered"
        );
        assert_eq!(
            app.world().get::<ShipNode>(second).map(|ship| ship.driver),
            Some(ShipDriver::Ai),
            "a ship built beside the player's is not a second thing to fly"
        );
    }

    /// A world inside an empty edited ship, with a part armed and the left
    /// button just pressed - the founding gesture, minus the pointer.
    fn founding_world(armed: Option<&str>) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(GameSections(vec![hull_config("hull")]));
        let ship = world
            .spawn((ShipNode::default(), NextChildOrdinal::default()))
            .id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        world.insert_resource(match armed {
            Some(id) => SectionChoice::Section(id.to_string()),
            None => SectionChoice::None,
        });
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        world.insert_resource(mouse);
        (world, ship)
    }

    /// The founding click: a part armed over an EMPTY ship lands at the ship's
    /// own origin, with a minted id - the pose that means the same thing in
    /// every saved file.
    #[test]
    fn a_click_founds_an_empty_ship_at_its_origin() {
        let (mut world, ship) = founding_world(Some("hull"));

        world.run_system_once(found_empty_ship).unwrap();

        let mut sections = world.query::<(&NodeId, &Transform, &ChildOf, &SectionNode)>();
        let placed: Vec<_> = sections.iter(&world).collect();
        assert_eq!(placed.len(), 1, "the founding click placed the part");
        let (id, transform, owner, _) = placed[0];
        assert_eq!(id, &NodeId("hull_1".to_string()));
        assert_eq!(transform.translation, Vec3::ZERO, "at the ship origin");
        assert_eq!(owner.parent(), ship);

        // Once founded, the ship is no longer empty: a FRESH click is the mate
        // solver's business, not this system's.
        {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.clear();
            mouse.press(MouseButton::Left);
        }
        world.run_system_once(found_empty_ship).unwrap();
        assert_eq!(
            world.query::<&SectionNode>().iter(&world).count(),
            1,
            "a founded ship is never founded twice"
        );
    }

    /// A placement tool is a ship-context verb: it survives being inside the
    /// ship and is put down on the way out to the scenario node.
    #[test]
    fn leaving_the_ship_puts_the_tool_down() {
        let mut world = World::new();
        world.insert_resource(SectionChoice::Section("hull".to_string()));
        let ship = world.spawn_empty().id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });

        world.run_system_once(disarm_outside_ship).unwrap();
        assert_eq!(
            *world.resource::<SectionChoice>(),
            SectionChoice::Section("hull".to_string()),
            "inside a ship the tool stays in hand"
        );

        world.resource_mut::<EditContext>().exit();
        world.run_system_once(disarm_outside_ship).unwrap();
        assert_eq!(*world.resource::<SectionChoice>(), SectionChoice::None);
    }

    /// The drag plane maths: a ray meets the ground where it says, and a ray
    /// that cannot reach the plane moves nothing rather than teleporting the
    /// ship to a projected infinity.
    #[test]
    fn a_drag_ray_lands_on_the_ground_plane_or_nowhere() {
        let down = Ray3d::new(Vec3::new(2.0, 10.0, 3.0), Dir3::NEG_Y);
        assert_eq!(ray_to_ground(down, 0.0), Some(Vec3::new(2.0, 0.0, 3.0)));

        let slanted = Ray3d::new(
            Vec3::new(0.0, 4.0, 0.0),
            Dir3::new(Vec3::new(1.0, -1.0, 0.0)).expect("a unit direction"),
        );
        let landed = ray_to_ground(slanted, 0.0).expect("a slanted ray lands");
        assert!(
            landed.abs_diff_eq(Vec3::new(4.0, 0.0, 0.0), 1e-5),
            "landed at {landed:?}"
        );

        let level = Ray3d::new(Vec3::new(0.0, 4.0, 0.0), Dir3::X);
        assert_eq!(ray_to_ground(level, 0.0), None, "parallel never lands");

        let behind = Ray3d::new(Vec3::new(0.0, 4.0, 0.0), Dir3::Y);
        assert_eq!(
            ray_to_ground(behind, 0.0),
            None,
            "a plane behind the camera is not a place to drag to"
        );
    }

    /// With no part armed the click is a select, and founding stays out of it.
    #[test]
    fn founding_needs_a_part_in_hand() {
        let (mut world, _) = founding_world(None);

        world.run_system_once(found_empty_ship).unwrap();

        assert_eq!(world.query::<&SectionNode>().iter(&world).count(), 0);
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
}
