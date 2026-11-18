//! field_repair_sandbox: fly a hurt ship with its docking port shot off, and
//! repair it in the field: plate repair jobs on the hurt sections and a field
//! rebuild of the lost port from the parts in the hold.
//!
//! The player's hull is the tender from `shared/docking_pair.rs`: a bow port,
//! a reinforced bow plate, the bridge, two light shoulder plates and a drive.
//! It starts undocked in a void, 400 m off the shipped waystation's collar.
//! THE DAMAGE AND THE HOLD ARE DISCLOSED FIXTURE STATE:
//!
//! | What | Fixture |
//! | - | - |
//! | `bow`, `shoulder_port`, `shoulder_starboard` | at 40 % of their max integrity |
//! | `dock_bow` (`DOCK-1`) | lost: put to zero the way the damage path does |
//! | hold | 6 hull plates and the port's recipe: 1 docking collar, 1 actuator pack, 2 section panels |
//!
//! What to do:
//!
//! 1. press Tab and open the Ship tab;
//! 2. click a hurt hull plate, press All, then Repair: the Repairs block
//!    shows the job, its bar and its time left;
//! 3. under the lost list, click Rebuild on `DOCK-1`: a field rebuild job
//!    holds the parts and takes them when its time is up;
//! 4. close Tab. The jobs run while you fly: the chip under the RCS chip
//!    shows the first one, and a notice says when each ends. The open
//!    interface holds them.
//! 5. with the port back, lock the waystation (Ctrl) and dock (D).
//!
//! The diagnostics layer (F3) is on at start: the running jobs, the plates
//! and parts in the hold, and whether the port is live.
//!
//! ```text
//! nix develop --command cargo run --example field_repair_sandbox --features debug
//! ```
//!
//! No deadline ends the run unless `NOVA_AUTOPILOT=1` is set. Then a smoke
//! walk checks the staging, opens the Ship pane, clicks Rebuild on `DOCK-1`,
//! closes the interface, waits for the port to come back, checks that the
//! recipe left the hold, then berths at the waystation and docks on the
//! rebuilt port. `NOVA_CAPTURE=1` shoots the flight view, the pane with the
//! job and the flight view with the port back.

#[expect(dead_code, reason = "this example flies the tender and moors no spar")]
#[path = "shared/docking_pair.rs"]
mod docking_pair;
#[path = "shared/open_world_stage.rs"]
mod open_world_stage;
#[path = "shared/station_void.rs"]
mod station_void;

use bevy::prelude::*;
use diagnostics::{DiagnosticsPanel, DiagnosticsSystems};
use nova_protocol::{nova_debug::diagnostics, prelude::*};

/// The waystation's park, meters: 400 m ahead of the tender's bow port.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -400.0);
/// The station's scenario id and name.
const STATION_ID: &str = "waystation";
const STATION_NAME: &str = "WAYSTATION";
/// The player's ship name.
const PLAYER_NAME: &str = "Tender";

/// The design id of the port the fixture loses.
const LOST_PORT: &str = "dock_bow";
/// The design ids of the plates the fixture hurts.
const HURT_SECTIONS: [&str; 3] = ["bow", "shoulder_port", "shoulder_starboard"];
/// The share of max integrity a hurt section keeps.
const HURT_SHARE: f32 = 0.4;
/// Hull plates in the hold.
const PLATES: u32 = 6;
/// The docking port's recipe parts, in the hold.
const PORT_PARTS: [(&str, u32); 3] = [
    ("DockingCollar", 1),
    ("ActuatorPack", 1),
    ("SectionPanel", 2),
];

/// The intro line.
const INTRO: &str =
    "Hurt and portless: Tab, Ship, Repair a plate and Rebuild DOCK-1, then close Tab and fly.";

fn main() -> bevy::app::AppExit {
    let mut app = AppBuilder::new()
        .with_game_plugins(field_repair_plugin)
        .build();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        PostUpdate,
        draw_the_repairs
            .in_set(DiagnosticsSystems)
            .after(TransformSystems::Propagate)
            .run_if(in_state(GameStates::Playing)),
    );

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        app.add_systems(Startup, force_capture_resolution);
    }
    app.add_plugins(smoke::script());

    app.run()
}

fn field_repair_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_the_void);
    app.add_systems(
        Update,
        stage_the_damage.run_if(in_state(GameStates::Playing)),
    );
}

fn load_the_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    let mut stacks = vec![(ItemRoleType::HullPlate.id(), PLATES)];
    stacks.extend(PORT_PARTS);
    let player = station_void::player(
        PLAYER_NAME,
        Meters3::ZERO,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(docking_pair::tender()),
            inventory: station_void::stock(&stacks),
            ..default()
        },
    );
    commands.trigger(LoadScenario(station_void::void(
        "field_repair_sandbox",
        "Field Repair Sandbox",
        vec![
            station_void::waystation(STATION_ID, STATION_NAME, STATION_POSITION),
            player,
        ],
        &[(Meters3::ZERO, 1.0), (STATION_POSITION, 1.0)],
        &game_assets,
    )));
}

/// Whether the fixture damage was put on the hull. The staging stops there,
/// so a repair or a rebuild is never undone. `docked` is set by the smoke
/// walk once the rebuilt port holds a dock, because a walk predicate reads
/// the world but cannot query it.
#[derive(Resource, Default)]
struct Staging {
    damaged: bool,
    docked: bool,
}

/// The live section of `player` with design id `id`.
fn section_of(world: &mut World, player: Entity, id: &str) -> Option<Entity> {
    let mut sections = world.query_filtered::<(Entity, &EntityId, &ChildOf), With<SectionMarker>>();
    sections
        .iter(world)
        .find(|(_, found, child)| found.0 == id && child.parent() == player)
        .map(|(entity, ..)| entity)
}

/// Hurt the three plates and put the port at zero, once, as soon as the
/// player's sections carry their health.
fn stage_the_damage(world: &mut World) {
    if world.resource::<Staging>().damaged {
        return;
    }
    let Some(player) = station_void::player_root(world) else {
        return;
    };
    let mut targets = Vec::new();
    for id in HURT_SECTIONS.into_iter().chain([LOST_PORT]) {
        match section_of(world, player, id) {
            // Wait for the ship graph: it is built a frame after the
            // sections spawn and reads every section, so a port despawned
            // before then races the graph's own writes.
            Some(section)
                if world.get::<Health>(section).is_some()
                    && world.get::<ConnectedTo>(section).is_some() =>
            {
                targets.push((id, section));
            }
            _ => return,
        }
    }
    for (id, section) in targets {
        let mut health = world
            .get_mut::<Health>(section)
            .expect("field_repair_sandbox: a section carries its health");
        if id == LOST_PORT {
            health.current = 0.0;
            world.entity_mut(section).insert(HealthZeroMarker);
        } else {
            health.current = health.max * HURT_SHARE;
        }
    }
    world.resource_mut::<Staging>().damaged = true;
    info!("field_repair_sandbox: hurt {HURT_SECTIONS:?} and lost {LOST_PORT}; TAB opens the interface");
}

/// The panel: the running jobs, the hold's plates and port parts, and
/// whether the port is live.
fn draw_the_repairs(
    mut panel: ResMut<DiagnosticsPanel>,
    items: Res<GameItems>,
    pause: Option<Res<State<PauseStates>>>,
    q_player: Query<
        (
            &ShipInventory,
            Option<&ShipRepairJobs>,
            Has<DockedShip>,
            Option<&Children>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_ids: Query<&EntityId, With<SectionMarker>>,
) {
    let Ok((hold, jobs, docked, children)) = q_player.single() else {
        return;
    };
    let jobs = jobs.map_or(&[][..], |jobs| jobs.0.as_slice());
    if jobs.is_empty() {
        panel.line("Jobs: none".to_string());
    }
    for job in jobs {
        let what = match job.work {
            RepairWorkType::Plates { queued, spent, .. } => format!("plates {spent}/{queued}"),
            RepairWorkType::FieldRebuild { .. } => "field rebuild".to_string(),
        };
        panel.line(format!(
            "Job {}: {what}, {:.0} %, {:.0} s left",
            job.code,
            job.progress() * 100.0,
            job.remaining_secs()
        ));
    }
    let plate = ItemDesignId::from(ItemRoleType::HullPlate);
    let parts: Vec<String> = PORT_PARTS
        .iter()
        .map(|(id, _)| {
            let id = ItemDesignId::from(*id);
            format!("{} {}", hold.count(&id), items.design(&id).name)
        })
        .collect();
    panel.line(format!(
        "Hold: {} hull plates, {}",
        hold.count(&plate),
        parts.join(", ")
    ));
    let port_live = children
        .into_iter()
        .flatten()
        .filter_map(|child| q_ids.get(*child).ok())
        .any(|id| id.0 == LOST_PORT);
    panel.line(format!(
        "DOCK-1: {}",
        if port_live { "live" } else { "lost" }
    ));
    panel.detail(format!(
        "Plate job: {:.0} HP per plate every {PLATE_REPAIR_SECS:.0} s",
        HULL_PLATE_HEALTH
    ));
    if station_void::interface_up(pause.as_deref()) {
        panel.hint(
            "Ship",
            "Repair a hurt plate, Rebuild DOCK-1, then close Tab",
        );
    } else if docked {
        panel.hint("D", "undock");
    } else if port_live {
        panel.hint("Ctrl", "hold on the station to lock it");
        panel.hint("D", "dock");
    } else {
        panel.hint("TAB", "Ship > Repair, Rebuild");
    }
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS},
        nova_interface::pane::InterfacePaneType,
    };

    use super::*;

    /// Seconds the walk gives the void to load and the damage to land.
    const LOAD_SECS: f32 = 120.0;
    /// Seconds any later beat may stall before the walk fails it.
    const BEAT_SECS: f32 = 60.0;
    /// Seconds the field rebuild gets: its game time, slowed by a
    /// software-rendered GPU.
    const REBUILD_SECS: f32 = 240.0;
    /// Frames the Ship pane gets to build its schematic and its lists.
    const SETTLE_FRAMES: u32 = 10;

    const REBUILD_BUTTON: &str = "ShipRebuild-DOCK-1";

    /// The frames the capture path writes.
    const FLIGHT_SHOT: &str = "field-repair-sandbox-flight.png";
    const PANE_SHOT: &str = "field-repair-sandbox-pane.png";
    const REBUILT_SHOT: &str = "field-repair-sandbox-rebuilt.png";

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    /// The design section `id` of the player is live.
    fn live(world: &World, id: &str) -> bool {
        let Some(player) = world
            .try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next())
        else {
            return false;
        };
        world
            .try_query_filtered::<(&EntityId, &ChildOf), With<SectionMarker>>()
            .is_some_and(|mut query| {
                query
                    .iter(world)
                    .any(|(found, child)| found.0 == id && child.parent() == player)
            })
    }

    fn jobs(world: &World) -> Vec<RepairJob> {
        world
            .try_query_filtered::<&ShipRepairJobs, With<PlayerSpaceshipMarker>>()
            .and_then(|mut query| query.iter(world).next().map(|jobs| jobs.0.clone()))
            .unwrap_or_default()
    }

    fn held(world: &mut World, id: &str) -> u32 {
        let player = station_void::player_root(world).expect("field_repair_sandbox: a player");
        world
            .get::<ShipInventory>(player)
            .expect("field_repair_sandbox: the player carries a hold")
            .count(&ItemDesignId::from(id))
    }

    /// The staging: undocked, the port lost, the plates hurt, the hold full.
    fn assert_the_staging(world: &mut World) {
        let player = station_void::player_root(world).expect("field_repair_sandbox: a player");
        assert!(
            world.get::<DockedShip>(player).is_none(),
            "field_repair_sandbox: the player starts undocked"
        );
        assert!(
            !live(world, LOST_PORT),
            "field_repair_sandbox: {LOST_PORT} is lost"
        );
        for id in HURT_SECTIONS {
            let section = section_of(world, player, id)
                .unwrap_or_else(|| panic!("field_repair_sandbox: {id} is live"));
            let health = world
                .get::<Health>(section)
                .expect("field_repair_sandbox: a section carries its health");
            assert!(
                health.current < health.max,
                "field_repair_sandbox: {id} is hurt: {health:?}"
            );
        }
        assert_eq!(held(world, ItemRoleType::HullPlate.id()), PLATES);
        for (id, count) in PORT_PARTS {
            assert_eq!(held(world, id), count, "field_repair_sandbox: {id} aboard");
        }
        info!("field_repair_sandbox: PASS staged undocked, {LOST_PORT} lost, three plates hurt");
    }

    /// The pane started one field rebuild of the port and took nothing yet.
    fn assert_the_job(world: &mut World) {
        let jobs = jobs(world);
        assert!(
            matches!(
                jobs.as_slice(),
                [RepairJob {
                    work: RepairWorkType::FieldRebuild { .. },
                    ..
                }]
            ),
            "field_repair_sandbox: one field rebuild runs: {jobs:?}"
        );
        for (id, count) in PORT_PARTS {
            assert_eq!(
                held(world, id),
                count,
                "field_repair_sandbox: a field rebuild takes {id} only at its end"
            );
        }
        info!("field_repair_sandbox: PASS the pane started a field rebuild of DOCK-1");
    }

    /// The port is back and its recipe left the hold.
    fn assert_the_rebuild(world: &mut World) {
        assert!(
            live(world, LOST_PORT),
            "field_repair_sandbox: the port is back"
        );
        for (id, _) in PORT_PARTS {
            assert_eq!(held(world, id), 0, "field_repair_sandbox: {id} was spent");
        }
        assert!(
            jobs(world).is_empty(),
            "field_repair_sandbox: the job ended"
        );
        info!("field_repair_sandbox: PASS DOCK-1 rebuilt in the field from the hold");
    }

    /// Berth at the waystation and ask to dock, each frame until the dock
    /// holds.
    fn dock_on_the_rebuilt_port(world: &mut World) {
        if station_void::berth_and_dock_at(world, STATION_ID) {
            world.resource_mut::<Staging>().docked = true;
        }
    }

    /// The dock holds on the rebuilt port, and the pair stands still.
    fn assert_the_dock(world: &mut World) {
        let player = station_void::player_root(world).expect("field_repair_sandbox: a player");
        let station = station_void::station_root(world, STATION_ID)
            .expect("field_repair_sandbox: the waystation is up");
        assert!(
            station_void::docked_at(world, STATION_ID),
            "field_repair_sandbox: docked at the waystation"
        );
        let port = section_of(world, player, LOST_PORT)
            .expect("field_repair_sandbox: the rebuilt port is live");
        let connection = world
            .get::<DockedShip>(player)
            .expect("field_repair_sandbox: a docked player carries DockedShip")
            .connection;
        let connection = world
            .get::<DockingConnection>(connection)
            .expect("field_repair_sandbox: the dock has its connection");
        assert!(
            [connection.first_section, connection.second_section].contains(&port),
            "field_repair_sandbox: the dock holds on the rebuilt DOCK-1"
        );
        station_void::assert_at_rest(world, player, station);
        info!("field_repair_sandbox: PASS docked at {STATION_NAME} on the rebuilt DOCK-1");
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        AutopilotPlugin::<GameStates>::new()
            .step("field repair sandbox: load the void and stage the damage")
            .enter(GameStates::Loading)
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.damaged)
                    && !live(world, LOST_PORT)
            }))
            .deadline(LOAD_SECS)
            .add()
            .step("field repair sandbox: check the staging")
            .on_enter(assert_the_staging)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("field repair sandbox: shoot the flight view")
            .on_enter(|world: &mut World| shoot(world, FLIGHT_SHOT))
            .until(shot_written(FLIGHT_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("field repair sandbox: open the Ship pane")
            .on_enter(|world: &mut World| {
                station_void::open_pane(world, InterfacePaneType::Ship);
            })
            .until(ui_node_present(REBUILD_BUTTON))
            .deadline(BEAT_SECS)
            .add()
            .step("field repair sandbox: settle the pane")
            .until(frames(SETTLE_FRAMES))
            .add()
            .click_named(
                "field repair sandbox: rebuild DOCK-1 in the field",
                REBUILD_BUTTON,
                when(|world| !jobs(world).is_empty()),
                BEAT_SECS,
            )
            .step("field repair sandbox: check the job")
            .on_enter(assert_the_job)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("field repair sandbox: shoot the pane")
            .on_enter(|world: &mut World| shoot(world, PANE_SHOT))
            .until(shot_written(PANE_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("field repair sandbox: close the interface")
            .on_enter(|world: &mut World| {
                world
                    .resource_mut::<NextState<PauseStates>>()
                    .set(PauseStates::Unpaused);
            })
            .until(when(|world| live(world, LOST_PORT)))
            .deadline(REBUILD_SECS)
            .add()
            .step("field repair sandbox: check the rebuild")
            .on_enter(assert_the_rebuild)
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("field repair sandbox: shoot the rebuilt port")
            .on_enter(|world: &mut World| shoot(world, REBUILT_SHOT))
            .until(shot_written(REBUILT_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("field repair sandbox: dock on the rebuilt port")
            .each(|world, _, _| dock_on_the_rebuilt_port(world))
            .until(when(|world| {
                world
                    .get_resource::<Staging>()
                    .is_some_and(|staging| staging.docked)
            }))
            .deadline(BEAT_SECS)
            .add()
            .step("field repair sandbox: check the dock")
            .on_enter(assert_the_dock)
            .until(frames(SETTLE_FRAMES))
            .add()
    }
}
