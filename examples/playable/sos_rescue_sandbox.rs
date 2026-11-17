//! sos_rescue_sandbox: drift stranded in the open world, call SOS from the
//! Ship pane, and let the nearest station that answers tow you in.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, as the Create World modal does but without a save folder. When
//! the start's window has streamed in and the start has docked the player,
//! the player lets go and jumps 20 km off the starting station, at rest.
//!
//! THE DAMAGE IS DISCLOSED FIXTURE STATE. When the new window has streamed
//! in, the sandbox puts the interceptor's bridge (`CTL-1`), its three drives
//! and its port collar (`DOCK-1`) at zero, the way the damage path does. The
//! ship has no drive, no flight computer and no free port, and an armed ship
//! with its computer lost is neutralized. The SOS offer reads only the
//! first three: a ship that lost only its guns flies and docks.
//!
//! What to do:
//!
//! 1. press Tab and open the Ship tab: the Distress block tags why the ship
//!    is stranded and shows what a rescue charges now;
//! 2. click Call SOS. The block shows the call, its bar, its time left and
//!    Cancel;
//! 3. close Tab. The call runs only while you fly: the chip under the RCS
//!    chip reads `SOS 42 S`. When it is due, the nearest living station that
//!    is not hostile answers: it takes the bill, rebuilds the bridge, a drive
//!    and the port, moves the ship beside it and docks it.
//!
//! The diagnostics layer (F3) is on at start: why the ship is stranded, the
//! bill, the balance and the running call.
//!
//! ```text
//! nix develop --command cargo run --example sos_rescue_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk waits for the staging, opens the Ship pane, clicks Call SOS, closes
//! the interface and waits for the dock, then checks the bill, the rebuilt
//! sections and the dock. `NOVA_CAPTURE=1` shoots the pane with the call
//! and the docked view.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use avian3d::prelude::Position;
use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};
use open_world_stage::OpenWorldStartPlugin;

/// The world seed and the start role: the interceptor, armed with two point
/// defense mounts.
const SANDBOX_SEED: u32 = 5;
const SANDBOX_ROLE: &str = "bounty_hunter";
/// How far from the starting station the player drifts.
const DRIFT_DISTANCE: Meters = Meters(20_000.0);

/// The intro line.
const INTRO: &str = "Stranded 20 km out: Tab, Ship, Call SOS, then close Tab and wait.";

fn main() -> bevy::app::AppExit {
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_rescue
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Where the staging stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The start's window is streaming in and the start is docking the
    /// player at its starting station.
    #[default]
    Streaming,
    /// The player let go of the starting station.
    Leaving,
    /// The player jumped [`DRIFT_DISTANCE`] off; its window is streaming in.
    Drifting,
    /// The damage landed; the lost sections are leaving the ship.
    Stranding,
    /// The ship is stranded. The player has the run.
    Ready,
}

/// The staging state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The design ids of the sections the fixture puts at zero.
    lost: Vec<String>,
}

/// The live sections of `player` the fixture loses: every flight computer,
/// thruster and docking port, by design id.
fn sections_to_lose(world: &mut World, player: Entity) -> Vec<(Entity, String)> {
    let mut sections = world.query_filtered::<(Entity, &EntityId, &ChildOf), (
        With<SectionMarker>,
        Or<(
            With<ControllerSectionMarker>,
            With<ThrusterSectionMarker>,
            With<DockingSectionMarker>,
        )>,
    )>();
    sections
        .iter(world)
        .filter(|(_, _, child)| child.parent() == player)
        .map(|(entity, id, _)| (entity, id.0.clone()))
        .collect()
}

/// Whether the design section `id` of `player` is live.
fn live(world: &mut World, player: Entity, id: &str) -> bool {
    let mut sections = world.query_filtered::<(&EntityId, &ChildOf), With<SectionMarker>>();
    sections
        .iter(world)
        .any(|(found, child)| found.0 == id && child.parent() == player)
}

/// Put every flight computer, thruster and docking port of `player` at
/// zero, once each carries its health and the ship graph is built. Returns
/// their design ids, or `None` while a section is not ready.
fn strand_the_ship(world: &mut World, player: Entity) -> Option<Vec<String>> {
    let targets = sections_to_lose(world, player);
    // The ship graph is built a frame after the sections spawn and reads
    // every section, so a section despawned before then races its writes.
    let ready = !targets.is_empty()
        && targets.iter().all(|(section, _)| {
            world.get::<Health>(*section).is_some() && world.get::<ConnectedTo>(*section).is_some()
        });
    if !ready {
        return None;
    }
    for (section, _) in &targets {
        world
            .get_mut::<Health>(*section)
            .expect("sos_rescue_sandbox: a section carries its health")
            .current = 0.0;
        world.entity_mut(*section).insert(HealthZeroMarker);
    }
    Some(targets.into_iter().map(|(_, id)| id).collect())
}

/// Advance the staging one phase at a time.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let docked = world.get::<DockedShip>(player).is_some();
    let next = match phase {
        SandboxPhaseType::Ready => return,
        SandboxPhaseType::Streaming => {
            if !(open_world_stage::window_settled(world) && docked) {
                return;
            }
            world.trigger(DockingReleaseRequest { entity: player });
            SandboxPhaseType::Leaving
        }
        SandboxPhaseType::Leaving => {
            if docked {
                return;
            }
            let at = Meters3::from_engine(
                world
                    .get::<Position>(player)
                    .expect("sos_rescue_sandbox: the player has a physics position")
                    .0,
            );
            open_world_stage::park_beside(world, player, at, DRIFT_DISTANCE);
            SandboxPhaseType::Drifting
        }
        SandboxPhaseType::Drifting => {
            // A lost section drops its cargo at its GlobalTransform, which
            // trails the jump's Position write by a frame or more. Damage
            // before it catches up drops the canisters beside the starting
            // station, the one the rescue later docks at.
            let caught_up = world
                .get::<Position>(player)
                .zip(world.get::<GlobalTransform>(player))
                .is_some_and(|(position, global)| position.0.distance(global.translation()) < 1.0);
            if !(caught_up && open_world_stage::window_settled(world)) {
                return;
            }
            let Some(lost) = strand_the_ship(world, player) else {
                return;
            };
            info!("sos_rescue_sandbox: put {lost:?} at zero; TAB opens the interface");
            world.resource_mut::<Sandbox>().lost = lost;
            SandboxPhaseType::Stranding
        }
        SandboxPhaseType::Stranding => {
            let lost = world.resource::<Sandbox>().lost.clone();
            if lost.iter().any(|id| live(world, player, id))
                || rescue_quote(world, player).is_none()
            {
                return;
            }
            info!("sos_rescue_sandbox: the ship is stranded");
            SandboxPhaseType::Ready
        }
    };
    world.resource_mut::<Sandbox>().phase = next;
}

/// The panel: why the ship is stranded, the bill, the balance and the call.
fn draw_the_rescue(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    if phase != SandboxPhaseType::Ready {
        world
            .resource_mut::<DiagnosticsPanel>()
            .line(format!("Staging the stranded ship: {phase:?}"));
        return;
    }
    let Some(player) = open_world_stage::the_player(world) else {
        return;
    };
    let balance = world
        .get::<ShipCredits>(player)
        .map_or(0, |credits| credits.0);
    let mut lines = vec![format!("Balance: {}", cr_text(balance))];
    if let Some(call) = world.get::<ShipRescueCall>(player) {
        lines.push(format!(
            "SOS to {}: {:.0} s left, rebuilds {}",
            call.station,
            call.remaining_secs(),
            call.chain
                .iter()
                .map(|rebuild| rebuild.code.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else if let Some(quote) = rescue_quote(world, player) {
        lines.push(format!("Stranded: {:?}", quote.reasons));
        lines.push(format!(
            "Bill: {} of {} owed, rebuilds {:?}",
            cr_text(quote.bill.charged_cr),
            cr_text(quote.bill.owed_cr),
            quote.chain
        ));
    } else if world.get::<DockedShip>(player).is_some() {
        lines.push("Docked: the ship flies again".to_string());
    }
    let interface = world
        .get_resource::<State<PauseStates>>()
        .is_some_and(|pause| *pause.get() == PauseStates::Interface);
    let mut panel = world.resource_mut::<DiagnosticsPanel>();
    for line in lines {
        panel.line(line);
    }
    if interface {
        panel.hint("Ship", "Call SOS, then close Tab");
    } else {
        panel.hint("TAB", "Ship: Call SOS");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds the world gets to start, stream, jump and strand the ship on
    /// a software GPU.
    const SETUP_SECS: f32 = 800.0;
    /// Seconds any pane beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds the call, the answer and the dock get: the call's game time,
    /// slowed by a software GPU, and the station's window streaming in.
    const RESCUE_WAIT_SECS: f32 = 600.0;
    /// Frames the Ship pane and the jump get to settle.
    const SETTLE_FRAMES: u32 = 10;

    const SOS_BUTTON: &str = "ShipSosCall";

    /// The frames the capture path writes.
    const PANE_SHOT: &str = "sos-rescue-sandbox-pane.png";
    const DOCKED_SHOT: &str = "sos-rescue-sandbox-docked.png";

    /// What the walk recorded before the call.
    #[derive(Resource, Debug)]
    struct Before {
        /// The balance before the click.
        balance_cr: u32,
        /// The charge the pane showed.
        charged_cr: u32,
        /// The sections the quote rebuilds.
        chain: Vec<String>,
        /// The station the call went to, once placed.
        station: Option<GeneratedBodyId>,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn player(world: &World) -> Option<Entity> {
        world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next())
    }

    fn has_call(world: &World) -> bool {
        player(world).is_some_and(|player| world.get::<ShipRescueCall>(player).is_some())
    }

    /// The station the player is docked with, by generated id.
    fn docked_station_id(world: &mut World) -> Option<GeneratedBodyId> {
        let station = station_void::docked_station(world)?;
        world.get::<GeneratedBodyId>(station).cloned()
    }

    /// The staging: undocked, stranded for every reason, neutralized, and
    /// the quote rebuilds a port, a drive and the bridge, which fixes every
    /// reason. Records the
    /// balance and the charge.
    fn assert_the_staging(world: &mut World) {
        let player = open_world_stage::the_player(world).expect("sos_rescue_sandbox: a player");
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "sos_rescue_sandbox: the player drifts undocked"
        );
        assert!(
            world.get::<NeutralizedMarker>(player).is_some(),
            "sos_rescue_sandbox: an armed ship with its computer lost is neutralized"
        );
        let quote = rescue_quote(world, player).expect("sos_rescue_sandbox: the ship is stranded");
        assert_eq!(
            quote.reasons,
            vec![
                StrandedType::NoThruster,
                StrandedType::NoFlightComputer,
                StrandedType::NoDockingPort,
            ],
            "sos_rescue_sandbox: stranded for every reason"
        );
        assert!(
            quote.unfixed.is_empty(),
            "sos_rescue_sandbox: the chain fixes every reason: {:?}",
            quote.unfixed
        );
        assert!(
            quote.chain.len() >= 3,
            "sos_rescue_sandbox: the rescue rebuilds a port, a drive and the bridge: {:?}",
            quote.chain
        );
        let balance_cr = world
            .get::<ShipCredits>(player)
            .expect("sos_rescue_sandbox: the player carries a balance")
            .0;
        info!(
            "sos_rescue_sandbox: PASS stranded {:?}; the bill charges {} of {} owed, \
             balance {}, rebuilds {:?}",
            quote.reasons,
            cr_text(quote.bill.charged_cr),
            cr_text(quote.bill.owed_cr),
            cr_text(balance_cr),
            quote.chain
        );
        world.insert_resource(Before {
            balance_cr,
            charged_cr: quote.bill.charged_cr,
            chain: quote.chain,
            station: None,
        });
    }

    /// The pane placed one call to a station that answers, with the quote's
    /// chain.
    fn assert_the_call(world: &mut World) {
        let player = open_world_stage::the_player(world).expect("sos_rescue_sandbox: a player");
        let call = world
            .get::<ShipRescueCall>(player)
            .expect("sos_rescue_sandbox: the call runs")
            .clone();
        let chain: Vec<String> = call
            .chain
            .iter()
            .map(|rebuild| rebuild.section.clone())
            .collect();
        let mut before = world.resource_mut::<Before>();
        assert_eq!(
            chain, before.chain,
            "sos_rescue_sandbox: the call rebuilds the quoted chain"
        );
        before.station = Some(call.station.clone());
        info!(
            "sos_rescue_sandbox: PASS the pane sent SOS to '{}' with {:.0} s left",
            call.station,
            call.remaining_secs()
        );
    }

    /// Docked at the answering station on a rebuilt port, the bill taken,
    /// the ship flies again and is no longer neutralized, and the pair
    /// stands still.
    fn assert_the_rescue(world: &mut World) {
        let player = open_world_stage::the_player(world).expect("sos_rescue_sandbox: a player");
        let (balance_cr, charged_cr, chain, station) = {
            let before = world.resource::<Before>();
            (
                before.balance_cr,
                before.charged_cr,
                before.chain.clone(),
                before
                    .station
                    .clone()
                    .expect("sos_rescue_sandbox: the call named a station"),
            )
        };
        assert_eq!(
            docked_station_id(world).as_ref(),
            Some(&station),
            "sos_rescue_sandbox: docked at the station that answered"
        );
        for id in &chain {
            assert!(
                live(world, player, id),
                "sos_rescue_sandbox: the rescue rebuilt {id}"
            );
        }
        let now_cr = world
            .get::<ShipCredits>(player)
            .expect("sos_rescue_sandbox: the player carries a balance")
            .0;
        assert_eq!(
            balance_cr - now_cr,
            charged_cr,
            "sos_rescue_sandbox: the rescue took the planned charge"
        );
        let facts = stranded_facts(world, player);
        assert!(
            facts.working_thruster && facts.working_computer,
            "sos_rescue_sandbox: the ship flies again: {facts:?}"
        );
        assert!(
            world.get::<NeutralizedMarker>(player).is_none(),
            "sos_rescue_sandbox: a repaired player is no longer neutralized"
        );
        assert!(
            world.get::<ShipRescueCall>(player).is_none()
                && world.get::<StationBerth>(player).is_none(),
            "sos_rescue_sandbox: the call and the berth ended"
        );
        let station_entity =
            station_void::docked_station(world).expect("sos_rescue_sandbox: a docked station");
        station_void::assert_at_rest(world, player, station_entity);
        info!(
            "sos_rescue_sandbox: PASS rescued by '{station}': paid {}, rebuilt {chain:?}, \
             docked at rest",
            cr_text(charged_cr)
        );
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("sos rescue sandbox: start, drift and strand the ship")
            .until(when(|world| {
                world
                    .get_resource::<Sandbox>()
                    .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
            }))
            .deadline(SETUP_SECS)
            .add()
            .step("sos rescue sandbox: settle the damage")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("sos rescue sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("sos rescue sandbox: open the Ship pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Ship);
            })
            .until(ui_node_present(SOS_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("sos rescue sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "sos rescue sandbox: call SOS",
                SOS_BUTTON,
                when(has_call),
                BEAT_SECS,
            )
            .step("sos rescue sandbox: check the call")
            .on_enter(assert_the_call)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("sos rescue sandbox: shoot the pane")
            .on_enter(|world: &mut World| shoot(world, PANE_SHOT))
            .until(shot_written(PANE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("sos rescue sandbox: close the interface and wait for the dock")
            .on_enter(|world: &mut World| {
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Unpaused);
            })
            .until(when(|world| {
                !has_call(world)
                    && player(world).is_some_and(|player| {
                        world.get::<DockedShip>(player).is_some()
                            && world.get::<StationBerth>(player).is_none()
                    })
            }))
            .deadline(RESCUE_WAIT_SECS)
            .add()
            .step("sos rescue sandbox: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("sos rescue sandbox: check the rescue")
            .on_enter(assert_the_rescue)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("sos rescue sandbox: shoot the docked view")
            .on_enter(|world: &mut World| shoot(world, DOCKED_SHOT))
            .until(shot_written(DOCKED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }
}
