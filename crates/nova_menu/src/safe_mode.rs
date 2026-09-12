//! The safe-mode report: the one modal that tells the player which mods the
//! game switched off, and why.
//!
//! Ownership: `nova_assets`' quarantine decides WHAT was disabled and keeps the
//! episode's list; this module only draws it and takes the acknowledgement.
//! Dismissing is all the button does - nothing here re-enables a mod, because
//! the thing that broke is still broken. The Mods screen is where the player
//! removes it, updates it, or switches it back on to try again.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_assets::prelude::ModQuarantine;
use nova_gameplay::prelude::GameStates;
use nova_ui::{
    prelude::{UiSkin, REPORT_Z},
    theme,
    widget::panel,
};

use crate::widgets::button;

/// How many disabled mods the modal names before it stops counting them out.
const REPORT_ROWS: usize = 8;

/// Marker for the report overlay root.
#[derive(Component)]
pub(crate) struct ModReportOverlay;

/// Raise the report while the episode owes one, and take it down when the
/// player acknowledges it.
///
/// Driven by [`ModQuarantine::report_pending`] rather than spawned once at menu
/// entry: a downloaded bundle can fail minutes after boot, while the player is
/// already on the front door, and that failure owes the same report.
pub(crate) fn sync_mod_report_overlay(
    mut commands: Commands,
    skin: Res<UiSkin>,
    quarantine: Res<ModQuarantine>,
    q_existing: Query<Entity, With<ModReportOverlay>>,
) {
    if !quarantine.is_changed() {
        return;
    }
    for entity in q_existing.iter() {
        commands.entity(entity).despawn();
    }
    if !quarantine.report_pending {
        return;
    }

    commands
        .spawn((
            ModReportOverlay,
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Mods Disabled Overlay"),
            // A modal blocker: the menu behind it must not take a click while
            // the player has not read why their mods are gone.
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            // ABSOLUTE, unlike the in-play overlays: the menu's own screens
            // are full-height roots that take layout space even while hidden,
            // so a modal left in the flow lays out BELOW the viewport - real,
            // laid out, and off the bottom of the screen.
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            // The menu's modal layer, shared with the in-play reports.
            GlobalZIndex(REPORT_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Mods Disabled Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(420),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Mods Disabled Banner"),
                        Text::new("MODS DISABLED"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(theme::semantic::THREAT),
                    ));
                    parent.spawn((
                        Name::new("Mods Disabled Summary"),
                        Text::new(
                            "The game started without these mods, because their content failed to load. They stay installed - open Mods to update, remove or re-enable them.",
                        ),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                        Node {
                            margin: UiRect::top(px(8)),
                            max_width: px(380),
                            ..default()
                        },
                    ));
                    for disabled in quarantine.disabled.iter().take(REPORT_ROWS) {
                        parent.spawn((
                            Name::new("Mods Disabled Row"),
                            Text::new(format!("{} - {}", disabled.id, disabled.reason)),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme::PHOSPHOR_MUTED),
                            Node {
                                margin: UiRect::top(px(4)),
                                max_width: px(380),
                                ..default()
                            },
                        ));
                    }
                    // A modal grows with its list, and a mod set can break in
                    // bulk (a dependency every one of them names). The rest are
                    // rows on the Mods screen, which is where the player acts
                    // on them anyway.
                    let rest = quarantine.disabled.len().saturating_sub(REPORT_ROWS);
                    if rest > 0 {
                        parent.spawn((
                            Name::new("Mods Disabled Overflow"),
                            Text::new(format!("...and {rest} more, listed under Mods.")),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme::PHOSPHOR_MUTED),
                            Node {
                                margin: UiRect::top(px(4)),
                                max_width: px(380),
                                ..default()
                            },
                        ));
                    }
                    parent.spawn((
                        Name::new("Mods Disabled Acknowledge Button"),
                        button("OK, I understand"),
                        observe(on_acknowledge_mod_report),
                    ));
                });
        });
}

/// Acknowledge the report. It closes the modal and nothing else: the disabled
/// set is unchanged, and the episode is done owing a report.
fn on_acknowledge_mod_report(_activate: On<Activate>, mut quarantine: ResMut<ModQuarantine>) {
    quarantine.report_pending = false;
}
