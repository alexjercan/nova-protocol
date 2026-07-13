# Contextual keybind hints: availability resolver, hint cluster, anchored hints

- STATUS: CLOSED
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
labels derive from the live bevy_enhanced_input bindings - introspection
RESOLVED clean at plan time: action entities carry `Bindings`
(relationship) whose binding entities each hold a `Binding` component with
a Display impl; the first Keyboard binding gives the label.

## Steps

- [x] `FlightVerbHints` resource (reflected) in input/player.rs - where
      the verbs and the (private) input-action types live: one entry per
      verb (Stop, Goto, Orbit, Cancel) with `key: String` (first Keyboard
      binding of the verb's action entity, "Key" prefix stripped, empty
      until the flight rig exists), `available: bool` (Stop = player ship
      present; Goto = aim lock present; Orbit = DominantWell present and
      not already orbiting; Cancel = Autopilot engaged), and
      `anchor: Option<Entity>` (Goto = the lock, Orbit = the well) for
      anchored hints.
- [x] Resolver system in SpaceshipInputSystems (Update) computing the
      resource every frame from the live world (bindings re-read each
      frame - cheap, four queries - so a future remap screen stays
      honest). Unit tests: availability truth table (no ship / no lock /
      lock / in well / orbiting / engaged) and label derivation from a
      spawned action entity with real bindings.
- [x] Hint cluster in a new hud/keybind_hints.rs: a small column docked
      above the flight-status line, one row per verb ("[X] STOP",
      "[G] GOTO", "[O] ORBIT", "[Z] CANCEL"), NAV_CYAN when available,
      dimmed gray when not; rows with empty keys hidden. Driven from
      FlightVerbHints only (render-dumb, per the instruments retro).
- [x] Anchored hints: absorb the hand-placed [O] ORBIT cue - the orbit
      cue's text becomes "[<key>] ORBIT" from the resource and its anchor
      comes from the resource's Orbit entry; add the sibling "[<key>]
      GOTO" cue anchored to the current lock while no autopilot is
      engaged (offset below the reticle, indicator substrate).
- [x] Spawn/despawn with the player HUD (hud/mod.rs observers, same
      pattern as the other layers).
- [x] Tests: cluster rows reflect availability + labels; orbit cue
      absorbs the resource (in-well/orbiting/outside states); goto cue
      follows the lock and hides while engaged.
- [x] fmt + check --workspace --examples + affected modules (input,
      hud); document in docs/retros/20260710-keybind-hints.md.

## Notes (planning)

- The resolver lives in input/player.rs because the action structs
  (AutopilotStopInput etc.) are deliberately private to the input layer;
  the HUD sees only the resource - same compute-at-the-truth,
  render-dumb shape as ManeuverTelemetry (instruments retro lesson).
- Keyboard labels only in v1 (spike open question: device awareness
  deferred until a pad-detection signal exists).
- KeyCode Debug formatting: "KeyX" -> strip the "Key" prefix; other
  variants (Space, Enter) pass through as-is.

## Resolution

Shipped per plan: FlightVerbHints resolver in the input layer (live-binding
labels, availability truth table, anchors), hud/keybind_hints.rs cluster +
anchored cues, the hand-placed [O] ORBIT cue absorbed and the [G] GOTO cue
added. 6 new tests; hud/input/flight modules green; fmt + check
--workspace --examples clean. Details: docs/retros/20260710-keybind-hints.md.
