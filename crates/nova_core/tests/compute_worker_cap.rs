//! Pins the compute-worker cap to the pool the composition root ACTUALLY
//! builds, not to the options it hands bevy.
//!
//! The distinction is the whole point of the test. The three task pools are
//! process-global statics built through `get_or_init`
//! (`bevy_app-0.19.0/src/task_pool_plugin.rs:229-258`): the first initializer
//! in the process wins, and a second `TaskPoolPlugin` - one added by a plugin
//! group that lands before ours, or a test that built an `App` earlier in the
//! same binary - has its pool built and dropped with no warning and no error.
//! A test that asserts on `nova_core::task_pool_plugin()` would stay green
//! through exactly that failure, because the value it reads is still correct;
//! only the pool is wrong.
//!
//! So this reads `ComputeTaskPool::get().thread_num()`, and it lives in its
//! own integration-test file so that it OWNS its process. Do not move it in
//! beside other tests that build an `App`: cargo runs a file's tests in one
//! binary, the pool is global to that binary, and the first `App` to be built
//! would decide the number this asserts on.

use bevy::tasks::{available_parallelism, ComputeTaskPool};
use nova_core::{AppBuilder, MAX_COMPUTE_WORKERS};

#[test]
fn the_assembled_app_runs_at_most_the_capped_number_of_compute_workers() {
    // Headless, because the cap is chosen in `assemble` before any of the
    // render/window branches split, and headless is the branch that needs no
    // display server to reach it.
    let _app = AppBuilder::headless();

    let workers = ComputeTaskPool::get().thread_num();
    assert!(
        workers <= MAX_COMPUTE_WORKERS,
        "the assembled app runs {workers} compute workers, over the cap of \
         {MAX_COMPUTE_WORKERS}; either the cap is no longer set on the plugin \
         group in `AppBuilder::assemble`, or something built the global pool \
         before it"
    );

    // Above 16 logical CPUs bevy's own split would hand compute `total - 8`
    // threads, which is over the cap - so there the cap must BITE, not merely
    // be respected. At or under 16 the default already lands at or below
    // eight and an equality check would assert bevy's arithmetic, not ours.
    let cpus = available_parallelism();
    if cpus > 2 * MAX_COMPUTE_WORKERS {
        assert_eq!(
            workers,
            MAX_COMPUTE_WORKERS,
            "on a {cpus}-CPU host bevy's default split gives compute {} \
             threads, so the cap must bring it to {MAX_COMPUTE_WORKERS}",
            cpus - 8
        );
    }
}
