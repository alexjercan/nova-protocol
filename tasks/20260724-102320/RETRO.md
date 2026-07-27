# Retro: NOVA OS `map` app (20260724-102320)

- STATUS: CLOSED
- DATE: 20260727
- OUTCOME: shipped the map app PoC + a shell-emulator polish pass, landed to
  master after 5 owner playtest rounds.

## What shipped

The terminal-launched `map` app: a schematic 3D minimap rendered to a texture
(distance rings + hub on a dedicated `RenderLayers`, a `MapOrbit` camera), with
contacts drawn as projected clickable UI blips (allegiance colours, range/bearing
readout), keyboard orbit (Q/E turn, R/F tilt, WASD pan, wheel zoom, T reset,
mouse-drag look), click/cycle selection that recenters the rings, and `G` GOTO
that sets a flight `Autopilot` on the player ship. Plus the `map view` CLI and,
folded in from playtest feedback, a shell-emulator layer: `<command> help` /
`<command> version` universal sub-verbs, fish-style sub-command completion +
ghost, and better bad-argument errors.

## What went well

- The plan-gate-delegated-to-out-of-context-review worked as the owner asked:
  the reviewer re-derived every load-bearing claim against the tree and APPROVEd;
  nothing it green-lit broke in the build. The DECISION.md's two hard constraints
  (trait gives no mouse; nested RTT mesh not pickable so contacts ride as UI
  blips) held all the way through.
- Headless App-driven tests (`run_system_once`, asset stores registered) proved
  the RTT scene build, focus-follow, GOTO, contact model and the whole shell
  language without a GPU - and stayed green across five refactors.

## What went wrong / difficulties

- Could NOT visually verify anything locally: this box's GPU segfaults on shader
  compile (`NVVM compilation failed`) the moment the app reaches Playing. Every
  feel/render bug (mouse dead, Q/E not rotating, scroll snapping, text under the
  bezel, footer overlap) surfaced only in the owner's hands, over five rounds.
- Two "it compiles and the unit test passes but it doesn't work" traps:
  1. Blip selection via `Interaction` polling did nothing - `Interaction` does
     not update through the CRT-composited RTT; the `Activate` Button observer
     (forwarded pointer) is the path that works, same as the terminal's buttons.
  2. Q/E / mouse rotation wrote `SphereOrbitInput`, but the shared `SphereOrbit`
     plugin's smoothed path never rotated this RTT camera (only the direct
     `center` pan moved). Owning the spherical math in a `MapOrbit` component
     fixed it immediately.
- The scroll regression (`rebuild_terminal_ui` pinning to the bottom on EVERY
  terminal-resource change) took two rounds to pin because its unit test forces
  overflow and passes; the real defeat only shows when the resource changes for
  unrelated reasons (prompt edits, app-command mirror).
- CRT overscan (~3.5%/edge under the bezel) plus "absolute children ignore
  container padding" meant the content safe-area had to be applied twice - once
  on the content root, once on the app root.

## Self-feedback (do differently next time)

- For anything behind an RTT + CRT + pointer-forwarding (or any reused
  input/animation component), VERIFY THE OUTPUT MOVES before building on it -
  a passing arity/lifecycle test proves the seam exists, not that the pixels or
  the rotation actually respond. Prefer the project's own working pattern
  (Activate observer, direct transform math) over the generic one when the
  generic one is unverified in this context.
- When you cannot render locally, say so up front and structure the work as a
  tight playtest loop with the owner; batch the headless-provable parts and
  flag the feel/render parts as owner-review from the first hand-off.
- "Pin to bottom on resource change" is a scroll anti-pattern: gate the auto
  scroll on new content, not on any change to the backing resource.

## Lessons folded to LESSONS.md

- `rtt-ui-pickable-via-activate-not-interaction`
- `verify-reused-driver-actually-moves`
- `autoscroll-on-new-content-not-any-change`
