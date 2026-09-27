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
    widget::{ButtonSpec, ThemedBorder, ThemedFill},
};

use super::{scene::*, sections::*};
use crate::{
    icons::{icon_node, InterfaceIcons, SectionIconType},
    pane::themed_label,
    terminal::section_kind_from_markers,
};

/// Fixed width of the section panel beside the ship view, in logical px.
const SHIP_PANEL_PX: f32 = 300.0;

/// Build the section panel: the selected section's icon, code, name, status
/// and condition bar, its detail, the Repair, Reload and Rebind buttons, and
/// the note line. [`update_ship_panel`] fills it. The texts carry a
/// [`ShipPanelField`] so one system refreshes them; the buttons carry a
/// [`ShipPanelButton`] and route through the [`ShipSectionCommand`] seam via
/// `Activate` observers.
pub(crate) fn spawn_ship_panel(parent: &mut ChildSpawnerCommands, icons: &InterfaceIcons) {
    parent
        .spawn((
            ShipPanelMarker,
            Node {
                width: Val::Px(SHIP_PANEL_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ThemedFill::alpha(UiColor::Secondary, 0.08),
            BorderColor::all(Color::NONE),
            ThemedBorder::new(UiColor::Secondary),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|head| {
                    head.spawn((
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            flex_shrink: 0.0,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        ThemedFill::alpha(UiColor::Surface, 0.6),
                        BorderColor::all(Color::NONE),
                        ThemedBorder::new(UiColor::Secondary),
                    ))
                    .with_children(|frame| {
                        frame.spawn((
                            ShipPreviewIcon,
                            icon_node(
                                icons.section(SectionIconType::Hull),
                                SectionIconType::Hull.color(),
                                48.0,
                            ),
                        ));
                    });
                    head.spawn(Node {
                        flex_grow: 1.0,
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
