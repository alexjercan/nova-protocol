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
