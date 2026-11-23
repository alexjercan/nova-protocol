//! A docked pair flown on real catalog ships: the helm, the autopilot and the
//! joint together, which no single crate's rig has.
//!
//! The partner is a second line warship (1:1) or a test-only station: a solid
//! 16x10x24 box of real hull cells with one real docking port, 20.7 times the
//! warship's mass. The station exists only in this file. Every assertion is
//! on motion - overshoot, reversals, the mark, joint drift - never on time.

use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use nova_events::prelude::MetersPerSecond3;
use nova_gameplay::{
    prelude::*,
    test_support::{settle, test_items, unfinished_integrity_physics_app},
};
use nova_ship::{flight::prelude::*, prelude::*};
use serde::Deserialize;

use crate::prelude::*;

const WARSHIP: &str = "line_warship";
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
    Style(ShipStyleConfig),
}

fn load(path: &str) -> Vec<Entry> {
    let file = format!("{}/../../assets/base/{path}", env!("CARGO_MANIFEST_DIR"));
    ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap()
}

/// The shipped catalogs, plus the test-only station.
fn catalogs() -> (GameSections, GameShipDesigns, GameStyles) {
    let sections = load("sections/base.content.ron")
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Section(section) => Some(section),
            Entry::Ship(_) | Entry::Style(_) => None,
        })
        .collect();
    let mut ships: Vec<ShipDesignPrototype> = load("ships/base.content.ron")
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Ship(ship) => Some(ship),
            Entry::Section(_) | Entry::Style(_) => None,
        })
        .collect();
    ships.push(station_design());
    let styles = load("styles/base.content.ron")
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Style(style) => Some(style),
            Entry::Section(_) | Entry::Ship(_) => None,
        })
        .collect();
    (
        GameSections(sections),
        GameShipDesigns(ships),
        GameStyles::new(styles, Vec::new()),
    )
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
    let (sections, ships, styles) = catalogs();
    app.insert_resource(sections);
    app.insert_resource(ships);
    app.insert_resource(styles);
    app.init_resource::<CargoCanisterIdAllocator>();
    app.insert_resource(test_items());
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
                initial_velocity: MetersPerSecond3::ZERO,
                allegiance: None,
                capabilities: default(),
                inventory: ShipInventoryStock::default(),
                lootable: false,
                credits: 0,
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

/// The pair's centre of mass from its roots' live avian state: the point a
/// docked pair's [`FlightPrediction`] follows.
fn pair_center_of_mass(app: &App, roots: [Entity; 2]) -> Vec3 {
    let world = app.world();
    let (sum, mass) = roots.iter().fold((Vec3::ZERO, 0.0), |(sum, total), &root| {
        let mass = world.get::<ComputedMass>(root).unwrap().value();
        let center = world.get::<Rotation>(root).unwrap().0
            * world.get::<ComputedCenterOfMass>(root).unwrap().0
            + world.get::<Position>(root).unwrap().0;
        (sum + center * mass, total + mass)
    });
    sum / mass
}

/// Fly the player's GOTO at a target `distance` ahead of the docked pair, one
/// fixed tick per update, and lay every published prediction over the flown
/// centre of mass of the pair at each tick it spans, the drawn polyline
/// between its points included. Returns the largest time-aligned deviation,
/// the largest distance of the flown point off the drawn line, the largest gap
/// between a path's last point and the flown arrival, and the leg's seconds.
fn fly_predicted_docked_goto(
    app: &mut App,
    player: Entity,
    partner: Entity,
    distance: f32,
    case: &str,
) -> (f32, f32, f32, f32) {
    let step = app.world().resource::<Time<Fixed>>().timestep();
    let dt = step.as_secs_f32();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    app.update();
    let roots = [player, partner];
    let ahead = app.world().get::<Rotation>(player).unwrap().0 * Vec3::NEG_Z;
    let goal = pair_center_of_mass(app, roots) + ahead * distance;
    let target = app
        .world_mut()
        .spawn((
            Transform::from_translation(goal),
            GlobalTransform::from(Transform::from_translation(goal)),
        ))
        .id();
    app.world_mut()
        .entity_mut(player)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));

    // Index u is the state after update u; index 0 is the engaged state.
    let mut coms = vec![pair_center_of_mass(app, roots)];
    let mut elapsed = vec![app.world().resource::<Time<Fixed>>().elapsed()];
    let mut predictions: Vec<FlightPrediction> = Vec::new();
    let mut released_at = None;
    for update in 1..=TICK_CAP {
        app.update();
        coms.push(pair_center_of_mass(app, roots));
        elapsed.push(app.world().resource::<Time<Fixed>>().elapsed());
        if let Some(prediction) = app.world().get::<FlightPrediction>(player) {
            if predictions
                .last()
                .is_none_or(|last| last.seed_time != prediction.seed_time)
            {
                predictions.push(prediction.clone());
            }
        }
        if app.world().get::<Autopilot>(player).is_none() {
            released_at = Some(update);
            break;
        }
    }
    let released_at = released_at.unwrap_or_else(|| panic!("{case}: the GOTO must arrive"));
    assert!(
        !predictions.is_empty(),
        "{case}: the docked pair's GOTO published no FlightPrediction in {released_at} ticks"
    );
    // The completing tick flies from the state after the update before it.
    let live_end = released_at - 1;
    let arrived = coms[live_end];
    let (mut deviation, mut end_gap, mut late, mut off_line) = (0.0f32, 0.0f32, 0usize, 0.0f32);
    let mut covered = vec![false; live_end + 1];
    for prediction in &predictions {
        assert_eq!(
            prediction.end,
            FlightPredictionEndType::Completed,
            "{case}: every path ends at the arrival"
        );
        let seed = elapsed
            .iter()
            .position(|&time| time == prediction.seed_time)
            .unwrap_or_else(|| panic!("{case}: a prediction seeds on a flown tick"));
        let last = prediction.points.len() - 1;
        let tick_of = |index: usize| {
            if index == last {
                seed + (prediction.final_point_time / dt).round() as usize
            } else {
                seed + 8 * index
            }
        };
        late = late.max(tick_of(last).abs_diff(live_end));
        end_gap = end_gap.max(prediction.points[last].distance(arrived));
        for tick in seed..=tick_of(last).min(live_end) {
            let index = (0..last)
                .find(|&index| tick <= tick_of(index + 1))
                .unwrap_or(last);
            let drawn = if index == last {
                prediction.points[last]
            } else {
                let (from, to) = (tick_of(index), tick_of(index + 1));
                let t = (tick - from) as f32 / (to - from) as f32;
                prediction.points[index].lerp(prediction.points[index + 1], t)
            };
            let error = drawn.distance(coms[tick]);
            deviation = deviation.max(error);
            // The flown point off the drawn line, wherever along it.
            let near = prediction.points[index.saturating_sub(4)..(index + 5).min(last + 1)]
                .windows(2)
                .map(|pair| {
                    let (a, b) = (pair[0], pair[1]);
                    let span = b - a;
                    let t = if span.length_squared() > 0.0 {
                        ((coms[tick] - a).dot(span) / span.length_squared()).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (a + span * t).distance(coms[tick])
                })
                .fold(drawn.distance(coms[tick]), f32::min);
            off_line = off_line.max(near);
            covered[tick] = true;
        }
    }
    assert!(
        covered[1..].iter().all(|&covered| covered),
        "{case}: the predictions cover every flown tick after the first"
    );
    let seconds = live_end as f32 * dt;
    println!(
        "{case}: {} predictions over {live_end} ticks ({seconds:.1} s); max deviation \
         {deviation} u, max end gap {end_gap} u, arrival tick off by at most {late}, max \
         off the drawn line {off_line} u; the first path ends {} u from the flown arrival",
        predictions.len(),
        predictions[0].points[predictions[0].points.len() - 1].distance(arrived),
    );
    (deviation, off_line, end_gap, seconds)
}

/// A GOTO flown by a docked pair is predicted as the pair flies it, end to
/// end: with the helm taken from a living warship, and with the helm a
/// derelict partner 20.7 times the player's mass hands over at capture.
///
/// The time-aligned bound is the undocked predictor's own on this 2000 u
/// leg: a lone warship measured 2.14 u (2026-11-22), from the flip and the
/// brake landing about one tick off the flown ones. The held pair measured
/// 1.69 u, the derelict station 0.04 u; every path ended within 0.65 u of
/// the flown arrival.
#[test]
fn a_docked_pair_goto_is_predicted_to_its_arrival() {
    for (case, partner, derelict, distance) in [
        ("held warship", WARSHIP, false, 2000.0),
        ("derelict station", STATION, true, 150.0),
    ] {
        let mut app = app();
        let offset = collar(&app, WARSHIP, PLAYER_COLLAR) + Vec3::X * (1.0 + GAP)
            - collar(&app, partner, PARTNER_COLLAR);
        let player = spawn_ship(&mut app, WARSHIP, Vec3::ZERO);
        let other = spawn_ship(&mut app, partner, offset);
        settle(&mut app);
        settle(&mut app);
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerSpaceshipMarker);
        if derelict {
            app.world_mut().entity_mut(other).insert(DerelictShipMarker);
        }
        dock(&mut app, player, other);
        settle(&mut app);
        if derelict {
            assert!(
                app.world().get::<DockedShip>(player).unwrap().drives,
                "{case}: a derelict hands the helm over at capture"
            );
        } else {
            take_helm(&mut app, player);
        }
        let (deviation, off_line, end_gap, seconds) =
            fly_predicted_docked_goto(&mut app, player, other, distance, case);
        assert!(
            app.world().get::<DockedShip>(player).is_some(),
            "{case}: still docked"
        );
        assert!(
            seconds > 30.0,
            "{case}: the leg must outlast 30 s: {seconds} s"
        );
        assert!(
            deviation <= 2.5,
            "{case}: the path strays {deviation} u from the flown pair at the same time"
        );
        assert!(
            off_line <= 1.0,
            "{case}: the flown pair runs {off_line} u off the drawn line"
        );
        assert!(
            end_gap <= 1.0,
            "{case}: a path ends {end_gap} u off the flown arrival"
        );
    }
}
