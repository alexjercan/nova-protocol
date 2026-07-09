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
