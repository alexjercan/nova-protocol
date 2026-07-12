# Inset scope: InsetZoomable marker so the target inset only zooms ships

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.5.0, hud, targeting, spike

## Goal

The target inset (task 20260710-104421) currently frames whatever is focused,
including beacons (authored `LockSignature`) - a nav waypoint is not worth
scoping. Add an opt-in flag for the bodies that ARE worth scoping and gate the
inset on it, "just skip beacons".

Direction (see spike + user refinement 2026-07-12): a dedicated `InsetZoomable`
marker authored on the lockable physical/combat bodies - ship roots, committed
torpedoes, and asteroids - but NOT beacons or trigger areas. Gate
`hud/target_inset.rs::drive_inset_camera`'s `framed` predicate on the target
carrying the marker.

Torpedoes and asteroids have no `SectionMarker` children, so the inset framing
(`ship_framing_radius`) must fall back to the target's own collider/mesh extent
for section-less bodies (generalize it, e.g. to `zoomable_framing_radius`).

The inset must also stay `ALL`-mode only (hidden in Minimal/None). This is
already the behaviour (the panel is `Chrome` tier and the camera gates on
`HudVisibility::shows(HudTier::Chrome)`, and Chrome shows only at `All`) - pin
it with a test rather than new code.

## Steps

- [ ] Add `InsetZoomable` marker component in `hud/target_inset.rs` (or
      sections), registered for reflection.
- [ ] Author it where the zoomable bodies spawn: ship roots
      (`sections/` ship spawn), committed torpedoes
      (`TorpedoProjectileMarker` path), asteroids (scenario asteroid spawn).
      NOT beacons.
- [ ] Gate `drive_inset_camera`'s `framed` predicate on
      `q.get(target).has::<InsetZoomable>()`.
- [ ] Generalize `ship_framing_radius` to section-less bodies: use the section
      spread when the target has sections, else the target's own collider
      (`Collider`) or mesh `Aabb` extent. Confirm the extent source against a
      torpedo/asteroid entity.
- [ ] Tests: a focused non-zoomable target (beacon) spawns no inset camera and
      keeps the panel hidden; a zoomable ship still does (delivery guard); the
      inset stays hidden at HUD Minimal/None; the framing radius is finite for a
      section-less body.
- [ ] Verify in `12_hud_range` (add a beacon to the range, or assert against a
      torpedo) and update the docs note.

## Notes

- Spike: docs/spikes/20260712-203235-lock-stickiness-and-inset-scope.md
  (Part 1 A2, Refinement).
- Relevant files: `crates/nova_gameplay/src/hud/target_inset.rs`
  (`drive_inset_camera` gate + `ship_framing_radius`), ship/torpedo/asteroid
  spawn paths, beacon spawn (`beacon/`) which must NOT get the marker.
- Independent of the sticky-lock task (20260712-203353); land first.
