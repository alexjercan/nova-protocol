use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::SectionClass;
use nova_input::prelude::{
    captured_binds, rebind_verdict, ActionContext, InputBindings, InputSource, InputSources,
    RebindSurface, RebindVerdict,
};
use nova_ship::prelude::*;

use super::ShipRuntime;

/// What flying the ship already spends `source` on, read off the LIVE table -
/// a player who rebinds the main drive frees Space for a section, and one who
/// moves a verb ONTO a section's key must be told here.
fn reserved_conflict(bindings: &InputBindings, source: InputSource) -> Option<String> {
    bindings
        .holder_in(ActionContext::Flight, source)
        .map(|action| format!("flight control: {}", action.label))
}

/// The binding a section of `class` rebinds from: its own, or an empty one
/// when it is of `kind` and carries none.
fn or_unbound(
    class: Option<SectionClass>,
    binding: Option<&[InputSource]>,
    kind: SectionClass,
) -> Option<&[InputSource]> {
    binding.or((class == Some(kind)).then_some(&[]))
}

#[expect(
    clippy::type_complexity,
    reason = "one filtered query over the rebind rows"
)]
pub(crate) fn apply_ship_rebind(
    sources: InputSources,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<InputBindings>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    targets: Query<(
        Entity,
        &ChildOf,
        &EntityId,
        Option<&SectionClass>,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
        Option<&SpaceshipRailgunInputBinding>,
        Option<&SpaceshipMiningInputBinding>,
    )>,
    mut changed: MessageWriter<SectionInputBindingChanged>,
) {
    if runtime.rebind_swallow && sources.all_released() {
        runtime.rebind_swallow = false;
    }
    let Some(target) = runtime.rebinding else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        runtime.rebinding = None;
        runtime.rebind_awaiting_release = false;
        runtime.rebind_swallow = true;
        runtime.note = Some(("Rebind cancelled".to_string(), 2.5));
        return;
    }
    // Wait for a CLEAN frame, not merely the next one. This capture is armed by
    // a click on the panel button or by a key, and one frame's grace let the
    // arming press itself through whenever it was still held.
    if runtime.rebind_awaiting_release {
        if sources.all_released() {
            runtime.rebind_awaiting_release = false;
        }
        return;
    }
    // Backspace takes a thruster's own key away, so the Main Drive fires it
    // again. Every other section binds Backspace like any key.
    if keys.just_pressed(KeyCode::Backspace) {
        if let Ok((_, parent, id, Some(SectionClass::Thruster), ..)) = targets.get(target) {
            commands
                .entity(target)
                .remove::<SpaceshipThrusterInputBinding>();
            changed.write(SectionInputBindingChanged {
                spaceship: parent.parent(),
                section: target,
                section_id: id.0.clone(),
                bindings: Vec::new(),
            });
            runtime.rebinding = None;
            runtime.rebind_swallow = true;
            runtime.note = Some((format!("{} fires on the Main Drive", id.0), 2.5));
            return;
        }
    }
    // The shared capture, not a hand-rolled one: `ButtonInput` iterates a
    // `HashSet`, so picking the FIRST just-pressed key bound either of two keys
    // pressed on one frame, run to run. The desk wins a frame both devices
    // press on.
    let Some(source) = sources.captured_desk().or_else(|| sources.captured_pad()) else {
        return;
    };
    let Ok((_, parent, id, class, thruster, turret, torpedo, railgun, mining)) =
        targets.get(target)
    else {
        runtime.rebinding = None;
        return;
    };
    let ship = parent.parent();
    // The pointer's own button, and a source flying the ship already spends:
    // `RebindSurface::ShipPanel` is where this panel's two refusals stand
    // against the editor's deliberate warn-instead-of-refuse.
    let held_by = reserved_conflict(&bindings, source);
    let verdict = rebind_verdict(RebindSurface::ShipPanel, source, held_by);
    if let RebindVerdict::Refuse(line) = verdict {
        runtime.note = Some((line, 2.5));
        return;
    }
    // Each mining section deploys on its own key, so a key another emitter on
    // this ship holds would deploy both on one press. The lint and the spawn
    // refuse the same. Other section kinds may still share a key on purpose.
    // A weapon, emitter or thruster the ship never mapped has no binding
    // component; it takes the kind's, empty, so the capture gives it its
    // first key. A thruster's own key takes it off the Main Drive.
    let class = class.copied();
    let mining = or_unbound(
        class,
        mining.map(|binding| binding.0.as_slice()),
        SectionClass::Mining,
    );
    if mining.is_some() {
        let other = targets
            .iter()
            .find(|(entity, other_parent, .., other_mining)| {
                *entity != target
                    && other_parent.parent() == ship
                    && other_mining.is_some_and(|binding| binding.0.contains(&source))
            });
        if let Some((_, _, other_id, ..)) = other {
            runtime.note = Some((
                format!(
                    "mining section {} already holds {}",
                    other_id.0,
                    source.readout_label()
                ),
                2.5,
            ));
            return;
        }
    }

    // The captured source replaces the half of its own device and the other
    // half is kept: a key says nothing about the controller trigger the same
    // section is also on, and a pad button says nothing about the key.
    let rebound = |current: &[InputSource]| captured_binds(current, source);
    let binds = if let Some(thruster) = or_unbound(
        class,
        thruster.map(|binding| binding.0.as_slice()),
        SectionClass::Thruster,
    ) {
        let binds = rebound(thruster);
        commands
            .entity(target)
            .insert(SpaceshipThrusterInputBinding(binds.clone()));
        binds
    } else if let Some(turret) = or_unbound(
        class,
        turret.map(|binding| binding.0.as_slice()),
        SectionClass::Turret,
    ) {
        let binds = rebound(turret);
        commands
            .entity(target)
            .insert(SpaceshipTurretInputBinding(binds.clone()));
        binds
    } else if let Some(torpedo) = or_unbound(
        class,
        torpedo.map(|binding| binding.0.as_slice()),
        SectionClass::Torpedo,
    ) {
        let binds = rebound(torpedo);
        commands
            .entity(target)
            .insert(SpaceshipTorpedoInputBinding(binds.clone()));
        binds
    } else if let Some(railgun) = or_unbound(
        class,
        railgun.map(|binding| binding.0.as_slice()),
        SectionClass::Railgun,
    ) {
        let binds = rebound(railgun);
        commands
            .entity(target)
            .insert(SpaceshipRailgunInputBinding(binds.clone()));
        binds
    } else if let Some(mining) = mining {
        let binds = rebound(mining);
        commands
            .entity(target)
            .insert(SpaceshipMiningInputBinding(binds.clone()));
        binds
    } else {
        runtime.rebinding = None;
        return;
    };
    changed.write(SectionInputBindingChanged {
        spaceship: ship,
        section: target,
        section_id: id.0.clone(),
        bindings: binds,
    });
    runtime.rebinding = None;
    runtime.rebind_swallow = true;
    runtime.note = Some((format!("Bound {} to {}", id.0, source.readout_label()), 2.5));
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::ship::{SectionCode, ShipViewSubject};

    fn rebind_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<ShipRuntime>();
        world.init_resource::<ShipViewSubject>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Messages<SectionInputBindingChanged>>();
        // The refusal reads the live table, so a rig with none would fail open.
        world.insert_resource(InputBindings::from_actions(flight_bindings()));
        let ship = world.spawn_empty().id();
        // A key AND a pad trigger, which is the shape a section really has:
        // the tutorial's `pdc` ships both, and every section the editor places
        // is written with a pad bind.
        let target = world
            .spawn((
                ChildOf(ship),
                EntityId("gun".to_string()),
                SectionCode("PDC-1".to_string()),
                SpaceshipTurretInputBinding(vec![
                    KeyCode::KeyF.into(),
                    GamepadButton::RightTrigger2.into(),
                ]),
            ))
            .id();
        world.resource_mut::<ShipRuntime>().rebinding = Some(target);
        (world, target)
    }

    #[test]
    fn sections_on_the_same_ship_can_share_one_binding() {
        let (mut world, target) = rebind_world();
        let ship = world.get::<ChildOf>(target).unwrap().parent();
        world.spawn((
            ChildOf(ship),
            SectionCode("TRB-1".to_string()),
            SpaceshipTorpedoInputBinding(vec![MouseButton::Middle.into()]),
        ));
        {
            let mut runtime = world.resource_mut::<ShipRuntime>();
            runtime.rebinding = Some(target);
        }
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Middle);

        world.run_system_once(apply_ship_rebind).unwrap();

        let target_bindings = &world.get::<SpaceshipTurretInputBinding>(target).unwrap().0;
        assert_eq!(target_bindings[0], InputSource::Mouse(MouseButton::Middle));
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
    }

    /// This panel is driven entirely by clicks, so an armed capture that took
    /// the pointer would bind the section to the next blip a player clicked -
    /// and Left Mouse is the button every shipped turret already fires on.
    #[test]
    fn the_pointers_own_button_is_never_captured() {
        let (mut world, target) = rebind_world();
        {
            let mut runtime = world.resource_mut::<ShipRuntime>();
            runtime.rebinding = Some(target);
        }
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        world.run_system_once(apply_ship_rebind).unwrap();

        assert_eq!(
            world.get::<SpaceshipTurretInputBinding>(target).unwrap().0,
            vec![
                InputSource::Keyboard(KeyCode::KeyF),
                InputSource::Gamepad(GamepadButton::RightTrigger2)
            ],
            "the section keeps what it had"
        );
        assert!(
            world.resource::<ShipRuntime>().rebinding.is_some(),
            "and the capture stays armed for a real press"
        );
    }

    #[test]
    fn reserved_flight_control_remains_blocked() {
        let (mut world, target) = rebind_world();
        let key = world
            .resource::<InputBindings>()
            .rows()
            .filter(|action| action.context.overlaps(ActionContext::Flight))
            .flat_map(|action| action.keyboard.clone())
            .find_map(|source| match source {
                InputSource::Keyboard(key) => Some(key),
                _ => None,
            })
            .expect("the flight rig reserves a keyboard input");
        world.resource_mut::<ButtonInput<KeyCode>>().press(key);

        world.run_system_once(apply_ship_rebind).unwrap();

        assert_eq!(world.resource::<ShipRuntime>().rebinding, Some(target));
        assert!(world
            .resource::<ShipRuntime>()
            .note
            .as_ref()
            .is_some_and(|(note, _)| note.contains("flight control")));
    }

    /// One press must not deploy two emitters: a key another mining section
    /// on the ship holds is refused and the capture stays armed, while the
    /// same key shared with a non-mining section is still allowed.
    #[test]
    fn a_mining_rebind_refuses_a_key_another_mining_section_holds() {
        let (mut world, gun) = rebind_world();
        let ship = world.get::<ChildOf>(gun).unwrap().parent();
        world.spawn((
            ChildOf(ship),
            EntityId("bow_beam".to_string()),
            SpaceshipMiningInputBinding(vec![KeyCode::KeyV.into()]),
        ));
        let keel = world
            .spawn((
                ChildOf(ship),
                EntityId("keel_beam".to_string()),
                SpaceshipMiningInputBinding(vec![KeyCode::KeyB.into()]),
            ))
            .id();
        world.resource_mut::<ShipRuntime>().rebinding = Some(keel);
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyV);

        world.run_system_once(apply_ship_rebind).unwrap();

        assert_eq!(
            world.get::<SpaceshipMiningInputBinding>(keel).unwrap().0,
            vec![InputSource::Keyboard(KeyCode::KeyB)]
        );
        let runtime = world.resource::<ShipRuntime>();
        assert_eq!(runtime.rebinding, Some(keel), "the capture stays armed");
        assert!(
            runtime
                .note
                .as_ref()
                .is_some_and(|(note, _)| note.contains("bow_beam")),
            "{:?}",
            runtime.note
        );

        // The gun fires on F; a mining key may share it.
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyV);
        world.resource_mut::<ButtonInput<KeyCode>>().clear();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);

        world.run_system_once(apply_ship_rebind).unwrap();

        assert_eq!(
            world.get::<SpaceshipMiningInputBinding>(keel).unwrap().0,
            vec![InputSource::Keyboard(KeyCode::KeyF)]
        );
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
    }

    /// A key pressed at this panel says nothing about the controller the
    /// section is also fired on, so the pad half of the trigger stays. This
    /// panel used to write the whole list, which took a shipped section's
    /// controller trigger away from a player who never touched a controller.
    #[test]
    fn a_rebind_replaces_the_key_keeps_the_pad_trigger_and_emits_the_change() {
        let (mut world, target) = rebind_world();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyK);

        world.run_system_once(apply_ship_rebind).unwrap();

        let binding = world
            .get::<SpaceshipTurretInputBinding>(target)
            .unwrap()
            .0
            .clone();
        assert_eq!(
            binding,
            vec![
                InputSource::Keyboard(KeyCode::KeyK),
                InputSource::Gamepad(GamepadButton::RightTrigger2)
            ],
            "the key moved and the pad trigger is still there"
        );
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        let messages = world.resource::<Messages<SectionInputBindingChanged>>();
        assert_eq!(messages.len(), 1);
        let sent = messages.iter_current_update_messages().next().unwrap();
        assert_eq!(
            sent.bindings, binding,
            "and the rig is told the whole trigger, not just the key"
        );
    }

    /// A weapon added with no sibling to copy is UNBOUND, and a gun the ship
    /// never mapped has no binding at all: the panel's capture gives each its
    /// first key, so it fires and the change is sent for the save.
    #[test]
    fn an_unbound_weapon_takes_its_first_key_from_a_rebind() {
        let (mut world, gun) = rebind_world();
        let ship = world.get::<ChildOf>(gun).unwrap().parent();
        let added = world
            .spawn((
                ChildOf(ship),
                EntityId("weapon_0+".to_string()),
                SectionClass::Turret,
                SpaceshipTurretInputBinding(Vec::new()),
            ))
            .id();
        let unmapped = world
            .spawn((
                ChildOf(ship),
                EntityId("lance".to_string()),
                SectionClass::Railgun,
            ))
            .id();

        for (target, key) in [(added, KeyCode::KeyK), (unmapped, KeyCode::KeyL)] {
            world.resource_mut::<ShipRuntime>().rebinding = Some(target);
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            keys.clear();
            keys.press(key);
            world.run_system_once(apply_ship_rebind).unwrap();
            assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        }

        assert_eq!(
            world.get::<SpaceshipTurretInputBinding>(added).unwrap().0,
            vec![InputSource::Keyboard(KeyCode::KeyK)]
        );
        assert_eq!(
            world
                .get::<SpaceshipRailgunInputBinding>(unmapped)
                .unwrap()
                .0,
            vec![InputSource::Keyboard(KeyCode::KeyL)]
        );
        let sent: Vec<String> = world
            .resource::<Messages<SectionInputBindingChanged>>()
            .iter_current_update_messages()
            .map(|change| change.section_id.clone())
            .collect();
        assert_eq!(sent, ["weapon_0+", "lance"]);
    }

    /// A thruster the ship never mapped fires on the Main Drive. A rebind
    /// gives it a key of its own, a Main Drive key stays refused, and
    /// Backspace takes the own key away, so the Main Drive fires it again.
    #[test]
    fn a_thruster_takes_its_own_key_and_backspace_puts_it_back_on_the_main_drive() {
        let (mut world, gun) = rebind_world();
        let ship = world.get::<ChildOf>(gun).unwrap().parent();
        let thruster = world
            .spawn((
                ChildOf(ship),
                EntityId("engine".to_string()),
                SectionClass::Thruster,
            ))
            .id();
        let press = |world: &mut World, key: KeyCode| {
            world.resource_mut::<ShipRuntime>().rebinding = Some(thruster);
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            keys.clear();
            keys.press(key);
            world.run_system_once(apply_ship_rebind).unwrap();
        };

        press(&mut world, KeyCode::KeyJ);
        assert_eq!(
            world
                .get::<SpaceshipThrusterInputBinding>(thruster)
                .unwrap()
                .0,
            vec![InputSource::Keyboard(KeyCode::KeyJ)]
        );

        press(&mut world, KeyCode::Space);
        assert_eq!(
            world
                .get::<SpaceshipThrusterInputBinding>(thruster)
                .unwrap()
                .0,
            vec![InputSource::Keyboard(KeyCode::KeyJ)],
            "Space stays the Main Drive's"
        );
        assert!(world.resource::<ShipRuntime>().rebinding.is_some());

        press(&mut world, KeyCode::Backspace);
        assert!(world
            .get::<SpaceshipThrusterInputBinding>(thruster)
            .is_none());
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        let sent: Vec<(String, Vec<InputSource>)> = world
            .resource::<Messages<SectionInputBindingChanged>>()
            .iter_current_update_messages()
            .map(|change| (change.section_id.clone(), change.bindings.clone()))
            .collect();
        assert_eq!(
            sent,
            [
                (
                    "engine".to_string(),
                    vec![InputSource::Keyboard(KeyCode::KeyJ)]
                ),
                ("engine".to_string(), Vec::new()),
            ]
        );
    }

    /// Press `button` on a connected pad and run the capture.
    fn press_pad(world: &mut World, button: GamepadButton) {
        let mut pad = Gamepad::default();
        pad.digital_mut().press(button);
        world.spawn(pad);
        world.run_system_once(apply_ship_rebind).unwrap();
    }

    /// A pad button pressed during a rebind sets the section's pad half and
    /// keeps its key.
    #[test]
    fn a_pad_button_sets_the_pad_half_and_keeps_the_key() {
        let (mut world, gun) = rebind_world();
        let ship = world.get::<ChildOf>(gun).unwrap().parent();
        let thruster = world
            .spawn((
                ChildOf(ship),
                EntityId("engine".to_string()),
                SectionClass::Thruster,
                SpaceshipThrusterInputBinding(vec![KeyCode::KeyJ.into()]),
            ))
            .id();
        world.resource_mut::<ShipRuntime>().rebinding = Some(thruster);

        press_pad(&mut world, GamepadButton::RightTrigger2);

        let binding = vec![
            InputSource::Keyboard(KeyCode::KeyJ),
            InputSource::Gamepad(GamepadButton::RightTrigger2),
        ];
        assert_eq!(
            world
                .get::<SpaceshipThrusterInputBinding>(thruster)
                .unwrap()
                .0,
            binding
        );
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        let sent: Vec<Vec<InputSource>> = world
            .resource::<Messages<SectionInputBindingChanged>>()
            .iter_current_update_messages()
            .map(|change| change.bindings.clone())
            .collect();
        assert_eq!(sent, [binding]);
    }

    /// A pad input the flight rig holds is refused like a reserved key: the
    /// Main Drive's Right Trigger cannot become a thruster's own button.
    #[test]
    fn a_reserved_pad_input_is_refused() {
        let (mut world, gun) = rebind_world();
        let ship = world.get::<ChildOf>(gun).unwrap().parent();
        let thruster = world
            .spawn((
                ChildOf(ship),
                EntityId("engine".to_string()),
                SectionClass::Thruster,
            ))
            .id();
        world.resource_mut::<ShipRuntime>().rebinding = Some(thruster);

        press_pad(&mut world, GamepadButton::RightTrigger);

        assert!(world
            .get::<SpaceshipThrusterInputBinding>(thruster)
            .is_none());
        let runtime = world.resource::<ShipRuntime>();
        assert_eq!(runtime.rebinding, Some(thruster));
        assert_eq!(
            runtime.note.as_ref().map(|(note, _)| note.as_str()),
            Some("Right Trigger is already bound to flight control: Main Drive")
        );
    }
}
