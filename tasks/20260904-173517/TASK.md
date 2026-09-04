# Budget the debris a collapse throws

- STATUS: OPEN
- PRIORITY: 67
- TAGS: v0.13.0, performance


## Report

`tasks/20260904-155338` ended on a design question it could not answer alone.
After the centre-of-mass fix the worst frames of a capital-hull collapse are
avian, not nova, and the population driving them is one collapse's worth of
bodies: about 800 wreck pieces all becoming `RigidBody::Dynamic` with a collider
on the SAME frame, plus about 4000 carve shards being simulated and despawned.
Two options were named there and neither was taken, because both change how a
collapse LOOKS. The owner took both.

## Decisions

### The instrument comes first

Nothing in the roster could see this. `system_railgun_lance` mounts the
STANDARD lance and pins its plates at a health no layer ever dies at, so no
range fired a siege lance into a capital hull and none recorded a collapse
frame cost.

`examples/systems/stress_hull_collapse.rs` is that range. It stands a 9x9x16
block of reinforced hull - 1296 cells - fires ONE siege slug down its long
axis, and lets the collapse run to completion. It asserts three behavioural
facts and records two costs:

| claim | kind |
|---|---|
| one siege slug opens exactly its rake corridor | asserted |
| every corridor cell left the hull | asserted |
| every wreck piece went physical | asserted |
| the collapse frame cost is recorded | recorded |
| the debris the collapse threw is recorded | recorded |

The content is PINNED in the range - the block, the lance, the cell health, the
range - so a scene re-cut cannot move the subject under the measurement.
`examples/playable/first_shift_08_attack_salvo.rs` stays the hand-run
cross-check against real mainline content.

Three delivery guards keep the range honest about what it is measuring: the
block must stand up as exactly 1296 cells, the corridor arithmetic must still
come to 45 cells per layer at a 3 unit rake, and the deepest layer must lie
inside ONE fixed step's sweep (1500 u/s x 1/64 s = 23.44 u of reach, less the
3 u rake radius, against the deepest layer's 19.0 u). A timestep or speed
change fails loudly rather than quietly measuring a partial corridor.

### The shard ceiling is on the FRAME, not on the carve

`spew_carved_material` asks `look.count(radius)` per carve, which is 2 chips
for a PDC round and 7 for anything that carves wide. That is right, and one
carve's worth was never the problem. The problem is that a raked corridor
announces hundreds of craters in ONE command flush, and the frame that has to
create them pays for every one at once and then carries them for 2.5 seconds.

So `SHARDS_PER_FRAME = 128` is a shared allowance, refilled in `First` so a
carve announced from `FixedUpdate` draws on the same budget as one from
`Update`. A lone carve is untouched - the widest single carve in the catalog is
rock at its ceiling, 12 chips, so eight simultaneous craters of the worst kind
still throw everything they would have. Over the budget, the craters that
arrive late in the frame go unchipped. That is the right thing to drop: a frame
over the budget is one in which hundreds of craters opened together, it is
already throwing real severed geometry, and nobody can count chips in it.

Held down by `one_carve_still_throws_everything_its_crater_is_worth`,
`one_frame_cannot_be_made_to_throw_more_than_its_budget` and
`the_next_frame_gets_its_chips_back`.

### Wreck pieces land a few dozen a frame

`ChunkGrace` is a fixed 0.5 s window, so pieces born together come out of it
together: 720 colliders and 720 dynamic bodies inserted on one frame, still
stacked where they were bolted, which the solver then meets as contacts on that
same frame.

`land_carved_chunks` now lands at most `CHUNK_ACTIVATIONS_PER_FRAME = 24` per
frame and leaves the rest for the next. This does not reduce the work, it
spreads it, which is what a frame-time tail cares about. Deferring is the SAFE
direction: a piece that waits stays kinematic and colliderless, which is what
the grace was for. Nothing may land EARLY; landing late costs only that the
piece cannot be flown into yet, and at this rate a 720-piece collapse is fully
physical about half a second after its first piece lands. A single hit is never
delayed at all, which is why `a_chunk_that_has_drifted_clear_becomes_physical`
and the `CHUNK_GRACE_SECS * 0.6` clock it uses are unchanged.

Held down by `a_crowd_of_pieces_lands_a_few_at_a_time`, which also asserts the
contract the deferral must not break: every piece still waiting is
`RigidBody::Kinematic` with no collider.

## Proof

### Instrument, and how to read it

`examples/systems/stress_hull_collapse.rs`, the same binary and the same host
for every arm. Dev build (`cargo run --features debug`), rendering under
Xvfb :99, `NOVA_AUTOPILOT=1`, three runs per arm, `/proc/loadavg` printed
before and after each run and inside 1.2-2.6 throughout.

Read every millisecond here as `docs/performance.md` says to. `present_frames`
alone is a 15-21 ms additive window copy at this resolution, and the fixed loop
amplifies whatever is left (`F = B / (1 - s/T)`). These numbers RANK, and the
before/after ratios stand because both arms used one instrument on one host.
They are not an FPS claim, and the range asserts none of them.

The two items were measured SEPARATELY by ablating the other:
`CHUNK_ACTIVATIONS_PER_FRAME` was set to `usize::MAX` for the shard-budget arm,
which is the previous behaviour exactly, and restored to 24 for the combined
arm. Each arm was built, then the host was allowed to go quiet, then measured.

### The recorded readings, three runs per arm

Median across the three runs, with the range in brackets:

| | worst collapse frame | fixed steps in it | frames in the 8 s window | peak shards | peak entities |
|---|---|---|---|---|---|
| before (`77f963b1`) | 153.4 ms [152.9-163.1] | 10 | 300 [291-308] | 10 666 | 18 866 |
| + shard budget | 70.8 ms [68.7-87.0] | 5 [4-6] | 395 [392-396] | 2 099 | 10 253 |
| + chunk batching | 67.6 ms [60.1-70.8] | 4 [4-4] | 405 [404-409] | 4 231 | 12 382 |

- The shard budget is where the frame cost is: 153.4 -> 70.8 ms, a 54 percent
  cut, and the worst frame stops paying for ten fixed steps.
- Batching the landings takes another 4.5 percent off the worst frame and, more
  usefully, pins the worst frame's step count at 4 in every run where the
  shard-budget arm ran 4, 5 and 6. It is the TAIL it flattens, which is what it
  was for.
- 300 -> 405 frames over the same eight seconds of app time is the whole
  collapse window, not one frame: 26.7 ms to 19.8 ms of average frame time
  across it.
- Peak shards RISING from 2 099 to 4 231 between the two after-arms is the
  budget working as specified rather than a regression: it is a ceiling on what
  one FRAME may create, so a collapse that renders more frames per second gets
  more chips in the air. Per frame it is still 128.
- Peak pieces (720) and peak pending activation (720) are identical in all
  three arms, as they must be: the collapse sheds the same wreckage, and
  batching changes when a piece lands, not whether it does.

### The solver spans, by name

One traced `probe run stress_hull_collapse` per arm, whole-run SELF time out of
the chrome trace. A traced build is slower than the arm above (its worst frame
reads 242.9 / 115.1 / 111.2 ms for the three arms), and the trace's own
overhead is inside every row - but the three runs cover almost exactly the same
number of fixed steps (749 / 743 / 740), so the rows compare directly.

| self ms | before | + shard budget | + chunk batching | calls (before -> both) |
|---|---|---|---|---|
| `solve_contacts<true>` | 712.03 | 680.24 | **418.77** | 4494 -> 4440 |
| `solve_contacts<false>` | 689.73 | 650.25 | **403.78** | 4494 -> 4440 |
| `warm_start` | 378.98 | 363.50 | **200.57** | 4494 -> 4440 |
| `update_narrow_phase` | 249.07 | 227.56 | **156.84** | 749 -> 740 |
| `prepare_contact_constraints` | 165.08 | 142.42 | **106.52** | 749 -> 740 |
| `par_for_each` VelocityIntegrationQuery | 475.98 | **269.14** | 280.63 | 82 410 -> 82 722 |
| `par_for_each` SolverBodyInertia | 427.81 | **249.44** | 300.24 | 82 410 -> 82 722 |
| `par_for_each` SolverBody | 395.73 | **222.69** | 262.85 | 82 410 -> 82 722 |
| `schedule: SubstepSchedule` (total) | 3130.95 | 2631.80 | **2026.11** | 4494 -> 4440 |
| `schedule: PhysicsSchedule` (total) | 4666.92 | 3887.36 | **3169.49** | 749 -> 740 |

The two items land on DIFFERENT spans, each on the one its mechanism predicts:

- The shard budget owns the per-body integration passes, because a shard is a
  kinematic body in exactly those three queries: VelocityIntegrationQuery
  -43 percent, SolverBodyInertia -42, SolverBody -44. It barely moves
  `solve_contacts` (-4 percent), because a shard carries no collider and so is
  never in a contact.
- Batching the landings owns the CONTACT work, because what it spreads is
  colliders appearing inside each other: `solve_contacts` -38 percent from the
  shard arm, `warm_start` -45, `update_narrow_phase` -31,
  `prepare_contact_constraints` -25. The range's own reading agrees - broad
  phase pairs at the worst frame fall from a median 2193 (before) and 2148
  (shards) to 1164.
- The two integration rows RISE slightly between the after-arms (269 -> 281 and
  so on). That is the same effect as the peak shard count rising: the collapse
  renders more frames, so more chips are alive to integrate. Per fixed step the
  whole physics schedule still falls 6.23 -> 5.23 -> 4.28 ms.

### Combined, against the investigation's baseline

`tasks/20260904-155338` left the aftermath at 334.77 ms with twelve fixed steps
in one frame, measured on mainline `first_shift_08_attack_salvo`. That scene is
not this range, so the honest combined figure is the one this instrument took
end to end:

| | before | after both |
|---|---|---|
| worst collapse frame | 153.4 ms | **67.6 ms** (-56 percent) |
| fixed steps in it | 10 | **4** |
| frames over the 8 s collapse window | 300 | **405** (26.7 -> 19.8 ms mean) |
| peak shards | 10 666 | 4 231 |
| peak entities | 18 866 | 12 382 |
| whole-run physics, per fixed step | 6.23 ms | **4.28 ms** (-31 percent) |

Behaviour is unchanged where it must be: 720 corridor cells destroyed, 720
wreck pieces shed, 0 left waiting on a grace, and `probe run` reports
`stress_hull_collapse OK measured 6/8` with every check PASS in all three arms.

### What a collapse looks like now

Both numbers were chosen so that nothing a player does to ONE target changes.
A PDC round carves a crater worth two chips and gets two; a ram or a scripted
mega-hit carves wide and gets the clamped seven; a section that dies alone
sheds one wreck piece and it goes physical on the same frame it always did. The
budgets only bind on an event that opens hundreds of craters at once, which in
the shipped content means a siege lance through a capital hull.

There, the collapse still looks like a collapse: the corridor is destroyed in
full, all 720 cells leave the hull as real wreck pieces with their plates and
greebles still bolted on, and they tumble out and become things you can fly
into. What is different is the dust. The chips no longer come off every crater
in the corridor at once - the frame's 128 go to the craters nearest the entry
face, which is the end the shot came from and the end a camera is looking at,
and the deep end of the bore throws none on that frame. Across the whole
collapse there are about 4 000 chips in the air at peak instead of about
10 000.

The other visible difference is a delay measured in fractions of a second: the
last of 720 wreck pieces becomes solid about half a second after the first,
rather than every one of them at the same instant. Nothing is drawn
differently while it waits - it is already tumbling, already lit, already
wearing its plates - so what the wait costs is only that a ship flying straight
into the wreck field within that window passes through the pieces at the back
of the queue.

### Unsettled

- **The shard budget is a creation rate, not a live population.** 128 per frame
  is a bound on the work one frame does, which is what the frame cost needed;
  it is not a bound on how many chips exist. A faster host renders more frames
  per second and so puts more chips in the air - which is why peak shards ROSE
  from 2 099 to 4 231 when the collapse got cheaper. If a live ceiling is ever
  wanted, it is a different mechanism (a count of live `CarveShardMarker`s,
  or a shorter `SHARD_LIFETIME_SECS`) and a different decision about what a
  collapse should look like.
- **The corridor's chips are front-loaded.** Over the budget the craters that
  arrive late in the frame go unchipped, and the arrival order is the rake's
  own depth order. That reads correctly for a shot fired INTO a hull, and it
  would read differently for an event that opens craters all round a body at
  once. Nothing shipped does that today.
- **Twelve fixed steps in one frame is still the shape of the cost.** The
  collapse frame is amplified by the fixed loop, and both budgets attack the
  population that drives it rather than the loop. The worst frame is down from
  ten steps to four; it is not down to one, and no arrangement of debris
  budgets will get it there.
- **`stress_hull_collapse` is a permanent CI cost.** It stands up 1296 sections
  and runs a collapse to completion; it takes about 25 s of wall clock per
  clean pass on this host.

## Review

Re-measured on the merge commit `5082ca61`, one instrument, one host, both arms
built from the SAME source with only the two constants changed
(`SHARDS_PER_FRAME` and `CHUNK_ACTIVATIONS_PER_FRAME` lifted to 1 000 000 for
the before arm, which is the previous behaviour). `probe run
stress_hull_collapse` runs a clean pass and a traced pass; both are quoted
because the traced pass is the harsher one and it ranks the same.

| clean pass | before | after | after, repeat |
|---|---|---|---|
| worst collapse frame | 147.4 ms | 80.9 ms | 83.1 ms |
| fixed steps in it | 9 | 6 | 5 |
| frames in the collapse window | 325 | 394 | 400 |
| peak shards | 10 317 | 4 169 | 4 189 |
| peak entities | 21 074 | 14 920 | 14 930 |
| worst-frame broad phase pairs | 2 260 | 1 269 | 1 423 |
| contact constraints in it | 594 | 311 | 296 |

| traced pass | before | after |
|---|---|---|
| worst collapse frame | 235.7 ms | 105.2 ms |
| fixed steps in it | 15 | 7 |
| peak shards | 10 439 | 3 945 |
| worst-frame broad phase pairs | 2 465 | 1 329 |

Behaviour is identical in every arm and both passes: 720 corridor cells
destroyed, 720 wreck pieces shed, 0 left waiting on a grace, `OK measured 6/8`
with all six checks PASS and `log_clean 0 offending lines`.

Also re-run on `5082ca61`: `cargo test -p nova_gameplay --lib -- integrity::chunk
integrity::spew` (25 pass), `cargo test -p nova_probe_cli --test catalog_drift`
(2 pass), `cargo fmt --all --check`, `cargo check --workspace --examples
--features debug`.
