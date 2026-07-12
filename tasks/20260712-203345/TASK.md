# Inset scope: InsetZoomable marker so the target inset only zooms ships

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.5.0, hud, targeting, spike

## Goal

The target inset (task 20260710-104421) currently frames whatever is focused,
including beacons (authored `LockSignature`) and committed torpedoes. Add a
flag component for what is worth scoping - ships today - and gate the inset on
it, so the scope only ever shows enemy ships.

Direction (see spike for reasoning): a dedicated `InsetZoomable` marker
authored on ship roots (not just reusing `SpaceshipRootMarker`), so
scope-worthiness is decoupled from ship-ness and a modded scenario could later
mark a boss asteroid/station zoomable. Gate
`hud/target_inset.rs::drive_inset_camera`'s `framed` predicate on the target
carrying the marker.

## Notes

- Spike: docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md
  (Part 1, option A2).
- Relevant files: `crates/nova_gameplay/src/hud/target_inset.rs`
  (`drive_inset_camera` gate), the ship spawn path in
  `crates/nova_gameplay/src/sections/` (author the marker on the root).
- Independent of the lock-feel tasks (20260712-203349, 20260712-203353); safe
  to land first.
- Buy-in requested before implementing.
