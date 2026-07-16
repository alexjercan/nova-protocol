# Spike: Gauntlet Run 2.0 - a real parkour/slalom course from the existing vocabulary

- DATE: 20260716-174631
- STATUS: RECOMMENDED
- TAGS: spike, v0.7.0, scenario, content, modding

## Question

Gauntlet Run (webmods/gauntlet) is v0.6.0's first portal mod but a thin one:
four beacon gates in a straight-ish line over empty space, one skybox swap, no
obstacles and no fail state. The user wants it turned into a REAL gauntlet in
the Minecraft-parkour-map sense - many gates forming an actual route, obstacles
to thread, hazards that punish sloppy flying, escalating pacing, and a finish
worth reaching - while keeping it pure flying skill (no combat required).

The uncertainty is not *whether* to do it but *how far the existing data-driven
scenario vocabulary already reaches*: can a compelling 2.0 ship as a pure
content.ron rewrite + version bump, or does it need engine work (damage
volumes, moving obstacles, a timer HUD)? A good answer names the exact
authoring primitives 2.0 is built from, the authoring hazards that will bite,
and a single implementing task scoped tightly enough that /plan can turn it
into steps without re-litigating the approach.

## Context

Current mod (webmods/gauntlet/gauntlet.content.ron, v1.0.0): OnStart spawns the
player racer + START/GATE 1/GATE 2/FINISH beacons (each with `area_radius`), a
`gate` counter (1..=3) whose value gates each gate's OnEnter so they must be
threaded in order, objective/marker chrome, and one SetSkybox on gate 1. No
asteroids, no gravity, no collision consequence, no Victory/Defeat overlay - the
"finish" is just an objective line. The content.ron header carries a hard
INVARIANT: gate trigger areas must NOT overlap, or a pilot loitering inside the
next area when it arms produces no fresh OnEnter and the race soft-locks.

What the vocabulary already offers (verified this spike, file:line in the
research agents' output):

- **Asteroids** (`ScenarioObjectKind::Asteroid`, objects/asteroid.rs:21-55):
  real `Collider::trimesh_from_mesh` solid bodies (crashing into one deals
  bcs `on_impact_collision_deal_damage` kinetic damage - a genuine hazard),
  `surface_gravity: Option<f32>` turning a big rock into a gravity WELL (SOI
  ~8x geometric radius, pulls the ship/autopilot off-line - "sling or avoid"),
  `invulnerable: bool` for permanent obstacles that can't be shot away,
  authored `radius` + `health` + `lock_signature`. Geometric factor 3.5-6.0x
  nominal radius: a "radius 3" rock has a body up to ~18u.
- **ScatterObjects** (actions.rs:1906-2011): deterministic (seed) fan-out of N
  objects in a Box or Ring region, with an `asteroid_radius: (lo,hi)` range -
  the primitive for dense asteroid CORRIDORS and fields.
- **Beacons with `area_radius`** (objects/beacon.rs): ordered gates, the proven
  pattern already in the mod.
- **SalvageCrates** (objects/salvage.rs): self-triggering pickups on OnEnter -
  optional risky-line collectibles.
- **Outcome(Victory|Defeat)** (actions.rs:414-433, shipped by task
  20260716-125856): the real finish frame + a fail frame, plus overlay with
  Retry/menu buttons. NEW since the gauntlet was authored.
- **Triggers**: OnEnter/OnExit (gates, hazard zones), OnDestroyed (player ship
  crashed out -> Defeat), OnUpdate + a variable accumulator (a `time` counter
  for a time-trial), OnOrbit. No native time trigger; timers are variable-driven.
- **SetSpeedCap / SetControllerVerb**: pacing knobs (e.g. disable GOTO in a
  section to force manual threading).
- Three skyboxes exist for per-act sense-of-place: assets/textures/cubemap.png,
  cubemap_alt.png, cubemap_alt2.png (base assets, referenced as
  `textures/*.png` since the mod depends on base).

Republish path (research agent 2): bump `meta.version` in
webmods/gauntlet/gauntlet.bundle.ron (1.0.0 -> 1.1.0); nova_portal_gen
regenerates the versioned tree + catalog.json (scripts/preview-web.sh writes
web/dist/mods; deploy-page.yaml writes site/mods). Enabled state is keyed by
mod id, not version (mod_prefs.rs / mod_cache upsert), so an in-place update
preserves the user's enabled toggle - the "enabled-state-preserving update"
dogfood the task calls for. Validation gate: webmods_validation.rs drives every
webmods bundle through the real Bevy loaders to `Loaded`; broadside_assault.rs
is the model for a production-faithful behavior rig.

## Options considered

- **A. Pure-data expansion of content.ron + version bump (RECOMMENDED).**
  Rewrite the course as data: 6-9 ordered gates forming a real route with
  direction changes and verticality; ScatterObjects asteroid corridors between
  gates (invulnerable rocks so they stay as walls); one deliberate gravity-well
  section (a big `surface_gravity` rock to sling past or avoid) placed OFF the
  immediate gate line so it hazards without soft-locking; collision damage as
  the punish (reinforced hull already gives crash tolerance -> "run and fail");
  Victory outcome frame at FINISH and Defeat on player-ship OnDestroyed;
  escalating density/tightness across acts with a skybox swap per act. Zero
  engine changes - every primitive exists. Pros: fits the modding thesis (a mod
  is data), fastest path to a compelling course, reuses proven patterns. Cons:
  the authoring hazards below are real and need a test rig + playtest.
- **B. Add engine hazards first (damage volumes, moving/rotating obstacles,
  a visible timer HUD widget).** Richer feel, but each is engine work and a
  separate task; none is required for a course that already reads as a real
  gauntlet. Scope creep against a content task. DEFER.
- **C. Minor tweak (a couple more beacons).** Cheapest, but fails the explicit
  "make it a REAL gauntlet" ask - no obstacles, no hazard, no fail. Rejected.

## Recommendation

Option A, delivered through the existing task 20260716-124722 as a single
data-only content.ron rewrite + bundle version bump to 1.1.0 + republish, with
a production-faithful behavior rig (modeled on broadside_assault.rs) and a
hands-on playtest for feel/balance (the one thing a harness cannot judge -
same deferral shape as Broadside's "too hard" verdict).

Authoring hazards the plan MUST respect (these are the real risk, not the
feature list):

1. **No-overlap gate invariant gets harder with more, tighter gates.** Every
   added gate area must stay spatially disjoint from its neighbours, or a pilot
   loitering in the next area when it arms gets no re-enter and the race
   soft-locks. Shrink `area_radius` and space deliberately; the rig must assert
   pairwise non-overlap of all gate areas.
2. **Asteroid geometric factor 3.5-6.0x nominal.** A corridor authored on
   nominal radius can silently wall off its own gap at a high-factor seed -
   shakedown.rs hit exactly this (an orbit ring parked outside a gate,
   soft-locking a beat). Pick the seed, then MEASURE the flyable gap against
   the whole 3.5-6.0 range in the rig, do not eyeball it.
3. **Gravity-well SOI (~8x geometric radius) yanks ship and autopilot GOTO.**
   Keep wells off the immediate gate approach lines so they hazard the route
   without preventing a gate from being threaded.
4. **Collision-damage balance is feel, not math.** Enough hull to survive a few
   grazes, a full broadside into a rock should hurt, ship-death -> Defeat. This
   is a playtest verdict; ship the structure, tune the numbers by hand.
5. **Republish hygiene.** Confirm nova_portal_gen output is regenerated cleanly
   (watch for a stale 1.0.0 dir left in web/dist/mods if the generator does not
   prune) and that webmods_validation still drives the new content to Loaded.

## Open questions

- **Visible timer / time-trial.** OnUpdate + a `time` variable can accumulate,
  but with no HUD readout it is invisible; a real time-trial needs a small
  modding-surface addition (a timer widget). Resolve by shipping 2.0 without the
  clock and filing the time-trial as a follow-up (seeded below).
- **Course art as a place.** Themed skyboxes + asteroid texture variants would
  make the course read as a location; that rides the asset variety pack
  (20260716-123544) and its mod-resources support. Build 2.0 on the three
  existing cubemaps now; the art upgrade is a later, non-blocking pass.
- **Feel/balance numbers** (gate spacing, asteroid density, well strength,
  crash damage, hull margin) are a playtest output, not decidable from source.

## Next steps

- tatr 20260716-124722 (EXISTING, refined by this spike): the implementing task
  - build Gauntlet 2.0 as the content.ron rewrite + v1.1.0 republish. /plan
  breaks it into steps; Notes updated to cite this spike and the five hazards.
- tatr 20260716-174729 (seeded): Gauntlet time-trial - visible run timer +
  clean-run bonus (backlog; needs a timer HUD modding-surface addition first).

## Fix record

(Single implementing task; this section stays a pointer.) Gauntlet 2.0 lands
under 20260716-124722 - see that TASK.md for what shipped. The time-trial
follow-up (20260716-174729) is backlog until a timer readout exists.
