//! widget_zoo: render the full nova_ui skin-aware widget set (task
//! 20260728-175734) in BOTH skins, for the render eyeball of demo 1
//! (`examples/ui/nova_ui_rework_poc.html`, the `Widget zoo` screen).
//!
//! The scaffold mirrors `examples/ui/nova_os_rtt_poc.rs` (reuse-known-good-stack):
//! a bespoke `DefaultPlugins` app + a `Camera2d`, with the nova_ui widget
//! observers/reconciler registered and the Iosevka Term font published as
//! `UiFont` so the typography routing is exercised end to end.
//!
//! Interactive run (eyeball both skins - press `S` to flip Phosphor/Hardware):
//! ```text
//! cargo run --example widget_zoo
//! ```
//!
//! Capture both skins to PNGs (real GPU) - phosphor then hardware, then exit:
//! ```text
//! NOVA_ZOO_CAPTURE=1 NOVA_SHOT_DIR=target/zoo cargo run --example widget_zoo
//! # writes widget_zoo-phosphor.png + widget_zoo-hardware.png, then exits
//! ```

use bevy::{
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
use nova_ui::{
    prelude::*,
    widget::{register, ButtonSpec},
};

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution: (1280, 860).into(),
            title: "nova_ui widget zoo".into(),
            ..default()
        }),
        ..default()
    }));
    // The shared widget observers + skin reconciler + font router (also inits
    // the UiSkin resource, defaulting to Phosphor).
    register(&mut app);
    app.init_resource::<Capture>();
    app.add_systems(Startup, setup);
    app.add_systems(Update, (toggle_skin_key, drive_capture));
    app.run()
}

/// Root of the currently-drawn zoo, so a skin flip can despawn + respawn the
/// non-button widgets (only buttons live-restyle; the rest read the skin at
/// spawn).
#[derive(Component)]
struct ZooRoot;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, skin: Res<UiSkin>) {
    commands.spawn(Camera2d);
    // Publish the shared UI font so `apply_ui_font` routes every UiText span.
    commands.insert_resource(UiFont(
        asset_server.load("fonts/SGr-IosevkaTerm-Medium.ttf"),
    ));
    spawn_zoo(&mut commands, *skin);
}

/// Frame-driven capture: settle, shoot phosphor, flip to hardware + respawn,
/// settle, shoot hardware, exit. Inert unless `NOVA_ZOO_CAPTURE` is set.
#[derive(Resource, Default)]
struct Capture {
    stage: u32,
    wait: u32,
}

fn drive_capture(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut skin: ResMut<UiSkin>,
    roots: Query<Entity, With<ZooRoot>>,
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
            for root in &roots {
                commands.entity(root).despawn();
            }
            *skin = UiSkin::Hardware;
            spawn_zoo(&mut commands, UiSkin::Hardware);
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

/// `S` flips the skin in an interactive run: despawn the zoo, flip the resource
/// (buttons would restyle live, but the static widgets read the skin at spawn),
/// respawn.
fn toggle_skin_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut skin: ResMut<UiSkin>,
    roots: Query<Entity, With<ZooRoot>>,
) {
    if !keys.just_pressed(KeyCode::KeyS) {
        return;
    }
    *skin = match *skin {
        UiSkin::Phosphor => UiSkin::Hardware,
        UiSkin::Hardware => UiSkin::Phosphor,
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_zoo(&mut commands, *skin);
}

// ------------------------------- the zoo ------------------------------------

fn spawn_zoo(commands: &mut Commands, skin: UiSkin) {
    commands
        .spawn((
            ZooRoot,
            Node {
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(20)),
                column_gap: px(16),
                row_gap: px(16),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::Start,
                ..default()
            },
            BackgroundColor(theme::SPACE),
            // A pair of soft nebula glows behind the panels, mirroring the demo
            // scene backdrop (blue upper-left, phosphor lower-right).
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
            button_states_cell(root, skin);
            controls_cell(root, skin);
            rows_cell(root, skin);
            header_badges_cell(root, skin);
        });
}

/// A titled zoo cell: a `panel(skin)` with a padded body.
fn cell(
    root: &mut ChildSpawnerCommands,
    skin: UiSkin,
    title: &str,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    // `panel(skin)` already carries its own `Node` (border + radius live on it),
    // so DON'T tuple a second `Node` here - that is a duplicate-component panic.
    // Size the cell via its 300px body; the panel column shrinks to fit it.
    root.spawn(panel(skin)).with_children(|cell| {
        cell.spawn((panel_head(title, None, skin),));
        cell.spawn(Node {
            width: px(300),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(15)),
            ..default()
        })
        .with_children(body);
    });
}

fn button_states_cell(root: &mut ChildSpawnerCommands, skin: UiSkin) {
    cell(root, skin, "Button states", |body| {
        // Idle / pressed / selected / disabled - forced via the state markers.
        body.spawn(row_wrap()).with_children(|r| {
            r.spawn(button(ButtonSpec::new("Idle")));
            r.spawn((button(ButtonSpec::new("Pressed")), bevy::ui::Pressed));
            r.spawn((button(ButtonSpec::new("Selected")), Selected));
            r.spawn((
                button(ButtonSpec::new("Disabled")),
                bevy::ui::InteractionDisabled,
            ));
        });
        // Emphasis: primary / default / ghost / danger.
        body.spawn(row_wrap()).with_children(|r| {
            r.spawn(button(ButtonSpec::new("Primary").primary()));
            r.spawn(button(ButtonSpec::new("Default")));
            r.spawn(button(ButtonSpec::new("Ghost").ghost()));
            r.spawn(button(ButtonSpec::new("Danger").danger()));
        });
        // Block button with a key-chip (the `Play` / `Enter` shape).
        body.spawn(button(
            ButtonSpec::new("Play").primary().block().key("Enter"),
        ));
    });
}

fn controls_cell(root: &mut ChildSpawnerCommands, skin: UiSkin) {
    cell(root, skin, "Segmented + slider", |body| {
        body.spawn(segmented(&["All", "Minimal", "None"], 0, skin));
        body.spawn(slider_track(0.66, skin));
        body.spawn(row_wrap()).with_children(|r| {
            r.spawn(checkbox(true, skin));
            r.spawn(checkbox(false, skin));
            r.spawn(toggle(true, skin));
            r.spawn(toggle(false, skin));
        });
    });
}

fn rows_cell(root: &mut ChildSpawnerCommands, skin: UiSkin) {
    cell(root, skin, "List rows", |body| {
        list_row_with_label(body, skin, true, "Selected row", "enabled");
        list_row_with_label(body, skin, false, "Idle row", "disabled");
    });
}

fn header_badges_cell(root: &mut ChildSpawnerCommands, skin: UiSkin) {
    cell(root, skin, "Badges", |body| {
        body.spawn((panel_head("Header", Some("TAG"), skin),));
        body.spawn(row_wrap()).with_children(|r| {
            r.spawn(badge(BadgeKind::Green, "online", skin));
            r.spawn(badge(BadgeKind::Amber, "warn", skin));
            r.spawn(badge(BadgeKind::Blue, "info", skin));
            r.spawn(badge(BadgeKind::Red, "fault", skin));
            r.spawn(badge(BadgeKind::Mute, "idle", skin));
        });
    });
}

fn list_row_with_label(
    body: &mut ChildSpawnerCommands,
    skin: UiSkin,
    selected: bool,
    title: &str,
    sub: &str,
) {
    let title = title.to_string();
    let sub = sub.to_string();
    body.spawn(list_row(selected, skin)).with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                nova_ui::widget::UiText,
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::SCREEN_TEXT),
            ));
            col.spawn((
                nova_ui::widget::UiText,
                Text::new(sub),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED.with_alpha(0.9)),
            ));
        });
        row.spawn(checkbox(selected, skin));
    });
}

fn row_wrap() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: px(8),
        row_gap: px(8),
        align_items: AlignItems::Center,
        ..default()
    }
}
