//! The Ship pane's side panel layout and the handler for the repair action
//! its key and panel button route into `SectionRepairCommand`.
//!
//! Touch this module when changing the ship panel or what the repair action
//! does.

use bevy::{
    prelude::{
        default, BackgroundColor, BorderRadius, ChildOf, ChildSpawnerCommands, Color, Display,
        Entity, FlexDirection, FlexWrap, Has, Justify, JustifyContent, LineBreak, MessageReader,
        Name, Node, Overflow, Query, ResMut, TextLayout, Val, With,
    },
    ui_widgets::{Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick},
};
use nova_gameplay::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{button, slider_track, text_field, ButtonSpec, TextFieldSpec, ThemedFill},
};

use super::{scene::*, sections::*};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{
        compact_button, control_row, divider, panel_preview_frame, side_panel, themed_label,
        PANEL_PREVIEW_PX, SUMMARY_LINES,
    },
};

/// Build the section panel: the selected section's icon over its code, name,
/// status and condition bar; its description; its labelled facts; Prev, Next
/// and Rebind; the repair form; and the note line, beside the view.
/// [`update_ship_panel`] fills it. The texts carry a [`ShipPanelField`] so one
/// system refreshes them, and a fact row or the form carries the field it
/// holds so the same system hides it. The buttons carry a [`ShipPanelButton`]
/// and route through the [`SectionRepairCommand`] seam via `Activate`
/// observers.
pub(crate) fn spawn_ship_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent
        .spawn((ShipPanelMarker, side_panel()))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|head| {
                    head.spawn(panel_preview_frame()).with_children(|frame| {
                        frame.spawn((
                            ShipPreviewIcon,
                            icon_node(
                                icons.section(SectionIconType::Hull),
                                SectionIconType::Hull.color(),
                                PANEL_PREVIEW_PX * 0.75,
                            ),
                        ));
                    });
                    head.spawn(Node {
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|detail| {
                        detail.spawn((
                            ShipPanelField::Title,
                            themed_label("", 16.0, UiColor::Primary),
                        ));
                        detail.spawn((
                            ShipPanelField::Status,
                            themed_label("", 12.0, UiColor::Body),
                        ));
                        detail
                            .spawn((
                                Node {
                                    height: Val::Px(6.0),
                                    flex_shrink: 0.0,
                                    border_radius: BorderRadius::all(Val::Px(3.0)),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                ThemedFill::alpha(UiColor::Secondary, 0.25),
                            ))
                            .with_children(|track| {
                                track.spawn((
                                    ShipConditionFill,
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    ThemedFill::new(UiColor::Nominal),
                                ));
                            });
                    });
                });
            panel.spawn(divider());
            panel.spawn((ShipPanelField::About, themed_label("", 12.0, UiColor::Body)));
            panel.spawn(divider());
            // The Inventory inspector's fact rows: the word on the left and
            // the value, clipped on one line, on the right.
            for (label, field) in [
                ("Integrity", ShipPanelField::Integrity),
                ("Ammunition", ShipPanelField::Ammo),
                ("Control", ShipPanelField::Control),
            ] {
                panel
                    .spawn((field, control_row(JustifyContent::SpaceBetween)))
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
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    for (label, step) in [("Prev", -1), ("Next", 1)] {
                        row.spawn((
                            button(ButtonSpec::new(label).fit()),
                            Name::new(format!("Ship{label}")),
                        ))
                        .observe(on_ship_step_button(step));
                    }
                    row.spawn((
                        ShipPanelButton::Rebind,
                        button(ButtonSpec::new("Rebind").fit()),
                    ))
                    .observe(on_ship_rebind_button);
                });
            panel
                .spawn((
                    ShipPanelField::RepairForm,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ))
                .with_children(|form| {
                    form.spawn(divider());
                    form.spawn(themed_label("Repair", 13.0, UiColor::Accent));
                    form.spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(6.0),
                        ..control_row(JustifyContent::FlexStart)
                    })
                    .with_children(|row| {
                        row.spawn(Node {
                            width: Val::Px(72.0),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|cell| {
                            cell.spawn((
                                ShipRepairQuantity,
                                text_field(TextFieldSpec::new("1").max_chars(5).dense()),
                            ));
                        });
                        row.spawn((
                            ShipPanelField::RepairStock,
                            themed_label("", 15.0, UiColor::Primary),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            TextLayout::new(Justify::Left, LineBreak::NoWrap),
                        ));
                        row.spawn((
                            Name::new("ShipRepairAll"),
                            compact_button(ButtonSpec::new("All").fit().ghost()),
                        ))
                        .observe(on_ship_repair_all_button);
                    });
                    form.spawn((
                        ShipRepairQuantity,
                        Slider {
                            track_click: TrackClick::Snap,
                            ..default()
                        },
                        SliderValue(1.0),
                        SliderRange::new(1.0, 1.0),
                        SliderStep(1.0),
                        SliderPrecision(0),
                        slider_track(0.0),
                    ))
                    .observe(on_ship_repair_slider);
                    form.spawn((
                        ShipPanelField::RepairPreview,
                        themed_label("", 13.0, UiColor::Body),
                        Node {
                            min_height: Val::Px(13.0 * 1.2 * SUMMARY_LINES),
                            ..default()
                        },
                    ));
                    form.spawn(control_row(JustifyContent::FlexStart))
                        .with_children(|row| {
                            row.spawn((
                                Name::new("ShipRepair"),
                                ShipPanelButton::Repair,
                                compact_button(ButtonSpec::new("Repair").fit().primary()),
                            ))
                            .observe(on_ship_repair_button);
                        });
                });
            panel.spawn((ShipPanelField::Note, themed_label("", 12.0, UiColor::Label)));
        });
}

/// Apply in-app [`SectionRepairCommand`] messages (the `P` action key and the
/// panel button) to player-ship sections, and flash the result on the panel
/// note line. A target that is not a live section of the player ship is
/// skipped with no note: every writer takes it from the pane's own list.
pub(crate) fn apply_ship_section_commands(
    mut messages: MessageReader<SectionRepairCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut q_player: Query<
        (Entity, Option<&mut ShipInventory>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_view: Query<(&ChildOf, &SectionCode, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
    mut q_health: Query<&mut Health>,
) {
    let Ok((player, inventory)) = q_player.single_mut() else {
        messages.clear();
        return;
    };
    let mut inventory = inventory.expect("the player ship carries a ShipInventory");
    for command in messages.read() {
        let Ok((child, code, disabled)) = q_view.get(command.target) else {
            continue;
        };
        if child.0 != player {
            continue;
        }
        let row = repair_section(
            &code.0,
            q_health.get_mut(command.target).ok().as_deref_mut(),
            disabled,
            command.requested_plates,
            &mut inventory,
        );
        runtime.note = Some((row.text, 2.5));
    }
}
