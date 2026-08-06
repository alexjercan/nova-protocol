//! A narrow, duration-based value tween over a Bevy `EaseFunction`.
//!
//! Nova owns it because it is the shared bookkeeping behind every UI fade and
//! pop, and `nova_ui` is where nova's UI chrome lives. Unlike the open-ended
//! exponential smoothing nova's gameplay rigs use (`LerpSnap`, which chases a
//! moving target), a [`Tween`] animates a value from a fixed `start` to a fixed
//! `end` over a fixed `duration`, shaped by a Bevy [`EaseFunction`]: the
//! elapsed/clamp/complete dance in one place.
//!
//! It is deliberately narrow -- a `Tween<T>` component plus a completion marker,
//! NOT a keyframe-timeline animation system. Like nova's transform rigs, the
//! tween is an *output*: the plugin advances it and you read [`Tween::value`] to
//! apply the current value wherever you want (a `Transform` field, a color, a
//! `Node` position), so one small component drives any target with no per-field
//! adapters or reflection.
//!
//! [`TweenPlugin`] advances every built-in `Tween<T>` (`f32`, `Vec2`, `Vec3`,
//! `Vec4`) each frame. On completion it applies the [`TweenOnComplete`] policy
//! (remove the tween, despawn the entity, or keep it at the end) and inserts a
//! [`TweenFinished`] marker so an `On<Add, TweenFinished>` observer can run a
//! side effect.
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_ui::tween::*;
//! # fn wire_up() {
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(TweenPlugin);
//! # }
//!
//! // Pop a sprite from 1x to 1.5x scale over 0.15s, then remove the tween.
//! fn spawn_pop(mut commands: Commands) {
//!     commands.spawn((
//!         Transform::default(),
//!         Tween::new(Vec3::ONE, Vec3::splat(1.5), 0.15, EaseFunction::QuadraticOut),
//!     ));
//! }
//!
//! // Apply the current value each frame (after `TweenSystems::Advance`).
//! fn apply_scale(mut q: Query<(&mut Transform, &Tween<Vec3>)>) {
//!     for (mut transform, tween) in &mut q {
//!         transform.scale = tween.value();
//!     }
//! }
//! ```

use bevy::prelude::*;

/// Glob-import surface for the tween: the component, its policies, the value
/// trait and the plugin.
pub mod prelude {
    pub use super::{Tween, TweenFinished, TweenOnComplete, TweenPlugin, TweenSystems, TweenValue};
}

/// A value that a [`Tween`] can interpolate. Implemented for the common
/// animatable primitives (`f32`, `Vec2`, `Vec3`, `Vec4`); a color tween can use
/// its linear-RGBA `Vec4`.
pub trait TweenValue: Clone + Send + Sync + 'static {
    /// Linearly interpolate from `a` to `b` at fraction `t` (already eased and
    /// clamped to `0..=1` by the tween).
    fn tween_lerp(a: &Self, b: &Self, t: f32) -> Self;
}

impl TweenValue for f32 {
    fn tween_lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl TweenValue for Vec2 {
    fn tween_lerp(a: &Self, b: &Self, t: f32) -> Self {
        a.lerp(*b, t)
    }
}

impl TweenValue for Vec3 {
    fn tween_lerp(a: &Self, b: &Self, t: f32) -> Self {
        a.lerp(*b, t)
    }
}

impl TweenValue for Vec4 {
    fn tween_lerp(a: &Self, b: &Self, t: f32) -> Self {
        a.lerp(*b, t)
    }
}

/// What [`TweenPlugin`] does to the entity when a [`Tween`] completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum TweenOnComplete {
    /// Leave the finished tween in place (its [`value`](Tween::value) stays at
    /// `end`). Useful when another system reads the held end value.
    Keep,
    /// Remove the `Tween<T>` component, leaving the entity otherwise untouched.
    Remove,
    /// Despawn the whole entity (a fire-and-forget one-shot).
    Despawn,
}

/// System sets for the tween plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TweenSystems {
    /// Advances every `Tween<T>` and applies completion. Runs in `Update`; read
    /// [`Tween::value`] in a system ordered after this set.
    Advance,
}

/// Marker inserted on an entity the frame one of its tweens completes, so an
/// `On<Add, TweenFinished>` observer (or a query) can react. It is per-entity,
/// not per-tween-type; put one tween per entity when a completion side effect
/// must be unambiguous.
#[derive(Component, Default, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct TweenFinished;

/// Animates a [`TweenValue`] from `start` to `end` over `duration` seconds,
/// shaped by an [`EaseFunction`].
///
/// Attach it to an entity and add [`TweenPlugin`]; the plugin advances it and,
/// on completion, applies [`on_complete`](Self::on_complete) and inserts
/// [`TweenFinished`]. Read the current value with [`value`](Self::value) from a
/// system ordered after [`TweenSystems::Advance`].
#[derive(Component, Debug, Clone)]
pub struct Tween<T: TweenValue> {
    /// The value at `t = 0`.
    pub start: T,
    /// The value at `t = 1`.
    pub end: T,
    /// Total duration in seconds. A non-positive duration completes on the first
    /// advance (a one-frame snap to `end`).
    pub duration: f32,
    /// The easing curve applied to the normalized time before interpolating.
    pub ease: EaseFunction,
    /// What happens to the entity when the tween completes (default
    /// [`TweenOnComplete::Remove`]).
    pub on_complete: TweenOnComplete,
    elapsed: f32,
    /// Set once the plugin has applied the completion policy, so it fires exactly
    /// once (and a zero-duration tween, `finished` from the start, still fires).
    completed: bool,
}

impl<T: TweenValue> Tween<T> {
    /// Create a tween from `start` to `end` over `duration` seconds with `ease`.
    /// Completes by removing the tween component ([`TweenOnComplete::Remove`]).
    pub fn new(start: T, end: T, duration: f32, ease: EaseFunction) -> Self {
        Self {
            start,
            end,
            duration,
            ease,
            on_complete: TweenOnComplete::Remove,
            elapsed: 0.0,
            completed: false,
        }
    }

    /// Set the completion policy (builder style).
    pub fn with_on_complete(mut self, on_complete: TweenOnComplete) -> Self {
        self.on_complete = on_complete;
        self
    }

    /// Normalized, un-eased progress in `0..=1` (`elapsed / duration`).
    pub fn fraction(&self) -> f32 {
        if self.duration > 0.0 {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// The eased progress in `0..=1`: [`fraction`](Self::fraction) passed through
    /// the [`EaseFunction`].
    pub fn eased_fraction(&self) -> f32 {
        self.ease.sample_clamped(self.fraction())
    }

    /// The current interpolated value: `start` eased toward `end` by
    /// [`eased_fraction`](Self::eased_fraction). Equals `end` once finished.
    pub fn value(&self) -> T {
        T::tween_lerp(&self.start, &self.end, self.eased_fraction())
    }

    /// Whether the tween has reached its duration.
    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Advance the elapsed time by `dt`, clamped at the duration.
    fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }
}

/// Advances and completes every built-in `Tween<T>`.
pub struct TweenPlugin;

impl Plugin for TweenPlugin {
    fn build(&self, app: &mut App) {
        debug!("TweenPlugin: build");

        app.register_type::<TweenFinished>()
            .register_type::<TweenOnComplete>()
            .add_systems(
                Update,
                (
                    advance_tween::<f32>,
                    advance_tween::<Vec2>,
                    advance_tween::<Vec3>,
                    advance_tween::<Vec4>,
                )
                    .in_set(TweenSystems::Advance),
            );
    }
}

/// Advance every `Tween<T>` by the frame delta and, on the frame one completes,
/// apply its [`TweenOnComplete`] and mark it [`TweenFinished`]. The completion
/// commands are despawn-safe (`try_*`): a tween whose entity is despawned before this
/// system's buffer flushes is a no-op, not a panic.
fn advance_tween<T: TweenValue>(
    time: Res<Time>,
    mut commands: Commands,
    mut q_tween: Query<(Entity, &mut Tween<T>)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tween) in q_tween.iter_mut() {
        // NOTE: guard on `completed`, not `finished` -- a `Keep` tween stays in place after
        // completing, and a zero-duration tween is `finished` before it is ever applied.
        if tween.completed {
            continue;
        }
        trace!("advance_tween: entity {:?}", entity);
        tween.advance(dt);
        if tween.finished() {
            tween.completed = true;
            // NOTE: `try_*` only -- another system may despawn this entity before the buffer
            // applies, and the plain commands panic with "Entity despawned".
            match tween.on_complete {
                TweenOnComplete::Keep => {
                    commands.entity(entity).try_insert(TweenFinished);
                }
                TweenOnComplete::Remove => {
                    commands
                        .entity(entity)
                        .try_remove::<Tween<T>>()
                        .try_insert(TweenFinished);
                }
                TweenOnComplete::Despawn => {
                    commands.entity(entity).try_despawn();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear(start: f32, end: f32, duration: f32) -> Tween<f32> {
        Tween::new(start, end, duration, EaseFunction::Linear)
    }

    #[test]
    fn starts_at_start_and_is_unfinished() {
        let t = linear(10.0, 20.0, 1.0);
        assert_eq!(t.fraction(), 0.0);
        assert_eq!(t.value(), 10.0);
        assert!(!t.finished());
    }

    #[test]
    fn linear_midpoint_is_halfway() {
        let mut t = linear(10.0, 20.0, 1.0);
        t.advance(0.5);
        assert!((t.fraction() - 0.5).abs() < 1e-6);
        assert!((t.value() - 15.0).abs() < 1e-6);
        assert!(!t.finished());
    }

    #[test]
    fn advance_clamps_at_duration_and_finishes_at_end() {
        let mut t = linear(10.0, 20.0, 1.0);
        t.advance(2.0); // overshoot
        assert_eq!(t.fraction(), 1.0);
        assert_eq!(t.value(), 20.0);
        assert!(t.finished());
    }

    #[test]
    fn zero_duration_is_immediately_finished_at_end() {
        let mut t = linear(10.0, 20.0, 0.0);
        assert!(t.finished());
        assert_eq!(t.value(), 20.0);
        t.advance(0.016);
        assert_eq!(t.value(), 20.0);
    }

    #[test]
    fn easing_bends_the_value_off_the_linear_line() {
        let mut linear_t = linear(0.0, 1.0, 1.0);
        let mut eased_t = Tween::new(0.0, 1.0, 1.0, EaseFunction::QuadraticIn);
        linear_t.advance(0.5);
        eased_t.advance(0.5);
        assert!((linear_t.value() - 0.5).abs() < 1e-6);
        // NOTE: 0.4 sits between QuadraticIn(0.5) = 0.25 and the linear 0.5.
        assert!(eased_t.value() < 0.4);
    }

    #[test]
    fn vec3_tween_interpolates_componentwise() {
        let mut t = Tween::new(
            Vec3::ZERO,
            Vec3::new(2.0, 4.0, 8.0),
            1.0,
            EaseFunction::Linear,
        );
        t.advance(0.25);
        assert!((t.value() - Vec3::new(0.5, 1.0, 2.0)).length() < 1e-6);
    }

    /// Drive the plugin one frame over a tween, returning the entity so the caller
    /// can assert its completion policy fired.
    ///
    /// The callers pass a zero-duration tween, which is `finished` from the start:
    /// that makes the `completed`-flag path fire on the first frame deterministically,
    /// without needing a controllable clock.
    fn app_with_tween(tween: Tween<f32>) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TweenPlugin));
        let entity = app.world_mut().spawn(tween).id();
        app.update();
        (app, entity)
    }

    #[test]
    fn keep_policy_marks_finished_and_leaves_the_tween() {
        let (app, e) = app_with_tween(
            Tween::new(0.0, 1.0, 0.0, EaseFunction::Linear).with_on_complete(TweenOnComplete::Keep),
        );
        assert!(app.world().entity(e).contains::<TweenFinished>());
        assert!(app.world().entity(e).contains::<Tween<f32>>());
    }

    #[test]
    fn remove_policy_drops_the_tween_but_keeps_the_entity() {
        // NOTE: no `with_on_complete` -- `Tween::new` already defaults to Remove.
        let (app, e) = app_with_tween(Tween::new(0.0, 1.0, 0.0, EaseFunction::Linear));
        assert!(app.world().get_entity(e).is_ok(), "entity survives");
        assert!(!app.world().entity(e).contains::<Tween<f32>>());
        assert!(app.world().entity(e).contains::<TweenFinished>());
    }

    #[test]
    fn despawn_policy_despawns_the_entity() {
        let (app, e) = app_with_tween(
            Tween::new(0.0, 1.0, 0.0, EaseFunction::Linear)
                .with_on_complete(TweenOnComplete::Despawn),
        );
        assert!(app.world().get_entity(e).is_err(), "entity is despawned");
    }

    #[derive(Component)]
    struct Doomed;

    fn despawn_doomed(mut commands: Commands, q: Query<Entity, With<Doomed>>) {
        for e in &q {
            commands.entity(e).try_despawn();
        }
    }

    /// Build an app whose `Doomed` entity is despawned in the same frame its tween
    /// completes, returning the entity so the caller can assert it is gone (no panic).
    ///
    /// `auto_insert_apply_deferred` is disabled so Bevy does NOT sync-point the despawn
    /// before `advance_tween` runs: the despawner (ordered first) only *queues* its
    /// despawn, `advance_tween` still sees the live entity in its query and queues its
    /// completion command, then both buffers flush at the end of the schedule in system
    /// order -- despawn first, completion second, landing on the stale entity. That is
    /// exactly the cross-flush race breach hit (a killed enemy's flash tween); with the
    /// auto sync point the despawn would apply too early and the race would never occur.
    fn app_with_doomed_tween(tween: Tween<f32>) -> (App, Entity) {
        use bevy::ecs::schedule::ScheduleBuildSettings;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TweenPlugin));
        app.edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..default()
            });
        });
        app.add_systems(Update, despawn_doomed.before(TweenSystems::Advance));
        let entity = app.world_mut().spawn((Doomed, tween)).id();
        app.update();
        (app, entity)
    }

    #[test]
    fn keep_completion_does_not_panic_if_entity_despawned_first() {
        let (app, e) = app_with_doomed_tween(
            Tween::new(0.0, 1.0, 0.0, EaseFunction::Linear).with_on_complete(TweenOnComplete::Keep),
        );
        assert!(
            app.world().get_entity(e).is_err(),
            "the Keep tween's try_insert is a no-op on the despawned entity, no panic"
        );
    }

    #[test]
    fn remove_completion_does_not_panic_if_entity_despawned_first() {
        // NOTE: no `with_on_complete` -- `Tween::new` already defaults to Remove.
        let (app, e) = app_with_doomed_tween(Tween::new(0.0, 1.0, 0.0, EaseFunction::Linear));
        assert!(
            app.world().get_entity(e).is_err(),
            "the Remove tween's try_remove/try_insert are no-ops on the despawned entity"
        );
    }
}
