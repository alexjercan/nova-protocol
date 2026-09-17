//! List rows: the [`list_row`] container and the observers + reconciler that
//! paint it from its `(selected, hovered)` state and the active theme.

use bevy::{picking::hover::Hovered, platform::collections::HashSet, prelude::*, reflect::Is};

use super::Selected;
use crate::theme::{ActiveUiTheme, RowState, UiMetric, BORDER_W};

/// Marks a list row, so the reconciler paints it and repaints it live.
///
/// A row that ALSO carries `Button` + `Hovered` is interactive and rides the
/// observers below as well; a plain row is a static display row. One marker
/// serves both, because the paint is the same lookup either way - a static row
/// simply never leaves [`RowState::Normal`].
#[derive(Component)]
pub struct ListRow;

/// A list row container (spawn an icon / text / trailing widget into it).
///
/// Spawns UNSELECTED and unpainted. Selection is the `Selected` component,
/// which the caller inserts and removes as its own state moves; the reconciler
/// and the observers both read it, so it is never a colour baked in at spawn.
pub fn list_row() -> impl Bundle {
    (
        ListRow,
        Node {
            width: percent(100),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(12),
            padding: UiRect::axes(px(13), px(10)),
            border: UiRect::all(px(BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    )
}

/// A row's `(background, border)` in the active theme - the accessor for a
/// caller that paints its OWN row entity (the editor's stage highlight, its
/// rail and its menus, which light rows from state the widget layer cannot
/// see). A row that is a plain [`list_row`] needs none of this: the reconciler
/// paints it.
pub fn list_row_colors(theme: &ActiveUiTheme, selected: bool, hovered: bool) -> (Color, Color) {
    let paint = theme.list_row(RowState::resolve(selected, hovered));
    (paint.fill.base, paint.border)
}

fn paint_list_row(
    theme: &ActiveUiTheme,
    state: RowState,
    node: &mut Node,
    bg: &mut BackgroundColor,
    border: &mut BorderColor,
) {
    let paint = theme.list_row(state);
    *bg = paint.fill.base.into();
    border.set_all(paint.border);
    node.border_radius = BorderRadius::all(px(theme.metric(UiMetric::Radius)));
}

/// Repaint one [`ListRow`] on a hover/selection change (the removed component
/// still reads present in its own `Remove` observer, so it is forced false).
pub(super) fn list_row_on_interaction<E: EntityEvent, C: Component>(
    event: On<E, C>,
    theme: Res<ActiveUiTheme>,
    mut q: Query<
        (
            &Hovered,
            Has<Selected>,
            &mut Node,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
) {
    if let Ok((hovered, selected, mut node, mut bg, mut border)) = q.get_mut(event.event_target()) {
        let selected = selected && !(E::is::<Remove>() && C::is::<Selected>());
        paint_list_row(
            &theme,
            RowState::resolve(selected, hovered.get()),
            &mut node,
            &mut bg,
            &mut border,
        );
    }
}

/// Restyle LIVE list rows on a theme change, and paint just-spawned rows
/// (`Added<ListRow>`) - the same override the button reconciler uses.
///
/// `Hovered` is optional here: a static display row has no picking components
/// at all, and requiring one left every such row unpainted.
#[expect(
    clippy::type_complexity,
    reason = "one query term per row visual state plus the just-added set"
)]
pub(super) fn reconcile_list_row_themes(
    theme: Res<ActiveUiTheme>,
    mut q: Query<
        (
            Entity,
            Option<&Hovered>,
            Has<Selected>,
            &mut Node,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ListRow>,
    >,
    added: Query<Entity, Added<ListRow>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    for (entity, hovered, selected, mut node, mut bg, mut border) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        let hovered = hovered.is_some_and(Hovered::get);
        paint_list_row(
            &theme,
            RowState::resolve(selected, hovered),
            &mut node,
            &mut bg,
            &mut border,
        );
    }
}
