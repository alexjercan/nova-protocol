//! Where a frame's milliseconds go, by NAME: the main world's schedules and
//! the render world's phases on the CPU, whatever is left outside both, and -
//! when the adapter can serve timestamp queries - each render pass on the GPU.
//!
//! Change this module when "the frame costs 16 ms" needs to become a list of
//! named items that sum to 16. The two CPU halves are always available; the GPU
//! half needs [`nova_core::RENDER_DIAG_ENV`] set, because the queries it turns
//! on cost a resolve pass and a readback of their own.
//!
//! Read the three together. Under pipelined rendering the main thread runs the
//! main world for frame N while the render thread is still on frame N-1, so a
//! frame costs roughly the LONGER of the two and `outside_main` is mostly the
//! main thread waiting. `render_world` larger than `main_world` says the
//! renderer sets the pace; GPU passes far under both says the device does not.

/// Glob-import surface for the frame-cost capability.
pub mod prelude {
    pub use super::{nova_framecost, FrameCostPlugin, DEFAULT_REPORT_FRAMES};
}

use std::time::{Duration, Instant};

use bevy::{
    app::MainScheduleOrder,
    diagnostic::DiagnosticsStore,
    ecs::schedule::{InternedScheduleLabel, ScheduleLabel},
    prelude::*,
    render::{diagnostic::RenderDiagnosticsPlugin, Render, RenderApp, RenderSystems},
};

use crate::capabilities::frametime::prelude::*;

/// Frames averaged into one report.
///
/// A report every 200 frames rather than one at the end: the run can exit on
/// its own completion protocol, and a window that never closes writes nothing.
/// The LAST report in a log is the steady state.
pub const DEFAULT_REPORT_FRAMES: u32 = 200;

/// The frame-cost breakdown, reported every [`DEFAULT_REPORT_FRAMES`] frames.
///
/// Inert unless the run is armed for capture ([`probe_armed`]).
pub fn nova_framecost() -> FrameCostPlugin {
    FrameCostPlugin {
        report_frames: DEFAULT_REPORT_FRAMES,
    }
}

/// Times every main-world schedule, and every render pass when GPU timestamps
/// are available, then logs the two tables side by side against the real frame
/// delta they have to add up to.
#[derive(Clone)]
pub struct FrameCostPlugin {
    report_frames: u32,
}

impl FrameCostPlugin {
    /// Average this many frames into one report.
    pub fn report_frames(mut self, frames: u32) -> Self {
        self.report_frames = frames;
        self
    }
}

/// A marker schedule inserted between two main-world schedules. The index is
/// the position in [`MainScheduleOrder::labels`] it runs BEFORE; the last one
/// runs after everything and carries `labels.len()`.
#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PhaseMark(usize);

impl Plugin for FrameCostPlugin {
    fn build(&self, app: &mut App) {
        if !probe_armed() {
            return;
        }
        if std::env::var_os(nova_core::RENDER_DIAG_ENV).is_some() {
            app.add_plugins(RenderDiagnosticsPlugin);
        }

        let labels: Vec<InternedScheduleLabel> =
            app.world().resource::<MainScheduleOrder>().labels.clone();
        let names: Vec<String> = labels.iter().map(|label| format!("{label:?}")).collect();
        app.insert_resource(FrameCost {
            names,
            elapsed: vec![Duration::ZERO; labels.len()],
            frame_total: Duration::ZERO,
            frames: 0,
            report_frames: probe_param("framecost_frames")
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(self.report_frames),
            last_mark: None,
        });

        // A marker schedule per boundary rather than a system ordered inside
        // each schedule: an ordering constraint only holds against systems that
        // name it, and nothing in the game names a probe system. A schedule of
        // its own is positionally exact by construction.
        let end = labels.len();
        for index in 0..end {
            app.add_systems(PhaseMark(index), move |mut cost: ResMut<FrameCost>| {
                cost.mark(index);
            });
        }
        app.add_systems(
            PhaseMark(end),
            move |mut cost: ResMut<FrameCost>, time: Res<Time<Real>>| {
                cost.mark(end);
                cost.close_frame(time.delta());
            },
        );
        // Rewritten wholesale, not through `insert_before`: that helper
        // compares the label it is handed against the interned ones by dynamic
        // equality, so handing it an already-interned label panics with
        // "Expected ... to exist".
        let mut interleaved = Vec::with_capacity(end * 2 + 1);
        for (index, label) in labels.iter().enumerate() {
            interleaved.push(PhaseMark(index).intern());
            interleaved.push(*label);
        }
        interleaved.push(PhaseMark(end).intern());
        app.world_mut().resource_mut::<MainScheduleOrder>().labels = interleaved;

        let render_cost = RenderCost::shared();
        app.insert_resource(render_cost.clone());
        app.add_systems(Last, report_when_due);
        self.wire_render_world(app, render_cost);
    }

    fn finish(&self, app: &mut App) {
        if !probe_armed() {
            return;
        }
        // The render sub-app exists by `finish` even when `RenderPlugin` built
        // its device asynchronously, so a second attempt here catches the case
        // `build` was too early for.
        let Some(shared) = app.world().get_resource::<RenderCost>().cloned() else {
            return;
        };
        self.wire_render_world(app, shared);
    }
}

impl FrameCostPlugin {
    /// Add the boundary markers to the render sub-app's `Render` schedule, once.
    fn wire_render_world(&self, app: &mut App, shared: RenderCost) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        if render_app.world().contains_resource::<RenderCost>() {
            return;
        }
        render_app.insert_resource(shared);
        // `RenderSystems`' top-level sets are `.chain()`ed by `RenderPlugin`, so
        // "after set i-1, before set i" is an exact boundary rather than a hint.
        for index in 0..=RENDER_SETS.len() {
            let mark = move |cost: Res<RenderCost>| cost.mark(index);
            match (index.checked_sub(1).map(|i| RENDER_SETS[i].clone()), index) {
                (None, _) => render_app.add_systems(Render, mark.before(RENDER_SETS[0].clone())),
                (Some(previous), i) if i == RENDER_SETS.len() => {
                    render_app.add_systems(Render, mark.after(previous))
                }
                (Some(previous), i) => render_app
                    .add_systems(Render, mark.after(previous).before(RENDER_SETS[i].clone())),
            };
        }
        // The same construction one level down, inside `Prepare`, which is
        // where a loaded frame's milliseconds actually sit. Its own chain
        // rather than more entries in the one above: these sets are nested in
        // `Prepare`, so their times are a BREAKDOWN of that row and must not be
        // added to it.
        //
        // `.in_set(Prepare)` on every one of them, and that is load-bearing:
        // without it the first marker has only a `.before(PrepareResources)`
        // constraint, which the scheduler is free to satisfy at the very top of
        // the `Render` schedule. It did, and the breakdown then charged
        // `Prepare/Resources` with more time than all of `Prepare`.
        for index in 0..=PREPARE_SETS.len() {
            let mark = (move |cost: Res<RenderCost>| cost.mark_prepare(index))
                .in_set(RenderSystems::Prepare);
            match (index.checked_sub(1).map(|i| PREPARE_SETS[i].clone()), index) {
                (None, _) => render_app.add_systems(Render, mark.before(PREPARE_SETS[0].clone())),
                (Some(previous), i) if i == PREPARE_SETS.len() => {
                    render_app.add_systems(Render, mark.after(previous))
                }
                (Some(previous), i) => render_app
                    .add_systems(Render, mark.after(previous).before(PREPARE_SETS[i].clone())),
            };
        }
        render_app.add_systems(
            bevy::render::renderer::RenderGraph,
            (
                (|cost: Res<RenderCost>| cost.graph_open())
                    .before(bevy::render::renderer::RenderGraphSystems::Begin),
                (|cost: Res<RenderCost>| cost.graph_close())
                    .after(bevy::render::renderer::RenderGraphSystems::Finish),
            ),
        );
    }
}

/// The render world's top-level phases, in the order `RenderPlugin` chains
/// them. Named here so the report can label a boundary; the sub-sets inside
/// `Queue` are deliberately left folded into their parent.
///
/// `PrepareAssets` is in the list although it is not in the top-level chain,
/// and it has to be: `RenderPlugin` chains it BETWEEN `ExtractCommands` and
/// `PrepareMeshes`, so a boundary marker that only names those two is
/// ambiguous with it and its time lands in whichever neighbour won the
/// scheduler that frame.
const RENDER_SETS: [RenderSystems; 12] = [
    RenderSystems::ExtractCommands,
    RenderSystems::PrepareAssets,
    RenderSystems::PrepareMeshes,
    RenderSystems::CreateViews,
    RenderSystems::Specialize,
    RenderSystems::PrepareViews,
    RenderSystems::Queue,
    RenderSystems::PhaseSort,
    RenderSystems::Prepare,
    RenderSystems::Render,
    RenderSystems::Cleanup,
    RenderSystems::PostCleanup,
];

/// The sets `RenderPlugin` chains INSIDE `Prepare`. Their times break the
/// `Prepare` row down; they do not add to the render-world total.
const PREPARE_SETS: [RenderSystems; 6] = [
    RenderSystems::PrepareResources,
    RenderSystems::PrepareResourcesBatchPhases,
    RenderSystems::PrepareResourcesWritePhaseBuffers,
    RenderSystems::PrepareResourcesCollectPhaseBuffers,
    RenderSystems::PrepareResourcesFlush,
    RenderSystems::PrepareBindGroups,
];

/// The render world's per-phase tally, shared with the main world.
///
/// A mutex rather than a channel because the two worlds run on different
/// threads under pipelined rendering and the main world only ever reads a
/// snapshot: a lock per boundary is 12 uncontended locks a frame.
#[derive(Resource, Clone)]
struct RenderCost(std::sync::Arc<std::sync::Mutex<RenderTally>>);

/// What [`RenderCost`] accumulates between reports.
///
/// `graph` is carved out of the `Render` phase rather than added to it:
/// `render_system` runs the whole render graph and THEN submits and presents,
/// all inside one system, so "Render minus graph" is the only way to see the
/// submit and the present at all.
#[derive(Default)]
struct RenderTally {
    elapsed: [Duration; RENDER_SETS.len()],
    prepare: [Duration; PREPARE_SETS.len()],
    graph: Duration,
    graph_open: Option<Instant>,
    frames: u32,
    last: Option<(usize, Instant)>,
    last_prepare: Option<(usize, Instant)>,
}

/// Accumulate the span that ended at boundary `index` of one chain.
///
/// `elapsed[i]` is the time between boundary `i` and boundary `i + 1`, so only
/// a boundary that immediately follows the one before it attributes anything -
/// a skipped marker (a set that ran no systems this frame) breaks the chain
/// and is charged to nobody rather than to its neighbour.
fn mark_chain(
    elapsed: &mut [Duration],
    last: &mut Option<(usize, Instant)>,
    index: usize,
    now: Instant,
) -> bool {
    let mut closed = false;
    if let Some((previous, at)) = *last {
        if index == previous + 1 && previous < elapsed.len() {
            elapsed[previous] += now - at;
            closed = index == elapsed.len();
        }
    }
    *last = if index == elapsed.len() {
        None
    } else if index == 0 || last.is_some_and(|(previous, _)| index == previous + 1) {
        Some((index, now))
    } else {
        *last
    };
    closed
}

impl RenderCost {
    fn shared() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(
            RenderTally::default(),
        )))
    }

    fn mark(&self, index: usize) {
        let now = Instant::now();
        let Ok(mut tally) = self.0.lock() else {
            return;
        };
        let RenderTally {
            elapsed,
            frames,
            last,
            ..
        } = &mut *tally;
        if mark_chain(elapsed, last, index, now) {
            *frames += 1;
        }
    }

    /// The same, for the chain nested inside `Prepare`.
    fn mark_prepare(&self, index: usize) {
        let now = Instant::now();
        let Ok(mut tally) = self.0.lock() else {
            return;
        };
        let RenderTally {
            prepare,
            last_prepare,
            ..
        } = &mut *tally;
        mark_chain(prepare, last_prepare, index, now);
    }

    fn graph_open(&self) {
        if let Ok(mut tally) = self.0.lock() {
            tally.graph_open = Some(Instant::now());
        }
    }

    fn graph_close(&self) {
        let now = Instant::now();
        if let Ok(mut tally) = self.0.lock() {
            if let Some(at) = tally.graph_open.take() {
                tally.graph += now - at;
            }
        }
    }

    /// Sum every phase since the last drain, per frame, and reset.
    fn drain(&self) -> Option<RenderReport> {
        let mut tally = self.0.lock().ok()?;
        if tally.frames == 0 {
            return None;
        }
        let frames = f64::from(tally.frames);
        let mut rows = Vec::with_capacity(RENDER_SETS.len() + 1);
        let mut total = 0.0;
        let graph_ms = tally.graph.as_secs_f64() * 1000.0 / frames;
        for (set, elapsed) in RENDER_SETS.iter().zip(tally.elapsed.iter()) {
            let ms = elapsed.as_secs_f64() * 1000.0 / frames;
            total += ms;
            if matches!(set, RenderSystems::Render) {
                rows.push(("Render/graph", graph_ms));
                rows.push(("Render/submit+present", ms - graph_ms));
            } else {
                rows.push((set.name(), ms));
            }
        }
        let prepare = PREPARE_SETS
            .iter()
            .zip(tally.prepare.iter())
            .map(|(set, elapsed)| (set.name(), elapsed.as_secs_f64() * 1000.0 / frames))
            .collect();
        let report = RenderReport {
            frames: tally.frames,
            rows,
            prepare,
            total,
        };
        *tally = RenderTally::default();
        Some(report)
    }
}

/// One drained render-world window: the top-level phase rows, the breakdown of
/// `Prepare`, and the total the top-level rows sum to.
struct RenderReport {
    frames: u32,
    rows: Vec<(&'static str, f64)>,
    prepare: Vec<(&'static str, f64)>,
    total: f64,
}

/// A stable label per render phase. `Debug` on the set would do, but this keeps
/// the report's column width fixed and does not move if bevy renames a variant
/// without renaming the phase.
trait RenderSetName {
    fn name(&self) -> &'static str;
}

impl RenderSetName for RenderSystems {
    fn name(&self) -> &'static str {
        match self {
            RenderSystems::ExtractCommands => "ExtractCommands",
            RenderSystems::PrepareAssets => "PrepareAssets",
            RenderSystems::PrepareMeshes => "PrepareMeshes",
            RenderSystems::CreateViews => "CreateViews",
            RenderSystems::Specialize => "Specialize",
            RenderSystems::PrepareViews => "PrepareViews",
            RenderSystems::Queue => "Queue",
            RenderSystems::PhaseSort => "PhaseSort",
            RenderSystems::Prepare => "Prepare",
            RenderSystems::PrepareResources => "Prepare/Resources",
            RenderSystems::PrepareResourcesBatchPhases => "Prepare/BatchPhases",
            RenderSystems::PrepareResourcesWritePhaseBuffers => "Prepare/WritePhaseBuffers",
            RenderSystems::PrepareResourcesCollectPhaseBuffers => "Prepare/CollectPhaseBuffers",
            RenderSystems::PrepareResourcesFlush => "Prepare/ResourcesFlush",
            RenderSystems::PrepareBindGroups => "Prepare/BindGroups",
            RenderSystems::Render => "Render",
            RenderSystems::Cleanup => "Cleanup",
            other => {
                debug_assert!(matches!(other, RenderSystems::PostCleanup));
                "PostCleanup"
            }
        }
    }
}

/// Per-schedule elapsed time accumulated over the current report window.
#[derive(Resource)]
struct FrameCost {
    names: Vec<String>,
    elapsed: Vec<Duration>,
    frame_total: Duration,
    frames: u32,
    report_frames: u32,
    /// When the previous boundary was crossed, and which one it was.
    last_mark: Option<(usize, Instant)>,
}

impl FrameCost {
    /// Cross boundary `index`: whatever ran since the previous boundary belongs
    /// to the schedule that boundary opened.
    fn mark(&mut self, index: usize) {
        let now = Instant::now();
        if let Some((previous, at)) = self.last_mark {
            if previous < self.elapsed.len() && index == previous + 1 {
                self.elapsed[previous] += now - at;
            }
        }
        self.last_mark = Some((index, now));
    }

    fn close_frame(&mut self, delta: Duration) {
        self.frame_total += delta;
        self.frames += 1;
        // The next frame opens at boundary 0, which is not `end + 1`, so the
        // gap across the frame edge is deliberately attributed to nothing:
        // that gap IS the work outside the main world.
        self.last_mark = None;
    }

    fn reset(&mut self) {
        self.elapsed.iter_mut().for_each(|d| *d = Duration::ZERO);
        self.frame_total = Duration::ZERO;
        self.frames = 0;
    }
}

fn report_when_due(
    mut cost: ResMut<FrameCost>,
    render: Res<RenderCost>,
    diagnostics: Res<DiagnosticsStore>,
) {
    if cost.frames < cost.report_frames {
        return;
    }
    let frames = f64::from(cost.frames);
    let frame_ms = cost.frame_total.as_secs_f64() * 1000.0 / frames;
    let mut main_ms = 0.0;
    let mut rows: Vec<(String, f64)> = Vec::new();
    for (name, elapsed) in cost.names.iter().zip(cost.elapsed.iter()) {
        let ms = elapsed.as_secs_f64() * 1000.0 / frames;
        main_ms += ms;
        rows.push((name.clone(), ms));
    }
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));

    let drained = render.drain();
    let render_ms = drained.as_ref().map_or(0.0, |report| report.total);
    let mut gpu_ms = 0.0;
    let mut passes: Vec<(&str, f64)> = Vec::new();
    let mut counts: Vec<(&str, f64)> = Vec::new();
    for diagnostic in diagnostics.iter() {
        let path = diagnostic.path().as_str();
        if !path.starts_with("render/") {
            continue;
        }
        let Some(value) = diagnostic.average() else {
            continue;
        };
        // A shader-invocation count and a millisecond both arrive as an f64 on
        // the same prefix. Mixing them into one table produced a row reading
        // "921600 ms, 4892313% of the frame", which is how a report stops being
        // read at all.
        if path.ends_with("/elapsed_gpu") {
            gpu_ms += value;
            passes.push((path, value));
        } else if path.ends_with("/elapsed_cpu") {
            passes.push((path, value));
        } else {
            counts.push((path, value));
        }
    }
    passes.sort_by(|a, b| b.1.total_cmp(&a.1));
    counts.sort_by(|a, b| b.1.total_cmp(&a.1));

    info!(
        "nova framecost: frames={} frame={:.3}ms main_world={:.3}ms render_world={:.3}ms gpu={:.3}ms unattributed={:.3}ms",
        cost.frames,
        frame_ms,
        main_ms,
        render_ms,
        gpu_ms,
        frame_ms - main_ms.max(render_ms),
    );
    for (name, ms) in &rows {
        info!(
            "nova framecost main: {ms:>8.3}ms {:>5.1}%  {name}",
            ms / frame_ms * 100.0
        );
    }
    if let Some(report) = drained {
        let render_frames = report.frames;
        let mut render_rows = report.rows;
        render_rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (name, ms) in &render_rows {
            info!(
                "nova framecost render[{render_frames}]: {ms:>8.3}ms {:>5.1}%  {name}",
                ms / frame_ms * 100.0
            );
        }
        // Nested inside the `Prepare` row above, so these are printed under
        // their own prefix and must never be added to the render total.
        let mut prepare_rows = report.prepare;
        prepare_rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (name, ms) in &prepare_rows {
            info!(
                "nova framecost prepare[{render_frames}]: {ms:>8.3}ms {:>5.1}%  {name}",
                ms / frame_ms * 100.0
            );
        }
    }
    for (path, ms) in &passes {
        info!(
            "nova framecost gpu: {ms:>8.3}ms {:>5.1}%  {path}",
            ms / frame_ms * 100.0
        );
    }
    for (path, value) in &counts {
        info!("nova framecost count: {value:>12.0}  {path}");
    }
    if passes.is_empty() && std::env::var_os(nova_core::RENDER_DIAG_ENV).is_some() {
        warn!("nova framecost: render diagnostics armed but the store holds no render/* readings");
    }

    cost.reset();
}
