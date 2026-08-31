# Stow the PDC out of combat

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.13.0, ship, gameplay, art

## Goal

A PDC that is not fighting stows into the hull, and deploys when it is
needed. The ship at rest should read as at rest.

Owner framing (2026-08-31): "maybe PDC to go inside the ship during no
combat, basically hide itself or something". The mount also wants more
geometry around it so the stow has something to disappear INTO - that art
overlaps with the section model pass.

Disposition (2026-08-31 planning round): a wish, not needed - a cool idea.
Stays in the backlog as a future promise beyond v0.14.0. The section model
pass (`20260831-083625`, v0.13.0) does not wait on it: it models the
turret mount with room for a future stow and records the shape decision
there.

Re-slotted into v0.13.0 (owner, 2026-08-31 evening): "let's do it now" -
the bay's iris doors landed the section animation interface the stow was
waiting on, and the owner settled the design the same day.

## Owner design (2026-08-31, agreed in review)

- The PDC starts STOWED and deploys "when I enter combat mode or have
  lock on something", then stays deployed for the fight.
- Housing: a 1x1 footprint, 0.5 tall box the turret disappears into -
  "make the PDC fit in its original cube but as a 0.5x1x1". The current
  mount is a ~0.3 box with nothing around it. Collider stays the
  authored spec; growing the hitbox is a separate balance call, recorded
  as deliberately NOT taken here.
- Stow choreography: barrel points up, the assembly slides down, doors
  shut above it. Deploy is the exact reverse.
- The lids are two sliding slab halves that part along the top and store
  flush against the flanks - a TRANSLATE read, deliberately distinct
  from the bay's rotating petals at silhouette range.
- Ownership split: the barrel-up is a commanded aim attitude through the
  existing look system (SmoothLookRotation REPLACES rotation, so a
  track composed onto the joints would fight it); the animation tracks
  own only what the aim stack does not - the lids and the lift.
- Sequencing lives in the kind system, not in track data: a state
  machine (Deployed / Stowing / Stowed / Deploying) steers the StowLift
  and StowDoors cues in order and advances phases by reading
  cue_progress, the same trick as the bay's ejection gate.
- Deploy on weapons hot OR a live tracking target (point defense comes
  up autonomously against an inbound); stow only after weapons are cold
  AND no target for a few settle seconds. Deploy fast, stow lazy.
- A turret cannot track or fire until fully deployed - the deploy time
  is a real combat cost.
- Rest pose = deployed (editor, gallery and animation-less apps keep
  showing the gun); scenes start stowed via a snap-to-stow when the rig
  first resolves, so a cold start has no spawn wiggle.

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
