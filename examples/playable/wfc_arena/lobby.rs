use bevy::{
    picking::hover::Hovered,
    ui::InteractionDisabled,
    ui_widgets::{Activate, Button},
};
use nova_ui::{skin::UiSkin, theme, widget::prelude::*};

use super::*;

const MAX_SHIPS_PER_SIDE: usize = 4;

#[derive(Clone)]
struct LobbyShip {
    team: usize,
    player: bool,
    seed: String,
    resolved_seed: u64,
}

#[derive(Resource)]
pub(super) struct LobbyModel {
    side_styles: [usize; TEAMS.len()],
    ships: Vec<LobbyShip>,
    next_seed: u64,
    binding_overrides: BTreeMap<(usize, String), Vec<Binding>>,
}

#[derive(Component)]
pub(super) struct LobbyRoot;

#[derive(Component)]
pub(super) struct LobbyCamera;

#[derive(Component, Clone, Copy)]
struct LobbySeedField(usize);

#[derive(Component)]
struct LobbyStart;

#[derive(Component, Clone, Copy)]
enum LobbyAction {
    Add(usize),
    Remove(usize),
    Reroll(usize),
    TogglePilot(usize),
    Style(usize, usize),
    Start,
    Quit,
}

pub(super) fn match_active(roots: Query<(), With<LobbyRoot>>) -> bool {
    roots.is_empty()
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), load_or_open_lobby);
    app.add_systems(Update, (validate_seed_fields, retain_binding_changes));
    app.add_observer(on_lobby_action);
}

fn style_index(styles: &GameStyles, id: &str, source: &str) -> usize {
    styles
        .iter()
        .position(|style| style.id == id)
        .unwrap_or_else(|| panic!("{source} style '{id}' is not in the merged content"))
}

fn side_style_indexes(
    styles: &GameStyles,
    requested: Option<&str>,
    ships: &[ShipSpec],
) -> [usize; TEAMS.len()] {
    let initial = requested.map_or(0, |id| style_index(styles, id, "--style"));
    let mut side_styles = [initial; TEAMS.len()];
    for ship in ships {
        if let Some(id) = ship.style.as_deref() {
            side_styles[ship.team] = style_index(styles, id, "--ship");
        }
    }
    side_styles
}

fn load_or_open_lobby(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    requested: Res<StyleRequest>,
    mut roster: ResMut<Roster>,
    // OPTIONAL because the skin belongs to the render-gated UI stack, and the
    // autopilot path below returns before any of it is drawn - a required `Res`
    // makes a `--norender` run panic here instead of fielding the match.
    skin: Option<Res<UiSkin>>,
) {
    let side_styles = side_style_indexes(&styles, requested.0.as_deref(), &roster.ships);
    for ship in &mut roster.ships {
        ship.style = Some(styles[side_styles[ship.team]].id.clone());
    }
    roster.style = side_styles[0];

    let tiles = tile_set(&sections);
    let looks: Vec<StyleId> = roster
        .ships
        .iter()
        .map(|ship| style_at(&styles, side_styles[ship.team]))
        .collect();
    let drafted = draft_roster(&tiles, &roster.ships, &looks, roster.seed);
    let drafted_seeds: Vec<u64> = drafted.iter().map(|(seed, _)| *seed).collect();
    for (ship, seed) in roster.ships.iter_mut().zip(drafted_seeds.iter().copied()) {
        ship.seed = Some(seed);
    }
    roster.drafted.clone_from(&drafted_seeds);

    if std::env::var_os("NOVA_AUTOPILOT").is_some() {
        start_match(&mut commands, &game_assets, &sections, &styles, &mut roster);
        return;
    }

    let model = LobbyModel {
        side_styles,
        ships: roster
            .ships
            .iter()
            .zip(drafted_seeds.iter())
            .map(|(ship, seed)| LobbyShip {
                team: ship.team,
                player: ship.player,
                seed: seed.to_string(),
                resolved_seed: *seed,
            })
            .collect(),
        next_seed: drafted_seeds
            .iter()
            .copied()
            .max()
            .unwrap_or(roster.seed)
            .wrapping_add(1),
        binding_overrides: roster.binding_overrides.clone(),
    };
    let Some(skin) = skin else {
        error!("wfc_arena lobby: no UiSkin, so there is no lobby to open - run with a renderer, or with NOVA_AUTOPILOT to field the match directly");
        return;
    };
    spawn_lobby(&mut commands, &model, &styles, *skin);
    commands.insert_resource(model);
}

fn start_match(
    commands: &mut Commands,
    game_assets: &GameAssets,
    sections: &GameSections,
    styles: &GameStyles,
    roster: &mut Roster,
) {
    commands.trigger(LoadScenario(arena(game_assets, sections, styles, roster)));
    result::begin_match(commands, roster.ships.len());
}

fn text(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        UiText,
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn action_button(label: &str, action: LobbyAction) -> impl Bundle {
    (button(ButtonSpec::new(label)), action)
}

pub(super) fn spawn_lobby(
    commands: &mut Commands,
    model: &LobbyModel,
    styles: &GameStyles,
    skin: UiSkin,
) {
    commands.spawn((LobbyCamera, Camera2d, IsDefaultUiCamera));
    commands
        .spawn((
            LobbyRoot,
            Node {
                width: vw(100),
                height: vh(100),
                max_width: vw(100),
                max_height: vh(100),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(36), px(18)),
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(theme::SPACE.with_alpha(0.97)),
        ))
        .with_children(|root| {
            root.spawn(text("WFC ARENA / MATCH CONFIGURATOR", 25.0, theme::PHOSPHOR));
            root.spawn(text(
                "Configure both formations. Seeds are exact; use REROLL for another combat-ready hull.",
                13.0,
                theme::PHOSPHOR_DIM,
            ));
            root.spawn(Node {
                width: vw(94),
                max_width: vw(94),
                height: vh(76),
                max_height: vh(76),
                min_height: px(0),
                flex_shrink: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(18),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|sides| {
                for team in 0..TEAMS.len() {
                    spawn_side(sides, team, model, styles, skin);
                }
            });
            root.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::FlexEnd,
                column_gap: px(12),
                ..default()
            })
            .with_children(|footer| {
                footer
                    .spawn(Node {
                        width: px(150),
                        ..default()
                    })
                    .with_children(|button_parent| {
                        button_parent.spawn(action_button("QUIT", LobbyAction::Quit));
                    });
                footer
                    .spawn(Node {
                        width: px(210),
                        ..default()
                    })
                    .with_children(|button_parent| {
                        button_parent.spawn((
                            action_button("START MATCH", LobbyAction::Start),
                            LobbyStart,
                        ));
                    });
            });
        });
}

fn spawn_side(
    parent: &mut ChildSpawnerCommands,
    team: usize,
    model: &LobbyModel,
    styles: &GameStyles,
    skin: UiSkin,
) {
    let count = model.ships.iter().filter(|ship| ship.team == team).count();
    parent
        .spawn((
            Node {
                width: vw(46),
                max_width: vw(46),
                height: vh(76),
                max_height: vh(76),
                flex_shrink: 0.0,
                min_width: px(0),
                min_height: px(0),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(theme::BORDER_W)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(TEAMS[team].tint.with_alpha(0.62)),
            BackgroundColor(theme::SCREEN_0.with_alpha(0.72)),
        ))
        .with_children(|side| {
            side.spawn(text(
                format!("{} FORMATION", TEAMS[team].callsign),
                19.0,
                TEAMS[team].tint,
            ));
            side.spawn(text("SIDE STYLE", 11.0, theme::PHOSPHOR_MUTED));
            side.spawn(Node {
                width: percent(100),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: px(6),
                row_gap: px(4),
                ..default()
            })
            .with_children(|list| {
                for (index, style) in styles.iter().enumerate() {
                    let selected = model.side_styles[team] == index;
                    let mut row = list.spawn((
                        list_row(selected, skin),
                        ListRow,
                        Button,
                        Hovered::default(),
                        LobbyAction::Style(team, index),
                        children![text(
                            style.name.to_ascii_uppercase(),
                            12.0,
                            theme::SCREEN_TEXT
                        )],
                    ));
                    if selected {
                        row.insert(Selected);
                    }
                }
            });
            side.spawn(text("SHIPS", 11.0, theme::PHOSPHOR_MUTED));
            side.spawn(Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                ..default()
            })
            .with_children(|rows| {
                for (slot, ship) in model.ships.iter().enumerate() {
                    if ship.team == team {
                        spawn_ship_row(rows, slot, ship, count == 1);
                    }
                }
            });
            let mut add = side.spawn(action_button("+ ADD SHIP", LobbyAction::Add(team)));
            if count >= MAX_SHIPS_PER_SIDE {
                add.insert(InteractionDisabled);
            }
        });
}

fn spawn_ship_row(
    parent: &mut ChildSpawnerCommands,
    slot: usize,
    ship: &LobbyShip,
    sole_ship: bool,
) {
    parent
        .spawn((
            Node {
                width: percent(100),
                min_height: px(48),
                align_items: AlignItems::FlexStart,
                column_gap: px(7),
                padding: UiRect::all(px(6)),
                border: UiRect::all(px(theme::BORDER_W)),
                ..default()
            },
            BorderColor::all(theme::PHOSPHOR.with_alpha(0.15)),
            BackgroundColor(theme::PHOSPHOR.with_alpha(0.025)),
        ))
        .with_children(|row| {
            row.spawn((
                text(format!("{:02}", slot + 1), 12.0, theme::PHOSPHOR_MUTED),
                Node {
                    margin: UiRect::top(px(23)),
                    ..default()
                },
            ));
            row.spawn(Node {
                width: px(205),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            })
            .with_children(|field| {
                field.spawn(text("SEED", 9.0, theme::PHOSPHOR_MUTED));
                field.spawn((
                    text_field(TextFieldSpec::new(&ship.seed).max_chars(20)),
                    LobbySeedField(slot),
                ));
            });
            row.spawn(Node {
                width: px(92),
                flex_shrink: 0.0,
                margin: UiRect::top(px(7)),
                ..default()
            })
            .with_children(|button_parent| {
                button_parent.spawn(action_button(
                    if ship.player { "PLAYER" } else { "AI" },
                    LobbyAction::TogglePilot(slot),
                ));
            });
            row.spawn(Node {
                width: px(112),
                flex_shrink: 0.0,
                margin: UiRect::top(px(7)),
                ..default()
            })
            .with_children(|button_parent| {
                button_parent.spawn(action_button("REROLL", LobbyAction::Reroll(slot)));
            });
            row.spawn(Node {
                width: px(112),
                flex_shrink: 0.0,
                margin: UiRect::top(px(7)),
                ..default()
            })
            .with_children(|button_parent| {
                let mut remove =
                    button_parent.spawn(action_button("REMOVE", LobbyAction::Remove(slot)));
                if sole_ship {
                    remove.insert(InteractionDisabled);
                }
            });
        });
}

fn capture_seed_values(model: &mut LobbyModel, fields: &Query<(&LobbySeedField, &TextFieldValue)>) {
    for (field, value) in fields {
        if let Some(ship) = model.ships.get_mut(field.0) {
            ship.seed.clone_from(&value.0);
        }
    }
}

fn viable_seed(sections: &GameSections, styles: &GameStyles, style: usize, from: u64) -> u64 {
    let tiles = tile_set(sections);
    for offset in 0..DRAFT_SCAN_CAP {
        let seed = from.wrapping_add(offset);
        let hull = combat_hull(&tiles, seed, style_at(styles, style));
        if armament(&hull).viable() {
            return seed;
        }
    }
    panic!(
        "wfc_arena: no combat-viable hull in seeds {from}..{}",
        from.wrapping_add(DRAFT_SCAN_CAP),
    );
}

fn rebuild_lobby(
    commands: &mut Commands,
    model: &LobbyModel,
    styles: &GameStyles,
    skin: UiSkin,
    roots: &Query<Entity, With<LobbyRoot>>,
) {
    for root in roots {
        commands.entity(root).despawn();
    }
    spawn_lobby(commands, model, styles, skin);
}

#[expect(clippy::too_many_arguments, reason = "one system over the whole lobby")]
fn on_lobby_action(
    activate: On<Activate>,
    mut commands: Commands,
    action: Query<&LobbyAction>,
    fields: Query<(&LobbySeedField, &TextFieldValue)>,
    roots: Query<Entity, With<LobbyRoot>>,
    cameras: Query<Entity, With<LobbyCamera>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    skin: Res<UiSkin>,
    model: Option<ResMut<LobbyModel>>,
    mut roster: ResMut<Roster>,
) {
    let Ok(action) = action.get(activate.event_target()) else {
        return;
    };
    let Some(mut model) = model else {
        return;
    };
    capture_seed_values(&mut model, &fields);
    let mut rebuild = true;
    match *action {
        LobbyAction::Add(team) => {
            if model.ships.iter().filter(|ship| ship.team == team).count() >= MAX_SHIPS_PER_SIDE {
                return;
            }
            let seed = viable_seed(&sections, &styles, model.side_styles[team], model.next_seed);
            model.next_seed = seed.wrapping_add(1);
            model.ships.push(LobbyShip {
                team,
                player: false,
                seed: seed.to_string(),
                resolved_seed: seed,
            });
        }
        LobbyAction::Remove(slot) => {
            let Some(ship) = model.ships.get(slot) else {
                return;
            };
            if model
                .ships
                .iter()
                .filter(|other| other.team == ship.team)
                .count()
                <= 1
            {
                return;
            }
            model.ships.remove(slot);
            model.binding_overrides = model
                .binding_overrides
                .iter()
                .filter_map(|((bound_slot, section), bindings)| {
                    if *bound_slot == slot {
                        None
                    } else {
                        Some((
                            (
                                if *bound_slot > slot {
                                    *bound_slot - 1
                                } else {
                                    *bound_slot
                                },
                                section.clone(),
                            ),
                            bindings.clone(),
                        ))
                    }
                })
                .collect();
        }
        LobbyAction::Reroll(slot) => {
            let Some(ship) = model.ships.get(slot) else {
                return;
            };
            let seed = viable_seed(
                &sections,
                &styles,
                model.side_styles[ship.team],
                model.next_seed,
            );
            model.next_seed = seed.wrapping_add(1);
            model.ships[slot].seed = seed.to_string();
            model.ships[slot].resolved_seed = seed;
            model
                .binding_overrides
                .retain(|(bound_slot, _), _| *bound_slot != slot);
        }
        LobbyAction::TogglePilot(slot) => {
            let Some(selected) = model.ships.get(slot) else {
                return;
            };
            let enable = !selected.player;
            if enable {
                for ship in &mut model.ships {
                    ship.player = false;
                }
            }
            model.ships[slot].player = enable;
        }
        LobbyAction::Style(team, style) => {
            model.side_styles[team] = style;
        }
        LobbyAction::Start => {
            let seeds: Option<Vec<u64>> = model
                .ships
                .iter()
                .map(|ship| ship.seed.parse::<u64>().ok())
                .collect();
            let Some(seeds) = seeds else {
                return;
            };
            let changed_slots: Vec<usize> = model
                .ships
                .iter()
                .zip(seeds.iter())
                .enumerate()
                .filter_map(|(slot, (ship, seed))| (ship.resolved_seed != *seed).then_some(slot))
                .collect();
            model
                .binding_overrides
                .retain(|(slot, _), _| !changed_slots.contains(slot));
            for (ship, seed) in model.ships.iter_mut().zip(seeds.iter().copied()) {
                ship.resolved_seed = seed;
            }
            roster.ships = model
                .ships
                .iter()
                .zip(seeds)
                .map(|(ship, seed)| ShipSpec {
                    team: ship.team,
                    style: Some(styles[model.side_styles[ship.team]].id.clone()),
                    seed: Some(seed),
                    player: ship.player,
                })
                .collect();
            roster.style = model.side_styles[0];
            roster.seed = model.next_seed;
            roster.drafted = roster.ships.iter().filter_map(|ship| ship.seed).collect();
            roster
                .binding_overrides
                .clone_from(&model.binding_overrides);
            for root in &roots {
                commands.entity(root).despawn();
            }
            for camera in &cameras {
                commands.entity(camera).despawn();
            }
            start_match(&mut commands, &game_assets, &sections, &styles, &mut roster);
            rebuild = false;
        }
        LobbyAction::Quit => {
            commands.write_message(AppExit::Success);
            rebuild = false;
        }
    }
    if rebuild {
        rebuild_lobby(&mut commands, &model, &styles, *skin, &roots);
    }
}

fn retain_binding_changes(
    mut changes: MessageReader<SectionInputBindingChanged>,
    roots: Query<&EntityId>,
    model: Option<ResMut<LobbyModel>>,
    mut roster: ResMut<Roster>,
) {
    let Some(mut model) = model else {
        changes.clear();
        return;
    };
    for change in changes.read() {
        let Ok(id) = roots.get(change.spaceship) else {
            continue;
        };
        let Some(slot) = fighter_slot(id) else {
            continue;
        };
        let key = (slot, change.section_id.clone());
        model
            .binding_overrides
            .insert(key.clone(), change.bindings.clone());
        roster
            .binding_overrides
            .insert(key, change.bindings.clone());
    }
}

fn validate_seed_fields(
    mut commands: Commands,
    fields: Query<(Entity, &TextFieldValue), With<LobbySeedField>>,
    start: Query<Entity, With<LobbyStart>>,
) {
    let mut valid = true;
    for (entity, value) in &fields {
        if value.0.parse::<u64>().is_ok() {
            commands.entity(entity).remove::<TextFieldError>();
        } else {
            valid = false;
            commands
                .entity(entity)
                .insert(TextFieldError("Enter an unsigned integer".to_string()));
        }
    }
    for entity in &start {
        if valid {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles() -> GameStyles {
        GameStyles(vec![
            ShipStyleConfig {
                id: "industrial".to_string(),
                ..default()
            },
            ShipStyleConfig {
                id: "salvage".to_string(),
                ..default()
            },
        ])
    }

    #[test]
    fn last_explicit_cli_style_wins_per_side() {
        let ships = vec![
            ShipSpec {
                team: 0,
                style: Some("salvage".to_string()),
                seed: Some(1),
                player: false,
            },
            ShipSpec {
                team: 0,
                style: Some("industrial".to_string()),
                seed: Some(2),
                player: false,
            },
            ShipSpec {
                team: 1,
                style: None,
                seed: Some(3),
                player: false,
            },
        ];

        assert_eq!(
            side_style_indexes(&styles(), Some("salvage"), &ships),
            [0, 1]
        );
    }
}
