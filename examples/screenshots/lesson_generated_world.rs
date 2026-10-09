//! lesson_generated_world: the handbook's two generated-world demonstrations -
//! `start_generated_ships` (a generated derelict travel-locked beside the
//! player, the target inset naming its civilization and former role) and
//! `interface_wreck_take` (the Inventory pane docked to that derelict: Take one
//! unit of its actual stock, then Give one back).
//!
//! One producer, two lessons, because they are one session: the still is shot
//! on the wreck the sheet then docks with.
//!
//! The walk boots the shipped app (via [`editor_app`]), clicks New Game, types
//! [`WORLD_SEED`], clicks Create and waits for the live window, the same entry
//! as `system_open_world_identity`. The seed puts a derelict about 9 km from the
//! spawn in the first window, so no beat has to fly there.
//!
//! ## The berth is posed, the dock and the transfer are not
//!
//! Docking is taught by `flight_dock` and `build_dock_envelope`, so this walk
//! does not fly the approach. It moves the player's line warship so one of its
//! docking ports faces one of the wreck's ports [`BERTH_GAP`] away, square and
//! at rest - inside the shipped capture envelope - and writes the travel lock a
//! radar dwell would. Everything after that is the player's input: the dock
//! key, the interface key, and pointer clicks on the Inventory pane's own named
//! widgets. Each transfer waits for the pane's note line and then asserts both
//! holds in the ECS, so a click that missed stalls on a named beat instead of
//! shipping a sheet of a transfer that did not happen.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole session, assert
//!   every transfer, exit clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the still and tile the sheet
//!   (staged under `NOVA_CAPTURE_DIR`), plus [`GIVE_SHOT`], the frame of the
//!   Give note the sheet is too short to reach.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_generated_world --features debug
//! ```

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;

#[cfg(feature = "debug")]
use avian3d::prelude::*;
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::pane::InterfacePaneType;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::SectorRoot;

#[derive(Parser)]
#[command(name = "lesson_generated_world")]
#[command(version = "1.0.0")]
#[command(
    about = "Record the handbook's generated-ship and wreck Take/Give demonstrations",
    long_about = None
)]
struct Cli;

/// The still for "Generated ships".
#[cfg(feature = "debug")]
const SHIPS_STILL: &str = "start_generated_ships.png";
/// The sheet for "Looting a wreck".
#[cfg(feature = "debug")]
const TAKE_LESSON: &str = "interface_wreck_take";
/// The frame of the Give note: proof, not a lesson demonstration.
#[cfg(feature = "debug")]
const GIVE_SHOT: &str = "generated_world_give.png";

/// The seed the walk plays: `system_open_world_identity`'s, whose first window
/// holds a derelict 9092 m from the spawn.
#[cfg(feature = "debug")]
const WORLD_SEED: u32 = 115;

#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
#[cfg(feature = "debug")]
const WORLD_NAME_FIELD: &str = "World Name Field";
#[cfg(feature = "debug")]
const WORLD_NAME: &str = "Probe World";
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";

/// How many cells the open world keeps live: radius 2 is a 5x5x5 window.
#[cfg(feature = "debug")]
const LIVE_SECTORS: usize = 125;

/// Seconds a load or a stream gets on a software-rendered GPU.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// The gap between the two retracted port faces: half the shipped 10 m
/// capture distance.
#[cfg(feature = "debug")]
const BERTH_GAP: Meters = Meters(5.0);

/// Where a retracted port's mouth sits in its own frame: half a cell along
/// the section's outward axis, local -Z. The docking section's own constant is
/// crate-private.
#[cfg(feature = "debug")]
const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

/// Cells of the sheet on the docked pane before the first click.
#[cfg(feature = "debug")]
const TAKE_LEAD_CELLS: u32 = 2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings and saved worlds, set before the app reads
    // them: Create makes a world, and neither a player's worlds nor an
    // earlier run's "Probe World" may be in its way.
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

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_systems(Update, hold_the_berth);
        app.add_plugins(world_script());
    }

    app.run()
}

/// The derelict the walk docks with, and what it carried when picked.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone)]
struct Wreck {
    entity: Entity,
    name: String,
    /// The item the walk takes and gives back: the lowest-ordered item the
    /// wreck's actual stock carries, picked once here. A generated wreck is no
    /// longer guaranteed a [`ItemType::HullPlate`], so Take and Give must work
    /// from whatever it actually holds, and never reroll it.
    item: ItemType,
    /// The two holds' counts of `item` when the wreck was picked: the
    /// player's, then the wreck's.
    stock: (u32, u32),
}

/// While present, the player and the wreck are held at rest: the berth is a
/// pose, and a drift of a few meters a second would carry the pair out of the
/// capture envelope before the key goes down.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct HoldBerth;

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

/// How many of `item` a ship's hold carries.
#[cfg(feature = "debug")]
fn count_of(world: &World, ship: Entity, item: ItemType) -> u32 {
    world
        .get::<ShipInventory>(ship)
        .expect("generated world: a ship carries a ShipInventory")
        .count(item)
}

/// Pick the nearest lootable derelict, named the way the world names one, and
/// the lowest-[`ItemType`]-ordered item its actual stock carries: the wreck's
/// mixed loot is no longer guaranteed a hull plate.
#[cfg(feature = "debug")]
fn pick_the_wreck(world: &mut World) {
    let player = the_player(world).expect("generated world: exactly one player ship");
    let origin = world
        .get::<GlobalTransform>(player)
        .expect("generated world: the player ship has a transform")
        .translation();
    let (entity, name) = world
        .query_filtered::<(Entity, &Name, &GlobalTransform), (
            With<DerelictShipMarker>,
            With<LootableShipMarker>,
        )>()
        .iter(world)
        .map(|(entity, name, at)| (at.translation().distance(origin), entity, name.to_string()))
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)))
        .map(|(_, entity, name)| (entity, name))
        .unwrap_or_else(|| panic!("generated world: seed {WORLD_SEED} streamed no derelict"));
    assert!(
        name.contains(" derelict, former "),
        "generated world: derelict {entity:?} is named '{name}'"
    );
    let item = world
        .get::<ShipInventory>(entity)
        .expect("generated world: a ship carries a ShipInventory")
        .stacks()
        .next()
        .map(|(item, _)| item)
        .unwrap_or_else(|| panic!("generated world: derelict '{name}' carries no stock"));
    let stock = (count_of(world, player, item), count_of(world, entity, item));
    assert!(
        stock.1 > 0,
        "generated world: derelict '{name}' carries no {item:?}"
    );
    info!(
        "generated world: wreck '{name}' {entity:?}, item {item:?} player {} wreck {}",
        stock.0, stock.1
    );
    world.insert_resource(Wreck {
        entity,
        name,
        item,
        stock,
    });
}

/// The working docking ports of `ship`, ordered by their cell in the hull so
/// the choice is stable across runs: every port carries the same `Name`.
#[cfg(feature = "debug")]
fn ports_of(world: &mut World, ship: Entity) -> Vec<(Vec3, GlobalTransform)> {
    let mut ports: Vec<(Vec3, GlobalTransform)> = world
        .query_filtered::<(&ChildOf, &Transform, &GlobalTransform), (
            With<DockingSectionMarker>,
            Without<SectionInactiveMarker>,
        )>()
        .iter(world)
        .filter(|(parent, ..)| parent.parent() == ship)
        .map(|(_, cell, at)| (cell.translation, *at))
        .collect();
    ports.sort_by(|a, b| {
        a.0.x
            .total_cmp(&b.0.x)
            .then(a.0.y.total_cmp(&b.0.y))
            .then(a.0.z.total_cmp(&b.0.z))
    });
    ports
}

/// Move the player so its first port faces the wreck's first port
/// [`BERTH_GAP`] away, axes opposed, and hold both at rest.
#[cfg(feature = "debug")]
fn berth_beside_the_wreck(world: &mut World) {
    let wreck = world.resource::<Wreck>().clone();
    let player = the_player(world).expect("generated world: exactly one player ship");
    let (wreck_port, wreck_at) = ports_of(world, wreck.entity)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("generated world: '{}' has no docking port", wreck.name));
    let (player_port, player_at) = ports_of(world, player)
        .into_iter()
        .next()
        .expect("generated world: the line warship has a docking port");
    let root = *world
        .get::<GlobalTransform>(player)
        .expect("generated world: the player ship has a transform");

    // Engine boundary: a transform counts world units, the gap is meters.
    let wreck_axis = wreck_at.rotation() * Vec3::NEG_Z;
    let target_face = wreck_at.transform_point(PORT_FACE) + wreck_axis * BERTH_GAP.to_engine();
    let to_root = root.affine().inverse();
    let local_face = to_root.transform_point3(player_at.transform_point(PORT_FACE));
    let local_axis = (root.rotation().inverse() * player_at.rotation()) * Vec3::NEG_Z;
    let rotation = Quat::from_rotation_arc(local_axis, -wreck_axis);
    let translation = target_face - rotation * local_face;

    let mut ship = world.entity_mut(player);
    if let Some(mut transform) = ship.get_mut::<Transform>() {
        transform.translation = translation;
        transform.rotation = rotation;
    }
    if let Some(mut position) = ship.get_mut::<Position>() {
        position.0 = translation;
    }
    if let Some(mut body) = ship.get_mut::<Rotation>() {
        body.0 = rotation;
    }
    world.insert_resource(HoldBerth);
    info!(
        "generated world: berthed the port at cell {player_port} against '{}' port at cell \
         {wreck_port}",
        wreck.name
    );
}

/// Zero both ships' motion every frame the berth is held.
#[cfg(feature = "debug")]
fn hold_the_berth(
    hold: Option<Res<HoldBerth>>,
    wreck: Option<Res<Wreck>>,
    player: Query<Entity, With<PlayerSpaceshipMarker>>,
    mut bodies: Query<(&mut LinearVelocity, &mut AngularVelocity)>,
) {
    let (Some(_), Some(wreck)) = (hold, wreck) else {
        return;
    };
    for ship in player.iter().chain([wreck.entity]) {
        if let Ok((mut linear, mut angular)) = bodies.get_mut(ship) {
            linear.0 = Vec3::ZERO;
            angular.0 = Vec3::ZERO;
        }
    }
}

/// Travel-lock the wreck, the way a radar dwell does.
#[cfg(feature = "debug")]
fn lock_the_wreck(world: &mut World) {
    let target = world.resource::<Wreck>().entity;
    let player = the_player(world).expect("generated world: exactly one player ship");
    world
        .get_mut::<TravelLock>(player)
        .expect("generated world: the player ship carries a travel lock")
        .0 = Some(target);
}

/// Advance once the inset caption's first line is the wreck's name.
#[cfg(feature = "debug")]
fn the_inset_names_the_wreck() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let name = &world.resource::<Wreck>().name;
        world
            .try_query_filtered::<&Text, With<TargetInsetCaptionMarker>>()
            .is_some_and(|mut captions| {
                captions
                    .iter(world)
                    .any(|caption| caption.0.lines().next() == Some(name.as_str()))
            })
    })
}

/// Advance once the player is docked to the wreck.
#[cfg(feature = "debug")]
fn docked_to_the_wreck() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| {
        let wreck = world.resource::<Wreck>().entity;
        the_player(world)
            .and_then(|player| world.get::<DockedShip>(player))
            .and_then(|docked| world.get::<DockingConnection>(docked.connection))
            .is_some_and(|connection| connection.joins(wreck))
    })
}

/// Advance once the interface shows `pane`.
#[cfg(feature = "debug")]
fn the_interface_shows(
    pane: InterfacePaneType,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<State<PauseStates>>()
            .is_some_and(|pause| *pause.get() == PauseStates::Interface)
            && world
                .get_resource::<InterfacePaneType>()
                .is_some_and(|shown| *shown == pane)
    })
}

/// Advance once a laid-out, visible text reads the note `verb` 1 `<item>`
/// with the wreck's name, as the pane's note line writes it.
#[cfg(feature = "debug")]
fn the_note_reads(
    verb: &'static str,
    preposition: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let wreck = world.resource::<Wreck>();
        let note = format!(
            "{verb} 1 {} {preposition} {}",
            wreck.item.label(),
            wreck.name
        );
        world
            .try_query::<(&Text, &ComputedNode, &InheritedVisibility)>()
            .is_some_and(|mut texts| {
                texts
                    .iter(world)
                    .any(|(text, node, shown)| text.0 == note && shown.get() && node.size().x > 0.0)
            })
    })
}

/// Assert both holds moved by `moved` units of the wreck's item from the wreck
/// to the player, and that the pair holds every unit it started with.
#[cfg(feature = "debug")]
fn check_stock(stage: &'static str, moved: i64) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let wreck = world.resource::<Wreck>().clone();
        let player = the_player(world).expect("generated world: exactly one player ship");
        let now = (
            count_of(world, player, wreck.item),
            count_of(world, wreck.entity, wreck.item),
        );
        let want = (
            (i64::from(wreck.stock.0) + moved) as u32,
            (i64::from(wreck.stock.1) - moved) as u32,
        );
        assert_eq!(
            now, want,
            "generated world {stage}: (player, wreck) {:?} counts",
            wreck.item
        );
        info!(
            "generated world {stage}: player {} wreck {} {:?}, {} in the pair",
            now.0,
            now.1,
            wreck.item,
            now.0 + now.1
        );
    }
}

/// The widget name [`inventory_row`](nova_interface) gives the wreck's
/// selected item's row in the `side` column (`"Own"` or `"Partner"`).
#[cfg(feature = "debug")]
fn row_name(side: &str, item: ItemType) -> String {
    format!("InventoryRow{side}{item:?}")
}

/// Advance once the docked pane lays out a visible row for the wreck's item in
/// the `side` column (`"Own"` or `"Partner"`).
///
/// Resolved from [`Wreck`] at the predicate's own poll rather than baked into
/// the script: the item is not known until the wreck is picked, long after
/// `world_script` builds every step.
#[cfg(feature = "debug")]
fn row_present(
    side: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        let item = world.resource::<Wreck>().item;
        ui_node_rect(world, &row_name(side, item)).is_some()
    })
}

/// One pointer click on the UI node `name` inside a recording: aim on entry,
/// press on the second driven frame, release on the third, then hold until
/// `landed`.
///
/// `click_named` spends a beat each on layout, aim, press and release, about
/// ten cells of a twenty-cell sheet for one Take. Here the node is already laid
/// out on a still pane, so the gesture costs three frames; a missed aim
/// activates nothing and stalls on `landed`.
#[cfg(feature = "debug")]
fn quick_click(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    name: &'static str,
    landed: std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(label)
        .on_enter(hover_named(name))
        .each(|world: &mut World, _, frame| match frame {
            2 => press_mouse(MouseButton::Left)(world),
            3 => release_mouse(MouseButton::Left)(world),
            _ => {}
        })
        .until(landed)
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// Like [`quick_click`], but aims at the wreck's selected item's row in the
/// `side` column, resolved from [`Wreck`] at the beat's own entry rather than
/// baked into the script: see [`row_present`].
#[cfg(feature = "debug")]
fn quick_click_item(
    script: nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates>,
    label: &str,
    side: &'static str,
    landed: std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>,
) -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    script
        .step(label)
        .on_enter(move |world: &mut World| {
            let item = world.resource::<Wreck>().item;
            hover_named(row_name(side, item))(world);
        })
        .each(|world: &mut World, _, frame| match frame {
            2 => press_mouse(MouseButton::Left)(world),
            3 => release_mouse(MouseButton::Left)(world),
            _ => {}
        })
        .until(landed)
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}

/// The walk: into the world, berth at the nearest derelict, shoot it locked,
/// dock, then Take and Give through the Inventory pane.
#[cfg(feature = "debug")]
fn world_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("name the world")
        .on_enter(|world: &mut World| {
            let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
            let (_, mut value) = fields
                .iter_mut(world)
                .find(|(name, _)| name.as_str() == WORLD_NAME_FIELD)
                .expect("generated world: the modal has a name field");
            value.0 = WORLD_NAME.to_string();
        })
        .until(frames(2))
        .add()
        .step("enter the seed")
        .on_enter(|world: &mut World| {
            let mut fields = world.query::<(&Name, &mut TextFieldValue)>();
            let (_, mut value) = fields
                .iter_mut(world)
                .find(|(name, _)| name.as_str() == SEED_FIELD)
                .expect("generated world: the modal has a seed field");
            value.0 = WORLD_SEED.to_string();
        })
        .until(frames(2))
        .add()
        .click_named(
            "click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("the window streams around the player")
        .until(std::sync::Arc::new(|world: &World| {
            the_player(world).is_some()
                && world
                    .try_query_filtered::<(), With<SectorRoot>>()
                    .is_some_and(|mut roots| roots.iter(world).count() == LIVE_SECTORS)
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("pick the nearest derelict")
        .on_enter(|world: &mut World| {
            assert_eq!(
                world.resource::<OpenWorldSession>().seed,
                WORLD_SEED,
                "generated world: Create must start the typed seed"
            );
            pick_the_wreck(world);
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .add()
        .step("berth beside the wreck")
        .on_enter(berth_beside_the_wreck)
        .until(frames(2))
        .add()
        .step("travel-lock the wreck")
        .on_enter(lock_the_wreck)
        .until(the_inset_names_the_wreck())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("settle the berth and the inset")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("shoot the locked wreck")
        .on_enter(|world: &mut World| {
            assert!(
                the_inset_names_the_wreck()(world),
                "generated world: the inset lost the wreck's name"
            );
            shoot(world, SHIPS_STILL);
        })
        .until(shot_written(SHIPS_STILL))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("press the dock key")
        .on_enter(press_action("dock"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the dock")
        .on_enter(release_action("dock"))
        .until(docked_to_the_wreck())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("let the joint hold the pair")
        .on_enter(|world: &mut World| {
            world.remove_resource::<HoldBerth>();
        })
        .until(frames(2))
        .add()
        .step("press the interface key")
        .on_enter(press_action("interface_toggle"))
        .until(frames(1))
        .add()
        .step("let the key up and wait for the interface")
        .on_enter(release_action("interface_toggle"))
        .until(the_interface_shows(InterfacePaneType::Map))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .click_named(
            "open the Inventory pane",
            "InterfaceTabInventory",
            row_present("Partner"),
            BEAT_DEADLINE_SECS,
        )
        .step("settle the docked pane")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the sheet on the docked pane")
        .on_enter(|world: &mut World| sheet_start(world, TAKE_LESSON, LESSON_GRID))
        .until(frames(TAKE_LEAD_CELLS))
        .add();
    let script = quick_click_item(
        script,
        "take: click the wreck's stock",
        "Partner",
        ui_node_present("InventoryDraftConfirm"),
    );
    let script = quick_click(
        script,
        "take: click Confirm",
        "InventoryDraftConfirm",
        the_note_reads("Took", "from"),
    );
    let script = script
        .step("check the Take")
        .on_enter(check_stock("after the Take", 1))
        .until(frames(1))
        .add()
        .step("hold the Take note to the end of the sheet")
        .until(sheet_written(TAKE_LESSON))
        .deadline(60.0)
        .add();
    let script = quick_click_item(
        script,
        "give: pick your stock",
        "Own",
        ui_node_present("InventoryDraftConfirm"),
    );
    script
        .click_named(
            "give: confirm",
            "InventoryDraftConfirm",
            the_note_reads("Gave", "to"),
            BEAT_DEADLINE_SECS,
        )
        // Shot on the frame the note lands: the note line clears, so a settle
        // here would photograph the restored counts without the Give.
        .step("check and shoot the Give")
        .on_enter(check_stock("after the Give", 0))
        .on_enter(|world: &mut World| shoot(world, GIVE_SHOT))
        .until(shot_written(GIVE_SHOT))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}
