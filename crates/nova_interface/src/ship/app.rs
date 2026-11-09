//! The Ship pane's side panel layout and the handler for the repair action
//! its key and panel button route into `SectionRepairCommand`.
//!
//! Touch this module when changing the ship panel or what the repair action
//! does.

use bevy::{
    input::mouse::MouseWheel,
    picking::hover::Hovered,
    prelude::{
        default, BackgroundColor, ChildOf, ChildSpawnerCommands, Color, Commands, Component,
        ComputedNode, DetectChangesMut, Display, Entity, FlexDirection, FlexWrap, Has, Justify,
        JustifyContent, LineBreak, MessageReader, Name, Node, Overflow, Pickable, PositionType,
        Query, Res, ResMut, ScrollPosition, TextLayout, UiRect, Val, Visibility, With, ZIndex,
    },
    ui::Pressed,
    ui_widgets::{
        Button, Slider, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick,
    },
};
use nova_command::prelude::TerminalRowKind;
use nova_gameplay::prelude::*;
use nova_ui::{
    screen::drive_wheel_scroll,
    theme::UiColorType,
    widget::{
        button, slider_track, text_field, ButtonCueType, ButtonSpec, TextFieldSpec, ThemedFill,
        ThemedRadius, Tooltip,
    },
};

use super::{
    delete::on_ship_delete_button,
    held::{spawn_ship_held_block, ShipSelectionBlock},
    rebuild::spawn_ship_rebuild_list,
    scene::*,
    sections::*,
};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{
        compact_button, control_row, divider, panel_preview_frame, side_panel, themed_label,
        PANEL_PREVIEW_PX, SUMMARY_LINES,
    },
};

/// Wheel-scroll the section panel while the pointer is over it, through the
/// shared driver; `ship_input` leaves the camera zoom alone then. A wheel
/// notch away from the panel is the camera's.
pub(crate) fn scroll_ship_panel(
    mut wheel: MessageReader<MouseWheel>,
    q_panel: Query<
        (&mut ScrollPosition, Option<&ComputedNode>, Option<&Hovered>),
        With<ShipPanelMarker>,
    >,
) {
    if !q_panel
        .iter()
        .any(|(_, _, hovered)| hovered.is_some_and(Hovered::get))
    {
        wheel.clear();
        return;
    }
    drive_wheel_scroll(wheel, q_panel);
}

/// Build the section panel: the selected section's icon over its code, name
/// and condition bar; its binding; Rebind; the Salvage block with what Delete
/// gives back over the Delete button, muted with the reason as its tooltip
/// while it is refused; the note line; the repair form;
/// and the lost-section list, beside the view. While a part is in hand the
/// Placing block (`super::held`) stands in place of all of it. The parts a station builds are
/// in the gallery the footer's Catalogue button opens (`super::catalogue`).
/// [`update_ship_panel`] fills it. The texts carry a [`ShipPanelField`] so one
/// system refreshes them, and the binding row or the form carries the field
/// it holds so the same system hides it. The buttons carry a
/// [`ShipPanelButton`] and route through their command seams via `Activate`
/// observers.
pub(crate) fn spawn_ship_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent
        // Scrolls: a ship with many lost sections runs past a 1080p panel.
        .spawn((
            ShipPanelMarker,
            Name::new("ShipPanel"),
            side_panel(Overflow::scroll_y()),
            ScrollPosition::default(),
            Hovered::default(),
        ))
        .with_children(|root| {
            spawn_ship_held_block(root, icons);
            root.spawn((
                ShipSelectionBlock,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
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
                                themed_label("", 16.0, UiColorType::Text),
                            ));
                            detail
                                .spawn((
                                    Node {
                                        height: Val::Px(6.0),
                                        flex_shrink: 0.0,
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    ThemedFill::alpha(UiColorType::TextSecondary, 0.25),
                                    ThemedRadius::control(),
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
                                        ThemedFill::new(UiColorType::Nominal),
                                    ));
                                });
                        });
                    });
                panel.spawn(divider());
                // The Inventory inspector's fact row: the word on the left and
                // the value, clipped on one line, on the right.
                panel
                    .spawn((
                        ShipPanelField::Control,
                        control_row(JustifyContent::SpaceBetween),
                    ))
                    .with_children(|fact| {
                        fact.spawn((
                            themed_label("Control", 12.0, UiColorType::TextMuted),
                            Node {
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                        fact.spawn((
                            ShipPanelField::Control,
                            Name::new("ShipControl"),
                            themed_label("", 13.0, UiColorType::Text),
                            TextLayout::new(Justify::Right, LineBreak::NoWrap),
                            Node {
                                min_width: Val::Px(0.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ));
                    });
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
                            ShipPanelButton::Rebind,
                            button(ButtonSpec::new("Rebind").fit()),
                        ))
                        .observe(on_ship_rebind_button);
                    });
                // The Salvage block is Delete's form, as the repair form is
                // Repair's: the yield tiles, then the action. The button sits
                // at the block's left edge so its 240 px tooltip stays inside
                // the 300 px panel at every window size.
                panel
                    .spawn((
                        ShipPanelField::Salvage,
                        Name::new("ShipSalvage"),
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|block| {
                        block.spawn(divider());
                        block.spawn(themed_label("Salvage", 12.0, UiColorType::TextMuted));
                        // One part per line, so two parts never read as one.
                        block.spawn((
                            ShipSalvageTiles::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(4.0),
                                ..default()
                            },
                        ));
                        block
                            .spawn(control_row(JustifyContent::FlexStart))
                            .with_children(|row| {
                                row.spawn((
                                    ShipPanelButton::Delete,
                                    compact_button(ButtonSpec::new("Delete").fit().danger()),
                                    Tooltip::default(),
                                    Name::new("ShipDelete"),
                                ))
                                .observe(on_ship_delete_button);
                            });
                    });
                // Under the actions: a result shows where the press was.
                panel.spawn((
                    ShipPanelField::Note,
                    themed_label("", 12.0, UiColorType::TextMuted),
                ));
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
                        form.spawn(themed_label("Repair", 13.0, UiColorType::Accent));
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
                                themed_label("", 15.0, UiColorType::Text),
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
                            themed_label("", 13.0, UiColorType::TextBody),
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
                                    ButtonCueType::Owned,
                                ))
                                .observe(on_ship_repair_button);
                            });
                    });
                spawn_ship_rebuild_list(panel);
            });
        });
}

/// The banner across the top of the view while a rebind is armed, hidden
/// until then. [`update_ship_panel`] names the section in it.
pub(crate) fn spawn_ship_rebind_prompt(view: &mut ChildSpawnerCommands) {
    view.spawn((
        ShipPanelField::RebindPrompt,
        Name::new("ShipRebindPrompt"),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        // Above the section badges, its siblings.
        ZIndex(2),
    ))
    .with_children(|row| {
        row.spawn((
            ShipPanelField::RebindPrompt,
            themed_label("", 14.0, UiColorType::Accent),
            Node {
                padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColorType::Background, 0.85),
            Pickable::IGNORE,
        ));
    });
}

/// The invisible cover over the whole interface while a section rebind owns the
/// mouse. Bevy picking sends a click to the panel whatever `apply_ship_rebind`
/// reads off `ButtonInput`, so without it the click that binds Left Mouse would
/// also fire the Delete button, tab or blip under the pointer on its release.
#[derive(Component, Debug)]
pub(crate) struct ShipRebindClickGuard;

/// Spawn the [`ShipRebindClickGuard`] under the interface root, above the card.
/// It takes no hover, so a blocked click has no target at all.
pub(crate) fn spawn_ship_rebind_click_guard(root: &mut ChildSpawnerCommands) {
    root.spawn((
        ShipRebindClickGuard,
        Name::new("ShipRebindClickGuard"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        Visibility::Hidden,
        Pickable {
            should_block_lower: true,
            is_hoverable: false,
        },
        ZIndex(1),
    ));
}

/// Show the [`ShipRebindClickGuard`] for as long as [`ShipRuntime::rebind_armed`]
/// holds. That covers the release of the captured click: `rebind_swallow` holds
/// until every button is up, and picking reads that release first.
///
/// A press already down when a key arms the capture needs more than the guard.
/// Bevy aims a click at the previous frame's hover, so the release one frame
/// later still reaches that button, and a later release reaches nothing and
/// leaves the button pressed. Taking [`Pressed`] off answers both: a click on a
/// button without it does not activate.
pub(crate) fn sync_ship_rebind_click_guard(
    runtime: Res<ShipRuntime>,
    mut commands: Commands,
    mut q_guard: Query<&mut Visibility, With<ShipRebindClickGuard>>,
    q_pressed: Query<Entity, (With<Button>, With<Pressed>)>,
) {
    let armed = runtime.rebind_armed();
    let shown = if armed {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut q_guard {
        visibility.set_if_neq(shown);
    }
    if armed {
        for button in &q_pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

/// Apply in-app [`SectionRepairCommand`] messages (the `P` action key and the
/// panel button) to player-ship sections, and flash the result on the panel
/// note line. A target that is not a live section of the player ship is
/// skipped with no note: every writer takes it from the pane's own list.
pub(crate) fn apply_ship_section_commands(
    mut messages: MessageReader<SectionRepairCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut q_player: Query<
        (
            Entity,
            Option<&mut ShipInventory>,
            &mut ShipInventoryReports,
        ),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_view: Query<(&ChildOf, &SectionCode, Has<IntegrityDisabledMarker>), With<SectionMarker>>,
    mut q_health: Query<&mut Health>,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok((player, inventory, mut reports)) = q_player.single_mut() else {
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
            &mut reports,
        );
        match row.kind {
            TerminalRowKind::Error => play_menu_deny(&mut commands, bank.as_deref()),
            _ => play_menu_select(&mut commands, bank.as_deref()),
        }
        runtime.note = Some((row.text, 2.5));
    }
}
