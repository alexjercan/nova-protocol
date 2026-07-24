//! The minimalist flight objective HINT (task 20260724-134312): a small
//! top-right status widget that replaced the always-on compact objectives panel.
//! It shows just an objective glyph + the ACTIVE-objective COUNT + a "TAB"
//! affordance (owner choice - the per-objective detail lives in the diegetic
//! reveal and the Tab drawer, not here). It hides when there are no objectives.
//!
//! It is also the source of the diegetic reveal's tuck anchor: the hint writes
//! [`DrawerTabAnchor`] from its own screen rect (the reveal, task 20260721-211520,
//! tucks a newly posted objective INTO this hint), replacing the old drawer tab
//! handle as the tuck target.

use bevy::prelude::*;
use bevy_common_systems::prelude::GameObjectives;
use nova_ui::theme;

use super::{drawer::DrawerTabAnchor, HudSelfDrivenVisibility, HudTier, NovaHudSystems};
use crate::prelude::*;

const HINT_TOP_PX: f32 = 16.0;
const HINT_RIGHT_PX: f32 = 8.0;
const HINT_GLYPH_PX: f32 = 10.0;
const HINT_FONT_PX: f32 = 14.0;
const HINT_CHIP_FONT_PX: f32 = 11.0;
/// Nominal hint size for the reveal's tuck rect (the exact width flexes with the
/// count; the CENTRE is what the tuck aims at). Task 20260721-211520's target.
const HINT_ANCHOR_SIZE: Vec2 = Vec2::new(120.0, 28.0);

/// The hint root - a top-right row (glyph + count + TAB chip). Also the tuck
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

/// Spawn the hint with the player ship (mirrors the other HUD widgets). Starts
/// hidden; `update_hint` shows it once there is an objective.
fn setup_hint(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    if q_spaceship.get(add.entity).is_err() {
        return;
    }
    commands
        .spawn((
            Name::new("ObjectiveHintHUD"),
            ObjectiveHintMarker,
            HudTier::Chrome,
            // The widget drives its own visibility (hidden at count 0), so the
            // HUD-level restore must not stomp it.
            HudSelfDrivenVisibility,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(HINT_TOP_PX),
                right: Val::Px(HINT_RIGHT_PX),
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|hint| {
            // Objective glyph: a small gold square.
            hint.spawn((
                Node {
                    width: Val::Px(HINT_GLYPH_PX),
                    height: Val::Px(HINT_GLYPH_PX),
                    ..default()
                },
                BackgroundColor(theme::semantic::OBJECTIVE),
            ));
            // Active-objective count.
            hint.spawn((
                ObjectiveHintCountMarker,
                Text::new("0"),
                TextFont::from_font_size(HINT_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
            ));
            // The "TAB" affordance chip.
            hint.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(theme::BORDER_BRIGHT),
                BackgroundColor(theme::PANEL),
                children![(
                    Text::new("TAB"),
                    TextFont::from_font_size(HINT_CHIP_FONT_PX),
                    TextColor(theme::TEXT_MUTED),
                )],
            ));
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

/// Keep the count current and hide the hint when there are no objectives.
fn update_hint(
    objectives: Res<GameObjectives>,
    mut q_root: Query<&mut Visibility, With<ObjectiveHintMarker>>,
    mut q_count: Query<&mut Text, With<ObjectiveHintCountMarker>>,
) {
    let count = objectives.objectives.len();
    for mut visibility in &mut q_root {
        // Hidden at 0; otherwise Inherited so the HUD-visibility cycle can still
        // hide the whole Chrome tier above it.
        let wanted = if count == 0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        visibility.set_if_neq(wanted);
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
        // A player ship spawns the hint (like the real HUD).
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();
        app
    }

    fn hint_visibility(app: &mut App) -> Visibility {
        *app.world_mut()
            .query_filtered::<&Visibility, With<ObjectiveHintMarker>>()
            .single(app.world())
            .expect("the hint spawned")
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
    fn objective_hint_shows_count_and_hides_when_empty() {
        let mut app = hint_app();
        // No objectives: hidden.
        assert_eq!(hint_visibility(&mut app), Visibility::Hidden);

        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn"), Objective::new("b2", "Dock")];
        app.update();
        assert_eq!(
            hint_visibility(&mut app),
            Visibility::Inherited,
            "two objectives -> the hint shows"
        );
        assert_eq!(hint_count_text(&mut app), "2", "the hint shows the count");

        app.world_mut().resource_mut::<GameObjectives>().objectives = Vec::new();
        app.update();
        assert_eq!(
            hint_visibility(&mut app),
            Visibility::Hidden,
            "no objectives -> the hint hides"
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
