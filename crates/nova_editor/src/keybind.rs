//! Section keybind labels + click-to-rebind. Each
//! bindable section (thruster/turret/torpedo) gets a screen-space chip showing
//! its current key; clicking it in select mode arms a rebind that captures the
//! next key or mouse-button press.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Binding;
use nova_ship::prelude::*;

use crate::{gallery::EditorCamera, node::SectionNode, ExampleStates};

/// The section currently awaiting a new keybind. Armed by clicking a bindable
/// section in select mode (`SectionChoice::None`); `apply_section_rebind`
/// consumes the next key or mouse-button press. Reset to `None` on every state
/// entry.
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

/// Every section node that takes an input binding.
///
/// Read off the DOCUMENT rather than off three optional components on a view:
/// a section's binds belong to its node, and the view they are drawn over is
/// despawned and rebuilt on every visit to the editor.
fn bindable_sections(
    q_sections: &Query<(Entity, &SectionNode)>,
    sections: Option<&GameSections>,
) -> Vec<Entity> {
    q_sections
        .iter()
        .filter(|(_, section)| section.bindable(sections))
        .map(|(entity, _)| entity)
        .collect()
}

/// Keep exactly one [`SectionKeybindLabel`] per bindable section: spawn for new
/// ones, despawn labels whose section is gone or is not bindable. Reconcile
/// shape mirrors the ammo readout's `sync_ammo_readouts`.
pub(crate) fn sync_section_keybind_labels(
    mut commands: Commands,
    catalog: Option<Res<GameSections>>,
    q_sections: Query<(Entity, &SectionNode)>,
    q_labels: Query<(Entity, &SectionKeybindLabel)>,
) {
    let bindable = bindable_sections(&q_sections, catalog.as_deref());
    for (label, SectionKeybindLabel { section }) in &q_labels {
        if !bindable.contains(section) {
            commands.entity(label).despawn();
        }
    }
    let has_label = |section: Entity| q_labels.iter().any(|(_, l)| l.section == section);
    for section in bindable {
        if !has_label(section) {
            commands.spawn((
                DespawnOnExit(ExampleStates::Editor),
                SectionKeybindLabel { section },
                Name::new("Section Keybind Label"),
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
                    // NOTE: pill padding + rounded corners so the background
                    // reads as a chip (BorderRadius is a Node field, not a
                    // component).
                    padding: UiRect::axes(px(6), px(2)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                // NOTE: dark semi-transparent pill so the amber text stays
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
    mut q_labels: Query<(&SectionKeybindLabel, &mut Node, &mut Text, &mut Visibility)>,
) {
    let (cam, cam_transform) = *camera;
    for (SectionKeybindLabel { section }, mut node, mut text, mut visibility) in &mut q_labels {
        let Ok((section_transform, node_section)) = q_section.get(*section) else {
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
            binding_label(&node_section.binds)
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
    mut q_labels: Query<&mut Visibility, With<SectionKeybindLabel>>,
) {
    for mut visibility in &mut q_labels {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Why `binding` cannot be given to a section, or `None` when it is free.
/// Applies the one rule the content lint's input-overlap check applies to
/// authored ships ([`flight_rig_reserved_sources`]) - an editor-built ship is
/// assembled at runtime and never linted, so this is the only place that
/// catches it.
///
/// Two SECTIONS may share a source. Every section action runs with
/// `consume_input: false`, so both fire, and firing two turrets (or two
/// thrusters) on one trigger is a loadout choice. The lint does not compare
/// sections to each other either, so an editor export stays authorable.
fn binding_conflict(binding: &Binding) -> Option<String> {
    let source = binding_source(binding)?;
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
    mut rebind: ResMut<EditorRebind>,
    mut q_sections: Query<&mut SectionNode>,
) {
    let Some(section) = rebind.target else {
        return;
    };
    // The section vanished (deleted while armed): drop the rebind.
    let still_bindable = q_sections
        .get(section)
        .is_ok_and(|node| node.bindable(catalog.as_deref()));
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

    // A conflicting key stays armed rather than being accepted: the chip keeps
    // prompting, so the player just presses another key.
    if let Some(taken_by) = binding_conflict(&new_binding) {
        warn!("editor: {new_binding:?} is already driven by {taken_by} - pick another key");
        return;
    }

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

    let Ok(mut node) = q_sections.get_mut(section) else {
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

    /// A section node of `kind`, carrying `binds` - the shape every one of
    /// these tests works on now that a section's inputs live on its node.
    fn section_node(world: &mut World, kind: SectionKind, binds: Vec<Binding>) -> Entity {
        world
            .spawn(SectionNode {
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
            })
            .id()
    }

    fn thruster(world: &mut World, binds: Vec<Binding>) -> Entity {
        section_node(world, SectionKind::Thruster(default()), binds)
    }

    fn turret(world: &mut World, binds: Vec<Binding>) -> Entity {
        section_node(world, SectionKind::Turret(default()), binds)
    }

    fn binds_of(world: &World, section: Entity) -> Vec<Binding> {
        world
            .entity(section)
            .get::<SectionNode>()
            .expect("a section node")
            .binds
            .clone()
    }

    fn armed(world: &mut World, section: Entity) {
        world.init_resource::<EditorRebind>();
        world.resource_mut::<EditorRebind>().target = Some(section);
    }

    #[test]
    fn keybind_labels_reconcile_to_one_per_bound_section() {
        let mut world = World::new();
        let section = thruster(&mut world, vec![Binding::from(KeyCode::KeyW)]);
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

    /// The bind lives in ONE place now - the section node - so this asserts the
    /// document, which is also what the scenario hand-off reads.
    #[test]
    fn rebind_replaces_the_keyboard_bind_on_the_node() {
        let mut world = World::new();
        let section = thruster(
            &mut world,
            vec![
                Binding::from(KeyCode::Space),
                Binding::from(GamepadButton::RightTrigger),
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
        thruster(&mut world, vec![Binding::from(KeyCode::KeyR)]);

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

    /// F32: rebinding onto a source the flight rig already drives is refused -
    /// the same rule the content lint enforces on authored ships. The section
    /// stays armed so the next key can be tried.
    #[test]
    fn rebind_refuses_a_key_the_flight_rig_drives() {
        let mut world = World::new();
        let section = turret(&mut world, vec![Binding::from(MouseButton::Left)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        // "autopilot goto" - see `flight_rig_reserved_sources`.
        input.press(KeyCode::KeyG);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert_eq!(
            binds_of(&world, section),
            vec![Binding::from(MouseButton::Left)],
            "the conflicting key is not bound"
        );
        assert_eq!(
            world.resource::<EditorRebind>().target,
            Some(section),
            "the section stays armed for another try"
        );
    }

    /// Two sections may hold one source - two turrets on one trigger, two
    /// thrusters together. Section actions run with `consume_input: false`, so
    /// both fire, and the content lint does not compare sections either.
    #[test]
    fn rebind_lets_two_sections_share_one_key() {
        let mut world = World::new();
        let taken = thruster(&mut world, vec![Binding::from(KeyCode::KeyR)]);
        let section = turret(&mut world, vec![Binding::from(MouseButton::Left)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert!(
            binds_of(&world, section)
                .iter()
                .any(|b| matches!(b, Binding::Keyboard { key, .. } if *key == KeyCode::KeyR)),
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
            vec![Binding::from(KeyCode::KeyR)],
            "the first section keeps the key"
        );
    }

    #[test]
    fn rebind_escape_cancels_without_changing_the_bind() {
        let mut world = World::new();
        let section = turret(&mut world, vec![Binding::from(KeyCode::Space)]);
        armed(&mut world, section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Escape);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert_eq!(
            binds_of(&world, section),
            vec![Binding::from(KeyCode::Space)],
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
                Binding::from(KeyCode::Space),
                Binding::from(GamepadButton::RightTrigger2),
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
        assert_eq!(
            world.resource::<EditorRebind>().target,
            None,
            "rebind consumed"
        );
    }
}
