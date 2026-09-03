//! system_nova_os: the Tab ship computer, driven the way a player drives it - opened
//! with a keystroke and clicked THROUGH the CRT glass.
//!
//! ONE SUBJECT: the NOVA OS as a live system. Its parts are unit-tested well
//! (`crates/nova_os_ui/src/terminal/tests/`): the shell's app lifecycle, the CRT
//! pipeline's structure and uniforms, the screen->image mapping against the
//! shader, the hover mirror, and that the RTT element renders its subtree. What
//! no test asserted before this range is the whole chain AT ONCE, in a real app:
//! press Tab, the computer opens, and a pointer landing on the glass reaches a
//! widget that lives behind an image camera (task 20260804-134347).
//!
//! That chain is the one place the monitor can fail invisibly. Every unit test
//! along it can pass while the live composite still puts a click somewhere the
//! player did not aim: the surface is `Pickable::IGNORE`, so a forwarded pointer
//! that misses hits NOTHING and the run stays green, quiet, and wrong.
//!
//! The click target is the header's `[ ESC ]` app-close control. It is a real
//! `Button` behind the image camera whose `Activate` observer returns the shell
//! to the prompt - so the claim "the click got through" is answered by the
//! terminal model rather than by the pointer agreeing with itself.
//!
//! It is `systems/`, not `screenshots/`: the product is a verdict, not a frame.
//! `screenshot_nova_os` walks the same computer to CAPTURE it, and asserts
//! nothing beyond not panicking.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example system_nova_os --features debug
//! # named beats, each waiting on the world rather than on a dwell: load the
//! # range; Tab the computer open and wait for the RASTER to finish opening
//! # (a click on a collapsing raster lands where the picture no longer is);
//! # launch the `ship` app over the real keyboard path; aim at the close
//! # control by undoing the CRT warp, and assert the forwarded pointer - not
//! # the mouse - is hovering it; press, and assert the widget behind the glass
//! # took the press; release, and wait for the SHELL to say the app closed;
//! # launch `map` and assert the switch left exactly one of every screen node.
//! # A beat that never resolves inside its deadline is an error exit naming it.
//! ```

use std::collections::BTreeMap;

use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    picking::{hover::HoverMap, pointer::PointerId},
    ui::Pressed,
    window::PrimaryWindow,
};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_protocol::nova_os_ui::{
    nova_os::prelude::{NovaOsTerminal, TerminalMode},
    prelude::{nova_os_openness, nova_os_pointer_id, nova_os_window_px_showing},
};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "system_nova_os")]
#[command(version = "1.0.0")]
#[command(about = "The Tab NOVA OS ship computer, clicked through its CRT glass. Autopilot-only correctness range", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env): run
        // timeline + engine-bound invariants, so `probe run` grades this range.
        // No frame-time capture - the walk is a sequence of gestures with no
        // steady-state window, so a captured fps would measure the script.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_os_script());
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

/// The `Name` of the widget this range clicks through the glass: the header's
/// app-close control, which lives behind the image camera and returns the shell
/// to the prompt when activated.
#[cfg(feature = "debug")]
const CLOSE_CONTROL: &str = "NovaOsAppClose";

/// The prefix every launched app's root node carries, so a ghost left behind by
/// a switch is countable by name.
#[cfg(feature = "debug")]
const APP_ROOT_PREFIX: &str = "NovaOsApp:";

/// The widget the run is aiming at, resolved once when it becomes visible so the
/// hover and press assertions name the same entity the aim did.
#[cfg(feature = "debug")]
#[derive(Resource)]
struct GlassTarget(Entity);

/// The walk, one beat per gesture or claim.
#[cfg(feature = "debug")]
fn nova_os_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // Wait for the ship to EXIST: the computer keys off the player ship
        // root, so a Tab pressed before it spawned would open nothing.
        .step("nova_os: load the range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("nova_os: press Tab")
        .on_enter(press_key(KeyCode::Tab))
        .until(state_is(PauseStates::NovaOs))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Not a dwell: the raster blooms on over real time and the CRT shows a
        // squeezed window onto the image until it settles, so a click aimed
        // mid-slide lands where the picture no longer is.
        .step("nova_os: let the raster open")
        .on_enter(release_key(KeyCode::Tab))
        .until(nova_os_raster_open())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the computer is open")
        .on_enter(assert_computer_open)
        .add()
        .step("nova_os: type the ship command")
        .on_enter(type_text("ship"))
        .until(nova_os_command_line_reads("ship"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: launch the ship app")
        .on_enter(submit_line)
        .until(nova_os_app_owns_the_screen("ship"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the ship app owns the screen")
        .on_enter(assert_app_on_screen)
        .add()
        // The WINDOW pointer arriving where the aim sent it. Not the same
        // claim as the next beat's - that one is about the FORWARDED pointer
        // reaching the offscreen tree, which this cannot and must not prove.
        .step("nova_os: aim at the close control through the glass")
        .on_enter(aim_through_the_glass)
        .until(the_pointer_reached_the_glass())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the pointer reached the offscreen tree")
        .on_enter(assert_hover_through_the_glass)
        .add()
        // Again the WINDOW mouse: the beat after it claims the press reached
        // the widget behind the glass, which is a different fact.
        .step("nova_os: press the close control")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the press landed on the widget")
        .on_enter(assert_press_through_the_glass)
        .add()
        // `Activate` fires on RELEASE over the same widget, so a click is two
        // beats. The shell returning to the prompt is the advance condition:
        // a click that missed stalls HERE, named, instead of failing later on
        // a symptom.
        .step("nova_os: release the close control")
        .on_enter(release_mouse(MouseButton::Left))
        .until(shell_is_at_the_prompt())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the click through the glass closed the app")
        .on_enter(assert_click_closed_the_app)
        .add()
        .step("nova_os: type the map command")
        .on_enter(type_text("map"))
        .until(nova_os_command_line_reads("map"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: launch the map app")
        .on_enter(submit_line)
        .until(nova_os_app_owns_the_screen("map"))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("nova_os: the app switch left one screen")
        .on_enter(assert_one_screen)
        .add()
}

/// Advance once the WINDOW pointer stands where [`aim_through_the_glass`] sent
/// it - the warp resolved again from the live layout, so a panel that moved
/// under the beat holds it open instead of letting a stale coordinate through.
#[cfg(feature = "debug")]
fn the_pointer_reached_the_glass() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate>
{
    std::sync::Arc::new(|world: &World| {
        let Some(image_px) = ui_node_centre(world, CLOSE_CONTROL) else {
            return false;
        };
        let Some(window_px) = nova_os_window_px_showing(world, image_px) else {
            return false;
        };
        pointer_at(window_px)(world)
    })
}

/// Advance once the shell is back at the command prompt.
#[cfg(feature = "debug")]
fn shell_is_at_the_prompt() -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    resource_where::<NovaOsTerminal>(|terminal| terminal.active_mode() == TerminalMode::Prompt)
}

/// Submit the current command line.
///
/// The autopilot's vocabulary types CHARACTERS ([`type_text`]); Enter is a named
/// key, and the terminal reads it off the keyboard message's `logical_key` like
/// any other text field would.
#[cfg(feature = "debug")]
fn submit_line(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let Ok(window) = query.single(world) else {
        warn!("nova_os: submitting the command line has no primary window");
        return;
    };
    world.write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
}

/// Put the real cursor where the CRT displays the close control.
///
/// The two halves a driven run needs and cannot get from the pointer vocabulary
/// alone: the widget reports its rect in IMAGE pixels (the space the offscreen
/// camera laid it out in), and `nova_os_window_px_showing` undoes the warp to
/// say which WINDOW pixel shows that point. Aiming at the image rect directly
/// would put the cursor a few hundred pixels off, somewhere in the cockpit.
#[cfg(feature = "debug")]
fn aim_through_the_glass(world: &mut World) {
    let image_px = ui_node_centre(world, CLOSE_CONTROL)
        .unwrap_or_else(|| panic!("no laid-out, visible `{CLOSE_CONTROL}` to aim at"));
    let window_px = nova_os_window_px_showing(world, image_px).unwrap_or_else(|| {
        panic!("the CRT displays image px {image_px:?} nowhere on the glass - nothing to aim at")
    });
    move_cursor(window_px)(world);
    info!("nova_os: aiming at image px {image_px:?} via window px {window_px:?}");
}

/// Tab reached the shared freeze axis AND the tube finished opening.
#[cfg(feature = "debug")]
fn assert_computer_open(world: &mut World) {
    assert_eq!(
        *world.resource::<State<PauseStates>>().get(),
        PauseStates::NovaOs,
        "Tab must open the computer by driving the shared pause axis"
    );
    let openness = nova_os_openness(world).expect("an open computer has a shell to be open");
    assert!(
        openness >= 1.0 - f32::EPSILON,
        "the raster must be fully open before anything is clicked on it; it is at {openness}"
    );
    nova_probe::probe_marker(
        world,
        "outcome: tab opens the computer",
        serde_json::json!({ "openness": openness }),
    );
    info!("nova_os: the computer is open at raster {openness}");
}

/// The launched app is on the screen, and the control this run clicks is
/// visible - it is hidden at the prompt, so resolving it is itself the claim
/// that an app owns the screen.
#[cfg(feature = "debug")]
fn assert_app_on_screen(world: &mut World) {
    let roots = app_roots(world);
    assert_eq!(
        roots,
        vec!["NovaOsApp:ship".to_string()],
        "launching `ship` must put exactly its app root on the screen"
    );
    let close = named_entities(world, CLOSE_CONTROL);
    let [close] = close[..] else {
        panic!(
            "the header carries exactly one `{CLOSE_CONTROL}`; found {}",
            close.len()
        )
    };
    assert!(
        ui_node_rect(world, CLOSE_CONTROL).is_some(),
        "the close control must be laid out and visible while an app owns the screen"
    );
    world.insert_resource(GlassTarget(close));
    nova_probe::probe_marker(
        world,
        "outcome: the ship app owns the screen",
        serde_json::json!({}),
    );
}

/// The forwarded pointer is hovering the widget - i.e. the cursor did not just
/// land on the glass, it reached a node BEHIND the image camera.
///
/// The mouse pointer is asserted NOT to have it, which is what makes this claim
/// about the forwarding rather than about the pointer being somewhere: window
/// picking cannot reach an image-camera node at all, so a mouse hit here would
/// mean the target was never offscreen and the whole range proved nothing.
#[cfg(feature = "debug")]
fn assert_hover_through_the_glass(world: &mut World) {
    let target = world.resource::<GlassTarget>().0;
    assert!(
        pointer_reached(world, nova_os_pointer_id(), target),
        "the forwarded NOVA OS pointer must be hovering `{CLOSE_CONTROL}`; it is \
         hovering {:?}",
        hovered_names(world, nova_os_pointer_id())
    );
    assert!(
        !pointer_reached(world, PointerId::Mouse, target),
        "`{CLOSE_CONTROL}` must be reachable only THROUGH the image - the window \
         mouse pointer hit it, so it is not behind the image camera at all"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the pointer reaches the offscreen subtree",
        serde_json::json!({}),
    );
    info!("nova_os: the forwarded pointer is hovering `{CLOSE_CONTROL}`");
}

/// Whether `pointer` is over `target` or anything inside it.
///
/// The ancestor walk is the whole point: `bevy_ui`'s picking reports the
/// DEEPEST node under the pointer, which for a labelled button is its text, and
/// a caller comparing hits to the widget itself would read a perfectly good
/// click as a miss. `mirror_nova_os_hover` walks the same way for the same
/// reason.
#[cfg(feature = "debug")]
fn pointer_reached(world: &World, pointer: PointerId, target: Entity) -> bool {
    let Some(hits) = world.resource::<HoverMap>().get(&pointer) else {
        return false;
    };
    hits.keys().any(|hit| {
        std::iter::successors(Some(*hit), |entity| {
            world.get::<ChildOf>(*entity).map(|child| child.parent())
        })
        .any(|entity| entity == target)
    })
}

/// What `pointer` is over, named where the nodes carry one - for a failure
/// message that says where the aim went instead of printing entity ids.
#[cfg(feature = "debug")]
fn hovered_names(world: &World, pointer: PointerId) -> Vec<String> {
    let Some(hits) = world.resource::<HoverMap>().get(&pointer) else {
        return Vec::new();
    };
    hits.keys()
        .map(|hit| {
            std::iter::successors(Some(*hit), |entity| {
                world.get::<ChildOf>(*entity).map(|child| child.parent())
            })
            .find_map(|entity| world.get::<Name>(entity).map(|name| name.to_string()))
            .unwrap_or_else(|| format!("{hit} (unnamed, and so are its ancestors)"))
        })
        .collect()
}

/// The press reached the widget, not just the pointer: `Pressed` is what
/// `bevy_ui_widgets` puts on a `Button` it dispatched a press to, and it is the
/// state `Activate` is gated on when the release arrives.
#[cfg(feature = "debug")]
fn assert_press_through_the_glass(world: &mut World) {
    let target = world.resource::<GlassTarget>().0;
    assert!(
        world.get::<Pressed>(target).is_some(),
        "`{CLOSE_CONTROL}` must be holding the press that came through the glass"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the press lands on the widget behind the glass",
        serde_json::json!({}),
    );
}

/// The release activated the widget: the shell is back at the prompt and the
/// app root is gone with it.
#[cfg(feature = "debug")]
fn assert_click_closed_the_app(world: &mut World) {
    assert_eq!(
        world.resource::<NovaOsTerminal>().active_mode(),
        TerminalMode::Prompt,
        "activating the close control must return the shell to the prompt"
    );
    let roots = app_roots(world);
    assert!(
        roots.is_empty(),
        "the closed app must take its root node with it; {roots:?} survived"
    );
    nova_probe::probe_marker(
        world,
        "outcome: the click through the glass closes the app",
        serde_json::json!({}),
    );
    info!("nova_os: the click through the glass closed the ship app");
}

/// One screen after an app switch: the map app owns it, the ship app left
/// nothing behind, and every node the monitor is built from is still singular.
///
/// A duplicate here is the failure mode a structural unit test cannot see: the
/// shell despawns and respawns its surfaces across a switch, and a second
/// content root or sampling surface renders a ghost that a screenshot shows and
/// a node-shape assertion does not.
#[cfg(feature = "debug")]
fn assert_one_screen(world: &mut World) {
    let roots = app_roots(world);
    assert_eq!(
        roots,
        vec!["NovaOsApp:map".to_string()],
        "the switch must leave exactly the map app on the screen"
    );
    for name in [
        "NovaOsScreen",
        "NovaOsCrtSurface",
        "NovaOsImageContentRoot",
        "NovaOsMain",
        "NovaOsTerminalContent",
        CLOSE_CONTROL,
    ] {
        let found = named_entities(world, name).len();
        assert_eq!(
            found, 1,
            "the monitor must carry exactly one `{name}` after an app switch; found {found}"
        );
    }
    nova_probe::probe_marker(
        world,
        "outcome: the app switch leaves one screen",
        serde_json::json!({ "app": "map" }),
    );
    info!("nova_os: the ship -> map switch left one of every screen node");
}

/// Every entity called `name`, laid out or not, visible or not.
///
/// Deliberately NOT [`ui_node_rect`]: a ghost left behind by a teardown is
/// usually hidden or unlaid-out, which is exactly the case that resolve would
/// filter away before it could be counted.
#[cfg(feature = "debug")]
fn named_entities(world: &mut World, name: &str) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .filter(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .collect()
}

/// The names of every launched app's root node, sorted.
#[cfg(feature = "debug")]
fn app_roots(world: &mut World) -> Vec<String> {
    let mut query = world.query::<&Name>();
    let mut roots: Vec<String> = query
        .iter(world)
        .filter(|name| name.as_str().starts_with(APP_ROOT_PREFIX))
        .map(|name| name.to_string())
        .collect();
    roots.sort();
    roots
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_range(&game_assets, &sections)));
}

/// One named player ship at the origin - all the computer needs (it keys off the
/// player ship root) and enough sections for the `ship` app to have a schematic
/// to draw, so launching it exercises the app's own render-to-texture scene
/// rather than an empty viewport.
fn nova_os_range(game_assets: &GameAssets, sections: &GameSections) -> ScenarioConfig {
    let section = |id: &str| {
        sections
            .get_section(id)
            .unwrap_or_else(|| panic!("section '{id}' not found"))
            .clone()
    };
    let at = |id: &str, kind: &str, position: Vec3| SpaceshipSectionConfig {
        id: id.to_string(),
        position,
        rotation: Quat::IDENTITY,
        source: SectionSource::Inline(section(kind)),
        modifications: vec![],
    };

    let player = SpaceshipConfig {
        allegiance: None,
        controller: SpaceshipController::Player(PlayerControllerConfig {
            input_mapping: BTreeMap::new(),
            speed_cap: None,
        }),
        hull: ShipSource::Inline(ShipHull {
            sections: vec![
                at(
                    "player_controller",
                    "basic_controller_section",
                    Vec3::new(0.0, 0.0, 0.0),
                ),
                at(
                    "player_hull",
                    "reinforced_hull_section",
                    Vec3::new(0.0, 0.0, 1.0),
                ),
                at(
                    "player_thruster",
                    "basic_thruster_section",
                    Vec3::new(0.0, 0.0, 2.0),
                ),
                SpaceshipSectionConfig {
                    id: "player_turret".to_string(),
                    // Seated on the hull's +X face. The shared PDC bolts down by
                    // its base plate alone, so it sits a quarter-cell in from
                    // that face rather than a whole cell out, and is rolled to
                    // stand out of it.
                    position: Vec3::new(0.75, 0.0, 1.0),
                    rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
                    source: SectionSource::Inline(section("pdc_kinetic_turret_section")),
                    modifications: vec![],
                },
            ],
            ..default()
        }),
        ..default()
    };

    let events = vec![ScenarioEventConfig {
        label: None,
        name: EventConfig::OnStart,
        once: false,
        filters: vec![],
        // The scene lights itself: the engine spawns no light, so a scenario
        // that authors none renders black.
        actions: [
            vec![EventActionConfig::SpawnScenarioObject(
                ScenarioObjectConfig {
                    base: BaseScenarioObjectConfig {
                        id: "player_ship".to_string(),
                        name: "Ceres Queen".to_string(),
                        position: Meters3::ZERO,
                        rotation: Quat::IDENTITY,
                    },
                    kind: ScenarioObjectKind::Spaceship(player),
                },
            )],
            ThreePointRig::around("range", Meters3::ZERO, 1.0).actions(),
        ]
        .concat(),
    }];

    ScenarioConfig {
        description: "A range for driving the NOVA OS ship computer.".to_string(),
        events,
        ..ScenarioConfig::new(
            "nova_os_range".to_string(),
            "NOVA OS Range".to_string(),
            game_assets.cubemap.clone().into(),
        )
    }
}
