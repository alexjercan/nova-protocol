# Feed locked-target velocity into player turret lead

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.4.0,turret,gameplay

Found during the review of the turret-lead-pip HUD (20260708-165701):
`TurretSectionTargetVelocity` defaults to zero and nothing in game code ever
writes it - only examples/08_turret_range.rs does. `lead_intercept_point`
therefore degenerates to the raw `TurretSectionTargetInput` point for both
the player path (input/player.rs `update_turret_target_input`, a point 100 m
down the camera ray) and the AI path (input/ai.rs). Consequences: turret
bullets aim at where targets ARE, not where they will be, and the new lead
pip sits on the crosshair instead of showing an actual lead.

Direction: when the player has a lock (`SpaceshipPlayerTorpedoTargetEntity`),
feed the lock's position into `TurretSectionTargetInput` and its
`LinearVelocity` into `TurretSectionTargetVelocity`; fall back to the camera
ray with zero velocity when unlocked. The AI path (20260709-155921 territory)
should feed its target's velocity the same way. The lead pip
(hud/turret_lead.rs) then shows a real intercept point with no HUD changes.

Rescoped (20260709) by docs/spikes/20260709-192358-component-lock-vats-lite.md:
the velocity feed lands as part of the turret auto-fire feed from the
ship/component lock - component lock -> section position, ship lock ->
live-structure anchor (150711 helper), no lock -> camera ray as today; lock
modes also feed the target root's LinearVelocity so lead_intercept_point
computes a real intercept. Depends on: 20260709-192522 (focus/component-lock
state), 20260709-150711 (anchor helper).

## Steps

- [ ] Rework the player `update_turret_target_input` (input/player.rs) into
      the three-tier feed: component lock -> that section's GlobalTransform
      translation; else ship lock -> the locked ship's live-structure anchor
      (150711 helper, querying the lock entity's transform +
      Option<ComputedCenterOfMass>); else -> the camera-ray point as today.
- [ ] Feed `TurretSectionTargetVelocity` alongside: the lock root's
      `LinearVelocity` in both lock tiers, `Vec3::ZERO` on the camera-ray
      fallback - `lead_intercept_point` then computes a real intercept and
      the lead pip (hud/turret_lead.rs) finally shows true lead.
- [ ] Tests: three-tier priority (component beats ship lock beats ray), the
      velocity feed values per tier, dead section/lock falling through to
      the next tier.
- [ ] Extend examples/12_hud_range.rs: with the target locked, assert the
      turret aim point now tracks the TARGET (pip near the reticle within
      tolerance) instead of the camera ray; the existing disable stage keeps
      asserting the pip hides.
- [ ] Verify: cargo fmt, cargo check --workspace, new + touched tests, one
      scripted 12_hud_range run under Xvfb (report skips).

## Notes

- Depends on: 20260709-192522 (component lock state), 20260709-150711
  (anchor helper).
- AI turret feed stays as 150711 left it (live-structure anchor, zero
  velocity); feeding AI target velocity can ride the AI rotation task
  (20260709-155921) later.
