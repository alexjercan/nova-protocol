//! Bordered panel surfaces: the [`panel`] paint, its [`PanelSkin`] live
//! reconciler, the ready-made [`panel_node`] and the [`panel_head`] title row.

use bevy::{ecs::relationship::RelatedSpawner, platform::collections::HashSet, prelude::*};

use super::{paint::drop_shadow, UiText};
use crate::{skin::UiSkin, theme};

/// The panel PAINT: a bordered surface's colours - phosphor is a dark screen
/// face with an inner phosphor border + top glow, hardware a case gradient +
/// drop shadow. It carries NO `Node`, so add it to your own sized `Node` (use
/// [`panel_node`] for a plain content-sized column, or a modal's own
/// width/height/padding Node that sets `border` + `border_radius`). Spawn
/// children (a `panel_head`, rows, ...) into that node.
pub fn panel(skin: UiSkin) -> impl Bundle {
    let (border, bg, shadow, gradient) = panel_paint(skin);
    (
        BorderColor::all(border),
        BackgroundColor(bg),
        shadow,
        gradient,
        // So the panel reconciler restyles it live on a UiSkin flip.
        PanelSkin,
    )
}

/// The panel paint pieces for a skin: `(border, background, shadow, gradient)` -
/// the single source [`panel`] and the [`PanelSkin`] reconciler both use.
fn panel_paint(skin: UiSkin) -> (Color, Color, BoxShadow, BackgroundGradient) {
    if skin.is_phosphor() {
        (
            theme::PHOSPHOR.with_alpha(0.16),
            theme::SCREEN_0,
            BoxShadow(vec![]),
            // PoC phosphor panel: a soft phosphor glow blooming down from the top
            // edge (demo `radial-gradient(ellipse 70% 60% at 50% 0%, phosphor .10,
            // transparent 70%)`).
            BackgroundGradient(vec![Gradient::from(RadialGradient::new(
                UiPosition::TOP,
                RadialGradientShape::FarthestSide,
                vec![
                    ColorStop::percent(theme::PHOSPHOR.with_alpha(0.06), 0.0),
                    ColorStop::percent(Color::NONE, 70.0),
                ],
            ))]),
        )
    } else {
        (
            theme::CASE_EDGE,
            theme::CASE_1,
            drop_shadow(),
            // PoC panel: linear-gradient(168deg, case-2 0%, case-0 88%, #05080a).
            BackgroundGradient(vec![LinearGradient::degrees(
                168.0,
                vec![
                    ColorStop::percent(theme::CASE_2, 0.0),
                    ColorStop::percent(theme::CASE_0, 88.0),
                    ColorStop::percent(theme::CASE_EDGE, 100.0),
                ],
            )
            .into()]),
        )
    }
}

/// Marks a [`panel`] so the skin reconciler restyles its paint live when the
/// `UiSkin` resource flips (mirrors the button/list-row reconcilers).
#[derive(Component)]
pub struct PanelSkin;

/// Restyle LIVE panels on a `UiSkin` change (and paint just-spawned ones).
#[allow(clippy::type_complexity)]
pub(super) fn reconcile_panel_skins(
    skin: Res<UiSkin>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut BackgroundColor, &mut BorderColor), With<PanelSkin>>,
    added: Query<Entity, Added<PanelSkin>>,
) {
    let restyle_all = skin.is_changed();
    let just_added: HashSet<Entity> = added.iter().collect();
    if !restyle_all && just_added.is_empty() {
        return;
    }
    let (border, bg, shadow, gradient) = panel_paint(*skin);
    for (entity, mut bgc, mut border_color) in &mut q {
        if !restyle_all && !just_added.contains(&entity) {
            continue;
        }
        *bgc = bg.into();
        border_color.set_all(border);
        commands
            .entity(entity)
            .try_insert((shadow.clone(), gradient.clone()));
    }
}

/// A ready-made panel `Node` for a plain content-sized panel: a column with the
/// panel border + [`theme::PANEL_RADIUS`] corners. Pair with [`panel`] for the
/// paint. Sized panels (modals) supply their own `Node` (width/height/padding)
/// with the same `border` + `border_radius` and add [`panel`].
pub fn panel_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        border: UiRect::all(px(theme::BORDER_W)),
        border_radius: BorderRadius::all(px(theme::PANEL_RADIUS)),
        ..default()
    }
}

/// A panel header row: a glowing uppercase title, a rule that fills the row, and
/// an optional trailing tag chip (phosphor-muted bordered).
pub fn panel_head(title: &str, tag: Option<&str>, _skin: UiSkin) -> impl Bundle {
    let title = title.to_uppercase();
    let tag = tag.map(str::to_string);
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(10),
            padding: UiRect::axes(px(15), px(11)),
            border: UiRect::bottom(px(theme::BORDER_W)),
            ..default()
        },
        BorderColor::all(theme::PHOSPHOR.with_alpha(0.18)),
        Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
            parent.spawn((
                UiText,
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR),
                // Flat, no drop shadow - imprinted on the screen, not floating.
            ));
            parent.spawn((
                Node {
                    flex_grow: 1.0,
                    height: px(theme::BORDER_W),
                    ..default()
                },
                BackgroundColor(theme::PHOSPHOR_MUTED.with_alpha(0.6)),
            ));
            if let Some(tag) = tag {
                parent.spawn((
                    Node {
                        padding: UiRect::axes(px(7.0), px(2.0)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(4.0)),
                        ..default()
                    },
                    BorderColor::all(theme::PHOSPHOR.with_alpha(0.4)),
                    children![(
                        UiText,
                        Text::new(tag),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(theme::PHOSPHOR),
                    )],
                ));
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::fixtures::{bg, skin_app};

    /// A `panel` restyles its face LIVE when the `UiSkin` resource flips - the
    /// reconciler that fixes "panels stay phosphor on the hardware skin". Fails
    /// if `reconcile_panel_skins` is unregistered.
    #[test]
    fn panel_reskins_on_skin_change() {
        let mut app = skin_app(UiSkin::Phosphor);
        let panel_e = app
            .world_mut()
            .spawn((panel_node(), panel(UiSkin::Phosphor)))
            .id();
        app.update();
        assert_eq!(bg(&app, panel_e), theme::SCREEN_0, "phosphor screen face");

        *app.world_mut().resource_mut::<UiSkin>() = UiSkin::Hardware;
        app.update();
        assert_eq!(
            bg(&app, panel_e),
            theme::CASE_1,
            "the panel reskinned to the hardware case face"
        );
    }
}
