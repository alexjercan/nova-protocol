use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_input::prelude::{ActionContext, InputBindings, InputSource};
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<InputBindings>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    targets: Query<(
        &ChildOf,
        &EntityId,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
    )>,
    mut changed: MessageWriter<SectionInputBindingChanged>,
) {
    let Some(target) = runtime.rebinding else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        runtime.rebinding = None;
        runtime.rebind_just_armed = false;
        runtime.note = Some(("Rebind cancelled".to_string(), 2.5));
        return;
    }
    if runtime.rebind_just_armed {
        runtime.rebind_just_armed = false;
        return;
    }
    let source = keys
        .get_just_pressed()
        .find(|key| **key != KeyCode::Escape)
        .map(|key| InputSource::Keyboard(*key))
        .or_else(|| {
            mouse
                .get_just_pressed()
                .next()
                .map(|button| InputSource::Mouse(*button))
        });
    let Some(source) = source else {
        return;
    };
    let Ok((parent, id, thruster, turret, torpedo)) = targets.get(target) else {
        runtime.rebinding = None;
        return;
    };
    let ship = parent.parent();
    if let Some(conflict) = reserved_conflict(&bindings, source) {
        runtime.note = Some((
            format!("{} is already used by {conflict}", source.label()),
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
    runtime.note = Some((format!("Bound {} to {}", id.0, source.label()), 2.5));
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
            SpaceshipTorpedoInputBinding(vec![MouseButton::Left.into()]),
        ));
        {
            let mut runtime = world.resource_mut::<ShipRuntime>();
            runtime.rebinding = Some(target);
        }
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        world.run_system_once(apply_ship_rebind).unwrap();

        let target_bindings = &world.get::<SpaceshipTurretInputBinding>(target).unwrap().0;
        assert_eq!(target_bindings[0], InputSource::Mouse(MouseButton::Left));
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
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
