# Give the scenario language the primitives its content hand-rolls

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

# Give the scenario language the primitives its content hand-rolls

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
