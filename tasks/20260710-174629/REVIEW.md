# Review: World-space holo instruments - ribbon, SOI shell, flip gate

- TASK: 20260710-174629
- BRANCH: holo-expansion

## Round 1

- VERDICT: APPROVE (all findings MINOR/NIT; fixed in-round before merge)

Reviewed commit 4ed90f4 against master with an independent adversarial
pass. Sound: the STOP telemetry lifecycle (insert-before-disengage flush
order, epsilon clearing, NaN guards), rest-point stability under per-tick
replanning (stationary to first order during the coast), ribbon scale
math (local Y-scale before rotation), flip-gate direction (colinear by
construction, retires before it could invert), the cleanup chain, system
ordering (no indicator edge needed), and the three lifecycle tests. No
correctness-critical defects.

- [x] R1.1 (MINOR) flight.rs (STOP telemetry) - publish gate
  `speed > stop_speed_epsilon` with removal below it flickers when a ship
  hovers at the threshold (gravity re-acceleration, hot engines while
  settling): the ribbon and chip strobe. Fix: hysteresis - publish above
  2x epsilon, keep publishing until below epsilon.
  - Response: fixed - the publish gate keeps the leg alive while telemetry
    already exists (enter at 2x stop_speed_epsilon, hold until epsilon).
- [x] R1.2 (MINOR) holo_instruments.rs + doc - "one shared material" was
  false: Local<HoloAssets> is per-system, so three identical materials
  (four with the orbit ring) defeat batching and the doc misstates the
  design. Fix: promote HoloAssets to a Resource shared by all four
  systems.
  - Response: fixed - HoloAssets is a Resource (init in both plugins); the
    ribbon, gate, shell, and the orbit ring all share one material, and
    the gate mesh is cached alongside the segment mesh (also R1.4).
- [x] R1.3 (MINOR) holo_instruments.rs (sync_soi_shell) - ring identity
  recovered by exact Quat equality on the written-back rotation; any
  future writer demotes rings to per-frame despawn/respawn mesh churn.
  Fix: carry the axis index on SoiShellRing like the ribbon does.
  - Response: fixed - SoiShellRing gains `index`; identity is data, not
    float equality.
- [x] R1.4 (NIT) gate torus rebuilt per respawn despite a const radius;
  flip flicker near the brake boundary would churn meshes.
  - Response: fixed - gate mesh cached in the shared HoloAssets.
- [x] R1.5 (NIT) approach selection by absolute distance can pick a small
  nearby well over the big well the ship is deep inside; and the module
  doc says "approaching" with no velocity test.
  - Response: fixed - selection by smallest r/soi ratio (deepest relative
    penetration); comment reworded to "within the approach band" (no
    velocity test, matching the task spec).
- [x] R1.6 (NIT) stale keep-in-step comment (GOTO-only), redundant second
  run_system_once in the ribbon test, undocumented lead-speed asymmetry
  in the STOP branch.
  - Response: fixed - comment updated, duplicate call kept but labeled as
    an explicit idempotence check, asymmetry documented at the call site.

Noted, no action: the SOI shell coincides with the F11 debug gizmo sphere
when debug is on (anticipated by the debug module's header); shell shows
for departing ships inside the band (spec says band, not velocity).

Verification of the in-round fixes: hysteresis gate in the STOP arm
(enter 2x epsilon, hold to epsilon), HoloAssets promoted to a shared
Resource consumed by ribbon/gate/shell and the orbit ring (one material,
gate mesh cached), SoiShellRing carries its axis index, deepest-ratio
well selection, comments and the idempotence-check label in place. All
affected modules green after the fixes (holo 3, instruments 3, flight 50,
stop 5); fmt + check --workspace --examples clean. Ready to land.
