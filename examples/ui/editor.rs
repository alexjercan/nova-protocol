//! editor: the ship editor, BUILT and INSPECTED through synthesized pointer input.
//!
//! This runs the exact same editor the `nova_protocol` binary launches (via the shared
//! [`editor_app`]), just with the autopilot + screenshot harness attached. Every beat is a real
//! gesture at a real screen position - the run clicks the menu, the rail buttons and the component
//! cards by `Name`, and it clicks the ship itself by projecting a section onto the viewport. No
//! editor code is reached by triggering its observer or by inserting its state component.
//!
//! The arc, one beat per gesture:
//!
//! 1. click Sandbox in the main menu, which is also the smoke coverage for the menu itself;
//! 2. click New Ship - which keeps the editor-preview controller fix (task 20260706-212909) honest:
//!    a live controller on the non-physics preview root used to flood the log with "root not found"
//!    every frame, so this run staying quiet is the regression check;
//! 3. hover a hull component card and assert the tooltip NAMES that section - the editor's one
//!    surface that identifies a section to the player;
//! 4. click that card, then place TWO sections by clicking the ship through the real picking
//!    pipeline (avian's physics-picking backend raycasts the pointer to a hit, and the editor's own
//!    `on_click_spaceship_section` observer places the section);
//! 5. click Select / Rebind and click the ship again - select mode must place NOTHING;
//! 6. click Delete Section and click the ship again - the count drops back.
//!
//! Controls (interactive run): use the on-screen buttons to create ships and place sections.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example editor --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `editor: ...` verdict lines per beat,
//! #           `autopilot: cycle complete, no panic`
//! ```

// Only the debug-gated autopilot below names bevy types directly.
#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "editor")]
#[command(version = "1.0.0")]
#[command(about = "The nova_protocol ship editor, wired to the smoke-test harness", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true);

    // Headless smoke-test harness: inert in a normal run (gated on NOVA_AUTOPILOT / NOVA_SHOT).
    #[cfg(feature = "debug")]
    {
        // Probe wiring (task 20260719-210443; each plugin is inert without
        // its NOVA_PERF_* env): run timeline + engine-bound invariants +
        // frame-time capture, so `probe run` can measure this example.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.init_resource::<EditorProbe>();
        app.add_plugins(editor_script());
        app.add_plugins(nova_screenshot());
    }

    app.run()
}

/// Frames a beat waits for a gesture to land: the picking backend needs a frame
/// to raycast the new pointer position and the editor's observers a frame to
/// react. Generous rather than tight - this runs on a software-rendered CI GPU.
#[cfg(feature = "debug")]
const SETTLE: u32 = 10;

/// Frames the run waits after creating a ship, for the preview to spawn and for
/// avian to prepare its section colliders before anything is clicked.
#[cfg(feature = "debug")]
const SHIP_SETTLE: u32 = 40;

/// What a beat measured, so a later beat can say whether the gesture changed
/// anything.
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct EditorProbe {
    /// The hull section the run places, resolved from the catalog once the
    /// component cards are up.
    hull: String,
    /// Live section count, stamped by the beat before the one that checks it.
    sections: usize,
}

/// The whole driven run.
///
/// A gesture beat and its VERDICT beat are separate on purpose: the gesture's
/// effect is only visible after the frames the editor needs, and a verdict that
/// panics names the beat it belongs to instead of stalling the step it was
/// folded into.
#[cfg(feature = "debug")]
fn editor_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("editor: reach the main menu")
        .until(state_is(GameStates::MainMenu))
        .add()
        .step("editor: let the menu lay out")
        .until(frames(SETTLE))
        .add()
        .step("editor: click Sandbox")
        .on_enter(click_named("Sandbox Button"))
        .until(frames(SETTLE))
        .add()
        // The menu buttons act on `Activate`, which fires on RELEASE over the
        // same widget - so a click is two beats throughout this script.
        .step("editor: release Sandbox")
        .on_enter(release_mouse(MouseButton::Left))
        .until(state_is(GameStates::Playing))
        .add()
        .step("editor: let the editor lay out")
        .until(frames(SETTLE))
        .add()
        .step("editor: click New Ship")
        .on_enter(click_named("Create New Spaceship Button V2"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release New Ship")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SHIP_SETTLE))
        .add()
        .step("editor: the ship came up with a controller")
        .on_enter(|world: &mut World| {
            // `create_new_spaceship_with_controller` spawns the preview with
            // exactly its controller section - the editor's own marker types are
            // crate-private, so the section COUNT is what an example can see, and
            // it is the same claim: the ship exists and it is not empty.
            let sections = count_sections(world);
            assert_eq!(
                sections, 1,
                "New Ship must create a ship carrying its controller section"
            );
            let hull = hull_section_name(world).expect("the catalog lists a hull section");
            info!("editor: ship created, will build with `{hull}`");
            world.resource_mut::<EditorProbe>().hull = hull;
        })
        .until(frames(1))
        .add()
        // Inspect: hovering a component card is the editor's one surface that
        // NAMES a section to the player.
        .step("editor: hover the hull card")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            hover_named(hull)(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: the tooltip names the section")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            let named = tooltip_text(world);
            assert!(
                named.contains(&hull),
                "hovering the `{hull}` card must raise a tooltip naming it; the \
                 tooltip read {named:?}"
            );
            info!("editor: tooltip names `{hull}`");
        })
        .until(frames(1))
        .add()
        .step("editor: click the hull card")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            click_named(hull)(world);
        })
        .until(frames(SETTLE))
        .add()
        // The cards carry `ButtonValue<SectionChoice>`, which `button_on_setting`
        // applies on `Add<Pressed>` - so the tool is already chosen here, and the
        // release only lets go of the card.
        .step("editor: release the hull card")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        // `SectionChoice` - the armed tool - is crate-private to `nova_editor`, so
        // the example cannot read it. The arming is proven the only way it is
        // observable from outside: the next two clicks on the ship place sections,
        // and the same clicks place nothing once Select mode disarms it.
        .step("editor: stamp the count before building")
        .on_enter(stamp_sections)
        .until(frames(1))
        .add()
        .click_the_ship("editor: place the first section")
        .click_the_ship("editor: place the second section")
        .step("editor: two sections were placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 2,
                "two pointer clicks on the ship must place two sections"
            );
            info!("editor: placed 2 sections ({before} -> {now})");
            stamp_sections(world);
        })
        .until(frames(1))
        .add()
        .step("editor: click Select / Rebind")
        .on_enter(click_named("Select Section Button"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Select / Rebind")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .click_the_ship("editor: click the ship in select mode")
        .step("editor: select mode placed nothing")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "select mode must not place anything; the same click built a \
                 section a moment ago"
            );
            info!("editor: select mode is inert for placement ({now} sections)");
        })
        .until(frames(1))
        .add()
        .step("editor: click Delete Section")
        .on_enter(click_named("Delete Section Button"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Delete Section")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .click_the_ship("editor: click the ship in delete mode")
        .step("editor: the count dropped back")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before - 1,
                "a click in delete mode must remove exactly the section under \
                 the pointer"
            );
            info!("editor: deleted a section ({before} -> {now})");
        })
        .until(frames(1))
        .add()
}

/// The display name of any hull section in the catalog (the section the run places).
#[cfg(feature = "debug")]
fn hull_section_name(world: &World) -> Option<String> {
    world
        .resource::<GameSections>()
        .iter()
        .find(|section| matches!(section.kind, SectionKind::Hull(_)))
        .map(|section| section.base.name.clone())
}

/// Count the preview ship's sections.
#[cfg(feature = "debug")]
fn count_sections(world: &mut World) -> usize {
    let mut q = world.query_filtered::<(), With<SectionMarker>>();
    q.iter(world).count()
}

/// Record the live section count, so the beat after the next gesture can say
/// what that gesture changed.
#[cfg(feature = "debug")]
fn stamp_sections(world: &mut World) {
    let count = count_sections(world);
    world.resource_mut::<EditorProbe>().sections = count;
}

/// Every line of text in the component tooltip, if one is up.
#[cfg(feature = "debug")]
fn tooltip_text(world: &mut World) -> Vec<String> {
    let Some(tooltip) = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, name)| name.as_str() == "Component Tooltip")
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut stack = vec![tooltip];
    while let Some(entity) = stack.pop() {
        if let Some(text) = world.get::<Text>(entity) {
            lines.push(text.0.clone());
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    lines
}

/// The viewport position (logical px) of a preview section, or `None` until a
/// section, the 3D camera and the window all exist.
///
/// `Camera::world_to_viewport` answers in LOGICAL pixels, which is the space
/// [`move_cursor`] takes - no scale-factor conversion belongs here.
#[cfg(feature = "debug")]
fn aim_at_a_section(world: &mut World) -> Option<Vec2> {
    let mut q_sections = world.query_filtered::<&GlobalTransform, With<SectionMarker>>();
    let section_pos = q_sections.iter(world).next()?.translation();

    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?;
    let camera_transform = world.get::<GlobalTransform>(camera_entity)?;
    camera.world_to_viewport(camera_transform, section_pos).ok()
}

/// The three-beat gesture the run performs on the SHIP rather than on the UI:
/// aim the pointer at a section, press, release.
///
/// An extension trait rather than a free function so a click on the ship reads
/// in the script exactly like a click on a widget does. The editor acts on
/// `Pointer<Press>`, so the press is what does the work and the release only
/// lets go.
///
/// Named for the GESTURE, not for placement: the same three beats also drive
/// the select-mode and delete-mode clicks, where placing nothing is the claim
/// (review R1.6).
#[cfg(feature = "debug")]
trait ClickTheShip {
    fn click_the_ship(self, label: &str) -> Self;
}

#[cfg(feature = "debug")]
impl ClickTheShip for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn click_the_ship(self, label: &str) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(|world: &mut World| {
                let at = aim_at_a_section(world)
                    .expect("a preview section, the 3D camera and the window are all up");
                move_cursor(at)(world);
            })
            .until(frames(SETTLE))
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(frames(SETTLE))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(SETTLE))
            .add()
    }
}
