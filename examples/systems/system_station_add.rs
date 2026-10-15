//! system_station_add: a section is added at a free attachment point of a
//! base hull and of a generated hull at a station, and the add is part of
//! the ship from then on.
//!
//! A free attachment point is a socket no section of the design mates; no
//! content marks it. Two rounds share one run. In the first, the player flies
//! the base `mining_tug` (unarmed) and docks at a living waystation. The
//! walk lists every free socket of a live section, in design order, that a
//! kinetic PDC fits at, by the pane's own rule. An add at a mated socket of
//! the first one's section is refused with its reason and changes nothing.
//! Then the player opens the Ship pane's Catalogue, clicks the kinetic PDC's
//! tile and Place on its card, points at the free sockets in turn until the
//! ghost stands valid at one, and clicks the view. In the second round the scenario is swapped
//! for one where the player flies a generated armored hull whose turrets are
//! bound to the left mouse button, and the add goes through the pane's add
//! command. Each round undocks, freezes and thaws the hull the way a retiring
//! cell does it.
//!
//! THE HOLD AND THE BALANCE ARE DISCLOSED FIXTURE STATE: each hold carries
//! the kinetic PDC recipe's parts (two panels, an actuator, a turret body and
//! a barrel) and 1,000 cr, so the price is the recipe's 100 cr of work.
//!
//! | # | marker | claim |
//! | - | - | - |
//! | 1 | `outcome: an add at an occupied point is refused` | the note names the section of the taken point; the design, the live sections, the holds and the balances are as they were |
//! | 2 | `outcome: a section is added to a base hull` | the ghost callout gave the price; the PDC is live at full health at the pose the aimed point gives, under the next PDC code, with an empty binding Rebind sets; the generator's post-build checks pass on the live ship; the design is inline and names it; the parts and the price moved; `OnAdded` fired once |
//! | 3 | `outcome: a base hull's added section survives a freeze and thaw` | the thawed hull has the PDC whole, with its binding |
//! | 4 | `outcome: a section is added to a generated hull` | as claim 2 on the generated hull, and the PDC flies on the left mouse button its sibling turrets carry |
//! | 5 | `outcome: a generated hull's added section survives a freeze and thaw` | the thawed hull has the PDC whole, on the left mouse button |
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_station_add --features debug
//! # look for: `station add probe: refused`,
//! #           `station add probe: added to the base hull`,
//! #           `station add probe: added to the generated hull`,
//! #           `autopilot: cycle complete, no panic`
//! ```
//! Add `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=<dir>` to shoot the catalogue gallery
//! as `station-add-offer.png` and the ghost as `station-add-ghost.png`.

use bevy::prelude::*;
use clap::Parser;
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;
use nova_world::prelude::{CivilizationId, ShipRoleType};

#[derive(Parser)]
#[command(name = "system_station_add")]
#[command(version = "1.0.0")]
#[command(about = "A PDC is placed at a free socket of a base hull with the Ship pane's ghost and of a generated hull, an add at a taken socket is refused, and each add survives a freeze and thaw. Autopilot-only correctness range", long_about = None)]
struct Cli;

/// The waystation's park, down -Z from the player.
const STATION_POSITION: Meters3 = Meters3::new(0.0, 0.0, -200.0);
/// The station object id.
const STATION_ID: &str = "waystation";
/// The player object id.
const PLAYER_ID: &str = "player_spaceship";
/// The base catalogue hull of the first round.
const BASE_HULL: &str = "mining_tug";
/// The base hull's mining beam, which a player ship must bind, and its key.
const BASE_MINING_SECTION: &str = "mining_beam";
const MINING_KEY: InputSource = InputSource::Mouse(MouseButton::Right);

/// The generated hull of the second round: the world and the hull seed.
const WORLD_SEED: u32 = 20_261_014;
const HULL_SEED: u32 = 3;
/// Advancement steps searched for the lowest one that fields the role.
const ADVANCEMENT_STEPS: u32 = 20;
/// The key the generated hull's turrets fly on.
const TURRET_KEY: InputSource = InputSource::Mouse(MouseButton::Left);

/// The player's balance and the kinetic PDC recipe's parts the hold carries.
const PLAYER_CREDITS: u32 = 1_000;
const HOLD: [(&str, u32); 4] = [
    ("SectionPanel", 2),
    ("ActuatorPack", 1),
    ("TurretBody", 1),
    ("TurretBarrel", 1),
];
/// The kinetic PDC recipe's work; the hold covers its parts.
const PRICE: u32 = 100;

/// The PNG the capture path shoots of the catalogue gallery.
const OFFER_SHOT: &str = "station-add-offer.png";

/// One round of the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Round {
    /// The base catalogue hull, added through the pane's buttons.
    Base,
    /// A generated hull, added through the pane's add command.
    Generated,
}

impl Round {
    fn label(self) -> &'static str {
        match self {
            Round::Base => "base",
            Round::Generated => "generated",
        }
    }
}

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
        // The fleet's 1920x1080, the size the add list is shot at.
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(walk::AddWatchPlugin);
        app.add_plugins(walk::script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_void);
}

fn setup_void(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(station_void(
        Round::Base,
        &game_assets,
        &sections,
    )));
}

/// The round's void: the living waystation and the player's hull, lit by a
/// three-point rig because the engine spawns no light.
fn station_void(round: Round, game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let station = ScenarioObjectConfig {
        base: BaseScenarioObjectConfig {
            id: STATION_ID.to_string(),
            name: "WAYSTATION".to_string(),
            position: STATION_POSITION,
            rotation: Quat::IDENTITY,
        },
        kind: ScenarioObjectKind::Station(StationConfig {
            design: BLOCK_WAYSTATION_STATION_ID.into(),
            allegiance: Allegiance::Neutral,
            derelict: false,
            initial_velocity: MetersPerSecond3::ZERO,
        }),
    };
    let (design, input_mapping): (_, std::collections::BTreeMap<String, Vec<InputSource>>) =
        match round {
            Round::Base => (
                ShipDesignSource::prototype(BASE_HULL),
                [(BASE_MINING_SECTION.to_string(), vec![MINING_KEY])].into(),
            ),
            Round::Generated => {
                let design = generated_hull();
                let turrets = design
                    .sections
                    .iter()
                    .filter(|section| {
                        let class = match &section.source {
                            SectionSource::Inline(config) => config.kind.class(),
                            SectionSource::Prototype { id, .. } => sections
                                .get_section(id)
                                .unwrap_or_else(|| panic!("station add: no catalogue part '{id}'"))
                                .kind
                                .class(),
                        };
                        class == SectionClass::Turret
                    })
                    .map(|section| (section.id.clone(), vec![TURRET_KEY]))
                    .collect();
                (ShipDesignSource::Inline(design), turrets)
            }
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
            controller: SpaceshipController::Player(PlayerControllerConfig { input_mapping }),
            design,
            inventory: ShipInventoryStock::new(HOLD.map(|(item, count)| (item.into(), count))),
            credits: PLAYER_CREDITS,
            ..default()
        }),
    };
    ScenarioConfig {
        description: "A waystation that refits, and a player who wants one more gun.".to_string(),
        events: vec![ScenarioEventConfig {
            label: None,
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
            format!("station_add_{}", round.label()),
            "Station Add Rig".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}

/// A generated armored hull of the base parts, at the lowest advancement
/// that fields the role.
fn generated_hull() -> ShipDesign {
    let ids = ["base"];
    let packs = nova_authoring::lint_walk::repo_ship_part_packs(&ids);
    let snapshot = ShipPartSnapshot::build(&packs)
        .unwrap_or_else(|faults| panic!("station add: the base snapshot is refused: {faults:?}"));
    let role = ShipRoleType::Armored;
    let advancement = (0..=ADVANCEMENT_STEPS)
        .map(|step| step as f32 / ADVANCEMENT_STEPS as f32)
        .find(|advancement| snapshot.eligible_roles(*advancement).contains(&role))
        .expect("station add: the base snapshot fields an armored hull");
    let request = ShipLayoutRequest {
        seed: HULL_SEED,
        civilization: CivilizationId {
            world_seed: WORLD_SEED,
            node: [0, 0, 0],
        },
        role,
        advancement,
    };
    generate_ship(&snapshot, request)
        .unwrap_or_else(|failure| panic!("station add: {failure}"))
        .design
}

#[cfg(feature = "debug")]
mod walk {
    use std::sync::Arc;

    use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
    use nova_protocol::{
        nova_debug::harness::{
            press_action, release_action, shoot, AutopilotPlugin, Predicate, SHOT_DEADLINE_SECS,
        },
        nova_events::engine::GameEvent,
        nova_interface::{
            pane::InterfacePaneType,
            ship::{
                ship_ghost_aim, ship_view_window_px, SectionAddCommand, SectionCode, ShipRuntime,
                SHIP_GHOST_CALLOUT,
            },
        },
    };
    use nova_world::prelude::{
        freeze_body, thaw_record, DockPolicy, FrozenBodyType, PreparedFrozenBody, SectorCoord,
    };

    use super::*;

    /// Seconds a load, a pane or a beat gets on a software-rendered GPU.
    const SESSION_SECS: f32 = 240.0;
    /// Frames the Ship pane gets to build its panel and lists.
    const SETTLE_FRAMES: u32 = 10;
    /// Seconds a tab click gets to switch the pane after an add.
    const TAB_SWITCH_SECS: f32 = 30.0;
    /// The PNG of the Ship pane right after the add.
    const PLACED_SHOT: &str = "station-add-placed.png";
    /// The PNG of the ghost standing valid at the aimed point.
    const GHOST_SHOT: &str = "station-add-ghost.png";
    /// Frames the pointer rests on one free socket before the walk tries the
    /// next one.
    const AIM_FRAMES: u32 = 6;
    /// How far the aim sits from the socket toward its section's centre, as
    /// a share of the way: on the section's face, beside the socket.
    const AIM_INSET: f32 = 0.15;
    /// The kinetic PDC's tile in the catalogue gallery, named by its
    /// catalogue name.
    fn pdc_tile(world: &World) -> String {
        let sections = world.resource::<GameSections>();
        let pdc = sections
            .get_section(PDC_KINETIC_TURRET_SECTION_ID)
            .expect("station add: the catalogue lists the kinetic PDC");
        format!("Gallery Tile {}", pdc.base.name)
    }
    /// The PNG of the Map pane the tab click opens after the add.
    const MAP_AFTER_SHOT: &str = "station-add-map-after.png";

    /// Log the pane tabs and the state that gates a tab click, so a tab that
    /// does not answer says why.
    fn report_the_tabs(world: &mut World) {
        let pane = world.get_resource::<InterfacePaneType>().copied();
        let mut tabs = world.query::<(
            Entity,
            &Name,
            Has<nova_ui::widget::Selected>,
            Has<bevy::ui::InteractionDisabled>,
            Option<&InheritedVisibility>,
        )>();
        let rows: Vec<String> = tabs
            .iter(world)
            .filter(|(_, name, ..)| name.as_str().starts_with("InterfaceTab"))
            .map(|(entity, name, selected, disabled, visible)| {
                format!(
                    "{name} {entity} selected={selected} disabled={disabled} visible={visible:?}"
                )
            })
            .collect();
        info!("station add: tabs after the add, pane {pane:?}: {rows:?}");
    }
    /// Gap between the two port faces on the berth, engine units.
    const BERTH_GAP: f32 = 0.5;
    /// Where a port's face sits in its own frame, as the docking verb reads it.
    const PORT_FACE: Vec3 = Vec3::new(0.0, 0.0, -0.5);

    /// The point a round adds at, picked by the pane's own rule.
    #[derive(Clone, Debug)]
    struct Target {
        /// The design section the PDC mounts on, and its pane code.
        section: String,
        code: String,
        /// The free socket the PDC mates, and a socket of the same section
        /// another section already mates.
        point: String,
        taken: String,
        /// The pose the free socket gives the PDC.
        position: Vec3,
        rotation: Quat,
        /// Where the walk points to aim the ghost at the free socket, in
        /// ship-local space.
        aim: Vec3,
        /// The id and pane code the new section takes.
        id: String,
        new_code: String,
    }

    /// What the run saw and recorded between beats.
    #[derive(Resource, Default, Debug)]
    struct AddWatch {
        /// Every add and balance event, by name, with its payload.
        events: Vec<(String, serde_json::Value)>,
        /// The round's point: the free socket the ghost was aimed at.
        target: Option<Target>,
        /// Every free socket the PDC fits at, in design order, and the one
        /// the pointer is on (`usize::MAX` before the first).
        candidates: Vec<Target>,
        aiming: usize,
        /// The design and live section counts before the add, and the
        /// station's balance.
        before: Option<(usize, usize, u32)>,
        /// The events seen before the round's add.
        events_before: usize,
        /// The player root the freeze despawned, and the thaw root.
        frozen_root: Option<Entity>,
        thaw_root: Option<Entity>,
    }

    impl AddWatch {
        fn named_since(&self, name: &str) -> Vec<&serde_json::Value> {
            self.events[self.events_before..]
                .iter()
                .filter(|(event, _)| event == name)
                .map(|(_, data)| data)
                .collect()
        }
    }

    pub(super) struct AddWatchPlugin;

    impl Plugin for AddWatchPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<AddWatch>();
            app.add_observer(|event: On<GameEvent>, mut watch: ResMut<AddWatch>| {
                let name = event.name();
                if !matches!(name, "onadded" | "oncreditschanged") {
                    return;
                }
                let data = event.info().data.clone().unwrap_or_default();
                info!("station add probe: {name} {data}");
                watch.events.push((name.to_string(), data));
            });
        }
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

    /// The live player root, never the one a freeze despawned.
    fn player_root(world: &World) -> Option<Entity> {
        let frozen = world.resource::<AddWatch>().frozen_root;
        let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
        players.iter(world).find(|player| Some(*player) != frozen)
    }

    fn player(world: &World) -> Entity {
        player_root(world).expect("station add probe: the player is up")
    }

    fn station(world: &World) -> Entity {
        station_root(world).expect("station add probe: the station is up")
    }

    fn target(world: &World) -> Target {
        world
            .resource::<AddWatch>()
            .target
            .clone()
            .expect("station add probe: the round picked its point")
    }

    /// The live section `id` of `root`, or `None` while it has none.
    fn section_of(world: &World, root: Entity, id: &str) -> Option<Entity> {
        world
            .get::<Children>(root)?
            .iter()
            .find(|child| world.get::<EntityId>(*child).is_some_and(|eid| eid.0 == id))
    }

    fn live_ids(world: &World, root: Entity) -> Vec<String> {
        world
            .get::<Children>(root)
            .into_iter()
            .flatten()
            .filter(|child| world.get::<SectionMarker>(**child).is_some())
            .filter_map(|child| world.get::<EntityId>(*child).map(|id| id.0.clone()))
            .collect()
    }

    fn held(world: &World, root: Entity, item: &str) -> u32 {
        world
            .get::<ShipInventory>(root)
            .expect("station add probe: every ship carries a hold")
            .count(&item.into())
    }

    fn credits(world: &World, root: Entity) -> u32 {
        world
            .get::<ShipCredits>(root)
            .expect("station add probe: every ship carries a balance")
            .0
    }

    fn design(world: &World, root: Entity) -> &ShipDesignSections {
        world
            .get::<ShipDesignSections>(root)
            .expect("station add probe: the player carries its design sections")
    }

    fn turret_key(world: &World, section: Entity) -> Option<Vec<InputSource>> {
        world
            .get::<SpaceshipTurretInputBinding>(section)
            .map(|binding| binding.0.clone())
    }

    fn full_health(world: &World, section: Entity) -> bool {
        world
            .get::<Health>(section)
            .is_some_and(|health| health.max > 0.0 && health.current == health.max)
    }

    /// The binding the round's new PDC must fly on.
    fn expected_key(round: Round) -> Vec<InputSource> {
        match round {
            Round::Base => Vec::new(),
            Round::Generated => vec![TURRET_KEY],
        }
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

    fn unpaused() -> Arc<Predicate> {
        when(|world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == PauseStates::Unpaused)
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
            .step(format!("station add: press {label}"))
            .on_enter(press_action(action))
            .until(frames(1))
            .add()
            .step(format!("station add: let {label} up"))
            .on_enter(release_action(action))
            .until(landed)
            .deadline(SESSION_SECS)
            .add()
    }

    /// `section`'s position and rotation in `root`'s frame, by its mount chain.
    fn local_pose(world: &World, section: Entity, root: Entity) -> (Vec3, Quat) {
        let (mut position, mut rotation, mut at) = (Vec3::ZERO, Quat::IDENTITY, section);
        while at != root {
            let transform = world
                .get::<Transform>(at)
                .expect("station add probe: a mounted section has a transform");
            position = transform.translation + transform.rotation * position;
            rotation = transform.rotation * rotation;
            at = world
                .get::<ChildOf>(at)
                .expect("station add probe: a section's chain reaches its root")
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
            .unwrap_or_else(|| panic!("station add probe: {root:?} has no free docking port"))
    }

    /// Put the player on the berth at the station's collar, its port facing
    /// the station's across [`BERTH_GAP`], then ask to dock. Every frame until
    /// the dock takes, so the attitude hold cannot carry it off the berth.
    fn berth_and_dock(world: &mut World) {
        let player = player(world);
        if world.get::<DockedShip>(player).is_some() {
            return;
        }
        let station = station(world);
        let station_port = free_port(world, station);
        let player_port = free_port(world, player);
        let station_at = world
            .get::<Position>(station)
            .expect("a hull has a position")
            .0;
        let station_rot = world
            .get::<Rotation>(station)
            .expect("a hull has a rotation")
            .0;
        let (port_at, port_rot) = local_pose(world, station_port, station);
        let station_face_rot = station_rot * port_rot;
        let station_face = station_at + station_rot * port_at + station_face_rot * PORT_FACE;
        let station_axis = station_face_rot * Vec3::NEG_Z;
        let (own_at, own_rot) = local_pose(world, player_port, player);
        let rotation = Quat::from_rotation_arc((own_rot * Vec3::NEG_Z).normalize(), -station_axis);
        let face_offset = rotation * own_at + rotation * own_rot * PORT_FACE;
        let position = station_face + station_axis * BERTH_GAP - face_offset;
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

    fn player_docked() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
        })
    }

    fn player_free() -> Arc<Predicate> {
        when(|world| {
            player_root(world).is_some_and(|player| world.get::<DockedShip>(player).is_none())
        })
    }

    /// The round's hull is up, coded and at rest, and the station stands.
    fn round_loaded() -> Arc<Predicate> {
        when(|world| {
            station_root(world).is_some_and(|station| world.get::<Position>(station).is_some())
                && player_root(world).is_some_and(|player| {
                    world.get::<Position>(player).is_some()
                        && world.get::<ShipDesignSections>(player).is_some()
                        && world.get::<Children>(player).is_some_and(|children| {
                            children
                                .iter()
                                .any(|child| world.get::<SectionCode>(child).is_some())
                        })
                })
        })
    }

    /// List the round's points by the pane's rule: every free socket of a
    /// live section, in design order, the kinetic PDC fits at. The first one
    /// is the round's point until the ghost is aimed. Record the counts and
    /// balances before the add, and the events so far.
    fn pick_the_point(world: &mut World) {
        let player = player(world);
        let station = station(world);
        let live = live_ids(world, player);
        let design = design(world, player);
        let offered = world
            .resource::<GameSections>()
            .get_section(PDC_KINETIC_TURRET_SECTION_ID)
            .expect("station add probe: the catalogue has the kinetic PDC");
        let placed: Vec<RefitPlacedSection<'_>> = design
            .0
            .iter()
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            })
            .collect();
        let live_indices: Vec<usize> = design
            .0
            .iter()
            .enumerate()
            .filter(|(_, section)| live.contains(&section.id))
            .map(|(index, _)| index)
            .collect();
        let section_of_class = design
            .0
            .iter()
            .filter(|other| other.config.kind.class() == SectionClass::Turret)
            .count();
        let mut candidates = Vec::new();
        for &index in &live_indices {
            let section = &design.0[index];
            let free = free_link_points(&placed, index);
            let Some(taken) =
                (0..section.config.base.link_points.len()).find(|socket| !free.contains(socket))
            else {
                continue;
            };
            let code = section_of(world, player, &section.id)
                .and_then(|entity| world.get::<SectionCode>(entity))
                .expect("station add probe: a live section is coded")
                .0
                .clone();
            for &point in &free {
                let Ok((position, rotation)) = section_add_fit(
                    &placed,
                    &live_indices,
                    index,
                    point,
                    offered,
                    MountPose::default(),
                    &[],
                ) else {
                    continue;
                };
                let (socket, _) = link_point_pose(
                    section.position,
                    section.rotation,
                    &section.config.base.link_points[point],
                );
                candidates.push(Target {
                    section: section.id.clone(),
                    code: code.clone(),
                    point: section.config.base.link_points[point].id.clone(),
                    taken: section.config.base.link_points[taken].id.clone(),
                    position,
                    rotation,
                    aim: socket.lerp(section.position, AIM_INSET),
                    id: added_section_id(design, SectionClass::Turret),
                    new_code: format!("PDC-{}", section_of_class + 1),
                });
            }
        }
        assert!(
            !candidates.is_empty(),
            "station add probe: the kinetic PDC fits at a free socket of the hull"
        );
        let before = (design.0.len(), live.len(), credits(world, station));
        info!(
            "station add probe: {} free sockets fit, first {:?}",
            candidates.len(),
            candidates[0]
        );
        let mut watch = world.resource_mut::<AddWatch>();
        watch.events_before = watch.events.len();
        watch.target = Some(candidates[0].clone());
        watch.candidates = candidates;
        watch.aiming = usize::MAX;
        watch.before = Some(before);
    }

    /// Ask the pane to add the kinetic PDC at a socket another section mates.
    fn ask_at_the_taken_point(world: &mut World) {
        let target = target(world);
        world.write_message(SectionAddCommand {
            section: target.section,
            point: target.taken,
            prototype: PDC_KINETIC_TURRET_SECTION_ID.to_string(),
            mount: MountPose::default(),
        });
    }

    fn the_note_reads_add() -> Arc<Predicate> {
        when(|world| {
            world
                .resource::<ShipRuntime>()
                .note()
                .is_some_and(|note| note.starts_with("add: "))
        })
    }

    /// The design, the live sections, the holds and the balances are as
    /// they were before the round's add.
    fn assert_nothing_changed(world: &World, what: &str) {
        let player = player(world);
        let station = station(world);
        let (design_len, live_len, station_cr) = world
            .resource::<AddWatch>()
            .before
            .expect("station add probe: the round recorded its counts");
        assert_eq!(
            design(world, player).0.len(),
            design_len,
            "{what}: the design"
        );
        assert_eq!(
            live_ids(world, player).len(),
            live_len,
            "{what}: the live sections"
        );
        assert_eq!(
            credits(world, player),
            PLAYER_CREDITS,
            "{what}: the balance"
        );
        assert_eq!(credits(world, station), station_cr, "{what}: the station");
        for (item, count) in HOLD {
            assert_eq!(
                held(world, player, item),
                count,
                "{what}: the hold's {item}"
            );
        }
    }

    /// Claim 1: the refusal names the taken point and changes nothing.
    fn assert_the_refusal(world: &mut World) {
        let target = target(world);
        let note = world
            .resource::<ShipRuntime>()
            .note()
            .expect("station add probe: the refusal shows a note")
            .to_string();
        assert_eq!(
            note,
            format!("add: that point on {} is taken", target.code),
            "station add probe: the refusal must name the taken point's section"
        );
        assert_nothing_changed(world, "a refused add");
        assert!(
            matches!(
                world
                    .get::<SpaceshipDesign>(player(world))
                    .map(|design| &design.0),
                Some(ShipDesignSource::Prototype { .. })
            ),
            "station add probe: a refused add leaves the catalogue design"
        );
        info!("station add probe: refused: {note}");
        let elapsed = world.resource::<Time>().elapsed_secs();
        nova_probe::probe_marker(
            world,
            "outcome: an add at an occupied point is refused",
            serde_json::json!({ "t": elapsed, "note": note }),
        );
    }

    fn pdc_in_hand() -> Arc<Predicate> {
        when(|world| world.resource::<HeldPart>().id() == Some(PDC_KINETIC_TURRET_SECTION_ID))
    }

    /// Point at the round's free sockets in turn, `AIM_FRAMES` frames each,
    /// until the ghost stands valid at the one pointed at. The one pointed at
    /// becomes the round's point.
    fn aim_the_ghost(world: &mut World, _elapsed: f32, frame: u32) {
        if ghost_on_target(world) {
            return;
        }
        if frame % AIM_FRAMES != 0 {
            return;
        }
        // `usize::MAX` until the first socket is pointed at, so the first
        // aim that lands is the first socket.
        let aiming = world.resource::<AddWatch>().aiming.wrapping_add(1);
        let candidates = world.resource::<AddWatch>().candidates.clone();
        let candidate = candidates.get(aiming).cloned().unwrap_or_else(|| {
            panic!(
                "station add probe: the ghost stood valid at none of {} free sockets, last aim {:?}",
                candidates.len(),
                ship_ghost_aim(world)
            )
        });
        let Some(pixel) = ship_view_window_px(world, candidate.aim) else {
            return;
        };
        info!(
            "station add probe: aiming at {} {} through {pixel:?}, the last aim stood at {:?}",
            candidate.code,
            candidate.point,
            ship_ghost_aim(world)
        );
        move_cursor(pixel)(world);
        let mut watch = world.resource_mut::<AddWatch>();
        watch.aiming = aiming;
        watch.target = Some(candidate);
    }

    /// The ghost stands valid at the round's point.
    fn ghost_on_target(world: &World) -> bool {
        let Some(target) = world.resource::<AddWatch>().target.clone() else {
            return false;
        };
        ship_ghost_aim(world) == Some((target.section, target.point, true))
    }

    /// The callout beside the valid ghost names the part, the section and
    /// the recipe's price.
    fn assert_the_ghost(world: &mut World) {
        let target = target(world);
        let name = world
            .resource::<GameSections>()
            .get_section(PDC_KINETIC_TURRET_SECTION_ID)
            .expect("station add probe: the catalogue has the kinetic PDC")
            .base
            .name
            .clone();
        let mut callouts = world.query::<(&Name, &Children, &InheritedVisibility)>();
        let (children, visible) = callouts
            .iter(world)
            .find(|(found, ..)| found.as_str() == SHIP_GHOST_CALLOUT)
            .map(|(_, children, visible)| (children.to_vec(), visible.get()))
            .expect("station add probe: the ghost callout is up");
        assert!(visible, "station add probe: the ghost callout shows");
        let text = children
            .iter()
            .find_map(|child| world.get::<Text>(*child))
            .map(|text| text.0.clone())
            .expect("station add probe: the ghost callout has a line");
        let held: u32 = HOLD.iter().map(|&(_, count)| count).sum();
        assert_eq!(
            text,
            format!(
                "{name} on {}, {PRICE} cr: {held} parts from the hold, 0 bought",
                target.code
            ),
            "station add probe: the ghost callout gives the price"
        );
        info!("station add probe: ghost callout: {text}");
    }

    /// Add through the pane's command, as the Add button raises it.
    fn ask_for_the_add(world: &mut World) {
        let target = target(world);
        world.write_message(SectionAddCommand {
            section: target.section,
            point: target.point,
            prototype: PDC_KINETIC_TURRET_SECTION_ID.to_string(),
            mount: MountPose::default(),
        });
    }

    fn pdc_added() -> Arc<Predicate> {
        when(|world| {
            let id = target(world).id;
            player_root(world).is_some_and(|player| {
                section_of(world, player, &id)
                    .is_some_and(|pdc| world.get::<SectionCode>(pdc).is_some())
            })
        })
    }

    fn add_reported() -> Arc<Predicate> {
        when(|world| {
            let watch = world.resource::<AddWatch>();
            !watch.named_since("onadded").is_empty()
                && watch.named_since("oncreditschanged").len() >= 2
        })
    }

    /// Claims 2 and 4: the new PDC, its pose, code and binding, the build
    /// checks, the inline design, the moved parts and credits, and the one
    /// `OnAdded`. Then undock.
    fn assert_the_add(world: &mut World, round: Round) {
        let player = player(world);
        let station = station(world);
        let target = target(world);
        let pdc = section_of(world, player, &target.id)
            .expect("station add probe: the PDC is live after the add");
        assert!(
            full_health(world, pdc),
            "station add probe: the PDC must spawn whole: {:?}",
            world.get::<Health>(pdc)
        );
        assert_eq!(
            turret_key(world, pdc),
            Some(expected_key(round)),
            "station add probe: the {} hull's PDC binding",
            round.label()
        );
        assert_eq!(
            world.get::<SectionCode>(pdc).map(|code| code.0.as_str()),
            Some(target.new_code.as_str()),
            "station add probe: the PDC takes the next PDC code"
        );
        let design = design(world, player);
        let added = design
            .get(&target.id)
            .expect("station add probe: the design carries the PDC");
        assert_eq!(added.config.base.id, PDC_KINETIC_TURRET_SECTION_ID);
        assert!(
            added.position.abs_diff_eq(target.position, 1e-4)
                && added.rotation.abs_diff_eq(target.rotation, 1e-4),
            "station add probe: the PDC stands where the chosen point puts it: {:?} {:?}",
            added.position,
            added.rotation
        );

        // The generator's post-build checks, on the ship as it lives now.
        let live = live_ids(world, player);
        let before: Vec<RefitPlacedSection<'_>> = design
            .0
            .iter()
            .filter(|section| section.id != target.id && live.contains(&section.id))
            .map(|section| RefitPlacedSection {
                position: section.position,
                rotation: section.rotation,
                config: &section.config,
            })
            .collect();
        let fault = add_build_fault(
            &before,
            RefitPlacedSection {
                position: added.position,
                rotation: added.rotation,
                config: &added.config,
            },
        );
        assert_eq!(fault, None, "station add probe: the post-build checks pass");

        let ShipDesignSource::Inline(owned) = &world
            .get::<SpaceshipDesign>(player)
            .expect("station add probe: the player carries its design")
            .0
        else {
            panic!("station add probe: an add owns the design inline");
        };
        let authored = owned
            .sections
            .iter()
            .find(|section| section.id == target.id)
            .expect("station add probe: the owned design names the PDC");
        assert!(
            matches!(&authored.source, SectionSource::Prototype { id, .. } if id == PDC_KINETIC_TURRET_SECTION_ID),
            "station add probe: the owned design names the kinetic PDC: {:?}",
            authored.source
        );

        for (item, _) in HOLD {
            assert_eq!(held(world, player, item), 0, "the hold's {item} was spent");
        }
        assert_eq!(credits(world, player), PLAYER_CREDITS - PRICE);
        let (_, _, station_cr) = world
            .resource::<AddWatch>()
            .before
            .expect("station add probe: the round recorded its counts");
        assert_eq!(credits(world, station), station_cr + PRICE);

        let watch = world.resource::<AddWatch>();
        let added_events = watch.named_since("onadded");
        assert_eq!(added_events.len(), 1, "station add probe: one OnAdded");
        let event = added_events[0].clone();
        assert_eq!(event["id"], PLAYER_ID);
        assert_eq!(event["section"], target.new_code);
        assert_eq!(event["part"], PDC_KINETIC_TURRET_SECTION_ID);
        assert_eq!(event["at"], target.code);
        assert_eq!(event["point"], target.point);
        assert_eq!(event["price_cr"], PRICE);
        assert_eq!(
            event["parts"],
            serde_json::json!({ "ActuatorPack": 1, "SectionPanel": 2, "TurretBarrel": 1, "TurretBody": 1 })
        );
        let causes: Vec<String> = watch
            .named_since("oncreditschanged")
            .iter()
            .map(|data| data["cause"].as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(causes, ["Add", "Add"], "one Add balance change per party");

        let elapsed = world.resource::<Time>().elapsed_secs();
        info!(
            "station add probe: added to the {} hull: {} at {} {} for {PRICE} cr",
            round.label(),
            target.new_code,
            target.code,
            target.point
        );
        nova_probe::probe_marker(
            world,
            match round {
                Round::Base => "outcome: a section is added to a base hull",
                Round::Generated => "outcome: a section is added to a generated hull",
            },
            serde_json::json!({ "t": elapsed, "price": PRICE, "event": event }),
        );
        world.trigger(DockingReleaseRequest { entity: player });
    }

    /// Freeze the undocked player as a retiring cell would, check the record
    /// carries the PDC, despawn the live hull and thaw the record.
    fn freeze_and_thaw_the_player(world: &mut World) {
        let player = player(world);
        let id = target(world).id;
        world
            .entity_mut(player)
            .insert((LinearVelocity(Vec3::ZERO), AngularVelocity(Vec3::ZERO)));
        let frozen = freeze_body(world, player, DockPolicy::Refuse)
            .expect("station add probe: an undocked hull at rest is settled");
        let FrozenBodyType::Ship(record) = frozen.body() else {
            panic!("station add probe: the player froze as {:?}", frozen.body());
        };
        assert!(
            record.has_section(&id),
            "station add probe: the frozen hull must carry the PDC"
        );
        world.entity_mut(player).despawn();
        let root = world
            .spawn((
                Name::new("thaw root"),
                Transform::IDENTITY,
                Visibility::default(),
            ))
            .id();
        let prepared = PreparedFrozenBody::prepare(SectorCoord::new(0, 0, 0), frozen, &[], &[]);
        let mut commands = world.commands();
        thaw_record(&mut commands, root, vec![prepared]);
        world.flush();
        let mut watch = world.resource_mut::<AddWatch>();
        watch.frozen_root = Some(player);
        watch.thaw_root = Some(root);
    }

    fn thawed_pdc_built() -> Arc<Predicate> {
        when(|world| {
            let id = target(world).id;
            player_root(world).is_some_and(|player| {
                section_of(world, player, &id).is_some_and(|pdc| turret_key(world, pdc).is_some())
            })
        })
    }

    /// Claims 3 and 5: the thawed hull has the PDC whole, with its binding.
    fn assert_the_thaw(world: &mut World, round: Round) {
        let player = player(world);
        let id = target(world).id;
        let pdc =
            section_of(world, player, &id).expect("station add probe: the thawed hull has the PDC");
        assert_eq!(
            design(world, player)
                .get(&id)
                .map(|section| section.config.base.id.as_str()),
            Some(PDC_KINETIC_TURRET_SECTION_ID),
            "station add probe: the thawed design carries the kinetic PDC"
        );
        assert!(
            full_health(world, pdc),
            "station add probe: the thawed PDC must be whole: {:?}",
            world.get::<Health>(pdc)
        );
        assert_eq!(
            turret_key(world, pdc),
            Some(expected_key(round)),
            "station add probe: the thawed PDC keeps its binding"
        );
        let elapsed = world.resource::<Time>().elapsed_secs();
        info!(
            "station add probe: the {} hull's PDC survives the freeze and thaw",
            round.label()
        );
        nova_probe::probe_marker(
            world,
            match round {
                Round::Base => "outcome: a base hull's added section survives a freeze and thaw",
                Round::Generated => {
                    "outcome: a generated hull's added section survives a freeze and thaw"
                }
            },
            serde_json::json!({ "t": elapsed }),
        );
    }

    /// Clear the first round's thawed hull and load the generated round.
    fn load_the_generated_round(world: &mut World) {
        if let Some(root) = world.resource_mut::<AddWatch>().thaw_root.take() {
            world.entity_mut(root).despawn();
        }
        world.resource_mut::<AddWatch>().frozen_root = None;
        let config = station_void(
            Round::Generated,
            world.resource::<GameAssets>(),
            world.resource::<GameSections>(),
        );
        world.trigger(LoadScenario(config));
    }

    /// Dock and pick the point.
    fn dock_and_pick(
        script: AutopilotPlugin<GameStates>,
        round: Round,
    ) -> AutopilotPlugin<GameStates> {
        let label = round.label();
        script
            .step(format!("station add: the {label} hull is up"))
            .until(round_loaded())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station add: dock the {label} hull"))
            .each(|world, _, _| berth_and_dock(world))
            .until(player_docked())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station add: pick the {label} hull's point"))
            .on_enter(pick_the_point)
            .until(frames(1))
            .add()
    }

    /// Wait for the reports, assert the add, undock, freeze, thaw and assert.
    fn finish(script: AutopilotPlugin<GameStates>, round: Round) -> AutopilotPlugin<GameStates> {
        let label = round.label();
        script
            .step(format!("station add: the {label} add reports"))
            .until(add_reported())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station add: assert the {label} add and undock"))
            .on_enter(move |world: &mut World| assert_the_add(world, round))
            .until(player_free())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station add: freeze and thaw the {label} hull"))
            .on_enter(freeze_and_thaw_the_player)
            .until(thawed_pdc_built())
            .deadline(SESSION_SECS)
            .add()
            .step(format!("station add: assert the {label} thaw"))
            .on_enter(move |world: &mut World| assert_the_thaw(world, round))
            .until(frames(1))
            .add()
    }

    /// The walk. Every beat waits on a world value, so a slow load delays it
    /// instead of truncating it.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new()
            .step("station add: load the void")
            .enter(GameStates::Loading)
            .until(round_loaded())
            .deadline(SESSION_SECS)
            .add();
        let script = dock_and_pick(script, Round::Base);
        let script = tap(
            script,
            "the interface key",
            "interface_toggle",
            the_interface_shows(InterfacePaneType::Map),
        )
        .click_named(
            "station add: open the Ship pane",
            "InterfaceTabShip",
            the_interface_shows(InterfacePaneType::Ship),
            SESSION_SECS,
        )
        .step("station add: settle the schematic")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("station add: ask at the taken point")
        .on_enter(ask_at_the_taken_point)
        .until(the_note_reads_add())
        .deadline(SESSION_SECS)
        .add()
        .step("station add: assert the refusal")
        .on_enter(assert_the_refusal)
        .until(frames(1))
        .add()
        .click_named(
            "station add: open the catalogue",
            "ShipCatalogue",
            ui_node_present("Gallery Grid"),
            SESSION_SECS,
        )
        .step("station add: settle the catalogue")
        .until(frames(SETTLE_FRAMES))
        .add();
        let script = script
            .step("station add: shoot the catalogue")
            .on_enter(|world: &mut World| shoot(world, OFFER_SHOT))
            .until(shot_written(OFFER_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("station add: press the kinetic PDC's tile")
            .on_enter(|world: &mut World| click_named(pdc_tile(world))(world))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step("station add: release the kinetic PDC's tile")
            .on_enter(release_mouse(MouseButton::Left))
            .until(ui_node_present("Gallery Place Button"))
            .deadline(SESSION_SECS)
            .add()
            .click_named(
                "station add: take the kinetic PDC in hand",
                "Gallery Place Button",
                pdc_in_hand(),
                SESSION_SECS,
            )
            .step("station add: aim the ghost at a free socket")
            .each(aim_the_ghost)
            .until(when(ghost_on_target))
            .deadline(SESSION_SECS)
            .add()
            .step("station add: settle the ghost")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station add: assert the ghost")
            .on_enter(assert_the_ghost)
            .until(frames(1))
            .add()
            .step("station add: shoot the ghost")
            .on_enter(|world: &mut World| shoot(world, GHOST_SHOT))
            .until(shot_written(GHOST_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .step("station add: press on the ghost")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(SESSION_SECS)
            .add()
            .step("station add: release on the ghost")
            .on_enter(release_mouse(MouseButton::Left))
            .until(pdc_added())
            .deadline(SESSION_SECS)
            .add()
            .step("station add: settle the placed section")
            .until(frames(SETTLE_FRAMES))
            .add()
            .step("station add: report the tabs after the add")
            .on_enter(report_the_tabs)
            .until(frames(1))
            .add()
            .step("station add: shoot the placed section")
            .on_enter(|world: &mut World| shoot(world, PLACED_SHOT))
            .until(shot_written(PLACED_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add()
            .click_named(
                "station add: open the Map pane after the add",
                "InterfaceTabMap",
                the_interface_shows(InterfacePaneType::Map),
                TAB_SWITCH_SECS,
            )
            .step("station add: shoot the Map pane after the add")
            .on_enter(|world: &mut World| shoot(world, MAP_AFTER_SHOT))
            .until(shot_written(MAP_AFTER_SHOT))
            .deadline(SHOT_DEADLINE_SECS)
            .add();
        // The pane pushed its reports under the interface pause; the tracker
        // drains them on the first fixed tick after the clock runs again.
        let script = tap(script, "the interface key", "interface_toggle", unpaused());
        let script = finish(script, Round::Base)
            .step("station add: load the generated round")
            .on_enter(load_the_generated_round)
            .until(frames(1))
            .add();
        let script = dock_and_pick(script, Round::Generated)
            .step("station add: add to the generated hull")
            .on_enter(ask_for_the_add)
            .until(pdc_added())
            .deadline(SESSION_SECS)
            .add();
        finish(script, Round::Generated)
    }
}
