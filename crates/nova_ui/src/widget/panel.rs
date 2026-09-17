//! Bordered panel surfaces: the [`panel`] marker, its live theme reconciler,
//! the ready-made [`panel_node`] and the [`panel_head`] title row.
//!
//! Neither factory takes paint. Both spawn UNPAINTED and carry a marker; the
//! reconcilers below paint them on the frame they appear and repaint them when
//! the theme changes. That is what makes a theme flip reach a panel that is
//! already on screen, and what keeps the spawned look and the reconciled look
//! from being two answers to one question.

use bevy::{ecs::relationship::RelatedSpawner, platform::collections::HashSet, prelude::*};

use super::UiText;
use crate::theme::{ActiveUiTheme, UiMetric, BORDER_W};

/// Marks a panel surface, so the reconciler paints it and repaints it live on a
/// theme change.
///
/// Carries NO `Node`, so add it to your own sized `Node` (use [`panel_node`] for
/// a plain content-sized column, or a modal's own width/height/padding `Node`
/// that sets `border` + `border_radius`). Spawn children (a [`panel_head`],
/// rows, ...) into that node.
#[derive(Component)]
pub struct ThemedPanel;

/// A bordered panel surface: the theme's `panel` role, painted live.
pub fn panel() -> impl Bundle {
    (
        ThemedPanel,
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    )
}

/// Paint panels on a theme change and on spawn.
pub(super) fn reconcile_panel_themes(
    theme: Res<ActiveUiTheme>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut BackgroundColor, &mut BorderColor), With<ThemedPanel>>,
    added: Query<Entity, Added<ThemedPanel>>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let paint = theme.panel();
    for (entity, mut bgc, mut border_color) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        *bgc = paint.fill.base.into();
        border_color.set_all(paint.border);
        // `try_*`, not the plain forms: a panel can be despawned the same frame
        // this deferred command is queued (a state teardown), and the plain
        // forms raise "Entity despawned", which the smoke examples promote to a
        // panic. (Repo idiom, e.g. nova_gameplay integrity/glue.rs.)
        let mut ent = commands.entity(entity);
        match &paint.fill.gradient {
            Some(gradient) => {
                ent.try_insert(gradient.clone());
            }
            None => {
                ent.try_remove::<BackgroundGradient>();
            }
        }
        match &paint.shadow {
            Some(shadow) => {
                ent.try_insert(shadow.clone());
            }
            None => {
                ent.try_remove::<BoxShadow>();
            }
        }
    }
}

/// A ready-made panel `Node` for a plain content-sized panel: a column with the
/// panel border. Pair with [`panel`] for the paint - the corner radius is the
/// theme's and lands with it. Sized panels (modals) supply their own `Node`
/// (width/height/padding) with the same `border` and add [`panel`].
pub fn panel_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        border: UiRect::all(px(BORDER_W)),
        ..default()
    }
}

/// Marks a [`panel_head`] row, so the reconciler paints its band and its three
/// text tones live.
#[derive(Component)]
pub struct ThemedPanelHead;

/// Marks a panel head's title span.
#[derive(Component)]
pub struct PanelHeadTitle;

/// Marks the rule that fills a panel head's row.
#[derive(Component)]
pub struct PanelHeadRule;

/// Marks a panel head's trailing tag chip, whose border and legend share one
/// tone.
#[derive(Component)]
pub struct PanelHeadTag;

/// A panel header row: an uppercase title, a rule that fills the row, and an
/// optional trailing tag chip.
pub fn panel_head(title: &str, tag: Option<&str>) -> impl Bundle {
    let title = title.to_uppercase();
    let tag = tag.map(str::to_string);
    (
        ThemedPanelHead,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(15), px(11)),
            border: UiRect::bottom(px(BORDER_W)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                PanelHeadTitle,
                UiText,
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                // Flat, no drop shadow - imprinted on the screen, not floating.
            ));
            parent.spawn((
                PanelHeadRule,
                Node {
                    flex_grow: 1.0,
                    height: px(BORDER_W),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
            if let Some(tag) = tag {
                parent.spawn((
                    PanelHeadTag,
                    Node {
                        padding: UiRect::axes(px(7.0), px(2.0)),
                        border: UiRect::all(px(BORDER_W)),
                        border_radius: BorderRadius::all(px(4.0)),
                        ..default()
                    },
                    BorderColor::all(Color::NONE),
                    children![(
                        PanelHeadTitle,
                        UiText,
                        Text::new(tag),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                    )],
                ));
            }
        })),
    )
}

/// Paint panel heads on a theme change and on spawn.
///
/// The tag chip's own legend is a [`PanelHeadTitle`] span nested under a
/// [`PanelHeadTag`], so it is recoloured by the tag pass rather than the title
/// pass - the queries are disjoint by parent, not by marker.
pub(super) fn reconcile_panel_head_themes(
    theme: Res<ActiveUiTheme>,
    mut q_head: Query<(Entity, &mut BorderColor, &Children), With<ThemedPanelHead>>,
    added: Query<Entity, Added<ThemedPanelHead>>,
    mut q_title: Query<&mut TextColor, With<PanelHeadTitle>>,
    mut q_rule: Query<&mut BackgroundColor, With<PanelHeadRule>>,
    mut q_tag: Query<(&mut BorderColor, &Children), (With<PanelHeadTag>, Without<ThemedPanelHead>)>,
) {
    let restyle_all = theme.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let paint = theme.panel_head();
    for (entity, mut border, children) in &mut q_head {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        border.set_all(paint.border);
        for &child in children {
            if let Ok(mut color) = q_title.get_mut(child) {
                *color = TextColor(paint.title);
            }
            if let Ok(mut bg) = q_rule.get_mut(child) {
                *bg = paint.rule.into();
            }
            if let Ok((mut tag_border, tag_children)) = q_tag.get_mut(child) {
                tag_border.set_all(paint.tag.with_alpha(0.4));
                for &legend in tag_children {
                    if let Ok(mut color) = q_title.get_mut(legend) {
                        *color = TextColor(paint.tag);
                    }
                }
            }
        }
    }
}

/// The corner radius a panel's own `Node` is cut to. Callers that build a sized
/// panel `Node` read it from the theme; this is the accessor they use.
pub fn panel_radius(theme: &ActiveUiTheme) -> f32 {
    theme.metric(UiMetric::PanelRadius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        theme::{HARDWARE_THEME_ID, PHOSPHOR_THEME_ID},
        widget::fixtures::{bg, hardware, phosphor, select, themed_app},
    };

    /// A `panel` restyles its face LIVE when the selected theme changes - the
    /// reconciler that fixes "panels stay phosphor on the hardware look". Fails
    /// if `reconcile_panel_themes` is unregistered.
    #[test]
    fn panel_repaints_on_theme_change() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let panel_e = app.world_mut().spawn((panel_node(), panel())).id();
        app.update();
        assert_eq!(
            bg(&app, panel_e),
            phosphor().panel().fill.base,
            "phosphor screen face"
        );

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(
            bg(&app, panel_e),
            hardware().panel().fill.base,
            "the panel repainted to the hardware case face"
        );
    }

    /// A panel HEAD's band and its three text tones follow the theme too. It
    /// had no reconciler at all before, so a head kept the look its screen was
    /// built in until the screen was rebuilt.
    #[test]
    fn panel_head_repaints_on_theme_change() {
        let mut app = themed_app(PHOSPHOR_THEME_ID);
        let head = app.world_mut().spawn(panel_head("Status", None)).id();
        app.update();

        let title_of = |app: &mut App, head: Entity| {
            let children: Vec<Entity> = app
                .world()
                .entity(head)
                .get::<Children>()
                .unwrap()
                .iter()
                .collect();
            let mut q = app
                .world_mut()
                .query_filtered::<&TextColor, With<PanelHeadTitle>>();
            for child in children {
                if let Ok(color) = q.get(app.world(), child) {
                    return color.0;
                }
            }
            panic!("a PanelHeadTitle span exists");
        };
        assert_eq!(title_of(&mut app, head), phosphor().panel_head().title);

        select(&mut app, HARDWARE_THEME_ID);
        app.update();
        assert_eq!(title_of(&mut app, head), hardware().panel_head().title);
    }
}
