# Contextual keybind hints: availability resolver, hint cluster, anchored hints

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.5.0, hud, input, ux, spike

Spike: docs/spikes/20260710-174523-diegetic-instruments-keybind-hints.md
Depends on: 20260709-103454 (maneuver instruments v1 - the hint cluster
docks with the instruments' status area)

## Goal

"Arma Reforger"-style keybind hints (user request 2026-07-10) as a
substrate, not per-feature hacks: one verb-availability resolver (STOP =
computer alive, GOTO = lock present, ORBIT = dominant well and not
orbiting, CANCEL = engaged) feeding (a) a hint cluster docked with the
flight-status line (key chip + verb, lit when available) and (b) anchored
hints on the object the verb applies to via the screen-indicator substrate
- absorbing the hand-placed [O] ORBIT cue as the first consumer. Key
labels derive from the live bevy_enhanced_input bindings if introspection
is clean, else an authored label table (spike open question; resolve at
plan time). Direction-level: /plan owns the steps.
