# Review: Turret crosshair (orange square) twitches while tracking

- TASK: 20260710-231929
- BRANCH: fix/crosshair-same-frame-aim

## Round 1

- VERDICT: APPROVE

Verified with fresh eyes:

- The design insight is correct and non-obvious: with UI layout preceding
  transform propagation, the aim chain's old `.after(Propagate)` slot
  made a same-frame pip STRUCTURALLY impossible - moving only the pips
  (the plan's original idea) could never fix the bug. Converting the aim
  chain to TransformHelper and publishing before the projection is the
  same pattern 20260710-231928 landed, applied consistently.
- Dependency direction preserved: sections export the public
  `TurretSectionAimSystems` set; hud orders against it. No hud knowledge
  leaked into sections.
- Consumers audited by the reviewer: the AI Update-schedule readers see
  the aim point with identical freshness as before (published last
  PostUpdate); rotator target semantics are value-identical (fresh
  composition equals post-propagation values); nova_debug gizmos
  untouched.
- The regression uses the REAL TurretLeadPlugin wiring, so the A/B
  (re-registering pips in Update fails the same-frame equality on the
  first measured frame) exercises production registration, not test-local
  wiring. Delivery guard (intercept must move every frame) doubles as the
  falsification of the solver-oscillation hypothesis and is recorded as
  such in TASK.md.
- Reviewer ran the full lib suite 357/357; diff ASCII-clean; rig updates
  (Transform instead of GlobalTransform spawns) follow the established
  pattern from the two prior cycles; pub(crate) widening cites the
  thruster-system precedent in a comment.

No findings.
