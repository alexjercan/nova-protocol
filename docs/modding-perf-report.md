# Modding scenario-dispatch performance: baseline, optimizations, results

Sprint v0.6.0, branch `modding-perf`. Covers three tatr tasks:

- `20260714-083331` (p45) - scenario dispatch benchmark + profile baseline (the gate).
- `20260525-133014` (p40) - index modding event handlers by event name.
- `20260714-083339` (p35) - hot-path: intern entity-filter string keys + cache/compile condition eval.

All three descend from spike `20260714-083224`, which argued the seeded optimizations
were plausible but unmeasured and had to be **gated on a benchmark** rather than done
blind. This report is that measurement, and the decisions it drove.

## TL;DR

- The built-in scenarios are tiny (1-19 handlers), so nothing here matters for
  shipped content. The work is **insurance for large third-party mods**.
- Built a synthetic many-handler benchmark (`crates/nova_scenario/benches/scenario_dispatch.rs`,
  criterion). It measures three layers: the entity filter, the condition eval, and
  the full `GameEventsPlugin` dispatch loop.
- New tooling (approved with the user): `criterion` dev-dep + a `[[bench]]` target;
  `samply` added to the Nix dev shell for sampling profiles.

## Tooling added

| Tool | Where | Why |
|------|-------|-----|
| `criterion` 0.7 (`html_reports`) | `crates/nova_scenario/Cargo.toml` dev-dep + `[[bench]]` | statistical bench harness with before/after diffing and HTML reports under `target/criterion/` |
| `serde_json` | same, dev-dep | build the entity-filter event payloads the bench feeds in |
| `samply` | `flake.nix` dev shell | sampling profiler: `samply record -- cargo bench ... -- --profile-time 10` to prove where time goes |

## The hot path (code map)

The dispatch loop itself lives in the external `bevy-common-systems` crate
(`src/modding/events.rs`, `queue_system`), pinned by git rev in nova. Baseline shape:

```rust
for handler in &q_handler {                 // every handler in the world
    if handler.name == event.name && handler.filter(&*world, &event.info) {
        for action in &handler.actions { action.action(&mut *world, &event.info); }
    }
}
```

- `O(all handlers)` per event: a fired `OnUpdate` scans past every `OnEnter`,
  `OnDestroyed`, ... handler too. Target of `20260525-133014`.
- `handler.filter` -> nova's `EntityFilterConfig::filter` (per-field `serde_json`
  map lookup + `String == String`, `filters.rs`) and `VariableConditionNode::evaluate`
  (recursive tree walk cloning `VariableLiteral`s, `variables.rs`). Target of `20260714-083339`.

## Results

### Micro (per matching handler, per event)

| Bench | Baseline |
|-------|----------|
| `filter_entity/match`  | 13.1 ns |
| `filter_entity/reject` | 11.2 ns |
| `condition_eval/greater_than` | 26.1 ns |

These are per matching handler, per event. They are left unchanged: the
condition/filter micro-opts (`20260714-083339`) are deferred on measured grounds
(see Decisions #3), so there is no "after" column.

### Dispatch, realistic (1 `OnUpdate`/frame, N handlers, 10% `OnUpdate`)

| N handlers | Baseline | Indexed (snapshot) |
|-----------:|---------:|-------------------:|
| 50   | 20.6 µs | 20.4 µs |
| 200  | 21.4 µs | 23.1 µs |
| 500  | 22.7 µs | 23.9 µs |
| 2000 | 29.0 µs | 29.3 µs |

Finding: near-flat, index-neutral. At one event/frame the fixed per-`update()`
frame overhead (~20 µs with `MinimalPlugins`) dominates; the marginal name-scan
is ~4.3 ns per extra handler and only becomes a visible fraction of the frame at
~2000 handlers (~8 µs, ~30%). The index neither helps nor hurts here - dispatch
just is not the cost at one event per frame. (The small 200/500 deltas are
within run-to-run variance; this group was re-measured with a shorter criterion
window than the batch group.)

### Dispatch, scan-isolated (128 events/frame batch, N handlers)

The batch drains many events in one frame so the fixed frame overhead is
amortized and the O(handlers) scan is the signal. This regime is not just a
microbenchmark trick: it is what a **burst** looks like - a wave of entities all
emitting `OnDestroyed`/`OnEnter` in a single frame, each event re-scanning every
handler.

| N handlers | Baseline | Naive index (entity-id) | Snapshot index | vs baseline |
|-----------:|---------:|------------------------:|---------------:|------------:|
| 50   | 47.9 µs  | 46.4 µs  | 45.4 µs  | -5%  |
| 500  | 254 µs   | 209 µs   | 194 µs   | -24% |
| 2000 | 885 µs   | 888 µs   | 737 µs   | -17% |
| 5000 | 2.05 ms  | 2.09 ms  | 1.70 ms  | -17% |

Two things this table records:

1. **The scan is real and O(N)** under bursts, and the index removes the ~90% of
   it spent on non-matching handlers: a steady ~17-24% at 500-5000 handlers.
2. **The first index design was wrong, and the benchmark caught it.** Indexing
   *entity ids* and looking each up with `Query::get` during dispatch (the "naive
   index" column) traded Bevy's fast linear archetype iteration for random-access
   component lookups. It helped at 500 (-18%) but the win *vanished* by 5000
   (cache thrash grows with N). Switching the index to store contiguous handler
   **snapshots** - no ECS touch during dispatch - restored a win that scales.

## Decisions

### 1. Benchmark (`20260714-083331`) - DONE

Shipped `crates/nova_scenario/benches/scenario_dispatch.rs` (criterion) with the
three groups above, and added `samply` to the dev shell for sampling profiles.
This is the gate the other two tasks were waiting on. It paid for itself twice:
it justified the index for burst dispatch, caught the naive-index regression, and
proved the hot-path micro-opts are not justified at realistic event rates (below).

### 2. Index handlers by event name (`20260525-133014`) - DONE (landed)

Implemented in `bevy-common-systems` (`src/modding/events.rs`): an
`EventHandlerIndex<W>` of contiguous handler snapshots, an ungated
`maintain_handler_index` system, and a snapshot-driven `queue_system`. Two
dispatch correctness tests added; all 59 nova_scenario tests pass against it.
Measured -17-24% on burst dispatch, neutral at 1 event/frame.

Landed cross-repo: committed on bcs `master` as `ae68e38` and pushed; the pinned
git rev was bumped `4a743b2 -> ae68e38` in the four nova crates that depend on it
(`nova_scenario`, `nova_gameplay`, `nova_events`, `nova_debug`), the temporary
`[patch]` was removed, and `cargo check --workspace` is green against the real
git dependency.

### 3. Hot-path filter/condition micro-opts (`20260714-083339`) - DEFER (measured)

Not justified by the numbers, per the task's own gating:

- **Condition eval** (`VariableConditionNode::evaluate`, measured **26 ns**) is
  hit only by *matching* handlers, and expression filters live on `OnUpdate`,
  which fires **once per frame and cannot burst**. So the cost is bounded by the
  live `OnUpdate`-handler count per frame: even 1000 such handlers = ~26 µs/frame,
  ~0.16% of a 16 ms frame. Interning/compiling the AST might roughly halve the
  26 ns, saving low-single-digit µs/frame on a mega-mod - deep in the noise.
- **Entity-filter string equality** (`EntityFilterConfig::filter`, measured
  **13 ns match / 11 ns reject**) *can* burst (it runs on discrete events), but
  it is already cheap and the index removes the surrounding non-matching scan,
  which was the larger burst cost. Interning the keys would complicate the RON
  data model and the `serde` layer for a sub-14 ns saving per matching handler.

Both are documented as **insurance to revisit only if a real community mod ever
profiles as filter/condition-bound**. Doing them now would be optimizing against
a cost the benchmark shows does not exist at realistic event rates - exactly the
blind optimization the gating benchmark existed to prevent.

## What was tried and rejected

- **Entity-id index + `Query::get` dispatch.** Clean-looking, but random-access
  component lookups erased the win at scale. Replaced by contiguous snapshots.
- **Condition-eval compile / entity-key interning.** Deferred on measured
  grounds (above), not attempted in code, to avoid `serde`/format churn for no
  measurable realistic-rate gain.

## Reproducing

```
cargo bench -p nova_scenario --bench scenario_dispatch          # all groups
cargo bench -p nova_scenario --bench scenario_dispatch -- dispatch   # dispatch only
samply record -- cargo bench -p nova_scenario --bench scenario_dispatch -- \
    dispatch_batch/5000 --profile-time 10                       # flamegraph
```

HTML reports (plots, before/after diffs) land in `target/criterion/`.
