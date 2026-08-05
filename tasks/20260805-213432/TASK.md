# Dress Shakedown Run to screenshot density: a slalom belt around a near planetoid

- PRIORITY: 71
- TAGS: v0.10.0, content, scenario, art, engine
- ACTIVITY: WORKING
- GATES: PLAN
- RESOLUTION: -

## Scope

DECISION.md item 1 (SOI from mass) LANDED in 529954d7. What is left:

- item 2: `ScatterRegion::Ring { center }`
- items 3-5: the belt content, the slalom tuning, the preserved pockets
- the planetoid move the whole decision exists to enable

Beats, script, objectives, comms and outcome wiring stay untouched.

## Geometry - round-1 numbers

`soi = sqrt(mu / soi_cutoff_accel)` = `2 * sqrt(mu)` at the default 0.25.
The pins (`shakedown/tests/pins.rs:416`) bound it from both sides:

| Constraint | Bound |
| --- | --- |
| crates/debris/derelict/beacon 3 outside `soi + 40` | soi < 367 (crate 1 is nearest at 407u) |
| beacon 4 inside `soi * 0.75` and outside `widest_ring + 30` (181.5 + 30) | soi > 282 |

Round-1 pick, every pin recomputed by hand and green:

| Constant | Today | Round 1 |
| --- | --- | --- |
| `PLANETOID_POS` | (1240, -105, -700) | (500, -40, -560) |
| `PLANETOID_MASS` | 45 000 (soi 424u) | 27 000 (soi 329u) |
| `BEACON_4_POS` | (985, -69, -545) | (680, 10, -410) - 240u from the body |
| `COAST_RING_RADIUS` | 300 | 240 |

Checks: mass cap `27 000 <= 10 * 70^2` holds on the smallest mesh seed, so the
SOI stays seed-independent. crate 1 407u, crate 2 440u, crate 3 468u, debris
431u, derelict 632u, beacon 3 700u - all clear `soi + 40 = 369`. beacon 4 240u
sits inside `0.75 * soi = 246` and outside `widest_ring + 30 = 212`. Coast ring
240u clears the widest orbit ring (182 + 20) and sits inside `soi - 50 = 279`
and inside the nominal park (240 + 50 - 20 = 270). Waypoint leg 542u fits
beacon 4's authored 900u lock range. Beacon 2 (437u) and beacon 1 (544u) stay
OUTSIDE the SOI, so beats 2-3 are still flown in flat space.

### Belt - round-1 table

Five near knots as world-space `Box` scatters (no engine change needed for
these), plus one far parallax layer as a planetoid-centred `Ring`:

| Scatter | Region | Count | Rock radius |
| --- | --- | --- | --- |
| `belt_k1_` | box +/-70 xz, +/-35 y at (55, -20, -160) | 26 | 1.2-3.4 |
| `belt_k2_` | same at (140, 35, -450) | 26 | 1.2-3.4 |
| `belt_k3_` | same at (285, 45, -365) | 26 | 1.2-3.4 |
| `belt_k4_` | +/-85 xz, +/-40 y at (200, 20, -560) | 26 | 1.2-3.4 |
| `belt_k5_` | +/-85 xz, +/-40 y at (20, 15, -540) | 26 | 1.2-3.4 |
| `belt_far_` | Ring around the planetoid, 620-900u, y +/-160 | 30 | 4.0-4.9 |

Knots 2 and 3 alternate sides of the beacon-1 -> beacon-2 line; 4 and 5 are the
arc tail bending around the body. The far layer caps at 4.9 nominal ON PURPOSE:
`GravitySettings::min_well_radius` is 5.0, so a 4-10u far layer as the reference
shot uses would give all 30 rocks a default well (mu 4 000, ~126u SOI each) and
put gravity all over the "gravity-free" beacon-3 leg. Under 5.0 the whole belt
stays flat space.

Pocket rule every knot obeys, and the new pin asserts: a knot's half-extent plus
20u must clear the player spawn (60u), each beacon trigger (70u), the debris
cluster (90u), the derelict (40u) and the planetoid's widest orbit ring (200u).

## Steps

- [x] 1-3: `ScatterRegion::Ring { center }` + tests + wiki.
- [x] 4-6: planetoid move, belt scatters, pins.
- [x] 7: content regen, lint, CHANGELOG.
- [ ] 8: look-and-retune rounds. Round 1 shown to the owner; round 2 landed
      (see the close-out). Owner sign-off outstanding.

1. `crates/nova_scenario/src/actions/spawn.rs`: add `center: Vec3` to
   `ScatterRegion::Ring`, `#[cfg_attr(feature = "serde", serde(default))]` so
   existing mod RON keeps deserializing, and add it to the sampled position in
   `ScatterRegion::sample`. Update the doc comment (it says "centred on the
   origin"). Fix every `Ring` literal: `spawn.rs:581`, `spawn.rs:617`,
   `lint/scenario.rs:703`, `lint/ship.rs:450`, and `examples/screenshots/shared/
   kit.rs` (`NearField`, whose doc also claims origin-centring).
2. `crates/nova_scenario/src/actions/spawn.rs` tests: one test that a
   non-zero-centre ring samples inside the annulus AROUND that centre, and that
   an omitted `center` in RON deserializes to `Vec3::ZERO`.
3. `web/src/wiki/dev/modding-ron.md` and `web/src/wiki/dev/
   guide-author-scenario.md`: document the new field where they show `Ring`.
4. `crates/nova_assets/src/scenario/shakedown/mod.rs`: move the planetoid and
   retune, per the table above - `PLANETOID_POS`, `PLANETOID_MASS`,
   `BEACON_4_POS`, `COAST_RING_RADIUS`. Rewrite the constant docs that cite the
   old distances (the `PLANETOID_POS` doc still argues the 650u playtest
   separation; the beacon-4 doc still says "scaled out to 300u").
5. Same file: add the six belt scatters as `EventActionConfig::ScatterObjects`
   on the OnStart event, next to `start_spawns`. Template mirrors the existing
   debris rock (health 100, `mass: None`, `invulnerable: false`, impact and
   destroy sounds, shared `asteroid_texture`) so the slalom is a real but
   forgiving hazard. Hoist the knot table into named consts so the pin can read
   it. Keep `ROCK_OFFSETS`/`ROCK_RADII` hand-placed - the cluster carries the
   crate beat's pocket.
6. `crates/nova_assets/src/scenario/shakedown/tests/pins.rs`: update
   `beat4_geometry_holds_against_the_planetoid_soi` (numbers only - the shape of
   every assert survives), and add `belt_knots_keep_every_beat_pocket_clear`
   asserting the pocket rule above plus `belt_far_` max radius <
   `GravitySettings::min_well_radius`.
7. `cargo run -p nova_assets --bin content -- gen` then `... -- lint`; commit
   the regenerated `assets/base/scenarios/shakedown_run.content.ron` with the
   builder. Add the CHANGELOG entry.
8. Round 1 look: run the scenario, capture the beacon-1 -> beacon-2 leg, and
   put it in front of the owner. Then rounds 2-3: retune knot centres, counts,
   radii and the lane gaps against what the look says. Each round re-runs the
   pins and the two probes below.

## Definition of Done

Timeboxed, per the decision: done is the owner calling it after two or three
look-and-retune rounds - NOT a measured density.

| Item | Proof |
| --- | --- |
| Ring centring works and defaults to ZERO | `cmd: cargo test --lib -p nova_scenario ring` |
| Geometry pins hold on the new layout | `cmd: cargo test --lib -p nova_assets shakedown` |
| Belt clears every beat pocket, and carries no wells | same run, `belt_knots_keep_every_beat_pocket_clear` |
| Generated RON matches the builder, content is lint-clean | `cmd: cargo run -p nova_assets --bin content -- gen && git diff --exit-code assets/base` then `cmd: cargo run -p nova_assets --bin content -- lint` |
| The denser scenario still loads inside the New Game budget | `cmd: cargo run -p nova_probe -- run menu_newgame` (BOOT_SECS 90 on a software GPU) |
| Frame time survives ~160 new rocks | `cmd: cargo run -p nova_probe -- run scene_baseline --fps --scenario shakedown_run --preset low --preset high` |
| The scenario still plays end to end | `cmd: cargo test --lib -p nova_assets shakedown::tests::walk` |
| The owner has looked at 2-3 rounds and called it | owner sign-off recorded in the task record |

All cargo commands run through `nix develop --command`.

## Notes

Discovered:

- The SOI is `2 * sqrt(mu)` and is NOT overridable - mass is the only knob, so
  the planetoid position and its mass are one joint decision (the sketch's
  "SOI authored at 300u" is not a thing that exists; mass 27 000 buys it).
- The sketch's beacon 4 (660, 20, -380) FAILS the pins: 207u from the body,
  inside `widest_ring + 30 = 212`. The table above moves it to (680, 10, -410).
- The sketch's knot 5 (-40, 10, -300) puts rocks inside beacon 1's 70u trigger.
  The pocket rule and its new pin exist to catch exactly that.
- `PLANETOID_POS`, `BEACON_4_POS`, `COAST_RING_RADIUS` and `DEBRIS_CENTER` are
  read only by `pins.rs` - no other crate reads shakedown geometry.
- Asteroids never carry `GravityAffected`, so belt rocks inside the SOI stay put.
- `scene_baseline --scenario <id>` is the perf path; there is no shakedown
  autopilot example, so the player-path probe does NOT cover this scenario.

Assumptions:

- "Just the scenario changes" includes the `Ring { center }` engine field: it is
  decision item 2 and the far parallax layer needs it. The knots do not - `Box`
  is already world-space - so if the owner wants zero engine change this round,
  the fallback is an origin-centred far ring at 900-1400u and the field lands
  later.
- Round-1 counts (26 per knot, 30 far, 160 total) are a starting point to argue
  with, not a target.
- The web wiki pages are the only docs that show a `Ring` literal.

Risks:

- The far layer at 620-900u from the planetoid overlaps the beacon-3 leg and
  the spawn's own distance band. Seeded scatter cannot be pinned rock-by-rock,
  so a far rock CAN land near a beat point. It carries no well and reads as
  distant dressing; if a round shows it in the way, push the inner radius past
  the play area (900-1400u) and lose some mid-depth parallax.
- Load time, not frame time, is the likelier failure: 160 extra asteroid meshes
  land in one OnStart burst on the scenario every new player loads first.
  `menu_newgame` is the guard and it is a 90s budget on a software GPU.
- Beat 2 gains a hazard a first-time player can die into. Forgiving gaps are an
  authored claim until a round is actually flown at the 25 u/s cap.

## Close-out

### What and why

- `ScatterRegion::Ring` gained `center: Vec3` (`serde(default)`), so the far
  belt layer can circle the planetoid. Every existing `Ring` literal is
  origin-centred and unchanged - four more than the plan listed
  (`nova_assets/scenario/menu.rs` x3, `final_tally.rs`).
- `ScatterObjectsConfig` gained `min_separation: Option<f32>` with bounded
  rejection sampling (64 tries, then the copy is DROPPED). Not planned - see
  difficulties.
- Planetoid moved to (500, -40, -560) with mass 27 000 (SOI 329u); beacon 4 to
  (680, 10, -410); coast ring 240u. Beat-4 pins hold on the derived numbers
  with no edit to the test - the asserts were already written against consts.
- The belt: five `BELT_KNOTS` boxes plus one planetoid-centred far ring, all on
  OnStart beside `start_spawns`.

### Round-1 -> round-2 (owner look)

Owner verdict on round 1: too cluttered, autopilot legs must stay clear, and
"asteroids just explode on spawn". Round 2:

| Knob | Round 1 | Round 2 |
| --- | --- | --- |
| Near count / knot | 26 | 12 |
| Near nominal radius | 1.2-3.4 | 0.8-2.0 |
| Near separation | none | 32u |
| Far ring | 620-900u, 30 rocks | 1050-1450u, 18 rocks, 80u separation |
| Total belt rocks | 160 | 78 |

### Difficulties and diagnosis

- **Spawn explosions.** An asteroid's COLLIDER is `nominal_radius *
  ASTEROID_GEOMETRIC_FACTOR` (3.5-6.0), so a 3.4-nominal belt rock is a 12-20u
  body; 26 of them in a 140x70x140 box interpenetrate, and avian resolves that
  by throwing them apart hard enough to deal impact damage. No count or radius
  is safe on its own - uniform sampling always produces close pairs - so the
  fix had to be a separation rule in the scatter itself.
- **Autopilot corridors.** The far ring at 620-900u from the planetoid ran
  straight through the play volume (the spawn is 752u out, beacon 3 is 699u), so
  a seeded rock could land on a hands-off leg. Seeded scatter cannot be aimed
  per rock, so the ring's HOLE now contains the whole playable volume.
- **A pin that passed by accident.** The planned knot 4 at x=200 cleared the
  planetoid's widest orbit ring by 0.9u. Moved to x=170.

### Evidence

- `cargo test --lib -p nova_scenario` 151 pass; `-p nova_assets` 97 pass.
- New pin falsified before it was trusted: knot 5 at the sketch's
  (-40, 10, -300) fails with "65u from beacon 1 ... 175u is the floor".
- `content gen` + `lint`: 0 errors, 0 warnings, 14 scenarios balance-audited.
- `nova_probe run menu_newgame`: OK, Playing at frame 56, 0 panic/ERROR lines.
- `nova_probe run scene_baseline --fps --scenario shakedown_run`, dev profile,
  RTX 3060 Ti: belt 46.4 low / 46.9 high mean fps vs 47.3 low / 34.6 high on
  the pre-belt tree. NOTE: the first belt run read 25.6 low; a re-run put it at
  46.4, so that number was warm-up, not the belt. Frame time is a non-issue.
- 30s shakedown run log: all 60 near rocks placed, 0 dropped by the separation
  budget, 0 destruction events.

### Reflection

- The authored-vs-derived lesson bit again, in a new place: the belt was sized
  against `radius` when everything physical uses `radius * geometric_factor`.
  Any new content that places rocks by hand should quote the DERIVED extent.
- The hand-placed debris cluster (`ROCK_OFFSETS`) has pairs ~33u apart with
  combined worst-seed extents of ~33u - the same latent overlap, pre-existing
  and unreported. Worth its own task rather than a drive-by change here.
- Round-2 numbers are still authored guesses; the DoD is the owner's call.
