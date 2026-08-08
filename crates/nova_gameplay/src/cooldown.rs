//! A cooldown countdown: weapon-fire gates, post-hit invulnerability windows.
//!
//! Games hand-roll the same countdown for "you can do this again in N seconds":
//! a float that a hit or a shot sets to a duration, decrements each frame, and
//! that gates an action while it is above zero. [`Cooldown`] owns that pattern
//! with the semantics a cooldown actually wants -- and which a raw `Timer` gets
//! backwards: a fresh [`Cooldown`] is **ready** (you can fire immediately),
//! whereas a fresh `Timer` in `Once` mode is *not* finished, so a weapon built
//! on one would start unable to fire.
//!
//! Nova owns this because the torpedo bay's fire gate and the AI's threat
//! memory, evade window and burst cadence are all written in it; the
//! ready-when-fresh semantics above are a gameplay contract those callers
//! depend on.
//!
//! It is a plain value with no plugin: [`tick`](Cooldown::tick) it from whatever
//! system already runs (a game usually has several cooldowns on one entity, so a
//! blanket auto-tick would not fit). It derives `Component` so a single cooldown
//! can also be attached directly.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_gameplay::prelude::*;
//! # const FIRE_COOLDOWN: f32 = 0.25;
//! #[derive(Component)]
//! struct Weapon {
//!     fire: Cooldown,
//! }
//!
//! fn setup(mut commands: Commands) {
//!     // Starts ready, so the first shot fires immediately.
//!     commands.spawn(Weapon { fire: Cooldown::new(FIRE_COOLDOWN) });
//! }
//!
//! fn fire(time: Res<Time>, mut q: Query<&mut Weapon>) {
//!     for mut weapon in &mut q {
//!         weapon.fire.tick(time.delta_secs());
//!         if weapon.fire.ready() {
//!             weapon.fire.trigger(); // ... spawn a bullet ...
//!         }
//!     }
//! }
//! ```

use bevy::prelude::*;

/// The `Cooldown` timer component.
pub mod prelude {
    pub use super::Cooldown;
}

/// A countdown that gates an action until it elapses. Starts [`ready`](Self::ready);
/// [`trigger`](Self::trigger) starts the wait, [`tick`](Self::tick) advances it.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Cooldown {
    remaining: f32,
    duration: f32,
}

impl Cooldown {
    /// A cooldown of `duration` seconds, created **ready** (remaining zero), so
    /// the first [`trigger`](Self::trigger) is what starts the first wait.
    pub fn new(duration: f32) -> Self {
        Self {
            remaining: 0.0,
            duration: duration.max(0.0),
        }
    }

    /// A cooldown that starts already counting down (remaining at the full
    /// duration), for something that begins on cooldown -- e.g. a ship that
    /// spawns with invulnerability frames already running.
    pub fn started(duration: f32) -> Self {
        let duration = duration.max(0.0);
        Self {
            remaining: duration,
            duration,
        }
    }

    /// Whether the cooldown has elapsed and the action is available again.
    pub fn ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Start the wait: set the remaining time to the full [`duration`](Self::duration).
    pub fn trigger(&mut self) {
        self.remaining = self.duration;
    }

    /// Start a wait of a specific length, independent of `duration` (a
    /// variable-length window, e.g. i-frames scaled by the hit). Never negative.
    pub fn trigger_for(&mut self, seconds: f32) {
        self.remaining = seconds.max(0.0);
    }

    /// Advance the cooldown by `dt` seconds, clamped at zero. A no-op once ready.
    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }

    /// Seconds left before the cooldown is ready (`0.0` when ready).
    pub fn remaining(&self) -> f32 {
        self.remaining
    }

    /// The full cooldown length this was constructed with.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Remaining time as a fraction of the full duration (`1.0` just after a
    /// [`trigger`](Self::trigger), `0.0` when ready), for a cooldown gauge or a
    /// blink. `0.0` when the duration is zero.
    pub fn fraction(&self) -> f32 {
        if self.duration > 0.0 {
            (self.remaining / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cooldown_is_ready() {
        let cd = Cooldown::new(1.0);
        assert!(cd.ready());
        assert_eq!(cd.remaining(), 0.0);
        assert_eq!(cd.duration(), 1.0);
        assert_eq!(cd.fraction(), 0.0);
    }

    #[test]
    fn trigger_starts_the_wait() {
        let mut cd = Cooldown::new(1.0);
        cd.trigger();
        assert!(!cd.ready());
        assert_eq!(cd.remaining(), 1.0);
        assert!((cd.fraction() - 1.0).abs() < 1e-6);
    }

    /// A tick that overshoots clamps at zero rather than going negative, and
    /// ticking an already-ready cooldown is a no-op.
    #[test]
    fn tick_counts_down_and_becomes_ready() {
        let mut cd = Cooldown::new(1.0);
        cd.trigger();
        cd.tick(0.4);
        assert!(!cd.ready());
        assert!((cd.remaining() - 0.6).abs() < 1e-6);
        cd.tick(1.0);
        assert!(cd.ready());
        assert_eq!(cd.remaining(), 0.0);
        cd.tick(1.0);
        assert_eq!(cd.remaining(), 0.0);
    }

    #[test]
    fn started_begins_on_cooldown() {
        let cd = Cooldown::started(1.5);
        assert!(!cd.ready());
        assert_eq!(cd.remaining(), 1.5);
        assert!((cd.fraction() - 1.0).abs() < 1e-6);
    }

    /// A negative window clamps to zero, leaving the cooldown immediately ready.
    #[test]
    fn trigger_for_sets_a_custom_window() {
        let mut cd = Cooldown::new(1.0);
        cd.trigger_for(2.5);
        assert_eq!(cd.remaining(), 2.5);
        cd.trigger_for(-1.0);
        assert!(cd.ready());
    }
}
