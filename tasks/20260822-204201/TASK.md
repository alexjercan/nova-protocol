# Make particle effects credible in vacuum

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.12.0,art,vfx

Rewritten 2026-08-24 for v0.12.0 with the round 4 inventory:
`tasks/20260815-231945/CONTENT-AND-ART.md` section 3. The torpedo baseline
this task used to scope is LANDED (88d7322a, b374c172): vacuum ejecta, no
blast sphere, punch pass accepted.

Re-planned 2026-08-30 from an annotated pairing session. The owner decisions
and the measurements behind them are in `## Decisions` and `## Findings`.

## Goal

Audit every player-visible effect and give each an explicit vacuum role -
brief flashes, incandescent ejecta, vapor, fragments, directional momentum;
no rolling fireballs, no gravity-driven smoke. Keep effects readable at
gameplay distance.

The reference is The Expanse: torpedoes are nuclear and cold-ejected before
the drive lights, and the PDC fires heavy rounds at a rate that reads as a
strobe rather than as separate shots.

## The inventory (the audit's gift: it is SMALL)

Exactly three hanabi graphs exist, all built-in defaults - no shipped
content overrides any of them:

1. Torpedo blast (torpedo_section/render.rs:247-318) - already retuned for
   vacuum; the reference treatment.
2. Torpedo launch puff (render.rs:395-503) - cold propellant flash. NOTE:
   its default asset is minted PER BAY SPAWNER (render.rs:484-494) unlike
   the other two shared assets - the cheap sharing fix belongs to stage 4.
3. Turret muzzle flash (turret_section/render.rs:353-409) - 3-px
   screen-space dots.

Non-hanabi families the audit also owns: damage sparks
(damage_sparks.rs, threshold :38), damage cracks, damage plume, the shader
exhaust plume (thruster_section.rs:119-170), juice impact rings + shake
(nova_gameplay/juice.rs:134-141, shake.rs), and carve shards
(integrity/spew.rs). The authored vocabulary is `DamageEffect`
(damage_effects.rs:59-74: Cracks, Sparks, Plume).

## Decisions

Taken by the owner in the 2026-08-30 session. Each overrides any earlier
note in this file.

- **The gizmo impact rings go.** They are the most-seen effect in a fight
  and the least real. Replaced by a spark burst at the hit point. Camera
  shake stays - it is the other half of `juice.rs` and it is not what was
  objected to.
- **Carve shards stay, and stop being cold grey.** They carry a cue sparks
  cannot: material came off, and it went that way. Shards are the mass, the
  new sparks are the flash, and they run together.
- **A shard's look comes from the body it came off.** See `## Findings`:
  a rock currently throws metal chips. Resolved with a query, not a match
  on a body-type enum.
- **The torpedo really cold-launches.** Not a visual trick. It is ejected
  from the bay, coasts with the drive dark, then ignites. While ejecting it
  is inert: not armed, cannot be damaged, cannot damage the ship.
- **The PDC keeps 100 RPS.** The rate is not what makes it illegible; the
  tracer is. See `## Findings`.
- **Dynamic light is in.** Measure frame time before and after every stage
  that adds any.

## Findings

Measured 2026-08-30 off the current tree.

**The PDC stream is 12% lit.** `fire_rate` 100/s and `muzzle_speed` 100 u/s
put 1.0 unit between rounds; the kinetic round mesh
(`round_mesh(0.025, 0.09, 0.03)`) is 0.12 units long. At 60fps a round also
travels 1.67 units per frame - further than the gap - so no cadence produces
a continuous stream. Lowering the rate widens the gap and makes it worse.
The fix is to stretch the tracer along velocity by about `velocity * dt`,
which is frame-rate correct by construction. A round is integrated and swept
by hand with no rigid body and no collider (`rounds.rs:1`), so the visual is
decoupled from hit detection and stretching it is free.

**Nothing in combat casts light.** `PointLight` appears nowhere in
`nova_ship` or `nova_gameplay`. A detonation that does not light the hull
beside it cannot read as a warhead whatever the particle graph does.

**Rocks throw metal chips.** `asteroid_carve.rs:425` triggers `CarveSpew`
exactly as a hull hit does, and `spew.rs` draws every shard from one shared
`Cuboid` in `srgb(0.30, 0.30, 0.33)` at `metallic: 0.3`. `chunk.rs:157-160`
already hit this split and recorded the reason it cannot be papered over:
"a rock's pieces want its triplanar `ExtendedMaterial` and a section's want
a plain `StandardMaterial`". It solved it by handing the entity back and
letting the caller insert the material.

`spew_carved_material` already has `spew.entity` - the body that lost the
material - and already queries it for `GlobalTransform`. So the body
declares its own debris look as a component and the observer reads
`Option<&CarveDebris>` off it, falling back to metal. The asteroid spawner
inserts the rock look; sections keep metal. The single cached
`(mesh, material)` pair becomes a small keyed cache, the same shape as
`DefaultTorpedoRender` keying warhead materials by tint and `ExhaustMeshes`
keying flames. No content-format change in the first cut; an authored
per-section look can follow if a mod wants one.

**The launch puff is minted per bay spawner.** `insert_torpedo_spawner_effect`
calls `effects.add(...)` inside the observer, so every bay in the scene
allocates its own byte-identical `EffectAsset`. Blast and muzzle already
share one through a resource. Fold this in with stage 4.

**The blast inherits none of the torpedo's velocity.** Carried over from the
round 4 audit and still true. Puff and muzzle both carry a `base_velocity`
property; the blast does not.

## Order of work

Stages land in order. Cheap visible wins first, so the range harness and the
frame-time loop are both proven before stage 4, which is roughly the size of
stages 1 to 3 together.

### Stage 0 - the range, and a frame-time floor

`examples/screenshots/loop_vfx_range.rs`: a parked target, a fixed camera,
and one repeating cycle that fires muzzle, impact, launch and blast in
sequence. Plain run for eyeballing; `NOVA_CAPTURE=1` records a loop for the
docs site.

It claims the `frametime` capability, so the same example is the before/after
perf number. A fixed cycle is a better perf probe than a live fight because
every run replays the same events. Only `wfc_ships` claims frametime today,
so this is the gauge this work does not currently have.

Take the first measurement here. Re-measure after every later stage.
Measurement lanes run one at a time; a shared GPU makes concurrent runs
meaningless.

#### Measurements

`probe run loop_vfx_range --release --repeat 3`, host `nixos`, gpu backend.
Three passes per row, reported mean fps / mean ms and the worst frame:

| Stage | Commit | Pass means (fps) | Mean ms | Worst frame (ms) |
|---|---|---|---|---|
| 0 - baseline | a704bb57 | 56.9 / 61.0 / 57.1 | 17.57 / 16.39 / 17.50 | 37.90 / 26.05 / 29.92 |
| 1 - blast | 7e0658be | 63.2 / 64.7 / 66.7 | 15.81 / 15.46 / 14.99 | 30.46 / 27.65 / 27.62 |
| 2 - impact | 07be5d6d | 59.8 / 60.8 / 58.9 | 16.72 / 16.46 / 16.97 | 35.04 / 29.38 / 29.07 |
| 3 - muzzle, unseen | c92dbf0f | 63.2 / 64.7 / 64.8 | 15.83 / 15.47 / 15.42 | 45.29 / 25.34 / 24.02 |
| 3 - muzzle, drawn | 2cc9053e | 57.2 / 61.2 / 60.3 | 17.48 / 16.35 / 16.58 | 37.74 / 28.99 / 27.82 |

Read the spread before reading a delta: the three baseline passes differ by
7% in the mean with nothing changed between them, so anything under about
1 ms is inside this host's noise and needs the repeat count raised rather
than a verdict. 68 to 89 frames per pass ran two fixed steps, which is
`Time<Virtual>::max_delta` discarding time - the range already sits on that
edge, so a stage that pushes the mean past 20 ms will show up as clamping
before it shows up as frame rate.

**Rows 0 and 1 are not like for like, and the difference is not a win.**
Stage 1 shortened the torpedo leg from 150 frames to 70 to fit the loop
recorder's cap, so the two runs measure different spans of a different
script: at row 0 the 600-frame window ended part way through pass 3, at row 1
it covers the whole cycle with frames to spare. What the pair does establish
is a bound: a detonation light, a second blast graph and a texture fetch per
particle cannot have made the range faster, so their combined cost is smaller
than the script change that moved these numbers. Row 1 is the reference for
stages 2 to 4, which leave the script alone.

The absolute number is the one to hold onto: 15.0 to 15.8 ms mean, under the
16.67 ms that 60 fps costs, with the worst frame at 27 to 30 ms.

**Rows 1 and 2 ARE like for like, and stage 2 cost 1.3 ms.** Nothing about the
script changed between them, so the whole spread between 15.4 and 16.7 ms is
the spark burst and the shard cooling. The probe's own baseline gate agrees and
fired `fps_within_baseline WARN worst +13.2%`. That spends the entire margin
row 1 was sitting on: the mean is now at the 16.67 ms that 60 fps costs rather
than under it.

**Row 3 gives it back.** The mean returns to row 1 and the probe's gate flips
with it, from that WARN to `fps_within_baseline PASS improved; best -8.9%`.
Nothing was optimised to get there: the muzzle stage deleted 100 screen-space
dots per shot and put 32 world-space particles per BARREL PER FRAME in their
place, which is the cheaper thing to draw by an order of magnitude at this fire
rate. The velocity-stretched tracer costs one scale write per live round.

The 45.29 ms in pass 1 is warm-up and not a cost. It is the first-pass figure
only; passes 2 and 3 hold 25.34 and 24.02 ms, the two best worst-frames
anywhere in this table, and the pass-1 outlier is the render pipeline being
specialised for the new muzzle shader on the frame the gun first fires.

**Row 3 measured a flash nobody could see, and drawing it costs 1.0 ms.** At
c92dbf0f the muzzle graph was complete and correct and reached the screen as a
smear: particles are first drawn one simulation step after birth, so a 0.01 to
0.05 s life against a 0.022 s frame sampled both gradients past their peak
every time. Lengthening the life to 0.05-0.12 s is what put the flash on
screen, and it also multiplied the live particle count by about 2.4 - roughly
230 overlapping quads at the bore during a burst where there had been 96. The
orient fix in the same commit is the other half: the blast core had been drawn
edge-on as a sliver and now draws its full area. Both are fill the earlier row
never paid for.

So row 3-drawn is where the stage actually lands, and it lands back on the
16.67 ms line with `fps_within_baseline WARN worst +13.3%`. NOT accepted as
final: 32 particles a frame was picked while the flash was invisible, and 230
overlapping quads inside one 0.55-unit ball is the kind of overdraw that
halves cheaply. The count is the lever, and pulling it is deferred to after
stage 4 so it costs one rebuild instead of two.

The suspected cost is entity count, not pixels. A PDC hit throws 5 sparks and a
kill throws 20, each its own kinematic body living 0.28 s, so a sustained burst
at 100 rounds a second holds on the order of 140 of them at once on top of the
shards already in the air. Stage 3 cuts muzzle particles from 100 a shot to 32
and the per-barrel buffer from 2048 to 512, which should hand some of this
back; if row 3 does not recover it, the spark burst is where to look, and the
first thing to try is dropping the rigid body a spark does not need - it
carries no collider and nothing queries it.

### Stage 1 - the blast

Core flash, expanding cooling shell, afterglow, ejecta. One short-lived
`PointLight`, gated beside `GraphicsBudget::particles`. Give the blast the
torpedo's velocity, which it does not inherit today.

This is where the transient-light budget gets defined, because this is the
first effect that wants one. Nothing else gets a light until it exists.

### Stage 2 - impact

Delete the gizmo flash rings from `juice.rs`: the `FlashSettings` block,
`draw_juice_flashes`, and `flash_progress` / `flash_radius` / `flash_alpha`
with their tests. Keep the shake.

Add a spark burst at the hit point. This is new: `damage_sparks.rs` is a
STATE effect - a damaged section throws sparks continuously, faster the
worse it is - and there is no burst at the moment of impact today.

Give shards their look from the body they came off, per `## Findings`. Two
shipped looks: metal chips with an emissive that starts near-white and cools
to grey over the first third of life, and rock chips that are dusty, do not
glow, and spall slower and in greater number.

### Stage 3 - the muzzle

Drop `ScreenSpaceSizeModifier` and the 3-px dots. A world-space flash quad
at the barrel, a thin gas cone that reaches far and dies in milliseconds,
and the velocity-stretched tracer from `## Findings`. The rate stays at
100 RPS.

### Stage 4 - torpedo cold launch

The big one. A launched torpedo is currently `RigidBody::Dynamic` carrying
two health-bearing collider sections and it can be shot down
(`TorpedoShotDownMarker`), so "inert while ejecting" is a real state and not
a flag.

- An `ignition_delay` bay field, serde-defaulted; `torpedo_section/mod.rs`
  already uses that pattern at line 170.
- An ejection phase: the torpedo rides out of the tube on the bay's velocity
  plus a small impulse, drive dark, no collider, no health, taking and
  dealing no damage.
- Ignition: the drive lights and it accelerates away.
- The visible emergence from the bay wants art. Decide during the stage
  whether that is a bay animation, a tube mesh, or a scripted transform on
  the ejecting body.
- Share the launch-puff `EffectAsset` through a resource instead of minting
  one per spawner.

Time-to-target moves, so the AI launch envelope and the PDC intercept window
move with it. Re-check `input/ai/guns.rs` and the balance audit.

Landed. The state is real, not a flag: an ejecting torpedo carries
`TorpedoColdLaunch`, guidance, thrust, the weave and the fuze all filter on
`Without<TorpedoColdLaunch>`, and both section children carry avian's
`ColliderDisabled`. `ignite_cold_torpedoes` removes the marker and the
disables together and fires `TorpedoIgnited`, which drives an ignition
light on the bay it just left. `torpedo_sync_system` is deliberately NOT
gated - gating it commands the warhead to world identity, because
`ControllerSectionRotationInput` defaults to `Quat::IDENTITY` rather than to
"leave it alone". The launch puff is now one shared asset behind
`DefaultLaunchPuffEffect`, is camera-facing, and rides the ship that fired
it.

The ejection charge had to move for the drop to READ. At the authored 1 to 2
u/s the torpedo drifted about a unit in the 0.6 s coast: on screen it was
still at the hull, and the first frame anybody could see it in was already
the frame its drive lit. The authored bays now eject at 8 u/s, which carries
it about 4 units into the damping - several body lengths, clear of the hull,
and still well inside the 5-unit `arm_distance`.

A pre-existing steering defect surfaced here and is fixed in the same
change: the bay seeded `TorpedoSteering` from the projectile transform's
`forward()`, but a torpedo leaves along the bay's +Y while its own nose is
its -Z, so the attitude command was 90 degrees off the way it was thrown.
Under power from tick one nobody saw it - PN overwrote the seed before the
controller could act. A 0.6 s coast with guidance suspended gave the
controller the whole window to turn the warhead sideways, and the drive then
lit across the run-in and threw the flight 13 u off the line.

Known and deliberately NOT changed: AI point defence acquires on
`TorpedoProjectileMarker`, so a defender can lock a torpedo that is still
ejecting and its rounds pass through the disabled colliders. The coast
happens at the launching ship and the point-defence envelope is 150 u, so
this only bites at knife range. Changing AI acquisition is outside this
task.

### Cross-cutting, folded into each stage

Both were written into the accepted torpedo baseline as refinement targets.
Fix them as common VFX direction on every family they touch, not per-family:

- the hanabi extraction delay, which lands the first visible ejecta frames
  after the event;
- the square billboards at close range.

## Constraints

- Preserve authored effect overrides (blast_effect / launch_effect /
  muzzle_effect config fields) and WASM support - hanabi needs compute, the
  web build forces WebGPU (nova_core/Cargo.toml:28-37).
- Deterministic captures per family for isolated shots, impacts,
  destruction, and salvo load, before accepting each family. The pattern:
  examples/screenshots/loop_torpedo_blast.rs (scripted, seeded, re-capture
  reproduces frames). From stage 0 on, `loop_vfx_range` is the first stop.
- Base content RON is generated. Any content change edits the Rust builders
  and runs `content -- gen`.

## Done when

- Every shipped effect family has an explicit vacuum visual role recorded
  here.
- The two cross-cutting fixes are verified on every family they touch.
- The gizmo impact rings are gone and nothing references `FlashSettings`.
- A shard's look is decided by the body it came off, and a rock throws rock.
- A torpedo is visibly ejected, coasts inert, then ignites.
- Representative isolated and stress captures reviewed.
- Graphics tiers, concurrent-effect budgets and the transient-light budget
  are defined, measured, and documented.
- Frame time is measured before stage 1 and after every stage that adds
  light, with the numbers recorded here.
- Player and creator documentation reflects any authored-effect contract
  changes.
