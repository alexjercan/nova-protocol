//! Section keybind labels + click-to-rebind. Each
//! bindable section (thruster/turret/torpedo) gets a screen-space chip showing
//! its current key; clicking it in select mode arms a rebind that captures the
//! next key or mouse-button press.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::Binding;
use nova_ship::prelude::*;

use crate::{config::PlayerSpaceshipConfig, gallery::EditorCamera, ExampleStates};

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
/// frame over the section by projecting its world position with the editor
/// camera. One per bindable (thruster/turret/torpedo) section.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct SectionKeybindLabel {
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
pub(crate) fn sync_section_keybind_labels(
    mut commands: Commands,
    q_bindable: Query<Entity, BindableFilter>,
    q_labels: Query<(Entity, &SectionKeybindLabel)>,
) {
    for (label, SectionKeybindLabel { section }) in &q_labels {
        if q_bindable.get(*section).is_err() {
            commands.entity(label).despawn();
        }
    }
    let has_label = |section: Entity| q_labels.iter().any(|(_, l)| l.section == section);
    for section in &q_bindable {
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
#[expect(
    clippy::type_complexity,
    reason = "one optional binding component per section kind"
)]
pub(crate) fn position_section_keybind_labels(
    rebind: Res<EditorRebind>,
    // Keyed on the editor's camera MARKER, not on the free-fly controller: the
    // gallery removes that controller while it parks the camera, and a `Single`
    // that stops matching stops the system - which used to leave every chip
    // frozen on screen, over the gallery, at the pose it last had.
    camera: Single<(&Camera, &GlobalTransform), With<EditorCamera>>,
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
/// preserved) on both the live component and `PlayerSpaceshipConfig::inputs`
/// (what the scenario reads).
pub(crate) fn apply_section_rebind(
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use nova_gameplay::prelude::*;

    use super::*;

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

    /// F30: the chip is a root UI node drawn over the section it labels, so
    /// without an IGNORE override it eats the click meant for that section.
    #[test]
    fn keybind_chips_do_not_block_the_picking_ray() {
        let mut world = World::new();
        world.spawn(SpaceshipThrusterInputBinding(vec![Binding::from(
            KeyCode::KeyR,
        )]));

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
        world.init_resource::<EditorRebind>();
        world.init_resource::<PlayerSpaceshipConfig>();
        let section = world
            .spawn(SpaceshipTurretInputBinding(vec![Binding::from(
                MouseButton::Left,
            )]))
            .id();
        world.resource_mut::<EditorRebind>().target = Some(section);
        let mut input = ButtonInput::<KeyCode>::default();
        // "autopilot goto" - see `flight_rig_reserved_sources`.
        input.press(KeyCode::KeyG);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        let binds = &world
            .entity(section)
            .get::<SpaceshipTurretInputBinding>()
            .unwrap()
            .0;
        assert_eq!(
            binds,
            &vec![Binding::from(MouseButton::Left)],
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
        world.init_resource::<EditorRebind>();
        let taken = world
            .spawn(SpaceshipThrusterInputBinding(vec![Binding::from(
                KeyCode::KeyR,
            )]))
            .id();
        let section = world
            .spawn(SpaceshipTurretInputBinding(vec![Binding::from(
                MouseButton::Left,
            )]))
            .id();
        let mut config = PlayerSpaceshipConfig::default();
        config
            .inputs
            .insert(taken, vec![Binding::from(KeyCode::KeyR)]);
        world.insert_resource(config);
        world.resource_mut::<EditorRebind>().target = Some(section);
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::KeyR);
        world.insert_resource(input);
        world.init_resource::<ButtonInput<MouseButton>>();

        world.run_system_once(apply_section_rebind).unwrap();

        assert!(
            world
                .entity(section)
                .get::<SpaceshipTurretInputBinding>()
                .unwrap()
                .0
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
        // the key off the section that had it.
        assert_eq!(
            &world
                .entity(taken)
                .get::<SpaceshipThrusterInputBinding>()
                .unwrap()
                .0,
            &vec![Binding::from(KeyCode::KeyR)],
            "the first section keeps the key"
        );
        // Both holders reach the scenario: `input_mapping` is keyed by section,
        // so one source can appear twice.
        let config = world.resource::<PlayerSpaceshipConfig>();
        let holders = [taken, section].map(|e| {
            config.inputs.get(&e).is_some_and(|binds| {
                binds
                    .iter()
                    .any(|b| matches!(b, Binding::Keyboard { key, .. } if *key == KeyCode::KeyR))
            })
        });
        assert_eq!(holders, [true, true], "both sections hand off the key");
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
