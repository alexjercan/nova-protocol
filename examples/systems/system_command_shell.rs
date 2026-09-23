//! system_command_shell: the `cmd>` shell driven end to end, off-screen.
//!
//! The NOVA OS range beside this one proves the terminal takes typing with no
//! renderer. This one proves the other half: that what is typed REACHES the
//! live world and comes back, that the two ways into the computer land in the
//! shell they name, and that `:` is a GLOBAL gesture - flight, the pause menu
//! and the main menu - which gives back the exact surface it covered.
//!
//! Three of those were live bugs. `:` opened the command shell and left it the
//! active shell for good, so Tab afterwards opened `cmd>` instead of NOVA OS;
//! Escape out of a shell opened over flight unpaused a player who had been
//! paused; and the shell opened on the main menu stopped the ambience behind
//! it, leaving the front door on a still frame. A shell that answers correctly
//! but strands the player who opened it is not working, so the range drives the
//! whole gesture on all three surfaces: open, run, complete, close, and open
//! the OTHER shell.
//!
//! The freeze is the part that is surface-dependent, and the range states both
//! halves: over flight the shell holds the simulation still, and over the main
//! menu it holds nothing - what runs there is a cinematic the shell is drawn
//! over, not a game the player is being kept out of. What the shell DOES take
//! on the menu is the pointer: it is the active modal, so the menu buttons
//! under it are unreachable until it closes, and reachable again the moment it
//! does.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_command_shell --features debug
//! # look for: `command shell: PASS the shell answers and gives the surface back`.
//! ```

#[cfg(feature = "debug")]
use bevy::{input::keyboard::Key, prelude::*, window::PrimaryWindow};
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::nova_os::prelude::{NovaOsTerminal, ShellKind};
#[cfg(feature = "debug")]
use nova_protocol::prelude::*;

#[cfg(not(feature = "debug"))]
fn main() {
    eprintln!("system_command_shell drives the app through the debug-only autopilot gestures;");
    eprintln!("run it with --features debug");
}

/// The id the tutorial gives the player's hull. The range types it half and
/// lets completion finish it, so a rename breaks this range loudly rather than
/// leaving the completion untested.
#[cfg(feature = "debug")]
const PLAYER_ID: &str = "trainer";

/// A UNIQUE prefix of it. Tab completion reads the live world, so this has to
/// name one ship and no other: the range's five targets start with a `t` too,
/// so one letter is not enough.
#[cfg(feature = "debug")]
const PLAYER_ID_PREFIX: &str = "tr";

/// The main menu's first button, and the one the open computer must cover.
#[cfg(feature = "debug")]
const NEW_GAME_BUTTON: &str = "New Game Button";

/// What the New Game click opens: the world setup modal's start button.
#[cfg(feature = "debug")]
const CREATE_WORLD_BUTTON: &str = "Create World Button";
/// The pause menu's way back into the game.
#[cfg(feature = "debug")]
const RESUME_BUTTON: &str = "Resume Button";
/// The pause menu's way into its Settings panel - the modal that used to carry
/// the same z as the open computer.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_BUTTON: &str = "Pause Settings Button";
/// A node only the open pause Settings panel has, so the click that opens it
/// can be waited on rather than assumed.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_BACK_BUTTON: &str = "Pause Settings Back Button";
/// The pause menu's way out to the main menu.
#[cfg(feature = "debug")]
const BACK_TO_MENU_BUTTON: &str = "Back To Menu Button";
/// The pause menu's own full-screen blocker.
#[cfg(feature = "debug")]
const PAUSE_OVERLAY: &str = "Pause Overlay";
/// The pause Settings panel's full-screen blocker.
#[cfg(feature = "debug")]
const PAUSE_SETTINGS_ROOT: &str = "Pause Settings Panel Root";
/// The dim field the open computer puts over whatever it covers.
#[cfg(feature = "debug")]
const NOVA_OS_BACKDROP: &str = "NovaOsBackdrop";
/// The computer itself.
#[cfg(feature = "debug")]
const NOVA_OS_MONITOR: &str = "NovaOsMonitor";

/// Seconds a scenario load or a content restart is given. Sized to outlast the
/// training range's load on a small shared runner, and kept under the harness
/// completion deadline so a stall names THIS beat.
#[cfg(feature = "debug")]
const SESSION_SECS: f32 = 90.0;

/// Frames a "nothing happened" beat waits before it believes nothing happened.
///
/// A click the menu DID take would have changed state well inside this: the
/// menu's New Game is one `Activate` observer and one state write.
#[cfg(feature = "debug")]
const SETTLE_FRAMES: u32 = 30;

/// Where the pause menu's two blockers were stacked, read off the live overlay
/// before the computer covers them.
///
/// Carried between beats rather than re-read, because the pause overlay is
/// `DespawnOnExit(Paused)`: by the time the computer is open the nodes whose
/// numbers the comparison needs are gone.
#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug, Clone, Copy)]
struct PauseLayers {
    /// The pause overlay's `GlobalZIndex`.
    overlay: Option<i32>,
    /// The pause Settings panel root's `GlobalZIndex`.
    settings: Option<i32>,
}

#[cfg(feature = "debug")]
fn main() -> bevy::app::AppExit {
    let mut app = editor_app(false, Some(StartupScenario::Id("tutorial".to_string())));

    // The virtual window, exactly as in `system_headless_novaos`: typing is
    // read off `KeyboardInput`, which carries the window it was typed into.
    // It is also what `bevy_ui` lays out against and what `bevy_picking` aims
    // into, which is how the blocked-click beats below are possible with no
    // renderer at all (`system_headless_pointer` is the worked proof).
    app.world_mut().spawn((
        Window {
            resolution: (1280, 720).into(),
            ..default()
        },
        PrimaryWindow,
    ));

    app.init_resource::<PauseLayers>();

    // The probe's timeline and invariant feed. Without it the run still asserts
    // - every beat panics on its own - but `probe run` grades it UNPROBEABLE,
    // because the markers beside those asserts reach no report. No frame-time
    // claim: most of this walk happens with the world deliberately frozen
    // under an open shell, and the number that comes out of that says nothing
    // about the game's speed.
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());

    app.add_plugins(
        nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
            .step("command shell: reach Playing with no renderer")
            .until(state_is(GameStates::Playing))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // `:` is read as the CHARACTER, so a layout where the colon is not
            // Shift+Semicolon opens the shell with the key that prints one.
            .step("command shell: `:` opens the computer")
            .on_enter(type_text(":"))
            .until(resource_where::<State<PauseStates>>(|pause| {
                *pause.get() == PauseStates::NovaOs
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The freeze is the terminal's, by name. A shell over FLIGHT stops
            // the world: the player is reading a computer, not flying.
            .step("command shell: the shell over flight holds the world still")
            .until(resource_where::<ClockFreeze>(|freeze| {
                freeze.is_held_by(FreezeOwner::Terminal)
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the flight freeze")
            .on_enter(assert_flight_is_frozen)
            .add()
            .step("command shell: the shell it opened is `cmd>`")
            .on_enter(assert_command_shell_is_active)
            .add()
            .step("command shell: the intro drained")
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.is_booted()
                    && !terminal.has_pending_boot_rows()
                    && terminal.is_revealed(ShellKind::Commands)
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The round trip: the line is parsed here, run against the live
            // world by `nova_console`, and printed back into this transcript.
            .step("command shell: `ships` reaches the live world")
            .on_enter(type_text("ships"))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == "ships"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the answer names the player's hull")
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal
                    .scrollback()
                    .iter()
                    .any(|row| row.text.contains(PLAYER_ID))
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the round trip")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: a typed command answers from the live world",
                    serde_json::json!({ "command": "ships" }),
                );
            })
            .add()
            // Completion asks the WORLD, not the catalog: `ship` takes a live
            // ship id, and the ids come from the ships that are actually there.
            .step("command shell: half an id at the prompt")
            .on_enter(type_text(format!("ship {PLAYER_ID_PREFIX}")))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == format!("ship {PLAYER_ID_PREFIX}")
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: Tab finishes it from the live world")
            .on_enter(press_edit_key(Key::Tab))
            .until(resource_where::<NovaOsTerminal>(|terminal| {
                terminal.prompt() == format!("ship {PLAYER_ID}")
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the completion")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: Tab completes an id only the live world knows",
                    serde_json::json!({ "completed": PLAYER_ID }),
                );
            })
            .add()
            // The shell is a surface OVER what was there, so closing it gives
            // that back. Opened from flight, Escape returns to flight.
            .step("command shell: Escape closes the computer")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Unpaused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape")
            .on_enter(release_key(KeyCode::Escape))
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: Escape gives back the surface the shell covered",
                    serde_json::json!({}),
                );
            })
            .add()
            // The second bug: `:` used to leave `cmd>` the active shell for the
            // rest of the run, so Tab reopened the shell the player had just
            // closed instead of the one it names.
            .step("command shell: Tab opens the computer again")
            .on_enter(press_key(KeyCode::Tab))
            .until(the_pause_axis_is(PauseStates::NovaOs))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Tab")
            .on_enter(release_key(KeyCode::Tab))
            .add()
            .step("command shell: and Tab opened NOVA OS")
            .on_enter(assert_nova_os_shell_is_active)
            .add()
            .step("command shell: Escape closes NOVA OS")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Unpaused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape after NOVA OS")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            // SURFACE TWO: the pause menu. ESC first, then `:` over it.
            .step("command shell: ESC opens the pause menu")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Paused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: let ESC go")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .step("command shell: the pause menu laid out")
            .until(ui_node_present(RESUME_BUTTON))
            .diagnose(ui_node_diagnosis(RESUME_BUTTON))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The pause Settings panel is the modal that used to carry the open
            // computer's own z. Opened here so the comparison below is made
            // against the deepest stack the pause menu can raise.
            .click_named(
                "command shell: open pause Settings",
                PAUSE_SETTINGS_BUTTON,
                ui_node_present(PAUSE_SETTINGS_BACK_BUTTON),
                BEAT_DEADLINE_SECS,
            )
            .step("command shell: record the pause menu's own layers")
            .on_enter(record_pause_layers)
            .add()
            .step("command shell: `:` opens the computer over the pause menu")
            .on_enter(type_text(":"))
            .until(the_pause_axis_is(PauseStates::NovaOs))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the pause-menu shell")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: `:` opens the command shell from the pause menu",
                    serde_json::json!({}),
                );
            })
            .add()
            .step("command shell: the open computer is the top layer")
            .on_enter(assert_the_computer_is_on_top)
            .add()
            // Close returns to the EXACT surface it covered, which here is the
            // pause menu and not the flight it was paused over.
            .step("command shell: Escape gives the pause menu back")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Paused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape over the pause menu")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .step("command shell: the pause menu is standing again")
            .until(ui_node_present(RESUME_BUTTON))
            .diagnose(ui_node_diagnosis(RESUME_BUTTON))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the surface the shell gave back")
            .on_enter(assert_the_pause_menu_came_back)
            .add()
            // SURFACE THREE: the main menu, reached the player's way - the
            // pause menu's own door, through the content reload behind it.
            .click_named(
                "command shell: back to the main menu",
                BACK_TO_MENU_BUTTON,
                state_is(GameStates::MainMenu),
                SESSION_SECS,
            )
            .step("command shell: the main menu laid out")
            .until(ui_node_present(NEW_GAME_BUTTON))
            .diagnose(ui_node_diagnosis(NEW_GAME_BUTTON))
            .deadline(SESSION_SECS)
            .add()
            // The precondition the ambience claim is made against: nobody is
            // holding the clocks when the shell opens, so a hold afterwards is
            // the shell's.
            .step("command shell: the menu's clocks are running")
            .until(resource_where::<ClockFreeze>(|freeze| !freeze.is_held()))
            .deadline(SESSION_SECS)
            .add()
            .step("command shell: `:` opens the computer on the main menu")
            .on_enter(type_text(":"))
            .until(the_pause_axis_is(PauseStates::NovaOs))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the main-menu shell")
            .on_enter(assert_the_menu_shell_opened)
            .add()
            .step("command shell: the ambience keeps running under the computer")
            .each(|world: &mut World, _, _| assert_nothing_holds_the_menu(world))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the running ambience")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: the shell over the menu leaves the ambience running",
                    serde_json::json!({}),
                );
            })
            .add()
            // What the shell DOES take on the menu is the pointer. Aimed with
            // `hover_named` rather than the composite click, because the
            // composite waits for the pick to REACH the button - which is the
            // very thing that must not happen here.
            .step("command shell: aim at New Game through the open computer")
            .on_enter(hover_named(NEW_GAME_BUTTON))
            .until(pointer_at_node(NEW_GAME_BUTTON, Vec2::ZERO))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: press where New Game is")
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release, and give the menu every chance to act")
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the menu did not take the click")
            .on_enter(assert_the_menu_was_blocked)
            .add()
            // ...and the negative control: the same button, the same aim, with
            // the computer closed. Without this the beat above passes on a
            // menu that is simply broken.
            .step("command shell: Escape closes the menu's computer")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Unpaused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape on the menu")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            .click_named(
                "command shell: New Game once the computer is closed",
                NEW_GAME_BUTTON,
                ui_node_present(CREATE_WORLD_BUTTON),
                SESSION_SECS,
            )
            .step("command shell: the menu takes the click again")
            .on_enter(assert_the_menu_takes_clicks_again)
            .add(),
    );

    app.run()
}

/// Advance once the shared freeze axis reads `wanted`.
///
/// The axis, not the pause MENU: `PauseStates` is what the terminal, the pause
/// overlay and flight all move along, and every beat here is a move on it.
#[cfg(feature = "debug")]
fn the_pause_axis_is(
    wanted: PauseStates,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<State<PauseStates>>(move |pause| *pause.get() == wanted)
}

/// The `GlobalZIndex` of the one node called `wanted`, if it has one.
#[cfg(feature = "debug")]
fn named_z(world: &mut World, wanted: &str) -> Option<i32> {
    let mut nodes = world.query::<(&Name, &GlobalZIndex)>();
    nodes
        .iter(world)
        .find(|(name, _)| name.as_str() == wanted)
        .map(|(_, z)| z.0)
}

/// How many entities called `wanted` are in the world.
#[cfg(feature = "debug")]
fn count_named(world: &mut World, wanted: &str) -> usize {
    let mut names = world.query::<&Name>();
    names
        .iter(world)
        .filter(|name| name.as_str() == wanted)
        .count()
}

/// A shell over flight stops the simulation, and says who is holding it.
#[cfg(feature = "debug")]
fn assert_flight_is_frozen(world: &mut World) {
    assert!(
        world.resource::<Time<Virtual>>().is_paused(),
        "the command shell over flight must stop the simulation"
    );
    let holds: Vec<String> = world
        .resource::<ClockFreeze>()
        .owners()
        .map(|owner| format!("{owner:?}"))
        .collect();
    info!("command shell: the shell over flight holds the clocks ({holds:?})");
    nova_probe::probe_marker(
        world,
        "outcome: the shell over flight keeps the game frozen",
        serde_json::json!({ "holds": holds }),
    );
}

/// `:` names the Command shell, and lands on its prompt.
#[cfg(feature = "debug")]
fn assert_command_shell_is_active(world: &mut World) {
    let shell = world.resource::<NovaOsTerminal>().active_shell();
    assert_eq!(
        shell,
        ShellKind::Commands,
        "`:` must open the command shell, not whichever shell was last active"
    );
    info!("command shell: `:` opened {}", shell.prompt_prefix().trim());
    nova_probe::probe_marker(
        world,
        "outcome: `:` opens the command shell",
        serde_json::json!({ "shell": shell.prompt_prefix() }),
    );
}

/// Tab names NOVA OS, whatever shell the player was in last.
#[cfg(feature = "debug")]
fn assert_nova_os_shell_is_active(world: &mut World) {
    let shell = world.resource::<NovaOsTerminal>().active_shell();
    assert_eq!(
        shell,
        ShellKind::NovaOs,
        "Tab must open NOVA OS even after `:` has opened the command shell"
    );
    info!("command shell: PASS the shell answers and gives the surface back");
    nova_probe::probe_marker(
        world,
        "outcome: Tab opens NOVA OS after the command shell has been used",
        serde_json::json!({ "shell": shell.prompt_prefix() }),
    );
}

/// Read the pause menu's two blockers off the live overlay, while they exist.
#[cfg(feature = "debug")]
fn record_pause_layers(world: &mut World) {
    let overlay = named_z(world, PAUSE_OVERLAY).expect("the pause overlay carries a GlobalZIndex");
    let settings = named_z(world, PAUSE_SETTINGS_ROOT)
        .expect("the pause Settings root carries a GlobalZIndex");
    assert!(
        settings > overlay,
        "the pause Settings panel must sit strictly above the pause buttons it blocks \
         (settings {settings}, overlay {overlay})"
    );
    world.insert_resource(PauseLayers {
        overlay: Some(overlay),
        settings: Some(settings),
    });
    info!("command shell: pause overlay z={overlay}, pause Settings z={settings}");
}

/// The open computer outranks every modal beneath it, by NUMBER.
///
/// Not by traversal: two full-screen blockers at the same `GlobalZIndex` are
/// ordered by whatever the UI stack produces that frame, which is not a
/// decision. The pause overlay is `DespawnOnExit(Paused)` and so is already
/// gone by the time this runs - that count is RECORDED rather than asserted,
/// because it is why the old tie never showed on screen, not why it was safe.
#[cfg(feature = "debug")]
fn assert_the_computer_is_on_top(world: &mut World) {
    let layers = *world.resource::<PauseLayers>();
    let overlay = layers.overlay.expect("the pause overlay's z was recorded");
    let settings = layers
        .settings
        .expect("the pause Settings root's z was recorded");
    let backdrop = named_z(world, NOVA_OS_BACKDROP).expect("the NOVA OS backdrop carries a z");
    let monitor = named_z(world, NOVA_OS_MONITOR).expect("the NOVA OS monitor carries a z");
    assert!(
        backdrop > settings,
        "the open computer's dim field must sit strictly above the pause Settings panel \
         (backdrop {backdrop}, settings {settings})"
    );
    assert!(
        monitor > backdrop,
        "the computer must sit strictly above its own dim field \
         (monitor {monitor}, backdrop {backdrop})"
    );
    let standing = count_named(world, PAUSE_OVERLAY);
    info!(
        "command shell: layers pause={overlay} settings={settings} \
         backdrop={backdrop} monitor={monitor}, pause overlays standing={standing}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the open computer sits above the modals it covers",
        serde_json::json!({
            "pause": overlay,
            "pause_settings": settings,
            "nova_os_backdrop": backdrop,
            "nova_os": monitor,
            "pause_overlays_standing": standing,
        }),
    );
}

/// Closing over the pause menu gives the PAUSE MENU back, not the flight.
#[cfg(feature = "debug")]
fn assert_the_pause_menu_came_back(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::Paused,
        "a shell opened from the pause menu must close back onto the pause menu"
    );
    assert_eq!(
        *world.resource::<State<GameStates>>().get(),
        GameStates::Playing,
        "closing the shell must not leave the scenario it was opened over"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the shell gives back the pause menu it covered",
        serde_json::json!({}),
    );
}

/// `:` reaches the main menu, and opens over the menu rather than a game.
#[cfg(feature = "debug")]
fn assert_the_menu_shell_opened(world: &mut World) {
    assert_eq!(
        *world.resource::<State<GameStates>>().get(),
        GameStates::MainMenu,
        "the main-menu shell must open while the main menu owns the screen"
    );
    let shell = world.resource::<NovaOsTerminal>().active_shell();
    assert_eq!(
        shell,
        ShellKind::Commands,
        "`:` must open the command shell on the menu too"
    );
    info!(
        "command shell: `:` opened {} on the main menu",
        shell.prompt_prefix().trim()
    );
    nova_probe::probe_marker(
        world,
        "outcome: `:` opens the command shell on the main menu",
        serde_json::json!({ "shell": shell.prompt_prefix() }),
    );
}

/// Nobody is holding the menu's clocks - checked every frame the shell is up.
#[cfg(feature = "debug")]
fn assert_nothing_holds_the_menu(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::NovaOs,
        "the beat is only meaningful while the computer is open"
    );
    let freeze = *world.resource::<ClockFreeze>();
    assert!(
        !freeze.is_held(),
        "the command shell over the main menu must leave the ambience running \
         (held by {:?})",
        freeze.owners().collect::<Vec<_>>()
    );
    assert!(
        !world.resource::<Time<Virtual>>().is_paused(),
        "the main menu's clock must keep running under the open shell"
    );
}

/// The open computer takes the pick: the button under it gets nothing.
#[cfg(feature = "debug")]
fn assert_the_menu_was_blocked(world: &mut World) {
    let over_the_button = pointer_over_node(NEW_GAME_BUTTON);
    assert!(
        !over_the_button(world),
        "the open computer must take the pick the menu button would otherwise get"
    );
    assert_eq!(
        *world.resource::<State<GameStates>>().get(),
        GameStates::MainMenu,
        "a click through the open computer must not start a game"
    );
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::NovaOs,
        "the computer must still be the active modal after the blocked click"
    );
    info!("command shell: the open computer blocked a click on New Game");
    nova_probe::probe_marker(
        world,
        "outcome: the open computer blocks the menu behind it",
        serde_json::json!({ "button": NEW_GAME_BUTTON }),
    );
}

/// ...and the same click lands once the computer is gone: New Game opens the
/// world setup modal over the menu.
#[cfg(feature = "debug")]
fn assert_the_menu_takes_clicks_again(world: &mut World) {
    assert!(
        ui_node_present(CREATE_WORLD_BUTTON)(world),
        "the menu must take the click the closed computer no longer blocks"
    );
    info!("command shell: PASS the menu took New Game once the computer closed");
    nova_probe::probe_marker(
        world,
        "outcome: the menu takes the click again once the computer closes",
        serde_json::json!({ "button": NEW_GAME_BUTTON }),
    );
}
