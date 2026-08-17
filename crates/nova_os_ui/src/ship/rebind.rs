use bevy::prelude::*;
use bevy_enhanced_input::prelude::Binding;
use nova_events::prelude::EntityId;
use nova_ship::prelude::*;

use super::{SectionCode, ShipRuntime};

fn section_bindings<'a>(
    thruster: Option<&'a SpaceshipThrusterInputBinding>,
    turret: Option<&'a SpaceshipTurretInputBinding>,
    torpedo: Option<&'a SpaceshipTorpedoInputBinding>,
) -> Option<&'a [Binding]> {
    thruster
        .map(|bindings| bindings.0.as_slice())
        .or_else(|| turret.map(|bindings| bindings.0.as_slice()))
        .or_else(|| torpedo.map(|bindings| bindings.0.as_slice()))
}

fn conflict_for(
    source: InputSource,
    target: Entity,
    ship: Entity,
    sections: &Query<(
        Entity,
        &ChildOf,
        Option<&SectionCode>,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
    )>,
) -> Option<String> {
    if let Some((_, verb)) = flight_rig_reserved_sources()
        .into_iter()
        .find(|(reserved, _)| *reserved == source)
    {
        return Some(format!("flight control: {verb}"));
    }
    sections
        .iter()
        .filter(|(entity, parent, ..)| *entity != target && parent.parent() == ship)
        .find_map(|(_, _, code, thruster, turret, torpedo)| {
            section_bindings(thruster, turret, torpedo)?
                .iter()
                .filter_map(binding_source)
                .any(|occupied| occupied == source)
                .then(|| {
                    code.map(|code| code.0.clone())
                        .unwrap_or_else(|| "another section".to_string())
                })
        })
}

#[allow(clippy::type_complexity)]
pub(crate) fn apply_ship_rebind(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut runtime: ResMut<ShipRuntime>,
    mut commands: Commands,
    targets: Query<(
        &ChildOf,
        &EntityId,
        Option<&SpaceshipThrusterInputBinding>,
        Option<&SpaceshipTurretInputBinding>,
        Option<&SpaceshipTorpedoInputBinding>,
    )>,
    sections: Query<(
        Entity,
        &ChildOf,
        Option<&SectionCode>,
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
    let binding = keys
        .get_just_pressed()
        .find(|key| **key != KeyCode::Escape)
        .map(|key| Binding::from(*key))
        .or_else(|| {
            mouse
                .get_just_pressed()
                .next()
                .map(|button| Binding::from(*button))
        });
    let Some(binding) = binding else {
        return;
    };
    let Some(source) = binding_source(&binding) else {
        return;
    };
    let Ok((parent, id, thruster, turret, torpedo)) = targets.get(target) else {
        runtime.rebinding = None;
        return;
    };
    let ship = parent.parent();
    if let Some(conflict) = conflict_for(source, target, ship, &sections) {
        runtime.note = Some((
            format!("{} is already used by {conflict}", source.label()),
            2.5,
        ));
        return;
    }

    let bindings = vec![binding];
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

    fn rebind_world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<ShipRuntime>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Messages<SectionInputBindingChanged>>();
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
    fn another_section_on_the_same_ship_is_a_conflict() {
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

        let runtime = world.resource::<ShipRuntime>();
        assert_eq!(runtime.rebinding, Some(target));
        assert!(runtime
            .note
            .as_ref()
            .is_some_and(|(note, _)| note.contains("TRB-1")));
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
        assert_eq!(
            binding_source(&binding[0]),
            Some(InputSource::Keyboard(KeyCode::KeyK))
        );
        assert!(world.resource::<ShipRuntime>().rebinding.is_none());
        let messages = world.resource::<Messages<SectionInputBindingChanged>>();
        assert_eq!(messages.len(), 1);
    }
}
