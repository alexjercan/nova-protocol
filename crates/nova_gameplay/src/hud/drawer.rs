//! The Tab ship-computer drawer (task 20260724-102304): two side panels that
//! slide in on Tab - a right panel (objectives) and a left panel (comms/log) -
//! freezing the sim and freeing the cursor while open. Opening the drawer also
//! HIDES the flight HUD and deepens the backdrop into a gray field so the old UI
//! does not fight the drawer for readability, keeping only the top status strip
//! and the lower-left keybind hints (task 20260724-134335). This module owns the
//! SHELL - the interaction model, the dual slide, the section framework - plus
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

/// Openness in `[0, 1]`: 0 fully closed (off-screen past the panel's edge), 1
/// fully open (flush with that edge). Eased toward the state-driven target with
/// real time so it keeps moving while the sim is frozen.
#[derive(Component, Default)]
struct DrawerOpenness(f32);

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
            // Placeholder section: the comms/flight-log content (task
            // 20260724-102309) drops into this framework, mirroring the
            // right panel's titled-block + list shape.
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|section| {
                    section.spawn((
                        Text::new("FLIGHT LOG"),
                        TextFont::from_font_size(DRAWER_SECTION_TITLE_FONT_PX),
                        TextColor(theme::TEXT_MUTED),
                    ));
                    section.spawn((
                        Text::new("No messages yet."),
                        TextFont::from_font_size(DRAWER_LINE_FONT_PX),
                        TextColor(theme::TEXT_MUTED),
                    ));
                });
        });
}

/// Despawn the drawer shell when the player ship goes away.
fn remove_drawer(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_parts: Query<Entity, Or<(With<DrawerRootMarker>, With<DrawerBackdropMarker>)>>,
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
