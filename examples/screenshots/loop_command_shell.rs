//! loop_command_shell: the `command-shell-open` webm loop - `:` over live
//! flight, and the computer answering.
//!
//! The commands wiki page opens on a claim the page cannot make in prose: that
//! the shell is not a menu but a surface that drops over whatever you were
//! doing. So the loop starts on flight - a ship under thrust, a holo ring, a
//! HUD - types `:`, and lets the CRT come up over it: the monitor, the COMMANDS
//! header, the introduction typing itself out, then a real command typed at the
//! prompt and answered off the live world.
//!
//! `system_command_shell` is the CORRECTNESS range for the same gesture and it
//! runs with no renderer at all. This one has the renderer and asserts nothing:
//! the two are the same seam photographed from its two sides.
//!
//! The set is "The ring" (`shared/ring.rs`), the same planetoid and orbit every
//! flight-computer figure is shot on, because the shell has to be seen ON
//! something and this is the one set already framed for it.
//!
//! The prompt is typed a CHARACTER AT A TIME ([`Typewriter`]). `type_text`
//! delivers a whole string in one frame, which is right for a range driving a
//! field to a value and wrong for a recording: a command that appears whole
//! between two frames reads as a paste, not as typing.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the full walk, recording
//!   nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: record and encode the loop into
//!   `NOVA_CAPTURE_DIR/command-shell-open.webm`.
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/loop-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_command_shell --features debug
//! ```

#[path = "shared/ring.rs"]
mod ring;

use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "loop_command_shell")]
#[command(version = "1.0.0")]
#[command(about = "The command-shell-open webm loop: `:` drops the computer over live flight and it answers. Autopilot-only: the walk types the gesture", long_about = None)]
struct Cli;

/// The loop this example records - the webm's file stem.
#[cfg(feature = "debug")]
const LOOP_NAME: &str = "command-shell-open";

/// The command the walk types at the prompt.
///
/// `ships` and not `help`: the page's claim is that the shell reads the RUN,
/// and a command that lists the hulls actually in the world is the shortest
/// thing that shows it. The answer names `ring_player`, which is the ship the
/// same frame is looking at.
#[cfg(feature = "debug")]
const COMMAND: &str = "ships";

/// How many recorded frames each typed character is held for.
///
/// The loop is frame-clocked at 30 fps, so three frames is a tenth of a second
/// per character - fast enough to read as typing rather than as a demonstration
/// of typing, slow enough that the characters are separate events.
#[cfg(feature = "debug")]
const FRAMES_PER_CHARACTER: u32 = 3;

/// Real seconds the typed command gets before the walk aborts naming the beat.
///
/// Its own number because the beat is FRAME-clocked and a deadline counts REAL
/// seconds. The typing is [`FRAMES_PER_CHARACTER`] per character of
/// [`COMMAND`] - sixteen frames with the entry one - which on the ARMED path
/// is sixteen thirtieths of a real second, because a loop capture pins
/// `Time<Real>` to the frame step (see "What a step deadline counts" in
/// `nova_debug::harness`), and on the unarmed path is sixteen wall frames of
/// a scene that costs about three quarters of a second each under CI's
/// software renderer. This is sized for the slower of the two with room over
/// it, and is a backstop no healthy walk reaches on either.
#[cfg(feature = "debug")]
const TYPING_DEADLINE_SECS: f32 = 45.0;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    // NovaMenuPlugin explicitly: the `:` gesture lives in nova_menu, and
    // `with_game_plugins` turns the menu plugin off. `:` IS the subject of this
    // figure, so the plugin has to be here; it stands alone in a slim app, and
    // the boot-into-MainMenu handoff it normally owns is not taken (this run
    // goes Loading -> Playing).
    let mut app = AppBuilder::new()
        .with_game_plugins((custom_plugin, NovaMenuPlugin))
        .build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants. No frame-time capture - the
        // walk has no steady-state window, so a captured fps would measure the
        // script.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.init_resource::<Typewriter>();
        app.add_systems(Update, drive_the_typewriter);
        app.add_plugins(shell_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_scene);
}

fn load_scene(mut commands: Commands, game_assets: Res<GameAssets>, ships: Res<GameShips>) {
    commands.trigger(LoadScenario(ring::the_ring(&game_assets, &ships)));
}

/// A command being typed, one character per [`FRAMES_PER_CHARACTER`] frames.
///
/// Idle when `pending` is empty, which is its state for the whole walk outside
/// the one step that fills it.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Typewriter {
    /// Characters not yet typed, in reverse so the next one pops off the end.
    pending: Vec<char>,
    /// Frames left to hold before the next character.
    hold: u32,
}

#[cfg(feature = "debug")]
impl Typewriter {
    /// Queue `text` to be typed out.
    fn write(&mut self, text: &str) {
        self.pending = text.chars().rev().collect();
        self.hold = 0;
    }

    /// Whether everything queued has been typed.
    fn idle(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Type the next queued character when its hold has run out.
#[cfg(feature = "debug")]
fn drive_the_typewriter(world: &mut World) {
    let next = {
        let mut typewriter = world.resource_mut::<Typewriter>();
        if typewriter.pending.is_empty() {
            return;
        }
        if typewriter.hold > 0 {
            typewriter.hold -= 1;
            return;
        }
        typewriter.hold = FRAMES_PER_CHARACTER;
        typewriter.pending.pop()
    };
    let Some(character) = next else {
        return;
    };
    type_text(character.to_string())(world);
}

/// The driven walk: fly, drop the shell over the flight, type a command, read
/// the answer.
#[cfg(feature = "debug")]
fn shell_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the ring")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle at the start")
        .on_enter(ring::hud_instrument)
        .until(elapsed(1.0))
        .add()
        // Something for the shell to drop OVER. The insertion burn is the
        // liveliest second this set has - full drive, the hull swinging onto
        // the plane, the holo ring and the radius spoke already up - and the
        // loop opens on it rather than waiting for the orbit to settle.
        .step("engage the orbit computer")
        .on_enter(ring::engage_orbit)
        .until(ring::orbit_burning())
        .deadline(30.0)
        .add()
        // The ONE framing: outboard of the ship and above it, aimed most of
        // the way at the body, so the burn, the ring and the ice planetoid are
        // all behind the monitor when it comes up. Posed before the loop opens
        // and never touched again - the shell freezes the world anyway, and a
        // camera that moved under a static CRT would be the only thing in the
        // frame that did.
        .step("frame the flight")
        .on_enter(|world: &mut World| {
            let ship = ring::ship_position(world);
            let out = ship.get().normalize_or_zero();
            let track = ring::ship_heading(world);
            ring::pin(
                world,
                ship + Meters3(out * 450.0 + Vec3::Y * 150.0 - track * 100.0),
                ship - Meters3(out * 1_200.0),
            );
        })
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("open the shell loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the live frame")
        .until(elapsed(0.8))
        .add()
        // `:` is read as the CHARACTER, so this is the gesture a player makes
        // whatever their layout puts a colon on.
        .step("drop the computer over it")
        .on_enter(type_text(":"))
        .until(resource_where::<State<PauseStates>>(|pause| {
            *pause.get() == PauseStates::NovaOs
        }))
        .deadline(10.0)
        .add()
        // The monitor's own power-on and the COMMANDS introduction typing
        // itself out. This is the beat the figure exists for, and it is the
        // terminal's animation, not the walk's: waiting on the reveal rather
        // than on a stopwatch keeps the recording in step with it.
        .step("let the introduction type itself out")
        .until(the_introduction_has_landed())
        .deadline(20.0)
        .add()
        // FRAMES, not seconds, from here on. `:` freezes the simulation - that
        // is what the pause axis is for - so `elapsed` stops advancing the
        // moment the computer is up, and every hold behind it would wait for a
        // clock that has stopped. The loop records one frame per update at
        // 30 fps, so a frame count IS the duration on the page.
        .step("hold the fresh prompt")
        .until(frames(15))
        .add()
        .step("type a command at the prompt")
        .on_enter(|world: &mut World| world.resource_mut::<Typewriter>().write(COMMAND))
        .until(and(
            resource_where::<Typewriter>(Typewriter::idle),
            the_prompt_reads(COMMAND),
        ))
        .deadline(TYPING_DEADLINE_SECS)
        .add()
        .step("hold before committing it")
        .until(frames(11))
        .add()
        .step("run it against the live world")
        .on_enter(press_edit_key(bevy::input::keyboard::Key::Enter))
        .until(the_answer_names_the_ship())
        .deadline(10.0)
        .add()
        .step("hold the answer")
        .until(frames(48))
        .add()
        .step("close the shell loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(60.0)
        .add()
}

/// Advance once the shell has booted and the COMMANDS introduction has
/// finished revealing itself.
#[cfg(feature = "debug")]
fn the_introduction_has_landed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use nova_protocol::nova_os_ui::nova_os::prelude::{NovaOsTerminal, ShellKind};
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<NovaOsTerminal>()
            .is_some_and(|terminal| {
                terminal.is_booted()
                    && !terminal.has_pending_boot_rows()
                    && terminal.is_revealed(ShellKind::Commands)
            })
    })
}

/// Advance once the prompt holds exactly `text`.
#[cfg(feature = "debug")]
fn the_prompt_reads(
    text: &'static str,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use nova_protocol::nova_os_ui::nova_os::prelude::NovaOsTerminal;
    std::sync::Arc::new(move |world: &World| {
        world
            .get_resource::<NovaOsTerminal>()
            .is_some_and(|terminal| terminal.prompt() == text)
    })
}

/// Advance once the transcript carries an answer naming the set's own ship.
#[cfg(feature = "debug")]
fn the_answer_names_the_ship() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    use nova_protocol::nova_os_ui::nova_os::prelude::NovaOsTerminal;
    std::sync::Arc::new(|world: &World| {
        world
            .get_resource::<NovaOsTerminal>()
            .is_some_and(|terminal| {
                terminal
                    .scrollback()
                    .iter()
                    .any(|row| row.text.contains(ring::PLAYER_ID))
            })
    })
}
