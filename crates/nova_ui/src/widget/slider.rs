//! Slider tracks: the phosphor block-meter and the hardware solid fill, the
//! system that follows a `SliderValue`, and the reconciler that REBUILDS a
//! track when the skin flips (the two skins are structurally different
//! widgets, so a repaint cannot carry it).

use bevy::{
    ecs::relationship::RelatedSpawner,
    prelude::*,
    ui_widgets::{SliderRange, SliderValue},
};

use crate::{skin::UiSkin, theme};

/// Number of segments in a phosphor slider's block-meter.
pub const SLIDER_SEGMENTS: usize = 24;

/// Marks one segment of a phosphor slider block-meter; the tuple is its index.
/// A driver (e.g. a slider's value-change system) reads the index + the current
/// fraction through [`slider_meter_color`] to light the bar up to the value.
#[derive(Component)]
pub struct SliderBlock(pub usize);

/// The colour of block `index` for a slider at `fraction`: full phosphor if the
/// bar is lit (below the value), dim phosphor otherwise.
///
/// Only an empty slider reads empty and only a full one reads full: a plain
/// `round()` lit 24/24 at 98% and 0/24 at 2%, so the meter lied about the two
/// values a player is most likely to check, while the hardware fill (a
/// continuous width) showed both correctly.
pub fn slider_meter_color(index: usize, fraction: f32) -> Color {
    let fraction = fraction.clamp(0.0, 1.0);
    let segments = SLIDER_SEGMENTS as f32;
    let lit = if fraction <= 0.0 {
        0
    } else if fraction >= 1.0 {
        SLIDER_SEGMENTS
    } else {
        (fraction * segments).round().clamp(1.0, segments - 1.0) as usize
    };
    if index < lit {
        theme::PHOSPHOR
    } else {
        theme::PHOSPHOR.with_alpha(0.16)
    }
}

/// Track height in the phosphor skin (the segmented block-meter).
const SLIDER_TRACK_PHOSPHOR_H: f32 = 14.0;
/// Track height in the hardware skin (the solid moulded fill).
const SLIDER_TRACK_HARDWARE_H: f32 = 10.0;

/// Marks the HARDWARE track's single solid fill child, so the value-sync system
/// can move it the way it lights the phosphor [`SliderBlock`]s. Without the
/// marker the fill kept whatever width it was spawned with and the bar never
/// followed a drag.
#[derive(Component)]
pub struct SliderFill;

/// Marks a [`slider_track`] so the skin reconciler REBUILDS it live when the
/// `UiSkin` resource flips (mirrors [`crate::widget::PanelSkin`]). The track is not just
/// repainted: the two skins are structurally different widgets (a row of
/// [`SliderBlock`]s vs one [`SliderFill`]), so its children are respawned.
///
/// It carries the fraction a rebuilt DISPLAY-ONLY track must show - one spawned
/// straight from `slider_track(0.7, skin)` with no slider attached, which the
/// signature invites, and which used to silently empty itself on a flip. It
/// lives HERE, on the surviving track, not on the children the rebuild throws
/// away (lesson `rebuilt-view-writes-go-to-state-not-the-entity`).
///
/// A track that HAS a `SliderValue` is read from that instead, so this stays a
/// cache and never becomes a second source of truth for a real slider.
#[derive(Component)]
pub struct SliderTrackSkin(f32);

impl SliderTrackSkin {
    /// The remembered fraction, `[0, 1]`. Only meaningful for a DISPLAY-ONLY
    /// track - one with no slider at all. A track that HAS a `SliderValue` is
    /// read from that instead, so this is a cache that may lag by a frame.
    ///
    /// A track whose fill re-skins but stays FROZEN at its spawn fraction is the
    /// symptom of a caller that spawned the visual on the wrong entity:
    /// `slider_track` goes ON the slider, not under it, because
    /// `sync_slider_tracks` reads `(&Children, &SliderValue, ..)` off one entity.
    fn seed(&self) -> f32 {
        self.0
    }
}

/// The track's own `Node` geometry in a given skin - the single source both
/// [`slider_track`] and the skin reconciler build from.
fn slider_track_node(skin: UiSkin) -> Node {
    let phosphor = skin.is_phosphor();
    Node {
        width: percent(100),
        height: px(if phosphor {
            SLIDER_TRACK_PHOSPHOR_H
        } else {
            SLIDER_TRACK_HARDWARE_H
        }),
        border: UiRect::all(px(theme::BORDER_W)),
        border_radius: BorderRadius::all(px(slider_track_radius(skin))),
        align_items: AlignItems::Center,
        // NO padding, in either skin: `slider_track` goes ON the slider entity,
        // and bevy's drag math maps the pointer across that node's FULL width.
        // The phosphor track used to inset its meter by 2px a side, so the lit
        // edge sat ~3px away from the value the click actually committed.
        padding: UiRect::ZERO,
        column_gap: if phosphor { px(2) } else { px(0) },
        ..default()
    }
}

/// The track's corner radius in a given skin (also used by the hardware fill,
/// so the fill's leading edge matches the track it sits in).
fn slider_track_radius(skin: UiSkin) -> f32 {
    if skin.is_phosphor() {
        theme::RADIUS
    } else {
        6.0
    }
}

/// The track's `(background, border)` in a given skin.
fn slider_track_colors(skin: UiSkin) -> (Color, Color) {
    if skin.is_phosphor() {
        (
            Color::srgba(0.0, 0.0, 0.0, 0.5),
            theme::PHOSPHOR.with_alpha(0.32),
        )
    } else {
        (Color::srgba(0.0, 0.0, 0.0, 0.55), theme::CASE_EDGE)
    }
}

/// Spawn the track's INNARDS for a skin: the phosphor block-meter, or the
/// hardware solid fill. Shared by [`slider_track`] and the skin reconciler, so a
/// rebuilt track is identical to a freshly spawned one.
fn spawn_slider_track_children(parent: &mut RelatedSpawner<ChildOf>, fraction: f32, skin: UiSkin) {
    let fraction = fraction.clamp(0.0, 1.0);
    if skin.is_phosphor() {
        for i in 0..SLIDER_SEGMENTS {
            parent.spawn((
                SliderBlock(i),
                Node {
                    flex_grow: 1.0,
                    height: percent(100),
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(slider_meter_color(i, fraction)),
            ));
        }
    } else {
        parent.spawn((
            SliderFill,
            Node {
                width: percent(fraction * 100.0),
                height: percent(100.0),
                border_radius: BorderRadius::all(px(slider_track_radius(skin))),
                ..default()
            },
            BackgroundColor(theme::PHOSPHOR_DIM),
        ));
    }
}

/// A re-skin of the audio Slider's track (the existing `bevy_ui_widgets::Slider`
/// keeps its value/range behaviour; this is the visual). Phosphor: a bordered
/// track filled with a segmented BLOCK-METER (a row of [`SliderBlock`] bars lit
/// up to `fraction`, no knob), matching the demo's dotted fill. Hardware: a
/// solid [`SliderFill`]. `fraction` in `[0, 1]`. Spawn ON the slider entity.
///
/// The track carries [`SliderTrackSkin`], so it re-skins live, and its fill
/// follows the value in BOTH skins - neither is the caller's job.
pub fn slider_track(fraction: f32, skin: UiSkin) -> impl Bundle {
    let (track_bg, track_border) = slider_track_colors(skin);
    (
        slider_track_node(skin),
        SliderTrackSkin(fraction.clamp(0.0, 1.0)),
        BorderColor::all(track_border),
        BackgroundColor(track_bg),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            spawn_slider_track_children(parent, fraction, skin);
        })),
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
/// phosphor [`SliderBlock`] meter, and move the hardware [`SliderFill`]. ONE
/// system owns "the value changed -> the track shows it" for both skins - the
/// hardware fill used to have no marker at all, so a drag moved the number and
/// not the bar.
///
/// Registered by [`register`], so a `bevy_ui_widgets::Slider` + `slider_track`
/// visual gets this for free (settings volume, the widget_zoo). It also runs for
/// a track the skin reconciler just rebuilt (via the `Changed<Children>` arm),
/// which keeps the rebuilt children honest even if the reconciler's own read of
/// the value ever goes stale.
pub(super) fn sync_slider_tracks(
    mut changed: Query<
        (
            &Children,
            &SliderValue,
            Option<&SliderRange>,
            Option<&mut SliderTrackSkin>,
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
    for (kids, value, range, track) in &mut changed {
        let fraction = slider_fraction(value, range);
        // Remember it for the skin reconciler, which rebuilds these children.
        if let Some(mut track) = track {
            if track.0 != fraction {
                track.0 = fraction;
            }
        }
        for &child in kids {
            if let Ok((block, mut bg)) = blocks.get_mut(child) {
                *bg = slider_meter_color(block.0, fraction).into();
            }
            if let Ok(mut node) = fills.get_mut(child) {
                node.width = percent(fraction * 100.0);
            }
        }
    }
}

/// Rebuild LIVE slider tracks on a `UiSkin` change: repaint the track's own face
/// and geometry, and respawn its children for the new skin (a segmented
/// block-meter and a solid fill are different widgets, so a colour swap cannot
/// carry it). The rebuilt children are spawned AT the track's current value, so
/// a display-only track (one with no `SliderValue`, spawned straight from
/// `slider_track(fraction, skin)`) survives a flip too; `sync_slider_tracks`
/// runs after this and refines a live slider's fill rather than being the only
/// thing that fills it.
///
/// Value state lives on the SLIDER, never on the rebuilt children, so a rebuild
/// cannot lose it (lesson `rebuilt-view-writes-go-to-state-not-the-entity`).
pub(super) fn reconcile_slider_track_skins(
    skin: Res<UiSkin>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        &SliderTrackSkin,
        Option<&SliderValue>,
        Option<&SliderRange>,
    )>,
) {
    if !skin.is_changed() {
        return;
    }
    let (bg, border) = slider_track_colors(*skin);
    let skin = *skin;
    for (entity, mut node, mut bgc, mut border_color, track, value, range) in &mut q {
        *node = slider_track_node(skin);
        *bgc = bg.into();
        border_color.set_all(border);
        // Prefer the live value: the seed is a CACHE for the display-only case,
        // and a cache read here could be a frame stale (a skin flip and a value
        // change landing together). A track with no `SliderValue` has only the
        // seed, which is the whole reason it exists.
        let fraction = value.map_or(track.seed(), |value| slider_fraction(value, range));
        commands
            .entity(entity)
            .queue_silenced(rebuild_slider_track_children(fraction, skin));
    }
}

/// The track rebuild, as ONE entity command.
///
/// It is one command, and a SILENCED one, on purpose. A caller may despawn a
/// track's subtree on the same `UiSkin` change (the widget zoo rebuilds its
/// whole body), and whether that lands before or after this is a
/// registration-order accident when no ordering edge forces a flush between
/// them - so neither half may panic on an already-dead entity. Splitting it as
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
fn rebuild_slider_track_children(fraction: f32, skin: UiSkin) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.despawn_related::<Children>();
        entity.insert(Children::spawn(SpawnWith(
            move |parent: &mut RelatedSpawner<ChildOf>| {
                spawn_slider_track_children(parent, fraction, skin);
            },
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::fixtures::skin_app;

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

    /// Spawn a slider the way the settings row does: the `Slider` entity IS the
    /// track, carrying its value, range and the `slider_track` visual.
    fn spawn_slider(app: &mut App, value: f32, skin: UiSkin) -> Entity {
        app.world_mut()
            .spawn((
                SliderValue(value),
                SliderRange::new(0.0, 1.0),
                slider_track(value, skin),
            ))
            .id()
    }

    /// The hardware track's single solid fill child, by width.
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

    /// `sync_slider_tracks` lights a slider's phosphor block-meter from its
    /// `SliderValue` - so a `Slider` wearing `slider_track` reacts to drags with
    /// no per-site code. Fails if the system is unregistered or the block
    /// recolour is wrong.
    #[test]
    fn sync_slider_tracks_lights_blocks_from_value() {
        let mut app = skin_app(UiSkin::Phosphor);
        let slider = app
            .world_mut()
            .spawn((SliderValue(0.0), slider_track(0.0, UiSkin::Phosphor)))
            .id();
        app.update();
        assert!(
            block_colors(&mut app, slider)
                .iter()
                .all(|c| *c == theme::PHOSPHOR.with_alpha(0.16)),
            "every bar is dim at value 0"
        );

        // `SliderValue` is immutable, so it is re-inserted (as the real widget
        // does), which fires `Changed` and lights the bars.
        app.world_mut().entity_mut(slider).insert(SliderValue(1.0));
        app.update();
        assert!(
            block_colors(&mut app, slider)
                .iter()
                .all(|c| *c == theme::PHOSPHOR),
            "every bar is lit at value 1.0"
        );
    }

    /// The clamp that makes the meter stop lying at the two ends. A plain
    /// `round()` lit 0 blocks at 2% and all 24 at 98%, so only the endpoints
    /// below distinguish the fix - the 0.0/1.0 cases pass either way and are
    /// kept as the delivery guard that the un-clamped path is still exact.
    #[test]
    fn the_meter_reserves_a_block_at_each_end() {
        let dim = theme::PHOSPHOR.with_alpha(0.16);

        assert_eq!(
            slider_meter_color(0, 0.02),
            theme::PHOSPHOR,
            "a non-empty slider lights at least one block"
        );
        assert_eq!(
            slider_meter_color(SLIDER_SEGMENTS - 1, 0.98),
            dim,
            "a non-full slider leaves at least one block dark"
        );

        assert_eq!(
            slider_meter_color(0, 0.0),
            dim,
            "an EMPTY slider lights nothing"
        );
        assert_eq!(
            slider_meter_color(SLIDER_SEGMENTS - 1, 1.0),
            theme::PHOSPHOR,
            "a FULL slider lights everything"
        );
    }

    /// The HARDWARE track's solid fill follows the value, like the phosphor
    /// block-meter does. Owner playtest 2026-07-29: "the hardware variant
    /// doesn't move the slider, it stays fixed, but the value changes
    /// correctly" - the fill child was unmarked, so the value-sync system could
    /// not see it.
    #[test]
    fn hardware_slider_fill_follows_the_value() {
        let mut app = skin_app(UiSkin::Hardware);
        let slider = spawn_slider(&mut app, 0.25, UiSkin::Hardware);
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

    /// The track RESTYLES live on a `UiSkin` flip. The two skins are different
    /// WIDGETS inside (a segmented block-meter vs one solid fill), so the
    /// reconciler rebuilds the children, not just the colours - and the rebuilt
    /// track must show the CURRENT value, not a default.
    #[test]
    fn slider_track_reskins_and_keeps_its_value() {
        let mut app = skin_app(UiSkin::Phosphor);
        let slider = spawn_slider(&mut app, 0.5, UiSkin::Phosphor);
        app.update();
        assert_eq!(
            block_colors(&mut app, slider).len(),
            SLIDER_SEGMENTS,
            "phosphor shows the segmented block-meter"
        );
        assert_eq!(fill_width(&mut app, slider), None, "and no solid fill");

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
        app.update();
        assert!(
            block_colors(&mut app, slider).is_empty(),
            "the block-meter is gone on the hardware skin"
        );
        assert_eq!(
            fill_width(&mut app, slider),
            Some(percent(50.0)),
            "the rebuilt hardware fill carries the CURRENT value, not a default"
        );
        assert_eq!(
            app.world().entity(slider).get::<Node>().unwrap().height,
            px(SLIDER_TRACK_HARDWARE_H),
            "the track geometry reskinned too, not just its children"
        );

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Phosphor;
        app.update();
        let colors = block_colors(&mut app, slider);
        assert_eq!(
            colors.len(),
            SLIDER_SEGMENTS,
            "and back: the block-meter returns"
        );
        assert!(
            colors
                .iter()
                .take(SLIDER_SEGMENTS / 2)
                .all(|c| *c == theme::PHOSPHOR),
            "lit up to the current half-value, not dark"
        );
    }

    /// A DISPLAY-ONLY track - one spawned straight from `slider_track(fraction,
    /// skin)` with no `SliderValue`, which the signature invites - must survive
    /// a skin flip too. The reconciler reads the fraction itself rather than
    /// leaning on `sync_slider_tracks`, which cannot see a track that has no
    /// value to sync from; otherwise such a track silently emptied itself on the
    /// flip.
    #[test]
    fn a_display_only_track_keeps_its_fraction_across_a_flip() {
        let mut app = skin_app(UiSkin::Phosphor);
        let track = app
            .world_mut()
            .spawn(slider_track(0.75, UiSkin::Phosphor))
            .id();
        app.update();

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
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
        let mut app = skin_app(UiSkin::Hardware);
        let slider = app
            .world_mut()
            .spawn((
                SliderValue(5.0),
                SliderRange::new(0.0, 10.0),
                slider_track(0.5, UiSkin::Hardware),
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

        fn despawn_all_tracks(mut commands: Commands, q: Query<Entity, With<SliderTrackSkin>>) {
            for entity in &q {
                commands.entity(entity).despawn();
            }
        }

        let mut app = skin_app(UiSkin::Phosphor);
        // NOTE: the handler the game installs under `NOVA_AUTOPILOT`, which is how
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
            despawn_all_tracks.before(reconcile_slider_track_skins),
        );

        app.world_mut()
            .spawn((SliderValue(0.5), slider_track(0.5, UiSkin::Phosphor)));
        app.update();

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
        app.update();
    }
}
