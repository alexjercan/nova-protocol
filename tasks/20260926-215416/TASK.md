# Add scroll-wheel zoom to the player chase camera

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.15.0,input,camera,flight

## User facts
- Zooming out and back in with the mouse wheel during flight is a MUST-NEXT feature. Today's ordinary camera framing is the closest permitted view. This task is separate from the active themed-interface migration.
- The user authorized a dedicated Sprout and a Claude-backed worker on 2026-09-26. The worker scouts and prepares a code-backed plan first; zoom implementation waits for owner review and approval.

## Decisions
- Scope: active player gameplay chase camera only. Other camera owners, themed Map/Ship panes, the command CRT, editor, and scripted cameras retain their wheel behavior.
- Wheel zoom is available in Normal and Alt/free-look; those modes share one zoom level. Alt remains farther from the ship than Normal at every manual level. In combat/Turret, ignore manual zoom and use the existing authored combat camera. Combat wheel still cycles locked components. RCS vertical thrust wins the wheel while RCS is held, including in Normal/Alt; releasing RCS restores zoom.
- Use camera dolly along the existing chase rig, not lens/FOV changes. Today's hull-cleared Normal/Alt framing is the 1x closest level; prototype a maximum of 8x that mode's baseline distance, then inspect rendered results on small and large hulls. Preserve the authored look-at focus, burn push and mode behavior unless a measured failure requires a reviewed change.
- Zoom is session-only: first flight starts at 1x, Normal/Alt share a level, and the level survives mode switches and respawn in that session; it does not persist across game launches. No saved-settings schema change. No zoom HUD indicator. Mouse wheel/trackpad first; gamepad parity belongs to `20260714-001140` and does not gate this slice.
- Entering a planned ORBIT starts at its usual automatic survey framing, independent of the prior Normal/Alt manual level. The first wheel action adjusts from the displayed ORBIT distance; wheel-in can approach but not cross the ordinary hull-cleared minimum. ORBIT's existing auto distance is not clamped down if it exceeds the new 8x manual cap; in that case wheel-out stops at the initial survey distance. On leaving ORBIT, restore the pre-ORBIT Normal/Alt level, not the temporary orbit override.
- In Normal/Alt without RCS, wheel is reserved for zoom. Component cycling remains on bracket keys and D-pad. In combat it retains its existing wheel binding. One gesture must never actuate zoom and targeting/RCS together.
- 2026-09-26 owner approved the scout plan, its types, functions and named focused tests, and policies 1-7 as recommended:
  1. `ChaseZoom` is one app-level resource, never reset.
  2. The opening pose and the respawn stamp use the session level.
  3. Wheel up moves the camera closer.
  4. Steps are multiplicative per wheel line. The factor is NOT approved: measure it with a disposable input/render spike and bring the number back for approval before finalizing. Interpolation reuses the chase smoothing.
  5. Registry rows `camera_zoom_in` / `camera_zoom_out` in the CAMERA group, labelled "Zoom In (combat: next component)" / "Zoom Out (combat: prev component)".
  6. The component-cycle hint reads "SCROLL" only while `WeaponsRaised`; otherwise it shows the bracket key label.
  7. Shift+brackets still nudges RCS vertical.
- 2026-09-26 owner flew the wheel zoom and accepted it by feel: factor 2^(1/4) (1.1892071) per wheel line and the 8x cap, provided independent checks pass.

## Agent findings (recheck on implementation branch)
- `crates/nova_ship/src/camera/framing.rs:165-206,296-391`: Normal, FreeLook and Turret rigs are hull-cleared, with authored offsets and focus; `update_camera_rig` rewrites ChaseCamera offset/focus every frame and includes burn push, orbit survey and velocity-lead compensation. Zoom written elsewhere would be lost the next frame.
- `crates/nova_ship/src/camera/framing.rs:141-153,246-268`: planned ORBIT applies survey dolly with a 250-engine-unit extra-distance cap. This can exceed 8x a small ship's base rig.
- `crates/nova_ship/src/input/player/flight_rig.rs:207-244` and `crates/nova_ship/src/input/targeting/component_lock.rs:226-281`: wheel drives component cycling and RCS vertical today. `crates/nova_ship/src/input/bindings.rs:77-84` also binds brackets and D-pad for component cycling. Their mode/modifier precedence and display hints need coordinated changes.
- `crates/nova_ship/src/camera/chase.rs:73-107` defines the smoothed offset and focus; `crates/nova_ship/src/camera/mode.rs:124-129` notes camera rig fields have one per-frame owner. Camera authority, death/respawn, pause and active contexts need inspection before editing.

## Delivery gate (plan before code)
- Scout exact input-context ordering for wheel lines and trackpad pixels, RCS and combat precedence, camera authority, pause states, respawn, camera clipping and active look ray; show paths, existing and proposed types/functions/signatures and before/after graph. Resolve step size/accumulation and interpolation from device and rendered evidence rather than inventing a default.
- Change the owning camera rig and wheel dispatch first; replace obsolete wheel-in-Normal/Alt paths rather than allow duplicate actions. Keep camera minimum outside the hull's physical envelope and refuse non-finite zoom inputs. Do not alter the cockpit or player aiming without named evidence.
- Work only in the dedicated zoom Sprout; do not change the active themed-interface Sprout. Implementation remains gated on the reviewed plan. Do not compete with the UI worker's build lane.

## Verification
- Pure bounds/step tests: wheel up/down, 1x and 8x clamps, non-finite refusal, orbit entry/manual override/exit restore; no timing assertions.
- Asserted ECS input matrix: Normal, Alt, combat, and each with RCS held; one wheel action has exactly one owner; brackets and D-pad still cycle targets; modal, editor and pause consume no flight-camera zoom. Check respawn and hull-size clearance.
- Matched rendered captures of minimum, intermediate and maximum zoom for a small ship and a large hull, Normal versus Alt, combat entry/exit, and auto/manual ORBIT. Inspect the ship, contacts, occlusion, far plane and camera motion. Update input hints/docs for the wheel's new per-mode role.

## Evidence (2026-09-26, branch flight-camera-zoom)
- Focused tests pass: `nova_ship` camera, wheel, hints, bindings, component lock and flight rig (77); `nova_scenario` `loader::lifecycle` (25); `nova_input` dispatch (9); `nova_hud` keybind_dock (21). `cargo fmt --check` is clean.
- `content gen` changed only the gunnery drill line; `content lint` reports 0 errors and 0 warnings.
- A disposable render spike (not committed) put a salvage skiff and the industrial carrier in the rock hollow on hardware Vulkan under Xvfb and sent wheel lines. Logged rig distance: 1x 20.6, 2x 41.2, 4x 82.5, 8x 164.9 engine units on both hulls. Eight more lines past the cap held 164.9. Free look at 8x gave 253.0. Combat gave the authored rig (skiff 11.2, carrier 19.9). Forty lines back in returned to 20.6.
- Inspected frames: at 8x the whole carrier sits in frame with the hollow around it; the skiff stays visible at the frame centre with its engine glow readable. At free look 8x the skiff is about 25 px wide. Past-cap frames match 8x, and back-in frames match 1x, pixel for pixel. No far-plane cut. Rocks can sit between the camera and the ship at 8x; no camera collision exists at any level.
- Not rendered: ORBIT auto and manual framing (no gravity well in the spike scene), and camera motion between stills. The unit test `orbit_zoom_starts_from_the_survey_and_leaving_restores_the_manual_level` covers the orbit policy.

## Done when
- Owner reviews the code-backed implementation plan and any remaining numeric/input policies, the approved implementation and permanent focused proofs pass, rendered comparisons support 8x or a revised approved cap, and affected docs/content are current. A gamepad gesture and saved zoom are not required.
