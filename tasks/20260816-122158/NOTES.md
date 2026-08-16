# Notes

## What was blocking

`state_to_world_system` applied EVERY queued scenario command in one
`CommandQueue::apply`. A shipped chapter's `OnStart` queues one closure per
object, so a chapter load paid the whole spawn burst as a single frame. The
loading panel was already up and rendered across it, but that frame cannot
animate - by the owner's rule it should not exist.

## The shape

Three changes, one idea: the world is not yet LIVE while its objects are still
arriving.

1. **Chunked drain** (`crates/nova_scenario/src/world.rs`). The flush pops ONE
   command at a time and applies it, stopping when `SPAWN_DRAIN_BUDGET` (3 ms)
   of wall clock is spent. A time budget rather than a command count: the
   commands are wildly uneven (one clad ship is worth hundreds of rocks), and a
   slower machine should take MORE FRAMES, never a longer frame. At least one
   command is always applied, so an object costing more than the whole budget
   still lands instead of deadlocking the queue behind it.

   Per-command application is also what keeps each object ATOMIC: a ship's
   sections all land inside one `apply`, so the `Added<SectionLinkPoints>` batch
   that `build_ship_integrity_graph` and `spawn_ship_skin` key off is complete
   the first time they see it.

2. **The gate** (`EventWorld::is_settling`, new required trait method in
   `crates/nova_events/src/engine.rs`). `NovaEventWorld` reports true while its
   command queue is non-empty. The dispatcher holds `queue_system` while it is
   true; `state_to_world_system` deliberately stays ungated, since it is what
   applies the rest. Fired events are NOT dropped - they queue and dispatch, in
   order, on the first frame the world is live. The chain's own run condition
   gained `.or_else(is_settling)` so the drain keeps being pumped on frames
   where the event queue is empty.

   `queue_system` also BREAKS its pass the moment the world starts settling.
   Without that break there is a hole the run condition cannot close: `OnStart`
   is queued on the frame the scenario loads, the `OnUpdate` pulse is queued in
   that same frame's `Update`, and both dispatch in one `queue_system` pass - so
   the pulse would read a world whose every object was still sitting in the
   queue `OnStart` had just filled. The hole predates this task; the break
   closes it, and is what makes "no handler ever runs against a world known to
   be incomplete" true rather than nearly true.

   `register_clock_and_pulse` (`loader/clock.rs`) gained the same gate
   (`scenario_has_settled`), so while spawns land the scenario clock does not
   advance, keyed timers do not expire, query watches are not sampled and the
   `OnUpdate` pulse does not fire.

3. **The panel** (`crates/nova_core/src/loading_screen.rs`).
   `dismiss_scenario_load_screen` will not take the screen down while the
   scenario is settling, so the panel and the gate are the same fact on screen.
   The spawn gate is checked BEFORE the hard cap (`SCENARIO_MAX_DWELL`): the cap
   is for a machine that never gets smooth again, not for a scene that is
   legitimately big, and a shipped chapter's queue takes 3.9 s on the software
   rig - close enough to the 6 s cap that leaving the cap in front of the gate
   would drop the panel mid-spawn on a slow machine. The queue is finite and
   always applies at least one object per frame, so this cannot hold forever.
   The dependency is optional, so a rig without the scenario engine keeps the
   frame rule alone.

## What the gate holds back

Held while queued spawns remain:

- every authored handler (`OnStart`, `OnUpdate`, `OnDestroyed`, `OnEnter`, ...)
- the scenario clock, so `Scenario.Elapsed` and every time-gated filter freeze
- keyed timers
- entity-speed query sampling and the watches it publishes
- the loading panel's dismissal

NOT held:

- `state_to_world_system` itself (the drain, the HUD/objective/comms mirrors,
  the pending scenario switch)
- `maintain_handler_index`
- physics, rendering, input, the camera - the frame is a normal frame
- `build_ship_integrity_graph` and `spawn_ship_skin`, which are ungated `Update`
  systems keyed off `Added<SectionLinkPoints>`

## Ordering hazards checked

- **`Add<...>` observers during the spawn.** Unchanged. Observer-queued commands
  already applied before the queue's remaining commands (the note in
  `actions/spawn.rs`), so the objects were already serialised one at a time
  inside the single big apply. Popping one command per apply preserves exactly
  that order.
- **The integrity graph and the skin batch.** Both run in `Update` on
  `Added<SectionLinkPoints>`, both ungated by scenario liveness, so the gate
  does not touch them. What DID change is when they run: ships now arrive over
  several frames, so those systems process one chunk's worth of ships per frame
  instead of the whole scene in one. Atomicity per ship is preserved (a ship is
  one command), which is the property they need.
- **`NovaEventWorld::clear`** already dropped undrained commands at teardown, so
  a load that interrupts a settling load discards the old queue and starts
  clean - and `is_settling` reads false again immediately.

## Test-rig fallout

Unit rigs that call `NovaEventWorld::state_to_world_system` directly and assert
on a whole multi-object batch (the three scatter tests in `actions/spawn.rs`)
now drain to settled through a `drain(&mut World)` helper instead of assuming
one call is enough. Single-object rigs are unchanged. No harness example needed
touching.

## Measurements

Rig: `menu_newgame` (the shipped New Game boot flow, so a real chapter load of
`shakedown_run` in `Playing` with the panel up) on Xvfb :77 at 1280x720,
llvmpipe, on a box also running three other build lanes. Instrumentation was
`Time<Real>` per-frame deltas plus `Instant` spans around the drain, behind an
env switch (`NOVA_PROBE_UNBOUNDED=1` restored the old single-apply flush) so
before and after came out of ONE build. All of it was removed before the commit.

Absolute numbers are a software-raster floor on a loaded box, not a target
machine; the before/after ratio is the result.

| | before (single apply) | after (chunked) |
|---|---|---|
| longest frame in the transition | **854.6 ms** | **126.2 ms** |
| the drain itself | 95 commands in 534.0 ms, one frame | 95 commands over 84 frames, worst frame 28.5 ms |
| rendered frames while the panel was up | 6 | 15 (and still going at exit) |

The whole-drain figures come from a second rig, `scene_baseline
NOVA_PERF_SCENARIO=shakedown_run`, which stays alive long enough to finish the
queue: 95 commands, 84 frames, 3.94 s, worst single drain 28.5 ms.

Per-command cost is ~5.6 ms on this box (534 ms / 95), which is above the 3 ms
budget - so here the drain takes ONE object per frame. That is the budget
working as intended, not a floor: on a machine where an object costs less than
the budget it takes several per frame.

### Panel evidence

`NOVA_PROBE_SHOTS=<dir>` shot one PNG per frame the panel was up. Twelve frames,
twelve distinct md5 sums; the logged sweep offsets are distinct on every frame
(0.0, 139.3, 175.7, 177.9, 46.5, 122.6, 159.6, 196.2, ...) and the eyeballed
frames differ in dot count, cursor blink and sweep position. Frame deltas in the
CAPTURE run are inflated (up to 1.9 s) because `save_to_disk` does a synchronous
1280x720 readback and PNG encode inside the frame; the timing table above comes
from the uninstrumented-capture runs.

### What still blocks

Two frames inside the drain window are still long, and neither is the spawn
queue: 530.5 ms and 218.8 ms, against drains of 28.5 ms and 4.9 ms in the same
frames. They land on the first frame that draws the scenario camera and the
first ship, and on the frame the 20 u planetoid's collider is ingested - the two
costs the brief named as out of scope (avian collider ingest, and first-draw
pipeline compilation under the deliberate `synchronous_pipeline_compilation:
true` from task 20260805-111329).

---

# Follow-up: the gate broke five base-content walks

`b5523a23` landed and `cargo test --lib -p nova_authoring` failed five beat
walks in `shakedown/tests/walk.rs`. The verification scope of the first round
(`-p nova_scenario -p nova_core`) was too narrow for a change that adds a
required trait method and gates every authored handler.

## The actual cause, measured

Not "the rigs never drain" - they do. `pulse` and every other helper call
`app.update()`, and each update drains a chunk. The cause is that they drain far
too SLOWLY for a fixed frame count:

- after `boot()` (2 updates) the world is still settling
- shakedown's `OnStart` burst needs **62 further `app.update()` calls** to drain
  on this box (an independent run measured 47 - see below)
- every rig helper ran 2 to 14 updates

So the rigs never reached a live world, `queue_system` stayed gated, and no
handler after `OnStart` ever fired: objectives never posted, markers never
moved, gates never unlocked. Confirmed with a throwaway probe that counted
updates-to-settle, not inferred from the symptom.

## Why a frame count can never be right

`SPAWN_DRAIN_BUDGET` is WALL CLOCK and the drain tests `elapsed()` AFTER
applying each command, so any single command costing more than the budget forces
one object per frame. How many objects land per frame is therefore a function of
machine load and build profile, not of the content: the same burst measured 47
and 62 frames on two runs. Every fixed-frame rig that queues spawns is
timing-dependent; the five deterministic failures were just the ones far enough
over the line. A rig that asks `is_settling` is immune.

## The shared helper

`crates/nova_scenario/src/test_support.rs`, behind a `test-support` feature
(`#[cfg(any(test, feature = "test-support"))]`), mirroring
`nova_gameplay::test_support`. Consumers enable it as a dev-dependency feature.

- `settle_spawns(&mut App)` - runs the app to a FIXED POINT: drain the queue,
  spend one frame letting the dispatcher release what it held, and repeat if
  those released handlers queued more. A single release frame is not enough -
  `OnStart` settles, the held `OnUpdate` dispatches, and beat one queues its
  beacon - and would hand back a settling world, making every call site
  responsible for calling it twice. Pinned by
  `settling_reaches_a_fixed_point_when_a_released_handler_spawns`, which is
  mutation-proven: reverting to one release frame fails it.
- `drain_spawns(&mut World)` - the `App`-free twin for rigs that drive
  `state_to_world_system` directly. Replaces the local `drain` helper the first
  round grew in `actions/spawn.rs`.

Both bound their loops and panic rather than hang.

## Which rigs needed it, and why those

A rig is at risk only if a handler it REGISTERS queues world work. The seven
`nova_assets` / `nova_authoring` slice rigs all register with
`.filter(|e| !matches!(e.name, EventConfig::OnStart))`, so the `OnStart` burst
never happens in them - but several of their scenarios spawn from LATER events.
Scanned the content rather than guessing:

| rig | spawns outside OnStart | action |
|---|---|---|
| `shakedown/tests/walk.rs` | OnUpdate 4, OnTimerEnd 1, OnDestroyed 1 | fixed (the five failures) |
| `nova_assets/tests/lifeline_convoy.rs` | OnUpdate 7 | hardened |
| `nova_assets/tests/final_tally_claim.rs` | OnUpdate 2 | hardened |
| `nova_authoring/tests/broadside_assault.rs` | OnEnter 2 | hardened |
| `nova_assets/tests/scenario_branch_choice.rs` | OnEnter 1, OnDefeated 1 | hardened |
| `nova_assets/tests/scenario_provocation.rs` | OnUpdate 2, plus hand-run ship spawns | hardened |
| `nova_assets/tests/scenario_act_machine.rs` | none | structurally immune |
| `nova_assets/tests/scenario_gate_course.rs` | none | structurally immune |
| `nova_assets/tests/neutralized_ships.rs` | none | structurally immune |
| `nova_scenario/tests/skybox_swap_e2e.rs` | none (already loops on a deadline) | structurally immune |

Each hardened rig gained a local `step(app)` - two frames to dispatch the event
just fired, then `settle_spawns`. Only the `app.update(); app.update();` PAIRS
were replaced; the lone `app.update()` delivery guards ("nothing advances on its
own") were deliberately left alone, since settling there would defeat what they
assert.

The gate itself was not touched. It is the feature.
