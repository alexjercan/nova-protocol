# Turret crosshair (orange square) twitches while tracking

- STATUS: CLOSED
- PRIORITY: 80
- TAGS: v0.5.0, turret, bug

## Goal

Playtest bug (user, 2026-07-10): the turret crosshair (orange square pip)
jitters while tracking. Root cause
(tasks/20260711-103527/SPIKE.md): the pip
anchors are driven in Update (turret_lead.rs:78-84) from
`TurretSectionAimPoint`, which is computed in PostUpdate
(turret_section.rs:245-255) - the pip always shows the PREVIOUS frame's
intercept, then gets projected with the stale camera pose fixed in
20260710-231928. The user's "target calculation takes time" hypothesis was
close: the solve is fresh every frame, but its consumption is a frame late.

## Steps

- [x] Reorder the crosshair chain. IMPLEMENTATION DETAIL DISCOVERED: the
      aim chain sat `.after(TransformSystems::Propagate)`, but the
      projection (and UI layout) run BEFORE propagation, so merely moving
      the pips could never make the pip same-frame. The whole aim chain
      (intercept solve + rotator targets) was converted to
      TransformHelper-composed fresh poses (the 20260710-231928 pattern)
      and moved early in PostUpdate under a new public
      `TurretSectionAimSystems` set; the pips
      (`sync_turret_pips, drive_pip_anchors`) moved from Update to
      PostUpdate between that set and `ScreenIndicatorSystems`. The hud
      module orders against the sections set, keeping the dependency
      direction sections <- hud.
- [x] One-clock audit of the aim inputs: muzzle pose, rotator chains and
      the ship COM lift all read the render clock (fresh eased poses via
      TransformHelper); velocities stay raw (physics-correct lead).
      Convention documented at the query site: the pip marks the intercept
      as the player SEES it; the physical bullet spawns from the raw pose,
      sub-tick apart.
- [x] Regression `pip_anchor_carries_the_same_frame_intercept`
      (turret_lead.rs): real TurretLeadPlugin wiring + the real aim system
      under its production set, target crossing at 60 u/s; every frame
      the pip anchor must equal the SAME frame's freshly published aim
      point, with a delivery guard that the intercept moves. A/B-proven:
      re-registering the pips in Update fails the equality on the first
      measured frame.
- [x] Solver-oscillation hypothesis: the regression's delivery guard
      doubles as the check - the intercept advances monotonically with
      the target every frame (strictly > 0.1 u per frame in the rig, no
      flip-flop), and the closed-form solve has no iterative state to
      oscillate. Falsified, as the spike predicted.
- [x] cargo check (workspace) + fmt clean; full lib suite 357/357;
      spike doc fix record extended.

## Notes

- Evidence: turret_lead.rs:78-84 (Update, before ScreenIndicatorSystems),
  turret_section.rs:245-255 (aim point in PostUpdate after Propagate).
- Depends on: 20260710-231928 (projection moves to PostUpdate first).

## Resolution

What changed: the aim chain (intercept + rotator targets) runs early in
PostUpdate on TransformHelper-composed fresh poses under the public
`TurretSectionAimSystems` set; the lead pips consume it same-frame between
the aim chain and the indicator projection. `update_turret_aim_point` is
pub(crate) for the regression (thruster_impulse_system precedent). Aim
test rigs updated from GlobalTransform spawns to Transform spawns.

Alternatives considered: leaving the aim chain post-propagation and
double-computing the intercept for the pip in a pre-projection driver -
rejected (duplicate solver, and every consumer benefits from the earlier
publish).

Difficulties: none beyond deref/type plumbing in the new test; the
20260710-231928 pattern mapped directly.

Self-reflection: the plan's steps survived contact with the code this
time BECAUSE the 231928 cycle had already mapped the schedule terrain -
the verify-ordering-first habit is earning its keep.
