# Scenario hot-path opt: intern entity-filter string keys + cache/compile condition eval

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.6.0,modding,perf

Spike: tasks/20260714-083224/SPIKE.md

Goal: the higher-ROI scenario optimizations the spike surfaced, beyond the
index-by-name lookup. Two hotspots hit every frame by per-frame `OnUpdate`
handlers:

- String equality in `EntityFilterConfig` (filters.rs:38-101, e.g. `id_value == id`
  at :52) - intern/pre-hash the string keys into ids at load time instead of
  comparing strings each frame.
- `VariableConditionNode::evaluate` recursion cloning `VariableLiteral`s
  (variables.rs:194-230) - cache or compile simple conditions to direct lookups.

Gated on the benchmark (20260714-083331): only do the ones the numbers justify.
Framed as insurance for large community mods, not for today's built-ins.

## Verdict after the benchmark (20260715, branch `modding-perf`): DEFER, measured

The benchmark ran; neither half is justified at realistic event rates. See
`docs/modding-perf-report.md` (Decisions #3) for the numbers. Summary:

- Condition eval measures **26 ns**, and expression filters live on `OnUpdate`,
  which fires once per frame and cannot burst. Cost is bounded by the live
  `OnUpdate`-handler count per frame: ~26 us/frame even at 1000 handlers
  (~0.16% of a 16 ms frame). Compiling the AST might halve that - noise.
- Entity-filter equality measures **13 ns match / 11 ns reject**. It can burst on
  discrete events, but it is already cheap, and the index handler-by-name change
  (20260525-133014) removes the surrounding non-matching scan that was the real
  burst cost. Interning the keys would churn the RON/`serde` data model for a
  sub-14 ns per-handler saving.

Leave OPEN as insurance: revisit only if a real community mod ever profiles as
filter/condition-bound. Doing it now optimizes a cost the benchmark shows does
not exist at realistic rates.

