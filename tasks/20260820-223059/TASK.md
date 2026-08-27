# Give the scenario language the primitives its content hand-rolls

- STATUS: OPEN
- PRIORITY: 72
- TAGS: v0.12.0,scenario,events

The modding vocabulary is sound and the review says so plainly
(`tasks/20260818-220812/review-modding-model.md`): a stateless
event-condition-action engine over a closed vocabulary, name-routed through
`EventHandlerIndex`, nothing polling. The Wesnoth inheritance took the good half
- closed tag set, declarative filters, data a lint can walk - and left the loops,
the `[fire_event]` recursion and the variable substitution.

**What is missing is not a language. It is three primitives**, and content
hand-rolls all three out of one mechanism.

## The evidence

- `shakedown_run` spends **21 lines of RON on one line of dialogue**, and a
  five-beat conversation becomes five sibling `OnUpdate` handlers coordinated by
  an `open_step` counter the author increments by hand.
- **19 of its 42 handlers are `OnUpdate`.** `ledger_ch3` is 15 of 27, `lifeline`
  13 of 27.
- `scenario_elapsed` appears 27 times in `shakedown_run` alone: time-based
  one-shots, re-evaluated every frame forever.
- Of 16 expression grammar nodes, authored content uses SEVEN. In 17,759 lines
  there is no multiply, no divide, no parens, no string or boolean literal.
  **A bigger language is not what anybody reached for.**

`OnUpdate` is doing three unrelated jobs: a timer, a change notification, and a
sequencer. Each is spelled "poll every frame and evaluate a filter" because that
is the only wake mechanism content can name.

The principle: **a handler should be woken by the thing that can change its
answer.** The engine already does this for its 16 domain events; `OnUpdate` is
the escape hatch that bypasses it.

## NOT a performance task

Measured (D19): the whole scenario engine is **150.6 us of a 12.24 ms fight
frame, 1.2%**, and the interpreter itself is **2.7 us, 0.02%**. A scenario needs
roughly 1,000 `OnUpdate` handlers before dispatch costs 1 ms.

So do not justify any of this with frame rate, and do not accept a design
because it is faster. **The payoff is authoring cost, correctness and
lintability.** Any frame-time gain is a rounding error collected on the way past.

## Stage 1 - one-shot handlers

There is **no `once` concept in the engine at all**. A handler that has fired and
can never fire again is walked every frame for the rest of the scenario. Wesnoth
has `first_time_only=yes`; it is the one part that should have been borrowed.

Ship this ALONE and first. It needs no new vocabulary, it is a correctness
improvement as well as a cost one, and it is independently useful whatever
happens to stages 2 and 3.

Known design cost: `EventHandlerIndex`'s snapshot is valid precisely because "a
handler is built, spawned once, and never changes its event"
(`crates/nova_events/src/engine.rs`). Retirement mutates that. Revisit the
invariant rather than assuming it survives.

## Stage 2 - `Sequence`

One action holding an ordered list of steps, with the ENGINE holding the cursor
instead of content holding it in a variable:

```
actions: [
    Sequence([
        (after: 2.0, StoryMessage(speaker: "Halloran", text: "...")),
        (after: 6.0, StoryMessage(speaker: "Halloran", text: "...")),
        (after: 4.0, ObjectiveComplete("ease_out")),
    ]),
]
```

One handler. No counter variable, no per-beat filters, no `OnUpdate` for the
beats. Still closed vocabulary, so `content lint` still walks it.

This is the linear case and the linear case is the common one. Expected to
delete ~19 handlers from `shakedown_run`, ~15 from `ledger_ch3`, ~13 from
`lifeline`.

### It is the autopilot chain, for a different consumer

`nova_debug::harness::AutopilotPlugin` ALREADY IS this shape, and every
`examples/systems/` range drives on it:

```rust
.step("open the tubes")
    .on_enter(open_the_tubes)
    .until(the_envelope_is_full())
    .deadline(FILL_DEADLINE_SECS)
    .add()
```

Step, entry actions, gate, deadline. So this is not a novel design - it is an
existing proven one that Rust examples can reach and scenario authors cannot.
Copy it rather than reinventing it.

**`deadline` belongs in v1, and that is the part the scenario version would
otherwise miss.** The autopilot carries one because a step that never completes
has to be a LOUD failure rather than a quiet pose. A scenario step waiting on an
`until:` that never fires would soft-lock the scenario silently, which is the
worst thing a mod author can ship - and a mod server cannot lint for it, because
whether a gate can ever open is a runtime question.

### The two gate kinds, from real content

Taken from `shakedown_run.content.ron:882-1046`, which hand-rolls all three
primitives at once - `open_step` as cursor, `opened == 0` as one-shot,
`beat_gate = scenario_elapsed + 4` as timer, across three handlers and five
variables, none of which are about the game:

- `after: <seconds>` - a delay from when the step became current. Replaces the
  `beat_gate` write plus the `GreaterThan(scenario_elapsed, beat_gate)` that
  reads it.
- `until: <event + filters>` - the real condition, unchanged, moved inside the
  step. In that passage it is `OnEnter` with an `Entity` filter on
  `beacon_1`/`player_spaceship`.

Both may sit on one step. **The ordering guard disappears entirely**, because the
cursor IS the "we have done the previous one" guarantee - that is the whole win,
and it is what deletes the five bookkeeping variables.

**Decide WAIT versus SKIP explicitly, and ship wait only.** A step whose
condition is false should block, not be skipped. Skipping is branching wearing a
smaller hat and belongs in stage 3, not smuggled into sequence semantics.

### Two consequences worth knowing before starting

**One-shot and `Sequence` are the same machinery.** A sequence is a chain of
one-shot handlers where completing one registers the next. Stage 1 is not merely
the cheapest thing first - it is the foundation stage 2 is built on.

**The cursor is a small integer, so it SERIALISES.** If mid-scenario saves ever
arrive, a `Sequence` survives one. A Lua coroutine - the construct that made the
Lua comparison look attractive in the review - does not. Same expressiveness for
this use case, and only one of them can be saved.

### The sequence is the SPINE, not the scenario

Anything genuinely out-of-band stays an ordinary handler beside it: the scavenger
jumping in, a hull taking damage, a player death. Those are not beats and must
not be forced into the list.

That is also the honest test for stage 3. After this lands, look at what is LEFT
outside the sequences. A handful of real reactions means done. Parallel sequences
that have to talk to each other is when named triggers earn themselves.

## Stage 3 - a DECISION POINT, not a promised feature

Chains, branching, tags, custom triggers.

**Do not design this before stages 1 and 2 have landed and their content has
been rewritten onto them.** The reason is concrete: most apparent branching in
this content is "linear with a guard", and until the linear case is cheap there
is no way to tell how much real branching exists. Sizing it now means sizing it
against ceremony rather than against need.

When the decision is taken, the shape is already half-built: `EventHandlerIndex`
buckets handlers by event NAME, so a content-declared custom event is a
`FireEvent(name)` action plus handlers on that name. Architecturally close to
free.

What makes it dangerous is also known:

- A named trigger is a general-purpose goto. File order stops matching execution
  order, which is the thing that makes WML content hard to follow.
- Reachability becomes a graph problem, and `lint/scenario.rs` is 1,452 lines of
  whole-program analysis that currently answers it by walking a tree.
- `[fire_event]` recursion is the exact Wesnoth failure the review credits this
  engine with avoiding.

Tags fan one trigger out to several handlers without naming each, which is
genuinely useful and makes reachability harder still. Both sides are real; that
is why it is a decision and not a bullet.

## The constraint that must survive all three stages

**Every id stays a LITERAL.** The moment a handler name, scenario id or object
id can be computed - `"gate_" .. i` - `content lint` stops being decidable and
`/create/reference.md` stops being exhaustive. That single property is what makes
the closed vocabulary worth more than a script language here, and it is the line
between extending this engine and turning it into WML.

Non-goals, explicitly: recursion, computed names, loops in authored content, and
a bigger expression grammar. The grammar is already larger than anything uses.

## Related, and deliberately separate

Two WAKE SOURCES would retire most of the remaining `OnUpdate` once stages 1-2
are in: a scheduled time wake (one priority-queue entry, zero per-frame cost),
and a variable-change wake (the engine already owns every write via
`ScenarioWorld::set_variable`, and can index handlers by variable name exactly as
`EventHandlerIndex` indexes by event name).

Naming warning: **"watch" is already taken** and means the opposite - an
engine-owned read-only value content is forbidden to write
(`crates/nova_scenario/src/world.rs`). Do not reuse the word.

The lint rule that stops regression: once better spellings exist, flag any
`OnUpdate` whose filters read only time or only variables. Without it the
ceremony comes back.

## Definition of done

- Stage 1 and stage 2 landed as separate commits, each with base content
  rewritten onto them so the win is demonstrated rather than asserted.
- Handler counts before and after for `shakedown_run`, `ledger_ch3`, `lifeline`.
- `content lint` still whole-program: no id in authored content is computed.
- `/create/` updated - it is the exhaustive authored contract and a new construct
  MUST land there (`docs/keeping-docs-in-sync.md`).
- Stage 3 written up as a recommendation with the content evidence stages 1-2
  produce, and NOT implemented as part of this task.

## Stage 1 landed - `once`

`ScenarioEventConfig` grew a serde-defaulted `once: bool`. A handler that says
`once: true` retires the first time its filters PASS - not the first time its
event fires, which is the distinction that keeps a beat waiting on a condition
alive through every event that refuses it.

### Decisions

- **A pass-local spent set, not a latch in `W`.** The task body said to latch
  it in the event world. `maintain_handler_index` is ungated, runs every frame
  `.before(queue_system)`, and drains `RemovedComponents`, so the ONLY window
  a despawn cannot cover is within one drain pass. `queue_system` holds a
  `HashSet<Entity>` for that pass. No `EventWorld` trait change, no new
  resource, and no borrow conflict with `Res<EventHandlerIndex<W>>`.
- **`once` means spent by DOING ITS JOB.** A refused dispatch leaves the
  handler live. The other reading - spent by being offered the event - would
  make a scenario silently lose the beat it is waiting for.
- **`ScenarioEventConfig::build_handler` is now the ONE place a config becomes
  a handler.** Nine headless rigs each re-implemented the loader's
  registration loop, and every one of them dropped `once` on the floor - the
  shakedown walk rig failed loudly and the rest would have passed while
  testing a scenario the game does not run. A field added to the config now
  reaches the loader and the rigs together or not at all.
- **A latch variable dies only when its ONLY reader was its own filter.** A
  flag another handler reads is a signal about the game and stays: `taunt_said`
  gates the cast-off, `pinch_warn_said` gates the far-side confirm, `w2_up`
  and `w3_up` say a wave is on the board, `arena_done` stops the timed beats
  nagging a finished player. Every deletion was audited by reference count.
- **Cycles keep their handlers.** The Ledger's overspeed ladder walks
  `speed_warned` 2 -> 3 -> 2 as often as the player rides it, and Shakedown's
  orbit trio must re-arm on every lost hold. Marking either `once` would
  strand the ladder; both carry a comment saying why.

### What it does and does not delete

`once` deletes latch variables and the filters that read them. It does NOT
delete handlers - that is stage 2's win, and the counts say so plainly.

| scenario | lines | handlers | filters | variables | `once` |
| --- | --- | --- | --- | --- | --- |
| `shakedown_run` | 2246 -> 2202 | 42 -> 42 | 98 -> 85 | 9 -> 6 | 36 |
| `broadside` | 844 -> 832 | 15 -> 15 | 36 -> 34 | 8 -> 6 | 12 |
| `broadside_gunship` | 693 -> 689 | 11 -> 11 | 26 -> 25 | 4 -> 3 | 8 |
| `final_tally` | 965 -> 905 | 17 -> 17 | 46 -> 39 | 16 -> 10 | 16 |
| `lifeline` | 1523 -> 1478 | 27 -> 27 | 73 -> 66 | 19 -> 14 | 23 |
| `ledger_ch3` | 1838 -> 1814 | 27 -> 27 | 82 -> 77 | 15 -> 11 | 24 |
| `example_arena` | 784 -> 768 | 8 -> 8 | 11 -> 8 | 4 -> 2 | 6 |

`final_tally` lost six of its sixteen variables; `example_arena`, the mod
`/create/` teaches from, lost half of its four and three of its eleven filters.
Both producer kinds are covered: the base scenarios are generated from the Rust
builders (`content -- gen`), `ledger_ch3` is a direct RON edit.

The webmod chapters other than `ledger_ch3` (ch2, ch2b, ch4, ch5) were left
alone. The field is defaulted, so they parse and run unchanged.

### Proof

- `cargo test -p nova_events --lib` - 9 pass, including three new ones: a
  `once` handler fires once and leaves the index, a refused event leaves it
  live, and two events of the SAME name in one drain pass reach it once.
  Mutation: deleting the `spent.contains` guard takes only the third down.
- `cargo test -p nova_authoring` (77 + 24 across its integration tests),
  `-p nova_assets` (16 suites), `-p nova_scenario --lib` (202).
- `nova-protocol content lint` - 0 errors, 0 warnings, 0 findings, 13
  scenarios balance-audited.
- LIVE: `examples/systems/system_scenario_grammar` under Xvfb. Its beat
  transitions carry `once` instead of latch filters, and it now holds an
  UNFILTERED `once` OnUpdate handler counting its own runs. Green:
  `once_ticks = 1` after three rounds. Mutation: `once: false` there reads 71
  and error-exits 101.
- Skipped: the workspace test suite and Clippy, per the standing instruction.

### For stage 2

What is LEFT is the evidence stage 3 asks for. Handler counts did not move,
because the remaining `OnUpdate` handlers are SEQUENCERS - `open_step` in
Shakedown and the Ledger, `gate` in the Ledger, the beat counter, the wave
schedule in Lifeline. Every one is a cursor a `Sequence` would hold.

## Stage 2 landed - `Sequence`

One action holds an ordered list of steps and the ENGINE holds the cursor.
`SequenceActionConfig` is an authored literal `key` plus `steps`; running it
files a `SequenceRun` in `NovaEventWorld` beside the keyed timers, and
`advance_scenario_sequences` walks it in `Update`, chained after
`sample_scenario_queries` and before `tick_scenario_timers` / `fire_on_update`.

A step waits on `after:` (scenario seconds), on `until:` (an event plus its
filters), or on both. WAIT, never SKIP: the beats behind a shut gate stay
behind it. That makes a stuck gate a soft-lock, so a gated step carries a
`deadline:` - expiry stops the run and logs an `error!` naming the key, the
step and the event it waited for. `content lint` refuses a gated step without
one.

### Decisions

- **WAIT, and both waits together.** A step with `after` and `until` owes
  both: an early gate still owes the delay, an elapsed delay still owes the
  gate. Skip was rejected outright - a skipped beat is a scenario silently
  losing a line, which is the failure `once` was designed away from in stage 1.
- **The gate is a real handler, not a poll.** `sequence_gate_handlers` spawns
  one extra `EventHandler` per gated step, carrying a private
  `SequenceGateAction { key, step }` that is inert unless the cursor stands on
  exactly that step. The gate OPENS the run; the driver runs the beat one frame
  later. Polling the gate from the driver would have needed the driver to
  re-implement filter dispatch.
- **One clock jump delivers one step.** `take_ready_sequence_step` stamps
  `since = now` on the step it hands back, so a rig that jumps the clock 60s
  does not fire a six-beat chain in one frame. Steps with no delay still
  collapse into a single pass, because the driver loops.
- **A shared key across handlers is the IDIOM, not a smell.** Every win variant
  of a scenario starts one outro chain, and only one of them can ever fire.
  Nothing static tells that apart from a real collision, so the runtime holds
  that half - `start_sequence` refuses a live key and logs it - and the lint
  flags duplicates only WITHIN one handler's action list, where the two starts
  definitely race.
- **`action_groups`, not `actions`, is what "one frame" now means.** A step's
  action list is a frame of its own, landing seconds after the handler that
  queued it. `ScenarioEventConfig::action_groups()` returns the handler's own
  actions plus one group per `Sequence` step it starts, however deeply nested;
  every pacing and ordering pin reads it. `EventActionConfig::walk` is the
  recursion the four walkers the round-4 audit named now share.
- **The regression rule shipped with the construct.** `lint/scenario.rs` warns
  on an `OnUpdate` handler whose filters read NOTHING but the scenario clock -
  a hand-rolled delay walked every frame. A value-gated milestone (a tally, a
  distance) stays silent, because that is what `OnUpdate` is legitimately for.

### What it deletes, and what it does not

`Sequence` deletes the SEQUENCER handlers - the ones whose only guard was "not
dead / still this beat" - and the step counters that ordered them. It does not
delete a handler that must still ASK something when the beat lands; those were
re-expressed as `OnTimerEnd` on a keyed timer instead of clock arithmetic.

Counted with one script over the shipped RON (`steps` counts sequence steps,
so the chains are visible as well as the handlers they replaced):

| scenario | lines | handlers | filters | variables | chains | steps |
| --- | --- | --- | --- | --- | --- | --- |
| `shakedown_run` | 2202 -> 1991 | 42 -> 25 | 85 -> 51 | 6 -> 3 | 12 | 19 |
| `lifeline` | 1478 -> 1513 | 27 -> 23 | 68 -> 59 | 14 -> 13 | 5 | 10 |
| `ledger_ch3` | 1814 -> 1672 | 27 -> 20 | 77 -> 55 | 11 -> 6 | 3 | 7 |
| `final_tally` | 905 -> 810 | 17 -> 11 | 41 -> 25 | 10 -> 4 | 5 | 8 |
| `broadside` | 832 -> 803 | 15 -> 11 | 34 -> 24 | 6 -> 4 | 4 | 6 |
| `broadside_gunship` | 689 -> 735 | 11 -> 8 | 25 -> 18 | 3 -> 2 | 5 | 9 |

The three the definition of done names: `shakedown_run` **42 -> 25**,
`ledger_ch3` **27 -> 20**, `lifeline` **27 -> 23**. Both producer kinds are
covered again - the base scenarios come from the Rust builders via
`content -- gen`, `ledger_ch3` is a direct RON edit.

Two rows deserve their honesty. `broadside_gunship` GREW 46 lines: a chain
spells its pacing out where the old handlers hid it in a filter, and eight
handlers reading 25 filters is still a worse thing to author than five chains.
`lifeline` moved least, 27 -> 23, and that is the stage-3 evidence rather than
a shortfall - see below.

### Proof

- `cargo test -p nova_scenario --lib` - 221 pass, including 11 new ones in
  `actions/sequence.rs` (both waits owed, deadline stops the run, a live key
  refuses a restart, a gate on the wrong step is inert) and the live-pulse test
  `a_sequence_walks_its_steps_on_the_live_pulse` in `loader/clock.rs`.
- `cargo test -p nova_authoring --lib` (77) and `--test broadside_assault`
  (14); the `nova_assets` rigs - `final_tally_claim` (7), `lifeline_convoy`
  (8), `neutralized_ships` (4), `scenario_act_machine` (7),
  `scenario_branch_choice` (9), `scenario_gate_course` (6),
  `scenario_provocation` (7).
- `cargo check --workspace --all-targets` clean; `cargo fmt --all` run.
- `nova-protocol content lint` - 0 errors, 0 warnings, 0 findings, 13
  scenarios balance-audited, 1 acked. No id in authored content is computed:
  a sequence key is an authored literal, exactly like a timer key.
- LIVE, `--norender --mute --scenario shakedown_run` for 45s: `loaded scenario
  'shakedown_run' with 25 handler(s) and 23 object(s)`, and the six-step
  opening chain walked through to `ObjectiveMarkerAttach: 'beacon_1' <-
  'BEACON 1'` at ~15s wall, no errors.
- LIVE, `--scenario-file webmods/the-ledger/ledger_ch3.content.ron`:
  `20 handler(s) and 13 object(s)`. (The `dep://` asset-source error is
  expected for a loose file outside a mod install.)
- Skipped: the workspace test suite and Clippy, per the standing instruction.

### Docs

`/create/actions.md` (26 actions now; new `## Pacing` group with the full
`Sequence` contract), `/create/events.md` (an event name also names an
`until` gate), `/create/reference.md` (family table and A-to-Z),
`/create/scenarios.md` (the chains-of-beats pointer beside `once`),
`docs/scenario-system.md` (the engine half: the cursor, the gate handlers, the
`Update` chain position, `action_groups`), `docs/guide-extend-scenarios.md` and
`docs/concept-index.md` (the new submodule), `CHANGELOG.md`.

## Stage 3 - the recommendation

**Do not build chains, branching, tags or custom triggers. Build the two wake
sources instead.** Stages 1 and 2 were the measurement the decision point was
waiting for, and the content came back with an unambiguous answer.

The evidence. After stage 2 the whole mainline holds **22 `OnUpdate` handlers**,
and not one of them polls the clock alone:

| scenario | `OnUpdate` | what their filters read |
| --- | --- | --- |
| `lifeline` | 11 | `act` plus per-wave kill flags (`r1a_down`, `w2_up`, `queen_down`, ...) |
| `ledger_ch3` | 5 | `act`, `spotted`, `speed_warned`, the watched `player_speed` |
| `shakedown_run` | 3 | `beat`, `crates_recovered` |
| `broadside` | 2 | `act` plus corvette and hauler kill flags |
| `final_tally` | 1 | `act`, `picket_a_down`, `picket_b_down` |
| `broadside_gunship` | 0 | - |

Every remaining one is a **variable-change** question. The new lint rule fires
on none of them, which is the point: the ceremony `Sequence` was built to
delete is gone, and what is left is handlers waiting on a value another handler
writes. A variable-change wake retires all 22 - the engine already owns every
write through `NovaEventWorld::insert_variable`, and handlers index by variable
name exactly as `EventHandlerIndex` indexes by event name. A scheduled time
wake takes the timers and the `after:` cursor off the frame with one
priority-queue entry. Neither adds a construct to the authored vocabulary, so
`/create/` does not grow and `content lint` stays whole-program.

Against that, the branching case the content actually makes is small.
`lifeline`'s eleven handlers look like the strongest argument for a richer
control-flow construct, and they are the strongest argument against one: they
are not a linear chain wearing a guard, they are a genuine wave schedule where
each beat asks whether the previous wave died. A chain construct would have to
grow a per-step condition and a way out of the chain - which is a branching
language, arriving to serve one scenario. The `Sequence` we shipped covers the
linear case at one action per chain, and the remaining handlers are the honest
non-linear remainder.

The dangers named in the body all still hold, and stage 2 sharpened one of
them. `Sequence` already spends the "file order stops matching execution order"
budget: a chain's beats run seconds apart from the handler that lists them, and
that is exactly why `action_groups` had to exist and why four walkers had to
learn to recurse. A named trigger spends the same budget again, on a graph
rather than a tree, against a `lint/scenario.rs` that answers reachability by
walking. Paying that for 22 handlers that a wake source retires outright is the
wrong trade.

Recommendation, in order: (1) variable-change wake, (2) scheduled time wake,
(3) revisit custom triggers only if content appears that a wake source cannot
express. Note the naming warning from the body - "watch" is taken and means the
opposite.

## Verified against the tree, round 4 (2026-08-24)

Scheduled into v0.12.0. Full audit:
`tasks/20260815-231945/SCENARIO-PIPELINE.md` section 3. The deltas that
change the design, none that change the direction:

- **The cursor CANNOT live in the `Sequence` action.** `EventAction::action`
  takes `&self`, actions are Arc-shared, and `EventHandlerIndex` stores
  CLONES of handlers (engine.rs:362-369) - a cursor inside the action
  diverges between the ECS copy and the index copy. The cursor lives in
  `NovaEventWorld`, keyed by an authored LITERAL sequence key, exactly as
  timers do (world.rs:504, loader/clock.rs:66-71). The key is lintable and
  serialises - the mid-scenario-save argument survives.
- **`once` needs a same-pass latch, not only despawn.** Retirement-by-despawn
  already works and is tested (engine.rs:643), but `queue_system`
  (engine.rs:429-462) drains the whole queue against one index snapshot and
  a commanded despawn lands next frame - two queued events of the same name
  would fire a `once` handler twice. Latch it in `W`, which `queue_system`
  holds as `ResMut`.
- **`once` is a serde-defaulted field** on `ScenarioEventConfig`
  (loader/mod.rs:308-323); old files parse unchanged.
- **The hidden half of `Sequence`: four walkers recurse.** Everything that
  walks `event.actions` walks it top-level only today and must recurse into
  steps: `inline_queries` (loader/mod.rs:259 - missing it silently disables
  the entity sampler for nested expressions), `object_count` (:352 -
  harness assertions read it), lint `collect_declared`/`check_action`
  (lint/scenario.rs:318, :346), and the per-event spawn-id pass (:120).
- **Corrections to this body**: `ledger_ch3` is a HAND-WRITTEN webmod
  (webmods/the-ledger/ledger_ch3.content.ron), not generated base - its
  rewrite is a direct RON edit, and the evidence now spans both producer
  kinds. `lint/scenario.rs` is 1391 lines today, not 1,452. Handler counts
  confirmed: shakedown_run 19/42, ledger_ch3 15/27, lifeline 13/27.
- Keep the `until:`/`deadline:` vocabulary consciously parallel with the
  autopilot predicates that `20260824-011329` extends - same idea, two
  consumers.
