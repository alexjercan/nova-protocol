# Review - 20260805-213432

Scope: `git diff 53ec7603..441db058` (worked directly on master, no sprout).
`dc02f5f9` and `26bc29e0` sit on top and are unrelated.

## Round 1

- Reviewer: out-of-context `general-purpose` subagent (`a9db15396b07a105a`),
  prompt carried task ID, commit range, dimensions and record format only.
- Primary re-ran both suites and independently re-derived the belt geometry
  (`scratchpad/geo2.py`), which found one MORE overlapping knot pair than the
  reviewer reported.
- Verdict: REQUEST_CHANGES (2 open MAJOR).

### F1 MAJOR - the pocket pin measures centre-to-centre, not clearance

`crates/nova_assets/src/scenario/shakedown/tests/pins.rs:614`

`reach = knot.half_extent.max_element()` is not a box's reach. A rock is
sampled uniformly in the WHOLE box, so the distance that matters is
point-to-AABB from the pocket centre, minus the pocket radius, minus the rock's
own collider (`BELT_ROCK_RADIUS.1 * ASTEROID_GEOMETRIC_FACTOR_MAX` = 12u). The
pin's failure message claims a clearance the layout does not have.

Independently recomputed (all five knots x all pockets, worst first):

| knot | pocket | test margin | true box-surface gap | gap after rock collider |
| --- | --- | --- | --- | --- |
| k3 | coast ring (240u) | -27.6 (not checked) | -42.1 | -54.1 |
| k4 | coast ring (240u) | -9.6 (not checked) | 5.8 | -6.2 |
| k2 | beacon 1 (70u) | +15.6 PASS | 6.2 | **-5.8** |
| k3 | orbit ring (181.5u) | +30.9 PASS | 16.4 | 4.4 |
| k1 | player spawn (60u) | +20.4 PASS | 30.0 | 18.0 |

So k2's worst-case rock reaches 5.8u INSIDE beacon 1's trigger sphere while the
pin passes by 15.6u. The tightest true margin in the layout is k3 vs the orbit
ring at 4.4u, not the "20u" the doc comment and `mod.rs:147` both assert.

This matters most because step 8 (look-and-retune) is still open: any raise of
`count`, widening of a box or raise of `BELT_ROCK_RADIUS` keeps this pin green.

Change: compute clearance as point-to-AABB distance from the pocket centre,
subtract the pocket radius and `BELT_ROCK_RADIUS.1 * ASTEROID_GEOMETRIC_FACTOR_MAX`,
require `POCKET_MARGIN` on THAT, then move k2 (and re-check k3) until it passes.
Apply the same correction to the autopilot-leg loop, which uses the same
`max_element` reach.

### F2 MAJOR - three knot boxes overlap and `min_separation` is per-scatter

`crates/nova_assets/src/scenario/shakedown/mod.rs:158-196`

`min_separation` only compares a candidate against copies placed by the SAME
`ScatterObjects` action. Each knot is its own action, so nothing keeps two
knots' rocks apart - and the boxes intersect:

| pair | overlap dims | volume |
| --- | --- | --- |
| k2 / k4 | 125 x 60 x 45 | 337 500 u^3 |
| k4 / k5 | 20 x 75 x 150 | 225 000 u^3 |
| k2 / k5 | 35 x 55 x 65 | 125 125 u^3 |

That is exactly the spawn-overlap-explosion bug `min_separation` was added to
fix, unguarded at three seams. The shipped seeds happen to land clear (probe:
78/78 spawned, 0 destructions), but nothing pins it and step 8 re-rolls it.

Change: either make the five knot boxes disjoint, or share one placed-position
set across the knots so separation is cross-scatter. Either way add a pin -
cheapest is asserting no two `BELT_KNOTS` boxes intersect.

### F3 MINOR - the `pockets` list omits the coast ring

`crates/nova_assets/src/scenario/shakedown/tests/pins.rs:596-606`

Beats 7-9 fire on `player_enters(ID_COAST_RING)` (`mod.rs:1134`, `:1198`), a
240u sphere on the planetoid. The list only carries the 181.5u widest orbit
ring, so the volume the coast beats actually own is unprotected: k3's box
surface comes to 197.9u of the planetoid, 42u inside the coast ring.

Change: add `("the coast ring", PLANETOID_POS, COAST_RING_RADIUS)` to `pockets`
and retune k3, or state in the doc comment why the coast shell may carry rock.

### F4 MINOR - CHANGELOG overstates the belt by 2x

`CHANGELOG.md:36`

"a 160-rock slalom belt" is the round-1 number. Shipped is `5 * 12 + 18 = 78`.
The same entry also carries rationale prose ("so the starter scenario stops
reading as empty black sky") against `AGENTS.md:177` ("No rationale...").

Change: correct to 78 and move the rationale to the news post.

### F5 MINOR - a dropped copy is invisible, and a neighbouring comment is now false

`crates/nova_scenario/src/actions/spawn.rs:309`, `:322-331`

The drop path logs at `debug!`, filtered out of a normal run, so an over-tight
authored region silently ships fewer bodies with no signal anywhere (content
lint has no volume-vs-separation check). `// NOTE: always the authored count`
at `:309` is no longer true once `min_separation` is set.

Change: `warn!` once per scatter with dropped-vs-authored counts; fix the
comment.

### F6 NIT - `SEPARATION_ATTEMPTS` is `pub` with one private consumer

`crates/nova_scenario/src/actions/spawn.rs:279`. The wiki quotes the literal
64, not the const. Make it private.

### F7 NIT - the determinism sentence is wrong

`crates/nova_scenario/src/actions/spawn.rs:281`: "the layout stays deterministic
because a rejected sample still advances the seeded RNG". Determinism comes from
the fixed seed plus a deterministic algorithm. Reword to "the same seed yields
the same layout, drops included".

## Judged and accepted

- `min_separation` landing in this task rather than a split: correct. The belt
  is unshippable without it (round-1 rocks destroyed each other on spawn), the
  root cause is `nominal_radius * ASTEROID_GEOMETRIC_FACTOR`, and the config is
  9 lines plus a helper. Splitting would have blocked the deliverable on a
  second task for no separation of concerns.
- Drop-on-failure over place-anyway or error: correct. Placing anyway
  reintroduces the exact bug; erroring turns an authoring-tuning mistake into a
  scenario load failure. Fewer bodies degrades gracefully. F5 is about
  visibility, not the contract.
- `ScatterRegion::Ring { center }` with `serde(default)`: every one of the 9
  construction sites carries `center`, the 4 unplanned ones are `Vec3::ZERO`,
  and the default reproduces the old hard-coded origin exactly. No silent shift.
- `web/src/wiki/dev/guide-author-scenario.md:300-341`: accurate and complete,
  including the 6x collider factor and the drop contract.
- Far-ring hole: farthest beat point is 811.8u from the planetoid, the ring
  starts at 1050u - holds with 138u to spare. Independent of F3.
- k4's move from x=200 to x=170: sound. No other knot sits on a knife edge by
  the pin's own metric (next tightest is k3 vs beacon 2 at 8.7u), but see F1 -
  that metric is the wrong one.

## Verified

| Claim | Command | Result |
| --- | --- | --- |
| nova_scenario green | `nix develop --command cargo test --lib -p nova_scenario` | 151 passed |
| nova_assets green | `nix develop --command cargo test --lib -p nova_assets` | 97 passed |
| content gen/lint clean | `content -- gen`, `content -- lint` | empty diff; 0 errors, 0 warnings, 14 scenarios |
| menu load budget | `nova_probe run menu_newgame` | OK, Playing at frame 54 |
| belt geometry | `scratchpad/geo2.py` (point-to-AABB, box intersection) | table in F1/F2 |
| SOI 329u, widest ring 181.5u | `2*sqrt(27000)`, `1.5*(20*6+1)` | 328.6, 181.5 - match |
| 78 rocks, not 160 | `5 * BELT_KNOTS.count + BELT_FAR_COUNT` | 78 |

## Pending user checks

- `manual:` step 8 owner sign-off on the round-2 belt look is still outstanding.
  The DoD is a timebox plus sign-off, not a metric. Not treated as done.

## Out of scope

- `ROCK_OFFSETS` (`shakedown/mod.rs:719`): the hand-placed debris cluster has
  pairs ~33u apart with worst-seed combined extents ~33u - the same latent
  overlap. Pre-existing. Confirmed as its own task, not this one.
- Content lint has no geometry check for scattered bodies (`close-spawn` covers
  ships only), which is why every clearance question lives in a scenario pin.
- The shared checkout carries unrelated uncommitted work (`nova_autopilot`,
  `nova_debug`, `examples/screenshots`) from a concurrent session. Only
  `tasks/20260805-213432/` was staged for this round.

## Inspection commands

```
git diff 53ec7603..441db058
nix develop --command cargo test --lib -p nova_assets shakedown
python3 <scratchpad>/geo2.py
```

## Verdict

REQUEST_CHANGES - F1 and F2 open.
