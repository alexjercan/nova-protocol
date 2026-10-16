//! The F3 diagnostics layer: world gizmos and labels, a short panel, and key
//! hints that show while they apply, so a person can check a feature by eye
//! while playing it.
//!
//! [`DiagnosticsPlugin`] owns the layer and `DebugPlugin` adds it, so every
//! debug build has it; adding it twice panics. It boots OFF. A hand-played
//! example adds [`DiagnosticsHintPlugin`], which turns the layer on at start
//! and shows its intro with the toggle keys for a few seconds. Draw from
//! systems in [`DiagnosticsSystems`]:
//!
//! ```ignore
//! use nova_debug::diagnostics;
//!
//! app.add_plugins(diagnostics::DiagnosticsHintPlugin::new("dock with an escort"));
//! app.add_systems(PostUpdate, draw_the_fleet.in_set(diagnostics::DiagnosticsSystems));
//!
//! fn draw_the_fleet(mut gizmos: Gizmos<DiagnosticsGizmos>, mut panel: ResMut<DiagnosticsPanel>) {
//!     gizmos.line(ship, slot, diagnostics::palette::TARGET);
//!     panel.label(slot.midpoint(ship), "42 m");
//!     if near_a_port {
//!         panel.hint("Ctrl", "lock the escort");
//!     }
//!     panel.line("setup: Ready");
//!     panel.detail(format!("escort 1: closing {:+.1} m/s", 0.4));
//! }
//! ```
//!
//! Put a number on a world label next to the thing it measures; keep the
//! panel to a few short lines. [`DiagnosticsPanel::detail`] lines show only
//! while [`DebugOverlays::diagnostics_detail`] is on. A key hint shows in one
//! row above the HUD's key chips for each frame an example writes it, so
//! write it only while its action is possible.
//!
//! [`DIAGNOSTICS_TOGGLE_KEY`] and the Nova Debug window's Diagnostics checkbox
//! flip [`DebugOverlays::diagnostics`]; Shift with the key, or the Diagnostics
//! detail checkbox, flips [`DebugOverlays::diagnostics_detail`]. Panel lines,
//! labels and hints are written fresh each frame: the plugin clears them in
//! `First`, so a value nobody writes this frame is gone, not stale.
//!
//! In flight the panel sits at the top left. While a surface is up (the TAB
//! interface, the command modal, the pause menu or the Nova Debug window) it
//! moves to the bottom left, clear of the pane's title row and filter
//! buttons, the hint row drops to the bottom edge, and the world labels hide:
//! the flight camera's points do not show through a pane. Gizmos stay: the
//! world's draw under the pane's card, and an example that draws in the Ship
//! view puts them there with [`draw_on_layers`].
//!
//! This layer draws only what one example names, and leaves the cursor and
//! the flight input alone.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use nova_gameplay::{GameStates, PauseStates};
use nova_scenario::prelude::ScenarioCameraMarker;
use nova_ui::prelude::{in_input_mode, InputMode};

use crate::hud::overlays::{apply_debug_overlays, overlay_on, DebugOverlays};

/// The one key that shows and hides the diagnostics in every playable
/// example. No default game, editor, debug or probe binding uses F3. With
/// Shift it shows and hides the detail lines.
pub const DIAGNOSTICS_TOGGLE_KEY: KeyCode = KeyCode::F3;

/// The toggle keys as the intro names them.
const DIAGNOSTICS_TOGGLE_HINT: &str = "[F3] diagnostics   [Shift+F3] detail";

/// Line width (px) of the diagnostics gizmos: over Bevy's 2 px default, so a
/// vector reads against a lit hull at 1080p.
const DIAGNOSTICS_LINE_WIDTH: f32 = 3.0;

/// Panel, hint and label text size (px): small, so the overlay does not
/// cover the scene it explains.
const PANEL_FONT_PX: f32 = 12.0;

/// World labels a frame can place. A label past this is dropped.
const MAX_LABELS: usize = 48;

/// Seconds the intro stays fully up once play starts.
const INTRO_HOLD_SECS: f32 = 6.0;

/// Seconds the intro takes to fade out after the hold.
const INTRO_FADE_SECS: f32 = 1.5;

/// The text colour of the panel, the hints and the intro.
const TEXT_COLOUR: Color = Color::srgb(0.9, 0.95, 1.0);

/// The colour of a label written without one.
const LABEL_COLOUR: Color = Color::srgb(1.0, 0.95, 0.7);

/// The alpha of the scrim behind panel, hint and label text.
const SCRIM_ALPHA: f32 = 0.55;

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

/// The systems that draw diagnostics, in `Update` and `PostUpdate`. They run
/// only while [`DebugOverlays::diagnostics`] is on. Draw world-space gizmos in
/// `PostUpdate` to read the propagated transforms.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticsSystems;

/// This frame's panel lines, detail lines, key hints and world labels.
#[derive(Resource, Default, Debug)]
pub struct DiagnosticsPanel {
    lines: Vec<String>,
    details: Vec<String>,
    hints: Vec<(String, String)>,
    labels: Vec<(Vec3, String, Color)>,
}

impl DiagnosticsPanel {
    /// Add one short line to the panel. Keep these to a few: a number about
    /// one thing belongs on a [`label`](Self::label) next to it.
    pub fn line(&mut self, text: impl Into<String>) {
        self.lines.push(text.into());
    }

    /// Add one line to the full value list, shown under the panel lines only
    /// while [`DebugOverlays::diagnostics_detail`] is on.
    pub fn detail(&mut self, text: impl Into<String>) {
        self.details.push(text.into());
    }

    /// Show `[key] action` in the hint row this frame. Write it only while
    /// the action is possible; a key already hinted this frame is skipped.
    pub fn hint(&mut self, key: impl Into<String>, action: impl Into<String>) {
        let key = key.into();
        if self.hints.iter().all(|(hinted, _)| *hinted != key) {
            self.hints.push((key, action.into()));
        }
    }

    /// Put `text` on screen over the world point `at`, engine units.
    pub fn label(&mut self, at: Vec3, text: impl Into<String>) {
        self.label_tinted(at, text, LABEL_COLOUR);
    }

    /// [`label`](Self::label) in `color`, such as a
    /// [`palette`] state colour.
    pub fn label_tinted(&mut self, at: Vec3, text: impl Into<String>, color: Color) {
        if self.labels.len() < MAX_LABELS {
            self.labels.push((at, text.into(), color));
        }
    }

    /// The panel's text: the lines, then the detail lines when `detail`.
    fn text(&self, detail: bool) -> String {
        let mut text = self.lines.join("\n");
        if detail && !self.details.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&self.details.join("\n"));
        }
        text
    }

    /// The hint row's text, empty with no hint.
    fn hint_text(&self) -> String {
        self.hints
            .iter()
            .map(|(key, action)| format!("[{key}] {action}"))
            .collect::<Vec<_>>()
            .join("   ")
    }
}

/// The layer: the panel, the hint row, the labels, the gizmo group and the
/// toggles. Off at boot. `DebugPlugin` adds it.
pub struct DiagnosticsPlugin;

/// Turns the layer on at start and shows an intro: one short line about the
/// example, with the toggle keys, which fades a few seconds into play. For a
/// hand-played example; needs [`DiagnosticsPlugin`] (a debug build's
/// `DebugPlugin`).
pub struct DiagnosticsHintPlugin {
    hint: String,
}

impl DiagnosticsHintPlugin {
    /// The intro `hint`. The plugin adds the toggle keys; an empty hint shows
    /// the keys alone.
    pub fn new(hint: impl Into<String>) -> Self {
        Self { hint: hint.into() }
    }
}

/// The intro text node.
#[derive(Component)]
struct IntroText;

/// The panel text node.
#[derive(Component)]
struct PanelText;

/// The hint row's text node.
#[derive(Component)]
struct HintText;

/// The node that centres the hint row.
#[derive(Component)]
struct HintRow;

/// One pooled world label node.
#[derive(Component)]
struct WorldLabel(usize);

/// The node that holds the intro and the panel.
#[derive(Component)]
struct OverlayRoot;

/// Distance (px) of the panel from the screen's left edge, and from its
/// bottom edge while a surface is up.
const OVERLAY_INSET_PX: f32 = 12.0;

/// Distance (px) of the panel from the screen's top edge in flight: under
/// the frame-rate readout.
const OVERLAY_TOP_PX: f32 = 44.0;

/// Distance (px) of the hint row from the screen's bottom edge in flight:
/// above the HUD's key chips.
const HINT_ROW_BOTTOM_PX: f32 = 64.0;

/// The intro and how long play has shown it.
#[derive(Resource)]
struct Intro {
    text: String,
    shown_secs: f32,
}

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugOverlays>();
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
            DiagnosticsSystems.run_if(overlay_on(|overlays| overlays.diagnostics)),
        );
        app.configure_sets(
            PostUpdate,
            DiagnosticsSystems.run_if(overlay_on(|overlays| overlays.diagnostics)),
        );
        app.add_systems(Startup, spawn_the_overlay);
        app.add_systems(First, clear_the_panel);
        app.add_systems(
            Update,
            toggle_the_diagnostics
                .run_if(in_input_mode(InputMode::Normal))
                .before(apply_debug_overlays),
        );
        app.add_systems(
            PostUpdate,
            (
                show_the_panel,
                show_the_hints,
                fade_the_intro,
                place_the_labels,
                move_the_overlay,
            )
                .after(DiagnosticsSystems)
                .after(TransformSystems::Propagate),
        );
    }
}

impl Plugin for DiagnosticsHintPlugin {
    fn build(&self, app: &mut App) {
        let text = if self.hint.is_empty() {
            DIAGNOSTICS_TOGGLE_HINT.to_string()
        } else {
            format!("{}   {DIAGNOSTICS_TOGGLE_HINT}", self.hint)
        };
        info!("playable: {text}");
        app.insert_resource(Intro {
            text,
            shown_secs: 0.0,
        });
        app.init_resource::<DebugOverlays>();
        app.world_mut().resource_mut::<DebugOverlays>().diagnostics = true;
    }

    fn finish(&self, app: &mut App) {
        assert!(
            app.is_plugin_added::<DiagnosticsPlugin>(),
            "nova_debug: DiagnosticsHintPlugin needs DiagnosticsPlugin; build with \
             the `debug` feature so DebugPlugin adds it"
        );
    }
}

fn clear_the_panel(mut panel: ResMut<DiagnosticsPanel>) {
    panel.lines.clear();
    panel.details.clear();
    panel.hints.clear();
    panel.labels.clear();
}

/// F3 flips the layer and Shift+F3 the detail lines; the detail key also
/// turns the layer on. Either ends the intro: the player found the key. The
/// gizmo group follows in [`apply_debug_overlays`].
fn toggle_the_diagnostics(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlays: ResMut<DebugOverlays>,
    intro: Option<ResMut<Intro>>,
) {
    if !keys.just_pressed(DIAGNOSTICS_TOGGLE_KEY) {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        overlays.diagnostics_detail = !overlays.diagnostics_detail;
        overlays.diagnostics |= overlays.diagnostics_detail;
    } else {
        overlays.diagnostics = !overlays.diagnostics;
    }
    if let Some(mut intro) = intro {
        intro.shown_secs = intro.shown_secs.max(INTRO_HOLD_SECS + INTRO_FADE_SECS);
    }
    info!(
        "playable: diagnostics {}{}",
        if overlays.diagnostics { "on" } else { "off" },
        if overlays.diagnostics_detail {
            ", detail on"
        } else {
            ""
        }
    );
}

/// One scrimmed text node.
fn text_node(text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(PANEL_FONT_PX),
            ..default()
        },
        TextColor(TEXT_COLOUR),
        Node {
            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, SCRIM_ALPHA)),
    )
}

/// The intro at the top left (when an example names one), the panel under
/// it, the hint row, and the label pool. Over the HUD and the interface, and
/// never in the pointer's way.
fn spawn_the_overlay(mut commands: Commands, intro: Option<Res<Intro>>) {
    commands
        .spawn((
            OverlayRoot,
            Name::new("PlayableDiagnostics"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(OVERLAY_INSET_PX),
                top: Val::Px(OVERLAY_TOP_PX),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(4.0),
                ..default()
            },
            GlobalZIndex(i32::MAX - 2),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            if let Some(intro) = &intro {
                parent.spawn((IntroText, Pickable::IGNORE, text_node(&intro.text)));
            }
            parent.spawn((PanelText, Pickable::IGNORE, text_node("")));
        });
    commands
        .spawn((
            HintRow,
            Name::new("PlayableDiagnosticsHints"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(HINT_ROW_BOTTOM_PX),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(i32::MAX - 2),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((HintText, Pickable::IGNORE, text_node("")));
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
                    font_size: FontSize::Px(PANEL_FONT_PX),
                    ..default()
                },
                TextColor(LABEL_COLOUR),
                Node {
                    position_type: PositionType::Absolute,
                    padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, SCRIM_ALPHA - 0.1)),
            ),
        ));
    }
}

/// Show or hide a node, writing only a change.
fn set_display(node: &mut Node, shown: bool) {
    let display = if shown { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
}

/// Write the panel's text, or hide it while the layer is off or empty.
fn show_the_panel(
    overlays: Res<DebugOverlays>,
    panel: Res<DiagnosticsPanel>,
    mut q_panel: Query<(&mut Text, &mut Node), With<PanelText>>,
) {
    let Ok((mut text, mut node)) = q_panel.single_mut() else {
        return;
    };
    let said = panel.text(overlays.diagnostics_detail);
    set_display(&mut node, overlays.diagnostics && !said.is_empty());
    if text.0 != said {
        text.0 = said;
    }
}

/// Write this frame's key hints, or hide the row with none.
fn show_the_hints(
    overlays: Res<DebugOverlays>,
    panel: Res<DiagnosticsPanel>,
    mut q_hints: Query<(&mut Text, &mut Node), With<HintText>>,
) {
    let Ok((mut text, mut node)) = q_hints.single_mut() else {
        return;
    };
    let said = panel.hint_text();
    set_display(&mut node, overlays.diagnostics && !said.is_empty());
    if text.0 != said {
        text.0 = said;
    }
}

/// Count the intro's time in play, on the real clock so a frozen world still
/// fades it, then fade it out and hide it.
fn fade_the_intro(
    time: Res<Time<Real>>,
    game: Option<Res<State<GameStates>>>,
    intro: Option<ResMut<Intro>>,
    mut q_intro: Query<(&mut Node, &mut TextColor, &mut BackgroundColor), With<IntroText>>,
) {
    let Some(mut intro) = intro else {
        return;
    };
    let Ok((mut node, mut color, mut scrim)) = q_intro.single_mut() else {
        return;
    };
    // Loading on a software rasterizer takes longer than the intro: count
    // only once play shows. No game states (a bare rig) counts at once.
    if game.is_none_or(|game| *game.get() == GameStates::Playing) {
        intro.shown_secs += time.delta_secs();
    }
    let fade = ((intro.shown_secs - INTRO_HOLD_SECS) / INTRO_FADE_SECS).clamp(0.0, 1.0);
    set_display(&mut node, fade < 1.0);
    let alpha = 1.0 - fade;
    if (color.0.alpha() - alpha).abs() > f32::EPSILON {
        color.0.set_alpha(alpha);
        scrim.0.set_alpha(SCRIM_ALPHA * alpha);
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

/// Whether a surface covers the flight view: the TAB interface, the command
/// modal, the pause menu or the Nova Debug window. Absent pause state (no
/// gameplay plugins) is flight.
fn surface_up(pause: Option<&State<PauseStates>>) -> bool {
    pause.is_some_and(|pause| *pause.get() != PauseStates::Unpaused)
}

/// Hold the panel at the top left and the hint row over the HUD's key chips
/// in flight; while a surface is up, the panel goes to the bottom left and
/// the hint row to the bottom edge.
fn move_the_overlay(
    pause: Option<Res<State<PauseStates>>>,
    mut q_root: Query<&mut Node, (With<OverlayRoot>, Without<HintRow>)>,
    mut q_row: Query<&mut Node, (With<HintRow>, Without<OverlayRoot>)>,
) {
    let up = surface_up(pause.as_deref());
    let (top, bottom) = if up {
        (Val::Auto, Val::Px(OVERLAY_INSET_PX))
    } else {
        (Val::Px(OVERLAY_TOP_PX), Val::Auto)
    };
    for mut node in &mut q_root {
        if node.top != top || node.bottom != bottom {
            node.top = top;
            node.bottom = bottom;
        }
    }
    let row_bottom = Val::Px(if up {
        OVERLAY_INSET_PX
    } else {
        HINT_ROW_BOTTOM_PX
    });
    for mut node in &mut q_row {
        if node.bottom != row_bottom {
            node.bottom = row_bottom;
        }
    }
}

/// Project each label over its world point; hide the unused and the
/// off-screen ones, and all of them while a surface is up.
fn place_the_labels(
    overlays: Res<DebugOverlays>,
    pause: Option<Res<State<PauseStates>>>,
    panel: Res<DiagnosticsPanel>,
    q_cameras: Query<(&Camera, &GlobalTransform, Has<ScenarioCameraMarker>), With<Camera3d>>,
    mut q_labels: Query<(
        &WorldLabel,
        &mut Text,
        &mut TextColor,
        &mut Node,
        &mut Visibility,
    )>,
) {
    let camera = view_camera(&q_cameras);
    for (label, mut text, mut color, mut node, mut visibility) in &mut q_labels {
        let placed = (overlays.diagnostics && !surface_up(pause.as_deref()))
            .then(|| panel.labels.get(label.0))
            .flatten()
            .zip(camera)
            .and_then(|((at, said, tint), (camera, transform))| {
                camera
                    .world_to_viewport(transform, *at)
                    .ok()
                    .map(|pixel| (pixel, said, tint))
            });
        let Some((pixel, said, tint)) = placed else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        if text.0 != *said {
            text.0.clone_from(said);
        }
        if color.0 != *tint {
            color.0 = *tint;
        }
        node.left = Val::Px(pixel.x + 5.0);
        node.top = Val::Px(pixel.y - 7.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Detail lines join the panel only with detail on, and the hint row
    /// carries one entry per key written this frame.
    #[test]
    fn detail_lines_and_key_hints_show_only_when_written_and_wanted() {
        let mut panel = DiagnosticsPanel::default();
        panel.line("setup: Ready");
        panel.detail("escort 1: closing +0.4 m/s");
        assert_eq!(panel.text(false), "setup: Ready");
        assert_eq!(panel.text(true), "setup: Ready\nescort 1: closing +0.4 m/s");
        assert_eq!(panel.hint_text(), "", "no hint written, no row");

        panel.hint("D", "dock");
        panel.hint("N", "next hull");
        panel.hint("D", "dock again");
        assert_eq!(panel.hint_text(), "[D] dock   [N] next hull");
    }
}
