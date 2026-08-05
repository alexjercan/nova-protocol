# NOTES - Dress Shakedown Run to screenshot density

Understanding record. Owner-confirmed 2026-08-05.

## Problem Statement

Shakedown Run - chapter one, the scenario New Game drops every new player into -
reads as empty black sky. It has **9 rocks** in the whole level (a hand-placed
debris cluster, r 1.0-3.0) plus one distant planetoid. The screenshot examples
that ship on the website get their look from **70+ seeded rocks in two size
layers**, and they look far richer than the game's own first scenario.

Two things follow:

- Density. Scenario 1 should hit the reference density of
  `web/src/assets/tutorial-radar-lock.png`.
- The beacon-1 -> beacon-2 leg should be a **flyable slalom** through a large,
  banana-shaped belt bending around a planetoid, instead of a straight line
  across open space.

It is explicitly NOT:

- A rewrite of the beat sheet. The five beats, the script, the objectives, the
  comms lines, the pacing gates and the outcome wiring all stay as they are.
- A pass over the other mainline scenarios. Broadside, Lifeline and Final Tally
  are out of scope (they may get the same treatment later, from what this task
  learns).
- A one-shot "get it right" job. This is **iterative**: a first cut will not
  look right, and the point is to land something lookable and then retune it.

## Context

### What scenario 1 is today

`crates/nova_assets/src/scenario/shakedown/mod.rs` (1223 lines), id
`shakedown_run`, first entry of the mainline campaign (`scenario.rs:492`).

| Thing | Where | Notes |
| --- | --- | --- |
| Player spawn | `(0, 0, 0)` | speed cap 25 u/s (`PLAYER_SPEED_CAP`) |
| Beacon 1 | `(0, 0, -350)` | dead ahead, beat 1 |
| Beacon 2 | `(260, 20, -200)` | ~120 deg off boresight, freelook beat |
| Debris cluster | `(350, 20, -160)` | 9 rocks, `ROCK_OFFSETS` / `ROCK_RADII`, r 1.0-3.0 |
| 3 salvage crates | `(345,30,-190)`, `(360,5,-145)`, `(395,35,-110)` | 8u pickup radius, >=53u apart |
| Planetoid | `(1240, -105, -700)` | nominal r20, `surface_gravity: 6.0`, invulnerable |
| Beacon 3 | `(600, 90, 120)` | first radar-lock target, must be gravity-free |
| Beacon 4 | `(985, -69, -545)` | inside the SOI, orbit beat |
| Coast ring | 300u around planetoid | invisible trigger sphere |
| Derelict | `(300, -40, 40)` | dynamic body, live-fire rehearsal |
| Pirate | spawns at `(380, 40, -100)` | beat 5, leash 150u |

Rock content total: **9 debris + 1 planetoid**.

### What the reference shot is

`tutorial-radar-lock.png` is the `rock_hollow` scene,
`examples/screenshots/screenshot_combat.rs:585-608`. Two seeded
`ScatterObjects` ring layers:

| Layer | Count | Radius | Distance band | Y spread |
| --- | --- | --- | --- | --- |
| Corridor (`hollow_far_`) | 26 | 4.0-10.0 | 200-640u | +/-160 |
| Shell (`hollow_rock_`) | 48 | 1.2-3.2 | 48-130u | +/-46 |

**74 rocks, two distinct scales, deliberate layering.** The far layer supplies
parallax and silhouette; the near layer supplies the wall of the pocket. The
example's own comment records that tighter than 48u put rocks on top of the
subject - the hollow is a hollow on purpose.

Also relevant, from the same file's hard-won notes: the near field carries **no
gravity wells** ("a near-field rock strong enough to pull the posed subject
would drift it out of frame").

### Engine constraints the answer must respect

1. **`ScatterRegion::Ring` is hard-centered on world origin.**
   `ScatterObjectsConfig::action` does `object.base.position =
   self.region.sample(&mut rng)` (`crates/nova_scenario/src/actions/spawn.rs:274`)
   - the sample OVERWRITES the template position, it does not offset it. A ring
   around a planetoid at `(1240, ...)` cannot be expressed today.

2. **SOI is derived from the runtime mesh radius, not from mass.**
   `GravityWell::from_surface_gravity` (`crates/nova_gameplay/src/gravity.rs:67`):
   `mu = g * body_radius^2`, `soi_radius = soi_factor(8) * body_radius`. The
   body radius is the *noise-mesh* radius, which runs
   `ASTEROID_GEOMETRIC_FACTOR_MIN..MAX` = 3.5-6.0 times nominal, per seed. So a
   nominal-20u planetoid has a body radius of 70-120u and an SOI anywhere in
   **560-960u**, depending on the seed. Everything downstream is authored
   against that whole range.

3. **The geometry pins encode that range.**
   `crates/nova_assets/src/scenario/shakedown/tests/pins.rs:414-522`
   (`beat4_geometry_holds_across_the_derived_radius_range`) asserts:
   - beacon 3, the derelict, the debris cluster and all 3 crates sit OUTSIDE the
     largest plausible SOI + 40u
   - beacon 4 sits inside `smallest_soi * 0.75`
   - the coast ring (300u) sits inside `smallest_soi - 50u`
   The dynamic-body ones are not cosmetic: a dynamic body inside the SOI falls
   into the planetoid.

4. **Playtest history.** Round 2 finding 1 moved the planetoid AWAY from the
   crate beat precisely because "the player was fighting gravity while weaving
   crates". Any move back toward the crates reverses that call, unless the well
   is made smaller.

5. **Scatter is never thinned by graphics quality.** `spawn.rs`: "always the
   authored count. Scatter is gameplay content, so no graphics-quality tier
   thins it." Density is a perf decision, not a settings-tier one.

6. **Collision is real.** Debris rocks are destructible (`health: 100`,
   `invulnerable: false`) and collidable. At the 25 u/s starter speed cap a
   collision is survivable, but the slalom is a genuine hazard surface.

7. **Determinism is load-bearing for the tests.** Today's cluster uses a fixed
   offset table on purpose ("the layout is content, and determinism keeps the
   config-shape tests honest"). Seeded scatter is equally deterministic, but the
   pins can only assert the *region*, not each rock.

### Level-design reading (why the reference shot reads better)

Not new theory - just what the reference and the literature agree on:

- **Two scales, not one.** The reference has a 4-10u far layer and a 1.2-3.2u
  near layer. A single size band reads as noise; the contrast is what creates
  depth. Hierarchy comes from contrast in size/shape/spread
  ([Level Design Book, Composition](https://book.leveldesignbook.com/process/blockout/massing/composition)).
- **Landmarks orient.** A big distinctive body in the distance is how a player
  knows where they are without a marker
  ([Wayfinding](https://book.leveldesignbook.com/process/blockout/wayfinding)).
  A near planetoid IS the landmark for the beacon-1 -> beacon-2 turn - which is
  the beat where the player is supposed to look around and find something.
- **Framing over corridors.** Geometry that frames the goal in the field of view
  guides without a corridor
  ([guiding without hand-holding](https://ludonodestudios.medium.com/level-design-fundamentals-guiding-players-without-holding-their-hand-43c9a84a065a)).
  A belt bending around the planetoid frames beacon 2 rather than walling the
  route.
- **Density is danger, and it should vary.** Asteroid fields read best when
  concentration varies - sparse lanes, dense knots - so the player reads where
  it is safe rather than facing uniform soup
  ([field navigation](https://30k.fun/star-citizen/guides/Mastering-Asteroid-Field-Navigation/)).
- **The hollow rule.** The reference keeps a clear pocket around the subject.
  Every beat here that needs clear air (crate pickups, the pirate fight, the
  orbit) needs its own pocket.

### Owner constraints (verbatim, 2026-08-05)

- "the mainline campaign still feels a bit empty and ugly"
- "the screenshot example are actually a lot lot richer in details"
- "the first scene should look the same" as `tutorial-radar-lock.png`
- "there is a slalom course to the second beacon, in the sense that the asteroid
  belt is much larger and like a banana shape around the planetoid"
- "the rest of the scenario can stay the same"
- "this is also kind of an iteration task, it will not look right from the get
  go, but we will iterate bit by bit"
- "use more of a Timebox for DoD because we can iterate a lot"
- On the SOI clash: "we need to make SOI not depend on radius and can be
  overriden; idealy it should depend on MASS"

## Ideas

Ranked best-first. All four assume the beat sheet is untouched.

### 1. Mass-based wells + re-centerable rings, then stacked scatter (CHOSEN)

Three moves, in order:

1. **Engine: author mass, derive SOI from it.** Replace the radius-derived
   `soi_radius = soi_factor * body_radius` with a mass/mu-derived one, plus an
   explicit override. Radius stops driving reach, so the 3.5-6.0x mesh variance
   stops swinging the SOI 560-960u and the pins collapse to single numbers.
2. **Engine: `ScatterRegion::Ring { center: Vec3, .. }`,** defaulted to
   `Vec3::ZERO` so every existing scatter is byte-identical.
3. **Content: stack 2-4 seeded Ring/Box scatters** into a banana bending around
   the now-near planetoid, plus a far parallax layer, matching the reference's
   two-scale split. Keep clear pockets at the crate cluster, the pirate fight
   and the orbit ring.

Cost: an engine change touching every scenario that authors `surface_gravity`
(shakedown, and whatever else) plus a serde/lint/test pass; then cheap,
fast content iteration on top. Buys: the composition the owner asked for is
actually expressible, and the SOI stops being a seed lottery.

### 2. Same content, but keep the radius-derived SOI and shrink the planetoid

Drop `PLANETOID_NOMINAL_RADIUS` until the SOI clears the crate beat, move it
near beacon 2, stack the scatters. No engine change to gravity.

Cost: cheapest. Loses: the planetoid stops being a landmark - shrinking it is
exactly the wrong direction for "a big body anchors the turn", and the SOI still
swings by seed, so every number is still authored against a 1.7x range.
**Rejected by the owner** in favour of fixing the derivation.

### 3. Add a `ScatterRegion::Arc` variant

A first-class arc region (center, radius, start angle, sweep, thickness, y
spread) exactly expresses a banana belt and would be reusable by every later
scenario.

Cost: a bigger engine surface (serde, lint, content-gen, tests) for a shape
that stacked rings approximate well enough. **Rejected by the owner** - YAGNI
until a second scenario wants it; if stacking turns out to be unworkable in
practice, this is the fallback and the iteration will say so.

### 4. Hand-authored offset table, just much longer

Extend `ROCK_OFFSETS` to 60-80 entries. Total control of every gap, no engine
change, tests stay trivial.

Cost: unworkable to iterate at this count, and iteration is the whole point of
the task. **Rejected.**

## Decisions (owner, 2026-08-05)

| Question | Answer |
| --- | --- |
| Problem framing | Density + one flyable slalom leg; beats/script/objectives unchanged |
| Planetoid | Move it CLOSER, so it anchors the beacon-1 -> beacon-2 leg |
| SOI clash | Fix the engine: SOI derives from mass, not radius, and is overridable |
| SOI rework scope | **Inside this task**, not a sibling |
| Slalom rocks | **Real hazard, forgiving** - collidable and destructible, gaps sized generously against the 25 u/s cap; clipping one costs a little hull, it does not end a first flight |
| Banana shape | **Stack existing Ring + Box scatters**; no new region variant |
| Ring centering | Add `center: Vec3` to the `Ring` variant, defaulted to `ZERO` |
| DoD | **Timeboxed: two or three iteration rounds.** Done is "the owner has looked at N rounds and called it", not a metric |

## Open assumptions

- The mass-based SOI formula is not yet chosen. The obvious shape is: author
  `mu` (or mass with a G), derive `soi_radius = sqrt(mu / a_min)` where `a_min`
  is a global "gravity we stop caring about" threshold, and allow an explicit
  `soi_radius` override on the body. To settle in planning.
- Rock budget for the belt is unknown. The reference is 74; scenario 1 will want
  more than that across a much larger volume. Perf is measurable with
  `nova_probe` and the `stress/` examples; the first round picks a number and
  the probe says whether it holds.
- Whether the existing 9-rock `ROCK_OFFSETS` cluster survives as-is inside the
  new belt, or is absorbed into it. It carries the crate beat's clear pocket, so
  it probably stays hand-placed. First round will show.
- Every scenario other than shakedown that authors `surface_gravity` needs
  re-checking once the SOI derivation changes. The set has not been enumerated.
