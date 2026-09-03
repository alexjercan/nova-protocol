//! Apply parsed lanes to the world, in the two schedule slots the design
//! record pinned - and collect the `applied` acks the snapshot echoes back.
//!
//! The lanes do not share a slot because their readers do not:
//!
//! - [`channel_pointer_writer`] runs in `First`, after the frame's messages
//!   swap and BEFORE `bevy_picking` consumes `WindowEvent` - the slot the
//!   autopilot's cursor pin already occupies. Writing later costs a frame of
//!   lag per gesture.
//! - [`channel_input_writer`] runs in `PreUpdate`, after bevy's `InputSystems`
//!   has cleared the `just_*` edges and replaced the axis accumulators, and
//!   before `bevy_enhanced_input` prepares - so a synthesized press is still
//!   an edge when `Update` reads it. The keyboard messages for the text/key
//!   lanes are written here too: their readers are `Update` systems on the
//!   message stream, same frame.
//!
//! Every synthesized keyboard event gets its Released twin, and a key-lane tap
//! releases on the NEXT frame - a held key never re-arms `just_pressed`, which
//! is the trap the spike round found and this module exists to not repeat.

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput, NativeKeyCode},
        ButtonState,
    },
    prelude::*,
    window::PrimaryWindow,
};
use nova_autopilot::prelude::{
    hover_named, move_cursor, press_mouse, release_mouse, scroll_lines, ui_node_rect,
};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::{PlayerSpaceshipMarker, SectionMarker, SpaceshipRootMarker};
use nova_input::prelude::{
    dispatch, ActionName, ActiveContexts, DispatchError, InputBindings, InputPhase, InputSource,
};
use nova_os::prelude::{
    command_shell_specs, resolve_command_line, CommandChannel, CommandClass, CommandOutcome,
    CommandResult, CommandSource,
};
use nova_ship::prelude::{
    SpaceshipRailgunInputBinding, SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding,
    SpaceshipTurretInputBinding,
};

use crate::protocol::{Lane, PointerCmd, PointerTarget};

/// The lanes staged for the frame the runner is about to step, split by the
/// slot that applies them. The runner fills it before `app.update()`; the two
/// writer systems drain their half.
#[derive(Resource, Default)]
pub struct ChannelFrame {
    /// Pointer gestures, applied in `First`.
    pub pointer: Vec<(usize, Lane)>,
    /// Named inputs, aim deltas, text and editing keys, applied in `PreUpdate`.
    pub input: Vec<(usize, Lane)>,
    /// The release half of last frame's key-lane taps: a tap is press on one
    /// frame and release on the next, never both edges in one.
    pub key_releases: Vec<(KeyCode, Key)>,
    /// Free-running only: the stdin line numbers whose named tick had already
    /// passed when they arrived. Their acks say `late`; step mode refuses a
    /// past tick outright, so the set stays empty there.
    pub late_lines: std::collections::HashSet<usize>,
}

/// What the frame's lines did - drained by the runner after the frame, echoed
/// in the snapshot's `applied` block (entries) and as error lines (errors).
#[derive(Resource, Default)]
pub struct ChannelAck {
    /// One entry per consumed line, in consumption order.
    pub applied: Vec<AppliedEntry>,
    /// Refused lines, echoed on stdout with their line number.
    pub errors: Vec<(usize, String)>,
}

/// One line's ack.
#[derive(Debug, Clone)]
pub struct AppliedEntry {
    /// The stdin line this acknowledges.
    pub line: usize,
    /// The wire name of what was driven.
    pub input: String,
    /// The gesture half: `start` / `stop` / `delta` / `type` / `tap` /
    /// `move` / `press` / `release` / `wheel`.
    pub phase: String,
    /// The outcome, or the named action whose `TriggerState` answers it.
    pub state: AckState,
    /// Free-running only: the line named a tick that had already passed, so it
    /// was applied on the next frame instead.
    pub late: bool,
}

/// How an ack's `state` field resolves.
#[derive(Debug, Clone)]
pub enum AckState {
    /// Known at apply time: `Fired` for a landed raw gesture, `refused` for a
    /// name whose context is not live.
    Done(String),
    /// A named action: the runner reads its [`TriggerState`] AFTER the frame
    /// evaluated, which is how a driver observes "the press did nothing".
    ///
    /// [`TriggerState`]: bevy_enhanced_input::prelude::TriggerState
    Action(String),
}

/// The pointer lane, in the picking backend's own slot.
pub fn channel_pointer_writer(world: &mut World) {
    let staged = std::mem::take(&mut world.resource_mut::<ChannelFrame>().pointer);
    for (line, lane) in staged {
        let Lane::Pointer(cmd) = lane else {
            continue;
        };
        apply_pointer(world, line, &cmd);
    }
}

/// The input, aim, text and key lanes, in the autopilot's slot.
pub fn channel_input_writer(world: &mut World) {
    let releases = std::mem::take(&mut world.resource_mut::<ChannelFrame>().key_releases);
    for (code, logical) in releases {
        world.resource_mut::<ButtonInput<KeyCode>>().release(code);
        write_keyboard(world, code, logical, ButtonState::Released, None);
    }
    let staged = std::mem::take(&mut world.resource_mut::<ChannelFrame>().input);
    for (line, lane) in staged {
        match lane {
            Lane::Input { wire, phase } => apply_input(world, line, &wire, phase),
            Lane::Aim { wire, delta } => apply_aim(world, line, &wire, delta),
            Lane::Text(text) => apply_text(world, line, &text),
            Lane::Key(key) => apply_key(world, line, &key),
            Lane::Command(text) => apply_command(world, line, &text),
            Lane::Pointer(_) => {}
        }
    }
}

fn ack(world: &mut World, mut entry: AppliedEntry) {
    entry.late = world
        .resource::<ChannelFrame>()
        .late_lines
        .contains(&entry.line);
    world.resource_mut::<ChannelAck>().applied.push(entry);
}

fn refuse(world: &mut World, line: usize, message: String) {
    world
        .resource_mut::<ChannelAck>()
        .errors
        .push((line, message));
}

/// The wire name an action answers to: its settings group, lowercased with
/// spaces as underscores, then its registry name - `flight.main_drive`,
/// `nova_os.novaos_orbit_left`.
pub fn wire_name(group: &str, name: &str) -> String {
    format!("{}.{name}", group.to_lowercase().replace(' ', "_"))
}

// -- command ------------------------------------------------------------------

/// One command line, through the SAME parser the CRT prompt uses.
///
/// This runs in `PreUpdate`, so the invocation is waiting when the dispatcher's
/// `Update` set drains it, and the answer is back before the runner collects
/// the frame's acks. A line the catalog can answer on its own - `help`, a typo,
/// a bad argument - never reaches the world and is acknowledged here.
///
/// The stdin line number is the sequence: an ack names the line that asked.
fn apply_command(world: &mut World, line: usize, text: &str) {
    let source = CommandSource { seq: line as u64 };
    let outcome = resolve_command_line(text, command_shell_specs());
    let Some(mut channel) = world.get_resource_mut::<CommandChannel>() else {
        // An app assembled without the dispatcher (a bare harness, not the
        // game) has no command shell at all. Say so, rather than dropping the
        // line silently.
        return refuse(world, line, "this app has no command shell".to_string());
    };
    match outcome {
        CommandOutcome::Answer(result) => channel.answer(source, *result),
        // `clear` and `close` control a screen. The channel has none, so it
        // says so instead of acknowledging `ok` for a command that did nothing.
        CommandOutcome::Invoke(invocation) if matches!(invocation.name, "clear" | "close") => {
            let name = invocation.name;
            channel.answer(
                source,
                CommandResult::refused(
                    name,
                    invocation.class,
                    format!("{name}: the channel has no screen to {name}"),
                ),
            );
        }
        CommandOutcome::Invoke(invocation) => channel.submit(source, invocation),
    }
}

/// One command's acknowledgement: what was asked, what it was allowed to touch,
/// how it ended, the one-line answer, and the lines the CRT would have printed.
///
/// `rows` is what makes the two front ends equal. `detail` alone answers a
/// driver that only wants to know it worked; a driver that wanted to READ
/// something - the ship list, the bindings - needs the same text a player sees.
///
/// Deliberately NOT the internal scenario action a cheat may have run: a driver
/// contracts against the public command vocabulary, not the enum behind it.
fn command_ack(line: u64, result: &CommandResult) -> serde_json::Value {
    serde_json::json!({
        "line": line,
        "command": result.command,
        "class": result.class.map(CommandClass::label),
        "state": result.status.label(),
        "detail": result.detail,
        "rows": result.rows.iter().map(|row| row.text.clone()).collect::<Vec<_>>(),
    })
}

// -- input --------------------------------------------------------------------

fn apply_input(world: &mut World, line: usize, wire: &str, phase: InputPhase) {
    let phase_word = match phase {
        InputPhase::Press => "start",
        InputPhase::Release => "stop",
    };
    if let Some(section) = wire.strip_prefix("section.") {
        return apply_section(world, line, section, phase, phase_word);
    }

    let name = wire.split_once('.').map_or(wire, |(_, name)| name);
    let known = world
        .resource::<InputBindings>()
        .get(name)
        .filter(|action| wire_name(action.group, action.name) == wire)
        .map(|action| action.context);
    let Some(context) = known else {
        return refuse(world, line, format!("no action named `{wire}`"));
    };
    if !world.resource::<ActiveContexts>().is_live(context) {
        return ack(
            world,
            entry(line, wire, phase_word, AckState::Done("refused".into())),
        );
    }
    match dispatch::apply(world, name, phase) {
        Ok(()) => ack(
            world,
            entry(line, wire, phase_word, AckState::Action(name.to_string())),
        ),
        Err(DispatchError::NoButton(_)) => {
            refuse(
                world,
                line,
                format!("`{wire}` has no button; it is an axis"),
            );
        }
        Err(error) => refuse(world, line, error.to_string()),
    }
}

fn apply_section(world: &mut World, line: usize, id: &str, phase: InputPhase, phase_word: &str) {
    let wire = format!("section.{id}");
    let Some(source) = section_source(world, id) else {
        return refuse(world, line, format!("no section `{id}` on the ship"));
    };
    if !world
        .resource::<ActiveContexts>()
        .is_live(nova_input::prelude::ActionContext::Flight)
    {
        return ack(
            world,
            entry(line, &wire, phase_word, AckState::Done("refused".into())),
        );
    }
    dispatch::press_source(world, source, phase);
    ack(
        world,
        entry(line, &wire, phase_word, AckState::Done("Fired".into())),
    );
}

/// The first bound source of the player-ship section whose authored id is
/// `id`. Sections have no `Name` requirement and entity ids churn, so the
/// resolve is by scenario [`EntityId`] (falling back to `Name`), re-run per
/// line the way every named address on this wire is.
fn section_source(world: &mut World, id: &str) -> Option<InputSource> {
    let players: Vec<Entity> = world
        .query_filtered::<Entity, (With<PlayerSpaceshipMarker>, With<SpaceshipRootMarker>)>()
        .iter(world)
        .collect();
    let sections: Vec<Entity> = world
        .query_filtered::<Entity, With<SectionMarker>>()
        .iter(world)
        .collect();
    let section = sections.into_iter().find(|section| {
        let labelled = world
            .get::<EntityId>(*section)
            .map(|entity_id| entity_id.0 == id)
            .or_else(|| world.get::<Name>(*section).map(|name| name.as_str() == id))
            .unwrap_or(false);
        labelled
            && std::iter::successors(Some(*section), |entity| {
                world.get::<ChildOf>(*entity).map(ChildOf::parent)
            })
            .any(|ancestor| players.contains(&ancestor))
    })?;
    let thruster = world
        .get::<SpaceshipThrusterInputBinding>(section)
        .and_then(|binding| binding.0.first().copied());
    let turret = world
        .get::<SpaceshipTurretInputBinding>(section)
        .and_then(|binding| binding.0.first().copied());
    let torpedo = world
        .get::<SpaceshipTorpedoInputBinding>(section)
        .and_then(|binding| binding.0.first().copied());
    let railgun = world
        .get::<SpaceshipRailgunInputBinding>(section)
        .and_then(|binding| binding.0.first().copied());
    thruster.or(turret).or(torpedo).or(railgun)
}

// -- aim ----------------------------------------------------------------------

fn apply_aim(world: &mut World, line: usize, wire: &str, delta: Vec2) {
    let name = wire.split_once('.').map_or(wire, |(_, name)| name);
    let known = world
        .resource::<InputBindings>()
        .get(name)
        .filter(|action| wire_name(action.group, action.name) == wire)
        .map(|action| action.context);
    let Some(context) = known else {
        return refuse(world, line, format!("no action named `{wire}`"));
    };
    if !world.resource::<ActiveContexts>().is_live(context) {
        return ack(
            world,
            entry(line, wire, "delta", AckState::Done("refused".into())),
        );
    }
    match dispatch::apply_axis(world, name, delta) {
        Ok(()) => ack(
            world,
            entry(line, wire, "delta", AckState::Done("Fired".into())),
        ),
        Err(DispatchError::NoAxis(_)) => {
            refuse(world, line, format!("`{wire}` is not driven by an axis"));
        }
        Err(error) => refuse(world, line, error.to_string()),
    }
}

// -- text ---------------------------------------------------------------------

/// One press [`KeyboardInput`] per character, each with its Released twin, so
/// a poller of `ButtonInput<Key>` never sees a phantom held character. The
/// characters go to whatever has focus - a `Ctrl`-holding driver loses them at
/// the prompt exactly as a player would, and the channel does not pretend to
/// know.
fn apply_text(world: &mut World, line: usize, text: &str) {
    for character in text.chars() {
        let character = character.to_string();
        let logical = Key::Character(character.as_str().into());
        write_keyboard(
            world,
            KeyCode::Unidentified(NativeKeyCode::Unidentified),
            logical.clone(),
            ButtonState::Pressed,
            Some(character.as_str()),
        );
        write_keyboard(
            world,
            KeyCode::Unidentified(NativeKeyCode::Unidentified),
            logical,
            ButtonState::Released,
            None,
        );
    }
    ack(
        world,
        entry(line, "text", "type", AckState::Done("Fired".into())),
    );
}

// -- key ----------------------------------------------------------------------

/// The editing keys the tree actually reads - the prompt's arms plus what
/// `TextField` handles. The lane passes both halves through (the message the
/// prompt and the fields read, the `ButtonInput` edge the mode chords and the
/// rebind capture poll) and promises nothing the readers do not.
fn editing_key(key: &str) -> Option<(KeyCode, Key, Option<&'static str>)> {
    Some(match key {
        "Enter" => (KeyCode::Enter, Key::Enter, None),
        "Tab" => (KeyCode::Tab, Key::Tab, None),
        "Backspace" => (KeyCode::Backspace, Key::Backspace, None),
        "Delete" => (KeyCode::Delete, Key::Delete, None),
        "Escape" => (KeyCode::Escape, Key::Escape, None),
        "Space" => (KeyCode::Space, Key::Space, Some(" ")),
        "ArrowLeft" => (KeyCode::ArrowLeft, Key::ArrowLeft, None),
        "ArrowRight" => (KeyCode::ArrowRight, Key::ArrowRight, None),
        "ArrowUp" => (KeyCode::ArrowUp, Key::ArrowUp, None),
        "ArrowDown" => (KeyCode::ArrowDown, Key::ArrowDown, None),
        "PageUp" => (KeyCode::PageUp, Key::PageUp, None),
        "PageDown" => (KeyCode::PageDown, Key::PageDown, None),
        _ => return None,
    })
}

fn apply_key(world: &mut World, line: usize, key: &str) {
    let Some((code, logical, text)) = editing_key(key) else {
        return refuse(world, line, format!("`{key}` is not an editing key"));
    };
    world.resource_mut::<ButtonInput<KeyCode>>().press(code);
    write_keyboard(world, code, logical.clone(), ButtonState::Pressed, text);
    world
        .resource_mut::<ChannelFrame>()
        .key_releases
        .push((code, logical));
    ack(
        world,
        entry(
            line,
            &format!("key.{key}"),
            "tap",
            AckState::Done("Fired".into()),
        ),
    );
}

fn write_keyboard(
    world: &mut World,
    key_code: KeyCode,
    logical_key: Key,
    state: ButtonState,
    text: Option<&str>,
) {
    let Ok(window) = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
    else {
        warn!("nova channel: a keyboard event has no primary window");
        return;
    };
    world.write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: text.map(Into::into),
        repeat: false,
        window,
    });
}

// -- pointer ------------------------------------------------------------------

fn apply_pointer(world: &mut World, line: usize, cmd: &PointerCmd) {
    let done = |phase: &str| entry(line, "pointer", phase, AckState::Done("Fired".into()));
    match cmd {
        PointerCmd::To(PointerTarget::Name(name)) => {
            if ui_node_rect(world, name).is_none() {
                return refuse(world, line, format!("no visible target named `{name}`"));
            }
            hover_named(name.clone())(world);
            ack(world, done("move"));
        }
        PointerCmd::To(PointerTarget::Px(position)) => {
            move_cursor(*position)(world);
            ack(world, done("move"));
        }
        PointerCmd::Press(button) => {
            press_mouse(*button)(world);
            ack(world, done("press"));
        }
        PointerCmd::Release(button) => {
            release_mouse(*button)(world);
            ack(world, done("release"));
        }
        PointerCmd::Wheel(lines) => {
            scroll_lines(*lines)(world);
            ack(world, done("wheel"));
        }
    }
}

// -- ack assembly -------------------------------------------------------------

fn entry(line: usize, input: &str, phase: &str, state: AckState) -> AppliedEntry {
    AppliedEntry {
        line,
        input: input.to_string(),
        phase: phase.to_string(),
        state,
        late: false,
    }
}

/// Resolve the frame's acks into `applied` JSON entries, reading each named
/// action's [`TriggerState`] off the rig NOW - after the frame evaluated.
///
/// [`TriggerState`]: bevy_enhanced_input::prelude::TriggerState
pub fn drain_acks(world: &mut World) -> (Vec<serde_json::Value>, Vec<(usize, String)>) {
    let ChannelAck { applied, errors } = std::mem::take(&mut *world.resource_mut::<ChannelAck>());
    let mut applied: Vec<serde_json::Value> = applied
        .into_iter()
        .map(|entry| {
            let state = match entry.state {
                AckState::Done(state) => state,
                AckState::Action(name) => action_state(world, &name),
            };
            let mut record = serde_json::json!({
                "line": entry.line,
                "input": entry.input,
                "phase": entry.phase,
                "state": state,
            });
            if entry.late {
                record["late"] = serde_json::Value::Bool(true);
            }
            record
        })
        .collect();
    // The command lane answers through the dispatcher's own queue rather than
    // `ChannelAck`, because the run happens a schedule later than the apply.
    if let Some(mut channel) = world.get_resource_mut::<CommandChannel>() {
        applied.extend(
            channel
                .drain_answers()
                .into_iter()
                .map(|(source, result)| command_ack(source.seq, &result)),
        );
    }
    (applied, errors)
}

/// The strongest `TriggerState` any rig entity holding this registry name
/// reports: `Fired` beats `Ongoing` beats `None`. No rig entity (the action is
/// registered but nothing spawned it - no ship on the field) reads as `None`,
/// the same answer a dead press gives.
fn action_state(world: &mut World, name: &str) -> String {
    use bevy_enhanced_input::prelude::TriggerState;
    let strongest = world
        .query::<(&TriggerState, &ActionName)>()
        .iter(world)
        .filter(|(_, action)| action.0 == name)
        .map(|(state, _)| *state)
        .max_by_key(|state| match state {
            TriggerState::None => 0,
            TriggerState::Ongoing => 1,
            TriggerState::Fired => 2,
        })
        .unwrap_or(TriggerState::None);
    format!("{strongest:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ack_world() -> World {
        let mut world = World::new();
        world.init_resource::<ChannelFrame>();
        world.init_resource::<ChannelAck>();
        world.init_resource::<CommandChannel>();
        world
    }

    /// A real command never runs here: it is queued for the dispatcher, which
    /// is a whole schedule away. What this pins is that the wire's answer comes
    /// back on the line that asked, in the public vocabulary.
    #[test]
    fn a_command_line_is_queued_for_the_dispatcher_under_its_own_line_number() {
        let mut world = ack_world();
        apply_command(&mut world, 12, "graphics low");
        let queued = world.resource_mut::<CommandChannel>().drain_pending();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].0, CommandSource { seq: 12 });
        assert_eq!(queued[0].1.name, "graphics");
        assert_eq!(queued[0].1.args, vec!["low".to_string()]);
    }

    /// A line the catalog can answer on its own must not reach the world, and
    /// must still ack - a driver that typo'd gets told, on its own line.
    #[test]
    fn a_command_the_catalog_answers_alone_acks_without_touching_the_world() {
        let mut world = ack_world();
        apply_command(&mut world, 3, "graphix");
        assert!(!world.resource::<CommandChannel>().has_pending());

        let (applied, errors) = drain_acks(&mut world);
        assert!(errors.is_empty());
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0]["line"], 3);
        assert_eq!(applied[0]["state"], "error");
        assert_eq!(applied[0]["command"], "graphix");
    }

    /// The ack names the command and its class, never the scenario action a
    /// cheat runs behind it.
    #[test]
    fn a_command_ack_carries_the_command_its_class_and_its_result() {
        let ack = command_ack(
            9,
            &CommandResult::ok("graphics", CommandClass::Setting, "graphics: low"),
        );
        assert_eq!(ack["line"], 9);
        assert_eq!(ack["command"], "graphics");
        assert_eq!(ack["class"], "setting");
        assert_eq!(ack["state"], "ok");
        assert_eq!(ack["detail"], "graphics: low");
        assert_eq!(ack["rows"], serde_json::json!([]));
    }

    #[test]
    fn a_line_staged_late_acks_late() {
        let mut world = ack_world();
        world.resource_mut::<ChannelFrame>().late_lines.insert(7);
        ack(
            &mut world,
            entry(
                7,
                "flight.main_drive",
                "start",
                AckState::Done("Fired".into()),
            ),
        );
        ack(
            &mut world,
            entry(
                8,
                "flight.main_drive",
                "stop",
                AckState::Done("None".into()),
            ),
        );
        let acks = &world.resource::<ChannelAck>().applied;
        assert!(acks[0].late, "line 7 was staged late");
        assert!(!acks[1].late, "line 8 was on time");
    }

    #[test]
    fn only_a_late_ack_serializes_the_flag() {
        let mut world = ack_world();
        world.resource_mut::<ChannelFrame>().late_lines.insert(7);
        ack(
            &mut world,
            entry(
                7,
                "flight.main_drive",
                "start",
                AckState::Done("Fired".into()),
            ),
        );
        ack(
            &mut world,
            entry(
                8,
                "flight.main_drive",
                "stop",
                AckState::Done("None".into()),
            ),
        );
        let (applied, errors) = drain_acks(&mut world);
        assert!(errors.is_empty());
        assert_eq!(applied[0]["late"], serde_json::Value::Bool(true));
        assert!(
            applied[1].get("late").is_none(),
            "on time: no flag on the wire"
        );
    }
}
