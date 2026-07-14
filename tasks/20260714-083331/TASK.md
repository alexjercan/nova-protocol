# Scenario dispatch benchmark + profile: baseline with a synthetic many-handler scenario

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.6.0,modding,perf,test

Spike: tasks/20260714-083224/SPIKE.md

Goal: measure before optimizing - this task answers the user's "check if the
optimizations are ok". Build a synthetic large-mod scenario (hundreds of handlers,
dense per-frame `OnUpdate` filters and conditions) and benchmark/profile the
scenario dispatch + filter/condition hot path, to get a baseline and prove where
the time actually goes.

Why: today's built-ins are tiny (1 handler each; shakedown 19) with microsecond
costs, so none of the seeded optimizations are justified by current content. They
only pay off once modding brings large third-party scenarios. This benchmark makes
that concrete and gates 20260525-133014 (index handlers by event name) and
20260714-083339 (hot-path interning/caching) - if the numbers show they do not
matter yet, we defer them honestly rather than optimizing blind.

Note: the dispatch loop itself lives in the external `bevy-common-systems`
`GameEventsPlugin` (git rev 4a743b2...); the benchmark should surface whether the
win is on nova's side (filters/conditions) or needs a change upstream.

