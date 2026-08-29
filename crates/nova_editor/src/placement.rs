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
use nova_input::prelude::InputSource;
use nova_ship::prelude::*;
use nova_ui::theme;

use crate::{
    config::{
        EditorGizmos, EditorOverlays, EditorSays, EditorStatus, LastClick, Placement,
        PlacementPose, PlacementPreview, SectionChoice, SectionGhost, SectionPreviewMarker,
        SelectedNode,
    },
    event::{ActionNode, EventNode, FilterNode, GateNode, StepNode},
    frame::{ask_for, FrameRequest},
    keybind::EditorRebind,
    node::{
        node_of_view, sections_of, spawn_object_node, spawn_section_node, spawn_ship_node,
        EditContext, NextChildOrdinal, NodeId, NodeView, ObjectChoice, ObjectNode, SectionNode,
        SectionNodes, ShipDriver, ShipNode, MINTED_SHIP_STEM,
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

/// A socket ring's radius as a fraction of its distance from the eye.
///
/// An ANGLE rather than a length: a world-space ring shrank to nothing the
/// moment the camera pulled back far enough to see the whole ship, which is
/// exactly when a builder is looking for somewhere to put a part.
const SOCKET_SCREEN_SIZE: f32 = 0.018;
/// The smallest and largest a ring is allowed to get in world units, so a
/// socket under the nose does not swallow the ship and one across the range is
/// still a mark rather than a pixel.
const SOCKET_RADIUS_RANGE: (f32, f32) = (0.05, 0.5);
/// The stub that shows which way a socket faces, as a multiple of the ring.
const SOCKET_NORMAL_LENGTH: f32 = 4.0;
/// How much wider the halo around the aimed socket is drawn.
const SOCKET_HALO: f32 = 1.8;
/// How far up the view ray a ring floats, as a multiple of its radius, to clear
/// the hull it sits on.
const SOCKET_LIFT: f32 = 2.0;

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
/// they clicked, else the kind's default. A missing keyboard resource
/// (headless) contributes no desk binding at all.
///
/// The pad half is ALWAYS written. It used to be gated on an
/// `Option<Res<ButtonInput<GamepadButton>>>`, which bevy 0.19 does not register
/// at all - so the gate was never open, and every ship built in the editor
/// saved an `input_mapping` with no gamepad source. A creator's published ship
/// could not be thrust or fired on a controller.
fn placement_binds(
    keyboard: Option<&ButtonInput<KeyCode>>,
    pad_held: Option<GamepadButton>,
    default_key: InputSource,
    default_pad: InputSource,
) -> Vec<InputSource> {
    let mut binds = Vec::new();
    if let Some(keyboard) = keyboard {
        binds.push(
            capture_binding(keyboard, &EDITOR_CAMERA_KEYS).map_or(default_key, InputSource::from),
        );
    }
    binds.push(pad_held.map_or(default_pad, InputSource::from));
    binds
}

/// The lowest pad button held across every connected controller, if any.
///
/// Bevy 0.19 keeps digital button state on the [`Gamepad`] COMPONENT, so this
/// is a query rather than a resource read.
fn pad_held(gamepads: &Query<&Gamepad>) -> Option<GamepadButton> {
    gamepads
        .iter()
        .flat_map(|pad| pad.digital().get_just_pressed().copied())
        .min()
}

/// The bindings a section of this kind takes when placed. Hull and controller
/// sections are not bindable and take none.
fn default_binds_for(
    kind: &SectionKind,
    keyboard: Option<&ButtonInput<KeyCode>>,
    pad_held: Option<GamepadButton>,
) -> Vec<InputSource> {
    match kind {
        SectionKind::Hull(_) | SectionKind::Controller(_) => vec![],
        SectionKind::Thruster(_) => placement_binds(
            keyboard,
            pad_held,
            KeyCode::Space.into(),
            GamepadButton::RightTrigger.into(),
        ),
        SectionKind::Turret(_) | SectionKind::Torpedo(_) => placement_binds(
            keyboard,
            pad_held,
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
    q_ships: Query<(&NodeId, &ShipNode)>,
    mut context: ResMut<EditContext>,
    mut says: EditorSays,
) {
    // MINTED ships only, for the slot: the stock range's hulks and pickets are
    // ship nodes too, and they stand where the scenario put them rather than in
    // this row - counting them would push the first blank ship a belt away.
    let ships = q_ships
        .iter()
        .filter(|(id, _)| id.0.starts_with(MINTED_SHIP_STEM))
        .count();
    // A document with no ship the player flies gets one; anything built beside
    // it is scenery until something says otherwise. A second Player ship would
    // make "which one do I fly" ambiguous, and the answer belongs to the ship
    // rather than to the order the buttons were pressed.
    let flown = q_ships
        .iter()
        .any(|(_, ship)| ship.driver == ShipDriver::Player);
    let driver = if flown {
        ShipDriver::Ai
    } else {
        ShipDriver::Player
    };
    if spawn_ship_node(&mut commands, &mut ordinals, &mut context, ships, driver).is_none() {
        says.refuse("there is no scenario to add a ship to");
    }
}

/// How far out in front of the stage camera a freshly placed object lands.
///
/// In front of the CAMERA rather than at a spaced-out slot, unlike a new ship:
/// a ship is a workbench the editor then flies you to, an object is scenery you
/// are pointing at a gap for. It arrives where you are looking, and the drag
/// gesture moves it from there.
const NEW_OBJECT_DISTANCE: f32 = 30.0;

/// Place one scenario object of the clicked kind - the scenario context's
/// object palette.
///
/// The world is not only ships: a rock, a beacon, a crate, an anchor or a light
/// is one more node under the scenario, placed and dragged the same way a ship
/// is. Selected on arrival, so the tree marks the thing that just appeared
/// rather than leaving the builder to find it.
pub(crate) fn create_scenario_object(
    activate: On<Activate>,
    mut commands: Commands,
    choices: Query<&ObjectChoice>,
    camera: Query<&GlobalTransform, With<crate::gallery::EditorCamera>>,
    mut ordinals: Query<&mut NextChildOrdinal>,
    context: Res<EditContext>,
    mut selected: ResMut<SelectedNode>,
    mut says: EditorSays,
) {
    let Ok(choice) = choices.get(activate.entity) else {
        return;
    };
    let Some(scenario) = context.scenario() else {
        says.refuse("there is no scenario to add an object to");
        return;
    };
    let at = camera.iter().next().map_or(Vec3::ZERO, in_front_of);
    let object = spawn_object_node(
        &mut commands,
        &mut ordinals,
        scenario,
        *choice,
        Transform::from_translation(at),
    );
    selected.0 = Some(object);
}

/// The point a placed object lands on, out in front of `camera`.
fn in_front_of(camera: &GlobalTransform) -> Vec3 {
    camera.translation() + camera.forward() * NEW_OBJECT_DISTANCE
}

/// The key that deletes the selection.
const DELETE_KEY: KeyCode = KeyCode::Delete;

/// Every node kind the editor can remove: the world's three, and the five the
/// script is made of.
///
/// One alias rather than the same filter written out at each of the places
/// that ask - the menu row, the row trash, the key and the verb they share -
/// because a kind missing from ONE of them is a row whose trash does nothing.
pub(crate) type DeletableNode = Or<(
    With<ShipNode>,
    With<ObjectNode>,
    With<SectionNode>,
    With<EventNode>,
    With<FilterNode>,
    With<ActionNode>,
    With<StepNode>,
    With<GateNode>,
)>;

/// Can `node` be deleted?
///
/// Everything a context LISTS can go - a ship, an object, a part - except the
/// containers the editor is currently standing inside: deleting one of those
/// would leave the context pointing into rubble, the same reason Play refuses
/// inside a ship.
pub(crate) fn deletable(
    node: Entity,
    context: &EditContext,
    nodes: &Query<(), DeletableNode>,
) -> bool {
    nodes.contains(node) && !context.path.contains(&node)
}

/// Delete the SELECTED node, at whatever depth the selection is.
///
/// ONE gesture with one name: what is marked is what goes, at a part and at a
/// scenario node alike.
pub(crate) fn delete_selected_node(
    _activate: On<Activate>,
    commands: Commands,
    selected: ResMut<SelectedNode>,
    context: Res<EditContext>,
    nodes: Query<(), DeletableNode>,
) {
    delete_selection(commands, selected, &context, &nodes);
}

/// The same verb from the keyboard.
///
/// Del is what every editor binds this to, and until now the editor answered
/// no key at all: Edit > Delete was the only way to remove anything.
pub(crate) fn delete_key(
    keys: Res<ButtonInput<KeyCode>>,
    commands: Commands,
    selected: ResMut<SelectedNode>,
    context: Res<EditContext>,
    nodes: Query<(), DeletableNode>,
) {
    if !keys.just_pressed(DELETE_KEY) {
        return;
    }
    delete_selection(commands, selected, &context, &nodes);
}

/// Despawn the marked node and clear the mark.
///
/// The node goes and takes its view with it. Its binds are part of the node,
/// so there is no second map left holding a key bound to a section that no
/// longer exists.
fn delete_selection(
    mut commands: Commands,
    mut selected: ResMut<SelectedNode>,
    context: &EditContext,
    nodes: &Query<(), DeletableNode>,
) {
    let Some(node) = selected.0 else {
        return;
    };
    if !deletable(node, context, nodes) {
        return;
    }
    commands.entity(node).despawn();
    selected.0 = None;
}

/// Ship > Roll the Part: one step of the same roll the R key and the wheel take.
///
/// A menu row for a pose verb rather than only a key, because R, F and the
/// wheel were named in one legend a builder can switch off. Inert with nothing
/// in hand, which is also when [`crate::ui::menu::sync_armed_menu`] greys it.
pub(crate) fn roll_armed_part(
    _activate: On<Activate>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    mut pose: ResMut<PlacementPose>,
) {
    if armed_socket_count(&selection, &sections).is_none() {
        return;
    }
    pose.roll = step_cycle(pose.roll as usize, 4, false) as u32;
}

/// Ship > Cycle the Socket: one step of what the F key and Ctrl+wheel take.
pub(crate) fn cycle_armed_socket(
    _activate: On<Activate>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    mut pose: ResMut<PlacementPose>,
) {
    let Some(sockets) = armed_socket_count(&selection, &sections) else {
        return;
    };
    if sockets > 0 {
        pose.source = step_cycle(pose.source % sockets, sockets, false);
    }
}

/// Ship > Put the Part Down: the same rung Escape takes with a part in hand.
pub(crate) fn put_armed_part_down(_activate: On<Activate>, mut choice: ResMut<SectionChoice>) {
    *choice = SectionChoice::None;
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
    gamepads: Query<&Gamepad>,
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
    let binds = default_binds_for(&config.kind, keyboard.as_deref(), pad_held(&gamepads));
    spawn_section_node(
        &mut commands,
        &mut ordinals,
        ship,
        config,
        Transform::default(),
        binds,
    );
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
    mut says: EditorSays,
) {
    if context.ship().is_some() {
        says.refuse("Play flies the whole scenario - leave the ship first");
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
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
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
    // The solver works in SHIP-LOCAL space while the picking hit arrives in
    // WORLD space. The first ship sits at the origin, where the two spaces
    // coincide - which is how a ship standing anywhere else ended up mating
    // every part onto whichever socket happened to sit nearest a point 24
    // units away.
    let hit = ship_local_hit(q_ships.get(edited).ok(), hit);

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
    mut gizmos: Gizmos<EditorGizmos>,
    ghosts: Query<(Entity, &SectionGhost, &mut Transform)>,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    mut status: ResMut<EditorStatus>,
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
        status.report(founding.then(|| {
            (
                "click empty space - the first part founds the ship".to_string(),
                theme::PHOSPHOR_MUTED,
            )
        }));
        return;
    };
    // The verdict itself is not written here: it is said beside the
    // ghost by `crate::ui::callout`, which is where a builder watching a part
    // snap around a hull is looking.
    status.report(None);

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

/// A world-space picking hit in the edited ship's own space - the space the
/// solver, the ghost and the section node it would become are all posed in.
/// A ship with no `GlobalTransform` yet (a headless rig's first frame) reads
/// as standing at the origin, where the two spaces coincide.
fn ship_local_hit(ship_pose: Option<&GlobalTransform>, hit: Vec3) -> Vec3 {
    ship_pose.map_or(hit, |pose| pose.affine().inverse().transform_point3(hit))
}

/// Place, delete, or SELECT - depending on the armed tool and on what was
/// clicked.
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
    gamepads: Query<&Gamepad>,
    sections: Res<GameSections>,
    preview: Res<PlacementPreview>,
    mut ordinals: Query<&mut NextChildOrdinal>,
    rebind: Res<EditorRebind>,
    time: Res<Time<Real>>,
    mut last: ResMut<LastClick>,
    mut request: ResMut<FrameRequest>,
    mut selected: ResMut<SelectedNode>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_objects: Query<(), With<ObjectNode>>,
    q_nodes: Query<(&SectionNode, &ChildOf)>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Some(node) = node_of_view(click.entity, &q_views) else {
        return;
    };

    // While a rebind is pending, the next click is the user PICKING a
    // mouse-button binding (e.g. LMB), so it must not also move the selection
    // out from under the chip that is prompting.
    if rebind.target.is_some() {
        return;
    }

    // An object has nothing to build on and nothing to enter: clicking one
    // SELECTS it, which is the same answer its tree row gives. A SECOND click
    // frames it - the gesture a ship spends on entering, spent on the one
    // thing a rock can do with it. Without this a double-click out here was
    // two selections of what was already selected.
    if q_objects.contains(node) {
        selected.0 = Some(node);
        if last.press(node, time.elapsed_secs()) {
            ask_for(&mut request, Some(node));
        }
        return;
    }

    let Ok((_, owner)) = q_nodes.get(node) else {
        return;
    };

    // A section of a ship you are NOT inside selects the SHIP: the world and
    // the tree answer a click the same way, and the tree is the door.
    //
    // Entering stays the TREE's gesture, and deliberately: out here a second
    // press on the same ship is far more often the start of a drag than a
    // request to go inside it, and a press cannot yet know which. See
    // `crate::ui::on_scene_row`.
    //
    // The solver only ever scans the edited ship, so such a click could never
    // have been a placement anyway.
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

            let binds = default_binds_for(&config.kind, keyboard.as_deref(), pad_held(&gamepads));
            spawn_section_node(
                &mut commands,
                &mut ordinals,
                owner,
                config,
                placement.solve.transform,
                binds,
            );
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
pub(crate) fn disarm_outside_ship(
    context: Res<EditContext>,
    mut choice: ResMut<SectionChoice>,
    mut says: EditorSays,
) {
    if context.ship().is_none() && *choice != SectionChoice::None {
        *choice = SectionChoice::None;
        says.note("the part went back on the rack - parts belong to a ship");
    }
}

/// The node being dragged across the stage, and how it was grabbed.
///
/// A resource rather than drag-event state because the grab OFFSET has to
/// survive from `DragStart` to every `Drag`: without it the node would jump to
/// put its origin under the pointer the moment the drag began.
#[derive(Resource, Default, Debug)]
pub(crate) struct StageDrag {
    /// The ship or object node under the grab.
    pub(crate) node: Option<Entity>,
    /// Node translation minus the grab point on the ground plane.
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

/// The staged node a picking hit belongs to: a section's view answers for its
/// SHIP, and an object's view answers for itself.
///
/// One rule, because the scenario node holds one kind of thing - something you
/// can move - however it is built underneath.
fn staged_node_of_view(
    view: Entity,
    q_views: &Query<&ChildOf, With<NodeView>>,
    q_sections: &Query<&ChildOf, With<SectionNode>>,
) -> Option<Entity> {
    let node = node_of_view(view, q_views)?;
    Some(
        q_sections
            .get(node)
            .map_or(node, |section_owner| section_owner.parent()),
    )
}

/// Grab a node at the scenario node: dragging its body slides it on the ground
/// plane. The first transform gesture, and deliberately the only one for now.
///
/// Scenario-node only, and only in select mode: inside a ship the pointer
/// belongs to the build tools, and mating is the rule in there - free-dragging
/// parts would build hulls the runtime's integrity graph rejects.
pub(crate) fn on_stage_drag_start(
    drag: On<Pointer<DragStart>>,
    context: Res<EditContext>,
    selection: Res<SectionChoice>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<crate::gallery::EditorCamera>>>,
    q_views: Query<&ChildOf, With<NodeView>>,
    q_sections: Query<&ChildOf, With<SectionNode>>,
    q_staged: Query<&Transform, Or<(With<ShipNode>, With<ObjectNode>)>>,
    mut selected: ResMut<SelectedNode>,
    mut state: ResMut<StageDrag>,
    mut says: EditorSays,
) {
    if drag.button != PointerButton::Primary || *selection != SectionChoice::None {
        return;
    }
    let Some(node) = staged_node_of_view(drag.entity, &q_views, &q_sections) else {
        return;
    };
    // Inside a ship the handles are gone and the drag does nothing, and until
    // now neither said why. A part's pose is its socket's; dragging one off the
    // mate would build a hull the runtime's integrity graph rejects.
    if context.ship().is_some() {
        says.refuse("a part moves by being placed again, not by being dragged");
        return;
    }
    let (Ok(transform), Some(camera)) = (q_staged.get(node), camera) else {
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
    state.node = Some(node);
    state.offset = transform.translation - grab;
    // Grabbing is also pointing at: the tree marks the node being moved.
    selected.0 = Some(node);
}

/// Slide the grabbed node to keep its grab point under the pointer.
///
/// The plane height is the node's own, so the drag never changes altitude:
/// position on the stage is a layout choice, the Y axis is not.
pub(crate) fn on_stage_drag(
    drag: On<Pointer<Drag>>,
    camera: Option<Single<(&Camera, &GlobalTransform), With<crate::gallery::EditorCamera>>>,
    state: Res<StageDrag>,
    mut q_staged: Query<&mut Transform, Or<(With<ShipNode>, With<ObjectNode>)>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Some(node) = state.node else {
        return;
    };
    let (Ok(mut transform), Some(camera)) = (q_staged.get_mut(node), camera) else {
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

/// Let go - of the button that grabbed. Any-button matching here let an RMB
/// camera-orbit release drop an LMB drag mid-gesture.
pub(crate) fn on_stage_drag_end(drag: On<Pointer<DragEnd>>, mut state: ResMut<StageDrag>) {
    if drag.button != PointerButton::Primary {
        return;
    }
    if state.node.is_some() {
        state.node = None;
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
///
/// View > Link Points turns them off, for a builder who knows where the sockets
/// are and wants to look at the ship instead.
pub(crate) fn draw_link_points(
    preview: Res<PlacementPreview>,
    overlays: Res<EditorOverlays>,
    selection: Res<SectionChoice>,
    sections: Res<GameSections>,
    context: Res<EditContext>,
    nodes: SectionNodes,
    q_ships: Query<&GlobalTransform, With<ShipNode>>,
    // The eye, because a socket is sized by how far away it is.
    cameras: Query<&GlobalTransform, With<crate::gallery::EditorCamera>>,
    mut gizmos: Gizmos<EditorGizmos>,
) {
    if !overlays.link_points {
        return;
    }
    let (SectionChoice::Section(armed), Some(edited)) = (&*selection, context.ship()) else {
        return;
    };
    // Sockets are solved in ship-local space and drawn in world space.
    let Ok(ship_pose) = q_ships.get(edited) else {
        return;
    };
    let Some(eye) = cameras.iter().next().map(GlobalTransform::translation) else {
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

    // An empty ship has no socket to draw, so the FOUNDING pose gets the
    // marker instead: a pad at the ship origin, with the armed part's bounds
    // over it - where the founding click will put it. Without this the first
    // part landed somewhere the builder was never shown.
    if ship.is_empty() {
        let origin = ship_pose.translation();
        draw_socket(
            &mut gizmos,
            eye,
            origin,
            (ship_pose.rotation() * Vec3::Y).normalize_or(Vec3::Y),
            theme::PHOSPHOR,
            true,
        );
        if let Some(part) = sections.get_section(armed) {
            let half = part.base.collider.unwrap_or_default().aabb_half_extents();
            gizmos.cube(
                ship_pose.compute_transform().with_scale(half * 2.0),
                theme::PHOSPHOR_MUTED,
            );
        }
        return;
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
            // The one the ghost would take is drawn AMBER and haloed, the
            // colour every other "this is the one" mark in the editor uses;
            // the rest are full phosphor, which a grey hull can carry. Muted
            // green ticks on grey plating read as scratches.
            let taken_next = aimed == Some((section_index, point_index));
            let colour = if taken_next {
                theme::AMBER_NOVA
            } else {
                theme::PHOSPHOR
            };
            draw_socket(&mut gizmos, eye, position, normal, colour, taken_next);
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
        eye,
        ship_pose.transform_point(transform.translation + transform.rotation * source.position),
        (ship_pose.rotation() * transform.rotation * source.normal).normalize_or(Vec3::Z),
        match placement.solve.refusal {
            None => theme::PHOSPHOR,
            Some(_) => theme::RED,
        },
        true,
    );
}

/// One socket marker: a ring facing the eye plus a stub along its normal, so
/// both WHERE it is and which way it faces read at a glance.
///
/// Sized from the EYE, not from the ship: see [`SOCKET_SCREEN_SIZE`]. `halo`
/// draws a second ring around the first - shape, not only colour, for the one
/// socket the ghost is about to take.
pub(crate) fn draw_socket(
    gizmos: &mut Gizmos<EditorGizmos>,
    eye: Vec3,
    position: Vec3,
    normal: Vec3,
    colour: Color,
    halo: bool,
) {
    let (near, far) = SOCKET_RADIUS_RANGE;
    let radius = (eye.distance(position) * SOCKET_SCREEN_SIZE).clamp(near, far);
    // The ring faces the EYE, not the socket's plane: a ring lying on the plane
    // goes edge-on the moment you look along the hull, which is a socket you
    // cannot see. The stub carries the facing instead.
    //
    // It also floats a little way up the view ray, because a ring drawn ON the
    // hull is half buried in it. Sliding towards the eye leaves it over the
    // same pixel and clear of the surface.
    let towards_eye = (eye - position).normalize_or(Vec3::Z);
    let centre = position + towards_eye * radius * SOCKET_LIFT;
    let facing = Isometry3d::new(centre, Quat::from_rotation_arc(Vec3::Z, towards_eye));
    gizmos.circle(facing, radius, colour);
    if halo {
        gizmos.circle(facing, radius * SOCKET_HALO, colour);
    }
    gizmos.line(
        position,
        position + normal * radius * SOCKET_NORMAL_LENGTH,
        colour,
    );
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
    mut gizmos: Gizmos<EditorGizmos>,
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::{ScenarioObjectKind, SectionSource};
    use nova_ui::prelude::{in_input_mode, InputMode};

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

    /// An app with a document and nothing observing `Activate`.
    ///
    /// A test that arms its OWN observer takes this one: a bare `Activate` is
    /// answered by every observer in the app, so a fixture that also carries
    /// "Add Ship" would build a ship and enter it on the same trigger.
    fn document_only(catalog: Vec<SectionConfig>) -> App {
        let mut app = App::new();
        app.insert_resource(GameSections(catalog));
        app.init_resource::<EditContext>();
        // Every verb here can refuse, and a refusal says so on the line.
        app.init_resource::<EditorStatus>();
        app.init_resource::<Time>();
        app.world_mut()
            .run_system_once(ensure_document)
            .expect("the document is created");
        app
    }

    /// The same, with the "Add Ship" observer armed.
    fn document_app(catalog: Vec<SectionConfig>) -> App {
        let mut app = document_only(catalog);
        app.add_observer(create_blank_ship);
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

    /// The ships the BUILDER added. The stock range seeds eight of its own -
    /// five hulks and three pickets - and every one of them is a ship node, so
    /// "how many ships are there" is not the question these tests ask.
    fn minted_ship_nodes(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<(Entity, &NodeId), With<ShipNode>>()
            .iter(app.world())
            .filter(|(_, id)| id.0.starts_with(MINTED_SHIP_STEM))
            .map(|(entity, _)| entity)
            .collect()
    }

    fn object_nodes(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<ObjectNode>>()
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
    /// the buttons did when an empty build state rebuilt nothing. The world it
    /// opens onto is seeded, hulls included; nothing on it is MINTED.
    #[test]
    fn a_fresh_document_is_one_scenario_node_with_no_minted_ships_on_it() {
        let mut app = document_app(vec![]);

        assert_eq!(
            app.world_mut()
                .query::<&ScenarioNode>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(minted_ship_nodes(&mut app).is_empty());
        assert!(
            !ship_nodes(&mut app).is_empty(),
            "the seeded hulks and pickets are ships of the document"
        );
        assert!(
            !object_nodes(&mut app).is_empty(),
            "the editor opens onto the sandbox range, not onto nothing"
        );
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

        let ships = minted_ship_nodes(&mut app);
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
            "a document with no ship the player flies gets one"
        );
        assert!(
            app.world().get::<Children>(ships[0]).is_none(),
            "and it starts empty"
        );
    }

    /// Two ships in one session, which is what the whole model exists for. A
    /// second "Add Ship" once despawned the first and reset the build state.
    #[test]
    fn a_second_new_ship_leaves_the_first_standing() {
        let mut app = document_app(vec![]);

        press_new_ship(&mut app);
        let first = minted_ship_nodes(&mut app);
        assert_eq!(first.len(), 1);

        press_new_ship(&mut app);
        let ships = minted_ship_nodes(&mut app);
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
        world.init_resource::<EditorStatus>();
        world.init_resource::<Time>();
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
        assert!(
            world
                .resource::<EditorStatus>()
                .line()
                .is_some_and(|(line, _)| line.contains("part")),
            "and it says so - an emptied hand used to be silent, so the next \
             click on the hull simply did nothing"
        );
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

    /// The picking hit is mapped INTO the ship's space, not out of it. The
    /// first ship stands at the origin where either direction reads the same,
    /// which is exactly how the second ship - 24 units along +X - ended up
    /// only ever mating one socket: every world hit was nearest the same
    /// ship-local point.
    #[test]
    fn a_picking_hit_lands_in_the_edited_ships_own_space() {
        let pose = GlobalTransform::from(
            Transform::from_xyz(24.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        );
        // A hit on what the WORLD sees at one unit -Z of the ship: the ship is
        // yawed a quarter turn, so its own +X face is what stands there.
        let local = ship_local_hit(Some(&pose), Vec3::new(24.0, 0.0, -1.0));
        assert!(
            local.abs_diff_eq(Vec3::X, 1e-5),
            "the hit must come home through the INVERSE of the ship pose, got {local:?}"
        );

        // No pose yet (a headless rig's first frame) reads as the origin.
        let hit = Vec3::new(0.5, 0.1, 0.0);
        assert_eq!(ship_local_hit(None, hit), hit);
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

    /// The object palette PLACES; it does not arm. One press puts the kind it
    /// names in the document and marks it, so the next press places another
    /// rather than re-interpreting a click somewhere on the stage.
    #[test]
    fn the_object_palette_places_the_kind_it_names_and_marks_it() {
        let mut app = document_app(vec![]);
        app.init_resource::<SelectedNode>();
        app.add_observer(create_scenario_object);
        let before = object_nodes(&mut app).len();

        let button = app.world_mut().spawn(ObjectChoice::Beacon).id();
        app.world_mut().trigger(Activate { entity: button });
        app.update();

        let placed: Vec<(Entity, String, ScenarioObjectKind)> = app
            .world_mut()
            .query::<(Entity, &NodeId, &ObjectNode)>()
            .iter(app.world())
            .map(|(entity, id, object)| (entity, id.0.clone(), object.kind.clone()))
            .filter(|(_, id, _)| {
                id.starts_with("beacon_") && id != "beacon_veil" && id != "beacon_home"
            })
            .collect();
        assert_eq!(
            placed.len(),
            1,
            "one press places one object: {:?}",
            object_nodes(&mut app).len()
        );
        assert_eq!(object_nodes(&mut app).len(), before + 1);
        assert!(matches!(placed[0].2, ScenarioObjectKind::Beacon(_)));
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            Some(placed[0].0),
            "what you just placed is what the tree has marked"
        );
    }

    /// A second press on a world object FRAMES it. There is nothing inside a
    /// rock to enter, so the gesture a ship spends on entering is spent on the
    /// one thing a rock can do with it - and a double-click that did nothing
    /// read as a dead spot on the stage.
    #[test]
    fn a_double_click_on_a_world_object_frames_it() {
        use bevy::{
            camera::NormalizedRenderTarget,
            picking::{
                backend::HitData,
                pointer::{Location, PointerId},
            },
            window::{Window, WindowRef},
        };

        let mut app = document_app(vec![]);
        app.init_resource::<SelectedNode>();
        app.init_resource::<LastClick>();
        app.init_resource::<FrameRequest>();
        app.init_resource::<EditorRebind>();
        app.init_resource::<SectionChoice>();
        app.init_resource::<PlacementPreview>();
        app.insert_resource(Time::<Real>::default());
        app.add_observer(on_click_spaceship_section);

        let rock = object_nodes(&mut app)[0];
        // A hit lands on the VIEW; the document node is its parent.
        let view = app.world_mut().spawn((NodeView, ChildOf(rock))).id();
        let screen = app.world_mut().spawn(Window::default()).id();
        let target = NormalizedRenderTarget::Window(
            WindowRef::Entity(screen)
                .normalize(None)
                .expect("a named window normalizes"),
        );
        let press = |app: &mut App| {
            app.world_mut().trigger(Pointer::new(
                PointerId::Mouse,
                Location {
                    target: target.clone(),
                    position: Vec2::ZERO,
                },
                Press {
                    button: PointerButton::Primary,
                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                    count: 1,
                },
                view,
            ));
            app.update();
        };

        press(&mut app);
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            Some(rock),
            "one press marks the object"
        );
        assert_eq!(
            app.world().resource::<FrameRequest>().node,
            None,
            "and leaves the camera where it is"
        );

        press(&mut app);
        assert_eq!(
            app.world().resource::<FrameRequest>().node,
            Some(rock),
            "the second press asks the camera for it"
        );
    }

    /// Delete takes the SELECTION off the document - a ship or a world object,
    /// the tree makes no distinction - and leaves nothing marked behind it.
    #[test]
    fn delete_removes_the_marked_node_and_clears_the_mark() {
        let mut app = document_only(vec![]);
        app.init_resource::<SelectedNode>();
        app.add_observer(delete_selected_node);
        let rock = object_nodes(&mut app)[0];
        let before = object_nodes(&mut app).len();
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(rock);

        let button = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(object_nodes(&mut app).len(), before - 1);
        assert!(app.world().get_entity(rock).is_err());
        assert_eq!(app.world().resource::<SelectedNode>().0, None);
    }

    /// A drag on a part inside a ship does nothing, and the handles are not
    /// there either. Both are deliberate - a part's pose is its socket's - so
    /// the gesture that asks for it is where the editor answers.
    #[test]
    fn dragging_a_part_inside_a_ship_says_why_it_cannot_move() {
        use bevy::{
            camera::NormalizedRenderTarget,
            picking::{
                backend::HitData,
                pointer::{Location, PointerId},
            },
            window::{Window, WindowRef},
        };

        let mut app = document_only(vec![]);
        app.init_resource::<SelectedNode>();
        app.init_resource::<SectionChoice>();
        app.init_resource::<StageDrag>();
        app.add_observer(on_stage_drag_start);
        let scenario = app
            .world()
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = app
            .world_mut()
            .spawn((ShipNode::default(), Transform::default(), ChildOf(scenario)))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionNode {
                    source: SectionSource::Prototype("hull".to_string()),
                    modifications: Vec::new(),
                    binds: Vec::new(),
                },
                ChildOf(ship),
            ))
            .id();
        let view = app.world_mut().spawn((NodeView, ChildOf(section))).id();
        app.world_mut().resource_mut::<EditContext>().enter(ship);

        let window = app.world_mut().spawn(Window::default()).id();
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(
                    WindowRef::Entity(window)
                        .normalize(None)
                        .expect("a window target"),
                ),
                position: Vec2::ZERO,
            },
            DragStart {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            },
            view,
        ));
        app.update();

        assert_eq!(
            app.world().resource::<StageDrag>().node,
            None,
            "the drag itself is still refused"
        );
        let (line, _) = app
            .world()
            .resource::<EditorStatus>()
            .line()
            .expect("the editor answers the gesture");
        assert!(
            line.contains("placed"),
            "and it names what to do instead; it read {line:?}"
        );
    }

    /// The same verb from the keyboard, at the depth the retired brush owned:
    /// a part inside the ship being edited. Nothing is under the pointer, and
    /// no mode is armed - the MARK is the target.
    #[test]
    fn del_takes_the_marked_section_off_the_ship_being_edited() {
        let mut app = document_only(vec![]);
        app.init_resource::<SelectedNode>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, delete_key);
        let scenario = app
            .world()
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = app
            .world_mut()
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                ChildOf(scenario),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionNode {
                    source: SectionSource::Prototype("hull".to_string()),
                    modifications: Vec::new(),
                    binds: Vec::new(),
                },
                NodeId("part_1".to_string()),
                ChildOf(ship),
            ))
            .id();
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(section);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(DELETE_KEY);
        app.update();

        assert!(
            !section_nodes(&mut app).contains(&section),
            "Del takes the marked part off"
        );
        assert_eq!(app.world().resource::<SelectedNode>().0, None);
        assert!(
            app.world().get_entity(ship).is_ok(),
            "and takes nothing else with it"
        );
    }

    /// The same key under Bind belongs to the CAPTURE.
    ///
    /// Binding Delete to a part deleted the part on the way in: the capture
    /// read the press and so did the tree. Delete is a verb, verbs answer in
    /// Normal, and a rebind waiting for a key is not Normal - so the verb is
    /// not held off by a predicate naming the rebind, it simply is not the
    /// keyboard's owner.
    #[test]
    fn del_does_not_delete_under_bind_mode() {
        let mut app = document_only(vec![]);
        app.init_resource::<SelectedNode>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(InputMode::Bind);
        app.add_systems(Update, delete_key.run_if(in_input_mode(InputMode::Normal)));
        let scenario = app
            .world()
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = app
            .world_mut()
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                ChildOf(scenario),
            ))
            .id();
        let section = app
            .world_mut()
            .spawn((
                SectionNode {
                    source: SectionSource::Prototype("hull".to_string()),
                    modifications: Vec::new(),
                    binds: Vec::new(),
                },
                NodeId("part_1".to_string()),
                ChildOf(ship),
            ))
            .id();
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(section);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(DELETE_KEY);
        app.update();

        assert!(
            section_nodes(&mut app).contains(&section),
            "the part being bound survives the key it is being bound to"
        );
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            Some(section),
            "and it is still the marked node the capture is aimed at"
        );
    }

    /// Delete refuses the node the editor is STANDING IN. Deleting the ship
    /// you are inside leaves the context pointing at a despawned entity, and
    /// every panel keyed on it reads an empty ship instead of a missing one.
    #[test]
    fn delete_refuses_the_node_the_editor_is_inside() {
        let mut app = document_only(vec![]);
        app.init_resource::<SelectedNode>();
        app.add_observer(delete_selected_node);
        let scenario = app
            .world()
            .resource::<EditContext>()
            .scenario()
            .expect("the document exists");
        let ship = app
            .world_mut()
            .spawn((
                ShipNode::default(),
                NodeId("ship_1".to_string()),
                ChildOf(scenario),
            ))
            .id();
        app.world_mut().resource_mut::<EditContext>().enter(ship);
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(ship);

        let button = app.world_mut().spawn_empty().id();
        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert!(
            app.world().get_entity(ship).is_ok(),
            "the ship the editor is inside survives its own Delete"
        );
        assert_eq!(
            app.world().resource::<SelectedNode>().0,
            Some(ship),
            "and stays marked, because nothing happened"
        );
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
            vec![
                InputSource::from(MouseButton::Left),
                InputSource::from(GamepadButton::RightTrigger2)
            ],
            "W drives the camera, so the turret keeps its defaults on both devices"
        );
    }
}
