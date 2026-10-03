//! The Ship pane's side panel layout and the handler for the repair action
//! its key and panel button route into `SectionRepairCommand`.
//!
//! Touch this module when changing the ship panel or what the repair action
//! does.

use bevy::{
    prelude::{
        default, AlignItems, BackgroundColor, BorderRadius, ChildOf, ChildSpawnerCommands, Color,
        Entity, FlexDirection, FlexWrap, Has, MessageReader, Name, Node, Overflow, Query, ResMut,
        Val, With,
    },
    ui_widgets::{Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick},
};
use nova_gameplay::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{button, slider_track, ButtonSpec, ThemedFill},
};

use super::{scene::*, sections::*};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{panel_preview_frame, side_panel, themed_label, PANEL_PREVIEW_PX},
};

/// Build the section panel: the selected section's icon over its code, name,
/// status and condition bar, its detail, Prev and Next, the Repair
/// and Rebind buttons, and the note line, beside the view.
/// [`update_ship_panel`] fills it. The texts carry a
/// [`ShipPanelField`] so one system refreshes them; the buttons carry a
/// [`ShipPanelButton`] and route through the [`SectionRepairCommand`] seam via
/// `Activate` observers.
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
            panel.spawn((
                ShipPanelField::Detail,
                themed_label("", 12.0, UiColor::Body),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
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
                });
            panel.spawn((
                ShipPanelField::RepairPreview,
                themed_label("", 12.0, UiColor::Body),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        ShipPanelField::RepairQuantity,
                        themed_label("", 13.0, UiColor::Primary),
                    ));
                    row.spawn(button(ButtonSpec::new("All").fit()))
                        .observe(on_ship_repair_all_button);
                });
            panel
                .spawn((
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
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(8.0),
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        ShipPanelButton::Repair,
                        button(ButtonSpec::new("Repair").fit()),
                    ))
                    .observe(on_ship_repair_button);
                    row.spawn((
                        ShipPanelButton::Rebind,
                        button(ButtonSpec::new("Rebind").fit()),
                    ))
                    .observe(on_ship_rebind_button);
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
