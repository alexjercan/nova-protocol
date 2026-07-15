//! Baseline + regression benchmark for the modding scenario-dispatch hot path.
//!
//! Task 20260714-083331 ("measure before optimizing"). The built-in scenarios
//! are tiny (1-19 handlers, microsecond costs), so this drives a *synthetic*
//! large-mod scenario - hundreds of handlers, dense per-frame `OnUpdate`
//! filters/conditions - to prove where the time actually goes before we touch
//! any of the seeded optimizations (20260525-133014 dispatch index,
//! 20260714-083339 filter interning + condition caching).
//!
//! Three groups, from micro to macro:
//!
//! * `filter_entity`   - one `EntityFilterConfig::filter` (the per-field string
//!   equality + `serde_json` map lookups hit every frame by entity filters).
//! * `condition_eval`  - one `VariableConditionNode::evaluate` (the recursive
//!   `VariableLiteral`-cloning tree walk hit every frame by expression filters).
//! * `dispatch/*`      - the whole `GameEventsPlugin` loop for one `OnUpdate`
//!   frame against N handlers, most of them for *other* event names (so the
//!   linear O(all handlers) name scan in `bevy-common-systems` is exercised).
//!
//! Run with `cargo bench -p nova_scenario`; HTML lands in
//! `target/criterion/`. To profile: `samply record -- cargo bench -p
//! nova_scenario --bench scenario_dispatch -- --profile-time 10`.

use std::hint::black_box;

use bevy::prelude::*;
use bevy_common_systems::{modding::events::GameEventQueue, prelude::*};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use nova_events::prelude::*;
use nova_scenario::prelude::*;

/// Every non-`OnUpdate` event kind, so the synthetic scenario can pad itself
/// with handlers that the `OnUpdate` frame must scan past but never name-match.
const OTHER_EVENTS: [EventConfig; 7] = [
    EventConfig::OnStart,
    EventConfig::OnDestroyed,
    EventConfig::OnEnter,
    EventConfig::OnExit,
    EventConfig::OnOrbit,
    EventConfig::OnTravelLock,
    EventConfig::OnCombatLock,
];

/// A representative per-frame expression filter: `progress > 0.5`. This is the
/// shape shakedown's milestone `OnUpdate` handlers use - a variable compared to
/// a literal - and the recursion the condition-eval optimization targets.
fn progress_condition() -> VariableConditionNode {
    VariableConditionNode::new_greater_than(
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name("progress"),
        )),
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(0.5)),
        )),
    )
}

/// A deeper condition exercising the full AST: `(progress * 2 + bonus) > (limit - 1)`.
/// The variable AST exists to express nested arithmetic/comparison like this, so
/// this is the worst case the condition-eval optimization would target - several
/// Boxed nodes and variable lookups per evaluation, not the single lookup of
/// `progress_condition`.
fn nested_condition() -> VariableConditionNode {
    // progress * 2 + bonus
    let left = VariableExpressionNode::new_add(
        VariableTermNode::new_multiply(
            VariableFactorNode::new_name("progress"),
            VariableTermNode::new_factor(VariableFactorNode::new_literal(VariableLiteral::Number(
                2.0,
            ))),
        ),
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_name("bonus"),
        )),
    );
    // limit - 1
    let right = VariableExpressionNode::new_subtract(
        VariableTermNode::new_factor(VariableFactorNode::new_name("limit")),
        VariableExpressionNode::new_term(VariableTermNode::new_factor(
            VariableFactorNode::new_literal(VariableLiteral::Number(1.0)),
        )),
    );
    VariableConditionNode::new_greater_than(left, right)
}

fn expression_filter() -> EventFilterConfig {
    EventFilterConfig::Expression(ExpressionFilterConfig(progress_condition()))
}

/// A representative entity filter: match a named entity of a given type. This is
/// the per-field string-equality path the interning optimization targets.
fn entity_filter() -> EntityFilterConfig {
    EntityFilterConfig {
        id: Some("player".to_string()),
        type_name: Some("ship".to_string()),
        ..Default::default()
    }
}

/// Event info carrying the entity fields the entity filter reads.
fn matching_info() -> GameEventInfo {
    GameEventInfo {
        data: Some(serde_json::json!({ "id": "player", "type": "ship" })),
    }
}

/// Build a headless app with `GameEventsPlugin` and `total` spawned handlers,
/// one tenth of them per-frame `OnUpdate` expression filters and the rest
/// spread across other event names (dead weight the name scan pays for).
fn build_dispatch_app(total: usize) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
    // NovaEventWorld::state_to_world_system reads GameObjectives every frame.
    app.init_resource::<GameObjectives>();

    app.world_mut()
        .resource_mut::<NovaEventWorld>()
        .insert_variable("progress".to_string(), VariableLiteral::Number(0.75));

    let onupdate = (total / 10).max(1);
    for i in 0..total {
        if i < onupdate {
            let mut handler = EventHandler::<NovaEventWorld>::from(EventConfig::OnUpdate);
            handler.add_filter(expression_filter());
            app.world_mut().spawn(handler);
        } else {
            let cfg = OTHER_EVENTS[i % OTHER_EVENTS.len()];
            app.world_mut()
                .spawn(EventHandler::<NovaEventWorld>::from(cfg));
        }
    }

    // One warm-up frame to init schedules/systems (also drains the initial
    // resource_changed run) so the timed frames measure only steady-state work.
    app.update();
    app
}

/// Push `count` `OnUpdate` events into the queue. `count == 1` is the realistic
/// per-frame pulse; a larger batch is drained in a single `queue_system` call,
/// which amortizes the fixed per-`update()` frame overhead so the dispatch scan
/// itself (the O(handlers) cost the index targets) becomes the dominant signal.
fn queue_on_update_n(app: &mut App, count: usize) {
    let name = <OnUpdateEvent as EventKind>::name();
    let mut queue = app
        .world_mut()
        .resource_mut::<GameEventQueue<NovaEventWorld>>();
    for _ in 0..count {
        queue
            .events
            .push_back(GameEvent::new(name, GameEventInfo::default()));
    }
}

fn bench_filter_entity(c: &mut Criterion) {
    let world = NovaEventWorld::default();
    let filter = entity_filter();
    let hit = matching_info();
    let miss = GameEventInfo {
        data: Some(serde_json::json!({ "id": "enemy", "type": "ship" })),
    };

    let mut group = c.benchmark_group("filter_entity");
    group.bench_function("match", |b| {
        b.iter(|| black_box(filter.filter(black_box(&world), black_box(&hit))))
    });
    group.bench_function("reject", |b| {
        b.iter(|| black_box(filter.filter(black_box(&world), black_box(&miss))))
    });
    group.finish();
}

fn bench_condition_eval(c: &mut Criterion) {
    let mut world = NovaEventWorld::default();
    world.insert_variable("progress".to_string(), VariableLiteral::Number(0.75));
    world.insert_variable("bonus".to_string(), VariableLiteral::Number(0.2));
    world.insert_variable("limit".to_string(), VariableLiteral::Number(1.0));
    let simple = progress_condition();
    let nested = nested_condition();

    let mut group = c.benchmark_group("condition_eval");
    // Trivial `var > literal` - the floor, the shape today's milestone checks use.
    group.bench_function("greater_than", |b| {
        b.iter(|| black_box(simple.evaluate(black_box(&world))))
    });
    // `(progress * 2 + bonus) > (limit - 1)` - the full-AST worst case.
    group.bench_function("nested", |b| {
        b.iter(|| black_box(nested.evaluate(black_box(&world))))
    });
    group.finish();
}

/// Realistic per-frame dispatch: one `OnUpdate` event against N handlers. At
/// these scales the fixed frame overhead dominates, so this mostly shows the
/// (near-flat) marginal cost of the linear name scan per extra handler.
fn bench_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dispatch");
    for total in [50_usize, 200, 500, 2000] {
        // Build once: handlers have no actions, so the world never mutates and
        // the queue fully drains each frame - the app is a stable steady state
        // reusable across every timed iteration.
        let mut app = build_dispatch_app(total);
        group.bench_with_input(BenchmarkId::from_parameter(total), &total, |b, _| {
            b.iter(|| {
                queue_on_update_n(&mut app, 1);
                app.update();
            })
        });
    }
    group.finish();
}

/// Scan-isolating dispatch: `BATCH` events drained in one frame, so the fixed
/// per-`update()` overhead is amortized and the O(handlers) scan is the signal.
/// This is where the index-by-name change (O(all) -> O(matching)) is visible.
const BATCH: usize = 128;
fn bench_dispatch_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dispatch_batch");
    group.throughput(criterion::Throughput::Elements(BATCH as u64));
    for total in [50_usize, 500, 2000, 5000] {
        let mut app = build_dispatch_app(total);
        group.bench_with_input(BenchmarkId::from_parameter(total), &total, |b, _| {
            b.iter(|| {
                queue_on_update_n(&mut app, BATCH);
                app.update();
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_filter_entity,
    bench_condition_eval,
    bench_dispatch,
    bench_dispatch_batch
);
criterion_main!(benches);
