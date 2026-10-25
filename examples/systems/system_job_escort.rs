//! system_job_escort: Escort a hauler to its mark past the raiders its cargo draws, from the board to the hand-in.
//!
//! Boots the shipped app (via [`editor_app`]), starts New Game on a pinned
//! seed as the armed `military` start role (the Trade starts that lean to
//! this kind fly unarmed hulls, so their starting station hides it; seed 10
//! posts a three-raider escort), and walks the shared located-job range in
//! [`located_jobs`]: find a living station posting a escort job, dock,
//! accept it off the Jobs tab, read the hand-in refused as not done, undock,
//! fly GOTO to the waiting hauler, hold formation on its leg, meet the
//! ambush by intercepting the weakest raider with the hull's turrets pinned
//! on its mounts until the hauler reaches its mark, see the open world mark
//! the job done, dock again and hand it in for the reward. The raiders hunt
//! the hauler and turn on the escort that shoots them. Nothing teleports the
//! hauler: the leg is flown in full.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: a escort job is posted and accepted` | a living station's board posts the kind; after the Accept click the job is held and its bodies stand under the poster's cell root |
//! | 2 | `outcome: the hand-in is refused until the escort job is done` | docked at the poster with the job held, Your jobs greys the Hand in button with the not-done reason in its tooltip |
//! | 3 | `outcome: the escort's hauler waits for the rendezvous` | on the undock the leg is laid and not started, and the hauler wears no helm order and no cruise cap while the player stands farther than 1 km from it |
//! | 4 | `outcome: the hauler leaves when the escort arrives` | the leg starts with the player inside the rendezvous, and the hauler wears a Move and a 100 m/s cruise cap |
//! | 5 | `outcome: the raiders spring with the escort in formation` | the first raider engages with the hauler at or under its cruise and the player within 1.5 km of it |
//! | 6 | `outcome: the escorted hauler survives the ambush to its mark` | the job is done, not lost; the player's fire landed on a raider; a raider engaged for 20 s of the flight after the spring; a hit hauler outlived its first hit by 20 s, an unhit one is reported unhit |
//! | 7 | `outcome: the world marks the escort job done` | after the work the job is listed done while still held |
//! | 8 | `outcome: the hand-in pays the escort job` | docked again, the Hand in click pays the reward from the station and lists the job completed |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`; the
//! leg is real flight, minutes on a software GPU):
//! ```text
//! NOVA_AUTOPILOT=1 NOVA_AUTOPILOT_DEADLINE=1800 \
//!   cargo run --example system_job_escort --features debug
//! # look for: `escort job: PASS the hand-in paid`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! `NOVA_CAPTURE=1` also writes `job-escort-board.png` of the Jobs tab
//! with the held job selected in the inspector, `job-escort-inspector-done.png`
//! of the inspector with the job done before the hand-in, and
//! `job-escort-done.png` after the hand-in.

use bevy::prelude::*;
use clap::Parser;

#[cfg(feature = "debug")]
#[path = "shared/located_jobs.rs"]
pub mod located_jobs;

#[derive(Parser)]
#[command(name = "system_job_escort")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game on a pinned seed, dock at a living generated station, accept a escort \
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
        use located_jobs::{
            script, EscortMarkers, LocatedJobMarkers, LocatedJobWalkPlugin, LocatedKindType,
        };

        // The roster test reads the markers off this file, beside the walk.
        const MARKERS: LocatedJobMarkers = LocatedJobMarkers {
            accepted: "outcome: a escort job is posted and accepted",
            refused: "outcome: the hand-in is refused until the escort job is done",
            done: "outcome: the world marks the escort job done",
            paid: "outcome: the hand-in pays the escort job",
            escort: Some(EscortMarkers {
                waits: "outcome: the escort's hauler waits for the rendezvous",
                leaves: "outcome: the hauler leaves when the escort arrives",
                springs: "outcome: the raiders spring with the escort in formation",
                survives: "outcome: the escorted hauler survives the ambush to its mark",
            }),
        };

        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the range holds the job's path.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(LocatedJobWalkPlugin(LocatedKindType::Escort, MARKERS));
        app.add_plugins(script(LocatedKindType::Escort));
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
