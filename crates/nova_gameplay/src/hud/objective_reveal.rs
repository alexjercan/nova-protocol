//! Diegetic objective reveal (task 20260721-211520): when a new objective
//! posts, a big, slightly-rotated card appears on the cockpit HUD, holds for a
//! couple of seconds, then TUCKS into the Tab drawer's tab handle (the
//! [`DrawerTabAnchor`]) and vanishes - the "big cockpit moment" the owner asked
//! for. It supersedes the small gold ghost line that fresh postings used to get
//! (task 20260717-163033); completions keep their green ghost line
//! (`objective_feedback`). The spawn is triggered from `objective_feedback`'s
//! single `GameObjectives` diff (one detection point); this module owns the card
//! and its animation.
//!
//! Placement follows the `screen_indicator` pattern: the card's screen position
//! is driven through `Node.left/top` in logical pixels, and `UiTransform` is used
//! only for scale + rotation - so the tuck target ([`DrawerTabAnchor`], already
//! in screen pixels) maps directly and there is no `GlobalTransform`-vs-
//! `UiTransform` coordinate ambiguity. The reveal plays during normal flight
//! (Unpaused), so the default `Res<Time>` clock is correct.

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_common_systems::prelude::Objective;
use nova_ui::theme;

use super::{drawer::DrawerTabAnchor, OBJECTIVE_GOLD};
use crate::prelude::*;

/// Grow-in time (seconds).
const REVEAL_APPEAR_SECS: f32 = 0.35;
/// Hold-at-full time (seconds) - the readable "big" moment.
const REVEAL_HOLD_SECS: f32 = 2.3;
/// Tuck-into-the-tab time (seconds).
const REVEAL_TUCK_SECS: f32 = 0.55;
/// Total lifetime; the card despawns after this.
const REVEAL_TOTAL_SECS: f32 = REVEAL_APPEAR_SECS + REVEAL_HOLD_SECS + REVEAL_TUCK_SECS;

/// Scale at full "big" reveal. Tuned smaller on the 2026-07-24 playtest (was
/// 1.9 - the owner found it too big/centered); it now reads as a modest cockpit
/// card that tucks up-and-right into the top-right objective hint (task
/// 20260724-134312).
const REVEAL_BIG_SCALE: f32 = 1.35;
/// Scale as it disappears into the hint.
const REVEAL_TUCK_SCALE: f32 = 0.22;
/// The jaunty cockpit tilt.
const REVEAL_ROTATION_DEG: f32 = -5.0;

const REVEAL_FONT_PX: f32 = 18.0;
const REVEAL_WIDTH_PX: f32 = 260.0;
/// Nominal card height, used only to centre the card on its target point.
/// This assumes a roughly single-line objective (the shipped ones are); a much
/// taller multi-line objective would land slightly high on the tab during the
/// tuck. Acceptable because the card is shrinking to a point as it arrives - the
/// offset vanishes with it (review R1.1).
const REVEAL_APPROX_HEIGHT_PX: f32 = 44.0;

/// Base cockpit position as a fraction of the viewport (upper-centre).
const REVEAL_BASE_FRAC: Vec2 = Vec2::new(0.5, 0.34);
/// Viewport used when no `PrimaryWindow` exists (headless rigs), so the
/// trajectory stays deterministic and unit-testable.
const FALLBACK_VIEWPORT: Vec2 = Vec2::new(1920.0, 1080.0);

/// A transient objective-reveal card; `elapsed` drives its appear/hold/tuck
/// phases.
#[derive(Component, Debug)]
pub struct ObjectiveRevealMarker {
    /// Seconds since the card was posted.
    pub elapsed: f32,
}

/// The reveal card's text child (alpha is faded on it each frame).
#[derive(Component)]
struct ObjectiveRevealTextMarker;

/// Runs the diegetic objective reveal: the animation + teardown. Spawning is
/// triggered from `objective_feedback` (the single `GameObjectives` diff).
pub struct ObjectiveRevealPlugin;

impl Plugin for ObjectiveRevealPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                animate_objective_reveals,
                clear_reveals_on_teardown.run_if(resource_changed::<GameObjectives>),
            )
                .in_set(super::NovaHudSystems),
        );
    }
}

/// Spawn a reveal card for a freshly posted objective. Called by
/// `objective_feedback` for each addition (replacing its gold ghost line).
pub fn spawn_objective_reveal(commands: &mut Commands, objective: &Objective) {
    // A deliberate top-level orphan node (no HUD-root parent): it is absolutely
    // positioned in screen pixels, transient (~3.2s then despawns, or cleared on
    // teardown), and must not inherit the HUD-visibility cycle - the big cockpit
    // moment shows regardless of the flight HUD level, like the drawer.
    commands
        .spawn((
            Name::new(format!("ObjectiveReveal {}", objective.id)),
            ObjectiveRevealMarker { elapsed: 0.0 },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(REVEAL_WIDTH_PX),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Start invisible + zero-scale; the appear phase grows it in.
            UiTransform {
                rotation: Rot2::degrees(REVEAL_ROTATION_DEG),
                scale: Vec2::ZERO,
                ..default()
            },
            BackgroundColor(theme::PANEL.with_alpha(0.0)),
            BorderColor::all(OBJECTIVE_GOLD.with_alpha(0.0)),
            // A cockpit flourish, never interactive.
            Pickable::IGNORE,
        ))
        .with_children(|card| {
            card.spawn((
                ObjectiveRevealTextMarker,
                Text::new(objective.message.clone()),
                TextFont::from_font_size(REVEAL_FONT_PX),
                TextColor(OBJECTIVE_GOLD.with_alpha(0.0)),
                TextLayout {
                    justify: Justify::Center,
                    linebreak: LineBreak::WordBoundary,
                },
                Pickable::IGNORE,
            ));
        });
}

/// Advance every reveal card: grow in, hold big, then tuck into the tab handle
/// (or fade in place if the drawer handle has not laid out yet), and despawn
/// when spent. Position rides `Node.left/top` (px), scale + rotation ride
/// `UiTransform` (the `screen_indicator` placement pattern).
fn animate_objective_reveals(
    time: Res<Time>,
    anchor: Res<DrawerTabAnchor>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
    mut q_reveal: Query<(
        Entity,
        &mut ObjectiveRevealMarker,
        &mut Node,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut BorderColor,
        &Children,
    )>,
    mut q_text: Query<&mut TextColor, With<ObjectiveRevealTextMarker>>,
) {
    let viewport = q_window
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(FALLBACK_VIEWPORT);
    let base = viewport * REVEAL_BASE_FRAC;
    // The tuck target is the drawer tab handle; before it has laid out
    // (anchor None) the card simply fades in place at `base`.
    let target = anchor.rect.map(|r| r.center()).unwrap_or(base);

    for (entity, mut marker, mut node, mut ui, mut bg, mut border, children) in &mut q_reveal {
        marker.elapsed += time.delta_secs();
        if marker.elapsed >= REVEAL_TOTAL_SECS {
            commands.entity(entity).despawn();
            continue;
        }

        let (scale, alpha, pos) = reveal_phase(marker.elapsed, base, target);
        node.left = Val::Px(pos.x - REVEAL_WIDTH_PX / 2.0);
        node.top = Val::Px(pos.y - REVEAL_APPROX_HEIGHT_PX / 2.0);
        ui.scale = Vec2::splat(scale);
        bg.0 = theme::PANEL.with_alpha(theme::PANEL.alpha() * alpha);
        *border = BorderColor::all(OBJECTIVE_GOLD.with_alpha(alpha));
        for &child in children {
            if let Ok(mut color) = q_text.get_mut(child) {
                color.0 = OBJECTIVE_GOLD.with_alpha(alpha);
            }
        }
    }
}

/// The reveal's `(scale, alpha, screen_position)` at `elapsed` seconds: grow in
/// from nothing, hold big and centred, then shrink + fade while sliding to the
/// tab. Eased with a smoothstep so the moment lands softly.
fn reveal_phase(elapsed: f32, base: Vec2, target: Vec2) -> (f32, f32, Vec2) {
    if elapsed < REVEAL_APPEAR_SECS {
        let t = smoothstep(elapsed / REVEAL_APPEAR_SECS);
        (REVEAL_BIG_SCALE * t, t, base)
    } else if elapsed < REVEAL_APPEAR_SECS + REVEAL_HOLD_SECS {
        (REVEAL_BIG_SCALE, 1.0, base)
    } else {
        let t = smoothstep((elapsed - REVEAL_APPEAR_SECS - REVEAL_HOLD_SECS) / REVEAL_TUCK_SECS);
        let scale = REVEAL_BIG_SCALE + (REVEAL_TUCK_SCALE - REVEAL_BIG_SCALE) * t;
        (scale, 1.0 - t, base.lerp(target, t))
    }
}

/// Smoothstep ease `3t^2 - 2t^3` on a clamped `[0, 1]`.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Emptying [`GameObjectives`] is scenario teardown: despawn any in-flight
/// reveal so a card does not linger over the menu or the next scenario
/// (`state-diff-aliases-reset`, mirroring the ghost-line teardown).
fn clear_reveals_on_teardown(
    mut commands: Commands,
    objectives: Res<GameObjectives>,
    q_reveals: Query<Entity, With<ObjectiveRevealMarker>>,
) {
    if !objectives.objectives.is_empty() {
        return;
    }
    for reveal in &q_reveals {
        commands.entity(reveal).despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn reveal_app() -> App {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Manual 0.1s/frame clock so the appear/hold/tuck phases advance
        // deterministically (the reveal reads the default Res<Time>).
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.1,
        )));
        app.init_resource::<GameObjectives>();
        app.init_resource::<DrawerTabAnchor>();
        app.add_systems(
            Update,
            (
                animate_objective_reveals,
                clear_reveals_on_teardown.run_if(resource_changed::<GameObjectives>),
            ),
        );
        app
    }

    fn reveal_left(app: &mut App) -> Option<f32> {
        let mut q = app
            .world_mut()
            .query_filtered::<&Node, With<ObjectiveRevealMarker>>();
        q.iter(app.world()).next().map(|node| match node.left {
            Val::Px(x) => x,
            _ => f32::NAN,
        })
    }

    fn reveal_top(app: &mut App) -> Option<f32> {
        let mut q = app
            .world_mut()
            .query_filtered::<&Node, With<ObjectiveRevealMarker>>();
        q.iter(app.world()).next().map(|node| match node.top {
            Val::Px(y) => y,
            _ => f32::NAN,
        })
    }

    fn reveal_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<ObjectiveRevealMarker>>()
            .iter(app.world())
            .count()
    }

    /// A posted reveal grows in, holds, then its screen position slides toward
    /// the tab anchor and it despawns after its lifetime. Deleting
    /// `animate_objective_reveals` leaves it frozen at spawn and never
    /// despawning, so this fails without the mechanism.
    #[test]
    fn objective_reveal_spawns_and_tucks_to_the_anchor() {
        let mut app = reveal_app();
        // Anchor far to the right of the upper-centre base (1920*0.5 = 960),
        // so a rightward slide is unambiguous.
        app.world_mut().resource_mut::<DrawerTabAnchor>().rect = Some(Rect::from_center_size(
            Vec2::new(1880.0, 300.0),
            Vec2::new(22.0, 96.0),
        ));
        // Objectives non-empty (as in production when a reveal exists), so the
        // teardown-clear does not fire on the empty-init frame.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();

        // Spawn one card through the production entry point.
        app.world_mut().run_system_once(spawn_one).unwrap();

        // Frame 1: exists, text set, sitting at the upper-centre base.
        app.update();
        assert_eq!(reveal_count(&mut app), 1, "a reveal card spawned");
        let base_left = reveal_left(&mut app).expect("card has a px left");
        // base centre x = 1920*0.5 = 960, card left = 960 - REVEAL_WIDTH_PX/2 = 830.
        let expected_base_left = 960.0 - REVEAL_WIDTH_PX / 2.0;
        assert!(
            (base_left - expected_base_left).abs() < 1.0,
            "card starts at the upper-centre base (got {base_left})"
        );

        // Run out the whole lifetime, tracking the furthest-right it slides and
        // whether it eventually despawns. The card must both tuck toward the
        // anchor (x=1880) and be gone by the end.
        let base_top = reveal_top(&mut app).expect("card has a px top");
        let mut max_left = base_left;
        let mut min_top = base_top;
        let mut despawned = false;
        for _ in 0..60 {
            app.update();
            match (reveal_left(&mut app), reveal_top(&mut app)) {
                (Some(x), Some(y)) => {
                    max_left = max_left.max(x);
                    min_top = min_top.min(y);
                }
                _ => {
                    despawned = true;
                    break;
                }
            }
        }
        // The anchor is up-and-right of the base (x 960->1880, y 367->300), so
        // the card slides right AND up before despawning.
        assert!(
            max_left > base_left + 50.0,
            "the card slides right toward the tab anchor (base {base_left}, max {max_left})"
        );
        assert!(
            min_top < base_top - 20.0,
            "the card slides up toward the tab anchor (base {base_top}, min {min_top})"
        );
        assert!(despawned, "the reveal despawns after its lifetime");
    }

    /// Emptying GameObjectives (teardown) clears any in-flight reveal.
    #[test]
    fn scenario_teardown_clears_reveals() {
        let mut app = reveal_app();
        // Post an objective FIRST so the teardown-clear does not fire on the
        // empty-init frame (empty + resource-just-added reads as changed).
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Burn for Beacon 1")];
        app.update();

        app.world_mut().run_system_once(spawn_one).unwrap();
        app.update();
        assert_eq!(reveal_count(&mut app), 1);

        // A non-empty change does NOT clear the reveal.
        app.world_mut().resource_mut::<GameObjectives>().objectives =
            vec![Objective::new("b1", "Still going")];
        app.update();
        assert_eq!(reveal_count(&mut app), 1, "a live change keeps the reveal");

        // Emptying it (teardown) does.
        app.world_mut().resource_mut::<GameObjectives>().objectives = Vec::new();
        app.update();
        assert_eq!(reveal_count(&mut app), 0, "teardown clears the reveal");
    }

    /// Helper system: spawn one reveal through the production entry point.
    fn spawn_one(mut commands: Commands) {
        spawn_objective_reveal(&mut commands, &Objective::new("b1", "Burn for Beacon 1"));
    }
}
