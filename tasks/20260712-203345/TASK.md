# Inset scope: InsetZoomable marker so the target inset only zooms ships

- STATUS: CLOSED
- PRIORITY: 30
- TAGS: v0.5.0, hud, targeting, spike

## Outcome (CLOSED 2026-07-12)

Added `InsetZoomable` (nova_gameplay `hud/target_inset.rs`) and gated
`drive_inset_camera` on it, so the inset only scopes flagged bodies. The flag is
authored by observers - `mark_inset_zoomable::<SpaceshipRootMarker>` and
`::<TorpedoTargetChosen>` in nova_gameplay (mirrors the loader's
`on_add_entity_with` pattern, so no spawn-site hunting), plus a bundle line on
asteroids in nova_scenario. Beacons never get it, so the inset skips them.

Framing generalized: `ship_framing_radius` (section-center spread) ->
`zoomable_framing_radius`, which unions the body's non-sensor collider AABBs
(reused `screen_indicator::target_world_aabb`, made pub(crate)) and takes the
anchor-to-farthest-corner distance. Works uniformly for sectioned ships and
section-less torpedoes/asteroids; falls back to the section half-extent when a
body has no collider AABB, so the pose stays finite.

ALL-mode-only was already true (Chrome tier + the `shows(Chrome)` camera gate);
pinned by the existing `camera_absent_while_hud_chrome_is_hidden` test.

Verified: 9 headless unit tests pass (added `inset_skips_non_zoomable_targets`
with a delivery guard, and `framing_radius_is_finite_for_a_section_less_body`);
`12_hud_range` autopilot confirms the range's ship still opens the inset via the
observer-added marker (PASS, no panic); AABB framing frames the hull cleanly in
a live capture; `fmt --check` + non-debug `cargo check --workspace` green.

Note: did not add a beacon to `12_hud_range` for a live skip assertion - the
unit test covers the gate and beacons provably lack the marker (no code adds it
to them). Torpedo/asteroid scoping ride the same marker+framing path.

## Steps

- [x] Add `InsetZoomable` marker component in `hud/target_inset.rs` (or
      sections), registered for reflection.
- [x] Author it where the zoomable bodies spawn: ship roots
      (`sections/` ship spawn), committed torpedoes
      (`TorpedoProjectileMarker` path), asteroids (scenario asteroid spawn).
      NOT beacons. (Done via observers on SpaceshipRootMarker +
      TorpedoTargetChosen; asteroid bundle line in nova_scenario.)
- [x] Gate `drive_inset_camera`'s `framed` predicate on
      `q.get(target).has::<InsetZoomable>()`.
- [x] Generalize `ship_framing_radius` to section-less bodies (now
      `zoomable_framing_radius` via `target_world_aabb`).
- [x] Tests: non-zoomable target opens no inset (delivery-guarded); ALL-mode
      gate pinned; framing radius finite for a section-less body.
- [x] Verify in `12_hud_range` (ship still opens via the observer) + docs note.

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
