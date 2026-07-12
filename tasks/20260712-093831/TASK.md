# Objective conveyance visuals: markers, item highlight, hint emphasis

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.5.0,scenario,hud,polish

Goal: the visual language that shows the player WHAT the current objective
is, WHERE it is, and WHICH button does it - as reusable substrate pieces,
not scenario hacks. The Shakedown Run scenario ships without these (its
layer-0 conveyance is text + small distances + emissive props) and
upgrades in place when they land:

- Objective marker action: an EventActionConfig that attaches/detaches a
  designated screen indicator (distinct objective styling, label +
  distance) to a scenario entity by id. Off-screen it clamps to the
  screen edge (existing ScreenIndicatorOffscreen::ClampToEdge), so the
  marker doubles as the direction-to-objective arrow for free.
- Item highlight: a pulsing treatment for interactable/collectible props
  (emissive pulse on the mesh and/or an apparent-size bracket chip that
  tightens close-in), applied to salvage crates and future pickups.
- Hint emphasis: a small API on the keybind-hint cluster to pulse or
  brighten one verb row on request, so an objective like "let the
  computer fly" can point at [G] GOTO without new UI; scenario gets an
  action to trigger it. Anchored on-object cues already exist.
- Objective panel progress: update an objective's message in place
  (counters like "2/3") without the complete+re-add flicker, if the
  ObjectivesPlugin read at /plan shows re-add is glitchy.

Notes:
- Spike: docs/spikes/20260712-092926-starter-scenario.md (section
  "Conveying objectives: layered, degrades to text")
- Enhances: 20260711-180506 (Shakedown Run works without this; each
  piece slots in via scenario data once available)
- Builds on: 20260712-093044 (nav beacon chip proves the indicator
  styling), the screen-indicator substrate, hud/keybind_hints.rs
