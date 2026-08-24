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

/// Advance once the loop named `name` has been encoded and its webm is on
/// disk - the await that closes a [`loop_end`](crate::loops::loop_end) step,
/// mirroring [`shot_written`] shot for shot.
///
/// `name` is the loop name the step ended with; the ack key is its
/// `<name>.webm` file name in [`CaptureLog`](crate::capture::CaptureLog).
///
/// On the SMOKE path it holds immediately, for the same reason
/// [`shot_written`] does: an unarmed run records nothing, and this is the one
/// place that difference belongs.
pub fn loop_written(name: impl Into<String>) -> Arc<Predicate> {
    if !crate::capture::capturing() {
        return Arc::new(|_: &World| true);
    }
    let file = crate::loops::loop_file_name(&name.into());
    resource_where::<crate::capture::CaptureLog>(move |log| log.wrote(&file))
}

/// Advance once a laid-out, VISIBLE UI node called `name` has a box a pointer
/// can be put in.
///
/// The wait a beat owes the widget it is about to point at:
/// [`click_named`](crate::input::click_named) warns and continues when the name
/// resolves to nothing, so a gesture fired at a panel that has not laid out yet
/// is a beat silently lost. This is that layout pass, waited on rather than
/// guessed at - and it is the layout that is waited on, not the spawn:
/// [`ui_node_rect`](crate::input::ui_node_rect) rejects the zero-size box a node
/// carries before `ui_layout_system` has sized it (and the one a `Display::None`
/// node keeps), so a beat cannot advance on an entity that exists but has no
/// place on screen yet.
pub fn ui_node_present(name: impl Into<String>) -> Arc<Predicate> {
    let name = name.into();
    Arc::new(move |world: &World| crate::input::ui_node_rect(world, &name).is_some())
}

/// Advance once the WINDOW MOUSE pointer has REGISTERED the primary button as
/// held.
///
/// The ack of a press beat. [`press_mouse`](crate::input::press_mouse) writes
/// the button event; the picking backend turns it into pointer state one
/// `PreUpdate` later, and a release fired before that is a click the widget
/// under the pointer never saw. Counting frames between the two was a guess at
/// exactly this.
pub fn pointer_pressed() -> Arc<Predicate> {
    Arc::new(|world: &World| mouse_holds_primary(world, true))
}

/// Advance once the window mouse pointer has registered the primary button as
/// released - [`pointer_pressed`]'s other half, and the ack a release beat
/// waits on before the next gesture moves the pointer away.
pub fn pointer_released() -> Arc<Predicate> {
    Arc::new(|world: &World| mouse_holds_primary(world, false))
}

/// Whether [`PointerId::Mouse`]'s primary button is in `held`.
///
/// [`PointerId::Mouse`] specifically, not "any pointer": the gestures synthesize
/// the WINDOW mouse, so that is the pointer whose state is the acknowledgement.
/// An app is free to carry others - Nova's terminal parks a forwarded pointer of
/// its own - and a second pointer sitting in the opposite state would otherwise
/// answer for the one the beat actually moved: its parked "released" would let a
/// release beat through while the mouse press was still unprocessed, and its
/// mirrored "pressed" would ack a press the window pointer never saw (review
/// a4a6 R3).
///
/// False while no mouse pointer exists at all, which is why the RELEASED case
/// asks for a pointer that reports not-pressed rather than for the absence of
/// one: an app whose picking backend never came up must stall its beat, not sail
/// through it.
fn mouse_holds_primary(world: &World, held: bool) -> bool {
    use bevy::picking::pointer::{PointerId, PointerPress};

    world
        .try_query::<(&PointerId, &PointerPress)>()
        .is_some_and(|mut query| {
            query
                .iter(world)
                .any(|(id, press)| id.is_mouse() && press.is_primary_pressed() == held)
        })
}

/// Advance once both predicates hold.
pub fn and(a: Arc<Predicate>, b: Arc<Predicate>) -> Arc<Predicate> {
    Arc::new(move |world: &World| a(world) && b(world))
}

/// Advance once EITHER predicate holds.
///
/// The wait for an ANSWER rather than for a particular one: a beat that aims at
/// a socket and then asserts what the editor decided holds on
/// `or(solved, refused)`, so the assertion runs on a verdict instead of on a
/// solver that has not spoken yet.
pub fn or(a: Arc<Predicate>, b: Arc<Predicate>) -> Arc<Predicate> {
    Arc::new(move |world: &World| a(world) || b(world))
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

    /// The loop ack mirrors the shot ack: keyed by the loop's webm file name,
    /// false until the encode lands it in the log. Built the way
    /// `loop_written` builds it on the ARMED path, for the same
    /// env-racing reason as the shot test above.
    #[test]
    fn a_loop_ack_holds_only_once_the_webm_has_landed() {
        let ack = resource_where::<crate::capture::CaptureLog>(|log| log.wrote("orbit.webm"));
        let mut world = World::new();
        world.init_resource::<crate::capture::CaptureLog>();
        assert!(!ack(&world), "an empty log is not a written loop");

        world
            .resource_mut::<crate::capture::CaptureLog>()
            .mark("orbit.webm");
        assert!(ack(&world));
    }

    /// The smoke path records no loops, so the ack must hold immediately -
    /// the same one-place branching as `shot_written`.
    #[test]
    fn a_loop_ack_holds_immediately_when_the_run_is_not_armed() {
        assert!(
            !crate::capture::capturing(),
            "this test asserts the UNARMED branch; something set NOVA_CAPTURE"
        );
        assert!(loop_written("never-encoded")(&World::new()));
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

    /// `or` is the "an answer arrived" wait: either branch alone advances it,
    /// and neither leaves it held.
    #[test]
    fn or_holds_as_soon_as_either_does() {
        let mut world = World::new();
        let either = or(
            resource_where::<Score>(|score| score.0 > 0),
            any_entity::<With<Enemy>>(),
        );
        assert!(!either(&world), "neither branch holds yet");

        world.insert_resource(Score(1));
        assert!(either(&world), "the first branch alone advances it");

        world.insert_resource(Score(0));
        let enemy = world.spawn(Enemy).id();
        assert!(either(&world), "and so does the second alone");

        world.despawn(enemy);
        assert!(!either(&world));
    }

    /// A node with a real box, as `ui_layout_system` leaves one.
    fn laid_out(size: Vec2, centre: Vec2) -> (UiGlobalTransform, ComputedNode) {
        (
            UiGlobalTransform::from(bevy::math::Affine2::from_translation(centre)),
            ComputedNode {
                size,
                inverse_scale_factor: 1.0,
                ..default()
            },
        )
    }

    /// The wait a beat owes the widget it is about to click: false until the
    /// node has a BOX, and false again once its ancestry hides it - a script
    /// must not point at a panel nobody can see.
    ///
    /// Having the components is not having a box. A node that has not been
    /// through `ui_layout_system` carries a zero-size `ComputedNode::default()`,
    /// and a beat that advanced on that would hand `click_named` a degenerate
    /// rect at the window corner and lose the gesture silently, which is the
    /// race the settles this replaced existed for (review a4a6 R2).
    #[test]
    fn ui_node_present_waits_for_a_laid_out_visible_node() {
        let mut world = World::new();
        let up = ui_node_present("Play Button");
        assert!(!up(&world), "an empty world has laid out nothing");

        let node = world
            .spawn((
                Name::new("Play Button"),
                UiGlobalTransform::default(),
                ComputedNode::default(),
                InheritedVisibility::VISIBLE,
            ))
            .id();
        assert!(
            !up(&world),
            "a spawned but UNSIZED node is not a target: the layout pass has \
             not run, and its zero-size box is the same one a Display::None \
             node keeps"
        );

        let (transform, computed) = laid_out(Vec2::new(120.0, 40.0), Vec2::new(400.0, 300.0));
        world.entity_mut(node).insert((transform, computed));
        assert!(up(&world), "a node with a real box is a target");
        assert_eq!(
            crate::input::ui_node_centre(&world, "Play Button"),
            Some(Vec2::new(400.0, 300.0)),
            "and it resolves to the centre of that box, not to the origin"
        );
        assert!(
            !ui_node_present("Quit Button")(&world),
            "another widget's name is not this one's"
        );

        world.entity_mut(node).insert(InheritedVisibility::HIDDEN);
        assert!(!up(&world), "a hidden node is not a target");
    }

    /// The press/release acks read the PICKING pointer - what the widgets react
    /// to - rather than the button resource the gesture wrote, which is already
    /// true on the frame the gesture ran and would ack nothing.
    ///
    /// Both are false with no pointer at all, so a run whose picking backend
    /// never came up stalls its beat instead of sailing through it. The PRESSED
    /// branch cannot be staged here (`PointerPress`'s fields are private and it
    /// has no constructor - only `bevy_picking`'s own systems set one), and is
    /// proved by the driven editor range, where every press beat holds on it.
    #[test]
    fn the_pointer_acks_read_the_picking_pointer() {
        use bevy::picking::pointer::{PointerId, PointerPress};

        let mut world = World::new();
        assert!(
            !pointer_pressed()(&world) && !pointer_released()(&world),
            "no picking pointer is not an ack of anything"
        );

        world.spawn((PointerId::Mouse, PointerPress::default()));
        assert!(
            !pointer_pressed()(&world),
            "a pointer holding no button is not a press"
        );
        assert!(pointer_released()(&world));
    }

    /// The acks answer for the WINDOW MOUSE, which is the pointer the gestures
    /// drive. Another pointer in the world - Nova's terminal parks a forwarded
    /// one - must not answer in its place: its parked "released" would let a
    /// release beat through while the mouse press was still unprocessed (review
    /// a4a6 R3).
    #[test]
    fn another_pointer_does_not_answer_for_the_mouse() {
        use bevy::picking::pointer::{PointerId, PointerPress};

        let mut world = World::new();
        // A forwarded pointer, parked and released, and NO mouse at all.
        world.spawn((PointerId::Touch(1), PointerPress::default()));
        assert!(
            !pointer_released()(&world),
            "a foreign pointer's released state is not the mouse's ack"
        );
        assert!(!pointer_pressed()(&world));

        // The mouse arrives; now there is an ack to read, and it is its own.
        world.spawn((PointerId::Mouse, PointerPress::default()));
        assert!(pointer_released()(&world));
        assert!(!pointer_pressed()(&world));
    }
}

/// The `Predicate` type and its combinators: time, frames, state, resource,
/// entity, pointer, UI-node, screenshot and loop conditions plus `and`/`or`/`not`.
pub mod prelude {
    pub use super::{
        and, any_entity, elapsed, frames, loop_written, not, or, pointer_pressed, pointer_released,
        resource_where, shot_written, state_is, ui_node_present, Predicate,
    };
}
