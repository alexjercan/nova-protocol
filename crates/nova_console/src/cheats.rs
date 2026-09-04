//! The Cheat commands, and the one Utility command that abandons a run.
//!
//! A cheat needs arming, and arming marks the run. The mark is the whole point:
//! a run that was ever armed was never clean, and only a fresh scenario is a
//! fresh attempt.

use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond;
use nova_gameplay::prelude::*;
use nova_os::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::FlightSpeedCap;

use crate::{
    lookup::{self, Resolved},
    units::cap_label,
};

const CLASS: CommandClass = CommandClass::Cheat;

/// Whether a cheat may run, as a refusal the caller can return.
///
/// `cheats enable` is the arming act and is exempt: it is the one command
/// whose whole purpose is to make the others available.
pub fn refuse_unarmed(world: &World, name: &str) -> Option<CommandResult> {
    let armed = world
        .get_resource::<RunCheats>()
        .is_some_and(|cheats| cheats.is_armed());
    if armed {
        return None;
    }
    Some(CommandResult::refused(
        name,
        CLASS,
        format!("{name}: cheats are not armed - run `cheats enable` first"),
    ))
}

/// `cheats enable`: arm cheats and mark the run.
pub fn enable(world: &mut World) -> CommandResult {
    let Some(mut cheats) = world.get_resource_mut::<RunCheats>() else {
        return CommandResult::error("cheats enable", Some(CLASS), "no run to arm");
    };
    if !cheats.arm() {
        return CommandResult::ok("cheats enable", CLASS, "already armed")
            .with_rows(vec![TerminalRow::warn("Cheats are already armed.")]);
    }
    CommandResult::ok("cheats enable", CLASS, "armed; this run is marked").with_rows(vec![
        TerminalRow::warn("Cheats armed. THIS RUN IS MARKED."),
        TerminalRow::dim("The mark stays until a fresh scenario is loaded."),
    ])
}

/// `ammo infinite <ship-id> <on|off>`.
pub fn ammo_infinite(world: &mut World, ship_id: &str, state: &str) -> Resolved {
    let enabled = match state.to_ascii_lowercase().as_str() {
        "on" => true,
        "off" => false,
        _ => {
            return Err(CommandResult::error(
                "ammo infinite",
                Some(CLASS),
                format!("ammo infinite: '{state}' is not on or off"),
            ))
        }
    };
    let ship = lookup::ship(world, ship_id).or_error("ammo infinite", CLASS)?;
    let changed = lookup::sections(world, ship)
        .into_iter()
        .filter(|(section, _)| apply_infinite_ammo(world, *section, enabled))
        .count();
    let word = if enabled { "unlimited" } else { "finite" };
    if changed == 0 {
        return Ok(
            CommandResult::ok("ammo infinite", CLASS, format!("{ship_id}: already {word}"))
                .with_rows(vec![TerminalRow::warn(format!(
                    "'{ship_id}' has no magazine to make {word}."
                ))]),
        );
    }
    Ok(CommandResult::ok(
        "ammo infinite",
        CLASS,
        format!("{ship_id}: {changed} weapons {word}"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: {changed} weapons now fire {word}."
    ))]))
}

/// `ammo refill <ship-id>`.
pub fn ammo_refill(world: &mut World, ship_id: &str) -> Resolved {
    let ship = lookup::ship(world, ship_id).or_error("ammo refill", CLASS)?;
    let refilled = lookup::sections(world, ship)
        .into_iter()
        .filter(|(section, _)| refill_section(world, *section))
        .count();
    if refilled == 0 {
        return Ok(CommandResult::ok(
            "ammo refill",
            CLASS,
            format!("{ship_id}: nothing to refill"),
        )
        .with_rows(vec![TerminalRow::warn(format!(
            "'{ship_id}' has no finite magazine to refill."
        ))]));
    }
    Ok(CommandResult::ok(
        "ammo refill",
        CLASS,
        format!("{ship_id}: {refilled} magazines full"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: {refilled} magazines refilled."
    ))]))
}

/// `ammo refill section <ship-id> <section-id>`.
pub fn ammo_refill_section(world: &mut World, ship_id: &str, section_id: &str) -> Resolved {
    const NAME: &str = "ammo refill section";
    let ship = lookup::ship(world, ship_id).or_error(NAME, CLASS)?;
    let section = lookup::section(world, ship, section_id).or_error(NAME, CLASS)?;
    if !refill_section(world, section) {
        return Ok(
            CommandResult::ok(NAME, CLASS, format!("{section_id}: nothing to refill")).with_rows(
                vec![TerminalRow::warn(format!(
                    "'{section_id}' has no finite magazine."
                ))],
            ),
        );
    }
    Ok(
        CommandResult::ok(NAME, CLASS, format!("{section_id}: magazine full")).with_rows(vec![
            TerminalRow::warn(format!("{ship_id} {section_id}: magazine refilled.")),
        ]),
    )
}

/// `speed-cap <ship-id> <number|off>`.
pub fn speed_cap(world: &mut World, ship_id: &str, value: &str) -> Resolved {
    // Engine boundary in: the player says meters per second. The way back out
    // is `cap_label`.
    let cap = if value.eq_ignore_ascii_case("off") {
        None
    } else {
        match value.parse::<f32>() {
            // `inf` parses and is greater than zero, and a cap that can never
            // be reached is not a cap; `nan` fails the comparison already.
            Ok(meters) if meters > 0.0 && meters.is_finite() => {
                Some(MetersPerSecond(meters).to_engine())
            }
            Ok(_) => {
                return Err(CommandResult::error(
                    "speed-cap",
                    Some(CLASS),
                    "speed-cap: a cap must be a real speed greater than zero, or 'off'",
                ))
            }
            Err(_) => {
                return Err(CommandResult::error(
                    "speed-cap",
                    Some(CLASS),
                    format!("speed-cap: '{value}' is not a number or 'off'"),
                ))
            }
        }
    };
    let ship = lookup::ship(world, ship_id).or_error("speed-cap", CLASS)?;
    let mut entity = world.entity_mut(ship);
    match cap {
        Some(cap) => {
            entity.insert(FlightSpeedCap(cap));
        }
        None => {
            entity.remove::<FlightSpeedCap>();
        }
    }
    let detail = match cap {
        Some(cap) => format!("{ship_id}: cap {}", cap_label(cap)),
        None => format!("{ship_id}: cap removed"),
    };
    Ok(CommandResult::ok("speed-cap", CLASS, detail.clone())
        .with_rows(vec![TerminalRow::warn(detail)]))
}

/// `scenario load <id>`: abandon the attempt and start a fresh one.
///
/// Utility, not Cheat: it decides nothing about how the abandoned attempt ended.
/// It assigns no outcome and advances no campaign, so a scenario left this way
/// is neither won nor lost - and the new run starts clean, because the loader's
/// teardown clears the arming and the mark on every road out of a scenario.
pub fn scenario_load(world: &mut World, id: &str) -> CommandResult {
    const NAME: &str = "scenario load";
    const CLASS: CommandClass = CommandClass::Utility;
    // The menu owns its own transition into a scenario (loading screen, camera,
    // state). Triggering the loader from under it spawns a scenario the menu is
    // still covering, so the shell refuses rather than half-starting a run.
    if world.get_resource::<State<GameStates>>().map(State::get) != Some(&GameStates::Playing) {
        return CommandResult::refused(
            NAME,
            CLASS,
            format!("{NAME}: only from a running game - start one from the menu first"),
        );
    }
    let Some(scenarios) = world.get_resource::<GameScenarios>() else {
        return CommandResult::error(NAME, Some(CLASS), "no scenarios are loaded");
    };
    let Some(config) = scenarios.get(id).cloned() else {
        let mut known: Vec<&str> = scenarios.keys().map(String::as_str).collect();
        known.sort_unstable();
        return CommandResult::error(
            NAME,
            Some(CLASS),
            format!("no scenario named '{id}' ({})", known.join(", ")),
        );
    };
    // A stale report from an earlier refusal would read back as this load's, so
    // clear it before the trigger; the loader files a fresh one if it refuses.
    if world
        .get_resource::<ScenarioStartFailure>()
        .is_some_and(|failure| failure.0.is_some())
    {
        world.resource_mut::<ScenarioStartFailure>().0 = None;
    }
    world.trigger(LoadScenario(config));
    // The loader refuses a scenario with Error-level findings and files the
    // report instead of tearing anything down, so the ack has to read what the
    // trigger actually did rather than assume it started.
    if let Some(report) = world
        .get_resource::<ScenarioStartFailure>()
        .and_then(|failure| failure.0.clone())
    {
        let rows = report
            .messages
            .iter()
            .map(|message| TerminalRow::output(format!("  {message}")))
            .collect();
        return CommandResult::error(
            NAME,
            Some(CLASS),
            format!(
                "'{id}' refused to start ({} content errors)",
                report.messages.len()
            ),
        )
        .with_rows(rows);
    }
    CommandResult::ok(NAME, CLASS, format!("loading {id}")).with_rows(vec![
        TerminalRow::info(format!("Loading '{id}'.")),
        TerminalRow::dim("The abandoned attempt has no outcome; the new run is clean."),
    ])
}

#[cfg(test)]
mod tests {
    use nova_scenario::prelude::ScenarioConfig;

    use super::*;

    /// A world with a scenario registry and a game state, which is what
    /// `scenario load` reads before it triggers anything.
    fn world_at(state: GameStates) -> World {
        let mut world = World::new();
        world.insert_resource(State::new(state));
        let mut scenarios = GameScenarios::default();
        scenarios.insert(
            "shakedown_run".to_string(),
            ScenarioConfig::new(
                "shakedown_run".to_string(),
                "Shakedown Run".to_string(),
                Handle::default().into(),
            ),
        );
        world.insert_resource(scenarios);
        world
    }

    /// The menu owns its own way into a scenario, so the shell refuses rather
    /// than spawning a run under a menu that is still covering it.
    #[test]
    fn scenario_load_is_refused_outside_a_running_game() {
        for state in [GameStates::MainMenu, GameStates::Loading] {
            let mut world = world_at(state.clone());
            let result = scenario_load(&mut world, "shakedown_run");
            assert_eq!(result.status, CommandStatus::Refused, "{state:?}");
        }
    }

    /// An id nobody has lists the ids that exist, so the next attempt can be
    /// typed rather than guessed.
    #[test]
    fn an_unknown_scenario_lists_the_known_ones() {
        let mut world = world_at(GameStates::Playing);
        let result = scenario_load(&mut world, "nope");
        assert_eq!(result.status, CommandStatus::Error);
        assert!(result.detail.contains("shakedown_run"), "{}", result.detail);
    }

    /// A cap that can never be reached is not a cap.
    #[test]
    fn a_speed_cap_has_to_be_a_real_speed() {
        let mut world = World::new();
        for value in ["inf", "-inf", "nan", "0", "-5"] {
            let Err(refusal) = speed_cap(&mut world, "block_gunship", value) else {
                panic!("'{value}' is not a speed cap");
            };
            assert_eq!(refusal.status, CommandStatus::Error, "{value}");
        }
    }
}
