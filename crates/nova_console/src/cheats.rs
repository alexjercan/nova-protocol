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
use nova_ship::prelude::{FlightSpeedCap, SuspendedSectionAmmo};

use crate::lookup;

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
pub fn ammo_infinite(world: &mut World, ship_id: &str, state: &str) -> CommandResult {
    let enabled = match state.to_ascii_lowercase().as_str() {
        "on" => true,
        "off" => false,
        _ => {
            return CommandResult::error(
                "ammo infinite",
                Some(CLASS),
                format!("ammo infinite: '{state}' is not on or off"),
            )
        }
    };
    let ship = match lookup::ship(world, ship_id).or_error("ammo infinite", CLASS) {
        Ok(ship) => ship,
        Err(result) => return result,
    };
    let mut changed = 0usize;
    for (section, _) in lookup::sections(world, ship) {
        let was_unlimited = world
            .entity(section)
            .get::<SuspendedSectionAmmo>()
            .is_some();
        apply_infinite_ammo(world, section, enabled);
        if world
            .entity(section)
            .get::<SuspendedSectionAmmo>()
            .is_some()
            != was_unlimited
        {
            changed += 1;
        }
    }
    let word = if enabled { "unlimited" } else { "finite" };
    if changed == 0 {
        return CommandResult::ok("ammo infinite", CLASS, format!("{ship_id}: already {word}"))
            .with_rows(vec![TerminalRow::warn(format!(
                "'{ship_id}' has no magazine to make {word}."
            ))]);
    }
    CommandResult::ok(
        "ammo infinite",
        CLASS,
        format!("{ship_id}: {changed} weapons {word}"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: {changed} weapons now fire {word}."
    ))])
}

/// `ammo refill <ship-id>`.
pub fn ammo_refill(world: &mut World, ship_id: &str) -> CommandResult {
    let ship = match lookup::ship(world, ship_id).or_error("ammo refill", CLASS) {
        Ok(ship) => ship,
        Err(result) => return result,
    };
    let refilled = lookup::sections(world, ship)
        .into_iter()
        .filter(|(section, _)| refill_section(world, *section))
        .count();
    if refilled == 0 {
        return CommandResult::ok(
            "ammo refill",
            CLASS,
            format!("{ship_id}: nothing to refill"),
        )
        .with_rows(vec![TerminalRow::warn(format!(
            "'{ship_id}' has no finite magazine to refill."
        ))]);
    }
    CommandResult::ok(
        "ammo refill",
        CLASS,
        format!("{ship_id}: {refilled} magazines full"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: {refilled} magazines refilled."
    ))])
}

/// `ammo refill section <section-id>`.
pub fn ammo_refill_section(world: &mut World, section_id: &str) -> CommandResult {
    let section = match lookup::section(world, section_id).or_error("ammo refill section", CLASS) {
        Ok(section) => section,
        Err(result) => return result,
    };
    if !refill_section(world, section) {
        return CommandResult::ok(
            "ammo refill section",
            CLASS,
            format!("{section_id}: nothing to refill"),
        )
        .with_rows(vec![TerminalRow::warn(format!(
            "'{section_id}' has no finite magazine."
        ))]);
    }
    CommandResult::ok(
        "ammo refill section",
        CLASS,
        format!("{section_id}: magazine full"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{section_id}: magazine refilled."
    ))])
}

/// `speed-cap <ship-id> <number|off>`.
pub fn speed_cap(world: &mut World, ship_id: &str, value: &str) -> CommandResult {
    // Engine boundary: the player says metres per second, because every figure
    // the game shows is in metres, and `FlightSpeedCap` is compared against an
    // avian velocity every tick. The cap crosses here, once, in each direction.
    let cap = if value.eq_ignore_ascii_case("off") {
        None
    } else {
        match value.parse::<f32>() {
            Ok(metres) if metres > 0.0 => Some(MetersPerSecond(metres).to_engine()),
            Ok(_) => {
                return CommandResult::error(
                    "speed-cap",
                    Some(CLASS),
                    "speed-cap: a cap must be greater than zero, or 'off'",
                )
            }
            Err(_) => {
                return CommandResult::error(
                    "speed-cap",
                    Some(CLASS),
                    format!("speed-cap: '{value}' is not a number or 'off'"),
                )
            }
        }
    };
    let ship = match lookup::ship(world, ship_id).or_error("speed-cap", CLASS) {
        Ok(ship) => ship,
        Err(result) => return result,
    };
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
        Some(cap) => format!(
            "{ship_id}: cap {:.0} m/s",
            MetersPerSecond::from_engine(cap).get()
        ),
        None => format!("{ship_id}: cap removed"),
    };
    CommandResult::ok("speed-cap", CLASS, detail.clone()).with_rows(vec![TerminalRow::warn(detail)])
}

/// `scenario load <id>`: abandon the attempt and start a fresh one.
///
/// Utility, not Cheat: it decides nothing about how the abandoned attempt ended.
/// It assigns no outcome and advances no campaign, so a scenario left this way
/// is neither won nor lost - and the new run starts clean, arming and mark
/// cleared.
pub fn scenario_load(world: &mut World, id: &str) -> CommandResult {
    const NAME: &str = "scenario load";
    let Some(scenarios) = world.get_resource::<GameScenarios>() else {
        return CommandResult::error(NAME, Some(CommandClass::Utility), "no scenarios are loaded");
    };
    let Some(config) = scenarios.get(id).cloned() else {
        let mut known: Vec<&str> = scenarios.keys().map(String::as_str).collect();
        known.sort_unstable();
        return CommandResult::error(
            NAME,
            Some(CommandClass::Utility),
            format!("no scenario named '{id}' ({})", known.join(", ")),
        );
    };
    if let Some(mut cheats) = world.get_resource_mut::<RunCheats>() {
        cheats.begin_new_run();
    }
    if let Some(mut outcome) = world.get_resource_mut::<CurrentOutcome>() {
        outcome.0 = None;
    }
    world.trigger(LoadScenario(config));
    CommandResult::ok(NAME, CommandClass::Utility, format!("loading {id}")).with_rows(vec![
        TerminalRow::info(format!("Loading '{id}'.")),
        TerminalRow::dim("The abandoned attempt has no outcome; the new run is clean."),
    ])
}
