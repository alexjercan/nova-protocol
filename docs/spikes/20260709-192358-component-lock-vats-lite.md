# Spike: Turret auto-lock + component fine-lock (VATS-lite)

- DATE: 20260709-192358
- STATUS: RECOMMENDED
- TAGS: spike, turret, targeting, hud, gameplay

## Question

Can nova get a lightweight component-targeting loop - turrets auto-lock an
enemy, and focusing on it for a while lets the player fine-lock individual
sections and move between them - without building a full VATS-style UI or a
second targeting system? A good answer defines the lock/focus/selection state
model, how turrets and the AI consume it, what the minimal HUD is, and how it
reconciles with the already-planned dwell (20260708-165703), subtarget-cycle
(20260708-165705), aim-anchor bug (20260709-150711) and turret velocity-feed
(20260709-173700) work. Triggered by 150711: once aiming is per-section
instead of root-origin, component targeting is the natural next step.

## Context

What exists today, and what it constrains:

- **One player lock.** `SpaceshipPlayerTorpedoTargetEntity` (resource,
  input/player.rs) is an instant aim-cone pick over dynamic rigid bodies
  (ship ROOTS, torpedoes, asteroids): 18 degree cone, 2000 m, recomputed
  every frame, drops the moment the cone empties. The reticle, the readout
  and torpedo launches all consume it.
- **Turrets eat a point, not an entity.** `TurretSectionTargetInput
  (Option<Vec3>)` + `TurretSectionTargetVelocity(Vec3)` feed
  `lead_intercept_point` -> `TurretSectionAimPoint`. The player path feeds a
  camera-ray point 100 m out (turrets slave to the crosshair); the AI path
  feeds the player ship's ROOT ORIGIN (bug 150711 - empty space once front
  sections die). Nothing feeds target velocity in game code (20260709-173700).
- **Sections are ready-made components.** Every section is a child entity of
  the ship root with `SectionMarker`, its own `Health`, `GlobalTransform` and
  `ColliderAabb`, moving through a disable (`SectionInactiveMarker`) ->
  destroy -> despawn pipeline. A component fine-lock is literally an `Entity`
  reference to a section.
- **The HUD substrate fits.** The screen-indicator widget
  (docs/retros/20260709-screen-indicator-widget.md) renders an entity-anchored,
  ApparentSize-scaled marker per section with zero new projection code -
  component markers are just more consumers, in a different color and size.
- **No faction model yet** (20260708-203708): for the player, "enemy" is
  minimally an AI-controlled ship (`AISpaceshipMarker`); for the AI, the
  player. Proximity auto-acquisition needs exactly this and no more.

## Decisions (made with the user, 20260709)

1. **One shared ship lock.** The existing lock resource becomes THE target
   lock; torpedoes, the reticle/readout, and auto-mode turrets all consume
   it, and the component fine-lock refines it. Rejected: a separate
   CIWS-style turret lock (two locks, two reticles, two mental models) and
   per-turret locks (state and UI far ahead of today's single-target combat).
2. **Hybrid acquisition.** Aiming designates as today (cone pick wins); when
   the cone finds nothing, the nearest enemy inside a shorter "signature"
   range auto-acquires - the heat-signature close-range lock. Rejected:
   proximity-only (loses deliberate designation at range) and aim-only (no
   auto-lock at all).
3. **Focus gates the component layer.** Holding the same lock for T seconds
   fills a focus meter (WoT aim-in analogy); when full, component markers
   appear and fine-locking becomes available. Turrets aim at the ship's live
   structure before focus, at the locked component after. Focus resets when
   the lock breaks or changes. The planned lock-on dwell (165703) folds into
   this: the SHIP lock stays instant, the dwell moved one level down to gate
   the component layer. Rejected for now: progressive marker reveal (visual
   polish on the same mechanic) and a dispersion/accuracy bonus (nova has no
   dispersion mechanic; revisit with variable section damage, 20260525-133004).
4. **Component selection: aim-snap AND cycle keys.** Default is snap - the
   fine lock follows the live section nearest the crosshair ray inside the
   locked ship, with hysteresis so it does not flicker between adjacent
   sections. A cycle input (next/prev) steps deliberately and pins the
   selection, suppressing snap for a short window (default: snap resumes
   ~2 s after the last cycle press, or immediately if the pinned section
   dies). The pin/hysteresis constants are feel knobs to tune in playtest.

## Architecture

State (player side, mirroring the existing resource pattern):

- `SpaceshipPlayerTorpedoTargetEntity` stays the ship lock (renaming it to a
  general "target lock" is a mechanical refactor, noted below).
- New focus state: `{ target: Option<Entity>, seconds: f32 }` - accumulates
  while the lock stays on the same entity, resets on change/None; focused =
  seconds >= FOCUS_TIME.
- New component lock: `Option<Entity>` (a section of the locked ship) plus
  the snap/pinned selection mode. Valid only while focused and while the
  section is a live (`Health` > 0, not despawned) child of the locked ship;
  cleared otherwise.

Consumers:

- **Acquisition** (`update_spaceship_target_input`): cone pick as today;
  on a miss, nearest `AISpaceshipMarker` root within SIGNATURE_RANGE. At
  long range the lock still requires aiming, so holding focus at range means
  keeping the crosshair on target - the WoT feel falls out for free; up
  close the lock holds hands-free.
- **Turret feed** (player `update_turret_target_input`): component lock ->
  that section's position; else ship lock -> the live-structure anchor (the
  150711 helper: COM or surviving-section bounds center, shared with the
  AI); else camera ray as today. In lock modes also feed the target root's
  `LinearVelocity` into `TurretSectionTargetVelocity` - folding in
  20260709-173700, which makes `lead_intercept_point` compute a real lead
  and the lead pip (hud/turret_lead.rs) show it.
- **AI** (input/ai.rs): switches from player root origin to the same
  live-structure anchor helper (the other half of 150711). AI picking weak
  components is future work, not this arc.
- **HUD** (screen-indicator consumers, no new substrate): small
  entity-anchored markers on the locked ship's live sections in a distinct
  color, appearing when focused; the selected one highlighted; a focus meter
  on the reticle while focusing. Minimal meter = a thin bar in the readout
  column style (zero new tech); a radial ring is polish later. Acquire/lock
  audio cues ride the existing SFX events when wired.

## Options considered (beyond the user-decided forks)

- **Where the dwell lives**: gate the ship lock itself (165703 as written) vs
  gate the component layer (chosen). Gating the ship lock punishes the common
  case (torpedo snapshots) to make the rare case legible; gating components
  keeps the fast loop fast and makes depth opt-in.
- **Turret aim before focus**: keep slaving turrets to the crosshair vs aim
  at the locked ship (chosen, via the live-structure anchor). Auto-fire on
  the lock is the whole point of "auto lock on turrets"; the crosshair path
  remains as the no-lock fallback, so manual gunnery still exists.
- **Component markers always visible on lock** (no focus gate): noisier HUD,
  and removes the earn-it beat the user asked for. Rejected.
- **Do nothing**: leaves 150711 as a point fix and 165703/165705 as separate
  HUD chores; loses the coherent mechanic that ties them together.

## Open questions

- Tuning: SIGNATURE_RANGE (start ~500-600 m, inside TARGETING_MAX_RANGE
  2000), FOCUS_TIME (~1.5-2 s), snap hysteresis margin, pin window (~2 s).
  Playtest knobs, all constants.
- Should disabled-but-attached sections (`SectionInactiveMarker`) stay
  fine-lockable (finish them off) or be skipped (only live threats)? Leaning
  lockable-while-attached; decide at plan time for the state task.
- Renaming `SpaceshipPlayerTorpedoTargetEntity` to a general target-lock name
  now that three systems consume it - mechanical, fold into the acquisition
  task or leave.
- Focus meter visual: bar first, radial ring later; ring needs an image
  sequence or shader the UI pass does not have today.
- Gamepad/keys for cycle next/prev - decide with the input task.

## Next steps

Direction-level tasks (for /plan to break into steps), in intended order:

- tatr 20260709-150711 (existing, unchanged): live-structure aim anchor
  helper + AI/player consumers - the arc's opening task, fixes the bug on its
  own even if the rest waits.
- tatr 20260709-192503: hybrid lock acquisition (aim cone + signature-range
  proximity fallback, minimal hostile = AI ships).
- tatr 20260709-192522: focus dwell + component fine-lock state and selection
  (focus timer, component lock validity, snap + cycle-pin input).
- tatr 20260709-173700 (existing, rescoped): turret auto-fire feed from the
  ship/component lock, including the target-velocity feed.
- tatr 20260709-192523: component-lock HUD (section markers, selection
  highlight, focus meter) on the screen-indicator widget.

Reconciled existing tasks: 20260708-165703 (dwell) is superseded - its dwell
now gates the component layer (noted in the task, kept for the audio cue
polish); 20260708-165705 keeps only the multi-target list half, its
subtarget-cycle half lands here.
