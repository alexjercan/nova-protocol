//! The run-timeline recorder: captures WHAT HAPPENED during a (headless
//! autopilot) run as an ordered, structured timeline - the correctness half of
//! the run-harness (spike tasks/20260719-112011/SPIKE.md, task
//! 20260719-112238).
//!
//! One env-gated plugin, [`nova_timeline`]: inert unless `NOVA_PERF_TIMELINE`
//! names an output path (native only - the browser has no filesystem). When
//! armed it appends one JSON object per line (JSONL) to that path, flushed
//! per entry so a panicked or backstopped run keeps everything up to the
//! panic - which is exactly when the timeline matters most.
//!
//! What lands on the timeline:
//!
//! - `run_start` / `run_end` - run bracket, with the git SHA + host the
//!   capture metadata also records.
//! - `state` - every [`GameStates`] / [`PauseStates`] transition, read from
//!   bevy's [`StateTransitionEvent`] messages (a `Message` with
//!   `exited`/`entered`; bevy_state-0.19.0/src/state/transitions.rs:67).
//! - `scenario_event` - every fired scenario [`GameEvent`] (kill, area
//!   enter/exit, orbit hold, travel/combat lock, the per-frame update pulse),
//!   name + payload, observed via `On<GameEvent>` WITHOUT touching the
//!   dispatch queue (the read accessors landed in bevy_common_systems
//!   v0.19.2 for exactly this).
//! - `variable` - every scenario-variable change (old/new), from a per-frame
//!   snapshot diff of [`NovaEventWorld`]'s variables that mirrors the
//!   engine's own write-on-diff logging (world.rs) and likewise ignores the
//!   every-frame `scenario_elapsed` clock.
//! - `marker` - beats an autopilot script pushes itself via [`probe_marker`],
//!   so a run reads "raise -> fire -> kill confirmed -> lowered -> goto" in
//!   the same stream as the engine's own signals.
//!
//! Entries are stamped with wall-clock run time (`t_real`), the render
//! [`FrameCount`], and the scenario clock when live - but consumers comparing
//! runs should key on ORDER and VALUES, not timestamps: wall-clock and frame
//! counts vary wildly across hosts (llvmpipe CI vs a dev GPU).

use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use bevy::{diagnostic::FrameCount, prelude::*, state::state::StateTransitionEvent};
use nova_gameplay::{bevy_common_systems::modding::events::GameEvent, GameStates, PauseStates};
use nova_scenario::{
    loader::SCENARIO_ELAPSED_VAR, variables::VariableLiteral, world::NovaEventWorld,
};

use crate::capture::{perf_param, resolve_git_sha, resolve_host};

/// Env var (via [`perf_param`], so `NOVA_PERF_TIMELINE` on native) naming the
/// JSONL output path that arms [`nova_timeline`]. Part of the `NOVA_PERF_*`
/// surface the runner-CLI task redesigns wholesale.
pub const TIMELINE_PARAM: &str = "timeline";

/// Env-gated run-timeline recorder preset. Inert unless `NOVA_PERF_TIMELINE`
/// is set (or an explicit [`out`](RunRecorderPlugin::out) override is given,
/// which tests use to avoid process-global env races). See the module docs.
pub fn nova_timeline() -> RunRecorderPlugin {
    RunRecorderPlugin { out: None }
}

/// Plugin returned by [`nova_timeline`]. Construct it through that preset.
pub struct RunRecorderPlugin {
    out: Option<PathBuf>,
}

impl RunRecorderPlugin {
    /// Force the output path, bypassing the env lookup. For tests (env vars
    /// are process-global and race across parallel tests) and for callers
    /// that already resolved a run directory.
    pub fn out(mut self, path: impl Into<PathBuf>) -> Self {
        self.out = Some(path.into());
        self
    }
}

impl Plugin for RunRecorderPlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self
            .out
            .clone()
            .or_else(|| perf_param(TIMELINE_PARAM).map(PathBuf::from))
        else {
            return;
        };
        let timeline = match ProbeTimeline::create(path) {
            Ok(timeline) => timeline,
            Err(error) => {
                warn!("nova probe: timeline disabled: {error}");
                return;
            }
        };
        info!("nova probe: timeline armed -> {:?}", timeline.path);
        app.insert_resource(timeline);
        app.add_systems(Startup, record_run_start);
        // State transitions: the Messages<StateTransitionEvent<S>>
        // resource only exists where init_state::<S>() ran; the run_if
        // guard keeps the recorder harmless on an app without that state
        // (messagereader-needs-resource-guard).
        app.add_systems(
            Update,
            (
                record_state_transitions::<GameStates>
                    .run_if(resource_exists::<Messages<StateTransitionEvent<GameStates>>>),
                record_state_transitions::<PauseStates>
                    .run_if(resource_exists::<Messages<StateTransitionEvent<PauseStates>>>),
            ),
        );
        // Every fired scenario event, name + payload, straight off the
        // observer - the queue is bcs's to drain, not ours.
        app.add_observer(record_game_event);
        // Variable diff runs in Last so it sees everything PostUpdate
        // wrote (the scenario clock, action writes) in the same frame;
        // run_end runs after it so final changes precede the bracket.
        app.add_systems(Last, (record_variable_changes, record_run_end).chain());
    }
}

/// One timeline entry, as written to (and parsed back from) the JSONL sink.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEvent {
    /// Wall-clock seconds since app start (`Time<Real>`). Host-dependent -
    /// compare runs by order and values, not by this.
    pub t_real: f64,
    /// Render frame number ([`FrameCount`]). Host-dependent too.
    pub frame: u32,
    /// The scenario clock (`scenario_elapsed`) when a scenario is live.
    pub scenario_elapsed: Option<f64>,
    /// Entry kind: `run_start`, `state`, `scenario_event`, `variable`,
    /// `marker`, `run_end`.
    pub kind: String,
    /// The entry's name within its kind (event name, variable key, state
    /// type, marker label).
    pub name: String,
    /// Kind-specific payload.
    pub data: serde_json::Value,
}

impl TimelineEvent {
    /// Render as one JSONL line (no trailing newline).
    fn to_json_line(&self) -> String {
        serde_json::json!({
            "t_real": self.t_real,
            "frame": self.frame,
            "scenario_elapsed": self.scenario_elapsed,
            "kind": self.kind,
            "name": self.name,
            "data": self.data,
        })
        .to_string()
    }

    /// Parse one JSONL line. `None` when the line is not a timeline entry.
    fn from_json_line(line: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        Some(Self {
            t_real: value.get("t_real")?.as_f64()?,
            frame: value.get("frame")?.as_u64()? as u32,
            scenario_elapsed: value.get("scenario_elapsed")?.as_f64(),
            kind: value.get("kind")?.as_str()?.to_string(),
            name: value.get("name")?.as_str()?.to_string(),
            data: value.get("data")?.clone(),
        })
    }
}

/// Parse a whole JSONL timeline file back into entries, preserving order.
/// Blank lines are skipped; a malformed line is an error naming its number,
/// so a corrupt file is caught instead of silently dropping entries.
pub fn parse_timeline(contents: &str) -> Result<Vec<TimelineEvent>, String> {
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            TimelineEvent::from_json_line(line)
                .ok_or_else(|| format!("malformed timeline line {}: {line:?}", i + 1))
        })
        .collect()
}

/// The live timeline: the JSONL sink plus the last variable snapshot the
/// diff system compared against. Present only while the recorder is armed,
/// so [`probe_marker`] can treat its absence as "recorder off".
#[derive(Resource)]
pub struct ProbeTimeline {
    sink: BufWriter<File>,
    path: PathBuf,
    /// Variables as of the last diff, as JSON values (VariableLiteral maps
    /// in), excluding the `scenario_elapsed` clock.
    last_vars: HashMap<String, serde_json::Value>,
    entries: u64,
}

impl ProbeTimeline {
    fn create(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        let file =
            File::create(&path).map_err(|e| format!("could not create {}: {e}", path.display()))?;
        Ok(Self {
            sink: BufWriter::new(file),
            path,
            last_vars: HashMap::new(),
            entries: 0,
        })
    }

    /// Append one entry and FLUSH it, so a panic a frame later cannot lose
    /// it - a truncated run's timeline is the debugging artifact.
    fn record(&mut self, entry: TimelineEvent) {
        let line = entry.to_json_line();
        if let Err(error) = writeln!(self.sink, "{line}").and_then(|()| self.sink.flush()) {
            warn!("nova probe: timeline write failed: {error}");
        }
        self.entries += 1;
    }
}

/// The (t_real, frame, scenario_elapsed) stamp for a new entry.
fn stamp(
    time: &Time<Real>,
    frame: &FrameCount,
    scenario: Option<&NovaEventWorld>,
) -> (f64, u32, Option<f64>) {
    let elapsed = scenario.and_then(|world| match world.get_variable(SCENARIO_ELAPSED_VAR) {
        Some(VariableLiteral::Number(n)) => Some(*n),
        _ => None,
    });
    (time.elapsed_secs_f64(), frame.0, elapsed)
}

/// Map a scenario variable to its JSON value. A NaN/infinite number (JSON
/// cannot carry those) maps to null rather than panicking the recorder.
fn variable_to_json(value: &VariableLiteral) -> serde_json::Value {
    match value {
        VariableLiteral::String(s) => serde_json::Value::String(s.clone()),
        VariableLiteral::Number(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        VariableLiteral::Boolean(b) => serde_json::Value::Bool(*b),
    }
}

/// Push a `marker` entry from an autopilot script or example. A no-op when
/// the recorder is not armed, so scripts call it unconditionally:
///
/// ```ignore
/// nova_probe::probe_marker(world, "beat: lowered", serde_json::json!({ "t": t }));
/// ```
pub fn probe_marker(world: &mut World, name: &str, data: serde_json::Value) {
    if world.get_resource::<ProbeTimeline>().is_none() {
        return;
    }
    let (t_real, frame, scenario_elapsed) = stamp(
        &world.resource::<Time<Real>>().clone(),
        world.resource::<FrameCount>(),
        world.get_resource::<NovaEventWorld>(),
    );
    world.resource_mut::<ProbeTimeline>().record(TimelineEvent {
        t_real,
        frame,
        scenario_elapsed,
        kind: "marker".to_string(),
        name: name.to_string(),
        data,
    });
}

/// Open the run bracket with the same identity metadata the perf capture
/// records (sha, host - see `capture::RunMeta`).
fn record_run_start(
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    mut timeline: ResMut<ProbeTimeline>,
) {
    let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, None);
    timeline.record(TimelineEvent {
        t_real,
        frame,
        scenario_elapsed,
        kind: "run_start".to_string(),
        name: "run".to_string(),
        data: serde_json::json!({
            "git_sha": resolve_git_sha(),
            "host": resolve_host(),
        }),
    });
}

/// Record every state transition of `S` from bevy's transition messages.
fn record_state_transitions<S: States>(
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    scenario: Option<Res<NovaEventWorld>>,
    mut timeline: ResMut<ProbeTimeline>,
) {
    for transition in transitions.read() {
        let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, scenario.as_deref());
        // The short type name ("GameStates"), not the full module path.
        let name = std::any::type_name::<S>()
            .rsplit("::")
            .next()
            .unwrap_or("state")
            .to_string();
        timeline.record(TimelineEvent {
            t_real,
            frame,
            scenario_elapsed,
            kind: "state".to_string(),
            name,
            data: serde_json::json!({
                "exited": transition.exited.as_ref().map(|s| format!("{s:?}")),
                "entered": transition.entered.as_ref().map(|s| format!("{s:?}")),
            }),
        });
    }
}

/// Record every fired scenario event (name + payload) off the `GameEvent`
/// observer. Reading is all this does: the dispatch queue stays bcs's.
fn record_game_event(
    event: On<GameEvent>,
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    scenario: Option<Res<NovaEventWorld>>,
    mut timeline: ResMut<ProbeTimeline>,
) {
    let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, scenario.as_deref());
    timeline.record(TimelineEvent {
        t_real,
        frame,
        scenario_elapsed,
        kind: "scenario_event".to_string(),
        name: event.name().to_string(),
        data: event.info().data.clone().unwrap_or(serde_json::Value::Null),
    });
}

/// Diff the scenario variables against the last snapshot and record one
/// `variable` entry per change (old/new; a removed variable records
/// `new: null`). Mirrors the engine's own write-on-diff logging: the
/// every-frame `scenario_elapsed` clock is excluded, everything else counts.
fn record_variable_changes(
    scenario: Option<Res<NovaEventWorld>>,
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    mut timeline: ResMut<ProbeTimeline>,
) {
    let current: HashMap<String, serde_json::Value> = scenario
        .as_deref()
        .map(|world| {
            world
                .variables()
                .filter(|(key, _)| key.as_str() != SCENARIO_ELAPSED_VAR)
                .map(|(key, value)| (key.clone(), variable_to_json(value)))
                .collect()
        })
        .unwrap_or_default();

    // Fast path: nothing changed (the overwhelmingly common frame).
    if current == timeline.last_vars {
        return;
    }

    let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, scenario.as_deref());
    // Added or changed keys...
    let mut changes: Vec<(String, serde_json::Value)> = Vec::new();
    for (key, new_value) in &current {
        let old = timeline.last_vars.get(key);
        if old != Some(new_value) {
            changes.push((
                key.clone(),
                serde_json::json!({
                    "old": old.cloned().unwrap_or(serde_json::Value::Null),
                    "new": new_value,
                }),
            ));
        }
    }
    // ...and removed ones (scenario teardown clears the map).
    for key in timeline.last_vars.keys() {
        if !current.contains_key(key) {
            changes.push((
                key.clone(),
                serde_json::json!({
                    "old": timeline.last_vars[key],
                    "new": serde_json::Value::Null,
                }),
            ));
        }
    }
    // Deterministic order within the frame: the map iteration order is not.
    changes.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, data) in changes {
        timeline.record(TimelineEvent {
            t_real,
            frame,
            scenario_elapsed,
            kind: "variable".to_string(),
            name,
            data,
        });
    }
    timeline.last_vars = current;
}

/// Close the run bracket on the first `AppExit` message.
fn record_run_end(
    mut exits: MessageReader<AppExit>,
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    scenario: Option<Res<NovaEventWorld>>,
    mut timeline: ResMut<ProbeTimeline>,
) {
    let Some(exit) = exits.read().next() else {
        return;
    };
    let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, scenario.as_deref());
    let entries = timeline.entries;
    timeline.record(TimelineEvent {
        t_real,
        frame,
        scenario_elapsed,
        kind: "run_end".to_string(),
        name: "run".to_string(),
        data: serde_json::json!({
            "exit": format!("{exit:?}"),
            "entries": entries,
        }),
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use bevy::state::app::StatesPlugin;
    use nova_gameplay::bevy_common_systems::modding::events::GameEventInfo;

    use super::*;

    /// A unique temp path per test (no Date::now: process id + counter).
    fn temp_timeline() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "nova_probe_timeline_{}_{n}.jsonl",
            std::process::id()
        ))
    }

    /// Production-faithful rig: the real plugin over the real GameStates /
    /// PauseStates and a real NovaEventWorld resource; only the window/render
    /// stack is absent (the recorder never touches it).
    fn rig(path: &PathBuf) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_state::<PauseStates>();
        app.init_resource::<NovaEventWorld>();
        app.add_plugins(nova_timeline().out(path.clone()));
        app
    }

    fn read_entries(path: &PathBuf) -> Vec<TimelineEvent> {
        parse_timeline(&std::fs::read_to_string(path).expect("timeline file exists"))
            .expect("timeline parses")
    }

    #[test]
    fn records_states_events_variables_and_markers_in_order() {
        let path = temp_timeline();
        let mut app = rig(&path);
        app.update(); // Startup: run_start; initial state transitions flush

        // A scenario event with a payload, through the real observer path.
        app.world_mut().trigger(GameEvent::new(
            "ondestroyed",
            GameEventInfo::from_data(serde_json::json!({ "id": "prey" })),
        ));

        // A state transition.
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);

        // A variable write (what a scenario action does).
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("target_down".to_string(), VariableLiteral::Number(1.0));

        app.update();

        // A script marker.
        probe_marker(
            app.world_mut(),
            "beat: lowered",
            serde_json::json!({ "why": "kill confirmed" }),
        );

        // FLUSH-PER-ENTRY PIN: read the file NOW, before any exit/teardown -
        // a panicked run must already have everything on disk.
        let entries = read_entries(&path);
        let kinds: Vec<(&str, &str)> = entries
            .iter()
            .map(|e| (e.kind.as_str(), e.name.as_str()))
            .collect();

        assert_eq!(entries[0].kind, "run_start");
        assert!(
            entries[0].data["git_sha"].is_string() && entries[0].data["host"].is_string(),
            "run_start carries identity metadata: {:?}",
            entries[0].data
        );
        assert!(
            kinds.contains(&("scenario_event", "ondestroyed")),
            "fired event recorded: {kinds:?}"
        );
        let destroyed = entries
            .iter()
            .find(|e| e.kind == "scenario_event" && e.name == "ondestroyed")
            .unwrap();
        assert_eq!(destroyed.data["id"], "prey", "payload preserved");
        assert!(
            kinds.contains(&("state", "GameStates")),
            "state transition recorded: {kinds:?}"
        );
        let playing = entries
            .iter()
            .rev()
            .find(|e| e.kind == "state" && e.name == "GameStates")
            .unwrap();
        assert_eq!(playing.data["entered"], "Playing");
        let variable = entries
            .iter()
            .find(|e| e.kind == "variable" && e.name == "target_down")
            .expect("variable change recorded");
        assert_eq!(variable.data["old"], serde_json::Value::Null);
        assert_eq!(variable.data["new"], 1.0);
        assert_eq!(entries.last().unwrap().kind, "marker");
        assert_eq!(entries.last().unwrap().name, "beat: lowered");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn variable_diff_reports_changes_and_removals_not_steady_state() {
        let path = temp_timeline();
        let mut app = rig(&path);
        app.update();

        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("leg".to_string(), VariableLiteral::Number(0.0));
        app.update();
        // Steady frames must not re-log the unchanged variable.
        app.update();
        app.update();
        // Change it, then clear the scenario (teardown).
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("leg".to_string(), VariableLiteral::Number(1.0));
        app.update();
        app.world_mut().resource_mut::<NovaEventWorld>().clear();
        app.update();

        let entries = read_entries(&path);
        let legs: Vec<&TimelineEvent> = entries
            .iter()
            .filter(|e| e.kind == "variable" && e.name == "leg")
            .collect();
        assert_eq!(legs.len(), 3, "exactly appear, change, disappear: {legs:?}");
        assert_eq!(legs[0].data["new"], 0.0);
        assert_eq!(legs[1].data["old"], 0.0);
        assert_eq!(legs[1].data["new"], 1.0);
        assert_eq!(legs[2].data["new"], serde_json::Value::Null, "teardown");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn run_end_closes_the_bracket_on_app_exit() {
        let path = temp_timeline();
        let mut app = rig(&path);
        app.update();
        app.world_mut().write_message(AppExit::Success);
        app.update();

        let entries = read_entries(&path);
        let end = entries.last().unwrap();
        assert_eq!(end.kind, "run_end");
        assert!(end.data["entries"].as_u64().unwrap() >= 1);
        assert!(end.data["exit"].as_str().unwrap().contains("Success"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unarmed_recorder_is_a_no_op_and_marker_is_safe() {
        // No env, no out override: the plugin must add nothing, and
        // probe_marker on an unarmed world must be a silent no-op.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(RunRecorderPlugin { out: None });
        app.update();
        assert!(app.world().get_resource::<ProbeTimeline>().is_none());
        probe_marker(app.world_mut(), "beat", serde_json::Value::Null);
    }

    #[test]
    fn parse_timeline_rejects_a_malformed_line() {
        let err = parse_timeline("{\"kind\": \"state\"}\n").expect_err("missing fields rejected");
        assert!(err.contains("malformed timeline line 1"), "{err}");
        let ok = parse_timeline("").expect("empty file is an empty timeline");
        assert!(ok.is_empty());
    }
}
