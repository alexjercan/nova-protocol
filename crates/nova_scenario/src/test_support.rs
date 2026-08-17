//! Driving a rig's scenario to LIVE, behind the `test-support` feature.
//!
//! The spawn queue is drained in CHUNKS: `state_to_world_system` applies queued
//! commands under [`SPAWN_DRAIN_BUDGET`](crate::world) worth of wall clock per
//! frame, and [`EventWorld::is_settling`] reports the world as still being
//! BUILT until the queue empties. The event dispatcher holds every authored
//! handler while that is true, so nothing runs against a half-populated world.
//!
//! That makes a frame COUNT the wrong way for a rig to wait. A rig that fires
//! events by hand and ticks a fixed number of frames silently stops driving
//! anything the moment its scenario queues more objects than those frames can
//! drain - the shipped shakedown chapter needs 62 updates to settle, and its
//! walk helpers ticked 2 to 14. These helpers say "the world is live now"
//! instead of guessing.
//!
//! Behind a FEATURE, not `#[cfg(test)]`: a `cfg(test)` module is invisible
//! across a crate boundary, and the base-content walks in `nova_authoring`
//! drive this same pipeline. Mirrors `nova_gameplay::test_support`.

use bevy::prelude::*;
use nova_events::prelude::EventWorld;

use crate::prelude::NovaEventWorld;

/// Iterations either helper spends before it calls the drain broken.
///
/// Far above any real scenario (the biggest shipped chapter settles in ~62), so
/// reaching it means the drain stopped making progress - a failure worth a
/// panic rather than a hang.
const SETTLE_LIMIT: usize = 10_000;

/// True while the rig's scenario is still building its world.
fn is_settling(world: &World) -> bool {
    world
        .get_resource::<NovaEventWorld>()
        .is_some_and(NovaEventWorld::is_settling)
}

/// Run `app` until its scenario world is built AND STAYS built. Returns the
/// updates spent.
///
/// Call it after firing an event whose handlers spawn - `OnStart` above all -
/// and before asserting on anything the SCRIPT did: until it returns, the
/// dispatcher is holding every handler and the rig is driving nothing.
///
/// Each pass drains the queued objects, then spends one frame letting the
/// dispatcher release the handlers it held while they landed. Those handlers
/// routinely queue MORE objects - `OnStart` settles, the held `OnUpdate` then
/// dispatches, and beat one queues its beacon - so the pass REPEATS until a
/// release frame adds nothing new. A single release frame would hand back a
/// world that is settling again, and every call site would have to know to call
/// this twice.
pub fn settle_spawns(app: &mut App) -> usize {
    let mut updates = 0;
    loop {
        while is_settling(app.world()) {
            tick(app, &mut updates);
        }
        // Release whatever the build held. If those handlers queued more
        // objects the world is settling again, so go round.
        tick(app, &mut updates);
        if !is_settling(app.world()) {
            return updates;
        }
    }
}

/// One update, counted, with the runaway guard.
fn tick(app: &mut App, updates: &mut usize) {
    app.update();
    *updates += 1;
    assert!(
        *updates < SETTLE_LIMIT,
        "the scenario never settled in {SETTLE_LIMIT} updates; the spawn drain \
         is not making progress"
    );
}

/// Apply every queued scenario command against a bare [`World`], for a rig that
/// drives [`NovaEventWorld::state_to_world_system`] directly instead of running
/// an [`App`]. Returns the runs spent.
///
/// The `App`-free twin of [`settle_spawns`]: no schedule means no dispatcher to
/// wait on, so this only empties the queue.
pub fn drain_spawns(world: &mut World) -> usize {
    let mut runs = 0;
    while is_settling(world) {
        NovaEventWorld::state_to_world_system(world);
        runs += 1;
        assert!(
            runs < SETTLE_LIMIT,
            "the spawn queue never drained in {SETTLE_LIMIT} runs; the drain is \
             not making progress"
        );
    }
    runs
}

#[cfg(test)]
mod tests {
    use core::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use nova_events::prelude::{
        CommandsGameEventExt, EventAction, EventHandler, GameEventInfo, GameEventsPlugin,
        OnUpdateEvent,
    };
    use nova_gameplay::prelude::GameObjectives;

    use super::*;
    use crate::prelude::*;

    /// Objects applied so far, across every rig in this module.
    static APPLIED: AtomicUsize = AtomicUsize::new(0);

    /// Queue `count` objects, each costing about the whole per-frame drain
    /// budget - the shape of a beat setup that spawns a handful of ships.
    fn queue_burst(world: &mut NovaEventWorld, count: usize) {
        for _ in 0..count {
            world.push_command(|commands| {
                commands.queue(|_: &mut World| {
                    std::thread::sleep(Duration::from_millis(2));
                    APPLIED.fetch_add(1, Ordering::SeqCst);
                });
            });
        }
    }

    /// A handler action that queues MORE work than one frame's drain budget.
    struct SpawnBurst;

    impl EventAction<NovaEventWorld> for SpawnBurst {
        fn action(&self, world: &mut NovaEventWorld, _: &GameEventInfo) {
            queue_burst(world, 3);
        }
    }

    /// The release frame can queue MORE work than it can drain: a build
    /// settles, the event held through it then dispatches, and ITS handler
    /// spawns a burst of its own. A helper that spent a SINGLE frame releasing
    /// would hand back a world that is settling again, and every call site
    /// would have to know to call it twice.
    ///
    /// Fail-first: replacing the loop with one release frame leaves the world
    /// settling and one of the released objects unapplied.
    #[test]
    fn settling_reaches_a_fixed_point_when_a_released_handler_spawns() {
        APPLIED.store(0, Ordering::SeqCst);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();

        let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
        handler.add_action(SpawnBurst);
        app.world_mut().spawn(handler);
        app.update();

        // The build burst, plus an event fired into it that the gate will hold
        // until the build is done.
        queue_burst(&mut app.world_mut().resource_mut::<NovaEventWorld>(), 3);
        app.world_mut().commands().fire::<OnUpdateEvent>(default());
        assert!(
            is_settling(app.world()),
            "delivery guard: the rig starts mid-build"
        );

        settle_spawns(&mut app);

        assert!(
            !is_settling(app.world()),
            "settle_spawns must hand back a LIVE world, not one that is \
             settling again on what the release frame queued"
        );
        assert_eq!(
            APPLIED.load(Ordering::SeqCst),
            6,
            "the build's three objects AND the released handler's three landed"
        );
    }
}
