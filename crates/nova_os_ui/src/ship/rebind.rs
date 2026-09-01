use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_input::prelude::{ActionContext, InputBindings, InputSource, InputSources};
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
        &ChildOf,
        &EntityId,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
        Option<&SpaceshipRailgunInputBinding>,
    )>,
    mut changed: MessageWriter<SectionInputBindingChanged>,
) {
    let Some(target) = runtime.rebinding else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        runtime.rebinding = None;
        runtime.rebind_awaiting_release = false;
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
    // The shared capture, not a hand-rolled one: `ButtonInput` iterates a
    // `HashSet`, so picking the FIRST just-pressed key bound either of two keys
    // pressed on one frame, run to run.
    let Some(source) = sources.captured_desk() else {
        return;
    };
    // The pointer's own button is never taken, for the reason the settings
    // screen refuses it: this panel is driven ENTIRELY by clicks, so an armed
    // capture would otherwise eat the next click on a blip - and bind the
    // section to the button every shipped turret already fires on.
    if source == InputSource::Mouse(MouseButton::Left) {
        runtime.note = Some(("Left Mouse stays the pointer".to_string(), 2.5));
        return;
    }
    let Ok((parent, id, thruster, turret, torpedo, railgun)) = targets.get(target) else {
        runtime.rebinding = None;
        return;
    };
    let ship = parent.parent();
    if let Some(conflict) = reserved_conflict(&bindings, source) {
        runtime.note = Some((
            format!("{} is already used by {conflict}", source.readout_label()),
            2.5,
        ));
        return;
    }

    let bindings = vec![source];
    if thruster.is_some() {
        commands
            .entity(target)
            .insert(SpaceshipThrusterInputBinding(bindings.clone()));
    } else if turret.is_some() {
        commands
            .entity(target)
            .insert(SpaceshipTurretInputBinding(bindings.clone()));
    } else if torpedo.is_some() {
        commands
            .entity(target)
            .insert(SpaceshipTorpedoInputBinding(bindings.clone()));
    } else if railgun.is_some() {
        commands
            .entity(target)
            .insert(SpaceshipRailgunInputBinding(bindings.clone()));
    } else {
        runtime.rebinding = None;
        return;
    }
    changed.write(SectionInputBindingChanged {
        spaceship: ship,
        section: target,
        section_id: id.0.clone(),
        bindings,
    });
    runtime.rebinding = None;
    runtime.note = Some((format!("Bound {} to {}", id.0, source.readout_label()), 2.5));
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::ship::SectionCode;

    fn rebind_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<ShipRuntime>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Messages<SectionInputBindingChanged>>();
        // The refusal reads the live table, so a rig with none would fail open.
        world.insert_resource(InputBindings::from_actions(flight_bindings()));
        let ship = world.spawn_empty().id();
        let target = world
            .spawn((
                ChildOf(ship),
                EntityId("gun".to_string()),
                SectionCode("PDC-1".to_string()),
                SpaceshipTurretInputBinding(vec![KeyCode::KeyF.into()]),
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
            vec![InputSource::Keyboard(KeyCode::KeyF)],
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

    #[test]
    fn successful_rebind_replaces_the_complete_list_and_emits_the_change() {
        let (mut world, target) = rebind_world();
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyK);

        world.run_system_once(apply_ship_rebind).unwrap();

        let binding = &world.get::<SpaceshipTurretInputBinding>(target).unwrap().0;
        assert_eq!(binding.len(), 1);
        assert_eq!(binding[0], InputSource::Keyboard(KeyCode::KeyK));
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        let messages = world.resource::<Messages<SectionInputBindingChanged>>();
        assert_eq!(messages.len(), 1);
    }
}
