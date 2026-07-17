# Comms pacing queue: ordered story lines, min display, per-line dwell, fades, comms blip, objective flash

- STATUS: OPEN
- PRIORITY: 39
- TAGS: spike,v0.7.0,hud,scenario,gameplay

Goal: kill the latest-wins story-line bug and make comms readable: queue
StoryMessage lines and display them in arrival order with a minimum
on-screen time (8s dwell stays the default; add an optional per-line
dwell seconds on the action, clamped ~[3,30], syntax documented); fade
in/out and the new-objective gold flash via the UNUSED bevy-common-systems
Tween/UiAnimate helpers (reuse-known-good-stack); a comms blip in the
UiSfx bank per displayed line (the anti-masking pattern from
objective_feedback applies). Queue depth ~4 drop-oldest (decide in-task);
the full log stays in StoryFeed. Spike: tasks/20260717-155740/SPIKE.md.
