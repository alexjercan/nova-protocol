//! The channel's runner: drain the reader thread, gate the clock in step
//! mode, step the app, and answer every batch with one snapshot line.
//!
//! The reader thread owns the blocking `read_line`; the runner only ever
//! receives from its mpsc, so the game loop never blocks on a file
//! descriptor. Installed with `App::set_runner` after the builder - last
//! runner wins, which is exactly how `ScheduleRunnerPlugin` installs its own.
//!
//! ## Step mode
//!
//! The clock runs on `TimeUpdateStrategy::ManualDuration`: one update is one
//! tick of [`TICK_DT`], and the world moves ONLY while a `run to here`
//! instruction is being honored - the observe-decide-act loop a driver wants,
//! with the world frozen while it thinks. A tick is required on every line
//! (it IS the schedule), a bare tick runs the clock to the target and
//! answers with the snapshot, and EOF is an exit - the clock could never
//! advance again.
//!
//! ## Free-running
//!
//! The app runs at its own pace; lines apply on the next frame after they
//! arrive. A line's tick is optional; one that has already passed is applied
//! anyway and reported `late`. A bare tick is a no-op, every frame that
//! consumed lines answers with a snapshot, and EOF just closes the lane -
//! the game plays on.

use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
    sync::mpsc::{Receiver, TryRecvError},
    time::Duration,
};

use bevy::{app::PluginsState, prelude::*};
use nova_gameplay::GameStates;
use nova_input::prelude::{ActionContext, ActiveContexts, InputBindings};

use crate::{
    apply::{drain_acks, wire_name, ChannelFrame},
    protocol::{parse_line, Envelope, Lane},
    ChannelMode,
};

/// One step-mode tick of simulated time, the fixed 60 Hz frame.
pub const TICK_DT: Duration = Duration::from_micros(16_667);

/// Wire-format version of the channel's own lines (errors); snapshots carry
/// `nova_probe`'s [`SNAPSHOT_SCHEMA`](nova_probe::SNAPSHOT_SCHEMA).
const CHANNEL_SCHEMA: u32 = 1;

/// Spawn the stdin reader thread. It sends `(line_number, line)` until EOF or
/// until the runner hangs up, whichever comes first.
pub fn spawn_reader() -> Receiver<(usize, String)> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("nova-channel-stdin".to_string())
        .spawn(move || {
            let stdin = std::io::stdin();
            for (index, line) in stdin.lock().lines().enumerate() {
                let Ok(line) = line else {
                    break;
                };
                if sender.send((index + 1, line)).is_err() {
                    break;
                }
            }
        })
        .expect("the stdin reader thread spawns");
    receiver
}

/// The runner closure [`crate::NovaChannelPlugin`] installs.
pub fn channel_runner(mode: ChannelMode) -> impl FnOnce(App) -> AppExit {
    move |mut app: App| {
        if app.plugins_state() != PluginsState::Cleaned {
            while app.plugins_state() == PluginsState::Adding {
                bevy::tasks::tick_global_task_pools_on_main_thread();
            }
            app.finish();
            app.cleanup();
        }
        let lines = spawn_reader();
        // The manual clock's first update advances time by nothing, so it is
        // spent here as the warm-up rather than sold to the driver as tick 1.
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
        if let Some(exit) = boot(&mut app) {
            return exit;
        }
        let exit = match mode {
            ChannelMode::Step => run_stepped(&mut app, &lines),
            ChannelMode::Free => run_free(&mut app, &lines),
        };
        // A recording session has captures still in flight at EOF.
        crate::record::flush_captures(&mut app);
        exit
    }
}

/// How long the asset boot may take before the channel gives up on the app.
const BOOT_DEADLINE: Duration = Duration::from_secs(300);

/// Hold the wire until the app leaves [`GameStates::Loading`], plus two settle
/// frames for the entered state's own spawn work (the scenario, the context
/// sync). Boot is wall-clock asset IO - a frame count no two runs share - so
/// it is never sold to the driver as ticks: tick 0 is the world, ready.
fn boot(app: &mut App) -> Option<AppExit> {
    let started = std::time::Instant::now();
    loop {
        let loading = app
            .world()
            .get_resource::<State<GameStates>>()
            .is_some_and(|state| *state.get() == GameStates::Loading);
        if !loading {
            break;
        }
        if started.elapsed() > BOOT_DEADLINE {
            emit_error("the app never finished loading", 0);
            return Some(AppExit::error());
        }
        app.update();
        if let Some(exit) = app.should_exit() {
            return Some(exit);
        }
    }
    for _ in 0..2 {
        app.update();
        if let Some(exit) = app.should_exit() {
            return Some(exit);
        }
    }
    None
}

fn run_stepped(app: &mut App, lines: &Receiver<(usize, String)>) -> AppExit {
    let mut scheduled: BTreeMap<u64, Vec<(usize, Lane)>> = BTreeMap::new();
    let mut applied: Vec<serde_json::Value> = Vec::new();
    let mut tick: u64 = 0;
    let mut target: u64 = 0;

    loop {
        let Ok((line_no, raw)) = lines.recv() else {
            // EOF is an exit: nothing can ever advance the clock again.
            return AppExit::Success;
        };
        if raw.trim().is_empty() {
            continue;
        }
        let envelope = match parse_line(&raw) {
            Ok(envelope) => envelope,
            Err(message) => {
                emit_error(&message, line_no);
                continue;
            }
        };
        let Some(line_tick) = envelope.tick else {
            emit_error("step mode needs a tick on every line", line_no);
            continue;
        };
        if line_tick <= tick {
            emit_error(&format!("tick {line_tick} is in the past"), line_no);
            continue;
        }
        target = target.max(line_tick);
        match envelope.lane {
            Some(lane) => scheduled
                .entry(line_tick)
                .or_default()
                .push((line_no, lane)),
            None => {
                // The step instruction: run the clock to the target.
                while tick < target {
                    crate::record::record_frame(app);
                    stage(app, scheduled.remove(&(tick + 1)).unwrap_or_default());
                    app.update();
                    tick += 1;
                    collect(app, &mut applied);
                    if let Some(exit) = app.should_exit() {
                        return exit;
                    }
                }
                if !emit(&snapshot(app, "step", &mut applied)) {
                    return AppExit::Success;
                }
            }
        }
    }
}

fn run_free(app: &mut App, lines: &Receiver<(usize, String)>) -> AppExit {
    let mut scheduled: BTreeMap<u64, Vec<(usize, Lane)>> = BTreeMap::new();
    let mut applied: Vec<serde_json::Value> = Vec::new();
    let mut late: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut tick: u64 = 0;
    let mut open = true;

    loop {
        while open {
            match lines.try_recv() {
                Ok((line_no, raw)) => {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    match parse_line(&raw) {
                        Err(message) => emit_error(&message, line_no),
                        // A bare tick is a no-op free-running; the clock is
                        // not the driver's to gate.
                        Ok(Envelope { lane: None, .. }) => {}
                        Ok(Envelope {
                            tick: line_tick,
                            lane: Some(lane),
                        }) => {
                            // Next frame at the earliest; a passed tick still
                            // applies and the ack says `late`.
                            if line_tick.is_some_and(|named| named <= tick) {
                                late.insert(line_no);
                            }
                            let due = line_tick.unwrap_or(0).max(tick + 1);
                            scheduled.entry(due).or_default().push((line_no, lane));
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => open = false,
            }
        }

        let due: Vec<(usize, Lane)> = {
            let mut due = Vec::new();
            let overdue: Vec<u64> = scheduled.range(..=tick + 1).map(|(t, _)| *t).collect();
            for t in overdue {
                due.extend(scheduled.remove(&t).unwrap_or_default());
            }
            due
        };
        let consumed = !due.is_empty();
        // A late line is always due on the very next frame, so the set only
        // ever holds lines this frame is about to consume.
        app.world_mut().resource_mut::<ChannelFrame>().late_lines = std::mem::take(&mut late);
        crate::record::record_frame(app);
        stage(app, due);
        app.update();
        tick += 1;
        collect(app, &mut applied);
        if let Some(exit) = app.should_exit() {
            return exit;
        }
        if consumed && !emit(&snapshot(app, "applied", &mut applied)) {
            return AppExit::Success;
        }
    }
}

/// Put the frame's lines where the two writer systems drain them.
fn stage(app: &mut App, staged: Vec<(usize, Lane)>) {
    let mut frame = app.world_mut().resource_mut::<ChannelFrame>();
    for (line, lane) in staged {
        match lane {
            Lane::Pointer(_) => frame.pointer.push((line, lane)),
            _ => frame.input.push((line, lane)),
        }
    }
}

/// After a frame: echo its refusals and bank its acks (each named action's
/// `TriggerState` read now, after the frame evaluated).
fn collect(app: &mut App, applied: &mut Vec<serde_json::Value>) {
    let (acks, errors) = drain_acks(app.world_mut());
    for (line_no, message) in errors {
        emit_error(&message, line_no);
    }
    applied.extend(acks);
}

/// The stepped snapshot: `nova_probe`'s whole serializer, plus the channel's
/// own blocks - `applied` (consumed since the last snapshot) and `input`
/// (what may be pressed THIS tick).
fn snapshot(
    app: &mut App,
    reason: &str,
    applied: &mut Vec<serde_json::Value>,
) -> serde_json::Value {
    let world = app.world_mut();
    let mut snapshot = nova_probe::capture_snapshot(world, reason);
    snapshot["applied"] = serde_json::Value::Array(std::mem::take(applied));
    snapshot["input"] = input_block(world);
    snapshot
}

/// `input.live` - the wire names that can fire this tick - and the raised
/// contexts, both sorted so two snapshots of one state diff clean.
fn input_block(world: &World) -> serde_json::Value {
    let bindings = world.resource::<InputBindings>();
    let active = world.resource::<ActiveContexts>();
    let mut live: Vec<String> = bindings
        .live(active)
        .map(|action| wire_name(action.group, action.name))
        .collect();
    live.sort();
    let mut contexts: Vec<String> = std::iter::once(ActionContext::Always)
        .chain(active.iter())
        .map(|context| match context {
            ActionContext::Always => "always".to_string(),
            ActionContext::Flight => "flight".to_string(),
            ActionContext::Viewer => "viewer".to_string(),
            ActionContext::ViewerApp(app) => format!("viewer:{app}"),
        })
        .collect();
    contexts.sort();
    contexts.dedup();
    serde_json::json!({ "live": live, "contexts": contexts })
}

/// Write one line to stdout. `false` means the client hung up, which ends a
/// session the same way EOF on stdin does.
fn emit(value: &serde_json::Value) -> bool {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{value}")
        .and_then(|()| lock.flush())
        .is_ok()
}

fn emit_error(message: &str, line_no: usize) {
    warn!("nova channel: line {line_no} refused: {message}");
    emit(&serde_json::json!({
        "schema": CHANNEL_SCHEMA,
        "error": message,
        "line": line_no,
    }));
}
