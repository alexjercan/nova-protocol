//! Asserted ECS examples for the whole-hull strip: the order the sections
//! come off in, the hold that holds a strip, the demolition that releases
//! the player, and the undock that stops it.

use std::{collections::BTreeMap, time::Duration};

use bevy::time::TimeUpdateStrategy;
use nova_events::prelude::EntityId;
use nova_gameplay::{prelude::*, test_support::test_items};
use nova_ship::prelude::*;

use super::*;

/// Game seconds each update advances: the virtual clock's own clamp, so no
/// step is cut short.
const STEP_SECS: f32 = 0.25;

/// The strip app: the strip systems, the real docking release, and a
/// stand-in for `nova_ship`'s section removal that records what came off.
fn strip_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        STEP_SECS,
    )));
    app.insert_resource(test_items());
    app.init_resource::<InventoryRuntime>();
    app.init_resource::<TakenOrder>();
    app.add_message::<HullStripCommand>();
    app.add_plugins(DockingSectionPlugin { render: false });
    app.add_observer(
        |request: On<SectionStripRequest>,
         mut order: ResMut<TakenOrder>,
         mut commands: Commands| {
            order.0.push(request.entity);
            commands.entity(request.entity).despawn();
        },
    );
    app.add_systems(
        Update,
        (apply_hull_strip_commands, advance_hull_strip).chain(),
    );
    // A first update of zero seconds, so every later one is a whole step.
    app.update();
    app
}

/// The sections `SectionStripRequest` took off, in order.
#[derive(Resource, Default)]
struct TakenOrder(Vec<Entity>);

/// The wreck: the berth port joined to hull A, hull A to hull B and to
/// turret C. Each hull and the port strip into one section panel in 3 s, the
/// turret into three panels and a spar in 8 s.
struct Wreck {
    root: Entity,
    berth: Entity,
    a: Entity,
    b: Entity,
    c: Entity,
}

fn recipe(panels: u32, spars: u32) -> BuildRecipe {
    let mut parts = BTreeMap::from([("SectionPanel".into(), panels)]);
    if spars > 0 {
        parts.insert("FrameSpar".into(), spars);
    }
    BuildRecipe {
        credits: 10,
        parts,
        strip_yield: 0.5,
    }
}

fn design_section(id: &str, kind: SectionKind, build: BuildRecipe) -> DesignSection {
    DesignSection {
        id: id.to_string(),
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        config: SectionConfig {
            base: BaseSectionConfig {
                id: id.to_string(),
                build,
                ..default()
            },
            kind,
        },
    }
}

/// A player with a hold of `capacity_g` docked port to port with the wreck.
fn dock_with_a_wreck(world: &mut World, capacity_g: u32) -> (Entity, Wreck) {
    let items = test_items();
    let player = world
        .spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            ShipInventory::new(&items, capacity_g, []),
            ShipInventoryReports::default(),
        ))
        .id();
    let player_port = world.spawn((DockingSectionMarker, ChildOf(player))).id();
    let root = world
        .spawn((
            SpaceshipRootMarker,
            LootableShipMarker,
            EntityId("wreck".to_string()),
            ShipDesignSections(vec![
                design_section(
                    "port",
                    SectionKind::Docking(DockingSectionConfig::default()),
                    recipe(2, 0),
                ),
                design_section(
                    "hull_a",
                    SectionKind::Hull(HullSectionConfig::default()),
                    recipe(2, 0),
                ),
                design_section(
                    "hull_b",
                    SectionKind::Hull(HullSectionConfig::default()),
                    recipe(2, 0),
                ),
                design_section(
                    "pdc",
                    SectionKind::Turret(TurretSectionConfig::default()),
                    recipe(6, 2),
                ),
            ]),
        ))
        .id();
    let section = |world: &mut World, id: &str| {
        world
            .spawn((
                SectionMarker,
                EntityId(id.to_string()),
                Health::new(100.0),
                ChildOf(root),
            ))
            .id()
    };
    let (berth, a, b, c) = (
        section(world, "port"),
        section(world, "hull_a"),
        section(world, "hull_b"),
        section(world, "pdc"),
    );
    world
        .entity_mut(berth)
        .insert((DockingSectionMarker, ConnectedTo(vec![a])));
    world.entity_mut(a).insert(ConnectedTo(vec![berth, b, c]));
    world.entity_mut(b).insert(ConnectedTo(vec![a]));
    world.entity_mut(c).insert(ConnectedTo(vec![a]));
    let connection = world
        .spawn(DockingConnection {
            first_ship: player,
            first_section: player_port,
            second_ship: root,
            second_section: berth,
            helm: DockedHelmType::Neutral,
            measurement_fault: false,
        })
        .id();
    world.entity_mut(player_port).insert(DockedPort(connection));
    world.entity_mut(berth).insert(DockedPort(connection));
    world.entity_mut(player).insert(DockedShip {
        connection,
        helm: Quat::IDENTITY,
        drives: false,
    });
    (
        player,
        Wreck {
            root,
            berth,
            a,
            b,
            c,
        },
    )
}

fn job(app: &App, player: Entity) -> Option<&HullStripJob> {
    app.world().get::<HullStripJob>(player)
}

fn hold(app: &App, player: Entity) -> &ShipInventory {
    app.world()
        .get::<ShipInventory>(player)
        .expect("the player has a hold")
}

/// Run updates until `done`, at most `limit`; the number run.
fn run_until(app: &mut App, limit: u32, done: impl Fn(&App) -> bool) -> u32 {
    for frame in 0..limit {
        if done(app) {
            return frame;
        }
        app.update();
    }
    panic!("not done after {limit} updates");
}

#[test]
fn a_whole_hull_strip_takes_the_berth_last_then_releases_the_player_and_demolishes_the_wreck() {
    let mut app = strip_app();
    let (player, wreck) = dock_with_a_wreck(app.world_mut(), 1_000_000);
    app.world_mut().write_message(HullStripCommand);
    app.update();
    let total = job(&app, player).expect("the strip starts").total_secs;
    assert_eq!(total, 3.0 + 3.0 + 3.0 + 8.0);

    let frames = run_until(&mut app, 200, |app| {
        app.world().get_entity(wreck.root).is_err()
    });

    // B and C are both two hops out; B wins the tie by code (HULL-2 before
    // PDC-1). Then A, and the berth comes off with the wreck.
    assert_eq!(
        app.world().resource::<TakenOrder>().0,
        vec![wreck.b, wreck.c, wreck.a]
    );
    // The strip ran on the game clock: no sooner than its planned time.
    assert!(frames as f32 * STEP_SECS >= total - STEP_SECS);
    assert!(app.world().get_entity(wreck.berth).is_err());
    assert!(app.world().get::<DockedShip>(player).is_none());
    assert!(job(&app, player).is_none());
    let hold = hold(&app, player);
    assert_eq!(hold.count(&"SectionPanel".into()), 6);
    assert_eq!(hold.count(&"FrameSpar".into()), 1);
    let reports = &app
        .world()
        .get::<ShipInventoryReports>(player)
        .expect("the player keeps reports")
        .0;
    let sections: Vec<&str> = reports
        .iter()
        .filter_map(|report| match report {
            ShipInventoryReport::Stripped { hull, section, .. } => {
                assert_eq!(hull, "wreck");
                Some(section.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(sections, ["HULL-2", "PDC-1", "HULL-1", "DOCK-1"]);
}

#[test]
fn a_full_hold_holds_the_strip_at_a_finished_section_until_there_is_room() {
    let mut app = strip_app();
    // Room for hull B's one 10 kg panel, not for the turret's 70 kg.
    let (player, wreck) = dock_with_a_wreck(app.world_mut(), 15_000);
    app.world_mut().write_message(HullStripCommand);
    app.update();
    run_until(&mut app, 200, |app| {
        job(app, player).is_some_and(|job| job.paused.is_some())
    });
    let job_now = job(&app, player).expect("the strip holds, it does not stop");
    assert_eq!(
        job_now.paused,
        Some(HullStripPauseType::HoldFull {
            need_g: 70_000,
            free_g: 5_000
        })
    );
    assert_eq!(
        job_now.step.as_ref().map(|step| step.section),
        Some(wreck.c)
    );
    let progress = job_now.progress();
    for _ in 0..5 {
        app.update();
    }
    let held = job(&app, player).expect("still held");
    assert_eq!(held.progress(), progress, "a held strip does not advance");
    assert!(app.world().get_entity(wreck.c).is_ok());
    assert_eq!(hold(&app, player).count(&"SectionPanel".into()), 1);

    // Room appears: the turret comes off at once and the strip goes on.
    let items = test_items();
    let roomy = ShipInventory::new(
        &items,
        1_000_000,
        hold(&app, player)
            .stacks()
            .map(|(item, count)| (item.clone(), count))
            .collect::<Vec<_>>(),
    );
    app.world_mut().entity_mut(player).insert(roomy);
    app.update();
    assert!(app.world().get_entity(wreck.c).is_err());
    assert_eq!(job(&app, player).and_then(|job| job.paused), None);
}

#[test]
fn an_undock_stops_the_strip_and_keeps_what_it_took() {
    let mut app = strip_app();
    let (player, wreck) = dock_with_a_wreck(app.world_mut(), 1_000_000);
    app.world_mut().write_message(HullStripCommand);
    app.update();
    run_until(&mut app, 200, |app| {
        app.world().get_entity(wreck.b).is_err()
    });
    app.world_mut()
        .trigger(DockingReleaseRequest { entity: player });
    app.update();
    assert!(job(&app, player).is_none());
    assert!(app.world().get_entity(wreck.root).is_ok());
    for kept in [wreck.berth, wreck.a, wreck.c] {
        assert!(app.world().get_entity(kept).is_ok());
    }
    assert_eq!(
        app.world().resource::<InventoryRuntime>().note(),
        Some("strip stopped: you undocked; 1 section taken")
    );
}
