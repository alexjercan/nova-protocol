# Gauntlet Run 2.0 - design & fix record

- TASK: 20260716-124722
- SPIKE: tasks/20260716-174631/SPIKE.md (direction A: pure-data rewrite)
- BRANCH: feature/gauntlet-2.0

## What shipped

A full rewrite of `webmods/gauntlet/gauntlet.content.ron` (portal mod
v1.0.0 -> v1.1.0), turning a thin four-gate line into a real parkour course.
Zero engine changes - every primitive already existed; the work was authoring
data and pinning it with a rig.

## The course

Three escalating acts, run toward -Z, threaded by the existing ordered-`gate`
counter (now 1..=7; 8 is the terminal done-state):

- **Act 1 (warmup)** - `cubemap.png`. Open gates (GATE 1-2, area radius 20u),
  gentle turns, one sparse rock off the first turn.
- **Act 2 (slalom)** - `cubemap_alt.png` (swapped in GATE 3's OnEnter). Tighter
  gates (GATE 3-4, radius 18u), four rocks flanking the line (~9-17u past the
  ship margin - scenery-tight now; playtest may pull them in), and a `ScatterObjects`
  belt-wall field off the +x side for depth.
- **Act 3 (hazard)** - `cubemap_alt.png` (swapped in GATE 5's OnEnter). Tightest
  gates (GATE 5-6, radius 16u), three more rocks, and a big invulnerable
  `surface_gravity` planetoid off the G5->G6 line whose SOI tugs the pilot.

Crossing FINISH (radius 28u) sets `gate = 8` and declares `Outcome(Victory)`.
Wrecking the ship before FINISH declares `Outcome(Defeat)` + a lingering Retry;
the Defeat handler is gated `gate < 8` so a death blast after the win cannot
flip the earned Victory (the Broadside R1.3 lesson, applied up front).

## The two invariants and how they are enforced

The course header spells out two invariants; `crates/nova_assets/tests/gauntlet_course.rs`
is the rig that holds them, modeled on `broadside_assault.rs`:

1. **Gate areas must not overlap.** With more, tighter gates this is the top
   soft-lock risk: a pilot loitering in the next area when it arms produces no
   fresh OnEnter. `gate_areas_are_pairwise_non_overlapping` checks every pair
   (all sit 80-101u apart, radii 16-28u, so every pair clears with room).
2. **The racing line stays flyable past the 6x asteroid geometric factor.**
   Asteroid meshes reach `ASTEROID_GEOMETRIC_FACTOR_MAX = 6.0x` their nominal
   radius (shakedown_run soft-locked a beat on exactly this). The design keeps
   the ideal GOTO line - the START->gate->...->FINISH polyline - clear of every
   rock's WORST-CASE body by a ship margin (`SHIP_CLEARANCE = 8u`), so the line
   is always flyable and deviation is what clips a rock. `every_rock_clears_the_racing_line`
   measures this for every solo rock (point-to-polyline) and for the scatter
   field (dense polyline-to-box sampling, worst-case rock anywhere in the box).
   Fail-first proof: moving rock_1 onto the line (0.8u away) drops clearance to
   -8.2u and the test fails with that exact diagnostic.

Behavior is pinned too: gates advance ONLY in order (out-of-order and
wrong-ship entries are inert), FINISH declares Victory with nothing queued, a
wreck declares Defeat + gauntlet_run retry, and a post-win wreck declares
nothing. The act-boundary SetSkybox actions resolve a cubemap off the
AssetServer, so the headless rig adds a minimal `AssetPlugin` + `init_asset::<Image>()`
(no scenario camera present, so the swap warns and returns - the gate advance
around it is what we assert).

## Dependency: base only (review MINOR-1)

v1.0.0 declared `dependencies: ["base", "demo"]`. The content only names base
prototypes/textures, but the demo mod OVERRIDES `reinforced_hull_section` by id
(health 200 -> 400, last-wins overlay), so v1.0.0's racer silently got a
double-tough hull AND players were forced to enable the demo arena scenario.
The out-of-context review flagged this as load-bearing, undocumented coupling.

2.0 drops `demo`: the racer's crash tolerance is now base's honest 200-health
`reinforced_hull_section`, tuned by playtest, and the mod no longer depends on a
scenario slated for removal (task 20260716-155816). This roughly halves the
racer's crash tolerance vs. v1.0.0 - which is fine, because balance was always
deferred to playtest; a documented 200-health starting point beats an accidental
400. `webmods_validation` (recursive dep resolution) stays green on `["base"]`.

## Republish dogfood

`nova_portal_gen` publishes `gauntlet 1.1.0` (3 files) cleanly into the preview
tree, catalog.json reflecting 1.1.0. Enabled state is keyed by mod id (not
version) in the cache/prefs, so an in-place update preserves the user's enabled
toggle - the enabled-state-preserving update the task asked for.

Republish hygiene note: the generator writes `<out>/<id>/<version>/` and does
NOT prune old version dirs. In a fresh checkout only 1.1.0 appears; in a
long-lived preview dir a stale `1.0.0/` would linger next to `1.1.0/`. It is
harmless (catalog.json lists only the current version) but the operator should
clear the out dir before a preview regen if they want a pruned tree. CI's
deploy job runs on a clean checkout, so the published site never carries it.

## Deferred to playtest (hands-on, not decidable from source)

Feel/balance is a human verdict, same shape as Broadside's "too hard":

- Crash-damage severity vs. base's 200-health reinforced hull (run-and-fail, not
  one-touch death, but a full broadside into a rock should hurt). Note the hull
  is now base's 200, not demo's 400 - see the dependency section above.
- Rock tightness to the racing line: currently ~9-17u past the ship margin
  (scenery), so a clean line barely grazes them. If sloppy flying should bite
  harder, pull the rocks in (the rig enforces only the >=8u floor).
- Gravity-well strength (`surface_gravity: 8.0`) - enough to feel, not enough
  to yank a careful pilot off a gate.
- Gate spacing / area tightness across the acts.

Findings become `balance`/`bug` tasks at release priority. The visible run
timer + clean-run bonus is the separate follow-up 20260716-174729 (backlog),
which needs a HUD timer readout the vocabulary lacks today.

## A decision worth recording

The racer keeps its full v1.0.0 loadout including the turret and infinite_ammo.
The spike floated dropping the turret as "combat kit," but the ship is already
balanced with it, removing a section changes mass/handling (which affects the
whole flight feel), and a pure-flight course means "no enemies to shoot," not
"no turret." Keeping the loadout is the smaller, safer change.
