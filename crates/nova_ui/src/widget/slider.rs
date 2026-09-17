//! Slider tracks: the well, the meter inside it, the system that follows a
//! `SliderValue`, and the reconciler that BUILDS a track's innards from the
//! active theme (a block-meter and a solid fill are structurally different
//! widgets, so a repaint cannot carry a theme change).

use bevy::{
    ecs::relationship::RelatedSpawner,
    platform::collections::HashSet,
    prelude::*,
    ui_widgets::{SliderRange, SliderValue},
};

use crate::theme::{ActiveUiTheme, ResolvedSlider, SliderMeter, BORDER_W};

/// Marks one segment of a block-meter track; the tuple is its index. The
/// value-sync system reads the index + the current fraction through
/// [`slider_meter_color`] to light the bar up to the value.
#[derive(Component)]
pub struct SliderBlock(pub usize);

/// The colour of block `index` for a slider at `fraction`: the theme's lit tone
/// below the value, its unlit tone above.
///
/// Only an empty slider reads empty and only a full one reads full: a plain
/// `round()` lit 24/24 at 98% and 0/24 at 2%, so the meter lied about the two
/// values a player is most likely to check, while a solid fill (a continuous
/// width) showed both correctly.
pub fn slider_meter_color(
    index: usize,
    fraction: f32,
    segments: usize,
    theme: &ResolvedSlider,
) -> Color {
    let fraction = fraction.clamp(0.0, 1.0);
    let count = segments as f32;
    let lit = if fraction <= 0.0 {
        0
    } else if fraction >= 1.0 {
        segments
    } else {
        (fraction * count).round().clamp(1.0, count - 1.0) as usize
    };
    if index < lit {
        theme.lit
    } else {
        theme.unlit
    }
}

/// Marks a solid-fill track's single fill child, so the value-sync system can
/// move it the way it lights the [`SliderBlock`]s. Without the marker the fill
/// kept whatever width it was spawned with and the bar never followed a drag.
#[derive(Component)]
pub struct SliderFill;

/// Marks a [`slider_track`], so the theme reconciler BUILDS it and rebuilds it
/// live. The track is not just repainted: a row of [`SliderBlock`]s and one
/// [`SliderFill`] are different widgets, so its children are respawned.
///
/// It carries the fraction a rebuilt DISPLAY-ONLY track must show - one spawned
/// straight from `slider_track(0.7)` with no slider attached, which the
/// signature invites, and which used to silently empty itself on a flip. It
/// lives HERE, on the surviving track, not on the children the rebuild throws
/// away (lesson `rebuilt-view-writes-go-to-state-not-the-entity`).
///
/// A track that HAS a `SliderValue` is read from that instead, so this stays a
/// cache and never becomes a second source of truth for a real slider.
#[derive(Component)]
pub struct SliderTrack(f32);

impl SliderTrack {
    /// The remembered fraction, `[0, 1]`. Only meaningful for a DISPLAY-ONLY
    /// track - one with no slider at all. A track that HAS a `SliderValue` is
    /// read from that instead, so this is a cache that may lag by a frame.
    ///
    /// A track whose fill re-themes but stays FROZEN at its spawn fraction is
    /// the symptom of a caller that spawned the visual on the wrong entity:
    /// `slider_track` goes ON the slider, not under it, because
    /// `sync_slider_tracks` reads `(&Children, &SliderValue, ..)` off one entity.
    fn seed(&self) -> f32 {
        self.0
    }
}

/// The track's own `Node` geometry under a theme - the single source the
/// reconciler builds from.
fn slider_track_node(paint: &ResolvedSlider) -> Node {
    Node {
        width: percent(100),
        height: px(paint.height),
        border: UiRect::all(px(BORDER_W)),
        border_radius: BorderRadius::all(px(paint.surface.radius)),
        align_items: AlignItems::Center,
        // NO padding, in any theme: `slider_track` goes ON the slider entity,
        // and bevy's drag math maps the pointer across that node's FULL width.
        // The block-meter track used to inset its meter by 2px a side, so the
        // lit edge sat ~3px away from the value the click actually committed.
        padding: UiRect::ZERO,
        column_gap: match paint.meter {
            SliderMeter::Blocks { gap, .. } => px(gap),
            SliderMeter::Fill => px(0),
        },
        ..default()
    }
}

/// Spawn the track's INNARDS for a theme: the block-meter, or the solid fill.
fn spawn_slider_track_children(
    parent: &mut RelatedSpawner<ChildOf>,
    fraction: f32,
    paint: &ResolvedSlider,
) {
    let fraction = fraction.clamp(0.0, 1.0);
    match paint.meter {
        SliderMeter::Blocks { segments, .. } => {
            for i in 0..segments {
                parent.spawn((
                    SliderBlock(i),
                    Node {
                        flex_grow: 1.0,
                        height: percent(100),
                        border_radius: BorderRadius::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(slider_meter_color(i, fraction, segments, paint)),
                ));
            }
        }
        SliderMeter::Fill => {
            parent.spawn((
                SliderFill,
                Node {
                    width: percent(fraction * 100.0),
                    height: percent(100.0),
                    border_radius: BorderRadius::all(px(paint.surface.radius)),
                    ..default()
                },
                BackgroundColor(paint.lit),
            ));
        }
    }
}

/// The audio Slider's track (the existing `bevy_ui_widgets::Slider` keeps its
/// value/range behaviour; this is the visual): a bordered well showing the
/// value either as a segmented BLOCK-METER or as one solid fill, whichever the
/// theme's `slider_track` role asks for. `fraction` in `[0, 1]`. Spawn ON the
/// slider entity.
///
/// Spawns EMPTY and unpainted: the reconciler below fills it on the frame it
/// appears and rebuilds it when the theme changes, so a track re-themes live
/// and its fill follows the value - neither is the caller's job.
pub fn slider_track(fraction: f32) -> impl Bundle {
    (
        SliderTrack(fraction.clamp(0.0, 1.0)),
        Node {
            width: percent(100),
            border: UiRect::all(px(BORDER_W)),
            align_items: AlignItems::Center,
            padding: UiRect::ZERO,
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    )
}

/// Where a slider's value sits in its range, as `[0, 1]`. A slider with no
/// `SliderRange` is read as an already-normalized value (the pre-range
/// behaviour), so an existing bare `SliderValue` keeps working.
fn slider_fraction(value: &SliderValue, range: Option<&SliderRange>) -> f32 {
    match range {
        Some(range) => range.thumb_position(value.0).clamp(0.0, 1.0),
        None => value.0.clamp(0.0, 1.0),
    }
}

/// Show the current value on any `Slider` wearing a [`slider_track`]: light the
/// [`SliderBlock`] meter, and move the [`SliderFill`]. ONE system owns "the
/// value changed -> the track shows it" for both meter shapes - the solid fill
/// used to have no marker at all, so a drag moved the number and not the bar.
///
/// Registered by [`build`](super::build), so a `bevy_ui_widgets::Slider` +
/// `slider_track` visual gets this for free (settings volume, the widget_zoo).
/// It also runs for a track the theme reconciler just rebuilt (via the
/// `Changed<Children>` arm), which keeps the rebuilt children honest even if
/// the reconciler's own read of the value ever goes stale.
pub(super) fn sync_slider_tracks(
    theme: Res<ActiveUiTheme>,
    mut changed: Query<
        (
            &Children,
            &SliderValue,
            Option<&SliderRange>,
            Option<&mut SliderTrack>,
        ),
        Or<(
            Changed<SliderValue>,
            Changed<SliderRange>,
            Changed<Children>,
        )>,
    >,
    mut blocks: Query<(&SliderBlock, &mut BackgroundColor)>,
    mut fills: Query<&mut Node, With<SliderFill>>,
) {
    let paint = theme.slider_track();
    let segments = match paint.meter {
        SliderMeter::Blocks { segments, .. } => segments,
        SliderMeter::Fill => 0,
    };
    for (kids, value, range, track) in &mut changed {
        let fraction = slider_fraction(value, range);
        // Remember it for the theme reconciler, which rebuilds these children.
        if let Some(mut track) = track {
            if track.0 != fraction {
                track.0 = fraction;
            }
        }
        for &child in kids {
            if let Ok((block, mut bg)) = blocks.get_mut(child) {
                *bg = slider_meter_color(block.0, fraction, segments, paint).into();
            }
            if let Ok(mut node) = fills.get_mut(child) {
                node.width = percent(fraction * 100.0);
            }
        }
    }
}

/// Build LIVE slider tracks on a theme change and on spawn: paint the track's
/// own well and geometry, and spawn its children for the theme (a segmented
/// block-meter and a solid fill are different widgets, so a colour swap cannot
/// carry it). The children are spawned AT the track's current value, so a
/// display-only track (one with no `SliderValue`, spawned straight from
/// `slider_track(fraction)`) survives a flip too; `sync_slider_tracks` runs
/// after this and refines a live slider's fill rather than being the only thing
/// that fills it.
///
/// Value state lives on the SLIDER, never on the rebuilt children, so a rebuild
/// cannot lose it (lesson `rebuilt-view-writes-go-to-state-not-the-entity`).
#[expect(
    clippy::type_complexity,
    reason = "the track's geometry, its paint, its value source and the just-added set"
)]
pub(super) fn reconcile_slider_track_themes(
    theme: Res<ActiveUiTheme>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &SliderTrack,
        Option<&SliderValue>,
        Option<&SliderRange>,
    )>,
    added: Query<Entity, Added<SliderTrack>>,
) {
    let rebuild_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !rebuild_all && just_added.is_empty() {
        return;
    }
    let paint = theme.slider_track().clone();
    for (entity, mut node, mut bgc, mut border_color, track, value, range) in &mut q {
        if !rebuild_all && !just_added.contains(&entity) {
            continue;
        }
        *node = slider_track_node(&paint);
        *bgc = paint.surface.fill.base.into();
        border_color.set_all(paint.surface.border);
        // Prefer the live value: the seed is a CACHE for the display-only case,
        // and a cache read here could be a frame stale (a theme flip and a value
        // change landing together). A track with no `SliderValue` has only the
        // seed, which is the whole reason it exists.
        let fraction = value.map_or(track.seed(), |value| slider_fraction(value, range));
        commands
            .entity(entity)
            .queue_silenced(rebuild_slider_track_children(fraction, paint.clone()));
    }
}

/// The track rebuild, as ONE entity command.
///
/// It is one command, and a SILENCED one, on purpose. A caller may despawn a
/// track's subtree on the same theme change (the widget zoo rebuilds its whole
/// body), and whether that lands before or after this is a registration-order
/// accident when no ordering edge forces a flush between them - so neither half
/// may panic on an already-dead entity. Splitting it as
/// `despawn_related().try_insert(..)` does NOT achieve that: `try_insert` is
/// silenced but `despawn_related` still queues through the default handler,
/// which the game escalates to a panic under `NOVA_AUTOPILOT` (see
/// examples/systems/system_menu_boot.rs). Verified directly: queueing
/// `despawn_related::<Children>()` at a despawned entity under
/// `FallbackErrorHandler(panic)` panics in `bevy_ecs`'s error handler.
///
/// Pinned by `rebuild_survives_a_track_despawned_in_the_same_frame`, which makes
/// the ordering accident deterministic by turning the schedule's
/// `auto_insert_apply_deferred` OFF - without that, the `.before` edge inserts a
/// flush, the reconciler stops matching the despawned entity and queues nothing,
/// and the race cannot happen at all. Keep this ONE `queue_silenced` command:
/// splitting it back out turns that test into a panic.
fn rebuild_slider_track_children(fraction: f32, paint: ResolvedSlider) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.despawn_related::<Children>();
        entity.insert(Children::spawn(SpawnWith(
            move |parent: &mut RelatedSpawner<ChildOf>| {
                spawn_slider_track_children(parent, fraction, &paint);
            },
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        theme::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        widget::fixtures::{hardware, phosphor, select, themed_app},
    };

    fn block_colors(app: &mut App, slider: Entity) -> Vec<Color> {
        let kids: Vec<Entity> = app
            .world()
            .entity(slider)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app
            .world_mut()
            .query_filtered::<&BackgroundColor, With<SliderBlock>>();
        kids.into_iter()
            .filter_map(|c| q.get(app.world(), c).ok().map(|bg| bg.0))
            .collect()
    }

    /// How many blocks the phosphor meter draws, read from the theme rather
    /// than restated here.
    fn phosphor_segments() -> usize {
        match phosphor().slider_track().meter {
            SliderMeter::Blocks { segments, .. } => segments,
            SliderMeter::Fill => panic!("base phosphor shows a block meter"),
        }
    }

    /// Spawn a slider the way the settings row does: the `Slider` entity IS the
    /// track, carrying its value, range and the `slider_track` visual.
    fn spawn_slider(app: &mut App, value: f32) -> Entity {
        app.world_mut()
            .spawn((
                SliderValue(value),
                SliderRange::new(0.0, 1.0),
                slider_track(value),
            ))
            .id()
    }

    /// A solid-fill track's single fill child, by width.
    fn fill_width(app: &mut App, slider: Entity) -> Option<Val> {
        let kids: Vec<Entity> = app
            .world()
            .entity(slider)
            .get::<Children>()
            .unwrap()
            .iter()
            .collect();
        let mut q = app.world_mut().query_filtered::<&Node, With<SliderFill>>();
        kids.into_iter()
            .find_map(|c| q.get(app.world(), c).ok().map(|node| node.width))
    }

    /// `sync_slider_tracks` lights a slider's block-meter from its
    /// `SliderValue` - so a `Slider` wearing `slider_track` reacts to drags with
    /// no per-site code. Fails if the system is unregistered or the block
    /// recolour is wrong.
    #[test]
    fn sync_slider_tracks_lights_blocks_from_value() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let slider = app
            .world_mut()
            .spawn((SliderValue(0.0), slider_track(0.0)))
            .id();
        app.update();
        let unlit = phosphor().slider_track().unlit;
        let lit = phosphor().slider_track().lit;
        assert!(
            block_colors(&mut app, slider).iter().all(|c| *c == unlit),
            "every bar is dim at value 0"
        );

        // `SliderValue` is immutable, so it is re-inserted (as the real widget
        // does), which fires `Changed` and lights the bars.
        app.world_mut().entity_mut(slider).insert(SliderValue(1.0));
        app.update();
        assert!(
            block_colors(&mut app, slider).iter().all(|c| *c == lit),
            "every bar is lit at value 1.0"
        );
    }

    /// The clamp that makes the meter stop lying at the two ends. A plain
    /// `round()` lit 0 blocks at 2% and all 24 at 98%, so only the endpoints
    /// below distinguish the fix - the 0.0/1.0 cases pass either way and are
    /// kept as the delivery guard that the un-clamped path is still exact.
    #[test]
    fn the_meter_reserves_a_block_at_each_end() {
        let theme = phosphor();
        let paint = theme.slider_track();
        let segments = phosphor_segments();

        assert_eq!(
            slider_meter_color(0, 0.02, segments, paint),
            paint.lit,
            "a non-empty slider lights at least one block"
        );
        assert_eq!(
            slider_meter_color(segments - 1, 0.98, segments, paint),
            paint.unlit,
            "a non-full slider leaves at least one block dark"
        );

        assert_eq!(
            slider_meter_color(0, 0.0, segments, paint),
            paint.unlit,
            "an EMPTY slider lights nothing"
        );
        assert_eq!(
            slider_meter_color(segments - 1, 1.0, segments, paint),
            paint.lit,
            "a FULL slider lights everything"
        );
    }

    /// A solid-fill track follows the value, like the block-meter does. Owner
    /// playtest 2026-07-29: "the hardware variant doesn't move the slider, it
    /// stays fixed, but the value changes correctly" - the fill child was
    /// unmarked, so the value-sync system could not see it.
    #[test]
    fn a_solid_fill_track_follows_the_value() {
        let mut app = themed_app(HARDWARE_THEME_ID);
        let slider = spawn_slider(&mut app, 0.25);
        app.update();
        assert_eq!(
            fill_width(&mut app, slider),
            Some(percent(25.0)),
            "the fill starts at its spawned fraction"
        );

        app.world_mut().entity_mut(slider).insert(SliderValue(0.8));
        app.update();
        assert_eq!(
            fill_width(&mut app, slider),
            Some(percent(80.0)),
            "the fill moves with the dragged value"
        );
    }

    /// The track REBUILDS live on a theme change. The two base themes ask for
    /// different WIDGETS inside (a segmented block-meter vs one solid fill), so
    /// the reconciler respawns the children, not just the colours - and the
    /// rebuilt track must show the CURRENT value, not a default.
    #[test]
    fn slider_track_rebuilds_and_keeps_its_value() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let slider = spawn_slider(&mut app, 0.5);
        app.update();
        assert_eq!(
            block_colors(&mut app, slider).len(),
            phosphor_segments(),
            "phosphor shows the segmented block-meter"
        );
        assert_eq!(fill_width(&mut app, slider), None, "and no solid fill");

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert!(
            block_colors(&mut app, slider).is_empty(),
            "the block-meter is gone on the hardware theme"
        );
        assert_eq!(
            fill_width(&mut app, slider),
            Some(percent(50.0)),
            "the rebuilt hardware fill carries the CURRENT value, not a default"
        );
        assert_eq!(
            app.world().entity(slider).get::<Node>().unwrap().height,
            px(hardware().slider_track().height),
            "the track geometry re-themed too, not just its children"
        );

        select(&mut app, PHOSPHOR_THEME_ID);
        app.update();
        let colors = block_colors(&mut app, slider);
        let segments = phosphor_segments();
        assert_eq!(colors.len(), segments, "and back: the block-meter returns");
        let lit = phosphor().slider_track().lit;
        assert!(
            colors.iter().take(segments / 2).all(|c| *c == lit),
            "lit up to the current half-value, not dark"
        );
    }

    /// A DISPLAY-ONLY track - one spawned straight from
    /// `slider_track(fraction)` with no `SliderValue`, which the signature
    /// invites - must survive a theme flip too. The reconciler reads the
    /// fraction itself rather than leaning on `sync_slider_tracks`, which cannot
    /// see a track that has no value to sync from; otherwise such a track
    /// silently emptied itself on the flip.
    #[test]
    fn a_display_only_track_keeps_its_fraction_across_a_flip() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let track = app.world_mut().spawn(slider_track(0.75)).id();
        app.update();

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(
            fill_width(&mut app, track),
            Some(percent(75.0)),
            "the rebuilt fill still shows the fraction it was spawned with"
        );
    }

    /// `sync_slider_tracks` derives its fraction from
    /// `SliderRange::thumb_position`, so it must WATCH the range, not just the
    /// value. With the range missing from the run filter a widened range left
    /// the fill where it was - the value had not changed, but what it MEANS had.
    #[test]
    fn a_changed_range_moves_the_fill_even_at_a_steady_value() {
        let mut app = themed_app(HARDWARE_THEME_ID);
        let slider = app
            .world_mut()
            .spawn((
                SliderValue(5.0),
                SliderRange::new(0.0, 10.0),
                slider_track(0.5),
            ))
            .id();
        app.update();
        assert_eq!(fill_width(&mut app, slider), Some(percent(50.0)));

        // Same value, twice the range - the fill must halve.
        app.world_mut()
            .entity_mut(slider)
            .insert(SliderRange::new(0.0, 20.0));
        app.update();
        assert_eq!(
            fill_width(&mut app, slider),
            Some(percent(25.0)),
            "the fill follows the range, not just the value"
        );
    }

    /// The track rebuild must be ONE silenced command, so a track despawned in
    /// the same frame - after the reconciler queued its rebuild, before that
    /// rebuild applies - no-ops instead of panicking. A
    /// `despawn_related().try_insert(..)` split silences only the SECOND half
    /// and panics here.
    ///
    /// `auto_insert_apply_deferred: false` is what makes the race deterministic:
    /// with the default ON, the `.before` edge inserts a flush, the despawn lands
    /// first, and the reconciler no longer matches the entity at all - so the
    /// bug cannot reproduce. The real race has no ordering edge between the two
    /// systems (the widget zoo's `rebuild_body` vs this reconciler); which
    /// applies first is a system-index accident.
    #[test]
    fn rebuild_survives_a_track_despawned_in_the_same_frame() {
        use bevy::ecs::{
            error::{panic, FallbackErrorHandler},
            schedule::ScheduleBuildSettings,
        };

        fn despawn_all_tracks(mut commands: Commands, q: Query<Entity, With<SliderTrack>>) {
            for entity in &q {
                commands.entity(entity).despawn();
            }
        }

        let mut app = themed_app(PHOSPHOR_THEME_ID);
        // The handler the game installs under `NOVA_AUTOPILOT`, which is how
        // every scripted run works - the configuration this failure mode bites in.
        app.insert_resource(FallbackErrorHandler(panic));
        app.edit_schedule(Update, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                auto_insert_apply_deferred: false,
                ..default()
            });
        });
        app.add_systems(
            Update,
            despawn_all_tracks.before(reconcile_slider_track_themes),
        );

        app.world_mut().spawn((SliderValue(0.5), slider_track(0.5)));
        app.update();

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
    }
}
