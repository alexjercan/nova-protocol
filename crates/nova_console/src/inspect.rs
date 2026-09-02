//! The ReadOnly commands: everything that looks and never touches.
//!
//! Each one answers from live state and returns rows, so the CRT and the
//! channel see the same text. Nothing here writes.

use bevy::prelude::*;
use nova_events::prelude::MetersPerSecond;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;
use nova_os::prelude::*;
use nova_scenario::prelude::*;
use nova_ship::prelude::*;

use crate::{lookup, surface::world_line};

const CLASS: CommandClass = CommandClass::ReadOnly;

/// `status`: the run and the world in a few lines.
pub fn status(world: &mut World) -> CommandResult {
    let cheats = world
        .get_resource::<RunCheats>()
        .copied()
        .unwrap_or_default();
    let ships = lookup::ships(world).len();
    let objectives = world
        .get_resource::<GameObjectives>()
        .map_or(0, |objectives| objectives.objectives.len());
    let line = world_line(world);
    CommandResult::ok("status", CLASS, line.clone()).with_rows(vec![
        TerminalRow::info(format!("WORLD ........ {line}")),
        TerminalRow::output(format!("SHIPS ........ {ships} live")),
        TerminalRow::output(format!("OBJECTIVES ... {objectives} open")),
        TerminalRow::output(format!("CHEATS ....... {}", cheats.banner())),
    ])
}

/// `scenario`: what is loaded, and how it ended if it has.
pub fn scenario(world: &mut World) -> CommandResult {
    let Some(config) = world
        .get_resource::<CurrentScenario>()
        .and_then(|current| current.0.clone())
    else {
        return CommandResult::ok("scenario", CLASS, "no scenario loaded")
            .with_rows(vec![TerminalRow::warn("No scenario is loaded.")]);
    };
    let outcome = world
        .get_resource::<CurrentOutcome>()
        .and_then(|outcome| outcome.0.as_ref().map(|config| config.outcome));
    let outcome_line = match outcome {
        Some(ScenarioOutcomeKind::Victory) => "victory",
        Some(ScenarioOutcomeKind::Defeat) => "defeat",
        None => "undecided",
    };
    CommandResult::ok("scenario", CLASS, format!("{} / {outcome_line}", config.id)).with_rows(vec![
        TerminalRow::info(format!("ID ........... {}", config.id)),
        TerminalRow::output(format!("NAME ......... {}", config.name)),
        TerminalRow::output(format!("STATE ........ {}", world_line(world))),
        TerminalRow::output(format!("OUTCOME ...... {outcome_line}")),
    ])
}

/// `ships`: every live ship by id.
pub fn ships(world: &mut World) -> CommandResult {
    let ships = lookup::ships(world);
    if ships.is_empty() {
        return CommandResult::ok("ships", CLASS, "no ships are live")
            .with_rows(vec![TerminalRow::warn("No ships are live.")]);
    }
    let width = ships.iter().map(|(_, id)| id.len()).max().unwrap_or(0);
    let rows = ships
        .iter()
        .map(|(entity, id)| {
            TerminalRow::output(format!("  {id:width$}  {}", ship_summary(world, *entity)))
        })
        .collect();
    CommandResult::ok("ships", CLASS, format!("{} live", ships.len())).with_rows(rows)
}

/// `ship <id>`: one ship's identity, allegiance, hull and cap.
pub fn ship(world: &mut World, id: &str) -> CommandResult {
    let entity = match lookup::ship(world, id).or_error("ship", CLASS) {
        Ok(entity) => entity,
        Err(result) => return result,
    };
    let sections = lookup::sections(world, entity).len();
    let cap = world
        .entity(entity)
        .get::<FlightSpeedCap>()
        .map(|cap| cap.0);
    let entity_ref = world.entity(entity);
    let name = entity_ref
        .get::<Name>()
        .map_or_else(|| id.to_string(), |name| name.to_string());
    let health = entity_ref.get::<Health>().cloned();
    CommandResult::ok("ship", CLASS, format!("{id}: {sections} sections")).with_rows(vec![
        TerminalRow::info(format!("ID ........... {id}")),
        TerminalRow::output(format!("NAME ......... {name}")),
        TerminalRow::output(format!(
            "SIDE ......... {}",
            allegiance_label(world, entity)
        )),
        TerminalRow::output(format!("HULL ......... {}", health_line(health.as_ref()))),
        TerminalRow::output(format!("SECTIONS ..... {sections}")),
        TerminalRow::output(format!("SPEED CAP .... {}", speed_cap_line(cap))),
        TerminalRow::output(format!("AMMO ......... {}", ammo_line(world, entity))),
    ])
}

/// `sections <ship-id>`: one ship's sections, one line each.
pub fn sections(world: &mut World, ship_id: &str) -> CommandResult {
    let entity = match lookup::ship(world, ship_id).or_error("sections", CLASS) {
        Ok(entity) => entity,
        Err(result) => return result,
    };
    let sections = lookup::sections(world, entity);
    if sections.is_empty() {
        return CommandResult::ok("sections", CLASS, format!("{ship_id} has no sections"))
            .with_rows(vec![TerminalRow::warn(format!(
                "'{ship_id}' has no sections."
            ))]);
    }
    let width = sections.iter().map(|(_, id)| id.len()).max().unwrap_or(0);
    let rows = sections
        .iter()
        .map(|(entity, id)| {
            TerminalRow::output(format!(
                "  {id:width$}  {}",
                section_summary(world, *entity)
            ))
        })
        .collect();
    CommandResult::ok(
        "sections",
        CLASS,
        format!("{} on {ship_id}", sections.len()),
    )
    .with_rows(rows)
}

/// `section <id>`: one section's kind, integrity and magazine.
pub fn section(world: &mut World, id: &str) -> CommandResult {
    let entity = match lookup::section(world, id).or_error("section", CLASS) {
        Ok(entity) => entity,
        Err(result) => return result,
    };
    let entity_ref = world.entity(entity);
    let health = entity_ref.get::<Health>().cloned();
    let ammo = entity_ref.get::<SectionAmmo>().copied();
    let reload = entity_ref.get::<SectionReload>().copied();
    let unlimited = entity_ref.get::<SuspendedSectionAmmo>().is_some();
    let kind = section_kind_label(world, entity);
    CommandResult::ok("section", CLASS, format!("{id}: {kind}")).with_rows(vec![
        TerminalRow::info(format!("ID ........... {id}")),
        TerminalRow::output(format!("KIND ......... {kind}")),
        TerminalRow::output(format!("INTEGRITY .... {}", health_line(health.as_ref()))),
        TerminalRow::output(format!("MAGAZINE ..... {}", magazine_line(ammo, unlimited))),
        TerminalRow::output(format!("RELOAD ....... {}", reload_line(reload))),
    ])
}

/// `objectives`: what the panel is showing.
pub fn objectives(world: &mut World) -> CommandResult {
    let objectives = world
        .get_resource::<GameObjectives>()
        .map(|objectives| objectives.objectives.clone())
        .unwrap_or_default();
    if objectives.is_empty() {
        return CommandResult::ok("objectives", CLASS, "no open objectives")
            .with_rows(vec![TerminalRow::warn("No objectives are open.")]);
    }
    // The panel holds only OPEN objectives - completing one removes it - so an
    // objective that is here is open, and there is no completed half to list.
    let rows = objectives
        .iter()
        .map(|objective| {
            TerminalRow::output(format!("  [ ] {}  {}", objective.id, objective.message))
        })
        .collect();
    CommandResult::ok("objectives", CLASS, format!("{} open", objectives.len())).with_rows(rows)
}

/// `variables`: every scenario variable and its value.
pub fn variables(world: &mut World) -> CommandResult {
    let Some(event_world) = world.get_resource::<NovaEventWorld>() else {
        return CommandResult::ok("variables", CLASS, "no scenario runtime")
            .with_rows(vec![TerminalRow::warn("No scenario runtime is loaded.")]);
    };
    let mut pairs: Vec<(String, String)> = event_world
        .variables()
        .map(|(key, value)| (key.clone(), literal(value)))
        .collect();
    pairs.sort();
    if pairs.is_empty() {
        return CommandResult::ok("variables", CLASS, "no variables")
            .with_rows(vec![TerminalRow::warn("This scenario holds no variables.")]);
    }
    let width = pairs.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    let rows = pairs
        .iter()
        .map(|(key, value)| TerminalRow::output(format!("  {key:width$}  {value}")))
        .collect();
    CommandResult::ok("variables", CLASS, format!("{} live", pairs.len())).with_rows(rows)
}

/// `variable <name>`: one variable's value.
pub fn variable(world: &mut World, name: &str) -> CommandResult {
    let Some(event_world) = world.get_resource::<NovaEventWorld>() else {
        return CommandResult::error("variable", Some(CLASS), "no scenario runtime is loaded");
    };
    match event_world.get_variable(name) {
        Some(value) => {
            let value = literal(value);
            CommandResult::ok("variable", CLASS, format!("{name} = {value}"))
                .with_rows(vec![TerminalRow::output(format!("{name} = {value}"))])
        }
        None => CommandResult::error(
            "variable",
            Some(CLASS),
            format!("no variable named '{name}'"),
        ),
    }
}

/// `bindings` / `bindings <action>`.
pub fn bindings(world: &mut World, action: Option<&str>) -> CommandResult {
    let Some(table) = world.get_resource::<InputBindings>() else {
        return CommandResult::error("bindings", Some(CLASS), "no input registry is loaded");
    };
    let live = world.get_resource::<ActiveContexts>();
    let Some(name) = action else {
        let width = table
            .iter()
            .map(|action| action.name.len())
            .max()
            .unwrap_or(0);
        let rows: Vec<TerminalRow> = table
            .iter()
            .map(|action| {
                let mark = live.map_or(' ', |live| {
                    if live.is_live(action.context) {
                        '*'
                    } else {
                        ' '
                    }
                });
                TerminalRow::output(format!(
                    "{mark} {:width$}  {}",
                    action.name,
                    action.keyboard_display()
                ))
            })
            .collect();
        let count = rows.len();
        return CommandResult::ok("bindings", CLASS, format!("{count} actions"))
            .with_rows(rows)
            .and_rows([TerminalRow::dim("* can fire right now.")]);
    };
    let Some(action) = table.get(name) else {
        return CommandResult::error("bindings", Some(CLASS), format!("no action named '{name}'"));
    };
    let context = live.map_or("unknown", |live| {
        if live.is_live(action.context) {
            "live"
        } else {
            "not live"
        }
    });
    CommandResult::ok(
        "bindings",
        CLASS,
        format!("{name}: {}", action.keyboard_display()),
    )
    .with_rows(vec![
        TerminalRow::info(format!("{} - {}", action.name, action.label)),
        TerminalRow::output(format!("GROUP ........ {}", action.group)),
        TerminalRow::output(format!("KEYBOARD ..... {}", action.keyboard_display())),
        TerminalRow::output(format!("GAMEPAD ...... {}", action.gamepad_display())),
        TerminalRow::output(format!("CONTEXT ...... {:?} ({context})", action.context)),
    ])
}

/// `cheats status`: arming and the run mark.
pub fn cheats_status(world: &mut World) -> CommandResult {
    let cheats = world
        .get_resource::<RunCheats>()
        .copied()
        .unwrap_or_default();
    CommandResult::ok("cheats status", CLASS, cheats.banner()).with_rows(vec![
        TerminalRow::output(format!(
            "ARMED ........ {}",
            if cheats.is_armed() { "yes" } else { "no" }
        )),
        TerminalRow::new(
            if cheats.is_marked() {
                TerminalRowKind::Warn
            } else {
                TerminalRowKind::Output
            },
            format!(
                "RUN .......... {}",
                if cheats.is_marked() {
                    "marked"
                } else {
                    "clean"
                }
            ),
        ),
    ])
}

/// One ship's listing line: side, hull and section count.
fn ship_summary(world: &World, entity: Entity) -> String {
    let entity_ref = world.entity(entity);
    let health = entity_ref.get::<Health>().cloned();
    format!(
        "{:<7} {}",
        allegiance_label(world, entity),
        health_line(health.as_ref())
    )
}

/// One section's listing line: kind, integrity and magazine.
fn section_summary(world: &World, entity: Entity) -> String {
    let entity_ref = world.entity(entity);
    let health = entity_ref.get::<Health>().cloned();
    let ammo = entity_ref.get::<SectionAmmo>().copied();
    let unlimited = entity_ref.get::<SuspendedSectionAmmo>().is_some();
    format!(
        "{:<10} {:<12} {}",
        section_kind_label(world, entity),
        health_line(health.as_ref()),
        magazine_line(ammo, unlimited)
    )
}

fn allegiance_label(world: &World, entity: Entity) -> String {
    match world.entity(entity).get::<Allegiance>() {
        Some(Allegiance::Player) => "player".to_string(),
        Some(Allegiance::Enemy) => "enemy".to_string(),
        Some(Allegiance::Neutral) => "neutral".to_string(),
        None => "unaligned".to_string(),
    }
}

fn section_kind_label(world: &World, entity: Entity) -> String {
    match world.entity(entity).get::<SectionClass>() {
        Some(class) => format!("{class:?}").to_lowercase(),
        None => "section".to_string(),
    }
}

fn health_line(health: Option<&Health>) -> String {
    match health {
        Some(health) => format!("{:.0}/{:.0}", health.current, health.max),
        None => "-".to_string(),
    }
}

fn magazine_line(ammo: Option<SectionAmmo>, unlimited: bool) -> String {
    match (ammo, unlimited) {
        (Some(ammo), _) => format!("{}/{}", ammo.rounds, ammo.capacity),
        // The cheat took the magazine off and kept what it was, so the shell
        // can say WHY a weapon has none rather than reporting it as unarmed.
        (None, true) => "unlimited (cheat)".to_string(),
        (None, false) => "unlimited".to_string(),
    }
}

fn reload_line(reload: Option<SectionReload>) -> String {
    match reload {
        Some(reload) => format!(
            "{:.1}s / {} rounds ({:.1}s elapsed)",
            reload.delay, reload.amount, reload.elapsed
        ),
        None => "none".to_string(),
    }
}

fn speed_cap_line(cap: Option<f32>) -> String {
    // Engine boundary: `FlightSpeedCap` holds world units because it is
    // compared against an avian velocity; a figure a player reads is metres.
    match cap {
        Some(cap) => format!("{:.0} m/s", MetersPerSecond::from_engine(cap).get()),
        None => "none".to_string(),
    }
}

/// How full a ship's magazines are, as one line.
fn ammo_line(world: &mut World, ship: Entity) -> String {
    let sections = lookup::sections(world, ship);
    let mut finite = 0usize;
    let mut unlimited = 0usize;
    for (entity, _) in &sections {
        let entity_ref = world.entity(*entity);
        if entity_ref.get::<SectionAmmo>().is_some() {
            finite += 1;
        } else if entity_ref.get::<SuspendedSectionAmmo>().is_some() {
            unlimited += 1;
        }
    }
    match (finite, unlimited) {
        (0, 0) => "no magazines".to_string(),
        (finite, 0) => format!("{finite} finite"),
        (0, unlimited) => format!("{unlimited} unlimited (cheat)"),
        (finite, unlimited) => format!("{finite} finite, {unlimited} unlimited (cheat)"),
    }
}

/// A variable's value as the shell prints it.
fn literal(value: &VariableLiteral) -> String {
    match value {
        VariableLiteral::Number(number) => format!("{number}"),
        VariableLiteral::Boolean(flag) => format!("{flag}"),
        VariableLiteral::String(text) => text.clone(),
    }
}
