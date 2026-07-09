# ORBIT autopilot verb: circularize and station-keep inside a gravity well

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.5.0,handling,autopilot,gravity,spike


Spike: docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md
Depends on: 20260709-193338 (gravity-well substrate)

Goal: third diegetic autopilot verb next to STOP/GOTO. Inside a well, one
input engages ORBIT; the maneuver machine flies a real insertion through the
existing actuator seams: Plan (target radius clamped into the stable SOI
band; plane from r x v with a fallback when velocity is near-zero/radial) ->
Align -> Burn to tangential v_circ -> Hold (micro-burn station-keeping
against drift). Breakout on any flight input; capability/destruction coupling
inherited (dead controller = no ORBIT, dead engines = aligns but cannot
burn, dead ship = orbit decays). HUD v1: flight-status states (GRAV well /
AP ORBIT phases) + an orbit-available cue on the screen-indicator substrate.
Physics-level tests: engage from near-rest, reach and hold a bounded orbit,
breakout restores manual. Direction-level: /plan owns the steps.