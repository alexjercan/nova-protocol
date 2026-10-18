//! system_job_distress: Answer a distress call, a hull in need or a trap, from the board to the hand-in.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed as the `bounty_hunter` start role, whose boards lean to this kind, and
//! walks the shared located-job range in [`located_jobs`]: find a living
//! station posting a distress job, dock, accept it off the Jobs tab, read
//! the hand-in refused as not done, undock, do the work in the world, see the
//! open world mark the job done, dock again and hand it in for the reward.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a distress job is posted and accepted` | a living station's board posts the kind; after the Accept click the job is held and its bodies stand under the poster's cell root |
//! | 2 | `outcome: the hand-in is refused until the distress job is done` | docked at the poster with the job held, Your jobs greys the Hand in button with the not-done reason in its tooltip |
//! | 3 | `outcome: the world marks the distress job done` | after the work the job is listed done while still held |
//! | 4 | `outcome: the hand-in pays the distress job` | docked again, the Hand in click pays the reward from the station and lists the job completed |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_job_distress --features debug
//! # look for: `distress job: PASS the hand-in paid`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `job-distress-board.png` of the Jobs tab
//! with the held job and `job-distress-done.png` after the hand-in.

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "shared/located_jobs.rs"]
mod located_jobs;

#[derive(Parser)]
#[command(name = "system_job_distress")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, dock at a living generated station, accept a distress \
             job off its board, do the work in the world and hand it in. Autopilot-only \
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
            accepted: "outcome: a distress job is posted and accepted",
            refused: "outcome: the hand-in is refused until the distress job is done",
            done: "outcome: the world marks the distress job done",
            paid: "outcome: the hand-in pays the distress job",
        };

        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds the job's path.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Distress, MARKERS));
        app.add_plugins(script(LocatedKindType::Distress));
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
