# Notes

Landed: two torpedo TYPES as an authored property of the bay, and the campaign
easing that goes with them. **One brief premise did not survive measurement -
see section 3 - and no second lever was invented in its place.**

## 1. Bay property, not a loadable

`TorpedoTypeConfig` on `TorpedoSectionConfig`, replacing the loose
`weave_angle` / `weave_rate` pair. Four fields: `name`, `tint`, `weave_angle`,
`weave_rate`. At launch it splits into `TorpedoWeave` (flight, needs per-torpedo
runtime state anyway) and a new `TorpedoType` component (identity: name + tint).

The loadable was rejected on machinery, not on taste:

- `LoadedBullet` has **zero mutators** in the tree. It is a documented growth
  seam, not a feature - nothing in the game swaps a turret's round at runtime,
  and no UI offers it. A `LoadedTorpedo` twin would be a second seam with zero
  mutators.
- It would FORK the source of truth. `input/ai/torpedo.rs` derives the launch
  envelope from `TorpedoSectionConfigHelper` (the bay's config) directly, as
  does the HUD. A runtime slot means two places to read and one of them wrong.
- A bay in this game is a hull PART (the cargo-B pod), fixed at build, not a
  magazine reloaded at a station. The runtime slot would model a fiction that
  does not exist.

Cost of the property version: one struct, one small component, three lines in
the render observer, two in the snapshot. The loadable needs all of that PLUS a
slot component, a seeding path, and the fork above.

**Data, not an enum**, so a mod authors its own type by writing one into its
bay's config. `#[serde(default)]` on the field, so a mod's existing bay RON
still loads and gets the Serpent.

### Where the identity lands

- catalog: `torpedo_section` -> "Torpedo Bay (Serpent)", new
  `lance_torpedo_section` -> "Torpedo Bay (Lance)". Both browsable in the
  editor gallery with descriptions that state the trade.
- semantic pods: `cargob_pod_{port,starboard}` gain a `_lance` twin, generated
  the same way `prototypes()` already generates the `_light` turret. `Ordnance`
  picks which id a ship references; nothing else about the ship moves.
- the siege bay's 0.22 weave became a third named type, **Breaker**. Its
  numbers are unchanged; it just has a name and a colour now.
- in flight: the warhead body is tinted from the type. Free - the material was
  already built per projectile (`SectionDamageTint` clones per section), so the
  shared-MESH invariant the render tests protect is untouched.
- the launched entity is `Name`d for the type ("Lance Torpedo"), and the probe
  snapshot's ordnance record gained a `"type"` field. Two bays on one hull are
  otherwise byte-identical in a snapshot.

Colours: Lance pale steel `(0.70, 0.78, 0.86)`, Serpent hazard orange
`(0.95, 0.45, 0.10)`, Breaker crimson `(0.75, 0.10, 0.12)`. The hot family ties
Serpent and Breaker to the Explosive hue the ammo readout already speaks.

## 2. What an intercept costs, per type

`point_defense_cost_tests::defend`, one stock PDC over the shipped 150 u
point-defense envelope (this rig IS the player's chapter-two loadout: 100
rounds/s, 4.0 authored, and a ship engages one torpedo at a time whatever its
mount count):

| | Lance | Serpent |
|---|---|---|
| rounds the defender fires | **116** | **369** |
| seconds inside the envelope | 1.17 | 3.69 |
| range it finally dies at | **114.0 u** | **38.8 u** |
| visible swing off the line | 0.0 u | 6.6 u (10.7 u on the real body) |

38.8 u is the load-bearing number. The warhead's blast radius is 30 u, so a
PERFECT defender kills a Serpent 8.8 u outside its own blast - with nothing
left over for a human's aim. It kills a Lance with 84 u to spare.

## 3. The trade the brief assumed does not exist - measured

The brief: "the cost of evasion is already in the geometry - a weave lengthens
the flight path, so an evasive torpedo arrives later and reaches less far for a
given lifetime. MEASURE that."

Measured, on the real stack (avian + PD attitude controller + thruster +
`SpaceshipSectionPlugin`), 300 u run-in onto a stationary target
(`bay::tests::the_weave_is_a_longer_path_that_costs_no_time_to_target`):

| | Lance | Serpent |
|---|---|---|
| path flown | 284.3 u | 289.6 u (**+1.9%**) |
| time to fuze | 9.10 s | **8.97 s** |
| speed along the line | 31.30 u/s | **31.83 u/s** |
| reach at the authored 100 s lifetime | 3130 u | **3183 u** |

**The evasive torpedo arrives SOONER and reaches FURTHER.** Two reasons, both
real and both in the shipped code:

1. **The path stretch is 1.9%, not the ideal 11%.** `1 / cos(0.44) = 1.106` is
   an upper bound on the COMMANDED cone. The body's linear damping is a
   first-order lag on the velocity, so the flown helix is far shallower than the
   commanded one - effective radius ~3.9 u against a 10.7 u peak excursion.
2. **`thrust_headroom` gates on the ALONG-NOSE speed, not on total speed**
   (`projectile.rs`, and the doc there says why: a total-speed cap leaves the
   torpedo ballistic at cruise and unable to steer). A weaving torpedo flies
   with its nose tilted off its own velocity, so it never reaches the taper
   band, keeps its engine lit, and settles at a higher terminal speed against
   the same drag. Equilibrium arithmetic: `25 * (35 - v*cos t) / 5 = 0.8 v`
   gives 30.2 u/s straight and 31.4 u/s at a 0.3 rad nose offset; measured 31.2
   and 32.3. Evasion runs a hotter engine, and the longer path is its fee.

The prior task's note that "the longer path IS the price, so straightening it
out is not an optimisation - it is deleting the balance" is therefore **wrong
on the real body**. The claim now has a test attached, and the test flips the
day the thrust law caps total speed.

Also worth stating: reach was never the differentiator it was assumed to be.
Both types out-reach `AI_TORPEDO_MAX_RANGE` (1000 u) by ~3x at the authored
100 s lifetime, so lifetime does not bind on any shipped arena.

### What this means, and what was NOT done

**Superseded by section 8 - the owner picked a lever. The measurement above
stands unchanged and is why the lever was needed.**

At the numbers as first shipped the Serpent was a **strict upgrade**: same
blast, same rack, three times harder to stop, and no cost anywhere. Per the
brief - "say so with the numbers and ask before adding a second lever, rather
than inventing one" - no lever was invented here; the sweep and the decision are
in section 8.

The options put to the owner, cheapest first:

1. **`projectile_lifetime` on the type.** The spike calls it the only reach
   lever in the tree with a provably empty blast radius on the damage model.
2. `ammo_capacity` / `reload` per type: a bigger or faster-regrowing Lance rack,
   the classic cheap-round trade.
3. `max_speed`: rejected AS A GLOBAL, because a faster Lance also shortens its
   own exposure window and would have undone the campaign easing. Per TYPE that
   objection does not apply, and per type is what the owner chose.

## 4. The first mainline torpedo encounter, and whether it was too hard

**It is `broadside_gunship`** - chapter two part two, the Gunship Rust Tally.
Not guessed: counted over the generated scenarios in campaign order
(`campaigns.rs`: shakedown_run -> broadside -> broadside_gunship -> lifeline ->
final_tally). Torpedo-bay prototype references per scenario: shakedown_run 0,
broadside 0, **broadside_gunship 2**, lifeline 0, final_tally 2. The ledger's
ch4 Auditor is a webmod, not the mainline campaign.

What the player brings: a cargo-A at player grade, TWO PDC mounts (100
rounds/s, 500 rounds, 3.0 s reload) both on LMB, hand-aimed at the crosshair.
The player has **no autonomous point defense at all** - that is an AI-only
behaviour - so every intercept is a human tracking a target by eye.

What the gunship brings: two pods x 6 = a **twelve-torpedo alpha strike**, then
+1 per bay per 10 s, plus two player-grade PDC turrets and 1730 hp of hull.

### Live A/B, same scene, perfect defender (this is the before/after)

The best available defender-side measurement in the shipped game is
`menu_gauntlet`: an AI corvette with TWO player-grade PDCs and a hard 800-round
magazine, standing against four torpedo batteries. Run headless on Xvfb through
`scene_baseline` with `NOVA_PERF_SNAPSHOT`, snapshotting every 20 frames. The
gauntlet was temporarily re-pointed at the Lance bay for the second arm and
reverted afterwards (`git status assets/` clean of it).

| | Serpent (before) | Lance (after) |
|---|---|---|
| point defense opens | t = 25.1 s | t = 26.2 s |
| corvette destroyed | between t = 32.1 and 34.6 s | between t = 39.7 and 43.6 s |
| **seconds it held** | **~8-10 s** | **~14-18 s** |
| rounds left when it died | 187 / 0 of 400+400 | 105 / 0 of 400+400 |

A **perfect** AI defender with two mounts and 800 rounds burns 613 of them in
about 8 seconds against Serpents and is overrun. Against Lances the same
corvette holds nearly twice as long on the same scene, the same seed, the same
batteries. That was the whole case for the retune: if the AI, which never
misses and never has to find the target, loses in eight seconds, a human
hand-aiming two turrets was never going to screen twelve of them.

Consistent with the catalog: `CORVETTE_ROUNDS_PER_TURRET`'s own comment claims
400 rounds is "roughly a TEN-torpedo defense", which is true at ~116 rounds an
intercept (6.9 per mount) and false at 369 (2.2). The comment was written for a
Lance-shaped world.

### The retune

`broadside_gunship` gunship -> `Ordnance::Lance`. `final_tally` flagship ->
`Ordnance::Serpent`, explicitly, so the escalation is authored rather than
inherited. Pinned by `the_gunship_keeps_its_torpedo_tubes`
(nova_authoring/tests/broadside_assault.rs), which now also asserts the tubes'
weave angle is zero.

Live proof it landed in the SHIPPED scenario, not just in the builder: a
`scene_baseline --scenario broadside_gunship` run with snapshots reads four
torpedoes in the air at t = 28.3 s, all `"type": "Lance"`. (The idle player is
dead by t = 55.8 s, which says nothing about difficulty - nobody was flying.)

### On the stale ledger ack

Confirmed stale as the brief said, and NOT fixed here - the ledger is a webmod
and out of this lane. `cargob_turret_*_light` does not exist: `cargo_b.rs`
passes `light_turrets: false`, so the ch4 Auditor mounts full player-grade
PDCs, not "the light mook turret (downgraded for exactly this spawn)". The ack
text is wrong about its own content. `content lint` still reports 0 errors.

## 5. Live verification

Display `:77` on Xvfb throughout, killed by PID at the end.

- `torpedo_section` example, `NOVA_AUTOPILOT=1`, three clean cycles
  ("cycle complete, no panic", exit 0). The range now flies BOTH bays off one
  trigger and draws each trail in its type's colour, which is the point: a
  corkscrew is only legible next to the straight line it departs from. Captured
  `variant-torpedo-weave.png` shows pale-steel Lance trails running dead
  straight through curving orange Serpent ones.
- `scene_baseline --scenario broadside_gunship` and `--scenario menu_gauntlet`,
  as above.

### One flake found and fixed on the way

`weave_trail_flown` (the example's weave-frame gate) counted trail SAMPLES, and
a sample is taken at most once per frame. On a slow run a frame covers more than
the 0.5 u sample spacing, so the predicate silently asks a slow run to fly
several times as far - it stalled its 8 s deadline on the first run of the new
two-bay range, which is heavier. Now measured in UNITS of flown ground. Both
that gate and the camera framing also filter to torpedoes that actually weave,
or the weave shot is a coin flip that can frame a Lance.

## 6. Checks

- `cargo check --workspace --all-targets` clean (one pre-existing dead-code
  warning in `broadside_assault.rs`, present at HEAD, untouched).
- `cargo fmt --check` clean.
- `content -- gen` twice; second run leaves `git status assets/` clean.
- `content -- lint`: 0 errors, 0 warnings, 14 scenarios balance-audited.
- `cargo test --lib -p nova_ship` (torpedo_section: 49 pass),
  `-p nova_authoring` (ordnance tests), `--test broadside_assault` (15 pass).

## 7. Merging `47ce8257` (a ship is a content kind now)

Master made a ship a first-class content entry resolved by id, and it landed
under this branch. Nine conflicts, all in the files this task touched. The
restructure did NOT invalidate anything measured - the numbers in sections 2-4
are properties of the torpedo and of the defender, and neither moved - but it
did make the campaign easing a strictly better change, so the expression was
redone rather than merged verbatim.

### What changed, and why

Master's `ships/mod.rs` states the rule this task needed: "A grade is a
build-time knob, not a spawn-time one ... so it is a second CATALOG entry
rather than a flag a scenario flips." Ordnance is the same shape of knob, so
the cargo-B now ships as **two catalog ships** the way the corvette ships as
`cargoa` / `cargoa_raider`:

| | before the merge | after |
|---|---|---|
| the easing | rewrite two pod prototype refs inside an inlined ship | `hull: hull(CARGOB_LANCE_SHIP_ID)` |
| what the scenario RON carries | the whole ship, pods included | one id |
| the diff in `broadside_gunship.content.ron` | 2 prototype lines | **1 line**, `cargob` -> `cargob_lance` |

So `Ordnance` stopped being something threaded to a scenario and became what it
always should have been: a build-time argument to `cargo_b::sections`, consumed
once in `ship_catalog()`. `ship_sections` takes it beside `grade` and rewrites
the pod prototype id exactly as `grade` rewrites the turret's - both knobs, one
mechanism.

The `_lance` pod prototypes and their generation in `prototypes()` survived
unchanged: the coordinator's warning was that the `_light` path might have
moved, and it did not - `shared.rs::prototypes` is byte-identical to what this
branch forked from.

### Conflict-by-conflict

- `assets/base/scenarios/broadside_gunship.content.ron` - generated. Took
  master, regenerated.
- `ships/mod.rs` - took master, added `CARGOB_LANCE_SHIP_ID` and its catalog
  entry, extended the module doc to say ordnance is a build-time knob too.
- `ships/shared.rs` - took master, re-applied `torpedo_kind(.., torpedo_type)`,
  the `_lance` twin in `prototypes`, `Ordnance`, and the third `ship_sections`
  argument. `ship_sections` had ALSO lost its `controller_modifications`
  argument on master (spawn-level `modifications` now), which is why this file
  conflicted rather than merging.
- `ships/{cargo_a,racer}.rs` - took master, passed the inert `Ordnance::Serpent`
  (neither craft has a pod).
- `ships/cargo_b.rs` - took master, `sections(ordnance)`.
- `scenarios/nova_protocol/broadside.rs` - took master, pointed the gunship at
  `CARGOB_LANCE_SHIP_ID`.
- `scenarios/nova_protocol/final_tally.rs` - took master; the flagship keeps
  `CARGOB_SHIP_ID`, which is now explicitly the Serpent ship. The prose saying
  so is the guard against a future merge quietly re-pointing it.
- `tests/broadside_assault.rs` - took master, re-applied the zero-weave pin.
  Master had rewritten the same helper to resolve the hull through
  `spawned_ship_sections`, so the pin now reads what the gunship actually flies
  rather than which id the scenario names - a better test than the one it
  replaced.

### Re-verified after the merge

`cargo check --workspace --all-targets` and `cargo fmt --check` clean;
`content -- gen` idempotent (second run leaves the tree clean); `content --
lint` 0 errors; `nova_ship --lib` 610 pass, `nova_authoring --lib` 73 pass, all
`nova_authoring` and `nova_assets` integration tests pass.

Both assignments re-confirmed from the GENERATED content, not the builder:

```
cargob         pods -> cargob_pod_port,       cargob_pod_starboard        (Serpent, weave 0.44)
cargob_lance   pods -> cargob_pod_port_lance, cargob_pod_starboard_lance  (Lance,   weave 0.00)
broadside_gunship  gunship  hull -> cargob_lance
final_tally        flagship hull -> cargob
```

Blast damage is 750.0 on both pods, so the owner's "same blast damage" rule
survived the restructure.

Live, on the merged tree: the `torpedo_section` example ran a clean autopilot
cycle and captured both trail colours again, and a `broadside_gunship` snapshot
read `"type": "Lance"` on every torpedo in the air. `final_tally` takes no live
reading - its flagship spawns behind the survey and picket gates, which an idle
headless player never trips - so that one rests on the generated content above.

### Docs the restructure moved

Master added `web/src/wiki/modding/ships.md` with a Base ships table; the two
cargo-B entries are listed there now, and `modding/base-content.md` says the two
pod prototypes are two SHIPS rather than something a scenario mixes.

## 8. The owner's lever: `max_speed` moves onto the type

Owner decision after reading section 3: **`max_speed` becomes a per-type field
and the evasive type authors a lower one.** Chosen over a shorter
`projectile_lifetime` because it makes the fiction true - a weaving torpedo
should BE slower, not fly just as fast behind a timer.

`max_speed` therefore moved from `TorpedoSectionConfig` to
`TorpedoTypeConfig`. The bay is the tube; the type is the run-in, which is now
literally "how fast and how evasively".

### Why the lever works, stated once

`thrust_headroom(velocity.dot(nose), max_speed)` reads the cap ALONG THE NOSE.
A torpedo holding its nose off its own velocity never reaches the taper band,
never throttles down, and settles at a higher total speed - which is the whole
of section 3. Lowering the evasive type's cap lowers the band it never quite
reaches. The alternative, gating on total speed, is rejected by
`thrust_headroom`'s own doc: it leaves the torpedo ballistic at cruise and
unable to steer at all. Not touched.

### The sweep

Three standing constraints, in the owner's priority order. Constraints 1 and 2
from `point_defense_cost_tests::defend` (one stock PDC, 150 u envelope);
constraint 3 from the real-body rig over a 300 u run-in.

| Serpent cap | rounds an intercept costs | ratio vs Lance | killed at | arrival vs Lance |
|---|---|---|---|---|
| 35.0 (as shipped) | 369 | 3.18x | 38.8 u | **-1.5%, FAILS c3** |
| 34.0 | 375 | 3.23x | 39.0 u | +1.3% |
| 33.0 | 380 | 3.28x | 39.5 u | +4.4% |
| **32.0 (chosen)** | **390** | **3.36x** | **39.9 u** | **+7.5%** |
| 31.0 | 399 | 3.44x | 40.9 u | +10.8% |
| 30.0 | 409 | 3.53x | 41.8 u | +14.5% |
| 28.0 | 430 | 3.71x | 43.9 u | +22.3% |
| 26.0 | 455 | 3.92x | 45.6 u | - |
| 24.0 | 484 | 4.17x | 47.6 u | - |

Reference arm, unchanged: Lance 116 rounds, killed at 114.0 u, 9.10 s.

**The owner's caveat did not materialise, and this is the one surprise.** A
slower torpedo IS easier to lead, so the hit rate rises - but the round count is
dominated by EXPOSURE, `fire_rate x seconds in the envelope`, and slowing the
torpedo lengthens the run-in by more than the better lead solution saves. So
constraint 2 does not erode at all; it improves monotonically, 3.18x -> 4.17x.
Nothing in the sweep threatens the two types collapsing into one.

Constraint 1 is the binding one, exactly as called: kill range creeps out
monotonically, 38.8 u -> 47.6 u. The blast radius is 30 u, so the margin the
Serpent already had (8.8 u outside its own kill zone) is what erodes.

### Why 32.0

The two live constraints pull opposite ways - c1 wants the cap high, c3 wants it
low - so the disciplined pick is the smallest cut that clears c3 by more than
the rig can measure:

- **c3 satisfied with room.** +7.5%, i.e. 0.68 s over a 300 u run. 34.0 clears
  c3 arithmetically (+1.3%, about 7 frames, above the rig's +/-0.19%
  resolution) but 0.12 s in nine seconds does not make "the evasive torpedo is
  slower" true to anyone playing it.
- **c1 barely moves.** 38.8 -> 39.9 u, 1.1 u off an 8.8 u margin. The cheapest
  qualifying option by this measure, which is the one that matters most.
- **c2 improves**, 3.18x -> 3.36x.

Not 30.0, though it is tempting - see below - because c1 is priority one and
30.0 costs 3.0 u of kill range against 32.0's 1.1 u.

### What the owner may want to revisit

The trade gets much louder at 30.0, and the reason is a target that RUNS. Closing
speed against a ship fleeing at the player's 25 u/s cap:

| | Lance | Serpent @32 | Serpent @30 |
|---|---|---|---|
| closing on a 25 u/s runner | 6.30 u/s | 4.14 u/s | 2.35 u/s |
| reach against that runner (100 s life) | 630 u | 414 u | 235 u |

At 30.0 the Serpent essentially cannot catch a full-speed runner inside its own
lifetime, which is the owner's "blast something long range away that doesn't
fight back" made absolute. 32.0 keeps it merely much worse at it. That is a
playtest call, not a derivation, and it is one number.

### Measured result at 32.0

| | Lance (35.0) | Serpent (32.0) | Serpent AT 35.0 |
|---|---|---|---|
| path flown over a 300 u run | 284.3 u | 289.2 u | 289.6 u |
| time to fuze | 9.10 s | **9.78 s (+7.5%)** | 8.97 s (-1.5%) |
| speed along the line | 31.30 u/s | 29.14 u/s | 31.83 u/s |
| reach at a 100 s lifetime | 3130 u | 2914 u (-6.9%) | 3183 u |
| rounds one PDC spends | 116 | 390 | 369 |
| killed at | 114.0 u | 39.9 u | 38.8 u |

Reach is now lower for the evasive type, but neither type is reach-limited on
any shipped arena: both still out-reach the 1000 u launch envelope by ~3x. The
trade is in TIME, and against a runner.

### The test that changed

`the_weave_is_a_longer_path_that_costs_no_time_to_target` failed on the new
value, with its own message ("if it now does, the trade is real and this test's
docs are stale"). Replaced by
`evasion_costs_time_because_the_type_is_slower_not_because_the_path_is_longer`,
which pins BOTH claims off three arms:

- the shipped pair: the evasive type arrives later and reaches less far;
- **a third control arm** - the same weave at the straight type's cap - which
  still arrives SOONER, longer path and all.

That third arm is the section 3 finding kept as a live regression pin rather
than as prose: it is what to read when someone proposes deleting the evasive
type's speed penalty as redundant with the corkscrew. If it ever flips, the
thrust law changed and the penalty can be revisited.

`a_wider_weave_costs_the_defender_more_still` now holds the cruise cap fixed
across its two arms, or it would have been measuring the shipped speed penalty
rather than the amplitude it claims to sweep. `straight_type()` in both rigs
spells `max_speed` out instead of inheriting it, because the engine default IS
the evasive type and `..default()` would quietly slow the control arm to match.

### The Breaker

Unchanged at 70.0 u/s. The field move forced it to be stated somewhere, and it
is stated on the type; no behaviour moved, and nothing about the capital round's
balance was being asked about.

### Live, on the shipped game

Snapshot velocities read the trade straight out of two shipped scenarios:

- `broadside_gunship` (Lances): 6 in the air at t = 38 s, cruising **31.0-31.3 u/s**
- `menu_gauntlet` (Serpents): 6 in the air at t = 28 s, cruising **29.4-29.5 u/s**

Both match the rigs (31.30 / 29.14 along-line) to within a rounding step, and
the `torpedo_section` range now draws the difference: from one trigger pull the
pale-steel Lance trails run straight and pull visibly AHEAD of the orange
Serpent trails curving behind them.

### Also touched by the field move

`nova_editor`'s gallery reads `torpedo.torpedo_type.max_speed` for its "speed"
stat row, so the editor still shows a bay's cruise. And
`torpedo_types_differ_only_in_how_the_ordnance_flies` drops `max_speed` from its
must-match list, with the reason recorded: it lives on the type now and IS how
the ordnance flies, which is exactly what a type is allowed to change. Blast,
radius, damping, nav constant, lifetime, warhead health, fire rate, magazine and
reload all still have to match, so the owner's "same blast damage" rule is
untouched.
