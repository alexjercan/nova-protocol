//! List rows: the [`list_row`] container, its `(selected, hovered, skin)`
//! colours and the observers + reconciler that repaint an interactive
//! [`ListRow`] live.

use bevy::{picking::hover::Hovered, platform::collections::HashSet, prelude::*, reflect::Is};

use super::Selected;
use crate::{skin::UiSkin, theme};

/// A list row container (spawn an icon / text / trailing widget into it).
/// Transparent with an inset border; selection tints amber (hardware) or inverts
/// phosphor. Not a `ThemedButton` - rows carry their own click handlers.
pub fn list_row(selected: bool, skin: UiSkin) -> impl Bundle {
    let (bg, border) = list_row_colors(selected, false, skin);
    (
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(13), px(10)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(if skin.is_phosphor() {
                theme::RADIUS
            } else {
                7.0
            })),
            ..default()
        },
        BorderColor::all(border),
        BackgroundColor(bg),
    )
}

/// The `(background, border)` of a list row in a given `(selected, hovered,
/// skin)` - the single source both [`list_row`] and the [`ListRow`] reconciler
/// paint from.
pub fn list_row_colors(selected: bool, hovered: bool, skin: UiSkin) -> (Color, Color) {
    let phosphor = skin.is_phosphor();
    match (selected, phosphor) {
        (true, true) => (theme::PHOSPHOR.with_alpha(0.14), theme::PHOSPHOR),
        (true, false) => (theme::CASE_2, theme::AMBER_NOVA.with_alpha(0.5)),
        (false, true) if hovered => (
            theme::PHOSPHOR.with_alpha(0.06),
            theme::PHOSPHOR.with_alpha(0.2),
        ),
        (false, true) => (Color::NONE, theme::PHOSPHOR.with_alpha(0.14)),
        (false, false) if hovered => (Color::WHITE.with_alpha(0.06), Color::WHITE.with_alpha(0.1)),
        (false, false) => (Color::WHITE.with_alpha(0.02), Color::WHITE.with_alpha(0.05)),
    }
}

/// Marks an INTERACTIVE list row (a `list_row` + `Button` + `Hovered`): the
/// reconciler repaints it from its `Selected`/`Hovered` state + the skin, so
/// clicking or hovering highlights it live (mods/scenarios rows). A plain
/// `list_row` without this marker is a static display row (the widget_zoo).
#[derive(Component)]
pub struct ListRow;

fn paint_list_row(
    skin: UiSkin,
    selected: bool,
    hovered: bool,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let (b, br) = list_row_colors(selected, hovered, skin);
    *bg = b.into();
    border.set_all(br);
}

/// Repaint one [`ListRow`] on a hover/selection change (the removed component
/// still reads present in its own `Remove` observer, so it is forced false).
pub(super) fn list_row_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    skin: Res<UiSkin>,
    mut q: Query<
        (
            &Hovered,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
) {
    if let Ok((hovered, selected, mut bg, mut border)) = q.get_mut(event.event_target()) {
        let selected = selected && !(E::is::<Remove>() && C::is::<Selected>());
        paint_list_row(*skin, selected, hovered.get(), &mut bg, &mut border);
    }
}

/// Restyle LIVE list rows on a `UiSkin` change, and paint just-spawned rows
/// (`Added<ListRow>`) - the same override the button reconciler uses.
#[allow(clippy::type_complexity)]
pub(super) fn reconcile_list_row_skins(
    skin: Res<UiSkin>,
    mut q: Query<
        (
            Entity,
            &Hovered,
            Has<Selected>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
    added: Query<Entity, Added<ListRow>>,
) {
    let restyle_all = skin.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    for (entity, hovered, selected, mut bg, mut border) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        paint_list_row(*skin, selected, hovered.get(), &mut bg, &mut border);
    }
}
