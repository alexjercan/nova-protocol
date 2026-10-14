//! The Cheat commands, and the one Utility command that abandons a run.
//!
//! A cheat needs arming, and arming marks the run. The mark is the whole point:
//! a run that was ever armed was never clean, and only a fresh scenario is a
//! fresh attempt.

use bevy::prelude::*;
use nova_command::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

use crate::lookup::{self, Resolved};

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

/// `item give <ship-id> <item-id> <quantity>`.
///
/// Every check runs before the inventory changes, so a refused give adds
/// nothing. The give never splits to fit: the whole quantity fits the ship's
/// free hold, or none of it is added.
pub fn item_give(world: &mut World, ship_id: &str, item_id: &str, quantity: &str) -> Resolved {
    const NAME: &str = "item give";
    let Some(items) = world.get_resource::<GameItems>().cloned() else {
        return Err(CommandResult::error(
            NAME,
            Some(CLASS),
            format!("{NAME}: no item catalog is loaded; the world is inconsistent"),
        ));
    };
    let item = ItemDesignId::from(item_id);
    let Some(design) = items.get(&item).cloned() else {
        let known: Vec<&str> = items.iter().map(|design| design.id.as_str()).collect();
        return Err(CommandResult::error(
            NAME,
            Some(CLASS),
            format!("{NAME}: no item named '{item_id}' ({})", known.join(", ")),
        ));
    };
    let Some(count) = quantity.parse::<u32>().ok().filter(|count| *count > 0) else {
        return Err(CommandResult::error(
            NAME,
            Some(CLASS),
            format!("{NAME}: '{quantity}' is not a positive whole number"),
        ));
    };
    let ship = lookup::ship(world, ship_id).or_error(NAME, CLASS)?;
    // Every ship root requires a `ShipInventory`, so a ship without one is a
    // broken world, not a ship with an empty hold.
    let Some(mut inventory) = world.get_mut::<ShipInventory>(ship) else {
        return Err(CommandResult::error(
            NAME,
            Some(CLASS),
            format!("'{ship_id}' has no inventory; the world is inconsistent"),
        ));
    };
    let (label, mass_g, free_g) = (
        design.name.as_str(),
        design.stack_mass_g(count),
        inventory.free_g(&items),
    );
    if mass_g > u64::from(free_g) {
        return Err(CommandResult::refused(
            NAME,
            CLASS,
            format!(
                "{NAME}: {ship_id} has room for {} more; {count} {label} weigh {}",
                kg_text(u64::from(free_g)),
                kg_text(mass_g),
            ),
        ));
    }
    inventory.add(&items, &item, count);
    let held = inventory.count(&item);
    let load = format!(
        "{} / {}",
        kg_text(u64::from(inventory.used_g(&items))),
        kg_text(u64::from(inventory.capacity_g())),
    );
    Ok(CommandResult::ok(
        NAME,
        CLASS,
        format!("{ship_id}: +{count} {label}, {held} held"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: {count} {label} added; {held} held, {load}."
    ))]))
}

/// `credits give <ship-id> <amount>`: add a signed amount to the balance.
pub fn credits_give(world: &mut World, ship_id: &str, amount: &str) -> Resolved {
    credits_write(world, "credits give", ship_id, amount, |held, typed| {
        i128::from(held) + i128::from(typed)
    })
}

/// `credits set <ship-id> <amount>`: replace the balance.
pub fn credits_set(world: &mut World, ship_id: &str, amount: &str) -> Resolved {
    credits_write(world, "credits set", ship_id, amount, |_, typed| {
        i128::from(typed)
    })
}

/// The shared body of the two `credits` cheats. `balance` turns the held
/// balance and the typed amount into the requested one.
///
/// Every check runs before the balance changes, so a refused write leaves it
/// as it was. The write goes through [`ShipInventoryReports::change_credits`],
/// the one balance writer, so `OnCreditsChanged` fires with cause `Cheat`.
fn credits_write(
    world: &mut World,
    name: &str,
    ship_id: &str,
    amount: &str,
    balance: impl FnOnce(u32, i64) -> i128,
) -> Resolved {
    let Ok(typed) = amount.parse::<i64>() else {
        return Err(CommandResult::error(
            name,
            Some(CLASS),
            format!("{name}: '{amount}' is not a whole number"),
        ));
    };
    let ship = lookup::ship(world, ship_id).or_error(name, CLASS)?;
    // Every ship root requires `ShipCredits` and `ShipInventoryReports`, so a
    // ship without them is a broken world, not a ship with an empty purse.
    let mut query = world.query::<(&mut ShipCredits, &mut ShipInventoryReports)>();
    let Ok((mut credits, mut reports)) = query.get_mut(world, ship) else {
        return Err(CommandResult::error(
            name,
            Some(CLASS),
            format!("'{ship_id}' holds no credits; the world is inconsistent"),
        ));
    };
    let held = credits.0;
    let requested = balance(held, typed);
    let Ok(current_cr) = u32::try_from(requested) else {
        let limit = if requested < 0 {
            "below zero".to_string()
        } else {
            format!("above the {} cr limit", u32::MAX)
        };
        return Err(CommandResult::refused(
            name,
            CLASS,
            format!("{name}: {ship_id} holds {held} cr; {requested} cr is {limit}"),
        ));
    };
    reports.change_credits(&mut credits, current_cr, CreditChangeCauseType::Cheat);
    let delta = i128::from(current_cr) - i128::from(held);
    Ok(CommandResult::ok(
        name,
        CLASS,
        format!("{ship_id}: {delta:+} cr, {current_cr} cr held"),
    )
    .with_rows(vec![TerminalRow::warn(format!(
        "{ship_id}: credits {held} cr -> {current_cr} cr."
    ))]))
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
    use nova_events::prelude::EntityId;
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

    /// Armed or not, a world with one ship whose 400 kg hold carries 12 hull
    /// plates (120 kg).
    fn world_with_hold(armed: bool) -> (World, Entity) {
        let mut world = World::new();
        let mut cheats = RunCheats::default();
        if armed {
            cheats.arm();
        }
        world.insert_resource(cheats);
        let items = nova_gameplay::test_support::test_items();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                EntityId("player_spaceship".to_string()),
                ShipInventory::new(
                    &items,
                    400_000,
                    [(ItemDesignId::from(ItemRoleType::HullPlate), 12)],
                ),
            ))
            .id();
        world.insert_resource(items);
        (world, ship)
    }

    fn give(world: &mut World, args: [&str; 3]) -> CommandResult {
        crate::dispatch::execute(
            world,
            &CommandInvocation {
                name: "item give",
                class: CLASS,
                args: args.map(str::to_string).to_vec(),
            },
        )
    }

    /// Each catalog item id adds its whole quantity, and the hold's load grows
    /// by the item's fixed mass until it is exactly full.
    #[test]
    fn item_give_adds_every_item_id_and_its_mass_to_the_hold() {
        let (mut world, ship) = world_with_hold(true);
        let hold = |world: &World| world.get::<ShipInventory>(ship).expect("hold").clone();

        let result = give(&mut world, ["player_spaceship", "HullPlate", "5"]);
        assert_eq!(result.status, CommandStatus::Ok, "{}", result.detail);
        assert_eq!(result.detail, "player_spaceship: +5 Hull plate, 17 held");
        assert_eq!(
            result.rows[0].text,
            "player_spaceship: 5 Hull plate added; 17 held, 170 kg / 400 kg."
        );
        // 170 kg + 150 kg torpedo + 20 kg slug leaves 60 kg: 300 rounds of 0.2 kg.
        for (id, count) in [("Torpedo", "1"), ("RailSlug", "1"), ("PdcRound", "300")] {
            let result = give(&mut world, ["player_spaceship", id, count]);
            assert_eq!(result.status, CommandStatus::Ok, "{id}: {}", result.detail);
        }
        let items = world.resource::<GameItems>().clone();
        let hold = hold(&world);
        assert_eq!(
            hold.stacks().collect::<Vec<_>>(),
            [
                (&ItemDesignId::from(ItemRoleType::HullPlate), 17),
                (&ItemDesignId::from(ItemRoleType::PdcRound), 300),
                (&ItemDesignId::from(ItemRoleType::RailSlug), 1),
                (&ItemDesignId::from(ItemRoleType::Torpedo), 1),
            ]
        );
        assert_eq!(hold.used_g(&items), 400_000);
        assert_eq!(hold.free_g(&items), 0);
    }

    /// A give that is unarmed, names no single live ship, names an unknown
    /// item, types a bad quantity or does not fit adds nothing.
    #[test]
    fn a_refused_item_give_adds_nothing() {
        let (mut world, ship) = world_with_hold(true);
        world.spawn((SpaceshipRootMarker, EntityId("twin".to_string())));
        world.spawn((SpaceshipRootMarker, EntityId("twin".to_string())));
        let before = world.get::<ShipInventory>(ship).expect("hold").clone();

        for (args, status, says) in [
            (
                ["nobody", "HullPlate", "1"],
                CommandStatus::Error,
                "no ship named 'nobody'",
            ),
            (
                ["twin", "HullPlate", "1"],
                CommandStatus::Error,
                "names 2 ships",
            ),
            (
                ["player_spaceship", "hullplate", "1"],
                CommandStatus::Error,
                "no item named 'hullplate' (ActuatorPack, CapacitorBank, CarbonOre, DockingCollar, DriveBell, FlightCore, FrameSpar, HullPlate, IronOre, PdcRound, RailSlug, Rations, SalvagedParts, SectionPanel, StoneOre, Torpedo, TorpedoTube, TurretBarrel, TurretBody, WaterIce)",
            ),
            (
                ["player_spaceship", "HullPlate", "0"],
                CommandStatus::Error,
                "'0' is not",
            ),
            (
                ["player_spaceship", "HullPlate", "-1"],
                CommandStatus::Error,
                "'-1' is not",
            ),
            (
                ["player_spaceship", "HullPlate", "1.5"],
                CommandStatus::Error,
                "'1.5' is not",
            ),
            // 280 kg free: 29 plates weigh 290 kg and are refused whole.
            (
                ["player_spaceship", "HullPlate", "29"],
                CommandStatus::Refused,
                "has room for 280 kg more; 29 Hull plate weigh 290 kg",
            ),
            // The largest count weighs without overflow and is refused.
            (
                ["player_spaceship", "Torpedo", "4294967295"],
                CommandStatus::Refused,
                "has room for 280 kg more",
            ),
        ] {
            let result = give(&mut world, args);
            assert_eq!(result.status, status, "{args:?}: {}", result.detail);
            assert!(result.detail.contains(says), "{args:?}: {}", result.detail);
        }
        assert_eq!(world.get::<ShipInventory>(ship), Some(&before));

        let (mut unarmed, ship) = world_with_hold(false);
        let result = give(&mut unarmed, ["player_spaceship", "HullPlate", "1"]);
        assert_eq!(result.status, CommandStatus::Refused);
        assert_eq!(unarmed.get::<ShipInventory>(ship), Some(&before));
    }

    /// Armed or not, a world with one ship that holds 410 cr.
    fn world_with_purse(armed: bool) -> (World, Entity) {
        let mut world = World::new();
        let mut cheats = RunCheats::default();
        if armed {
            cheats.arm();
        }
        world.insert_resource(cheats);
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                EntityId("player_spaceship".to_string()),
                ShipCredits(410),
            ))
            .id();
        (world, ship)
    }

    fn credits(world: &mut World, name: &'static str, args: [&str; 2]) -> CommandResult {
        crate::dispatch::execute(
            world,
            &CommandInvocation {
                name,
                class: CLASS,
                args: args.map(str::to_string).to_vec(),
            },
        )
    }

    /// `credits give` adds a signed amount, `credits set` replaces the
    /// balance, and each change is queued once for `OnCreditsChanged` with
    /// cause `Cheat`.
    #[test]
    fn credits_give_and_set_write_the_balance_through_the_one_writer() {
        let (mut world, ship) = world_with_purse(true);

        let result = credits(&mut world, "credits give", ["player_spaceship", "590"]);
        assert_eq!(result.status, CommandStatus::Ok, "{}", result.detail);
        assert_eq!(result.detail, "player_spaceship: +590 cr, 1000 cr held");
        assert_eq!(
            result.rows[0].text,
            "player_spaceship: credits 410 cr -> 1000 cr."
        );
        let result = credits(&mut world, "credits give", ["player_spaceship", "-400"]);
        assert_eq!(result.status, CommandStatus::Ok, "{}", result.detail);
        assert_eq!(world.get::<ShipCredits>(ship), Some(&ShipCredits(600)));

        let result = credits(&mut world, "credits set", ["player_spaceship", "75"]);
        assert_eq!(result.status, CommandStatus::Ok, "{}", result.detail);
        assert_eq!(result.detail, "player_spaceship: -525 cr, 75 cr held");
        assert_eq!(world.get::<ShipCredits>(ship), Some(&ShipCredits(75)));

        let reports = &world.get::<ShipInventoryReports>(ship).expect("reports").0;
        let moves: Vec<(u32, u32)> = reports
            .iter()
            .map(|report| match report {
                ShipInventoryReport::CreditsChanged {
                    previous_cr,
                    current_cr,
                    cause: CreditChangeCauseType::Cheat,
                } => (*previous_cr, *current_cr),
                other => panic!("unexpected report {other:?}"),
            })
            .collect();
        assert_eq!(moves, [(410, 1000), (1000, 600), (600, 75)]);
    }

    /// A write that is unarmed, would leave the balance below zero or past
    /// the `u32` limit, types a non-integer amount, names no single live ship
    /// or names a ship without a purse changes nothing.
    #[test]
    fn a_refused_credits_write_leaves_the_balance() {
        let (mut world, ship) = world_with_purse(true);
        world.spawn((SpaceshipRootMarker, EntityId("twin".to_string())));
        world.spawn((SpaceshipRootMarker, EntityId("twin".to_string())));
        let broken = world
            .spawn((SpaceshipRootMarker, EntityId("broken".to_string())))
            .id();
        world.entity_mut(broken).remove::<ShipCredits>();

        for (name, args, status, says) in [
            (
                "credits give",
                ["player_spaceship", "-411"],
                CommandStatus::Refused,
                "credits give: player_spaceship holds 410 cr; -1 cr is below zero",
            ),
            (
                "credits set",
                ["player_spaceship", "-1"],
                CommandStatus::Refused,
                "credits set: player_spaceship holds 410 cr; -1 cr is below zero",
            ),
            (
                "credits give",
                ["player_spaceship", "4294967295"],
                CommandStatus::Refused,
                "is above the 4294967295 cr limit",
            ),
            (
                "credits set",
                ["player_spaceship", "1.5"],
                CommandStatus::Error,
                "credits set: '1.5' is not a whole number",
            ),
            (
                "credits give",
                ["player_spaceship", "lots"],
                CommandStatus::Error,
                "'lots' is not a whole number",
            ),
            (
                "credits give",
                ["nobody", "10"],
                CommandStatus::Error,
                "no ship named 'nobody'",
            ),
            (
                "credits set",
                ["twin", "10"],
                CommandStatus::Error,
                "names 2 ships",
            ),
            (
                "credits give",
                ["broken", "10"],
                CommandStatus::Error,
                "'broken' holds no credits",
            ),
        ] {
            let result = credits(&mut world, name, args);
            assert_eq!(result.status, status, "{name} {args:?}: {}", result.detail);
            assert!(
                result.detail.contains(says),
                "{name} {args:?}: {}",
                result.detail
            );
        }
        assert_eq!(world.get::<ShipCredits>(ship), Some(&ShipCredits(410)));
        let reports = world.get::<ShipInventoryReports>(ship).expect("reports");
        assert!(reports.0.is_empty(), "{:?}", reports.0);

        let (mut unarmed, ship) = world_with_purse(false);
        for name in ["credits give", "credits set"] {
            let result = credits(&mut unarmed, name, ["player_spaceship", "10"]);
            assert_eq!(result.status, CommandStatus::Refused, "{name}");
            assert!(
                result.detail.contains("cheats are not armed"),
                "{}",
                result.detail
            );
        }
        assert_eq!(unarmed.get::<ShipCredits>(ship), Some(&ShipCredits(410)));
    }
}
