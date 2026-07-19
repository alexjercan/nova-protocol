//! Continuous invariant assertions over a running app: always-true checks
//! evaluated every frame, whose violations are structured evidence of a bug -
//! the second correctness layer of the run-harness (spike
//! tasks/20260719-112011/SPIKE.md, task 20260719-114931), complementing the
//! run-timeline recorder: the recorder shows what happened, the invariants
//! flag what must NEVER happen. Immune to host timing noise by construction
//! (they assert state bounds, not schedules).
//!
//! One env-gated plugin, [`nova_invariants`]: inert unless
//! `NOVA_PERF_INVARIANTS` is set. Any value arms record+warn mode; the value
//! `strict` additionally PANICS on the first violation (turning a corrupted
//! state into a loud harness failure at the moment of corruption). Each
//! violation logs a `warn!` and, when the run-timeline recorder is armed,
//! lands on the timeline as a `kind: "invariant"` entry; the
//! [`InvariantState`] resource keeps the running count for the report's
//! `invariants held` check.
//!
//! The v1 set asserts only what the ENGINE guarantees
//! (rule-inputs-rederive-from-engine):
//!
//! - **Health bounds**: every [`Health`] holds finite values with
//!   `0 <= current <= max` - bcs's `on_damage` clamps to exactly this, so a
//!   violation means some code path bypassed the clamp.
//! - **Velocity sanity**: every avian [`LinearVelocity`] is finite (NaN =
//!   the physics exploded). When the entity carries [`FlightSpeedCap`], the
//!   speed must stay under `cap * `[`SPEED_SANITY_MULTIPLIER`] - the cap
//!   itself is a SOFT taper gate (manual burns taper toward it; autopilot
//!   maneuvers and gravity wells legitimately exceed it), so this is
//!   absurdity detection, not cap enforcement.
//! - **Scenario variables**: every Number variable is finite, and variables
//!   REGISTERED as monotonic never decrease. Monotonicity is scenario-script
//!   discipline, not a type guarantee, so it is opt-in via
//!   [`monotonic`](InvariantsPlugin::monotonic) - never inferred.
//! - **Entity-count sanity**: the world's total entity count stays under
//!   [`ENTITY_SANITY_CAP`] (a leak detector; no gameplay cap exists).
//!
//! Deliberately NOT in v1: the ship-root-equals-section-sum health aggregate
//! (mid-despawn frames make it schedule-flaky) and projectile lifetimes
//! (the DespawnEntityPlugin's own contract, pinned in bcs).

use std::collections::HashMap;

use avian3d::prelude::LinearVelocity;
use bevy::{diagnostic::FrameCount, prelude::*};
use nova_gameplay::{bevy_common_systems::health::Health, flight::FlightSpeedCap};
use nova_scenario::{variables::VariableLiteral, world::NovaEventWorld};

use crate::{
    capture::perf_param,
    recorder::{stamp, ProbeTimeline, TimelineEvent},
};

/// Env value (via [`perf_param`], so `NOVA_PERF_INVARIANTS` on native) that
/// arms the checks; the value `strict` also panics on the first violation.
pub const INVARIANTS_PARAM: &str = "invariants";

/// A [`FlightSpeedCap`] is a soft taper gate, not a clamp: autopilot
/// maneuvers, gravity wells and RCS diagonals all legitimately exceed it.
/// The invariant only flags ABSURD speeds - beyond this multiple of the cap
/// nothing in the engine can honestly produce the value.
pub const SPEED_SANITY_MULTIPLIER: f32 = 10.0;

/// Leak detector: no shipped scene approaches this entity count (heavy
/// combat scenes run in the low thousands); crossing it means something is
/// spawning without despawning.
pub const ENTITY_SANITY_CAP: u32 = 200_000;

/// Env-gated continuous-invariants preset. Inert unless `NOVA_PERF_INVARIANTS`
/// is set (or [`strict`](InvariantsPlugin::strict) is called, which tests use
/// to avoid process-global env races). See the module docs for the v1 set.
pub fn nova_invariants() -> InvariantsPlugin {
    InvariantsPlugin {
        armed_override: None,
        strict_override: None,
        monotonic: Vec::new(),
    }
}

/// Plugin returned by [`nova_invariants`]. Construct it through that preset.
pub struct InvariantsPlugin {
    armed_override: Option<bool>,
    strict_override: Option<bool>,
    monotonic: Vec<String>,
}

impl InvariantsPlugin {
    /// Force armed/strict regardless of the env (tests; env vars are
    /// process-global and race across parallel tests). `strict = true`
    /// panics on the first violation.
    pub fn strict(mut self, strict: bool) -> Self {
        self.armed_override = Some(true);
        self.strict_override = Some(strict);
        self
    }

    /// Register scenario variables whose values must never decrease.
    /// Monotonicity is script discipline (any action can SET any value), so
    /// the caller declares which variables the SCENARIO's design promises
    /// only ever go up - e.g. a beat counter or a kill tally.
    pub fn monotonic<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.monotonic.extend(keys.into_iter().map(Into::into));
        self
    }
}

impl Plugin for InvariantsPlugin {
    fn build(&self, app: &mut App) {
        let param = perf_param(INVARIANTS_PARAM);
        let armed = self.armed_override.unwrap_or(param.is_some());
        if !armed {
            return;
        }
        let strict = self
            .strict_override
            .unwrap_or_else(|| param.as_deref() == Some("strict"));
        info!(
            "nova probe: invariants armed (strict={strict}, monotonic={:?})",
            self.monotonic
        );
        app.insert_resource(InvariantState {
            strict,
            monotonic_keys: self.monotonic.clone(),
            monotonic_last: HashMap::new(),
            checks: 0,
            violations: 0,
        });
        // In Last, BEFORE the recorder's variable-diff + run_end chain, so
        // the exit frame's violations land on the timeline before the
        // bracket closes; the summary follows the checks so its tally
        // includes the exit frame. (before() a system the app may not have -
        // the recorder can be unarmed - is fine: ordering against an absent
        // system is a no-op.)
        app.add_systems(
            Last,
            (check_invariants, record_invariant_summary)
                .chain()
                .before(crate::recorder::record_variable_changes),
        );
    }
}

/// On the first `AppExit`, write one `invariant_summary` timeline entry
/// (frames checked, total violations) so the report reads the verdict off
/// the timeline itself, right before the recorder's `run_end` bracket.
fn record_invariant_summary(
    mut exits: MessageReader<AppExit>,
    time: Res<Time<Real>>,
    frame: Res<FrameCount>,
    scenario: Option<Res<NovaEventWorld>>,
    state: Res<InvariantState>,
    timeline: Option<ResMut<ProbeTimeline>>,
) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(mut timeline) = timeline else {
        return;
    };
    let (t_real, frame, scenario_elapsed) = stamp(&time, &frame, scenario.as_deref());
    timeline.record(TimelineEvent {
        t_real,
        frame,
        scenario_elapsed,
        kind: "invariant_summary".to_string(),
        name: "invariants".to_string(),
        data: serde_json::json!({
            "checks": state.checks,
            "violations": state.violations,
        }),
    });
}

/// Armed-invariants state: configuration plus the running tallies the
/// report's `invariants held` check reads.
#[derive(Resource)]
pub struct InvariantState {
    strict: bool,
    monotonic_keys: Vec<String>,
    /// Last seen value per registered monotonic variable.
    monotonic_last: HashMap<String, f64>,
    /// Frames checked.
    pub checks: u64,
    /// Total violations recorded.
    pub violations: u64,
}

/// One violation, on its way to the log/timeline/panic.
struct Violation {
    name: &'static str,
    data: serde_json::Value,
}

/// The whole v1 check pass, one exclusive system so it reads a settled world
/// (Last: physics, damage, scenario writes for the frame are done).
fn check_invariants(world: &mut World) {
    let mut violations: Vec<Violation> = Vec::new();

    // (a) Health bounds: finite, 0 <= current <= max.
    {
        let mut healths = world.query::<(Entity, &Health)>();
        for (entity, health) in healths.iter(world) {
            let ok = health.current.is_finite()
                && health.max.is_finite()
                && health.current >= 0.0
                && health.current <= health.max;
            if !ok {
                violations.push(Violation {
                    name: "health_bounds",
                    data: serde_json::json!({
                        "entity": format!("{entity:?}"),
                        "current": health.current,
                        "max": health.max,
                    }),
                });
            }
        }
    }

    // (b) Velocity sanity: finite always; absurd-speed vs a soft cap.
    {
        let mut velocities = world.query::<(Entity, &LinearVelocity, Option<&FlightSpeedCap>)>();
        for (entity, velocity, cap) in velocities.iter(world) {
            if !velocity.0.is_finite() {
                violations.push(Violation {
                    name: "velocity_finite",
                    data: serde_json::json!({
                        "entity": format!("{entity:?}"),
                        "velocity": format!("{:?}", velocity.0),
                    }),
                });
                continue;
            }
            if let Some(cap) = cap {
                let speed = velocity.0.length();
                let bound = cap.0 * SPEED_SANITY_MULTIPLIER;
                if cap.0 > 0.0 && speed > bound {
                    violations.push(Violation {
                        name: "speed_sanity",
                        data: serde_json::json!({
                            "entity": format!("{entity:?}"),
                            "speed": speed,
                            "cap": cap.0,
                            "bound": bound,
                        }),
                    });
                }
            }
        }
    }

    // (c) Scenario variables: Numbers finite; registered monotonic keys
    // never decrease.
    {
        let monotonic_keys = world.resource::<InvariantState>().monotonic_keys.clone();
        let mut monotonic_seen: Vec<(String, f64)> = Vec::new();
        if let Some(scenario) = world.get_resource::<NovaEventWorld>() {
            for (key, value) in scenario.variables() {
                if let VariableLiteral::Number(n) = value {
                    if !n.is_finite() {
                        violations.push(Violation {
                            name: "variable_finite",
                            data: serde_json::json!({ "variable": key, "value": format!("{n}") }),
                        });
                        continue;
                    }
                    if monotonic_keys.iter().any(|k| k == key) {
                        monotonic_seen.push((key.clone(), *n));
                    }
                }
            }
        }
        let state = world.resource_mut::<InvariantState>();
        let state = state.into_inner();
        for (key, value) in monotonic_seen {
            if let Some(last) = state.monotonic_last.get(&key) {
                if value < *last {
                    violations.push(Violation {
                        name: "monotonic_regression",
                        data: serde_json::json!({
                            "variable": key,
                            "last": last,
                            "now": value,
                        }),
                    });
                }
            }
            state.monotonic_last.insert(key, value);
        }
    }

    // (d) Entity-count sanity (leak detector).
    {
        let count = world.entities().len();
        if count > ENTITY_SANITY_CAP {
            violations.push(Violation {
                name: "entity_count_sanity",
                data: serde_json::json!({ "entities": count, "cap": ENTITY_SANITY_CAP }),
            });
        }
    }

    // Deliver: count, warn, timeline, then (strict) panic - in that order,
    // so even a strict run flushes the evidence before dying.
    world.resource_mut::<InvariantState>().checks += 1;
    if violations.is_empty() {
        return;
    }
    let strict = {
        let mut state = world.resource_mut::<InvariantState>();
        state.violations += violations.len() as u64;
        state.strict
    };
    let (t_real, frame, scenario_elapsed) = stamp(
        &world.resource::<Time<Real>>().clone(),
        world.resource::<FrameCount>(),
        world.get_resource::<NovaEventWorld>(),
    );
    for violation in &violations {
        warn!(
            "nova probe: INVARIANT VIOLATION {}: {}",
            violation.name, violation.data
        );
        if let Some(mut timeline) = world.get_resource_mut::<ProbeTimeline>() {
            timeline.record(TimelineEvent {
                t_real,
                frame,
                scenario_elapsed,
                kind: "invariant".to_string(),
                name: violation.name.to_string(),
                data: violation.data.clone(),
            });
        }
    }
    if strict {
        panic!(
            "nova probe: {} invariant violation(s) in strict mode; first: {} {}",
            violations.len(),
            violations[0].name,
            violations[0].data
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    fn temp_timeline() -> std::path::PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "nova_probe_invariants_{}_{n}.jsonl",
            std::process::id()
        ))
    }

    /// Armed, non-strict rig (no env dependence).
    fn rig() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(nova_invariants().strict(false));
        app
    }

    fn violations(app: &App) -> u64 {
        app.world().resource::<InvariantState>().violations
    }

    #[test]
    fn healthy_world_records_zero_violations() {
        let mut app = rig();
        app.world_mut().spawn(Health {
            current: 50.0,
            max: 100.0,
        });
        app.world_mut()
            .spawn(LinearVelocity(Vec3::new(3.0, 0.0, 0.0)));
        app.update();
        app.update();
        assert_eq!(violations(&app), 0);
        assert!(app.world().resource::<InvariantState>().checks >= 2);
    }

    #[test]
    fn negative_and_overfull_health_violate_bounds() {
        let mut app = rig();
        app.world_mut().spawn(Health {
            current: -1.0,
            max: 100.0,
        });
        app.world_mut().spawn(Health {
            current: 150.0,
            max: 100.0,
        });
        app.world_mut().spawn(Health {
            current: f32::NAN,
            max: 100.0,
        });
        app.update();
        assert_eq!(violations(&app), 3);
    }

    #[test]
    fn nan_velocity_and_absurd_speed_violate() {
        let mut app = rig();
        app.world_mut()
            .spawn(LinearVelocity(Vec3::new(f32::NAN, 0.0, 0.0)));
        // 25 u/s cap: 10x bound is 250; 300 is absurd, 100 is fine.
        app.world_mut().spawn((
            LinearVelocity(Vec3::new(300.0, 0.0, 0.0)),
            FlightSpeedCap(25.0),
        ));
        app.world_mut().spawn((
            LinearVelocity(Vec3::new(100.0, 0.0, 0.0)),
            FlightSpeedCap(25.0),
        ));
        app.update();
        assert_eq!(violations(&app), 2, "NaN + absurd violate; 4x cap does not");
    }

    #[test]
    fn monotonic_regression_violates_but_increase_does_not() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(nova_invariants().strict(false).monotonic(["beat"]));
        app.init_resource::<NovaEventWorld>();
        let set = |app: &mut App, v: f64| {
            app.world_mut()
                .resource_mut::<NovaEventWorld>()
                .insert_variable("beat".to_string(), VariableLiteral::Number(v));
        };
        set(&mut app, 0.0);
        app.update();
        set(&mut app, 1.0);
        app.update();
        set(&mut app, 2.0);
        app.update();
        assert_eq!(violations(&app), 0, "increasing beat is clean");
        set(&mut app, 1.0);
        app.update();
        assert_eq!(violations(&app), 1, "regression 2 -> 1 violates");
        // An UNREGISTERED variable may move freely.
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("free".to_string(), VariableLiteral::Number(5.0));
        app.update();
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("free".to_string(), VariableLiteral::Number(1.0));
        app.update();
        assert_eq!(violations(&app), 1, "unregistered variables are free-form");
    }

    #[test]
    fn nan_scenario_variable_violates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(nova_invariants().strict(false));
        app.init_resource::<NovaEventWorld>();
        app.world_mut()
            .resource_mut::<NovaEventWorld>()
            .insert_variable("bad".to_string(), VariableLiteral::Number(f64::NAN));
        app.update();
        assert_eq!(violations(&app), 1);
    }

    #[test]
    fn violations_land_on_the_timeline_when_recorder_is_armed() {
        let path = temp_timeline();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::recorder::nova_timeline().out(path.clone()));
        app.add_plugins(nova_invariants().strict(false));
        app.world_mut().spawn(Health {
            current: -5.0,
            max: 10.0,
        });
        app.update();

        let entries = crate::recorder::parse_timeline(
            &std::fs::read_to_string(&path).expect("timeline exists"),
        )
        .expect("timeline parses");
        let violation = entries
            .iter()
            .find(|e| e.kind == "invariant" && e.name == "health_bounds")
            .expect("violation on the timeline");
        assert_eq!(violation.data["current"], -5.0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn summary_entry_lands_before_run_end_on_exit() {
        let path = temp_timeline();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(crate::recorder::nova_timeline().out(path.clone()));
        app.add_plugins(nova_invariants().strict(false));
        app.world_mut().spawn(Health {
            current: -1.0,
            max: 1.0,
        });
        app.update();
        app.world_mut().write_message(AppExit::Success);
        app.update();

        let entries = crate::recorder::parse_timeline(
            &std::fs::read_to_string(&path).expect("timeline exists"),
        )
        .expect("timeline parses");
        let summary_at = entries
            .iter()
            .position(|e| e.kind == "invariant_summary")
            .expect("summary written on exit");
        let end_at = entries
            .iter()
            .position(|e| e.kind == "run_end")
            .expect("run_end written");
        assert!(summary_at < end_at, "summary precedes the bracket close");
        let summary = &entries[summary_at];
        assert!(summary.data["checks"].as_u64().unwrap() >= 2);
        assert!(summary.data["violations"].as_u64().unwrap() >= 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[should_panic(expected = "invariant violation(s) in strict mode")]
    fn strict_mode_panics_on_violation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(nova_invariants().strict(true));
        app.world_mut().spawn(Health {
            current: -1.0,
            max: 1.0,
        });
        app.update();
    }

    #[test]
    fn unarmed_plugin_is_a_no_op() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // No env override, no NOVA_PERF_INVARIANTS in the test env: inert.
        app.add_plugins(InvariantsPlugin {
            armed_override: Some(false),
            strict_override: None,
            monotonic: Vec::new(),
        });
        app.update();
        assert!(app.world().get_resource::<InvariantState>().is_none());
    }
}
