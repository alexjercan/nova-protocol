//! The play-time clock of an open world.
//!
//! Nothing frozen simulates, so the world needs one number that says how
//! much play passed while a cell stood frozen: the seconds the virtual clock
//! ran while a world was armed. Every hold on the simulation (the pause
//! overlay, the interface, a scenario load, a world resume) pauses the
//! virtual clock, so the play-time clock stops with it by construction.
//!
//! [`WorldClock`] advances in [`crate::NovaWorldSystems::Observe`], so a
//! body frozen in `Retire` or `Adopt` is stamped with this frame's time and
//! a save after `Retire` carries the same value. [`crate::clear_sector_work`]
//! resets it on the frames it forgets the ledger, and `nova_world_base`
//! restores a saved clock on the arming frame, between the two.

use bevy::prelude::*;

/// Play time of the open world: the seconds the virtual clock ran while a
/// world was armed. Saved with the world, reset with the frozen ledger.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WorldClock {
    /// Seconds of unpaused, armed play.
    pub play_seconds: f64,
}

impl WorldClock {
    /// The play seconds that passed since a body was frozen at `frozen_at`.
    ///
    /// # Panics
    ///
    /// When `frozen_at` is ahead of the clock: a body stamped by a later
    /// world than the one thawing it, which no save or session can produce.
    pub fn elapsed_since(self, frozen_at: f64) -> f64 {
        assert!(
            frozen_at <= self.play_seconds,
            "nova_world: a body froze at {frozen_at} s of play but the clock reads {} s",
            self.play_seconds
        );
        self.play_seconds - frozen_at
    }
}

/// On a body thawed from a frozen record: how many play seconds it stood
/// frozen. A body the generator spawned fresh has none.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ThawedAfter {
    /// Play seconds between the body's freeze and its thaw.
    pub play_seconds: f64,
}

/// Advance the clock by this frame's virtual delta, which is zero while
/// anything holds the simulation.
pub fn advance_world_clock(time: Res<Time<Virtual>>, mut clock: ResMut<WorldClock>) {
    let delta = time.delta_secs_f64();
    if delta > 0.0 {
        clock.play_seconds += delta;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// An app on a manual 100 ms clock that advances the world clock every
    /// update, warmed past the first update, whose delta is zero.
    fn clock_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.init_resource::<WorldClock>();
        app.add_systems(Update, advance_world_clock);
        app.update();
        app
    }

    fn play_seconds(app: &App) -> f64 {
        app.world().resource::<WorldClock>().play_seconds
    }

    /// The clock counts unpaused virtual time and stands still while the
    /// virtual clock is paused, so a pause overlay never adds play time.
    #[test]
    fn the_clock_counts_unpaused_time_only() {
        let mut app = clock_app();
        assert_eq!(play_seconds(&app), 0.0, "a fresh world has seen no play");

        app.update();
        app.update();
        let running = play_seconds(&app);
        assert!(
            (running - 0.2).abs() < 1e-9,
            "two 100 ms frames are 0.2 s of play, not {running}"
        );

        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            play_seconds(&app),
            running,
            "a paused frame adds no play time"
        );

        app.world_mut().resource_mut::<Time<Virtual>>().unpause();
        app.update();
        let resumed = play_seconds(&app);
        assert!(
            (resumed - 0.3).abs() < 1e-9,
            "the clock runs again after the unpause, not {resumed}"
        );
    }

    /// The elapsed time a thaw reads is the clock less the freeze stamp, and
    /// a stamp from the future is a fault, not a negative number.
    #[test]
    fn elapsed_is_the_clock_less_the_freeze_stamp() {
        let clock = WorldClock { play_seconds: 25.0 };
        assert_eq!(clock.elapsed_since(10.0), 15.0);
        assert_eq!(clock.elapsed_since(25.0), 0.0);
        let ahead = std::panic::catch_unwind(|| clock.elapsed_since(30.0));
        assert!(
            ahead.is_err(),
            "a body stamped ahead of the clock is refused"
        );
    }
}
