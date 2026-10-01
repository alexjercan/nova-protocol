use bevy::time::TimeUpdateStrategy;

use super::*;

/// Seconds per frame, and per track travel: five frames a track.
const DT: f32 = 0.05;
const TRAVEL: f32 = 0.25;

fn emitter_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        SectionAnimationPlugin,
        MiningSectionPlugin { render: false },
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(DT),
    ));
    app
}

fn config() -> MiningSectionConfig {
    MiningSectionConfig {
        render_mesh: AssetRef::default(),
        render_mesh_transform: None,
        pulse_sound: AssetRef::from("pulse.wav"),
        door_open_sound: AssetRef::from("door_open.wav"),
        door_close_sound: AssetRef::from("door_close.wav"),
        reach: Meters(100.0),
        pulse_interval_seconds: 1.0,
        carve_radius_cells: 1.5,
    }
}

fn tracks() -> SectionAnimations {
    SectionAnimations::new(vec![
        SectionAnimation {
            cue: SectionAnimationCue::StowDoors,
            node_prefix: "stow_lid_".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(-0.22, 0.0, 0.0),
            },
            open_seconds: TRAVEL,
            close_seconds: TRAVEL,
        },
        SectionAnimation {
            cue: SectionAnimationCue::StowLift,
            node_prefix: "beam_tip".to_string(),
            motion: SectionAnimationMotion::Translate {
                offset: Vec3::new(0.0, 0.0, 0.16),
            },
            open_seconds: TRAVEL,
            close_seconds: TRAVEL,
        },
    ])
}

/// One frame of an emitter: door and tip progress and the deploy flag.
#[derive(Clone, Copy, Debug)]
struct Pose {
    doors: f32,
    tip: f32,
    deployed: bool,
}

fn pose(app: &App, emitter: Entity) -> Pose {
    let animations = app.world().get::<SectionAnimations>(emitter).unwrap();
    Pose {
        doors: animations
            .cue_progress(SectionAnimationCue::StowDoors)
            .unwrap(),
        tip: animations
            .cue_progress(SectionAnimationCue::StowLift)
            .unwrap(),
        deployed: app
            .world()
            .get::<MiningEmitter>(emitter)
            .unwrap()
            .is_deployed(),
    }
}

fn run(app: &mut App, emitter: Entity, frames: usize) -> Vec<Pose> {
    (0..frames)
        .map(|_| {
            app.update();
            pose(app, emitter)
        })
        .collect()
}

/// A live emitter starts stowed. Held, its doors open fully before the tip
/// leaves, and it reports deployed only with both out, one frame after the
/// tip lands. Released, it stops reporting deployed at once, and the tip is
/// fully in before the doors move. A press during the retraction sends the
/// tip back out past open doors.
#[test]
fn an_emitter_opens_doors_before_the_tip_and_retracts_in_reverse() {
    let mut app = emitter_app();
    let ship = app.world_mut().spawn_empty().id();
    let emitter = app
        .world_mut()
        .spawn((mining_section(config()), ChildOf(ship), tracks()))
        .id();

    for frame in run(&mut app, emitter, 10) {
        assert_eq!((frame.doors, frame.tip, frame.deployed), (1.0, 1.0, false));
    }

    app.world_mut().entity_mut(ship).insert(MiningHeld);
    let deploy = run(&mut app, emitter, 20);
    for frame in &deploy {
        assert!(frame.doors == 0.0 || frame.tip == 1.0, "{deploy:?}");
        if frame.deployed {
            assert_eq!((frame.doors, frame.tip), (0.0, 0.0), "{deploy:?}");
        }
    }
    assert!(deploy.last().unwrap().deployed, "{deploy:?}");

    app.world_mut().entity_mut(ship).remove::<MiningHeld>();
    let retract = run(&mut app, emitter, 3);
    for frame in &retract {
        assert!(!frame.deployed, "{retract:?}");
        assert_eq!(frame.doors, 0.0, "the doors moved before the tip was in");
    }
    let partway = retract.last().unwrap().tip;
    assert!(partway > 0.0 && partway < 1.0, "{retract:?}");

    app.world_mut().entity_mut(ship).insert(MiningHeld);
    let back = run(&mut app, emitter, 10);
    for frame in &back {
        assert_eq!(frame.doors, 0.0, "{back:?}");
    }
    assert!(back.last().unwrap().deployed, "{back:?}");

    app.world_mut().entity_mut(ship).remove::<MiningHeld>();
    let stow = run(&mut app, emitter, 20);
    for frame in &stow {
        assert!(frame.tip == 1.0 || frame.doors == 0.0, "{stow:?}");
        assert!(!frame.deployed);
    }
    let last = stow.last().unwrap();
    assert_eq!((last.doors, last.tip), (1.0, 1.0));
}

/// The door turns an emitter reported, as their `opening` flags.
#[derive(Resource, Default)]
struct DoorTurns(Vec<bool>);

/// The spawn snap and an idle emitter report nothing. A press reports one
/// opening however long the key stays held, and a release while the doors
/// are still parting reports one closing.
#[test]
fn an_emitter_reports_a_door_turn_only_when_the_door_target_changes() {
    let mut app = emitter_app();
    app.init_resource::<DoorTurns>();
    app.add_observer(
        |moved: On<MiningDoorsMoved>, mut turns: ResMut<DoorTurns>| {
            turns.0.push(moved.opening);
        },
    );
    let ship = app.world_mut().spawn_empty().id();
    let emitter = app
        .world_mut()
        .spawn((mining_section(config()), ChildOf(ship), tracks()))
        .id();

    run(&mut app, emitter, 10);
    assert_eq!(app.world().resource::<DoorTurns>().0, Vec::<bool>::new());

    app.world_mut().entity_mut(ship).insert(MiningHeld);
    let parting = run(&mut app, emitter, 3);
    assert_eq!(app.world().resource::<DoorTurns>().0, vec![true]);
    let doors = parting.last().unwrap().doors;
    assert!(doors > 0.0 && doors < 1.0, "{parting:?}");

    app.world_mut().entity_mut(ship).remove::<MiningHeld>();
    run(&mut app, emitter, 10);
    assert_eq!(app.world().resource::<DoorTurns>().0, vec![true, false]);
}

/// A stat that is not finite and positive is refused by name.
#[test]
fn a_mining_config_refuses_a_non_positive_or_non_finite_stat() {
    assert_eq!(config().validate(), Ok(()));
    for (field, broken) in [
        (
            "reach",
            MiningSectionConfig {
                reach: Meters(0.0),
                ..config()
            },
        ),
        (
            "pulse_interval_seconds",
            MiningSectionConfig {
                pulse_interval_seconds: f32::NAN,
                ..config()
            },
        ),
        (
            "carve_radius_cells",
            MiningSectionConfig {
                carve_radius_cells: -1.5,
                ..config()
            },
        ),
    ] {
        assert_eq!(broken.validate().map_err(|fault| fault.field), Err(field));
    }
}
