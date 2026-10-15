//! recycle_sandbox: start docked at a generated wreck, strip the whole hull
//! by hand from the Inventory pane, port last, and loot its hold.
//!
//! Boots the shipped app and starts New Game on a pinned seed as the
//! scavenger, as the Create World modal does after the role page, but
//! without a save folder: nothing this run does is saved. When the window
//! has streamed in, it picks the nearest generated derelict ship that may be
//! looted and has a free port: the wreck `system_station_strip` strips. The
//! player jumps beside it and is put on a berth at its collar each frame
//! until the dock takes. No other fixture lands.
//!
//! What to try:
//!
//! 1. TAB opens the interface; on the Inventory pane the strip block sits
//!    over the wreck's column (the Station tab does not show: this is no
//!    living station). It names the sections left, the parts the whole hull
//!    is expected to give and the time it takes;
//! 2. Strip starts the strip; shut the interface and fly on docked: one
//!    section comes off about every few seconds, the farthest from the port
//!    first, and the HUD chip reads `STRIP n%`;
//! 3. the berth port comes off last: the wreck is gone and the player floats
//!    free;
//! 4. on another run, undock part-way (the docking key): the strip stops and
//!    what it took stays taken; dock again and Strip takes the rest;
//! 5. Take all in the wreck's column header takes every stack that fits and
//!    the credits, and names what stayed; Take credits takes the credits.
//!
//! ```text
//! nix develop --command cargo run --example recycle_sandbox --features debug
//! ```
//!
//! The diagnostics panel (`F3`, on at start) shows three lines: the strip's
//! progress and time left, the wreck's state with its sections left, and the
//! player's hold. `Shift+F3` adds the full list: the section coming off and
//! the time to it, the parts yielded, the last section off and the free hold.
//! The gizmos mark each section still on the wreck by its place in the strip
//! order, from green (first off) to violet (late); the next one is ringed in
//! yellow with its seconds left, and the berth port in red, labelled `last`.
//!
//! Under `NOVA_AUTOPILOT=1` a smoke walk waits for the dock,
//! opens the Inventory pane, presses Strip, shuts the interface, waits for the
//! first section to come off and shoots `recycle-sandbox.png` with
//! `NOVA_CAPTURE=1`. Unset, no walk and no deadline run.

#[path = "shared/open_world_stage.rs"]
mod open_world_stage;

use std::collections::{BTreeMap, BTreeSet};

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{
    nova_debug::diagnostics, nova_events::engine::GameEvent,
    nova_interface::ship::design_section_code, prelude::*,
};
use open_world_stage::{
    berth_and_dock, body_by_id, cell_of, free_ports, park_beside, the_player, window_is,
    OpenWorldStartPlugin,
};

#[derive(Parser)]
#[command(name = "recycle_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a generated wreck: strip the whole hull from the Inventory pane, \
             port last, and loot its hold",
    long_about = None
)]
struct Cli;

/// The world seed: `system_station_strip`'s, whose window around the spawn
/// holds a lootable generated derelict ship with a free port.
const SANDBOX_SEED: u32 = 115;
/// The start role the player spawns as: the scavenger's salvage hauler, a
/// compact box with a deep hold.
const SANDBOX_ROLE: &str = "scavenger";
/// How far from the wreck the player parks before the berth.
const PARK_DISTANCE: Meters = Meters(2_000.0);
/// The radius of a section's marker, engine units: about half a cell.
const SECTION_MARK: f32 = 0.55;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.add_plugins(OpenWorldStartPlugin {
        seed: SANDBOX_SEED,
        role: SANDBOX_ROLE.to_string(),
    });
    app.init_resource::<Sandbox>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(
        "Docked at a wreck: strip it and loot it from the Inventory pane.",
    ));
    app.add_observer(count_the_strip);
    app.add_systems(
        Update,
        drive_the_sandbox.run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(
        PostUpdate,
        draw_the_strip
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

/// Where the setup stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SetupPhaseType {
    /// The window around the spawn is streaming in.
    #[default]
    Streaming,
    /// The player jumped beside the wreck; its window is streaming in.
    Jumped,
    /// The player is put on the berth each frame until the dock takes.
    Docking,
    /// Docked once; from here the player plays.
    Docked,
}

/// The sandbox's setup and what the strip did so far.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SetupPhaseType,
    /// The chosen wreck's generated id, its cell and where it stood.
    wreck: Option<(GeneratedBodyId, SectorCoord, Meters3)>,
    /// The wreck's section count at the first dock.
    sections: usize,
    /// Each `OnStripped` section code, in order.
    taken: Vec<String>,
    /// The parts every `OnStripped` carried, summed.
    yielded: BTreeMap<String, u64>,
}

/// Choose the wreck, jump beside it and berth until the dock takes, once.
fn drive_the_sandbox(world: &mut World) {
    let phase = world.resource::<Sandbox>().phase;
    match phase {
        SetupPhaseType::Streaming => {
            if !window_is(world, SectorCoord::ORIGIN) {
                return;
            }
            let wreck = nearest_wreck(world)
                .expect("recycle sandbox: the spawn's window holds a lootable derelict ship");
            let player = the_player(world).expect("recycle sandbox: one player ship");
            park_beside(world, player, wreck.2, PARK_DISTANCE);
            info!(
                "recycle sandbox: chose the wreck '{}' in cell {}",
                wreck.0, wreck.1
            );
            let mut sandbox = world.resource_mut::<Sandbox>();
            sandbox.wreck = Some(wreck);
            sandbox.phase = SetupPhaseType::Jumped;
        }
        SetupPhaseType::Jumped => {
            let Some((id, cell, _)) = world.resource::<Sandbox>().wreck.clone() else {
                return;
            };
            let stands = body_by_id(world, &id).is_some_and(|wreck| {
                world.get::<Position>(wreck).is_some()
                    && world.get::<ShipDesignSections>(wreck).is_some()
                    && world.get::<Children>(wreck).is_some()
            });
            if window_is(world, cell) && stands {
                world.resource_mut::<Sandbox>().phase = SetupPhaseType::Docking;
            }
        }
        SetupPhaseType::Docking => {
            let Some((id, ..)) = world.resource::<Sandbox>().wreck.clone() else {
                return;
            };
            let (Some(player), Some(wreck)) = (the_player(world), body_by_id(world, &id)) else {
                return;
            };
            if world.get::<DockedShip>(player).is_none() {
                berth_and_dock(world, player, wreck);
                return;
            }
            let sections = live_sections(world, wreck).len();
            info!("recycle sandbox: docked at the wreck, {sections} sections");
            let mut sandbox = world.resource_mut::<Sandbox>();
            sandbox.sections = sections;
            sandbox.phase = SetupPhaseType::Docked;
        }
        SetupPhaseType::Docked => {}
    }
}

/// The lootable generated derelict ship with a free port nearest the
/// origin, ties broken by id.
fn nearest_wreck(world: &mut World) -> Option<(GeneratedBodyId, SectorCoord, Meters3)> {
    let seen: Vec<(Entity, GeneratedBodyId, Vec3)> = world
        .query_filtered::<(Entity, &GeneratedBodyId, &Position), (
            With<DerelictShipMarker>,
            With<LootableShipMarker>,
            With<ShipDesignSections>,
            Without<StationMarker>,
        )>()
        .iter(world)
        .map(|(entity, id, position)| (entity, id.clone(), position.0))
        .collect();
    let mut wrecks: Vec<(f32, GeneratedBodyId, Vec3)> = seen
        .into_iter()
        .filter(|(entity, ..)| !free_ports(world, *entity).is_empty())
        .map(|(_, id, at)| {
            let distance = Meters3::from_engine(at).distance(Meters3::ZERO).get();
            (distance, id, at)
        })
        .collect();
    wrecks.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let (_, id, at) = wrecks.into_iter().next()?;
    Some((id, cell_of(world, at)?, Meters3::from_engine(at)))
}

/// The live sections of `root`.
fn live_sections(world: &World, root: Entity) -> Vec<Entity> {
    world
        .get::<Children>(root)
        .into_iter()
        .flatten()
        .copied()
        .filter(|child| world.get::<SectionMarker>(*child).is_some())
        .collect()
}

/// Count each section a strip takes and the parts it gives.
fn count_the_strip(event: On<GameEvent>, mut sandbox: ResMut<Sandbox>) {
    if event.name() != "onstripped" {
        return;
    }
    let Some(data) = event.info().data.clone() else {
        return;
    };
    if let Some(section) = data["section"].as_str() {
        sandbox.taken.push(section.to_string());
    }
    if let Some(parts) = data["parts"].as_object() {
        for (item, count) in parts {
            *sandbox.yielded.entry(item.clone()).or_default() += count.as_u64().unwrap_or(0);
        }
    }
}

/// The pane's tie order for a section code: `HULL-12` as `("HULL", 12)`.
fn code_order(code: &str) -> (String, u32) {
    match code.rsplit_once('-') {
        Some((kind, number)) => (kind.to_string(), number.parse().unwrap_or(u32::MAX)),
        None => (code.to_string(), u32::MAX),
    }
}

/// The order a strip takes `live` in, by [`next_strip_section`] over what
/// is left after each, ties in pane code order: the strip block's rule.
fn strip_order(berth: Entity, live: &[(Entity, String, Vec<Entity>)]) -> Vec<Entity> {
    let mut left: BTreeSet<Entity> = live.iter().map(|(entity, ..)| *entity).collect();
    let mut ranked: Vec<&(Entity, String, Vec<Entity>)> = live.iter().collect();
    ranked.sort_by_key(|(_, code, _)| code_order(code));
    let mut order = Vec::with_capacity(live.len());
    while !left.is_empty() {
        let graph: Vec<(Entity, usize, Vec<Entity>)> = ranked
            .iter()
            .enumerate()
            .filter(|(_, (entity, ..))| left.contains(entity))
            .map(|(rank, (entity, _, neighbours))| {
                let kept = neighbours
                    .iter()
                    .copied()
                    .filter(|neighbour| left.contains(neighbour))
                    .collect();
                (*entity, rank, kept)
            })
            .collect();
        let Some(next) = next_strip_section(
            berth,
            graph
                .iter()
                .map(|(entity, rank, neighbours)| (*entity, *rank, neighbours.as_slice())),
        ) else {
            break;
        };
        order.push(next);
        left.remove(&next);
    }
    order
}

/// The wreck's sections by strip order, and the panel reading the strip,
/// the hold and the wreck's state.
#[expect(
    clippy::too_many_arguments,
    reason = "one panel reading the wreck, its sections, the dock and the player's hold"
)]
fn draw_the_strip(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    sandbox: Res<Sandbox>,
    items: Option<Res<GameItems>>,
    q_player: Query<
        (Option<&DockedShip>, Option<&HullStripJob>, &ShipInventory),
        With<PlayerSpaceshipMarker>,
    >,
    q_connections: Query<&DockingConnection>,
    q_wrecks: Query<(Entity, &GeneratedBodyId, &ShipDesignSections, &Children)>,
    q_sections: Query<(&GlobalTransform, &EntityId, Option<&ConnectedTo>), With<SectionMarker>>,
) {
    let Some((wreck_id, ..)) = sandbox.wreck.as_ref() else {
        panel.line(format!("setup: {:?}", sandbox.phase));
        return;
    };
    let Ok((docked, job, hold)) = q_player.single() else {
        panel.line("setup: no player ship");
        return;
    };
    let wreck = q_wrecks.iter().find(|(_, id, ..)| *id == wreck_id);
    let connection = docked.and_then(|docked| q_connections.get(docked.connection).ok());
    let berth = connection.and_then(|connection| {
        let wreck = wreck?.0;
        if connection.first_ship == wreck {
            Some(connection.first_section)
        } else if connection.second_ship == wreck {
            Some(connection.second_section)
        } else {
            None
        }
    });

    // The wreck's state, first: the rest reads it.
    let wreck_state = match (wreck.is_some(), berth.is_some()) {
        (_, true) => "docked",
        (true, false) => "standing, not docked",
        (false, false) if sandbox.taken.is_empty() => "out of the window",
        (false, false) => "demolished",
    };
    let player_state = match (docked.is_some(), berth.is_some()) {
        (true, true) => "docked at the wreck",
        (true, false) => "docked elsewhere",
        (false, _) => "released",
    };
    let live: Vec<(Entity, String, Vec<Entity>)> = wreck
        .map(|(_, _, design, children)| {
            children
                .iter()
                .filter_map(|child| {
                    let (_, id, connected) = q_sections.get(child).ok()?;
                    let code = design_section_code(design, &id.0).unwrap_or_else(|| id.0.clone());
                    let neighbours =
                        connected.map_or_else(Vec::new, |connected| connected.0.clone());
                    Some((child, code, neighbours))
                })
                .collect()
        })
        .unwrap_or_default();
    let total = sandbox.sections.max(live.len());

    // The short panel: the strip, the wreck, the hold.
    panel.line(match job {
        Some(job) if job.paused.is_some() => "strip HELD: hold full".to_string(),
        Some(job) => format!(
            "strip {:.0}%, {:.0} s left",
            job.progress() * 100.0,
            job.remaining_secs()
        ),
        None => "strip idle".to_string(),
    });
    panel.line(format!(
        "wreck {wreck_state}, {} of {total} sections",
        live.len()
    ));
    if let Some(items) = items.as_deref() {
        panel.line(format!(
            "hold {:.1} / {:.1} t",
            f64::from(hold.used_g(items)) / 1.0e6,
            f64::from(hold.capacity_g()) / 1.0e6,
        ));
    }
    if berth.is_some() && job.is_none() {
        panel.hint("TAB", "Inventory: Strip or Take all");
    }

    // The full value list, behind Shift+F3.
    panel.detail(format!("wreck: {wreck_id}"));
    panel.detail(format!(
        "player: {player_state}   setup: {:?}",
        sandbox.phase
    ));
    if let Some(job) = job {
        let next = job.step.as_ref().map_or("choosing".to_string(), |step| {
            format!(
                "{} in {:.1} s",
                step.code,
                (step.duration_secs - step.elapsed_secs).max(0.0)
            )
        });
        panel.detail(format!("next off: {next}"));
        if let Some(pause) = job.paused {
            panel.detail(format!("held: {pause:?}"));
        }
    }
    panel.detail(format!("taken: {}", sandbox.taken.len()));
    if let Some(last) = sandbox.taken.last() {
        panel.detail(format!("last off: {last}"));
    }
    let yielded = if sandbox.yielded.is_empty() {
        "nothing yet".to_string()
    } else {
        sandbox
            .yielded
            .iter()
            .map(|(item, count)| format!("{count} {item}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    panel.detail(format!("yielded: {yielded}"));
    if let Some(items) = items.as_deref() {
        panel.detail(format!(
            "hold free: {:.1} t",
            f64::from(hold.free_g(items)) / 1.0e6
        ));
    }

    let Some(berth) = berth else {
        return;
    };
    let order = strip_order(berth, &live);
    let next = job
        .and_then(|job| job.step.as_ref())
        .map(|step| step.section)
        .or_else(|| order.first().copied());
    let codes: BTreeMap<Entity, &str> = live
        .iter()
        .map(|(entity, code, _)| (*entity, code.as_str()))
        .collect();
    let late = order.len().saturating_sub(1).max(1) as f32;
    for (rank, section) in order.iter().enumerate() {
        let Ok((at, ..)) = q_sections.get(*section) else {
            continue;
        };
        let at = at.translation();
        let color = if *section == berth {
            palette::ALERT
        } else if Some(*section) == next {
            palette::TARGET
        } else {
            palette::OK.mix(&palette::ZONE, rank as f32 / late)
        };
        let radius = if Some(*section) == next {
            SECTION_MARK * 1.4
        } else {
            SECTION_MARK
        };
        diagnostics::sphere(&mut gizmos, at, radius, color);
        let code = codes.get(section).copied().unwrap_or("?");
        if *section == berth {
            panel.label_tinted(at, format!("{code} last"), palette::ALERT);
        } else if Some(*section) == next {
            let secs = job
                .and_then(|job| job.step.as_ref())
                .filter(|step| step.section == *section)
                .map_or(String::new(), |step| {
                    format!(
                        " {:.1} s",
                        (step.duration_secs - step.elapsed_secs).max(0.0)
                    )
                });
            panel.label_tinted(at, format!("{code} next{secs}"), palette::TARGET);
        } else {
            panel.label(at, format!("{}", rank + 1));
        }
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::{inventory::INVENTORY_STRIP_BUTTON, pane::InterfacePaneType},
    };

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds one beat gets.
    const BEAT_SECS: f32 = 240.0;
    /// Frames the scene gets to settle before the shot.
    const SETTLE_FRAMES: u32 = 10;
    /// The frame the walk shoots.
    const SHOT: &str = "recycle-sandbox.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn docked(world: &World) -> bool {
        world
            .get_resource::<Sandbox>()
            .is_some_and(|sandbox| sandbox.phase == SetupPhaseType::Docked)
    }

    fn job(world: &mut World) -> Option<HullStripJob> {
        world
            .query_filtered::<&HullStripJob, With<PlayerSpaceshipMarker>>()
            .iter(world)
            .next()
            .cloned()
    }

    fn pause_is(state: PauseStates) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == state)
        })
    }

    fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
                && world
                    .get_resource::<InterfacePaneType>()
                    .is_some_and(|shown| *shown == pane)
        })
    }

    /// Press and release a key action, one frame each.
    fn tap(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        action: &'static str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("recycle sandbox: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("recycle sandbox: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(BEAT_SECS)
            .add()
    }

    /// The smoke walk: dock, Strip from the Inventory pane, shut the interface,
    /// wait for a section to come off and shoot the overlay.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("recycle sandbox: dock at the wreck")
            .until(when(docked))
            .diagnose(|world| format!("{:?}", world.get_resource::<Sandbox>()))
            .deadline(SETUP_SECS)
            .add()
            .step("recycle sandbox: settle the dock")
            .until(frames(SETTLE_FRAMES))
            .add();
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            pause_is(PauseStates::Interface),
        )
        .click_named(
            "recycle sandbox: open the Inventory pane",
            "InterfaceTabInventory",
            the_interface_shows(InterfacePaneType::Inventory),
            BEAT_SECS,
        )
        .step("recycle sandbox: settle the pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "recycle sandbox: press Strip",
            INVENTORY_STRIP_BUTTON,
            Arc::new(|world: &World| {
                world
                    .iter_entities()
                    .any(|entity| entity.contains::<HullStripJob>())
            }),
            BEAT_SECS,
        );
        tap(
            script,
            "the interface key again",
            "interface_toggle",
            pause_is(PauseStates::Unpaused),
        )
        .step("recycle sandbox: a section comes off")
        .until(when(|world| {
            world
                .get_resource::<Sandbox>()
                .is_some_and(|sandbox| !sandbox.taken.is_empty())
        }))
        .deadline(BEAT_SECS)
        .add()
        .step("recycle sandbox: settle")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("recycle sandbox: check the strip runs")
        .on_enter(|world: &mut World| {
            let job = job(world).expect("recycle sandbox: the strip still runs");
            assert!(job.taken >= 1, "recycle sandbox: a section came off");
            info!(
                "recycle sandbox: PASS the strip runs with the interface shut, {} taken",
                job.taken
            );
        })
        .add()
        .step("recycle sandbox: shoot the overlay")
        .on_enter(|world: &mut World| shoot(world, SHOT))
        .until(shot_written(SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
    }
}
