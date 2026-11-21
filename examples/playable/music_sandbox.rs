//! music_sandbox: hear the music director pick songs by situation, from the menu to a fight and back to a dock.
//!
//! Boots the shipped app on the main menu, where the menu theme plays. `N`
//! starts New Game on a pinned seed, as the Create World modal does, but
//! without a save folder: nothing this run does is saved. The start docks the
//! player at its starting station, so the docked songs come in after their
//! entry gap. Undock with the game's own key (`D`) and the cruise songs follow
//! theirs. The keys below put a fight in reach and take the hull back.
//!
//! | key | what it does |
//! | - | - |
//! | `N` | on the main menu: start New Game, docked at the starting station |
//! | `1` | ambush: raiders 6 km ahead (the hull carries weapons) |
//! | `2` | jump 1.5 km off the first raider: under its guns |
//! | `3` | jump 14 km on into open space |
//! | `4` | go back and dock at the starting station |
//!
//! Try: `N`, wait in the dock for a docked song, `D`, then `3` for open space
//! and the cruise gap. `1`, `2`: the threat song comes in when a raider holds
//! its lock for 2 s, the combat song on the first hit. `3` three times leaves
//! the raiders behind; the combat song holds 15 s past the last hit, then the
//! cruise song comes after 10 to 20 s of silence. `4` docks you back. `Esc`
//! ducks the music under the pause menu; `Tab` does not. The diagnostics
//! panel (`F3`) reads the situation, the mood, the song and the wait.
//!
//! With `NOVA_AUTOPILOT=1` a smoke walk reads the menu theme, starts the
//! world, reads a docked song, runs `volume music 0` in NOVA COMMANDS and
//! reads the same music voice alive at gain 0, undocks into open space and
//! reads Cruise, calls the ambush and jumps under the raiders' guns until the
//! context is Combat, leaves them and reads Cruise again, docks back and reads
//! Docked. It judges the director's context, its song and the deck, and logs
//! every context it heard. `NOVA_CAPTURE=1` shoots `sandbox-music-combat.png`.
//! Unset, no walk and no deadline run.

use avian3d::prelude::Position;
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::{nova_debug::diagnostics, prelude::*};

#[path = "../shared/encounter_range.rs"]
mod encounter_range;

use encounter_range::{
    berth_and_dock, body_wearing, open_space, park_off, park_player, the_player, window_streamed,
};

#[derive(Parser)]
#[command(name = "music_sandbox")]
#[command(version = "1.0.0")]
#[command(
    about = "Hear the music director by situation: the menu theme, New Game docked, cruise, \
             an ambush for threat and combat, and the dock again",
    long_about = None
)]
struct Cli;

/// The world seed: `encounter_sandbox`'s, whose start docks the player.
const SANDBOX_SEED: u32 = 17;
/// The start role: an armed hull, which the ambush asks for.
const SANDBOX_ROLE: &str = "bounty_hunter";
/// The ambush design key `1` asks for.
const AMBUSH: &str = "base/ambush";
/// How far off the first raider key `2` puts the hull.
const UNDER_THE_GUNS: Meters = Meters(1_500.0);
/// How far off the station the hull comes back to before it berths, so the
/// station's sector streams in first.
const STATION_APPROACH: Meters = Meters(3_000.0);
const START_KEY: KeyCode = KeyCode::KeyN;
const AMBUSH_KEY: KeyCode = KeyCode::Digit1;
const JUMP_KEY: KeyCode = KeyCode::Digit2;
const LEAVE_KEY: KeyCode = KeyCode::Digit3;
const DOCK_KEY: KeyCode = KeyCode::Digit4;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings: the console's `volume music` saves here, not
    // in the player's profile.
    std::env::set_var(
        nova_assets::storage::CONFIG_ROOT_ENV,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/example-profiles/{}-{}-{}",
            env!("CARGO_CRATE_NAME"),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after 1970")
                .as_nanos(),
        )),
    );

    let autopilot = std::env::var_os("NOVA_AUTOPILOT").is_some();
    let mut app = nova_protocol::prelude::editor_app(true, None);
    app.init_resource::<Sandbox>();
    app.init_resource::<HeardContexts>();
    app.add_plugins(diagnostics::DiagnosticsHintPlugin::new(INTRO));
    app.add_systems(
        Update,
        start_the_world.run_if(in_state(GameStates::MainMenu)),
    );
    app.add_systems(
        Update,
        (drive_the_sandbox, read_the_keys)
            .chain()
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(Update, hear_the_context.after(MusicSystems));
    app.add_systems(
        PostUpdate,
        read_the_director.in_set(diagnostics::DiagnosticsSystems),
    );

    if autopilot {
        app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
            bevy::ecs::error::panic,
        ));
        // The software rasterizer's frames are seconds long: the clamp
        // keeps a jump from integrating a whole frame of drift.
        app.add_systems(Update, smoke::hold_the_walk_clock);
        app.add_systems(
            Startup,
            nova_protocol::nova_debug::harness::force_capture_resolution,
        );
    }
    app.add_plugins(smoke::script());

    app.run()
}

/// What the sandbox is for, for the diagnostics intro.
const INTRO: &str =
    "Music. N on the menu starts docked. D undock, 1 ambush, 2 jump to it, 3 open space, 4 dock.";

/// Where the sandbox stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SandboxPhaseType {
    /// The main menu, before `N`.
    #[default]
    Menu,
    /// New Game is up; the start is docking the player.
    Streaming,
    /// The player has the run.
    Ready,
}

/// The sandbox's state.
#[derive(Resource, Default, Debug)]
struct Sandbox {
    phase: SandboxPhaseType,
    /// The walk asks for the start here instead of pressing `N`.
    start_asked: bool,
    /// The starting station: its generated id and where it stands.
    station: Option<(GeneratedBodyId, Meters3)>,
    /// Key `4` is in progress: berth at the station every frame until the
    /// dock takes.
    returning: bool,
}

/// Every context the director entered, in order, for the walk's report.
#[derive(Resource, Default, Debug)]
struct HeardContexts(Vec<MusicContextType>);

fn hear_the_context(director: Res<MusicDirector>, mut heard: ResMut<HeardContexts>) {
    let context = director.context();
    if heard.0.last() != Some(&context) {
        heard.0.push(context);
    }
}

/// `N` on the main menu, or the walk's ask: New Game on [`SANDBOX_SEED`]
/// with no save folder. A later Back to Main Menu stays on the menu.
fn start_the_world(world: &mut World) {
    let asked = world.resource::<Sandbox>().start_asked
        || world
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(START_KEY);
    if !asked || world.resource::<Sandbox>().phase != SandboxPhaseType::Menu {
        return;
    }
    let session = start_session(
        SANDBOX_SEED,
        SANDBOX_ROLE.into(),
        world.resource::<LoadedSectionPacks>(),
        world.resource::<GameStyles>(),
        world.resource::<GameItems>(),
        world.resource::<GameConsumables>(),
        world.resource::<GameTradeProfiles>(),
    )
    .unwrap_or_else(|refusal| panic!("music sandbox: seed {SANDBOX_SEED} refused: {refusal}"));
    world.insert_resource(session);
    world.resource_mut::<NewGameScenario>().0 = None;
    *world.resource_mut::<GameMode>() = GameMode::NewGame;
    world
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    world.resource_mut::<Sandbox>().phase = SandboxPhaseType::Streaming;
    info!("music sandbox: New Game on seed {SANDBOX_SEED}, no save folder");
}

/// The station the player is docked at, by its generated id and position.
fn docked_station(world: &World) -> Option<(GeneratedBodyId, Meters3)> {
    let player = the_player(world)?;
    let connection = world.get::<DockedShip>(player)?.connection;
    let station = world.get::<StationDock>(connection)?.0;
    Some((
        world.get::<GeneratedBodyId>(station)?.clone(),
        Meters3::from_engine(world.get::<Position>(station)?.0),
    ))
}

fn docked(world: &World) -> bool {
    the_player(world).is_some_and(|player| world.get::<DockedShip>(player).is_some())
}

/// Note the starting station once the start has docked the player, and
/// berth at it while key `4` is in progress.
fn drive_the_sandbox(world: &mut World) {
    if the_player(world).is_none() {
        return;
    }
    let sandbox = world.resource::<Sandbox>();
    if sandbox.phase == SandboxPhaseType::Streaming && window_streamed(world) {
        if let Some(station) = docked_station(world) {
            info!(
                "music sandbox: docked at {:?}; keys 1 to 4 are live",
                station.0
            );
            let mut sandbox = world.resource_mut::<Sandbox>();
            sandbox.station = Some(station);
            sandbox.phase = SandboxPhaseType::Ready;
        }
    }
    if world.resource::<Sandbox>().returning {
        if docked(world) {
            world.resource_mut::<Sandbox>().returning = false;
            info!("music sandbox: docked at the starting station again");
        } else {
            return_to_the_station(world);
        }
    }
}

/// One frame of key `4`: berth at the starting station when it stands,
/// else come back to [`STATION_APPROACH`] off it so its sector streams in.
fn return_to_the_station(world: &mut World) {
    let Some((id, at)) = world.resource::<Sandbox>().station.clone() else {
        return;
    };
    match body_wearing(world, &id) {
        Some(station) if window_streamed(world) => berth_and_dock(world, station),
        Some(_) => {}
        None => {
            let player = the_player(world).expect("music sandbox: one player ship");
            let own = Meters3::from_engine(
                world
                    .get::<Position>(player)
                    .expect("music sandbox: the player stands")
                    .0,
            );
            let away = (own.get() - at.get()).normalize_or(Vec3::X);
            park_player(
                world,
                Meters3(at.get() + away * STATION_APPROACH.get()),
                -away,
                MetersPerSecond(0.0),
            );
        }
    }
}

/// Key `3`: put the hull at rest in open space, 14 km on.
fn leave(world: &mut World) {
    match open_space(world) {
        Some((at, heading)) => park_player(world, at, heading, MetersPerSecond(0.0)),
        None => warn!("music sandbox: no axis out of here is clear of stations and bodies"),
    }
}

/// The live encounter's first standing body.
fn first_raider(world: &World) -> Option<Entity> {
    world
        .resource::<Encounters>()
        .live
        .as_ref()
        .and_then(|live| live.bodies.first().cloned())
        .and_then(|id| body_wearing(world, &id))
}

/// Key `2`: put the hull [`UNDER_THE_GUNS`] off the first raider.
fn jump_to_the_raiders(world: &mut World) {
    match first_raider(world) {
        Some(raider) => park_off(world, raider, UNDER_THE_GUNS),
        None => info!("music sandbox: no raider stands to jump to"),
    }
}

/// The sandbox keys, read in flight only: the shell and the panes take
/// digits as text.
fn read_the_keys(world: &mut World) {
    let flying = world.resource::<Sandbox>().phase == SandboxPhaseType::Ready
        && *world.resource::<State<PauseStates>>().get() == PauseStates::Unpaused;
    if !flying {
        return;
    }
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let [ambush, jump, away, dock] =
        [AMBUSH_KEY, JUMP_KEY, LEAVE_KEY, DOCK_KEY].map(|key| keys.just_pressed(key));
    if ambush {
        world
            .resource_mut::<Messages<EncounterFireCommand>>()
            .write(EncounterFireCommand {
                design: AMBUSH.into(),
            });
        info!("music sandbox: asked the director for '{AMBUSH}'");
    }
    if jump {
        jump_to_the_raiders(world);
    }
    if away {
        leave(world);
    }
    if dock && !docked(world) {
        world.resource_mut::<Sandbox>().returning = true;
    }
}

/// The song the director has the deck play, as the panel reads it.
fn song_line(world: &World) -> String {
    let director = world.resource::<MusicDirector>();
    match director.current() {
        Some(MusicSong::Track(id)) => match world.resource::<GameMusicTracks>().get(id) {
            Some(track) => format!("\"{}\" by {}", track.title, track.artist),
            None => id.to_string(),
        },
        Some(MusicSong::Scenario(_)) => "the scenario's song".to_string(),
        None => match director.wait() {
            Some(wait) => format!("silence, next song in {wait:.0} s"),
            None => "silence".to_string(),
        },
    }
}

/// The diagnostics panel: the situation, the mood, the song and the deck.
fn read_the_director(world: &mut World) {
    let Some(director) = world.get_resource::<MusicDirector>() else {
        return;
    };
    let (context, mood) = (director.context(), director.mood());
    let status = world
        .get_resource::<MusicDeck>()
        .map(|deck| format!("{:?}", deck.status()));
    let song = song_line(world);
    let volume = world.resource::<MusicVolume>().factor();
    let mut panel = world.resource_mut::<diagnostics::DiagnosticsPanel>();
    panel.line(format!("Music: {context:?}, mood {mood:?}"));
    panel.line(format!("Song: {song}"));
    panel.detail(format!("deck: {}", status.as_deref().unwrap_or("none")));
    panel.detail(format!("music volume: {volume:.2}"));
}

mod smoke {
    use std::sync::Arc;

    use nova_protocol::{
        nova_debug::harness::{shoot, AutopilotPlugin, Predicate},
        nova_interface::nova_command::prelude::CommandTerminal,
    };

    use super::*;

    /// Seconds the world gets to start, stream and dock on a software GPU.
    const SETUP_SECS: f32 = 600.0;
    /// Seconds a frame capture gets.
    const SHOT_SECS: f32 = 120.0;
    /// Seconds a beat gets. A calm context waits its entry gap first: up to
    /// 40 s of real time.
    const BEAT_SECS: f32 = 300.0;
    /// Seconds the raiders get to lock and land a hit.
    const FIGHT_SECS: f32 = 900.0;
    /// The virtual clock's clamp on the software rasterizer.
    const WALK_CLOCK_SECS: f32 = 1.0;
    /// How far past the ambush's 30 km the walk leaves the raiders.
    const BREAK_OFF: Meters = Meters(31_000.0);
    const COMBAT_SHOT: &str = "sandbox-music-combat.png";
    /// The id of the menu theme.
    const MENU_THEME: &str = "base/exploration_theme";

    pub(super) fn hold_the_walk_clock(mut time: ResMut<Time<Virtual>>) {
        let clamp = std::time::Duration::from_secs_f32(WALK_CLOCK_SECS);
        if time.max_delta() != clamp {
            time.set_max_delta(clamp);
        }
    }

    fn when(check: impl Fn(&World) -> bool + Send + Sync + 'static) -> Arc<Predicate> {
        Arc::new(check)
    }

    fn context(world: &World) -> MusicContextType {
        world.resource::<MusicDirector>().context()
    }

    fn deck_status(world: &World) -> MusicDeckStatusType {
        world.resource::<MusicDeck>().status()
    }

    /// The deck holds a song: it plays, or it holds with no audio device.
    fn deck_holds(world: &World) -> bool {
        matches!(
            deck_status(world),
            MusicDeckStatusType::Playing | MusicDeckStatusType::NoOutput
        )
    }

    /// The director has the deck play a track of `context`, and the deck
    /// holds it.
    fn plays_a_track_of(wanted: MusicContextType) -> Arc<Predicate> {
        when(move |world| {
            let Some(MusicSong::Track(id)) = world.resource::<MusicDirector>().current() else {
                return false;
            };
            context(world) == wanted
                && deck_holds(world)
                && world
                    .resource::<GameMusicTracks>()
                    .get(id)
                    .is_some_and(|track| track.contexts.contains(&wanted))
        })
    }

    fn context_is(wanted: MusicContextType) -> Arc<Predicate> {
        when(move |world| context(world) == wanted)
    }

    /// Every music voice's pre-master gain.
    fn music_gains(world: &World) -> Vec<f32> {
        world
            .try_query_filtered::<&VoiceMix, With<MusicVoice>>()
            .map(|mut voices| voices.iter(world).map(|mix| mix.gain).collect())
            .unwrap_or_default()
    }

    fn describe(world: &World) -> String {
        let director = world.resource::<MusicDirector>();
        format!(
            "context {:?}, mood {:?}, song {}, deck {:?}, music gains {:?}, docked {}, \
             live encounter {:?}",
            director.context(),
            director.mood(),
            song_line(world),
            deck_status(world),
            music_gains(world),
            docked(world),
            world.get_resource::<Encounters>().and_then(|record| record
                .live
                .as_ref()
                .map(|live| live.design.as_str().to_string())),
        )
    }

    fn paused_as(wanted: PauseStates) -> Arc<Predicate> {
        when(move |world| {
            world
                .get_resource::<State<PauseStates>>()
                .is_some_and(|pause| *pause.get() == wanted)
        })
    }

    /// Type `line` at the open NOVA COMMANDS prompt and run it.
    fn run_command(
        script: AutopilotPlugin<GameStates>,
        line: &'static str,
    ) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("music sandbox: type `{line}`"))
            .on_enter(type_text(line))
            .until(resource_where::<CommandTerminal>(move |terminal| {
                terminal.prompt() == line
            }))
            .deadline(BEAT_SECS)
            .add()
            .step(format!("music sandbox: run `{line}`"))
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<CommandTerminal>(|terminal| {
                terminal.prompt().is_empty()
            }))
            .deadline(BEAT_SECS)
            .add()
    }

    /// The walk. Unarmed (no `NOVA_AUTOPILOT`) it adds nothing.
    pub(super) fn script() -> AutopilotPlugin<GameStates> {
        let script = AutopilotPlugin::<GameStates>::new();
        if std::env::var_os("NOVA_AUTOPILOT").is_none() {
            return script;
        }

        // The menu theme.
        let script = script
            .step("music sandbox: the menu theme plays")
            .until(and(
                plays_a_track_of(MusicContextType::Menu),
                when(|world| {
                    matches!(
                        world.resource::<MusicDirector>().current(),
                        Some(MusicSong::Track(id)) if id.as_str() == MENU_THEME
                    )
                }),
            ))
            .diagnose(describe)
            .deadline(SETUP_SECS)
            .add()
            .step("music sandbox: report the menu theme")
            .on_enter(|world: &mut World| {
                info!(
                    "music sandbox: PASS the menu plays {MENU_THEME}, deck {:?}",
                    deck_status(world)
                );
                world.resource_mut::<Sandbox>().start_asked = true;
            })
            .add();

        // New Game, docked at the starting station.
        let script = script
            .step("music sandbox: New Game docks the player and a docked song plays")
            .until(and(
                when(|world| world.resource::<Sandbox>().phase == SandboxPhaseType::Ready),
                plays_a_track_of(MusicContextType::Docked),
            ))
            .diagnose(describe)
            .deadline(SETUP_SECS)
            .add()
            .step("music sandbox: report the dock")
            .on_enter(|world: &mut World| {
                info!("music sandbox: PASS docked plays {}", song_line(world));
            })
            .add();

        // `volume music 0` silences the song and keeps its voice.
        let script = script
            .step("music sandbox: `;` opens NOVA COMMANDS")
            .on_enter(type_text(";"))
            .until(paused_as(PauseStates::Commands))
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: the intro drained")
            .until(resource_where::<CommandTerminal>(|terminal| {
                terminal.is_revealed() && !terminal.has_pending_boot_rows()
            }))
            .deadline(BEAT_SECS)
            .add();
        let script = run_command(script, "volume music 0")
            .step("music sandbox: the music voice plays on at gain 0")
            .until(when(|world| {
                world.resource::<MusicVolume>().factor() == 0.0
                    && !music_gains(world).is_empty()
                    && music_gains(world).iter().all(|gain| *gain == 0.0)
            }))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: report the silent voice")
            .on_enter(|world: &mut World| {
                assert!(
                    deck_holds(world),
                    "music sandbox: the deck holds its song at slider 0: {}",
                    describe(world)
                );
                info!(
                    "music sandbox: PASS `volume music 0` holds {} music voice(s) at gain 0, \
                     deck {:?}",
                    music_gains(world).len(),
                    deck_status(world)
                );
            })
            .add();
        let script = run_command(script, "volume music 1")
            .step("music sandbox: the same voice comes back up")
            .until(when(|world| {
                world.resource::<MusicVolume>().factor() == 1.0
                    && music_gains(world).iter().any(|gain| *gain > 0.0)
            }))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: Escape closes NOVA COMMANDS")
            .on_enter(press_key(KeyCode::Escape))
            .until(paused_as(PauseStates::Unpaused))
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: let Escape up")
            .on_enter(release_key(KeyCode::Escape))
            .until(frames(2))
            .add();

        // Undock into open space: Cruise.
        let script = script
            .step("music sandbox: undock")
            .on_enter(|world: &mut World| {
                let player = the_player(world).expect("music sandbox: one player ship");
                world.trigger(DockingReleaseRequest { entity: player });
            })
            .until(when(|world| !docked(world)))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: open space streams in")
            .on_enter(leave)
            .until(and(
                when(window_streamed),
                context_is(MusicContextType::Cruise),
            ))
            .diagnose(describe)
            .deadline(SETUP_SECS)
            .add()
            .step("music sandbox: report the cruise")
            .on_enter(|_: &mut World| info!("music sandbox: PASS undocked is Cruise"))
            .add();

        // The ambush: Threat on a held lock, Combat on the first hit.
        let script = script
            .step("music sandbox: call the ambush")
            .on_enter(|world: &mut World| {
                world
                    .resource_mut::<Messages<EncounterFireCommand>>()
                    .write(EncounterFireCommand {
                        design: AMBUSH.into(),
                    });
            })
            .until(when(|world| first_raider(world).is_some()))
            .diagnose(describe)
            .deadline(BEAT_SECS)
            .add()
            .step("music sandbox: under the raiders' guns the fight starts")
            .on_enter(jump_to_the_raiders)
            .until(plays_a_track_of(MusicContextType::Combat))
            .diagnose(describe)
            .deadline(FIGHT_SECS)
            .add();
        let script = shot_step(script)
            .step("music sandbox: report the fight")
            .on_enter(|world: &mut World| {
                info!("music sandbox: PASS the fight plays {}", song_line(world));
            })
            .add();

        // Leave the raiders: the fight winds down to Cruise.
        let script = script
            .step("music sandbox: leave the raiders and the fight winds down")
            .on_enter(|world: &mut World| {
                if let Some(raider) = first_raider(world) {
                    park_off(world, raider, BREAK_OFF);
                }
            })
            .until(context_is(MusicContextType::Cruise))
            .diagnose(describe)
            .deadline(FIGHT_SECS)
            .add()
            .step("music sandbox: report the wind-down")
            .on_enter(|_: &mut World| info!("music sandbox: PASS the fight wound down to Cruise"))
            .add();

        // Dock back: Docked.
        script
            .step("music sandbox: dock at the starting station")
            .on_enter(|world: &mut World| {
                world.resource_mut::<Sandbox>().returning = true;
            })
            .until(and(when(docked), context_is(MusicContextType::Docked)))
            .diagnose(describe)
            .deadline(SETUP_SECS)
            .add()
            .step("music sandbox: report the walk")
            .on_enter(|world: &mut World| {
                let heard = world.resource::<HeardContexts>().0.clone();
                for wanted in [
                    MusicContextType::Menu,
                    MusicContextType::Docked,
                    MusicContextType::Cruise,
                    MusicContextType::Combat,
                ] {
                    assert!(
                        heard.contains(&wanted),
                        "music sandbox: the walk never heard {wanted:?}: {heard:?}"
                    );
                }
                info!("music sandbox: PASS docked again; contexts heard {heard:?}");
            })
            .add()
    }

    fn shot_step(script: AutopilotPlugin<GameStates>) -> AutopilotPlugin<GameStates> {
        script
            .step(format!("music sandbox: shoot {COMBAT_SHOT}"))
            .on_enter(|world: &mut World| shoot(world, COMBAT_SHOT))
            .until(shot_written(COMBAT_SHOT))
            .deadline(SHOT_SECS)
            .add()
    }
}
