//! Grow-in-use emphasis: the one mechanism every contextual HUD widget uses to
//! say "this is the thing you are using right now" (task 20260728-175747).
//!
//! Demo 2 (`examples/ui/hud_rework_poc.html`) expresses emphasis as a CSS
//! `transform: scale(...)` with a 0.2s ease, plus one oscillating case (the
//! reticle pulsing while the trigger is down). Both are the same idea, so both
//! live in one component and one system rather than a bespoke tween per widget:
//! attach [`HudEmphasis`] to the node, let the widget's own driver say when the
//! situation is live, and [`drive_hud_emphasis`] writes `UiTransform::scale`.
//!
//! Why `UiTransform` and not `Node`: `screen_indicator` drives projected chips
//! through `Node.left/top` and only ever touches `UiTransform` for the arrow
//! ROTATION, so scale is a free axis - an emphasized chip keeps its projected
//! position exactly. The default `Res<Time>` clock is correct here (the HUD
//! emphasizes during unpaused flight), which also makes every one-shot testable
//! by advancing virtual time.

use bevy::prelude::*;

/// Glob-import surface: `use nova_gameplay::hud::emphasis::prelude::*` re-exports the public API of this module.
pub mod prelude {
    pub use super::{HudEmphasis, HudEmphasisMode};
}

/// Seconds to ease between rest and full emphasis - demo 2's `transition:
/// transform 0.2s ease`.
const EASE_SECS: f32 = 0.2;

/// How an emphasis behaves while its situation is live.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum HudEmphasisMode {
    /// Grow to the peak and HOLD there while the situation lasts, settling back
    /// when it ends (the speed chip on an autopilot burn, a hot dock chip, the
    /// objective pop).
    Settle,
    /// Oscillate between rest and the peak while the situation lasts (the
    /// combat reticle while the trigger is down). `period_secs` is one full
    /// there-and-back cycle.
    Pulse {
        /// One full rest -> peak -> rest cycle, in seconds.
        period_secs: f32,
    },
}

/// Scale emphasis on one HUD node. The widget's driver owns WHEN
/// ([`set_held`](Self::set_held) for a situation that holds,
/// [`pop`](Self::pop) for a one-shot); this component owns HOW MUCH, and
/// [`drive_hud_emphasis`] writes the result to `UiTransform::scale`.
#[derive(Component, Debug, Clone, Reflect)]
#[require(UiTransform)]
pub struct HudEmphasis {
    /// Scale at full emphasis (1.14 for the speed chip, 1.08 for a hot dock
    /// chip, ...). 1.0 is no emphasis at all.
    pub peak: f32,
    /// Hold-while-live or oscillate-while-live.
    pub mode: HudEmphasisMode,
    /// Whether the driving situation is live right now.
    held: bool,
    /// Remaining seconds of a one-shot pop; 0.0 when none is playing.
    one_shot_secs: f32,
    /// The eased 0..1 emphasis amount the scale is rendered from.
    amount: f32,
    /// Seconds this emphasis has been continuously active, for the pulse wave.
    /// Reset when the emphasis goes fully idle so every pulse starts at rest.
    active_secs: f32,
}

impl HudEmphasis {
    /// A hold-and-settle emphasis peaking at `peak`.
    pub fn settle(peak: f32) -> Self {
        Self {
            peak,
            mode: HudEmphasisMode::Settle,
            held: false,
            one_shot_secs: 0.0,
            amount: 0.0,
            active_secs: 0.0,
        }
    }

    /// An oscillating emphasis peaking at `peak`, one full cycle per
    /// `period_secs`.
    pub fn pulse(peak: f32, period_secs: f32) -> Self {
        Self {
            mode: HudEmphasisMode::Pulse { period_secs },
            ..Self::settle(peak)
        }
    }

    /// A one-shot pop of `secs` that is already `age` seconds in.
    ///
    /// For widgets whose nodes are REBUILT every frame from their age rather
    /// than retained (the comms cards): a freshly spawned component would
    /// restart the ease every frame and never leave rest, so the state is
    /// seeded from the age instead, exactly like those widgets' alpha. A
    /// retained node should use [`pop`](Self::pop).
    pub fn popped_at_age(peak: f32, secs: f32, age: f32) -> Self {
        let remaining = (secs - age).max(0.0);
        let amount = if remaining > 0.0 {
            (age / EASE_SECS).clamp(0.0, 1.0)
        } else {
            (1.0 - (age - secs) / EASE_SECS).clamp(0.0, 1.0)
        };
        Self {
            one_shot_secs: remaining,
            amount,
            active_secs: age,
            ..Self::settle(peak)
        }
    }

    /// Say whether the driving situation is live. Idempotent - a driver can
    /// call this every frame.
    pub fn set_held(&mut self, held: bool) {
        self.held = held;
    }

    /// Whether the driving situation is currently held.
    pub fn held(&self) -> bool {
        self.held
    }

    /// Start (or restart) a one-shot emphasis lasting `secs`. Re-popping while
    /// one is playing restarts the countdown rather than stacking.
    pub fn pop(&mut self, secs: f32) {
        self.one_shot_secs = secs;
    }

    /// Whether a one-shot pop is still playing.
    pub fn popping(&self) -> bool {
        self.one_shot_secs > 0.0
    }

    /// The eased emphasis amount in 0..1 (0 = at rest, 1 = at the peak).
    pub fn amount(&self) -> f32 {
        self.amount
    }

    /// The scale factor to render right now.
    pub fn scale(&self) -> f32 {
        1.0 + (self.peak - 1.0) * self.amount
    }

    /// Advance by `delta` seconds. Split out from the system so the easing is
    /// testable without a UI tree.
    fn advance(&mut self, delta: f32) {
        self.one_shot_secs = (self.one_shot_secs - delta).max(0.0);
        let active = self.held || self.popping();
        if !active && self.amount <= 0.0 {
            self.active_secs = 0.0;
        } else if active {
            self.active_secs += delta;
        }

        // A live pulse IS its own smooth curve (it starts at rest and returns
        // there every cycle), so it drives the amount directly - easing toward
        // a moving target would just lag the wave and clip its peak. Everything
        // else eases toward a fixed target at a constant rate.
        if let (true, HudEmphasisMode::Pulse { period_secs }) = (active, self.mode) {
            // Phased so it leaves rest immediately: 0 at t=0, 1 at the half
            // cycle, back to 0 at the full cycle.
            let phase = (self.active_secs / period_secs.max(f32::EPSILON)) * std::f32::consts::TAU
                - std::f32::consts::FRAC_PI_2;
            self.amount = 0.5 + 0.5 * phase.sin();
            return;
        }

        let target = if active { 1.0 } else { 0.0 };
        let step = delta / EASE_SECS;
        let gap = target - self.amount;
        self.amount = if gap.abs() <= step {
            target
        } else {
            self.amount + step * gap.signum()
        };
    }
}

/// Drive every [`HudEmphasis`] node's `UiTransform::scale` from its eased
/// amount. One system for the whole HUD - widgets only decide when their
/// situation is live.
pub fn drive_hud_emphasis(time: Res<Time>, mut q: Query<(&mut HudEmphasis, &mut UiTransform)>) {
    let delta = time.delta_secs();
    for (mut emphasis, mut transform) in &mut q {
        emphasis.advance(delta);
        let scale = Vec2::splat(emphasis.scale());
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A settle emphasis reaches its peak while held and returns to exactly 1.0
    /// once the situation ends - "reverted", not "nearly reverted".
    #[test]
    fn a_settle_emphasis_grows_while_held_and_returns_to_rest() {
        let mut emphasis = HudEmphasis::settle(1.14);
        assert_eq!(emphasis.scale(), 1.0, "starts at rest");

        emphasis.set_held(true);
        for _ in 0..20 {
            emphasis.advance(1.0 / 60.0);
        }
        assert_eq!(emphasis.scale(), 1.14, "held reaches the peak");

        emphasis.set_held(false);
        for _ in 0..20 {
            emphasis.advance(1.0 / 60.0);
        }
        assert_eq!(emphasis.scale(), 1.0, "released settles all the way back");
    }

    /// A one-shot pop runs for its own duration without anything holding it,
    /// then settles - the objective/comms case.
    #[test]
    fn a_one_shot_pop_settles_on_its_own() {
        let mut emphasis = HudEmphasis::settle(1.16);
        emphasis.pop(1.2);

        emphasis.advance(0.3);
        assert!(emphasis.popping(), "still popping at 0.3s");
        assert_eq!(emphasis.scale(), 1.16, "the pop reaches the peak");

        // Past the pop plus the ease-out.
        for _ in 0..90 {
            emphasis.advance(1.0 / 60.0);
        }
        assert!(!emphasis.popping(), "the pop has expired");
        assert_eq!(emphasis.scale(), 1.0, "and the node is back at rest");
    }

    /// Re-popping restarts the countdown instead of stacking, so a burst of
    /// postings cannot leave a chip stuck large.
    #[test]
    fn re_popping_restarts_rather_than_stacking() {
        let mut emphasis = HudEmphasis::settle(1.16);
        emphasis.pop(1.2);
        emphasis.advance(1.0);
        emphasis.pop(1.2);
        emphasis.advance(0.5);
        assert!(
            emphasis.popping(),
            "the second pop is still running at 0.5s"
        );
        emphasis.advance(0.8);
        assert!(!emphasis.popping(), "and expires 1.2s after the RE-pop");
    }

    /// A pulse oscillates while held: it leaves rest, comes back toward it and
    /// leaves again inside one period, rather than parking at the peak.
    #[test]
    fn a_pulse_oscillates_while_held() {
        let mut emphasis = HudEmphasis::pulse(1.12, 0.56);
        emphasis.set_held(true);

        let mut samples = Vec::new();
        for _ in 0..34 {
            emphasis.advance(1.0 / 60.0);
            samples.push(emphasis.amount());
        }
        let peak = samples.iter().cloned().fold(f32::MIN, f32::max);
        let trough_after_peak = samples
            .iter()
            .skip_while(|&&a| a < peak)
            .cloned()
            .fold(f32::MAX, f32::min);
        assert!(peak > 0.9, "the wave reaches the peak (peak {peak})");
        assert!(
            trough_after_peak < 0.2,
            "and comes back down inside the period (trough {trough_after_peak})"
        );
    }

    /// The system writes the scale onto the node's `UiTransform` - the seam the
    /// widgets rely on.
    #[test]
    fn the_system_writes_the_scale_onto_ui_transform() {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        // Manual clock, like the reveal's rig: `Time` is re-written from
        // `Time<Real>` every frame, so a hand-advanced clock would be stomped.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ));
        app.add_systems(Update, drive_hud_emphasis);

        let mut emphasis = HudEmphasis::settle(1.14);
        emphasis.set_held(true);
        let node = app
            .world_mut()
            .spawn((emphasis, UiTransform::default()))
            .id();

        for _ in 0..20 {
            app.update();
        }

        let transform = app.world().get::<UiTransform>(node).unwrap();
        assert_eq!(
            transform.scale,
            Vec2::splat(1.14),
            "the held emphasis is on the node"
        );
    }
}
