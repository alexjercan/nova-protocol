//! The minimalist flight objective HINT (task 20260724-134312): the
//! ACTIVE-objective COUNT + a "TAB" affordance (owner choice - the per-objective
//! detail lives in the diegetic reveal and the Tab drawer, not here). It
//! collapses when there are no objectives.
//!
//! The hint is a BLOCK IN THE STATUS BAR (task 20260724-161545): it is parented
//! into the bcs status-bar row (the fps/version bar) so it flows beside those
//! items and can never overlap the version, instead of floating as its own
//! top-right node (which collided with the version). It renders as plain text
//! (no pill/glyph) to match the other bar items. It is parented as our OWN child
//! of `StatusBarRootMarker` rather than a `status_bar_item`, because it needs its
//! own node markers for the two behaviours below (the registry's auto-insert
//! builds an unmarked text-only visual).
//!
//! It is also the source of the diegetic reveal's tuck anchor: the hint writes
//! [`DrawerTabAnchor`] from its own screen rect (the reveal, task 20260721-211520,
//! tucks a newly posted objective INTO this hint), replacing the old drawer tab
//! handle as the tuck target.

use bevy::prelude::*;
use bevy_common_systems::prelude::{GameObjectives, StatusBarRootMarker};
use nova_ui::theme;

use super::{drawer::DrawerTabAnchor, NovaHudSystems};
use crate::prelude::*;

const HINT_FONT_PX: f32 = 14.0;
const HINT_CHIP_FONT_PX: f32 = 11.0;
/// Nominal hint size for the reveal's tuck rect (the exact width flexes with the
/// count; the CENTRE is what the tuck aims at). Task 20260721-211520's target.
const HINT_ANCHOR_SIZE: Vec2 = Vec2::new(120.0, 28.0);

/// The hint block in the status bar (count + TAB, plain text). Also the tuck
/// anchor source for the diegetic reveal.
#[derive(Component)]
struct ObjectiveHintMarker;

/// The count text inside the hint.
#[derive(Component)]
struct ObjectiveHintCountMarker;

/// Wires the flight objective hint: spawn/despawn with the player ship, keep the
/// count current, and publish the reveal's tuck anchor.
pub struct ObjectiveHintPlugin;

impl Plugin for ObjectiveHintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_hint, update_tab_anchor).in_set(NovaHudSystems),
        );
        app.add_observer(setup_hint);
        app.add_observer(remove_hint);
    }
}

/// Spawn the hint as a child of the status-bar row when the player ship appears
/// (mirrors the other HUD widgets). It starts collapsed (`Display::None`);
/// `update_hint` reveals it once there is an objective. Parenting it under
/// [`StatusBarRootMarker`] is what puts it in the bar's flex row beside fps +
/// version, so it can never overlap them. Visibility (the grave/tilde HUD cycle
/// and the drawer hide) is INHERITED from the `Status`-tier bar root - the hint
/// carries no `HudTier` of its own.
fn setup_hint(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_bar: Query<Entity, With<StatusBarRootMarker>>,
) {
    if q_spaceship.get(add.entity).is_err() {
        return;
    }
    let Ok(bar) = q_bar.single() else {
        // No status bar (a minimal rig without nova_core's setup_status_ui);
        // the hint lives in the bar, so without it there is nothing to attach to.
        warn!("objective hint: no status bar root found; hint not spawned");
        return;
    };
    commands.entity(bar).with_children(|bar| {
        bar.spawn((
            Name::new("ObjectiveHintItem"),
            ObjectiveHintMarker,
            Pickable::IGNORE,
            // Metrics match the bcs status-bar item so the block sits flush with
            // fps/version. Starts collapsed so an objective-less bar has no gap.
            Node {
                display: Display::None,
                height: Val::Px(24.0),
                margin: UiRect::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|hint| {
            // Active-objective count (gold, the objective accent).
            hint.spawn((
                ObjectiveHintCountMarker,
                Text::new("0"),
                TextFont::from_font_size(HINT_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
            ));
            // The "TAB" affordance, plain muted text (no pill).
            hint.spawn((
                Text::new("TAB"),
                TextFont::from_font_size(HINT_CHIP_FONT_PX),
                TextColor(theme::TEXT_MUTED),
            ));
        });
    });
}

/// Despawn the hint with the player ship.
fn remove_hint(
    _remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_hint: Query<Entity, With<ObjectiveHintMarker>>,
) {
    for hint in &q_hint {
        commands.entity(hint).despawn();
    }
}

/// Keep the count current and collapse the hint when there are no objectives.
/// Toggles `Display` (not `Visibility`): as a flex child of the status bar a
/// hidden-but-laid-out node would leave a gap in the row, so `Display::None`
/// removes it from layout entirely and the bar closes up. The grave/tilde HUD
/// cycle and the drawer hide still work - they act on the `Status`-tier bar root,
/// whose computed visibility this child inherits.
fn update_hint(
    objectives: Res<GameObjectives>,
    mut q_root: Query<&mut Node, With<ObjectiveHintMarker>>,
    mut q_count: Query<&mut Text, With<ObjectiveHintCountMarker>>,
) {
    let count = objectives.objectives.len();
    let wanted = if count == 0 {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut q_root {
        if node.display != wanted {
            node.display = wanted;
        }
    }
    for mut text in &mut q_count {
        let s = count.to_string();
        if text.0 != s {
            text.0 = s;
        }
    }
}

/// Publish the hint's screen rect as [`DrawerTabAnchor`] - the diegetic reveal's
/// tuck target (task 20260721-211520). Mirrors the old drawer-tab-handle anchor;
/// a fixed nominal size keeps the math unit-testable and the tuck aims at the
/// hint's centre.
fn update_tab_anchor(
    q_hint: Query<&GlobalTransform, With<ObjectiveHintMarker>>,
    mut anchor: ResMut<DrawerTabAnchor>,
) {
    let Ok(gt) = q_hint.single() else {
        return;
    };
    anchor.rect = Some(Rect::from_center_size(
        gt.translation().truncate(),
        HINT_ANCHOR_SIZE,
    ));
}

#[cfg(test)]
mod tests {
    use bevy_common_systems::prelude::Objective;

    use super::*;

    fn hint_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameObjectives>();
        app.init_resource::<DrawerTabAnchor>();
        app.add_observer(setup_hint);
        app.add_systems(Update, (update_hint, update_tab_anchor));
        // The hint lives in the status bar, so the bar root must exist first
        // (the real bar comes from nova_core's setup_status_ui).
        app.world_mut().spawn(StatusBarRootMarker);
        // A player ship spawns the hint (like the real HUD).
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();
        app
    }

    fn hint_display(app: &mut App) -> Display {
        app.world_mut()
            .query_filtered::<&Node, With<ObjectiveHintMarker>>()
            .single(app.world())
            .expect("the hint spawned in the status bar")
            .display
    }

    /// The hint is parented under the status bar root, not spawned free-floating.
    fn hint_parent_is_bar(app: &mut App) -> bool {
        let bar = app
            .world_mut()
            .query_filtered::<Entity, With<StatusBarRootMarker>>()
            .single(app.world())
            .expect("the bar root exists");
        let parent = app
            .world_mut()
            .query_filtered::<&ChildOf, With<ObjectiveHintMarker>>()
            .single(app.world())
            .expect("the hint has a parent")
            .0;
        parent == bar
    }

    fn hint_count_text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<ObjectiveHintCountMarker>>()
            .single(app.world())
            .expect("the hint has a count")
            .0
            .clone()
    }

    #[test]
    fn objective_hint_is_a_status_bar_block_that_collapses_when_empty() {
        let mut app = hint_app();
        // It lives in the bar (not a free-floating top-right node), so it flows
        // beside fps/version and cannot overlap them.
        assert!(
            hint_parent_is_bar(&mut app),
            "the hint is parented under the status bar root"
        );
        // No objectives: collapsed out of the row (Display::None, not just hidden,
        // so it leaves no gap).
        assert_eq!(hint_display(&mut app), Display::None);

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn"), Objective::new("b2", "Dock")];
        app.update();
        assert_eq!(
            hint_display(&mut app),
            Display::Flex,
            "two objectives -> the hint takes its place in the row"
        );
        assert_eq!(hint_count_text(&mut app), "2", "the hint shows the count");

        app.world_mut().resource_mut::<GameObjectives>().objectives = Vec::new();
        app.update();
        assert_eq!(
            hint_display(&mut app),
            Display::None,
            "no objectives -> the hint collapses out of the row"
        );
    }

    /// The hint is the reveal's tuck anchor: `update_tab_anchor` publishes the
    /// hint's screen rect into `DrawerTabAnchor`. Deleting the system leaves it
    /// `None`.
    #[test]
    fn objective_hint_provides_the_drawer_anchor() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DrawerTabAnchor>();
        app.add_systems(Update, update_tab_anchor);
        app.world_mut().spawn((
            ObjectiveHintMarker,
            GlobalTransform::from_translation(Vec3::new(1850.0, 24.0, 0.0)),
        ));
        app.update();

        let rect = app
            .world()
            .resource::<DrawerTabAnchor>()
            .rect
            .expect("the anchor is published from the hint");
        assert!(
            (rect.center() - Vec2::new(1850.0, 24.0)).length() < 0.01,
            "the anchor centres on the hint (top-right, so the reveal tucks up-right)"
        );
    }
}
