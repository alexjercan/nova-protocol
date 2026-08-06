//! The advance conditions a scripted step waits on.
//!
//! A [`Predicate`] is an observation of the world: `Fn(&World) -> bool`, polled
//! once a frame by [`autopilot_drive`](crate::autopilot). A step advances the
//! first frame its predicate holds, so a script says what it is waiting FOR
//! rather than how long it guesses that takes - the difference that keeps a
//! run honest when llvmpipe collapses a wall-clock window into a handful of
//! frames.
//!
//! Elapsed time is one predicate among many, not the driver's clock:
//! [`elapsed`] reads the IN-STEP seconds, so `.until(elapsed(2.0))` means "two
//! seconds after this step began", never "two seconds into the run".
//!
//! ```rust,no_run
//! # use bevy::prelude::*;
//! # use nova_autopilot::prelude::*;
//! # #[derive(Resource)] struct Score(u32);
//! # #[derive(Component)] struct Enemy;
//! // Advance once the score is up AND no enemy is left - but never before a
//! // settling second has passed. `not` is spelled out: the name collides with
//! // `bevy::prelude::not`, so a file globbing both preludes must say which.
//! let done = and(
//!     and(
//!         resource_where::<Score>(|score| score.0 > 0),
//!         nova_autopilot::predicate::not(any_entity::<With<Enemy>>()),
//!     ),
//!     elapsed(1.0),
//! );
//! ```
//!
//! Anything this vocabulary cannot express is a plain closure:
//! `Arc::new(|world: &World| ...) as Arc<Predicate>`. Nova-typed predicates are
//! built that way in `nova_debug::harness`; they cannot live here, because this
//! crate depends on `bevy` alone.

use std::sync::Arc;

use bevy::{ecs::query::QueryFilter, prelude::*};

use crate::autopilot::AutopilotClock;

/// A step's advance condition: an observation of the world, polled every frame
/// while the step is current.
///
/// Handed around as `Arc<Predicate>` so the combinators can share one condition
/// between several steps. Every constructor in this module returns that shape.
pub type Predicate = dyn Fn(&World) -> bool + Send + Sync;

/// Hold the step for `secs` seconds of IN-STEP time, then advance.
///
/// The clock is per step, not per run: it zeroes when the step is entered (and
/// again on every loop cycle), which is what the scripts' hand-rolled
/// `playing_since` offsets used to do. [`hold`](crate::autopilot::AutopilotPlugin::hold)
/// is this predicate plus a state entry.
///
/// False before the driver has armed (no clock in the world yet), so a
/// predicate evaluated outside a run never claims its time is up.
pub fn elapsed(secs: f32) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<AutopilotClock>()
            .is_some_and(|clock| clock.step_elapsed >= secs)
    })
}

/// Hold the step for `count` driven frames, then advance.
///
/// The frame-count twin of [`elapsed`], for beats that need the game's systems
/// to have RUN a given number of times (a UI reconcile, a physics settle)
/// rather than a wall-clock duration - the two differ sharply under a software
/// renderer.
pub fn frames(count: u32) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<AutopilotClock>()
            .is_some_and(|clock| clock.step_frames >= count)
    })
}

/// Advance once the app's state machine is in `state`.
///
/// The asset-gated wait: a Nova script opens on
/// `step("load").until(state_is(GameStates::Playing))` instead of holding
/// `Loading` for a guessed number of seconds, so a slow load delays the beats
/// rather than eating them.
pub fn state_is<S: States>(state: S) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        world
            .get_resource::<State<S>>()
            .is_some_and(|current| *current.get() == state)
    })
}

/// Advance once resource `R` exists and satisfies `f`.
///
/// False while the resource is absent, so it doubles as "wait for `R` to be
/// inserted".
pub fn resource_where<R: Resource>(
    f: impl Fn(&R) -> bool + Send + Sync + 'static,
) -> Arc<Predicate> {
    Arc::new(move |world: &World| world.get_resource::<R>().is_some_and(&f))
}

/// Advance once at least one entity matches the query filter `F`
/// (`any_entity::<With<Marker>>()`).
///
/// False when `F` names a component the app never registered, which is the
/// same answer as "nothing matches" - a spawn that has not happened yet has
/// not registered its component either.
pub fn any_entity<F: QueryFilter + 'static>() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<Entity, F>()
            .is_some_and(|mut query| query.iter(world).next().is_some())
    })
}

/// Advance once the shot named `path` is on disk - the await that replaces a
/// hand-guessed hold after a capture step.
///
/// `path` is the string the step shot with, matched against
/// [`CaptureLog`](crate::capture::CaptureLog).
///
/// On the SMOKE path it holds immediately. An unarmed run shoots nothing, so
/// waiting for a write that will never happen would hang the walk - and this
/// is the only place that difference belongs: the same
/// `.until(shot_written(path))` line then drives both paths, with no example
/// branching its step timing on whether it is capturing.
pub fn shot_written(path: impl Into<String>) -> Arc<Predicate> {
    if !crate::capture::capturing() {
        return Arc::new(|_: &World| true);
    }
    let path = path.into();
    resource_where::<crate::capture::CaptureLog>(move |log| log.wrote(&path))
}

/// Advance once both predicates hold.
pub fn and(a: Arc<Predicate>, b: Arc<Predicate>) -> Arc<Predicate> {
    Arc::new(move |world: &World| a(world) && b(world))
}

/// Advance once the predicate does NOT hold (`not(any_entity::<With<Enemy>>())`
/// is "the last enemy died").
pub fn not(a: Arc<Predicate>) -> Arc<Predicate> {
    Arc::new(move |world: &World| !a(world))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource)]
    struct Score(u32);

    #[derive(Component)]
    struct Enemy;

    #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
    enum TestState {
        #[default]
        Boot,
        Playing,
    }

    /// A world with the driver's clock in it, so the time predicates have
    /// something to read.
    fn world_at(step_elapsed: f32, step_frames: u32) -> World {
        let mut world = World::new();
        world.insert_resource(AutopilotClock {
            elapsed: step_elapsed,
            step_elapsed,
            step_frames,
        });
        world
    }

    #[test]
    fn elapsed_and_frames_read_the_in_step_clock() {
        let world = world_at(0.5, 30);
        assert!(!elapsed(1.0)(&world));
        assert!(elapsed(0.5)(&world));
        assert!(!frames(31)(&world));
        assert!(frames(30)(&world));
    }

    /// Without the driver armed there is no clock, and a time predicate that
    /// defaulted to TRUE would advance a step that never started.
    #[test]
    fn the_time_predicates_are_false_without_a_clock() {
        let world = World::new();
        assert!(!elapsed(0.0)(&world));
        assert!(!frames(0)(&world));
    }

    /// The ack a shot step waits on: false until the write lands, true after.
    /// Built the way `shot_written` builds it on the ARMED path - the env gate
    /// itself is asserted below, because arming it here would race the other
    /// tests sharing this binary.
    #[test]
    fn a_shot_ack_holds_only_once_the_write_has_landed() {
        let ack = resource_where::<crate::capture::CaptureLog>(|log| log.wrote("wiki-hud.png"));
        let mut world = World::new();
        assert!(!ack(&world), "no log yet is not a written shot");

        world.init_resource::<crate::capture::CaptureLog>();
        assert!(!ack(&world), "an empty log is not a written shot");

        world
            .resource_mut::<crate::capture::CaptureLog>()
            .mark("feature-hud.png");
        assert!(!ack(&world), "another shot's ack is not this one's");

        world
            .resource_mut::<crate::capture::CaptureLog>()
            .mark("wiki-hud.png");
        assert!(ack(&world));
    }

    /// The smoke path shoots nothing, so an ack that waited would hang the
    /// walk on a write that is never coming. `NOVA_CAPTURE` is unset under
    /// `cargo test`, which is exactly the unarmed case.
    #[test]
    fn a_shot_ack_holds_immediately_when_the_run_is_not_armed() {
        assert!(
            !crate::capture::capturing(),
            "this test asserts the UNARMED branch; something set NOVA_CAPTURE"
        );
        assert!(shot_written("never-written.png")(&World::new()));
    }

    #[test]
    fn state_is_reads_the_live_state() {
        let mut world = World::new();
        assert!(
            !state_is(TestState::Playing)(&world),
            "no state machine at all is not `in Playing`"
        );
        world.insert_resource(State::new(TestState::Boot));
        assert!(!state_is(TestState::Playing)(&world));
        world.insert_resource(State::new(TestState::Playing));
        assert!(state_is(TestState::Playing)(&world));
    }

    #[test]
    fn resource_where_waits_for_the_resource_then_reads_it() {
        let mut world = World::new();
        let scored = resource_where::<Score>(|score| score.0 > 0);
        assert!(!scored(&world), "an absent resource satisfies nothing");
        world.insert_resource(Score(0));
        assert!(!scored(&world));
        world.insert_resource(Score(3));
        assert!(scored(&world));
    }

    #[test]
    fn any_entity_and_not_bracket_a_spawn_and_a_death() {
        let mut world = World::new();
        let alive = any_entity::<With<Enemy>>();
        let cleared = not(alive.clone());
        assert!(!alive(&world) && cleared(&world));

        let enemy = world.spawn(Enemy).id();
        assert!(alive(&world) && !cleared(&world));

        world.despawn(enemy);
        assert!(!alive(&world) && cleared(&world));
    }

    #[test]
    fn and_holds_only_when_both_do() {
        let mut world = world_at(2.0, 1);
        world.insert_resource(Score(0));
        let both = and(elapsed(1.0), resource_where::<Score>(|score| score.0 > 0));
        assert!(!both(&world), "the clock alone is not enough");
        world.insert_resource(Score(1));
        assert!(both(&world));
    }
}
