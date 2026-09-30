# Unruined wrecks: cause and proposed invariant (task #91)

Status: A is implemented (see Outcome). C3 is implemented, bounded at the target (see C3 outcome). The test plan under Proposal A predates the implementation; Outcome supersedes it.

## Observed requests

All requests: civ_0_0_0@20260922, Scavenger, advancement 0.04, base catalog.
Request seeds 4114192336, 1911787720 and 58850713 each fail with
`Unruined { omitted: 0, needed: 1 }`. The test-pack request 2431246023 was
not dumped. The three nova_world_base tests that failed on it pass with the
invariant (see Coverage).

Intact layout of 4114192336 (hull plane y=1). The other two have the same
shape with one more z=2 row and two drives:

    z=-1: cube(0)            weapons (+-1, y0)
    z= 0: dock(-3) cube(-2) cube(-1) controller(0) cube(1) cube(2) dock(3)
    z= 1: cube(0)
    z= 2: drive(0)

## Cause 1: no cube can be removed alone (42 of 66 sweep failures)

- `generate_wreck` removes dock-backing cubes from the candidates at
  `crates/nova_world_base/src/ship_layout.rs:444-445`. That removes cubes
  (+-2, 1, 0).
- `ruin` keeps a cube when removing it changes the cell bounds or splits the
  socket graph (`ship_layout.rs:645`, `joined`). Cubes (+-1, 1, 0) are the only
  path to the dock spars. Cubes (+-1, 1, 1) are the only path to the drives.
- So a tiny hull that is a tree, with a part on every leaf, has no removable
  off-centre outer cube. `wreck_floor` clamps to 1
  (`ship_layout.rs:574`), and every one of the 8 plans reaches 0.
- PR A accepted this case. `crates/nova_authoring/tests/generated_wrecks.rs:44-52`
  expects `Unruined { omitted: 0, needed: 1 }` for such hulls.

## Cause 2: the floor counts cubes that are removable alone, not together (24 of 66)

- `wreck_floor` (`ship_layout.rs:551-578`) removes each cube, tests it, and
  puts it back. So `pairs` counts cubes that can each be removed alone.
- `ruin` removes cubes cumulatively. After earlier removals, a later cube can
  become a cut vertex or a bounds carrier, so no plan reaches the floor.
- The sweep shows these failures as `omits k needs k+1`, with k = 2..5.
- The observed seeds do not have this cause. It can still occur in a world
  window.

## Sweep (temporary test, deleted)

The sweep ran on the base catalog. It covers advancement 0, 0.04, 0.1, 0.25,
0.5 and 1.0, every eligible role, seeds 0..400, and world_seed 1: 9600
requests.

- Baseline: 66 requests are Unruined, all Civilian or Scavenger at
  advancement 0.25 or lower. 42 are `omits 0 needs 1` and 24 are
  `omits k needs k+1`. One Industrial request fails its intact layout at
  advancement 0 (no cargo intake fits).

## Proposal A: an attempt-level invariant in generate_ship

In `generate_ship` (`ship_layout.rs:408-414`), reject a layout attempt whose
hull has no off-centre outer structural cube outside the dock backing that
can be removed alone with the cell bounds and `joined` kept. The loop then
tries the next seeded attempt, as it does for every other constraint. This is
the same test that `wreck_floor` applies with `pairs >= 1`.

    match built {
        Ok(layout) if spare_pairs(snapshot, &layout.design) == 0 => {
            last = Some(ShipLayoutConstraintType::<new variant>)
        }
        Ok(layout) => return Ok(layout),
        ...

Measured with a temporary flag, which is now reverted:

- Coverage: all 3 observed base requests become ruinable wrecks. The sweep
  drops from 66 to 24 Unruined, and the 24 left are exactly the Cause 2 set.
  The world_clusters example tests pass 15 of 15, where they passed 11 of 15
  before. nova_world_base lib tests pass 47 of 51, where they passed 46 of 51
  before. These three tests now pass:
  `a_cluster_is_the_same_cluster_from_every_cell_that_replays_it`,
  `every_cluster_body_has_one_owner_and_clusters_cross_faces` and
  `generated_rocks_carry_a_well_from_50_m`.
- Intact twin: the twin stays `generate_ship(request)`, because the rule
  applies to every ship. 42 of 9600 intact designs change (0.44%), all
  Civilian or Scavenger at advancement 0.1 or lower. Living ships change
  too.
- Tests that change, with the reason for each:
  - `tests::a_pinned_window_generates_the_recorded_bodies`: re-record the
    digest.
  - `ship_layout::tests::a_hull_with_no_removable_outer_cube_fails_as_unruined`:
    `generate_ship` itself now fails for that pack. Replace the test with
    one that expects the new constraint from `generate_ship`.
  - `ship_layout::tests::a_wreck_keeps_its_fittings_size_and_connection_and_breaks_its_mirror`:
    seed 1 Scavenger at advancement 0 fails with `2 < 3`. This is a test
    defect. The test computes `wreck_floor` over all cubes
    (`ship_layout.rs:2565-2570`), but production first removes the dock
    backing (`ship_layout.rs:444-445`). The new layout exposes the mismatch.
  - `clusters::tests::every_placed_hull_has_a_companion_of_its_own_cluster_in_its_cell`:
    the test window now holds 0 hulls beside only an escort (a coverage
    assertion). The window loses its example, and nothing panics.
  - `crates/nova_authoring/tests/generated_wrecks.rs:44-52`: `generate_wreck`
    no longer returns `Unruined { 0, 1 }`. The test must expect the new
    constraint from `generate_ship`.

## Alternatives

- B: apply the invariant only to wreck requests. No living ship changes, but
  a wreck is no longer the ruin of `generate_ship(request)`.
- C: add a Cause 2 fix. Compute the floor cumulatively (removed cubes stay
  removed), so that one deterministic plan reaches it, and make that plan
  one of the attempts. This removes the last 24. It needs its own review,
  because it changes the wreck quality floor.

Recommendation: A now. Review C next, because without it about 0.25% of
low-advancement Civilian and Scavenger requests can still fail a sector.

## Outcome (after owner approval of A)

The owner approved A as `ShipLayoutConstraintType::Unruinable`. It is
implemented in `check` in ship_layout.rs. `wreck_floor` now uses
`.min(target)`, because every generated hull holds at least one pair.

- Base catalog: 343 cells around the origin at each of seeds 20260922,
  20260923, 1, 2 and 3. There are 0 sector faults, where 20260922 had 2
  before.
- Tests: nova_world_base lib 51/51, world_clusters example 15/15, and
  nova_authoring generated_wrecks 1/1.
- Mutation check: with the Unruinable check disabled,
  `a_layout_with_no_removable_outer_cube_is_refused_as_unruinable` fails with
  "one layout cannot be ruined".
- The escort coverage failure in
  `every_placed_hull_has_a_companion_of_its_own_cluster_in_its_cell` did not
  come from A. PR B added `ClusterBody::Ship`, but the test filter still named
  only `Hull`, so a sibling ship counted as a companion. The test now uses
  `is_hull()`. Master passes the test. The counts (25 away, 0 escorted,
  1 alone) were the same with the check on and off.

Cause 2 is still open. The clusters scan over seeds 0..40 (a temporary
probe, deleted) found these loud faults on the test packs:

- seed 4, cell (0,-3,-5): request 245496831, civ_0_0_0@4, scavenger,
  advancement 0.10. Unruined, omits 3 of 4.
- seed 30, cell (-5,-1,0): request 4013415073, civ_0_0_0@30, scavenger,
  advancement 0.00. Unruined, omits 3 of 4.
- seed 17, cell (-3,-1,1): request 1049378793, civ_0_0_0@17, industrial,
  advancement 0.00. The intact layout fails: "no eligible cargo intake part
  fits the layout". This is a third fault class, not a wreck fault. The
  base sweep has one such request (Industrial at advancement 0, seed 92).

## Cause 2 proposal: C3, a cumulative witness plan (not implemented)

The prototype ran behind a temporary flag and is reverted. The sweep used
the same 9600 base requests as above, with A applied.

| Variant | Unruined | Wrecks that A built and that change |
|---|---|---|
| A only | 24 | - |
| C1: floor = greedy cumulative count | 26 | - |
| C2: C1 plus the witness plan | 0 | 2 (Civilian seed 304 at 0 and 0.04) |
| C3: floor = min(alone count, greedy count, target), plus the witness plan | 0 | 0 |

The greedy plan visits the non-backing cubes in `BTreeSet` order. It
removes each off-centre cube that is outer in the CURRENT hull, whose
mirror pair it has not taken, and whose removal keeps the cell bounds and
`joined`. A removed cube stays removed. It returns the count and the removed
cells. No pair loses both cubes, so every removal stays off the mirror
image.

C3 in `generate_wreck` (ship_layout.rs about 446-489):

    let (witnessed, witness) = cumulative_ruin(&cells, &cubes);   // new fn
    let floor = wreck_floor(&cells, &cubes, target).min(witnessed);
    ... seeded plans unchanged ...
    if let Some((design, _)) = best { return Ok(..) }
    // new: the plan that proves the floor is one of the bounded plans
    let design = intact design without `witness`;
    check_wreck(snapshot, request, &intact.design, &design, floor)?;  // loud on failure
    return Ok(ShipLayout { design, ..intact });

- Why C3 changes no wreck that A builds: the floor never rises, so a seeded
  plan that won before still wins. The witness runs only where every seeded
  plan fell short.
- Why the witness reaches the floor: it omits `witnessed` cubes off the
  mirror by construction, and the floor is at most `witnessed`. Unruinable
  guarantees `witnessed >= 1`, because the first cube removable alone is
  also the greedy plan's first removal.
- Not proven: that the witness always passes `check_wreck`. That check
  also runs `connected_and_clear`. In 24 of 24 uses it passed. A failure
  stays a loud layout fault.
- Open points for review:
  1. The witness is a deterministic plan that runs only after the seeded
     plans fall short. Say whether this counts as a fallback under the
     fail-loud rule.
  2. The witness removes every cube the greedy pass can remove, not
     `target` of them. It should probably stop at `target`.
  3. The witness holes are single scattered cubes, not grown breaches. No
     witness wreck was rendered.
  4. It needs a new function (`cumulative_ruin` or a better name) and a
     test on a Cause 2 request, for example base request seed 245496831 of
     civ_0_0_0@4, scavenger, advancement 0.10 (a test-pack seed; pin a
     base-catalog request from the sweep instead).

## C3 outcome (after owner approval of a bounded witness)

The owner approved C3 with the witness capped at the target. It is
implemented in ship_layout.rs as `cumulative_ruin`, which stops at `target`,
and `omit`, which is the design tail that `ruin` already had.

- `generate_wreck`: floor = `wreck_floor(..).min(cumulative_ruin(..).len())`.
  The seeded plans keep priority. The witness design is built only when no
  seeded plan passes `check_wreck` at the floor. It must pass `check_wreck`,
  else the request fails with the witness constraint and `attempts` 9.
- Base sweep (the same 9600 requests, temporary, deleted), A-only against
  C3 per request: 0 of the 9575 wrecks that A built change, all 24 Cause 2
  requests now build, and the one error left is Industrial seed 92 at
  advancement 0 (no cargo intake fits the intact layout).
- Test-pack scan (seeds 0..600, the test civilization, advancement 0 to
  0.25, temporary): 27 requests where no seeded plan reaches the floor; all
  27 build through the witness. Recorded sector faults: seed 4 request
  245496831 and seed 30 request 4013415073 now build; seed 17 industrial
  request 1049378793 still fails with no cargo intake.
- Regression: `a_wreck_whose_seeded_plans_all_fall_short_is_its_cumulative_pass`
  pins request 245496831 on the test packs. It asserts that all 8 seeded
  plans fail as `Unruined` at floor 4 and that the wreck is the witness.
  With the witness path disabled it fails with `Unruined { 0, 4 }`.
- Tests: nova_world_base lib 52/52, including the pinned-window digest,
  which did not change. world_clusters example 15/15. nova_authoring
  generated_wrecks 1/1.
- Not done: no witness wreck was rendered. The witness holes are single
  cubes, not grown breaches.

## Industrial intake fault outcome (#113)

The intact-layout fault above (base Industrial seed 92 at advancement 0, and
test-pack request 1049378793 of civ_0_0_0@17) is a spine fault, not a wreck
fault. A short spine had no flat flank for the 1x3x2 intake pair, or the pair
took the whole flank and a nose or stern port lane was clad by the intake's
sockets.

- Fix: `plan` holds one flat body run for intake-carrying roles
  (`FlankRun`, `flank_run`, `hold_flank` in ship_layout.rs), long and tall
  enough for the first ranked intake's mirrored pair beside the first ranked
  dock. A drawn body that already holds such a run is not changed.
- Base sweep (the 9600 requests above plus Industrial advancement 0 seeds
  400..2000, temporary, deleted): errors 3 -> 0 (seeds 92, 471, 1451).
  Only Industrial outputs change: 354, 329, 298, 235, 77 and 3 of 400 at
  advancement 0, 0.04, 0.1, 0.25, 0.5 and 1. Every Industrial ship carries
  2 intakes and 2 docks, and clearance stays inside the old per-advancement
  range. Other roles, ships and wrecks, are unchanged.
- The pinned-window digest does not change.
- Digest check (temporary, deleted): FNV-1a over the ship and wreck results
  of the same requests, world seed 1. With the flank run off, only the 6
  Industrial rows change, Industrial 0 has errors [92, 471, 1451], and the
  other 18 rows are equal. With it on, there are no errors, and calling each
  request twice gives the same design.
- Regressions: with the flank run off, both
  `a_short_industrial_spine_holds_a_flat_flank_for_its_intake_pair_and_a_dock`
  and `the_base_industrial_ship_that_fit_no_intake_carries_an_intake_pair_and_a_dock`
  fail with "no eligible cargo intake part fits the layout".
- Tests: nova_world_base lib 54/54. In nova_authoring, generated_wrecks 2/2,
  content_ron_parity 2/2 and content_lint_gate 3/3. nova_interface
  `inventory::` 9/9, including the docked generated wreck Take/Give.
- Render: world_ships `world-ships-low.png` under lavapipe. The industrial
  hulls are connected, with a flank dock and an intake. Only the near flank
  shows. The initial autopilot then failed at world_ships.rs:1704 because its
  assertion expected wreck docking ports to be inactive.

## PR B verification after the wreck assertion fix

- `world_ships` now checks that hull and docking sections remain active while
  other systems are inactive. The Xvfb/lavapipe autopilot completed without
  panic, captured all views, and reported four wrecks with inert drives and
  intact collision and health. The wreck frame was opened; it shows four bare
  generated hulls, not a docked inventory transaction. Capture:
  `/tmp/regional-b-gate1-IrPE0j/` (local evidence only).
- The base-content regression now exercises advancement-zero Industrial seeds
  92, 471, and 1451 at world seed 1, requiring two intakes, a dock, and a
  successful wreck for each. `generated_wrecks` passed 2/2.
- After rebasing onto PR A c0de850c9, `nova_world_base --lib` passed 54/54,
  `nova_world --lib` 29/29, and the real-ECS generated-wreck inventory
  Take/Give test passed. Authoring content lint/parity, example check, fmt,
  and diff-check passed. No workspace-wide or Clippy run is claimed.
