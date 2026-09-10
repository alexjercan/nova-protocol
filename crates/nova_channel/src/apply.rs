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
    dispatch, ActiveContexts, DispatchError, InputBindings, InputPhase, InputSource,
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

/// One line's ack: an ECHO of a line this frame consumed, never a verdict on
/// what it did.
///
/// A button press has no verdict to give. Every gate a pilot actually meets -
/// needs a lock, needs a well, needs the stance, needs range - fires at the
/// input layer and stops above it, so an ack that claimed to know would be
/// wrong in both directions. A driver reads the effect off the world, the way
/// a player reads it off the HUD. A line the wire itself cannot parse - an
/// unknown name, an axis driven as a button - is still an ERROR, because that
/// is the driver writing nonsense rather than the game deciding anything.
#[derive(Debug, Clone)]
pub struct AppliedEntry {
    /// The stdin line this acknowledges.
    pub line: usize,
    /// The wire name of what was driven.
    pub input: String,
    /// The gesture half: `start` / `stop` / `delta` / `type` / `tap` /
    /// `move` / `press` / `release` / `wheel`.
    pub phase: String,
    /// Free-running only: the line named a tick that had already passed, so it
    /// was applied on the next frame instead.
    pub late: bool,
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
fn command_ack(line: u64, tick: u64, result: &CommandResult) -> serde_json::Value {
    serde_json::json!({
        "line": line,
        "tick": tick,
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
    // A lowered context swallows the PRESS exactly as it swallows a player's
    // key. `input.live` is what says so; the ack only ever echoes the line.
    //
    // A RELEASE is never swallowed. bevy clears only the `just_*` edges, so a
    // key this channel pressed stays down across frames: dropping its release
    // because NOVA OS happened to be open leaves the drive nailed on the
    // moment Flight comes back up, and no field on the wire says so. Releasing
    // unconditionally is also idempotent - `held_source` resolves the source
    // the press pushed, falling back to the action's own binding, and releasing
    // a button that is already up writes nothing anyone can observe. That is
    // what keeps the invariant this module opens with: every synthesized event
    // gets its Released twin.
    if phase == InputPhase::Press && !world.resource::<ActiveContexts>().is_live(context) {
        return ack(world, entry(line, wire, phase_word));
    }
    match dispatch::apply(world, name, phase) {
        Ok(()) => ack(world, entry(line, wire, phase_word)),
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
    // A RELEASE lifts what the PRESS pushed, and only falls back to resolving
    // the id again when nothing was recorded under it. The mount a `start`
    // reached can be shot off between the two lines - the section despawns
    // with it - and re-resolving would then find nothing and refuse, leaving
    // the source the press pushed held down for the rest of the process with
    // every other consumer bound to it reading it held. Resolving is refused
    // for a PRESS alone: that is the line naming a section that is not there.
    let source = match phase {
        InputPhase::Press => section_source(world, id),
        InputPhase::Release => {
            dispatch::driven_press(world, &wire).or_else(|| section_source(world, id))
        }
    };
    let Some(source) = source else {
        if phase == InputPhase::Release {
            // Nothing was pushed and nothing resolves, so nothing is held.
            return ack(world, entry(line, &wire, phase_word));
        }
        return refuse(world, line, format!("no section `{id}` on the ship"));
    };
    // Press only, for the reason `apply_input` states above it.
    if phase == InputPhase::Press
        && !world
            .resource::<ActiveContexts>()
            .is_live(nova_input::prelude::ActionContext::Flight)
    {
        return ack(world, entry(line, &wire, phase_word));
    }
    dispatch::press_source(world, source, phase);
    dispatch::record_press(world, &wire, source, phase);
    ack(world, entry(line, &wire, phase_word));
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
        return ack(world, entry(line, wire, "delta"));
    }
    match dispatch::apply_axis(world, name, delta) {
        Ok(()) => ack(world, entry(line, wire, "delta")),
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
    ack(world, entry(line, "text", "type"));
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
    ack(world, entry(line, &format!("key.{key}"), "tap"));
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
    let done = |phase: &str| entry(line, "pointer", phase);
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

fn entry(line: usize, input: &str, phase: &str) -> AppliedEntry {
    AppliedEntry {
        line,
        input: input.to_string(),
        phase: phase.to_string(),
        late: false,
    }
}

/// Resolve the frame's acks into `applied` JSON entries, stamped with the tick
/// the frame just finished.
///
/// An input entry is `{line, input, phase, tick}` and says nothing more: the
/// frame consumed this line. A command entry keeps its `state`, `detail` and
/// `rows`, because a command IS a request the shell answers - see
/// [`AppliedEntry`] for why the two differ.
pub fn drain_acks(world: &mut World, tick: u64) -> (Vec<serde_json::Value>, Vec<(usize, String)>) {
    let ChannelAck { applied, errors } = std::mem::take(&mut *world.resource_mut::<ChannelAck>());
    let mut applied: Vec<serde_json::Value> = applied
        .into_iter()
        .map(|entry| {
            let mut record = serde_json::json!({
                "line": entry.line,
                "input": entry.input,
                "phase": entry.phase,
                "tick": tick,
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
                .map(|(source, result)| command_ack(source.seq, tick, &result)),
        );
    }
    (applied, errors)
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

        let (applied, errors) = drain_acks(&mut world, 42);
        assert!(errors.is_empty());
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0]["line"], 3);
        assert_eq!(applied[0]["tick"], 42);
        assert_eq!(applied[0]["state"], "error");
        assert_eq!(applied[0]["command"], "graphix");
    }

    /// The ack names the command and its class, never the scenario action a
    /// cheat runs behind it.
    #[test]
    fn a_command_ack_carries_the_command_its_class_and_its_result() {
        let ack = command_ack(
            9,
            120,
            &CommandResult::ok("graphics", CommandClass::Setting, "graphics: low"),
        );
        assert_eq!(ack["line"], 9);
        assert_eq!(ack["tick"], 120);
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
        ack(&mut world, entry(7, "flight.main_drive", "start"));
        ack(&mut world, entry(8, "flight.main_drive", "stop"));
        let acks = &world.resource::<ChannelAck>().applied;
        assert!(acks[0].late, "line 7 was staged late");
        assert!(!acks[1].late, "line 8 was on time");
    }

    #[test]
    fn only_a_late_ack_serializes_the_flag() {
        let mut world = ack_world();
        world.resource_mut::<ChannelFrame>().late_lines.insert(7);
        ack(&mut world, entry(7, "flight.main_drive", "start"));
        ack(&mut world, entry(8, "flight.main_drive", "stop"));
        let (applied, errors) = drain_acks(&mut world, 9);
        assert!(errors.is_empty());
        assert_eq!(applied[0]["late"], serde_json::Value::Bool(true));
        assert!(
            applied[1].get("late").is_none(),
            "on time: no flag on the wire"
        );
    }

    /// The echo rule, pinned: an input ack carries the line, the name, the
    /// half and the tick - and no verdict on what the press achieved.
    #[test]
    fn an_input_ack_is_an_echo_with_no_verdict_field() {
        let mut world = ack_world();
        ack(&mut world, entry(4, "flight.main_drive", "start"));
        let (applied, _) = drain_acks(&mut world, 61);
        assert_eq!(
            applied[0],
            serde_json::json!({
                "line": 4, "input": "flight.main_drive", "phase": "start", "tick": 61
            })
        );
    }

    /// The invariant this module opens with, over the seam that used to break
    /// it: a driver presses the drive in Flight, opens NOVA OS, and releases.
    /// The release must land even though the context that took the press is
    /// down, or the drive is nailed on the moment Flight comes back up.
    #[test]
    fn a_release_sent_while_the_context_is_down_still_lifts_the_key() {
        use nova_input::prelude::{
            ActionBinding, ActionContext, ActiveContexts, InputBindings, InputSource,
        };

        let mut world = ack_world();
        world.insert_resource(InputBindings::from_actions([ActionBinding::new(
            "main_drive",
            "FLIGHT",
            "Main Drive",
        )
        .context(ActionContext::Flight)
        .keyboard([InputSource::Keyboard(KeyCode::KeyW)])]));
        world.init_resource::<ButtonInput<KeyCode>>();
        let mut contexts = ActiveContexts::default();
        contexts.set(ActionContext::Flight, true);
        world.insert_resource(contexts);

        apply_input(&mut world, 1, "flight.main_drive", InputPhase::Press);
        assert!(
            world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyW),
            "the press never reached the key"
        );

        world
            .resource_mut::<ActiveContexts>()
            .set(ActionContext::Flight, false);
        apply_input(&mut world, 2, "flight.main_drive", InputPhase::Release);

        assert!(
            !world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyW),
            "the release was swallowed with the context and the drive stayed on"
        );
    }

    /// The same invariant on the SECTION lane, whose press-only gate is a copy
    /// of the one above: a driver holds a mount's trigger in Flight, opens
    /// NOVA OS, and stops. The stop must reach the bound source, or the mount
    /// is left firing the moment Flight comes back up.
    #[test]
    fn a_section_stop_under_a_lowered_context_still_lifts_the_trigger() {
        use nova_input::prelude::{ActionContext, ActiveContexts};

        let mut world = ack_world();
        world.init_resource::<ButtonInput<MouseButton>>();
        let mut contexts = ActiveContexts::default();
        contexts.set(ActionContext::Flight, true);
        world.insert_resource(contexts);

        let ship = world
            .spawn((PlayerSpaceshipMarker, SpaceshipRootMarker))
            .id();
        world.spawn((
            SectionMarker,
            EntityId::new("port_turret"),
            SpaceshipTurretInputBinding(vec![InputSource::Mouse(MouseButton::Left)]),
            ChildOf(ship),
        ));

        apply_section(&mut world, 1, "port_turret", InputPhase::Press, "start");
        assert!(
            world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left),
            "the press never reached the mount"
        );

        world
            .resource_mut::<ActiveContexts>()
            .set(ActionContext::Flight, false);
        apply_section(&mut world, 2, "port_turret", InputPhase::Release, "stop");

        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left),
            "the stop was swallowed with the context and the mount stayed hot"
        );
    }

    /// The other half of that invariant, over the seam the context gate does
    /// not cover: the mount is SHOT OFF between the start and the stop, so the
    /// id the stop names resolves to nothing. Re-resolving per line would
    /// refuse the stop and leave the source the start pushed held for the rest
    /// of the process, with every other consumer bound to it reading it held.
    #[test]
    fn a_section_stop_still_lifts_the_trigger_after_the_mount_is_destroyed() {
        use nova_input::prelude::{ActionContext, ActiveContexts};

        let mut world = ack_world();
        world.init_resource::<ButtonInput<MouseButton>>();
        let mut contexts = ActiveContexts::default();
        contexts.set(ActionContext::Flight, true);
        world.insert_resource(contexts);

        let ship = world
            .spawn((PlayerSpaceshipMarker, SpaceshipRootMarker))
            .id();
        let turret = world
            .spawn((
                SectionMarker,
                EntityId::new("port_turret"),
                SpaceshipTurretInputBinding(vec![InputSource::Mouse(MouseButton::Left)]),
                ChildOf(ship),
            ))
            .id();

        apply_section(&mut world, 1, "port_turret", InputPhase::Press, "start");
        assert!(
            world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left),
            "the press never reached the mount"
        );

        world.entity_mut(turret).despawn();
        apply_section(&mut world, 2, "port_turret", InputPhase::Release, "stop");

        assert!(
            !world
                .resource::<ButtonInput<MouseButton>>()
                .pressed(MouseButton::Left),
            "the stop was refused with the mount and the trigger stayed held"
        );
        let (applied, refused) = drain_acks(&mut world, 7);
        assert!(refused.is_empty(), "the stop was refused: {refused:?}");
        assert_eq!(applied.len(), 2, "both lines answered on their own line");
    }

    /// A `stop` for a section that was never started and is not there is not a
    /// refusal: nothing is held, so nothing has to be lifted. The `start` that
    /// named it is where the driver was told.
    #[test]
    fn a_section_stop_for_a_mount_that_was_never_started_is_a_no_op() {
        let mut world = ack_world();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.insert_resource(ActiveContexts::default());

        apply_section(&mut world, 1, "port_turret", InputPhase::Press, "start");
        apply_section(&mut world, 2, "port_turret", InputPhase::Release, "stop");

        let (applied, refused) = drain_acks(&mut world, 7);
        assert_eq!(
            refused.len(),
            1,
            "the start named a section that is not there"
        );
        assert_eq!(refused[0].0, 1);
        assert_eq!(applied.len(), 1, "the stop answered without a verdict");
    }
}
