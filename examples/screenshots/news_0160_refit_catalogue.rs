//! news_0160_refit_catalogue: the v0.16.0 news Refit loop - the Ship pane's
//! Catalogue at a docked Refit station, a kinetic PDC taken in hand, the
//! ghost standing valid on a dorsal socket, a roll, and the fit
//! (`news-0160-refit-catalogue.webm`).
//!
//! The world starts the way the Create World modal starts it, on
//! [`WORLD_SEED`] as `industrial`, with no save folder, and the start docks
//! the tug at its starting station: on this seed a Refit station
//! (`news_0160_station_pane` pins the same start). The walk asserts the
//! station offers Refit and builds the kinetic PDC. A seed that gives either
//! up stops the run, naming it.
//!
//! No fixture state: the tug flies its start kit and its start balance. The
//! industrial kit holds none of the PDC's parts, so the station sells every
//! one and the price is the recipe at ask; the walk asserts both, because
//! the news caption says so. A balance or a station stock that cannot pay it
//! mutes Place, and the run stops at the take-in-hand beat's deadline.
//!
//! The walk opens the interface with the real key and the Ship tab with a
//! click, then the loop records, one beat after another: the Ship pane; a
//! click on the footer's Catalogue button; the Weapons chip; the kinetic
//! PDC's tile and its focus card; Place, which empties the gallery into the
//! hand and shows the panel's Placing block; the pointer over the free
//! dorsal sockets in turn until the ghost stands valid at one; the real
//! `ship_part_roll` key (R), the Placing block's roll stepping to 90 deg;
//! the click that fits the part; and, with the pointer moved off the view
//! onto the Placing block so no second ghost aims, the view easing onto the
//! new section while the note line reads the add. Before the loop closes it
//! asserts the new PDC is live at the rolled ghost's pose under a `PDC-`
//! code and selected, the price moved from the tug's balance to the
//! station's, the hold's parts and the station's sold parts moved, and the
//! new section drew to the view's centre.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-refit-catalogue.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_refit_catalogue --features debug
//! ```

#[path = "../playable/shared/open_world_stage.rs"]
mod open_world_stage;

use std::{collections::BTreeMap, sync::Arc};

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, LoopCapturePlugin, Predicate},
    nova_interface::{
        pane::InterfacePaneType,
        ship::prelude::{
            ship_ghost_aim, ship_placement, ship_view_window_px, SectionCode, ShipRuntime,
            SHIP_HELD_PANEL,
        },
    },
    prelude::*,
};
use open_world_stage::{body_by_id, local_pose, the_player, window_settled};

#[derive(Parser)]
#[command(name = "news_0160_refit_catalogue")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 Refit loop: Catalogue, a kinetic PDC on a dorsal socket, a roll and the fit",
    long_about = None
)]
struct Cli {
    /// The world seed. Another seed must still dock the tug at a Refit
    /// station that builds the kinetic PDC.
    #[arg(long, default_value_t = WORLD_SEED)]
    seed: u32,
}

/// The seed the loop plays: its industrial start docks at a Refit station.
const WORLD_SEED: u32 = 23;

/// The start role the walk flies.
const ROLE: &str = "industrial";

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0160-refit-catalogue";

/// Real seconds the world gets to start, stream and dock the tug on a
/// software rasterizer.
const SETUP_SECS: f32 = 600.0;

/// Real seconds a beat gets.
const BEAT_SECS: f32 = 60.0;

/// Real seconds the pointer gets to find a dorsal socket the ghost stands
/// valid at.
const AIM_SECS: f32 = 120.0;

/// Frames the Ship pane gets to build its scene and panel before the loop.
const PANE_FRAMES: u32 = 10;

/// Frames each recorded state holds on screen, so each reads apart: the
/// pane, the catalogue, the Weapons grid, the focus card, the Placing block,
/// the valid ghost and the rolled ghost. One capture frame is 1/30 s.
const HOLD_FRAMES: u32 = 24;

/// Frames the view gets to ease onto the new section after the fit.
const EASE_FRAMES: u32 = 45;

/// Frames the loop holds the eased view before it ends.
const END_FRAMES: u32 = 20;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

/// Frames the pointer rests on one dorsal socket before the walk tries the
/// next one.
const AIM_FRAMES: u32 = 6;

/// How far the aim sits from the socket toward its section's centre, as a
/// share of the way: on the section's face, beside the socket.
const AIM_INSET: f32 = 0.15;

/// The least `y` of a ship-local socket normal the walk takes as dorsal.
const DORSAL_MIN_Y: f32 = 0.9;

/// How far from a quarter turn the rolled ghost's rotation may sit off the
/// unrolled one, in radians.
const ROLL_TOLERANCE_RAD: f32 = 1e-2;

/// How far the fitted section may sit off the ghost's pose: engine units
/// and radians.
const POSE_TOLERANCE: f32 = 1e-3;

/// The most logical pixels the new section may sit off the view's centre
/// once the view has eased onto it.
const EASED_PX: f32 = 8.0;

fn main() -> bevy::app::AppExit {
    let cli = Cli::parse();
    open_world_stage::isolate_the_profile(env!("CARGO_CRATE_NAME"));

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(open_world_stage::OpenWorldStartPlugin {
        seed: cli.seed,
        role: ROLE.to_string(),
    });
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(LoopCapturePlugin::default());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_systems(
        Update,
        open_world_stage::hold_the_walk_clock.run_if(in_state(GameStates::Playing)),
    );
    app.add_plugins(script());
    app.run()
}

/// The starting station, once the window is in and the start docked the
/// tug there.
#[derive(Resource)]
struct Docked(Entity);

/// One free dorsal socket the walk points at.
#[derive(Clone, Debug)]
struct Aim {
    /// The design section id that carries the socket.
    section: String,
    /// The socket id.
    point: String,
    /// Where the pointer goes, in the Ship pane's ship-local scene.
    at: Vec3,
}

/// The dorsal sockets in design order and the one pointed at.
#[derive(Resource, Default)]
struct Aiming {
    candidates: Vec<Aim>,
    /// `usize::MAX` until the first socket is pointed at.
    index: usize,
    target: Option<Aim>,
}

/// The valid ghost before the roll: its pose.
#[derive(Resource)]
struct Unrolled(Transform);

/// What a click on the rolled ghost commits, and the books before it.
#[derive(Resource, Clone, Debug)]
struct Committed {
    section: String,
    point: String,
    pose: Transform,
    payment: RecipePayment,
    /// The design section id the add takes.
    id: String,
    player_cr: u32,
    station_cr: u32,
    hold: BTreeMap<ItemDesignId, u32>,
    stock: BTreeMap<ItemDesignId, u32>,
}

/// How far the new section sat off the view's centre when it went live.
#[derive(Resource)]
struct EaseFrom(f32);

fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
    Arc::new(check)
}

/// The one player ship, if exactly one stands.
fn player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

fn the_ship(world: &World) -> Entity {
    player(world).expect("refit catalogue: one player ship")
}

/// The docked starting station.
fn station(world: &World) -> Entity {
    world
        .get_resource::<Docked>()
        .expect("refit catalogue: docked at the starting station")
        .0
}

/// The kinetic PDC as the catalogue lists it.
fn pdc(world: &World) -> &SectionConfig {
    world
        .resource::<GameSections>()
        .get_section(PDC_KINETIC_TURRET_SECTION_ID)
        .expect("refit catalogue: the catalogue lists the kinetic PDC")
}

/// The kinetic PDC's tile in the catalogue gallery.
fn pdc_tile(world: &World) -> String {
    format!("Gallery Tile {}", pdc(world).base.name)
}

/// The live section `id` of `root`.
fn section_of(world: &World, root: Entity, id: &str) -> Option<Entity> {
    world
        .get::<Children>(root)?
        .iter()
        .find(|child| world.get::<EntityId>(*child).is_some_and(|eid| eid.0 == id))
}

/// The ids of `root`'s live sections.
fn live_ids(world: &World, root: Entity) -> Vec<String> {
    world
        .get::<Children>(root)
        .into_iter()
        .flatten()
        .filter(|child| world.get::<SectionMarker>(**child).is_some())
        .filter_map(|child| world.get::<EntityId>(*child).map(|id| id.0.clone()))
        .collect()
}

fn credits(world: &World, root: Entity) -> u32 {
    world
        .get::<ShipCredits>(root)
        .expect("refit catalogue: every ship carries a balance")
        .0
}

/// How many of each of `items` `root`'s hold carries.
fn counts(
    world: &World,
    root: Entity,
    items: &BTreeMap<ItemDesignId, u32>,
) -> BTreeMap<ItemDesignId, u32> {
    let hold = world
        .get::<ShipInventory>(root)
        .expect("refit catalogue: every ship carries a hold");
    items
        .keys()
        .map(|item| (item.clone(), hold.count(item)))
        .collect()
}

/// Every visible, laid-out text under the node `name`.
fn texts_under(world: &World, name: &str) -> Vec<String> {
    let Some(mut named) = world.try_query::<(Entity, &Name)>() else {
        return Vec::new();
    };
    let Some(root) = named
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut texts = Vec::new();
    let mut open = vec![root];
    while let Some(at) = open.pop() {
        let shown = world
            .get::<InheritedVisibility>(at)
            .is_some_and(|visible| visible.get())
            && world
                .get::<ComputedNode>(at)
                .is_some_and(|node| node.size().x > 0.0);
        if let (Some(text), true) = (world.get::<Text>(at), shown) {
            texts.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(at) {
            open.extend(children.iter());
        }
    }
    texts
}

/// Whether the Placing block draws `value` among its texts.
fn placing_reads(world: &World, value: &str) -> bool {
    texts_under(world, SHIP_HELD_PANEL)
        .iter()
        .any(|text| text == value)
}

/// The interface is open on `pane`.
fn the_interface_shows(pane: InterfacePaneType) -> Arc<Predicate> {
    when(move |world| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world.get_resource::<InterfacePaneType>() == Some(&pane)
    })
}

/// The starting station, once it stands.
fn start_station(world: &mut World) -> Option<Entity> {
    let id = world.get_resource::<OpenWorldSession>()?.start.id.clone();
    body_by_id(world, &id)
}

/// Record the starting station once the window is in and the tug is docked.
fn record_the_dock(world: &mut World) {
    if world.contains_resource::<Docked>() {
        return;
    }
    let docked = the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some());
    let Some(station) = start_station(world) else {
        return;
    };
    if docked && window_settled(world) {
        world.insert_resource(Docked(station));
    }
}

/// Assert the docked station is a living Refit station that builds the
/// kinetic PDC, and log the tug's real balance.
///
/// # Panics
///
/// When the station offers no Refit or does not build the kinetic PDC: the
/// seed no longer gives the loop its subject.
fn assert_the_station(world: &mut World) {
    let station = station(world);
    let name = world
        .get::<Name>(station)
        .map_or_else(String::new, ToString::to_string);
    let services = world
        .get::<StationServices>(station)
        .unwrap_or_else(|| panic!("refit catalogue: '{name}' carries no station services"));
    assert!(
        services.offers(StationServiceType::Refit),
        "refit catalogue: '{name}' offers {:?}, no Refit",
        services.0
    );
    assert!(
        world.get::<DerelictShipMarker>(station).is_none(),
        "refit catalogue: '{name}' is derelict"
    );
    let builds = world
        .get::<StationBuilds>(station)
        .unwrap_or_else(|| panic!("refit catalogue: '{name}' lists no builds"));
    assert!(
        builds
            .0
            .iter()
            .any(|id| id == PDC_KINETIC_TURRET_SECTION_ID),
        "refit catalogue: '{name}' does not build '{PDC_KINETIC_TURRET_SECTION_ID}': {:?}",
        builds.0
    );
    let ship = the_ship(world);
    info!(
        "refit catalogue: docked at Refit station '{name}', balance {} cr, station {} cr",
        credits(world, ship),
        credits(world, station)
    );
}

/// List every free socket of a live section whose outward normal points up
/// the hull, in design order. The pane's own solve judges each one when the
/// pointer is over it.
///
/// # Panics
///
/// When the hull has no free dorsal socket.
fn list_the_dorsal_sockets(world: &mut World) {
    let ship = the_ship(world);
    let live = live_ids(world, ship);
    let design = world
        .get::<ShipDesignSections>(ship)
        .expect("refit catalogue: the player ship carries its design");
    let placed: Vec<RefitPlacedSection<'_>> = design
        .0
        .iter()
        .map(|section| RefitPlacedSection {
            position: section.position,
            rotation: section.rotation,
            config: &section.config,
            working: true,
        })
        .collect();
    let mut candidates = Vec::new();
    for (index, section) in design.0.iter().enumerate() {
        if !live.contains(&section.id) {
            continue;
        }
        for point in free_link_points(&placed, index) {
            let socket = &section.config.base.link_points[point];
            let (anchor, normal) = link_point_pose(section.position, section.rotation, socket);
            if normal.y >= DORSAL_MIN_Y {
                candidates.push(Aim {
                    section: section.id.clone(),
                    point: socket.id.clone(),
                    at: anchor.lerp(section.position, AIM_INSET),
                });
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "refit catalogue: the player hull has no free dorsal socket"
    );
    info!(
        "refit catalogue: {} free dorsal socket(s), first {:?}",
        candidates.len(),
        candidates[0]
    );
    world.insert_resource(Aiming {
        candidates,
        index: usize::MAX,
        target: None,
    });
}

fn pdc_in_hand() -> Arc<Predicate> {
    when(|world| {
        world.resource::<HeldPart>().id() == Some(PDC_KINETIC_TURRET_SECTION_ID)
            && !world.resource::<GalleryState>().open
    })
}

/// The ghost stands valid at the socket pointed at.
fn ghost_on_target(world: &World) -> bool {
    let Some(target) = world.resource::<Aiming>().target.clone() else {
        return false;
    };
    ship_ghost_aim(world) == Some((target.section, target.point, true))
}

/// Point at the dorsal sockets in turn, [`AIM_FRAMES`] frames each, until
/// the ghost stands valid at the one pointed at.
fn aim_the_ghost(world: &mut World, _elapsed: f32, frame: u32) {
    if ghost_on_target(world) || !frame.is_multiple_of(AIM_FRAMES) {
        return;
    }
    let index = world.resource::<Aiming>().index.wrapping_add(1);
    let candidates = world.resource::<Aiming>().candidates.clone();
    let aim = candidates.get(index).cloned().unwrap_or_else(|| {
        let verdict = ship_placement(world).map(|placement| placement.verdict.clone());
        panic!(
            "refit catalogue: the ghost stood valid at none of {} dorsal socket(s), last {:?}: \
             {verdict:?}",
            candidates.len(),
            ship_ghost_aim(world)
        )
    });
    let Some(pixel) = ship_view_window_px(world, aim.at) else {
        return;
    };
    info!(
        "refit catalogue: aiming at {} {} through {pixel:?}",
        aim.section, aim.point
    );
    move_cursor(pixel)(world);
    let mut aiming = world.resource_mut::<Aiming>();
    aiming.index = index;
    aiming.target = Some(aim);
}

/// The valid ghost's placement.
///
/// # Panics
///
/// When no placement is aimed, it is refused, it has no pose, or its socket
/// is not dorsal.
fn valid_placement(world: &World, what: &str) -> (Transform, RecipePayment) {
    let placement =
        ship_placement(world).unwrap_or_else(|| panic!("refit catalogue: {what}: no ghost"));
    if let Err(refusal) = &placement.verdict {
        panic!("refit catalogue: {what}: the ghost is refused: {refusal}");
    }
    assert!(
        placement.normal.y >= DORSAL_MIN_Y,
        "refit catalogue: {what}: the ghost's socket faces {:?}, not dorsal",
        placement.normal
    );
    let pose = placement
        .pose
        .unwrap_or_else(|| panic!("refit catalogue: {what}: the ghost has no pose"));
    let payment = placement
        .payment
        .clone()
        .unwrap_or_else(|| panic!("refit catalogue: {what}: a valid ghost has no payment"));
    (pose, payment)
}

/// The valid ghost before the roll: dorsal, unrolled, and the Placing block
/// names the part, its price and 0 deg.
fn assert_the_ghost(world: &mut World) {
    let (pose, payment) = valid_placement(world, "before the roll");
    let mount = world.resource::<ShipRuntime>().mount();
    assert_eq!(
        mount.roll % 4,
        0,
        "refit catalogue: the hand starts unrolled"
    );
    let name = pdc(world).base.name.clone();
    let price = cr_text(payment.price_cr);
    for value in [name.as_str(), price.as_str(), "0 deg"] {
        assert!(
            placing_reads(world, value),
            "refit catalogue: the Placing block does not read '{value}': {:?}",
            texts_under(world, SHIP_HELD_PANEL)
        );
    }
    info!(
        "refit catalogue: the ghost stands valid at {:?} for {price}",
        ship_ghost_aim(world)
    );
    world.insert_resource(Unrolled(pose));
}

/// The hand rolled one quarter turn.
fn rolled() -> Arc<Predicate> {
    when(|world| {
        world.resource::<ShipRuntime>().mount().roll % 4 == 1
            && ship_placement(world).is_some_and(|placement| placement.mount.roll % 4 == 1)
    })
}

/// The rolled ghost: still valid at the same dorsal socket, a quarter turn
/// off the unrolled pose, the Placing block reads 90 deg, and the station
/// sells the whole recipe at ask. Record what a click commits and the books
/// before it.
fn assert_the_roll(world: &mut World) {
    let target = world
        .resource::<Aiming>()
        .target
        .clone()
        .expect("refit catalogue: a socket is pointed at");
    let aim = ship_ghost_aim(world);
    assert_eq!(
        aim,
        Some((target.section.clone(), target.point.clone(), true)),
        "refit catalogue: the rolled ghost left its socket or was refused: {:?}",
        ship_placement(world).map(|placement| placement.verdict.clone())
    );
    let (pose, payment) = valid_placement(world, "after the roll");
    let unrolled = world.resource::<Unrolled>().0;
    let turn = unrolled.rotation.angle_between(pose.rotation);
    assert!(
        (turn - std::f32::consts::FRAC_PI_2).abs() < ROLL_TOLERANCE_RAD,
        "refit catalogue: the roll turned the ghost {turn} rad, not a quarter turn"
    );
    assert!(
        placing_reads(world, "90 deg"),
        "refit catalogue: the Placing block does not read 90 deg: {:?}",
        texts_under(world, SHIP_HELD_PANEL)
    );
    let recipe = &pdc(world).base.build;
    assert!(
        payment.from_hold.is_empty(),
        "refit catalogue: the start hold pays {:?}, not none of the PDC's parts",
        payment.from_hold
    );
    assert_eq!(
        payment.bought, recipe.parts,
        "refit catalogue: the station sells every part the PDC's recipe takes"
    );
    assert_eq!(
        u64::from(payment.price_cr),
        recipe.value_cr(world.resource::<GameItems>()),
        "refit catalogue: the price is the PDC's recipe at ask"
    );
    let ship = the_ship(world);
    let station = station(world);
    let design = world
        .get::<ShipDesignSections>(ship)
        .expect("refit catalogue: the player ship carries its design");
    let committed = Committed {
        section: target.section,
        point: target.point,
        pose,
        id: added_section_id(design, SectionClass::Turret),
        player_cr: credits(world, ship),
        station_cr: credits(world, station),
        hold: counts(world, ship, &payment.from_hold),
        stock: counts(world, station, &payment.bought),
        payment,
    };
    info!("refit catalogue: committing {committed:?}");
    world.insert_resource(committed);
}

/// The added PDC is live under a section code.
fn pdc_added() -> Arc<Predicate> {
    when(|world| {
        let id = &world.resource::<Committed>().id;
        player(world).is_some_and(|ship| {
            section_of(world, ship, id).is_some_and(|pdc| world.get::<SectionCode>(pdc).is_some())
        })
    })
}

/// How far the new PDC sits off the Ship view's centre, in logical pixels.
fn offset_from_centre(world: &World) -> f32 {
    let ship = the_ship(world);
    let id = &world.resource::<Committed>().id;
    let pdc = section_of(world, ship, id).expect("refit catalogue: the PDC is live");
    let (at, _) = local_pose(world, pdc, ship);
    let pixel =
        ship_view_window_px(world, at).expect("refit catalogue: the PDC projects into the view");
    let view = ui_node_rect(world, "ShipViewport").expect("refit catalogue: the Ship view is up");
    pixel.distance(view.center())
}

/// The add's note line reads in the Placing block, and where the view's
/// ease starts. Read the frame the PDC goes live: the note shows for 2.5
/// seconds, which the ease beat outlasts on an unclocked software run.
fn check_the_note_and_record_the_ease(world: &mut World) {
    let ship = the_ship(world);
    let id = world.resource::<Committed>().id.clone();
    let section = section_of(world, ship, &id).expect("refit catalogue: the PDC is live");
    let code = world
        .get::<SectionCode>(section)
        .expect("refit catalogue: the PDC is coded")
        .0
        .clone();
    let name = pdc(world).base.name.clone();
    let note = world
        .resource::<ShipRuntime>()
        .note()
        .map(ToString::to_string);
    let added = format!("added {name} as {code} on ");
    assert!(
        note.as_deref().is_some_and(|note| note.starts_with(&added)),
        "refit catalogue: the note reads {note:?}, not '{added}...'"
    );
    assert!(
        texts_under(world, SHIP_HELD_PANEL)
            .iter()
            .any(|text| text.starts_with(&added)),
        "refit catalogue: the Placing block's note line does not read the add: {:?}",
        texts_under(world, SHIP_HELD_PANEL)
    );
    let from = offset_from_centre(world);
    info!("refit catalogue: the PDC went live {from:.1} px off the view's centre");
    world.insert_resource(EaseFrom(from));
}

/// The fit: the PDC live at the rolled ghost's pose under a `PDC-` code and
/// selected, the price and the parts moved, no ghost aimed, and the view
/// eased onto it.
fn assert_the_fit(world: &mut World) {
    let committed = world.resource::<Committed>().clone();
    let ship = the_ship(world);
    let station = station(world);
    let pdc = section_of(world, ship, &committed.id).expect("refit catalogue: the PDC is live");

    let code = world
        .get::<SectionCode>(pdc)
        .expect("refit catalogue: the PDC is coded")
        .0
        .clone();
    assert!(
        code.starts_with("PDC-"),
        "refit catalogue: the new section is coded '{code}', not PDC-"
    );
    assert_eq!(
        world.resource::<ShipRuntime>().selected(),
        Some(pdc),
        "refit catalogue: the pane selects the new PDC"
    );

    let (position, rotation) = local_pose(world, pdc, ship);
    assert!(
        position.distance(committed.pose.translation) < POSE_TOLERANCE
            && rotation.angle_between(committed.pose.rotation) < POSE_TOLERANCE,
        "refit catalogue: the PDC stands at {position:?} {rotation:?}, the ghost at {:?}",
        committed.pose
    );

    let price = committed.payment.price_cr;
    assert_eq!(
        credits(world, ship),
        committed.player_cr - price,
        "refit catalogue: the tug's balance pays {price} cr"
    );
    assert_eq!(
        credits(world, station),
        committed.station_cr + price,
        "refit catalogue: the station earns {price} cr"
    );
    for (item, count) in counts(world, ship, &committed.payment.from_hold) {
        assert_eq!(
            count,
            committed.hold[&item] - committed.payment.from_hold[&item],
            "refit catalogue: the hold gives its {item}"
        );
    }
    for (item, count) in counts(world, station, &committed.payment.bought) {
        assert_eq!(
            count,
            committed.stock[&item] - committed.payment.bought[&item],
            "refit catalogue: the station sells its {item}"
        );
    }

    assert!(
        ship_placement(world).is_none(),
        "refit catalogue: a ghost is still aimed after the pointer left the view"
    );
    let from = world.resource::<EaseFrom>().0;
    let now = offset_from_centre(world);
    assert!(
        now < from && now <= EASED_PX,
        "refit catalogue: the view eased the PDC from {from:.1} px to {now:.1} px off centre, \
         not within {EASED_PX} px"
    );
    info!(
        "refit catalogue: fitted {code} on {} {} for {}, the view eased {from:.1} px to \
         {now:.1} px",
        committed.section,
        committed.point,
        cr_text(price)
    );
}

/// The walk: wait at the starting station, open the Ship tab, then the
/// Refit loop.
fn script() -> AutopilotPlugin<GameStates> {
    AutopilotPlugin::<GameStates>::new()
        .step("refit catalogue: the window streams in and the start docks")
        // `window_settled` reads through a query state and needs the world
        // mutably, which a predicate never has, so each frame records it.
        .each(|world: &mut World, _, _| record_the_dock(world))
        .until(when(|world| world.contains_resource::<Docked>()))
        .deadline(SETUP_SECS)
        .add()
        .step("refit catalogue: check the station")
        .on_enter(|world: &mut World| {
            hide_dev_overlays(world);
            hide_status_bar(world);
            assert_the_station(world);
        })
        .add()
        .step("refit catalogue: press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("refit catalogue: let the interface key up")
        .on_enter(release_action("interface_toggle"))
        .until(when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Interface)
        }))
        .deadline(BEAT_SECS)
        .add()
        .click_named(
            "refit catalogue: open the Ship tab",
            "InterfaceTabShip",
            and(
                the_interface_shows(InterfacePaneType::Ship),
                ui_node_present("ShipCatalogue"),
            ),
            BEAT_SECS,
        )
        .step("refit catalogue: settle the schematic")
        .until(frames(PANE_FRAMES))
        .add()
        .step("refit catalogue: list the dorsal sockets")
        .on_enter(list_the_dorsal_sockets)
        .add()
        .step("refit catalogue: open the loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .add()
        .step("refit catalogue: hold the Ship pane")
        .until(frames(HOLD_FRAMES))
        .add()
        .click_named(
            "refit catalogue: open the Catalogue",
            "ShipCatalogue",
            ui_node_present("Gallery Grid"),
            BEAT_SECS,
        )
        .step("refit catalogue: hold the catalogue")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("refit catalogue: press the Weapons chip")
        .on_enter(click_named(format!(
            "Gallery Category {}",
            GalleryCategory::Weapons.label()
        )))
        .until(pointer_pressed())
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: let the Weapons chip up")
        .on_enter(release_mouse(MouseButton::Left))
        .until(when(|world| {
            world.resource::<GalleryState>().category == GalleryCategory::Weapons
        }))
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: hold the Weapons grid")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("refit catalogue: press the kinetic PDC's tile")
        .on_enter(|world: &mut World| click_named(pdc_tile(world))(world))
        .until(pointer_pressed())
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: let the kinetic PDC's tile up")
        .on_enter(release_mouse(MouseButton::Left))
        .until(ui_node_present("Gallery Place Button"))
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: hold the focus card")
        .until(frames(HOLD_FRAMES))
        .add()
        // A part the station cannot build or the holds cannot pay for keeps
        // the hand empty, so this beat stalls on a muted tile and names it.
        .click_named(
            "refit catalogue: take the kinetic PDC in hand",
            "Gallery Place Button",
            and(pdc_in_hand(), ui_node_present(SHIP_HELD_PANEL)),
            BEAT_SECS,
        )
        .step("refit catalogue: hold the Placing block")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("refit catalogue: aim the ghost at a dorsal socket")
        .each(aim_the_ghost)
        .until(when(ghost_on_target))
        .deadline(AIM_SECS)
        .add()
        .step("refit catalogue: hold the valid ghost")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("refit catalogue: check the valid ghost")
        .on_enter(assert_the_ghost)
        .add()
        .step("refit catalogue: press R")
        .on_enter(press_action("ship_part_roll"))
        .until(frames(1))
        .add()
        .step("refit catalogue: let R up")
        .on_enter(release_action("ship_part_roll"))
        .until(rolled())
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: hold the rolled ghost")
        .until(frames(HOLD_FRAMES))
        .add()
        .step("refit catalogue: check the rolled ghost")
        .on_enter(assert_the_roll)
        .add()
        .step("refit catalogue: press on the ghost")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_SECS)
        .add()
        // The click commits on the release over the view and the part stays
        // in hand, so the pointer leaves for the Placing block the frame the
        // price moves. A later move let the next solves aim a second ghost
        // beside the new PDC, refused on the stock the fit drew down.
        .step("refit catalogue: let the click up and fit")
        .on_enter(release_mouse(MouseButton::Left))
        .each(|world: &mut World, _, _| {
            if credits(world, station(world)) != world.resource::<Committed>().station_cr {
                hover_named(SHIP_HELD_PANEL)(world);
            }
        })
        .until(pdc_added())
        .deadline(BEAT_SECS)
        .add()
        .step("refit catalogue: the view eases onto the PDC")
        .on_enter(check_the_note_and_record_the_ease)
        .until(frames(EASE_FRAMES))
        .add()
        .step("refit catalogue: check the fit")
        .on_enter(assert_the_fit)
        .add()
        .step("refit catalogue: hold the eased view")
        .until(frames(END_FRAMES))
        .add()
        .step("refit catalogue: close the loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}
