//! Section keybind labels + the Rebind action. Each bindable section
//! (thruster/turret/torpedo) gets a screen-space chip showing its current key;
//! the top bar's Rebind button arms a capture for the SELECTED section, and
//! `apply_section_rebind` consumes the next key or mouse-button press.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_input::prelude::{source_label, InputSource};
use nova_ship::prelude::*;
use nova_ui::prelude::{clear_of, hang_at, take_keyboard_now, Hang, InputMode};

use crate::{
    config::{EditorSays, SelectedNode},
    gallery::EditorCamera,
    node::{EditContext, SectionNode},
    ExampleStates,
};

/// The section currently awaiting a new keybind. Armed by the top bar's
/// Rebind action on the selected section ([`on_rebind_action`]);
/// `apply_section_rebind` consumes the next key or mouse-button press. Reset
/// to `None` on every state entry.
#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct EditorRebind {
    pub(crate) target: Option<Entity>,
    /// Set true when armed by a mouse click: the capture waits until that click
    /// is released before reading a press, so the arming LMB is not itself
    /// bound. False = ready to capture (e.g. armed in a test).
    pub(crate) awaiting_release: bool,
}

/// A screen-space UI chip showing `section`'s current keybind, positioned each
/// frame over the section NODE by projecting its world position with the editor
/// camera. One per bindable (thruster/turret/torpedo) section.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SectionKeybindLabel {
    section: Entity,
}

/// The chip text of the currently-armed section (see [`EditorRebind`]).
const REBIND_PROMPT: &str = "press key";

/// The line from a chip down to the part it names.
///
/// A chip sat ON its section, hiding the thing it was about and giving no way
/// to tell which of two neighbours it named. It floats above the part now, and
/// this is what still ties the two together.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SectionKeybindLeader {
    section: Entity,
}

/// How far above its part a chip floats, and how long its leader runs.
const LEADER_PX: f32 = 24.0;
/// How far to the right of the leader the chip stands, so the line meets the
/// chip's corner rather than running up through its middle.
const CHIP_OFFSET_PX: f32 = 5.0;
/// The chip stands on its part: its bottom-left corner sits a leader's length
/// above the part and a hair to the right.
const CHIP_HANG: Hang = Hang {
    align: Vec2::new(0.0, 1.0),
    gap: Vec2::new(CHIP_OFFSET_PX, -LEADER_PX),
};
/// How close two chips may come before one is pushed clear of the other.
///
/// Several bound parts a hand's width apart on the hull project to the same
/// few pixels, and a pile of amber pills names nothing.
const CHIP_CLEARANCE_PX: Vec2 = Vec2::new(52.0, 22.0);

/// Every section of the EDITED ship that takes an input binding.
///
/// Read off the DOCUMENT rather than off three optional components on a view:
/// a section's binds belong to its node, and the view they are drawn over is
/// despawned and rebuilt on every visit to the editor.
///
/// Scoped to the edit context for the same reason the solver and the skin
/// are: a ship standing beside the one you are inside is not what these chips
/// label, and only the player-driven ship's binds reach the hand-off at all.
fn bindable_sections(
    ship: Option<Entity>,
    q_sections: &Query<(Entity, &ChildOf, &SectionNode)>,
    sections: Option<&GameSections>,
) -> Vec<Entity> {
    let Some(ship) = ship else {
        return Vec::new();
    };
    q_sections
        .iter()
        .filter(|(_, owner, section)| owner.parent() == ship && section.bindable(sections))
        .map(|(entity, ..)| entity)
        .collect()
}

/// Keep exactly one [`SectionKeybindLabel`] per bindable section: spawn for new
/// ones, despawn labels whose section is gone or is not bindable. Reconcile
/// shape mirrors the ammo readout's `sync_ammo_readouts`.
pub(crate) fn sync_section_keybind_labels(
    mut commands: Commands,
    catalog: Option<Res<GameSections>>,
    context: Res<EditContext>,
    q_sections: Query<(Entity, &ChildOf, &SectionNode)>,
    q_labels: Query<(Entity, &SectionKeybindLabel)>,
    q_leaders: Query<(Entity, &SectionKeybindLeader)>,
) {
    let bindable = bindable_sections(context.ship(), &q_sections, catalog.as_deref());
    for (label, SectionKeybindLabel { section }) in &q_labels {
        if !bindable.contains(section) {
            commands.entity(label).despawn();
        }
    }
    for (leader, SectionKeybindLeader { section }) in &q_leaders {
        if !bindable.contains(section) {
            commands.entity(leader).despawn();
        }
    }
    let has_label = |section: Entity| q_labels.iter().any(|(_, l)| l.section == section);
    for section in bindable {
        if !has_label(section) {
            commands.spawn((
                DespawnOnExit(ExampleStates::Editor),
                SectionKeybindLeader { section },
                Name::new("Section Keybind Leader"),
                GlobalZIndex(crate::ui::layer::STAGE_LABEL_Z),
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(1),
                    height: px(LEADER_PX),
                    ..default()
                },
                BackgroundColor(nova_ui::theme::AMBER_NOVA.with_alpha(0.6)),
                Visibility::Hidden,
            ));
            commands.spawn((
                DespawnOnExit(ExampleStates::Editor),
                SectionKeybindLabel { section },
                Name::new("Section Keybind Label"),
                // Under the panels, like every other name the stage hangs on a
                // node: see `crate::ui::layer`.
                GlobalZIndex(crate::ui::layer::STAGE_LABEL_Z),
                // A chip sits over the section it labels, so without this it
                // blocks the picking ray to that section and clicking it does
                // nothing (`ui::card` and `ui::tooltip` do the same).
                Pickable::IGNORE,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(nova_ui::theme::AMBER_NOVA),
                TextShadow::default(),
                Node {
                    position_type: PositionType::Absolute,
                    // Pill padding + rounded corners so the background
                    // reads as a chip (BorderRadius is a Node field, not a
                    // component).
                    padding: UiRect::axes(px(6), px(2)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                // Dark semi-transparent pill so the amber text stays
                // legible over the 3D scene.
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
pub(crate) fn position_section_keybind_labels(
    rebind: Res<EditorRebind>,
    // Keyed on the editor's camera MARKER, not on the free-fly controller: the
    // gallery removes that controller while it parks the camera, and a `Single`
    // that stops matching stops the system - which used to leave every chip
    // frozen on screen, over the gallery, at the pose it last had.
    camera: Single<(&Camera, &GlobalTransform), With<EditorCamera>>,
    q_section: Query<(&GlobalTransform, &SectionNode)>,
    mut q_labels: Query<(
        &SectionKeybindLabel,
        &mut Node,
        &mut Text,
        &mut Visibility,
        &ComputedNode,
    )>,
    mut q_leaders: Query<
        (&SectionKeybindLeader, &mut Node, &mut Visibility),
        Without<SectionKeybindLabel>,
    >,
) {
    let (cam, cam_transform) = *camera;
    let Some(viewport) = cam.logical_viewport_size() else {
        return;
    };
    // Where each part IS on screen, and where its chip ended up: the second is
    // the first pushed clear of the chips already placed.
    let mut standing: Vec<Vec2> = Vec::new();
    for (SectionKeybindLabel { section }, mut node, mut text, mut visibility, computed) in
        &mut q_labels
    {
        // The chip's own height, from last frame's layout: the chip hangs
        // ABOVE the part, and where its top edge goes depends on how tall it
        // is. One frame stale is exact for a still camera and a pixel off
        // while one moves.
        let placed = q_section
            .get(*section)
            .ok()
            .and_then(|(section_transform, node_section)| {
                let anchor = cam
                    .world_to_viewport(cam_transform, section_transform.translation())
                    .ok()?;
                let spot = clear_of(anchor, CHIP_CLEARANCE_PX, viewport, &mut standing);
                let corner = hang_at(spot, CHIP_HANG, computed, viewport)?;
                Some((anchor, corner, node_section))
            });
        let Some((anchor, corner, node_section)) = placed else {
            // Behind the camera, or beside the frame: `world_to_viewport` errs
            // on the first and answers an off-screen point for the second, so
            // `hang_at` is the one that decides. Either way, do not draw.
            *visibility = Visibility::Hidden;
            for (SectionKeybindLeader { section: named }, _, mut leader) in &mut q_leaders {
                if named == section && *leader != Visibility::Hidden {
                    *leader = Visibility::Hidden;
                }
            }
            continue;
        };
        node.left = Val::Px(corner.x);
        node.top = Val::Px(corner.y);
        *visibility = Visibility::Visible;

        // The leader runs from the chip's foot down to the part itself, so a
        // chip pushed clear of a pile still says which part it came from. From
        // the chip's OWN corner rather than from the spot it asked for: a chip
        // slid along an edge is a chip the leader has to follow.
        let foot = corner.y + computed.size().y * computed.inverse_scale_factor();
        for (SectionKeybindLeader { section: named }, mut leader, mut shown) in &mut q_leaders {
            if named != section {
                continue;
            }
            leader.left = Val::Px(corner.x - CHIP_OFFSET_PX);
            leader.top = Val::Px(foot);
            leader.height = Val::Px((anchor.y - foot).max(0.0));
            *shown = Visibility::Visible;
        }

        let wanted = if rebind.target == Some(*section) {
            REBIND_PROMPT.to_string()
        } else {
            source_label(&node_section.binds)
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

/// Take every chip off screen. The chips label the ship the builder is
/// standing in front of, so a surface that COVERS that ship - the parts gallery
/// - must not be read through them.
///
/// Paired with the run condition that stops the positioner while the gallery is
/// up: exactly one of the two writes the
/// chips' visibility in any frame, so neither can fight the other.
pub(crate) fn hide_section_keybind_labels(
    mut q_labels: Query<
        &mut Visibility,
        Or<(With<SectionKeybindLabel>, With<SectionKeybindLeader>)>,
    >,
) {
    for mut visibility in &mut q_labels {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

/// The top bar's Rebind action: arm a keybind capture for the SELECTED
/// section.
///
/// A click selects and the intent to rebind is its own button, so the capture
/// can never be armed by accident. The guards mirror `apply_section_rebind`'s
/// validity check: a selection that is not a bindable section of the edited
/// ship arms nothing.
pub(crate) fn on_rebind_action(
    _activate: On<Activate>,
    catalog: Option<Res<GameSections>>,
    context: Res<EditContext>,
    selected: Res<SelectedNode>,
    q_sections: Query<(&SectionNode, &ChildOf)>,
    mut rebind: ResMut<EditorRebind>,
    mut mode: ResMut<InputMode>,
) {
    let Some(section) = selected.0 else {
        return;
    };
    let Ok((node, owner)) = q_sections.get(section) else {
        return;
    };
    if context.ship() != Some(owner.parent()) || !node.bindable(catalog.as_deref()) {
        return;
    }
    rebind.target = Some(section);
    // The press that armed this is a mouse click on the button: wait for it to
    // release, so the arming LMB is not captured as the new binding.
    rebind.awaiting_release = true;
    // And the mode with it, in THIS frame. The claimant that reads
    // `rebind.target` runs in the next `PreUpdate`, and the verbs gated on
    // Normal run in between: one Escape inside that window cancelled the
    // capture and put the armed part down as well.
    take_keyboard_now(&mut mode, InputMode::Bind);
}

/// What else `binding` already drives, or `None` when nothing does.
///
/// A WARNING, not a veto. The flight rig holds Space for the main burn, and
/// the editor used to refuse it - which meant a builder who wanted Space to
/// fire their thrusters could not have it, on the editor's say-so. Every
/// section action runs with `consume_input: false`, so a shared key fires both
/// things, and whether that is what you meant is yours to decide.
///
/// Two SECTIONS may share a source for the same reason: firing two turrets on
/// one trigger is a loadout choice, and the content lint does not compare
/// sections to each other either.
fn binding_conflict(source: InputSource) -> Option<String> {
    flight_rig_reserved_sources()
        .into_iter()
        .find(|(reserved, _)| *reserved == source)
        .map(|(_, verb)| format!("the flight rig's {verb}"))
}

/// Consume the next key or mouse-button press to rebind the armed section (see
/// [`EditorRebind`]). Escape cancels. The new binding replaces the section's
/// previous PRIMARY input (keyboard or mouse button; any gamepad binding is
/// preserved) on the section NODE - the one place the binds live, and what the
/// scenario hand-off reads.
pub(crate) fn apply_section_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    catalog: Option<Res<GameSections>>,
    context: Res<EditContext>,
    mut rebind: ResMut<EditorRebind>,
    mut q_sections: Query<(&mut SectionNode, &ChildOf)>,
    mut says: EditorSays,
) {
    let Some(section) = rebind.target else {
        return;
    };
    // The section vanished (deleted while armed), or the editor left the ship
    // it belongs to. Either way the chip that was prompting is gone, so a key
    // pressed now would be captured by a section nothing is showing.
    let still_bindable = q_sections.get(section).is_ok_and(|(node, owner)| {
        context.ship() == Some(owner.parent()) && node.bindable(catalog.as_deref())
    });
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
        .map(|k| InputSource::Keyboard(*k))
        .or_else(|| {
            mouse
                .get_just_pressed()
                .next()
                .map(|b| InputSource::Mouse(*b))
        });
    let Some(new_binding) = new_binding else {
        return;
    };

    // A key the flight rig also drives is TAKEN, and said out loud: both things
    // fire on it, which is a choice a builder is allowed to make.
    if let Some(taken_by) = binding_conflict(new_binding) {
        says.note(format!("{} also drives {taken_by}", new_binding.label()));
    }

    // Replace the PRIMARY input (keyboard OR mouse button), keep gamepad binds.
    let rebind_binds = |current: &[InputSource]| -> Vec<InputSource> {
        let mut binds: Vec<InputSource> = current
            .iter()
            .filter(|b| !matches!(b, InputSource::Keyboard(_) | InputSource::Mouse(_)))
            .copied()
            .collect();
        binds.insert(0, new_binding);
        binds
    };

    let Ok((mut node, _)) = q_sections.get_mut(section) else {
        rebind.target = None;
        return;
    };
    node.binds = rebind_binds(&node.binds);
    rebind.target = None;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_scenario::prelude::SectionSource;

    use super::*;
    use crate::node::ShipNode;

    /// The ship these tests edit: spawned and ENTERED on first use, so a
    /// section from `section_node` hangs on the ship in the edit context. The
    /// chips are scoped to that ship, so a fixture without one labels nothing.
    fn edited_ship(world: &mut World) -> Entity {
        let mut ships = world.query_filtered::<Entity, With<ShipNode>>();
        if let Some(ship) = ships.iter(world).next() {
            return ship;
        }
        let ship = world.spawn(ShipNode::default()).id();
        world.insert_resource(EditContext {
            path: vec![Entity::PLACEHOLDER, ship],
        });
        ship
    }

    /// A section node of `kind` on `ship`, carrying `binds` - the shape every
    /// one of these tests works on now that a section's inputs live on its node.
    fn section_on(
        world: &mut World,
        ship: Entity,
        kind: SectionKind,
        binds: Vec<InputSource>,
    ) -> Entity {
        world
            .spawn((
                SectionNode {
                    source: SectionSource::Inline(SectionConfig {
                        base: BaseSectionConfig {
                            id: "part".to_string(),
                            name: "part".to_string(),
                            ..default()
                        },
                        kind,
                    }),
                    modifications: vec![],
                    binds,
                },
                ChildOf(ship),
            ))
            .id()
    }

    /// The same, on the ship the editor is inside.
    fn section_node(world: &mut World, kind: SectionKind, binds: Vec<InputSource>) -> Entity {
        let ship = edited_ship(world);
        section_on(world, ship, kind, binds)
    }

    fn thruster(world: &mut World, binds: Vec<InputSource>) -> Entity {
        section_node(world, SectionKind::Thruster(default()), binds)
    }

    fn turret(world: &mut World, binds: Vec<InputSource>) -> Entity {
        section_node(world, SectionKind::Turret(default()), binds)
    }

    fn binds_of(world: &World, section: Entity) -> Vec<InputSource> {
        world
            .entity(section)
            .get::<SectionNode>()
            .expect("a section node")
            .binds
            .clone()
    }

    fn armed(world: &mut World, section: Entity) {
        world.init_resource::<EditorRebind>();
        // A refusal reaches for the status line and the clock behind it.
        world.init_resource::<crate::config::EditorStatus>();
        world.init_resource::<Time>();
        world.resource_mut::<EditorRebind>().target = Some(section);
    }

    #[test]
    fn keybind_labels_reconcile_to_one_per_bound_section() {
        let mut world = World::new();
        let section = thruster(&mut world, vec![InputSource::from(KeyCode::KeyW)]);
        // A hull takes no binding at all, so it gets no label.
        let _unbound = section_node(&mut world, SectionKind::Hull(default()), vec![]);

        world.run_system_once(sync_section_keybind_labels).unwrap();
        let labels: Vec<Entity> = world
            .query::<&SectionKeybindLabel>()
            .iter(&world)
            .map(|l| l.section)
            .collect();
        assert_eq!(
            labels,
            vec![section],
            "one label, for the bindable section only"
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

    /// Chips label the ship you are INSIDE. A second ship standing beside it
    /// carries its own bindable sections, and drawing their keys over it would
    /// offer a rebind the hand-off never reads - only the player-driven ship's
    /// binds are lowered.
    #[test]
    fn a_ship_you_are_not_inside_gets_no_chips() {
        let mut world = World::new();
        let mine = thruster(&mut world, vec![InputSource::from(KeyCode::KeyW)]);
        let other = world.spawn(ShipNode::default()).id();
        section_on(&mut world, other, SectionKind::Turret(default()), vec![]);

        world.run_system_once(sync_section_keybind_labels).unwrap();
        let labels: Vec<Entity> = world
            .query::<&SectionKeybindLabel>()
            .iter(&world)
            .map(|l| l.section)
            .collect();
        assert_eq!(labels, vec![mine], "only the edited ship is labelled");

        // Backing out to the scenario context takes the chips with it.
        world.resource_mut::<EditContext>().exit();
        world.run_system_once(sync_section_keybind_labels).unwrap();
        assert_eq!(
            world.query::<&SectionKeybindLabel>().iter(&world).count(),
            0,
            "no ship entered, nothing to label"
        );
    }

    /// A rebind armed on one ship must not survive leaving it: the chip that
    /// was prompting is gone, so the next key would be captured by a section
    /// nothing is showing.
    #[test]
    fn leaving_the_ship_drops_a_pending_rebind() {
        let mut world = World::new();
        let section = turret(&mut world, vec![InputSource::from(KeyCode::Space)]);
        armed(&mut world, section);
        world.resource_mut::<EditContext>().exit();
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "the rebind is dropped"
        );
        assert_eq!(
            binds_of(&world, section),
            vec![InputSource::from(KeyCode::Space)],
            "and the key that was pressed bound nothing"
        );
    }

    /// The bind lives in ONE place now - the section node - so this asserts the
    /// document, which is also what the scenario hand-off reads.
    #[test]
    fn rebind_replaces_the_keyboard_bind_on_the_node() {
        let mut world = World::new();
        let section = thruster(
            &mut world,
            vec![
                InputSource::from(KeyCode::Space),
                InputSource::from(GamepadButton::RightTrigger),
            ],
        );
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        let binds = binds_of(&world, section);
        assert!(
            binds
                .iter()
                .any(|b| matches!(b, InputSource::Keyboard(key) if *key == KeyCode::KeyR)),
            "the new key is bound"
        );
        assert!(
            !binds
                .iter()
                .any(|b| matches!(b, InputSource::Keyboard(key) if *key == KeyCode::Space)),
            "the old key is replaced"
        );
        assert!(
            binds.iter().any(|b| matches!(b, InputSource::Gamepad(_))),
            "a non-keyboard bind is preserved"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "the rebind is consumed"
        );
    }

    /// F30: the chip is a root UI node drawn over the section it labels, so
    /// without an IGNORE override it eats the click meant for that section.
    #[test]
    fn keybind_chips_do_not_block_the_picking_ray() {
        let mut world = World::new();
        thruster(&mut world, vec![InputSource::from(KeyCode::KeyR)]);

        world.run_system_once(sync_section_keybind_labels).unwrap();

        let chip = world
            .query_filtered::<Entity, With<SectionKeybindLabel>>()
            .single(&world)
            .unwrap();
        assert_eq!(
            world.entity(chip).get::<Pickable>().copied(),
            Some(Pickable::IGNORE),
            "the chip must not block or absorb the pointer"
        );
    }

    /// A key the flight rig drives is BOUND, and the line says what else it
    /// does. The editor knows what Space is for; it does not get to decide
    /// that a builder may not fire their thrusters with it.
    #[test]
    fn rebind_takes_a_key_the_flight_rig_drives_and_says_so() {
        let mut world = World::new();
        let section = turret(&mut world, vec![InputSource::from(MouseButton::Left)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        // Space is the main drive - see `flight_rig_reserved_sources`.
        input.press(KeyCode::Space);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert_eq!(
            binds_of(&world, section),
            vec![InputSource::from(KeyCode::Space)],
            "the key is bound"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "the rebind is finished, not left prompting"
        );
        let (line, _) = world
            .resource::<crate::config::EditorStatus>()
            .line()
            .expect("a shared key is reported on the line");
        // The verb comes from the registry, not from a word spelled here: a
        // renamed action must move this assertion with it, not break it.
        let verb = flight_rig_reserved_sources()
            .into_iter()
            .find(|(source, _)| *source == InputSource::Keyboard(KeyCode::Space))
            .map(|(_, verb)| verb)
            .expect("the flight rig holds Space");
        assert!(
            line.contains("Space") && line.contains(verb),
            "a builder pressing a key the rig holds is owed the other thing it \
             does; the line read {line:?}"
        );
    }

    /// Two sections may hold one source - two turrets on one trigger, two
    /// thrusters together. Section actions run with `consume_input: false`, so
    /// both fire, and the content lint does not compare sections either.
    #[test]
    fn rebind_lets_two_sections_share_one_key() {
        let mut world = World::new();
        let taken = thruster(&mut world, vec![InputSource::from(KeyCode::KeyR)]);
        let section = turret(&mut world, vec![InputSource::from(MouseButton::Left)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert!(
            binds_of(&world, section)
                .iter()
                .any(|b| matches!(b, InputSource::Keyboard(key) if *key == KeyCode::KeyR)),
            "the second section takes the shared key"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "the rebind is consumed rather than left armed"
        );
        // The first section keeps it: sharing adds a holder, it does not move
        // the key off the section that had it. Both reach the scenario, because
        // `input_mapping` is keyed by section - one source can appear twice.
        assert_eq!(
            binds_of(&world, taken),
            vec![InputSource::from(KeyCode::KeyR)],
            "the first section keeps the key"
        );
    }

    /// The Rebind action arms the SELECTED section, and only a selection the
    /// capture could actually serve: a bindable section of the edited ship.
    #[test]
    fn the_rebind_action_arms_only_a_bindable_selection() {
        use bevy::ui_widgets::Activate;

        use crate::config::SelectedNode;

        let mut app = App::new();
        app.init_resource::<EditorRebind>();
        app.init_resource::<SelectedNode>();
        // Arming takes the keyboard in the same frame, so the rig carries the
        // mode the arbiter would otherwise write a frame later.
        app.init_resource::<InputMode>();
        app.add_observer(on_rebind_action);
        let turret = turret(app.world_mut(), vec![InputSource::from(MouseButton::Left)]);
        let hull = section_node(app.world_mut(), SectionKind::Hull(default()), vec![]);
        let button = app.world_mut().spawn_empty().id();

        // Nothing selected: the press arms nothing.
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(app.world().resource::<EditorRebind>().target, None);

        // A hull selected: not bindable, still nothing.
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(hull);
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(app.world().resource::<EditorRebind>().target, None);

        // The turret selected: armed, and waiting out the arming click.
        app.world_mut().resource_mut::<SelectedNode>().0 = Some(turret);
        app.world_mut().trigger(Activate { entity: button });
        let rebind = app.world().resource::<EditorRebind>();
        assert_eq!(rebind.target, Some(turret));
        assert!(
            rebind.awaiting_release,
            "the arming click must not be captured as the binding"
        );
        assert_eq!(
            *app.world().resource::<InputMode>(),
            InputMode::Bind,
            "and the mode is Bind in the arming frame, not the one after it"
        );
    }

    #[test]
    fn rebind_escape_cancels_without_changing_the_bind() {
        let mut world = World::new();
        let section = turret(&mut world, vec![InputSource::from(KeyCode::Space)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Escape);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert_eq!(
            binds_of(&world, section),
            vec![InputSource::from(KeyCode::Space)],
            "unchanged"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "Escape still consumes the arm"
        );
    }

    #[test]
    fn rebind_binds_a_mouse_button_after_the_arming_click_releases() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        // Turret with a KEYBOARD primary + a gamepad bind; we'll swap the primary
        // to LMB.
        let section = turret(
            &mut world,
            vec![
                InputSource::from(KeyCode::Space),
                InputSource::from(GamepadButton::RightTrigger2),
            ],
        );
        armed(&mut world, section);
        world.resource_mut::<EditorRebind>().awaiting_release = true; // armed by a click
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

        let binds = binds_of(&world, section);
        assert!(
            binds
                .iter()
                .any(|b| matches!(b, InputSource::Mouse(button) if *button == MouseButton::Left)),
            "LMB is now bound"
        );
        assert!(
            !binds.iter().any(|b| matches!(b, InputSource::Keyboard(_))),
            "the old keyboard primary is replaced"
        );
        assert!(
            binds.iter().any(|b| matches!(b, InputSource::Gamepad(_))),
            "the gamepad bind is preserved"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "rebind consumed"
        );
    }
}
