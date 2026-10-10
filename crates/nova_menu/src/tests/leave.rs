//! Leaving a saved world: every way out waits on the leave save, a failed
//! save leaves only on the player's consent, Load last save writes nothing,
//! and after a death the Defeat offers Load last save and every way out
//! leaves without a save. The world saves through the production save
//! systems on a fixture ship (`nova_world_base`'s `test-support`).

use std::{path::Path, sync::mpsc, time::Duration};

use bevy::{
    prelude::*, tasks::IoTaskPool, time::update_virtual_time, ui_widgets::Activate,
    window::WindowCloseRequested,
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::{
    thaw_torpedo, FrozenTorpedo, SavedTorpedoTarget, TorpedoArming, TorpedoSectionConfig,
    TorpedoTargetEntity, TorpedoWeave,
};
use nova_world_base::{
    prelude::{
        create_world, list_worlds, open_world, FrozenTransient, FrozenTransientType, WorldRefusal,
        WorldSaveSession, WorldSaveStatus,
    },
    test_support::{arm_save_fixture, WorldSaveTestPlugin},
};
use tempfile::TempDir;

use super::support::{
    app, clocks_paused, dummy_scenario, dummy_scenarios, enter_playing, find_named,
    observe_load_scenario, pause_state, press_escape, LoadedScenario,
};
use crate::{
    leave::{LeaveTarget, PendingLeave},
    world_setup::WorldsRoot,
};

/// A game in a world saved once, at its first frame, under a scratch
/// worlds folder. Each frame moves 16 ms of real time, and virtual time
/// follows unless something holds it. Returns the player ship.
fn saved_world() -> (App, TempDir, Entity) {
    let root = tempfile::tempdir().unwrap();
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(WorldsRoot(Some(root.path().to_path_buf())));
    app.init_resource::<Time>();
    app.add_systems(
        First,
        |mut real: ResMut<Time<Real>>, mut virt: ResMut<Time<Virtual>>, mut time: ResMut<Time>| {
            real.update_with_duration(Duration::from_millis(16));
            update_virtual_time(&mut time, &mut virt, &real);
        },
    );
    app.add_plugins(WorldSaveTestPlugin);
    // Before the world arms: a scenario changing under an armed world is a
    // reload, and a reload ends the session's run.
    app.insert_resource(CurrentScenario(Some(dummy_scenario("open_world").1)));
    observe_load_scenario(&mut app);
    let player = arm_save_fixture(app.world_mut());
    let (folder, lock) = create_world(root.path(), "Leave").unwrap();
    app.insert_resource(WorldSaveSession::created(
        folder,
        lock,
        "Leave".to_string(),
        42,
    ));
    enter_playing(&mut app);
    update_until(&mut app, "the first save", |app| {
        session(app).is_some_and(|session| {
            session.is_idle() && *session.status() == WorldSaveStatus::Saved { generation: 1 }
        })
    });
    (app, root, player)
}

fn session(app: &App) -> Option<&WorldSaveSession> {
    app.world().get_resource::<WorldSaveSession>()
}

fn update_until(app: &mut App, what: &str, done: impl Fn(&App) -> bool) {
    for _ in 0..2_000 {
        if done(app) {
            return;
        }
        app.update();
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("{what} never happened");
}

fn press(app: &mut App, name: &str) {
    let entity = find_named(app, name).unwrap_or_else(|| panic!("no entity named '{name}'"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn game_state(app: &App) -> GameStates {
    app.world().resource::<State<GameStates>>().get().clone()
}

fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The generation and credits the world's header on disk names. Read
/// through the list, which takes no lock, so it reads under a live session.
fn on_disk(app: &App, root: &Path) -> (u64, u32) {
    let listed = list_worlds(
        root,
        app.world()
            .resource::<nova_assets::prelude::LoadedSectionPacks>(),
    )
    .unwrap();
    let [world] = &listed[..] else {
        panic!("one world is saved, not {}", listed.len());
    };
    let header = world.header.as_ref().unwrap();
    (header.generation, header.credits)
}

/// Back to Main Menu waits for the leave save. While a body cannot freeze
/// the game stays paused and the player has no input, but virtual time runs,
/// so the body can settle. The state changes only once the leave save is on
/// disk. The pause gate stops torpedo guidance, so a torpedo whose target
/// died before guidance dropped the link does not hold the save: it is
/// saved on its last known position.
#[test]
fn leaving_a_saved_world_waits_for_the_leave_save_with_input_still_paused() {
    let (mut app, root, player) = saved_world();
    app.world_mut()
        .entity_mut(player)
        .insert((ShipCredits(410), IntegrityDestroyMarker));
    press_escape(&mut app);
    assert_eq!(clocks_paused(&app), (true, true));
    let last = Vec3::new(4.0, 0.0, -30.0);
    spawn_torpedo_on_a_dead_target(app.world_mut(), player, last);

    press(&mut app, "Back To Menu Button");
    let before = app.world().resource::<Time<Virtual>>().elapsed();
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(game_state(&app), GameStates::Playing);
    assert_eq!(pause_state(&app), PauseStates::Paused, "input stays off");
    assert_eq!(
        clocks_paused(&app),
        (false, false),
        "the pause hold is released"
    );
    assert!(
        app.world().resource::<Time<Virtual>>().elapsed() > before,
        "virtual time advances while the save waits"
    );
    assert!(matches!(
        session(&app).unwrap().status(),
        WorldSaveStatus::Waiting(_)
    ));
    assert!(find_named(&mut app, "Leave Overlay").is_some());
    assert!(find_named(&mut app, "Pause Overlay").is_none());
    assert_eq!(on_disk(&app, root.path()), (1, 300));

    app.world_mut()
        .entity_mut(player)
        .remove::<IntegrityDestroyMarker>();
    update_until(&mut app, "the way back to the menu", |app| {
        game_state(app) == GameStates::MainMenu
    });
    assert!(session(&app).is_none(), "the session and its lock are gone");
    assert_eq!(
        files(&root.path().join("leave")),
        ["state.2.ron", "world.lock", "world.ron"]
    );
    assert_eq!(on_disk(&app, root.path()), (2, 410));
    let (.., state) = open_world(
        root.path(),
        "leave",
        app.world()
            .resource::<nova_assets::prelude::LoadedSectionPacks>(),
        app.world().resource::<GameItems>(),
    )
    .unwrap();
    let [FrozenTransient {
        body: FrozenTransientType::Torpedo(torpedo),
        ..
    }] = &state.transients[..]
    else {
        panic!("the torpedo alone is saved");
    };
    assert!(
        matches!(torpedo.target, SavedTorpedoTarget::Frozen(position) if position == last),
        "the torpedo is saved on its last known position, got {:?}",
        torpedo.target
    );
}

/// Spawn a torpedo fired by `owner` that tracks a despawned target, as on
/// the frame its target died before guidance dropped the link. Its last
/// known target position is `last`.
fn spawn_torpedo_on_a_dead_target(world: &mut World, owner: Entity, last: Vec3) {
    let record = FrozenTorpedo {
        config: TorpedoSectionConfig::default(),
        owner: SavedOwner::Gone,
        section: None,
        translation: Vec3::new(1.0, 0.0, -5.0),
        rotation: Quat::IDENTITY,
        linear: Vec3::new(0.0, 0.0, -40.0),
        angular: Vec3::ZERO,
        allegiance: None,
        arming: TorpedoArming::new(0.5, 50.0, Vec3::ZERO, 0.0),
        cold: None,
        target: SavedTorpedoTarget::Frozen(last),
        steering: Vec3::NEG_Z,
        weave: TorpedoWeave::new(0.0, 0.0, Vec3::NEG_Z, 0.0),
        ignited: true,
        controller_health: 10.0,
        thruster_health: 10.0,
    };
    let dead = world.spawn_empty().id();
    world.despawn(dead);
    let mut commands = world.commands();
    let torpedo = thaw_torpedo(&mut commands, &record, owner, None);
    commands.entity(torpedo).insert((
        resumed_lifetime(SavedLifetime {
            total: 30.0,
            remaining: 30.0,
        }),
        TorpedoTargetEntity(dead),
    ));
    world.flush();
}

/// A leave save that fails holds the clock again and shows why. Try again
/// fails the same way; only Leave without saving goes, and the last good
/// save stays as it was.
#[test]
fn a_failed_leave_save_holds_the_clock_and_leaves_only_on_consent() {
    let (mut app, root, player) = saved_world();
    app.world_mut().entity_mut(player).insert(ShipCredits(410));
    // A folder where the next state file goes: its write fails.
    std::fs::create_dir(root.path().join("leave/state.2.ron")).unwrap();
    press_escape(&mut app);

    press(&mut app, "Back To Menu Button");
    let failed = |app: &App| {
        session(app).is_some_and(|session| {
            session.is_idle() && matches!(session.status(), WorldSaveStatus::Failed(_))
        })
    };
    update_until(&mut app, "the failed leave save", failed);
    app.update();
    assert_eq!(clocks_paused(&app), (true, true), "the hold is taken back");
    assert_eq!(game_state(&app), GameStates::Playing);
    assert!(find_named(&mut app, "Leave Without Saving Button").is_some());

    press(&mut app, "Leave Try Again Button");
    assert!(!failed(&app), "Try again asks for the leave save again");
    update_until(&mut app, "the second failed leave save", failed);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        game_state(&app),
        GameStates::Playing,
        "a failure never leaves"
    );

    press(&mut app, "Leave Without Saving Button");
    app.update();
    assert_eq!(game_state(&app), GameStates::MainMenu);
    assert!(session(&app).is_none());
    assert_eq!(on_disk(&app, root.path()), (1, 300));
}

/// In a saved world the pause Retry is Load last save: it writes nothing,
/// drops the session's lock, opens the world from disk again and reloads
/// the scenario around the saved ship.
#[test]
fn load_last_save_reopens_the_saved_world_without_writing() {
    let (mut app, root, player) = saved_world();
    app.world_mut().entity_mut(player).insert(ShipCredits(410));
    press_escape(&mut app);
    let retry = find_named(&mut app, "Pause Retry Button").unwrap();
    assert!(super::support::all_text(&mut app)
        .iter()
        .any(|text| text == "Load last save"));

    app.world_mut().trigger(Activate { entity: retry });
    update_until(&mut app, "the reopen", |app| {
        app.world().get_resource::<PendingLeave>().is_none()
    });
    for _ in 0..5 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some("open_world")
    );
    assert_eq!(pause_state(&app), PauseStates::Unpaused);
    assert_eq!(
        app.world().resource::<ResumedSpaceship>().id.0,
        "player",
        "the player spawn thaws the saved ship"
    );
    assert_eq!(on_disk(&app, root.path()), (1, 300));
    assert_eq!(
        session(&app).unwrap().status(),
        &WorldSaveStatus::Saved { generation: 1 }
    );
    assert_eq!(
        files(&root.path().join("leave")),
        ["state.1.ron", "world.lock", "world.ron"]
    );
}

/// A saved world whose player died after its first save, with the open
/// world's Defeat shown. The player had 410 credits when it died; the save
/// on disk holds 300.
///
/// The rig arms no world, so nothing disarms it on the death. A scenario
/// change spends the session's run the same way the disarm does
/// (`request_world_save`).
fn dead_saved_world() -> (App, TempDir) {
    let (mut app, root, player) = saved_world();
    app.world_mut().entity_mut(player).insert(ShipCredits(410));
    app.world_mut().despawn(player);
    app.world_mut()
        .resource_mut::<CurrentScenario>()
        .set_changed();
    app.init_resource::<CurrentOutcome>();
    app.init_resource::<NovaEventWorld>();
    app.world_mut().resource_mut::<CurrentOutcome>().0 = Some(OutcomeActionConfig::new(
        ScenarioOutcomeKind::Defeat,
        "Your ship was destroyed.",
    ));
    for _ in 0..5 {
        app.update();
    }
    (app, root)
}

/// A Defeat in a saved world offers Load last save beside Main Menu, and
/// [Enter] still goes to the menu. Load last save writes nothing and reopens
/// the world from disk.
#[test]
fn a_defeat_in_a_saved_world_offers_load_last_save() {
    let (mut app, root) = dead_saved_world();
    let texts = super::support::all_text(&mut app);
    for text in [
        "DEFEAT",
        "Your ship was destroyed.",
        "Load last save",
        "Main Menu",
        "[Enter] Main Menu",
    ] {
        assert!(texts.iter().any(|t| t == text), "{text:?} in {texts:?}");
    }

    press(&mut app, "Outcome Primary Button");
    update_until(&mut app, "the reopen", |app| {
        app.world().get_resource::<PendingLeave>().is_none()
    });

    assert_eq!(
        app.world().resource::<LoadedScenario>().0.as_deref(),
        Some("open_world")
    );
    assert_eq!(
        app.world().resource::<ResumedSpaceship>().id.0,
        "player",
        "the player spawn thaws the saved ship"
    );
    assert_eq!(on_disk(&app, root.path()), (1, 300));
    assert_eq!(
        files(&root.path().join("leave")),
        ["state.1.ron", "world.lock", "world.ron"]
    );
}

/// After a death the leave writes nothing and does not fail: Main Menu goes
/// to the menu once no write is in flight, and the last save stays.
#[test]
fn main_menu_after_a_death_leaves_without_a_save_failure() {
    let (mut app, root) = dead_saved_world();

    press(&mut app, "Outcome Menu Button");
    for _ in 0..20 {
        app.update();
    }

    let texts = super::support::all_text(&mut app);
    assert_eq!(game_state(&app), GameStates::MainMenu, "texts: {texts:?}");
    assert!(!texts.iter().any(|t| t == "SAVE FAILED"), "{texts:?}");
    assert!(session(&app).is_none(), "the session and its lock are gone");
    assert_eq!(on_disk(&app, root.path()), (1, 300));
    assert_eq!(
        files(&root.path().join("leave")),
        ["state.1.ron", "world.lock", "world.ron"]
    );
}

/// [Enter] over the Defeat goes to the menu without the leave flow. The
/// spent session is dropped on the way out, so its lock does not keep the
/// world from opening again.
#[test]
fn entering_the_menu_after_a_death_releases_the_world() {
    let (mut app, root) = dead_saved_world();

    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app.update();

    assert_eq!(game_state(&app), GameStates::MainMenu);
    assert!(session(&app).is_none(), "the session and its lock are gone");
    open_world(
        root.path(),
        "leave",
        app.world()
            .resource::<nova_assets::prelude::LoadedSectionPacks>(),
        app.world().resource::<GameItems>(),
    )
    .expect("the world opens again");
    assert_eq!(on_disk(&app, root.path()), (1, 300));
}

/// How long [`IoPoolHold`] waits for a held thread to start, and how long a
/// held thread waits for its release.
const IO_HOLD_MAX: Duration = Duration::from_secs(10);

/// Every `IoTaskPool` thread blocked, so a task spawned meanwhile stays
/// queued and cannot finish before the test looks. Dropping the hold, also
/// on a failed assertion, disconnects each thread's channel and releases it.
/// The pool is shared with the tests running beside this one, so a thread
/// also gives up after [`IO_HOLD_MAX`].
struct IoPoolHold(
    #[expect(dead_code, reason = "held, never read; dropping it releases")] Vec<mpsc::Sender<()>>,
);

impl IoPoolHold {
    fn new() -> Self {
        let pool = IoTaskPool::get();
        let (started, held) = mpsc::channel();
        let releases = (0..pool.thread_num())
            .map(|_| {
                let (release, waiting) = mpsc::channel::<()>();
                let started = started.clone();
                pool.spawn(async move {
                    // The test may have panicked and dropped the receiver.
                    let _ = started.send(());
                    let _ = waiting.recv_timeout(IO_HOLD_MAX);
                })
                .detach();
                release
            })
            .collect();
        let hold = Self(releases);
        for _ in 0..pool.thread_num() {
            held.recv_timeout(IO_HOLD_MAX)
                .expect("every IoTaskPool thread is held");
        }
        hold
    }
}

/// A death exit that drops the session while a write is in flight keeps the
/// world locked until the write ends, and the write lands whole. The write
/// is held queued, so the session drops before it can start.
#[test]
fn a_write_in_flight_keeps_the_world_locked_after_a_death_exit() {
    let (mut app, root, player) = saved_world();
    app.world_mut().entity_mut(player).insert(ShipCredits(410));
    let hold = IoPoolHold::new();
    app.world_mut()
        .resource_mut::<WorldSaveSession>()
        .request_leave();
    app.update();
    let mut session_mut = app.world_mut().resource_mut::<WorldSaveSession>();
    assert!(session_mut.is_writing(), "the leave save is in flight");
    session_mut.stop_saving();
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();

    assert_eq!(game_state(&app), GameStates::MainMenu);
    assert!(session(&app).is_none(), "the spent session is dropped");
    let reopen = |app: &App| {
        open_world(
            root.path(),
            "leave",
            app.world()
                .resource::<nova_assets::prelude::LoadedSectionPacks>(),
            app.world().resource::<GameItems>(),
        )
    };
    assert!(
        matches!(reopen(&app), Err(WorldRefusal::Locked)),
        "the write in flight holds the world"
    );
    drop(hold);
    update_until(&mut app, "the write in flight ends", |app| {
        reopen(app).is_ok()
    });
    assert_eq!(on_disk(&app, root.path()), (2, 410));
    assert_eq!(
        files(&root.path().join("leave")),
        ["state.2.ron", "world.lock", "world.ron"]
    );
}

/// The window's close button in a saved world is Exit: it pauses, waits for
/// the leave save, and quits only once the save is on disk. A second close
/// while it waits cannot cut it short.
#[test]
fn window_close_with_a_saved_world_waits_for_the_leave_save() {
    let (mut app, root, player) = saved_world();
    app.world_mut()
        .entity_mut(player)
        .insert((ShipCredits(410), IntegrityDestroyMarker));
    let window = app.world_mut().spawn_empty().id();
    // Read after every frame: a message lives two frames.
    let mut cursor = app.world().resource::<Messages<AppExit>>().get_cursor();
    let mut exits = 0;
    let mut frames = |app: &mut App, count: usize, exits: &mut usize| {
        for _ in 0..count {
            app.update();
            *exits += cursor
                .read(app.world().resource::<Messages<AppExit>>())
                .count();
            std::thread::sleep(Duration::from_millis(1));
        }
    };

    app.world_mut()
        .write_message(WindowCloseRequested { window });
    frames(&mut app, 5, &mut exits);
    assert_eq!(pause_state(&app), PauseStates::Paused);
    assert_eq!(
        app.world().resource::<PendingLeave>().target,
        LeaveTarget::Exit
    );
    app.world_mut()
        .write_message(WindowCloseRequested { window });
    frames(&mut app, 5, &mut exits);
    assert_eq!(exits, 0, "nothing quits while the save waits");
    assert_eq!(on_disk(&app, root.path()), (1, 300));

    app.world_mut()
        .entity_mut(player)
        .remove::<IntegrityDestroyMarker>();
    for _ in 0..2_000 {
        if exits > 0 {
            break;
        }
        frames(&mut app, 1, &mut exits);
    }
    assert_eq!(exits, 1, "one exit, after the save");
    assert_eq!(on_disk(&app, root.path()), (2, 410));
}
