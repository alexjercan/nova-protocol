//! The Ship pane's side panel layout and the handler that starts a plate
//! repair job from the `SectionRepairCommand` its key and panel button raise.
//!
//! Touch this module when changing the ship panel or how a repair starts.

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
use nova_gameplay::prelude::*;
use nova_ship::prelude::{RepairJobKey, ShipRepairJobs};
use nova_sound::prelude::*;
use nova_ui::{
    prelude::{
        control_row, divider, inspector, metrics, preview_frame, text, text_toned, TextRole,
    },
    screen::drive_wheel_scroll,
    theme::UiColorType,
    widget::{
        button, slider_track, text_field, ButtonCueType, ButtonSpec, TextFieldSpec, ThemedFill,
        ThemedRadius, Tooltip,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use super::rescue::spawn_ship_rescue;
use super::{
    delete::on_ship_delete_button,
    held::{spawn_ship_held_block, ShipSelectionBlock},
    rebuild::spawn_ship_rebuild_list,
    repair_jobs::spawn_ship_repair_jobs,
    scene::*,
    sections::*,
};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::SUMMARY_LINES,
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
/// while it is refused; the note line; the repair form; the Repairs block
/// with the running jobs; and the lost-section list, beside the view. While a part is in hand the
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
            inspector(Overflow::scroll_y()),
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
                        head.spawn(preview_frame()).with_children(|frame| {
                            frame.spawn((
                                ShipPreviewIcon,
                                icon_node(
                                    icons.section(SectionIconType::Hull),
                                    SectionIconType::Hull.color(),
                                    metrics::PREVIEW * 0.75,
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
                            detail.spawn((ShipPanelField::Title, text("", TextRole::Title)));
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
                            text("Control", TextRole::Label),
                            Node {
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                        fact.spawn((
                            ShipPanelField::Control,
                            Name::new("ShipControl"),
                            text("", TextRole::Value),
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
                            button(ButtonSpec::control("Rebind")),
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
                        block.spawn(text("Salvage", TextRole::Label));
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
                                    button(ButtonSpec::control("Delete").danger()),
                                    Tooltip::default(),
                                    Name::new("ShipDelete"),
                                ))
                                .observe(on_ship_delete_button);
                            });
                    });
                // Under the actions: a result shows where the press was.
                panel.spawn((ShipPanelField::Note, text("", TextRole::Label)));
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
                        form.spawn(text("Repair", TextRole::Heading));
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
                                    text_field(TextFieldSpec::new("1").max_chars(5).tool()),
                                ));
                            });
                            row.spawn((
                                ShipPanelField::RepairStock,
                                text("", TextRole::Value),
                                Node {
                                    flex_grow: 1.0,
                                    ..default()
                                },
                                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                            ));
                            row.spawn((
                                Name::new("ShipRepairAll"),
                                button(ButtonSpec::control("All").ghost()),
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
                            text("", TextRole::Body),
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
                                    button(ButtonSpec::control("Repair").primary()),
                                    ButtonCueType::Owned,
                                ))
                                .observe(on_ship_repair_button);
                            });
                    });
                spawn_ship_repair_jobs(panel);
                #[cfg(not(target_arch = "wasm32"))]
                spawn_ship_rescue(panel);
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
            text_toned("", TextRole::Title, UiColorType::Accent),
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
/// panel button) to player-ship sections: start a plate repair job by
/// [`start_plate_repair`], or flash its refusal on the panel note line. A
/// target that is not a live section of the player ship is skipped with no
/// note: every writer takes it from the pane's own list.
#[expect(
    clippy::type_complexity,
    reason = "the player ship's hold and jobs beside the section reads"
)]
pub(crate) fn apply_ship_section_commands(
    mut messages: MessageReader<SectionRepairCommand>,
    mut runtime: ResMut<ShipRuntime>,
    mut q_player: Query<
        (Entity, Option<&ShipInventory>, Option<&mut ShipRepairJobs>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_view: Query<
        (
            &ChildOf,
            &SectionCode,
            Option<&Health>,
            Has<IntegrityDisabledMarker>,
        ),
        With<SectionMarker>,
    >,
    bank: Option<Res<SoundBank<UiSfx>>>,
    mut commands: Commands,
) {
    let Ok((player, inventory, jobs)) = q_player.single_mut() else {
        messages.clear();
        return;
    };
    let inventory = inventory.expect("the player ship carries a ShipInventory");
    // The jobs this frame starts, on top of the ones already running, so two
    // commands in one frame see each other.
    let mut running = jobs.as_deref().cloned().unwrap_or_default();
    let mut started = Vec::new();
    for command in messages.read() {
        let Ok((child, code, health, disabled)) = q_view.get(command.target) else {
            continue;
        };
        if child.0 != player {
            continue;
        }
        let free_plates = running
            .free_hold(inventory)
            .count(&ItemDesignId::from(ItemRoleType::HullPlate));
        match start_plate_repair(
            &code.0,
            command.target,
            health,
            disabled,
            running
                .get(&RepairJobKey::Section(command.target))
                .is_some(),
            command.requested_plates,
            free_plates,
        ) {
            // The new job row in the Repairs block shows a start; an
            // earlier refusal's note goes.
            Ok(job) => {
                runtime.note = None;
                running.push(job.clone());
                started.push(job);
                play_menu_select(&mut commands, bank.as_deref());
            }
            Err(row) => {
                play_menu_deny(&mut commands, bank.as_deref());
                runtime.note = Some((row.text, 2.5));
            }
        }
    }
    if started.is_empty() {
        return;
    }
    match jobs {
        Some(mut jobs) => jobs.0.extend(started),
        None => {
            commands.entity(player).insert(ShipRepairJobs(started));
        }
    }
}
