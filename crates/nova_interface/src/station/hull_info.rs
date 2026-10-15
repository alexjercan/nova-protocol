//! A card's Info button: the hull for sale or the escort for hire shown in
//! the Ship pane's view, read-only.
//!
//! Info sets [`ShipViewSubject`] to the card's hull and switches to the Ship
//! pane, which then draws that hull's sections in place of the player
//! ship's: right-drag turns it, the wheel zooms, and a click selects a
//! section to show its name and condition. The Ship pane takes no edit while
//! the subject is set (see [`ShipViewSubject`]). A header over the Ship panel
//! holds the card's line and Close, which goes back to the Station pane.
//! Leaving the Ship pane by any way clears the subject, so the Ship tab shows
//! the player ship again.
//!
//! Touch this module when changing what Info shows or how it closes.

use bevy::{prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::*;
use nova_ui::{theme::UiColor, widget::ButtonSpec};

use crate::{
    pane::{compact_button, interface_shown, play_menu_select, themed_label, InterfacePaneType},
    ship::{ShipPanelMarker, ShipViewSubject},
    terminal::NovaOsCloseTransition,
};

/// The name of the header's Close button.
pub const HULL_INFO_CLOSE: &str = "HullInfoClose";

/// The card line of the hull Info shows, for the header over the Ship panel.
#[derive(Resource, Default, Debug)]
pub(crate) struct HullInfoLine(String);

/// The header Info puts over the Ship panel.
#[derive(Component)]
pub(crate) struct HullInfoHeader;

/// Spawn a card's Info button for `hull`, whose card reads `line`.
pub(crate) fn spawn_hull_info_button(
    row: &mut ChildSpawnerCommands,
    name: String,
    hull: Entity,
    line: String,
) {
    row.spawn((
        Name::new(name),
        compact_button(ButtonSpec::new("Info").fit()),
    ))
    .observe(
        move |_activate: On<Activate>,
              mut subject: ResMut<ShipViewSubject>,
              mut shown: ResMut<HullInfoLine>,
              mut pane: ResMut<InterfacePaneType>,
              bank: Option<Res<SoundBank<UiSfx>>>,
              mut commands: Commands| {
            subject.0 = Some(hull);
            shown.0.clone_from(&line);
            *pane = InterfacePaneType::Ship;
            play_menu_select(&mut commands, bank.as_deref());
        },
    );
}

/// Clear [`ShipViewSubject`] once the Ship pane is not on screen, and drop a
/// subject that no longer exists.
pub(crate) fn end_hull_info(
    pause: Res<State<PauseStates>>,
    close: Res<NovaOsCloseTransition>,
    pane: Res<InterfacePaneType>,
    mut subject: ResMut<ShipViewSubject>,
    q_hulls: Query<(), With<SpaceshipRootMarker>>,
) {
    let Some(hull) = subject.0 else {
        return;
    };
    let shown = interface_shown(&pause, &close) && *pane == InterfacePaneType::Ship;
    if !shown || !q_hulls.contains(hull) {
        subject.0 = None;
    }
}

/// Keep the header over the Ship panel while Info shows a hull: the card's
/// line and Close.
pub(crate) fn sync_hull_info_header(
    mut commands: Commands,
    subject: Res<ShipViewSubject>,
    shown: Res<HullInfoLine>,
    q_panel: Query<Entity, With<ShipPanelMarker>>,
    q_header: Query<Entity, With<HullInfoHeader>>,
) {
    let wanted = subject.0.is_some();
    if !wanted {
        for header in &q_header {
            commands.entity(header).despawn();
        }
        return;
    }
    let Ok(panel) = q_panel.single() else {
        return;
    };
    if !q_header.is_empty() {
        return;
    }
    let header = commands
        .spawn((
            HullInfoHeader,
            Name::new("HullInfoHeader"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|header| {
            header.spawn(themed_label(&shown.0, 13.0, UiColor::Primary));
            header
                .spawn((
                    Name::new(HULL_INFO_CLOSE),
                    compact_button(ButtonSpec::new("Close").fit()),
                ))
                .observe(
                    |_activate: On<Activate>,
                     mut pane: ResMut<InterfacePaneType>,
                     bank: Option<Res<SoundBank<UiSfx>>>,
                     mut commands: Commands| {
                        *pane = InterfacePaneType::Station;
                        play_menu_select(&mut commands, bank.as_deref());
                    },
                );
        })
        .id();
    commands.entity(panel).insert_children(0, &[header]);
}
