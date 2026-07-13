# Contextual keybind hints: resolver, cluster, anchored cues

- TASK: 20260710-174646
- SPIKE: tasks/20260710-174523/SPIKE.md
- MODULE: crates/nova_gameplay/src/hud/keybind_hints.rs (+ the
  FlightVerbHints resolver in input/player.rs)

## What was built

The "Arma Reforger"-style hint substrate the user asked for (2026-07-10):

- **FlightVerbHints resolver** (input/player.rs, where the verbs and their
  private input-action types live): one system resolving, every frame, each
  flight verb's availability (STOP/GOTO/ORBIT additionally require a
  flyable ship - live controller + live engine, the same bar the autopilot
  disengages under, so a lit hint never lies on a crippled ship; GOTO
  needs the aim lock, ORBIT the dominant well when not already orbiting;
  CANCEL = engaged, and Z answers even crippled), its anchor
  entity (the lock / the well), and its keyboard label - read from the
  flight rig's live bevy_enhanced_input `Bindings` (the spike's open
  question resolved: action entities relate to binding entities carrying a
  `Binding`; the first Keyboard binding's KeyCode, "Key"/"Digit" prefix
  stripped, is the chip label). A remap screen can never desync the hints.
- **Hint cluster** (hud/keybind_hints.rs): a small column docked above the
  flight-status line - `[X] STOP / [G] GOTO / [O] ORBIT / [Z] CANCEL` -
  nav-cyan when pressing would do something, dimmed otherwise, hidden
  until the flight rig exists.
- **Anchored cues**: the hand-placed `[O] ORBIT` cue from the ORBIT task is
  absorbed (deleted from flight_status.rs) and reborn resolver-driven, and
  a sibling `[G] GOTO` cue anchors to the aim lock while nothing is
  engaged. The hint sits on the thing you would act on - the "more
  diegetic than a print" part of the request.

## Decisions

- Compute-at-the-truth, render-dumb (the instruments retro lesson applied
  deliberately): the resolver lives in the input layer because the action
  structs are private there and availability rules mirror the input
  observers' own gates; the HUD consumes one resource.
- Keyboard labels only in v1; device awareness (pad chips) stays a
  recorded open question until a pad-detection signal exists.
- The GOTO cue hides while any maneuver is engaged (the destination marker
  owns that space); the cluster row stays lit since pressing G re-engages.

## Verification

- 2 resolver tests (label derivation from real spawned bindings incl.
  keyboard-over-gamepad selection and prefix stripping; the availability
  truth table across lock/well/orbiting/despawned-ship states), 4 HUD
  tests (cluster labels + colors, empty-rig emptiness, orbit cue
  offer/retire, goto cue lock-follow + engage-hide).
- Affected modules green: hud (50), input (111), flight (48). fmt + check
  --workspace --examples clean. Full suite and clippy on CI.

## Difficulties

None material. bevy_enhanced_input's binding introspection worked exactly
as the plan-time code read suggested.

## Self-reflection

- Two spikes-to-shipped in one day on the same substrate validates the
  spike's "hints are one table row once the instruments exist" bet; the
  next verb (dock, match-velocity) costs one resolver entry and one
  cluster row.
