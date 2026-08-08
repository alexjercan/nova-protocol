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

## Implemented and landed (20260715, branch `modding-perf`): upstream change

The benchmark answered the open question: the win is upstream, in
`bevy-common-systems`' dispatch loop, not on nova's side. Implemented there and
pushed to bcs `master` (`4c81117`): `EventHandlerIndex<W>` of contiguous handler
snapshots + `maintain_handler_index` + snapshot-driven `queue_system`. See
`tasks/20260714-083331/modding-perf-report.md`.

Measured -17-24% on burst dispatch (many events in one frame) at 500-5000
handlers, neutral at 1 event/frame. Two dispatch correctness tests upstream; all
59 nova_scenario tests pass against it.

Landed: the pinned git rev was bumped `4a743b2 -> 4c81117` in the four nova
crates (`nova_scenario`, `nova_gameplay`, `nova_events`, `nova_debug`), the
temporary `[patch]` was removed, and `cargo check --workspace` is green against
the git dependency.
