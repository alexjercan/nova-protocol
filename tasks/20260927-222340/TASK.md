# Debounce the binary chase-camera burn push

- STATUS: OPEN
- PRIORITY: 65
- TAGS: camera, flight, bug

## User report
- Main-drive pulses during autopilot deceleration repeatedly change the chase-camera burn push, causing visible zoom flicker. A short Space thrust press should not engage push; sustained thrust should engage it. After release, a short gap should not disengage it. Burn push should be binary, not proportional to throttle or acceleration. Keep manual wheel zoom, planned ORBIT survey, Turret behavior, hull clearance and camera smoothing intact.

## Code map and boundary
- `crates/nova_ship/src/camera/framing.rs:248-255` scales burn push by authored acceleration and live heat. `update_camera_rig` at lines 303-393 reads forward main-drive inputs and composes the push into the chase offset each frame. `crates/nova_ship/src/camera/framing.rs:441-511` tests immediate full-on/full-off transitions; revise proof to match the new behavior.
- `crates/nova_ship/src/camera/zoom.rs` owns session wheel/ORBIT scale and must not absorb this separate burn state.
- Intended end state: the varying, heat-scaled push dies. `BurnPush` on the player root is stepped in `FixedUpdate` from `MainDriveCommanded` and eases one flat rig fraction (15%) out and home.

## Decisions
- Asymmetric push (owner, 2026-09-28), superseding the engage debounce in the user report: the first commanded tick engages and eases out at the 6/s spool-up rate. A quiet tick while engaged holds the extension. 0.5 s of continuous quiet releases (15/32/60 ticks at 30/64/120 Hz), then it eases home at 10/s. A 1-tick tap at 64 Hz leans 9% of the push; station-keeping trims spaced over 0.5 s each lean and return.

## Delivery gate
- In the isolated Sprout, reproduce a pulsed-burn before trace/test, show exact owning paths/types/signatures, before/after call graph and failure proof. Present timing/criterion options and a recommended default for review. Wait for approval before adding a type, function, test, or changing behavior.
- Then implement approved design, run focused ship camera tests and rendered autopilot/Space comparisons, inspect frames and traces, update invalidated docs/changelog, commit explicit paths, and open a small PR against master. No merge.
