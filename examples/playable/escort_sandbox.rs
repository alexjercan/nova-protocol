//! escort_sandbox: fly with two hired escorts in open space, and dock with
//! one.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the bounty
//! hunter, as the Create World modal does, but without a save folder. When
//! the origin's window has streamed in, it picks the nearest generated
//! station that is living, not on the Enemy side, offers Refit and hires out
//! escorts, and berths the player at it until the dock takes. Then the
//! fixture lands, once:
//!
//! - the station's civilization is warmed to Ally by merits, the way a player
//!   who hands in its jobs would, since the hire gate needs an Ally;
//! - the player holds [`PLAYER_CREDITS`], enough for two hires;
//! - two escorts are hired through [`EscortHireCommand`], the message the
//!   Station tab's Hire button writes, one at a time;
//! - the player undocks, and the player and both escorts jump
//!   [`CLEAR_DISTANCE`] straight out from the station together, at rest, so
//!   the formation starts in open space.
//!
//! From there it is yours. Nothing ends the run.
//!
//! What to try:
//!
//! 1. burn ahead (`W`), turn and stop (`X`): each escort flies its own drive
//!    to its slot, starboard and port, behind your travel;
//! 2. stop and hold still: inside 60 m of its slot an escort trims on its
//!    RCS, and the gas puffs show the push;
//! 3. lock an escort (hold `Ctrl` on it) inside its dock-hold radius: it
//!    holds still, and you can fly your port onto its port and dock.
//!
//! ```text
//! nix develop --command cargo run --example escort_sandbox --features debug
//! ```
//!
//! The diagnostics (`F3`) draw each escort's formation slot and the line to
//! it, the 60 m trim zone round the slot, each escort's velocity and the
//! leader's velocity as arrows, the dock-hold radius round the leader, every
//! hull's containment radius (red where two overlap), and every RCS push as
//! an arrow. Each slot line carries its distance (green inside the trim
//! zone), a locked escort reads LOCKED, and `Ctrl` is hinted while no escort
//! is locked. Detail (`Shift+F3`) lists, per escort, its closing speed, its
//! speed, its drive and RCS, and its dock-hold distance.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for both escorts
//! in the fleet in open space, checks them, lets the formation settle, and
//! shoots `escort-sandbox.png` with `NOVA_CAPTURE=1`. Unset, no walk and no
//! deadline run.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use avian3d::prelude::{AngularVelocity, ComputedCenterOfMass, LinearVelocity, Position};
use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{
    nova_debug::diagnostics,
    nova_interface::station::{EscortHireCommand, StationRuntime},
    prelude::*,
};
use open_world_stage::{
    advance_station_dock, the_player, warm_to_ally, OpenWorldStartPlugin, StationCandidate,
    StationDock, StationDockPhaseType,
};

#[derive(Parser)]
#[command(name = "escort_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Fly with two escorts hired at a generated station, in open space, and dock with one",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_hire`'s, whose window around the
/// origin holds a living station that hires out escorts, warmed to Ally by
/// the fixture.
const SANDBOX_SEED: u32 = 10;
/// The start role the player flies.
const SANDBOX_ROLE: &str = "bounty_hunter";
/// The player's balance once docked: past two hires.
const PLAYER_CREDITS: u32 = 1_000_000;
/// How many escorts the fixture hires: one per fleet slot.
const ESCORTS: usize = 2;
/// How far the pair jumps straight out from the station after the undock.
const CLEAR_DISTANCE: Meters = Meters(2_000.0);
/// The trim zone round a slot, world units (60 m). Mirrors nova_ship's
/// private `AI_FOLLOW_TRIM.0`; the overlay draws it, nothing reads it.
const TRIM_ZONE: f32 = 6.0;
/// The far hold gap, world units (400 m). Mirrors `AI_FOLLOW_HOLD.1`: a
/// locked escort this far past both hulls' radii holds still for a dock.
const DOCK_HOLD_GAP: f32 = 40.0;
/// Seconds of velocity an arrow draws.
const VELOCITY_ARROW_SECS: f32 = 2.0;
/// World units an RCS arrow draws at full deflection.
const RCS_ARROW: f32 = 4.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.into(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "fly with your escorts; lock one and dock port to port",
    ));
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        (draw_the_fleet, draw_the_rcs)
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Update, open_world_stage::hold_the_walk_clock);
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// Where the sandbox's setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// Streaming in and docking at the station.
    #[default]
    Docking,
    /// Docked; hiring one escort at a time, once the station is on the
    /// Player side.
    Hiring,
    /// Both escorts hired; undocking.
    Undocking,
    /// In open space with the fleet. The player has the run.
    Ready,
}

/// The sandbox's setup.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    dock: StationDock,
    /// The chosen station's civilization has been warmed to Ally, once.
    warmed: bool,
    /// Hires asked for so far.
    asked: usize,
}

/// The escorts in the player's fleet.
fn fleet(world: &mut World) -> Vec<Entity> {
    let mut fleet: Vec<Entity> = world
        .query_filtered::<Entity, (With<PlayerFleetMarker>, With<AIFollow>)>()
        .iter(world)
        .collect();
    fleet.sort();
    fleet
}

/// The first escort for hire coasting beside `station`.
fn hire_offer(world: &World, station: Entity) -> Option<Entity> {
    world.get::<ParkedHulls>(station)?.iter().find(|hull| {
        matches!(
            world.get::<ParkedHullType>(*hull),
            Some(ParkedHullType::Hire { .. })
        )
    })
}

/// Advance the setup one phase at a time.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SandboxPhaseType::Docking => {
            let mut dock = std::mem::take(&mut world.resource_mut::<Sandbox>().dock);
            let docked = advance_station_dock(
                world,
                &mut dock,
                "living, non-Enemy station with Refit that hires out escorts",
                |station: &StationCandidate<'_>| {
                    station.offers_service(StationServiceType::Refit)
                        && station.not_enemy()
                        && station.hires_escorts
                },
            );
            let phase = dock.phase;
            let station = dock.station.clone();
            world.resource_mut::<Sandbox>().dock = dock;
            if !world.resource::<Sandbox>().warmed
                && matches!(
                    phase,
                    StationDockPhaseType::Docking | StationDockPhaseType::Docked
                )
            {
                if let Some(station) =
                    station.and_then(|chosen| open_world_stage::body_by_id(world, &chosen.id))
                {
                    warm_to_ally(world, station, "escort sandbox");
                    world.resource_mut::<Sandbox>().warmed = true;
                }
            }
            if let Some((player, _)) = docked {
                if let Some(mut credits) = world.get_mut::<ShipCredits>(player) {
                    credits.0 = PLAYER_CREDITS;
                }
                info!("escort sandbox: docked; the player holds {PLAYER_CREDITS} cr");
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Hiring;
            }
        }
        SandboxPhaseType::Hiring => {
            let mut dock = world.resource::<Sandbox>().dock.clone();
            let Some((player, station)) = advance_station_dock(world, &mut dock, "", |_| false)
            else {
                return;
            };
            let hired = fleet(world).len();
            if hired >= ESCORTS {
                world.trigger(DockingReleaseRequest { entity: player });
                world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Undocking;
                info!("escort sandbox: {hired} escorts hired; undocking");
                return;
            }
            if let Some(note) = world.resource::<StationRuntime>().notice.text() {
                if note.starts_with("hire:") {
                    panic!("escort sandbox: the station refused the hire: '{note}'");
                }
            }
            if world.get::<Allegiance>(station) != Some(&Allegiance::Player) {
                return;
            }
            // One hire at a time, each once the last escort stands: a hire
            // before the last escort spawns plans the same fleet slot.
            if hired == world.resource::<Sandbox>().asked {
                if let Some(hull) = hire_offer(world, station) {
                    world.write_message(EscortHireCommand { hull });
                    world.resource_mut::<Sandbox>().asked += 1;
                }
            }
        }
        SandboxPhaseType::Undocking => {
            let Some(player) = the_player(world) else {
                return;
            };
            if world.get::<DockedShip>(player).is_some() {
                return;
            }
            let Some(station) = world
                .resource::<Sandbox>()
                .dock
                .station
                .clone()
                .and_then(|chosen| open_world_stage::body_by_id(world, &chosen.id))
            else {
                return;
            };
            let from = world.get::<Position>(station).expect("a station stands").0;
            let at = world.get::<Position>(player).expect("a ship stands").0;
            let offset = (at - from).normalize_or(Vec3::Y) * CLEAR_DISTANCE.to_engine();
            let mut roots = vec![player];
            roots.extend(fleet(world));
            for root in roots {
                let at = world.get::<Position>(root).expect("a ship stands").0;
                world.entity_mut(root).insert((
                    Position(at + offset),
                    LinearVelocity::ZERO,
                    AngularVelocity::ZERO,
                ));
            }
            world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Ready;
            info!(
                "escort sandbox: the player and its fleet jumped {CLEAR_DISTANCE:?} clear; the run \
                 is yours"
            );
        }
        SandboxPhaseType::Ready => {}
    }
}

/// A ship's live anchor: its transform about its centre of mass.
fn anchor(transform: &GlobalTransform, com: Option<&ComputedCenterOfMass>) -> Vec3 {
    let transform = transform.compute_transform();
    live_structure_anchor(&transform, com)
}

/// Each escort's slot, trim zone, velocity and the line to its slot, the
/// leader's velocity and dock-hold radius, every hull's radius, and the
/// panel rows that number them.
fn draw_the_fleet(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    q_leader: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&ComputedCenterOfMass>,
            Option<&LinearVelocity>,
            &HullRadius,
            Option<&TravelLock>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_fleet: Query<
        (
            Entity,
            &Name,
            &AIFollow,
            &GlobalTransform,
            Option<&ComputedCenterOfMass>,
            Option<&LinearVelocity>,
            &HullRadius,
            Option<&RcsBudget>,
        ),
        With<PlayerFleetMarker>,
    >,
    q_drives: Query<(&ChildOf, &ThrusterSectionInput)>,
    q_hulls: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&ComputedCenterOfMass>,
            &HullRadius,
        ),
        With<SpaceshipRootMarker>,
    >,
) {
    if sandbox.phase != SandboxPhaseType::Ready {
        panel.line(format!("setup: {:?}", sandbox.phase));
    }
    let Ok((leader, leader_at, leader_com, leader_velocity, leader_radius, lock)) =
        q_leader.single()
    else {
        return;
    };
    let leader_anchor = anchor(leader_at, leader_com);
    let leader_velocity = leader_velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
    let leader_up = leader_at.rotation() * Vec3::Y;
    diagnostics::vector(
        &mut gizmos,
        leader_anchor,
        leader_velocity,
        VELOCITY_ARROW_SECS,
        palette::VELOCITY,
    );
    if sandbox.phase == SandboxPhaseType::Ready {
        if lock.and_then(|lock| lock.0).is_none() {
            panel.hint("Ctrl", "hold on an escort to lock it");
        } else {
            panel.hint("X", "stop, then fly port to port");
        }
    }
    panel.detail(format!(
        "leader: {:.1} m/s, locked on {}",
        Meters::from_engine(leader_velocity.length()).get(),
        lock.and_then(|lock| lock.0)
            .and_then(|locked| q_fleet.get(locked).ok())
            .map_or("no escort".to_string(), |(_, name, ..)| name.to_string())
    ));

    for (escort, name, follow, at, com, velocity, radius, rcs) in &q_fleet {
        if follow.leader != leader {
            continue;
        }
        let from = anchor(at, com);
        let velocity = velocity.map_or(Vec3::ZERO, |velocity| velocity.0);
        let arms = radius.0 + leader_radius.0;
        let slot = formation_goal(
            from,
            leader_anchor,
            leader_velocity,
            leader_up,
            follow.side,
            arms,
        );
        let to_slot = slot - from;
        let distance = to_slot.length();
        let closing = (velocity - leader_velocity).dot(to_slot.normalize_or_zero());
        let in_trim = distance <= TRIM_ZONE;
        gizmos.line(
            from,
            slot,
            if in_trim {
                palette::OK
            } else {
                palette::TARGET
            },
        );
        diagnostics::sphere(&mut gizmos, slot, 0.6, palette::TARGET);
        diagnostics::ring(&mut gizmos, slot, leader_up, TRIM_ZONE, palette::ZONE);
        diagnostics::vector(
            &mut gizmos,
            from,
            velocity,
            VELOCITY_ARROW_SECS,
            palette::VELOCITY,
        );
        let hold = arms + DOCK_HOLD_GAP;
        let locked = lock.is_some_and(|lock| lock.0 == Some(escort));
        diagnostics::ring(
            &mut gizmos,
            leader_anchor,
            leader_up,
            hold,
            if locked && from.distance(leader_anchor) < hold {
                palette::OK
            } else {
                palette::MUTED
            },
        );
        let drive = q_drives
            .iter()
            .filter(|(parent, input)| parent.parent() == escort && input.0 > 0.0)
            .count();
        let rcs = rcs.map_or(0.0, |budget| budget.applied);
        let side = if follow.side.x >= 0.0 {
            "starboard"
        } else {
            "port"
        };
        panel.label_tinted(
            from.midpoint(slot),
            format!("{:.0} m", Meters::from_engine(distance).get()),
            if in_trim {
                palette::OK
            } else {
                palette::TARGET
            },
        );
        if locked {
            panel.label_tinted(from, "LOCKED", palette::OK);
        }
        panel.detail(format!(
            "{name} ({side}): {:.0} m to slot{}, closing {:+.1} m/s, speed {:.1} m/s, drive {}, \
             RCS {:.0}%",
            Meters::from_engine(distance).get(),
            if in_trim { " (trim zone)" } else { "" },
            Meters::from_engine(closing).get(),
            Meters::from_engine(velocity.length()).get(),
            if drive > 0 {
                format!("on ({drive})")
            } else {
                "off".to_string()
            },
            rcs * 100.0,
        ));
        panel.detail(format!(
            "    dock hold: {:.0} m from you of {:.0} m{}",
            Meters::from_engine((from - leader_anchor).length()).get(),
            Meters::from_engine(hold).get(),
            if locked {
                ", LOCKED: holds for a dock"
            } else {
                ""
            }
        ));
    }

    // Spawn clearance: every hull's containment radius, red where two meet.
    let hulls: Vec<(Entity, Vec3, f32)> = q_hulls
        .iter()
        .map(|(hull, at, com, radius)| (hull, anchor(at, com), radius.0))
        .collect();
    let near = |centre: Vec3| centre.distance(leader_anchor) < 300.0;
    let mut overlaps = 0;
    for (hull, centre, radius) in &hulls {
        if !near(*centre) {
            continue;
        }
        let touching = hulls.iter().any(|(other, other_centre, other_radius)| {
            other != hull && centre.distance(*other_centre) < radius + other_radius
        });
        overlaps += usize::from(touching);
        if touching {
            panel.label_tinted(*centre, "overlap", palette::ALERT);
        }
        diagnostics::sphere(
            &mut gizmos,
            *centre,
            *radius,
            if touching {
                palette::ALERT
            } else {
                palette::MUTED
            },
        );
    }
    panel.detail(format!(
        "hull radii within 3 km: {overlaps} overlapping (red)"
    ));
}

/// Every RCS push as an arrow along the push, and the puff count.
fn draw_the_rcs(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    q_ships: Query<(
        &GlobalTransform,
        Option<&ComputedCenterOfMass>,
        &RcsBudget,
        &RcsIntent,
    )>,
    q_puffs: Query<(), With<RcsPuffMarker>>,
) {
    let mut pushing = 0;
    for (at, com, budget, intent) in &q_ships {
        if budget.applied <= 0.0 {
            continue;
        }
        pushing += 1;
        let push = at.rotation() * intent.0.normalize_or_zero();
        diagnostics::vector(
            &mut gizmos,
            anchor(at, com),
            push,
            RCS_ARROW * budget.applied,
            palette::PUSH,
        );
    }
    panel.detail(format!(
        "RCS: {pushing} ships pushing (orange arrows), {} puffs in flight",
        q_puffs.iter().count()
    ));
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{shoot, AutopilotPlugin};

    use super::*;

    /// Seconds the world gets to start, stream, dock and hire on a software
    /// GPU.
    const SETUP_SECS: f32 = 900.0;
    /// Seconds a shot gets to land.
    const SHOT_SECS: f32 = 120.0;
    /// Frames the formation gets before the shot.
    const SETTLE_FRAMES: u32 = 20;
    /// The PNG of the fleet in open space.
    const SHOT: &str = "escort-sandbox.png";

    fn ready(world: &World) -> bool {
        world
            .get_resource::<Sandbox>()
            .is_some_and(|sandbox| sandbox.phase == SandboxPhaseType::Ready)
    }

    /// Both escorts follow the undocked player, and no hull overlaps another.
    fn assert_the_fleet(world: &mut World) {
        let player = the_player(world).expect("escort sandbox: one player ship");
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "escort sandbox: the player is undocked"
        );
        let escorts = fleet(world);
        assert_eq!(escorts.len(), ESCORTS, "escort sandbox: the fleet");
        let mut sides = Vec::new();
        for escort in &escorts {
            let follow = world
                .get::<AIFollow>(*escort)
                .expect("escort sandbox: an escort follows");
            assert_eq!(
                follow.leader, player,
                "escort sandbox: it follows the player"
            );
            sides.push(follow.side);
        }
        assert_ne!(sides[0], sides[1], "escort sandbox: one escort per slot");
        info!("escort sandbox: PASS {ESCORTS} escorts follow the player on sides {sides:?}");
    }

    /// The smoke walk: the fleet stands in open space, and a frame of it with
    /// the diagnostics up is shot.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("escort sandbox: dock, hire two escorts and fly clear")
            .until(Arc::new(ready))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("escort sandbox: check the fleet")
            .on_enter(assert_the_fleet)
            .add()
            .step("escort sandbox: settle the formation")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("escort sandbox: shoot the fleet")
            .on_enter(|world: &mut World| shoot(world, SHOT))
            .until(shot_written(SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
