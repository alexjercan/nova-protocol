//! Weapons safety: HOT while the stance is raised or a combat lock is held,
//! SAFE otherwise, with held triggers zeroed the moment safety re-engages.

use bevy::prelude::*;

use crate::prelude::*;

/// Derive the weapons safety on every managed ship: HOT while raised or
/// combat-locked, SAFE otherwise. Generic over player and AI (the AI mirror
/// maintains its CombatLock/WeaponsRaised; a ship without WeaponsHot is
/// unmanaged and unaffected).
pub(super) fn update_weapons_safety(
    mut q_ships: Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    update_weapons_safety_impl(&mut q_ships);
}

/// Test-visible alias (the AI mirror test drives the REAL derivation).
#[cfg(test)]
pub(crate) fn update_weapons_safety_for_tests(
    mut q_ships: Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    update_weapons_safety_impl(&mut q_ships);
}

fn update_weapons_safety_impl(
    q_ships: &mut Query<(Option<&WeaponsRaised>, &CombatLock, &mut WeaponsHot)>,
) {
    for (raised, lock, mut hot) in q_ships.iter_mut() {
        let next = raised.is_some_and(|raised| raised.0) || lock.0.is_some();
        hot.set_if_neq(WeaponsHot(next));
    }
}

/// The trigger-interrupt (adversarial finding: the fire inputs are LATCHED
/// bools, so a press-time gate alone cannot stop a held burst): the frame the
/// safety engages, zero every weapon input on that ship - resuming fire once
/// hot again requires a fresh press. The section fire systems ALSO check
/// [`WeaponsHot`] live, so even a same-frame race cannot leak a shot.
pub(super) fn enforce_safety_trigger_interrupt(
    q_ships: Query<(Entity, &WeaponsHot), Changed<WeaponsHot>>,
    mut q_turrets: Query<(&mut TurretSectionInput, &ChildOf)>,
    mut q_torpedoes: Query<(&mut TorpedoSectionInput, &ChildOf), Without<TurretSectionInput>>,
) {
    for (ship, hot) in &q_ships {
        if hot.0 {
            continue;
        }
        for (mut input, ChildOf(parent)) in &mut q_turrets {
            if *parent == ship && **input {
                **input = false;
            }
        }
        for (mut input, ChildOf(parent)) in &mut q_torpedoes {
            if *parent == ship && **input {
                **input = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn weapons_safety_derives_from_stance_and_lock() {
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let ship = world.spawn((CombatLock(None), WeaponsHot::default())).id();

        world.run_system_once(update_weapons_safety).unwrap();
        assert!(
            !world.get::<WeaponsHot>(ship).unwrap().0,
            "lowered + no lock = safe"
        );

        // Raised alone: hot.
        world.entity_mut(ship).insert(WeaponsRaised(true));
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(world.get::<WeaponsHot>(ship).unwrap().0, "raised = hot");

        // Lowered but combat-locked: still hot.
        world.entity_mut(ship).insert(WeaponsRaised(false));
        world.get_mut::<CombatLock>(ship).unwrap().0 = Some(target);
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(
            world.get::<WeaponsHot>(ship).unwrap().0,
            "a combat lock keeps the guns hot"
        );

        // Neither: safe again.
        world.get_mut::<CombatLock>(ship).unwrap().0 = None;
        world.run_system_once(update_weapons_safety).unwrap();
        assert!(!world.get::<WeaponsHot>(ship).unwrap().0);
    }

    /// The trigger-interrupt: the frame the safety engages, held weapon
    /// inputs are zeroed (a latched bool would otherwise keep firing).
    /// Registered system: `Changed<WeaponsHot>` needs real tick history.
    #[test]
    fn safety_engage_zeroes_held_triggers() {
        let mut world = World::new();
        let ship = world.spawn(WeaponsHot(true)).id();
        let turret = world.spawn((TurretSectionInput(true), ChildOf(ship))).id();
        let torpedo = world.spawn((TorpedoSectionInput(true), ChildOf(ship))).id();
        let enforce = world.register_system(enforce_safety_trigger_interrupt);

        // Delivery guard: while hot, held triggers survive the pass.
        world.run_system(enforce).unwrap();
        assert!(**world.get::<TurretSectionInput>(turret).unwrap());
        assert!(**world.get::<TorpedoSectionInput>(torpedo).unwrap());

        // The safety engages: both inputs zero the same frame.
        world.get_mut::<WeaponsHot>(ship).unwrap().0 = false;
        world.run_system(enforce).unwrap();
        assert!(
            !**world.get::<TurretSectionInput>(turret).unwrap(),
            "a held turret trigger is interrupted when the safety engages"
        );
        assert!(
            !**world.get::<TorpedoSectionInput>(torpedo).unwrap(),
            "a held torpedo trigger is interrupted too"
        );
    }
}
