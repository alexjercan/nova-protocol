//! Segmented controls: the recessed [`segmented_container`] and its ghost
//! [`segmented_option`] buttons.

use bevy::{ecs::relationship::RelatedSpawner, platform::collections::HashSet, prelude::*};

use super::{button, ButtonSpec, ButtonVariant, Selected};
use crate::theme::{ActiveUiTheme, BORDER_W};

/// Marks a [`segmented_container`] so the theme reconciler paints it and
/// repaints it live. Its OPTIONS are `ThemedButton`s and already follow the
/// button reconciler; without this the container's own recess and corner radius
/// stayed in the old look until the screen was next rebuilt.
#[derive(Component)]
pub struct SegmentedSkin;

/// The bordered/recessed container of a segmented control. Spawn
/// [`segmented_option`]s into it, pairing each with a `ButtonValue<T>` (and
/// `Selected` on the active one) for a functional settings row, or use the
/// display-only [`segmented`] convenience.
pub fn segmented_container() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(3),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(BORDER_W)),
            align_self: AlignSelf::Start,
            ..default()
        },
        SegmentedSkin,
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    )
}

/// A [`segmented_container`] whose options WRAP onto further lines.
///
/// For a bar with more segments than a panel is wide - the Controls tab's
/// group bar, which has one segment per binding group. A single-line container
/// would push its last groups off the panel edge, where nothing can click
/// them.
pub fn segmented_container_wrapping() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(3),
            row_gap: px(3),
            padding: UiRect::all(px(3)),
            border: UiRect::all(px(BORDER_W)),
            align_self: AlignSelf::Start,
            ..default()
        },
        SegmentedSkin,
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    )
}

/// Paint segmented containers on a theme change and on spawn (their options
/// ride the button reconciler).
pub(super) fn reconcile_segmented_themes(
    theme: Res<ActiveUiTheme>,
    mut q: Query<(Entity, &mut Node, &mut BackgroundColor, &mut BorderColor), With<SegmentedSkin>>,
    added: Query<Entity, Added<SegmentedSkin>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let paint = theme.segmented();
    for (entity, mut node, mut bgc, mut border_color) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        node.border_radius = BorderRadius::all(px(paint.radius));
        *bgc = paint.fill.base.into();
        border_color.set_all(paint.border);
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

/// One option of a WRAPPING segmented control ([`segmented_container_wrapping`]):
/// a [`segmented_option`] sized to its own label.
///
/// The full-width kind is right in a single-line bar, where the segments share
/// the row evenly. In a wrapping one it puts every segment on a line of its
/// own.
pub fn segmented_option_fit(label: &str) -> impl Bundle {
    let mut spec = ButtonSpec::new(label).fit();
    spec.variant = ButtonVariant::Ghost;
    spec.min_height = 28.0;
    spec.font_size = 12.0;
    button(spec)
}

/// A display-only segmented control: a [`segmented_container`] of
/// [`segmented_option`]s with `active` selected. For a FUNCTIONAL settings row,
/// build the container yourself and add `ButtonValue<T>` to each option.
pub fn segmented(options: &[&str], active: usize) -> impl Bundle {
    let options: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    (
        segmented_container(),
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
    use crate::{
        theme::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        widget::fixtures::{bg, hardware, phosphor, select, themed_app},
    };

    /// The segmented CONTAINER repaints live on a theme change. Its options
    /// already did (they are `ThemedButton`s), which is what made the stale
    /// recess read as a half-re-themed row.
    #[test]
    fn segmented_container_repaints_on_theme_change() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let seg = app.world_mut().spawn(segmented_container()).id();
        app.update();
        let radius = |app: &App| {
            app.world()
                .entity(seg)
                .get::<Node>()
                .unwrap()
                .border_radius
                .top_left
        };
        assert_eq!(
            radius(&app),
            px(phosphor().segmented().radius),
            "phosphor recess"
        );

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(
            bg(&app, seg),
            hardware().segmented().fill.base,
            "the container repainted for the hardware theme"
        );
        assert_eq!(
            radius(&app),
            px(hardware().segmented().radius),
            "and took the hardware corner radius"
        );
    }
}
