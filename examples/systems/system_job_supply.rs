//! system_job_supply: Bring a stranded caller the medkits it asks for, from the board to the hand-in.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed as the `bounty_hunter` start role, whose boards lean to this kind, and
//! walks the shared located-job range in [`located_jobs`]: find a living
//! station posting a supply job framed as a medkit rescue, dock, accept it
//! off the Jobs tab, read the hand-in refused as not done, undock, do the
//! work in the world, see the open world mark the job done, dock again and
//! hand it in for the reward. The work is two docks with the caller: one with
//! the hold one medkit short, which leaves the job open and takes nothing,
//! then one with the whole stack aboard, which the dock takes.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a supply job is posted and accepted` | a living station's board posts a medkit rescue; after the Accept click the job is held and its caller stands under the poster's cell root with none of the asked item aboard |
//! | 2 | `outcome: the hand-in is refused until the supply job is done` | docked at the poster with the job held, Your jobs greys the Hand in button with the not-done reason in its tooltip |
//! | 3 | `outcome: a dock with the hold short leaves the supply job open` | docked with the caller with one medkit fewer than asked, the job is still held and not done, the hold is unchanged and the HUD notice reads `JOB-n: SHORT 1 MEDKIT` |
//! | 4 | `outcome: the world marks the supply job done` | after a second dock with the whole stack aboard the job is listed done while still held, and every medkit is gone from the hold |
//! | 5 | `outcome: the hand-in pays the supply job` | docked again, the Hand in click pays the reward from the station, the open world grants its first-delivery 50 cr and the job is listed completed |
//!
//! Between claims 2 and 3 the range also parks in flight off the caller and
//! asserts the caller wears its in-flight objective marker chip.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_job_supply --features debug
//! # look for: `supply job: PASS the caller wears the in-flight marker`,
//! #           `supply job: PASS the short dock left the job open`,
//! #           `supply job: PASS the hand-in paid`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `job-supply-board.png` of the Jobs tab
//! with the held job selected in the inspector, `job-supply-inflight.png`
//! of the caller in flight wearing its marker chip, `job-supply-short.png`
//! of the HUD notice at the short dock, `job-supply-inspector-done.png` of
//! the inspector with the job done before the hand-in, and
//! `job-supply-done.png` after the hand-in.

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "shared/located_jobs.rs"]
pub mod located_jobs;

#[derive(Parser)]
#[command(name = "system_job_supply")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, dock at a living generated station, accept a medkit \
             rescue supply job off its board, do the work in the world and hand it in. Autopilot-only \
             correctness range",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: no player's saved world may be in the way.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let mut app = nova_protocol::prelude::editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        use located_jobs::{script, LocatedJobMarkers, LocatedJobWalkPlugin, LocatedKindType};

        // The roster test reads the markers off this file, beside the walk.
        const MARKERS: LocatedJobMarkers = LocatedJobMarkers {
            accepted: "outcome: a supply job is posted and accepted",
            refused: "outcome: the hand-in is refused until the supply job is done",
            done: "outcome: the world marks the supply job done",
            paid: "outcome: the hand-in pays the supply job",
            escort: None,
            short: Some("outcome: a dock with the hold short leaves the supply job open"),
        };

        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds the job's path.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Supply, MARKERS));
        app.add_plugins(script(LocatedKindType::Supply));
        app.add_systems(
            Startup,
            (
                nova_protocol::nova_debug::harness::force_capture_resolution,
                nova_protocol::nova_debug::harness::hide_dev_overlays,
            ),
        );
    }

    app.run()
}
