//! storage_quick_loop: start docked at a station with your own second ship
//! beside you, and move stacks by double click in the Inventory pane.
//!
//! Three hulls, at rest, start docked as one assembly:
//!
//! - **Tender**, the player: hull plates and rations in the hold, 500 cr;
//! - **Depot**, a station hub with a port at each end: rations and salvaged
//!   parts for sale, empty storage;
//! - **Hauler**, the player's own ship on the Depot's far port: hull plates
//!   and salvaged parts in its hold.
//!
//! THE DEPOT IS DISCLOSED FIXTURE STATE: a moored spar from
//! `shared/docking_pair.rs` with a second port, marked as the shipped
//! `block_waystation` design so it trades and stores as that station does.
//! The Hauler is a second tender marked as a hull the player owns but does
//! not fly ([`OwnedShipMarker`]), as a traded-in hull parked for rebuy and
//! a hired escort are. Neither hull is shipped content. The docks are made
//! by the docking verb itself, with each hull put on its berth until the
//! dock takes.
//!
//! The Inventory pane opens once both docks hold. What to try:
//!
//! 1. double-click Hull plate in the Hauler's section: the whole stack comes
//!    into your hold;
//! 2. click Hull plate in your column, then **Store**, then double-click it:
//!    the whole stack goes into the Depot's storage;
//! 3. Shift+double-click it in the storage column: one comes back;
//! 4. click **To Hauler** on your row and double-click: the stack goes into
//!    the Hauler's hold;
//! 5. double-click Rations in the Depot's column: you buy the stack; with
//!    **Sell** on your row, a double click sells yours;
//! 6. a move that does not fit is refused whole, on the note line.
//!
//! The diagnostics layer (F3) is on at start: a line from your ship to each
//! own ship whose hold is offered as storage, labeled with its name, and a
//! panel with each hold's load and item count and the last quick action, the
//! item, the quantity moved and the credit change.
//!
//! ```text
//! nix develop --command cargo run --example storage_quick_loop --features debug
//! ```
//!
//! Nothing closes the pane and no deadline ends the run unless
//! `NOVA_AUTOPILOT=1` (debug only) is set. Then a smoke walk withdraws the
//! Hauler's plates, stores them at the Depot, takes one back with Shift,
//! checking both holds and the storage after each, and closes the pane.
//! `NOVA_CAPTURE=1` shoots one frame per move and one of the flight view
//! with the overlay.

#[path = "shared/docking_pair.rs"]
mod docking_pair;

use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::{prelude::*, ui_widgets::Activate};
use clap::Parser;
use diagnostics::{palette, DiagnosticsGizmos, DiagnosticsPanel, DiagnosticsSystems};
use docking_pair::{spar, tender};
use nova_protocol::{nova_debug::diagnostics, nova_interface::pane::InterfacePaneType, prelude::*};
use nova_ui::prelude::DoubleClick;

#[derive(Parser)]
#[command(name = "storage_quick_loop")]
#[command(version = "1.0.0")]
#[command(
    about = "Start docked at a station beside your own ship and move stacks by double click",
    long_about = None
)]
struct Cli;

/// Scenario id of the hull the player flies.
const TENDER_ID: &str = "storage_tender";
/// Scenario id of the station hub.
const DEPOT_ID: &str = "storage_depot";
/// Scenario id of the player's own second ship.
const HAULER_ID: &str = "storage_hauler";

/// The hulls' names, which the pane's note line reads.
const TENDER_NAME: &str = "Tender";
const DEPOT_NAME: &str = "Depot";
const HAULER_NAME: &str = "Hauler";

/// The shipped station design the Depot trades and stores as: Trade and
/// Storage among its services.
const DEPOT_DESIGN: &str = "block_waystation";
/// The Depot's storage cap, grams.
const STORAGE_CAP_G: u32 = 100_000;

/// Where the Depot stands, meters: ahead of the Tender and turned to face
/// it, so the two near port faces stand 5 m apart as in `trade_loop`.
const DEPOT_POSITION: Meters3 = Meters3::new(0.0, 0.0, -55.0);
/// Where the Hauler stands, meters: past the Depot's far port, turned to
/// face it, the same 5 m face gap.
const HAULER_POSITION: Meters3 = Meters3::new(0.0, 0.0, -110.0);

/// What each hull carries at spawn, and its credits.
const TENDER_STOCK: &[(&str, u32)] = &[(ItemRoleType::HullPlate.id(), 6), ("Rations", 2)];
const TENDER_CREDITS: u32 = 500;
const DEPOT_STOCK: &[(&str, u32)] = &[("Rations", 10), ("SalvagedParts", 2)];
const DEPOT_CREDITS: u32 = 5_000;
const HAULER_STOCK: &[(&str, u32)] = &[(ItemRoleType::HullPlate.id(), 4), ("SalvagedParts", 1)];

/// The gap the berth leaves between two port faces, engine units: half a
/// cell, inside the 10 m capture distance.
const BERTH_GAP: f32 = 0.5;
/// Where a port's face sits in its own frame, as the docking verb reads it.
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(storage_plugin).build();
    app.add_plugins((
        diagnostics::DiagnosticsHintPlugin::new(
            "double-click an item: move its stack   Shift+double-click: one",
        ),
        QuickMoveDiagnosticsPlugin,
    ));

    #[cfg(feature = "debug")]
    {
        app.add_plugins(smoke::script());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    }

    app.run()
}

fn storage_plugin(app: &mut App) {
    app.init_resource::<Staging>();
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_storage);
    app.add_systems(
        Update,
        stage_the_docks.run_if(in_state(GameStates::Playing)),
    );
}

fn load_storage(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(storage(&game_assets)));
}

fn ship_object(
    id: &str,
    name: &str,
    position: Meters3,
    rotation: Quat,
    ship: SpaceshipConfig,
) -> EventActionConfig {
    EventActionConfig::SpawnScenarioObject(ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: id.to_string(),
            name: name.to_string(),
            position,
            rotation,
        },
        kind: ScenarioObjectKind::Spaceship(ship),
    })
}

fn stock(stacks: &[(&str, u32)]) -> ShipInventoryStock {
    ShipInventoryStock::new(stacks.iter().map(|&(id, count)| (id.into(), count)))
}

/// The spar with a second port on its far end, turned to face away, so two
/// hulls dock at it at once.
fn depot() -> ShipDesign {
    let mut design = spar();
    design.sections.push(SpaceshipSectionConfig {
        id: "dock_aft".to_string(),
        position: Vec3::new(0.0, 0.0, 2.0),
        rotation: Quat::from_rotation_y(std::f32::consts::PI),
        source: SectionSource::prototype(DOCKING_PORT_SECTION_ID),
    });
    design
}

fn storage(game_assets: &GameAssets) -> ScenarioConfig {
    let facing_back = Quat::from_rotation_y(std::f32::consts::PI);
    let player = ship_object(
        TENDER_ID,
        TENDER_NAME,
        Meters3::ZERO,
        Quat::IDENTITY,
        SpaceshipConfig {
            controller: SpaceshipController::Player(PlayerControllerConfig::default()),
            design: ShipDesignSource::Inline(tender()),
            inventory: stock(TENDER_STOCK),
            credits: TENDER_CREDITS,
            ..default()
        },
    );
    let hub = ship_object(
        DEPOT_ID,
        DEPOT_NAME,
        DEPOT_POSITION,
        facing_back,
        SpaceshipConfig {
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(depot()),
            allegiance: Some(Allegiance::Neutral),
            inventory: stock(DEPOT_STOCK),
            lootable: false,
            credits: DEPOT_CREDITS,
            ..default()
        },
    );
    let hauler = ship_object(
        HAULER_ID,
        HAULER_NAME,
        HAULER_POSITION,
        facing_back,
        SpaceshipConfig {
            controller: SpaceshipController::None,
            design: ShipDesignSource::Inline(tender()),
            allegiance: Some(Allegiance::Player),
            inventory: stock(HAULER_STOCK),
            lootable: false,
            ..default()
        },
    );

    ScenarioConfig {
        description: "A tender and its own hauler docked at a depot".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![player, hub, hauler],
                ThreePointRig::around("storage", DEPOT_POSITION, 8.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "storage_quick_loop".to_string(),
            "Storage Quick Loop".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// Whether the docks are staged and the pane opened.
#[derive(Resource, Default)]
struct Staging {
    opened: bool,
}

/// The ship root with scenario id `id`, if it has spawned.
fn find_ship(world: &mut World, id: &str) -> Option<Entity> {
    let mut roots = world.query_filtered::<(Entity, &EntityId), With<SpaceshipRootMarker>>();
    roots
        .iter(world)
        .find(|(_, entity_id)| entity_id.0 == id)
        .map(|(entity, _)| entity)
}

/// Mark the Depot a station and the Hauler an owned ship, put each hull on
/// its berth and ask to dock until both docks hold, then open the Inventory
/// pane once.
fn stage_the_docks(world: &mut World) {
    if world.resource::<Staging>().opened {
        return;
    }
    let (Some(tender), Some(depot), Some(hauler)) = (
        find_ship(world, TENDER_ID),
        find_ship(world, DEPOT_ID),
        find_ship(world, HAULER_ID),
    ) else {
        return;
    };
    if !world.entity(depot).contains::<StationMarker>() {
        world.entity_mut(depot).insert((
            StationMarker(StationDesignId::from(DEPOT_DESIGN)),
            StationStorage::new(STORAGE_CAP_G),
        ));
        world.entity_mut(hauler).insert(OwnedShipMarker);
        return;
    }
    let mut waiting = false;
    for ship in [tender, hauler] {
        if !world.entity(ship).contains::<DockedShip>() {
            waiting = true;
            berth_and_dock(world, ship, depot);
        }
    }
    if waiting {
        return;
    }
    world.resource_mut::<Staging>().opened = true;
    *world.resource_mut::<InterfacePaneType>() = InterfacePaneType::Inventory;
    world
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Interface);
    info!("storage_quick_loop: docked; the Inventory pane is open, TAB closes it");
}

/// `section`'s position and rotation in `root`'s frame, by its mount chain.
fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
    let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
    while at != root {
        let transform = world
            .get::<Transform>(at)
            .expect("storage_quick_loop: a mounted section has a transform");
        position = transform.translation + transform.rotation * position;
        rotation = transform.rotation * rotation;
        at = world
            .get::<ChildOf>(at)
            .expect("storage_quick_loop: a section's chain reaches its root")
            .parent();
    }
    (position, rotation)
}

/// Whether `section` is mounted under `root`.
fn mounted_on(world: &World, section: Entity, root: Entity) -> bool {
    let mut at = section;
    while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
        if parent == root {
            return true;
        }
        at = parent;
    }
    false
}

/// The free, live docking port on `root` nearest `near`, world space.
fn free_port(world: &mut World, root: Entity, near: Vec3) -> Option<Entity> {
    let ports: Vec<(Entity, Vec3)> = world
        .query_filtered::<(Entity, &GlobalTransform), (
            With<DockingSectionMarker>,
            Without<DockedPort>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .map(|(port, transform)| (port, transform.translation()))
        .collect();
    ports
        .into_iter()
        .filter(|(port, _)| mounted_on(world, *port, root))
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(near)
                .total_cmp(&b.distance_squared(near))
        })
        .map(|(port, _)| port)
}

/// Put `ship` on the berth at the Depot port nearest it, at rest, and ask to
/// dock. Refused while a port is not up yet; the next frame asks again.
fn berth_and_dock(world: &mut World, ship: Entity, depot: Entity) {
    let Some(&Position(ship_at)) = world.get::<Position>(ship) else {
        return;
    };
    let (Some(depot_port), Some(ship_port)) = (
        free_port(world, depot, ship_at),
        free_port(world, ship, ship_at),
    ) else {
        return;
    };
    let (Some(&Position(position)), Some(&Rotation(rotation))) =
        (world.get::<Position>(depot), world.get::<Rotation>(depot))
    else {
        return;
    };
    let (port_at, port_rot) = local_pose(world, depot_port, depot);
    let face_rot = rotation * port_rot;
    let face = position + rotation * port_at + face_rot * PORT_FACE;
    let axis = face_rot * Vec3::NEG_Z;
    let (own_at, own_rot) = local_pose(world, ship_port, ship);
    let turn = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -axis);
    let face_offset = turn * own_at + turn * own_rot * PORT_FACE;
    world.entity_mut(ship).insert((
        Position(face + axis * BERTH_GAP - face_offset),
        Rotation(turn),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
    ));
    world.trigger(DockingConnectionRequest {
        entity: ship,
        target: depot,
    });
}

/// The overlay this example draws on the shared diagnostics layer: the own
/// holds offered as storage, every hold's load, and the last quick move.
struct QuickMoveDiagnosticsPlugin;

impl Plugin for QuickMoveDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuickMoveLog>();
        app.add_observer(watch_row_clicks);
        app.add_systems(PostUpdate, resolve_the_quick_move);
        app.add_systems(Last, snapshot_the_holds);
        app.add_systems(
            PostUpdate,
            draw_the_holds
                .in_set(DiagnosticsSystems)
                .after(TransformSystems::Propagate),
        );
    }
}

/// The player's stacks and credits, and every store's stacks, by name.
#[derive(Clone, Default, Debug, PartialEq)]
struct Snapshot {
    player: Vec<(ItemDesignId, u32)>,
    credits: u32,
}

impl Snapshot {
    fn count(&self, item: &ItemDesignId) -> u32 {
        self.player
            .iter()
            .find(|(held, _)| held == item)
            .map_or(0, |(_, count)| *count)
    }
}

/// The overlay's own double-click watch, the last quick move it resolved and
/// the snapshot it diffs against. A copy of the pane's detection on the same
/// helper and threshold, so the panel reads what the gesture was.
#[derive(Resource, Default)]
struct QuickMoveLog {
    click: DoubleClick<String>,
    pending: Option<(String, String)>,
    last: Option<String>,
    before: Snapshot,
}

/// The item and the column a row's name holds: `InventoryRow<side><item>`,
/// where an own hold's side is `Hold(<entity>)`.
fn row_parts(name: &str) -> Option<(&'static str, &str)> {
    let rest = name.strip_prefix("InventoryRow")?;
    for (prefix, side) in [
        ("Own", "your hold"),
        ("Partner", "the station"),
        ("Storage", "storage"),
    ] {
        if let Some(item) = rest.strip_prefix(prefix) {
            return Some((side, item));
        }
    }
    let item = rest.strip_prefix("Hold(")?.split_once(')')?.1;
    Some(("an own hold", item))
}

/// Note a double click, or a Shift+double click, on an Inventory row.
fn watch_row_clicks(
    activate: On<Activate>,
    q_name: Query<&Name>,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut log: ResMut<QuickMoveLog>,
) {
    let Ok(name) = q_name.get(activate.entity) else {
        return;
    };
    if row_parts(name.as_str()).is_none() {
        return;
    }
    if log.click.press(name.to_string(), time.elapsed_secs()) {
        let gesture = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            "Shift+double-click"
        } else {
            "double-click"
        };
        log.pending = Some((gesture.to_string(), name.to_string()));
    }
}

/// The player's snapshot now.
fn player_snapshot(
    q_player: &Query<(&ShipInventory, &ShipCredits), With<PlayerSpaceshipMarker>>,
) -> Option<Snapshot> {
    let (hold, credits) = q_player.single().ok()?;
    Some(Snapshot {
        player: hold
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect(),
        credits: credits.0,
    })
}

/// Resolve a pending quick move against last frame's snapshot: the pane
/// applies it in `Update`, so by `PostUpdate` the holds have moved.
fn resolve_the_quick_move(
    mut log: ResMut<QuickMoveLog>,
    items: Res<GameItems>,
    q_player: Query<(&ShipInventory, &ShipCredits), With<PlayerSpaceshipMarker>>,
) {
    let Some((gesture, row)) = log.pending.take() else {
        return;
    };
    let (Some(now), Some((side, item))) = (player_snapshot(&q_player), row_parts(&row)) else {
        return;
    };
    let item = ItemDesignId::from(item);
    let moved = now.count(&item).abs_diff(log.before.count(&item));
    let credits = i64::from(now.credits) - i64::from(log.before.credits);
    let label = &items.design(&item).name;
    log.last = Some(if moved == 0 && credits == 0 {
        format!("{gesture} {label} in {side}: refused, nothing moved")
    } else {
        format!("{gesture} {label} in {side}: moved {moved}, credits {credits:+} cr")
    });
}

/// Keep the snapshot a quick move is diffed against.
fn snapshot_the_holds(
    mut log: ResMut<QuickMoveLog>,
    q_player: Query<(&ShipInventory, &ShipCredits), With<PlayerSpaceshipMarker>>,
) {
    if let Some(now) = player_snapshot(&q_player) {
        log.before = now;
    }
}

/// `count` items, in words.
fn items_text(count: u32) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}

/// One hold's panel line: load against capacity and units held.
fn hold_line(title: &str, hold: &ShipInventory, items: &GameItems) -> String {
    let units: u32 = hold.stacks().map(|(_, count)| count).sum();
    format!(
        "{title}: {} / {}, {}",
        kg_text(u64::from(hold.used_g(items))),
        kg_text(u64::from(hold.capacity_g())),
        items_text(units)
    )
}

/// Lines to each own ship whose hold the pane offers as storage, its name
/// over it, and the panel.
#[expect(
    clippy::type_complexity,
    reason = "one query per role the overlay reads"
)]
fn draw_the_holds(
    mut gizmos: Gizmos<DiagnosticsGizmos>,
    mut panel: ResMut<DiagnosticsPanel>,
    items: Res<GameItems>,
    log: Res<QuickMoveLog>,
    q_player: Query<
        (
            &GlobalTransform,
            &ShipInventory,
            &ShipCredits,
            Option<&DockedShip>,
        ),
        With<PlayerSpaceshipMarker>,
    >,
    q_fleet: Query<(&GlobalTransform, &Name, &ShipInventory, &DockedShip), With<OwnedShipMarker>>,
    q_stations: Query<(&Name, &StationStorage), With<StationMarker>>,
    q_connections: Query<&DockingConnection>,
) {
    let Ok((at, hold, credits, docked)) = q_player.single() else {
        return;
    };
    panel.line(format!(
        "{}, {} cr",
        hold_line("Your hold", hold, &items),
        credits.0
    ));
    let station = docked
        .and_then(|docked| q_connections.get(docked.connection).ok())
        .and_then(|connection| {
            [connection.first_ship, connection.second_ship]
                .into_iter()
                .find(|ship| q_stations.contains(*ship))
        });
    if let Some((name, storage)) = station.and_then(|station| q_stations.get(station).ok()) {
        let units: u32 = storage.stacks().map(|(_, count)| count).sum();
        panel.line(format!(
            "{name} storage: {} / {}, {}",
            kg_text(u64::from(storage.used_g(&items))),
            kg_text(u64::from(storage.cap_g())),
            items_text(units)
        ));
    }
    for (ship_at, name, ship_hold, ship_docked) in &q_fleet {
        let offered = station.is_some_and(|station| {
            q_connections
                .get(ship_docked.connection)
                .is_ok_and(|connection| connection.joins(station))
        });
        if !offered {
            continue;
        }
        let (from, to) = (at.translation(), ship_at.translation());
        gizmos.line(from, to, palette::OK);
        panel.label(to, format!("{name} (own hold, storage)"));
        panel.line(hold_line(&format!("{name} (own)"), ship_hold, &items));
    }
    panel.line(format!(
        "Last quick move: {}",
        log.last.as_deref().unwrap_or("none yet")
    ));
}

#[cfg(feature = "debug")]
mod smoke {
    use std::sync::Arc;

    use nova_protocol::nova_debug::harness::{AutopilotPlugin, Predicate};

    use super::*;

    /// Seconds the walk gives the scenario to load and both docks to hold.
    const LOAD_DEADLINE: f32 = 60.0;
    /// Seconds any later beat may stall before the walk fails it.
    const STEP_DEADLINE: f32 = 20.0;

    /// The frames the capture path writes.
    const WITHDRAW_OWN_SHOT: &str = "quick-withdraw-own-ship.png";
    const STORE_SHOT: &str = "quick-store-station.png";
    const WITHDRAW_ONE_SHOT: &str = "quick-withdraw-one-station.png";
    const FLIGHT_SHOT: &str = "quick-overlay-flight.png";

    fn ship(world: &mut World, id: &str) -> Entity {
        find_ship(world, id).unwrap_or_else(|| panic!("storage_quick_loop: no ship {id}"))
    }

    /// Hull plates in a hold.
    fn plates(world: &mut World, id: &str) -> u32 {
        let root = ship(world, id);
        world
            .get::<ShipInventory>(root)
            .unwrap_or_else(|| panic!("storage_quick_loop: {id} has no ShipInventory"))
            .count(&ItemDesignId::from(ItemRoleType::HullPlate))
    }

    /// Hull plates in the Depot's storage.
    fn stored(world: &mut World) -> u32 {
        let depot = ship(world, DEPOT_ID);
        world
            .get::<StationStorage>(depot)
            .expect("storage_quick_loop: the Depot has storage")
            .count(&ItemDesignId::from(ItemRoleType::HullPlate))
    }

    /// Assert the plates in (Tender, Depot storage, Hauler), and that none
    /// was made or lost.
    fn check(world: &mut World, stage: &str, wanted: (u32, u32, u32)) {
        let got = (
            plates(world, TENDER_ID),
            stored(world),
            plates(world, HAULER_ID),
        );
        assert_eq!(
            got, wanted,
            "storage_quick_loop {stage}: plates in (tender, storage, hauler)"
        );
        info!("storage_quick_loop: PASS {stage} {got:?}");
    }

    /// Advance once a laid-out, visible UI text reads `wanted`.
    fn text_shown(wanted: &'static str) -> Arc<Predicate> {
        Arc::new(move |world: &World| {
            world
                .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
                .is_some_and(|mut texts| {
                    texts.iter(world).any(|(text, node, visible)| {
                        text.0 == wanted && visible.get() && node.size().x > 0.0
                    })
                })
        })
    }

    /// The Hauler's hull plate row: an own hold's row is named by its ship.
    fn hauler_row(world: &mut World) -> String {
        let hauler = ship(world, HAULER_ID);
        format!(
            "InventoryRowHold({hauler:?}){}",
            ItemDesignId::from(ItemRoleType::HullPlate)
        )
    }

    /// `AutopilotPlugin::double_click_named` on the Hauler's row, whose name
    /// holds an entity the script cannot know up front.
    fn double_click_hauler_row(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step("own ship: double-click the Hauler's Hull plate")
            .on_enter(|world: &mut World| {
                let row = hauler_row(world);
                click_named(row)(world);
            })
            .each(|world: &mut World, _, frame| match frame {
                1 | 3 => release_mouse(MouseButton::Left)(world),
                2 => {
                    let row = hauler_row(world);
                    click_named(row)(world);
                }
                _ => {}
            })
            .until(and(frames(4), pointer_released()))
            .deadline(STEP_DEADLINE)
            .add()
    }

    /// Wait for `note`, check the plates, and shoot `shot`.
    fn result(
        script: AutopilotPlugin<GameStates>,
        stage: &'static str,
        note: &'static str,
        wanted: (u32, u32, u32),
        shot: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("{stage}: the note line"))
            .until(text_shown(note))
            .deadline(STEP_DEADLINE)
            .add()
            .step(format!("{stage}: the overlay reads the move"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<QuickMoveLog>()
                    .is_some_and(|log| log.last.is_some() && log.pending.is_none())
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step(format!("{stage}: check the plates"))
            .on_enter(move |world: &mut World| check(world, stage, wanted))
            .until(frames(2))
            .add()
            .step(format!("{stage}: shoot"))
            .on_enter(move |world: &mut World| shoot(world, shot))
            .until(shot_written(shot))
            .deadline(STEP_DEADLINE)
            .add()
    }

    /// The smoke walk. Inert unless `NOVA_AUTOPILOT` is set.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("load and dock the three hulls")
            .enter(GameStates::Loading)
            .until(ui_node_present("InventoryRowOwnHullPlate"))
            .deadline(LOAD_DEADLINE)
            .add()
            .step("settle the pane")
            .on_enter(|world: &mut World| {
                hide_status_bar(world);
                check(world, "at start", (6, 0, 4));
            })
            .until(frames(5))
            .add();

        // 1. The Hauler's row withdraws its whole stack.
        let script = double_click_hauler_row(script);
        let script = result(
            script,
            "withdraw the Hauler's stack",
            "Withdrew 4 Hull plate from Hauler",
            (10, 0, 4 - 4),
            WITHDRAW_OWN_SHOT,
        );

        // 2. At a trader the own row opens Sell; the Store chip sticks, and
        // the double click stores the stack.
        let script = script
            .click_named(
                "station: pick Hull plate",
                "InventoryRowOwnHullPlate",
                ui_node_present("InventoryDraftStore"),
                STEP_DEADLINE,
            )
            .click_named(
                "station: switch to Store",
                "InventoryDraftStore",
                pointer_released(),
                STEP_DEADLINE,
            )
            .step("let the pick's double-click window pass")
            .until(elapsed(1.0))
            .add()
            .double_click_named(
                "station: double-click your Hull plate",
                "InventoryRowOwnHullPlate",
                STEP_DEADLINE,
            );
        let script = result(
            script,
            "store the stack at the Depot",
            "Stored 10 Hull plate at Depot",
            (0, 10, 0),
            STORE_SHOT,
        );

        // 3. Shift moves one unit.
        let script = script
            .step("hold Shift")
            .on_enter(press_key(KeyCode::ShiftLeft))
            .until(frames(1))
            .add()
            .double_click_named(
                "station: Shift+double-click the stored Hull plate",
                "InventoryRowStorageHullPlate",
                STEP_DEADLINE,
            );
        let script = result(
            script,
            "withdraw one",
            "Withdrew 1 Hull plate from Depot",
            (1, 9, 0),
            WITHDRAW_ONE_SHOT,
        )
        .step("let Shift up")
        .on_enter(release_key(KeyCode::ShiftLeft))
        .until(frames(1))
        .add();

        // 4. Close the pane: the flight view shows the line to the Hauler.
        script
            .step("press the interface key")
            .on_enter(press_action("interface_toggle"))
            .until(frames(1))
            .add()
            .step("let the key up and wait for flight")
            .on_enter(release_action("interface_toggle"))
            .until(Arc::new(|world: &World| {
                world
                    .get_resource::<State<PauseStates>>()
                    .is_some_and(|pause| *pause.get() != PauseStates::Interface)
            }))
            .deadline(STEP_DEADLINE)
            .add()
            .step("settle the flight view")
            .until(frames(5))
            .add()
            .step("shoot the flight view")
            .on_enter(|world: &mut World| shoot(world, FLIGHT_SHOT))
            .until(shot_written(FLIGHT_SHOT))
            .deadline(STEP_DEADLINE)
            .add()
    }
}
