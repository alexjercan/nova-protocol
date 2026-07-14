//! The component tooltip: hovering a `ComponentCard` spawns a small floating
//! panel showing the section's name, HP and description (read from
//! `GameSections`); leaving the card despawns it. Rebuilt fresh per hover rather
//! than kept persistent, so there are no child text nodes to clear.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use crate::{
    ui::{card::ComponentCard, theme},
    ExampleStates,
};

/// The floating detail panel. At most one exists at a time. Tagged with the
/// card it belongs to so `hide` only removes the tooltip for the card being
/// left - if `Over(cardB)` is delivered before `Out(cardA)` when crossing
/// directly between two cards, B's fresh tooltip must survive A's `Out`.
#[derive(Component)]
pub(crate) struct Tooltip {
    card: Entity,
}

/// A tooltip must not capture the pointer (it is spawned under the cursor).
const IGNORE: Pickable = Pickable {
    should_block_lower: false,
    is_hoverable: false,
};

pub(crate) fn register(app: &mut App) {
    app.add_observer(show_component_tooltip)
        .add_observer(hide_component_tooltip);
}

fn show_component_tooltip(
    over: On<Pointer<Over>>,
    mut commands: Commands,
    q_card: Query<&ComponentCard>,
    sections: Res<GameSections>,
    q_existing: Query<Entity, With<Tooltip>>,
) {
    let Ok(card) = q_card.get(over.entity) else {
        return;
    };
    let Some(section) = sections.get_section(&card.id) else {
        return;
    };

    // At most one tooltip: clear any leftover before spawning the new one.
    for entity in &q_existing {
        commands.entity(entity).despawn();
    }

    let pos = over.pointer_location.position;
    commands.spawn((
        Tooltip { card: over.entity },
        DespawnOnExit(ExampleStates::Editor),
        Name::new("Component Tooltip"),
        IGNORE,
        GlobalZIndex(50),
        Node {
            position_type: PositionType::Absolute,
            left: px(pos.x + 16.0),
            top: px(pos.y + 16.0),
            max_width: px(240),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(theme::BORDER_W)),
            row_gap: px(2),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BackgroundColor(theme::PANEL_RAISED),
        BorderColor::all(theme::BORDER_BRIGHT),
        children![
            (
                Text::new(section.base.name.clone()),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme::CYAN_BRIGHT),
            ),
            (
                Text::new(format!("HP {}", section.base.health)),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::AMBER),
            ),
            (
                Text::new(section.base.description.clone()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::TEXT_MUTED),
            ),
        ],
    ));
}

fn hide_component_tooltip(
    out: On<Pointer<Out>>,
    mut commands: Commands,
    q_card: Query<&ComponentCard>,
    q_existing: Query<(Entity, &Tooltip)>,
) {
    if q_card.get(out.entity).is_err() {
        return;
    }
    // Only despawn the tooltip that belongs to the card being left, so a fresh
    // tooltip for a newly-entered card (whose Over may have arrived first)
    // survives.
    for (entity, tooltip) in &q_existing {
        if tooltip.card == out.entity {
            commands.entity(entity).despawn();
        }
    }
}
