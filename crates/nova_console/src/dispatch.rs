//! Routing one parsed command to the thing that runs it.
//!
//! [`resolve_command_line`](nova_os::prelude::resolve_command_line) has already
//! matched the name, checked the arity and answered everything the catalog
//! alone could answer. What arrives here is a [`CommandInvocation`]: a catalog
//! name, its class, and the argument words. One `match` on the name is the
//! whole routing table, and it is exhaustive against the catalog by test.

use bevy::prelude::*;
use nova_os::prelude::*;

use crate::{cheats, inspect, lookup::answered, settings};

/// Run one command against the live world.
///
/// The single execution point: the CRT prompt and the process channel both
/// arrive here, so neither can grow a command the other does not have.
pub fn execute(world: &mut World, invocation: &CommandInvocation) -> CommandResult {
    // Arming is checked once, here, rather than in each cheat: a class is the
    // permission model, and a cheat that forgot to ask would be a silent hole.
    if invocation.class == CommandClass::Cheat && invocation.name != "cheats enable" {
        if let Some(refusal) = cheats::refuse_unarmed(world, invocation.name) {
            return refusal;
        }
    }
    let arg = |at: usize| invocation.args.get(at).map(String::as_str).unwrap_or("");
    match invocation.name {
        // Utility. `clear` and `close` are shell control: the dispatcher
        // reports them and the front end acts, because only it has a screen.
        "clear" => CommandResult::ok("clear", CommandClass::Utility, "cleared"),
        "close" => CommandResult::ok("close", CommandClass::Utility, "closing"),
        "scenario load" => cheats::scenario_load(world, arg(0)),

        // ReadOnly.
        "status" => inspect::status(world),
        "scenario" => inspect::scenario(world),
        "ships" => inspect::ships(world),
        "ship" => answered(inspect::ship(world, arg(0))),
        "sections" => answered(inspect::sections(world, arg(0))),
        "section" => answered(inspect::section(world, arg(0), arg(1))),
        "objectives" => inspect::objectives(world),
        "variables" => inspect::variables(world),
        "variable" => inspect::variable(world, arg(0)),
        "bindings" => inspect::bindings(world, invocation.args.first().map(String::as_str)),
        "settings" => settings::settings(world),
        "cheats status" => inspect::cheats_status(world),

        // Setting.
        "graphics" => settings::graphics(world, invocation.args.first().map(String::as_str)),
        "volume" => settings::volume(world, &invocation.args),
        "window" => settings::window(world, invocation.args.first().map(String::as_str)),
        "bind" => settings::bind(world, arg(0), arg(1)),
        "bind reset" => settings::bind_reset(world, arg(0)),

        // Cheat.
        "cheats enable" => cheats::enable(world),
        "ammo infinite" => answered(cheats::ammo_infinite(world, arg(0), arg(1))),
        "ammo refill" => answered(cheats::ammo_refill(world, arg(0))),
        "ammo refill section" => answered(cheats::ammo_refill_section(world, arg(0), arg(1))),
        "speed-cap" => answered(cheats::speed_cap(world, arg(0), arg(1))),

        name => CommandResult::error(
            name,
            Some(invocation.class),
            format!("{name}: in the catalog but not wired to anything"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world with nothing in it. Every command must answer rather than
    /// panic: the shell opens over the main menu, where most of these
    /// resources do not exist yet.
    fn bare_world() -> World {
        let mut world = World::new();
        world.init_resource::<nova_gameplay::prelude::RunCheats>();
        world
    }

    /// The routing table and the catalog are one list stated twice, and the
    /// fallback arm makes a missing route silent. This is what catches it.
    #[test]
    fn every_catalog_command_is_wired_to_something() {
        let mut world = bare_world();
        world
            .resource_mut::<nova_gameplay::prelude::RunCheats>()
            .arm();
        for spec in COMMAND_CATALOG {
            // `help` and `commands` are answered by the parser and never reach
            // the dispatcher.
            if matches!(spec.name, "help" | "commands") {
                continue;
            }
            let result = execute(
                &mut world,
                &CommandInvocation {
                    name: spec.name,
                    class: spec.class,
                    args: Vec::new(),
                },
            );
            assert!(
                !result.detail.contains("not wired"),
                "{} reached the fallback arm",
                spec.name
            );
        }
    }

    /// The permission model is one check on the class, so a new cheat is
    /// covered the moment it is added to the catalog.
    #[test]
    fn every_cheat_but_the_arming_one_is_refused_before_arming() {
        let mut world = bare_world();
        for spec in COMMAND_CATALOG {
            if spec.class != CommandClass::Cheat || spec.name == "cheats enable" {
                continue;
            }
            let result = execute(
                &mut world,
                &CommandInvocation {
                    name: spec.name,
                    class: spec.class,
                    args: vec!["player_ship".to_string(), "on".to_string()],
                },
            );
            assert_eq!(
                result.status,
                CommandStatus::Refused,
                "{} ran without arming",
                spec.name
            );
        }
        assert!(
            !world
                .resource::<nova_gameplay::prelude::RunCheats>()
                .is_marked(),
            "a refused cheat must not mark the run"
        );
    }

    /// Arming is the one cheat that needs no arming, and it marks the run.
    #[test]
    fn arming_is_the_one_cheat_that_runs_unarmed() {
        let mut world = bare_world();
        let result = execute(
            &mut world,
            &CommandInvocation {
                name: "cheats enable",
                class: CommandClass::Cheat,
                args: Vec::new(),
            },
        );
        assert_eq!(result.status, CommandStatus::Ok);
        assert!(world
            .resource::<nova_gameplay::prelude::RunCheats>()
            .is_marked());
    }
}
