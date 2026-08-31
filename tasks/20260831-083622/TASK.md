# Stow the PDC out of combat

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

## Goal

A PDC that is not fighting stows into the hull, and deploys when it is
needed. The ship at rest should read as at rest.

Owner framing (2026-08-31): "maybe PDC to go inside the ship during no
combat, basically hide itself or something". The mount also wants more
geometry around it so the stow has something to disappear INTO - that art
overlaps with the section model pass, so the two tasks should agree on the
turret before either commits to a shape.

## This is a combat change wearing an animation costume

Scope it as gameplay, not art. A stowed PDC cannot fire, so there is a
deploy delay before the first round, and that moves numbers that are already
published:

- The point-defence envelope and the intercept window. The wiki's
  torpedo-run widget (`web/src/wiki/sections/torpedo-bay.md`) is built from
  measured rounds-to-kill and where each torpedo type dies. A deploy delay
  changes both, so the widget and the measured table at the head of
  `crates/nova_authoring/src/base_content/sections/ordnance.rs` have to be
  re-measured, not hand-edited.
- The AI has to decide to deploy BEFORE it needs to shoot, which is a new
  kind of decision for `input/ai/guns.rs` - it currently reasons about
  whether to fire, not about readiness.
- A stowed turret is arguably harder to hit. That touches damage and the
  section's collider, so decide it explicitly rather than inheriting it.

## Shape

- A state on the turret section: stowed / deploying / deployed / stowing,
  with authored deploy and stow times.
- Name the set `TurretSectionSystems` ordering per AGENTS.md and state the
  cross-plugin ordering against the AI input explicitly.
- Author the times per turret so a light PDC snaps out and a heavy mount
  does not.
- Decide whether stowing is automatic (no target for N seconds) or ordered.
  Automatic is the cue the owner asked for; ordered is what a player would
  want during a stealth approach. They can coexist.

## Done when

- A PDC visibly retracts out of combat and deploys before it fires.
- Deploy delay is authored, not hardcoded.
- The AI deploys ahead of need and does not waste the first engagement.
- The intercept numbers are RE-MEASURED and the wiki widget updated from
  the measurement.
- Frame time measured before and after if the stow adds any per-frame work.
