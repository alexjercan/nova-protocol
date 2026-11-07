//! A hover hint: a node that carries a [`Tooltip`] shows its text in a small
//! themed panel under it while the pointer is over it. A node near the bottom
//! or the right edge of the window hangs its panel above it or from its right
//! edge, so the panel stays on screen.
//!
//! The panel is a child of the node, so it renders through the same UI camera
//! as the node, a NOVA OS image included. [`OverrideClip`] keeps a scrolling
//! ancestor from cutting it off, and [`TOOLTIP_Z`] puts it over the node's
//! later siblings.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::{ComputedUiRenderTargetInfo, OverrideClip, UiGlobalTransform},
};

use super::{panel, ThemedRadius, ThemedText, UiText};
use crate::{
    layer::TOOLTIP_Z,
    theme::{UiColor, BORDER_W},
};

/// The hover hint of a node. An empty text shows no panel, so a caller keeps
/// the component and writes the text only when there is something to say.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[require(Hovered)]
pub struct Tooltip(pub String);

/// The panel a [`Tooltip`] node spawns as its child.
#[derive(Component)]
pub struct TooltipPanel;

/// The gap between the node and its panel.
const TOOLTIP_GAP: f32 = 4.0;

/// The panel width. Fixed, because an absolute child takes its node's width
/// as the space to wrap in, and a button is narrower than its reason.
const TOOLTIP_W: f32 = 240.0;

/// Give a new [`Tooltip`] node its hidden panel.
pub(super) fn spawn_tooltip_panel(add: On<Add, Tooltip>, mut commands: Commands) {
    commands.entity(add.entity).with_child((
        Name::new("Tooltip"),
        TooltipPanel,
        GlobalZIndex(TOOLTIP_Z),
        OverrideClip,
        // A panel that took the hover would take it from its own node.
        Pickable::IGNORE,
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: Val::Percent(100.0),
            left: Val::Px(0.0),
            margin: UiRect::top(Val::Px(TOOLTIP_GAP)),
            width: Val::Px(TOOLTIP_W),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
            border: UiRect::all(Val::Px(BORDER_W)),
            ..default()
        },
        ThemedRadius::control(),
        panel(),
        children![(
            UiText,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
            Pickable::IGNORE,
        )],
    ));
}

/// Horizontal padding plus border of the panel, both sides, in logical px.
const TOOLTIP_CHROME_W: f32 = 2.0 * (8.0 + BORDER_W);

/// Vertical padding plus border of the panel, both sides, in logical px.
const TOOLTIP_CHROME_H: f32 = 2.0 * (5.0 + BORDER_W);

/// Advance and line height of the 12 px panel text, in logical px. The UI
/// font is monospace (Iosevka Term), so a text's line count is its length
/// over the chars one line holds.
const TOOLTIP_ADVANCE: f32 = 6.0;
const TOOLTIP_LINE_H: f32 = 15.0;

/// Which side of its node a panel hangs from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TooltipPlacement {
    /// Above the node instead of under it.
    above: bool,
    /// From the node's right edge leftward instead of from its left edge.
    end: bool,
}

/// The panel height a `text` needs, in logical px. An estimate: a hidden
/// panel has no layout to read, and the side is chosen before it shows. It
/// counts one line more than the chars fill, because word wrap breaks a line
/// before it is full.
fn tooltip_height(text: &str) -> f32 {
    let line_chars = ((TOOLTIP_W - TOOLTIP_CHROME_W) / TOOLTIP_ADVANCE).floor();
    let lines: f32 = text
        .lines()
        .map(|line| (line.chars().count() as f32 / line_chars).ceil().max(1.0))
        .sum();
    (lines + 1.0) * TOOLTIP_LINE_H + TOOLTIP_CHROME_H
}

/// The side that keeps a `panel` box on a `target`, for a node at `center`
/// with `size`. All in one unit. A side that does not fit either way keeps
/// the default: under the node, from its left edge. So does a `target` with
/// no size, which is a camera not measured yet.
fn tooltip_placement(
    center: Vec2,
    size: Vec2,
    panel: Vec2,
    gap: f32,
    target: Vec2,
) -> TooltipPlacement {
    if target.x <= 0.0 || target.y <= 0.0 {
        return TooltipPlacement::default();
    }
    let min = center - size / 2.0;
    let max = center + size / 2.0;
    TooltipPlacement {
        above: max.y + gap + panel.y > target.y && min.y - gap - panel.y >= 0.0,
        end: min.x + panel.x > target.x && max.x - panel.x >= 0.0,
    }
}

/// Show each panel while its node is hovered and its text is not empty, and
/// keep the panel text equal to the node's [`Tooltip`]. The side is chosen on
/// the frame the panel shows and held while it stays shown.
pub(super) fn sync_tooltips(
    q_owner: Query<(
        &Tooltip,
        &Hovered,
        &Children,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
    mut q_panel: Query<(&mut Node, &Children), With<TooltipPanel>>,
    mut q_text: Query<&mut Text>,
) {
    for (tooltip, hovered, children, computed, transform, target) in &q_owner {
        let Some(panel) = children.iter().find(|child| q_panel.contains(*child)) else {
            continue;
        };
        let Ok((mut node, lines)) = q_panel.get_mut(panel) else {
            continue;
        };
        let display = if hovered.get() && !tooltip.0.is_empty() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
            if display == Display::Flex {
                // Physical px throughout: the node box and the target are
                // stored that way.
                let scale = 1.0 / computed.inverse_scale_factor();
                let placement = tooltip_placement(
                    transform.translation,
                    computed.size(),
                    Vec2::new(TOOLTIP_W, tooltip_height(&tooltip.0)) * scale,
                    TOOLTIP_GAP * scale,
                    target.physical_size().as_vec2(),
                );
                place_tooltip(&mut node, placement);
            }
        }
        for line in lines.iter() {
            if let Ok(mut text) = q_text.get_mut(line) {
                if text.0 != tooltip.0 {
                    text.0.clone_from(&tooltip.0);
                }
            }
        }
    }
}

/// Hang a panel node on the side `placement` names.
fn place_tooltip(node: &mut Node, placement: TooltipPlacement) {
    let gap = Val::Px(TOOLTIP_GAP);
    if placement.above {
        node.top = Val::Auto;
        node.bottom = Val::Percent(100.0);
        node.margin = UiRect::bottom(gap);
    } else {
        node.top = Val::Percent(100.0);
        node.bottom = Val::Auto;
        node.margin = UiRect::top(gap);
    }
    if placement.end {
        node.left = Val::Auto;
        node.right = Val::Px(0.0);
    } else {
        node.left = Val::Px(0.0);
        node.right = Val::Auto;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tooltip_near_the_bottom_or_right_edge_hangs_on_the_side_that_stays_on_screen() {
        let target = Vec2::new(1280.0, 720.0);
        let size = Vec2::new(120.0, 34.0);
        let panel = Vec2::new(240.0, 60.0);
        let place = |center| tooltip_placement(center, size, panel, 4.0, target);
        assert_eq!(place(Vec2::new(300.0, 300.0)), TooltipPlacement::default());
        assert_eq!(
            place(Vec2::new(300.0, 690.0)),
            TooltipPlacement {
                above: true,
                end: false
            }
        );
        assert_eq!(
            place(Vec2::new(1200.0, 690.0)),
            TooltipPlacement {
                above: true,
                end: true
            }
        );
        // No target size yet (first frame): the default side.
        assert_eq!(
            tooltip_placement(Vec2::new(300.0, 300.0), size, panel, 4.0, Vec2::ZERO),
            TooltipPlacement::default()
        );
    }
}
