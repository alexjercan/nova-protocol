//! A docked pair flown on real catalog ships: the helm, the autopilot and the
//! joint together, which no single crate's rig has.
//!
//! The partner is a second line warship (1:1) or a test-only station: a solid
//! 16x10x24 box of real hull cells with one real docking port, 20.7 times the
//! warship's mass. The station exists only in this file. Every assertion is
//! on motion - overshoot, reversals, the mark, joint drift - never on time.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::{
    prelude::*,
    test_support::{settle, unfinished_integrity_physics_app},
};
use nova_ship::{flight::prelude::*, prelude::*};
use serde::Deserialize;

use crate::prelude::*;

const WARSHIP: &str = "block_line_warship";
const STATION: &str = "test_only_station";
const STATION_CELLS: IVec3 = IVec3::new(16, 10, 24);
const PLAYER_COLLAR: &str = "starboard_collar";
const PARTNER_COLLAR: &str = "port_collar";
/// Face-to-face gap at capture, cells: inside the one-cell capture distance.
const GAP: f32 = 0.5;
/// The fixed updates a leg may take before the test calls it lost.
const TICK_CAP: usize = 60 * 240;
/// How far the mark stands ahead of the pair's centre of mass, and how far
/// short of it the pair's face parks, engine units.
const MARK: f32 = 100.0;
const STANDOFF: f32 = 5.0;

#[derive(Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "each catalog entry is parsed once and moved out"
)]
enum Entry {
    Section(SectionConfig),
    Ship(ShipDesignPrototype),
}

fn load(path: &str) -> Vec<Entry> {
    let file = format!("{}/../../assets/base/{path}", env!("CARGO_MANIFEST_DIR"));
    ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
}

/// The shipped catalogs, plus the test-only station.
fn catalogs() -> (GameSections, GameShipDesigns) {
    let sections = load("sections/base.content.ron")
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Section(section) => Some(section),
            Entry::Ship(_) => None,
        })
        .collect();
    let mut ships: Vec<ShipDesignPrototype> = load("ships/base.content.ron")
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Ship(ship) => Some(ship),
            Entry::Section(_) => None,
        })
        .collect();
    ships.push(station_design());
    (GameSections(sections), GameShipDesigns(ships))
}

/// A solid box of real hull cells with its port collar on `-X`, turned like
/// the catalog's port flank collars. Dynamic, with no computer and no drive.
fn station_design() -> ShipDesignPrototype {
    let mut sections = Vec::new();
    for x in 0..STATION_CELLS.x {
        for y in 0..STATION_CELLS.y {
            for z in 0..STATION_CELLS.z {
                let cell = IVec3::new(x, y - STATION_CELLS.y / 2, z - STATION_CELLS.z / 2);
                sections.push(SpaceshipSectionConfig {
                    id: format!("hull_{}_{}_{}", cell.x, cell.y, cell.z),
                    position: cell.as_vec3(),
                    rotation: Quat::IDENTITY,
                    source: SectionSource::prototype(REINFORCED_HULL_SECTION_ID),
                });
            }
        }
    }
    sections.push(SpaceshipSectionConfig {
        id: PARTNER_COLLAR.to_string(),
        position: Vec3::new(-1.0, 0.0, 0.0),
        rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        source: SectionSource::prototype(DOCKING_PORT_SECTION_ID),
    });
    ShipDesignPrototype {
        id: STATION.into(),
        name: "Test-only station".to_string(),
        design: ShipDesign {
            sections,
            ..default()
        },
    }
}

/// The attitude a test asks the player's helm for.
#[derive(Component, Clone, Copy)]
struct HelmTarget(Quat);

/// The player's attitude command, slewed at the turn rate the hull publishes,
/// as the mouse helm slews it.
fn slew_helm(
    time: Res<Time>,
    q_ship: Query<(Entity, &HelmTarget, &FlightAuthority)>,
    mut q_command: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
) {
    for (ship, target, authority) in &q_ship {
        let step = authority.turn_rate * time.delta_secs();
        for (mut command, &ChildOf(parent)) in &mut q_command {
            if parent == ship {
                **command = command.rotate_towards(target.0, step);
            }
        }
    }
}

fn app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        PDControllerPlugin,
        SpaceshipSectionPlugin { render: false },
        SpaceshipPlugin,
        NovaFlightPlugin,
    ));
    let (sections, ships) = catalogs();
    app.insert_resource(sections);
    app.insert_resource(ships);
    app.add_systems(
        FixedUpdate,
        slew_helm
            .after(NovaFlightSystems)
            .before(ControllerSectionSystems::SyncRotationInput),
    );
    app.finish();
    app
}

fn collar(app: &App, design: &str, id: &str) -> Vec3 {
    let (design, errors) = resolve_ship_design(
        &ShipDesignSource::prototype(design),
        app.world().resource::<GameShipDesigns>(),
        app.world().resource::<GameSections>(),
    );
    assert!(errors.is_empty(), "{errors:?}");
    design
        .sections
        .iter()
        .find(|section| section.id == id)
        .unwrap_or_else(|| panic!("no '{id}'"))
        .position
}

fn spawn_ship(app: &mut App, design: &str, at: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_translation(at),
            spaceship_scenario_object(SpaceshipConfig {
                design: ShipDesignSource::prototype(design),
                controller: SpaceshipController::None,
                allegiance: None,
                capabilities: default(),
            }),
        ))
        .id()
}

fn dock(app: &mut App, player: Entity, partner: Entity) {
    app.world_mut().trigger(DockingConnectionRequest {
        entity: player,
        target: partner,
    });
    app.update();
    assert!(
        app.world().get::<DockedShip>(player).is_some(),
        "the real dock was refused"
    );
}

/// A player line warship docked, at rest, to `partner` off its starboard
/// collar.
fn docked_pair(partner: &str) -> (App, Entity, Entity) {
    let mut app = app();
    let offset = collar(&app, WARSHIP, PLAYER_COLLAR) + Vec3::X * (1.0 + GAP)
        - collar(&app, partner, PARTNER_COLLAR);
    let player = spawn_ship(&mut app, WARSHIP, Vec3::ZERO);
    let partner = spawn_ship(&mut app, partner, offset);
    settle(&mut app);
    settle(&mut app);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSpaceshipMarker);
    dock(&mut app, player, partner);
    settle(&mut app);
    (app, player, partner)
}

fn take_helm(app: &mut App, player: Entity) {
    app.world_mut()
        .trigger(DockingHelmRequest { entity: player });
    app.update();
    assert!(app.world().get::<DockedShip>(player).unwrap().drives);
}

/// The partner's pose in the player's frame: what the joint holds fixed.
fn relative(app: &App, player: Entity, partner: Entity) -> (Vec3, Quat) {
    let pose = |ship: Entity| {
        (
            app.world().get::<Position>(ship).unwrap().0,
            app.world().get::<Rotation>(ship).unwrap().0,
        )
    };
    let ((p0, r0), (p1, r1)) = (pose(player), pose(partner));
    (r0.inverse() * (p1 - p0), r0.inverse() * r1)
}

/// How far the joint has let the pair move against each other since `start`,
/// in engine units and degrees.
fn joint_drift(app: &App, player: Entity, partner: Entity, start: (Vec3, Quat)) -> (f32, f32) {
    let (offset, turn) = relative(app, player, partner);
    (
        (offset - start.0).length(),
        turn.angle_between(start.1).to_degrees(),
    )
}

#[test]
fn a_held_warship_pair_turns_like_one_body() {
    let (mut app, player, partner) = docked_pair(WARSHIP);
    take_helm(&mut app, player);
    let axis = Vec3::Y;
    let start = app.world().get::<Rotation>(player).unwrap().0;
    let target = Quat::from_axis_angle(axis, std::f32::consts::FRAC_PI_2) * start;
    let pose = relative(&app, player, partner);
    app.world_mut()
        .entity_mut(player)
        .insert(HelmTarget(target));

    let (mut overshoot, mut reversals, mut drift) = (0.0f32, 0, (0.0f32, 0.0f32));
    let mut last_rate = 0.0f32;
    for _ in 0..60 * 40 {
        app.update();
        let rotation = app.world().get::<Rotation>(player).unwrap().0;
        // Signed turn past the target about the commanded axis.
        let mut error = target.inverse() * rotation;
        if error.w < 0.0 {
            error = -error;
        }
        overshoot = overshoot.max(error.to_scaled_axis().dot(axis).to_degrees());
        let rate = app
            .world()
            .get::<AngularVelocity>(player)
            .unwrap()
            .0
            .dot(axis);
        if rate.abs() > 1e-3 {
            if last_rate * rate < 0.0 {
                reversals += 1;
            }
            last_rate = rate;
        }
        let (metres, degrees) = joint_drift(&app, player, partner, pose);
        drift = (drift.0.max(metres), drift.1.max(degrees));
    }

    let rotation = app.world().get::<Rotation>(player).unwrap().0;
    assert!(
        rotation.angle_between(target).to_degrees() < 0.5,
        "the pair reaches the command"
    );
    assert!(overshoot < 1.0, "overshoot {overshoot} deg");
    assert_eq!(reversals, 0, "the pair turns one way and stops");
    assert!(
        drift.0 < 0.01 && drift.1 < 0.1,
        "the joint holds: {drift:?} (units, deg)"
    );
}

/// Fly `driver` to a mark [`MARK`] ahead of the pair and judge the leg: the
/// pair comes to rest where its face is [`STANDOFF`] off the mark, which is
/// `rest` along the way, and must not brake late past it.
fn fly_to_mark(app: &mut App, driver: Entity, player: Entity, partner: Entity) {
    app.world_mut()
        .entity_mut(driver)
        .insert(FlightArrivalStandoff(STANDOFF));
    let assembly = *app.world().get::<DockedAssembly>(driver).unwrap();
    let rest = MARK - assembly.reach - STANDOFF;
    let ahead = app.world().get::<Rotation>(driver).unwrap().0 * Vec3::NEG_Z;
    let mark = assembly.center_of_mass + ahead * MARK;
    let pose = relative(app, player, partner);
    let player_engines: Vec<Entity> = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf), With<ThrusterSectionMarker>>()
        .iter(app.world())
        .filter(|(_, parent)| parent.0 == player)
        .map(|(engine, _)| engine)
        .collect();
    app.world_mut()
        .entity_mut(driver)
        .insert(Autopilot::engage(AutopilotAction::GotoPos {
            position: mark,
        }));

    let (mut furthest, mut drift, mut player_burned) = (f32::NEG_INFINITY, (0.0f32, 0.0f32), false);
    let mut arrived = false;
    for _ in 0..TICK_CAP {
        app.update();
        let center = app
            .world()
            .get::<DockedAssembly>(driver)
            .unwrap()
            .center_of_mass;
        furthest = furthest.max((center - assembly.center_of_mass).dot(ahead));
        let (metres, degrees) = joint_drift(app, player, partner, pose);
        drift = (drift.0.max(metres), drift.1.max(degrees));
        if driver != player {
            player_burned |= player_engines
                .iter()
                .any(|&engine| **app.world().get::<ThrusterSectionInput>(engine).unwrap() > 0.0);
        }
        if app.world().get::<Autopilot>(driver).is_none() {
            arrived = true;
            break;
        }
    }

    assert!(arrived, "the leg ends in an arrival");
    let speed = app
        .world()
        .get::<DockedAssembly>(driver)
        .unwrap()
        .linear_velocity
        .length();
    assert!(speed < 0.5, "the pair stops at the mark: {speed} u/s");
    assert!(
        (furthest - rest).abs() < 1.0,
        "the pair stops at its rest point and never brakes past it: {furthest} of {rest}"
    );
    assert!(
        drift.0 < 0.05 && drift.1 < 0.2,
        "the joint holds: {drift:?} (units, deg)"
    );
    assert!(!player_burned, "a suppressed player never lights a drive");
    assert!(
        app.world().get::<DockedShip>(player).is_some(),
        "still docked"
    );
}

#[test]
fn a_held_pair_stops_and_arrives_without_crossing_the_mark() {
    let (mut app, player, partner) = docked_pair(WARSHIP);
    take_helm(&mut app, player);
    fly_to_mark(&mut app, player, player, partner);
}

#[test]
fn a_held_station_pair_stops_and_arrives_without_crossing_the_mark() {
    let (mut app, player, partner) = docked_pair(STATION);
    take_helm(&mut app, player);
    fly_to_mark(&mut app, player, player, partner);
}

#[test]
fn a_neutral_partner_goto_flies_the_pair() {
    let (mut app, player, partner) = docked_pair(WARSHIP);
    assert!(app.world().get::<DockedShip>(partner).unwrap().drives);
    fly_to_mark(&mut app, partner, player, partner);
}

#[test]
fn a_re_dock_starts_neutral() {
    let (mut app, player, partner) = docked_pair(WARSHIP);
    take_helm(&mut app, player);
    app.world_mut()
        .trigger(DockingReleaseRequest { entity: player });
    app.update();
    assert!(app.world().get::<DockedShip>(player).is_none());

    dock(&mut app, player, partner);
    let connection = app.world().get::<DockedShip>(player).unwrap().connection;
    assert_eq!(
        app.world()
            .get::<DockingConnection>(connection)
            .unwrap()
            .helm,
        DockedHelmType::Neutral
    );
    app.update();
    assert!(app.world().get::<DockedShip>(partner).unwrap().drives);
    assert!(!app.world().get::<DockedShip>(player).unwrap().drives);
}
