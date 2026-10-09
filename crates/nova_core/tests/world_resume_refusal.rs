//! A Load whose window never fills is refused past
//! `WORLD_RESUME_SECONDS_MAX`, releasing the clocks instead of holding it
//! forever. Case one is an unfilled sector window; case two is a saved
//! owner id with no live ship.
//!
//! The rig is the production path (`NovaMenuPlugin`'s Retry pipeline,
//! `LoadingScreenPlugin`'s progress line, `nova_world_base`'s real restore)
//! through its `test_support` fixture, reaching the `pub(crate)` `WorldsRoot`
//! via `NOVA_CONFIG_ROOT`, the lever `NovaMenuPlugin::build` reads first.

use std::time::Duration;

use bevy::{
    prelude::*,
    state::app::StatesPlugin,
    time::{TimePlugin, TimeUpdateStrategy},
    ui_widgets::Activate,
};
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use nova_assets::prelude::LoadedSectionPacks;
use nova_core::loading_screen::LoadingScreenPlugin;
use nova_gameplay::prelude::{
    resumed_lifetime, AssetRef, ClockFreeze, DamageType, FreezeOwner, FrozenRoundFlight, GameMode,
    GameStates, PauseStates, PlayerSpaceshipMarker, ProjectileDamage, SavedLifetime, SavedOwner,
};
use nova_menu::prelude::NovaMenuPlugin;
use nova_scenario::prelude::{CurrentScenario, GameScenarios, ScenarioConfig};
use nova_ship::prelude::{thaw_round, FrozenRound, RoundSourceType};
use nova_world::prelude::{CurrentSector, SectorCoord, SectorRoot, WorldConfig};
use nova_world_base::{
    prelude::{
        create_world, open_world, NovaLayeredWorld, ResumedWorld, WorldResumeProgress,
        WorldResumeRefused, WorldSaveSession, WorldSaveState, WorldSaveStatus,
        WORLD_RESUME_SECONDS_MAX,
    },
    test_support::{arm_save_fixture, WorldSaveTestPlugin},
};

/// A scenario config with only the fields `CurrentScenario` needs: the Retry
/// path's "Pause Retry Button" only spawns over a `Some` scenario
/// (`nova_menu::pause::reconcile_pause_overlay`'s `live` check).
fn dummy_scenario(id: &str) -> ScenarioConfig {
    ScenarioConfig {
        description: "Test".to_string(),
        events: vec![],
        ..ScenarioConfig::new(id.to_string(), "Test".to_string(), AssetRef::default())
    }
}

fn find_named(app: &mut App, name: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Name)>();
    q.iter(app.world())
        .find(|(_, n)| n.as_str() == name)
        .map(|(e, _)| e)
}

fn all_texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

/// The same scan as [`all_texts`], but from `&App` so it can run inside an
/// [`update_until`] predicate.
fn texts_now(app: &App) -> Vec<String> {
    app.world()
        .iter_entities()
        .filter_map(|entity| entity.get::<Text>())
        .map(|text| text.0.clone())
        .collect()
}

fn press(app: &mut App, name: &str) {
    let entity = find_named(app, name).unwrap_or_else(|| panic!("no entity named '{name}'"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

/// Step frames with a tiny real-time tick, sleeping a little between them so
/// the save's background `IoTaskPool` write actually gets wall-clock time to
/// run (matches `nova_menu::tests::leave`'s own `update_until`).
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

fn session(app: &App) -> Option<&WorldSaveSession> {
    app.world().get_resource::<WorldSaveSession>()
}

fn game_state(app: &App) -> GameStates {
    app.world().resource::<State<GameStates>>().get().clone()
}

fn files_bytes(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = std::fs::read(entry.path()).unwrap();
            (name, bytes)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn a_load_that_cannot_fill_its_window_is_refused_and_releases_the_clocks() {
    let config_root = tempfile::tempdir().unwrap();
    // This binary runs exactly one `#[test]`, so mutating the env is not a race.
    std::env::set_var(nova_assets::storage::CONFIG_ROOT_ENV, config_root.path());
    let worlds_root = config_root.path().join("worlds");

    let mut app = App::new();
    app.add_plugins(TimePlugin);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )));
    app.add_plugins(StatesPlugin);
    // The menu backdrop's shared RNG entity; `nova_gameplay::plugin` adds this
    // in the production app, which this rig does not bring in whole.
    app.add_plugins(EntropyPlugin::<WyRand>::with_seed(42u64.to_ne_bytes()));
    app.init_state::<GameStates>();
    app.init_state::<PauseStates>();
    app.init_resource::<GameMode>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<ClockFreeze>();
    // `load_menu_ambience` (`OnEnter(MainMenu)`) reads this with a hard `Res`;
    // empty is a legal "no clean backdrop" catalog.
    app.init_resource::<GameScenarios>();

    app.add_plugins(LoadingScreenPlugin);
    app.add_plugins(NovaMenuPlugin);

    // Before the world arms: a scenario changing under an armed world is a
    // reload, and `reconcile_pause_overlay` only spawns "Pause Retry Button"
    // over a live scenario.
    app.insert_resource(CurrentScenario(Some(dummy_scenario("open_world"))));

    app.add_plugins(WorldSaveTestPlugin);
    arm_save_fixture(app.world_mut());
    // Captured before Retry removes it: `resume_world` always clears
    // `WorldConfig<NovaLayeredWorld>`, and the scenario loader that would
    // normally re-arm it is not part of this rig. See the re-arm below.
    let armed_config = app
        .world()
        .resource::<WorldConfig<NovaLayeredWorld>>()
        .clone();
    let packs = app.world().resource::<LoadedSectionPacks>().clone();

    let (folder, lock) = create_world(&worlds_root, "Resume").unwrap();
    let slug = folder.slug.clone();
    app.insert_resource(WorldSaveSession::created(
        folder,
        lock,
        "Resume".to_string(),
        42,
    ));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    app.update();
    update_until(&mut app, "the first save", |app| {
        session(app).is_some_and(|session| {
            session.is_idle() && *session.status() == WorldSaveStatus::Saved { generation: 1 }
        })
    });
    let before = files_bytes(&worlds_root.join(&slug));

    // ESC, then Retry ("Load last save"): the world was never listed by
    // `on_load_screen`, so this is exactly the New Game -> Retry gap D-T8
    // closes with a fresh re-read rather than a patch to a stale list.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
    }
    app.update();
    assert_eq!(pause_state(&app), PauseStates::Paused);
    assert!(
        find_named(&mut app, "Pause Retry Button").is_some(),
        "a live scenario offers Retry"
    );

    press(&mut app, "Pause Retry Button");
    // `PendingLeave` is private; `ResumedWorld` is the public signal that
    // `resume_world` ran, inserted first after it clears `WorldConfig`.
    update_until(&mut app, "the retry to resume the world", |app| {
        app.world().get_resource::<ResumedWorld>().is_some()
    });

    // Standing in for the scenario arm this rig has no loader for: a real
    // Load re-arms `WorldConfig` from the scenario (same seed, same
    // generator); here it is re-armed at `active_radius: 0` on purpose, so
    // the desired window is exactly the one sector (the origin) that this
    // rig never spawns a `SectorRoot` for - "a load that cannot fill its
    // window" without a generator or a streaming pipeline.
    app.world_mut().remove_resource::<ResumedWorld>();
    app.world_mut().insert_resource(WorldConfig {
        active_radius: 0,
        ..armed_config.clone()
    });
    app.update();
    // The loading screen's text animates a frame after `resume_world` fires
    // it, so wait for the line instead of assuming one `app.update()` covers both.
    update_until(&mut app, "the resume progress line to render", |app| {
        texts_now(app)
            .iter()
            .any(|text| text == "RESTORING SECTORS 0 / 1")
    });

    assert!(
        app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the Load holds the clocks while its window is not live"
    );
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    let progress = *app.world().resource::<WorldResumeProgress>();
    assert_eq!(progress.live, 0);
    assert_eq!(progress.desired, 1);

    // Comfortably under `WORLD_RESUME_SECONDS_MAX` (240 s): still held.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            WORLD_RESUME_SECONDS_MAX - 1.0,
        )));
    app.update();
    assert!(
        app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "still under the bound"
    );
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get_resource::<WorldResumeRefused>().is_none());

    // Past the bound: refused on this frame.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            2.0,
        )));
    app.update();
    // Still `Playing` this frame (`StateTransition` runs before `Update`):
    // proves `end_resume` released the clocks, not the later `OnExit` safety net.
    assert_eq!(game_state(&app), GameStates::Playing);
    assert!(
        !app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the refusal releases the clocks on the frame it happens, before `OnExit(Playing)`'s own safety net could"
    );
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());

    // The Load list rebuilds only once `LoadPanel` exists (`OnEnter(MainMenu)`
    // UI), so drive frames until the MainMenu re-entry settles.
    update_until(&mut app, "the MainMenu transition to settle", |app| {
        game_state(app) == GameStates::MainMenu
            && app.world().get_resource::<WorldSaveSession>().is_none()
    });
    update_until(&mut app, "the refused Load row to render", |app| {
        texts_now(app)
            .iter()
            .any(|text| text.contains("did not come back within 240"))
    });

    assert!(
        !app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the refusal releases the clocks"
    );
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get_resource::<WorldSaveSession>().is_none());
    assert!(app.world().get_resource::<WorldResumeRefused>().is_none());
    assert_eq!(game_state(&app), GameStates::MainMenu);

    let texts = all_texts(&mut app);
    assert!(
        !texts
            .iter()
            .any(|text| text.starts_with("RESTORING SECTORS")),
        "the progress line is empty once the resume ends"
    );
    assert!(
        app.world().get_resource::<WorldResumeProgress>().is_none(),
        "the progress resource is gone, which is what empties the line"
    );
    texts
        .iter()
        .find(|text| text.contains("did not come back within 240"))
        .unwrap_or_else(|| panic!("no Load row names the refusal; texts were {texts:?}"));

    // The lock is free: a fresh open succeeds (and is dropped at once).
    {
        let opened = open_world(&worlds_root, &slug, &packs);
        assert!(opened.is_ok(), "world.lock is free: {opened:?}");
    }

    assert_eq!(
        before,
        files_bytes(&worlds_root.join(&slug)),
        "nothing was written by the refused Load"
    );

    // === Case two: a saved owner id with no live ship ===
    //
    // The fixture player outlives case one (this rig has no `nova_world`
    // `Cleanup` to despawn it). It fires a round that saves with
    // `SavedOwner::Ship`, then the owner is despawned so the resumed window
    // cannot find it - the same rig gap as case one, on a ship id instead of
    // a sector.
    //
    // Case one left `TimeUpdateStrategy` at 2 s/frame to breach its bound;
    // reset it, or every setup frame below would breach case two's bound too.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )));

    let player = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .single(app.world())
        .expect("the fixture player is still alive");

    let flight = FrozenRoundFlight {
        translation: Vec3::new(3.0, 0.0, -10.0),
        rotation: Quat::IDENTITY,
        velocity: Vec3::new(0.0, 0.0, -80.0),
        damage: ProjectileDamage {
            amount: 20.0,
            power: 0.0,
            kind: DamageType::Kinetic,
        },
        allegiance: None,
        owner: SavedOwner::Gone,
        rake_radius: None,
    };
    let frozen_round = FrozenRound {
        flight,
        source: RoundSourceType::Turret { render_mesh: None },
    };
    {
        let mut commands = app.world_mut().commands();
        let round = thaw_round(&mut commands, &frozen_round, player);
        commands
            .entity(round)
            .insert(resumed_lifetime(SavedLifetime {
                total: 2.0,
                remaining: 1.5,
            }));
    }
    app.world_mut().flush();

    let (folder2, lock2) = create_world(&worlds_root, "Resume2").unwrap();
    let slug2 = folder2.slug.clone();
    app.insert_resource(WorldSaveSession::created(
        folder2,
        lock2,
        "Resume2".to_string(),
        42,
    ));
    // Re-insert to mark it changed: this is not the app's first update, so
    // `request_world_save` would otherwise never see a reason to save.
    app.world_mut()
        .insert_resource(CurrentSector(SectorCoord::ORIGIN));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    app.update();
    update_until(&mut app, "the second save", |app| {
        session(app).is_some_and(|session| {
            session.is_idle() && *session.status() == WorldSaveStatus::Saved { generation: 1 }
        })
    });

    let state_text = std::fs::read_to_string(worlds_root.join(&slug2).join("state.1.ron")).unwrap();
    let state: WorldSaveState = ron::from_str(&state_text).unwrap();
    assert_eq!(state.transients.len(), 1, "the save recorded the round");
    let before2 = files_bytes(&worlds_root.join(&slug2));

    // Standing in for the `Cleanup` teardown and the missing scenario loader:
    // the owner never comes back.
    app.world_mut().despawn(player);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Escape);
        keys.clear();
    }
    app.update();
    assert_eq!(pause_state(&app), PauseStates::Paused);
    assert!(
        find_named(&mut app, "Pause Retry Button").is_some(),
        "a live scenario offers Retry"
    );

    press(&mut app, "Pause Retry Button");
    update_until(&mut app, "the retry to resume the world", |app| {
        app.world().get_resource::<ResumedWorld>().is_some()
    });

    app.world_mut().remove_resource::<ResumedWorld>();
    app.world_mut().insert_resource(WorldConfig {
        active_radius: 0,
        ..armed_config
    });
    app.world_mut().spawn(SectorRoot(SectorCoord::ORIGIN));
    app.update();
    update_until(&mut app, "the resume progress line to render (case two)", |app| {
        texts_now(app)
            .iter()
            .any(|text| text == "RESTORING SECTORS 1 / 1")
    });

    assert!(
        app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the Load holds the clocks while the owner is not in the window"
    );
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    let progress = *app.world().resource::<WorldResumeProgress>();
    assert_eq!(progress.live, 1);
    assert_eq!(progress.desired, 1);

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            WORLD_RESUME_SECONDS_MAX - 1.0,
        )));
    app.update();
    assert!(
        app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "still under the bound"
    );
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get_resource::<WorldResumeRefused>().is_none());

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            2.0,
        )));
    app.update();
    // Still `Playing` this frame: proves `end_resume` released the clocks,
    // not the later `OnExit` safety net.
    assert_eq!(game_state(&app), GameStates::Playing);
    assert!(
        !app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the refusal releases the clocks on the frame it happens"
    );
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());

    update_until(
        &mut app,
        "the second MainMenu transition to settle",
        |app| {
            game_state(app) == GameStates::MainMenu
                && app.world().get_resource::<WorldSaveSession>().is_none()
        },
    );
    update_until(&mut app, "the refused Load row to name the owner", |app| {
        texts_now(app)
            .iter()
            .any(|text| text.contains("the ship 'player' is not in the window"))
    });

    assert!(
        !app.world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::WorldResume),
        "the refusal releases the clocks"
    );
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().get_resource::<WorldSaveSession>().is_none());
    assert!(app.world().get_resource::<WorldResumeRefused>().is_none());
    assert_eq!(game_state(&app), GameStates::MainMenu);

    let texts2 = all_texts(&mut app);
    assert!(
        !texts2
            .iter()
            .any(|text| text.starts_with("RESTORING SECTORS")),
        "the progress line is empty once the resume ends"
    );
    assert!(
        app.world().get_resource::<WorldResumeProgress>().is_none(),
        "the progress resource is gone, which is what empties the line"
    );
    texts2
        .iter()
        .find(|text| text.contains("the ship 'player' is not in the window"))
        .unwrap_or_else(|| panic!("no Load row names the owner; texts were {texts2:?}"));

    // The lock is free: a fresh open succeeds (and is dropped at once).
    {
        let opened = open_world(&worlds_root, &slug2, &packs);
        assert!(opened.is_ok(), "world.lock is free: {opened:?}");
    }

    assert_eq!(
        before2,
        files_bytes(&worlds_root.join(&slug2)),
        "nothing was written by the refused Load"
    );
}

fn pause_state(app: &App) -> PauseStates {
    *app.world().resource::<State<PauseStates>>().get()
}
