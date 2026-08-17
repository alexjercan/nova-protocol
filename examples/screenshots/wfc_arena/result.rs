use std::collections::BTreeSet;

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::{
    ui_widgets::Activate,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_ui::{skin::UiSkin, theme, widget::prelude::*};

use super::*;

const RESULT_DELAY_SECS: f64 = 1.5;
const INACTIVITY_SECS: f64 = 180.0;
const ARENA_RADIUS: f32 = 20_000.0;
const OUTSIDE_GRACE_SECS: f64 = 30.0;
const ACTIVE_THRUST_EPSILON: f32 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchOutcome {
    Amber,
    Onyx,
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchEnd {
    Elimination(MatchOutcome),
    Stalemate(MatchOutcome),
}

impl MatchEnd {
    fn label(self) -> &'static str {
        match self {
            Self::Elimination(MatchOutcome::Amber) => "AMBER VICTORY",
            Self::Elimination(MatchOutcome::Onyx) => "ONYX VICTORY",
            Self::Elimination(MatchOutcome::Draw) => "MUTUAL DESTRUCTION",
            Self::Stalemate(MatchOutcome::Amber) => "STALEMATE - AMBER ADVANTAGE",
            Self::Stalemate(MatchOutcome::Onyx) => "STALEMATE - ONYX ADVANTAGE",
            Self::Stalemate(MatchOutcome::Draw) => "STALEMATE - DRAW",
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct MatchFlow {
    active: bool,
    expected: usize,
    started_at: Option<f64>,
    seen_all: bool,
    candidate_structure: Vec<f32>,
    starting_structure: Vec<f32>,
    last_activity_at: Option<f64>,
    last_fired: u32,
    last_damage: f32,
    outside_since: Vec<Option<f64>>,
    disqualified: Vec<bool>,
    final_operational: Vec<bool>,
    finishing: Option<(MatchEnd, f64, f64)>,
    shown: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundaryWarning {
    slot: usize,
    distance_km: u32,
    remaining_secs: u32,
}

#[derive(Resource, Default, PartialEq, Eq)]
struct BoundaryWarnings(Vec<BoundaryWarning>);

impl MatchFlow {
    pub(super) fn can_pause(&self) -> bool {
        self.active && self.finishing.is_none()
    }
}

#[derive(Component)]
struct ResultRoot;

#[derive(Component)]
struct BoundaryWarningRoot;

#[derive(Component, Clone, Copy)]
enum ResultAction {
    Restart,
    Lobby,
    Quit,
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<MatchFlow>();
    app.init_resource::<BoundaryWarnings>();
    app.add_systems(
        Update,
        (
            detect_and_show_result.after(track_damage),
            show_boundary_warnings.after(detect_and_show_result),
        ),
    );
    app.add_systems(PostUpdate, keep_interactive_screen_owned);
    app.add_observer(on_result_action);
}

pub(super) fn begin_match(commands: &mut Commands, expected: usize) {
    commands.insert_resource(MatchFlow {
        active: true,
        expected,
        outside_since: vec![None; expected],
        disqualified: vec![false; expected],
        ..default()
    });
    commands.insert_resource(BoundaryWarnings::default());
}

pub(super) fn leave_match(commands: &mut Commands) {
    commands.insert_resource(MatchFlow::default());
    commands.insert_resource(BoundaryWarnings::default());
}

fn fighter_slot_from_root(id: &EntityId) -> Option<usize> {
    id.0.strip_prefix(FIGHTER_ID_PREFIX)?.parse().ok()
}

fn structure_by_slot(
    expected: usize,
    health: &Query<(Entity, &Health)>,
    parents: &Query<&ChildOf>,
    roots: &Query<&EntityId, With<SpaceshipRootMarker>>,
) -> Vec<f32> {
    let mut structure = vec![0.0; expected];
    for (entity, health) in health {
        let mut current = entity;
        loop {
            if let Ok(id) = roots.get(current) {
                if let Some(slot) = fighter_slot_from_root(id).filter(|slot| *slot < expected) {
                    structure[slot] += health.current;
                }
                break;
            }
            let Ok(parent) = parents.get(current) else {
                break;
            };
            current = parent.parent();
        }
    }
    structure
}

fn root_slot(
    entity: Entity,
    parents: &Query<&ChildOf>,
    roots: &Query<&EntityId, With<SpaceshipRootMarker>>,
) -> Option<usize> {
    let mut current = entity;
    loop {
        if let Ok(id) = roots.get(current) {
            return fighter_slot_from_root(id);
        }
        current = parents.get(current).ok()?.parent();
    }
}

fn match_outcome(standing: [usize; TEAMS.len()]) -> Option<MatchOutcome> {
    match (standing[0] == 0, standing[1] == 0) {
        (true, true) => Some(MatchOutcome::Draw),
        (false, true) => Some(MatchOutcome::Amber),
        (true, false) => Some(MatchOutcome::Onyx),
        (false, false) => None,
    }
}

fn outside_countdown(since: f64, now: f64) -> Option<u32> {
    let remaining = OUTSIDE_GRACE_SECS - (now - since);
    (remaining > 0.0).then(|| remaining.ceil() as u32)
}

fn structure_advantage(roster: &Roster, starting: &[f32], remaining: &[f32]) -> MatchOutcome {
    let mut team_starting = [0.0; TEAMS.len()];
    let mut team_remaining = [0.0; TEAMS.len()];
    for (slot, ship) in roster.ships.iter().enumerate() {
        team_starting[ship.team] += starting.get(slot).copied().unwrap_or(0.0);
        team_remaining[ship.team] += remaining.get(slot).copied().unwrap_or(0.0);
    }
    let fraction = |team: usize| {
        if team_starting[team] > 0.0 {
            team_remaining[team] / team_starting[team]
        } else {
            0.0
        }
    };
    match fraction(0).total_cmp(&fraction(1)) {
        std::cmp::Ordering::Greater => MatchOutcome::Amber,
        std::cmp::Ordering::Less => MatchOutcome::Onyx,
        std::cmp::Ordering::Equal => MatchOutcome::Draw,
    }
}

#[allow(clippy::too_many_arguments)]
fn detect_and_show_result(
    mut commands: Commands,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut flow: ResMut<MatchFlow>,
    mut warnings: ResMut<BoundaryWarnings>,
    roster: Res<Roster>,
    score: Res<Scoreboard>,
    skin: Res<UiSkin>,
    q_fighters: Query<
        (Entity, &EntityId, &Allegiance, &GlobalTransform),
        With<SpaceshipRootMarker>,
    >,
    q_root_ids: Query<&EntityId, With<SpaceshipRootMarker>>,
    q_health: Query<(Entity, &Health)>,
    q_controllers: Query<
        (Entity, &Health),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_thrusters: Query<
        (Entity, &ThrusterSectionInput),
        (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_parents: Query<&ChildOf>,
) {
    if !flow.active {
        return;
    }
    let real_now = real_time.elapsed_secs_f64();
    let game_now = virtual_time.elapsed_secs_f64();
    flow.started_at.get_or_insert(game_now);

    let fighters: Vec<_> = q_fighters
        .iter()
        .filter_map(|(entity, id, allegiance, transform)| {
            fighter_slot_from_root(id)
                .map(|slot| (entity, slot, allegiance, transform.translation()))
        })
        .collect();
    if !flow.seen_all && fighters.len() == flow.expected {
        let current = structure_by_slot(flow.expected, &q_health, &q_parents, &q_root_ids);
        if current == flow.candidate_structure && current.iter().all(|health| *health > 0.0) {
            flow.seen_all = true;
            flow.starting_structure = current;
            flow.last_activity_at = Some(game_now);
            flow.last_fired = score.fired.iter().map(Salvo::total).sum();
            flow.last_damage = score.dealt.iter().sum();
        } else {
            flow.candidate_structure = current;
        }
    }

    if flow.seen_all && flow.finishing.is_none() {
        let fired: u32 = score.fired.iter().map(Salvo::total).sum();
        let damage: f32 = score.dealt.iter().sum();
        let thrusting = q_thrusters.iter().any(|(entity, input)| {
            input.0.abs() > ACTIVE_THRUST_EPSILON
                && root_slot(entity, &q_parents, &q_root_ids).is_some()
        });
        if fired != flow.last_fired || damage != flow.last_damage || thrusting {
            flow.last_activity_at = Some(game_now);
            flow.last_fired = fired;
            flow.last_damage = damage;
        }

        let mut next_warnings = Vec::new();
        for (_, slot, _, position) in &fighters {
            if *slot >= flow.expected || flow.disqualified[*slot] {
                continue;
            }
            let distance = position.length();
            if distance <= ARENA_RADIUS {
                flow.outside_since[*slot] = None;
                continue;
            }
            let since = *flow.outside_since[*slot].get_or_insert(game_now);
            if let Some(remaining_secs) = outside_countdown(since, game_now) {
                next_warnings.push(BoundaryWarning {
                    slot: *slot,
                    distance_km: (distance / 1_000.0).ceil() as u32,
                    remaining_secs,
                });
                continue;
            }
            flow.disqualified[*slot] = true;
            for (controller, health) in &q_controllers {
                if root_slot(controller, &q_parents, &q_root_ids) == Some(*slot) {
                    commands.trigger(HealthApplyDamage {
                        entity: controller,
                        source: None,
                        amount: health.current,
                    });
                }
            }
            info!(
                "wfc_arena: ship {:02} disqualified outside the {:.0} km boundary",
                slot + 1,
                ARENA_RADIUS / 1_000.0
            );
        }
        next_warnings.sort_by_key(|warning| warning.slot);
        if warnings.0 != next_warnings {
            warnings.0 = next_warnings;
        }

        let live_controller_slots: BTreeSet<usize> = q_controllers
            .iter()
            .filter(|(_, health)| health.current > 0.0)
            .filter_map(|(entity, _)| root_slot(entity, &q_parents, &q_root_ids))
            .collect();
        let mut standing = [0usize; TEAMS.len()];
        for (_, slot, allegiance, _) in &fighters {
            if live_controller_slots.contains(slot) {
                if let Some(team) = team_of(allegiance) {
                    standing[team] += 1;
                }
            }
        }

        let elimination = match_outcome(standing).map(MatchEnd::Elimination);
        let inactive = game_now - flow.last_activity_at.unwrap_or(game_now) >= INACTIVITY_SECS;
        let ending = elimination.or_else(|| {
            inactive.then(|| {
                let remaining =
                    structure_by_slot(flow.expected, &q_health, &q_parents, &q_root_ids);
                MatchEnd::Stalemate(structure_advantage(
                    &roster,
                    &flow.starting_structure,
                    &remaining,
                ))
            })
        });
        if let Some(ending) = ending {
            flow.final_operational = (0..flow.expected)
                .map(|slot| live_controller_slots.contains(&slot))
                .collect();
            let duration = game_now - flow.started_at.unwrap_or(game_now);
            flow.finishing = Some((ending, real_now + RESULT_DELAY_SECS, duration));
            if !warnings.0.is_empty() {
                warnings.0.clear();
            }
            virtual_time.pause();
            physics_time.pause();
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
            info!("wfc_arena: match ended - {}", ending.label());
        }
    }

    let Some((ending, reveal_at, duration)) = flow.finishing else {
        return;
    };
    if flow.shown || real_now < reveal_at {
        return;
    }
    let remaining = structure_by_slot(flow.expected, &q_health, &q_parents, &q_root_ids);
    spawn_result(
        &mut commands,
        ending,
        duration,
        &roster,
        &score,
        &flow.starting_structure,
        &remaining,
        &flow.final_operational,
        *skin,
    );
    flow.shown = true;
}

fn show_boundary_warnings(
    mut commands: Commands,
    warnings: Res<BoundaryWarnings>,
    roots: Query<Entity, With<BoundaryWarningRoot>>,
) {
    if !warnings.is_changed() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    if warnings.0.is_empty() {
        return;
    }
    commands
        .spawn((
            BoundaryWarningRoot,
            Node {
                position_type: PositionType::Absolute,
                top: px(28),
                left: percent(50),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(theme::BORDER_W)),
                row_gap: px(3),
                ..default()
            },
            UiTransform::from_translation(Val2::percent(-50, 0)),
            BackgroundColor(Color::srgba(0.02, 0.0, 0.0, 0.9)),
            BorderColor::all(theme::semantic::THREAT),
            GlobalZIndex(20),
        ))
        .with_children(|panel| {
            panel.spawn(result_text("ARENA BOUNDARY", 15.0, theme::semantic::THREAT));
            for warning in &warnings.0 {
                panel.spawn(result_text(
                    format!(
                        "SHIP {:02}  {} KM  DISQUALIFICATION IN {}",
                        warning.slot + 1,
                        warning.distance_km,
                        warning.remaining_secs
                    ),
                    12.0,
                    theme::SCREEN_TEXT,
                ));
            }
        });
}

fn keep_interactive_screen_owned(
    flow: Res<MatchFlow>,
    pause: Res<State<PauseStates>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if flow.finishing.is_none() && *pause.get() != PauseStates::NovaOs {
        return;
    }
    virtual_time.pause();
    physics_time.pause();
    if cursor.grab_mode != CursorGrabMode::None || !cursor.visible {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn result_text(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
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

fn result_button(label: &str, action: ResultAction, primary: bool) -> impl Bundle {
    let spec = if primary {
        ButtonSpec::new(label).primary()
    } else {
        ButtonSpec::new(label)
    };
    (button(spec), action)
}

#[allow(clippy::too_many_arguments)]
fn spawn_result(
    commands: &mut Commands,
    ending: MatchEnd,
    duration: f64,
    roster: &Roster,
    score: &Scoreboard,
    starting: &[f32],
    remaining: &[f32],
    operational: &[bool],
    skin: UiSkin,
) {
    commands
        .spawn((
            ResultRoot,
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.84)),
            GlobalZIndex(22),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(920),
                        max_height: percent(92),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(px(24)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        row_gap: px(8),
                        ..default()
                    },
                    nova_ui::widget::panel(skin),
                ))
                .with_children(|panel| {
                    panel.spawn(result_text(ending.label(), 28.0, theme::AMBER_NOVA));
                    panel.spawn(result_text(
                        format!("MATCH TIME  {:02}:{:02}", duration as u64 / 60, duration as u64 % 60),
                        13.0,
                        theme::PHOSPHOR_DIM,
                    ));
                    panel.spawn(Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(18),
                        ..default()
                    })
                    .with_children(|teams| {
                        for (team, team_info) in TEAMS.iter().enumerate() {
                            let started = roster.strength(team);
                            let survived = roster
                                .ships
                                .iter()
                                .enumerate()
                                .filter(|(slot, ship)| {
                                    ship.team == team
                                        && operational.get(*slot).copied().unwrap_or(false)
                                })
                                .count();
                            teams
                                .spawn(Node {
                                    width: percent(50),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(px(10)),
                                    border: UiRect::all(px(theme::BORDER_W)),
                                    ..default()
                                })
                                .insert(BorderColor::all(team_info.tint.with_alpha(0.6)))
                                .with_children(|column| {
                                    column.spawn(result_text(
                                        team_info.callsign,
                                        20.0,
                                        team_info.tint,
                                    ));
                                    column.spawn(result_text(
                                        format!(
                                            "SHIPS  {survived}/{started}    STRUCTURE  {:.0}    DAMAGE  {:.0}",
                                            score.pool[team].unwrap_or(0.0), score.dealt[team]
                                        ),
                                        13.0,
                                        theme::SCREEN_TEXT,
                                    ));
                                });
                        }
                    });

                    panel.spawn(result_text("AMMUNITION FIRED", 12.0, theme::PHOSPHOR_MUTED));
                    let ammunition: BTreeSet<&str> = score
                        .fired
                        .iter()
                        .flat_map(|salvo| salvo.0.keys().map(String::as_str))
                        .collect();
                    for name in ammunition {
                        panel.spawn(result_text(
                            format!(
                                "{:<28}  {:>6} / {:<6}",
                                name.to_ascii_uppercase(),
                                score.fired[0].0.get(name).copied().unwrap_or(0),
                                score.fired[1].0.get(name).copied().unwrap_or(0),
                            ),
                            12.0,
                            theme::SCREEN_TEXT,
                        ));
                    }

                    panel.spawn(result_text("SHIP OUTCOMES", 12.0, theme::PHOSPHOR_MUTED));
                    for (slot, ship) in roster.ships.iter().enumerate() {
                        let state = if operational.get(slot).copied().unwrap_or(false) {
                            "OPERATIONAL"
                        } else if remaining[slot] > 0.0 {
                            "NEUTRALIZED"
                        } else {
                            "DESTROYED"
                        };
                        panel.spawn(result_text(
                            format!(
                                "{:02}  {:<6}  SEED {:<20}  {:<12}  {:>11}  {:.0}/{:.0}",
                                slot + 1,
                                TEAMS[ship.team].callsign,
                                ship.seed.unwrap_or_default(),
                                ship.style.as_deref().unwrap_or("bare"),
                                state,
                                remaining[slot],
                                starting.get(slot).copied().unwrap_or(0.0),
                            ),
                            12.0,
                            theme::SCREEN_TEXT,
                        ));
                    }
                    panel.spawn(Node {
                        width: percent(100),
                        justify_content: JustifyContent::FlexEnd,
                        column_gap: px(10),
                        ..default()
                    })
                    .with_children(|actions| {
                        actions.spawn(result_button("QUIT", ResultAction::Quit, false));
                        actions.spawn(result_button("RETURN TO LOBBY", ResultAction::Lobby, false));
                        actions.spawn(result_button("RESTART MATCH", ResultAction::Restart, true));
                    });
                });
        });
}

fn unpause(virtual_time: &mut Time<Virtual>, physics_time: &mut Time<Physics>) {
    virtual_time.unpause();
    physics_time.unpause();
}

#[allow(clippy::too_many_arguments)]
fn on_result_action(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&ResultAction>,
    roots: Query<Entity, With<ResultRoot>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
    game_assets: Res<GameAssets>,
    sections: Res<GameSections>,
    styles: Res<GameStyles>,
    skin: Res<UiSkin>,
    model: Option<Res<lobby::LobbyModel>>,
    mut roster: ResMut<Roster>,
) {
    let Ok(action) = actions.get(activate.event_target()) else {
        return;
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    match action {
        ResultAction::Restart => {
            unpause(&mut virtual_time, &mut physics_time);
            commands.trigger(LoadScenario(arena(
                &game_assets,
                &sections,
                &styles,
                &mut roster,
            )));
            commands.insert_resource(Scoreboard::default());
            begin_match(&mut commands, roster.ships.len());
        }
        ResultAction::Lobby => {
            let Some(model) = model else {
                return;
            };
            unpause(&mut virtual_time, &mut physics_time);
            commands.trigger(UnloadScenario);
            commands.insert_resource(Scoreboard::default());
            leave_match(&mut commands);
            lobby::spawn_lobby(&mut commands, &model, &styles, *skin);
        }
        ResultAction::Quit => {
            commands.write_message(AppExit::Success);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn standing_counts_resolve_wins_and_mutual_destruction() {
        assert_eq!(match_outcome([1, 1]), None);
        assert_eq!(match_outcome([1, 0]), Some(MatchOutcome::Amber));
        assert_eq!(match_outcome([0, 1]), Some(MatchOutcome::Onyx));
        assert_eq!(match_outcome([0, 0]), Some(MatchOutcome::Draw));
    }

    #[test]
    fn result_screen_reclaims_the_cursor_from_flight() {
        let mut world = World::new();
        world.insert_resource(MatchFlow {
            finishing: Some((MatchEnd::Elimination(MatchOutcome::Amber), 0.0, 0.0)),
            ..default()
        });
        world.insert_resource(State::new(PauseStates::Unpaused));
        world.init_resource::<Time<Virtual>>();
        world.init_resource::<Time<Physics>>();
        let window = world
            .spawn((
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    visible: false,
                    ..default()
                },
            ))
            .id();

        world
            .run_system_once(keep_interactive_screen_owned)
            .unwrap();

        let cursor = world.get::<CursorOptions>(window).unwrap();
        assert_eq!(cursor.grab_mode, CursorGrabMode::None);
        assert!(cursor.visible);
        assert!(world.resource::<Time<Virtual>>().is_paused());
        assert!(world.resource::<Time<Physics>>().is_paused());
    }

    #[test]
    fn nova_os_owns_the_cursor_and_match_clocks() {
        let mut world = World::new();
        world.init_resource::<MatchFlow>();
        world.insert_resource(State::new(PauseStates::NovaOs));
        world.init_resource::<Time<Virtual>>();
        world.init_resource::<Time<Physics>>();
        let window = world
            .spawn((
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    visible: false,
                    ..default()
                },
            ))
            .id();

        world
            .run_system_once(keep_interactive_screen_owned)
            .unwrap();

        let cursor = world.get::<CursorOptions>(window).unwrap();
        assert_eq!(cursor.grab_mode, CursorGrabMode::None);
        assert!(cursor.visible);
        assert!(world.resource::<Time<Virtual>>().is_paused());
        assert!(world.resource::<Time<Physics>>().is_paused());
    }

    #[test]
    fn boundary_countdown_expires_after_the_grace() {
        assert_eq!(outside_countdown(10.0, 10.0), Some(30));
        assert_eq!(outside_countdown(10.0, 39.1), Some(1));
        assert_eq!(outside_countdown(10.0, 40.0), None);
    }

    #[test]
    fn stalemate_compares_team_structure_percentages() {
        let roster = Roster {
            seed: 0,
            ships: default_roster(),
            drafted: Vec::new(),
            style: 0,
            binding_overrides: BTreeMap::new(),
        };

        assert_eq!(
            structure_advantage(&roster, &[100.0, 200.0], &[50.0, 50.0]),
            MatchOutcome::Amber
        );
        assert_eq!(
            structure_advantage(&roster, &[100.0, 200.0], &[50.0, 100.0]),
            MatchOutcome::Draw
        );
    }

    #[test]
    fn salvo_accepts_new_ammunition_labels_without_schema_changes() {
        let mut salvo = Salvo::default();
        salvo.record("Breaker torpedoes".to_string());
        salvo.record("Breaker torpedoes".to_string());
        salvo.record("Ion rounds".to_string());

        assert_eq!(salvo.total(), 3);
        assert_eq!(salvo.0["Breaker torpedoes"], 2);
        assert_eq!(salvo.0["Ion rounds"], 1);
    }
}
