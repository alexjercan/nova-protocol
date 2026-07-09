# Focus dwell + component fine-lock state and selection

- STATUS: CLOSED
- PRIORITY: 56
- TAGS: v0.4.0, targeting, gameplay, spike

Spike: docs/spikes/20260709-192358-component-lock-vats-lite.md

The core mechanic: a focus timer that accumulates while the ship lock stays on
the same entity (focused when >= FOCUS_TIME, reset on lock change/break), and
a component fine-lock (Option<section Entity> of the locked ship) that is only
available while focused. Selection is aim-snap by default (nearest live
section to the crosshair ray, with hysteresis) plus cycle next/prev keys that
pin the selection and suppress snap for a short window (snap resumes ~2 s
after the last cycle press or when the pinned section dies). Validity: the
section must stay an attached child with Health; decide at plan time whether
`SectionInactiveMarker` (disabled-in-place) sections stay lockable. Depends
on: 20260709-192503 (acquisition) for the lock semantics it rides on.

## Steps

- [x] State in `input/targeting.rs`: `SpaceshipPlayerLockFocus { target:
      Option<Entity>, seconds: f32 }` resource ticked by a system - seconds
      accumulate via `Time` while the lock stays on the same entity, reset on
      change/None; `FOCUS_TIME` const (start 1.5 s); focused = seconds >=
      FOCUS_TIME. Runs after the acquisition system.
- [x] `SpaceshipPlayerComponentLock { section: Option<Entity>, mode:
      Snap | Pinned { until: f32 } }` resource + validity system: cleared
      whenever unfocused, the section despawns, leaves the locked ship, or
      its Health hits zero. Decide here whether `SectionInactiveMarker`
      sections stay lockable (spike leans lockable-while-attached).
- [x] Snap selection system (while focused, mode Snap): nearest live section
      of the locked ship to the crosshair ray (ship anchor origin +
      `PointRotationOutput` aim, the acquisition ray), by point-to-ray
      distance, with hysteresis - only switch when the challenger is closer
      than the incumbent by a margin factor (start 0.75x). Pure helper for
      the pick + hysteresis so it is unit-testable.
- [x] Cycle input: `ComponentCycleNextInput` / `ComponentCyclePrevInput`
      InputActions (bevy_enhanced_input, player.rs actions! pattern) bound to
      BracketRight/BracketLeft + gamepad bumpers; a press steps the selection
      through the locked ship's live sections in a stable order (sort by
      local translation z, then x, then y) and sets mode = Pinned { until:
      now + PIN_WINDOW (2 s) }; pin expiry or pinned-section death resumes
      Snap.
- [x] Tests (world tests, advance `Time` manually): focus accumulates and
      resets on lock change; component lock clears on unfocus/section death/
      ship change; snap picks nearest-to-ray and respects hysteresis; cycle
      steps the stable order and pins; pin expires back to snap.
- [x] Verify: cargo fmt, cargo check --workspace, new targeting tests only
      (report skips).

## Notes

- Depends on: 20260709-192503 (acquisition + targeting module it rides on).
- HUD rendering of all this state is 20260709-192523, not here; turret
  consumption is 20260709-173700.

## Resolution (20260709)

Shipped in input/targeting.rs: SpaceshipPlayerLockFocus (dwell timer with
fraction()/focused_on(), FOCUS_TIME 1.5 s, reset on lock change),
SpaceshipPlayerComponentLock { section, mode: Snap | Pinned { until } },
chained acquisition -> focus -> component systems, pure helpers
(ray_distance with behind-origin clamp, snap_pick with SNAP_HYSTERESIS 0.75,
cycle_order by local z/x/y), cycle InputActions on the flight rig
(BracketRight/Left + DPadRight/Left) whose observers step + pin
(COMPONENT_PIN_WINDOW 2 s). Decision recorded in the component-lock doc
comment: sections stay lockable while ATTACHED, including
SectionInactiveMarker ones (blow disabled sections off the hull); despawn/
detach clears. 9 new tests (focus accrual/reset, ray/hysteresis math, snap
wiring, cycle order + wrap + pin, pin expiry, pinned-section death, lock
loss); 21 targeting / 29 input tests green.

Skipped honestly per user instruction: full local suite and clippy (check +
fmt + new/touched tests). One compile round lost to `Time::default()`
ambiguity in tests (needs Time::<()>). No aim rig in a state (menu/headless)
holds the current selection instead of guessing - noted in
update_component_lock.
