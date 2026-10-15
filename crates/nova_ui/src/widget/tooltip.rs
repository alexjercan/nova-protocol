//! A hover hint: a node that carries a [`Tooltip`] shows its text in a small
//! themed panel under it while the pointer is over it.
//!
//! The panel is a child of the node, so it renders through the same UI camera
//! as the node, a NOVA OS image included. [`OverrideClip`] keeps a scrolling
//! ancestor from cutting it off, and [`TOOLTIP_Z`] puts it over the node's
//! later siblings.

use bevy::{picking::hover::Hovered, prelude::*, ui::OverrideClip};

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

/// Show each panel while its node is hovered and its text is not empty, and
/// keep the panel text equal to the node's [`Tooltip`].
pub(super) fn sync_tooltips(
    q_owner: Query<(&Tooltip, &Hovered, &Children)>,
    mut q_panel: Query<(&mut Node, &Children), With<TooltipPanel>>,
    mut q_text: Query<&mut Text>,
) {
    for (tooltip, hovered, children) in &q_owner {
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
