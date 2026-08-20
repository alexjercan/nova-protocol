# What makes the frame spike, and what the scenario engine costs

Measured 2026-08-20 on `1386c68f`, headless (`NOVA_NORENDER=1`), never under
Xvfb. Host: i9-12900F, 24 threads, `dev` profile (opt-level 1,
debug-assertions on) - every millisecond here is a DEV millisecond.

The question this answers: the arena's mean is survivable and its 1% low is
9-15 fps. **The gap is the defect.** Mean self-time tables cannot find it - a
system that costs 0.1 ms on 99 frames and 60 ms on the hundredth averages to
nothing - so everything below ranks by the WORST frame and attributes that
frame, not by the mean.

## First: the trace on master measures the wrong thing

`probe-runs/96c68e22/wfc_arena/trace.json` is a **4v4**, not the 1v1
`notes-headless-simulation.md` calls it, and **it never reaches the fight.**

The profiled pass sets `TRACE_CHROME` but NOT `NOVA_PERF`
(`crates/nova_probe_cli/src/native/env.rs`, `trace_pass_env`), and the arena's
autopilot exits one step after first contact. So the traced process dies at
t=10.9 s, and the frame-time capture - which is gated on `fight_happened` and
runs 60+360 frames PAST it - never opens. Split by phase, that trace reads:

| phase | frames | mean | max |
|---|--:|--:|--:|
| load (0-249) | 250 | 7.44 ms | 141.6 ms |
| approach (250-2482) | 2233 | 4.04 ms | 14.4 ms |

Steady state 4 ms, no frame over 15 ms. That is why the old table named
visibility and `PostUpdate`: **those are what dominates a CHEAP frame.** None
of it is the fight.

A trace that covers the fight needs `NOVA_PERF` armed as well, so the capture
collector holds the process open:

```bash
BEVY_ASSET_ROOT=$PWD NOVA_NORENDER=1 NOVA_AUTOPILOT=1 NOVA_PERF=1 \
  NOVA_PERF_OUT=<dir> TRACE_CHROME=<dir>/trace.json ./target/debug/examples/wfc_arena
```

That is the run everything below reads: 2,968 frames, 16.8 s, 4.9 GB.

## The defect reproduces

Three clean headless 1v1 captures, same binary, same seed, 360-frame window
gated on the fight. Run b additionally suppressed the `nova=debug` log stream
with `RUST_LOG`; that is the logging control, read below.

| run | mean | p50 | p95 | p99 | max | 1% low |
|---|--:|--:|--:|--:|--:|--:|
| a | 13.60 | 12.72 | 30.88 | 73.57 | 101.7 | 13.6 fps |
| b (quiet) | 14.45 | 10.33 | 51.30 | 94.08 | 136.9 | 10.6 fps |
| c | 9.13 | 2.77 | 33.09 | 74.84 | 114.6 | 13.4 fps |

The mean does not reproduce (9.1-14.5). **The 1% low does: 10.6-13.6 fps.** So
the tail is the stable, measurable property, and it is the one to fix.

## Cause 1 - a fixed step eats 58% of its own interval

Capture window, 365 frames, 285 fixed steps:

- **58.3% of all wall time in the window is inside `FixedMain`.**
- **9.02 ms mean per step**, against the 15.625 ms interval a 64 Hz step owns.

Frames bucketed by their own duration, split into step time and everything
else:

| frame time | n | mean ms in FixedMain | mean ms outside | steps | ms per step |
|---|--:|--:|--:|--:|--:|
| < 8 ms | 144 | 0.00 | 3.52 | 0 | - |
| 8-16 ms | 143 | 7.48 | 4.26 | 142 | 7.53 |
| 16-32 ms | 58 | 14.89 | 7.62 | 87 | 9.93 |
| 32-64 ms | 18 | 28.68 | 15.52 | 52 | 9.93 |
| >= 64 ms | 2 | 78.19 | 11.87 | 8 | 19.55 |

Read this against the cause-versus-effect trap: a slow frame OWES steps, so a
high step count on a slow frame proves nothing by itself. The last column is
the answer to that objection. Per-step cost **RISES** with frame time (7.5 ->
9.9 -> 19.6 ms) instead of falling. A frame that was slow for an unrelated
reason and then paid its debt would show CHEAP steps; these are the expensive
ones.

So the mechanism is arithmetic, not mysterious. One step costs 7.5-19.6 ms of a
15.625 ms interval. Per-frame work costs another 3.5-15.5 ms. Their sum exceeds
the interval, the fixed accumulator never drains, and the next frame owes more
steps. Bevy's 0.25 s max-delta clamp is the only floor: up to 16 steps can land
in one frame, and 8 were seen.

Note also the third column: the non-step part grows 3.5 -> 15.5 ms too, so the
step is not the whole story - see cause 2, which lives there.

**A step is avian, almost entirely.** Self time inside `FixedMain` over the
window's 285 steps (summed across threads, so it exceeds wall):

| what | us/step | invocations |
|---|--:|--:|
| `FixedPostUpdate` schedule self | 1206 | 285 |
| avian solver-body `par_for_each` (3 queries) | 2077 | 36,630 each |
| avian position/transform `par_for_each` | 1434 | 24,545 |
| `collect_collision_pairs<ProjectileHooks>` | 675 | 285 |
| `update_solver_body_aabbs` | 664 | 285 |
| `trigger_collision_events` | 592 | 285 |
| `FixedUpdate` schedule self | 447 | 285 |
| `shoot_spawn_projectile` command flush | 392 | 285 |
| `SubstepSchedule` + XPBD solver | ~1050 | 1,710 |
| `on_impact_collision_deal_damage` | 180 | 23,363 |

1,710 substeps over 285 steps is **6 substeps per step**, each doing three full
solver-body passes over the scene's colliders. The census counts **4,663
colliders** (`Collider`, `ColliderAabb`, `ColliderOf`, ... all at 4,663) because
every hull SECTION is its own collider. Nova's own code inside a step is
`shoot_spawn_projectile` (0.39 ms) and `on_impact_collision_deal_damage` (0.18
ms); everything else above is the physics engine.

**The lever is the collider count and the substep count, not nova's step
systems.**

## Cause 2 - `torpedo_detonate_system` searches the whole collider tree, per torpedo, per frame

Worst single named system in the run: **17.39 ms in one frame**, 802.1 ms total
across 2,826 invocations.

On frame 2606 (63.41 ms) the main thread sits in a **single 17.15 ms gap inside
`Update` with no traced child on tid 0 at all**; the only activity in the gap is
`torpedo_detonate_system` running on tid 7. The schedule cannot close until it
returns, so it is on the critical path.

`crates/nova_ship/src/sections/torpedo_section/projectile.rs:98`:

```rust
let projection =
    spatial.project_point_predicate(at, true, &SpatialQueryFilter::default(), &|collider| {
        q_collider_of.get(collider).is_ok_and(|of| of.body == target)
    })?;
```

`SpatialQueryFilter::default()` is NO filter, so this is a nearest-point search
over the whole BVH of 4,663 colliders, and the target test is a PREDICATE. A
predicate rejection cannot tighten the search radius, so the traversal degrades
towards a full tree walk for every torpedo, every frame. It is registered in
`Update` (`bay.rs:849`, `projectile.rs:481`), so it also runs more often the
faster the machine is.

The cost is a distance to ONE known body. It does not need a world query.

## Cause 3 - the blast and severing cascade

Frame 2618, the worst frame of the fight at **112.78 ms**: 99.47 ms of it is
four fixed steps. Attribution:

| what | ms | count |
|---|--:|--:|
| `FixedPostUpdate` self | 17.23 | 4 |
| `resolve_nova_blast_hits` command flush | 14.43 | 4 |
| `resolve_nova_blast_hits` | 11.08 | 4 |
| `trigger_collision_events` | 9.88 | 4 |
| `queue_depleted_section_sever` | 4.74 | 773 |
| `remove_collider_on<Remove, ColliderMarker>` | 2.47 | 800 |
| `add_to_tree_on<Insert, Collider...>` | 1.65 | 722 |
| `on_impact_collision_deal_damage` | 2.18 | 4,607 |

**1,522 collider-tree edits and 4,607 collision observers in one frame.** This
is the shape to instrument: the count that causes the cost is collider-tree
churn per frame, and it is countable without asserting a millisecond.

## Cause 4 - the load hitch is the scenario spawn drain (see the engine section)

## What is RULED OUT, with the number that rules it out

- **`synchronous_pipeline_compilation`** (`nova_core/src/lib.rs:540`). Cannot
  apply: `NOVA_NORENDER` sets `wgpu.backends = None`, so there is no render
  sub-app, no pipeline cache and no compile. It remains a real hazard for a
  RENDERED run and is untested here.
- **Asset loading on the main thread mid-run.** 2,034 `asset loading` spans
  inside the 4.1 s window, **11.78 ms total, max 0.07 ms each, on tids 1/3/4** -
  the asset IO threads, never tid 0. Not a frame cost.
  It IS a defect of its own: **14,567 loads of `base/sounds/rcs_loop.wav` in one
  16.8 s run** (867/s). `compute_rcs_loop_volume` calls
  `AssetRef::resolve(&asset_server)` per controller section per frame, and
  `resolve` is `asset_server.load(path)`
  (`crates/nova_gameplay/src/asset_ref.rs:101`).
- **`nova=debug` log volume.** Run a logs 52,446 `integrity::core` DEBUG lines
  (~10 MB); run b suppresses them with `RUST_LOG` and reads p99 **94.08 ms
  against 73.57**. Logging does not explain the tail. (Two runs, and the window
  does not reproduce its mean, so this rules logging OUT as the dominant cause,
  not as a contributor.)
- **Archetype fragmentation.** 527 archetypes at frame 90, **525 at frame
  1500** - flat, not growing with the fight. 20 archetypes hold 5,504 of 6,171
  entities (89%); the other 505 hold 667 between them. Iteration runs over the
  dense 20. Not a per-frame cost at this scale.
- **A 52.34 ms `on_impact_collision_deal_damage` invocation** on frame 2560 is
  in the data and is NOT claimed as a cause: it is 1 of 34,508 invocations, the
  other 34,507 are all under 0.03 ms, and it happened in a traced run where the
  chrome writer can block. Unexplained, one sighting, probably instrument.

## The scenario / modding engine: 1.2% per frame, 100% of the load hitch

### What runs when

| when | what |
|---|---|
| per FRAME (`Update`) | `tick_scenario_clock`, `sample_scenario_queries`, `tick_scenario_timers`, `fire_on_update`, `track_player_locks`, `apply_pending_skybox_swaps`, `reconcile_render_scale`, four `asteroid_carve` systems |
| per FRAME (`PostUpdate`) | `maintain_handler_index`, `world_to_state_system`, `queue_system`, `state_to_world_system` |
| per FIXED STEP | `track_orbit_transitions` only |
| per EVENT | the observers: `area::on_collision_start_event`, `area::on_collision_end_event`, `salvage::on_crate_pickup_play_sfx`, `on_add_entity_with<...>` |

`fire_on_update` fires `OnUpdateEvent` every live frame, which keeps the queue
non-empty, which un-gates the whole `PostUpdate` dispatch chain every frame.
So "the interpreter runs every frame" is true - and it is not where the money
goes.

### Measured, in the FIGHT window (365 frames, mean frame 12.24 ms)

| span | us/frame | runs in window |
|---|--:|--:|
| `salvage::on_crate_pickup_play_sfx` | 29.2 | 23,363 |
| `area::on_collision_start_event` | 27.3 | 23,363 |
| `sample_scenario_queries` | 24.1 | 365 |
| `asteroid_carve` (4 systems + their flushes) | 14.4 | 365 each |
| `lifecycle::on_add_entity_with<...>` | 10.4 | 7,535 |
| `fire_on_update` (system + commands) | 5.6 | 365 each |
| `scenario_is_live` run condition | 4.9 | 2,030 |
| `area::on_collision_end_event` | 4.1 | 3,532 |
| `maintain_handler_index` | 3.0 | 365 |
| **`queue_system` - the interpreter** | **2.7** | 365 |
| `state_to_world_system` | 2.7 | 365 |
| `world_to_state_system` | 0.8 | 365 |
| the other 23 spans in the two crates | 21.4 | - |
| **TOTAL, 37 span names** | **150.6** | |

**150.6 us on a 12.24 ms frame = 1.2%.** The interpreter itself is 2.7 us -
**1.8% of the scenario engine's own cost and 0.02% of the frame.**

The `asteroid_carve` line deserves its own note: four systems (`seed`,
`collect_seeds`, `carve`, `collect_remeshes`) run EVERY frame plus their
command flushes, in a scenario whose asteroid fields were seeded once during
load. 14.4 us/frame for nothing.

Corroborated on the master 4v4 trace, approach phase (2,233 frames, 4.04 ms
mean): `state_to_world_system` mean **3.1 us**, p99 7.2 us, max 14 us;
`sample_scenario_queries` 23.4 us; `queue_system` 2.2 us. Same shape.

### Does it scale with authored content, or with the world?

**With the world, not with the scenario.** This is the finding.

- `queue_system` is O(events fired) x O(handlers bound to that event NAME) x
  O(filter tree). `EventHandlerIndex` buckets handlers by name, so an unrelated
  handler costs nothing. Filters are matched against the event's
  `GameEventInfo` - a `serde_json::Value` - and NOT against candidate entities.
  **Nothing in the dispatcher iterates entities.** `wfc_arena` loads 1 handler;
  the heaviest shipped chapter, `shakedown_run`, has 19 `OnUpdate` handlers and
  127 objects. 2.7 us is one event through one handler including the queue
  drain, so a scenario needs on the order of a THOUSAND `OnUpdate` handlers
  before dispatch costs 1 ms a frame.
- `sample_scenario_queries` (`loader/clock.rs:23`) queries
  `(&EntityId, &LinearVelocity)` over the whole world **every frame**, clones
  `id.0` into a fresh `HashMap<String, Option<f64>>`, then `sample_entity_speeds`
  re-keys every entry into a `QueryConfig` with ANOTHER `String`. That is two
  String allocations per matching entity per frame, **whether or not the loaded
  scenario declares a single watch.** 1,839 entities carry `EntityId` in this
  1v1 (1,765 of them ship sections); the ones that also carry `LinearVelocity`
  are the free bodies, and severing turns sections into free bodies.
- The two `add_observer` collision observers - `area::on_collision_start_event`
  and `salvage::on_crate_pickup_play_sfx` - are GLOBAL, so avian's every
  `CollisionStart` dispatches into `nova_scenario`. 23,363 invocations each in
  4.1 s **in a scenario with zero areas and zero salvage crates.** Together
  56.5 us/frame, **21x the interpreter**, all of it declined in the first two
  lines of each body.
- `state_to_world_system` clones `objectives` and `story_messages` and rebuilds
  the `hud_readouts` vec every frame, and compares `variables` against
  `last_logged_variables` every frame. All O(authored size), all unconditional.

### The load hitch IS the engine

Frames 0-399 of the traced run: `state_to_world_system` totals **568.4 ms**,
**max 23.1 ms in one frame**. On the master 4v4 the same system runs 12-24 ms
on 43 consecutive frames (155-205), and it is the top main-thread contributor
in 35 of the 50 worst frames of that trace.

`SPAWN_DRAIN_BUDGET` is 3 ms (`world.rs:46`) and the check is AFTER the command:

```rust
command(&mut commands);
queue.apply(world);
if started.elapsed() >= SPAWN_DRAIN_BUDGET { break; }
```

One authored object is one command, and one WFC hull is 200-250 sections. The
budget can only stop BETWEEN ships, so it is overrun by **4x to 8x**, every
frame, for the whole spawn. The doc comment already knows ("an object costing
more than the whole budget still lands"); what the measurement adds is that in
the shipped arena this is the NORMAL case, not the edge case, and it is the
loading hitch a player sees.

### Can a mod author tank the frame rate?

Yes - but not through the interpreter. Through four unbounded surfaces:

1. **`StoryMessage` on an `OnUpdate` handler.** `story_messages` is append-only
   for the scenario's life and `state_to_world_system` CLONES it every frame.
   Cost per frame grows without bound; total work is quadratic in scenario
   length.
2. **`Objective` / `HudReadout` churn.** Same clone-per-frame shape.
3. **`SpawnScenarioObject` / `ScatterObjects` on `OnUpdate`.** Every pulse
   queues commands, `is_settling` stays true, `state_to_world_system` burns its
   3 ms (or one object's cost) every frame forever. It self-limits only because
   the pulse is gated on `scenario_has_settled` - so the scenario silently
   STOPS instead of recovering.
4. **Entity count**, via `sample_scenario_queries`, which no authored content
   controls at all.

**Cheapest guardrails, in order of ratio:**

1. Gate `sample_scenario_queries` on the loaded scenario declaring at least one
   `QueryConfig::Entity` watch. One `run_if`. Removes 24 us/frame and the whole
   entity-count coupling, and it is currently dead work in every shipped
   scenario that has no watch.
2. Scope the two collision observers to their own entities
   (`commands.entity(area).observe(...)`) instead of `app.add_observer`.
   Removes 56.5 us/frame and stops the scenario crate from paying per collision
   in the whole world.
3. Gate the four `asteroid_carve` systems on there being a carve to do.
   14.4 us/frame, all of it after the fields are seeded.
4. Cap `story_messages` (ring buffer) and make the objective/story sync
   write-on-change rather than clone-then-compare.
5. Subdivide the spawn command so `SPAWN_DRAIN_BUDGET` can stop INSIDE a hull.

For the Lua question: **the current interpreter costs 2.7 us/frame.** Moving it
to Lua cannot buy frame time, because the interpreter is not spending any. What
a rewrite must not lose is `EventHandlerIndex`'s name bucketing and the fact
that filters never touch the ECS - those are why the number is 2.7 us. And it
must not inherit the four surfaces above, which are properties of the WORLD
SYNC, not of the language.

## What I would fix first, and why

**`torpedo_detonate_system`'s spatial query.** It is one function, it is the
worst named system of the FIGHT (17.39 ms in one frame), it is on the frame's
critical path, and the fix is local: the system already HAS the target entity,
so read the target's collider and project onto it, or pass a
`SpatialQueryFilter` naming the target's colliders instead of rejecting the
world in a predicate. Nothing about the game changes.

Second: the two global scenario collision observers. One line each, 56.5
us/frame, and it removes a coupling that will get worse as scenarios grow.

Third, and it is the big one but it is not a one-liner: **4,663 colliders is
the fixed step's cost.** Six substeps over per-section colliders is what puts a
step at 9 ms out of 15.625, and everything else in the tail follows from that.
A compound collider per hull, or fewer substeps, is the only change that moves
the 1% low.

Explicitly NOT first: the visibility work the previous round flagged. It is
~1.15 ms of a 4 ms APPROACH frame and a much smaller share of a 12 ms fight
frame, and D18 already records that cutting it buys the player nothing.

## Instruments this needs and does not have

- The traced pass cannot see the fight (above). Until `trace_pass_env` arms
  `NOVA_PERF`, every profiled run measures the approach.
- No per-frame frame-time dump. `PerfState::samples` is summarised and dropped,
  so a spike cannot be correlated with anything without a chrome trace. A CSV
  of `(frame, ms, fixed_steps)` would make every question above cheap.
- The census refuses a frame past the run's length (`NOVA_PERF_CENSUS_FRAME=2700`
  wrote nothing at all, silently) and it indexes by FRAME COUNT, which D17
  already records as wrong headless.
