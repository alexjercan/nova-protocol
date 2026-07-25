//! The Tab ship-computer drawer (task 20260724-102304): two side panels that
//! slide in on Tab - a right panel (objectives) and a left panel (comms/log) -
//! freezing the sim and freeing the cursor while open. Opening the drawer also
//! HIDES the flight HUD and deepens the backdrop into a gray field so the old UI
//! does not fight the drawer for readability, keeping only the top status strip
//! and the lower-left keybind hints (task 20260724-134335). This module owns the
//! SHELL - the interaction model, the dual slide, the section framework -
//! plus the right current-objectives section and the left combined flight-log
//! stream. The minimap and ship-status sections (tasks 20260724-102320/102332)
//! slot into the same section framework later.
//!
//! # Interaction model
//!
//! Tab toggles the drawer by driving the shared [`PauseStates`] axis
//! (`Unpaused <-> Drawer`); the freeze + cursor-free are wired in `nova_menu`
//! on `OnEnter/OnExit(PauseStates::Drawer)`, reusing the exact hooks the pause
//! overlay uses (see this task's DECISION.md - the drawer is a THIRD variant of
//! the one freeze axis, not a separate freeze). ESC while the drawer is open
//! closes it (`nova_menu`'s `toggle_pause`). The drawer is inert while the
//! pause menu owns the freeze (`PauseStates::Paused`), which also means a live
//! outcome overlay - which forces `Paused` - implicitly blocks the drawer
//! without this crate depending on `nova_scenario`'s `CurrentOutcome`.
//!
//! # Animation clock
//!
//! The slide is driven by [`Time<Real>`], NOT the bcs `Tween` (which advances
//! on the default `Res<Time>` = `Time<Virtual>`). Opening the drawer PAUSES
//! virtual time, so a virtual-clocked tween would freeze mid-slide; the slide
//! must keep moving while the sim is frozen, so it reads real time
//! (`verify-engine-guarantees-in-source`: bcs `tween::advance_tweens` uses
//! `Res<Time>`).

use bevy::{picking::hover::Hovered, prelude::*};
use bevy_common_systems::prelude::{GameObjectives, Objective};
use nova_ui::theme;

use super::NovaHudSystems;
use crate::{prelude::*, GameStates, PauseStates};

/// Panel width in logical pixels.
const DRAWER_WIDTH_PX: f32 = 340.0;
/// Seconds for the panel to slide fully open (or closed).
const DRAWER_SLIDE_SECS: f32 = 0.22;
/// Backdrop dim at full open. Deepened from the original 0.55 (task
/// 20260724-134335): with the flight HUD hidden while the drawer is open, the
/// backdrop is the ONLY thing separating the drawer from the frozen scene, so
/// it doubles as the "you do not notice the old UI is gone" gray field. The
/// owner chose a deeper gray over a real scene blur at the /flow gate (bevy
/// 0.19 has no UI backdrop-filter; see this task's DECISION.md).
const DRAWER_BACKDROP_ALPHA: f32 = 0.86;
const DRAWER_TITLE_FONT_PX: f32 = 16.0;
const DRAWER_SECTION_TITLE_FONT_PX: f32 = 12.0;
const DRAWER_LINE_FONT_PX: f32 = 13.0;
const DRAWER_ROW_GAP_PX: f32 = 6.0;
const DRAWER_ROW_PADDING_X_PX: f32 = 8.0;
const DRAWER_ROW_PADDING_Y_PX: f32 = 7.0;
const DRAWER_OBJECTIVE_GLYPH_WIDTH_PX: f32 = 18.0;
const DRAWER_LOG_ICON_SIZE_PX: f32 = 20.0;
const DRAWER_SCROLL_LINE_HEIGHT_PX: f32 = 20.0;

/// Top inset for BOTH panels, reserving the top status strip (`readout`) as a
/// window-manager-style status bar: no drawer UI sits in it (task 20260724-134335).
const DRAWER_TOP_INSET_PX: f32 = 52.0;
/// Bottom inset for the LEFT panel so it never covers the lower-left keybind
/// hint cluster (`keybind_hints`, anchored at bottom:8 left:8). Sized to clear
/// the cluster's seven rows (task 20260724-134335).
const DRAWER_LEFT_BOTTOM_INSET_PX: f32 = 140.0;

/// Global stacking-context z for the OPEN drawer: it is a modal, so backdrop and
/// panel rise above the flight HUD chrome (which carries no `GlobalZIndex` = 0).
/// Same modal tier the pause overlay uses (`nova_menu`); the drawer and the
/// pause menu are mutually exclusive `PauseStates` variants, so sharing the tier
/// is fine. The tab handle stays at the HUD z (it is chrome). Task 20260724-121541.
const DRAWER_BACKDROP_Z: i32 = 10;
const DRAWER_PANEL_Z: i32 = 11;
/// z for the drawer-exempt flight chrome (the status strip + keybind hints) that
/// STAYS visible while the drawer is open: it must sit ABOVE the deepened
/// backdrop so the gray field cannot dim it (task 20260724-134335). Read by
/// `readout` and `keybind_hints` when they tag themselves [`super::HudDrawerExempt`].
pub(crate) const DRAWER_EXEMPT_Z: i32 = 12;

/// The sliding panel root. Carries [`DrawerOpenness`] and a [`DrawerSide`].
#[derive(Component)]
struct DrawerRootMarker;

/// Which edge a drawer panel slides in from. The right panel (objectives) and
/// the left panel (comms/log placeholder, content in task 20260724-102309)
/// share the shell, slide and openness; only the animated edge differs.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum DrawerSide {
    Left,
    Right,
}

/// The dim full-screen backdrop behind the panel.
#[derive(Component)]
struct DrawerBackdropMarker;

/// The container the objectives-section lines are (re)built into.
#[derive(Component)]
struct DrawerObjectivesListMarker;

/// Scrollable viewport around a drawer row list.
#[derive(Component)]
struct DrawerScrollViewportMarker;

/// One objective row in the drawer's mission-log list.
#[derive(Component)]
struct DrawerObjectiveRowMarker;

/// Objective id copied onto each drawer row for rebuild and tests.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct DrawerObjectiveId(String);

/// Whether a row is still active or retained as completed history.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum DrawerObjectiveRowStatus {
    Active,
}

/// The small status glyph at the start of a drawer objective row.
#[derive(Component)]
struct DrawerObjectiveGlyphMarker;

/// The text entity for a drawer objective row.
#[derive(Component)]
struct DrawerObjectiveTextMarker;

/// Thin overlay used as a completed row's line-through.
#[cfg(test)]
#[derive(Component)]
struct DrawerObjectiveStrikeMarker;

/// Styled empty-state row for the objective list.
#[derive(Component)]
struct DrawerObjectiveEmptyMarker;

/// The container the combined left-panel flight log is rebuilt into.
#[derive(Component)]
struct DrawerFlightLogListMarker;

/// One row in the left-panel combined flight log stream.
#[derive(Component)]
struct DrawerFlightLogRowMarker;

/// Text entity for a combined flight log row.
#[derive(Component)]
struct DrawerFlightLogTextMarker;

/// Styled empty-state row for the combined flight log.
#[derive(Component)]
struct DrawerFlightLogEmptyMarker;

/// Icon semantics for a combined flight log row.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct DrawerFlightLogIconMarker {
    kind: DrawerFlightLogIconKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrawerFlightLogIconKind {
    CommsAuthored,
    Fallback,
    Objective,
}

/// Openness in `[0, 1]`: 0 fully closed (off-screen past the panel's edge), 1
/// fully open (flush with that edge). Eased toward the state-driven target with
/// real time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
struct DrawerOpenness(f32);

/// Drawer-local combined flight log derived from [`StoryFeed`] and
/// [`GameObjectives`].
///
/// The right drawer panel is current-only. The left panel keeps the historical
/// stream: comms rows plus objective posted/completed rows, in the order the HUD
/// observes them. Objective text updates edit the open posted row rather than
/// appending duplicate events.
#[derive(Resource, Default, Debug, Clone)]
struct DrawerFlightLog {
    entries: Vec<DrawerFlightLogEntry>,
    active_objective_entries: Vec<DrawerFlightLogActiveObjective>,
    previous_active: Vec<Objective>,
    seen_story: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DrawerFlightLogActiveObjective {
    id: String,
    entry_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct DrawerFlightLogEntry {
    kind: DrawerFlightLogEntryKind,
    objective_id: Option<String>,
    speaker: Option<String>,
    message: String,
    icon: Option<AssetRef<Image>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawerFlightLogEntryKind {
    Comms,
    ObjectivePosted,
    ObjectiveCompleted,
}

impl DrawerFlightLog {
    fn clear(&mut self) {
        self.entries.clear();
        self.active_objective_entries.clear();
        self.previous_active.clear();
        self.seen_story = 0;
    }
}

/// The reveal's tuck-target rect in logical pixels. This is task 20260721-211520's
/// tween TARGET: the big cockpit objective animates INTO this rect. It is
/// published each frame by `objective_hint` (the minimalist top-right hint
/// replaced the old drawer tab handle as the anchor source - task
/// 20260724-134312). `None` until the hint has laid out at least once (headless
/// rigs without a UI layout pass leave it `None`).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DrawerTabAnchor {
    /// The hint rect in logical window pixels, or `None` before first layout.
    pub rect: Option<Rect>,
}

/// Wires the Tab drawer shell: the toggle, the slide and the objectives section.
/// The reveal's tuck anchor ([`DrawerTabAnchor`]) is published by `objective_hint`.
/// Registered by [`super::NovaHudPlugin`].
pub struct NovaDrawerPlugin;

impl Plugin for NovaDrawerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerTabAnchor>();
        app.init_resource::<DrawerFlightLog>();

        // Tab toggles the drawer. Runs in all of Playing (NOT the
        // Unpaused-gated flight rig) so it can also CLOSE the drawer while the
        // sim is frozen.
        app.add_systems(Update, toggle_drawer.run_if(in_state(GameStates::Playing)));

        // Shell upkeep while the HUD is live: ease the slide and rebuild the
        // objectives section on change / first spawn. (The reveal's tuck anchor
        // is published by `objective_hint`.)
        app.add_systems(
            Update,
            (
                drive_drawer_slide,
                (
                    sync_drawer_logs,
                    rebuild_drawer_objectives,
                    rebuild_drawer_flight_log,
                )
                    .chain()
                    .run_if(
                        resource_changed::<GameObjectives>
                            .or_else(resource_changed::<StoryFeed>)
                            .or_else(drawer_lists_just_spawned),
                    ),
            )
                .in_set(NovaHudSystems),
        );
        app.add_systems(
            Update,
            scroll_drawer_panels
                .run_if(in_state(PauseStates::Drawer))
                .run_if(resource_exists::<Messages<bevy::input::mouse::MouseWheel>>)
                .in_set(NovaHudSystems),
        );

        // The drawer is a flight surface: spawn/despawn it with the player ship,
        // like the rest of the HUD.
        app.add_observer(setup_drawer);
        app.add_observer(remove_drawer);
    }
}

/// Tab (or the gamepad right-stick click) drives the shared freeze axis.
/// `Unpaused <-> Drawer`; inert while the pause menu owns the freeze (`Paused`) -
/// which is also how a live outcome (it forces `Paused`) blocks the drawer
/// without a cross-crate dependency. The pad button is `RightThumb`, the one free
/// button (task 20260724-134312), mirroring `nova_menu`'s optional-gamepad guard.
fn toggle_drawer(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::RightThumb))
        .unwrap_or(false);
    if !keys.just_pressed(KeyCode::Tab) && !pad {
        return;
    }
    match current.get() {
        PauseStates::Unpaused => next.set(PauseStates::Drawer),
        PauseStates::Drawer => next.set(PauseStates::Unpaused),
        PauseStates::Paused => {}
    }
}

/// Run condition: the objectives list container was spawned this frame, so its
/// initial contents must be built from the current [`GameObjectives`] even
/// though the resource itself did not change.
fn drawer_lists_just_spawned(
    q_objectives: Query<(), Added<DrawerObjectivesListMarker>>,
    q_log: Query<(), Added<DrawerFlightLogListMarker>>,
) -> bool {
    !q_objectives.is_empty() || !q_log.is_empty()
}

fn scroll_drawer_panels(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut q_panels: Query<(&mut ScrollPosition, Option<&Hovered>), With<DrawerScrollViewportMarker>>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let dy: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * DRAWER_SCROLL_LINE_HEIGHT_PX,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if dy == 0.0 {
        return;
    }

    let any_hovered = q_panels
        .iter()
        .any(|(_, hovered)| hovered.is_some_and(Hovered::get));

    for (mut scroll, hovered) in &mut q_panels {
        if any_hovered && !hovered.is_some_and(Hovered::get) {
            continue;
        }
        scroll.0.y = (scroll.0.y - dy).max(0.0);
    }
}

/// Ease [`DrawerOpenness`] toward the state-driven target (1 open, 0 closed)
/// with REAL time, and map it onto the panel offset, the backdrop alpha and
/// both nodes' visibility. Real time because virtual time is paused while the
/// drawer is open (see the module docs).
fn drive_drawer_slide(
    time: Res<Time<Real>>,
    pause: Res<State<PauseStates>>,
    mut q_panel: Query<
        (&mut DrawerOpenness, &mut Node, &mut Visibility, &DrawerSide),
        (With<DrawerRootMarker>, Without<DrawerBackdropMarker>),
    >,
    mut q_backdrop: Query<
        (&mut BackgroundColor, &mut Visibility),
        (With<DrawerBackdropMarker>, Without<DrawerRootMarker>),
    >,
) {
    let target = if *pause.get() == PauseStates::Drawer {
        1.0
    } else {
        0.0
    };
    let step = time.delta_secs() / DRAWER_SLIDE_SECS.max(f32::EPSILON);

    // The backdrop tracks the panels' openness; default to the target when no
    // panel exists (headless rigs) so the two stay consistent. Both panels
    // share the same eased openness, so either one is a faithful source.
    let mut openness = target;
    for (mut panel_openness, mut node, mut visibility, side) in &mut q_panel {
        panel_openness.0 = approach(panel_openness.0, target, step);
        openness = panel_openness.0;
        // Closed -> off-screen past its edge; open -> flush (offset = 0).
        let offset = Val::Px((panel_openness.0 - 1.0) * DRAWER_WIDTH_PX);
        match side {
            DrawerSide::Left => node.left = offset,
            DrawerSide::Right => node.right = offset,
        }
        *visibility = visibility_for(panel_openness.0);
    }

    for (mut background, mut visibility) in &mut q_backdrop {
        background.0 = theme::semantic::BACKDROP.with_alpha(DRAWER_BACKDROP_ALPHA * openness);
        *visibility = visibility_for(openness);
    }
}

/// Hidden once fully closed (so a closed drawer never eats a raycast), visible
/// otherwise.
fn visibility_for(openness: f32) -> Visibility {
    if openness <= f32::EPSILON {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Move `current` toward `target` by at most `step` (a linear approach; the
/// step is a fraction of the full travel per frame).
fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// Update the drawer's combined left-panel flight log from the story feed and
/// active objective list.
fn sync_drawer_logs(
    story: Res<StoryFeed>,
    objectives: Res<GameObjectives>,
    mut log: ResMut<DrawerFlightLog>,
) {
    if story.0.len() < log.seen_story {
        log.clear();
    }

    for line in story.0.iter().skip(log.seen_story) {
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::Comms,
            objective_id: None,
            speaker: Some(line.speaker.clone()),
            message: line.text.clone(),
            icon: line.icon.clone(),
        });
    }
    log.seen_story = story.0.len();

    let completed: Vec<Objective> = log
        .previous_active
        .iter()
        .filter(|old| {
            !objectives
                .objectives
                .iter()
                .any(|current| current.id == old.id)
        })
        .cloned()
        .collect();
    for objective in completed {
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectiveCompleted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .retain(|entry| entry.id != objective.id);
    }

    for objective in &objectives.objectives {
        if let Some(active) = log
            .active_objective_entries
            .iter()
            .find(|entry| entry.id == objective.id)
            .cloned()
        {
            if let Some(entry) = log.entries.get_mut(active.entry_index) {
                entry.message = objective.message.clone();
            }
            continue;
        }

        let entry_index = log.entries.len();
        log.entries.push(DrawerFlightLogEntry {
            kind: DrawerFlightLogEntryKind::ObjectivePosted,
            objective_id: Some(objective.id.clone()),
            speaker: None,
            message: objective.message.clone(),
            icon: None,
        });
        log.active_objective_entries
            .push(DrawerFlightLogActiveObjective {
                id: objective.id.clone(),
                entry_index,
            });
    }

    log.previous_active = objectives.objectives.clone();
}

/// Rebuild the right objectives-section rows from the active objectives list.
fn rebuild_drawer_objectives(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    q_list: Query<(Entity, Option<&Children>), With<DrawerObjectivesListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if objectives.objectives.is_empty() {
            spawn_drawer_empty_objective_row(parent);
            return;
        }
        for objective in &objectives.objectives {
            spawn_drawer_objective_row(parent, objective);
        }
    });
}

fn spawn_drawer_empty_objective_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("DrawerObjectiveEmpty"),
            DrawerObjectiveEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No active objectives."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
        });
}

fn spawn_drawer_objective_row(parent: &mut ChildSpawnerCommands, objective: &Objective) {
    parent
        .spawn((
            Name::new(format!("DrawerObjective {}", objective.id)),
            DrawerObjectiveRowMarker,
            DrawerObjectiveId(objective.id.clone()),
            DrawerObjectiveRowStatus::Active,
            Node {
                min_height: Val::Px(34.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::BORDER_BRIGHT),
            BackgroundColor(theme::PANEL_RAISED),
        ))
        .with_children(|row| {
            row.spawn((
                DrawerObjectiveGlyphMarker,
                Text::new(">"),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
                Node {
                    width: Val::Px(DRAWER_OBJECTIVE_GLYPH_WIDTH_PX),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            row.spawn(Node {
                position_type: PositionType::Relative,
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|text_wrap| {
                text_wrap.spawn((
                    DrawerObjectiveTextMarker,
                    Text::new(objective.message.clone()),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextLayout {
                        justify: Justify::Left,
                        linebreak: LineBreak::WordBoundary,
                    },
                    TextColor(theme::TEXT),
                ));
            });
        });
}

/// Rebuild the left combined flight-log stream.
fn rebuild_drawer_flight_log(
    mut commands: Commands,
    log: Res<DrawerFlightLog>,
    asset_server: Option<Res<AssetServer>>,
    q_list: Query<(Entity, Option<&Children>), With<DrawerFlightLogListMarker>>,
) {
    let Ok((list, children)) = q_list.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands.entity(list).with_children(|parent| {
        if log.entries.is_empty() {
            spawn_drawer_empty_flight_log_row(parent);
            return;
        }
        for entry in &log.entries {
            spawn_drawer_flight_log_row(parent, entry, asset_server.as_deref());
        }
    });
}

fn spawn_drawer_empty_flight_log_row(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Name::new("DrawerFlightLogEmpty"),
            DrawerFlightLogEmptyMarker,
            Node {
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.45)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("No log entries."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
        });
}

fn spawn_drawer_flight_log_row(
    parent: &mut ChildSpawnerCommands,
    entry: &DrawerFlightLogEntry,
    asset_server: Option<&AssetServer>,
) {
    let icon_kind = match entry.kind {
        DrawerFlightLogEntryKind::Comms if entry.icon.is_some() => {
            DrawerFlightLogIconKind::CommsAuthored
        }
        DrawerFlightLogEntryKind::Comms => DrawerFlightLogIconKind::Fallback,
        DrawerFlightLogEntryKind::ObjectivePosted
        | DrawerFlightLogEntryKind::ObjectiveCompleted => DrawerFlightLogIconKind::Objective,
    };
    let accent = match entry.kind {
        DrawerFlightLogEntryKind::Comms => theme::CYAN,
        DrawerFlightLogEntryKind::ObjectivePosted => theme::semantic::OBJECTIVE,
        DrawerFlightLogEntryKind::ObjectiveCompleted => theme::semantic::ALLY,
    };

    parent
        .spawn((
            Name::new("DrawerFlightLogRow"),
            DrawerFlightLogRowMarker,
            DrawerFlightLogIconMarker { kind: icon_kind },
            Node {
                min_height: Val::Px(30.0),
                padding: UiRect::axes(
                    Val::Px(DRAWER_ROW_PADDING_X_PX),
                    Val::Px(DRAWER_ROW_PADDING_Y_PX),
                ),
                border: UiRect::all(Val::Px(theme::BORDER_W)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(DRAWER_ROW_GAP_PX),
                ..default()
            },
            BorderColor::all(theme::BORDER),
            BackgroundColor(theme::PANEL_RAISED.with_alpha(0.58)),
        ))
        .with_children(|row| {
            spawn_drawer_flight_log_icon(row, entry, icon_kind, accent, asset_server);
            row.spawn((
                DrawerFlightLogTextMarker,
                Text::new(drawer_flight_log_text(entry)),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT),
                TextLayout {
                    justify: Justify::Left,
                    linebreak: LineBreak::WordBoundary,
                },
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
        });
}

fn spawn_drawer_flight_log_icon(
    row: &mut ChildSpawnerCommands,
    entry: &DrawerFlightLogEntry,
    icon_kind: DrawerFlightLogIconKind,
    accent: Color,
    asset_server: Option<&AssetServer>,
) {
    let node = Node {
        width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        height: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        min_width: Val::Px(DRAWER_LOG_ICON_SIZE_PX),
        border: UiRect::all(Val::Px(theme::BORDER_W)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        ..default()
    };
    match (&entry.icon, icon_kind) {
        (Some(icon), DrawerFlightLogIconKind::CommsAuthored) => {
            row.spawn((
                node,
                ImageNode::new(
                    asset_server
                        .map(|server| icon.resolve(server))
                        .unwrap_or_default(),
                ),
                BorderColor::all(accent),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ));
        }
        _ => {
            row.spawn((
                node,
                BorderColor::all(accent),
                BackgroundColor(accent.with_alpha(0.16)),
            ))
            .with_children(|icon| {
                icon.spawn((
                    Text::new(match icon_kind {
                        DrawerFlightLogIconKind::Objective => ">",
                        DrawerFlightLogIconKind::CommsAuthored
                        | DrawerFlightLogIconKind::Fallback => "#",
                    }),
                    TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                    TextColor(accent),
                ));
            });
        }
    }
}

fn drawer_flight_log_text(entry: &DrawerFlightLogEntry) -> String {
    match entry.kind {
        DrawerFlightLogEntryKind::Comms => format!(
            "COMMS {} > {}",
            entry.speaker.as_deref().unwrap_or("UNKNOWN").to_uppercase(),
            entry.message
        ),
        DrawerFlightLogEntryKind::ObjectivePosted => format!("OBJ + {}", entry.message),
        DrawerFlightLogEntryKind::ObjectiveCompleted => format!("OBJ x {}", entry.message),
    }
}

/// Spawn the drawer shell (backdrop, sliding panel with sections, tab handle)
/// when the player ship appears - mirrors the other HUD widgets.
fn setup_drawer(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    if q_spaceship.get(add.entity).is_err() {
        return;
    }

    // Dim backdrop behind the panel (hidden until the drawer opens). NO
    // `HudTier`: the drawer is a modal overlay on its own axis, so the
    // grave/tilde HUD-visibility cycle must not touch it - `apply_hud_visibility`
    // force-hides a non-shown Chrome tier every frame (even self-driven ones),
    // which would blank the drawer if the player opened it with the HUD
    // minimized. The panel's visibility is driven entirely by `drive_drawer_slide`.
    commands.spawn((
        Name::new("DrawerBackdrop"),
        DrawerBackdropMarker,
        GlobalZIndex(DRAWER_BACKDROP_Z),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(theme::semantic::BACKDROP.with_alpha(0.0)),
    ));

    // (The old flight-view tab handle was removed in task 20260724-134312; the
    // top-right objective hint is the drawer affordance + the reveal's tuck
    // anchor now.)

    // The RIGHT sliding panel (objectives), starting closed (off-screen right).
    // Top-inset below the status strip so the reserved top bar stays clear.
    commands
        .spawn((
            Name::new("DrawerPanelRight"),
            DrawerRootMarker,
            DrawerSide::Right,
            DrawerOpenness(0.0),
            GlobalZIndex(DRAWER_PANEL_Z),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(DRAWER_TOP_INSET_PX),
                bottom: Val::Px(0.0),
                right: Val::Px(-DRAWER_WIDTH_PX),
                width: Val::Px(DRAWER_WIDTH_PX),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::left(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BorderColor::all(theme::BORDER_BRIGHT),
            BackgroundColor(theme::PANEL),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("SHIP COMPUTER"),
                TextFont::from_font_size(DRAWER_TITLE_FONT_PX),
                TextColor(theme::CYAN_BRIGHT),
            ));
            // Section: OBJECTIVES. The section framework is one titled block
            // plus a list container; later sections (comms log, map, ship)
            // follow the same shape.
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    ..default()
                })
                .with_children(|section| {
                    section.spawn((
                        Text::new("OBJECTIVES"),
                        TextFont::from_font_size(DRAWER_SECTION_TITLE_FONT_PX),
                        TextColor(theme::TEXT_MUTED),
                    ));
                    section
                        .spawn((
                            DrawerScrollViewportMarker,
                            ScrollPosition::default(),
                            Hovered::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ))
                        .with_children(|viewport| {
                            viewport.spawn((
                                DrawerObjectivesListMarker,
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(3.0),
                                    ..default()
                                },
                            ));
                        });
                });
        });

    // The LEFT sliding panel (comms/flight-log), starting closed (off-screen
    // left). This task builds the SHELL + a titled placeholder section; the
    // comms/flight-log content lands in task 20260724-102309, which fills this
    // panel. Top-inset below the status strip like the right panel, and
    // bottom-inset above the lower-left keybind hint cluster so it never covers
    // the keys (task 20260724-134335).
    commands
        .spawn((
            Name::new("DrawerPanelLeft"),
            DrawerRootMarker,
            DrawerSide::Left,
            DrawerOpenness(0.0),
            GlobalZIndex(DRAWER_PANEL_Z),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(DRAWER_TOP_INSET_PX),
                bottom: Val::Px(DRAWER_LEFT_BOTTOM_INSET_PX),
                left: Val::Px(-DRAWER_WIDTH_PX),
                width: Val::Px(DRAWER_WIDTH_PX),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::right(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BorderColor::all(theme::BORDER_BRIGHT),
            BackgroundColor(theme::PANEL),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("COMMS / LOG"),
                TextFont::from_font_size(DRAWER_TITLE_FONT_PX),
                TextColor(theme::CYAN_BRIGHT),
            ));
            // Combined server-style stream: comms transcript rows and mission
            // objective events interleave in the order the HUD observes them
            // (task 20260724-102309).
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    ..default()
                })
                .with_children(|section| {
                    section.spawn((
                        Text::new("FLIGHT LOG"),
                        TextFont::from_font_size(DRAWER_SECTION_TITLE_FONT_PX),
                        TextColor(theme::TEXT_MUTED),
                    ));
                    section
                        .spawn((
                            DrawerScrollViewportMarker,
                            ScrollPosition::default(),
                            Hovered::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                        ))
                        .with_children(|viewport| {
                            viewport.spawn((
                                DrawerFlightLogListMarker,
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(3.0),
                                    ..default()
                                },
                            ));
                        });
                });
        });
}

/// Despawn the drawer shell when the player ship goes away.
fn remove_drawer(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    mut log: ResMut<DrawerFlightLog>,
    q_parts: Query<Entity, Or<(With<DrawerRootMarker>, With<DrawerBackdropMarker>)>>,
) {
    log.clear();
    for entity in &q_parts {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, input::touch::TouchPhase, state::app::StatesPlugin};
    use bevy_common_systems::prelude::Objective;

    use super::*;

    /// A headless app with just the states + the drawer toggle, enough to drive
    /// the interaction-model state machine.
    fn toggle_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, toggle_drawer.run_if(in_state(GameStates::Playing)));
        // Enter Playing so the toggle runs.
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app
    }

    fn press_tab(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        // Clear the just_pressed edge like nova_menu's `press_escape` (no
        // InputPlugin in this rig, so nothing clears it automatically - a stale
        // edge would re-fire the toggle on the next update).
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::Tab);
        keys.clear();
        app.update();
    }

    fn pause_state(app: &App) -> PauseStates {
        app.world().resource::<State<PauseStates>>().get().clone()
    }

    #[test]
    fn tab_toggles_drawer_state() {
        let mut app = toggle_app();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "Tab from Unpaused opens the drawer"
        );
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "Tab again closes the drawer"
        );
    }

    /// One right-stick-click press: press + update (toggle sets NextState), then
    /// release + clear + update (applies the transition; the clear stops the
    /// stale edge re-firing next frame - same shape as `press_tab`).
    fn press_pad(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<GamepadButton>>()
            .press(GamepadButton::RightThumb);
        app.update();
        let mut pad = app.world_mut().resource_mut::<ButtonInput<GamepadButton>>();
        pad.release(GamepadButton::RightThumb);
        pad.clear();
        app.update();
    }

    /// The gamepad right-stick click (`RightThumb`) opens the drawer too (task
    /// 20260724-134312). Narrowing the pad button away fails this.
    #[test]
    fn pad_toggles_drawer_state() {
        let mut app = toggle_app();
        app.init_resource::<ButtonInput<GamepadButton>>();
        assert_eq!(pause_state(&app), PauseStates::Unpaused);

        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Drawer,
            "the right-stick click opens the drawer"
        );
        press_pad(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Unpaused,
            "the right-stick click closes it again"
        );
    }

    #[test]
    fn tab_is_inert_while_the_pause_menu_owns_the_freeze() {
        let mut app = toggle_app();
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        press_tab(&mut app);
        assert_eq!(
            pause_state(&app),
            PauseStates::Paused,
            "Tab does nothing while the pause menu is up"
        );
    }

    // (The tab-handle anchor test moved to `objective_hint` -
    // `objective_hint_provides_the_drawer_anchor` - now that the hint is the
    // reveal's tuck-anchor source, task 20260724-134312.)

    fn objectives_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.init_resource::<StoryFeed>();
        app.init_resource::<DrawerFlightLog>();
        app.add_systems(
            Update,
            (
                sync_drawer_logs,
                rebuild_drawer_objectives,
                rebuild_drawer_flight_log,
            )
                .chain()
                .run_if(
                    resource_changed::<GameObjectives>
                        .or_else(resource_changed::<StoryFeed>)
                        .or_else(drawer_lists_just_spawned),
                ),
        );
        app
    }

    fn spawn_objectives_list(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                DrawerObjectivesListMarker,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .id()
    }

    fn spawn_flight_log_list(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                DrawerFlightLogListMarker,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .id()
    }

    fn spawn_drawer_shell(app: &mut App) {
        app.add_observer(setup_drawer);
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();
    }

    fn parent_of(app: &App, entity: Entity) -> Entity {
        app.world()
            .entity(entity)
            .get::<ChildOf>()
            .expect("entity has parent")
            .0
    }

    fn assert_scrollable_viewport(app: &App, viewport: Entity, label: &str) {
        let node = app.world().entity(viewport).get::<Node>().expect(label);
        assert_eq!(
            node.overflow,
            Overflow::scroll_y(),
            "{label} clips overflowing rows on the y axis"
        );
        assert_eq!(
            node.flex_grow, 1.0,
            "{label} consumes the panel's remaining height instead of growing past it"
        );
        assert!(
            app.world().entity(viewport).contains::<ScrollPosition>(),
            "{label} carries ScrollPosition so wheel input can move it"
        );
    }

    #[test]
    fn drawer_left_flight_log_lives_in_scrollable_viewport() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_drawer_shell(&mut app);

        let list = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerFlightLogListMarker>>()
            .single(app.world())
            .expect("left flight-log inner list");
        let viewport = parent_of(&app, list);

        assert_scrollable_viewport(&app, viewport, "left flight-log viewport");
    }

    #[test]
    fn drawer_right_objectives_live_in_scrollable_viewport() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        spawn_drawer_shell(&mut app);

        let list = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerObjectivesListMarker>>()
            .single(app.world())
            .expect("right objectives inner list");
        let viewport = parent_of(&app, list);

        assert_scrollable_viewport(&app, viewport, "right objectives viewport");
    }

    #[test]
    fn drawer_wheel_scrolls_viewports_and_clamps_at_top() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let scroll_after = |start_y: f32, wheel_y: f32| -> f32 {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.world_mut().init_resource::<Messages<MouseWheel>>();
            app.world_mut().spawn((
                DrawerScrollViewportMarker,
                ScrollPosition(Vec2::new(0.0, start_y)),
            ));
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: wheel_y,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            app.world_mut()
                .run_system_once(scroll_drawer_panels)
                .expect("drawer scroll system runs");
            app.world_mut()
                .query::<&ScrollPosition>()
                .single(app.world())
                .expect("one scroll position")
                .0
                .y
        };

        assert!(
            scroll_after(0.0, -1.0) > 0.0,
            "wheel down from the top scrolls the drawer panel down"
        );
        assert_eq!(
            scroll_after(12.0, 1.0),
            0.0,
            "wheel up clamps at the top instead of going negative"
        );
    }

    #[test]
    fn drawer_wheel_scrolls_only_hovered_viewport_when_one_is_hovered() {
        use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<Messages<MouseWheel>>();
        let hovered = app
            .world_mut()
            .spawn((
                DrawerScrollViewportMarker,
                Hovered(true),
                ScrollPosition(Vec2::ZERO),
            ))
            .id();
        let not_hovered = app
            .world_mut()
            .spawn((
                DrawerScrollViewportMarker,
                Hovered(false),
                ScrollPosition(Vec2::ZERO),
            ))
            .id();

        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
            phase: TouchPhase::Moved,
        });
        app.world_mut()
            .run_system_once(scroll_drawer_panels)
            .expect("drawer scroll system runs");

        let hovered_y = app
            .world()
            .entity(hovered)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        let not_hovered_y = app
            .world()
            .entity(not_hovered)
            .get::<ScrollPosition>()
            .unwrap()
            .0
            .y;
        assert!(
            hovered_y > 0.0,
            "the hovered viewport receives the wheel scroll"
        );
        assert_eq!(
            not_hovered_y, 0.0,
            "a non-hovered viewport does not scroll when another drawer viewport is hovered"
        );
    }

    fn set_objectives(app: &mut App, objectives: Vec<Objective>) {
        app.world_mut().resource_mut::<GameObjectives>().objectives = objectives;
    }

    fn push_story_line(app: &mut App, speaker: &str, text: &str) {
        app.world_mut()
            .resource_mut::<StoryFeed>()
            .0
            .push(StoryLine {
                speaker: speaker.to_string(),
                text: text.to_string(),
                dwell: None,
                icon: None,
            });
    }

    fn row_entities(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<DrawerObjectiveRowMarker>>()
            .iter(app.world())
            .collect()
    }

    fn row_text(app: &App, row: Entity) -> String {
        let mut text = None;
        for child in app
            .world()
            .entity(row)
            .get::<Children>()
            .expect("row children")
        {
            if let Some(found) = text_in_tree(app, *child) {
                text = Some(found);
                break;
            }
        }
        text.expect("row has objective text")
    }

    fn text_in_tree(app: &App, entity: Entity) -> Option<String> {
        let entity_ref = app.world().entity(entity);
        if entity_ref.contains::<DrawerObjectiveTextMarker>() {
            return entity_ref.get::<Text>().map(|text| text.0.clone());
        }
        entity_ref
            .get::<Children>()
            .and_then(|children| children.iter().find_map(|child| text_in_tree(app, child)))
    }

    fn flight_log_texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query_filtered::<&Text, With<DrawerFlightLogTextMarker>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn drawer_objectives_section_uses_styled_rows() {
        let mut app = objectives_app();
        set_objectives(
            &mut app,
            vec![
                Objective::new("b1", "Burn for Beacon 1"),
                Objective::new("b2", "Dock at the relay"),
            ],
        );
        let list = spawn_objectives_list(&mut app);
        app.update();

        let rows = row_entities(&mut app);
        let direct_text_children = app
            .world()
            .entity(list)
            .get::<Children>()
            .expect("list children")
            .iter()
            .filter(|child| app.world().entity(*child).contains::<Text>())
            .count();
        assert_eq!(
            direct_text_children, 0,
            "objectives render as row nodes, not direct bare Text children"
        );
        let row_ids: Vec<String> = rows
            .iter()
            .map(|&row| {
                app.world()
                    .entity(row)
                    .get::<DrawerObjectiveId>()
                    .expect("row id")
                    .0
                    .clone()
            })
            .collect();
        assert_eq!(
            row_ids,
            vec!["b1".to_string(), "b2".to_string()],
            "the objectives section renders one styled row per active objective"
        );
        for &row in &rows {
            assert_eq!(
                *app.world()
                    .entity(row)
                    .get::<DrawerObjectiveRowStatus>()
                    .expect("row status"),
                DrawerObjectiveRowStatus::Active
            );
            assert!(
                app.world().entity(row).get::<BackgroundColor>().is_some(),
                "styled rows carry a fill"
            );
            assert!(
                app.world().entity(row).get::<BorderColor>().is_some(),
                "styled rows carry a border"
            );
            let has_glyph = app
                .world()
                .entity(row)
                .get::<Children>()
                .expect("row children")
                .iter()
                .any(|child| {
                    app.world()
                        .entity(child)
                        .contains::<DrawerObjectiveGlyphMarker>()
                });
            assert!(has_glyph, "styled rows carry a status glyph");
        }
        assert_eq!(row_text(&app, rows[0]), "Burn for Beacon 1");
    }

    #[test]
    fn drawer_left_panel_has_combined_flight_log_stream() {
        let mut app = objectives_app();
        let list = spawn_flight_log_list(&mut app);
        app.update();

        assert!(
            app.world().entity(list).get::<Children>().is_some(),
            "the left panel owns one stream container with an empty row"
        );
        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerFlightLogEmptyMarker>>()
            .single(app.world())
            .expect("combined log empty state");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "combined log empty state carries drawer chrome fill"
        );
    }

    #[test]
    fn drawer_combined_log_renders_story_feed_rows() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        push_story_line(&mut app, "Okono", "Strip it clean.");
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec!["COMMS OKONO > Strip it clean.".to_string()],
            "story feed lines append as comms rows in the combined stream"
        );
        let icon = app
            .world_mut()
            .query_filtered::<&DrawerFlightLogIconMarker, With<DrawerFlightLogRowMarker>>()
            .single(app.world())
            .expect("comms row has an icon marker");
        assert_eq!(icon.kind, DrawerFlightLogIconKind::Fallback);
    }

    #[test]
    fn drawer_combined_log_records_objective_events_once() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        app.update();
        set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
        app.update();
        set_objectives(&mut app, Vec::new());
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "OBJ + Recovered: 1/3".to_string(),
                "OBJ x Recovered: 1/3".to_string(),
            ],
            "an objective text update edits the posted row rather than appending a duplicate"
        );
    }

    #[test]
    fn drawer_combined_log_interleaves_comms_and_objective_rows() {
        let mut app = objectives_app();
        spawn_flight_log_list(&mut app);
        app.update();

        push_story_line(&mut app, "Okono", "First transmission.");
        app.update();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        app.update();
        push_story_line(&mut app, "Relay", "Telemetry locked.");
        app.update();
        set_objectives(&mut app, Vec::new());
        app.update();

        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "COMMS OKONO > First transmission.".to_string(),
                "OBJ + Burn for Beacon 1".to_string(),
                "COMMS RELAY > Telemetry locked.".to_string(),
                "OBJ x Burn for Beacon 1".to_string(),
            ],
            "comms and objective rows share one chronological stream"
        );
    }

    #[test]
    fn drawer_right_panel_shows_only_active_objectives() {
        let mut app = objectives_app();
        set_objectives(
            &mut app,
            vec![
                Objective::new("b1", "Burn for Beacon 1"),
                Objective::new("b2", "Dock at the relay"),
            ],
        );
        spawn_objectives_list(&mut app);
        app.update();

        set_objectives(&mut app, vec![Objective::new("b2", "Dock at the relay")]);
        app.update();

        let rows = row_entities(&mut app);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            *app.world()
                .entity(rows[0])
                .get::<DrawerObjectiveRowStatus>()
                .expect("row status"),
            DrawerObjectiveRowStatus::Active
        );
        assert_eq!(row_text(&app, rows[0]), "Dock at the relay");
        assert!(
            app.world_mut()
                .query_filtered::<(), With<DrawerObjectiveStrikeMarker>>()
                .iter(app.world())
                .next()
                .is_none(),
            "completed objectives are not duplicated as struck-through right-panel rows"
        );
    }

    #[test]
    fn drawer_final_objective_moves_to_flight_log_only() {
        let mut app = objectives_app();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn for Beacon 1")]);
        spawn_objectives_list(&mut app);
        spawn_flight_log_list(&mut app);
        app.update();

        set_objectives(&mut app, Vec::new());
        app.update();

        assert!(row_entities(&mut app).is_empty());
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<DrawerObjectiveEmptyMarker>>()
                .iter(app.world())
                .next()
                .is_some(),
            "the right panel returns to its no-active-objectives empty state"
        );
        assert_eq!(
            flight_log_texts(&mut app),
            vec![
                "OBJ + Burn for Beacon 1".to_string(),
                "OBJ x Burn for Beacon 1".to_string(),
            ],
            "the completed objective remains only in the left Flight Log"
        );
    }

    #[test]
    fn drawer_flight_log_clears_on_drawer_teardown() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DrawerFlightLog>();
        app.add_observer(setup_drawer);
        app.add_observer(remove_drawer);

        let player = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        app.update();
        {
            let mut log = app.world_mut().resource_mut::<DrawerFlightLog>();
            log.entries.push(DrawerFlightLogEntry {
                kind: DrawerFlightLogEntryKind::ObjectiveCompleted,
                objective_id: Some("b1".to_string()),
                speaker: None,
                message: "Burn for Beacon 1".to_string(),
                icon: None,
            });
            log.previous_active = vec![Objective::new("b2", "Dock at the relay")];
            log.seen_story = 1;
        }

        app.world_mut()
            .entity_mut(player)
            .remove::<PlayerSpaceshipMarker>();
        app.update();

        let log = app.world().resource::<DrawerFlightLog>();
        assert!(
            log.entries.is_empty() && log.previous_active.is_empty() && log.seen_story == 0,
            "drawer teardown clears the retained left-panel log"
        );
    }

    #[test]
    fn drawer_objectives_empty_state_is_styled() {
        let mut app = objectives_app();
        spawn_objectives_list(&mut app);
        app.update();

        let empty = app
            .world_mut()
            .query_filtered::<Entity, With<DrawerObjectiveEmptyMarker>>()
            .single(app.world())
            .expect("styled empty row");
        assert!(
            app.world().entity(empty).get::<BackgroundColor>().is_some(),
            "empty state carries drawer chrome fill"
        );
        assert!(
            app.world().entity(empty).get::<BorderColor>().is_some(),
            "empty state carries drawer chrome border"
        );
    }

    #[test]
    fn drawer_objectives_rebuild_replaces_stale_rows() {
        let mut app = objectives_app();
        set_objectives(&mut app, vec![Objective::new("b1", "Burn")]);
        spawn_objectives_list(&mut app);
        app.update();
        let first_rows = row_entities(&mut app);
        assert_eq!(first_rows.len(), 1);

        set_objectives(&mut app, vec![Objective::new("b1", "Recovered: 1/3")]);
        app.update();

        let rows = row_entities(&mut app);
        assert_eq!(rows.len(), 1, "old row entity was replaced");
        assert_ne!(rows[0], first_rows[0], "rebuild despawns stale rows");
        assert_eq!(row_text(&app, rows[0]), "Recovered: 1/3");
    }

    /// The open drawer is a modal: its panel and backdrop must carry an explicit
    /// `GlobalZIndex` above the HUD chrome (which carries none = 0), or the
    /// top-right objectives panel and other flight HUD draw over it. Mirrors
    /// nova_menu's overlay-z assertion. Fails before the fix (no `GlobalZIndex`).
    #[test]
    fn drawer_renders_above_the_hud() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_drawer);
        // setup_drawer fires on the player ship's PlayerSpaceshipMarker add.
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        let backdrop_z = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<DrawerBackdropMarker>>()
            .single(app.world())
            .expect("the drawer backdrop carries an explicit GlobalZIndex")
            .0;
        assert!(
            backdrop_z > 0,
            "the backdrop must stack above the HUD chrome (z = {backdrop_z})"
        );
        // BOTH panels (left + right) sit at or above the backdrop.
        let panel_zs: Vec<i32> = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<DrawerRootMarker>>()
            .iter(app.world())
            .map(|z| z.0)
            .collect();
        assert_eq!(
            panel_zs.len(),
            2,
            "the shell spawns two panels (left + right)"
        );
        for panel_z in panel_zs {
            assert!(
                panel_z >= backdrop_z,
                "each panel sits at or above the backdrop (panel {panel_z}, backdrop {backdrop_z})"
            );
        }
        // The drawer-exempt flight chrome (status strip + keys) must out-rank
        // the backdrop so the deepened gray field cannot dim it.
        assert!(
            DRAWER_EXEMPT_Z > backdrop_z,
            "exempt chrome z ({DRAWER_EXEMPT_Z}) must beat the backdrop ({backdrop_z})"
        );
    }

    /// The shell builds BOTH sliding panels: a right panel that starts off the
    /// right edge and a left panel that starts off the left edge, both inset
    /// below the status strip, and the left one inset above the keybind cluster
    /// so it never covers the keys (task 20260724-134335).
    #[test]
    fn setup_spawns_both_sliding_panels() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(setup_drawer);
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        let mut panels: Vec<(DrawerSide, Node)> = app
            .world_mut()
            .query_filtered::<(&DrawerSide, &Node), With<DrawerRootMarker>>()
            .iter(app.world())
            .map(|(side, node)| (*side, node.clone()))
            .collect();
        assert_eq!(panels.len(), 2);
        panels.sort_by_key(|(side, _)| *side == DrawerSide::Right);

        let (left_side, left) = &panels[0];
        let (right_side, right) = &panels[1];
        assert_eq!(*left_side, DrawerSide::Left);
        assert_eq!(*right_side, DrawerSide::Right);

        // Closed: each panel parked off ITS edge.
        assert_eq!(left.left, Val::Px(-DRAWER_WIDTH_PX));
        assert_eq!(right.right, Val::Px(-DRAWER_WIDTH_PX));
        // Top strip reserved on both.
        assert_eq!(left.top, Val::Px(DRAWER_TOP_INSET_PX));
        assert_eq!(right.top, Val::Px(DRAWER_TOP_INSET_PX));
        // The left panel clears the lower-left keybind cluster.
        assert_eq!(left.bottom, Val::Px(DRAWER_LEFT_BOTTOM_INSET_PX));
    }

    /// `drive_drawer_slide` eases BOTH panels open from their own edges: the
    /// left panel's `left` offset and the right panel's `right` offset both
    /// advance toward 0 while the drawer is open (task 20260724-134335).
    #[test]
    fn slide_drives_both_panel_edges() {
        use std::time::Duration;

        let mut app = App::new();
        // Disable the real TimePlugin so its per-frame clock update cannot
        // overwrite the deltas we advance by hand; drive_drawer_slide reads
        // Time<Real>, which we own here.
        app.add_plugins(MinimalPlugins.build().disable::<bevy::time::TimePlugin>());
        app.insert_resource(Time::<Real>::default());
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.add_systems(Update, drive_drawer_slide);

        let backdrop = app
            .world_mut()
            .spawn((
                DrawerBackdropMarker,
                BackgroundColor(theme::semantic::BACKDROP.with_alpha(0.0)),
                Visibility::Hidden,
            ))
            .id();
        let _ = backdrop;
        let left = app
            .world_mut()
            .spawn((
                DrawerRootMarker,
                DrawerSide::Left,
                DrawerOpenness(0.0),
                Visibility::Hidden,
                Node {
                    left: Val::Px(-DRAWER_WIDTH_PX),
                    ..default()
                },
            ))
            .id();
        let right = app
            .world_mut()
            .spawn((
                DrawerRootMarker,
                DrawerSide::Right,
                DrawerOpenness(0.0),
                Visibility::Hidden,
                Node {
                    right: Val::Px(-DRAWER_WIDTH_PX),
                    ..default()
                },
            ))
            .id();

        // Open the drawer, then advance real time and step the slide.
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Drawer);
        app.update();
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_millis(30));
            app.update();
        }

        let left_off = px(app.world().get::<Node>(left).unwrap().left);
        let right_off = px(app.world().get::<Node>(right).unwrap().right);
        // Both moved in from -WIDTH toward 0 (flush at 0).
        assert!(
            left_off > -DRAWER_WIDTH_PX && left_off <= 0.0,
            "left panel slid in from its edge (offset {left_off})"
        );
        assert!(
            right_off > -DRAWER_WIDTH_PX && right_off <= 0.0,
            "right panel slid in from its edge (offset {right_off})"
        );
        // They share the eased openness, so they track together.
        assert!((left_off - right_off).abs() < f32::EPSILON);
    }

    /// Read a `Val::Px` back as an f32 for the slide assertions.
    fn px(val: Val) -> f32 {
        match val {
            Val::Px(p) => p,
            other => panic!("expected Val::Px, got {other:?}"),
        }
    }
}
