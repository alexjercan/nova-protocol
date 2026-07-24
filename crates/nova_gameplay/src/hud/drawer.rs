//! The Tab ship-computer drawer (task 20260724-102304): a right-side surface
//! that slides in on Tab, freezes the sim and frees the cursor while it is
//! open, and hosts expandable sections. This module owns the SHELL - the
//! interaction model, the slide, the tab handle, the section framework - plus
//! the first section (expanded objectives). The comms-log, minimap and
//! ship-status sections (tasks 20260724-102309/102320/102332) slot into the
//! same section framework later.
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

use bevy::prelude::*;
use bevy_common_systems::prelude::GameObjectives;
use nova_ui::theme;

use super::{HudTier, NovaHudSystems};
use crate::{prelude::*, GameStates, PauseStates};

/// Panel width in logical pixels.
const DRAWER_WIDTH_PX: f32 = 340.0;
/// Seconds for the panel to slide fully open (or closed).
const DRAWER_SLIDE_SECS: f32 = 0.22;
/// The collapsed tab handle on the right edge.
const TAB_HANDLE_WIDTH_PX: f32 = 22.0;
const TAB_HANDLE_HEIGHT_PX: f32 = 96.0;
/// Backdrop dim at full open.
const DRAWER_BACKDROP_ALPHA: f32 = 0.55;
const DRAWER_TITLE_FONT_PX: f32 = 16.0;
const DRAWER_SECTION_TITLE_FONT_PX: f32 = 12.0;
const DRAWER_LINE_FONT_PX: f32 = 13.0;

/// Global stacking-context z for the OPEN drawer: it is a modal, so backdrop and
/// panel rise above the flight HUD chrome (which carries no `GlobalZIndex` = 0).
/// Same modal tier the pause overlay uses (`nova_menu`); the drawer and the
/// pause menu are mutually exclusive `PauseStates` variants, so sharing the tier
/// is fine. The tab handle stays at the HUD z (it is chrome). Task 20260724-121541.
const DRAWER_BACKDROP_Z: i32 = 10;
const DRAWER_PANEL_Z: i32 = 11;

/// The sliding panel root. Carries [`DrawerOpenness`].
#[derive(Component)]
struct DrawerRootMarker;

/// The dim full-screen backdrop behind the panel.
#[derive(Component)]
struct DrawerBackdropMarker;

/// The always-visible tab handle on the right edge. Its screen rect is
/// republished as [`DrawerTabAnchor`] - the tuck-target for the diegetic
/// objective animation (task 20260721-211520).
#[derive(Component)]
struct DrawerTabHandleMarker;

/// The container the objectives-section lines are (re)built into.
#[derive(Component)]
struct DrawerObjectivesListMarker;

/// Openness in `[0, 1]`: 0 fully closed (off-screen right), 1 fully open
/// (flush with the right edge). Eased toward the state-driven target with real
/// time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
struct DrawerOpenness(f32);

/// The tab handle's screen-space rectangle in logical pixels, republished every
/// frame from the handle node's global transform. This is task
/// 20260721-211520's tween TARGET: the big cockpit objective animates INTO this
/// rect before tucking into the drawer. `None` until the handle has been laid
/// out at least once (headless rigs without a UI layout pass leave it `None`).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DrawerTabAnchor {
    /// The handle rect in logical window pixels, or `None` before first layout.
    pub rect: Option<Rect>,
}

/// Wires the Tab drawer shell: the toggle, the slide, the tab-handle anchor and
/// the objectives section. Registered by [`super::NovaHudPlugin`].
pub struct NovaDrawerPlugin;

impl Plugin for NovaDrawerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawerTabAnchor>();

        // Tab toggles the drawer. Runs in all of Playing (NOT the
        // Unpaused-gated flight rig) so it can also CLOSE the drawer while the
        // sim is frozen.
        app.add_systems(Update, toggle_drawer.run_if(in_state(GameStates::Playing)));

        // Shell upkeep while the HUD is live: ease the slide, republish the tab
        // anchor, and rebuild the objectives section on change / first spawn.
        app.add_systems(
            Update,
            (
                drive_drawer_slide,
                update_tab_anchor,
                rebuild_drawer_objectives
                    .run_if(resource_changed::<GameObjectives>.or_else(list_just_spawned)),
            )
                .in_set(NovaHudSystems),
        );

        // The drawer is a flight surface: spawn/despawn it with the player ship,
        // like the rest of the HUD.
        app.add_observer(setup_drawer);
        app.add_observer(remove_drawer);
    }
}

/// Tab drives the shared freeze axis. `Unpaused <-> Drawer`; inert while the
/// pause menu owns the freeze (`Paused`) - which is also how a live outcome
/// (it forces `Paused`) blocks the drawer without a cross-crate dependency.
fn toggle_drawer(
    keys: Res<ButtonInput<KeyCode>>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
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
fn list_just_spawned(q: Query<(), Added<DrawerObjectivesListMarker>>) -> bool {
    !q.is_empty()
}

/// Ease [`DrawerOpenness`] toward the state-driven target (1 open, 0 closed)
/// with REAL time, and map it onto the panel offset, the backdrop alpha and
/// both nodes' visibility. Real time because virtual time is paused while the
/// drawer is open (see the module docs).
fn drive_drawer_slide(
    time: Res<Time<Real>>,
    pause: Res<State<PauseStates>>,
    mut q_panel: Query<
        (&mut DrawerOpenness, &mut Node, &mut Visibility),
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

    // The backdrop tracks the panel's openness; default to the target when no
    // panel exists (headless rigs) so the two stay consistent.
    let mut openness = target;
    for (mut panel_openness, mut node, mut visibility) in &mut q_panel {
        panel_openness.0 = approach(panel_openness.0, target, step);
        openness = panel_openness.0;
        // Closed -> off-screen right; open -> flush (right = 0).
        node.right = Val::Px((panel_openness.0 - 1.0) * DRAWER_WIDTH_PX);
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

/// Republish the tab handle's screen rect as [`DrawerTabAnchor`] for task
/// 20260721-211520. The handle has a fixed pixel size, so the rect is its
/// laid-out center (from the UI global transform) plus that size - no dependence
/// on `ComputedNode`, which keeps the anchor math unit-testable.
fn update_tab_anchor(
    q_handle: Query<&GlobalTransform, With<DrawerTabHandleMarker>>,
    mut anchor: ResMut<DrawerTabAnchor>,
) {
    let Ok(gt) = q_handle.single() else {
        return;
    };
    let center = gt.translation().truncate();
    let size = Vec2::new(TAB_HANDLE_WIDTH_PX, TAB_HANDLE_HEIGHT_PX);
    anchor.rect = Some(Rect::from_center_size(center, size));
}

/// Rebuild the objectives-section lines from [`GameObjectives`]: despawn the
/// old lines and spawn one text line per objective. Runs on an objectives
/// change or the first frame the list container exists.
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
            parent.spawn((
                Text::new("No active objectives."),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
            return;
        }
        for objective in &objectives.objectives {
            parent.spawn((
                Text::new(objective.message.clone()),
                TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                TextColor(theme::TEXT),
            ));
        }
    });
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

    // The tab handle on the right edge. Chrome tier: it IS flight chrome, so it
    // rides the grave/tilde HUD-visibility cycle (hidden when the HUD is off) -
    // unlike the panel/backdrop, which must show even with the HUD minimized.
    commands.spawn((
        Name::new("DrawerTabHandle"),
        DrawerTabHandleMarker,
        HudTier::Chrome,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Percent(50.0),
            width: Val::Px(TAB_HANDLE_WIDTH_PX),
            height: Val::Px(TAB_HANDLE_HEIGHT_PX),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(theme::BORDER_BRIGHT),
        BackgroundColor(theme::PANEL_RAISED),
    ));

    // The sliding panel, starting closed (off-screen right).
    commands
        .spawn((
            Name::new("DrawerPanel"),
            DrawerRootMarker,
            DrawerOpenness(0.0),
            GlobalZIndex(DRAWER_PANEL_Z),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
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
                    ..default()
                })
                .with_children(|section| {
                    section.spawn((
                        Text::new("OBJECTIVES"),
                        TextFont::from_font_size(DRAWER_SECTION_TITLE_FONT_PX),
                        TextColor(theme::TEXT_MUTED),
                    ));
                    section.spawn((
                        DrawerObjectivesListMarker,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(3.0),
                            ..default()
                        },
                    ));
                });
        });
}

/// Despawn the drawer shell when the player ship goes away.
fn remove_drawer(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_parts: Query<
        Entity,
        Or<(
            With<DrawerRootMarker>,
            With<DrawerBackdropMarker>,
            With<DrawerTabHandleMarker>,
        )>,
    >,
) {
    for entity in &q_parts {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;
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

    /// The anchor math: a handle at a known laid-out center republishes a rect
    /// centered there with the handle's fixed size. Deleting the update system
    /// leaves the anchor `None`, so this fails without the mechanism.
    #[test]
    fn drawer_exposes_tab_handle_anchor() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DrawerTabAnchor>();
        app.add_systems(Update, update_tab_anchor);

        app.world_mut().spawn((
            DrawerTabHandleMarker,
            GlobalTransform::from_translation(Vec3::new(1900.0, 540.0, 0.0)),
        ));
        app.update();

        let rect = app
            .world()
            .resource::<DrawerTabAnchor>()
            .rect
            .expect("the anchor is published once the handle is laid out");
        assert!(
            (rect.center() - Vec2::new(1900.0, 540.0)).length() < 0.01,
            "the anchor is centered on the handle"
        );
        assert!(
            (rect.width() - TAB_HANDLE_WIDTH_PX).abs() < 0.01
                && (rect.height() - TAB_HANDLE_HEIGHT_PX).abs() < 0.01,
            "the anchor carries the handle's size"
        );
    }

    #[test]
    fn drawer_objectives_section_lists_objectives() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.world_mut().resource_mut::<GameObjectives>().objectives = vec![
            Objective::new("b1", "Burn for Beacon 1"),
            Objective::new("b2", "Dock at the relay"),
        ];
        app.add_systems(
            Update,
            rebuild_drawer_objectives
                .run_if(resource_changed::<GameObjectives>.or_else(list_just_spawned)),
        );
        let list = app.world_mut().spawn(DrawerObjectivesListMarker).id();
        app.update();

        let children: Vec<Entity> = app
            .world()
            .entity(list)
            .get::<Children>()
            .map(|c| c.to_vec())
            .expect("the list has lines");
        let texts: Vec<String> = children
            .iter()
            .filter_map(|&c| app.world().entity(c).get::<Text>().map(|t| t.0.clone()))
            .collect();
        assert_eq!(
            texts,
            vec![
                "Burn for Beacon 1".to_string(),
                "Dock at the relay".to_string()
            ],
            "the objectives section renders one line per objective"
        );
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

        let panel_z = app
            .world_mut()
            .query_filtered::<&GlobalZIndex, With<DrawerRootMarker>>()
            .single(app.world())
            .expect("the drawer panel carries an explicit GlobalZIndex")
            .0;
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
        assert!(
            panel_z >= backdrop_z,
            "the panel sits at or above the backdrop (panel {panel_z}, backdrop {backdrop_z})"
        );
    }
}
