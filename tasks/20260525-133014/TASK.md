# Optimize modding event handler lookup

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.6.0,modding,chore

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
