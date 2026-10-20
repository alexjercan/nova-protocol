//! The Ship panel's Placing block: while a part is in hand ([`HeldPart`]),
//! the panel shows that part in place of the selection. The block names the
//! part under its family icon, gives its price (the credits, the parts from
//! the hold and the parts the station sells into it), the socket it mates by
//! and its roll. When the hand empties, the selection comes back.
//!
//! Touch this module when changing what the panel shows for a part in hand.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{ThemedImageTint, ThemedText},
};

use super::{
    catalogue::tile_refusal_text,
    ghost::ShipPlacementPreview,
    rebuild::StationSite,
    scene::{ShipPanelField, ShipRuntime},
    sections::ShipViewSubject,
};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{control_row, divider, panel_preview_frame, themed_label, PANEL_PREVIEW_PX},
};

/// The `Name` of the Placing block, for driven runs.
pub const SHIP_HELD_PANEL: &str = "ShipHeldPanel";

/// The panel's selection content, hidden while a part is in hand.
#[derive(Component)]
pub(crate) struct ShipSelectionBlock;

/// The panel's Placing block, shown while a part is in hand.
#[derive(Component)]
pub(crate) struct ShipHeldBlock;

/// The part in hand's family icon in the Placing block.
#[derive(Component)]
pub(crate) struct ShipHeldIcon;

/// Which value of the part in hand a Placing block text shows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShipHeldField {
    /// The part's name.
    Name,
    /// The credits, or why the part cannot be paid.
    Price,
    /// The parts the hold gives.
    FromHold,
    /// The parts the station sells into it.
    Bought,
    /// The socket the part mates by, of its sockets.
    Socket,
    /// The roll about the mating axis.
    Roll,
}

/// Build the Placing block, hidden until a part is in hand:
/// [`update_ship_held_block`] fills it. Its note line is a
/// [`ShipPanelField::Note`], so an add's result shows here too.
pub(crate) fn spawn_ship_held_block(panel: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    panel
        .spawn((
            ShipHeldBlock,
            Name::new(SHIP_HELD_PANEL),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|block| {
            block.spawn(themed_label("Placing", 13.0, UiColor::Accent));
            block.spawn(panel_preview_frame()).with_children(|frame| {
                frame.spawn((
                    ShipHeldIcon,
                    icon_node(
                        icons.section(SectionIconType::Hull),
                        SectionIconType::Hull.color(),
                        PANEL_PREVIEW_PX * 0.75,
                    ),
                ));
            });
            block.spawn((
                ShipHeldField::Name,
                themed_label("", 16.0, UiColor::Primary),
            ));
            block.spawn(divider());
            for (label, field) in [
                ("Price", ShipHeldField::Price),
                ("From the hold", ShipHeldField::FromHold),
                ("Bought", ShipHeldField::Bought),
                ("Socket", ShipHeldField::Socket),
                ("Roll", ShipHeldField::Roll),
            ] {
                block
                    .spawn(control_row(JustifyContent::SpaceBetween))
                    .with_children(|fact| {
                        fact.spawn((
                            themed_label(label, 12.0, UiColor::Label),
                            Node {
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                        fact.spawn((
                            field,
                            themed_label("", 13.0, UiColor::Primary),
                            TextLayout::new(Justify::Right, LineBreak::NoWrap),
                            Node {
                                min_width: Val::Px(0.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ));
                    });
            }
            block.spawn((ShipPanelField::Note, themed_label("", 12.0, UiColor::Label)));
        });
}

/// Show the Placing block in place of the selection while a part is in hand
/// on the player's own ship, and fill it: the part, its price at the docked
/// station, and the socket and roll of [`ShipRuntime::mount`]. The price is
/// the aimed placement's when the ghost stands valid, else the part's build
/// recipe, the price the catalogue gives.
#[expect(
    clippy::too_many_arguments,
    reason = "one block read from the hand, the mount, the site and the holds"
)]
pub(crate) fn update_ship_held_block(
    held: Res<HeldPart>,
    runtime: Res<ShipRuntime>,
    subject: Res<ShipViewSubject>,
    preview: Res<ShipPlacementPreview>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    icons: Res<InterfaceIcons>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    mut q_block: Query<
        (&mut Node, Has<ShipHeldBlock>),
        Or<(With<ShipHeldBlock>, With<ShipSelectionBlock>)>,
    >,
    mut q_text: Query<(&ShipHeldField, &mut Text, &mut ThemedText)>,
    mut q_icon: Query<(&mut ImageNode, &mut ThemedImageTint), With<ShipHeldIcon>>,
) {
    let offered = held
        .0
        .as_ref()
        .filter(|_| runtime.active && subject.0.is_none())
        .zip(sections.as_ref())
        .map(|(prototype, sections)| {
            sections.get_section(prototype).unwrap_or_else(|| {
                panic!("held part: the section catalogue has no part '{prototype}'")
            })
        });
    for (mut node, is_held) in &mut q_block {
        let display = if offered.is_some() == is_held {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some(offered) = offered else {
        return;
    };

    let aimed = preview
        .placement
        .as_ref()
        .filter(|placement| placement.prototype == offered.base.id)
        .and_then(|placement| placement.payment.clone());
    let payment = aimed.map(Ok).or_else(|| {
        let items = items.as_ref()?;
        let resolved = site.resolve(StationServiceType::Refit)?;
        let (own, own_cr) = q_holds.get(resolved.player).ok()?;
        let (stock, stock_cr) = resolved
            .partner
            .and_then(|(partner, ..)| q_holds.get(partner).ok())?;
        Some(plan_recipe_payment(
            items,
            &offered.base.build,
            stock,
            stock_cr.0,
            own,
            own_cr.0,
        ))
    });
    let parts = |count: u32| {
        if count == 1 {
            "1 part".to_string()
        } else {
            format!("{count} parts")
        }
    };
    let (price, from_hold, bought, price_color) = match &payment {
        Some(Ok(payment)) => (
            cr_text(payment.price_cr),
            parts(payment.from_hold.values().sum()),
            parts(payment.bought.values().sum()),
            UiColor::Primary,
        ),
        Some(Err(refusal)) => (
            tile_refusal_text(refusal),
            "-".to_string(),
            "-".to_string(),
            UiColor::Danger,
        ),
        None => (
            "No refit here".to_string(),
            "-".to_string(),
            "-".to_string(),
            UiColor::Danger,
        ),
    };
    let sockets = offered.base.link_points.len();
    let socket = if sockets == 0 {
        "none".to_string()
    } else {
        format!("{} of {sockets}", runtime.mount.source % sockets + 1)
    };
    let roll = format!("{} deg", runtime.mount.roll % 4 * 90);

    for (field, mut text, mut themed) in &mut q_text {
        let (value, color) = match field {
            ShipHeldField::Name => (&offered.base.name, UiColor::Primary),
            ShipHeldField::Price => (&price, price_color),
            ShipHeldField::FromHold => (&from_hold, UiColor::Primary),
            ShipHeldField::Bought => (&bought, UiColor::Primary),
            ShipHeldField::Socket => (&socket, UiColor::Primary),
            ShipHeldField::Roll => (&roll, UiColor::Primary),
        };
        if text.0 != *value {
            text.0 = value.clone();
        }
        if themed.color != color {
            themed.color = color;
        }
    }
    let icon = SectionIconType::of(offered.kind.class());
    for (mut image, mut tint) in &mut q_icon {
        let wanted = icons.section(icon);
        if image.image != wanted {
            image.image = wanted;
        }
        if tint.color != icon.color() {
            tint.color = icon.color();
        }
    }
}
