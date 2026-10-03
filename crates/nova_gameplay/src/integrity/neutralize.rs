//! Combat-death (neutralized) detection covers two independent rules. An
//! ARMED ship (one that was ever a combatant) is "out of the fight" when it
//! loses ALL working weapons OR (if it ever had one) its last working flight
//! computer - a brain-dead ship cannot aim or fly, so live guns on a
//! computer-less hulk do not keep it in the fight. Thrusters play no part in
//! the armed rule (a disarmed ship that can still run is beaten, not
//! fighting). An UNARMED ship (of any allegiance) is instead neutralized when
//! it loses the last working thruster it ever had - a hull that never had a
//! thruster is never neutralized by this rule, only destroyed. Unlike
//! destruction (see [`explode`] and [`glue`]), a neutralized ship is NOT
//! despawned - it lingers as a powerless drifting wreck. This module inserts
//! [`NeutralizedMarker`], switches an AI [`NeutralizedMarker`] and fires the
//! distinct [`OnNeutralizedEvent`] so scenarios can treat it as beaten.
//!
//! It does NOT know about AI. Taking a neutralized enemy out of combat is the
//! AI's own reaction to [`NeutralizedMarker`] landing (`input::ai`), which is
//! what keeps this module free of the AI vocabulary.
//!
//! The "was armed" guard ([`WasArmedCombatant`]) and its thruster-side
//! sibling ([`HadThruster`]) are what keep this honest: a ship losing
//! capability it never had is not out of a fight it was never in, and both
//! guards also avoid a false neutralize during the frames after spawn before
//! a ship's sections have attached to its root.
//!
//! [`explode`]: super::explode
//! [`glue`]: super::glue

use bevy::prelude::*;
use nova_events::prelude::{CommandsGameEventExt, *};

use super::core::prelude::*;
use crate::prelude::{
    ControllerSectionMarker, DerelictShipMarker, RailgunSectionMarker, SectionInactiveMarker,
    SpaceshipRootMarker, ThrusterSectionMarker, TorpedoSectionMarker, TurretSectionMarker,
};

/// Defeat and neutralization state markers.
pub mod prelude {
    pub use super::{
        DefeatedMarker, HadFlightComputer, HadThruster, NeutralizedMarker, WasArmedCombatant,
    };
}

/// Marks a ship that has already crossed the unified scenario defeat edge.
/// Persists on a neutralized wreck so later physical destruction cannot fire
/// `OnDefeated` twice.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct DefeatedMarker;

/// Marks a ship root that has been NEUTRALIZED - either it was an armed
/// combatant and now has zero working weapon sections, or lost the flight
/// computer it once had (no brain: nothing aims or flies the ship); or it was
/// an unarmed hull and lost the last working thruster it once had. The ship
/// stays in the world (a drifting wreck); this marker is inserted once and
/// never removed. Its presence gates the detection system so a ship is only
/// neutralized once.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct NeutralizedMarker;

/// Internal guard: stamped on a ship root the first time it is seen carrying at
/// least one weapon (turret/torpedo) section. A `WasArmedCombatant` root is
/// neutralized by the armed (weapon/computer) rule; an unarmed root instead
/// falls to the thruster rule gated by [`HadThruster`]. Stamping also avoids a
/// freshly spawned ship, whose sections have not yet attached, being
/// neutralized in that same-frame window.
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

/// Internal guard, the thruster-side history stamp: stamped on a ship root
/// the first frame it is seen carrying a [`ThrusterSectionMarker`] section.
/// Only an unarmed root that HAD a thruster can be neutralized by losing its
/// last working one - a thrusterless hull never had one to lose, so it is
/// only destroyed, never neutralized by this rule.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct HadThruster;

pub(super) struct NeutralizePlugin;

impl Plugin for NeutralizePlugin {
    fn build(&self, app: &mut App) {
        trace!("NeutralizePlugin: build");

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

/// Per-frame predicate, covering two independent rules. An armed ship root
/// (one with `WasArmedCombatant`): no working weapon left - or a lost
/// computer the ship once had - neutralizes it; thrusters play no part here.
/// An unarmed ship root (one that never carried a weapon): losing the last
/// working thruster it once had (`HadThruster`) neutralizes it instead; a
/// hull that never had a thruster is never neutralized by this rule. A
/// [`DerelictShipMarker`] root is skipped: its sections spawned inactive, and
/// a wreck was never in the fight.
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
            Has<HadThruster>,
        ),
        (
            With<SpaceshipRootMarker>,
            Without<NeutralizedMarker>,
            Without<DerelictShipMarker>,
        ),
    >,
    // A weapon section: turret, torpedo or railgun. `Has<SectionInactiveMarker>`
    // reports whether it is disabled (destroyed non-leaf); a destroyed leaf
    // section is despawned and so is simply absent from the root's children.
    // Every kind `spaceship.rs` counts as arming a ship belongs here, or a
    // hull keeping one of them is called beaten while it still shoots.
    q_weapon: Query<
        Has<SectionInactiveMarker>,
        Or<(
            With<TurretSectionMarker>,
            With<TorpedoSectionMarker>,
            With<RailgunSectionMarker>,
        )>,
    >,
    q_controller: Query<Has<SectionInactiveMarker>, With<ControllerSectionMarker>>,
    q_thruster: Query<Has<SectionInactiveMarker>, With<ThrusterSectionMarker>>,
) {
    for (root, children, id, type_name, was_armed, had_computer, had_thruster) in &q_root {
        let mut has_weapon_section = false;
        let mut working_weapon = false;
        let mut has_controller_section = false;
        let mut working_controller = false;
        let mut has_thruster_section = false;
        let mut working_thruster = false;

        for child in children.iter() {
            if let Ok(inactive) = q_weapon.get(child) {
                has_weapon_section = true;
                working_weapon |= !inactive;
            }
            if let Ok(inactive) = q_controller.get(child) {
                has_controller_section = true;
                working_controller |= !inactive;
            }
            if let Ok(inactive) = q_thruster.get(child) {
                has_thruster_section = true;
                working_thruster |= !inactive;
            }
        }

        // History stamps. Both land even on a not-yet-armed hull: history is
        // history, whichever section attaches first. try_insert, for the
        // reason the neutralization write below states: the root can be
        // despawned earlier in this same command flush, and a history stamp
        // that missed its entity is not worth a panic.
        if !had_computer && has_controller_section {
            commands.entity(root).try_insert(HadFlightComputer);
        }
        if !had_thruster && has_thruster_section {
            commands.entity(root).try_insert(HadThruster);
        }

        // Arming guard: a root only becomes eligible for the armed rule once
        // it has carried a weapon section. Stamp it the first frame we see
        // one, and never neutralize on that same frame.
        if !was_armed {
            if has_weapon_section {
                commands.entity(root).try_insert(WasArmedCombatant);
                continue;
            }
            // Confirmed unarmed this frame: the thruster rule applies
            // instead. `had_thruster` is this frame's EARLIER-frame read, so
            // a thruster seen inactive for the first time this frame does not
            // neutralize until the next frame - the same same-frame spawn
            // guard the computer stamp gives the armed rule.
            if had_thruster && !working_thruster {
                neutralize(&mut commands, root, id, type_name);
            }
            continue;
        }

        let disarmed = !working_weapon;
        let brain_dead = had_computer && !working_controller;
        if !disarmed && !brain_dead {
            continue;
        }

        neutralize(&mut commands, root, id, type_name);
    }
}

/// Stamps the unified defeat edge and the persistent wreck state, then fires
/// the scenario-facing signal, mirroring the destroy path's use of the ship's
/// scenario id/type name. Shared by both neutralization rules so neither
/// duplicates the event-firing code.
fn neutralize(
    commands: &mut Commands,
    root: Entity,
    id: Option<&EntityId>,
    type_name: Option<&EntityTypeName>,
) {
    // The integrity root can be destroyed later in this same command flush.
    // A stale neutralization reaction must not turn that valid race into a panic.
    commands
        .entity(root)
        .try_insert((DefeatedMarker, NeutralizedMarker));

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
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
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

    /// Every write this pass makes to a root is a `try_`, including the two
    /// history stamps - the arming stamp and the flight-computer stamp - that
    /// used to be plain inserts.
    ///
    /// The pass runs after `IntegritySystems`, and a destruction earlier in
    /// that same frame queues the root's despawn without applying it, so the
    /// stamp is written onto a root that will be gone by the time the buffers
    /// flush. `EntityCommands::insert` PANICS there, and a stamp that missed
    /// its entity costs nothing: a despawned root has no history to keep.
    #[test]
    fn a_root_despawned_in_the_same_flush_is_stamped_without_a_panic() {
        fn reap_every_root(
            mut commands: Commands,
            q_root: Query<Entity, With<SpaceshipRootMarker>>,
        ) {
            for root in &q_root {
                commands.entity(root).try_despawn();
            }
        }

        let mut app = neutralize_app();
        // The reaper runs first with the automatic sync points off, so its
        // despawn is still pending while the pass stamps the root - the same
        // pinning the audio teardown test uses.
        app.edit_schedule(Update, |schedule| {
            schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..default()
            });
        });
        app.add_systems(Update, (reap_every_root, detect_neutralized).chain());
        // A first-frame root: never stamped, so this update writes BOTH the
        // arming stamp and the flight-computer stamp.
        let (root, ..) = spawn_ship(&mut app, 1, 1, 1);

        app.update();

        assert!(
            !app.world().entities().contains(root),
            "the reaper is the owner that wins: the root is gone"
        );
    }

    #[test]
    fn a_ship_keeping_a_working_lance_is_not_yet_neutralized() {
        let mut app = neutralize_app();
        let (root, turrets, _thrusters, _controllers, _hull) = spawn_ship(&mut app, 1, 1, 1);
        let lance = app
            .world_mut()
            .spawn((ChildOf(root), SectionMarker, RailgunSectionMarker))
            .id();

        app.update();
        assert!(
            !is_neutralized(&app, root),
            "must not neutralize while armed"
        );

        // Every turret gone, the lance still working: a spinal gun is a weapon,
        // so the ship is still in the fight.
        disable(&mut app, turrets[0]);
        app.update();
        assert!(
            !is_neutralized(&app, root),
            "a hull that lost its turrets but keeps a working lance still shoots"
        );

        // The lance goes too: now there is nothing left to fire.
        disable(&mut app, lance);
        app.update();
        assert!(
            is_neutralized(&app, root),
            "losing the lance as well disarms the ship"
        );
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

    /// A derelict spawns with its weapons and flight computer already
    /// inactive. It was never in the fight, so it is never neutralized and
    /// fires no defeat edge, while a normal armed ship beside it that loses
    /// its weapon still is.
    #[test]
    fn a_derelict_with_inactive_weapons_is_never_neutralized() {
        let mut app = neutralize_app();
        let (derelict, weapons, thrusters, controllers, _hull) = spawn_ship(&mut app, 1, 1, 1);
        app.world_mut()
            .entity_mut(derelict)
            .insert(DerelictShipMarker);
        for section in weapons.into_iter().chain(thrusters).chain(controllers) {
            disable(&mut app, section);
        }
        let (armed, armed_weapons, ..) = spawn_ship(&mut app, 1, 1, 1);

        app.update();
        disable(&mut app, armed_weapons[0]);
        app.update();
        app.update();

        assert!(
            !is_neutralized(&app, derelict),
            "a derelict must not be neutralized"
        );
        assert!(
            !app.world().entity(derelict).contains::<DefeatedMarker>(),
            "a derelict must not be defeated"
        );
        assert!(
            is_neutralized(&app, armed),
            "an armed ship that loses its weapon must still be neutralized"
        );
        assert_eq!(
            fired(&app),
            [OnDefeatedEvent::name(), OnNeutralizedEvent::name()],
            "only the armed ship fires the defeat and neutralization edges"
        );
    }

    /// The thruster half of the rule: an
    /// unarmed ship of any allegiance is out of the fight when it loses the
    /// last working thruster it once had, even though a spare thruster keeps
    /// it running (the mirror of the computer stack curve test above).
    #[test]
    fn an_unarmed_ship_losing_its_last_thruster_is_neutralized() {
        let mut app = neutralize_app();
        // No weapon sections: an unarmed hull with two thrusters and a computer.
        let (root, _weapons, thrusters, _controllers, _hull) = spawn_ship(&mut app, 0, 2, 1);
        app.update();

        disable(&mut app, thrusters[0]);
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "a spare thruster keeps an unarmed ship running"
        );
        assert!(fired(&app).is_empty(), "and fires no defeat edge");

        disable(&mut app, thrusters[1]);
        app.update();
        assert!(
            is_neutralized(&app, root),
            "the LAST thruster dying takes an unarmed ship out of the fight"
        );
        assert_eq!(
            fired(&app),
            [OnDefeatedEvent::name(), OnNeutralizedEvent::name()],
            "unified defeat precedes the detailed neutralization edge"
        );
    }

    /// A ship that NEVER had a thruster (a stationary unarmed hull) is not
    /// neutralized by the thruster rule - only a ship that HAD one and lost
    /// it is.
    #[test]
    fn an_unarmed_ship_that_never_had_a_thruster_is_not_neutralized() {
        let mut app = neutralize_app();
        // No weapon and no thruster sections: a stationary unarmed hull.
        let (root, _weapons, _thrusters, controllers, _hull) = spawn_ship(&mut app, 0, 0, 1);
        app.update();

        disable(&mut app, controllers[0]);
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "no thruster to lose - a hull that never ran is never neutralized by this rule"
        );
        assert!(
            fired(&app).is_empty(),
            "no defeat event for an unarmed ship"
        );
    }

    /// Thrusters play no part in the armed rule: an armed ship keeping a
    /// working weapon and computer is not neutralized by losing every
    /// thruster, however it runs.
    #[test]
    fn an_armed_ship_losing_all_thrusters_is_not_neutralized() {
        let mut app = neutralize_app();
        let (root, _weapons, thrusters, _controllers, _hull) = spawn_ship(&mut app, 1, 2, 1);
        app.update();

        for thruster in thrusters {
            disable(&mut app, thruster);
        }
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !is_neutralized(&app, root),
            "thrusters play no part for an armed ship with a working weapon and computer"
        );
        assert!(fired(&app).is_empty(), "no defeat event");
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
