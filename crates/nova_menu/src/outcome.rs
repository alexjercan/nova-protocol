//! The in-play modal overlays: the win/lose outcome frame and the content
//! gate's FAILED TO START report, plus the cursor and pause they own.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ui::{prelude::UiSkin, theme, widget::panel};

use crate::{pause::on_back_to_menu, widgets::button};

/// Marker for the outcome overlay root (see `sync_outcome_overlay`). Carries
/// the queued-switch snapshot the overlay was built against, so the sync can
/// rebuild when a LATER event queues a NextScenario under a shown outcome
/// (outcome review R1.3) - otherwise the buttons/hint would say Main Menu
/// while Enter actually releases the queued switch.
#[derive(Component)]
pub(crate) struct OutcomeOverlay {
    pub(crate) queued: bool,
}

/// Spawn/despawn the win/lose overlay to mirror [`CurrentOutcome`]. Rebuilds
/// from scratch on outcome change OR when the queued-switch snapshot goes
/// stale - an outcome flips at most once per scenario, so there is nothing
/// worth diffing. The overlay dies with the outcome (scenario teardown
/// clears the resource) and with the Playing state (`DespawnOnExit`),
/// whichever comes first.
pub(crate) fn sync_outcome_overlay(
    mut commands: Commands,
    skin: Res<UiSkin>,
    outcome: Res<CurrentOutcome>,
    world: Option<Res<NovaEventWorld>>,
    q_existing: Query<(Entity, &OutcomeOverlay)>,
) {
    // What Continue means is whatever the scenario queued: a Victory pairs it
    // with the next chapter, a Defeat with a retry of itself. Nothing queued
    // means the story ends here and the only road is back to the menu.
    let queued = world
        .as_ref()
        .is_some_and(|world| world.next_scenario.is_some());
    let stale = q_existing
        .iter()
        .any(|(_, overlay)| overlay.queued != queued);
    if !outcome.is_changed() && !stale {
        return;
    }
    for (entity, _) in q_existing.iter() {
        commands.entity(entity).despawn();
    }
    let Some(config) = outcome.0.as_ref() else {
        return;
    };

    let (banner, accent) = match config.outcome {
        ScenarioOutcomeKind::Victory => ("VICTORY", theme::semantic::OBJECTIVE),
        ScenarioOutcomeKind::Defeat => ("DEFEAT", theme::semantic::THREAT),
    };
    let primary = queued.then_some(match config.outcome {
        ScenarioOutcomeKind::Victory => "Continue",
        ScenarioOutcomeKind::Defeat => "Retry",
    });
    let message = config.message.clone();

    commands
        .spawn((
            OutcomeOverlay { queued },
            DespawnOnExit(GameStates::Playing),
            Name::new("Outcome Overlay"),
            // Same modal rule as the pause overlay: nothing beneath this
            // (HUD, editor panels) may receive clicks through it.
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            // Above the HUD chrome. Below the pause overlay's z (10) as a defensive
            // ordering, though the two no longer coexist: a shown outcome holds its own
            // pause and makes ESC inert, so the pause overlay cannot stack over it.
            GlobalZIndex(9),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Outcome Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(320),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Outcome Banner"),
                        Text::new(banner),
                        TextFont {
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(accent),
                    ));
                    if let Some(message) = message {
                        parent.spawn((
                            Name::new("Outcome Message"),
                            Text::new(message),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(theme::SCREEN_TEXT),
                            Node {
                                margin: UiRect::top(px(8)),
                                max_width: px(280),
                                ..default()
                            },
                        ));
                    }
                    if let Some(primary) = primary {
                        parent.spawn((
                            Name::new("Outcome Primary Button"),
                            button(primary),
                            observe(on_outcome_advance),
                        ));
                    }
                    parent.spawn((
                        Name::new("Outcome Menu Button"),
                        button("Main Menu"),
                        observe(on_back_to_menu),
                    ));
                    // The keyboard/gamepad route into the same mechanics
                    // (the loader's scenario-advance input).
                    let hint = match primary {
                        Some(label) => format!("[Enter] {label}"),
                        None => "[Enter] Main Menu".to_string(),
                    };
                    parent.spawn((
                        Name::new("Outcome Key Hint"),
                        Text::new(hint),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(theme::PHOSPHOR_MUTED),
                        Node {
                            margin: UiRect::top(px(4)),
                            ..default()
                        },
                    ));
                });
        });
}

/// The outcome overlay's Continue/Retry button: release the lingering
/// `NextScenario` the scenario queued next to its `Outcome` action - the
/// same mechanism the Enter key drives through the loader.
pub(crate) fn on_outcome_advance(
    _activate: On<Activate>,
    mut world: Option<ResMut<NovaEventWorld>>,
) {
    if let Some(world) = world.as_deref_mut() {
        world.release_lingering_next();
    }
}

/// The timed overlay: an outcome declared with `auto_advance_secs` advances its queued
/// LINGERING chain by itself after N REAL seconds - the overlay pauses virtual time, so
/// the wall clock is the only one still moving - via exactly the Continue button's
/// release. The local clock re-arms per outcome (reset on any CurrentOutcome change)
/// and idles when no lingering chain waits (nothing to advance).
pub(crate) fn auto_advance_outcome(
    // Optional: headless rigs run without TimePlugin (the menu tests feed
    // their clocks by hand) - no wall clock, no auto-advance.
    time: Option<Res<Time<Real>>>,
    outcome: Res<CurrentOutcome>,
    mut world: Option<ResMut<NovaEventWorld>>,
    mut clock: Local<Option<Timer>>,
) {
    let Some(time) = time else {
        return;
    };
    if outcome.is_changed() {
        *clock = None;
    }
    let Some(secs) = outcome.0.as_ref().and_then(|o| o.auto_advance_secs) else {
        *clock = None;
        return;
    };
    let Some(world) = world.as_deref_mut() else {
        return;
    };
    if !world.next_scenario.as_ref().is_some_and(|next| next.linger) {
        *clock = None;
        return;
    }
    // Finite-check and cap before Timer::from_seconds: an authored 1e300
    // parses fine and `as f32` is inf, which panics Duration construction
    // (review R1.1).
    if !secs.is_finite() {
        *clock = None;
        return;
    }
    let capped = secs.clamp(0.0, nova_scenario::prelude::OUTCOME_AUTO_ADVANCE_MAX_SECS) as f32;
    let timer = clock.get_or_insert_with(|| Timer::from_seconds(capped, TimerMode::Once));
    if timer.tick(time.delta()).just_finished() {
        world.release_lingering_next();
        *clock = None;
    }
}

/// Marker for the FAILED TO START overlay root (runtime content gate).
#[derive(Component)]
pub(crate) struct StartFailureOverlay;

/// Show the Wesnoth-style refusal report: banner, the scenario's name, one
/// line per content error, and the only road out - Main Menu. Mirrors the
/// outcome overlay's modal shell.
pub(crate) fn sync_start_failure_overlay(
    mut commands: Commands,
    skin: Res<UiSkin>,
    failure: Res<ScenarioStartFailure>,
    q_existing: Query<Entity, With<StartFailureOverlay>>,
) {
    if !failure.is_changed() {
        return;
    }
    for entity in q_existing.iter() {
        commands.entity(entity).despawn();
    }
    let Some(report) = failure.0.as_ref() else {
        return;
    };

    commands
        .spawn((
            StartFailureOverlay,
            DespawnOnExit(GameStates::Playing),
            Name::new("Start Failure Overlay"),
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            // Same layer as the outcome overlay (which a refusal clears).
            GlobalZIndex(9),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Start Failure Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(380),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Start Failure Banner"),
                        Text::new("FAILED TO START"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(theme::semantic::THREAT),
                    ));
                    parent.spawn((
                        Name::new("Start Failure Scenario"),
                        Text::new(format!("Failed to start '{}':", report.scenario_name)),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                        Node {
                            margin: UiRect::top(px(8)),
                            max_width: px(340),
                            ..default()
                        },
                    ));
                    for message in &report.messages {
                        parent.spawn((
                            Name::new("Start Failure Issue"),
                            Text::new(message.clone()),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme::PHOSPHOR_MUTED),
                            Node {
                                margin: UiRect::top(px(4)),
                                max_width: px(340),
                                ..default()
                            },
                        ));
                    }
                    parent.spawn((
                        Name::new("Start Failure Menu Button"),
                        button("Main Menu"),
                        observe(on_back_to_menu),
                    ));
                });
        });
}

/// Menu entry clears any stale refusal report (its overlay died with the
/// Playing state; the resource must not re-show it next run).
pub(crate) fn clear_start_failure(mut failure: ResMut<ScenarioStartFailure>) {
    failure.0 = None;
}

/// Free the cursor while the outcome overlay is up (its buttons need a
/// pointer, exactly like the pause overlay). Re-grabbing after a Retry is
/// not this system's job: the old player ship is gone by the time the
/// outcome shows, so the regrab rides the NEXT ship's spawn
/// (`regrab_cursor_on_player_spawn`).
pub(crate) fn sync_outcome_cursor(
    outcome: Res<CurrentOutcome>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if outcome.is_changed() && outcome.0.is_some() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Freeze the simulation while the win/lose overlay is up, exactly like the pause menu:
/// mirror [`CurrentOutcome`] into [`PauseStates`] so entering it fires the same
/// `OnEnter(Paused)` freeze (`pause_clocks` + the `Unpaused` set-gates) that ESC does,
/// and clearing it releases the pause. The overlay's own input stays live because it is
/// a modal over a paused world (the pause overlay is interactive the same way): the
/// buttons dispatch through observers, and the [Enter] advance is re-allowed under an
/// outcome by `decide_advance`.
///
/// Single source of truth is `CurrentOutcome`: teardown clears it on Continue/Retry
/// (the queued switch still processes - `state_to_world_system` runs in PostUpdate
/// ungated by pause), and this unpauses on the next frame. The Main Menu / Enter-to-
/// menu paths leave `Playing`, where `force_unpause` already resets the pause, so those
/// need no explicit unpause here.
pub(crate) fn sync_outcome_pause(
    outcome: Res<CurrentOutcome>,
    current: Res<State<PauseStates>>,
    mut next: ResMut<NextState<PauseStates>>,
) {
    if !outcome.is_changed() {
        return;
    }
    if outcome.0.is_some() {
        next.set(PauseStates::Paused);
    } else if *current.get() == PauseStates::Paused {
        // Only an outcome-driven pause can be live here: the ESC toggle is
        // suppressed while an outcome is shown, so a set outcome is the only
        // reason we could be Paused when it clears.
        next.set(PauseStates::Unpaused);
    }
}

/// Re-grab the cursor when a player ship spawns during play: a Retry reloads the
/// scenario WITHOUT a state transition, so the editor's OnEnter(Scenario) grab never
/// re-fires and the cursor the outcome overlay freed would leak into the replay. Same
/// guards as `restore_cursor` (Playing only), plus unpaused - a spawn cannot race the
/// pause overlay's freed cursor. Grabs unconditionally now, debug builds included; the
/// F11 inspector reclaims it via nova_debug's `sync_inspector_cursor`.
pub(crate) fn regrab_cursor_on_player_spawn(
    _add: On<Add, PlayerSpaceshipMarker>,
    game_state: Res<State<GameStates>>,
    pause: Res<State<PauseStates>>,
    outcome: Option<Res<CurrentOutcome>>,
    // A plain Query, not Single: an observer must stay a no-op in headless
    // rigs with no window (Single's skip-when-unsatisfied is a system
    // guarantee; not verified for observers, so don't lean on it here).
    mut q_cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if *game_state.get() != GameStates::Playing || pause.get().is_frozen() {
        return;
    }
    // Symmetric with restore_cursor (outcome review R1.1): never grab out
    // from under a live outcome overlay. Teardown clears the outcome before
    // a Retry's ship respawns, so this is a belt-and-braces guard, not the
    // normal path.
    if outcome.is_some_and(|outcome| outcome.0.is_some()) {
        return;
    }
    let Ok(mut cursor) = q_cursor.single_mut() else {
        return;
    };
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}
