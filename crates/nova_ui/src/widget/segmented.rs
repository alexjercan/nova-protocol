//! Segmented controls: the recessed [`segmented_container`] and its ghost
//! [`segmented_option`] buttons.

use bevy::{ecs::relationship::RelatedSpawner, prelude::*};

use super::{button, ButtonSpec, ButtonVariant, Selected};
use crate::{skin::UiSkin, theme};

/// Marks a [`segmented_container`] so the skin reconciler repaints it live on a
/// `UiSkin` flip. Its OPTIONS are `ThemedButton`s and already reskin through the
/// button reconciler; without this the container's own recess and corner radius
/// stayed in the old skin until the screen was next rebuilt.
#[derive(Component)]
pub struct SegmentedSkin;

/// The segmented container's `(background, border, corner radius)` per skin -
/// the single source both the factory and the reconciler paint from.
fn segmented_container_paint(skin: UiSkin) -> (Color, Color, f32) {
    if skin.is_phosphor() {
        (
            Color::srgba(0.0, 0.0, 0.0, 0.35),
            theme::PHOSPHOR.with_alpha(0.25),
            theme::RADIUS,
        )
    } else {
        (Color::srgba(0.0, 0.0, 0.0, 0.4), theme::CASE_EDGE, 7.0)
    }
}

/// The bordered/recessed container of a segmented control. Spawn
/// [`segmented_option`]s into it, pairing each with a `ButtonValue<T>` (and
/// `Selected` on the active one) for a functional settings row, or use the
/// display-only [`segmented`] convenience.
pub fn segmented_container(skin: UiSkin) -> impl Bundle {
    let (bg, border, radius) = segmented_container_paint(skin);
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(3),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            align_self: AlignSelf::Start,
            ..default()
        },
        SegmentedSkin,
        BorderColor::all(border),
        BackgroundColor(bg),
    )
}

/// Repaint LIVE segmented containers on a `UiSkin` change (their options ride
/// the button reconciler).
pub(super) fn reconcile_segmented_skins(
    skin: Res<UiSkin>,
    mut q: Query<(&mut Node, &mut BackgroundColor, &mut BorderColor), With<SegmentedSkin>>,
) {
    if !skin.is_changed() {
        return;
    }
    let (bg, border, radius) = segmented_container_paint(*skin);
    for (mut node, mut bgc, mut border_color) in &mut q {
        node.border_radius = BorderRadius::all(px(radius));
        *bgc = bg.into();
        border_color.set_all(border);
    }
}

/// One option of a segmented control: a segment-sized ghost `ThemedButton`. The
/// caller adds a `ButtonValue<T>` (+ `Selected` on the active option) so the
/// shared `button_on_setting::<T>` observer drives the setting on click.
pub fn segmented_option(label: &str) -> impl Bundle {
    let mut spec = ButtonSpec::new(label);
    spec.variant = ButtonVariant::Ghost;
    spec.min_height = 28.0;
    spec.font_size = 12.0;
    button(spec)
}

/// A display-only segmented control: a [`segmented_container`] of
/// [`segmented_option`]s with `active` selected. For a FUNCTIONAL settings row,
/// build the container yourself and add `ButtonValue<T>` to each option.
pub fn segmented(options: &[&str], active: usize, skin: UiSkin) -> impl Bundle {
    let options: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    (
        segmented_container(skin),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            for (i, opt) in options.iter().enumerate() {
                let mut ent = parent.spawn(segmented_option(opt));
                if i == active {
                    ent.insert(Selected);
                }
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::fixtures::{bg, skin_app};

    /// The segmented CONTAINER repaints live on a skin flip. Its options already
    /// did (they are `ThemedButton`s), which is what made the stale recess read
    /// as a half-reskinned row.
    #[test]
    fn segmented_container_reskins_on_skin_change() {
        let mut app = skin_app(UiSkin::Phosphor);
        let seg = app
            .world_mut()
            .spawn(segmented_container(UiSkin::Phosphor))
            .id();
        app.update();
        let radius = |app: &App| {
            app.world()
                .entity(seg)
                .get::<Node>()
                .unwrap()
                .border_radius
                .top_left
        };
        assert_eq!(radius(&app), px(theme::RADIUS), "phosphor recess");

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
        app.update();
        assert_eq!(
            bg(&app, seg),
            Color::srgba(0.0, 0.0, 0.0, 0.4),
            "the container repainted for the hardware skin"
        );
        assert_eq!(radius(&app), px(7.0), "and took the hardware corner radius");
    }
}
