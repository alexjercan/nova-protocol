//! loop_hull_generate: the editor rolling a hull nobody drew -
//! `news-0130-hull-generate`, with the two stills beside it.
//!
//! A blank ship, entered through Add > Ship the way a builder enters one, and
//! then the Generate block driven by real pointer and keyboard gestures: a seed
//! typed into its field, a capital drive and a railgun lance ticked in the
//! draw, a turret's zone chip cycled round to the flank, Generate pressed, and
//! the hull the collapse chose landing on the stage while the HULL PLAN line
//! says which engine and which bow gun the ticks add up to.
//!
//! Three assets, in order:
//! - `news-0130-hull-generate.webm`: the whole roll, as a loop.
//! - `news-0130-hull-plan.png`: the frame after the landing, with the block.
//! - `news-0130-editor-save-as.png`: the Save As window over that ship, a name
//!   typed and the slot it derives printed under it.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - the whole walk, writing
//!   nothing. Save As is CANCELLED, so no run ever writes into the mod cache.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also record the loop and the stills
//!   (staged under `NOVA_CAPTURE_DIR`).
//!
//! Capture:
//! ```text
//! NOVA_CAPTURE_DIR=target/news-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example loop_hull_generate --features debug
//! ```

#[cfg(feature = "debug")]
use std::sync::Arc;

#[cfg(feature = "debug")]
use bevy::{input::keyboard::Key, prelude::*, window::PrimaryWindow};
use clap::Parser;
#[cfg(feature = "debug")]
use nova_assets::mod_cache::prelude::EDITOR_ID_PREFIX;
#[cfg(feature = "debug")]
use nova_debug::prelude::capturing;
// `not` by its harness path: the bevy prelude's run-condition `not` shadows
// the predicate combinator under the glob.
#[cfg(feature = "debug")]
use nova_protocol::nova_debug::harness::{
    not, AutopilotPlugin, LoopProfile, Predicate, CAPTURE_RESOLUTION,
};
use nova_protocol::prelude::*;

// The pointer gestures, shared with the other editor walks. Script-only, so
// the whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use ui_walk::{count_sections, the_editor_is_inside_a_ship, Gestures};

#[derive(Parser)]
#[command(name = "loop_hull_generate")]
#[command(version = "1.0.0")]
#[command(about = "The editor generating a hull into a blank ship, then naming it in Save As. Autopilot-only: a scripted walk over the real editor", long_about = None)]
struct Cli;

/// The loop: a seed typed, two parts ticked, a chip cycled, Generate, and the
/// hull landing.
#[cfg(feature = "debug")]
const GENERATE_LOOP: &str = "news-0130-hull-generate";
/// The still after the landing: the Generate block beside the hull it rolled.
#[cfg(feature = "debug")]
const PLAN_SHOT: &str = "news-0130-hull-plan.png";
/// The still of the Save As window over that ship.
#[cfg(feature = "debug")]
const SAVE_AS_SHOT: &str = "news-0130-editor-save-as.png";

/// The seed the walk types. Fixed, so the footage shows the same hull on
/// every capture.
#[cfg(feature = "debug")]
const SEED: &str = "1300";
/// How many Backspaces clear whatever seed the field opened on: a `u64` is at
/// most twenty digits, and the field holds no more.
#[cfg(feature = "debug")]
const SEED_CLEAR: usize = 20;

/// The draw-list rows the walk ticks: the drive the ship is then built
/// around, and the spinal gun its nose is built around.
#[cfg(feature = "debug")]
const DRIVE_ROW: &str = "Part: capital_thruster_section";
#[cfg(feature = "debug")]
const LANCE_ROW: &str = "Part: railgun_lance_section";
/// The row whose zone chip is cycled. Ticked by the shipped grammar, so its
/// chip is on screen before the walk touches the list.
#[cfg(feature = "debug")]
const ZONED_ROW: &str = "Part: pdc_kinetic_turret_section";
/// What the chip reads after each press, from `any` round to `flank` - the
/// order `nova_editor`'s `next_zone` cycles them in.
#[cfg(feature = "debug")]
const ZONE_CYCLE: [&str; 6] = ["bow", "mid", "stern", "dorsal", "ventral", "flank"];

/// What the HULL PLAN line reads once both parts are ticked: the two seeded
/// roles the ticks derive, by catalog name.
#[cfg(feature = "debug")]
const HULL_PLAN: &str = "stern Capital Thruster Section\nbow Railgun Lance";

/// The name typed into Save As.
///
/// A fresh name. The window's "overwrites" line is measured against the ranges
/// SAVED in the mod cache, which the capture sandbox keeps empty, so the line
/// this walk photographs is the one a first save gets.
#[cfg(feature = "debug")]
const SAVE_NAME: &str = "Hull Roll 1300";
/// The slug the window derives from [`SAVE_NAME`].
#[cfg(feature = "debug")]
const SAVE_SLUG: &str = "hull_roll_1300";
/// How many Backspaces clear the name the window opened on: the field holds
/// forty-eight characters (`nova_editor`'s `NAME_CHARS`).
#[cfg(feature = "debug")]
const NAME_CLEAR: usize = 48;

/// The rail's scrolling viewport, which the "is it on screen" reads measure
/// against.
#[cfg(feature = "debug")]
const RAIL_VIEWPORT: &str = "Editor Rail Scroll";
/// The bar beside it, which the wheel beats aim at. Not the rows: a hovered
/// Scene row raises its hint beside the rail, and a hull of 150 sections is
/// 150 rows under wherever the pointer lands. With no viewport under the
/// pointer the wheel goes to every viewport at once, and the rail is the only
/// one on screen with anything to scroll.
#[cfg(feature = "debug")]
const RAIL_BAR: &str = "Rail Scrollbar";
/// The rail's top: where the wheel takes it back to before the top bar is
/// clicked again. A row scrolled up past the viewport still takes the pick
/// over the top bar, and a Scene row would have the File button's click.
#[cfg(feature = "debug")]
const RAIL_TOP: &str = "Rail Tabs";

/// Where the pointer is parked between gestures, in logical pixels: over the
/// top bar's empty stretch between the Ship button and the Play button, so no
/// rail row is hovered and no section on the stage is cross-highlighted.
/// Empty at both window sizes this walk runs at - the bar's buttons are laid
/// out from the left, and Play is centred.
#[cfg(feature = "debug")]
const PARK: Vec2 = Vec2::new(400.0, 18.0);

/// Where the lens stands for the landing, meters.
///
/// Pinned BEFORE Generate, so the hull lands into a frame that already holds
/// it and the loop has no cut. The standard grammar grown around a capital
/// drive is a grid thirteen cells across, five high and eleven long - up to
/// 130 m by 50 m by 110 m around the ship's origin, which is where a first
/// Add > Ship stands. A three-quarter view from 230 m off the BOW quarter,
/// twenty degrees up: the lances the plan names stand nearest the lens, the
/// deck reads, and the drives close the far end. Ships fly along -Z, so the
/// bow side is -Z.
///
/// Sized for the 720p loop window, where the stage is the 770 px between the
/// rail and the inspector (both keep their pixel width) and the hull's frame
/// box spans some 700 of them; the same reach leaves the 1080p stills with
/// the hull on two thirds of theirs.
#[cfg(feature = "debug")]
const HULL_EYE: Meters3 = Meters3::new(133.0, 84.0, -168.0);

/// What the lens looks at: the ship's origin, pushed 12 m along the camera's
/// screen-right, so the hull sits on the centre of the band between the rail
/// and the inspector rather than on the centre of the window.
#[cfg(feature = "debug")]
const HULL_LOOK: Meters3 = Meters3::new(-9.0, 0.0, -7.0);

/// How close the camera has to be to [`HULL_EYE`] before a beat calls the
/// pose reached. Compared against the camera Transform, so it is an engine
/// world-unit figure.
#[cfg(feature = "debug")]
const POSE_EPSILON: f32 = 1e-2;

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // The same app the game/binary runs (main menu over the ambience backdrop).
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        // Probe wiring (each plugin is inert without its NOVA_PROBE_* env):
        // run timeline + engine-bound invariants. No frame-time capture - the
        // walk is a sequence of gestures with no steady-state window.
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            generate_loop_profile(),
        ));
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // Turn command errors (despawned-entity targets on the menu/editor
            // teardown) into panics so the run fails loudly on them.
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        // Clean frames at a known 16:9 size: force the window to the loop
        // profile's 720p and drop the dev overlays. The HUD chrome is re-hidden
        // right before the loop and each shot (entering the editor re-raises
        // it).
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(generate_script());
    }

    app.run()
}

/// The loop's encode: the fleet's cadence, quality, cap and 720p frame, with
/// the WINDOW at 720p too, so the encode carries no scale.
///
/// The fleet records at 1920x1080 and scales the webm down. This loop's
/// subject is the rail - a seed in a 10 px field, chips with 10 px words, a
/// two-line plan - and that scale would put the text at 7 px. The editor's
/// chrome keeps its pixel size whatever the window's, so a 720p window is the
/// same 10 px text in a smaller frame. The stills are shot after the loop, at
/// the fleet's figure size (see `size_the_window_for_the_figures`).
#[cfg(feature = "debug")]
fn generate_loop_profile() -> LoopProfile {
    let fleet = LoopProfile::default();
    LoopProfile {
        window_resolution: fleet.output_resolution,
        ..fleet
    }
}

/// The first UI node called `name`.
#[cfg(feature = "debug")]
fn named_node(world: &World, name: &str) -> Option<Entity> {
    world.try_query::<(Entity, &Name)>().and_then(|mut nodes| {
        nodes
            .iter(world)
            .find(|(_, named)| named.as_str() == name)
            .map(|(entity, _)| entity)
    })
}

/// The zone chip of the draw-list row called `row`: the row's last child, the
/// way `part_row` lays a row out - tick, name, chip. The chip carries no name
/// of its own.
#[cfg(feature = "debug")]
fn zone_chip(world: &World, row: &str) -> Option<Entity> {
    let row = named_node(world, row)?;
    world.get::<Children>(row)?.last().copied()
}

/// The logical-pixel centre of the zone chip on `row`, once it is laid out -
/// `ui_node_rect`'s read, on an entity rather than a name.
#[cfg(feature = "debug")]
fn zone_chip_centre(world: &World, row: &str) -> Option<Vec2> {
    let chip = zone_chip(world, row)?;
    let transform = world.get::<UiGlobalTransform>(chip)?;
    let computed = world.get::<ComputedNode>(chip)?;
    let scale = computed.inverse_scale_factor();
    let rect = Rect::from_center_size(transform.translation * scale, computed.size() * scale);
    (rect.width() > 0.0 && rect.height() > 0.0).then(|| rect.center())
}

/// What the zone chip on `row` says.
#[cfg(feature = "debug")]
fn zone_chip_label(world: &World, row: &str) -> Option<String> {
    let chip = zone_chip(world, row)?;
    let word = world.get::<Children>(chip)?.first().copied()?;
    world.get::<Text>(word).map(|text| text.0.clone())
}

/// Advance once the zone chip on `row` reads `label`.
#[cfg(feature = "debug")]
fn the_zone_chip_reads(row: &'static str, label: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| zone_chip_label(world, row).as_deref() == Some(label))
}

/// Advance once the pick map says the pointer is over the zone chip on `row` -
/// the chip itself or the word inside it. `pointer_over_node` by entity,
/// because the chip has no name to ask for.
#[cfg(feature = "debug")]
fn the_pointer_is_over_the_zone_chip(row: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        use bevy::picking::{hover::HoverMap, pointer::PointerId};

        let Some(chip) = zone_chip(world, row) else {
            return false;
        };
        world
            .get_resource::<HoverMap>()
            .and_then(|hover| hover.get(&PointerId::Mouse))
            .is_some_and(|hits| {
                hits.keys().any(|hit| {
                    std::iter::successors(Some(*hit), |entity| {
                        world.get::<ChildOf>(*entity).map(ChildOf::parent)
                    })
                    .any(|entity| entity == chip)
                })
            })
    })
}

/// Advance once the draw-list row called `row` is ticked. The MARK is the
/// setting; the row's glyph is only a picture of it.
#[cfg(feature = "debug")]
fn the_row_is_ticked(row: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        named_node(world, row)
            .is_some_and(|row| world.get::<nova_ui::prelude::Selected>(row).is_some())
    })
}

/// Advance once the text field called `name` holds `value`.
#[cfg(feature = "debug")]
fn the_field_reads(name: &'static str, value: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        named_node(world, name).is_some_and(|field| {
            world
                .get::<nova_ui::prelude::TextFieldValue>(field)
                .is_some_and(|typed| typed.0 == value)
        })
    })
}

/// Advance once the text field called `name` holds the caret. Typing reaches
/// a field only while it does.
#[cfg(feature = "debug")]
fn the_field_holds_the_caret(name: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        named_node(world, name).is_some_and(|field| {
            world
                .get::<nova_ui::prelude::TextFieldFocused>(field)
                .is_some()
        })
    })
}

/// Advance once the text node called `name` reads `text`.
#[cfg(feature = "debug")]
fn the_node_reads(name: &'static str, text: impl Into<String>) -> Arc<Predicate> {
    let text = text.into();
    Arc::new(move |world: &World| {
        named_node(world, name)
            .and_then(|node| world.get::<Text>(node))
            .is_some_and(|shown| shown.0 == text)
    })
}

/// Advance once the node called `name` is inside the rail's viewport - the
/// honest end of a wheel beat, where a frame count only guessed at the
/// reflow.
#[cfg(feature = "debug")]
fn the_rail_shows(name: &'static str) -> Arc<Predicate> {
    Arc::new(move |world: &World| {
        let (Some(rail), Some(node)) = (
            ui_node_rect(world, RAIL_VIEWPORT),
            ui_node_rect(world, name),
        ) else {
            return false;
        };
        rail.contains(node.center())
    })
}

/// Advance once the collapse has laid its hull: the edited ship holds
/// sections, and the status line says which seed built them.
#[cfg(feature = "debug")]
fn the_hull_landed() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        count_sections(world) > 0
            && world
                .get_resource::<EditorProbe>()
                .is_some_and(|probe| probe.status.contains("built a hull"))
    })
}

/// Put the editor's camera on [`HULL_EYE`] and PIN it there, the way
/// `pose_editor_camera` pins the build pose: the free-fly rig rewrites the
/// Transform every frame, so only a `ScriptedCameraPose` holds.
#[cfg(feature = "debug")]
fn pose_for_the_landing(world: &mut World) {
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .iter(world)
        .next()
        .expect("the editor is up, so it has a 3D camera");
    world.entity_mut(camera).insert(ScriptedCameraPose {
        position: HULL_EYE,
        look_at: HULL_LOOK,
    });
}

/// Put the window at the fleet's figure size for the stills. The loop is
/// recorded at the profile's 720p window; the figures are 1080p like every
/// other still the site shows, and `shoot` reads the window back as it is.
/// The rail's viewport clamps its own scroll to the taller box, which leaves
/// the block where the loop left it: at the rail's bottom.
#[cfg(feature = "debug")]
fn size_the_window_for_the_figures(world: &mut World) {
    let (width, height) = CAPTURE_RESOLUTION;
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world) {
        window.resolution.set(width as f32, height as f32);
    }
}

/// Advance once the window REPORTS the figure size and a layout pass has
/// consumed it - `window_size_is` answers for the request alone.
#[cfg(feature = "debug")]
fn the_window_is_figure_sized() -> Arc<Predicate> {
    let (width, height) = CAPTURE_RESOLUTION;
    and(window_size_is(width as f32, height as f32), frames(2))
}

/// Advance once the editor camera has REACHED [`HULL_EYE`]; the loader's
/// enforcer applies the pin a system later.
#[cfg(feature = "debug")]
fn the_landing_camera_is_posed() -> Arc<Predicate> {
    Arc::new(|world: &World| {
        world
            .try_query_filtered::<&Transform, With<Camera3d>>()
            .is_some_and(|mut cameras| {
                cameras.iter(world).any(|camera| {
                    camera
                        .translation
                        .abs_diff_eq(HULL_EYE.to_engine(), POSE_EPSILON)
                })
            })
    })
}

/// The gesture shapes this walk is written in, over the shared [`Gestures`].
#[cfg(feature = "debug")]
trait GenerateGestures {
    /// Hold the frame for `seconds` - only while recording. The smoke path
    /// has nothing to hold for.
    fn hold(self, label: &str, seconds: f32) -> Self;

    /// Wheel the rail until the node called `name` is inside its viewport.
    /// The draw list is the whole catalog, and its last rows start below the
    /// fold at 1080 px.
    fn scroll_the_rail_to(self, label: &str, name: &'static str) -> Self;

    /// Click into the text field called `field`, clear it with `clear`
    /// Backspaces from the end, type `text`, and Enter it when `commit` -
    /// which drops the caret - or leave the caret in it otherwise.
    fn retype(
        self,
        label: &str,
        field: &'static str,
        clear: usize,
        text: &'static str,
        commit: bool,
    ) -> Self;

    /// Tick the draw-list row called `row`.
    fn tick(self, label: &str, row: &'static str) -> Self;

    /// Press the zone chip on `row` once, and wait for it to read `then`.
    fn press_the_zone_chip(self, label: &str, row: &'static str, then: &'static str) -> Self;

    /// Click Generate and wait for the hull to land, with the pointer taken
    /// off the rail the frame the release registers. A click's last beat
    /// leaves the pointer on the button, and the landing grows the Scene
    /// tree over that spot: the row that reflows under it is a hovered row,
    /// and a hovered row raises its hint beside the rail.
    fn press_generate(self, label: &str) -> Self;
}

#[cfg(feature = "debug")]
impl GenerateGestures for AutopilotPlugin<GameStates> {
    fn hold(self, label: &str, seconds: f32) -> Self {
        if capturing() {
            self.step(label).until(elapsed(seconds)).add()
        } else {
            self
        }
    }

    fn scroll_the_rail_to(self, label: &str, name: &'static str) -> Self {
        self.step(format!("{label}: the rail holds it"))
            .until(ui_node_present(name))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // The aim is its own beat: the pick map catches up with a move one
            // PreUpdate later, and a wheel sent before that would land wherever
            // the pointer was.
            .step(format!("{label}: aim the wheel at the rail's bar"))
            .on_enter(hover_named(RAIL_BAR))
            .until(pointer_over_node(RAIL_BAR))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: wheel it into view"))
            .on_enter(move |world: &mut World| {
                let (Some(rail), Some(node)) = (
                    ui_node_rect(world, RAIL_VIEWPORT),
                    ui_node_rect(world, name),
                ) else {
                    return;
                };
                // `scroll_viewports` subtracts the delta from the offset, so
                // a negative delta moves the content UP the screen - what a
                // node below the box needs - and a positive one brings a node
                // scrolled up past it back down.
                scroll_pixels(rail.center().y - node.center().y)(world);
            })
            .until(the_rail_shows(name))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn retype(
        self,
        label: &str,
        field: &'static str,
        clear: usize,
        text: &'static str,
        commit: bool,
    ) -> Self {
        // The click lands the caret wherever the pixels fell, so the caret is
        // put at the end and the old value cleared back from there.
        let settled = if commit {
            not(the_field_holds_the_caret(field))
        } else {
            the_field_holds_the_caret(field)
        };
        self.click(&format!("{label}: reach for the field"), field)
            .step(format!("{label}: the field holds the caret"))
            .until(the_field_holds_the_caret(field))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: retype it"))
            .on_enter(move |world: &mut World| {
                press_edit_key(Key::End)(world);
                for _ in 0..clear {
                    press_edit_key(Key::Backspace)(world);
                }
                type_text(text)(world);
                if commit {
                    press_edit_key(Key::Enter)(world);
                }
            })
            .until(and(the_field_reads(field, text), settled))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn tick(self, label: &str, row: &'static str) -> Self {
        self.click(label, row)
            .step(format!("{label}: it is ticked"))
            .until(the_row_is_ticked(row))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn press_the_zone_chip(self, label: &str, row: &'static str, then: &'static str) -> Self {
        let aim = move |world: &mut World| {
            if let Some(at) = zone_chip_centre(world, row) {
                move_cursor(at)(world);
            }
        };
        self.step(format!("{label}: aim at the chip"))
            .on_enter(aim)
            .each(move |world: &mut World, _, _| aim(world))
            .until(the_pointer_is_over_the_zone_chip(row))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            // Widgets act on `Activate`, which fires on the release; the chip
            // reading its next zone is the release having landed.
            .step(format!("{label}: release, and it reads `{then}`"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(and(pointer_released(), the_zone_chip_reads(row, then)))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    }

    fn press_generate(self, label: &str) -> Self {
        // The three beats of `click_named`, with the release beat's every-frame
        // hook parking the pointer as soon as the pick map holds the release:
        // the move is then read a frame after the release, so it cannot pull
        // the release off the button.
        let button = "Generate Button";
        let released = pointer_released();
        self.step(format!("{label}: the button is up"))
            .until(ui_node_present(button))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: aim"))
            .on_enter(hover_named(button))
            .until(pointer_over_node(button))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(BEAT_DEADLINE_SECS)
            .add()
            .step(format!("{label}: release, and the hull lands"))
            .on_enter(release_mouse(MouseButton::Left))
            .each(move |world: &mut World, _, _| {
                if released(world) {
                    move_cursor(PARK)(world);
                }
            })
            .until(the_hull_landed())
            .deadline(STEP_DEADLINE_SECS)
            .add()
    }
}

/// The driven walk: menu -> editor -> a blank ship -> seed, ticks, chip,
/// Generate -> the plan still -> Save As -> the window still -> Cancel.
#[cfg(feature = "debug")]
fn generate_script() -> AutopilotPlugin<GameStates> {
    // The HUD is re-raised by entering the editor, so it is dropped again right
    // before each shot. `shoot` itself is the capture gate: unarmed, this whole
    // walk runs and writes nothing.
    let shot = |path: &'static str| {
        move |world: &mut World| {
            hide_hud(world);
            shoot(world, path);
        }
    };
    let park = |label: &str, script: AutopilotPlugin<GameStates>| {
        script
            .step(format!(
                "{label}: park the pointer clear of the rail and the stage"
            ))
            .on_enter(move_cursor(PARK))
            .until(pointer_at(PARK))
            .deadline(BEAT_DEADLINE_SECS)
            .add()
    };

    let mut script = AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The editor: reached the way a player reaches it. `click` waits for the
        // button to lay out, so the menu needs no settle of its own.
        .click("leave for the editor", "Sandbox Button")
        .step("reach the editor")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Through the top bar, because that is where Add lives: the row does
        // not exist until its menu is open. Add Ship enters the blank ship,
        // and the Generate block is shown only inside one.
        .click("open the Add menu", "Add Menu Button")
        .click("create the ship", "Add Ship Button")
        .step("the blank ship is entered")
        .until(the_editor_is_inside_a_ship())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("pose the lens for the landing")
        .on_enter(|world: &mut World| {
            hide_hud(world);
            pose_for_the_landing(world);
        })
        .until(the_landing_camera_is_posed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .scroll_the_rail_to("bring the whole draw list up", LANCE_ROW);
    script = park("before the roll", script);

    if capturing() {
        script = script
            .step("open the generate loop")
            .on_enter(|world| loop_start(world, GENERATE_LOOP))
            .add();
    }

    script = script
        .hold("hold the blank stage", 0.8)
        .retype("seed the roll", "Hull Seed Field", SEED_CLEAR, SEED, true)
        .hold("hold the typed seed", 0.5)
        .tick("tick the capital drive", DRIVE_ROW)
        .hold("hold the ticked drive", 0.3)
        .tick("tick the railgun lance", LANCE_ROW)
        // The plan line is written from the TICKS, so it names the engine and
        // the bow gun before Generate is pressed - and a line that still read
        // the stock drive would mean a tick was lost.
        .step("the plan names the engine and the bow gun")
        .until(the_node_reads("Hull Plan Line", HULL_PLAN))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .hold("hold the plan", 0.5);

    for (index, zone) in ZONE_CYCLE.iter().enumerate() {
        script = script.press_the_zone_chip(
            &format!("cycle the turret's zone {}/{}", index + 1, ZONE_CYCLE.len()),
            ZONED_ROW,
            zone,
        );
    }

    script = script
        .hold("hold the flank chip", 0.5)
        .press_generate("press Generate")
        // The landing fills the Scene tree above the block, which pushes the
        // block down the rail; the wheel brings it back under the grown tree.
        .scroll_the_rail_to("bring the block back under the grown tree", LANCE_ROW);
    script = park("after the landing", script);
    script = script.hold("hold the landed hull", 2.5);

    if capturing() {
        script = script
            .step("close the generate loop")
            .on_enter(|world| loop_end(world, GENERATE_LOOP))
            .until(loop_written(GENERATE_LOOP))
            .deadline(120.0)
            .add();
    }

    // Never shoot while a loop is open: the still and the recorder would
    // contend for the one readback target.
    script = script
        .step("size the window for the figures")
        .on_enter(size_the_window_for_the_figures)
        .until(the_window_is_figure_sized())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("capture the plan and the hull")
        .on_enter(shot(PLAN_SHOT))
        .until(shot_written(PLAN_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        // Back to the rail's top before the top bar is touched: the Scene rows
        // the block was wheeled up past are still pickable where they sit,
        // over the File button. See `RAIL_TOP`.
        .scroll_the_rail_to("clear the top bar of scrolled-off rows", RAIL_TOP);
    script = park("before the File menu", script);

    script
        .click("open the File menu", "File Menu Button")
        .click("ask to Save As", "Save As... Item")
        // The caret stays in the field for the still: a name being typed, with
        // the slot the window derives from it printed underneath.
        .retype(
            "name the range",
            "Save Name Field",
            NAME_CLEAR,
            SAVE_NAME,
            false,
        )
        .step("the window says where the name lands")
        .until(the_node_reads(
            "Save Id Readout",
            format!("saves as {EDITOR_ID_PREFIX}{SAVE_SLUG}"),
        ))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("capture the Save As window")
        .on_enter(shot(SAVE_AS_SHOT))
        .until(shot_written(SAVE_AS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
        .click("cancel, writing nothing", "File Cancel Button")
        .step("the window is down")
        .until(not(ui_node_present("Save Name Field")))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
}
