//! news_0160_world_setup: the v0.16.0 news New Game wizard loop - the world
//! step's seed chart turned under a right-drag, Next, then the role list
//! stepped from the industrial start to the bounty hunter while the hull view
//! and the card's Weapons line change.
//!
//! The walk drives the real menu under the harness pointer. The fresh profile
//! is a first launch, so the menu's bottom-left corner holds the New pilot
//! offer and a field note; the walk answers both the way a player does -
//! `Not now` on the offer, `Don't show again` on the note - so neither stands
//! in the loop. Then New Game, the seed field cleared and typed with
//! [`WORLD_SEED`], then, recorded, a right-button drag over the seed chart
//! fed as the [`MouseMotion`] a real drag writes, Next, and a click on each
//! role row from the second to [`LAST_ROLE`] in the order the list shows
//! them. It never presses Create: the loop is the wizard, and the release
//! lead is the world it makes.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk with its
//!   asserts, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write
//!   `news-0160-world-setup.webm` (staged under `NOVA_CAPTURE_DIR`).
//!
//! ```text
//! NOVA_CAPTURE_DIR=target/news-0160-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example news_0160_world_setup --features debug
//! ```

use bevy::{input::mouse::MouseMotion, prelude::*};
use clap::Parser;
use nova_protocol::prelude::*;
use nova_ui::widget::TextFieldValue;

#[derive(Parser)]
#[command(name = "news_0160_world_setup")]
#[command(version = "1.0.0")]
#[command(
    about = "Capture the v0.16.0 New Game wizard: the seed chart turned, Next, and the start roles stepped",
    long_about = None
)]
struct Cli;

/// The seed the wizard shows: the release lead's world.
const WORLD_SEED: u32 = 2;

/// The role the stepping ends on. The list opens on its first role.
const LAST_ROLE: &str = "bounty_hunter";

/// The Weapons line the first role's card must read, and a word the last
/// role's must hold: the change the figure is about.
const FIRST_WEAPONS: &str = "1 mining beam";
const LAST_WEAPONS_WORD: &str = "turret";

/// The world name a fresh profile's wizard opens with.
const WORLD_NAME: &str = "New World";

const NEW_GAME_BUTTON: &str = "New Game Button";
const NAME_FIELD: &str = "World Name Field";
const SEED_FIELD: &str = "World Seed Field";
const SEED_PREVIEW: &str = "Seed Preview";
const SEED_CAPTION: &str = "Seed Preview Caption";
const WORLD_NEXT_BUTTON: &str = "World Next Button";
const CREATE_WORLD_BUTTON: &str = "Create World Button";
const ROLE_DETAILS_NAME: &str = "Start Role Details Name";
const ROLE_HULL_LINE: &str = "Start Role Details Hull Line";
const TRAINING_PROMPT: &str = "Training Prompt";
const TRAINING_PROMPT_DISMISS: &str = "Training Prompt Dismiss";
const FIELD_NOTE: &str = "Menu Field Note";
const FIELD_NOTE_DISMISS: &str = "Menu Field Note Dismiss";

/// Backspaces that clear the seed the window opens with: it pre-fills a
/// random u32, at most ten digits.
const SEED_DIGITS_MAX: usize = 10;

/// Logical pixels the drag moves the pointer each frame, and for how many
/// frames: about a quarter turn of the chart over two seconds.
const DRAG_PX: Vec2 = Vec2::new(8.0, -1.0);
const DRAG_FRAMES: u32 = 60;

/// Frames the loop holds the world step before the drag, after it, and on
/// each role before the next click; the last role holds longer so its hull
/// turns in its view.
const OPEN_FRAMES: u32 = 30;
const AFTER_DRAG_FRAMES: u32 = 20;
const ROLE_FRAMES: u32 = 24;
const LAST_ROLE_FRAMES: u32 = 60;

/// Real seconds the menu gets to load on a software rasterizer.
const MENU_SECS: f32 = 300.0;

/// Real seconds the loop gets to encode once its last frame is in.
const LOOP_CLOSE_SECS: f32 = 60.0;

/// The loop this producer writes.
const LOOP_NAME: &str = "news-0160-world-setup";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // This run's own settings and saved worlds: the name field opens on the
    // first free name, and a player's own "New World" would make it
    // "New World 2".
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

    let mut app = editor_app(true, None);
    app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
        bevy::ecs::error::panic,
    ));
    app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
    app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::default());
    app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
    app.add_plugins(wizard_script());
    app.run()
}

fn when(
    check: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> std::sync::Arc<nova_protocol::nova_debug::harness::Predicate> {
    std::sync::Arc::new(check)
}

/// The text of the text node named `name`, if one stands.
fn text_of(world: &World, name: &str) -> Option<String> {
    world
        .try_query::<(&Name, &Text)>()?
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, text)| text.0.clone())
}

/// The value of the laid-out text field named `name`, if one stands.
fn field_value(world: &World, name: &str) -> Option<String> {
    world
        .try_query::<(&Name, &TextFieldValue)>()?
        .iter(world)
        .find(|(node, _)| node.as_str() == name)
        .map(|(_, value)| value.0.clone())
}

/// The value of the role card's Weapons line, if the card stands.
fn weapons_line(world: &World) -> Option<String> {
    let mut rows = world.try_query::<(&Name, &Children)>()?;
    let mut texts = world.try_query::<&Text>()?;
    rows.iter(world)
        .filter(|(name, _)| name.as_str() == ROLE_HULL_LINE)
        .find_map(|(_, children)| {
            let mut cells = children
                .iter()
                .filter_map(|cell| texts.get(world, cell).ok());
            let label = cells.next()?;
            let value = cells.next()?;
            (label.0 == "WEAPONS").then(|| value.0.clone())
        })
}

/// The start role ids in the order the list shows them, from the first to
/// [`LAST_ROLE`].
///
/// # Panics
///
/// When no loaded role is [`LAST_ROLE`]: the figure's last step is gone.
fn roles_to_step(world: &World) -> Vec<(String, String)> {
    let roles = world.resource::<GameStartRoles>();
    let last = roles
        .0
        .iter()
        .position(|role| role.id.as_str() == LAST_ROLE)
        .unwrap_or_else(|| panic!("world setup: no start role '{LAST_ROLE}' is loaded"));
    roles.0[..=last]
        .iter()
        .map(|role| (role.id.to_string(), role.name.clone()))
        .collect()
}

/// The corner's offer and field note are answered and laid out nowhere.
fn corner_is_answered(world: &World) -> bool {
    *world.resource::<TrainingPromptSetting>() == TrainingPromptSetting::Hidden
        && *world.resource::<FieldNoteSetting>() == FieldNoteSetting::Hidden
        && ui_node_rect(world, TRAINING_PROMPT).is_none()
        && ui_node_rect(world, FIELD_NOTE).is_none()
}

/// Move the pointer to the seed field's right edge, so the caret lands after
/// the pre-filled seed.
fn move_to_seed_field_edge(world: &mut World) {
    let rect = ui_node_rect(world, SEED_FIELD)
        .unwrap_or_else(|| panic!("world setup: the seed field '{SEED_FIELD}' is not laid out"));
    move_cursor(Vec2::new(rect.max.x - 2.0, rect.center().y))(world);
}

/// The walk: New Game, the seed typed, then the recorded drag, Next and the
/// role steps.
fn wizard_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    let seed_caption = format!("The sectors round the start of seed {WORLD_SEED}:");
    let mut script = nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .until(ui_node_present(NEW_GAME_BUTTON))
        .deadline(MENU_SECS)
        .add()
        .step("settle the menu and its backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click_named(
            "answer the New pilot offer: Not now",
            TRAINING_PROMPT_DISMISS,
            when(|world| {
                *world.resource::<TrainingPromptSetting>() == TrainingPromptSetting::Hidden
                    && ui_node_rect(world, TRAINING_PROMPT).is_none()
            }),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "switch the field note off: Don't show again",
            FIELD_NOTE_DISMISS,
            when(corner_is_answered),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "click New Game",
            NEW_GAME_BUTTON,
            ui_node_present(WORLD_NEXT_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("move to the seed field's right edge")
        .on_enter(move_to_seed_field_edge)
        .until(frames(1))
        .add()
        .step("click the seed field to place the caret")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("release and let the caret land")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("clear the pre-filled seed")
        .on_enter(|world: &mut World| {
            press_edit_key(Key::End)(world);
            for _ in 0..SEED_DIGITS_MAX {
                press_edit_key(Key::Backspace)(world);
            }
        })
        .until(frames(1))
        .add()
        .step("type the seed")
        .on_enter(type_text(WORLD_SEED.to_string()))
        .until(when(|world| {
            field_value(world, SEED_FIELD) == Some(WORLD_SEED.to_string())
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("the chart draws the typed seed")
        .until(when(move |world| {
            text_of(world, SEED_CAPTION).is_some_and(|text| text.starts_with(&seed_caption))
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("check the name and drop the overlays")
        .on_enter(|world: &mut World| {
            assert_eq!(
                field_value(world, NAME_FIELD).as_deref(),
                Some(WORLD_NAME),
                "world setup: a fresh profile's wizard must open on '{WORLD_NAME}'"
            );
            assert!(
                corner_is_answered(world),
                "world setup: the New pilot offer or the field note is still up"
            );
            hide_dev_overlays(world);
            hide_status_bar(world);
        })
        .add()
        .step("rest the pointer on the chart")
        .on_enter(hover_named(SEED_PREVIEW))
        .until(and(pointer_over_node(SEED_PREVIEW), frames(SETTLE_FRAMES)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // ---- the recorded wizard ----
        .step("open the world-setup loop")
        .on_enter(|world: &mut World| loop_start(world, LOOP_NAME))
        .add()
        .step("hold the world step")
        .until(frames(OPEN_FRAMES))
        .add()
        .step("press the right button over the chart")
        .on_enter(press_mouse(MouseButton::Right))
        .until(frames(1))
        .add()
        .step("drag the chart round")
        .each(|world: &mut World, _, _| {
            world.write_message(MouseMotion { delta: DRAG_PX });
        })
        .until(frames(DRAG_FRAMES))
        .add()
        .step("let the right button go")
        .on_enter(release_mouse(MouseButton::Right))
        .until(frames(AFTER_DRAG_FRAMES))
        .add()
        .click_named(
            "click Next",
            WORLD_NEXT_BUTTON,
            ui_node_present(CREATE_WORLD_BUTTON),
            BEAT_DEADLINE_SECS,
        )
        .step("the first role's card stands")
        .until(when(|world| {
            let first = roles_to_step(world).swap_remove(0).1;
            text_of(world, ROLE_DETAILS_NAME).as_deref() == Some(first.as_str())
                && weapons_line(world).is_some()
        }))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("check the first role's weapons")
        .on_enter(|world: &mut World| {
            assert_eq!(
                weapons_line(world).as_deref(),
                Some(FIRST_WEAPONS),
                "world setup: the first role's card must read Weapons '{FIRST_WEAPONS}'"
            );
        })
        .until(frames(ROLE_FRAMES))
        .add();
    // The roles are content, read once the menu is up; the steps are built
    // before that, so each click is resolved by its place in the list. The
    // base list puts LAST_ROLE fourth; `step role` checks the click took.
    for place in 1..BASE_ROLES_TO_LAST {
        script = script
            .step(format!("aim at role {place}"))
            .on_enter(move |world: &mut World| {
                let (id, _) = roles_to_step(world).swap_remove(place);
                hover_named(format!("Start Role Button: {id}"))(world);
            })
            .until(when(move |world| {
                let (id, _) = roles_to_step(world).swap_remove(place);
                pointer_over_node(format!("Start Role Button: {id}"))(world)
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("press role {place}"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("release role {place}"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(when(move |world| {
                let roles = roles_to_step(world);
                roles.get(place).is_some_and(|(_, name)| {
                    text_of(world, ROLE_DETAILS_NAME).as_deref() == Some(name.as_str())
                })
            }))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("hold role {place}"))
            .until(frames(if place + 1 == BASE_ROLES_TO_LAST {
                LAST_ROLE_FRAMES
            } else {
                ROLE_FRAMES
            }))
            .add();
    }
    script
        .step("check the last role")
        .on_enter(|world: &mut World| {
            let roles = roles_to_step(world);
            assert_eq!(
                roles.len(),
                BASE_ROLES_TO_LAST,
                "world setup: '{LAST_ROLE}' is no longer role {BASE_ROLES_TO_LAST} of the list"
            );
            let weapons = weapons_line(world).unwrap_or_default();
            assert!(
                weapons.contains(LAST_WEAPONS_WORD),
                "world setup: '{LAST_ROLE}' reads Weapons '{weapons}', no '{LAST_WEAPONS_WORD}'"
            );
            info!("world setup: Weapons '{FIRST_WEAPONS}' -> '{weapons}'");
        })
        .add()
        .step("close the world-setup loop")
        .on_enter(|world: &mut World| loop_end(world, LOOP_NAME))
        .until(loop_written(LOOP_NAME))
        .deadline(LOOP_CLOSE_SECS)
        .add()
}

/// How many rows the base list holds from its first role to [`LAST_ROLE`].
const BASE_ROLES_TO_LAST: usize = 4;
