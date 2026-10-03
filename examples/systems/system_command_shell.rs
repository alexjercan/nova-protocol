//! system_command_shell: the `cmd>` NOVA COMMANDS modal driven end to end,
//! off-screen.
//!
//! This range proves that what is typed at the `:` modal REACHES the live world
//! and comes back, and that `:` is a GLOBAL gesture - flight, the pause menu,
//! the main menu and the TAB interface - which gives back the exact surface it
//! covered.
//!
//! Two of those were live bugs. Escape out of a shell opened over flight
//! unpaused a player who had been paused; and the shell opened on the main menu
//! stopped the ambience behind it, leaving the front door on a still frame. A
//! shell that answers correctly but strands the player who opened it is not
//! working, so the range drives the whole gesture on all three surfaces: open,
//! run, complete, close.
//!
//! The freeze is the part that is surface-dependent, and the range states both
//! halves: over flight the shell holds the simulation still, and over the main
//! menu it holds nothing - what runs there is a cinematic the shell is drawn
//! over, not a game the player is being kept out of. What the shell DOES take
//! on the menu is the pointer: it is the active modal, so the menu buttons
//! under it are unreachable until it closes, and reachable again the moment it
//! does.
//!
//! One surface refuses `:` on purpose: over the Ship pane, a section rebind
//! that is waiting for its key owns the next key, so a typed `:` goes to the
//! capture (which binds `;`) and the modal stays shut. TAB and M are held
//! back the same way: TAB is refused as the interface toggle and the capture
//! waits on, and M binds without switching the pane. Once the capture is
//! spent, `:` opens over the pane again.
//!
//! Run (no display needed):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_command_shell --features debug
//! # look for: `command shell: PASS the menu took New Game once the computer closed`.
//! ```

#[cfg(feature = "debug")]
use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
    window::PrimaryWindow,
};
#[cfg(feature = "debug")]
use nova_input::prelude::InputSource;
#[cfg(feature = "debug")]
use nova_protocol::nova_interface::{
    nova_command::prelude::CommandTerminal,
    prelude::{InterfacePaneType, ShipRuntime},
};
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

/// The main menu's first button, and the one the open modal must cover.
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
/// The dim field the open modal puts over whatever it covers.
#[cfg(feature = "debug")]
const NOVA_OS_BACKDROP: &str = "NovaOsBackdrop";
/// The modal itself.
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

    // The virtual window: typing is read off `KeyboardInput`, which carries the
    // window it was typed into. It is also what `bevy_ui` lays out against and
    // what `bevy_picking` aims into, which is how the blocked-click beats below
    // are possible with no renderer at all (`system_headless_pointer` is the
    // worked proof).
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
                *pause.get() == PauseStates::Commands
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The freeze is the terminal's, by name. A shell over FLIGHT stops
            // the world: the player is reading a computer, not flying.
            .step("command shell: the shell over flight holds the world still")
            .until(resource_where::<ClockFreeze>(|freeze| {
                freeze.is_held_by(FreezeOwner::Interface)
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the flight freeze")
            .on_enter(assert_flight_is_frozen)
            .add()
            .step("command shell: the intro drained")
            .until(resource_where::<CommandTerminal>(|terminal| {
                terminal.is_revealed() && !terminal.has_pending_boot_rows()
            }))
            .deadline(STEP_DEADLINE_SECS)
            .add()
            // The round trip: the line is parsed here, run against the live
            // world by `nova_console`, and printed back into this transcript.
            .step("command shell: `ships` reaches the live world")
            .on_enter(type_text("ships"))
            .until(resource_where::<CommandTerminal>(|terminal| {
                terminal.prompt() == "ships"
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the answer names the player's hull")
            .on_enter(press_edit_key(Key::Enter))
            .until(resource_where::<CommandTerminal>(|terminal| {
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
            .until(resource_where::<CommandTerminal>(|terminal| {
                terminal.prompt() == format!("ship {PLAYER_ID_PREFIX}")
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: Tab finishes it from the live world")
            .on_enter(press_edit_key(Key::Tab))
            .until(resource_where::<CommandTerminal>(|terminal| {
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
            // The rebind gate. A section waiting for its key owns the next
            // key, so `:` typed then goes to the capture and must not open the
            // shell over the pane.
            .step("command shell: TAB opens the interface over flight")
            .on_enter(press_key(KeyCode::Tab))
            .until(the_pause_axis_is(PauseStates::Interface))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release TAB")
            .on_enter(release_key(KeyCode::Tab))
            .add()
            .step("command shell: M switches to the Ship pane")
            .on_enter(press_key(KeyCode::KeyM))
            .until(resource_where::<InterfacePaneType>(|pane| {
                *pane == InterfacePaneType::Ship
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release M")
            .on_enter(release_key(KeyCode::KeyM))
            .add()
            // The pane selects its first section, which may carry no trigger
            // to rebind. B does nothing there, so the walk steps the selection
            // and presses B until one takes it.
            .step("command shell: B arms the rebind on a bindable section")
            .each(|world: &mut World, _, frame| match frame % 8 {
                1 => press_key(KeyCode::BracketRight)(world),
                2 => release_key(KeyCode::BracketRight)(world),
                3 => press_key(KeyCode::KeyB)(world),
                4 => release_key(KeyCode::KeyB)(world),
                _ => {}
            })
            .until(a_ship_rebind_is_armed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The capture holds until every key is up, so the press that armed
            // it is not the one it takes.
            .step("command shell: let the arming keys go")
            .on_enter(release_key(KeyCode::BracketRight))
            .on_enter(release_key(KeyCode::KeyB))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // TAB and M are interface keys, but the armed capture owns the
            // next key too: TAB is refused as the interface toggle, so the
            // pane stays open and the capture stays armed.
            .step("command shell: TAB while the rebind is armed")
            .on_enter(press_key(KeyCode::Tab))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: TAB left the pane and the capture alone")
            .on_enter(release_key(KeyCode::Tab))
            .on_enter(assert_tab_kept_the_armed_rebind)
            .add()
            .step("command shell: Shift+; while the rebind is armed")
            .on_enter(record_section_bindings)
            .on_enter(shift_semicolon(ButtonState::Pressed))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the armed rebind took the key, not the shell")
            .on_enter(shift_semicolon(ButtonState::Released))
            .on_enter(assert_the_armed_rebind_took_the_key)
            .add()
            .step("command shell: B arms the rebind again")
            .on_enter(press_key(KeyCode::KeyB))
            .until(a_ship_rebind_is_armed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: let B go")
            .on_enter(release_key(KeyCode::KeyB))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: M while the rebind is armed")
            .on_enter(record_section_bindings)
            .on_enter(press_key(KeyCode::KeyM))
            .until(frames(SETTLE_FRAMES))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: the armed rebind took M, not the pane switch")
            .on_enter(release_key(KeyCode::KeyM))
            .on_enter(assert_the_armed_rebind_took_m)
            .add()
            .step("command shell: `:` opens the shell over the Ship pane")
            .on_enter(type_text(":"))
            .until(the_pause_axis_is(PauseStates::Commands))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: report the pane shell")
            .on_enter(|world: &mut World| {
                nova_probe::probe_marker(
                    world,
                    "outcome: `:` opens the command shell over the Ship pane once the rebind is spent",
                    serde_json::json!({}),
                );
            })
            .add()
            .step("command shell: Escape gives the Ship pane back")
            .on_enter(press_key(KeyCode::Escape))
            .until(the_pause_axis_is(PauseStates::Interface))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release Escape over the pane")
            .on_enter(release_key(KeyCode::Escape))
            .add()
            // Back to flight, so the pause-menu surface below starts from the
            // state the last one left.
            .step("command shell: TAB closes the interface")
            .on_enter(press_key(KeyCode::Tab))
            .until(the_pause_axis_is(PauseStates::Unpaused))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step("command shell: release TAB again")
            .on_enter(release_key(KeyCode::Tab))
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
            .until(the_pause_axis_is(PauseStates::Commands))
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
            .until(the_pause_axis_is(PauseStates::Commands))
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

/// Advance once the Ship pane has a section rebind waiting for its key.
#[cfg(feature = "debug")]
fn a_ship_rebind_is_armed() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<ShipRuntime>(|ship| ship.rebind_armed())
}

/// Every bindable section's desk and pad sources, by section.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, PartialEq)]
struct SectionBindings(Vec<(Entity, Vec<InputSource>)>);

/// Read every section trigger binding off the live ship.
#[cfg(feature = "debug")]
fn section_bindings(world: &mut World) -> SectionBindings {
    let mut bound = Vec::new();
    bound.extend(
        world
            .query::<(Entity, &SpaceshipThrusterInputBinding)>()
            .iter(world)
            .map(|(section, binding)| (section, binding.0.clone())),
    );
    bound.extend(
        world
            .query::<(Entity, &SpaceshipTurretInputBinding)>()
            .iter(world)
            .map(|(section, binding)| (section, binding.0.clone())),
    );
    bound.extend(
        world
            .query::<(Entity, &SpaceshipTorpedoInputBinding)>()
            .iter(world)
            .map(|(section, binding)| (section, binding.0.clone())),
    );
    bound.extend(
        world
            .query::<(Entity, &SpaceshipRailgunInputBinding)>()
            .iter(world)
            .map(|(section, binding)| (section, binding.0.clone())),
    );
    bound.extend(
        world
            .query::<(Entity, &SpaceshipMiningInputBinding)>()
            .iter(world)
            .map(|(section, binding)| (section, binding.0.clone())),
    );
    bound.sort_by_key(|(section, _)| *section);
    SectionBindings(bound)
}

/// Remember the section bindings the armed capture must leave alone.
#[cfg(feature = "debug")]
fn record_section_bindings(world: &mut World) {
    let bound = section_bindings(world);
    world.insert_resource(bound);
}

/// Shift and `;` as one chord: a `KeyboardInput` per key, the `;` carrying the
/// `:` the layout prints for it.
///
/// [`type_text`] reports an unidentified key code, which is right for a
/// character and wrong here: a capture reads the PHYSICAL keys, so the chord
/// has to name them.
#[cfg(feature = "debug")]
fn shift_semicolon(state: ButtonState) -> impl Fn(&mut World) + Send + Sync + 'static {
    move |world: &mut World| {
        let window = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
            .expect("the range spawns one primary window");
        world.write_message(KeyboardInput {
            key_code: KeyCode::ShiftLeft,
            logical_key: Key::Shift,
            state,
            text: None,
            repeat: false,
            window,
        });
        world.write_message(KeyboardInput {
            key_code: KeyCode::Semicolon,
            logical_key: Key::Character(":".into()),
            state,
            text: (state == ButtonState::Pressed).then(|| ":".into()),
            repeat: false,
            window,
        });
    }
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

/// TAB over an armed Ship rebind neither closes the interface nor drops the
/// capture: the capture refuses it as the interface toggle and waits on.
#[cfg(feature = "debug")]
fn assert_tab_kept_the_armed_rebind(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::Interface,
        "TAB over an armed section rebind must not close the interface"
    );
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Ship,
        "TAB over an armed section rebind must leave the Ship pane up"
    );
    assert!(
        world.resource::<ShipRuntime>().rebind_armed(),
        "TAB is refused, so the capture stays armed for the next key"
    );
    nova_probe::probe_marker(
        world,
        "outcome: an armed Ship rebind keeps the interface open under TAB",
        serde_json::json!({}),
    );
}

/// Shift+; over an armed Ship rebind goes to the capture: the shell stays shut
/// and exactly one section takes the `;` key.
#[cfg(feature = "debug")]
fn assert_the_armed_rebind_took_the_key(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::Interface,
        "`:` typed over an armed section rebind must not open the command shell"
    );
    assert_the_armed_section_took(world, KeyCode::Semicolon);
    nova_probe::probe_marker(
        world,
        "outcome: an armed Ship rebind takes `:` as its key and opens no shell",
        serde_json::json!({}),
    );
}

/// M over an armed Ship rebind goes to the capture: the pane stays on Ship and
/// exactly one section takes the M key.
#[cfg(feature = "debug")]
fn assert_the_armed_rebind_took_m(world: &mut World) {
    assert_eq!(
        *world.resource::<InterfacePaneType>(),
        InterfacePaneType::Ship,
        "M over an armed section rebind must not switch the pane"
    );
    assert_the_armed_section_took(world, KeyCode::KeyM);
    nova_probe::probe_marker(
        world,
        "outcome: an armed Ship rebind takes M as its key and keeps the pane",
        serde_json::json!({}),
    );
}

/// The capture disarmed, and exactly the armed section changed its bindings,
/// to `key`.
#[cfg(feature = "debug")]
fn assert_the_armed_section_took(world: &mut World, key: KeyCode) {
    assert!(
        !world.resource::<ShipRuntime>().rebind_armed(),
        "the capture must have taken the key and disarmed"
    );
    let before = world
        .remove_resource::<SectionBindings>()
        .expect("recorded");
    let after = section_bindings(world);
    let changed: Vec<_> = before
        .0
        .iter()
        .zip(&after.0)
        .filter(|(was, now)| was != now)
        .map(|(_, now)| now)
        .collect();
    assert_eq!(
        changed.len(),
        1,
        "exactly the armed section rebinds (before {before:?}, after {after:?})"
    );
    assert!(
        changed[0].1.contains(&InputSource::Keyboard(key)),
        "the captured key is the physical {key:?} (got {:?})",
        changed[0].1
    );
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
    let backdrop =
        named_z(world, NOVA_OS_BACKDROP).expect("the command modal backdrop carries a z");
    let monitor = named_z(world, NOVA_OS_MONITOR).expect("the command modal monitor carries a z");
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
    info!("command shell: `:` opened the shell on the main menu");
    nova_probe::probe_marker(
        world,
        "outcome: `:` opens the command shell on the main menu",
        serde_json::json!({}),
    );
}

/// Nobody is holding the menu's clocks - checked every frame the shell is up.
#[cfg(feature = "debug")]
fn assert_nothing_holds_the_menu(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::Commands,
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
        PauseStates::Commands,
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
