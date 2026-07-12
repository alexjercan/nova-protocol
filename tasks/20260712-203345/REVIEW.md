# Review: InsetZoomable flag - inset scopes ships/torpedoes/asteroids, not beacons

- TASK: 20260712-203345
- BRANCH: feature/inset-zoomable-scope

## Round 1

- VERDICT: APPROVE

Scope: `hud/target_inset.rs` (marker + observers + gate + AABB framing),
`hud/screen_indicator.rs` (`target_world_aabb` -> pub(crate)),
`nova_scenario/objects/asteroid.rs` (bundle line), docs + CHANGELOG + TASK.

Independent verification (shared-session blind-spot guard):
- Re-derived the load-bearing claim that beacons cannot get the flag: grepped
  every `InsetZoomable` authoring site - two observers keyed on
  `SpaceshipRootMarker` / `TorpedoTargetChosen`, and the asteroid bundle. The
  beacon bundle carries `BeaconMarker` only (no ship/torpedo marker, no explicit
  insert), so no path reaches it. The gate is therefore correct by construction,
  and the delivery-guarded unit test confirms the behaviour.
- Re-checked the framing "regression" risk: for the range's 3-section ship the
  AABB-corner radius (~1.66) and the old section-spread radius (~1.5) both fall
  under `INSET_MIN_DISTANCE`, so the camera distance clamps to 6 either way - no
  framing change for small ships; larger ships pull back slightly more (whole
  hull framed), which is if anything more correct. Live capture confirms a clean
  frame.
- Query disjointness holds: `q_children`/`q_aabb` read `Children`/`ColliderAabb`,
  disjoint from `q_camera`'s `&mut Transform`; `q_anchor` stays marker-excluded.
  Compiles and runs (12_hud_range autopilot green).

- [x] R1.1 (NIT) hud/target_inset.rs:1-4 - the module header still says the
  inset is a close-up of "the currently focused/locked enemy ship"; it now
  scopes any `InsetZoomable` body (ships, torpedoes, asteroids). Reword to
  "focused/locked zoomable body" when convenient. Non-blocking.
  - Response: Reworded the header to "focused/locked body - a ship, torpedo or
    asteroid flagged InsetZoomable, but not a nav beacon".

## Round 2

- VERDICT: APPROVE

R1.1 reworded. No code change, no new findings. Branch ready to land.

Notes: the split authoring (observers for ships/torpedoes in nova_gameplay, a
bundle line for asteroids in nova_scenario) is justified by the crate boundary
(nova_gameplay cannot observe nova_scenario's `AsteroidMarker`) and is not worth
unifying. Reusing `target_world_aabb` (already sensor-excluding, subtree-walking)
for framing is good reuse. Tests are meaningful and delivery-guarded.

Check suite (repo policy: full suite + clippy in CI): `cargo test -p
nova_gameplay target_inset` (9 pass), `cargo fmt --check` clean, `cargo check
--workspace` non-debug clean, `12_hud_range` autopilot PASS/no-panic.
