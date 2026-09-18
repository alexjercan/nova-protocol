//! widget_zoo: a live, FUNCTIONAL showcase of the nova_ui widget library (the
//! `nova_ui::widget` factories) in both base themes - the same constructors the
//! game spawns (`button(...)`, `segmented(...)`, `slider_track(...)`,
//! `checkbox(...)`, `toggle(...)`, `badge(...)`, `panel(...)`,
//! `list_row(...)`). Everything here is interactive: the buttons hover/press,
//! the Theme control repaints the whole zoo live WITHOUT respawning a widget,
//! the segmented selects, the checkboxes/toggles flip, and the slider drags
//! (its phosphor block-meter tracks the value).
//!
//! It doubles as the render eyeball: phosphor reads as flat CLI elements,
//! hardware as light-3D moulded controls.
//!
//! Interactive run:  `cargo run --example widget_zoo`  (drag the slider, click
//! the Theme control / checks / toggles; `S` also flips the theme).
//! Capture both themes: `NOVA_CAPTURE=1 NOVA_CAPTURE_DIR=target/zoo cargo run
//! --example widget_zoo --features debug` -> widget_zoo-{phosphor,hardware}.png
//! then exit. (`--features debug` because the shot goes through the fleet's
//! shared `capture_window`.)
//!
//! Headless smoke test (needs a display, e.g. `Xvfb :99 & DISPLAY=:99`):
//! ```text
//! NOVA_AUTOPILOT=1 cargo run --example widget_zoo --features debug
//! # look for: `nova harness: reached Playing`,
//! #           `zoo: <beat>` verdict lines,
//! #           `autopilot: cycle complete, no panic`
//! ```
//!
//! The zoo carries `GameStates` and reaches `Playing` in `Startup` purely so
//! the shared harness can drive it. It is NOT a gameplay app: "reached Playing"
//! here means "the widget library is up", nothing more. The transition is
//! unconditional so an interactive run and a harnessed one take the identical
//! path.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{
        observe, Activate, Slider, SliderRange, SliderStep, SliderValue, TrackClick, ValueChange,
    },
};
use clap::Parser;
use nova_protocol::prelude::GameStates;
use nova_ui::{
    prelude::*,
    theme::{base::base_ui_themes, ActiveUiTheme, UiColor, HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
    widget::{ButtonSpec, ThemedFill, ThemedText, UiText},
    NovaUiPlugin,
};

#[derive(Parser)]
#[command(name = "widget_zoo")]
#[command(version = "1.0.0")]
#[command(
    about = "Every nova_ui widget factory, live and clickable, in both base themes",
    long_about = None
)]
struct Cli;

// The `Name`s the harness clicks by. Resolving a target by name rather than by
// coordinates is what makes a run survive a layout move: only a RENAME breaks a
// beat.
const IDLE_BUTTON: &str = "Zoo Idle Button";
const THEME_HARDWARE: &str = "Zoo Theme Hardware";
const THEME_PHOSPHOR: &str = "Zoo Theme Phosphor";
const LEVEL_ALL: &str = "Zoo Level All";
const LEVEL_MINIMAL: &str = "Zoo Level Minimal";
const LEVEL_NONE: &str = "Zoo Level None";
const CHECK_FIRST: &str = "Zoo Check First";
const CHECK_SECOND: &str = "Zoo Check Second";
const TOGGLE_FIRST: &str = "Zoo Toggle First";
const TOGGLE_SECOND: &str = "Zoo Toggle Second";
const SLIDER: &str = "Zoo Slider";

fn main() -> AppExit {
    let _ = Cli::parse();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1280, 860).into(),
            title: "nova_ui widget library".into(),
            ..default()
        }),
        ..default()
    }));
    // The shared widget observers + theme reconcilers + font router.
    app.add_plugins(NovaUiPlugin);
    // The zoo loads no mod content, so it registers the two base themes itself -
    // the same `Vec<UiThemeConfig>` the base mod's generated RON carries.
    app.insert_resource(GameUiThemes(base_ui_themes()));
    // The Theme control + the demo HUD-level control drive their resources
    // through the same `button_on_setting` path the game's Settings use.
    app.add_observer(button_on_setting::<SelectedUiTheme>);
    app.add_observer(button_on_setting::<DemoLevel>);
    app.init_resource::<DemoLevel>();
    app.init_resource::<ZooChecks>();
    app.insert_resource(ZooSliderValue(0.66));
    #[cfg(feature = "debug")]
    app.init_resource::<Capture>();
    // The state machine the shared harness drives, and the one-shot transition
    // that satisfies it. Unconditional: the interactive run must not sit in a
    // `Loading` it never leaves.
    app.init_state::<GameStates>();
    app.add_systems(Startup, (setup, reach_playing));
    app.add_systems(
        Update,
        (
            // NOT on a theme change: every widget here repaints itself through
            // its own reconciler, so a theme flip that needed a rebuild would be
            // the bug this zoo exists to catch.
            rebuild_body.run_if(resource_changed::<ZooChecks>),
            repaint_backdrop.after(UiThemeSystems),
            toggle_theme_key,
            sync_theme_marks.run_if(resource_changed::<SelectedUiTheme>),
        ),
    );
    // The two-theme capture pass. Behind `debug` because it shoots through the
    // shared `capture_window`, which is where `NOVA_CAPTURE_DIR` is resolved for
    // the whole fleet - the zoo used to resolve it a second time itself.
    #[cfg(feature = "debug")]
    app.add_systems(Update, drive_capture);

    // Headless smoke-test harness: inert in a normal run (gated on NOVA_AUTOPILOT).
    #[cfg(feature = "debug")]
    {
        // Probe wiring, matching every other harnessed example (each plugin is
        // inert without its NOVA_PROBE_* env). `nova_invariants` takes its world
        // resource by `Option` and its component queries simply find nothing in
        // a bare app, so it needs no special-casing here.
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        if std::env::var_os("NOVA_AUTOPILOT").is_some() {
            // The zoo does not carry `nova_debug`'s `DebugPlugin` (it would drag
            // an inspector, a wireframe toggle and gameplay state into a widget
            // showcase), so it logs the smoke sentinel itself - the same const,
            // so the two cannot drift.
            app.add_systems(OnEnter(GameStates::Playing), || {
                info!("{}", nova_protocol::nova_debug::harness::REACHED_PLAYING)
            });
        }
        app.init_resource::<ZooProbe>();
        app.add_plugins(zoo_script());
    }

    app.run()
}

/// The zoo's whole state machine: one unconditional step into `Playing`.
fn reach_playing(mut next: ResMut<NextState<GameStates>>) {
    next.set(GameStates::Playing);
}

/// The demo "HUD level" the middle segmented control drives (a stand-in for a
/// real `ButtonValue<T>` settings row).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
enum DemoLevel {
    #[default]
    All,
    Minimal,
    None,
}

/// The flip state of the four interactive checks/toggles, by id.
#[derive(Resource, Clone, Copy)]
struct ZooChecks([bool; 4]);

impl Default for ZooChecks {
    fn default() -> Self {
        Self([true, false, true, false])
    }
}

/// The draggable slider's value (`0..1`), kept in a resource so a body rebuild
/// (a checks change) restores the slider where the player left it.
#[derive(Resource)]
struct ZooSliderValue(f32);

#[derive(Component)]
struct ZooRoot;
#[derive(Component)]
struct ZooBody;
/// A clickable check/toggle carrying its index into [`ZooChecks`].
#[derive(Component)]
struct CheckId(usize);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.insert_resource(UiFont(
        asset_server.load("fonts/SGr-IosevkaTerm-Medium.ttf"),
    ));
    commands
        .spawn((
            ZooRoot,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(24)),
                row_gap: px(18),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::new(UiColor::Void),
            // The glows are painted by `repaint_backdrop`, which has the live
            // theme to mix them from. A gradient is not a plain colour, so no
            // `Themed*` marker covers it.
            BackgroundGradient(Vec::new()),
        ))
        .with_children(|root| {
            top_bar(root);
        });
    // The body is spawned by `rebuild_body` on the first frame (`ZooChecks` is
    // "changed" at startup), so there is one code path for spawn + rebuild.
}

/// The persistent header: title + the live Theme control (Phosphor | Hardware),
/// a functional `ButtonValue<SelectedUiTheme>` segmented row.
fn top_bar(root: &mut ChildSpawnerCommands) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(16),
        ..default()
    })
    .with_children(|bar| {
        bar.spawn((
            UiText,
            Text::new("NOVA UI // WIDGET LIBRARY"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Primary),
        ));
        bar.spawn((
            UiText,
            Text::new("Theme"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Label),
            Node {
                margin: UiRect::left(px(12)),
                ..default()
            },
        ));
        // A functional segmented: each option carries
        // `ButtonValue<SelectedUiTheme>`, so a click drives the shared selection
        // and the resolver repaints the whole zoo.
        bar.spawn(segmented_container()).with_children(|seg| {
            for (label, id, name) in [
                ("Phosphor", PHOSPHOR_THEME_ID, THEME_PHOSPHOR),
                ("Hardware", HARDWARE_THEME_ID, THEME_HARDWARE),
            ] {
                let mut b = seg.spawn((
                    segmented_option(label),
                    ButtonValue(SelectedUiTheme(id.to_string())),
                    Name::new(name),
                ));
                if id == PHOSPHOR_THEME_ID {
                    b.insert(Selected);
                }
            }
        });
    });
}

/// (Re)build the panel grid for the current interactive state. One path serves
/// the first spawn and every check flip. A THEME change never comes through
/// here: the widgets repaint themselves.
fn rebuild_body(
    mut commands: Commands,
    checks: Res<ZooChecks>,
    slider: Res<ZooSliderValue>,
    roots: Query<Entity, With<ZooRoot>>,
    bodies: Query<Entity, With<ZooBody>>,
) {
    for body in &bodies {
        commands.entity(body).despawn();
    }
    let Ok(root) = roots.single() else {
        return;
    };
    let checks = *checks;
    let value = slider.0;
    commands.entity(root).with_children(|root| {
        root.spawn((
            ZooBody,
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Start,
                column_gap: px(16),
                row_gap: px(16),
                width: percent(100),
                ..default()
            },
        ))
        .with_children(|body| {
            buttons_panel(body);
            controls_panel(body, checks, value);
            content_panel(body, checks);
        });
    });
}

/// A titled panel with a padded body.
fn panel_cell(
    body: &mut ChildSpawnerCommands,
    title: &str,
    tag: Option<&str>,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    body.spawn((panel_node(), panel())).with_children(|cell| {
        cell.spawn((panel_head(title, tag),));
        cell.spawn(Node {
            width: px(320),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(16)),
            ..default()
        })
        .with_children(build);
    });
}

fn buttons_panel(body: &mut ChildSpawnerCommands) {
    panel_cell(body, "Buttons", None, |c| {
        sub_header(c, "States");
        c.spawn(flow_row()).with_children(|r| {
            r.spawn((button(ButtonSpec::new("Idle")), Name::new(IDLE_BUTTON)));
            r.spawn((button(ButtonSpec::new("Pressed")), bevy::ui::Pressed));
            r.spawn((button(ButtonSpec::new("Selected")), Selected));
            r.spawn((
                button(ButtonSpec::new("Disabled")),
                bevy::ui::InteractionDisabled,
            ));
        });
        note(
            c,
            "Hover for the hover face; selection inverts to dark glyphs.",
        );
        sub_header(c, "Emphasis");
        c.spawn(flow_row()).with_children(|r| {
            r.spawn(button(ButtonSpec::new("Primary").primary()));
            r.spawn(button(ButtonSpec::new("Default")));
            r.spawn(button(ButtonSpec::new("Ghost").ghost()));
            r.spawn(button(ButtonSpec::new("Danger").danger()));
        });
        sub_header(c, "Block + key-chip");
        c.spawn(button(
            ButtonSpec::new("Play").primary().block().key("Enter"),
        ));
    });
}

fn controls_panel(body: &mut ChildSpawnerCommands, checks: ZooChecks, value: f32) {
    panel_cell(body, "Controls", None, |c| {
        sub_header(c, "Segmented (HUD detail)");
        // Functional: `ButtonValue<DemoLevel>` drives the DemoLevel resource.
        c.spawn(segmented_container()).with_children(|seg| {
            for (label, level, name) in [
                ("All", DemoLevel::All, LEVEL_ALL),
                ("Minimal", DemoLevel::Minimal, LEVEL_MINIMAL),
                ("None", DemoLevel::None, LEVEL_NONE),
            ] {
                let mut b =
                    seg.spawn((segmented_option(label), ButtonValue(level), Name::new(name)));
                if level == DemoLevel::All {
                    b.insert(Selected);
                }
            }
        });
        sub_header(c, "Slider (drag me)");
        // A real draggable `bevy_ui_widgets::Slider` wearing the shared track;
        // `on_slider_change` writes the value, and nova_ui's
        // `sync_slider_tracks` shows it - lighting the phosphor block-meter or
        // moving the hardware fill.
        c.spawn((
            Name::new(SLIDER),
            slider_track(value),
            Slider {
                track_click: TrackClick::Snap,
                ..default()
            },
            SliderValue(value),
            SliderRange::new(0.0, 1.0),
            SliderStep(0.02),
            Hovered::default(),
            observe(on_slider_change),
        ));
        sub_header(c, "Checks + toggles (click)");
        c.spawn(flow_row()).with_children(|r| {
            // Named HERE and not inside `clickable`: the Content panel's list
            // rows wrap the same ids, and a driven name must resolve to exactly
            // one entity.
            r.spawn((clickable(checkbox(checks.0[0]), 0), Name::new(CHECK_FIRST)));
            r.spawn((clickable(checkbox(checks.0[1]), 1), Name::new(CHECK_SECOND)));
            r.spawn((clickable(toggle(checks.0[2]), 2), Name::new(TOGGLE_FIRST)));
            r.spawn((clickable(toggle(checks.0[3]), 3), Name::new(TOGGLE_SECOND)));
        });
    });
}

fn content_panel(body: &mut ChildSpawnerCommands, checks: ZooChecks) {
    panel_cell(body, "Content", Some("DELTA-9"), |c| {
        sub_header(c, "List rows");
        list_row_entry(c, checks.0[0], "Deep salvage", "v1.2 // nova.labs", 0);
        list_row_entry(c, checks.0[1], "Hard vacuum", "v0.4 // driftco", 1);
        sub_header(c, "Badges");
        c.spawn(flow_row()).with_children(|r| {
            r.spawn(badge(BadgeKind::Green, "online"));
            r.spawn(badge(BadgeKind::Amber, "warn"));
            r.spawn(badge(BadgeKind::Blue, "info"));
            r.spawn(badge(BadgeKind::Red, "fault"));
            r.spawn(badge(BadgeKind::Mute, "idle"));
        });
    });
}

/// A list row with a title/subtitle and a trailing clickable checkbox that
/// shares its enabled bit with the matching Controls checkbox.
fn list_row_entry(c: &mut ChildSpawnerCommands, on: bool, title: &str, sub: &str, id: usize) {
    let title = title.to_string();
    let sub = sub.to_string();
    c.spawn(list_row()).with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                UiText,
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Body),
            ));
            col.spawn((
                UiText,
                Text::new(sub),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Secondary),
            ));
        });
        row.spawn(clickable(checkbox(on), id));
    });
}

/// Wrap a visual widget (checkbox/toggle) so it receives clicks: add `Button` +
/// `Hovered` + its `CheckId` + the flip observer.
fn clickable(widget: impl Bundle, id: usize) -> impl Bundle {
    (
        widget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        CheckId(id),
        observe(on_check_click),
    )
}

/// Flip the clicked check/toggle's bit; `rebuild_body` re-renders it for the new
/// state (and every sibling that mirrors the same bit, e.g. the mods rows).
fn on_check_click(activate: On<Activate>, q: Query<&CheckId>, mut checks: ResMut<ZooChecks>) {
    if let Ok(CheckId(id)) = q.get(activate.entity) {
        checks.0[*id] = !checks.0[*id];
    }
}

/// Commit a slider drag: store the value so a body rebuild restores it. The
/// track itself is shown by nova_ui's shared `sync_slider_tracks` (wired by
/// `NovaUiPlugin`) reacting to the `SliderValue` change - it lights the phosphor
/// block-meter and moves the hardware fill, so there is no per-site recolour.
fn on_slider_change(change: On<ValueChange<f32>>, mut value: ResMut<ZooSliderValue>) {
    value.0 = change.value.clamp(0.0, 1.0);
}

/// The two soft nebula glows behind the panels (the demo scene backdrop), mixed
/// from the live theme.
///
/// Its own system because a `BackgroundGradient` carries whole `Color` stops
/// rather than a semantic name, so there is no marker component to hang it on.
fn repaint_backdrop(
    theme: Res<ActiveUiTheme>,
    mut roots: Query<&mut BackgroundGradient, With<ZooRoot>>,
) {
    if !theme.is_changed() {
        return;
    }
    for mut gradient in &mut roots {
        gradient.0 = vec![
            Gradient::from(RadialGradient::new(
                UiPosition::TOP_LEFT,
                RadialGradientShape::FarthestSide,
                vec![
                    ColorStop::percent(theme.color_alpha(UiColor::Info, 0.10), 0.0),
                    ColorStop::percent(Color::NONE, 40.0),
                ],
            )),
            Gradient::from(RadialGradient::new(
                UiPosition::BOTTOM_RIGHT,
                RadialGradientShape::FarthestSide,
                vec![
                    ColorStop::percent(theme.color_alpha(UiColor::Primary, 0.06), 0.0),
                    ColorStop::percent(Color::NONE, 42.0),
                ],
            )),
        ];
    }
}

/// `S` flips the theme (the segmented control does the same via ButtonValue).
fn toggle_theme_key(keys: Res<ButtonInput<KeyCode>>, mut selected: ResMut<SelectedUiTheme>) {
    if keys.just_pressed(KeyCode::KeyS) {
        let next = if selected.0 == PHOSPHOR_THEME_ID {
            HARDWARE_THEME_ID
        } else {
            PHOSPHOR_THEME_ID
        };
        *selected = SelectedUiTheme(next.to_string());
    }
}

/// Move the Theme control's mark onto whichever theme is live.
///
/// `button_on_setting` moves `Selected` when a button is PRESSED, and the two
/// other ways the selection moves here - the `S` key and the capture pass -
/// write the resource directly. Without this the chip stayed on Phosphor while
/// the whole zoo was drawn in hardware, which is exactly the disagreement the
/// zoo exists to catch.
fn sync_theme_marks(
    mut commands: Commands,
    selected: Res<SelectedUiTheme>,
    buttons: Query<(Entity, &ButtonValue<SelectedUiTheme>, Has<Selected>)>,
) {
    for (entity, value, marked) in &buttons {
        let wanted = value.0 == *selected;
        if wanted == marked {
            continue;
        }
        let mut entity = commands.entity(entity);
        if wanted {
            entity.try_insert(Selected);
        } else {
            entity.try_remove::<Selected>();
        }
    }
}

/// A section sub-header inside a panel body (demo `h3.sub`).
fn sub_header(c: &mut ChildSpawnerCommands, text: &str) {
    c.spawn((
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Label),
        Node {
            margin: UiRect::top(px(4)),
            ..default()
        },
    ));
}

/// A small muted caption line.
fn note(c: &mut ChildSpawnerCommands, text: &str) {
    c.spawn((
        UiText,
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Secondary),
    ));
}

fn flow_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(8),
        row_gap: px(8),
        align_items: AlignItems::Center,
        ..default()
    }
}

// The segmented container + option come from nova_ui (`segmented_container` /
// `segmented_option`) - the same helpers the game's settings rows use.

#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct Capture {
    stage: u32,
    wait: u32,
    /// A shot whose PNG is still being written. The machine holds here until
    /// `CaptureLog` acks it, rather than counting off a guessed number of
    /// frames - the last one gates `AppExit`, so a short guess exits the app
    /// out from under the write.
    pending: Option<String>,
}

#[cfg(feature = "debug")]
fn drive_capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut selected: ResMut<SelectedUiTheme>,
    mut exit: MessageWriter<AppExit>,
    log: Option<Res<nova_protocol::prelude::CaptureLog>>,
) {
    if !nova_protocol::prelude::capturing() {
        return;
    }
    if cap.wait > 0 {
        cap.wait -= 1;
        return;
    }
    if let Some(shot) = cap.pending.clone() {
        if !log.is_some_and(|log| log.wrote(&shot)) {
            return;
        }
        cap.pending = None;
    }
    match cap.stage {
        0 => {
            cap.stage = 1;
            cap.wait = 60;
        }
        1 => {
            shoot("widget_zoo-phosphor.png", &mut commands);
            cap.pending = Some("widget_zoo-phosphor.png".to_string());
            cap.stage = 2;
        }
        2 => {
            *selected = SelectedUiTheme(HARDWARE_THEME_ID.to_string());
            cap.stage = 3;
            cap.wait = 60;
        }
        3 => {
            shoot("widget_zoo-hardware.png", &mut commands);
            cap.pending = Some("widget_zoo-hardware.png".to_string());
            cap.stage = 4;
        }
        _ => {
            exit.write(AppExit::Success);
        }
    }
}

/// Capture the window to `name` (relative paths stage under `NOVA_CAPTURE_DIR`).
///
/// Queued as a command rather than spawned here: `capture_window` is the
/// fleet's one capture primitive and takes `&mut World`, which is also what
/// resolves `NOVA_CAPTURE_DIR` in exactly one place.
#[cfg(feature = "debug")]
fn shoot(name: &str, commands: &mut Commands) {
    let name = name.to_string();
    commands.queue(move |world: &mut World| {
        nova_protocol::prelude::capture_window(world, &name);
        info!("widget_zoo capture: {name}");
    });
}

//
// The zoo DRIVES its own widgets with synthesized pointer input and checks the
// live tree afterwards. Every beat is a real gesture at a real screen position,
// resolved from the widget's `Name`; nothing reaches a widget by triggering its
// observer or inserting its state component.

/// What a beat measured, so the beat after it can say whether the gesture
/// actually changed anything. Comparing against a RECORDED value keeps the
/// verdicts free of nova_ui's private paint tables - the claim under test is
/// "hovering lights the face", not "the face is #rrggbb".
#[cfg(feature = "debug")]
#[derive(Resource, Default)]
struct ZooProbe {
    idle_alpha: f32,
    hover_alpha: f32,
    slider_after_press: f32,
}

/// The whole driven run, one beat per gesture.
///
/// A gesture beat and its VERDICT beat are separate on purpose: the gesture's
/// effect is only visible after the frames the widget needs, and a verdict that
/// panics names the beat it belongs to instead of stalling the step it was
/// folded into.
#[cfg(feature = "debug")]
fn zoo_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    use nova_protocol::prelude::{
        hover_named, move_cursor, pointer_at_node, pointer_pressed, pointer_released, press_mouse,
        release_mouse, resource_where, ui_node_centre, ui_node_diagnosis, ui_node_present,
        BEAT_DEADLINE_SECS,
    };

    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        // Let the first body spawn and lay out before anything is pointed at.
        .step("zoo: the panel is up")
        .until(ui_node_present(IDLE_BUTTON))
        .diagnose(ui_node_diagnosis(IDLE_BUTTON))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: hover a button")
        .on_enter({
            // Record the idle face BEFORE the pointer arrives; a step has one
            // `on_enter`, so the measure and the gesture share it.
            let hover = hover_named(IDLE_BUTTON);
            move |world: &mut World| {
                let alpha = background_alpha(world, IDLE_BUTTON);
                world.resource_mut::<ZooProbe>().idle_alpha = alpha;
                hover(world);
            }
        })
        .until(pointer_at_node(IDLE_BUTTON, Vec2::ZERO))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: the hover face is lit")
        .on_enter(|world: &mut World| {
            let button = named(world, IDLE_BUTTON);
            assert!(
                world
                    .get::<Hovered>(button)
                    .is_some_and(|hovered| hovered.get()),
                "the pointer is over {IDLE_BUTTON} but picking never hovered it"
            );
            let alpha = background_alpha(world, IDLE_BUTTON);
            let idle = world.resource::<ZooProbe>().idle_alpha;
            assert!(
                alpha > idle,
                "hovering {IDLE_BUTTON} must repaint it (idle bg alpha {idle}, hovered {alpha})"
            );
            world.resource_mut::<ZooProbe>().hover_alpha = alpha;
            info!("zoo: hover face lit ({idle} -> {alpha})");
        })
        .add()
        .step("zoo: press the hovered button")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: the pressed face is lit")
        .on_enter(|world: &mut World| {
            let button = named(world, IDLE_BUTTON);
            assert!(
                world.get::<bevy::ui::Pressed>(button).is_some(),
                "a press over {IDLE_BUTTON} must mark it Pressed"
            );
            let alpha = background_alpha(world, IDLE_BUTTON);
            let hover = world.resource::<ZooProbe>().hover_alpha;
            assert!(
                alpha > hover,
                "pressing {IDLE_BUTTON} must repaint it past the hover face \
                 (hovered bg alpha {hover}, pressed {alpha})"
            );
            info!("zoo: pressed face lit ({hover} -> {alpha})");
        })
        .add()
        .step("zoo: release the button")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        // Retheme: a full click on the Theme control's Hardware option, waited
        // out on the SETTING - `Activate` fires on release, and the option is
        // one of a set, so "the pointer let go" is not yet "the theme took".
        .click_named(
            "zoo: click Hardware",
            THEME_HARDWARE,
            resource_where::<SelectedUiTheme>(|theme| theme.0 == HARDWARE_THEME_ID),
            BEAT_DEADLINE_SECS,
        )
        .step("zoo: the theme flipped")
        .on_enter(|world: &mut World| {
            assert_eq!(
                world.resource::<SelectedUiTheme>().0,
                HARDWARE_THEME_ID,
                "clicking {THEME_HARDWARE} must drive the shared SelectedUiTheme"
            );
            info!("zoo: theme is {HARDWARE_THEME_ID}");
            // Nothing was rebuilt for the flip - every widget repainted in
            // place - so this is where a reconciler that ghosts or duplicates
            // would show.
            assert_live_tree(world, "after the theme flip");
        })
        .add()
        .click_named(
            "zoo: click a HUD-level option",
            LEVEL_MINIMAL,
            resource_where::<DemoLevel>(|level| *level == DemoLevel::Minimal),
            BEAT_DEADLINE_SECS,
        )
        .step("zoo: the HUD level changed")
        .on_enter(|world: &mut World| {
            assert!(
                *world.resource::<DemoLevel>() == DemoLevel::Minimal,
                "clicking {LEVEL_MINIMAL} must drive the DemoLevel resource"
            );
            info!("zoo: demo level is Minimal");
        })
        .add()
        // Each flip RESPAWNS the body, so the toggle is clicked by name on the
        // tree the checkbox's rebuild left behind - which is the whole point of
        // waiting for the widget instead of aiming at a remembered box.
        .click_named(
            "zoo: click a checkbox",
            CHECK_FIRST,
            resource_where::<ZooChecks>(|checks| checks.0[0] != ZooChecks::default().0[0]),
            BEAT_DEADLINE_SECS,
        )
        .click_named(
            "zoo: click a toggle",
            TOGGLE_FIRST,
            resource_where::<ZooChecks>(|checks| checks.0[2] != ZooChecks::default().0[2]),
            BEAT_DEADLINE_SECS,
        )
        .step("zoo: both flips landed")
        .on_enter(|world: &mut World| {
            let checks = *world.resource::<ZooChecks>();
            let expected = {
                let mut default = ZooChecks::default().0;
                default[0] = !default[0];
                default[2] = !default[2];
                default
            };
            assert_eq!(
                checks.0, expected,
                "clicking {CHECK_FIRST} and {TOGGLE_FIRST} must flip exactly their own bits"
            );
            info!("zoo: checks are {:?}", checks.0);
            // The second rebuild trigger: a check flip respawns the body too.
            assert_live_tree(world, "after the check flip");
        })
        .add()
        // The slider drag: hover the track, press (which SNAPS the value to the
        // press position), then move along the track and release.
        .step("zoo: hover the slider")
        .on_enter(hover_named(SLIDER))
        .until(pointer_at_node(SLIDER, Vec2::ZERO))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: press on the slider")
        .on_enter(press_mouse(MouseButton::Left))
        .until(pointer_pressed())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: drag along the track")
        .on_enter(|world: &mut World| {
            // The press already SNAPPED the value to the press position; record
            // that, so the verdict beat measures the DRAG and not the snap.
            world.resource_mut::<ZooProbe>().slider_after_press =
                world.resource::<ZooSliderValue>().0;
            let Some(centre) = ui_node_centre(world, SLIDER) else {
                panic!("the slider must be laid out before it can be dragged");
            };
            // A drag is a sequence of moves, not a teleport: `Pointer<Drag>`
            // reports the distance from the press, and one leg is enough to
            // move the value while staying inside the track.
            move_cursor(centre + Vec2::new(SLIDER_DRAG_PX * 0.5, 0.0))(world);
            move_cursor(centre + Vec2::new(SLIDER_DRAG_PX, 0.0))(world);
        })
        .until(pointer_at_node(SLIDER, Vec2::new(SLIDER_DRAG_PX, 0.0)))
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: release the slider")
        .on_enter(release_mouse(MouseButton::Left))
        .until(pointer_released())
        .deadline(BEAT_DEADLINE_SECS)
        .add()
        .step("zoo: the slider moved")
        .on_enter(|world: &mut World| {
            let after_press = world.resource::<ZooProbe>().slider_after_press;
            let now = world.resource::<ZooSliderValue>().0;
            assert!(
                now > after_press + 0.02,
                "dragging {SLIDER} right must raise its value (snapped to \
                 {after_press}, still {now} after the drag)"
            );
            let slider = named(world, SLIDER);
            let live = world
                .get::<SliderValue>(slider)
                .expect("the slider carries its value")
                .0;
            assert!(
                (live - now).abs() < f32::EPSILON,
                "the widget's SliderValue ({live}) and the resource the zoo \
                 rebuilds from ({now}) must agree"
            );
            info!("zoo: slider dragged {after_press} -> {now}");
        })
        .add()
}

/// How far right the drag leg runs, in logical px. Comfortably inside the
/// 320px-wide panel body the track sits in, so the pointer never leaves the
/// track and the drag stays a drag.
#[cfg(feature = "debug")]
const SLIDER_DRAG_PX: f32 = 60.0;

/// The one entity carrying `name`, or a panic naming what was missing. A beat
/// that cannot find its target has already failed; warning and continuing would
/// only move the failure somewhere less legible.
#[cfg(feature = "debug")]
fn named(world: &mut World, name: &str) -> Entity {
    let mut q = world.query::<(Entity, &Name)>();
    let found: Vec<Entity> = q
        .iter(world)
        .filter(|(_, n)| n.as_str() == name)
        .map(|(entity, _)| entity)
        .collect();
    match found.as_slice() {
        [entity] => *entity,
        [] => panic!("no entity named `{name}`"),
        many => panic!(
            "{} entities named `{name}`; names must be unique",
            many.len()
        ),
    }
}

/// The alpha of a named widget's `BackgroundColor` - the observable half of
/// nova_ui's paint, and what a hover/press repaint moves.
#[cfg(feature = "debug")]
fn background_alpha(world: &mut World, name: &str) -> f32 {
    let entity = named(world, name);
    world
        .get::<BackgroundColor>(entity)
        .expect("a themed button carries a BackgroundColor")
        .0
        .alpha()
}

/// Check the LIVE TREE after a rebuild: exactly one body, exactly one entity
/// per driven name, and no `TextShadow` anywhere under the root.
///
/// This is the check `cargo check` cannot make. `rebuild_body` despawns and
/// respawns the body on every check change, so a reconciler that spawns
/// before it despawns leaves a ghost - visible only as a duplicate in the live
/// tree. `TextShadow` is refused by nova_ui on purpose (it is a hard drop
/// shadow, not the phosphor glow); a stray one only shows up here.
#[cfg(feature = "debug")]
fn assert_live_tree(world: &mut World, when: &str) {
    let bodies = world
        .query_filtered::<Entity, With<ZooBody>>()
        .iter(world)
        .count();
    assert_eq!(
        bodies, 1,
        "exactly one ZooBody must survive a rebuild ({when}); {bodies} found"
    );

    const DRIVEN: &[&str] = &[
        IDLE_BUTTON,
        THEME_PHOSPHOR,
        THEME_HARDWARE,
        LEVEL_ALL,
        LEVEL_MINIMAL,
        LEVEL_NONE,
        CHECK_FIRST,
        CHECK_SECOND,
        TOGGLE_FIRST,
        TOGGLE_SECOND,
        SLIDER,
    ];
    for name in DRIVEN {
        // Panics on 0 or 2+ - the ghost and the vanished widget both.
        let _ = named(world, name);
    }

    let root = world
        .query_filtered::<Entity, With<ZooRoot>>()
        .single(world)
        .expect("the zoo root is up");
    let mut stack = vec![root];
    let mut shadowed = Vec::new();
    while let Some(entity) = stack.pop() {
        if world.get::<TextShadow>(entity).is_some() {
            shadowed.push(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    assert!(
        shadowed.is_empty(),
        "nova_ui refuses TextShadow, but {} node(s) under the zoo root carry \
         one ({when}): {shadowed:?}",
        shadowed.len()
    );
}
