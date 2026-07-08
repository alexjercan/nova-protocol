//! A minimal reusable slider widget on top of bevy's `ui_widgets::Slider`.
//!
//! The core `ui_widgets` slider deliberately does two things *not*: it neither writes its own
//! `SliderValue` back on a drag (it only fires a `ValueChange` event) nor moves the visible
//! thumb - both are left to the app (see the `bevy_ui_widgets` slider docs). This module closes
//! that loop once so callers do not each reimplement it:
//!
//! - `echo_value_into_slider` writes each `ValueChange` into the source slider's `SliderValue`,
//!   so the widget's own state tracks the drag.
//! - `position_thumbs` moves the thumb whenever `SliderValue` (or the range) changes - gated on
//!   `Changed`, so it tracks drags, not just the initial insert. (The earlier version keyed the
//!   thumb off `On<Insert, SliderValue>`, which never fired for the widget's in-place value
//!   edits, so the thumb sat still while the value moved.)
//!
//! It knows nothing about turrets or any particular value, so it can be lifted into a shared
//! crate later. Callers build a slider with [`slider`], add [`SliderWidgetPlugin`], and observe
//! `ValueChange<f32>` on the slider entity for their own binding.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue, ValueChange},
};

const SLIDER_TRACK: Color = Color::srgb(0.05, 0.05, 0.05);
const SLIDER_THUMB: Color = Color::srgb(0.35, 0.75, 0.35);

/// Marks a slider built by [`slider`].
#[derive(Component)]
pub struct SliderWidget;

/// Marks a slider widget's draggable thumb.
#[derive(Component)]
struct SliderWidgetThumb;

/// Registers the behaviour that makes [`slider`] widgets self-consistent: value echo, thumb
/// positioning, and thumb hover styling. Add once.
pub struct SliderWidgetPlugin;

impl Plugin for SliderWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(echo_value_into_slider);
        app.add_observer(highlight_hovered_thumb::<Insert, Hovered>);
        app.add_systems(Update, position_thumbs);
    }
}

/// A slider widget bundle for a value in `min..=max`, starting at `value`.
///
/// The caller owns the meaning of the value: observe `ValueChange<f32>` on this entity to read
/// edits. The widget keeps its own `SliderValue` and thumb in sync via [`SliderWidgetPlugin`].
pub fn slider(min: f32, max: f32, value: f32) -> impl Bundle {
    (
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            height: px(12),
            width: percent(100),
            ..default()
        },
        Name::new("Slider"),
        Hovered::default(),
        SliderWidget,
        Slider::default(),
        SliderValue(value),
        SliderRange::new(min, max),
        Children::spawn((
            // Track rail.
            Spawn((
                Node {
                    height: px(6),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK),
            )),
            // Invisible track, short by the thumb width, so the thumb can be placed by a simple
            // percentage of its length.
            Spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(12),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                children![(
                    SliderWidgetThumb,
                    SliderThumb,
                    Node {
                        display: Display::Flex,
                        width: px(12),
                        height: px(12),
                        position_type: PositionType::Absolute,
                        left: percent(0),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(SLIDER_THUMB),
                )],
            )),
        )),
    )
}

/// Echo each `ValueChange` into the source slider's own `SliderValue`. The core widget fires the
/// event but does not update its value, so without this the widget's state (and thumb) would
/// never move.
fn echo_value_into_slider(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    q_slider: Query<(), With<SliderWidget>>,
) {
    if q_slider.contains(change.source) {
        commands
            .entity(change.source)
            .insert(SliderValue(change.value));
    }
}

/// Keep each slider's thumb at the position of its current value. Gated on `Changed` so it
/// tracks live drags (the widget edits `SliderValue` in place) as well as the initial spawn.
fn position_thumbs(
    q_slider: Query<
        (Entity, &SliderValue, &SliderRange),
        (
            With<SliderWidget>,
            Or<(Changed<SliderValue>, Changed<SliderRange>)>,
        ),
    >,
    children: Query<&Children>,
    mut q_thumb: Query<&mut Node, With<SliderWidgetThumb>>,
) {
    for (slider, value, range) in &q_slider {
        for descendant in children.iter_descendants(slider) {
            if let Ok(mut thumb) = q_thumb.get_mut(descendant) {
                thumb.left = percent(range.thumb_position(value.0) * 100.0);
            }
        }
    }
}

/// Lighten the thumb while its slider is hovered.
fn highlight_hovered_thumb<E: EntityEvent, C: Component>(
    event: On<E, C>,
    q_slider: Query<(Entity, &Hovered), With<SliderWidget>>,
    children: Query<&Children>,
    mut q_thumb: Query<&mut BackgroundColor, With<SliderWidgetThumb>>,
) {
    if let Ok((slider, hovered)) = q_slider.get(event.event_target()) {
        for descendant in children.iter_descendants(slider) {
            if let Ok(mut thumb_bg) = q_thumb.get_mut(descendant) {
                thumb_bg.0 = if hovered.0 {
                    SLIDER_THUMB.lighter(0.3)
                } else {
                    SLIDER_THUMB
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The thumb must follow later value edits (a drag mutates `SliderValue` in place), not
    /// only the initial insert - this is the bug the `Changed`-gated system fixes.
    #[test]
    fn thumb_tracks_value_changes() {
        let mut app = App::new();
        app.add_systems(Update, position_thumbs);

        let slider = app
            .world_mut()
            .spawn((SliderWidget, SliderValue(25.0), SliderRange::new(0.0, 100.0)))
            .id();
        let thumb = app
            .world_mut()
            .spawn((SliderWidgetThumb, Node::default()))
            .id();
        app.world_mut().entity_mut(slider).add_child(thumb);

        // Initial spawn positions the thumb at 25%.
        app.update();
        assert_eq!(app.world().get::<Node>(thumb).unwrap().left, percent(25.0));

        // A later edit (the widget re-inserts `SliderValue`, which is immutable) must move the
        // thumb too - the case the old `On<Insert>`-per-slider wiring missed once nothing was
        // re-inserting it.
        app.world_mut()
            .entity_mut(slider)
            .insert(SliderValue(75.0));
        app.update();
        assert_eq!(app.world().get::<Node>(thumb).unwrap().left, percent(75.0));
    }
}
