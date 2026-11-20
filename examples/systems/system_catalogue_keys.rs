//! system_catalogue_keys: TAB and Esc in the Ship pane's catalogue while the
//! ship is docked at a station that refits.
//!
//! The player flies the base `mining_tug` and docks at a living waystation.
//! TAB opens the interface and a click opens the Ship pane. There TAB is the
//! catalogue's key: the footer's Catalogue button carries its chip, and the
//! top bar's Close chip names Esc. TAB opens the catalogue; the wheel over the
//! grid pages it; Weapons and the filter `kinetic` narrow it; Place puts the
//! kinetic PDC in hand and a click on the ghost adds it, the part still in
//! hand. TAB puts the part away and returns to the catalogue on Weapons,
//! `kinetic` and the same page; TAB closes it again and the interface stays.
//! Esc then steps back one layer per press: the filter caret, the focus card,
//! the catalogue, the part in hand, the interface. Last, a pane switch closes
//! the catalogue and TAB in the Map pane closes the interface.
//!
//! THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: the hold carries
//! the kinetic PDC recipe's parts (two panels, an actuator, a turret body and
//! a barrel) and 1,000 cr.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: the catalogue keys are named` | at the refit dock the Catalogue button's chip reads the `interface_toggle` key and the Close chip reads Esc |
//! | 2 | `outcome: tab opens and the wheel pages the catalogue` | TAB opens the catalogue with the interface still on the Ship pane; one wheel notch down over the grid moves the selection one page, one notch up moves it back |
//! | 3 | `outcome: tab returns to the catalogue as it was left` | after Place and an add the part is still in hand; TAB empties the hand and opens the catalogue on Weapons, `kinetic` and the page it left; TAB closes it with the interface still on the Ship pane |
//! | 4 | `outcome: escape steps back one layer per press` | Esc leaves the caret, then the card, then the catalogue, then the part in hand, then the interface, one each, and nothing else |
//! | 5 | `outcome: tab elsewhere closes only the interface` | a pane switch closes the open catalogue and TAB in the Map pane closes the interface |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_catalogue_keys --features debug
//! # look for the five `outcome:` lines and `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot each step as
//! `catalogue-keys-NN-<step>.png`; `NOVA_CAPTURE_SIZE=1024x768` shoots the
//! same walk at that size.

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_catalogue_keys")]
#[command(version = "1.0.0")]
#[command(about = "TAB opens and closes the Ship pane's catalogue at a refit dock and returns to it as it was left after a Place, the wheel pages its grid, and Esc steps back one layer per press. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -200.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";
/// The player's hull, its mining beam, which a player ship must bind, and
/// its key.
const HULL: &str = "mining_tug";
const MINING_SECTION: &str = "mining_beam";
const MINING_KEY: InputSource = InputSource::Mouse(MouseButton::Right);

/// The player's balance and the kinetic PDC recipe's parts the hold carries.
const PLAYER_CREDITS: u32 = 1_000;
const HOLD: [(&str, u32); 4] = [
    ("SectionPanel", 2),
    ("ActuatorPack", 1),
    ("TurretBody", 1),
    ("TurretBarrel", 1),
];

/// The filter the walk types, which narrows Weapons to the kinetic PDCs.
const FILTER: &str = "kinetic";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.trigger(LoadScenario(station_void(&game_assets)));
}

/// The living waystation and the player's hull, lit by a three-point rig
/// because the engine spawns no light.
fn station_void(game_assets: &GameAssets) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: StationDesignSource::Prototype(BLOCK_WAYSTATION_STATION_ID.into()),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let player = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: PLAYER_ID.to_string(),
            name: "Player".to_string(),
            position: Meters3::ZERO,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Spaceship(SpaceshipConfig {
            allegiance: None,
            controller: SpaceshipController::Player(PlayerControllerConfig {
                input_mapping: [(MINING_SECTION.to_string(), vec![MINING_KEY])].into(),
            }),
            design: ShipDesignSource::prototype(HULL),
            inventory: ShipInventoryStock::new(HOLD.map(|(item, count)| (item.into(), count))),
            credits: PLAYER_CREDITS,
            ..default()
        }),
    };
    ScenarioConfig {
        description: "A waystation that refits, and a player browsing its catalogue.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
            position: None,
            name: EventConfig::OnStart,
            once: false,
            filters: vec![],
            actions: [
                vec![
                    EventActionConfig::SpawnScenarioObject(station),
                    EventActionConfig::SpawnScenarioObject(player),
                ],
                ThreePointRig::around("rig", STATION_POSITION, 1.0).actions(),
            ]
            .concat(),
        }],
        ..ScenarioConfig::new(
            "catalogue_keys".to_string(),
            "Catalogue Keys Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_input::prelude::{InputBindings, InputDeviceType};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_interface::{
            pane::InterfacePaneType,
            ship::{ship_ghost_aim, ship_view_window_px, SectionCode},
        },
    };
    use nova_ui::widget::KeyChipLabel;

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Frames a pane or the catalogue gets to build before it is read.
    const SETTLE_FRAMES: u32 = 10;
    /// Frames the pointer rests on one free socket before the walk tries the
    /// next one.
    const AIM_FRAMES: u32 = 6;
    /// How far the aim sits from the socket toward its section's centre, as
    /// a share of the way: on the section's face, beside the socket.
    const AIM_INSET: f32 = 0.15;

    /// A free socket the kinetic PDC fits at, and where the walk points to
    /// aim the ghost at it, in ship-local space.
    #[derive(Clone, Debug)]
    struct Socket {
        section: String,
        point: String,
        aim: Vec3,
    }

    /// What the walk recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct KeysWatch {
        /// The free sockets the PDC fits at, and the one the pointer is on
        /// (`usize::MAX` before the first).
        sockets: Vec<Socket>,
        aiming: usize,
        /// The design's section count before the add.
        sections_before: usize,
        /// The browse the catalogue was left on: category, filter, selection.
        left: Option<(GalleryCategory, String, usize)>,
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn station_root(world: &World) -> Option<Entity> {
        let mut stations =
            world.try_query_filtered::<(Entity, &EntityId), With<StationMarker>>()?;
        stations
            .iter(world)
            .find(|(_, id)| id.0 == STATION_ID)
            .map(|(entity, _)| entity)
    }

    fn player_root(world: &World) -> Option<Entity> {
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).next()
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("catalogue keys: the player is up")
    }

    fn design(world: &World) -> &ShipDesignSections {
        world
            .get::<ShipDesignSections>(player(world))
            .expect("catalogue keys: the player carries its design sections")
    }

    fn pause(world: &World) -> PauseStates {
        *world.resource::<State<PauseStates>>().get()
    }

    fn pane(world: &World) -> InterfacePaneType {
        *world.resource::<InterfacePaneType>()
    }

    fn gallery(world: &World) -> &GalleryState {
        world.resource::<GalleryState>()
    }

    fn held(world: &World) -> Option<&str> {
        world.resource::<HeldPart>().id()
    }

    fn loaded() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                && player_root(world).is_some_and(|player| {
                    world.get::<Position>(player).is_some()
                        && world.get::<Children>(player).is_some_and(|children| {
                            children
                                .iter()
                                .any(|child| world.get::<SectionCode>(child).is_some())
                        })
                })
        })
    }

    /// The pose `section` holds in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("catalogue keys: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("catalogue keys: a section's chain reaches its root")
                .parent();
        }
        (position, rotation)
    }

    /// The lowest free, live docking port mounted on `root`.
    fn free_port(world: &mut World, root: Entity) -> Entity {
        let ports: Vec<Entity> = world
            .query_filtered::<Entity, (
                With<DockingSectionMarker>,
                Without<DockedPort>,
                Without<SectionInactiveMarker>,
            )>()
            .iter(world)
            .collect();
        ports
            .into_iter()
            .filter(|port| {
                let mut at = *port;
                while let Some(parent) = world.get::<ChildOf>(at).map(ChildOf::parent) {
                    if parent == root {
                        return true;
                    }
                    at = parent;
                }
                false
            })
            .min()
            .unwrap_or_else(|| panic!("catalogue keys: {root:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar and ask to dock,
    /// every frame until the dock takes.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station_root(world).expect("catalogue keys: the station is up");
        let station_port = free_port(world, station);
        let player_port = free_port(world, player);
        let station_at = world.get::<Position>(station).expect("a position").0;
        let station_rot = world.get::<Rotation>(station).expect("a rotation").0;
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let (position, rotation) = parked_hull_pose(
            (station_at, station_rot),
            (port_at, port_rot),
            (own_at, own_rot),
        );
        world.entity_mut(player).insert((
            Position(position),
            Rotation(rotation),
            LinearVelocity(Vec3::ZERO),
            AngularVelocity(Vec3::ZERO),
        ));
        world.trigger(DockingConnectionRequest {
            entity: player,
            target: station,
        });
    }

    fn docked() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    /// Every free socket of a live section, in design order, the kinetic PDC
    /// fits at, by the pane's own rule.
    fn list_the_sockets(world: &mut World) {
        let player = player(world);
        let live: Vec<String> = world
            .get::<Children>(player)
            .into_iter()
            .flatten()
            .filter(|child| world.get::<SectionMarker>(**child).is_some())
            .filter_map(|child| world.get::<EntityId>(*child).map(|id| id.0.clone()))
            .collect();
        let design = design(world);
        let offered = world
            .resource::<GameSections>()
            .get_section(PDC_KINETIC_TURRET_SECTION_ID)
            .expect("catalogue keys: the catalogue has the kinetic PDC");
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
        let live_indices: Vec<usize> = design
            .0
            .iter()
            .enumerate()
            .filter(|(_, section)| live.contains(&section.id))
            .map(|(index, _)| index)
            .collect();
        let mut sockets = Vec::new();
        for &index in &live_indices {
            let section = &design.0[index];
            for point in free_link_points(&placed, index) {
                if section_add_fit(
                    &placed,
                    &live_indices,
                    index,
                    point,
                    offered,
                    MountPose::default(),
                    &[],
                )
                .is_err()
                {
                    continue;
                }
                let (socket, _) = link_point_pose(
                    section.position,
                    section.rotation,
                    &section.config.base.link_points[point],
                );
                sockets.push(Socket {
                    section: section.id.clone(),
                    point: section.config.base.link_points[point].id.clone(),
                    aim: socket.lerp(section.position, AIM_INSET),
                });
            }
        }
        assert!(
            !sockets.is_empty(),
            "catalogue keys: the kinetic PDC fits at a free socket of the hull"
        );
        let sections_before = design.0.len();
        world.insert_resource(KeysWatch {
            sockets,
            aiming: usize::MAX,
            sections_before,
            left: None,
        });
    }

    /// Point at the free sockets in turn until the ghost stands valid at the
    /// one pointed at.
    fn aim_the_ghost(world: &mut World, _elapsed: f32, frame: u32) {
        if ghost_valid(world) || !frame.is_multiple_of(AIM_FRAMES) {
            return;
        }
        let aiming = world.resource::<KeysWatch>().aiming.wrapping_add(1);
        let socket = world
            .resource::<KeysWatch>()
            .sockets
            .get(aiming)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "catalogue keys: the ghost stood valid at no free socket, last aim {:?}",
                    ship_ghost_aim(world)
                )
            });
        let Some(pixel) = ship_view_window_px(world, socket.aim) else {
            return;
        };
        move_cursor(pixel)(world);
        world.resource_mut::<KeysWatch>().aiming = aiming;
    }

    /// The ghost stands valid at the socket the pointer is on.
    fn ghost_valid(world: &World) -> bool {
        let watch = world.resource::<KeysWatch>();
        let Some(socket) = watch.sockets.get(watch.aiming) else {
            return false;
        };
        ship_ghost_aim(world) == Some((socket.section.clone(), socket.point.clone(), true))
    }

    fn pdc_added() -> Arc<Predicate> {
        when(|world| design(world).0.len() > world.resource::<KeysWatch>().sections_before)
    }

    /// The key chip text inside the UI node called `name`.
    fn chip_of(world: &mut World, name: &str) -> String {
        let node = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find(|(_, found)| found.as_str() == name)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("catalogue keys: no node {name}"));
        let mut stack = vec![node];
        while let Some(at) = stack.pop() {
            if world.get::<KeyChipLabel>(at).is_some() {
                return world.get::<Text>(at).expect("a chip label").0.clone();
            }
            stack.extend(world.get::<Children>(at).into_iter().flatten().copied());
        }
        panic!("catalogue keys: {name} carries no key chip");
    }

    fn toggle_key(world: &World) -> String {
        world
            .resource::<InputBindings>()
            .get("interface_toggle")
            .expect("the interface toggle is registered")
            .hint_display(*world.resource::<InputDeviceType>())
    }

    fn marker(world: &mut World, name: &str, data: serde_json::Value) {
        info!("catalogue keys: {name} {data}");
        nova_probe::probe_marker(world, name, data);
    }

    /// Claim 1: the Catalogue chip is the toggle key, the Close chip Esc.
    fn assert_the_chips(world: &mut World) {
        let toggle = toggle_key(world);
        let catalogue = chip_of(world, "ShipCatalogue");
        let close = chip_of(world, "InterfaceClose");
        assert_eq!(catalogue, toggle, "catalogue keys: the Catalogue chip");
        assert_eq!(close, "Esc", "catalogue keys: the Close chip at the refit");
        marker(
            world,
            "outcome: the catalogue keys are named",
            serde_json::json!({ "catalogue": catalogue, "close": close }),
        );
    }

    fn the_interface_shows(shown: InterfacePaneType) -> Arc<Predicate> {
        when(move |world| pause(world) == PauseStates::Interface && pane(world) == shown)
    }

    fn catalogue_open() -> Arc<Predicate> {
        when(|world| {
            gallery(world).open
                && pause(world) == PauseStates::Interface
                && pane(world) == InterfacePaneType::Ship
        })
    }

    fn catalogue_closed_on_the_ship() -> Arc<Predicate> {
        when(|world| {
            !gallery(world).open
                && pause(world) == PauseStates::Interface
                && pane(world) == InterfacePaneType::Ship
        })
    }

    fn unpaused() -> Arc<Predicate> {
        when(|world| pause(world) == PauseStates::Unpaused)
    }

    fn page(world: &World) -> usize {
        world.resource::<GalleryLayout>().page()
    }

    /// Claim 2, the wheel half: one notch each way moved one page each way.
    fn assert_the_wheel(world: &mut World) {
        let selected = gallery(world).selected;
        assert_eq!(selected, 0, "catalogue keys: the notch up came back");
        let page = page(world);
        marker(
            world,
            "outcome: tab opens and the wheel pages the catalogue",
            serde_json::json!({ "page": page }),
        );
    }

    /// Record the browse the catalogue is left on before Place.
    fn record_the_browse(world: &mut World) {
        let state = gallery(world);
        let left = (state.category, state.filter.clone(), state.selected);
        info!("catalogue keys: left the catalogue on {left:?}");
        world.resource_mut::<KeysWatch>().left = Some(left);
    }

    /// Claim 3, the return half: the catalogue is up as it was left, the
    /// hand empty.
    fn assert_the_return(world: &mut World) {
        let left = world
            .resource::<KeysWatch>()
            .left
            .clone()
            .expect("catalogue keys: the browse was recorded");
        let state = gallery(world);
        let back = (state.category, state.filter.clone(), state.selected);
        assert_eq!(back, left, "catalogue keys: TAB returns to the browse");
        assert_eq!(left.0, GalleryCategory::Weapons);
        assert_eq!(left.1, FILTER);
        assert!(!state.focused, "catalogue keys: on the grid");
        assert_eq!(held(world), None, "catalogue keys: the hand is empty");
    }

    /// Press and release one key, one frame each; the step after waits on
    /// `landed`.
    fn tap_key(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        key: KeyCode,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("catalogue keys: press {label}"))
            .on_enter(press_key(key))
            .until(frames(1))
            .add()
            .step(format!("catalogue keys: let {label} up"))
            .on_enter(release_key(key))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Press and release the interface toggle, one frame each.
    fn tap_tab(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("catalogue keys: press TAB {label}"))
            .on_enter(press_action("interface_toggle"))
            .until(frames(1))
            .add()
            .step(format!("catalogue keys: let TAB up {label}"))
            .on_enter(release_action("interface_toggle"))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// Settle, then shoot `catalogue-keys-<name>.png`.
    fn shot(script: AutopilotPlugin<GameStates>, name: &str) -> AutopilotPlugin<GameStates> {
        let path = format!("catalogue-keys-{name}.png");
        let wait = path.clone();
        script
            .step(format!("catalogue keys: settle for {name}"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step(format!("catalogue keys: shoot {name}"))
            .on_enter(move |world: &mut World| shoot(world, &path))
            .until(shot_written(wait))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
    }

    /// Click the UI node `name` and release it once `landed` holds.
    fn click(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        name: impl Into<String>,
        landed: Arc<Predicate>,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("catalogue keys: press {label}"))
            .on_enter(click_named(name))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("catalogue keys: release {label}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// The kinetic PDC's tile, named by its catalogue name.
    fn pdc_tile(world: &World) -> String {
        let pdc = world
            .resource::<GameSections>()
            .get_section(PDC_KINETIC_TURRET_SECTION_ID)
            .expect("catalogue keys: the catalogue lists the kinetic PDC");
        format!("Gallery Tile {}", pdc.base.name)
    }

    /// Click the kinetic PDC's tile, opening its focus card.
    fn focus_the_pdc(
        script: AutopilotPlugin<GameStates>,
        label: &str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("catalogue keys: press the PDC tile {label}"))
            .on_enter(|world: &mut World| click_named(pdc_tile(world))(world))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("catalogue keys: release the PDC tile {label}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(when(|world| gallery(world).focused))
            .deadline(SESSION_SECS)
            .add()
    }

    fn pdc_in_hand() -> Arc<Predicate> {
        when(|world| {
            held(world) == Some(PDC_KINETIC_TURRET_SECTION_ID)
                && !gallery(world).open
                && pause(world) == PauseStates::Interface
        })
    }

    /// One Esc and the layers it must leave standing, as (caret, card,
    /// catalogue, part in hand, interface).
    fn layers(world: &World) -> (bool, bool, bool, bool, bool) {
        let state = gallery(world);
        (
            state.filter_focused,
            state.focused,
            state.open,
            held(world).is_some(),
            pause(world) == PauseStates::Interface,
        )
    }

    fn layers_are(expected: (bool, bool, bool, bool, bool)) -> Arc<Predicate> {
        when(move |world| layers(world) == expected)
    }

    /// Esc, then hold for a settle and check the layers did not move again.
    fn escape(
        script: AutopilotPlugin<GameStates>,
        label: &str,
        expected: (bool, bool, bool, bool, bool),
    ) -> AutopilotPlugin<GameStates> {
        let check = format!("catalogue keys: Esc leaves {label} and nothing else");
        tap_key(
            script,
            &format!("Esc for {label}"),
            KeyCode::Escape,
            layers_are(expected),
        )
        .step(format!("catalogue keys: hold after Esc for {label}"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step(format!("catalogue keys: assert Esc for {label}"))
        .on_enter(move |world: &mut World| {
            assert_eq!(layers(world), expected, "{check}");
        })
        .until(frames(1))
        .add()
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("catalogue keys: load the void")
            .enter(GameStates::Loading)
            .until(loaded())
            .deadline(SESSION_SECS)
            .add()
            .step("catalogue keys: dock")
            .each(|world, _, _| berth_and_dock(world))
            .until(docked())
            .deadline(SESSION_SECS)
            .add()
            .step("catalogue keys: list the free sockets")
            .on_enter(list_the_sockets)
            .until(frames(1))
            .add();
        let script = tap_tab(
            script,
            "to open the interface",
            the_interface_shows(InterfacePaneType::Map),
        );
        let script = click(
            script,
            "the Ship tab",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
        );
        let script = shot(script, "01-ship-pane-chips")
            .step("catalogue keys: assert the chips")
            .on_enter(assert_the_chips)
            .until(frames(1))
            .add();

        // Claim 2: TAB opens; the wheel pages.
        let script = tap_tab(script, "to open the catalogue", catalogue_open());
        let script = shot(script, "02-tab-opens-catalogue")
            .step("catalogue keys: hover the grid")
            .on_enter(hover_named("Gallery Grid"))
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("catalogue keys: wheel down over the grid")
            .on_enter(scroll_lines(-1.0))
            .until(when(|world| gallery(world).selected == page(world)))
            .deadline(SESSION_SECS)
            .add();
        let script = shot(script, "03-wheel-next-page")
            .step("catalogue keys: wheel up over the grid")
            .on_enter(|world: &mut World| {
                hover_named("Gallery Grid")(world);
                scroll_lines(1.0)(world);
            })
            .until(when(|world| gallery(world).selected == 0))
            .deadline(SESSION_SECS)
            .add()
            .step("catalogue keys: assert the wheel")
            .on_enter(assert_the_wheel)
            .until(frames(1))
            .add();

        // Claim 3: narrow, Place, add, TAB back as left, TAB closes.
        let script = click(
            script,
            "the Weapons chip",
            format!("Gallery Category {}", GalleryCategory::Weapons.label()),
            when(|world| gallery(world).category == GalleryCategory::Weapons),
        );
        let script = tap_key(
            script,
            "the filter key",
            KeyCode::Slash,
            when(|world| gallery(world).filter_focused),
        )
        .step("catalogue keys: type the filter")
        .on_enter(type_text(FILTER))
        .until(when(|world| gallery(world).filter == FILTER))
        .deadline(SESSION_SECS)
        .add();
        let script = shot(script, "04-filter-typed");
        let script = escape(
            script,
            "the filter caret before Place",
            (false, false, true, false, true),
        );
        let script = focus_the_pdc(script, "to place it")
            .step("catalogue keys: record the browse")
            .on_enter(record_the_browse)
            .until(frames(1))
            .add();
        let script = click(script, "Place", "Gallery Place Button", pdc_in_hand());
        let script = shot(script, "05-place-in-hand")
            .step("catalogue keys: aim the ghost at a free socket")
            .each(aim_the_ghost)
            .until(when(ghost_valid))
            .deadline(SESSION_SECS)
            .add()
            .step("catalogue keys: settle the ghost")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("catalogue keys: press on the ghost")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step("catalogue keys: release on the ghost")
            .on_enter(release_mouse(MouseButton::Left))
            .until(pdc_added())
            .deadline(SESSION_SECS)
            .add();
        let script = shot(script, "06-added-part-still-in-hand")
            .step("catalogue keys: the part is still in hand")
            .on_enter(|world: &mut World| {
                assert_eq!(
                    held(world),
                    Some(PDC_KINETIC_TURRET_SECTION_ID),
                    "catalogue keys: an add keeps the part in hand"
                );
            })
            .until(frames(1))
            .add();
        let script = tap_tab(script, "with the part in hand", catalogue_open())
            .step("catalogue keys: assert the return")
            .on_enter(assert_the_return)
            .until(frames(1))
            .add();
        let script = shot(script, "07-tab-returns-filter-kept");
        let script = tap_tab(
            script,
            "to close the catalogue",
            catalogue_closed_on_the_ship(),
        )
        .step("catalogue keys: assert the close")
        .on_enter(|world: &mut World| {
            marker(
                world,
                "outcome: tab returns to the catalogue as it was left",
                serde_json::json!({ "filter": FILTER }),
            );
        })
        .until(frames(1))
        .add();
        let script = shot(script, "08-tab-closes-catalogue");

        // Claim 4: Esc one layer per press. Caret, card, catalogue first.
        let script = tap_tab(script, "to open the catalogue for Esc", catalogue_open());
        let script = focus_the_pdc(script, "for Esc");
        let script = tap_key(
            script,
            "the filter key over the card",
            KeyCode::Slash,
            when(|world| gallery(world).filter_focused),
        );
        let script = shot(script, "09-esc-chain-caret-card");
        let script = escape(script, "the caret", (false, true, true, false, true));
        let script = shot(script, "10-esc-left-caret");
        let script = escape(script, "the card", (false, false, true, false, true));
        let script = shot(script, "11-esc-left-card");
        let script = escape(script, "the catalogue", (false, false, false, false, true));
        let script = shot(script, "12-esc-left-catalogue");
        // Then the part in hand, then the interface.
        let script = tap_tab(script, "to take a part", catalogue_open());
        let script = focus_the_pdc(script, "to take it");
        let script = click(script, "Place again", "Gallery Place Button", pdc_in_hand());
        let script = shot(script, "13-esc-chain-part-in-hand");
        let script = escape(
            script,
            "the part in hand",
            (false, false, false, false, true),
        );
        let script = shot(script, "14-esc-left-part");
        let script = escape(script, "the interface", (false, false, false, false, false))
            .step("catalogue keys: assert the Esc chain")
            .on_enter(|world: &mut World| {
                marker(
                    world,
                    "outcome: escape steps back one layer per press",
                    serde_json::json!({ "layers": 5 }),
                );
            })
            .until(frames(1))
            .add();
        let script = shot(script, "15-esc-left-interface");

        // Claim 5: a pane switch closes the catalogue; TAB in Map closes the
        // interface.
        let script = tap_tab(
            script,
            "to reopen the interface",
            the_interface_shows(InterfacePaneType::Ship),
        );
        let script = tap_tab(script, "to open the catalogue again", catalogue_open());
        let script = click(
            script,
            "the Map tab",
            "InterfaceTabMap",
            when(|world| {
                pane(world) == InterfacePaneType::Map
                    && !gallery(world).open
                    && pause(world) == PauseStates::Interface
            }),
        );
        let script = shot(script, "16-map-tab-closed-catalogue");
        tap_tab(script, "in the Map pane", unpaused())
            .step("catalogue keys: assert TAB elsewhere")
            .on_enter(|world: &mut World| {
                assert!(!gallery(world).open, "catalogue keys: no catalogue left up");
                marker(
                    world,
                    "outcome: tab elsewhere closes only the interface",
                    serde_json::json!({}),
                );
                info!("catalogue keys: autopilot walk done");
            })
            .until(frames(1))
            .add()
    }
}
