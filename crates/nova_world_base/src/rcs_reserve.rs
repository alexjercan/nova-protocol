//! RCS magazine catch-up on a thaw: the magazine's consumer of the play-time
//! clock.
//!
//! Nothing frozen simulates, so a root's [`RcsBudget`] stands still while its
//! cell is off the window. [`recover_thawed_rcs_budgets`] reads the
//! [`ThawedAfter`] stamp the thaw put on the body and catches the magazine up
//! by that play time, at `RcsBudget::recover`'s own rate. A magazine that
//! froze idle (no held command) catches up; one a command held at freeze
//! (`idle == 0.0`, mid-brake or mid-burn) gets nothing, the same as it would
//! have gotten holding that command live.
//!
//! `SpaceshipRootMarker` requires `RcsBudget`, so a thaw's spawn command
//! fills it in with the type's own default the instant the bundle lands,
//! before `nova_scenario`'s `insert_spaceship_sections` observer queues the
//! command that overwrites it with the record's spent and idle. `Added<T>`
//! reads the component's value the next time a query sees it, not each
//! insert, so by the time this system runs the entity shows the frozen
//! reading, not the transient default - the same guarantee the PDC reserve's
//! catch-up leans on, even though that reserve is not a required component.

use bevy::prelude::*;
use nova_ship::prelude::{FlightSettings, RcsBudget};
use nova_world::prelude::ThawedAfter;

/// Catch up the [`RcsBudget`] of every root that just thawed by the play
/// time it stood frozen, unless a command was held at freeze. `thaw_ship`
/// inserts the budget as it froze, so the catch-up keys on the budget
/// landing, and runs in the thaw's frame, before the next fixed tick reads
/// it.
pub(crate) fn recover_thawed_rcs_budgets(
    settings: Res<FlightSettings>,
    mut roots: Query<(Entity, Option<&Name>, &ThawedAfter, &mut RcsBudget), Added<RcsBudget>>,
) {
    for (entity, name, thawed, mut budget) in &mut roots {
        let label = name.map_or_else(|| format!("{entity:?}"), |name| name.to_string());
        if budget.idle > 0.0 {
            let before = budget.spent;
            budget.recover(thawed.play_seconds as f32, &settings);
            info!(
                "rcs reserve: '{label}' stood frozen {:.1} s, {before:.2} -> {:.2} u/s spent",
                thawed.play_seconds, budget.spent
            );
        } else {
            debug!(
                "rcs reserve: '{label}' froze with a command held, so it catches up nothing over \
                 {:.1} s",
                thawed.play_seconds
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// An idle thawed magazine catches up exactly the play time it stood
    /// frozen past the recovery delay; one frozen mid-command gets nothing.
    #[test]
    fn an_idle_thawed_magazine_catches_up_and_a_held_command_gets_nothing() {
        let mut world = World::new();
        let settings = FlightSettings::default();
        let stood = |play_seconds: f32| ThawedAfter {
            play_seconds: f64::from(play_seconds),
        };
        let budget = |spent, idle| RcsBudget {
            capacity: 100.0,
            spent,
            idle,
            applied: 0.0,
        };
        let idle_past_delay = settings.rcs_recovery_delay + 2.0;
        let idle = world
            .spawn((stood(idle_past_delay), budget(10.0, 1.0)))
            .id();
        let held = world
            .spawn((stood(idle_past_delay), budget(10.0, 0.0)))
            .id();
        world.insert_resource(settings);

        world.run_system_once(recover_thawed_rcs_budgets).unwrap();
        let settings = world.resource::<FlightSettings>();
        let mut expected = budget(10.0, 1.0);
        expected.recover(idle_past_delay, settings);
        let caught_up = world.get::<RcsBudget>(idle).unwrap();
        assert_eq!(
            (caught_up.spent, caught_up.idle),
            (expected.spent, expected.idle)
        );
        let held = world.get::<RcsBudget>(held).unwrap();
        assert_eq!((held.spent, held.idle), (10.0, 0.0));
    }
}
