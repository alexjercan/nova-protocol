//! The saved-world folder: names, locks, refusals and the write order.

use std::{path::Path, time::Duration};

use avian3d::prelude::{LinearVelocity, RigidBody, Rotation};
use bevy::prelude::*;
use nova_assets::prelude::{ContentCatalogDigest, LoadedSectionPack, LoadedSectionPacks};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{
    nova_blast, resumed_lifetime, CargoCanisterRuntimeId, ClockFreeze, DamageMarks, DamageType,
    FreezeOwner, FrozenRoundFlight, Health, IntegrityDestroyMarker, PlayerSpaceshipMarker,
    PointRotationOutput, ProjectileDamage, ProjectileOwner, RailgunSlugProjectileMarker, RoundRake,
    SavedBodyRef, SavedLifetime, SavedOwner, SavedSectionRef, SavedTargetRef, ShipCredits,
    ShipInventory, SpaceshipRootMarker, TorpedoProjectileMarker, TurretBulletProjectileMarker,
};
use nova_scenario::prelude::{
    freeze_ship, spaceship_scenario_object, CurrentScenario, SpaceshipConfig,
};
use nova_ship::prelude::{
    thaw_round, thaw_torpedo, CameraView, ChaseZoom, FrozenRound, FrozenTorpedo, RoundSourceType,
    SavedTorpedoTarget, SpaceshipCameraController, SpaceshipCameraInputMarker,
    SpaceshipCameraNormalInputMarker, TorpedoArming, TorpedoControllerMarker, TorpedoSectionConfig,
    TorpedoTargetChosen, TorpedoTargetEntity, TorpedoTargetPosition, TorpedoWeave,
};
use nova_world::{
    prelude::{CurrentSector, FrozenSectors, SectorCoord, WorldConfig},
    SectorRoot,
};

use super::{
    create_world, list_worlds, open_world, restore_resumed_world, resume_world, write_world,
    FrozenTransient, FrozenTransientType, ResumedTransients, SavedPlayer, WorldRefusal,
    WorldResumeProgress, WorldResumeRefused, WorldSaveHeader, WorldSaveSession, WorldSaveState,
    WorldSaveStatus, WORLD_SAVE_FORMAT,
};
use crate::NovaLayeredWorld;

/// A loaded catalog of digest `digest` from the base game and one mod.
fn catalog(digest: u64) -> LoadedSectionPacks {
    let pack = |id: &str| LoadedSectionPack {
        id: id.to_string(),
        dependencies: Vec::new(),
        sections: Vec::new(),
    };
    LoadedSectionPacks {
        packs: vec![pack("base"), pack("extra_hulls")],
        digest: ContentCatalogDigest(digest),
    }
}

/// The header of generation `generation` of the world `name`, saved with the
/// catalog `catalog(7)`.
fn header(name: &str, generation: u64) -> WorldSaveHeader {
    WorldSaveHeader {
        format: WORLD_SAVE_FORMAT,
        name: name.to_string(),
        seed: 42,
        catalog: 7,
        mods: vec!["base".to_string(), "extra_hulls".to_string()],
        game_version: "0.0.0-test".to_string(),
        saved_at_unix: 1_700_000_000,
        generation,
        player_sector: SectorCoord::new(1, 0, -2),
        credits: 750,
    }
}

/// The state of generation `generation`: a real frozen ship with `credits`
/// and an empty ledger.
fn state(generation: u64, credits: u32) -> WorldSaveState {
    let mut world = World::new();
    let ship = world
        .spawn((
            spaceship_scenario_object(SpaceshipConfig {
                credits,
                ..default()
            }),
            ShipInventory::default(),
            DamageMarks::default(),
        ))
        .id();
    WorldSaveState {
        format: WORLD_SAVE_FORMAT,
        generation,
        player: SavedPlayer {
            id: EntityId("player".to_string()),
            transform: Transform::from_xyz(1.5, -2.25, 3.0)
                .with_rotation(Quat::from_rotation_y(0.3)),
            motion: (Vec3::new(0.1, 0.0, -4.0), Vec3::new(0.0, 0.02, 0.0)),
            ship: freeze_ship(&world, ship).expect("a bare ship is settled"),
            view: CameraView {
                position_from_ship: Vec3::new(0.0, 4.0, 12.0),
                rotation_from_ship: Quat::from_rotation_x(-0.2),
                steer_from_ship: Quat::from_rotation_y(0.1),
                zoom: 2.0,
            },
        },
        sectors: FrozenSectors::default(),
        canister_ids_next: 12,
        transients: Vec::new(),
    }
}

/// The names of the files in `dir`, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A name outside the rule is refused before anything is made on disk.
#[test]
fn create_refuses_an_empty_long_or_unsafe_name_and_makes_nothing() {
    let root = tempfile::tempdir().unwrap();
    for name in ["   ", &"a".repeat(33), "a/b", "..", "dot.name"] {
        assert!(
            matches!(
                create_world(root.path(), name),
                Err(WorldRefusal::InvalidName(_))
            ),
            "{name:?} must be refused"
        );
    }
    assert!(
        files(root.path()).is_empty(),
        "a refused name makes nothing"
    );
}

/// Two names with the same folder name are one world: the second is refused
/// and the first world is untouched.
#[test]
fn create_refuses_a_name_whose_folder_exists() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "  Deep Field ").unwrap();
    assert_eq!(folder.slug, "deep-field");
    write_world(&folder, &header("Deep Field", 1), &state(1, 750)).unwrap();
    drop(lock);

    assert_eq!(
        create_world(root.path(), "deep field").unwrap_err(),
        WorldRefusal::NameTaken
    );
    assert_eq!(
        files(&folder.path),
        ["state.1.ron", "world.lock", "world.ron"]
    );
}

/// While one game holds a world, a second open of it is refused; once the
/// lock drops, the world opens.
#[test]
fn a_world_another_game_holds_open_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Held").unwrap();
    write_world(&folder, &header("Held", 1), &state(1, 750)).unwrap();

    assert_eq!(
        open_world(root.path(), "held", &catalog(7)).unwrap_err(),
        WorldRefusal::Locked
    );
    drop(lock);
    assert!(open_world(root.path(), "held", &catalog(7)).is_ok());
}

/// The list shows every folder with the reason it cannot load: another
/// catalog with the mods it was saved with, another format, a missing header,
/// and a folder no world name gives. Open refuses the same headers.
#[test]
fn the_list_shows_why_each_refused_world_cannot_load() {
    let root = tempfile::tempdir().unwrap();
    for name in ["Good", "Other Format"] {
        let (folder, _lock) = create_world(root.path(), name).unwrap();
        write_world(&folder, &header(name, 1), &state(1, 750)).unwrap();
    }
    // A later layout: another format and a field this build does not know.
    let other = root.path().join("other-format").join("world.ron");
    let text = std::fs::read_to_string(&other).unwrap().replacen(
        &format!("format: {WORLD_SAVE_FORMAT},"),
        "format: 99, weather: \"rain\",",
        1,
    );
    std::fs::write(&other, text).unwrap();
    drop(create_world(root.path(), "Empty").unwrap());
    std::fs::create_dir(root.path().join("Hand Made")).unwrap();

    let listed = list_worlds(root.path(), &catalog(8)).unwrap();
    let shown: Vec<(&str, Result<&str, &WorldRefusal>)> = listed
        .iter()
        .map(|listing| {
            (
                listing.folder.slug.as_str(),
                listing.header.as_ref().map(|header| header.name.as_str()),
            )
        })
        .collect();
    assert_eq!(shown.len(), 4);
    assert_eq!(shown[0].0, "Hand Made");
    assert!(matches!(shown[0].1, Err(WorldRefusal::InvalidName(_))));
    assert_eq!(shown[1].0, "empty");
    assert!(matches!(shown[1].1, Err(WorldRefusal::Unreadable(_))));
    assert_eq!(
        shown[2],
        (
            "good",
            Err(&WorldRefusal::Catalog {
                saved: 7,
                loaded: 8,
                saved_mods: vec!["base".to_string(), "extra_hulls".to_string()],
            })
        )
    );
    assert_eq!(
        shown[3],
        ("other-format", Err(&WorldRefusal::Format { found: 99 }))
    );

    assert!(matches!(
        open_world(root.path(), "good", &catalog(8)),
        Err(WorldRefusal::Catalog { .. })
    ));
    assert!(matches!(
        open_world(root.path(), "other-format", &catalog(7)),
        Err(WorldRefusal::Format { found: 99 })
    ));
    assert!(matches!(
        open_world(root.path(), "../good", &catalog(7)),
        Err(WorldRefusal::InvalidName(_))
    ));
    let good = list_worlds(root.path(), &catalog(7)).unwrap();
    assert_eq!(good[2].header.as_ref().unwrap(), &header("Good", 1));
}

/// No root yet is no worlds; a root that cannot be read is an error, not an
/// empty list.
#[test]
fn a_missing_root_lists_nothing_and_an_unreadable_root_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    assert!(list_worlds(&root.path().join("worlds"), &catalog(7))
        .unwrap()
        .is_empty());
    let file = root.path().join("not-a-folder");
    std::fs::write(&file, "").unwrap();
    assert!(matches!(
        list_worlds(&file, &catalog(7)),
        Err(WorldRefusal::Io(_))
    ));
}

/// A later save replaces the earlier one, and the state opens exactly as it
/// was written: the same file encodes again byte for byte.
#[test]
fn a_written_world_opens_as_it_was_saved() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Round Trip").unwrap();
    write_world(&folder, &header("Round Trip", 1), &state(1, 750)).unwrap();
    write_world(&folder, &header("Round Trip", 2), &state(2, 990)).unwrap();
    assert_eq!(
        files(&folder.path),
        ["state.2.ron", "world.lock", "world.ron"]
    );
    drop(lock);

    let (opened, _lock, read, saved) = open_world(root.path(), "round-trip", &catalog(7)).unwrap();
    assert_eq!(opened, folder);
    assert_eq!(read, header("Round Trip", 2));
    assert_eq!(saved.generation, 2);
    assert_eq!(saved.canister_ids_next, 12);
    assert_eq!(saved.player.transform, state(2, 990).player.transform);
    assert_eq!(
        ron::to_string(&saved).unwrap(),
        std::fs::read_to_string(folder.path.join("state.2.ron")).unwrap()
    );
    assert_eq!(
        ron::to_string(&saved).unwrap(),
        ron::to_string(&state(2, 990)).unwrap()
    );
}

/// A write that fails leaves the last good save as the one that opens.
#[test]
fn a_failed_write_leaves_the_last_good_save() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Kept").unwrap();
    write_world(&folder, &header("Kept", 1), &state(1, 750)).unwrap();
    // The next state file cannot be renamed into place.
    std::fs::create_dir(folder.path.join("state.2.ron")).unwrap();
    assert!(write_world(&folder, &header("Kept", 2), &state(2, 990)).is_err());
    drop(lock);

    let (_, _lock, read, saved) = open_world(root.path(), "kept", &catalog(7)).unwrap();
    assert_eq!(read.generation, 1);
    assert_eq!(
        ron::to_string(&saved).unwrap(),
        ron::to_string(&state(1, 750)).unwrap()
    );
}

/// What an interrupted write leaves - a state the header does not name and a
/// temp file - goes at the next open; the named state and the lock stay.
#[test]
fn opening_a_world_removes_the_files_its_header_does_not_name() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Swept").unwrap();
    write_world(&folder, &header("Swept", 1), &state(1, 750)).unwrap();
    drop(lock);
    std::fs::write(folder.path.join("state.2.ron"), "half").unwrap();
    std::fs::write(folder.path.join(".world.ron.123.tmp"), "half").unwrap();
    std::fs::write(folder.path.join("notes.txt"), "mine").unwrap();

    open_world(root.path(), "swept", &catalog(7)).unwrap();
    assert_eq!(
        files(&folder.path),
        ["notes.txt", "state.1.ron", "world.lock", "world.ron"]
    );
}

/// An armed open world with its player ship and the session of a world
/// created under `root`. [`frame`] runs it through the app's schedules: the
/// save systems in `Update`, then the chase sync in `PostUpdate`.
fn armed_session(root: &Path) -> (World, Entity) {
    let mut app = App::new();
    app.add_plugins(crate::test_support::WorldSaveTestPlugin);
    let mut world = std::mem::take(app.world_mut());
    let player = crate::test_support::arm_save_fixture(&mut world);
    let (folder, lock) = create_world(root, "Session").unwrap();
    world.insert_resource(WorldSaveSession::created(
        folder,
        lock,
        "Session".to_string(),
        42,
    ));
    (world, player)
}

/// Run one frame of [`armed_session`]'s world.
fn frame(world: &mut World) {
    world.run_schedule(Main);
}

/// Run frames until the session is idle, with a cap.
fn run_until_idle(world: &mut World) {
    for _ in 0..2_000 {
        frame(world);
        if world.resource::<WorldSaveSession>().is_idle() {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("the session never went idle");
}

/// The arming save writes a new world, and a leave asked while that write
/// runs saves it again: the session goes idle and saved only with the leave
/// save on disk, and it reads back with the player as it stood.
#[test]
fn the_first_frame_and_a_leave_each_write_a_save() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Unsaved
    );
    // The first frame's chase sync solves the camera after the save
    // systems ran, so the arming save starts on the second frame.
    frame(&mut world);
    frame(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Writing
    );

    world.entity_mut(player).insert(ShipCredits(410));
    world.resource_mut::<WorldSaveSession>().request_leave();
    assert!(!world.resource::<WorldSaveSession>().is_idle());
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    world.remove_resource::<WorldSaveSession>();
    let (_, _lock, header, state) = open_world(root.path(), "session", &catalog(digest)).unwrap();
    assert_eq!((header.generation, header.credits), (2, 410));
    assert_eq!(state.player.id.0, "player");
}

/// Once the world it armed with arms again from the seed - disarmed and
/// re-armed, or its scenario reloaded under the same config - the session
/// never writes: a crossing after the re-arm leaves the last good save as it
/// was, and a leave fails visibly.
#[test]
fn a_world_that_disarms_and_rearms_never_overwrites_its_save() {
    for scenario_reload in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut world, player) = armed_session(root.path());
        run_until_idle(&mut world);
        assert_eq!(
            world.resource::<WorldSaveSession>().status(),
            &WorldSaveStatus::Saved { generation: 1 }
        );
        let saved = std::fs::read(root.path().join("session/state.1.ron")).unwrap();

        world.entity_mut(player).insert(ShipCredits(410));
        if scenario_reload {
            world.resource_mut::<CurrentScenario>().set_changed();
        } else {
            let config = world
                .remove_resource::<WorldConfig<NovaLayeredWorld>>()
                .unwrap();
            frame(&mut world);
            world.insert_resource(config);
        }
        world.insert_resource(CurrentSector(SectorCoord::ORIGIN.offset(1, 0, 0)));
        for _ in 0..10 {
            frame(&mut world);
        }
        assert!(world.resource::<WorldSaveSession>().is_idle());
        assert_eq!(
            std::fs::read(root.path().join("session/state.1.ron")).unwrap(),
            saved,
            "scenario reload: {scenario_reload}"
        );
        assert_eq!(
            files(&root.path().join("session")),
            ["state.1.ron", "world.lock", "world.ron"]
        );

        world.resource_mut::<WorldSaveSession>().request_leave();
        run_until_idle(&mut world);
        assert!(matches!(
            world.resource::<WorldSaveSession>().status(),
            WorldSaveStatus::Failed(_)
        ));
        let digest = world.resource::<LoadedSectionPacks>().digest.0;
        world.remove_resource::<WorldSaveSession>();
        let (_, _lock, header, _) = open_world(root.path(), "session", &catalog(digest)).unwrap();
        assert_eq!((header.generation, header.credits), (1, 300));
    }
}

/// With no player ship there is nothing to save: a crossing writes nothing
/// and keeps the status, and a leave goes idle FAILED, never saved.
#[test]
fn with_no_player_a_crossing_writes_nothing_and_a_leave_fails() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    world.entity_mut(player).despawn();
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Unsaved
    );
    assert_eq!(files(&root.path().join("session")), ["world.lock"]);

    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert!(matches!(
        world.resource::<WorldSaveSession>().status(),
        WorldSaveStatus::Failed(_)
    ));
    assert_eq!(files(&root.path().join("session")), ["world.lock"]);
}

/// A leave save waits for a body that cannot freeze yet, counting only the
/// frames on which virtual time advanced, and fails visibly past the bound.
#[test]
fn a_leave_save_that_never_settles_fails_at_the_bound() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    run_until_idle(&mut world);
    world.entity_mut(player).insert(IntegrityDestroyMarker);
    world.resource_mut::<WorldSaveSession>().request_leave();
    let advance = |world: &mut World, by: Duration| {
        world.resource_mut::<Time<Virtual>>().advance_by(by);
        frame(world);
    };
    for _ in 0..100 {
        advance(&mut world, Duration::ZERO);
    }
    for _ in 0..600 {
        advance(&mut world, Duration::from_millis(16));
    }
    assert!(matches!(
        world.resource::<WorldSaveSession>().status(),
        WorldSaveStatus::Waiting(_)
    ));
    advance(&mut world, Duration::from_millis(16));
    let session = world.resource::<WorldSaveSession>();
    assert!(session.is_idle());
    let WorldSaveStatus::Failed(reason) = session.status() else {
        panic!(
            "the 601st advancing frame fails the save, not {:?}",
            session.status()
        );
    };
    assert!(reason.contains("did not settle"), "{reason}");
    assert_eq!(
        files(&root.path().join("session")),
        ["state.1.ron", "world.lock", "world.ron"],
        "the last good save is untouched"
    );
}

/// A wanted save waits for a player camera, then writes its solved pose once
/// the controller has produced one.
#[test]
fn a_save_waits_visibly_until_the_player_camera_is_ready() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    let camera = {
        let mut cameras = world.query_filtered::<Entity, With<SpaceshipCameraController>>();
        cameras.single(&world).expect("fixture camera")
    };
    world.entity_mut(camera).despawn();

    frame(&mut world);
    let WorldSaveStatus::Waiting(reason) = world.resource::<WorldSaveSession>().status() else {
        panic!(
            "a missing camera must wait, got {:?}",
            world.resource::<WorldSaveSession>().status()
        );
    };
    assert!(reason.contains("player camera"), "{reason}");
    assert_eq!(files(&root.path().join("session")), ["world.lock"]);

    let camera = world
        .spawn(SpaceshipCameraController)
        .with_child((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput::default(),
        ))
        .id();
    frame(&mut world);
    let camera_transform = *world
        .get::<Transform>(camera)
        .expect("solved camera transform");
    let ship_global = *world
        .get::<GlobalTransform>(player)
        .expect("fixture ship global transform");
    let ship_rotation = world
        .get::<Rotation>(player)
        .expect("fixture ship rotation")
        .0;
    let normal = {
        let mut rigs = world.query_filtered::<Entity, With<SpaceshipCameraNormalInputMarker>>();
        rigs.single(&world).expect("fixture Normal rig")
    };
    let normal_steer = **world
        .get::<PointRotationOutput>(normal)
        .expect("fixture Normal rig output");
    let expected_view = CameraView {
        position_from_ship: ship_global.rotation().inverse()
            * (camera_transform.translation - ship_global.translation()),
        rotation_from_ship: ship_global.rotation().inverse() * camera_transform.rotation,
        steer_from_ship: ship_rotation.inverse() * normal_steer,
        zoom: world.resource::<ChaseZoom>().manual(),
    };

    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 1 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    world.remove_resource::<WorldSaveSession>();
    let (_, _lock, _, saved) = open_world(root.path(), "session", &catalog(digest)).unwrap();
    assert_eq!(saved.player.view, expected_view);
}

/// A turret round in flight, saved with the owner `owner`.
fn round(owner: SavedOwner) -> FrozenRound {
    FrozenRound {
        flight: FrozenRoundFlight {
            translation: Vec3::new(3.0, 0.0, -10.0),
            rotation: Quat::IDENTITY,
            velocity: Vec3::new(0.0, 0.0, -80.0),
            damage: ProjectileDamage {
                amount: 20.0,
                power: 0.0,
                kind: DamageType::Kinetic,
            },
            allegiance: None,
            owner,
            rake_radius: None,
        },
        source: RoundSourceType::Turret { render_mesh: None },
    }
}

/// Spawn a turret round fired by `owner` as a Load spawns one, with 1.5 s
/// left of a 2 s lifetime.
fn spawn_round(world: &mut World, owner: Entity) -> Entity {
    let round = round(SavedOwner::Gone);
    let mut commands = world.commands();
    let entity = thaw_round(&mut commands, &round, owner);
    commands
        .entity(entity)
        .insert(resumed_lifetime(SavedLifetime {
            total: 2.0,
            remaining: 1.5,
        }));
    world.flush();
    entity
}

/// The world saved under `root` with one round in flight from its player,
/// Loaded by [`load`].
fn loaded_with_a_round(root: &Path) -> World {
    let (mut saved, player) = armed_session(root);
    spawn_round(&mut saved, player);
    saved.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut saved);
    assert_eq!(
        saved.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 1 }
    );
    let digest = saved.resource::<LoadedSectionPacks>().digest.0;
    drop(saved);
    let world = load(root, digest);
    assert_eq!(
        world.resource::<ResumedTransients>().transients.len(),
        1,
        "the save holds the round"
    );
    world
}

/// The world saved under `root`, Loaded into a fresh fixture whose window is
/// the origin sector alone. It is armed, as on the frame the scenario arms
/// it, and no sector is live.
fn load(root: &Path, digest: u64) -> World {
    let mut app = App::new();
    app.add_plugins(crate::test_support::WorldSaveTestPlugin);
    let mut world = std::mem::take(app.world_mut());
    crate::test_support::arm_save_fixture(&mut world);
    let mut config = world
        .remove_resource::<WorldConfig<NovaLayeredWorld>>()
        .unwrap();
    config.active_radius = 0;
    let (folder, lock, header, state) = open_world(root, "session", &catalog(digest)).unwrap();
    resume_world(&mut world, folder, lock, &header, state);
    world.insert_resource(config);
    restore_resumed_world(&mut world);
    world
}

/// The live turret rounds.
fn rounds(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<TurretBulletProjectileMarker>>()
        .iter(world)
        .collect()
}

/// The live torpedoes.
fn torpedoes(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
        .iter(world)
        .collect()
}

/// A torpedo in flight, saved with the owner `owner`, the launching bay
/// `section`, and tracking `target`.
fn torpedo(
    owner: SavedOwner,
    section: Option<SavedSectionRef>,
    target: SavedTorpedoTarget,
) -> FrozenTorpedo {
    FrozenTorpedo {
        config: TorpedoSectionConfig::default(),
        owner,
        section,
        translation: Vec3::new(1.0, 0.0, -5.0),
        rotation: Quat::IDENTITY,
        linear: Vec3::new(0.0, 0.0, -40.0),
        angular: Vec3::ZERO,
        allegiance: None,
        arming: TorpedoArming::new(0.5, 50.0, Vec3::ZERO, 0.0),
        cold: None,
        target,
        steering: Vec3::NEG_Z,
        weave: TorpedoWeave::new(0.0, 0.0, Vec3::NEG_Z, 0.0),
        ignited: true,
        controller_health: 10.0,
        thruster_health: 10.0,
    }
}

/// Spawn a torpedo fired by `owner` from bay `section`, as a Load spawns
/// one, with 3 s left of a 4 s lifetime and no target locked yet. The
/// caller inserts `TorpedoTargetEntity`/`TorpedoTargetChosen` itself for a
/// tracking fixture, the same way a live targeting system would have
/// before the save.
fn spawn_torpedo_fixture(world: &mut World, owner: Entity, section: Option<Entity>) -> Entity {
    let record = torpedo(SavedOwner::Gone, None, SavedTorpedoTarget::Unchosen);
    let mut commands = world.commands();
    let entity = thaw_torpedo(&mut commands, &record, owner, section);
    commands
        .entity(entity)
        .insert(resumed_lifetime(SavedLifetime {
            total: 4.0,
            remaining: 3.0,
        }));
    world.flush();
    entity
}

/// A Load holds the clocks while its window is not the exact desired set,
/// even with as many live sectors as desired ones, and spawns its saved
/// round, with its owner and remaining lifetime, on the frame the set is
/// live. The clocks run from that frame.
#[test]
fn a_load_holds_the_world_until_every_saved_sector_is_live() {
    let root = tempfile::tempdir().unwrap();
    let mut world = loaded_with_a_round(root.path());
    let held = |world: &World| {
        world
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume)
            && world.resource::<Time<Virtual>>().is_paused()
    };
    assert!(held(&world), "the Load holds the clocks from the start");
    assert_eq!(
        world.get_resource::<WorldResumeProgress>(),
        Some(&WorldResumeProgress {
            live: 0,
            desired: 0
        }),
        "the Loading screen has a line from the start"
    );

    frame(&mut world);
    assert!(held(&world));
    assert!(rounds(&mut world).is_empty());
    assert_eq!(
        world.get_resource::<WorldResumeProgress>(),
        Some(&WorldResumeProgress {
            live: 0,
            desired: 1
        })
    );

    // One live sector for one desired, but not the desired one.
    world.spawn(SectorRoot(SectorCoord::ORIGIN.offset(1, 0, 0)));
    frame(&mut world);
    assert!(
        held(&world),
        "a live count equal to the window is not the window"
    );
    assert!(rounds(&mut world).is_empty());

    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    frame(&mut world);
    assert!(!held(&world), "the clocks run once the window is live");
    assert!(world.get_resource::<WorldResumeProgress>().is_none());
    assert!(!world.contains_resource::<ResumedTransients>());
    let [round] = rounds(&mut world)[..] else {
        panic!("the saved round spawns once");
    };
    let player = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(&world)
        .unwrap();
    assert_eq!(
        world.get::<ProjectileOwner>(round).map(|owner| owner.0),
        Some(player),
        "the saved owner id resolves to the live player"
    );
    assert_eq!(
        SavedLifetime::of(&world, round),
        Some(SavedLifetime {
            total: 2.0,
            remaining: 1.5
        })
    );
}

/// The arming frame of a Load wants a save. While the saved round is not
/// back, that save waits and writes nothing, so no save without the round
/// replaces the one with it. After the restore the save writes the round.
#[test]
fn a_load_saves_nothing_until_its_transients_are_back() {
    let root = tempfile::tempdir().unwrap();
    let mut world = loaded_with_a_round(root.path());
    let folder = root.path().join("session");
    let saved = std::fs::read(folder.join("state.1.ron")).unwrap();
    for _ in 0..50 {
        world
            .resource_mut::<Time<Virtual>>()
            .advance_by(Duration::from_millis(16));
        frame(&mut world);
    }
    let WorldSaveStatus::Waiting(reason) = world.resource::<WorldSaveSession>().status() else {
        panic!(
            "the arming save waits, not {:?}",
            world.resource::<WorldSaveSession>().status()
        );
    };
    assert!(reason.contains("still resuming"), "{reason}");
    assert_eq!(files(&folder), ["state.1.ron", "world.lock", "world.ron"]);
    assert_eq!(std::fs::read(folder.join("state.1.ron")).unwrap(), saved);

    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    world.remove_resource::<WorldSaveSession>();
    let (_, _lock, header, state) = open_world(root.path(), "session", &catalog(digest)).unwrap();
    assert_eq!(header.generation, 2);
    assert_eq!(state.transients.len(), 1, "the new save keeps the round");
}

/// A save waits while a blast resolves, while a raking slug is mid-body,
/// and while a torpedo part is at zero health awaiting its destruction
/// markers, and writes once all three are done: the slug is in the record
/// and the blast and the torpedo are not.
#[test]
fn a_save_waits_out_a_live_blast_and_a_raking_slug() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    run_until_idle(&mut world);
    let folder = root.path().join("session");
    let saved_before_leave = std::fs::read(folder.join("state.1.ron")).unwrap();
    let blast = world
        .spawn((
            nova_blast(3.0, 100.0, DamageType::Explosive),
            Transform::from_xyz(0.0, 0.0, -20.0),
        ))
        .id();
    let slug = spawn_round(&mut world, player);
    world
        .entity_mut(slug)
        .remove::<TurretBulletProjectileMarker>();
    world
        .entity_mut(slug)
        .insert((RailgunSlugProjectileMarker, RoundRake::new(0.5)));
    // The sweep arms a rake when its tip strikes a body; this slug's tip is
    // in the player's hull. The field is private to the sweep.
    set_rake_armed(&mut world, slug, vec![(player, 0.1)]);
    world.resource_mut::<WorldSaveSession>().request_leave();
    let waiting = |world: &mut World| {
        frame(world);
        match world.resource::<WorldSaveSession>().status() {
            WorldSaveStatus::Waiting(reason) => reason.clone(),
            status => panic!("the save waits, not {status:?}"),
        }
    };

    assert!(waiting(&mut world).contains("a blast is resolving"));
    world.entity_mut(blast).despawn();
    assert!(waiting(&mut world).contains("a raking slug is mid-body"));
    set_rake_armed(&mut world, slug, Vec::new());

    // A controller part at zero, the window before its destruction markers
    // land (see `freeze_torpedo`'s `part_health`), waits the same way.
    let torpedo = spawn_torpedo_fixture(&mut world, player, None);
    let children: Vec<Entity> = world
        .get::<Children>(torpedo)
        .expect("thaw_torpedo spawns a controller and a thruster child")
        .iter()
        .collect();
    let controller = children
        .into_iter()
        .find(|&child| world.get::<TorpedoControllerMarker>(child).is_some())
        .expect("the torpedo has a controller child");
    world.get_mut::<Health>(controller).unwrap().current = 0.0;
    assert!(waiting(&mut world).contains("a torpedo part is being destroyed"));
    assert_eq!(
        files(&folder),
        ["state.1.ron", "world.lock", "world.ron"],
        "no new save while the torpedo part waits"
    );
    assert_eq!(
        std::fs::read(folder.join("state.1.ron")).unwrap(),
        saved_before_leave,
        "the last good save is untouched while the torpedo part waits"
    );
    world.entity_mut(torpedo).despawn();

    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    world.remove_resource::<WorldSaveSession>();
    let (_, _lock, _, state) = open_world(root.path(), "session", &catalog(digest)).unwrap();
    let [FrozenTransient {
        body: FrozenTransientType::Round(round),
        ..
    }] = &state.transients[..]
    else {
        panic!("the record holds the slug alone");
    };
    assert!(matches!(round.source, RoundSourceType::Railgun));
    assert_eq!(round.flight.rake_radius, Some(0.5));
}

/// Set the bodies `slug`'s rake is armed against, through its reflection.
fn set_rake_armed(world: &mut World, slug: Entity, armed: Vec<(Entity, f32)>) {
    let mut rake = world.get_mut::<RoundRake>(slug).unwrap();
    let bevy::reflect::ReflectMut::Struct(rake) = rake.reflect_mut() else {
        unreachable!("RoundRake is a struct");
    };
    *rake
        .field_mut("armed")
        .and_then(|field| field.try_downcast_mut::<Vec<(Entity, f32)>>())
        .expect("RoundRake reflects its armed bodies") = armed;
}

/// A saved round or torpedo keeps who fired it. A live shooter with an id
/// is that id and resolves to that ship on Load. A shooter that died is
/// `Gone` and thaws to a handle that names no entity, as the live round or
/// torpedo held - and a dead-owner torpedo's arming then reads that
/// launcher as gone the same way a live one does. A live shooter with no
/// id fails the save visibly and keeps the last good save. When two live
/// ships have the shooter's id, or two live sections of its ship have the
/// bay's id, the Load cannot tell which one fired: it is refused by name on
/// its first restore frame, nothing spawns, the clocks run, and the save on
/// disk is untouched.
#[test]
fn a_resumed_shot_still_belongs_to_its_shooter() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    let fallen = world.spawn(Name::new("Fallen Raider")).id();
    spawn_round(&mut world, player);
    spawn_round(&mut world, fallen);
    spawn_torpedo_fixture(&mut world, player, None);
    spawn_torpedo_fixture(&mut world, fallen, None);
    world.entity_mut(fallen).despawn();
    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 1 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    drop(world);

    let mut world = load(root.path(), digest);
    let mut saved: Vec<String> = world
        .resource::<ResumedTransients>()
        .transients
        .iter()
        .map(|transient| {
            let owner = match &transient.body {
                FrozenTransientType::Round(round) => &round.flight.owner,
                FrozenTransientType::Torpedo(torpedo) => &torpedo.owner,
                other => panic!("only rounds and torpedoes are in flight here, not {other:?}"),
            };
            match owner {
                SavedOwner::Ship(id) => id.0.clone(),
                SavedOwner::Gone => "gone".to_string(),
            }
        })
        .collect();
    saved.sort();
    assert_eq!(saved, ["gone", "gone", "player", "player"]);
    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    frame(&mut world);
    let player = world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(&world)
        .unwrap();
    let mut owners: Vec<Entity> = rounds(&mut world)
        .into_iter()
        .chain(torpedoes(&mut world))
        .map(|entity| world.get::<ProjectileOwner>(entity).unwrap().0)
        .collect();
    owners.sort();
    let mut expected = vec![player, Entity::PLACEHOLDER, player, Entity::PLACEHOLDER];
    expected.sort();
    assert_eq!(owners, expected);

    let dead_owner_torpedo = torpedoes(&mut world)
        .into_iter()
        .find(|&torpedo| world.get::<ProjectileOwner>(torpedo).unwrap().0 == Entity::PLACEHOLDER)
        .expect("the fallen owner's torpedo resumes with a placeholder owner");
    assert_eq!(
        world.get::<ProjectileOwner>(dead_owner_torpedo).unwrap().0,
        Entity::PLACEHOLDER
    );
    assert!(
        world.get_entity(Entity::PLACEHOLDER).is_err(),
        "update_torpedo_arming reads the launcher live off ProjectileOwner; \
         nothing lives at the placeholder handle, so it reads as gone the \
         same way TorpedoArming::tick treats a None launcher (see \
         nova_ship::sections::torpedo_section::mod::tests::\
         a_launcher_that_died_leaves_nothing_for_its_salvo_to_clear)"
    );

    run_until_idle(&mut world);
    let folder = root.path().join("session");
    let saved = std::fs::read(folder.join("state.2.ron")).unwrap();
    let drone = world.spawn(Name::new("Unnamed Drone")).id();
    spawn_round(&mut world, drone);
    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    let WorldSaveStatus::Failed(reason) = world.resource::<WorldSaveSession>().status() else {
        panic!(
            "a shooter with no id fails the save, not {:?}",
            world.resource::<WorldSaveSession>().status()
        );
    };
    assert!(
        reason.contains("owner 'Unnamed Drone' has no durable id"),
        "{reason}"
    );
    assert_eq!(files(&folder), ["state.2.ron", "world.lock", "world.ron"]);
    assert_eq!(std::fs::read(folder.join("state.2.ron")).unwrap(), saved);
    drop(world);

    let player_id = || EntityId("player".to_string());
    for (case, expected) in [
        ("ships", "2 live ships have the id 'player'"),
        (
            "sections",
            "2 live sections of the ship 'player' have the id 'bay'",
        ),
    ] {
        // Each Load reopens the folder, so the last one released its lock.
        let mut world = load(root.path(), digest);
        let player = world
            .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
            .single(&world)
            .unwrap();
        // The saved player has no sections, and a file whose bay ref names
        // none is refused at open. The records go to the held list instead,
        // past the open check, to reach the restore-time resolve.
        let body = if case == "ships" {
            world.spawn((SpaceshipRootMarker, player_id()));
            FrozenTransientType::Round(round(SavedOwner::Ship(player_id())))
        } else {
            world.spawn((EntityId("bay".to_string()), ChildOf(player)));
            world.spawn((EntityId("bay".to_string()), ChildOf(player)));
            FrozenTransientType::Torpedo(Box::new(torpedo(
                SavedOwner::Ship(player_id()),
                Some(SavedSectionRef {
                    ship: player_id(),
                    section: EntityId("bay".to_string()),
                }),
                SavedTorpedoTarget::DumbFire,
            )))
        };
        world
            .resource_mut::<ResumedTransients>()
            .transients
            .push(FrozenTransient {
                lifetime: SavedLifetime {
                    total: 2.0,
                    remaining: 1.5,
                },
                body,
            });
        world.spawn(SectorRoot(SectorCoord::ORIGIN));
        frame(&mut world);
        assert_eq!(
            world
                .get_resource::<WorldResumeRefused>()
                .map(|refused| refused.reason.as_str()),
            Some(expected),
            "{case}: refused on the first restore frame, not after the wait"
        );
        assert!(
            rounds(&mut world).is_empty() && torpedoes(&mut world).is_empty(),
            "{case}: nothing spawns"
        );
        assert!(
            !world
                .resource::<ClockFreeze>()
                .is_held_by(FreezeOwner::WorldResume)
                && !world.resource::<Time<Virtual>>().is_paused(),
            "{case}: the clocks run"
        );
        drop(world);
        assert_eq!(files(&folder), ["state.2.ron", "world.lock", "world.ron"]);
        assert_eq!(std::fs::read(folder.join("state.2.ron")).unwrap(), saved);
    }
}

/// A torpedo that tracks a moving severed wreck survives a save and Load
/// together: the wreck's minted `<ship>/wreck/<section>` id is the body the
/// record names, and the restore frame links the torpedo straight to the
/// wreck once both are back, on the same frame, with no extra tick.
///
/// This fixture streams no sectors, and the sever path that mints a wreck id
/// lives in `nova_ship`'s integrity systems with no public entry point this
/// crate can call to build one for real. The wreck here is a hand-spawned
/// stand-in - an `EntityId` of that shape, a `RigidBody` and a velocity -
/// for the live body a real sever and a real window would put in the
/// window; it is not a real wreck fragment.
#[test]
fn a_moving_wreck_and_the_torpedo_tracking_it_come_back_together() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    run_until_idle(&mut world);

    let wreck_id = "carrier/wreck/hull";
    let east = SectorCoord::new(1, 0, 0);
    world.insert_resource(ledger(&[(east, &[wreck_id])]));
    let wreck = world
        .spawn((
            EntityId(wreck_id.to_string()),
            RigidBody::Dynamic,
            Transform::from_xyz(10.0, 0.0, -30.0),
            LinearVelocity(Vec3::new(1.0, 0.0, 0.0)),
        ))
        .id();
    let torpedo = spawn_torpedo_fixture(&mut world, player, None);
    world
        .entity_mut(torpedo)
        .insert((TorpedoTargetChosen, TorpedoTargetEntity(wreck)));

    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    drop(world);

    let mut world = load(root.path(), digest);
    {
        let transients = &world.resource::<ResumedTransients>().transients;
        let [FrozenTransient {
            body: FrozenTransientType::Torpedo(frozen),
            ..
        }] = &transients[..]
        else {
            panic!("the record holds the torpedo alone");
        };
        match &frozen.target {
            SavedTorpedoTarget::Tracking {
                target: SavedTargetRef::Body(SavedBodyRef(id)),
                ..
            } => assert_eq!(id.0, wreck_id),
            other => panic!("expected a Tracking Body target, got {other:?}"),
        }
    }

    // The window streaming the wreck back in, by hand (see this test's doc).
    let wreck = world
        .spawn((
            EntityId(wreck_id.to_string()),
            RigidBody::Dynamic,
            Transform::from_xyz(11.0, 0.0, -30.0),
            LinearVelocity(Vec3::new(1.0, 0.0, 0.0)),
        ))
        .id();
    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    frame(&mut world);

    let [torpedo] = torpedoes(&mut world)[..] else {
        panic!("the saved torpedo spawns once");
    };
    assert_eq!(
        world.get::<TorpedoTargetEntity>(torpedo).map(|t| t.0),
        Some(wreck),
        "the restore frame links the torpedo to the wreck with no extra tick"
    );
}

/// A torpedo that tracks another saved torpedo keeps that link by the
/// tracked torpedo's index in the saved list, across the RON round trip,
/// and resumes tracking the respawned torpedo once the Load's window is
/// live. When the tracked torpedo then dies, a leave save writes the
/// tracker on its last known position without waiting for guidance.
#[test]
fn a_torpedo_tracking_a_torpedo_resumes_on_it() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    run_until_idle(&mut world);

    let b = spawn_torpedo_fixture(&mut world, player, None);
    let a = spawn_torpedo_fixture(&mut world, player, None);
    world
        .entity_mut(a)
        .insert((TorpedoTargetChosen, TorpedoTargetEntity(b)));

    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
    let digest = world.resource::<LoadedSectionPacks>().digest.0;
    drop(world);

    let mut world = load(root.path(), digest);
    {
        let transients = &world.resource::<ResumedTransients>().transients;
        assert_eq!(transients.len(), 2, "both torpedoes are saved");
        let tracking = transients.iter().find_map(|transient| {
            let FrozenTransientType::Torpedo(torpedo) = &transient.body else {
                panic!("only torpedoes are in flight here");
            };
            match &torpedo.target {
                SavedTorpedoTarget::Tracking {
                    target: SavedTargetRef::Transient(tracked),
                    ..
                } => Some(*tracked),
                _ => None,
            }
        });
        let Some(tracked) = tracking else {
            panic!("A's record must track B by index");
        };
        let FrozenTransientType::Torpedo(b_record) = &transients[tracked].body else {
            unreachable!("only torpedoes are in flight here");
        };
        assert!(
            matches!(b_record.target, SavedTorpedoTarget::Unchosen),
            "the index A names must be B, which tracks nothing itself"
        );
    }

    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    frame(&mut world);

    let live = torpedoes(&mut world);
    assert_eq!(live.len(), 2, "both torpedoes spawn");
    let a = live
        .iter()
        .copied()
        .find(|&t| world.get::<TorpedoTargetEntity>(t).is_some())
        .expect("A resolves its target to a live entity");
    let b = live
        .into_iter()
        .find(|&t| t != a)
        .expect("B is the other live torpedo");
    assert_eq!(
        world.get::<TorpedoTargetEntity>(a).map(|t| t.0),
        Some(b),
        "A's target resolves to the respawned B with no extra tick"
    );

    // B dies before guidance drops A's link. No guidance runs here, as none
    // runs under the pause gate while a leave save settles, so the save
    // cannot wait for that pass. It writes the state the pass leaves: A on
    // its last known position, not retargeted, and with its live pose.
    let last = Vec3::new(4.0, 0.0, -30.0);
    world.entity_mut(a).insert(TorpedoTargetPosition(last));
    world.despawn(b);
    let translation = world.get::<Transform>(a).unwrap().translation;
    let linear = world.get::<LinearVelocity>(a).unwrap().0;
    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert!(
        matches!(
            world.resource::<WorldSaveSession>().status(),
            WorldSaveStatus::Saved { .. }
        ),
        "the leave save does not wait for a dropped link: {:?}",
        world.resource::<WorldSaveSession>().status()
    );
    drop(world);

    let mut world = load(root.path(), digest);
    {
        let transients = &world.resource::<ResumedTransients>().transients;
        let [FrozenTransient {
            body: FrozenTransientType::Torpedo(record),
            ..
        }] = &transients[..]
        else {
            panic!("A alone is saved");
        };
        assert!(
            matches!(record.target, SavedTorpedoTarget::Frozen(position) if position == last),
            "A is saved on its last known position, got {:?}",
            record.target
        );
        assert_eq!(record.translation, translation);
        assert_eq!(record.linear, linear);
    }
    world.spawn(SectorRoot(SectorCoord::ORIGIN));
    frame(&mut world);
    let [a] = torpedoes(&mut world)[..] else {
        panic!("A spawns once");
    };
    assert!(world.get::<TorpedoTargetEntity>(a).is_none());
    assert_eq!(
        world.get::<TorpedoTargetPosition>(a).map(|p| p.0),
        Some(last)
    );
}

/// A Load whose shed fixture or detached piece wears a style the pinned
/// catalog does not have is refused by that style's name, before any
/// transient spawns, and the clocks run. A piece with no style on a game
/// with no render resources is refused by the missing resource's name.
#[test]
fn a_load_whose_piece_wears_an_unknown_style_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Session").unwrap();
    drop(lock);
    let zero = ron::to_string(&Vec3::ZERO).unwrap();
    let identity = ron::to_string(&Quat::IDENTITY).unwrap();
    let pose = ron::to_string(&Transform::IDENTITY).unwrap();
    let transient = |body: String| -> FrozenTransient {
        ron::from_str(&format!(
            "(lifetime: (total: 2.0, remaining: 1.5), body: {body})"
        ))
        .unwrap()
    };
    let shed = |style: &str| {
        transient(format!(
            "ShedFixture((fixture: (kind: Decor(name: \"vent\", model: \"vent.glb\", \
             collider_size: (1.0, 1.0, 1.0)), pose: {pose}, health: (current: 5.0, max: 5.0), \
             children: []), style: {style}, translation: {zero}, rotation: {identity}, \
             linear: {zero}, angular: {zero}))"
        ))
    };
    let piece = |style: &str| {
        transient(format!(
            "DetachedPiece((name: \"hull\", translation: {zero}, rotation: {identity}, \
             linear: {zero}, angular: {zero}, center_of_mass: {zero}, \
             collider: Cuboid(size: (1.0, 1.0, 1.0)), grace: None, style: {style}, nodes: []))"
        ))
    };
    let cases = [
        (
            shed("Some(\"lost_style\")"),
            "transient 1 wears the style 'lost_style', which is not loaded",
        ),
        (
            piece("Some(\"lost_style\")"),
            "transient 1 wears the style 'lost_style', which is not loaded",
        ),
        (
            piece("None"),
            "this game has no AssetServer to rebuild the saved transients with",
        ),
    ];
    for (generation, (record, expected)) in (1..).zip(cases) {
        let state = WorldSaveState {
            transients: vec![
                FrozenTransient {
                    lifetime: SavedLifetime {
                        total: 2.0,
                        remaining: 1.5,
                    },
                    body: FrozenTransientType::Round(round(SavedOwner::Gone)),
                },
                record,
            ],
            ..state(generation, 750)
        };
        write_world(&folder, &header("Session", generation), &state).unwrap();
        let mut world = load(root.path(), 7);
        world.spawn(SectorRoot(SectorCoord::ORIGIN));
        frame(&mut world);
        assert_eq!(
            world
                .get_resource::<WorldResumeRefused>()
                .map(|refused| refused.reason.as_str()),
            Some(expected),
            "case at generation {generation}"
        );
        assert!(
            rounds(&mut world).is_empty(),
            "no transient spawns with a refused one"
        );
        assert!(!world.contains_resource::<ResumedTransients>());
        assert!(
            !world
                .resource::<ClockFreeze>()
                .is_held_by(FreezeOwner::WorldResume)
                && !world.resource::<Time<Virtual>>().is_paused(),
            "the clocks run after a refusal"
        );
    }
}

/// A ledger whose cells each hold a bare frozen ship for each of their ids.
/// It is built through the record's RON form, which is the only way to make
/// a ledger body outside `nova_world`.
fn ledger(cells: &[(SectorCoord, &[&str])]) -> FrozenSectors {
    let ship = ron::to_string(&state(1, 0).player.ship).unwrap();
    let transform = ron::to_string(&Transform::IDENTITY).unwrap();
    let cells: Vec<String> = cells
        .iter()
        .map(|(coord, ids)| {
            let bodies: Vec<String> = ids
                .iter()
                .map(|id| {
                    format!(
                        "(id: Some({id:?}), name: None, transform: {transform}, \
                         visibility: None, motion: None, body: Ship({ship}))"
                    )
                })
                .collect();
            format!(
                "{}: Visited([{}])",
                ron::to_string(coord).unwrap(),
                bodies.join(", ")
            )
        })
        .collect();
    ron::from_str(&format!("{{{}}}", cells.join(", "))).unwrap()
}

/// A save whose ids break the rule fails visibly and writes nothing: two
/// bodies with one id across cells or against the player, an id with a `/`
/// that is not a minted wreck id, or a round whose owner the save does not
/// keep. A round whose owner died and a wreck of a wreck save.
#[test]
fn a_save_with_a_duplicate_id_fails_and_keeps_the_last_save() {
    let root = tempfile::tempdir().unwrap();
    let (mut world, player) = armed_session(root.path());
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 1 }
    );
    let folder = root.path().join("session");
    let saved = std::fs::read(folder.join("state.1.ron")).unwrap();
    let refused = |world: &mut World, expected: &str| {
        world.resource_mut::<WorldSaveSession>().request_leave();
        run_until_idle(world);
        let WorldSaveStatus::Failed(reason) = world.resource::<WorldSaveSession>().status() else {
            panic!(
                "'{expected}' fails the save, not {:?}",
                world.resource::<WorldSaveSession>().status()
            );
        };
        assert!(reason.contains(expected), "{reason}");
        assert_eq!(files(&folder), ["state.1.ron", "world.lock", "world.ron"]);
        assert_eq!(std::fs::read(folder.join("state.1.ron")).unwrap(), saved);
    };
    let east = SectorCoord::new(1, 0, 0);
    let south = SectorCoord::new(0, 0, -1);

    world.insert_resource(ledger(&[(east, &["player"])]));
    refused(&mut world, "two saved bodies have the id 'player'");
    world.insert_resource(ledger(&[(east, &["rock_1"]), (south, &["rock_1"])]));
    refused(&mut world, "two saved bodies have the id 'rock_1'");
    world.insert_resource(ledger(&[(east, &["rock_1/wreck/hull/plate"])]));
    refused(
        &mut world,
        "the id 'rock_1/wreck/hull/plate' has a '/' but is not a minted wreck id",
    );

    world.insert_resource(FrozenSectors::default());
    let ghost = world
        .spawn((Name::new("Ghost"), EntityId("ghost".to_string())))
        .id();
    let shot = spawn_round(&mut world, ghost);
    refused(&mut world, "transient 0: its owner 'ghost' is not saved");

    world.entity_mut(shot).despawn();
    let stray = world
        .spawn((Name::new("Stray"), EntityId("stray".to_string())))
        .id();
    let tracker = spawn_torpedo_fixture(&mut world, player, None);
    world
        .entity_mut(tracker)
        .insert((TorpedoTargetChosen, TorpedoTargetEntity(stray)));
    refused(&mut world, "transient 0: its target 'stray' is not saved");
    world.entity_mut(tracker).despawn();
    world.entity_mut(stray).despawn();

    world.entity_mut(ghost).despawn();
    spawn_round(&mut world, ghost);
    world.insert_resource(ledger(&[(east, &["rock_1/wreck/hull/wreck/plate"])]));
    world.resource_mut::<WorldSaveSession>().request_leave();
    run_until_idle(&mut world);
    assert_eq!(
        world.resource::<WorldSaveSession>().status(),
        &WorldSaveStatus::Saved { generation: 2 }
    );
}

/// A saved world whose ids break the rule is refused on open with the state
/// file and the reason. A round whose owner died, a round of the player and
/// a wreck of a wreck open.
#[test]
fn a_world_with_a_duplicate_id_is_refused_on_open() {
    let root = tempfile::tempdir().unwrap();
    let (folder, lock) = create_world(root.path(), "Ids").unwrap();
    drop(lock);
    let east = SectorCoord::new(1, 0, 0);
    let south = SectorCoord::new(0, 0, -1);
    let shot = |owner: SavedOwner| FrozenTransient {
        lifetime: SavedLifetime {
            total: 2.0,
            remaining: 1.5,
        },
        body: FrozenTransientType::Round(round(owner)),
    };
    let fish = |target: SavedTorpedoTarget, section: Option<SavedSectionRef>| FrozenTransient {
        lifetime: SavedLifetime {
            total: 4.0,
            remaining: 3.0,
        },
        body: FrozenTransientType::Torpedo(Box::new(torpedo(
            SavedOwner::Ship(EntityId("player".to_string())),
            section,
            target,
        ))),
    };
    let cases: [(FrozenSectors, Vec<FrozenTransient>, &str); 9] = [
        (
            ledger(&[(east, &["player"])]),
            Vec::new(),
            "two saved bodies have the id 'player'",
        ),
        (
            ledger(&[(east, &["rock_1"]), (south, &["rock_1"])]),
            Vec::new(),
            "two saved bodies have the id 'rock_1'",
        ),
        (
            ledger(&[(east, &["rock_1/wreck/hull/plate"])]),
            Vec::new(),
            "the id 'rock_1/wreck/hull/plate' has a '/' but is not a minted wreck id",
        ),
        (
            FrozenSectors::default(),
            vec![
                shot(SavedOwner::Gone),
                shot(SavedOwner::Ship(EntityId("ghost".to_string()))),
            ],
            "transient 1: its owner 'ghost' is not saved",
        ),
        (
            // A torpedo's Body target that names no saved body.
            FrozenSectors::default(),
            vec![fish(
                SavedTorpedoTarget::Tracking {
                    target: SavedTargetRef::Body(SavedBodyRef(EntityId(
                        "missing_body".to_string(),
                    ))),
                    last: None,
                },
                None,
            )],
            "transient 0: its target 'missing_body' is not saved",
        ),
        (
            // A bay ref whose ship is saved (the player) but whose section
            // is not: the bare fixture ship has no sections.
            FrozenSectors::default(),
            vec![fish(
                SavedTorpedoTarget::DumbFire,
                Some(SavedSectionRef {
                    ship: EntityId("player".to_string()),
                    section: EntityId("missing_section".to_string()),
                }),
            )],
            "transient 0: its bay 'missing_section' of the ship 'player' is not saved",
        ),
        (
            // A Canister target with no matching canister anywhere saved.
            FrozenSectors::default(),
            vec![fish(
                SavedTorpedoTarget::Tracking {
                    target: SavedTargetRef::Canister(CargoCanisterRuntimeId(7)),
                    last: None,
                },
                None,
            )],
            "transient 0: its target canister 7 is not saved",
        ),
        (
            // Transient(self): the only transient names its own index.
            FrozenSectors::default(),
            vec![fish(
                SavedTorpedoTarget::Tracking {
                    target: SavedTargetRef::Transient(0),
                    last: None,
                },
                None,
            )],
            "transient 0: its target transient 0 is not another of the 1 saved",
        ),
        (
            // Transient(out of range): the only transient names an index
            // past the one-element saved list.
            FrozenSectors::default(),
            vec![fish(
                SavedTorpedoTarget::Tracking {
                    target: SavedTargetRef::Transient(5),
                    last: None,
                },
                None,
            )],
            "transient 0: its target transient 5 is not another of the 1 saved",
        ),
    ];
    let refusal_count = cases.len() as u64;
    for (generation, (sectors, transients, expected)) in (1..).zip(cases) {
        let state = WorldSaveState {
            sectors,
            transients,
            ..state(generation, 750)
        };
        write_world(&folder, &header("Ids", generation), &state).unwrap();
        assert_eq!(
            open_world(root.path(), "ids", &catalog(7)).map(|_| ()),
            Err(WorldRefusal::Unreadable(format!(
                "state.{generation}.ron: {expected}"
            ))),
            "case at generation {generation}"
        );
    }

    // Open-time successes: a `Frozen` target is never checked against the
    // saved ids, and a `Body` target that names a ledger id that IS saved
    // resolves.
    let frozen_generation = refusal_count + 1;
    let frozen_target = WorldSaveState {
        sectors: FrozenSectors::default(),
        transients: vec![fish(
            SavedTorpedoTarget::Frozen(Vec3::new(0.0, 0.0, -50.0)),
            None,
        )],
        ..state(frozen_generation, 750)
    };
    write_world(&folder, &header("Ids", frozen_generation), &frozen_target).unwrap();
    {
        let (_, _lock, _, saved) = open_world(root.path(), "ids", &catalog(7)).unwrap();
        assert_eq!(saved.transients.len(), 1);
    }

    let saved_body_generation = frozen_generation + 1;
    let body_target_saved = WorldSaveState {
        sectors: ledger(&[(east, &["rock_1"])]),
        transients: vec![fish(
            SavedTorpedoTarget::Tracking {
                target: SavedTargetRef::Body(SavedBodyRef(EntityId("rock_1".to_string()))),
                last: None,
            },
            None,
        )],
        ..state(saved_body_generation, 750)
    };
    write_world(
        &folder,
        &header("Ids", saved_body_generation),
        &body_target_saved,
    )
    .unwrap();
    {
        let (_, _lock, _, saved) = open_world(root.path(), "ids", &catalog(7)).unwrap();
        assert_eq!(saved.transients.len(), 1);
    }

    let final_generation = saved_body_generation + 1;
    let state = WorldSaveState {
        sectors: ledger(&[(east, &["rock_1/wreck/hull/wreck/plate"])]),
        transients: vec![
            shot(SavedOwner::Gone),
            shot(SavedOwner::Ship(EntityId("player".to_string()))),
        ],
        ..state(final_generation, 750)
    };
    write_world(&folder, &header("Ids", final_generation), &state).unwrap();
    let (_, _lock, _, saved) = open_world(root.path(), "ids", &catalog(7)).unwrap();
    assert_eq!(saved.transients.len(), 2);
    assert_eq!(saved.sectors.len(), 1);
}
