//! The Ship pane's side panel layout and the handler for the section actions
//! its keys and buttons route into `ShipSectionCommand`.
//!
//! Touch this module when changing the ship panel or what a section action
//! does.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;
use nova_ui::{
    prelude::*,
    theme::UiColor,
    widget::{ButtonSpec, ThemedFill},
};

use super::{scene::*, sections::*};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::{panel_preview_frame, side_panel, themed_label, PANEL_PREVIEW_PX},
    terminal::section_kind_from_markers,
};

/// Build the section panel: the selected section's icon over its code, name,
/// status and condition bar, its detail, Prev and Next, the Repair, Reload
/// and Rebind buttons, and the note line, beside the view.
/// [`update_ship_panel`] fills it. The texts carry a
/// [`ShipPanelField`] so one system refreshes them; the buttons carry a
/// [`ShipPanelButton`] and route through the [`ShipSectionCommand`] seam via
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
                        ShipPanelButton::Reload,
                        button(ButtonSpec::new("Reload").fit()),
                    ))
                    .observe(on_ship_reload_button);
                    row.spawn((
                        ShipPanelButton::Rebind,
                        button(ButtonSpec::new("Rebind").fit()),
                    ))
                    .observe(on_ship_rebind_button);
                });
            panel.spawn((ShipPanelField::Note, themed_label("", 12.0, UiColor::Label)));
        });
}

/// Apply in-app [`ShipSectionCommand`] messages (the `L`/`P` action keys and the
/// panel buttons), and flash the result on the panel note line.
pub(crate) fn apply_ship_section_commands(
    mut messages: MessageReader<ShipSectionCommand>,
    mut runtime: ResMut<ShipRuntime>,
    q_view: Query<(
        &SectionCode,
        Option<&SectionClass>,
        Has<HullSectionMarker>,
        Has<ControllerSectionMarker>,
        Has<ThrusterSectionMarker>,
        Has<TurretSectionMarker>,
        Has<TorpedoSectionMarker>,
    )>,
    mut q_health: Query<&mut Health>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    for command in messages.read() {
        let Ok((code, class, hull, controller, thruster, turret, torpedo)) =
            q_view.get(command.target)
        else {
            continue;
        };
        let Some(kind) =
            section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)
        else {
            continue;
        };
        let is_weapon = kind.is_weapon();
        let mut health = q_health.get_mut(command.target).ok();
        let mut ammo = q_ammo.get_mut(command.target).ok();
        let row = apply_action_to_section(
            command.action,
            &code.0,
            kind,
            is_weapon,
            health.as_deref_mut(),
            ammo.as_deref_mut(),
        );
        runtime.note = Some((row.text, 2.5));
    }
}
