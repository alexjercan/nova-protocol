//! The diagnostics layer every hand-played example shares: a hint line, world
//! gizmos and a text panel of live values, so a person can check a feature by
//! eye while playing it.
//!
//! Include it with `#[path = "shared/diagnostics.rs"] mod diagnostics;`, add
//! [`PlayableDiagnosticsPlugin`], and draw from systems in
//! [`DiagnosticsSystems`]:
//!
//! ```ignore
//! app.add_plugins(diagnostics::PlayableDiagnosticsPlugin::new(
//!     "fly with WASD, dock with an escort",
//! ));
//! app.add_systems(PostUpdate, draw_the_fleet.in_set(diagnostics::DiagnosticsSystems));
//!
//! fn draw_the_fleet(mut gizmos: Gizmos<DiagnosticsGizmos>, mut panel: ResMut<DiagnosticsPanel>) {
//!     gizmos.arrow(from, to, diagnostics::palette::VELOCITY);
//!     panel.line(format!("escort 1: {:.0} m to slot", 42.0));
//!     panel.label(at, "slot 1");
//! }
//! ```
//!
//! The layer is ON at start and [`DIAGNOSTICS_TOGGLE_KEY`] flips it. The hint
//! line stays up while the layer is off, and names the key. Panel lines and
//! world labels are written fresh each frame: the plugin clears them in
//! `First`, so a value nobody writes this frame is gone, not stale.
//!
//! This is not `nova_debug`'s F11 layer. That one starts OFF, raises the egui
//! inspector (which frees the cursor) and draws every section's barrels and
//! drives. This layer draws only what one example names, needs no `debug`
//! feature, and leaves the cursor and the flight input alone.

#![allow(
    dead_code,
    reason = "each example uses part of the shared layer, and an include compiles it per example"
)]

use bevy::{camera::visibility::RenderLayers, prelude::*};
use nova_protocol::prelude::*;

/// The one key that shows and hides the diagnostics in every playable
/// example. No default game, editor, debug or probe binding uses F3.
pub const DIAGNOSTICS_TOGGLE_KEY: KeyCode = KeyCode::F3;

/// The key's name in the hint line.
const DIAGNOSTICS_TOGGLE_LABEL: &str = "F3";

/// Line width (px) of the diagnostics gizmos: over Bevy's 2 px default, so a
/// vector reads against a lit hull at 1080p.
const DIAGNOSTICS_LINE_WIDTH: f32 = 3.0;

/// Panel and label text size (px).
const PANEL_FONT_PX: f32 = 15.0;

/// World labels a frame can place. A label past this is dropped: the panel
/// carries the numbers, labels only name the thing drawn.
const MAX_LABELS: usize = 32;

/// Colours the examples share, so one meaning keeps one colour everywhere.
pub mod palette {
    use bevy::prelude::*;

    /// A velocity vector.
    pub const VELOCITY: Color = Color::srgb(0.3, 0.9, 1.0);
    /// A target, slot or goal point.
    pub const TARGET: Color = Color::srgb(1.0, 0.85, 0.2);
    /// A zone or radius the feature is defined by.
    pub const ZONE: Color = Color::srgb(0.6, 0.5, 1.0);
    /// A state that holds or passes.
    pub const OK: Color = Color::srgb(0.35, 1.0, 0.45);
    /// A state that fails, refuses or is over a limit.
    pub const ALERT: Color = Color::srgb(1.0, 0.35, 0.3);
    /// A force or push, such as a drive or an RCS puff.
    pub const PUSH: Color = Color::srgb(1.0, 0.55, 0.15);
    /// A neutral reference line.
    pub const MUTED: Color = Color::srgb(0.6, 0.65, 0.7);
}

/// The gizmo group every diagnostics line draws in. It draws over hulls and
/// hides with the layer.
#[derive(Clone, Default, Reflect, GizmoConfigGroup)]
pub struct DiagnosticsGizmos;

/// Put the diagnostics gizmos on `layers`, for a view that renders on its
/// own render layers, such as the Ship pane's schematic camera. `None` puts
/// them back on the world's default layer. The gizmos show on one set of
/// layers at a time, so an example draws in one such view at a time.
pub fn draw_on_layers(store: &mut GizmoConfigStore, layers: Option<&RenderLayers>) {
    let (config, _) = store.config_mut::<DiagnosticsGizmos>();
    let layers = layers.cloned().unwrap_or_default();
    if config.render_layers != layers {
        config.render_layers = layers;
    }
}

/// Whether the diagnostics show. Starts `true`.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticsShown(pub bool);

/// The systems that draw diagnostics, in `Update` and `PostUpdate`. They run
/// only while [`DiagnosticsShown`] is `true`. Draw world-space gizmos in
/// `PostUpdate` to read the propagated transforms.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticsSystems;

/// This frame's panel lines and world labels.
#[derive(Resource, Default, Debug)]
pub struct DiagnosticsPanel {
    lines: Vec<String>,
    labels: Vec<(Vec3, String)>,
}

impl DiagnosticsPanel {
    /// Add one line to the panel.
    pub fn line(&mut self, text: impl Into<String>) {
        self.lines.push(text.into());
    }

    /// Put `text` on screen over the world point `at`, engine units.
    pub fn label(&mut self, at: Vec3, text: impl Into<String>) {
        if self.labels.len() < MAX_LABELS {
            self.labels.push((at, text.into()));
        }
    }
}

/// The hint line, the panel and the toggle.
pub struct PlayableDiagnosticsPlugin {
    hint: String,
}

impl PlayableDiagnosticsPlugin {
    /// The layer with `hint`, one line of the keys to try. The plugin adds
    /// the toggle key to it.
    pub fn new(hint: impl Into<String>) -> Self {
        Self { hint: hint.into() }
    }
}

/// The hint text node.
#[derive(Component)]
struct HintText;

/// The panel text node.
#[derive(Component)]
struct PanelText;

/// One pooled world label node.
#[derive(Component)]
struct WorldLabel(usize);

/// The hint, kept for the root's spawn.
#[derive(Resource)]
struct Hint(String);

impl Plugin for PlayableDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DiagnosticsShown(true));
        app.insert_resource(Hint(format!(
            "{}   [{DIAGNOSTICS_TOGGLE_LABEL}] diagnostics",
            self.hint
        )));
        app.init_resource::<DiagnosticsPanel>();
        app.insert_gizmo_config(
            DiagnosticsGizmos,
            GizmoConfig {
                line: GizmoLineConfig {
                    width: DIAGNOSTICS_LINE_WIDTH,
                    ..default()
                },
                // In front of every hull: a slot or a radius behind the ship
                // must still read.
                depth_bias: -1.0,
                ..default()
            },
        );
        app.configure_sets(
            Update,
            DiagnosticsSystems.run_if(resource_equals(DiagnosticsShown(true))),
        );
        app.configure_sets(
            PostUpdate,
            DiagnosticsSystems.run_if(resource_equals(DiagnosticsShown(true))),
        );
        app.add_systems(Startup, spawn_the_overlay);
        app.add_systems(First, clear_the_panel);
        app.add_systems(Update, toggle_the_diagnostics);
        app.add_systems(
            PostUpdate,
            (show_the_panel, place_the_labels)
                .after(DiagnosticsSystems)
                .after(TransformSystems::Propagate),
        );
        info!("playable: {}", app.world().resource::<Hint>().0.as_str());
    }
}

fn clear_the_panel(mut panel: ResMut<DiagnosticsPanel>) {
    panel.lines.clear();
    panel.labels.clear();
}

fn toggle_the_diagnostics(
    keys: Res<ButtonInput<KeyCode>>,
    mut shown: ResMut<DiagnosticsShown>,
    mut config: ResMut<GizmoConfigStore>,
) {
    if keys.just_pressed(DIAGNOSTICS_TOGGLE_KEY) {
        shown.0 = !shown.0;
        config.config_mut::<DiagnosticsGizmos>().0.enabled = shown.0;
        info!(
            "playable: diagnostics {}",
            if shown.0 { "on" } else { "off" }
        );
    }
}

/// One scrimmed text node.
fn text_node(text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(PANEL_FONT_PX),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.95, 1.0)),
        Node {
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
    )
}

/// The hint at the top centre, the panel under it at the left, and the label
/// pool. Over the HUD and the interface, and never in the pointer's way.
fn spawn_the_overlay(mut commands: Commands, hint: Res<Hint>) {
    commands
        .spawn((
            Name::new("PlayableDiagnostics"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(44.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(4.0),
                ..default()
            },
            GlobalZIndex(i32::MAX - 2),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((HintText, Pickable::IGNORE, text_node(&hint.0)));
            parent.spawn((PanelText, Pickable::IGNORE, text_node("")));
        });
    for index in 0..MAX_LABELS {
        commands.spawn((
            WorldLabel(index),
            Pickable::IGNORE,
            GlobalZIndex(i32::MAX - 3),
            Visibility::Hidden,
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(PANEL_FONT_PX - 2.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.7)),
                Node {
                    position_type: PositionType::Absolute,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ),
        ));
    }
}

/// Write the panel's lines, or hide it while the layer is off or empty.
fn show_the_panel(
    shown: Res<DiagnosticsShown>,
    panel: Res<DiagnosticsPanel>,
    mut q_panel: Query<(&mut Text, &mut Node), With<PanelText>>,
) {
    let Ok((mut text, mut node)) = q_panel.single_mut() else {
        return;
    };
    let display = if shown.0 && !panel.lines.is_empty() {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    let joined = panel.lines.join("\n");
    if text.0 != joined {
        text.0 = joined;
    }
}

/// The camera the player sees the world through: the scenario's camera, or
/// else the first active 3D camera.
fn view_camera<'a>(
    q_cameras: &'a Query<(&Camera, &GlobalTransform, Has<ScenarioCameraMarker>), With<Camera3d>>,
) -> Option<(&'a Camera, &'a GlobalTransform)> {
    q_cameras
        .iter()
        .filter(|(camera, ..)| camera.is_active)
        .max_by_key(|(_, _, scenario)| *scenario)
        .map(|(camera, transform, _)| (camera, transform))
}

/// Project each label over its world point; hide the unused and the
/// off-screen ones.
fn place_the_labels(
    shown: Res<DiagnosticsShown>,
    panel: Res<DiagnosticsPanel>,
    q_cameras: Query<(&Camera, &GlobalTransform, Has<ScenarioCameraMarker>), With<Camera3d>>,
    mut q_labels: Query<(&WorldLabel, &mut Text, &mut Node, &mut Visibility)>,
) {
    let camera = view_camera(&q_cameras);
    for (label, mut text, mut node, mut visibility) in &mut q_labels {
        let placed = shown
            .0
            .then(|| panel.labels.get(label.0))
            .flatten()
            .zip(camera)
            .and_then(|((at, said), (camera, transform))| {
                camera
                    .world_to_viewport(transform, *at)
                    .ok()
                    .map(|pixel| (pixel, said))
            });
        let Some((pixel, said)) = placed else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if text.0 != *said {
            text.0.clone_from(said);
        }
        node.left = Val::Px(pixel.x + 6.0);
        node.top = Val::Px(pixel.y - 8.0);
        *visibility = Visibility::Inherited;
    }
}

/// A wire circle of `radius` about `centre`, square to `normal`.
pub fn ring(
    gizmos: &mut Gizmos<DiagnosticsGizmos>,
    centre: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
) {
    let rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Y));
    gizmos
        .circle(Isometry3d::new(centre, rotation), radius, color)
        .resolution(64);
}

/// A wire sphere of `radius` about `centre`, drawn as three great circles.
pub fn sphere(gizmos: &mut Gizmos<DiagnosticsGizmos>, centre: Vec3, radius: f32, color: Color) {
    for normal in [Vec3::X, Vec3::Y, Vec3::Z] {
        ring(gizmos, centre, normal, radius, color);
    }
}

/// An arrow from `from` along `vector` scaled by `scale`, skipped when it is
/// shorter than a hair.
pub fn vector(
    gizmos: &mut Gizmos<DiagnosticsGizmos>,
    from: Vec3,
    vector: Vec3,
    scale: f32,
    color: Color,
) {
    let to = from + vector * scale;
    if from.distance_squared(to) > 1.0e-6 {
        gizmos.arrow(from, to, color);
    }
}
