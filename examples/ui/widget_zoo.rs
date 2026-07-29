//! widget_zoo: a live, FUNCTIONAL showcase of the nova_ui widget library (the
//! `nova_ui::widget` factories) in both skins - the same constructors the game
//! spawns (`button(...)`, `segmented(...)`, `slider_track(...)`, `checkbox(...)`,
//! `toggle(...)`, `badge(...)`, `panel(...)`, `list_row(...)`). Everything here
//! is interactive: the buttons hover/press, the Skin control reskins the whole
//! zoo live, the segmented selects, the checkboxes/toggles flip, and the slider
//! drags (its phosphor block-meter tracks the value).
//!
//! It doubles as the render eyeball for tasks 20260728-175734/-175738: phosphor
//! reads as flat CLI elements, hardware as light-3D moulded controls.
//!
//! Interactive run:  `cargo run --example widget_zoo`  (drag the slider, click
//! the Skin control / checks / toggles; `S` also flips the skin).
//! Capture both skins: `NOVA_ZOO_CAPTURE=1 NOVA_SHOT_DIR=target/zoo cargo run
//! --example widget_zoo` -> widget_zoo-{phosphor,hardware}.png then exit.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
    ui_widgets::{
        observe, slider_self_update, Activate, Slider, SliderRange, SliderStep, SliderValue,
        TrackClick, ValueChange,
    },
};
use nova_ui::{
    prelude::*,
    widget::{register, ButtonSpec, SliderBlock, UiText},
};

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1280, 860).into(),
            title: "nova_ui widget library".into(),
            ..default()
        }),
        ..default()
    }));
    // The shared widget observers + skin reconciler + font router (inits UiSkin).
    register(&mut app);
    // The Skin control + the demo HUD-level control drive their resources through
    // the same `button_on_setting` path the game's Settings use.
    app.add_observer(button_on_setting::<UiSkin>);
    app.add_observer(button_on_setting::<DemoLevel>);
    app.add_observer(slider_self_update);
    app.init_resource::<DemoLevel>();
    app.init_resource::<ZooChecks>();
    app.insert_resource(ZooSliderValue(0.66));
    app.init_resource::<Capture>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            rebuild_body.run_if(resource_changed::<UiSkin>.or_else(resource_changed::<ZooChecks>)),
            sync_slider_meter,
            toggle_skin_key,
            drive_capture,
        ),
    );
    app.run()
}

// ------------------------------- state --------------------------------------

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
/// (skin/checks change) restores the slider where the player left it.
#[derive(Resource)]
struct ZooSliderValue(f32);

#[derive(Component)]
struct ZooRoot;
#[derive(Component)]
struct ZooBody;
/// A clickable check/toggle carrying its index into [`ZooChecks`].
#[derive(Component)]
struct CheckId(usize);

// ------------------------------- setup --------------------------------------

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
            BackgroundColor(theme::SPACE),
            // Soft nebula glows behind the panels (demo scene backdrop).
            BackgroundGradient(vec![
                Gradient::from(RadialGradient::new(
                    UiPosition::TOP_LEFT,
                    RadialGradientShape::FarthestSide,
                    vec![
                        ColorStop::percent(theme::BLUE.with_alpha(0.10), 0.0),
                        ColorStop::percent(Color::NONE, 40.0),
                    ],
                )),
                Gradient::from(RadialGradient::new(
                    UiPosition::BOTTOM_RIGHT,
                    RadialGradientShape::FarthestSide,
                    vec![
                        ColorStop::percent(theme::PHOSPHOR.with_alpha(0.06), 0.0),
                        ColorStop::percent(Color::NONE, 42.0),
                    ],
                )),
            ]),
        ))
        .with_children(|root| {
            top_bar(root);
        });
    // The body is spawned by `rebuild_body` on the first frame (UiSkin is
    // "changed" at startup), so there is one code path for spawn + reskin.
}

/// The persistent header: title + the live Skin control (Phosphor | Hardware),
/// a functional `ButtonValue<UiSkin>` segmented row.
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
            TextColor(theme::PHOSPHOR),
        ));
        bar.spawn((
            UiText,
            Text::new("Skin"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR_MUTED),
            Node {
                margin: UiRect::left(px(12)),
                ..default()
            },
        ));
        // A functional segmented: each option carries `ButtonValue<UiSkin>`, so a
        // click drives the shared `UiSkin` resource (reskinning the whole zoo).
        bar.spawn(segmented_group()).with_children(|seg| {
            for (label, value) in [
                ("Phosphor", UiSkin::Phosphor),
                ("Hardware", UiSkin::Hardware),
            ] {
                let mut b = seg.spawn((seg_option(label), ButtonValue(value)));
                if value == UiSkin::Phosphor {
                    b.insert(Selected);
                }
            }
        });
    });
}

// ------------------------------- body ---------------------------------------

/// (Re)build the panel grid for the current skin + interactive state. One path
/// serves the first spawn and every reskin/flip.
fn rebuild_body(
    mut commands: Commands,
    skin: Res<UiSkin>,
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
    let skin = *skin;
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
            buttons_panel(body, skin);
            controls_panel(body, skin, checks, value);
            content_panel(body, skin, checks);
        });
    });
}

/// A titled panel with a padded body.
fn panel_cell(
    body: &mut ChildSpawnerCommands,
    skin: UiSkin,
    title: &str,
    tag: Option<&str>,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    body.spawn(panel(skin)).with_children(|cell| {
        cell.spawn((panel_head(title, tag, skin),));
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

fn buttons_panel(body: &mut ChildSpawnerCommands, skin: UiSkin) {
    panel_cell(body, skin, "Buttons", None, |c| {
        sub_header(c, "States");
        c.spawn(flow_row()).with_children(|r| {
            r.spawn(button(ButtonSpec::new("Idle")));
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

fn controls_panel(body: &mut ChildSpawnerCommands, skin: UiSkin, checks: ZooChecks, value: f32) {
    panel_cell(body, skin, "Controls", None, |c| {
        sub_header(c, "Segmented (HUD detail)");
        // Functional: `ButtonValue<DemoLevel>` drives the DemoLevel resource.
        c.spawn(segmented_group()).with_children(|seg| {
            for (label, level) in [
                ("All", DemoLevel::All),
                ("Minimal", DemoLevel::Minimal),
                ("None", DemoLevel::None),
            ] {
                let mut b = seg.spawn((seg_option(label), ButtonValue(level)));
                if level == DemoLevel::All {
                    b.insert(Selected);
                }
            }
        });
        sub_header(c, "Slider (drag me)");
        // A real draggable `bevy_ui_widgets::Slider` wearing the phosphor
        // block-meter; `on_slider_change` lights the bars to the value.
        c.spawn((
            slider_track(value, skin),
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
            r.spawn(clickable(checkbox(checks.0[0], skin), 0));
            r.spawn(clickable(checkbox(checks.0[1], skin), 1));
            r.spawn(clickable(toggle(checks.0[2], skin), 2));
            r.spawn(clickable(toggle(checks.0[3], skin), 3));
        });
    });
}

fn content_panel(body: &mut ChildSpawnerCommands, skin: UiSkin, checks: ZooChecks) {
    panel_cell(body, skin, "Content", Some("DELTA-9"), |c| {
        sub_header(c, "List rows");
        list_row_entry(c, skin, checks.0[0], "Deep salvage", "v1.2 // nova.labs", 0);
        list_row_entry(c, skin, checks.0[1], "Hard vacuum", "v0.4 // driftco", 1);
        sub_header(c, "Badges");
        c.spawn(flow_row()).with_children(|r| {
            r.spawn(badge(BadgeKind::Green, "online", skin));
            r.spawn(badge(BadgeKind::Amber, "warn", skin));
            r.spawn(badge(BadgeKind::Blue, "info", skin));
            r.spawn(badge(BadgeKind::Red, "fault", skin));
            r.spawn(badge(BadgeKind::Mute, "idle", skin));
        });
    });
}

/// A list row with a title/subtitle and a trailing clickable checkbox that
/// shares its enabled bit with the matching Controls checkbox.
fn list_row_entry(
    c: &mut ChildSpawnerCommands,
    skin: UiSkin,
    on: bool,
    title: &str,
    sub: &str,
    id: usize,
) {
    let title = title.to_string();
    let sub = sub.to_string();
    c.spawn(list_row(on, skin)).with_children(|row| {
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
                TextColor(theme::SCREEN_TEXT),
            ));
            col.spawn((
                UiText,
                Text::new(sub),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_DIM),
            ));
        });
        row.spawn(clickable(checkbox(on, skin), id));
    });
}

// ------------------------------- interactivity ------------------------------

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

/// Commit a slider drag: store the value + light the block-meter to it. Does NOT
/// touch a rebuild-triggering resource, so dragging is smooth (no respawn).
fn on_slider_change(
    change: On<ValueChange<f32>>,
    mut value: ResMut<ZooSliderValue>,
    children: Query<&Children>,
    mut blocks: Query<(&SliderBlock, &mut BackgroundColor)>,
) {
    value.0 = change.value.clamp(0.0, 1.0);
    if let Ok(kids) = children.get(change.source) {
        recolor_blocks(kids, value.0, &mut blocks);
    }
}

/// Keep the block-meter in sync if `SliderValue` changes from elsewhere (e.g. a
/// track-click, which snaps without a `ValueChange`).
fn sync_slider_meter(
    changed: Query<(&Children, &SliderValue), Changed<SliderValue>>,
    mut blocks: Query<(&SliderBlock, &mut BackgroundColor)>,
) {
    for (kids, value) in &changed {
        recolor_blocks(kids, value.0, &mut blocks);
    }
}

fn recolor_blocks(
    kids: &Children,
    value: f32,
    blocks: &mut Query<(&SliderBlock, &mut BackgroundColor)>,
) {
    for &child in kids {
        if let Ok((block, mut bg)) = blocks.get_mut(child) {
            *bg = slider_meter_color(block.0, value).into();
        }
    }
}

/// `S` flips the skin (the segmented control does the same via ButtonValue).
fn toggle_skin_key(keys: Res<ButtonInput<KeyCode>>, mut skin: ResMut<UiSkin>) {
    if keys.just_pressed(KeyCode::KeyS) {
        *skin = match *skin {
            UiSkin::Phosphor => UiSkin::Hardware,
            UiSkin::Hardware => UiSkin::Phosphor,
        };
    }
}

// ------------------------------- small helpers ------------------------------

/// A section sub-header inside a panel body (demo `h3.sub`).
fn sub_header(c: &mut ChildSpawnerCommands, text: &str) {
    c.spawn((
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(theme::PHOSPHOR_MUTED),
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
        TextColor(theme::PHOSPHOR_DIM),
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

/// The bordered/recessed container of a segmented control (the options are
/// spawned as `seg_option` children carrying a `ButtonValue<T>`).
fn segmented_group() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(3),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            align_self: AlignSelf::Start,
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR.with_alpha(0.25)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
    )
}

/// One segmented option: a small ghost `ThemedButton` (the caller adds the
/// `ButtonValue<T>` + `Selected` for the active one).
fn seg_option(label: &str) -> impl Bundle {
    let mut spec = ButtonSpec::new(label);
    spec.variant = ButtonVariant::Ghost;
    spec.min_height = 28.0;
    spec.font_size = 12.0;
    button(spec)
}

// ------------------------------- capture ------------------------------------

#[derive(Resource, Default)]
struct Capture {
    stage: u32,
    wait: u32,
}

fn drive_capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut skin: ResMut<UiSkin>,
    mut exit: MessageWriter<AppExit>,
) {
    if std::env::var_os("NOVA_ZOO_CAPTURE").is_none() {
        return;
    }
    if cap.wait > 0 {
        cap.wait -= 1;
        return;
    }
    match cap.stage {
        0 => {
            cap.stage = 1;
            cap.wait = 60;
        }
        1 => {
            shoot("widget_zoo-phosphor.png", &mut commands);
            cap.stage = 2;
            cap.wait = 20;
        }
        2 => {
            *skin = UiSkin::Hardware;
            cap.stage = 3;
            cap.wait = 60;
        }
        3 => {
            shoot("widget_zoo-hardware.png", &mut commands);
            cap.stage = 4;
            cap.wait = 20;
        }
        _ => {
            exit.write(AppExit::Success);
        }
    }
}

fn shoot(name: &str, commands: &mut Commands) {
    let path = match std::env::var("NOVA_SHOT_DIR") {
        Ok(dir) if !dir.is_empty() => std::path::Path::new(&dir).join(name),
        _ => std::path::PathBuf::from(name),
    };
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    info!("widget_zoo capture: {name}");
}
