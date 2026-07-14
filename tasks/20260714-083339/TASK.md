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

