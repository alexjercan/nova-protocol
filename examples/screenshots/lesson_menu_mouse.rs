//! lesson_menu_mouse: the ADVANCED lesson about mouse sensitivity - the one
//! Controls group that holds no bindings at all, three sliders reading in
//! percentages of their own baseline.
//!
//! One producer, one sheet. The walk opens Settings on the real menu, opens the
//! Controls tab, opens the MOUSE group, takes hold of the Look Sensitivity
//! handle and sweeps it through the whole of that path's range and back, with
//! the readout counting under the pointer.
//!
//! ## Why this lesson became a loop
//!
//! It was authored as `still(...)`: a picture of the page. A picture cannot say
//! the thing the lesson is FOR, which is that this group is dragged rather than
//! rebound - a row of tracks and a row of keycaps are the same silhouette at a
//! glance. So the lesson was flipped to `looping(...)` in
//! `crates/nova_authoring/src/base_content/lessons.rs` and its alt text
//! rewritten to describe the footage.
//!
//! Nothing is lost by the flip. The Lessons pane draws BOTH forms into the same
//! 16:9 box (`spawn_media_frame`, `crates/nova_menu/src/training.rs`), so a
//! 1920x1080 still and a 960x540 cell arrive at the reader's eye at the same
//! size - the still's extra pixels are spent on the downscale either way. The
//! cell buys the motion for free.
//!
//! ## Why the sweep is one period of a sine
//!
//! Twenty cells at ten a second is two seconds played back forever, so a
//! one-way drag would jump-cut on every wrap. The pointer therefore travels
//! `reach * sin(2*pi*cell/20)` either side of where it took hold: out to one
//! end of the range, back through the middle, out to the other end, home. The
//! last cell is one step short of the first, exactly as every other pair of
//! cells is one step apart, so the wrap reads as the drag continuing.
//!
//! Look Sensitivity is the row that gets dragged because its range is the one
//! the sine fits: 100% to 300% with the default at 200%, which is DEAD CENTRE.
//! The pointer can spend half the track each way and the readout tours the
//! whole range. On the RCS row (100% to 500%, default 100%) the same gesture
//! would spend three quarters of its travel clamped against the bottom stop.
//!
//! ## Why the grip is the middle of the track
//!
//! nova's tracks carry `TrackClick::Snap` and no `SliderThumb`, so a press
//! takes the value from the fraction of the box it lands in
//! (`slider_on_pointer_down`, `bevy_ui_widgets`) - and the middle of the Look
//! track IS 200%, the value a fresh install starts on. The press that takes
//! hold therefore changes nothing, and the sheet opens on the page as the
//! reader's own game shows it.
//!
//! The drag itself is a real drag: the button is held and the cursor moved, so
//! the value comes out of `bevy_ui_widgets`' own drag arithmetic
//! (`drag.offset + distance * span / track_width`) rather than out of a
//! `SetSliderValue` trigger or a write to `MouseSensitivity` behind the
//! widget's back. The closing assertion reads the resource: if the footage
//! shows a handle moving and the setting did not follow it, the walk fails.
//!
//! ## Why the backdrop is pinned and frozen
//!
//! The menu draws one of four `role: Backdrop` scenarios at random on entry,
//! and its traffic flies. Unpinned, every re-capture would ship a different
//! scene around the panel; unfrozen, the hulls behind it would jump back two
//! seconds on every wrap. So this pins `menu_waystation` - the quietest of the
//! four, a planetoid held at 3.35 km with its freight working the flanks -
//! and freezes every dynamic body while the recorder is armed.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - walk the screens, sweep the
//!   handle for one period, exit clean, recording nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also tile the sheet (staged under
//!   `NOVA_CAPTURE_DIR`). `scripts/capture-lesson-media.sh` runs this and
//!   encodes it as the WebP the handbook ships.
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_menu_mouse --features debug
//! ```

#[cfg(feature = "debug")]
use bevy::prelude::*;
use clap::Parser;
use nova_protocol::prelude::*;

#[cfg(feature = "debug")]
#[path = "shared/lesson.rs"]
mod lesson;
// The pointer gestures, shared with the other menu walks. Script-only, so the
// whole module sits behind one gate here.
#[cfg(feature = "debug")]
#[path = "shared/ui_walk.rs"]
mod ui_walk;
#[cfg(feature = "debug")]
use bevy::ui_widgets::SliderValue;
#[cfg(feature = "debug")]
use lesson::{lesson_profile, LESSON_GRID};
#[cfg(feature = "debug")]
use nova_input::sensitivity::prelude::{MousePath, MouseSensitivity, MouseSensitivityRange};
#[cfg(feature = "debug")]
use ui_walk::{hide_menu_version, Gestures};

#[derive(Parser)]
#[command(name = "lesson_menu_mouse")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's mouse-sensitivity demonstration", long_about = None)]
struct Cli;

/// The sheet this tiles: "Mouse sensitivity".
#[cfg(feature = "debug")]
const MOUSE_LESSON: &str = "advanced_mouse";

/// The backdrop the panel is shot against. See the module docs: pinned so the
/// scene around the panel is the same one on every re-capture.
const BACKDROP: &str = "menu_waystation";

/// The Controls group the lesson is about.
#[cfg(feature = "debug")]
const MOUSE_GROUP: &str = "Controls Group: MOUSE";

/// The track the pointer drags, and the two the sheet shows beside it.
///
/// The names are `build_slider_row`'s (`crates/nova_menu/src/settings.rs`):
/// one row per `MousePath`, named after the path's own label.
#[cfg(feature = "debug")]
const LOOK_TRACK: &str = "Look Sensitivity Slider Track";
#[cfg(feature = "debug")]
const RCS_TRACK: &str = "RCS Sensitivity Slider Track";
#[cfg(feature = "debug")]
const FREE_CAMERA_TRACK: &str = "Free Camera Sensitivity Slider Track";

/// The prefix every binding row on this tab carries - and therefore the prefix
/// the MOUSE page must have none of.
#[cfg(feature = "debug")]
const KEYBIND_PREFIX: &str = "Keybind: ";

/// How far the pointer travels each way, as a fraction of the track's width.
///
/// Half the track spends the whole of Look's range, because the default sits
/// dead centre of it (see the module docs). A little past half would only sit
/// clamped against the stop for a cell or two.
#[cfg(feature = "debug")]
const REACH: f32 = 0.5;

/// Cells of sweep the recorder is handed BEFORE it opens, to pay for the lag
/// between the pointer moving and the page showing it.
///
/// A cursor written in `Update` is turned into a `Pointer<Drag>` by
/// `bevy_picking` against the FOLLOWING frame's hit map, and the value the
/// widget's own `slider_self_update` commits for it reaches the track and the
/// readout after that - so a FRAME shows where the pointer was two cells ago,
/// and the sheet has to be opened that far into the sweep or it stops short of
/// the period and its wrap jumps a quarter of the track.
///
/// The lead is spent INSIDE the sweeping step rather than in a step before it.
/// A step seam costs a frame - the sweep ran on neither side of it - and a cell
/// the pointer did not move on is a cell the page repeats, which showed up as a
/// duplicated frame at the head of the sheet and a missing phase at its tail.
#[cfg(feature = "debug")]
const LEAD_CELLS: u32 = 1;

/// How close to the ends of the range the sweep must actually get before the
/// walk calls the footage good, in percentage points.
///
/// One detent of the Look slider is 10 points
/// (`MouseSensitivityRange::INTERVALS`), so this is well inside a detent: the
/// assertion is "the handle reached the stops", not "the arithmetic rounded the
/// way I expect".
#[cfg(feature = "debug")]
const RANGE_TOLERANCE: f32 = 4.0;

/// How much bigger than the sweep's own biggest step the wrap may be, in
/// percentage points.
///
/// The sheet is played on a cycle, so the step from the last cell back to the
/// first is a step like any other - and a one-way drag, or a sweep the lead-in
/// has mis-phased, shows up here as a jump several steps wide. One point of
/// slack for the float round trip.
#[cfg(feature = "debug")]
const WRAP_TOLERANCE: f32 = 1.0;

/// How far the setting may sit from the slider's own reading at the end of the
/// walk, in percentage points. A float round trip through the raw engine gain,
/// nothing more.
#[cfg(feature = "debug")]
const SETTING_TOLERANCE: f32 = 0.5;

/// Real seconds the sheet is given to record and tile.
///
/// Twenty frames of a lit menu scene on a software Vulkan, plus the tile and
/// the PNG write. The frame count is fixed, so this only catches a sheet that
/// never opened.
#[cfg(feature = "debug")]
const SHEET_DEADLINE_SECS: f32 = 60.0;

/// Where the pointer took hold of the Look track, and how far it may travel.
///
/// Measured once, before the recording opens: the panel does not reflow while
/// a slider is dragged (`settings_tab_dirty` is deliberately not armed by
/// `MouseSensitivity`), so the box the sweep is spread across cannot move
/// under it.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy)]
struct LookGrip {
    /// The press point: the middle of the track, which is also 200%.
    grip: Vec2,
    /// Half the track, in logical pixels - the travel that spends the range.
    reach: f32,
}

/// How far through the sweep the pointer is, in cells.
///
/// The sweep's own counter rather than the step's frame index, because the
/// sweep starts one step BEFORE the recording (see [`LEAD_CELLS`]) and has to
/// carry its phase across the seam.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Copy, Default)]
struct SweepCell(u32);

/// What each cell of the RECORDING was seen to show, in order.
///
/// The sheet is twenty pictures and the walk cannot look at them, so this is
/// what stands in for looking: the slider's own reading on every recorded
/// frame. A drag that never took leaves twenty identical numbers, a mis-phased
/// lead-in leaves a first cell that is not the default, and a sweep that
/// clipped the range leaves ends that never reached the stops - each of which
/// the closing assertion names.
#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone, Default)]
struct Swept(Vec<f32>);

#[cfg(feature = "debug")]
impl Swept {
    /// The biggest step between two cells the sheet plays in sequence.
    fn biggest_step(&self) -> f32 {
        self.0
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max)
    }

    /// The step the WRAP makes: the last cell back to the first.
    fn wrap_step(&self) -> f32 {
        match (self.0.first(), self.0.last()) {
            (Some(first), Some(last)) => (last - first).abs(),
            _ => f32::INFINITY,
        }
    }
}

/// The Look slider's own reading, off the widget rather than off the resource.
#[cfg(feature = "debug")]
fn look_slider_percent(world: &mut World) -> Option<f32> {
    let mut sliders = world.query::<(&Name, &SliderValue)>();
    sliders
        .iter(world)
        .find(|(name, _)| name.as_str() == LOOK_TRACK)
        .map(|(_, value)| value.0)
}

/// Measure the Look track and put the pointer in the middle of it.
#[cfg(feature = "debug")]
fn grip_the_look_track(world: &mut World) {
    let track = ui_node_rect(world, LOOK_TRACK).unwrap_or_else(|| {
        panic!("`{LOOK_TRACK}` has not laid out, so the handle cannot be taken hold of")
    });
    let grip = LookGrip {
        grip: track.center(),
        reach: track.width() * REACH,
    };
    info!(
        "lesson mouse: the Look track is {:.0} px wide at {:?}; the sweep spends {:.0} px each way",
        track.width(),
        grip.grip,
        grip.reach
    );
    world.insert_resource(grip);
    move_cursor(grip.grip)(world);
}

/// Carry the drag one cell further round the sine, and - once the recorder is
/// open - write down what the page shows.
///
/// One driven frame is one cell: the recorder pins the armed run's clock to
/// `LESSON_FPS`, so a rendered frame IS a frame of the sheet. The reading taken
/// here is the page as it stands BEFORE this cell's cursor move reaches it (see
/// [`LEAD_CELLS`]), which is what the frame about to be recorded shows.
#[cfg(feature = "debug")]
fn sweep_the_handle(world: &mut World) {
    let cells = LESSON_GRID.frames();
    let cell = world.resource::<SweepCell>().0;

    // Opened from inside the sweep, on the cell whose RENDER is the untouched
    // page (see `LEAD_CELLS`). `Swept` arrives with it, so what is written
    // down below is exactly what the sheet holds.
    if cell == LEAD_CELLS {
        world.insert_resource(Swept::default());
        sheet_start(world, MOUSE_LESSON, LESSON_GRID);
    }

    // Written down from the cell AFTER the recorder opened, because a reading
    // taken here is one cell behind the render: `Update` sees the value the
    // widget committed for the frame BEFORE this one, while the frame the
    // recorder is about to keep shows the one committed for this one. A record
    // that ignored that would pass a closed loop while the sheet held a jump
    // cut, which is the whole thing it exists to catch.
    if cell > LEAD_CELLS {
        if let Some(percent) = look_slider_percent(world) {
            if let Some(mut swept) = world.get_resource_mut::<Swept>() {
                if swept.0.len() < cells as usize {
                    swept.0.push(percent);
                    let recorded = swept.0.len();
                    debug!("lesson mouse: recorded cell {recorded} shows {percent:.0}%");
                }
            }
        }
    }

    let grip = *world.resource::<LookGrip>();
    let phase = (cell % cells) as f32 / cells as f32 * std::f32::consts::TAU;
    let at = Vec2::new(grip.grip.x + grip.reach * phase.sin(), grip.grip.y);
    move_cursor(at)(world);
    world.resource_mut::<SweepCell>().0 = cell.wrapping_add(1);
}

/// Assert the page is the one the lesson describes: three sensitivity sliders,
/// and nothing to rebind.
///
/// Read off the SCREEN, not off the group resource. A click that opened the
/// wrong group leaves a page of keycaps that photographs perfectly well.
#[cfg(feature = "debug")]
fn the_group_is_three_sliders(world: &mut World) {
    for track in [LOOK_TRACK, RCS_TRACK, FREE_CAMERA_TRACK] {
        assert!(
            ui_node_rect(world, track).is_some(),
            "the MOUSE group must show every mouse path's slider: `{track}` is not on the page"
        );
    }
    let bindings: Vec<String> = world
        .query::<&Name>()
        .iter(world)
        .filter(|name| name.as_str().starts_with(KEYBIND_PREFIX))
        .map(|name| name.as_str().to_string())
        .collect();
    assert!(
        bindings.is_empty(),
        "the MOUSE group is the one Controls page with no bindings on it, and this one carries \
         {}: {bindings:?}",
        bindings.len()
    );
}

/// Assert the recorded cells are a whole period of the sweep: they open on the
/// page as a fresh install shows it, they reach both stops, and they close.
#[cfg(feature = "debug")]
fn the_sweep_spent_the_range(world: &mut World) {
    let swept = world.resource::<Swept>().clone();
    let cells = LESSON_GRID.frames() as usize;
    assert_eq!(
        swept.0.len(),
        cells,
        "the recording is {cells} cells, and the sweep wrote down {}: {:?}",
        swept.0.len(),
        swept.0
    );
    let range = MousePath::Look.range();
    let floor = MouseSensitivityRange::MIN_PERCENT;
    let low = swept.0.iter().copied().fold(f32::INFINITY, f32::min);
    let high = swept.0.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let opening = swept.0[0];
    assert!(
        (opening - range.default_percent).abs() <= RANGE_TOLERANCE,
        "the sheet must open on the page as a fresh install shows it ({}%), and the first cell \
         reads {opening:.1}% - the lead-in is mis-phased",
        range.default_percent
    );
    assert!(
        low <= floor + RANGE_TOLERANCE,
        "the sweep must carry the handle down to {floor}%: the lowest cell reads {low:.1}%"
    );
    assert!(
        high >= range.max_percent - RANGE_TOLERANCE,
        "the sweep must carry the handle up to {}%: the highest cell reads {high:.1}%",
        range.max_percent
    );
    let wrap = swept.wrap_step();
    let step = swept.biggest_step();
    assert!(
        wrap <= step + WRAP_TOLERANCE,
        "the sheet plays on a cycle, so the wrap must be a step like any other: the biggest step \
         inside the sheet is {step:.1} points and the wrap is {wrap:.1} ({:?})",
        swept.0
    );
    let live = world
        .resource::<MouseSensitivity>()
        .percent(MousePath::Look);
    let handle = look_slider_percent(world).expect("the page is still up");
    assert!(
        (live - handle).abs() <= SETTING_TOLERANCE,
        "the slider writes the setting as it is dragged: the handle reads {handle:.1}% and the \
         live look sensitivity is {live:.1}%"
    );
    info!(
        "lesson mouse: twenty cells of {low:.0}%..{high:.0}%, a {step:.0}-point step and a \
         {wrap:.0}-point wrap, with the setting on {live:.0}%"
    );
}

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();

    // Pinned before the app boots, the way `screenshot_menu` pins its own: the
    // packaging script runs every producer through one generic command, so a
    // producer that needs an environment carries it itself.
    std::env::set_var(MENU_BACKDROP_ENV, BACKDROP);

    // The same app the game binary runs: the main menu over its live backdrop.
    let mut app = editor_app(true, None);

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_plugins(nova_protocol::nova_debug::harness::LoopCapturePlugin::new(
            lesson_profile(),
        ));
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            app.insert_resource(bevy::ecs::error::FallbackErrorHandler(
                bevy::ecs::error::panic,
            ));
        }
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        // Under the recorder only: the backdrop's freight is what would
        // jump-cut on the wrap, and a smoke run has nothing to wrap.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
        app.add_plugins(menu_mouse_script());
    }

    app.run()
}

/// Menu -> Settings -> Controls -> MOUSE, then a drag of the Look handle,
/// recorded.
#[cfg(feature = "debug")]
fn menu_mouse_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    // Re-aimed every frame of the beat that takes hold: the panel has just
    // been rebuilt by the group click, and the pointer must be over the track
    // the frame the button goes down or the press lands on the page behind it.
    let hold_the_track = keep_hovering_named(LOOK_TRACK);

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("reach the main menu")
        .enter(GameStates::Loading)
        .until(state_is(GameStates::MainMenu))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("settle the menu and its ambience backdrop")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("open Settings", "Settings Button")
        .step("settle the settings panel")
        .until(frames(SETTLE_FRAMES))
        .add()
        // The panel is toggled by Visibility alone, so this asserts VISIBLE
        // rather than laid out: the sheet would otherwise be the bare menu.
        .step("the settings panel is up")
        .on_enter(assert_named_visible("Settings Panel"))
        .add()
        .click("open the Controls tab", "Settings Tab: Controls")
        .step("settle the controls tab")
        .until(frames(SETTLE_FRAMES))
        .add()
        .click("open the MOUSE group", MOUSE_GROUP)
        .step("settle the mouse group")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("the group is three sliders and no bindings")
        .on_enter(the_group_is_three_sliders)
        .add()
        // The card's footer reads `v<version>+<commit>`, and its corner is in
        // frame behind the panel. Art the game SHIPS cannot carry a build id.
        .step("take the build footer out of shot")
        .on_enter(hide_menu_version)
        .add()
        // Aimed and pressed BEFORE the recording opens, so the twenty cells
        // hold nothing but the drag.
        .step("take hold of the Look handle")
        .on_enter(grip_the_look_track)
        .each(move |world: &mut World, _, _| hold_the_track(world))
        .until(pointer_over_node(LOOK_TRACK))
        .deadline(STEP_DEADLINE_SECS)
        .diagnose(pointer_hover_diagnosis(LOOK_TRACK))
        .add()
        .step("press on the middle of the track")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // The press SNAPS the value to where it landed. That is 200% - the
        // value already on the row - but the snap still travels through the
        // observers, and the sheet must open on a settled page.
        .step("let the snap settle")
        .until(frames(2))
        .add()
        // ONE beat: the lead, the recording and the tail of the sweep, with
        // the recorder opened from inside it (see `LEAD_CELLS`).
        .step("sweep the handle through the whole range and back")
        .on_enter(|world: &mut World| world.insert_resource(SweepCell::default()))
        .each(|world: &mut World, _, _| sweep_the_handle(world))
        // Both halves, because they are true at different times in the two run
        // modes: armed, the sheet closes itself at its twentieth frame and
        // tiling takes a few more; on the smoke path `sheet_written` is true at
        // once and the frame count is what plays the whole sweep.
        .until(and(
            sheet_written(MOUSE_LESSON),
            frames(LEAD_CELLS + LESSON_GRID.frames() + 3),
        ))
        .deadline(SHEET_DEADLINE_SECS)
        .add()
        .step("put the handle back where it was found")
        .on_enter(|world: &mut World| {
            let grip = world.resource::<LookGrip>().grip;
            move_cursor(grip)(world);
        })
        .until(frames(2))
        .add()
        .step("let go of the handle")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("the sweep spent the whole range, and the setting followed")
        .on_enter(the_sweep_spent_the_range)
        .add()
        // Out through the menu's own front door. A harnessed example owes the
        // smoke contract either way: a run that ends in a settings modal cannot
        // be told from an app that died while still loading, which is what
        // `reached_playing` is there to catch.
        .click("close Settings", "Settings Back Button")
        .click("start a new game", "New Game Button")
        .click("create the world", "Create World Button")
        .step("reach the first flight")
        .until(state_is(GameStates::Playing))
        .deadline(STEP_DEADLINE_SECS)
        .add()
}
