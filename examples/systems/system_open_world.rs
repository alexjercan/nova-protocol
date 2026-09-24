//! system_open_world: New Game into the open world and back, driven the way a
//! player drives it.
//!
//! Boots the shipped app (via [`editor_app`]), clicks New Game, reads the seed
//! the world setup modal drew, clicks Create, and walks the session: the line
//! warship, the world armed around it, the sectors streaming in, each bound
//! weapon fired, a Retry, and the way back to the menu.
//!
//! ONE SUBJECT: the open world's session lifecycle. What a sector CONTAINS is
//! `system_world_sectors`' range; this one holds the edges the base world adds
//! over it. The world arms on the session's seed around exactly one player
//! ship, Retry streams the same seed again, and nothing of the world is left
//! armed or standing once a scenario of any other role owns the screen.
//!
//! Two claims are read every frame, not once at the end, because each is an
//! ORDER: a frame that ends with the world armed ran `nova_world`'s Observe
//! stage in that same frame, and a frame that ends on any other scenario holds
//! no config and no sector root. A config written too late, or a teardown that
//! lands a frame after the menu backdrop, shows up as that frame.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_open_world --features debug
//! # look for: `open_world: Create started seed <n>`,
//! #           `open_world: every bound weapon fired`,
//! #           `open_world: PASS the world disarmed before the menu came back`,
//! #           `autopilot: cycle complete, no panic`
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
#[cfg(feature = "debug")]
use nova_input::prelude::InputSource;
use nova_protocol::prelude::*;
#[cfg(feature = "debug")]
use nova_ui::widget::TextFieldValue;
#[cfg(feature = "debug")]
use nova_world::prelude::{NovaWorldSystems, SectorRoot, WorldConfig, WorldObserver};

#[derive(Parser)]
#[command(name = "system_open_world")]
#[command(version = "1.0.0")]
#[command(
    about = "New Game into the open world: the seed modal, the warship and its weapons, the streamed sectors, Retry and the way back to the menu. Autopilot-only correctness range - play the game to fly the world",
    long_about = None
)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // No frame-time claim: the walk pauses for Retry and for the way out,
        // and this range holds orders and counts, not milliseconds.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(WorldWatchPlugin);
        app.add_plugins(open_world_script());
    }

    app.run()
}

/// The main menu's way in.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";
/// The world setup modal's seed field and its start button.
#[cfg(feature = "debug")]
const SEED_FIELD: &str = "World Seed Field";
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";
/// The pause overlay's two ways on.
#[cfg(feature = "debug")]
const RETRY_BUTTON: &str = "Pause Retry Button";
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";

/// How many cells the open world keeps live: radius 2 is a 5x5x5 window.
#[cfg(feature = "debug")]
const LIVE_SECTORS: usize = 125;

/// Seconds a load or a stream gets on a software-rendered CI GPU. Under the
/// harness completion deadline, so a stall names its beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// What the range watches over the whole run.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug)]
struct WorldWatch {
    /// The seed the modal showed when Create was pressed.
    seed: Option<u32>,
    /// Whether `nova_world`'s Observe stage ran this frame. Set by the stage,
    /// taken by the end-of-frame watch.
    observed: bool,
    /// Frames that ended with the world armed.
    armed_frames: u32,
    /// Frames that ended on a scenario of another role, or on none.
    off_world_frames: u32,
    /// Rounds, lance shots and torpedoes the warship put out.
    rounds: u32,
    lance_shots: u32,
    torpedoes: u32,
    /// The player ship the first session spawned, so Retry can be told apart.
    first_player: Option<Entity>,
    /// The inputs the script holds this frame, and the ones it held last.
    held: Held,
    applied: Held,
}

/// Inputs re-applied every frame by [`hold_inputs`].
#[cfg(feature = "debug")]
#[derive(Default, Debug, Clone, Copy)]
struct Held {
    combat: bool,
    trigger: bool,
    lance: bool,
    torpedo: bool,
}

/// The watch, wired to the stages and events it reads.
#[cfg(feature = "debug")]
struct WorldWatchPlugin;

#[cfg(feature = "debug")]
impl Plugin for WorldWatchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldWatch>();
        // In the Observe set, so it inherits that set's run conditions and
        // runs exactly on the frames the stage does.
        app.add_systems(Update, mark_observed.in_set(NovaWorldSystems::Observe));
        app.add_systems(Last, watch_world);
        app.add_observer(
            |_: On<Add, TurretBulletProjectileMarker>, mut watch: ResMut<WorldWatch>| {
                watch.rounds += 1;
            },
        );
        app.add_observer(|_: On<RailgunFired>, mut watch: ResMut<WorldWatch>| {
            watch.lance_shots += 1;
        });
        app.add_observer(
            |_: On<Add, TorpedoProjectileMarker>, mut watch: ResMut<WorldWatch>| {
                watch.torpedoes += 1;
            },
        );
    }
}

#[cfg(feature = "debug")]
fn mark_observed(mut watch: ResMut<WorldWatch>) {
    watch.observed = true;
}

/// The two per-frame orders.
#[cfg(feature = "debug")]
fn watch_world(
    current: Res<CurrentScenario>,
    config: Option<Res<WorldConfig<NovaLayeredWorld>>>,
    roots: Query<(), With<SectorRoot>>,
    mut watch: ResMut<WorldWatch>,
) {
    let open = current
        .0
        .as_ref()
        .is_some_and(|scenario| scenario.role == ScenarioRole::OpenWorld);
    if config.is_some() {
        assert!(
            open,
            "open_world: the world is armed over {:?}, which is not the open world",
            current.0.as_ref().map(|scenario| &scenario.id)
        );
        assert!(
            watch.observed,
            "open_world: a frame ended armed without running the Observe stage; the config \
             landed after it"
        );
        watch.armed_frames += 1;
    }
    watch.observed = false;
    if !open {
        assert!(
            config.is_none(),
            "open_world: the world stayed armed into {:?}",
            current.0.as_ref().map(|scenario| &scenario.id)
        );
        assert!(
            roots.is_empty(),
            "open_world: {} sector roots outlived the open world into {:?}",
            roots.iter().count(),
            current.0.as_ref().map(|scenario| &scenario.id)
        );
        watch.off_world_frames += 1;
    }
}

/// Re-apply the held inputs, and release each one on the frame it is let go.
///
/// The script's `input` hook, which runs in `PreUpdate` after `InputSystems`:
/// the only slot where a synthesized press is still `just_pressed` when the
/// game's input systems read it (see `system_turret_gunnery`'s `hold_inputs`).
/// Releases only on the edge, so the harness's own pointer clicks are left
/// alone.
#[cfg(feature = "debug")]
fn hold_inputs(world: &mut World, _elapsed: f32, _frame: u32) {
    let watch = world.resource::<WorldWatch>();
    let (held, was) = (watch.held, watch.applied);
    world.resource_mut::<WorldWatch>().applied = held;

    if held.combat {
        drive_action(world, "combat_stance", InputPhase::Press);
    } else if was.combat {
        drive_action(world, "combat_stance", InputPhase::Release);
    }
    let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
    if held.trigger {
        mouse.press(MouseButton::Left);
    } else if was.trigger {
        mouse.release(MouseButton::Left);
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    for (key, down, was_down) in [
        (KeyCode::KeyR, held.lance, was.lance),
        (KeyCode::KeyF, held.torpedo, was.torpedo),
    ] {
        if down {
            keys.press(key);
        } else if was_down {
            keys.release(key);
        }
    }
}

/// Set the held inputs from a step.
#[cfg(feature = "debug")]
fn hold(held: Held) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| world.resource_mut::<WorldWatch>().held = held
}

/// The one player ship, if exactly one stands.
#[cfg(feature = "debug")]
fn the_player(world: &World) -> Option<Entity> {
    let mut players = world.try_query_filtered::<Entity, With<PlayerSpaceshipMarker>>()?;
    let mut players = players.iter(world);
    let player = players.next()?;
    players.next().is_none().then_some(player)
}

#[cfg(feature = "debug")]
fn count<M: Component>(world: &World) -> usize {
    world
        .try_query_filtered::<(), With<M>>()
        .map_or(0, |mut q| q.iter(world).count())
}

/// The text in the node called `name`'s field.
#[cfg(feature = "debug")]
fn field_text(world: &mut World, name: &str) -> String {
    let mut fields = world.query::<(&Name, &TextFieldValue)>();
    fields
        .iter(world)
        .find(|(found, _)| found.as_str() == name)
        .map(|(_, value)| value.0.clone())
        .unwrap_or_else(|| panic!("open_world: no field named '{name}'"))
}

/// Advance once one player ship stands in an armed world, other than
/// `before`.
#[cfg(feature = "debug")]
fn armed_around_a_player(
    before: Option<Entity>,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(move |world: &World| {
        world.contains_resource::<WorldConfig<NovaLayeredWorld>>()
            && the_player(world).is_some_and(|player| Some(player) != before)
    })
}

/// Advance once the whole live window stands.
#[cfg(feature = "debug")]
fn the_window_streamed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(|world: &World| count::<SectorRoot>(world) == LIVE_SECTORS)
}

#[cfg(feature = "debug")]
fn watched(
    read: fn(&WorldWatch) -> u32,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<WorldWatch>(move |watch| read(watch) > 0)
}

/// The walk, one beat per gesture.
#[cfg(feature = "debug")]
fn open_world_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .input(hold_inputs)
        .step("open_world: reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(SESSION_SECS)
        .add()
        .click_named(
            "open_world: click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("open_world: the modal is up over the menu")
        .on_enter(|world: &mut World| {
            assert_eq!(
                *world.resource::<State<GameStates>>().get(),
                GameStates::MainMenu,
                "New Game must open the modal, not leave the menu"
            );
            let text = field_text(world, SEED_FIELD);
            let seed = text
                .parse::<u32>()
                .unwrap_or_else(|_| panic!("open_world: the modal drew '{text}', not a seed"));
            world.resource_mut::<WorldWatch>().seed = Some(seed);
            nova_probe::probe_marker(
                world,
                "outcome: new game opens the world setup modal without leaving the menu",
                serde_json::json!({ "seed": seed }),
            );
        })
        .add()
        .click_named(
            "open_world: click Create",
            CREATE_WORLD_BUTTON,
            state_is(GameStates::Playing),
            SESSION_SECS,
        )
        .step("open_world: the world arms around the player")
        .until(armed_around_a_player(None))
        .deadline(SESSION_SECS)
        .add()
        .step("open_world: Create started the modal's seed")
        .on_enter(assert_the_session)
        .add()
        .step("open_world: the warship carries its bound weapons")
        .on_enter(assert_the_warship)
        .add()
        .step("open_world: the sectors stream in")
        .until(the_window_streamed())
        .deadline(SESSION_SECS)
        .add()
        .step("open_world: report the stream")
        .on_enter(|world: &mut World| {
            nova_probe::probe_marker(
                world,
                "outcome: the sectors around the player stream in",
                serde_json::json!({ "roots": count::<SectorRoot>(world) }),
            );
            info!("open_world: {LIVE_SECTORS} sectors stand around the warship");
        })
        .add()
        // Each weapon on its own binding, under the combat stance the weapons
        // safety gates on.
        .step("open_world: raise the combat stance")
        .on_enter(hold(Held {
            combat: true,
            ..default()
        }))
        .until(std::sync::Arc::new(|world: &World| {
            the_player(world)
                .and_then(|player| world.get::<WeaponsHot>(player))
                .is_some_and(|hot| hot.0)
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: the trigger fires the point defense")
        .on_enter(hold(Held {
            combat: true,
            trigger: true,
            ..default()
        }))
        .until(watched(|watch| watch.rounds))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: R fires the lance")
        .on_enter(hold(Held {
            combat: true,
            lance: true,
            ..default()
        }))
        .until(watched(|watch| watch.lance_shots))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: F launches a torpedo")
        .on_enter(hold(Held {
            combat: true,
            torpedo: true,
            ..default()
        }))
        .until(watched(|watch| watch.torpedoes))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: report the weapons")
        .on_enter(|world: &mut World| {
            hold(Held::default())(world);
            let watch = world.resource::<WorldWatch>();
            let fired = serde_json::json!({
                "rounds": watch.rounds,
                "lance_shots": watch.lance_shots,
                "torpedoes": watch.torpedoes,
            });
            nova_probe::probe_marker(
                world,
                "outcome: each bound weapon fires from the warship",
                fired,
            );
            info!("open_world: every bound weapon fired");
        })
        .add()
        // Retry, through the pause overlay.
        .step("open_world: press ESC")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(RETRY_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: let ESC go")
        .on_enter(|world: &mut World| {
            release_key(KeyCode::Escape)(world);
            let player = the_player(world);
            world.resource_mut::<WorldWatch>().first_player = player;
        })
        .add()
        .click_named(
            "open_world: click Retry",
            RETRY_BUTTON,
            pointer_released(),
            BEAT_DEADLINE_SECS,
        )
        .step("open_world: the retried world arms around a new warship")
        .until(std::sync::Arc::new(|world: &World| {
            let before = world.resource::<WorldWatch>().first_player;
            armed_around_a_player(before)(world)
        }))
        .deadline(SESSION_SECS)
        .add()
        .step("open_world: Retry kept the seed")
        .on_enter(|world: &mut World| {
            let seed = world.resource::<WorldWatch>().seed;
            let session = world.resource::<OpenWorldSession>().seed;
            let armed = world.resource::<WorldConfig<NovaLayeredWorld>>().seed;
            assert_eq!(Some(session), seed, "Retry must keep the session's seed");
            assert_eq!(
                armed, session,
                "the retried world must stream the same seed"
            );
            nova_probe::probe_marker(
                world,
                "outcome: retry keeps the world seed",
                serde_json::json!({ "seed": armed }),
            );
        })
        .add()
        // And out, to a scenario of another role.
        .step("open_world: press ESC again")
        .on_enter(press_key(KeyCode::Escape))
        .until(ui_node_present(BACK_TO_MENU_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("open_world: let ESC go again")
        .on_enter(release_key(KeyCode::Escape))
        .add()
        .click_named(
            "open_world: click Back to Main Menu",
            BACK_TO_MENU_BUTTON,
            ui_node_present(NEW_GAME_BUTTON),
            SESSION_SECS,
        )
        .step("open_world: the menu came back on a disarmed world")
        .on_enter(assert_the_world_disarmed)
        .add()
}

/// The session Create inserted is the seed the modal showed, and the world
/// armed on it around the one player ship.
#[cfg(feature = "debug")]
fn assert_the_session(world: &mut World) {
    let seed = world.resource::<WorldWatch>().seed;
    let session = world.resource::<OpenWorldSession>().seed;
    let current = world
        .resource::<CurrentScenario>()
        .0
        .as_ref()
        .map(|scenario| scenario.id.clone());
    assert_eq!(Some(session), seed, "Create must start the modal's seed");
    assert_eq!(current.as_deref(), Some(OPEN_WORLD_SCENARIO_ID));
    nova_probe::probe_marker(
        world,
        "outcome: create starts the open world on the modal's seed",
        serde_json::json!({ "seed": session }),
    );
    info!("open_world: Create started seed {session}");

    let player = the_player(world).expect("open_world: exactly one player ship");
    let armed = world.resource::<WorldConfig<NovaLayeredWorld>>().seed;
    assert_eq!(armed, session, "the world must arm on the session's seed");
    assert!(
        world.get::<WorldObserver>(player).is_some(),
        "the world must stream around the player ship"
    );
    assert_eq!(count::<WorldObserver>(world), 1);
    nova_probe::probe_marker(
        world,
        "outcome: the world arms on the session seed around the one player ship",
        serde_json::json!({ "seed": armed }),
    );
}

/// One line warship, with six mounts on the trigger, the lance on R and both
/// bays on F.
#[cfg(feature = "debug")]
fn assert_the_warship(world: &mut World) {
    let player = the_player(world).expect("open_world: exactly one player ship");
    let trigger = [
        InputSource::Mouse(MouseButton::Left),
        InputSource::Gamepad(GamepadButton::RightTrigger2),
    ];
    let of_player = |parent: &ChildOf| parent.parent() == player;

    let mut turrets = world.query::<(&ChildOf, &SpaceshipTurretInputBinding)>();
    let turrets: Vec<_> = turrets
        .iter(world)
        .filter(|(parent, _)| of_player(parent))
        .map(|(_, binding)| binding.0.clone())
        .collect();
    let mut lances = world.query::<(&ChildOf, &SpaceshipRailgunInputBinding)>();
    let lances: Vec<_> = lances
        .iter(world)
        .filter(|(parent, _)| of_player(parent))
        .map(|(_, binding)| binding.0.clone())
        .collect();
    let mut bays = world.query::<(&ChildOf, &SpaceshipTorpedoInputBinding)>();
    let bays: Vec<_> = bays
        .iter(world)
        .filter(|(parent, _)| of_player(parent))
        .map(|(_, binding)| binding.0.clone())
        .collect();

    assert_eq!(turrets.len(), 6, "six bound point-defense mounts");
    assert!(turrets.iter().all(|binding| binding == &trigger));
    assert_eq!(lances, [vec![InputSource::Keyboard(KeyCode::KeyR)]]);
    assert_eq!(bays.len(), 2, "two bound torpedo bays");
    assert!(bays
        .iter()
        .all(|binding| binding == &[InputSource::Keyboard(KeyCode::KeyF)]));
    nova_probe::probe_marker(
        world,
        "outcome: the open world spawns one line warship with its weapons bound",
        serde_json::json!({ "turrets": turrets.len(), "lances": lances.len(), "bays": bays.len() }),
    );
}

/// Back on the menu, over its backdrop, with nothing of the world left.
#[cfg(feature = "debug")]
fn assert_the_world_disarmed(world: &mut World) {
    assert!(!world.contains_resource::<WorldConfig<NovaLayeredWorld>>());
    assert_eq!(count::<SectorRoot>(world), 0);
    let watch = world.resource::<WorldWatch>();
    let (armed, off_world) = (watch.armed_frames, watch.off_world_frames);
    assert!(armed > 0, "the watch never saw the world armed");
    assert!(off_world > 0, "the watch never saw another scenario");
    nova_probe::probe_marker(
        world,
        "outcome: every armed frame ran the observe stage",
        serde_json::json!({ "armed_frames": armed }),
    );
    nova_probe::probe_marker(
        world,
        "outcome: no other scenario runs over an armed world or a sector root",
        serde_json::json!({ "off_world_frames": off_world }),
    );
    info!("open_world: PASS the world disarmed before the menu came back");
}
