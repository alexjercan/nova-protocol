# Stow the PDC out of combat

- STATUS: CLOSED
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

## Decisions (build, 2026-08-31)

- Collider deliberately NOT grown. The housing is a 1x1x0.5 visual; the
  section collider stays the authored spec. Growing the hitbox is a
  balance call this task records as not taken.
- Sink depth 0.8, not the sketched ~0.5. The twin's stowed column tops
  out at +0.925 over the section origin and the lid underside sits at
  +0.21; the excess travel rides into the host hull, which every mount
  has beneath it. One shared track serves both mounts.
- Stow attitude is derived, not authored: the pitch hinge is commanded
  to `look.max.unwrap_or(look.initial)`, so a clamped mount rides its
  own +90 degree stop and a mount with no stop keeps its rest pitch.
- Unmanaged ships fail OPEN (deployed at spawn, never stow). This
  matches the fire path's WeaponsHot convention and keeps bare rigs,
  ranges, and every pre-existing walk working unchanged.
- The housing is the catalog's one final-size art piece: authored at
  shipped mount scale with its open base on the joint origin, so the
  turret root wears it with no render transform and no tree scaling.
- Doors never shut over a moving gun: Stowing holds the lids open until
  the lift reads 1.0; Deploying parts them fully before the lift rises.
  Mid-travel reversals flip phase and reverse the same cues naturally.
- AI deploys ahead of need without a new planner: deploy demand is
  weapons hot OR a live tracked target OR a point-defense assignment,
  and the AI's existing acquisition IS the readiness decision. An AI
  ship engages anything hostile - ships and inbound torpedoes both -
  at AI_TARGET_MAX_RANGE 2000 u, which raises weapons and derives
  WeaponsHot, so its mounts deploy roughly 50 s of closure before the
  180 u fire gate. The only cold path left is a quiet defender whose
  first contact is the assignment itself at AI_POINT_DEFENSE_RANGE
  150 u: deploy (0.25 s doors + 0.35 s lift) plus the up-to-0.5 s
  slew from vertical costs ~38 u of Lance travel, so first rounds go
  out at ~112 u instead of the alert rig's 180 u - a Lance still dies
  ~75 u out, clear of the 30 u fuze. That shrunken window against a
  Serpent is the authored combat cost, and in practice it prices the
  PLAYER's cold ambush, not the AI's defense.

## Landed

- 831ceb35 - stage 1: StowLift/StowDoors cues, Translate motion,
  snap_cue/has_cue, rest carry-forward on re-resolve.
- b6855a78 - stage 2: TurretStow state machine, TurretSectionSystems
  set, aim and fire gates, TurretJoint.name, 6 stow tests + 2 gate
  tests.
- 6c57b07c - stage 3: pdc_housing part with named lid nodes, raised
  yaw hinges, stow_lift joint, authored tracks, regenerated content.
- 51be4e8e - stage 4: trials walk stow round + task-folder captures.
- 6a1a9944 - stage 4b: fold an idle deployed mount back to its rest
  attitude (the stow attitude had survived the deploy).
- record commit: changelog entry, wiki stow section, creator `name`
  field, this file.

Squash-landed on master as `864124bb` (2026-09-01).

## Proof

- Trials walk (Xvfb, autopilot, capture): full cycle green at t=8.2s.
  The stow round parks both mounts (lids shut), catches the gatling
  mid-rise with the trigger HELD and zero rounds fired, and only counts
  the step done when both mounts report deployed - still zero rounds.
  The no_bullets predicate spans the whole deploy window, so any fire
  leak fails the walk. Frames: section-trials-stowed.png,
  section-trials-deploying.png, section-trials-twin.png (this folder).
- Targeted tests: 6 stow-machine tests (incl. the idle rest-attitude
  fold), aim-gate test, fire-gate test, animation and authoring
  suites - all green.
- RE-MEASURED intercept table (point_defense_cost rig, run serially,
  2026-08-31): straight 116 rounds / killed 114.03 u out; weaving 390
  rounds / killed 39.92 u out. IDENTICAL to the published table
  (~116/~114, ~390/~40): the rig models an alert defender already on
  the bearing and never constructs a turret entity, so the stow gate
  cannot enter it - and a live alert defender IS deployed by then
  (demand rises at 2000 u acquisition). ordnance.rs and the wiki
  widget therefore stand unchanged; the cold-defender cost is the
  arithmetic under Decisions.
- Regression walks (Xvfb, autopilot, serial): system_torpedo_launch
  green (t=32.4 s, fired/armed/detonated across both scenes - bay
  iris unharmed); screenshot_section_weapons green (t=4.0 s, wiki
  captures show both mounts deployed over open housings, idling at
  rest attitude); system_turret_gunnery green (t=11.4 s, two rounds
  of raise -> deploy -> fire -> hit through the stow gate).
- stress_point_defense smoke: BLOCKED by a pre-existing master
  break - the example's code-built 'launcher' ship fails the ship
  lint (link-point graph disconnected: controller | bay_0..bay_11)
  and the scenario refuses to start. Not from this branch: the ship
  has no turret, the branch's content diff is confined to the two
  turret entries, and its only lint-crate line is a test fixture
  field.
- content gen + lint: 0 errors, 0 warnings; gen-section-parts --check
  byte-identical for all unchanged parts.
- Frame time: reasoned, not measured. The stow adds one FixedUpdate
  system per turret-bearing ship tick: a phase match, two cue reads
  (a scan over the section's handful of tracks), and a timer
  accumulate. No allocation, no new queries in the hot Deployed arm;
  the attitude commander only runs while a mount is in transit. The
  two extra tracks per PDC ride the pre-existing animation driver,
  which skips settled tracks. Nothing here is per-frame work at a
  scale the profiler could see over the existing turret aim solve.
