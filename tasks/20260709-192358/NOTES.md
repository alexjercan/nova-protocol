# Component fine-lock on turret auto-lock (VATS-lite)

> SUPERSEDED IN PART (2026-07-13, deliberate-radar spike 20260713-082207,
> tasks 20260713-082324/-082330/-082337): ACQUISITION changed - there is no
> aim-assist cone pick or close-range signature auto-lock anymore; the combat
> lock is taken deliberately (raise RMB, hold CTRL radar, release to commit)
> and lives in the ship-root `CombatLock` component (the
> `SpaceshipPlayerTargetLock`/`...ComponentLock` resources are gone). The
> section FINE-LOCK layer this doc describes - focus dwell, snap with
> hysteresis, wheel/bracket cycling, pin window, HUD markers - survives
> unchanged on top of the combat lock.

- TASKS: 20260709-150711 (live-structure anchor), 20260709-192503 (targeting
  module + hybrid acquisition), 20260709-192522 (focus dwell + fine-lock),
  20260709-173700 (turret feed), 20260709-192523 (HUD)
- SPIKE: tasks/20260709-192358/SPIKE.md

## The mechanic, end to end

Lock a ship by aiming at it (as always), or just fly close: with nothing in
the aim cone, the nearest AI ship inside `TARGETING_SIGNATURE_RANGE` (550 m)
auto-acquires - the heat-signature close-range lock. Hold the lock for
`FOCUS_TIME` (1.5 s) while a thin meter under the reticle fills; when it
completes, one small hot-red marker appears over every attached section of
the target and the fine-lock layer is live. The fine lock follows the section
nearest your crosshair (snap, with `SNAP_HYSTERESIS` 0.75 so it does not
flicker), or step it deliberately with `[` / `]` (dpad left/right), which
pins the choice for `COMPONENT_PIN_WINDOW` (2 s). Turrets auto-fire at the
fine-locked section, else the locked ship's live structure, else your
crosshair ray as before - and they finally receive the target's velocity, so
turret lead (and the lead pip) is a real intercept.

## Where each piece lives

- `sections/mod.rs::live_structure_anchor` - the shared COM-lift anchor used
  by the chase camera, all AI aim reads, the player lock cone and the turret
  feed. The root ORIGIN is only the build spot of the first sections; aiming
  at it shoots empty space once they die (bug 20260709-150711).
- `input/targeting.rs` - the lock resource (`SpaceshipPlayerTargetLock`,
  renamed from the torpedo-specific name), hybrid acquisition
  (`pick_target` cone + `pick_signature_target` fallback), the focus dwell
  (`SpaceshipPlayerLockFocus`), the fine lock
  (`SpaceshipPlayerComponentLock`, Snap | Pinned) and the cycle input
  observers. All selection rules are pure functions with unit tests.
- `input/player.rs::update_turret_target_input` - the three-tier turret feed
  (component -> ship anchor -> camera ray) plus the velocity feed into
  `TurretSectionTargetVelocity`.
- `hud/component_lock.rs` - marker layer (screen-indicator consumers,
  `Entity` anchors on section entities), reconcile membership while focused,
  highlight the selection. `hud/torpedo_target.rs` gained the focus meter
  under the reticle.

## Behavior deltas (enumerated per consumer)

- Player turrets no longer slave to the crosshair while a lock exists: they
  fire on the lock (ship anchor) or the fine-locked section. The crosshair
  path remains when unlocked - manual gunnery is the no-lock tier.
- The lock can now exist without aiming (signature range); torpedoes
  launched hands-free commit to that lock like any other.
- Turret lead is real: `TurretSectionTargetVelocity` was zero in all game
  code before; both lock tiers now feed the lock root's `LinearVelocity`, so
  `lead_intercept_point` stops degenerating to the aim point and the amber
  lead pip shows actual lead (gap filed as 20260709-173700 during the
  weapons-HUD arc, closed here).
- AI ships aim at the player's live structure everywhere (chase direction,
  thrust gating, turret input, fire alignment) instead of the root origin.
- HUD: a focus meter (48x4 px, under the reticle, hot-red fill) while the
  dwell runs; 10 px section markers (16 px + brighter when selected) while
  focused. Palette: untinted reticle, nav-cyan destination, amber lead pip,
  hot-red component layer.

## Tuning constants (all in input/targeting.rs unless noted)

- TARGETING_SIGNATURE_RANGE 550 m (cone range stays 2000 m)
- FOCUS_TIME 1.5 s, COMPONENT_PIN_WINDOW 2 s, SNAP_HYSTERESIS 0.75
- Marker/meter sizes and colors in hud/component_lock.rs and
  hud/torpedo_target.rs

## Verification

- 60+ unit/behavioral tests across targeting, input and hud modules: pure
  selection rules (cone, signature, snap hysteresis, cycle order), dwell
  accrual/reset, every fine-lock clearing path, the three turret-feed tiers
  with fall-through, marker reconcile/highlight, meter windows.
- Scripted range `examples/12_hud_range.rs` (Xvfb): meter filling at 49%
  mid-dwell with zero markers, then 3/3 markers and no meter after focus,
  turret aim point within 5 m of the locked ship's anchor (world-space
  discriminator - dead-ahead geometry projects both tiers to the same
  pixel), a script-pinned tail section rendering the 16 px highlight, and
  every marker gone after the target dies.

## Deliberately deferred

- Radial focus ring (image/shader tech the UI pass lacks; the bar ships).
- AI component picks (AI aims at structure; per-section AI targeting later).
- Faction-based hostility (signature lock is AI-ships-only until
  20260708-203708); acquire/lock audio cues (ride the SFX events when the
  audio tasks land).
- Own-ship anchor for the AI's chase-vector origin (noted on
  20260709-155921).
