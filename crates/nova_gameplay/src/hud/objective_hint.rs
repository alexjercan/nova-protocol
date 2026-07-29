//! The minimalist flight objective HINT (task 20260724-134312): the
//! ACTIVE-objective COUNT + a "TAB" affordance (owner choice - the per-objective
//! detail lives in the diegetic reveal and the Tab NOVA OS, not here). It
//! collapses when there are no objectives.
//!
//! The hint is a BLOCK IN THE STATUS BAR (task 20260724-161545): it is parented
//! into the bcs status-bar row (the fps/version bar) so it flows beside those
//! items and can never overlap the version, instead of floating as its own
//! top-right node (which collided with the version). It renders flat (no
//! bordered pill) to match the other bar items, leading with the NOVA CRT star
//! mark icon (the drawer's brand mark - it ties the TAB affordance to the NOVA
//! OS computer it opens) followed by the count + "TAB". It is parented as our
//! OWN child of `StatusBarRootMarker` rather than a `status_bar_item`, because
//! it needs its own node markers for the behaviours below (the registry's
//! auto-insert builds an unmarked text-only visual).
//!
//! It is also the source of the diegetic reveal's tuck anchor: the hint writes
//! [`NovaOsTabAnchor`] from its own screen rect (the reveal, task 20260721-211520,
//! tucks a newly posted objective INTO this hint), replacing the old NOVA OS tab
//! handle as the tuck target.

use bevy::prelude::*;
use bevy_common_systems::prelude::{GameObjectives, StatusBarRootMarker};
use nova_ui::theme;

use super::{emphasis::prelude::*, nova_os::NovaOsTabAnchor, NovaHudAssets, NovaHudSystems};
use crate::prelude::*;

const HINT_FONT_PX: f32 = 14.0;
const HINT_CHIP_FONT_PX: f32 = 11.0;
/// The leading star mark icon size - matched to the count's cap height so it
/// sits flush in the status-bar row.
const HINT_ICON_PX: f32 = 15.0;

/// The TAB keycap's on-screen size (px) in the top bar - a touch taller than
/// the brand mark so the key reads as a key, still inside the 24 px bar.
const HINT_TAB_GLYPH_PX: f32 = 18.0;

/// The posting POP: peak scale and duration when a reveal card tucks into the
/// hint - demo 2's `.obj.emph` (1.16) held for 1.2 s.
const HINT_POP_SCALE: f32 = 1.16;
const HINT_POP_SECS: f32 = 1.2;

/// The slow BREATH the hint settles into while objectives are outstanding -
/// demo 2's `@keyframes breathe` (2.4 s, down to 0.72 alpha). Quiet enough to
/// ignore, alive enough to say "there is still work".
const HINT_BREATH_PERIOD_SECS: f32 = 2.4;
const HINT_BREATH_MIN_ALPHA: f32 = 0.72;
/// Nominal hint size for the reveal's tuck rect (the exact width flexes with the
/// count; the CENTRE is what the tuck aims at). Task 20260721-211520's target.
const HINT_ANCHOR_SIZE: Vec2 = Vec2::new(120.0, 28.0);

/// The hint block in the status bar (star mark icon + count + TAB). Also the
/// tuck anchor source for the diegetic reveal.
#[derive(Component)]
struct ObjectiveHintMarker;

/// The count text inside the hint.
#[derive(Component)]
struct ObjectiveHintCountMarker;

/// A hint part that takes the slow breath, carrying the alpha it renders at
/// rest. The parts do not share a base alpha (gold text at 1.0, the muted TAB
/// fallback below it), and bevy_ui has no node-level opacity, so the breath is
/// applied per part as `base * wave` - absolute, never multiplied onto the
/// previous frame's value, so it cannot drift.
#[derive(Component)]
struct ObjectiveHintBreath {
    base_alpha: f32,
}

/// The leading NOVA CRT star mark icon inside the hint (ties the TAB affordance
/// to the NOVA OS computer it opens).
#[derive(Component)]
struct ObjectiveHintIconMarker;

/// Marker for the TAB keycap picture in the hint (absent on the text fallback).
#[derive(Component, Debug, Clone, Reflect)]
struct ObjectiveHintTabGlyphMarker;

/// Wires the flight objective hint: spawn/despawn with the player ship, keep the
/// count current, and publish the reveal's tuck anchor.
pub struct ObjectiveHintPlugin;

impl Plugin for ObjectiveHintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_hint,
                update_tab_anchor,
                pop_hint_on_reveal_tuck,
                // The breath asks whether the pop is still playing, so it must
                // read the same frame's pop, not the previous one's.
                breathe_hint.after(pop_hint_on_reveal_tuck),
            )
                .in_set(NovaHudSystems),
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
/// and the NOVA OS hide) is INHERITED from the `Status`-tier bar root - the hint
/// carries no `HudTier` of its own.
fn setup_hint(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_bar: Query<Entity, With<StatusBarRootMarker>>,
    hud_assets: Option<Res<NovaHudAssets>>,
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
            // The posting pop (task 20260728-175747). It fires when the reveal
            // card finishes tucking in here, not when the objective posts, so
            // the card's arrival and this reaction are one motion.
            HudEmphasis::settle(HINT_POP_SCALE),
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
            // Leading NOVA CRT star mark (the same brand mark the drawer plate
            // uses), so the top-bar TAB affordance visually reads as "the NOVA OS
            // computer". Native colours (like the plate); rendered from a PNG
            // because Bevy UI cannot draw SVG. Guarded on NovaHudAssets so
            // headless rigs without the asset pipeline still spawn the count + TAB.
            if let Some(hud_assets) = hud_assets.as_deref() {
                hint.spawn((
                    ObjectiveHintIconMarker,
                    ImageNode::new(hud_assets.nova_crt_mark.clone()),
                    ObjectiveHintBreath { base_alpha: 1.0 },
                    Node {
                        width: Val::Px(HINT_ICON_PX),
                        height: Val::Px(HINT_ICON_PX),
                        ..default()
                    },
                ));
            }
            // Active-objective count (gold, the objective accent).
            hint.spawn((
                ObjectiveHintCountMarker,
                Text::new("0"),
                TextFont::from_font_size(HINT_FONT_PX),
                TextColor(theme::semantic::OBJECTIVE),
                ObjectiveHintBreath {
                    base_alpha: theme::semantic::OBJECTIVE.alpha(),
                },
            ));
            // The TAB affordance: the real keycap picture (task
            // 20260728-175742's key-glyph pipeline), so the top bar says
            // "press this key" in the same language as the dock and the
            // anchored cues. Falls back to the plain muted word when the
            // glyph is not loaded (headless rigs, an unmapped rebind) - the
            // affordance must never vanish.
            match hud_assets
                .as_deref()
                .and_then(|assets| assets.key_glyphs.get("Tab"))
            {
                Some(glyph) => {
                    hint.spawn((
                        ObjectiveHintTabGlyphMarker,
                        ImageNode::new(glyph),
                        ObjectiveHintBreath { base_alpha: 1.0 },
                        Node {
                            width: Val::Px(HINT_TAB_GLYPH_PX),
                            height: Val::Px(HINT_TAB_GLYPH_PX),
                            ..default()
                        },
                    ));
                }
                None => {
                    hint.spawn((
                        Text::new("TAB"),
                        TextFont::from_font_size(HINT_CHIP_FONT_PX),
                        TextColor(theme::PHOSPHOR_DIM),
                        ObjectiveHintBreath {
                            base_alpha: theme::PHOSPHOR_DIM.alpha(),
                        },
                    ));
                }
            }
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
/// cycle and the NOVA OS hide still work - they act on the `Status`-tier bar root,
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

/// Pop the hint when a reveal card lands in it (task 20260728-175747). The
/// message is the handover point of the posting animation, so the pop cannot
/// drift out of step with the card - and a card cleared early by scenario
/// teardown never sends it, so a teardown does not pop a hint that is about to
/// collapse.
fn pop_hint_on_reveal_tuck(
    mut tucked: MessageReader<super::objective_reveal::ObjectiveRevealTucked>,
    mut q_hint: Query<&mut HudEmphasis, With<ObjectiveHintMarker>>,
) {
    if tucked.read().count() == 0 {
        return;
    }
    for mut emphasis in &mut q_hint {
        emphasis.pop(HINT_POP_SECS);
    }
}

/// The slow breath while objectives are outstanding: once the pop has settled,
/// the hint keeps a quiet ~2.4 s alpha breath so "there is work" stays present
/// without a second animation competing with the pop. With no objectives (the
/// hint is collapsed anyway) everything returns to its rest alpha, so nothing
/// can be left mid-breath.
fn breathe_hint(
    time: Res<Time>,
    objectives: Res<GameObjectives>,
    q_hint: Query<&HudEmphasis, With<ObjectiveHintMarker>>,
    mut q_parts: Query<(
        &ObjectiveHintBreath,
        Option<&mut TextColor>,
        Option<&mut ImageNode>,
    )>,
) {
    // While the pop is playing, the pop IS the emphasis - the breath resumes
    // when it settles (demo 2: `.obj:not(.emph)` is what breathes).
    let popping = q_hint.iter().any(|emphasis| emphasis.popping());
    let wave = if objectives.objectives.is_empty() || popping {
        1.0
    } else {
        let phase = time.elapsed_secs() * std::f32::consts::TAU / HINT_BREATH_PERIOD_SECS;
        let up = 0.5 + 0.5 * phase.sin();
        HINT_BREATH_MIN_ALPHA + (1.0 - HINT_BREATH_MIN_ALPHA) * up
    };

    for (breath, text, image) in &mut q_parts {
        let alpha = breath.base_alpha * wave;
        if let Some(mut text) = text {
            text.0.set_alpha(alpha);
        }
        if let Some(mut image) = image {
            image.color.set_alpha(alpha);
        }
    }
}

/// Publish the hint's screen rect as [`NovaOsTabAnchor`] - the diegetic reveal's
/// tuck target (task 20260721-211520). Mirrors the old NOVA OS-tab-handle anchor;
/// a fixed nominal size keeps the math unit-testable and the tuck aims at the
/// hint's centre.
fn update_tab_anchor(
    q_hint: Query<&GlobalTransform, With<ObjectiveHintMarker>>,
    mut anchor: ResMut<NovaOsTabAnchor>,
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
        app.init_resource::<NovaOsTabAnchor>();
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

    /// With an AssetServer present the hint leads with the NOVA CRT star mark
    /// icon (the same brand mark the drawer plate uses), parented inside the hint
    /// item beside the count + TAB.
    #[test]
    fn objective_hint_shows_the_nova_crt_star_icon() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // AssetPlugin gives the AssetServer; the Image asset type must be
        // registered or `load` of the PNG panics (mirrors the nova_os rigs).
        app.add_plugins(AssetPlugin::default());
        app.init_asset::<Image>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<NovaOsTabAnchor>();
        // The brand mark and the TAB keycap come from NovaHudAssets, which
        // `NovaHudPlugin` inits in production; this rig runs the observer alone,
        // so it must supply the resource itself (without it the hint takes its
        // no-assets fallback path and spawns neither picture).
        let server = app.world().resource::<AssetServer>().clone();
        app.insert_resource(NovaHudAssets {
            nova_crt_mark: server.load("icons/nova_crt_mark.png"),
            key_glyphs: crate::hud::key_glyphs::KeyGlyphs::from_stems(|stem| {
                Some(server.load(format!(
                    "{}/{stem}.png",
                    crate::hud::key_glyphs::KEY_GLYPH_DIR
                )))
            }),
            ..default()
        });
        app.add_observer(setup_hint);
        app.world_mut().spawn(StatusBarRootMarker);
        app.world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker));
        app.update();

        // The TAB affordance is the real keycap picture now (task
        // 20260728-175742), not the word.
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ObjectiveHintTabGlyphMarker>>()
                .iter(app.world())
                .count(),
            1,
            "the hint shows the TAB keycap"
        );

        // Exactly one star icon, and it is an ImageNode parented under the hint.
        let (icon, parent) = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf), (With<ObjectiveHintIconMarker>, With<ImageNode>)>(
            )
            .single(app.world())
            .map(|(icon, child_of)| (icon, child_of.0))
            .expect("the hint has exactly one star-mark ImageNode");
        let hint = app
            .world_mut()
            .query_filtered::<Entity, With<ObjectiveHintMarker>>()
            .single(app.world())
            .expect("the hint spawned");
        assert_eq!(parent, hint, "the star icon is a child of the hint item");
        // It leads the row (spawned before the count + TAB).
        let first_child = app
            .world()
            .entity(hint)
            .get::<Children>()
            .expect("the hint has children")[0];
        assert_eq!(first_child, icon, "the star icon leads the count + TAB");
    }

    /// The hint is the reveal's tuck anchor: `update_tab_anchor` publishes the
    /// hint's screen rect into `NovaOsTabAnchor`. Deleting the system leaves it
    /// `None`.
    #[test]
    fn objective_hint_provides_the_nova_os_anchor() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<NovaOsTabAnchor>();
        app.add_systems(Update, update_tab_anchor);
        app.world_mut().spawn((
            ObjectiveHintMarker,
            GlobalTransform::from_translation(Vec3::new(1850.0, 24.0, 0.0)),
        ));
        app.update();

        let rect = app
            .world()
            .resource::<NovaOsTabAnchor>()
            .rect
            .expect("the anchor is published from the hint");
        assert!(
            (rect.center() - Vec2::new(1850.0, 24.0)).length() < 0.01,
            "the anchor centres on the hint (top-right, so the reveal tucks up-right)"
        );
    }

    // -- the posting pop + breath (task 20260728-175747) --

    /// An App-driven pop: the reveal card's tuck pops the hint, the pop reaches
    /// full scale, and it settles back to rest on its own as virtual time
    /// advances. Drives the real message rather than calling `pop` directly, so
    /// the HANDOVER is what is pinned.
    #[test]
    fn the_hint_pops_when_the_reveal_tucks_in_and_settles_back() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = hint_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.1,
        )));
        app.add_message::<super::super::objective_reveal::ObjectiveRevealTucked>();
        app.add_systems(
            Update,
            (
                pop_hint_on_reveal_tuck,
                super::super::emphasis::drive_hud_emphasis.after(pop_hint_on_reveal_tuck),
            ),
        );

        let scale = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&UiTransform, With<ObjectiveHintMarker>>()
                .single(app.world())
                .expect("the hint spawned")
                .scale
                .x
        };

        app.update();
        assert_eq!(scale(&mut app), 1.0, "the hint rests before any posting");

        app.world_mut()
            .write_message(super::super::objective_reveal::ObjectiveRevealTucked);
        // 0.3s: past the 0.2s ease-in, well inside the 1.2s pop.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            scale(&mut app),
            HINT_POP_SCALE,
            "the card landing pops the hint to full"
        );

        // Past the pop plus its ease-out.
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(
            scale(&mut app),
            1.0,
            "and the pop settles by itself - no second trigger needed"
        );
    }

    /// The breath is the RESTING state while work is outstanding, and it stops
    /// dead when there is none: an objective-less hint (which is collapsed
    /// anyway) is never left mid-breath.
    #[test]
    fn the_hint_breathes_only_while_objectives_are_outstanding() {
        use core::time::Duration;

        use bevy::time::TimeUpdateStrategy;

        let mut app = hint_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            0.3,
        )));
        app.add_systems(Update, breathe_hint);

        let count_alpha = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&TextColor, With<ObjectiveHintCountMarker>>()
                .single(app.world())
                .expect("the count spawned")
                .0
                .alpha()
        };

        // The count's rest alpha is the theme's, not necessarily 1.0 - the
        // breath is a FRACTION of whatever the part renders at rest.
        let rest = theme::semantic::OBJECTIVE.alpha();

        // No objectives: rest alpha, every frame.
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            count_alpha(&mut app),
            rest,
            "an objective-less hint does not breathe"
        );

        app.world_mut()
            .resource_mut::<GameObjectives>()
            .objectives
            .push(Objective::new("survey", "Survey the wreck"));
        let mut seen_dimmed = false;
        let mut seen_bright = false;
        for _ in 0..16 {
            app.update();
            let alpha = count_alpha(&mut app);
            assert!(
                (rest * HINT_BREATH_MIN_ALPHA..=rest).contains(&alpha),
                "the breath stays inside its band (alpha {alpha})"
            );
            seen_dimmed |= alpha < rest * 0.95;
            seen_bright |= alpha > rest * 0.98;
        }
        assert!(
            seen_dimmed && seen_bright,
            "an outstanding objective breathes both ways"
        );
    }
}
