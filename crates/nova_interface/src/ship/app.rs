//! The Ship pane's side panel layout and the handler that starts a plate
//! repair job from the `SectionRepairCommand` its key and panel button raise.
//!
//! Touch this module when changing the ship panel or how a repair starts.

use bevy::{
    input::mouse::MouseWheel,
    picking::hover::Hovered,
    prelude::{
        default, BackgroundColor, ChildOf, ChildSpawnerCommands, Color, Commands, Component,
        ComputedNode, DetectChangesMut, Display, Entity, FlexDirection, Has, JustifyContent,
        MessageReader, Name, Node, Overflow, Pickable, PositionType, Query, Res, ScrollPosition,
        UiRect, Val, Visibility, With, ZIndex,
    },
    ui::Pressed,
    ui_widgets::Button,
};
use nova_gameplay::prelude::*;
use nova_ship::prelude::{RepairJobKey, ShipRepairJobs};
use nova_sound::prelude::*;
use nova_ui::{
    prelude::{
        amount_row, bar, inspector, key_chip, metrics, preview_frame, spawn_block, tag, text,
        text_toned, verb_row, AmountSpec, RowAlign, TextRole, VerbGate,
    },
    screen::drive_wheel_scroll,
    theme::UiColorType,
    widget::{button, ButtonCueType, ButtonSpec, ThemedFill},
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
use crate::icons::{icon_node, InterfaceIcons, SectionIconType};

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

/// Build the section panel beside the view: the selected section's icon over
/// its code, name and condition bar, then one kit block per thing the
/// selection can do, each with its verbs at its bottom right. Control holds
/// the binding and Rebind; Repair the amount row, the summary and Repair;
/// then the Repairs block with the running jobs, the Distress block, the
/// lost sections, and Salvage with what Delete gives back over Delete. While
/// a part is in hand the Placing block (`super::held`) stands in place of all
/// of it. The parts a station builds are in the gallery the footer's
/// Catalogue button opens (`super::catalogue`). [`update_ship_panel`] fills
/// it: a block that cannot work here hides and takes no space, and each verb
/// carries a [`VerbGate`]. The buttons carry a [`ShipPanelButton`] and route
/// through their command seams via `Activate` observers.
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
                    row_gap: Val::Px(metrics::GAP_L),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(metrics::GAP),
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
                        head.spawn((ShipPanelField::Title, text("", TextRole::Title)));
                        head.spawn((ShipConditionBar, bar(1.0, UiColorType::Nominal)));
                    });
                spawn_block(panel, "Control", ShipPanelField::Control).with_children(|block| {
                    block.spawn((
                        ShipPanelField::Control,
                        Name::new("ShipControl"),
                        text("", TextRole::Value),
                    ));
                    block.spawn(verb_row(RowAlign::End)).with_children(|row| {
                        row.spawn((
                            ShipPanelButton::Rebind,
                            Name::new("ShipRebind"),
                            button(ButtonSpec::control("Rebind")),
                            VerbGate::Hidden,
                        ))
                        .observe(on_ship_rebind_button);
                    });
                });
                spawn_block(panel, "Repair", ShipPanelField::RepairForm).with_children(|form| {
                    form.spawn((
                        ShipRepairQuantity,
                        amount_row(AmountSpec { name: "ShipRepair" }),
                    ))
                    .observe(on_ship_repair_amount);
                    form.spawn((ShipPanelField::RepairPreview, text("", TextRole::Body)));
                    form.spawn(verb_row(RowAlign::End)).with_children(|row| {
                        row.spawn((
                            Name::new("ShipRepair"),
                            ShipPanelButton::Repair,
                            button(ButtonSpec::control("Repair").primary()),
                            VerbGate::Disabled,
                            ButtonCueType::Owned,
                        ))
                        .observe(on_ship_repair_button);
                    });
                });
                spawn_ship_repair_jobs(panel);
                #[cfg(not(target_arch = "wasm32"))]
                spawn_ship_rescue(panel);
                spawn_ship_rebuild_list(panel);
                // Delete's block: the parts it gives back, one per line so two
                // never read as one, over Delete. Last, as the one verb that
                // takes a section away.
                spawn_block(
                    panel,
                    "Salvage",
                    (ShipPanelField::Salvage, Name::new("ShipSalvage")),
                )
                .with_children(|block| {
                    block.spawn((
                        ShipSalvageTiles::default(),
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(metrics::GAP_S),
                            ..default()
                        },
                    ));
                    block.spawn(verb_row(RowAlign::End)).with_children(|row| {
                        row.spawn((
                            ShipPanelButton::Delete,
                            Name::new("ShipDelete"),
                            button(ButtonSpec::control("Delete").danger()),
                            VerbGate::Hidden,
                        ))
                        .observe(on_ship_delete_button);
                    });
                });
            });
        });
}

/// The banner across the top of the view while a rebind is armed, hidden
/// until then: `Bind` and the section, the keys that end the capture, and
/// the tag of a refused source. [`update_ship_rebind_prompt`] fills it.
pub(crate) fn spawn_ship_rebind_prompt(view: &mut ChildSpawnerCommands) {
    view.spawn((
        ShipPanelField::RebindPrompt,
        Name::new("ShipRebindPrompt"),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: Val::Px(metrics::GAP_L),
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
            Node {
                column_gap: Val::Px(metrics::GAP),
                align_items: bevy::prelude::AlignItems::Center,
                height: Val::Px(metrics::CONTROL_H),
                padding: UiRect::axes(Val::Px(metrics::GAP), Val::Px(0.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColorType::Background, 0.85),
            Pickable::IGNORE,
        ))
        .with_children(|banner| {
            banner.spawn((
                ShipPanelField::RebindPrompt,
                text_toned("", TextRole::Value, UiColorType::Accent),
            ));
            for word in [REBIND_IN_USE, REBIND_RESERVED] {
                banner.spawn((ShipRebindRefusal(word), tag(word, UiColorType::Danger)));
            }
            banner.spawn((ShipRebindDriveKey, key_chip("Backspace")));
            banner.spawn(key_chip("Esc"));
        });
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
/// [`start_plate_repair`], or buzz a refusal: the disabled Repair and its
/// Danger "/ max" already show why. A target that is not a live section of
/// the player ship is skipped: every writer takes it from the pane's own
/// list.
#[expect(
    clippy::type_complexity,
    reason = "the player ship's hold and jobs beside the section reads"
)]
pub(crate) fn apply_ship_section_commands(
    mut messages: MessageReader<SectionRepairCommand>,
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
            // The new job row in the Repairs block shows a start.
            Some(job) => {
                running.push(job.clone());
                started.push(job);
                play_menu_select(&mut commands, bank.as_deref());
            }
            None => play_menu_deny(&mut commands, bank.as_deref()),
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
