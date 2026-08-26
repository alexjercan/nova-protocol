//! Scrollable viewports: the unit conversion, the wheel driver and the
//! every-frame clamp.

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb},
};

use crate::{skin::UiSkin, theme};

/// Pixels scrolled per line of wheel movement.
///
/// Three lines of type, not one: a wheel notch that moved a single line made a
/// thirty-row inspector a wrist exercise.
const SCROLL_LINE_HEIGHT: f32 = 60.0;

/// How wide a scrollbar's track is, in logical pixels.
const BAR_W: f32 = 8.0;

/// The shortest a thumb is allowed to be, so a very long list still leaves
/// something to grab.
const MIN_THUMB: f32 = 24.0;

/// A keyboard page as a fraction of the viewport height, so a page leaves a
/// couple of lines of overlap to read against.
const PAGE_FRACTION: f32 = 0.8;

/// How far a viewport's content overflows its box, in the LOGICAL pixels
/// [`ScrollPosition`] is expressed in.
///
/// [`ComputedNode`] reports PHYSICAL pixels, so the conversion happens here and
/// nowhere else: on a 2x display an unconverted maximum is twice the real one
/// and the pane scrolls into empty space. No `ComputedNode` yet (first frame)
/// means layout has not run, so nothing is known to clamp against and the
/// maximum is unbounded.
pub fn max_scroll_y(node: Option<&ComputedNode>) -> f32 {
    node.map(|node| {
        (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0)
            * node.inverse_scale_factor()
    })
    .unwrap_or(f32::MAX)
}

/// One keyboard page step for a viewport, in the same logical pixels as
/// [`max_scroll_y`]. Zero before layout has measured the node.
pub fn page_step(node: Option<&ComputedNode>) -> f32 {
    node.map(|node| node.size.y * PAGE_FRACTION * node.inverse_scale_factor())
        .unwrap_or(0.0)
}

/// Marks a scrollable viewport driven by [`scroll_viewports`] and clamped every
/// frame by [`clamp_viewports`].
///
/// The every-frame clamp is the point: clamping only on wheel input leaves the
/// stored offset past the end when the CONTENT shrinks, and the pane renders
/// blank until the player scrolls again.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScrollViewport;

/// Wheel-scroll every [`ScrollViewport`], clamped at both ends.
///
/// A hovered viewport takes the whole wheel delta: with two viewports on screen
/// the pointer picks which one moves, and only when none is hovered do they all
/// scroll together.
pub fn scroll_viewports(
    mut wheel: MessageReader<MouseWheel>,
    mut viewports: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<ScrollViewport>,
    >,
) {
    let dy: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * SCROLL_LINE_HEIGHT,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if dy == 0.0 {
        return;
    }

    let any_hovered = viewports
        .iter()
        .any(|(_, _, hovered)| hovered.is_some_and(Hovered::get));

    for (mut scroll, node, hovered) in &mut viewports {
        if any_hovered && !hovered.is_some_and(Hovered::get) {
            continue;
        }
        // Clamp the STORED offset, not just the top: bevy clamps only the value
        // it writes into `ComputedNode`, so invisible bottom overscroll would
        // otherwise accumulate and the next wheel-up does nothing.
        scroll.0.y = (scroll.0.y - dy).clamp(0.0, max_scroll_y(node));
    }
}

/// Pull every [`ScrollViewport`] back inside its content after layout has
/// measured it, so a SHRINKING content size cannot leave the pane scrolled past
/// its end.
pub fn clamp_viewports(
    mut viewports: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollViewport>>,
) {
    for (mut scroll, node) in &mut viewports {
        let clamped = scroll.0.y.clamp(0.0, max_scroll_y(Some(node)));
        if scroll.0.y != clamped {
            scroll.0.y = clamped;
        }
    }
}

/// Marks a scrollbar that drives the scrolling pane standing BESIDE it.
///
/// [`Scrollbar`] wants the target entity when the bar is spawned, which a
/// `children!` bundle cannot know: the pane and the bar are minted in the same
/// breath. This says "the viewport next to me" instead, and
/// [`wire_scroll_bars`] fills the entity in once the row exists.
#[derive(Component, Debug, Clone, Copy)]
pub struct SiblingScrollBar;

/// A vertical scrollbar for the [`scroll_column`] it is spawned NEXT TO.
///
/// Always drawn, even with nothing to scroll - the widget simply gives the
/// thumb the whole track then. A bar that appeared only once the content
/// overflowed would take its own width out of the pane on the way in, which
/// reflows the content that summoned it.
pub fn scroll_bar(skin: UiSkin) -> impl Bundle {
    let phosphor = skin.is_phosphor();
    let (track, thumb) = if phosphor {
        (
            theme::PHOSPHOR.with_alpha(0.07),
            theme::PHOSPHOR.with_alpha(0.35),
        )
    } else {
        (Color::WHITE.with_alpha(0.04), theme::CASE_3)
    };
    (
        SiblingScrollBar,
        Node {
            width: px(BAR_W),
            align_self: AlignSelf::Stretch,
            flex_shrink: 0.0,
            margin: UiRect::left(px(4)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BackgroundColor(track),
        children![(
            ScrollbarThumb {
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            BackgroundColor(thumb),
        )],
    )
}

/// A row holding a [`scroll_column`] and the [`scroll_bar`] beside it.
pub fn scroll_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_self: AlignSelf::Stretch,
        align_items: AlignItems::Stretch,
        flex_grow: 1.0,
        min_height: px(0),
        min_width: px(0),
        ..default()
    }
}

/// Point every [`SiblingScrollBar`] at the viewport it stands beside.
///
/// Re-runs for a bar whose target has gone, so a pane rebuilt under the same
/// bar is picked up again rather than leaving a thumb driving a dead entity.
pub fn wire_scroll_bars(
    mut commands: Commands,
    bars: Query<(Entity, &ChildOf, Option<&Scrollbar>), With<SiblingScrollBar>>,
    rows: Query<&Children>,
    viewports: Query<(), With<ScrollViewport>>,
) {
    for (bar, owner, wired) in &bars {
        if wired.is_some_and(|wired| viewports.contains(wired.target)) {
            continue;
        }
        let Ok(siblings) = rows.get(owner.parent()) else {
            continue;
        };
        let Some(target) = siblings.iter().find(|sibling| viewports.contains(*sibling)) else {
            continue;
        };
        commands.entity(bar).insert(Scrollbar::new(
            target,
            ControlOrientation::Vertical,
            MIN_THUMB,
        ));
    }
}

/// Paint a bar only while its pane has somewhere to go.
///
/// HIDDEN rather than undisplayed: the track keeps its width in the row either
/// way, so the pane it drives is the same width whether the bar is painted or
/// not. A bar that took its own width back on the way out would rewrap the
/// content that summoned it, which is how a pane ends up flickering one line
/// of text in and out.
///
/// Last frame's measurements, like every other reader of [`ComputedNode`]:
/// whether a bar is worth painting is not a question a frame of lag can get
/// wrong twice.
pub fn hide_idle_scroll_bars(
    mut bars: Query<(&Scrollbar, &mut Visibility), With<SiblingScrollBar>>,
    panes: Query<&ComputedNode, With<ScrollViewport>>,
) {
    for (bar, mut visibility) in &mut bars {
        let wanted = if max_scroll_y(panes.get(bar.target).ok()) > 0.5 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Register the wheel driver, the bar wiring and the every-frame clamp.
pub(crate) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (
            scroll_viewports.run_if(resource_exists::<Messages<MouseWheel>>),
            wire_scroll_bars,
            hide_idle_scroll_bars.after(wire_scroll_bars),
        ),
    );
    // AFTER layout, or it clamps this frame's offset against last frame's
    // measurements and a viewport whose content just grew snaps back.
    app.add_systems(
        PostUpdate,
        clamp_viewports.after(bevy::ui::UiSystems::Layout),
    );
}
