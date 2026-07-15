# Optimize modding event handler lookup

- STATUS: CLOSED
- PRIORITY: 40
- TAGS: v0.6.0, modding, chore

Index handlers by event name for fast lookup. Legacy #118.

Spike: tasks/20260714-083224/SPIKE.md

Re-scoped after the detailed spike. The optimization is VALID but small today
(built-ins have 1 handler each; shakedown 19; microsecond costs), so it is GATED on
the dispatch benchmark (20260714-083331) - do it when the numbers justify it, as
insurance for large community mods, not for current content. Note the dispatch loop
lives in the external `bevy-common-systems` `GameEventsPlugin` (git rev 4a743b2...),
so this may need grouping handlers on nova's side or a change upstream; the
benchmark clarifies which. Higher-ROI sibling optimizations (filter string
interning, condition-eval caching) are split into 20260714-083339.

## Implemented (20260715, branch `modding-perf`): upstream change, landing pending

The benchmark answered the open question: the win is upstream, in
`bevy-common-systems`' dispatch loop, not on nova's side. Implemented there
(committed on the `perf-event-index` branch of the local bcs checkout,
`f788627`): `EventHandlerIndex<W>` of contiguous handler snapshots +
`maintain_handler_index` + snapshot-driven `queue_system`. See
`docs/modding-perf-report.md`.

Measured -17-24% on burst dispatch (many events in one frame) at 500-5000
handlers, neutral at 1 event/frame. Two dispatch correctness tests upstream; all
59 nova_scenario tests pass against the patched crate (via a temporary `[patch]`
in the root `Cargo.toml`).

Remaining to CLOSE: push the bcs commit to the remote and bump the pinned git rev
in the four nova crates (`nova_scenario`, `nova_gameplay`, `nova_events`,
`nova_debug`), then drop the `[patch]`.
