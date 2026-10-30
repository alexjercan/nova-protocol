use avian3d::prelude::*;
use bevy_rand::prelude::*;
use nova_gameplay::test_support::{settle, test_items, unfinished_integrity_physics_app};

use super::*;
use crate::{
    loader::prelude::ScenarioScopedMarker,
    mining::{MinedOreDrop, MiningPlugin},
};

/// Frames a drop may take to drain before the test calls it hung. A cap that
/// names a hang, not a budget.
const FRAME_CAP: usize = 20_000;

/// One canister's stacks, in id order.
type Stacks = Vec<(String, u32)>;

/// Every drop as it spawned: the drop, its parent, its world position and
/// its waiting canisters front first.
#[derive(Resource, Default)]
struct Drops(Vec<(Entity, Option<Entity>, Vec3, Vec<Stacks>)>);

fn stacks(canister: &CargoCanister) -> Stacks {
    canister
        .stacks()
        .map(|(item, count)| (item.to_string(), count))
        .collect()
}

fn wreck_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.add_plugins((
        ShipIntegrityPlugin,
        MiningPlugin { render: false },
        WreckCargoPlugin,
    ));
    // The destroy path's debris observers need material assets and the
    // global rng even in a headless run.
    app.init_asset::<StandardMaterial>();
    app.add_plugins(EntropyPlugin::<WyRand>::default());
    app.init_resource::<CargoCanisterIdAllocator>();
    app.insert_resource(test_items());
    app.init_resource::<Drops>();
    app.add_observer(
        |add: On<Add, MinedOreDrop>,
         q_drops: Query<(&MinedCanisterQueue, &GlobalTransform, Option<&ChildOf>)>,
         mut drops: ResMut<Drops>| {
            let (queue, frame, child_of) = q_drops.get(add.entity).expect("a drop has its queue");
            drops.0.push((
                add.entity,
                child_of.map(ChildOf::parent),
                frame.translation(),
                queue.canisters().map(stacks).collect(),
            ));
        },
    );
    app.finish();
    app
}

/// The recipe every test section carries: a 0.3 strip yield, so three panels
/// return none, seven spars two and four bells one.
fn recipe() -> BuildRecipe {
    BuildRecipe {
        credits: 100,
        parts: [("SectionPanel", 3), ("FrameSpar", 7), ("DriveBell", 4)]
            .into_iter()
            .map(|(item, count)| (ItemDesignId::from(item), count))
            .collect(),
        strip_yield: 0.3,
    }
}

/// A ship of `sections` unit cubes in a line along +X from its root, at
/// `(2, 0, 0)` in a sector placed at `(100, 0, 0)`, holding `hold`. Returns
/// the sector, the root and the sections.
fn spawn_ship(
    app: &mut App,
    sections: usize,
    hold: &[(ItemDesignId, u32)],
) -> (Entity, Entity, Vec<Entity>) {
    let items = test_items();
    let world = app.world_mut();
    let sector = world
        .spawn((Transform::from_xyz(100.0, 0.0, 0.0), Visibility::default()))
        .id();
    let root = world
        .spawn((
            RigidBody::Dynamic,
            Transform::from_xyz(2.0, 0.0, 0.0),
            Visibility::default(),
            ChildOf(sector),
            SpaceshipRootMarker,
            ShipInventory::new(&items, 10_000_000, hold.iter().cloned()),
        ))
        .id();
    let config = SectionConfig {
        base: BaseSectionConfig {
            build: recipe(),
            ..default()
        },
        kind: SectionKind::Hull(HullSectionConfig::default()),
    };
    let sections = (0..sections)
        .map(|i| {
            world
                .spawn((
                    ChildOf(root),
                    SectionMarker,
                    SectionBuildConfig(config.clone()),
                    Transform::from_translation(Vec3::X * i as f32),
                    SectionLinkPoints(unit_cube_link_points()),
                    ConnectedTo::default(),
                    Collider::cuboid(1.0, 1.0, 1.0),
                    ColliderDensity(1.0),
                    Health::new(100.0),
                ))
                .id()
        })
        .collect();
    settle(app);
    (sector, root, sections)
}

fn hit(app: &mut App, target: Entity, amount: f32) {
    app.world_mut().trigger(HealthApplyDamage {
        entity: target,
        source: None,
        amount,
    });
}

/// Run frames until `drop` has let every canister out, and return each
/// canister born meanwhile in birth order: its runtime id, its stacks and
/// where it stood on the first frame it was seen.
fn drain(app: &mut App, drop: Entity) -> Vec<(CargoCanisterRuntimeId, Stacks, Vec3)> {
    let mut born: Vec<(Entity, CargoCanisterRuntimeId, Stacks, Vec3)> = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        let world = app.world_mut();
        let fresh: Vec<_> = world
            .query::<(
                Entity,
                &CargoCanister,
                &CargoCanisterRuntimeId,
                &GlobalTransform,
            )>()
            .iter(world)
            .filter(|(canister, ..)| !born.iter().any(|(each, ..)| each == canister))
            .map(|(canister, contents, id, frame)| {
                (canister, *id, stacks(contents), frame.translation())
            })
            .collect();
        for (canister, ..) in &fresh {
            assert!(
                world.get::<ScenarioScopedMarker>(*canister).is_some(),
                "a wreck canister belongs to the scenario"
            );
        }
        born.extend(fresh);
        if world.get_entity(drop).is_err() {
            return born
                .into_iter()
                .map(|(_, id, stacks, at)| (id, stacks, at))
                .collect();
        }
    }
    panic!("the drop never drained");
}

/// A loaded ship that collapses is destroyed whole and leaves ONE drop where
/// its root stood, under the root's sector: every stack of its hold plus
/// `floor(0.3 * count)` of each part of all six sections it peeled, 1300 kg
/// packed in item id order into a full 1000 kg canister and a 300 kg one.
/// Panels round down to none. The drop lets the canisters out first in,
/// first out, each with its own id.
#[test]
fn a_ship_destroyed_whole_drops_its_hold_and_its_sections_parts_in_one_drop() {
    let mut app = wreck_app();
    let hold = [
        (ItemDesignId::from(ItemRoleType::IronOre), 25),
        (ItemDesignId::from(ItemRoleType::PdcRound), 300),
        (ItemDesignId::from(ItemRoleType::HullPlate), 3),
    ];
    let (sector, root, sections) = spawn_ship(&mut app, 6, &hold);
    // Each section down to four percent: 24 of a pinned 600 is under the
    // default collapse threshold with every section still standing, so the
    // collapse peels the whole ship.
    for section in &sections {
        hit(&mut app, *section, 96.0);
    }
    for _ in 0..60 {
        if app.world().get_entity(root).is_err() {
            break;
        }
        app.update();
    }
    assert!(app.world().get_entity(root).is_err(), "the ship never died");
    for _ in 0..4 {
        app.update();
    }

    let drops = &app.world().resource::<Drops>().0;
    let [(drop, parent, at, canisters)] = &drops[..] else {
        panic!("a ship destroyed whole leaves one drop: {drops:?}");
    };
    assert_eq!(*parent, Some(sector), "the drop stays in the root's sector");
    assert!(at.distance(Vec3::new(102.0, 0.0, 0.0)) < 1e-3, "{at}");
    // Six sections: 6 bells and 12 spars, 960 kg, then the hold's 30 kg of
    // plates and the one 10 kg ore that still fits.
    let expected = vec![
        vec![
            ("DriveBell".to_string(), 6),
            ("FrameSpar".to_string(), 12),
            ("HullPlate".to_string(), 3),
            ("IronOre".to_string(), 1),
        ],
        vec![("IronOre".to_string(), 24), ("PdcRound".to_string(), 300)],
    ];
    assert_eq!(canisters, &expected);
    let drop = *drop;

    let born = drain(&mut app, drop);
    let contents: Vec<Stacks> = born.iter().map(|(_, stacks, _)| stacks.clone()).collect();
    assert_eq!(contents, expected, "first in, first out, nothing lost");
    let mut ids: Vec<u64> = born.iter().map(|(id, ..)| id.0).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), expected.len(), "every canister has its own id");
    assert_eq!(app.world().resource::<Drops>().0.len(), 1, "no second drop");
}

/// A destroyed station drops the player's storage with its stock: the
/// stored stacks join the hold in the one drop, so a station blown apart
/// loses the player nothing that a canister can carry.
#[test]
fn a_destroyed_station_drops_the_players_storage_with_its_stock() {
    let mut app = wreck_app();
    let items = test_items();
    let hold = [(ItemDesignId::from(ItemRoleType::IronOre), 5)];
    let (_, root, sections) = spawn_ship(&mut app, 2, &hold);
    let mut storage = StationStorage::new(10_000_000);
    storage.add(&items, &ItemDesignId::from(ItemRoleType::HullPlate), 2);
    storage.add(&items, &ItemDesignId::from(ItemRoleType::IronOre), 3);
    app.world_mut()
        .entity_mut(root)
        .insert((StationMarker, storage));

    for section in &sections {
        hit(&mut app, *section, 100.0);
    }
    for _ in 0..60 {
        if app.world().get_entity(root).is_err() {
            break;
        }
        app.update();
    }
    assert!(
        app.world().get_entity(root).is_err(),
        "the station never died"
    );

    let drops = &app.world().resource::<Drops>().0;
    let mut root_drop = BTreeMap::<String, u32>::new();
    for (item, count) in drops
        .iter()
        .flat_map(|(.., canisters)| canisters.iter().flatten())
    {
        if item == "HullPlate" || item == "IronOre" {
            *root_drop.entry(item.clone()).or_default() += count;
        }
    }
    assert_eq!(
        root_drop,
        [("HullPlate".to_string(), 2), ("IronOre".to_string(), 8)].into(),
        "the stored plates and ore leave with the stock: {drops:?}"
    );
}

/// One section shot off a ship that keeps flying drops only its own share,
/// `floor(0.3 * count)` of each part, where it died and under the ship's
/// sector: no panel, two spars, one bell, in one canister. The hold stays
/// aboard. The canister leaves outward, along the line from the ship's centre through
/// the section.
#[test]
fn a_section_shot_off_a_standing_ship_drops_its_parts_share_alone() {
    let mut app = wreck_app();
    let hold = [(ItemDesignId::from(ItemRoleType::IronOre), 25)];
    let (sector, root, sections) = spawn_ship(&mut app, 4, &hold);
    let held = app.world().get::<ShipInventory>(root).cloned();

    hit(&mut app, sections[3], 100.0);
    for _ in 0..10 {
        app.update();
    }

    assert!(app.world().get_entity(root).is_ok(), "the ship still flies");
    assert_eq!(app.world().get::<ShipInventory>(root).cloned(), held);
    let drops = &app.world().resource::<Drops>().0;
    let [(drop, parent, at, canisters)] = &drops[..] else {
        panic!("one lost section leaves one drop: {drops:?}");
    };
    assert_eq!(*parent, Some(sector));
    assert!(at.distance(Vec3::new(105.0, 0.0, 0.0)) < 1e-3, "{at}");
    let expected = vec![vec![
        ("DriveBell".to_string(), 1),
        ("FrameSpar".to_string(), 2),
    ]];
    assert_eq!(canisters, &expected);
    let drop = *drop;

    let born = drain(&mut app, drop);
    let contents: Vec<Stacks> = born.iter().map(|(_, stacks, _)| stacks.clone()).collect();
    assert_eq!(contents, expected);
    let (_, _, first_at) = &born[0];
    assert!(
        first_at.distance(Vec3::new(106.0, 0.0, 0.0)) < 0.1,
        "born one unit outward of the section: {first_at}"
    );
    assert_eq!(app.world().resource::<Drops>().0.len(), 1, "no second drop");
}

/// A section shot off a moving ship leaves canisters that move with the
/// ship: left at rest, they sit in its path and it rams its own salvage on
/// the next frame.
#[test]
fn a_section_shot_off_a_moving_ship_drops_canisters_that_keep_its_speed() {
    let mut app = wreck_app();
    let (_, root, sections) = spawn_ship(&mut app, 4, &[]);
    let velocity = Vec3::new(0.0, 0.0, 5.0);
    app.world_mut()
        .get_mut::<LinearVelocity>(root)
        .expect("a dynamic root has a velocity")
        .0 = velocity;

    hit(&mut app, sections[3], 100.0);
    for _ in 0..10 {
        app.update();
    }
    let drops = &app.world().resource::<Drops>().0;
    let [(drop, ..)] = &drops[..] else {
        panic!("one lost section leaves one drop: {drops:?}");
    };
    let drop = *drop;
    drain(&mut app, drop);

    let world = app.world_mut();
    let velocities: Vec<Vec3> = world
        .query_filtered::<&LinearVelocity, With<CargoCanister>>()
        .iter(world)
        .map(|linear| linear.0)
        .collect();
    assert!(!velocities.is_empty(), "the drop let its canisters out");
    for canister in velocities {
        assert!(
            (canister.z - velocity.z).abs() < 0.1,
            "a canister left at {canister} beside a ship moving at {velocity}"
        );
    }
}
