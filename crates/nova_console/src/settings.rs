//! The Setting commands: the same persisted player settings the settings
//! screen writes.
//!
//! Every one follows one grammar: the bare command READS the setting, the
//! command with a value CHANGES it. Nothing here marks the run, and nothing
//! here writes the store directly - `nova_menu` persists on change, so a value
//! written into the live resource is saved and shown by the settings UI on its
//! own.

use bevy::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;
use nova_menu::prelude::WindowModeSetting;
use nova_os::prelude::*;

const CLASS: CommandClass = CommandClass::Setting;

/// `graphics [low|medium|high]`.
pub fn graphics(world: &mut World, value: Option<&str>) -> CommandResult {
    let Some(current) = world.get_resource::<GraphicsQuality>().copied() else {
        return CommandResult::error("graphics", Some(CLASS), "no graphics setting is loaded");
    };
    let Some(word) = value else {
        return read("graphics", format!("quality is {}", current.label()));
    };
    let Some(wanted) = GraphicsQuality::ALL
        .into_iter()
        .find(|quality| quality.label().eq_ignore_ascii_case(word))
    else {
        return CommandResult::error(
            "graphics",
            Some(CLASS),
            format!(
                "graphics: no preset named '{word}' ({})",
                GraphicsQuality::ALL
                    .iter()
                    .map(|quality| quality.label().to_lowercase())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    };
    *world.resource_mut::<GraphicsQuality>() = wanted;
    changed("graphics", format!("quality is {}", wanted.label()))
}

/// `window [windowed|borderless]`.
pub fn window(world: &mut World, value: Option<&str>) -> CommandResult {
    let Some(current) = world.get_resource::<WindowModeSetting>().copied() else {
        return CommandResult::error("window", Some(CLASS), "no window setting is loaded");
    };
    let Some(word) = value else {
        return read("window", format!("mode is {}", window_label(current)));
    };
    let wanted = match word.to_ascii_lowercase().as_str() {
        "windowed" => WindowModeSetting::Windowed,
        "borderless" => WindowModeSetting::Borderless,
        _ => {
            return CommandResult::error(
                "window",
                Some(CLASS),
                format!("window: no mode named '{word}' (windowed, borderless)"),
            )
        }
    };
    *world.resource_mut::<WindowModeSetting>() = wanted;
    changed("window", format!("mode is {}", window_label(wanted)))
}

/// The mixer channels `volume` addresses, in the order the command prints them.
const CHANNELS: [&str; 4] = ["master", "music", "world", "interface"];

/// `volume` / `volume <channel>` / `volume <channel> <0..1>`.
pub fn volume(world: &mut World, args: &[String]) -> CommandResult {
    let Some(channel) = args.first() else {
        let rows = CHANNELS
            .iter()
            .map(|channel| {
                TerminalRow::output(format!(
                    "  {channel:<9}  {:.2}",
                    read_channel(world, channel).unwrap_or_default()
                ))
            })
            .collect();
        return CommandResult::ok("volume", CLASS, "listed 4 channels").with_rows(rows);
    };
    let channel = channel.to_ascii_lowercase();
    let Some(current) = read_channel(world, &channel) else {
        return CommandResult::error(
            "volume",
            Some(CLASS),
            format!(
                "volume: no channel named '{channel}' ({})",
                CHANNELS.join(", ")
            ),
        );
    };
    let Some(word) = args.get(1) else {
        return read("volume", format!("{channel} is {current:.2}"));
    };
    let Ok(wanted) = word.parse::<f32>() else {
        return CommandResult::error(
            "volume",
            Some(CLASS),
            format!("volume: '{word}' is not a number between 0 and 1"),
        );
    };
    if !(0.0..=1.0).contains(&wanted) {
        return CommandResult::error(
            "volume",
            Some(CLASS),
            format!("volume: {wanted} is outside 0..1"),
        );
    }
    write_channel(world, &channel, wanted);
    changed("volume", format!("{channel} is {wanted:.2}"))
}

/// `bind <action> <source>`.
pub fn bind(world: &mut World, action: &str, source: &str) -> CommandResult {
    let Some(wanted) = InputSource::parse(source) else {
        return CommandResult::error(
            "bind",
            Some(CLASS),
            format!("bind: '{source}' is not an input source"),
        );
    };
    let Some(mut table) = world.get_resource_mut::<InputBindings>() else {
        return CommandResult::error("bind", Some(CLASS), "no input registry is loaded");
    };
    let Some(current) = table.get(action).map(ActionBinding::spec) else {
        return CommandResult::error(
            "bind",
            Some(CLASS),
            format!("bind: no action named '{action}'"),
        );
    };
    // A source belongs to the column of its own device; the registry refuses
    // the other arrangement, so the command sorts it here rather than letting
    // a correct request be rejected as malformed.
    let spec = match wanted {
        InputSource::Gamepad(_) => BindingSpec {
            keyboard: current.keyboard,
            gamepad: vec![wanted],
        },
        _ => BindingSpec {
            keyboard: vec![wanted],
            gamepad: current.gamepad,
        },
    };
    if !table.rebind(action, spec) {
        return CommandResult::refused(
            "bind",
            CLASS,
            format!(
                "bind: '{action}' will not take {}; see `bindings {action}`",
                wanted.readout_label()
            ),
        );
    }
    let display = binding_line(world, action);
    changed("bind", format!("{action} is {display}"))
}

/// `bind reset <action>`.
pub fn bind_reset(world: &mut World, action: &str) -> CommandResult {
    let Some(mut table) = world.get_resource_mut::<InputBindings>() else {
        return CommandResult::error("bind reset", Some(CLASS), "no input registry is loaded");
    };
    if table.get(action).is_none() {
        return CommandResult::error(
            "bind reset",
            Some(CLASS),
            format!("bind reset: no action named '{action}'"),
        );
    }
    if !table.reset(action) {
        return CommandResult::refused(
            "bind reset",
            CLASS,
            format!("bind reset: '{action}' would not take its own default"),
        );
    }
    let display = binding_line(world, action);
    changed("bind reset", format!("{action} is {display}"))
}

/// `settings`: every setting a command can read, in one block.
pub fn settings(world: &mut World) -> CommandResult {
    let quality = world
        .get_resource::<GraphicsQuality>()
        .copied()
        .unwrap_or_default();
    let mode = world
        .get_resource::<WindowModeSetting>()
        .copied()
        .unwrap_or_default();
    let moved = world
        .get_resource::<InputBindings>()
        .map_or(0, |table| table.overrides().len());
    let mut rows = vec![
        TerminalRow::info(format!("GRAPHICS ..... {}", quality.label())),
        TerminalRow::output(format!("WINDOW ....... {}", window_label(mode))),
    ];
    rows.extend(CHANNELS.iter().map(|channel| {
        TerminalRow::output(format!(
            "{:<12} {:.2}",
            format!("{}.", channel.to_uppercase()),
            read_channel(world, channel).unwrap_or_default()
        ))
    }));
    rows.push(TerminalRow::output(format!("KEYBINDS ..... {moved} moved")));
    CommandResult::ok("settings", CommandClass::ReadOnly, "listed the settings").with_rows(rows)
}

/// Reading a setting: the answer is the value, printed once.
fn read(name: &'static str, detail: String) -> CommandResult {
    CommandResult::ok(name, CLASS, detail.clone()).with_rows(vec![TerminalRow::output(detail)])
}

/// Changing one: the same line, marked so the shell shows it landed.
fn changed(name: &'static str, detail: String) -> CommandResult {
    CommandResult::ok(name, CLASS, detail.clone()).with_rows(vec![TerminalRow::info(detail)])
}

fn window_label(mode: WindowModeSetting) -> &'static str {
    match mode {
        WindowModeSetting::Windowed => "windowed",
        WindowModeSetting::Borderless => "borderless",
    }
}

/// One mixer channel's current factor, or `None` when the name is not a
/// channel.
fn read_channel(world: &World, channel: &str) -> Option<f32> {
    match channel {
        "master" => world.get_resource::<MasterVolume>().map(|v| v.factor()),
        "music" => world.get_resource::<MusicVolume>().map(|v| v.factor()),
        "world" => world.get_resource::<WorldVolume>().map(|v| v.factor()),
        "interface" => world.get_resource::<InterfaceVolume>().map(|v| v.factor()),
        _ => None,
    }
}

fn write_channel(world: &mut World, channel: &str, value: f32) {
    match channel {
        "master" => world.resource_mut::<MasterVolume>().0 = value,
        "music" => world.resource_mut::<MusicVolume>().0 = value,
        "world" => world.resource_mut::<WorldVolume>().0 = value,
        "interface" => world.resource_mut::<InterfaceVolume>().0 = value,
        _ => {}
    }
}

/// What one action reads as now, both columns.
fn binding_line(world: &World, action: &str) -> String {
    world
        .get_resource::<InputBindings>()
        .and_then(|table| table.get(action))
        .map(|action| {
            format!(
                "{} / {}",
                action.keyboard_display(),
                action.gamepad_display()
            )
        })
        .unwrap_or_else(|| "Unbound".to_string())
}
