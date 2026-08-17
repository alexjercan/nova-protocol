//! ship_editor: the ship editor, BUILT and INSPECTED through synthesized pointer input.
//!
//! This runs the exact same editor the `nova_protocol` binary launches (via the shared
//! [`editor_app`]), just with the autopilot + screenshot harness attached. Every beat is a real
//! gesture at a real screen position - the run clicks the menu, the rail buttons and the gallery
//! tiles by `Name`, and it clicks the ship itself by projecting a section onto the viewport. No
//! editor code is reached by triggering its observer or by inserting its state component.
//!
//! The arc, one beat per gesture:
//!
//! 1. click Sandbox in the main menu, which is also the smoke coverage for the menu itself;
//! 2. click New Ship - which keeps the editor-preview controller fix (task 20260706-212909) honest:
//!    a live controller on the non-physics preview root used to flood the log with "root not found"
//!    every frame, so this run staying quiet is the regression check;
//! 3. arm a hull through the parts gallery - the editor's only parts picker;
//! 4. place TWO sections by clicking the ship through the real picking pipeline (avian's
//!    physics-picking backend raycasts the pointer to a hit, and the editor's own
//!    `on_click_spaceship_section` observer places the section);
//! 5. click Select / Rebind and click the ship again - select mode must place NOTHING;
//! 6. click Delete Section and click the ship again - the count drops back;
//! 7. walk the gallery end to end - browse (the tiles are up), filter (typing narrows the grid to
//!    one part), focus (the card names that part), select (Place arms the tool and closes the
//!    gallery) and place (a click on the ship builds the part the gallery picked);
//! 8. meet both placement REFUSALS in words and in pixels - an occupied socket, and a drive
//!    aimed up a lane the hull already stands beside, which is `nova_ship`'s clearance rule and
//!    the same one the ship generator collapses under;
//! 9. turn the SKIN on and watch it follow the build - the bare ship, the same ship clad from
//!    its own structure, and the cladding reflowing around a hull that is still in the
//!    builder's hand. Play then proves the toggle rode the hand-off: the flown ship wears it.
//!
//! Controls (interactive run): use the on-screen buttons to create ships and place sections.
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example ship_editor --features debug
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
#[command(name = "ship_editor")]
#[command(version = "1.0.0")]
#[command(about = "The nova_protocol ship editor, wired to the smoke-test harness", long_about = None)]
struct Cli;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same editor app the game/binary runs - not a bespoke copy.
    let mut app = editor_app(true, None);

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

/// The hull the run builds with. The same id `create_new_spaceship` seeds a
/// hull ship from, so a catalog that dropped it has already broken the editor.
#[cfg(feature = "debug")]
const HULL_PROTOTYPE: &str = "reinforced_hull_section";

/// A viewport point (logical px) with neither the ship nor a rail panel under
/// it, on the 1024x768 window the app opens. Pointing here is how a beat puts
/// the ghost away without disarming the part it is holding.
#[cfg(feature = "debug")]
const EMPTY_SPACE: Vec2 = Vec2::new(760.0, 640.0);

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
    /// Gallery tiles on screen, stamped before the filter beat narrows them.
    tiles: usize,
    /// Mates the editor's assembled ship derives, to compare against the flown
    /// ship's.
    mates: usize,
    /// Skin plates on the build, stamped before the beat that drags a part
    /// through them.
    plates: usize,
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
            nova_probe::probe_marker(
                world,
                "outcome: new ship carries its controller",
                serde_json::json!({}),
            );
            info!("editor: ship created, will build with `{hull}`");
            world.resource_mut::<EditorProbe>().hull = hull;
        })
        .until(frames(1))
        .add()
        // Arm the hull through the gallery - the editor's only parts picker now
        // that the component drawer is gone.
        .arm_from_the_gallery("editor: arm the hull", HULL_PROTOTYPE)
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
            nova_probe::probe_marker(
                world,
                "outcome: two clicks place two sections",
                serde_json::json!({}),
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
            nova_probe::probe_marker(
                world,
                "outcome: select mode places nothing",
                serde_json::json!({}),
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
            nova_probe::probe_marker(
                world,
                "outcome: delete removes the section clicked",
                serde_json::json!({}),
            );
            info!("editor: deleted a section ({before} -> {now})");
        })
        .until(frames(1))
        .add()
        // The parts gallery, walked the way a player walks it: browse, filter,
        // focus, select, place. Every beat is a real gesture on a real widget -
        // the gallery state is crate-private, so what the run can see is what
        // the player can see (tiles, the focus card, the section that appears).
        .step("editor: open the parts gallery")
        .on_enter(click_named("Parts Gallery Category"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release the gallery category")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the gallery is browsing the catalog")
        .on_enter(|world: &mut World| {
            let tiles = count_named_with_prefix(world, GALLERY_TILE);
            assert!(
                tiles > 0,
                "opening the gallery must list the browsable prototypes"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the gallery lists the catalog",
                serde_json::json!({}),
            );
            info!("editor: gallery is up with {tiles} tiles");
            world.resource_mut::<EditorProbe>().tiles = tiles;
        })
        .until(frames(1))
        .add()
        // The two gallery figures. `shoot` writes nothing unless NOVA_CAPTURE
        // is armed, and `shot_written` is inert in the same case, so the smoke
        // run walks these beats without waiting on a file.
        .step("editor: shoot the gallery grid")
        .on_enter(|world: &mut World| shoot(world, "editor-gallery.png"))
        .until(shot_written("editor-gallery.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Typing reaches the filter only once the field has the caret. `/` is
        // the keyboard way to give it one - the mouse way (a click on the
        // field) is what `arm_from_the_gallery` drives.
        .step("editor: put the caret in the filter")
        .on_enter(press_key(KeyCode::Slash))
        .until(frames(SETTLE))
        .add()
        .step("editor: release /")
        .on_enter(release_key(KeyCode::Slash))
        .until(frames(SETTLE))
        .add()
        .step("editor: filter the gallery")
        .on_enter(|world: &mut World| {
            let needle = filter_needle(world);
            type_text(needle)(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: the filter narrowed the grid to the hull")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            let before = world.resource::<EditorProbe>().tiles;
            let now = count_named_with_prefix(world, GALLERY_TILE);
            assert!(
                now < before,
                "typing a name into the gallery must narrow it ({before} -> {now})"
            );
            assert!(
                ui_node_rect(world, &format!("{GALLERY_TILE}{hull}")).is_some(),
                "the filtered grid must still list `{hull}`"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the filter narrows the gallery",
                serde_json::json!({}),
            );
            info!("editor: filter narrowed the gallery ({before} -> {now} tiles)");
        })
        .until(frames(1))
        .add()
        .step("editor: focus the filtered tile")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            click_named(format!("{GALLERY_TILE}{hull}"))(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: release the tile")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the focus card names the part and reads its stats")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            let lines = subtree_text(world, "Gallery Focus Card");
            assert!(
                lines.contains(&hull),
                "the focus card must name `{hull}`; it read {lines:?}"
            );
            for stat in ["hp", "size", "sockets"] {
                assert!(
                    lines.iter().any(|line| line == stat),
                    "the focus card must read `{stat}`; it read {lines:?}"
                );
            }
            nova_probe::probe_marker(
                world,
                "outcome: the focus card names the part",
                serde_json::json!({}),
            );
            info!("editor: focus card reads `{hull}`");
        })
        .until(frames(1))
        .add()
        .step("editor: shoot the gallery focus card")
        .on_enter(|world: &mut World| shoot(world, "editor-gallery-focus.png"))
        .until(shot_written("editor-gallery-focus.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("editor: place the focused part")
        .on_enter(click_named("Gallery Place Button"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Place")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the gallery closed and armed the tool")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_none(),
                "placing from the gallery must close it"
            );
            // Only now is the section count the SHIP's again: a gallery tile is
            // a section preview too, and it despawns with the overlay.
            stamp_sections(world);
        })
        .until(frames(SETTLE))
        .add()
        .click_the_ship("editor: place the gallery's pick")
        .step("editor: the gallery's pick was placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "the part selected in the gallery must be the one a click builds"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the gallery pick builds",
                serde_json::json!({}),
            );
            info!("editor: placed the gallery's pick ({before} -> {now})");
            stamp_sections(world);
        })
        .until(frames(1))
        .add()
        // Snapping: the roll is the one degree of freedom a mate leaves, so a
        // rolled part must still mate. R turns the ghost a quarter turn about
        // the mating axis before the click commits it.
        .step("editor: roll the ghost a quarter turn")
        .on_enter(press_key(KeyCode::KeyR))
        .until(frames(SETTLE))
        .add()
        .step("editor: release R")
        .on_enter(release_key(KeyCode::KeyR))
        .until(frames(SETTLE))
        .add()
        .click_the_ship("editor: place the rolled part")
        .step("editor: the ship is one structure, with a rolled mate in it")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(now, before + 1, "the rolled part was built");
            assert!(
                sections_with_a_rotation(world) > 0,
                "R must leave a section rolled off its parent's frame"
            );

            // The claim snapping exists to make: what the editor built is a
            // ship the RUNTIME derivation accepts - one connected structure,
            // every mate unambiguous.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            assert!(
                mates >= now - 1,
                "a connected ship of {now} sections needs at least {} mates, derived {mates}",
                now - 1
            );
            nova_probe::probe_marker(
                world,
                "outcome: the build derives one connected graph",
                serde_json::json!({}),
            );
            info!("editor: {now} sections, {mates} mates, one connected structure");
            world.resource_mut::<EditorProbe>().mates = mates;
        })
        .until(frames(1))
        .add()
        // The FAST path through the same picker: Tab opens it, and a part is
        // taken by pointing at it and pressing Q. No focus card, no Place
        // button, no click. This is the shape a builder repeats all session, so
        // it is walked as a whole gesture rather than trusted to unit tests.
        .step("editor: open the gallery with Tab")
        .on_enter(press_key(KeyCode::Tab))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Tab")
        .on_enter(release_key(KeyCode::Tab))
        .until(frames(SETTLE))
        .add()
        .step("editor: Tab put the gallery up")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_some(),
                "Tab must open the parts gallery from the build view"
            );
            stamp_sections(world);
        })
        .until(frames(1))
        .add()
        .step("editor: hover a tile")
        .on_enter(|world: &mut World| {
            let hull = world.resource::<EditorProbe>().hull.clone();
            hover_named(format!("{GALLERY_TILE}{hull}"))(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: take it with Q")
        .on_enter(press_key(KeyCode::KeyQ))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Q")
        .on_enter(release_key(KeyCode::KeyQ))
        .until(frames(SETTLE))
        .add()
        .step("editor: Q closed the gallery holding that part")
        .on_enter(|world: &mut World| {
            assert!(
                ui_node_rect(world, "Parts Gallery").is_none(),
                "taking a part with Q must hand the builder back to the ship"
            );
        })
        .until(frames(SETTLE))
        .add()
        .click_the_ship("editor: place the part Q took")
        .step("editor: the pipette's part was placed")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "hover + Q must arm the hovered part, so the next click builds it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: hover and Q arm the part",
                serde_json::json!({}),
            );
            info!("editor: placed the part Q took ({before} -> {now})");
        })
        .until(frames(1))
        .add()
        // The SKIN, which is the one part of the build view nobody builds:
        // cladding derived from the structure under it. Three figures - the
        // bare ship, the same ship clad, and the cladding closing around a hull
        // that is still in the builder's hand.
        //
        // The hull the pipette took stays armed throughout. Pointing AWAY from
        // the ship is what puts the ghost away for the first two figures: with
        // nothing under the pointer there is no placement to solve, so no ghost
        // and no status line.
        .step("editor: look away from the ship")
        .on_enter(move_cursor(EMPTY_SPACE))
        .until(frames(SETTLE))
        .add()
        .step("editor: shoot the bare build")
        .on_enter(|world: &mut World| {
            assert_eq!(
                count_plates(world),
                0,
                "nothing is clad until the toggle asks for it"
            );
            shoot(world, "editor-skin-off.png");
        })
        .until(shot_written("editor-skin-off.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("editor: click Ship Skin")
        .on_enter(click_named("Ship Skin Toggle"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Ship Skin")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the build came up clad")
        .on_enter(|world: &mut World| {
            let plates = count_plates(world);
            assert!(
                plates > 0,
                "the toggle must clad the ship on the stage, derived from the \
                 structure the builder assembled"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the skin clads the build",
                serde_json::json!({}),
            );
            info!("editor: the skin laid {plates} plates over the build");
            world.resource_mut::<EditorProbe>().plates = plates;
            stamp_sections(world);
            shoot(world, "editor-skin.png");
        })
        .until(shot_written("editor-skin.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The feature itself: a hull held against the ship is structure the
        // skin can already see, so the cladding closes around it BEFORE the
        // click that would commit it. The pointer goes to the same face the PDC
        // mounts on two beats later, which is this run's own evidence that the
        // aim lands on a socket a part can take.
        .step("editor: hold the hull against the ship")
        .on_enter(|world: &mut World| {
            let (centre, _) = aim_at_a_visible_face(world).expect("a section faces the camera");
            move_cursor(centre)(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: the skin reflowed around the part in hand")
        .on_enter(|world: &mut World| {
            let settled = world.resource::<EditorProbe>().plates;
            let now = count_plates(world);
            assert_ne!(
                now, settled,
                "the part under the pointer must reflow the skin around it \
                 ({settled} plates with it out of the way)"
            );
            assert_eq!(
                count_sections(world),
                world.resource::<EditorProbe>().sections,
                "and it must reflow without anything being BUILT - the click \
                 has not happened yet"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the skin reflows around the held part",
                serde_json::json!({}),
            );
            info!("editor: the ghost reflowed the skin ({settled} -> {now} plates)");
            shoot(world, "editor-skin-drag.png");
        })
        .until(shot_written("editor-skin-drag.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The refusal path needs a socket that is taken but still POINTABLE, so
        // the run mounts the compact PDC on a hull face: it fills that face's
        // socket while covering only a corner of the face. It is also the part
        // that proves a mount authored at its own size mates against a unit
        // cube at all - which is what `box_link_points` is for.
        .arm_from_the_gallery(
            "editor: arm the shared PDC turret",
            "pdc_kinetic_turret_section",
        )
        .step("editor: aim at the face the camera can see")
        .on_enter(|world: &mut World| {
            let (centre, _) = aim_at_a_visible_face(world).expect("a section faces the camera");
            move_cursor(centre)(world);
            stamp_sections(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: press on the free face")
        .on_enter(press_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: release")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the module mounted on that face")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now,
                before + 1,
                "a mount authored at its own size must mate onto a unit-cube \
                 hull face - one turret for every craft is the whole point"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the shared mount fits a hull face",
                serde_json::json!({}),
            );
            info!("editor: mounted the shared PDC ({before} -> {now})");
        })
        .until(frames(1))
        .add()
        .step("editor: aim at the same socket, now occupied")
        .on_enter(|world: &mut World| {
            let (_, off_centre) = aim_at_a_visible_face(world).expect("the same face is up");
            move_cursor(off_centre)(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: the occupied socket says so")
        .on_enter(|world: &mut World| {
            let status = subtree_text(world, "Placement Status");
            info!("editor: placement status reads {status:?}");
            assert!(
                status.iter().any(|line| line.contains("occupied")),
                "an occupied socket must be refused in words; the status read {status:?}"
            );
            stamp_sections(world);
        })
        .until(frames(1))
        .add()
        .step("editor: press on the occupied socket")
        .on_enter(press_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: release")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the refused click built nothing")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "a refused placement must not build; the same gesture built a \
                 section a moment ago"
            );
            nova_probe::probe_marker(
                world,
                "outcome: an occupied socket refuses",
                serde_json::json!({}),
            );
            info!("editor: the occupied socket refused the click ({now} sections)");

            // The graph as BUILT, the PDC mount included - what the flown
            // ship has to re-derive from the flat saved poses.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            info!("editor: the finished ship derives {mates} mates over {now} sections");
            world.resource_mut::<EditorProbe>().mates = mates;
        })
        .until(frames(1))
        .add()
        // The refusal figure: the ghost at the pose a click would build, its
        // bounds box red, and the reason under the ship.
        .step("editor: shoot the refused placement")
        .on_enter(|world: &mut World| {
            let (_, off_centre) = aim_at_a_visible_face(world).expect("the same face is up");
            move_cursor(off_centre)(world);
            shoot(world, "editor-placement-refused.png");
        })
        .until(shot_written("editor-placement-refused.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // The OTHER refusal a builder can meet, and the one the GENERATOR is
        // held to as well: a part that fires may not be built where it cannot.
        //
        // The seeded hull has nowhere like that yet, so the run builds one: a
        // two-cell tower beside the upper arm. Anything mounted on the arm's
        // roof then fires up a lane that tower stands beside, and the skin would
        // close the lane over rather than leave the tower's own face bare.
        .arm_from_the_gallery("editor: arm the hull again", HULL_PROTOTYPE)
        .place_on_the_face(
            "editor: raise a tower, first course",
            Vec3::new(0.0, 1.5, 1.0),
        )
        .place_on_the_face(
            "editor: raise a tower, second course",
            Vec3::new(0.0, 2.5, 1.0),
        )
        // A DRIVE rather than a hull slab: this is the case where the ghost is
        // the thing that fires, and a slab has no lane of its own.
        .arm_from_the_gallery("editor: arm a drive", "basic_thruster_section")
        .step("editor: aim the drive up the tower's lane")
        .on_enter(|world: &mut World| {
            let at = aim_at_world(world, Vec3::new(0.0, 1.5, 2.0))
                .expect("the roof of the upper arm is on screen");
            move_cursor(at)(world);
            stamp_sections(world);
        })
        .until(frames(SETTLE))
        .add()
        .step("editor: a drive that cannot fire says so")
        .on_enter(|world: &mut World| {
            let status = subtree_text(world, "Placement Status");
            info!("editor: placement status reads {status:?}");
            assert!(
                status.iter().any(|line| line.contains("block")),
                "a drive whose plume would fire into the ship's own plating must \
                 be refused in words; the status read {status:?}"
            );
            shoot(world, "editor-placement-blocked-exit.png");
        })
        .until(shot_written("editor-placement-blocked-exit.png"))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .step("editor: press on the blocked lane")
        .on_enter(press_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: release")
        .on_enter(release_mouse(MouseButton::Left))
        .until(frames(SETTLE))
        .add()
        .step("editor: the blocked exit built nothing either")
        .on_enter(|world: &mut World| {
            let before = world.resource::<EditorProbe>().sections;
            let now = count_sections(world);
            assert_eq!(
                now, before,
                "a drive with nowhere to fire must not be built; the editor and \
                 the generator hold one rule between them"
            );
            nova_probe::probe_marker(
                world,
                "outcome: a blocked drive lane refuses",
                serde_json::json!({}),
            );
            info!("editor: the blocked exit refused the click ({now} sections)");

            // The tower moved the graph on, so the figure the flown ship is
            // compared against is re-taken here.
            let mates = mate_graph(world, None).unwrap_or_else(|error| {
                panic!("the assembled ship must derive one connected graph: {error}")
            });
            world.resource_mut::<EditorProbe>().mates = mates;
            info!("editor: the finished ship derives {mates} mates over {now} sections");
        })
        .until(frames(1))
        .add()
        // Play: the ship the editor assembled becomes the ship that flies, and
        // the runtime derives the SAME graph from the flat saved poses.
        .step("editor: press Play")
        .on_enter(click_named("Play Button"))
        .until(frames(SETTLE))
        .add()
        .step("editor: release Play")
        .on_enter(release_mouse(MouseButton::Left))
        .until(player_ship_present())
        .deadline(60.0)
        .add()
        .step("editor: let the scenario settle")
        .until(frames(SHIP_SETTLE))
        .add()
        .step("editor: the flown ship derives the same mate graph")
        .on_enter(|world: &mut World| {
            let built = world.resource::<EditorProbe>().mates;
            let root = player_root(world);
            let flown = mate_graph(world, Some(root))
                .unwrap_or_else(|error| panic!("the spawned ship must derive a graph: {error}"));
            assert_eq!(
                flown, built,
                "the flown ship must re-derive the mate graph the editor built"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the flown ship re-derives the graph",
                serde_json::json!({}),
            );
            info!("editor: the flown ship carries the same {flown} mates");

            // What you see is what you fly: the toggle rides the hand-off, so
            // a ship built clad is spawned clad.
            let plates = count_plates(world);
            assert!(
                plates > 0,
                "the ship was built with its skin on, so the flown one wears it"
            );
            nova_probe::probe_marker(
                world,
                "outcome: the flown ship wears the skin",
                serde_json::json!({}),
            );
            info!("editor: the flown ship came up in {plates} plates");
        })
        .until(frames(1))
        .add()
}

/// Name prefix every gallery tile carries; the part's display name follows it.
#[cfg(feature = "debug")]
const GALLERY_TILE: &str = "Gallery Tile ";

/// The sections of ONE ship, as the pure link-point derivation takes them.
///
/// Works on the editor's preview ship and on the flown one alike: both are a
/// root plus section children carrying local poses and authored sockets, which
/// is all the derivation reads.
///
/// `None` takes every section in the world, which is only the same thing in
/// the EDITOR scene: the preview is the only ship there. In flight it is not -
/// the sandbox spawns target hulks and pickets whose sections carry the same
/// marker - and the derivation demands ONE connected structure, so a
/// world-wide sweep out there hands it several ships at once and is rejected.
#[cfg(feature = "debug")]
fn placed_sections(world: &mut World, root: Option<Entity>) -> Vec<(Transform, SectionLinkPoints)> {
    world
        .query_filtered::<(&Transform, &SectionLinkPoints, &ChildOf), With<SectionMarker>>()
        .iter(world)
        .filter(|(_, _, ChildOf(parent))| root.is_none_or(|root| *parent == root))
        .map(|(transform, points, _)| (*transform, points.clone()))
        .collect()
}

/// The flown player ship's root.
#[cfg(feature = "debug")]
fn player_root(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<PlayerSpaceshipMarker>>()
        .iter(world)
        .next()
        .expect("the beat before this one waited for the player ship")
}

/// How many mates the runtime derivation finds over one ship's sections, or
/// the errors that rejected it.
#[cfg(feature = "debug")]
fn mate_graph(world: &mut World, root: Option<Entity>) -> Result<usize, String> {
    let sections = placed_sections(world, root);
    let placed: Vec<PlacedSectionLinkPoints> = sections
        .iter()
        .map(|(transform, points)| PlacedSectionLinkPoints {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: points,
        })
        .collect();
    derive_link_point_graph(&placed)
        .map(|mates| mates.len())
        .map_err(|errors| format!("{errors:?}"))
}

/// Sections whose pose is rolled off their parent's frame - what the R key
/// leaves behind.
#[cfg(feature = "debug")]
fn sections_with_a_rotation(world: &mut World) -> usize {
    placed_sections(world, None)
        .iter()
        .filter(|(transform, _)| transform.rotation.angle_between(Quat::IDENTITY) > 1e-3)
        .count()
}

/// The viewport aim for a point in ship space, so a beat can point at a
/// NAMED face rather than at whatever the camera happens to like.
#[cfg(feature = "debug")]
fn aim_at_world(world: &mut World, point: Vec3) -> Option<Vec2> {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?.clone();
    let camera_transform = *world.get::<GlobalTransform>(camera_entity)?;
    camera
        .world_to_viewport(&camera_transform, point)
        .ok()
        .filter(|aim: &Vec2| aim.x.is_finite() && aim.y.is_finite())
}

/// The camera-facing socket of the section nearest the camera, as two viewport
/// aims: its centre, and a point on the same face 0.35 off centre.
///
/// Both land on the SAME socket - the editor mates the socket nearest the
/// pointer's hit, and on a unit face nothing else is within 0.35 - which is
/// what lets one beat fill that socket and the next meet the refusal, without
/// the second aim being swallowed by the part the first one mounted.
#[cfg(feature = "debug")]
fn aim_at_a_visible_face(world: &mut World) -> Option<(Vec2, Vec2)> {
    let camera_entity = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()?;
    let camera = world.get::<Camera>(camera_entity)?.clone();
    let camera_transform = *world.get::<GlobalTransform>(camera_entity)?;
    let eye = camera_transform.translation();

    // The unit-cube hull sections are the ship's body; a mounted module is
    // smaller than a face and must not become the target itself.
    let sections = placed_sections(world, None);
    let (transform, points) = sections
        .iter()
        .filter(|(_, points)| points.len() >= 6)
        .min_by(|(a, _), (b, _)| {
            a.translation
                .distance_squared(eye)
                .partial_cmp(&b.translation.distance_squared(eye))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let (position, normal) = points
        .iter()
        .map(|point| {
            (
                transform.translation + transform.rotation * point.position,
                (transform.rotation * point.normal).normalize(),
            )
        })
        .max_by(|(_, a), (_, b)| {
            let facing = |normal: &Vec3| normal.dot((eye - transform.translation).normalize());
            facing(a)
                .partial_cmp(&facing(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let tangent = normal.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    Some((
        camera.world_to_viewport(&camera_transform, position).ok()?,
        camera
            .world_to_viewport(&camera_transform, position + tangent * 0.35)
            .ok()?,
    ))
}

/// The text the filter beat types: the first word of the hull's display name,
/// lowercased, so the run proves the filter is case-insensitive as well as
/// narrowing.
#[cfg(feature = "debug")]
fn filter_needle(world: &World) -> String {
    world
        .resource::<EditorProbe>()
        .hull
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase()
}

/// How many laid-out, visible UI nodes have a name starting with `prefix`.
#[cfg(feature = "debug")]
fn count_named_with_prefix(world: &mut World, prefix: &str) -> usize {
    world
        .query::<(&Name, &InheritedVisibility)>()
        .iter(world)
        .filter(|(name, visibility)| visibility.get() && name.as_str().starts_with(prefix))
        .count()
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

/// Count the skin plates on screen - the editor's preview cladding, and after
/// Play the flown ship's real one. Both wear the same marker, which is the
/// point: one derivation, two places.
#[cfg(feature = "debug")]
fn count_plates(world: &mut World) -> usize {
    let mut q = world.query_filtered::<(), With<ShipSkinMarker>>();
    q.iter(world).count()
}

/// Record the live section count, so the beat after the next gesture can say
/// what that gesture changed.
#[cfg(feature = "debug")]
fn stamp_sections(world: &mut World) {
    let count = count_sections(world);
    world.resource_mut::<EditorProbe>().sections = count;
}

/// Every line of text under the named UI node, empty when no such node is up.
#[cfg(feature = "debug")]
fn subtree_text(world: &mut World, name: &str) -> Vec<String> {
    let Some(root) = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, node_name)| node_name.as_str() == name)
        .map(|(entity, _)| entity)
    else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut stack = vec![root];
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

/// The same gesture aimed at a NAMED face, plus the verdict that it built.
///
/// [`ClickTheShip`] takes whatever face the camera likes, which is what most of
/// the run wants. Building a particular SHAPE needs a particular face, and the
/// verdict is what says the shape is really there rather than assumed.
#[cfg(feature = "debug")]
trait PlaceOnTheFace {
    fn place_on_the_face(self, label: &str, socket: Vec3) -> Self;
}

#[cfg(feature = "debug")]
impl PlaceOnTheFace for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn place_on_the_face(self, label: &str, socket: Vec3) -> Self {
        self.step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let at = aim_at_world(world, socket).expect("that face is on screen");
                move_cursor(at)(world);
                stamp_sections(world);
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
            .step(format!("{label}: it built"))
            .on_enter(move |world: &mut World| {
                let before = world.resource::<EditorProbe>().sections;
                let now = count_sections(world);
                assert_eq!(
                    now,
                    before + 1,
                    "the beat aimed at {socket:?} built nothing, so the shape \
                     the next beat needs is not there"
                );
            })
            .until(frames(1))
            .add()
    }
}

/// Arm a prototype through the gallery, by keyboard: open it, type enough of
/// the catalog id to leave one tile, Enter to focus it, Enter to place.
///
/// The keyboard path rather than clicking a tile by name: the semantic parts
/// are named for their ROLE (three craft each carry a "Turret Port"), so a name
/// is not a unique target where a catalog id is.
#[cfg(feature = "debug")]
trait ArmFromTheGallery {
    fn arm_from_the_gallery(self, label: &str, prototype: &str) -> Self;
}

#[cfg(feature = "debug")]
impl ArmFromTheGallery for nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    fn arm_from_the_gallery(self, label: &str, prototype: &str) -> Self {
        let filter = prototype.to_string();
        let mut script = self
            .step(format!("{label}: open the gallery"))
            .on_enter(click_named("Parts Gallery Category"))
            .until(frames(SETTLE))
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(SETTLE))
            .add()
            // The filter takes the keyboard only once it has the caret; a
            // click on the field is how a mouse user gives it one.
            .step(format!("{label}: click the filter field"))
            .on_enter(click_named("Gallery Filter"))
            .until(frames(SETTLE))
            .add()
            .step(format!("{label}: release the filter field"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(frames(SETTLE))
            .add()
            .step(format!("{label}: filter to `{prototype}`"))
            .on_enter(move |world: &mut World| type_text(filter.clone())(world))
            .until(frames(SETTLE))
            .add();
        // Enter focuses the selected tile; Enter again places it and closes.
        for beat in ["focus", "place"] {
            script = script
                .step(format!("{label}: press Enter to {beat}"))
                .on_enter(press_key(KeyCode::Enter))
                .until(frames(SETTLE))
                .add()
                .step(format!("{label}: release Enter"))
                .on_enter(release_key(KeyCode::Enter))
                .until(frames(SETTLE))
                .add();
        }
        script
            .step(format!("{label}: the gallery closed"))
            .on_enter(|world: &mut World| {
                assert!(
                    ui_node_rect(world, "Parts Gallery").is_none(),
                    "placing from the gallery must close it"
                );
            })
            .until(frames(SETTLE))
            .add()
    }
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
