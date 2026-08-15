//! Combat-death (neutralized) detection: a ship that was an armed combatant
//! is "out of the fight" when it loses ALL working weapons OR (if it ever had
//! one) its last working flight computer - a brain-dead ship cannot aim or
//! fly, so live guns on a computer-less hulk do not keep it in the fight, and
//! thrusters play no part in the rule (a disarmed ship that can still run is
//! beaten, not fighting). Unlike destruction (see [`explode`] and [`glue`]),
//! a neutralized ship is NOT despawned - it lingers as a powerless drifting
//! wreck. This module inserts [`NeutralizedMarker`], switches an AI
//! [`NeutralizedMarker`] and fires the distinct [`OnNeutralizedEvent`] so
//! scenarios can treat it as beaten.
//!
//! It does NOT know about AI. Taking a neutralized enemy out of combat is the
//! AI's own reaction to [`NeutralizedMarker`] landing (`input::ai`), which is
//! what keeps this module free of the AI vocabulary.
//!
//! The "was armed" guard ([`WasArmedCombatant`]) is what keeps this honest: an
//! unarmed hull losing its engines is not out of a fight it was never in, and
//! the guard also avoids a false neutralize during the frames after spawn before
//! a ship's sections have attached to its root.
//!
//! [`explode`]: super::explode
//! [`glue`]: super::glue

use bevy::prelude::*;
use nova_events::prelude::{CommandsGameEventExt, *};

use super::core::prelude::*;
use crate::prelude::{
    ControllerSectionMarker, SectionInactiveMarker, SpaceshipRootMarker, TorpedoSectionMarker,
    TurretSectionMarker,
};

/// Defeat and neutralization state markers.
pub mod prelude {
    pub use super::{DefeatedMarker, HadFlightComputer, NeutralizedMarker, WasArmedCombatant};
}

/// Marks a ship that has already crossed the unified scenario defeat edge.
/// Persists on a neutralized wreck so later physical destruction cannot fire
/// `OnDefeated` twice.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct DefeatedMarker;

/// Marks a ship root that has been NEUTRALIZED - it was an armed combatant
/// and now has zero working weapon sections, or lost the flight computer it
/// once had (no brain: nothing aims or flies the ship). The ship stays in the
/// world (a drifting wreck); this marker is inserted once and never removed.
/// Its presence gates the detection system so a ship is only neutralized
/// once.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct NeutralizedMarker;

/// Internal guard: stamped on a ship root the first time it is seen carrying at
/// least one weapon (turret/torpedo) section. Only a `WasArmedCombatant` root
/// can be neutralized, so an unarmed ship (which never had a weapon) is never
/// "out of the fight" - it can only be destroyed - and a freshly spawned ship
/// whose sections have not yet attached is not neutralized in that window.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct WasArmedCombatant;

/// Internal guard, the controller-side sibling of [`WasArmedCombatant`]:
/// stamped on a ship root the first time it is seen carrying a flight
/// computer section. Only a ship that HAD a computer can be neutralized by
/// losing it - a computer-less emplacement (a scripted battery) is not
/// brain-dead, it never had a brain. Destroyed leaf sections despawn, so
/// per-frame presence cannot stand in for history.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct HadFlightComputer;

pub(super) struct NeutralizePlugin;

impl Plugin for NeutralizePlugin {
    fn build(&self, app: &mut App) {
        debug!("NeutralizePlugin: build");

        // Ordered after the integrity systems so this frame's aggregate/leaf
        // work is done first. Note `SectionInactiveMarker` is actually written
        // by the `on_section_disable` OBSERVER (glue.rs), not by a system in
        // `IntegritySystems`, so this ordering does NOT guarantee same-frame
        // detection - a section disabled this frame may only be counted next
        // frame. That is fine and deliberate: an absent-or-not-yet-inactive
        // section counts as "working", so the worst case is detecting a
        // neutralize one frame LATE, never a false neutralize. Do not tighten
        // this expecting same-frame semantics.
        app.add_systems(Update, detect_neutralized.after(IntegritySystems));
    }
}

/// Per-frame predicate: for every armed ship root not already neutralized,
/// count its working weapon and flight-computer sections; no working weapon -
/// or a lost computer the ship once had - neutralizes it. Thrusters play no
/// part (owner direction, 2026-08-14): a disarmed runner is beaten, and a
/// computer-less hulk with live guns cannot aim them.
fn detect_neutralized(
    mut commands: Commands,
    q_root: Query<
        (
            Entity,
            &Children,
            Option<&EntityId>,
            Option<&EntityTypeName>,
            Has<WasArmedCombatant>,
            Has<HadFlightComputer>,
        ),
        (With<SpaceshipRootMarker>, Without<NeutralizedMarker>),
    >,
    // A weapon section: turret or torpedo. `Has<SectionInactiveMarker>` reports
    // whether it is disabled (destroyed non-leaf); a destroyed leaf section is
    // despawned and so is simply absent from the root's children.
    q_weapon: Query<
        Has<SectionInactiveMarker>,
        Or<(With<TurretSectionMarker>, With<TorpedoSectionMarker>)>,
    >,
    q_controller: Query<Has<SectionInactiveMarker>, With<ControllerSectionMarker>>,
) {
    for (root, children, id, type_name, was_armed, had_computer) in &q_root {
        let mut has_weapon_section = false;
        let mut working_weapon = false;
        let mut has_controller_section = false;
        let mut working_controller = false;

        for child in children.iter() {
            if let Ok(inactive) = q_weapon.get(child) {
                has_weapon_section = true;
                working_weapon |= !inactive;
            }
            if let Ok(inactive) = q_controller.get(child) {
                has_controller_section = true;
                working_controller |= !inactive;
            }
        }

        // History stamps. The computer stamp lands even on a not-yet-armed
        // hull: history is history, whichever section attaches first.
        if !had_computer && has_controller_section {
            commands.entity(root).insert(HadFlightComputer);
        }
        // Arming guard: a root only becomes eligible once it has carried a
        // weapon section. Stamp it the first frame we see one, and never
        // neutralize on that same frame (nor for a ship that was never armed).
        if !was_armed {
            if has_weapon_section {
                commands.entity(root).insert(WasArmedCombatant);
            }
            continue;
        }

        let disarmed = !working_weapon;
        let brain_dead = had_computer && !working_controller;
        if !disarmed && !brain_dead {
            continue;
        }

        // Combat-dead: disarmed, or brain-dead. Stamp the unified edge before
        // the persistent wreck state.
        // The integrity root can be destroyed later in this same command flush.
        // A stale neutralization reaction must not turn that valid race into a panic.
        commands
            .entity(root)
            .try_insert((DefeatedMarker, NeutralizedMarker));

        // Fire the scenario-facing signal, mirroring the destroy path's use of
        // the ship's scenario id/type name.
        if let (Some(id), Some(type_name)) = (id, type_name) {
            debug!(
                "detect_neutralized: entity {:?} neutralized (id: {:?}, type: {:?})",
                root, id, type_name
            );
            let defeated = OnDefeatedEventInfo {
                id: id.to_string(),
                type_name: type_name.to_string(),
            };
            commands.fire::<OnDefeatedEvent>(defeated.clone());
            commands.fire::<OnNeutralizedEvent>(OnNeutralizedEventInfo {
                id: defeated.id,
                type_name: defeated.type_name,
            });
        } else {
            // A shipped scenario ship always carries both, so this is a
            // mis-spawned ship: mark it neutralized but leave a trace so the
            // silently-un-neutralized-at-the-scenario-layer case is diagnosable.
            debug!(
                "detect_neutralized: entity {:?} neutralized but has no EntityId/EntityTypeName - \
                 no OnNeutralizedEvent fired",
                root
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::GameEvent;

    use super::*;
    use crate::{
        integrity::health::prelude::Health,
        prelude::{SectionMarker, ThrusterSectionMarker},
        test_support::unfinished_integrity_physics_app,
    };

    /// Records lifecycle event names in dispatch order.
    #[derive(Resource, Default)]
    struct FiredEvents(Vec<&'static str>);

    fn neutralize_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FiredEvents>();
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<FiredEvents>| {
            fired.0.push(event.name());
        });
        app.finish();
        app
    }

    fn fired(app: &App) -> &[&'static str] {
        &app.world().resource::<FiredEvents>().0
    }

    /// Spawn a ship root with `weapons` turret sections, `thrusters` thruster
    /// sections, `controllers` flight-computer sections and one always-intact
    /// hull section. Returns (root, weapons, thrusters, controllers, hull).
    fn spawn_ship(
        app: &mut App,
        weapons: usize,
        thrusters: usize,
        controllers: usize,
    ) -> (Entity, Vec<Entity>, Vec<Entity>, Vec<Entity>, Entity) {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("rig_ship"),
                EntityTypeName::new("spaceship"),
            ))
            .id();
        let weapon_ents = (0..weapons)
            .map(|_| {
                app.world_mut()
                    .spawn((ChildOf(root), SectionMarker, TurretSectionMarker))
                    .id()
            })
            .collect();
        let thruster_ents = (0..thrusters)
            .map(|_| {
                app.world_mut()
                    .spawn((ChildOf(root), SectionMarker, ThrusterSectionMarker))
                    .id()
            })
            .collect();
        let controller_ents = (0..controllers)
            .map(|_| {
                app.world_mut()
                    .spawn((ChildOf(root), SectionMarker, ControllerSectionMarker))
                    .id()
            })
            .collect();
        // An intact hull section that is never touched, standing in for "hull
        // health notwithstanding".
        let hull = app
            .world_mut()
            .spawn((ChildOf(root), SectionMarker, Health::new(100.0)))
            .id();
        (root, weapon_ents, thruster_ents, controller_ents, hull)
    }

    /// Disable a section the way the integrity glue does for a destroyed
    /// non-leaf section, without touching hull health.
    fn disable(app: &mut App, section: Entity) {
        app.world_mut()
            .entity_mut(section)
            .insert(SectionInactiveMarker);
    }

    fn is_neutralized(app: &App, root: Entity) -> bool {
        app.world().entity(root).contains::<NeutralizedMarker>()
    }

    #[test]
    fn armed_ship_losing_all_weapons_is_neutralized() {
        let mut app = neutralize_app();
        let (root, weapons, _thrusters, _controllers, hull) = spawn_ship(&mut app, 1, 1, 1);

        // Frame with working weapon: armed-stamp only, not neutral.
        app.update();
        assert!(
            !is_neutralized(&app, root),
            "must not neutralize while armed"
        );
        assert!(fired(&app).is_empty(), "no event while still able to fight");

        // Lose the weapon ONLY - the thruster and the computer keep working.
        // Thrusters play no part in the rule: a disarmed ship that can still
        // run is beaten, not fighting.
        disable(&mut app, weapons[0]);
        app.update();

        assert!(
            is_neutralized(&app, root),
            "all weapons gone => neutralized, working thrusters notwithstanding"
        );
        assert!(
            app.world().entities().contains(root),
            "a neutralized ship is NOT despawned - it lingers as a wreck"
        );
        assert!(
            app.world().entity(hull).get::<Health>().unwrap().current > 0.0,
            "hull health is untouched by neutralization"
        );
        assert_eq!(
            fired(&app),
            [OnDefeatedEvent::name(), OnNeutralizedEvent::name()],
            "unified defeat precedes the detailed neutralization edge"
        );

        // It stays neutralized and neither edge re-fires on later frames.
        app.update();
        app.update();
        assert_eq!(fired(&app).len(), 2, "neutralization does not re-fire");
    }

    #[test]
    fn unarmed_ship_losing_everything_is_not_neutralized() {
        let mut app = neutralize_app();
        // No weapon sections: an unarmed hull with a thruster and a computer.
        let (root, _weapons, thrusters, controllers, _hull) = spawn_ship(&mut app, 0, 1, 1);
        app.update();

        // Stimulus: kill the thruster AND the computer (asserted applied).
        disable(&mut app, thrusters[0]);
        disable(&mut app, controllers[0]);
        app.update();
        assert!(
            app.world()
                .entity(controllers[0])
                .contains::<SectionInactiveMarker>(),
            "the computer-kill stimulus really fired"
        );

        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "a ship that was never armed is never neutralized, only destroyed"
        );
        assert!(
            fired(&app).is_empty(),
            "no defeat event for an unarmed ship"
        );
    }

    /// The brain-death half of the rule (owner direction, 2026-08-14): an
    /// armed ship that loses the flight computer it HAD is out of the fight
    /// even with every gun and thruster working - nothing aims or flies it.
    /// Live motivation: a duel cripple whose computer died by integrity
    /// disconnection drifted un-defeated for 11 minutes.
    #[test]
    fn armed_ship_losing_its_computer_is_neutralized() {
        let mut app = neutralize_app();
        let (root, _weapons, _thrusters, controllers, _hull) = spawn_ship(&mut app, 1, 1, 1);
        app.update();
        assert!(!is_neutralized(&app, root));

        disable(&mut app, controllers[0]);
        app.update();

        assert!(
            is_neutralized(&app, root),
            "computer gone => neutralized, working guns notwithstanding"
        );
        assert_eq!(
            fired(&app),
            [OnDefeatedEvent::name(), OnNeutralizedEvent::name()],
        );
    }

    /// The rule is "no working computer LEFT", not "a computer died": a hull
    /// that stacks flight computers for the handling (see the controller
    /// section's stack curve) trades one for redundancy, so killing one of
    /// two degrades its steering and leaves it in the fight. Only the last
    /// one is brain death.
    #[test]
    fn an_armed_ship_with_a_spare_computer_survives_losing_one() {
        let mut app = neutralize_app();
        let (root, _weapons, _thrusters, controllers, _hull) = spawn_ship(&mut app, 1, 1, 2);
        app.update();

        disable(&mut app, controllers[0]);
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "a spare computer keeps the ship in the fight"
        );
        assert!(fired(&app).is_empty(), "and fires no defeat edge");

        disable(&mut app, controllers[1]);
        app.update();
        assert!(
            is_neutralized(&app, root),
            "the LAST computer dying is what is brain death"
        );
    }

    /// A ship that NEVER had a computer (a scripted battery emplacement) is
    /// not brain-dead - only the disarm half can neutralize it.
    #[test]
    fn a_computer_less_emplacement_only_neutralizes_by_disarm() {
        let mut app = neutralize_app();
        let (root, weapons, _thrusters, _controllers, _hull) = spawn_ship(&mut app, 1, 0, 0);
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "no computer to lose - the battery is not brain-dead"
        );

        disable(&mut app, weapons[0]);
        app.update();
        assert!(
            is_neutralized(&app, root),
            "its gun dying is what takes an emplacement out of the fight"
        );
    }
}
