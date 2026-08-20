# What the simulation costs with the renderer gone

Phase 6 steps 2 and 3 of `tasks/20260818-220812/PLAN.md`, run 2026-08-20 on
master at `cfdfd397`, after the point-defence clock fix and the torpedo material
fix both landed.

The question the epic set: **are physics and logic under 10 ms a frame?** The
plan argues for a stricter reading - the fixed step is 15.625 ms, so ~8 ms is
the ceiling and 5 ms the target, because a step that fills most of its own
interval cascades rather than degrades.

**Read every number here against the standing caveat**: headless is not the same
simulation minus pixels. The `Update` loop runs 5-10x more often per fixed step,
so anything per-FRAME is inflated per unit of simulated time and anything
per-STEP is unchanged.

## How to run one

The asset root resolves against the EXECUTABLE, not the working directory, so a
hand-run of a built example needs it named:

```bash
BEVY_ASSET_ROOT=$PWD NOVA_NORENDER=1 NOVA_AUTOPILOT=1 NOVA_PERF=1 \
  NOVA_PERF_OUT=<dir> ./target/debug/examples/stress_point_defense
```

Without `BEVY_ASSET_ROOT` the run dies in `fill_ui_font` and the error blames a
path "declared in a collection", which is a plausible and wrong explanation of
an invocation mistake.

## Result 1 - the point-defence range is CHEAP, and it reproduces

Four captures, saturated (the range gates its own capture on saturation with
`ready_frametime(envelope_is_full)`, so the window is the worst case it builds).

| run | mean | p50 | p95 | p99 | 1% low | peak rounds | colliders |
|--:|--:|--:|--:|--:|--:|--:|--:|
| 1 | 3.15 ms | 1.45 | 12.14 | 21.63 | 46.2 fps | 2,415 | 2,618 |
| 2 | 2.86 ms | 1.40 | 11.26 | 18.10 | 55.3 fps | 2,421 | 2,626 |
| 3 | 3.11 ms | 1.43 | 11.61 | 21.57 | 46.4 fps | 2,419 | 2,624 |
| 4 | 1.81 ms | 1.26 | 4.77 | 9.76 | 102.5 fps | 2,421 | 2,626 |

Fixed steps per frame: 0 to 2, and 64 steps a second in every run - the sim
clock is exact. Where the frame runs no step at all it still costs 1.8-2.5 ms;
where it runs one, 2.3-5.2 ms.

**Envelope fill 7.1 s, against the point-defence lane's 7.1 s.** Peak rounds
2,415-2,421 against its 2,411. The fix reproduces exactly on a different day.

So at 2,400 rounds, 2,600 colliders, 55 inbound torpedoes, 12 mounts and 12
bays, **the simulation is nowhere near its budget.** Physics and logic are not
what makes this range expensive; its cost was assets, and that has now been cut
twice.

## Result 2 - the ARENA misses 60 fps with no renderer at all

Same protocol, `wfc_arena`. Captures that were not contaminated (see the footgun
below):

| roster | mean | p95 | p99 | 1% low | max steps/frame |
|---|--:|--:|--:|--:|--:|
| 1v1 | 9.76 ms | 34.5 | 66.8 | 15.0 fps | 6 |
| 1v1 | 12.25 ms | 49.0 | 82.9 | 12.1 fps | 6 |
| 1v1 | 12.62 ms | 46.2 | 78.3 | 12.8 fps | - |
| 1v1 | 16.38 ms | 56.3 | 89.9 | 11.1 fps | - |
| 4v4 | 11.69 ms | 45.4 | 86.0 | 11.6 fps | - |
| 4v4 | 12.81 ms | 58.8 | 110.8 | 9.0 fps | 8 |
| 4v4 | 13.20 ms | 52.9 | 84.9 | 11.8 fps | 8 |

**This is the finding.** Nothing is drawn. There is no window, no adapter, no
render world. And the arena still averages 10-16 ms a frame with a 1% low
between 9 and 15 fps, running up to EIGHT fixed steps in one frame.

Two consequences, and the second is the uncomfortable one:

1. **The owner's "1v1 at 60 fps" target cannot be reached by render work
   alone.** Deleting the entire renderer does not get 1v1 to a reliable 60.
2. **Four times the ships costs 15-30%, not 4x.** 1v1 and 4v4 overlap. Whatever
   dominates the headless arena frame is a per-SCENE constant, not a per-ship
   cost - which is the exact opposite of the rendered line (roughly 8 ms per
   ship), and it means the two transports are limited by different things.

## Where the headless time goes

Traced 1v1, 2,483 frames, self-time per frame:

| what | per frame | note |
|---|--:|---|
| `PostUpdate` schedule, unattributed | 1.23 ms | executor plus untraced systems |
| visibility, all systems together | ~1.15 ms | **for a view that does not exist** |
| `Update` schedule, unattributed | 0.53 ms | |
| `propagate_parent_transforms` | 0.29 ms | 3,180 calls over 2,483 frames |
| `state_to_world_system` (`nova_scenario`) | 0.23 ms | |
| `update_ai_target` (`nova_ship`) | 0.21 ms | |
| `wfc_arena::track_damage` | 0.15 ms | the EXAMPLE's own instrument |
| `mark_dirty_trees` | 0.12 ms | |

Per fixed step: `collect_collision_pairs<ProjectileHooks>` 0.46 ms,
`FixedPostUpdate` 0.77 ms unattributed, `FixedUpdate` 0.45 ms unattributed,
`PhysicsSchedule` 0.21 ms.

**The visibility line is the one to look at.** `reset_view_visibility`,
`check_visibility_cpu_culling`, `mark_newly_hidden_entities_invisible` and their
`par_for_each` bodies run every frame over thousands of entities in a run with
no camera output. Rendered that work is necessary; headless it is pure waste,
and it is roughly 10% of the headless frame. Not fixed here - recorded, because
it also means **every headless number in this document is pessimistic by about
that much**, and the honest sim-only figure is lower than what is tabulated.

`track_damage` deserves its own note: the arena's own measurement instrument
costs about 1.2% of its own frame.

## The systemic defect: three instruments index by FRAME COUNT

This cost the most time here and it will cost it again.

- `HOLD_FRAMES = 120` - how long `stress_point_defense` holds saturation.
- `DEFAULT_CENSUS_FRAME = 90` - when the census counts the world.
- `wfc_arena`'s 360-frame measured window.

Each is a count of FRAMES standing in for a duration. Headless multiplies the
frame rate by 5 to 10, so each becomes a window of a fifth to a tenth of the
simulated time it was sized for. Three separate wrong answers came out of this
in one session:

1. **The point-defence range's own summary line contradicted the lane that had
   just fixed it** - trigger duty 0.401 against its reported 0.811, mean aim
   error 20.0 deg against its 3.6. Both are real readings of a 120-frame window,
   which at 442 fps is 0.27 s of simulated time and catches a transient. The
   INVARIANT readings - envelope fill, peak rounds - agreed to 0.3%. Nearly read
   as a regression in a fix that is correct.
2. **The census reported the same scene for 1v1 and 4v4** - 6,443 against 6,446
   entities, an identical 1,686 skin plates - because 90 frames after `Playing`
   is 0.3 s headless, before the ships are fielded. Pushed to frame 1,200 the
   same rosters read 11,811 entities / 4,474 mesh instances and 6,169 / 0. **The
   headless census is currently measuring whatever happens to exist.**
3. **Arena captures do not reproduce**, because a 360-frame window at 100+ fps
   is about 3 s of a fight that runs a minute, and it lands somewhere different
   every run.

The fix is to index these by simulated time or by fixed steps, not by frames.
Not done here: `HOLD_FRAMES` is a frame count deliberately, so that a measured
hold outlasts the capture's own frame window, and another lane has just tuned
against it. It needs one change across all three, not three local patches.

## Two things fixed to get here

`wfc_arena` could not run headless at all. Two systems took a render-gated
resource as a required `Res`, so the app panicked before fielding a ship:
`gate_team_chevrons` on `HudVisibility`, and `lobby::load_or_open_lobby` on
`UiSkin` - the latter on a path that returns before drawing anything under
`NOVA_AUTOPILOT`. Both now take `Option<Res<...>>` (`cfdfd397`).

This is the same class as the nine `systems/` ranges that cannot run headless.
The general shape: **a render-gated plugin's resource, required by an example.**

## The footgun, second sighting

`probe run` leaves a `debug,trace` binary at the same path the plain build
writes, so the NEXT hand-run is silently a traced run. It contaminated two
captures here (a 1v1 reading 19.9 ms and a 4v4 reading 7.1 ms, both discarded),
detected only because two 50 MB `trace-*.json` files appeared in the repo root.
It is already recorded in the epic's decisions and it caught the next person
anyway, which is an argument for making the traced binary a different path
rather than a better note.
