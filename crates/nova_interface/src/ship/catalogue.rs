//! The Ship pane's station catalogue: the parts a docked refit station can
//! build, each with its price, a Place button and, for a part that can
//! replace the selected section, a Swap button.
//!
//! A part is listed when it is not hidden from the editor palette and the
//! player's hold and the station's stock together carry every part of its
//! recipe ([`plan_recipe_payment`] finds no missing part). Credits do not
//! filter: a part the ship cannot pay for shows the credit refusal.
//!
//! Place puts the part in hand ([`ShipRuntime::held`]); the placement ghost
//! (`super::ghost`) shows where a click adds it. Place on the held row puts it
//! down. Swap raises a [`SectionSwapCommand`] for the selected section.
//!
//! Touch this module when changing what the catalogue lists or how a row
//! reads.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use super::{
    rebuild::{payment_refusal_text, rebuild_price_text, StationSite},
    refit::{on_ship_swap_button, swap_refusal_text},
    scene::ShipRuntime,
    sections::design_section_code,
};
use crate::pane::{compact_button, play_menu_select, themed_label};

/// The catalogue container, hidden while the refit gate does not hold.
#[derive(Component)]
pub(crate) struct ShipCatalogue;

/// The catalogue's header text: what it is, or which part is in hand.
#[derive(Component)]
pub(crate) struct ShipCatalogueHeader;

/// The rows inside [`ShipCatalogue`], and the row texts they were built
/// from, so the rows respawn only when one changes.
#[derive(Component, Default)]
pub(crate) struct ShipCatalogueRows(Vec<CatalogueRow>);

/// One listed part as the catalogue shows it, or the line saying nothing is
/// listed when `prototype` is empty.
#[derive(Clone, Debug, PartialEq)]
struct CatalogueRow {
    prototype: String,
    name: String,
    line: String,
    held: bool,
    /// The selected design section and the swap line, when the part can
    /// replace it.
    swap: Option<(String, String)>,
}

/// The line shown when the station builds nothing from the two holds.
const EMPTY_LINE: &str = "This station stocks the parts for no section.";

/// Build the catalogue under the lost list: a header and the rows
/// [`update_ship_catalogue`] fills.
pub(crate) fn spawn_ship_catalogue(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn((
            ShipCatalogue,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|list| {
            list.spawn(crate::pane::divider());
            list.spawn((
                ShipCatalogueHeader,
                themed_label("Station catalogue", 13.0, UiColor::Accent),
            ));
            list.spawn((
                ShipCatalogueRows::default(),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ));
        });
}

/// Show the catalogue while the refit gate holds, and respawn its rows when a
/// listed part, a price, a refusal, the part in hand or the selection's swap
/// offers change.
#[expect(
    clippy::too_many_arguments,
    reason = "one system reading the selection, the catalogue, both holds and the list"
)]
pub(crate) fn update_ship_catalogue(
    mut commands: Commands,
    runtime: Res<ShipRuntime>,
    items: Option<Res<GameItems>>,
    sections: Option<Res<GameSections>>,
    site: StationSite,
    q_holds: Query<(&ShipInventory, &ShipCredits)>,
    mut q_list: Query<&mut Node, With<ShipCatalogue>>,
    mut q_header: Query<&mut Text, With<ShipCatalogueHeader>>,
    mut q_rows: Query<(Entity, &mut ShipCatalogueRows)>,
) {
    let Ok(mut list) = q_list.single_mut() else {
        return;
    };
    let Ok((rows_entity, mut shown)) = q_rows.single_mut() else {
        return;
    };
    let resolved = if site.gate(StationServiceType::Refit) {
        site.resolve(StationServiceType::Refit)
    } else {
        None
    };
    let mut header = "Station catalogue".to_string();
    let mut rows = Vec::new();
    let mut open = false;
    if let (Some(resolved), Some(items), Some(sections)) = (resolved, items, sections) {
        let (own, own_cr) = q_holds
            .get(resolved.player)
            .expect("the player ship carries a ShipInventory and ShipCredits");
        let partner = resolved
            .partner
            .and_then(|(partner, _)| q_holds.get(partner).ok())
            .map(|(stock, credits)| (stock, credits.0));
        if let Some((stock, stock_cr)) = partner {
            open = true;
            let selected = resolved.selected_section(runtime.selected);
            for offered in sections
                .iter()
                .filter(|offered| !offered.base.hide_in_editor)
            {
                let recipe = &offered.base.build;
                let stocked = !matches!(
                    plan_recipe_payment(&items, recipe, stock, stock_cr, own, u32::MAX),
                    Err(RecipePaymentRefusalType::MissingParts { .. })
                );
                if !stocked {
                    continue;
                }
                let name = offered.base.name.clone();
                let line = match plan_recipe_payment(&items, recipe, stock, stock_cr, own, own_cr.0)
                {
                    Ok(payment) => rebuild_price_text(&payment),
                    Err(refusal) => payment_refusal_text(&items, &name, &refusal),
                };
                let swap = selected
                    .filter(|section| section_swap_family(&section.config, offered).is_ok())
                    .map(|section| {
                        let code = design_section_code(resolved.design, &section.id)
                            .expect("a live design section has a code");
                        let line = match resolved
                            .plan_swap(&items, section, offered, own, own_cr.0, partner)
                        {
                            Ok(payment) => {
                                format!("Swap {code}: {}", rebuild_price_text(&payment))
                            }
                            Err(refusal) => swap_refusal_text(&items, &code, &name, &refusal),
                        };
                        (section.id.clone(), line)
                    });
                let held = runtime.held.as_deref() == Some(offered.base.id.as_str());
                if held {
                    header = format!("Holding {name}: click the ship to place it");
                }
                rows.push(CatalogueRow {
                    prototype: offered.base.id.clone(),
                    name,
                    line,
                    held,
                    swap,
                });
            }
            if rows.is_empty() {
                rows.push(CatalogueRow {
                    prototype: String::new(),
                    name: String::new(),
                    line: EMPTY_LINE.to_string(),
                    held: false,
                    swap: None,
                });
            }
        }
    }
    let display = if open { Display::Flex } else { Display::None };
    if list.display != display {
        list.display = display;
    }
    if let Ok(mut text) = q_header.single_mut() {
        if text.0 != header {
            text.0 = header;
        }
    }
    if shown.0 == rows {
        return;
    }
    commands.entity(rows_entity).despawn_children();
    commands.entity(rows_entity).with_children(|parent| {
        for row in &rows {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|cell| {
                    if row.prototype.is_empty() {
                        cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                        return;
                    }
                    cell.spawn(themed_label(&row.name, 13.0, UiColor::Primary));
                    cell.spawn(themed_label(&row.line, 12.0, UiColor::Body));
                    if let Some((_, line)) = &row.swap {
                        cell.spawn(themed_label(line, 12.0, UiColor::Body));
                    }
                    cell.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|buttons| {
                        let place = if row.held {
                            ButtonSpec::new("Put down").fit()
                        } else {
                            ButtonSpec::new("Place").fit().primary()
                        };
                        buttons
                            .spawn((
                                Name::new(format!("ShipPlace-{}", row.prototype)),
                                compact_button(place),
                            ))
                            .observe(on_ship_place_button(row.prototype.clone()));
                        if let Some((section, _)) = &row.swap {
                            buttons
                                .spawn((
                                    Name::new(format!("ShipSwap-{}", row.prototype)),
                                    compact_button(ButtonSpec::new("Swap").fit().primary()),
                                ))
                                .observe(on_ship_swap_button(
                                    section.clone(),
                                    row.prototype.clone(),
                                ));
                        }
                    });
                });
        }
    });
    shown.0 = rows;
}

/// Put catalogue part `prototype` in hand when its Place button is clicked,
/// or put it down when it already is.
fn on_ship_place_button(
    prototype: String,
) -> impl FnMut(On<Activate>, ResMut<ShipRuntime>, Option<Res<SoundBank<UiSfx>>>, Commands) {
    move |_activate, mut runtime, bank, mut commands| {
        runtime.held = if runtime.held.as_deref() == Some(prototype.as_str()) {
            None
        } else {
            Some(prototype.clone())
        };
        play_menu_select(&mut commands, bank.as_deref());
    }
}
