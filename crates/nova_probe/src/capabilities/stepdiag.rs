//! Per-step physics diagnostics: one CSV row per fixed step, plus the
//! BODY-COUNT REGIME the summary is taken over.
//!
//! Change this module when a fixed-step hypothesis needs a number the frame
//! clock cannot give. A whole-run average over an arena is not comparable
//! between two arms, because a faster simulation ends the fight sooner and
//! quietly measures a lighter scene; selecting the steps by how many dynamic
//! bodies were live is what makes the two arms describe the same workload.

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

use avian3d::{
    collision::CollisionDiagnostics, dynamics::solver::SolverDiagnostics, prelude::*,
    spatial_query::SpatialQueryDiagnostics,
};
use bevy::{
    app::{FixedFirst, FixedLast},
    prelude::*,
};

use crate::{capabilities::frametime::prelude::*, stats::prelude::*};

/// Logical parameter naming the CSV the per-step rows are written to
/// (`NOVA_PROBE_STEPDIAG` natively). Unset, the plugin adds nothing.
pub const STEPDIAG_PARAM: &str = "stepdiag";

/// Logical parameter naming the REGIME floor: the fewest live dynamic bodies a
/// step must have carried to be counted in the summary
/// (`NOVA_PROBE_STEPDIAG_BODIES` natively).
pub const STEPDIAG_BODIES_PARAM: &str = "stepdiag_bodies";

/// Regime floor when none is named: every recorded step counts.
///
/// Zero rather than a guess, because the floor that separates a fight from its
/// approach is a property of the SCENE - the fixed-step investigation used 500
/// bodies for a 1v1 arena, 800 for a 4v4 and 1500 for a saturated point-defense
/// range, and no single number serves all three.
pub const DEFAULT_REGIME_BODIES: u32 = 0;

/// The CSV header, and the order `record_step` writes its columns in.
const HEADER: &str = "step,wall_ms,broad_ms,narrow_ms,contacts,constraints,\
                      prepare_ms,solve_ms,finalize_ms,spatial_ms,dynamic_bodies,colliders";

/// Per-step physics diagnostics, written to the CSV named by
/// [`STEPDIAG_PARAM`].
///
/// Inert unless that parameter names a path, so an example may add it
/// permanently and an ordinary run pays nothing.
pub fn nova_stepdiag() -> StepDiagPlugin {
    StepDiagPlugin {
        out: None,
        regime_bodies: None,
    }
}

/// Writes one CSV row per fixed step - avian's own phase timings plus the
/// step's wall time and the world's body and collider counts - and logs a
/// summary over the steps inside the body-count regime when the run ends.
#[derive(Clone, Default)]
pub struct StepDiagPlugin {
    out: Option<PathBuf>,
    regime_bodies: Option<u32>,
}

impl StepDiagPlugin {
    /// Write the rows here instead of to the path [`STEPDIAG_PARAM`] names.
    pub fn out(mut self, path: impl Into<PathBuf>) -> Self {
        self.out = Some(path.into());
        self
    }

    /// Summarise only the steps that carried at least this many live dynamic
    /// bodies. Overridden by [`STEPDIAG_BODIES_PARAM`] when that is set, so a
    /// sweep can move the floor without editing the scene.
    pub fn regime_bodies(mut self, bodies: u32) -> Self {
        self.regime_bodies = Some(bodies);
        self
    }
}

impl Plugin for StepDiagPlugin {
    fn build(&self, app: &mut App) {
        let Some(path) = self
            .out
            .clone()
            .or_else(|| probe_param(STEPDIAG_PARAM).map(PathBuf::from))
        else {
            return;
        };
        let regime_bodies = probe_param(STEPDIAG_BODIES_PARAM)
            .and_then(|value| value.trim().parse().ok())
            .or(self.regime_bodies)
            .unwrap_or(DEFAULT_REGIME_BODIES);
        let sink = match StepDiag::create(path, regime_bodies) {
            Ok(sink) => sink,
            Err(error) => {
                // ERROR, not WARN, for the reason the timeline sink uses one:
                // the CSV was explicitly asked for, and a run that silently
                // records nothing looks exactly like a run that measured.
                error!("nova probe: step diagnostics disabled: {error}");
                return;
            }
        };
        info!(
            "nova probe: step diagnostics armed -> {:?} (regime bodies >= {regime_bodies})",
            sink.path
        );
        app.insert_resource(sink);
        app.init_resource::<StepStamp>();
        app.add_systems(FixedFirst, stamp_step);
        app.add_systems(FixedLast, record_step);
        // The summary is a RUN-END line, ordered behind the exit write for the
        // same reason the timeline's is: bevy exits after the frame `AppExit`
        // is written, so an unordered reader never sees it.
        crate::capabilities::timeline::order_run_end(app);
        app.add_systems(
            Last,
            summarise_regime.in_set(crate::capabilities::timeline::ProbeRecorderSystems::RunEnd),
        );
    }
}

/// The wall-clock stamp taken at the top of the fixed step, read back at its
/// bottom. `None` between runs and before the first step.
#[derive(Resource, Default)]
struct StepStamp(Option<Instant>);

/// The open CSV plus the regime tallies the summary is taken over.
#[derive(Resource)]
struct StepDiag {
    sink: BufWriter<File>,
    path: PathBuf,
    steps: u64,
    /// Fewest live dynamic bodies a step must carry to join the regime.
    regime_bodies: u32,
    /// Wall time (ms) of every step inside the regime.
    regime_wall_ms: Vec<f64>,
    /// Live dynamic bodies summed over the regime's steps, for its mean.
    regime_bodies_total: u64,
    /// Contact constraints summed over the regime's steps, for its mean.
    regime_constraints_total: u64,
    summarised: bool,
}

impl StepDiag {
    fn create(path: PathBuf, regime_bodies: u32) -> Result<Self, String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
        // Opened without truncating and then `set_len(0)`, never `File::create`:
        // a plain create truncates to offset 0 while an earlier writer's
        // `BufWriter` keeps writing at its own, splicing two streams together.
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("could not create {}: {e}", path.display()))?;
        file.set_len(0)
            .map_err(|e| format!("could not truncate {}: {e}", path.display()))?;
        let mut sink = BufWriter::new(file);
        writeln!(sink, "{HEADER}").map_err(|e| format!("could not write the header: {e}"))?;
        Ok(Self {
            sink,
            path,
            steps: 0,
            regime_bodies,
            regime_wall_ms: Vec::new(),
            regime_bodies_total: 0,
            regime_constraints_total: 0,
            summarised: false,
        })
    }
}

/// Milliseconds, the unit every column of the CSV reports time in.
fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn stamp_step(mut stamp: ResMut<StepStamp>) {
    stamp.0 = Some(Instant::now());
}

/// One row per fixed step.
///
/// The diagnostics resources are `Option`: avian registers them in its
/// `Plugin::finish`, so an app that wires this plugin without `PhysicsPlugins`
/// has none of them and must not panic for it.
#[expect(
    clippy::too_many_arguments,
    reason = "one system param per CSV column group"
)]
fn record_step(
    mut diag: ResMut<StepDiag>,
    mut stamp: ResMut<StepStamp>,
    collision: Option<Res<CollisionDiagnostics>>,
    solver: Option<Res<SolverDiagnostics>>,
    spatial: Option<Res<SpatialQueryDiagnostics>>,
    bodies: Query<&RigidBody>,
    colliders: Query<(), With<Collider>>,
) {
    let (Some(collision), Some(solver)) = (collision, solver) else {
        return;
    };
    let wall_ms = stamp
        .0
        .take()
        .map_or(0.0, |start| start.elapsed().as_secs_f64() * 1000.0);
    // The solver reports its passes separately; the step's solve cost is their
    // sum. `swept_ccd` is deliberately out: it is off unless a body asks for
    // it, so folding it in would make two scenes incomparable.
    let solve_ms = ms(solver.update_velocity_increments)
        + ms(solver.integrate_velocities)
        + ms(solver.warm_start)
        + ms(solver.solve_constraints)
        + ms(solver.integrate_positions)
        + ms(solver.relax_velocities)
        + ms(solver.apply_restitution);
    let spatial_ms = spatial.map_or(0.0, |spatial| {
        ms(spatial.update_ray_casters) + ms(spatial.update_shape_casters)
    });
    let dynamic_bodies = bodies.iter().filter(|body| body.is_dynamic()).count() as u32;
    let colliders = colliders.iter().count();

    diag.steps += 1;
    let step = diag.steps;
    if dynamic_bodies >= diag.regime_bodies {
        diag.regime_wall_ms.push(wall_ms);
        diag.regime_bodies_total += u64::from(dynamic_bodies);
        diag.regime_constraints_total += u64::from(solver.contact_constraint_count);
    }
    if let Err(error) = writeln!(
        diag.sink,
        "{step},{wall_ms:.4},{:.4},{:.4},{},{},{:.4},{solve_ms:.4},{:.4},{spatial_ms:.4},\
         {dynamic_bodies},{colliders}",
        ms(collision.broad_phase),
        ms(collision.narrow_phase),
        collision.contact_count,
        solver.contact_constraint_count,
        ms(solver.prepare_constraints),
        ms(solver.finalize) + ms(solver.store_impulses),
    ) {
        warn!("nova probe: step diagnostics write failed: {error}");
    }
}

/// Flush the CSV and log the regime summary once, as the run exits.
///
/// The summary is the point of the capability: a mean over the WHOLE run is
/// not comparable between two arms, because the faster one ends the fight
/// sooner and averages in a lighter scene. Reporting the regime's own step
/// count and mean body count beside the percentiles is what lets a reader see
/// that the two arms really did measure the same weight of world.
fn summarise_regime(mut exits: MessageReader<AppExit>, mut diag: ResMut<StepDiag>) {
    if diag.summarised || exits.read().next().is_none() {
        return;
    }
    diag.summarised = true;
    if let Err(error) = diag.sink.flush() {
        warn!("nova probe: step diagnostics flush failed: {error}");
    }
    let (steps, floor, path) = (diag.steps, diag.regime_bodies, diag.path.clone());
    if diag.regime_wall_ms.is_empty() {
        warn!(
            "nova probe: step diagnostics {path:?}: {steps} steps recorded, none at or above \
             {floor} dynamic bodies - the regime floor selected nothing"
        );
        return;
    }
    let stats = FrameStats::from_samples(&diag.regime_wall_ms);
    let in_regime = diag.regime_wall_ms.len() as u64;
    info!(
        "nova probe: step diagnostics {path:?}: {in_regime}/{steps} steps in regime \
         (bodies >= {floor}), mean_bodies={:.0} mean_constraints={:.0} \
         mean={:.3}ms p50={:.3}ms p95={:.3}ms p99={:.3}ms max={:.3}ms",
        diag.regime_bodies_total as f64 / in_regime as f64,
        diag.regime_constraints_total as f64 / in_regime as f64,
        stats.mean_ms,
        stats.p50_ms,
        stats.p95_ms,
        stats.p99_ms,
        stats.max_ms,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header names every column the row writer emits, in order. Two
    /// lists that drift silently mislabel every number in the file.
    #[test]
    fn the_header_names_one_column_per_written_field() {
        assert_eq!(HEADER.split(',').count(), 12);
        assert!(HEADER.starts_with("step,wall_ms,"));
        assert!(HEADER.ends_with(",dynamic_bodies,colliders"));
    }

    /// The floor is a property of the SCENE, so the default must select
    /// everything rather than guess one scene's fight threshold.
    #[test]
    fn the_default_regime_floor_selects_every_step() {
        assert_eq!(DEFAULT_REGIME_BODIES, 0);
    }
}

/// The per-step diagnostics plugin, its preset and its parameter names.
pub mod prelude {
    pub use super::{
        nova_stepdiag, StepDiagPlugin, DEFAULT_REGIME_BODIES, STEPDIAG_BODIES_PARAM, STEPDIAG_PARAM,
    };
}
