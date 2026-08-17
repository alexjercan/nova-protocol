//! The SHIP app's runtime seam: its id, title, hints, the side panel it spawns,
//! and the CLI verbs and panel buttons that route into `ShipSectionCommand`.
//!
//! Touch this module when changing what the ship app is called, shows on entry,
//! or accepts as a command.

use bevy::{prelude::*, ui_widgets::Button};
use nova_gameplay::prelude::*;
use nova_os::prelude::*;
use nova_ship::prelude::*;

use super::{scene::*, sections::*, *};
use crate::terminal::{
    nova_os_text_font, section_kind_from_markers, DRAWER_LINE_FONT_PX, NOVA_OS_PHOSPHOR,
    NOVA_OS_PHOSPHOR_MUTED, NOVA_OS_SCREEN, NOVA_OS_TEXT,
};

pub(crate) struct ShipApp;

impl NovaOsAppRuntime for ShipApp {
    fn id(&self) -> &'static str {
        SHIP_APP_ID
    }
    fn title(&self) -> &'static str {
        "SHIP / SCHEMATIC"
    }
    fn hints(&self) -> &'static [&'static str] {
        SHIP_HINTS
    }
    fn spawn_body(&self, body: &mut ChildSpawnerCommands, font: Handle<Font>) {
        // The body is a Column; lay the viewport + inspector panel out as a Row
        // that grows to fill it: [ 3D viewport (flex-grow) | fixed-width panel ].
        body.spawn(Node {
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                ShipViewportMarker,
                Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    ..default()
                },
                ImageNode {
                    image: Handle::default(),
                    ..default()
                },
                BackgroundColor(SHIP_VIEW_BG),
            ));
            spawn_ship_panel(row, font.clone());
        });
    }
}

/// Fixed width of the inspector panel column, in px.
pub(crate) const SHIP_PANEL_PX: f32 = 232.0;

/// Build the inspector-panel subtree (title, live detail, action row, note) as a
/// bordered CRT column. The three info text nodes carry a [`ShipPanelField`] so
/// one system can refresh them; the two buttons carry a [`ShipPanelButton`] and
/// route through the [`ShipSectionCommand`] seam via `Activate` observers.
pub(crate) fn spawn_ship_panel(parent: &mut ChildSpawnerCommands, font: Handle<Font>) {
    parent
        .spawn((
            ShipPanelMarker,
            Node {
                width: Val::Px(SHIP_PANEL_PX),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(NOVA_OS_PHOSPHOR.with_alpha(0.36)),
            BackgroundColor(NOVA_OS_SCREEN),
        ))
        .with_children(|panel| {
            panel.spawn((
                ShipPanelField::Title,
                Text::new("INSPECTOR"),
                nova_os_text_font(DRAWER_LINE_FONT_PX, font.clone()),
                TextColor(NOVA_OS_PHOSPHOR),
            ));
            panel.spawn((
                ShipPanelField::Detail,
                Text::new("Select a section:\nclick a block or press [ / ]."),
                nova_os_text_font(DRAWER_LINE_FONT_PX - 3.0, font.clone()),
                TextColor(NOVA_OS_TEXT),
            ));
            // Action row: Repair + Reload buttons, each routed through the
            // ShipSectionCommand seam by an `Activate` observer.
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(panel_button_bundle(ShipPanelButton::Repair))
                        .observe(on_ship_repair_button)
                        .with_children(|b| {
                            b.spawn(panel_button_label("P Repair", font.clone()));
                        });
                    row.spawn(panel_button_bundle(ShipPanelButton::Reload))
                        .observe(on_ship_reload_button)
                        .with_children(|b| {
                            b.spawn(panel_button_label("L Reload", font.clone()));
                        });
                });
            panel
                .spawn(panel_button_bundle(ShipPanelButton::Rebind))
                .observe(on_ship_rebind_button)
                .with_children(|button| {
                    button.spawn(panel_button_label("B Rebind", font.clone()));
                });
            panel.spawn((
                ShipPanelField::Note,
                Text::new(String::new()),
                nova_os_text_font(DRAWER_LINE_FONT_PX - 4.0, font),
                TextColor(NOVA_OS_PHOSPHOR_MUTED),
            ));
        });
}

/// The bundle for a CRT action button (enabled styling; recoloured per frame in
/// `update_ship_panel`). The `Activate` observer is attached by the caller.
pub(crate) fn panel_button_bundle(kind: ShipPanelButton) -> impl Bundle {
    (
        kind,
        Button,
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(NOVA_OS_PHOSPHOR),
        BackgroundColor(NOVA_OS_PHOSPHOR.with_alpha(0.14)),
    )
}

/// The label bundle for a panel button.
pub(crate) fn panel_button_label(label: &str, font: Handle<Font>) -> impl Bundle {
    (
        Text::new(label.to_string()),
        nova_os_text_font(DRAWER_LINE_FONT_PX - 3.0, font),
        TextColor(NOVA_OS_PHOSPHOR),
    )
}
/// Keep the terminal's arg-completion set in sync with the live section codes, so
/// `ship repair <TAB>` offers them. Only writes on a real change.
pub(crate) fn sync_ship_arg_completions(
    sections: ShipSections,
    mut runtime: ResMut<ShipRuntime>,
    mut terminal: ResMut<NovaOsTerminal>,
) {
    let codes = sections.codes();
    if codes == runtime.completion_codes {
        return;
    }
    runtime.completion_codes = codes.clone();
    // Merge (not replace) so the `map goto` completions the map app owns survive;
    // the `!=` gate above already ensured this set changed.
    terminal.merge_arg_completions(
        ["ship section", "ship reload", "ship repair"]
            .into_iter()
            .map(|verb| (verb, codes.clone())),
    );
}

/// Drain the arg-bearing `ship` CLI verb the terminal queued on submit, apply it
/// against the live world, and append the result rows to the scrollback.
///
/// This resolves the section with a query that does NOT touch `Health`/
/// `SectionAmmo`, so the `&mut` health/ammo queries it also holds do not conflict
/// (a `ShipSections` here would read those components immutably and deadlock the
/// scheduler); integrity/ammo are read back through the mutable queries' `get`.
#[expect(
    clippy::type_complexity,
    reason = "one query term per section field the CLI prints"
)]
pub(crate) fn apply_ship_cli_commands(
    mut terminal: ResMut<NovaOsTerminal>,
    q_player: Query<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    q_sections: Query<
        (
            Entity,
            &ChildOf,
            &SectionCode,
            Option<&Name>,
            SectionKindQuery,
            (Has<SectionInactiveMarker>, Has<HealthZeroMarker>),
        ),
        With<SectionMarker>,
    >,
    mut q_health: Query<&mut Health>,
    mut q_ammo: Query<&mut SectionAmmo>,
) {
    // Peek first: the single pending slot is shared with the `map` gameplay verbs,
    // so only consume an invocation this handler owns (leave `map ...` for its own
    // system, `cross-app-invocation-peek-before-take`).
    let owns = terminal
        .peek_pending_invocation()
        .is_some_and(|inv| matches!(inv.name, "ship section" | "ship reload" | "ship repair"));
    if !owns {
        return;
    }
    let Some(invocation) = terminal.take_pending_invocation() else {
        return;
    };
    let Some(code) = invocation.args.first() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: expected a section id", invocation.name),
        }]);
        return;
    };
    let Ok(ship) = q_player.single() else {
        terminal.extend_scrollback([TerminalRow {
            kind: TerminalRowKind::Error,
            text: "no live player ship".to_string(),
        }]);
        return;
    };

    // Resolve the code (no Health/Ammo access here) and gather the valid codes for
    // the not-found listing.
    let wanted = code.to_ascii_uppercase();
    let mut codes: Vec<String> = Vec::new();
    let mut target: Option<(Entity, String, String, SectionClass, bool, bool)> = None;
    for (
        entity,
        child,
        section_code,
        name,
        (class, hull, controller, thruster, turret, torpedo),
        (inactive, zero),
    ) in &q_sections
    {
        if child.0 != ship {
            continue;
        }
        codes.push(section_code.0.clone());
        if section_code.0.eq_ignore_ascii_case(&wanted) {
            if let Some(kind) =
                section_kind_from_markers(class, hull, controller, thruster, turret, torpedo)
            {
                target = Some((
                    entity,
                    section_code.0.clone(),
                    name.map(|n| n.as_str().to_string())
                        .unwrap_or_else(|| section_code.0.clone()),
                    kind,
                    inactive,
                    zero,
                ));
            }
        }
    }
    codes.sort();
    let Some((entity, code_str, name, kind, inactive, zero)) = target else {
        terminal.extend_scrollback(unknown_code_rows(code, &codes));
        return;
    };
    let is_weapon = matches!(kind, SectionClass::Turret | SectionClass::Torpedo);

    let rows = match invocation.name {
        "ship section" => {
            // Read integrity/ammo back through the mutable queries (read-only get).
            let view = ShipSectionView {
                entity,
                code: code_str,
                kind,
                name,
                local: Transform::default(),
                half_extents: Vec3::ONE,
                link_points: Vec::new(),
                health: q_health.get(entity).ok().cloned(),
                ammo: q_ammo.get(entity).ok().copied(),
                bindings: None,
                inactive,
                zero_health: zero,
            };
            section_detail_rows(&view)
        }
        "ship reload" | "ship repair" => {
            let action = if invocation.name == "ship reload" {
                ShipAction::Reload
            } else {
                ShipAction::Repair
            };
            let mut health = q_health.get_mut(entity).ok();
            let mut ammo = q_ammo.get_mut(entity).ok();
            vec![apply_action_to_section(
                action,
                &code_str,
                kind,
                is_weapon,
                health.as_deref_mut(),
                ammo.as_deref_mut(),
            )]
        }
        _ => vec![TerminalRow {
            kind: TerminalRowKind::Error,
            text: format!("{}: unhandled ship verb", invocation.name),
        }],
    };
    terminal.extend_scrollback(rows);
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
        let is_weapon = matches!(kind, SectionClass::Turret | SectionClass::Torpedo);
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

// ---------------------------------------------------------------------------
// Scene systems
// ---------------------------------------------------------------------------
