# Design review: the scenario / modding model

- DATE: 2026-08-20
- STATUS: REVIEW (report only, no code changed)
- SCOPE: `nova_scenario`, `nova_events`, `nova_mod_format`, `nova_modding`,
  `nova_authoring`, the authored content under `assets/base/`, `assets/mods/`
  and `webmods/`, and the `/create/` contract.

Answers the four owner questions:

1. Is it the right model for this game?
2. Would I do it differently (Lua, guardrailed RON, or extend to Lua)?
3. Is nicer-to-write, easier-to-lint code worth a script language?
4. Wesnoth is turn based - does its lesson still hold in real time?

Short answers, defended below:

1. **Yes.** It is a stateless event-condition-action rule engine over a closed
   vocabulary, and that is the correct model for a game whose scenarios are
   mission scripts rather than simulation rules.
2. **No Lua at runtime.** Ship a Lua (or Rust) AUTHORING front-end that emits
   the same RON. The middle option is already half-built and shipping:
   `nova_authoring`.
3. The pain is real, but it is SYNTAX, not expressiveness. The evidence is that
   the expression language is over-built, not under-built - see E.
4. The Wesnoth lineage is not the problem. The one place real time actually
   bites is `OnUpdate`, and the engine already pays for it deliberately.

---

## A. What the model actually is

### One sentence

A **stateless, forward-chaining event-condition-action (ECA) rule engine**, with
a **closed vocabulary of 16 events, 4 filters, 25 actions and 6 object kinds**,
whose only mutable state is a flat `HashMap<String, VariableLiteral>` living
outside the ECS, dispatched from a **queue drained once per frame in
`PostUpdate`**.

It is not a behaviour tree (no tick-resumable node state, no
running/success/failure), and not a state machine (the engine has no state
concept at all). State machines exist in shipped content, but they are HAND
ROLLED out of one numeric variable plus equality filters - `docs/scenario-system.md`
names this the "gate-counter ordering pattern", and it is the only control-flow
idiom the vocabulary supports.

### What triggers evaluation

Nothing polls. Producers call `Commands::fire::<E>(info)`
(`crates/nova_events/src/engine.rs:251`), which triggers a `GameEvent` observer
(`engine.rs:334`) that pushes onto `GameEventQueue<W>`. The queue drains in
`PostUpdate` under a three-way run condition (`engine.rs:293-315`):

```rust
.run_if(
    not(is_queue_empty::<W>)
        .or_else(resource_changed::<W>)
        .or_else(is_settling::<W>),
)
```

So on a frame with no fired event and an unchanged event world, **zero rule
work happens**. That is the important architectural fact and it is the answer to
half of question 4.

The dispatcher itself (`engine.rs:429`, private `queue_system`) is a flat loop:
pop event, look its name up in `EventHandlerIndex` (`engine.rs:371` - a
`HashMap<&'static str, Vec<(Entity, EventHandler<W>)>>` of cloned handler
snapshots, refreshed by `maintain_handler_index` at `engine.rs:405`), run every
handler's filters, run its actions in order. No priority, no rule ordering
semantics beyond spawn order, no re-entry, no conflict resolution. Handler order
within one event is explicitly NOT load-bearing (`docs/scenario-system.md`), and
shipped content honours that by keying on variables instead.

One deliberate non-uniformity: the pass BREAKS when `world.is_settling()`
(`engine.rs:457`). A handler that queued spawns has made the world incomplete,
so everything behind it waits. That is a real, hard-won piece of design - it is
what makes "the world is not yet live" true instead of "the world is briefly
inconsistent".

### What an "event" is

A `&'static str` name plus a payload that has been **erased into
`serde_json::Value`** (`engine.rs:172`, `GameEventInfo`). Producers hold typed
structs (`OnDestroyedEventInfo`, `OnEnterEventInfo`, ... in
`crates/nova_events/src/lib.rs`), and `GameEventInfo::from_data`
(`engine.rs:184`) serialises them at fire time. Filters read them back by string
key against the constants in `nova_events/src/lib.rs:66-74`
(`ENTITY_ID_COMPONENT_NAME = "id"` and friends).

This is the one genuinely Wesnoth-shaped artifact in the design: a typed struct
goes through an untyped attribute bag and comes out the other side compared as
strings. It is not free (see B), and it is the thing a serialisation failure
silently converts into "this handler never matches again" - which the code knows
and logs loudly at `engine.rs:187`.

The 16 event kinds are `EventConfig` (`crates/nova_scenario/src/events.rs:21`),
lowered to concrete `EventHandler<NovaEventWorld>` by one `From` impl
(`events.rs:60`). Three sources fire them: engine lifecycle (`loader/lifecycle.rs`,
`loader/clock.rs`, `loader/trackers.rs`), scenario objects (`objects/area.rs`,
`objects/asteroid_carve.rs`), and gameplay (`nova_gameplay`'s integrity stack for
`OnNeutralized`).

### What a "filter" is

`fn filter(&self, world: &W, info: &GameEventInfo) -> bool`
(`engine.rs:79`). Read-only, no ECS access, ALL must pass
(`engine.rs:165`). Four kinds (`crates/nova_scenario/src/filters.rs:27`):

| kind | what it reads | file:line |
| --- | --- | --- |
| `Entity` | four optional string fields out of the JSON payload | `filters.rs:81` |
| `Timer` | the `key` field of an `OnTimerEnd` payload | `filters.rs:154` |
| `Conditional` | `Not` / `And` / `Or` over other filters | `filters.rs:193` |
| `Expression` | a `VariableConditionNode` over the variable map | `filters.rs:213` |

`Expression` **fails closed** on an undefined variable or an unavailable query
(`filters.rs:227`), and the comment there is one of the best in the repo: it
records that a healthy `shakedown_run` boot raised nineteen of these, which is
how a log teaches a reader to ignore the word ERROR.

### What an "action" is

`fn action(&self, world: &mut W, info: &GameEventInfo)` (`engine.rs:68`). 25
kinds (`crates/nova_scenario/src/actions/mod.rs:45`), fanned into six submodules.
Actions **never touch the Bevy `World`**. They mutate `NovaEventWorld`
(`crates/nova_scenario/src/world.rs:66`) or push a boxed closure onto
`queued_commands`. `state_to_world_system` drains that queue under a 3 ms
wall-clock budget (`world.rs:45`, `SPAWN_DRAIN_BUDGET`) so a big scene arrives
over several frames instead of in one 300 ms stall.

This staging seam is the single best structural decision in the design. It is
why the rule layer has no ECS borrow problems, why it is trivially testable
headless (every test in `filters.rs`, `clock.rs` runs on `MinimalPlugins`), and
why - importantly for question 2 - a script VM could be dropped in behind the
same trait without touching anything else.

### What the expression language can express

`crates/nova_scenario/src/variables.rs`. A three-level precedence chain over
three literal types:

- `VariableLiteral` = `String | Number(f64) | Boolean` (`variables.rs:38`)
- `VariableFactorNode` = `Parens | Literal | Name | Query` (`variables.rs:51`)
- `VariableTermNode` = `Multiply | Divide | Factor` (`variables.rs:104`)
- `VariableExpressionNode` = `Add | Subtract | Term` (`variables.rs:173`)
- `VariableConditionNode` = `LessThan | GreaterThan | Equal` (`variables.rs:248`)

`Add` is overloaded: numeric sum, boolean OR, string concat. `Multiply` is
numeric product or boolean AND. `Equal` compares within `EQUAL_EPSILON = 1e-6`
(`variables.rs:242`), a fix for exact float equality that essentially never
fired.

What it CANNOT express: no functions, no locals, no loops, no collections, no
string manipulation beyond concat, no negation of a condition (only of a
filter), no `>=` or `<=`, no `!=`, no reading an event payload field into an
expression. Every one of those absences is visible in shipped content as a
workaround.

Read-only world state arrives through `QueryConfig`
(`crates/nova_scenario/src/queries.rs:14`): exactly two queries today,
`Scenario(Elapsed)` and `Entity{id}.Speed`. A `WatchConfig`
(`queries.rs:66`) samples one per live update into a reserved variable name.

### Where RON is parsed vs interpreted

**Parsed once, at asset load.** `ContentAssetLoader`
(`crates/nova_modding/src/lib.rs`, `ron::de::from_bytes` at `:194`, `:234`,
`:340`) decodes `*.content.ron` into `Vec<Content>`; `nova_assets/src/merge.rs`
merges them into `GameScenarios`. `LoadScenario` then spawns one entity per
handler carrying already-built `Arc<dyn EventFilter>` / `Arc<dyn EventAction>`
trait objects.

**Interpreted every dispatch.** Filters and actions are enum-dispatched match
arms, and `Expression` walks a `Box`-recursive AST cloning a `VariableLiteral`
at every leaf. There is no compilation step, no constant folding, no interning.
The bench (`crates/nova_scenario/benches/scenario_dispatch.rs`) exists precisely
to watch that.

Recursion depth is bounded not by the grammar but by `ron`'s default
`recursion_limit` of 128, which `variables.rs:331` pins with a test and a good
explanation of why bounding the grammar itself would be dead machinery.

---

## B. Is it the right model for a REAL-TIME game?

**Yes, and the Wesnoth lineage is not what is costing anything.**

Take the worry seriously and state it precisely. In a turn-based game a rule
engine re-evaluates on discrete turns, so re-evaluating "all rules" is bounded
and cheap. In real time there is no turn, so a naive port re-evaluates every
rule every frame and pays continuously for a discrete abstraction.

This design does **not** do that. Three specific defences, in the code:

**1. Dispatch is event-driven, not polled.** `engine.rs:293-315`. A quiet frame
runs `maintain_handler_index` (cheap, and ungated on purpose so
`RemovedComponents` does not overflow its double buffer) and nothing else. There
is no "evaluate all rules" step.

**2. Routing is by event name, not a linear scan.** `EventHandlerIndex`
(`engine.rs:371`). The doc comment records the design history honestly: the
dispatcher used to be O(all handlers) per event; an entity-id index was tried
and lost most of the win to random-access component lookups; the shipped answer
snapshots cheap handler clones grouped by name. That is exactly the right shape
and it was reached by measurement.

**3. Simulation cost is not in the rule layer at all.** The expensive things in
this game are asteroid carving (async compute pool, `objects/asteroid_carve.rs`),
physics, and section integrity. Those do not go through the rule engine. The rule
engine reacts to their results.

### Where the mismatch actually bites: `OnUpdate`

There is exactly one place the discrete abstraction is billed continuously, and
it is `EventConfig::OnUpdate`. `fire_on_update`
(`crates/nova_scenario/src/loader/clock.rs:90`) fires an `OnUpdateEvent` **every
frame** while live, unpaused and settled (`clock.rs:69-85`). Every `OnUpdate`
handler then re-runs every one of its filters.

Measured against shipped content:

| scenario | `OnUpdate` handlers | total handlers |
| --- | --- | --- |
| `shakedown_run` | 19 | 42 |
| `ledger_ch3` | 15 | 27 |
| `lifeline` | 13 | 27 |
| `final_tally` | 8 | 17 |
| `broadside` | 4 | 15 |
| `gauntlet` | 0 | 14 |
| every menu backdrop | 0 | 2 to 8 |

`shakedown_run`'s typical `OnUpdate` handler carries two `Expression` filters,
so the worst shipped scenario walks roughly 40 expression trees per frame, each
one a `HashMap<String, _>` lookup plus a `VariableLiteral` clone. Plus one
`serde_json::to_value` per frame for the `OnUpdate` payload, which is a unit
struct that serialises to `null`.

That is small. It is also **pure overhead that a turn-based engine would never
pay**, and it is the honest answer to the owner's question: yes, there is a
continuous cost for a discrete abstraction, it is confined to `OnUpdate`, and it
scales with the number of time-gated or value-gated beats a scenario has - which
is exactly the thing a story-heavy chapter has most of.

Two second-order costs ride along with it:

- `resource_changed::<W>` is in the dispatch run condition, and `OnUpdate`
  handlers mutate the world most frames, so the whole `PostUpdate` chain
  effectively runs every frame during a scripted scenario. `world.rs` already
  carries scar tissue from this: `state_to_world_system` had to become
  write-on-diff for `GameObjectives` because "this system now runs every frame
  (the `OnUpdate` pulse keeps the event queue warm)", and an unconditional write
  made the objectives panel despawn and respawn its text lines every frame for
  the whole session.
- The `serde_json` payload erasure means `EntityFilterConfig::filter`
  (`filters.rs:81`) does up to four map lookups and four string comparisons per
  handler per event. The bench isolates this as `filter_entity`. For `OnEnter`
  storms (a scatter field of salvage crates) this is the hot path, not
  `OnUpdate`.

### Is the design paying for its lineage?

No. Grep the vocabulary against Wesnoth's WML and the borrowed parts are the
GOOD parts: a closed tag set, filters as declarative subtags, one-shot events,
and content shipped as data a lint can walk. The parts of WML that are bad in
real time - `[while]` loops, `[fire_event]` recursion, variable substitution
into arbitrary attributes, `[if]/[then]/[else]` nesting - are all ABSENT here.

The one place turn-based thinking leaks through is that the engine offers no
first-class "while condition holds" or "after N seconds" construct, so content
rebuilds both out of `OnUpdate` plus a counter. Wesnoth does not need those
because a turn IS the tick. Here that omission is what turns a five-line
conversation into five `OnUpdate` handlers - which is section C.

**Verdict: right model, one honest cost, correctly located.**

---

## C. The Lua question

### C1. What is actually painful

Not expressiveness. **Ceremony, and the missing sequencing primitive.**

Exhibit A, verbatim from `assets/base/scenarios/shakedown_run.content.ron:767-811`
(two of nineteen such handlers in that one file):

```ron
(
    name: OnUpdate,
    filters: [
        Expression((Equal(
            Term(Factor(Name("open_step"))),
            Term(Factor(Literal(Number(0.0)))),
        ))),
        Expression((GreaterThan(
            Term(Factor(Name("scenario_elapsed"))),
            Term(Factor(Literal(Number(2.0)))),
        ))),
    ],
    actions: [
        VariableSet((
            key: "open_step",
            expression: Term(Factor(Literal(Number(1.0)))),
        )),
        StoryMessage((
            speaker: "Capt. Halloran",
            text: "Shakedown's your own now - fresh hull, cold guns. ...",
        )),
    ],
),
(
    name: OnUpdate,
    filters: [
        Expression((Equal(
            Term(Factor(Name("open_step"))),
            Term(Factor(Literal(Number(1.0)))),
        ))),
        Expression((GreaterThan(
            Term(Factor(Name("scenario_elapsed"))),
            Term(Factor(Literal(Number(5.0)))),
        ))),
    ],
    actions: [
        VariableSet((
            key: "open_step",
            expression: Term(Factor(Literal(Number(2.0)))),
        )),
        StoryMessage((
            speaker: "You",
            text: "Copy, Halloran. Board's green, lines are cold.",
        )),
    ],
),
```

Twenty-three lines of RON per line of dialogue. The scale:

| corpus | lines |
| --- | --- |
| `assets/base/scenarios/*.content.ron` (generated) | 8,068 |
| `webmods/*/*.content.ron` (hand-written) | 8,907 |
| `assets/mods/example` | 784 |
| **total authored scenario RON** | **17,759** |

Three separate problems are visible in that excerpt:

1. **`Term(Factor(Literal(Number(2.0))))` is four wrapper nodes around a
   number.** The grammar's precedence chain is EXPOSED as syntax. That is a
   parser-internals leak, not a language.
2. **There is no sequencing primitive**, so a linear conversation is encoded as
   a hand-rolled program counter (`open_step`) fanned across N sibling handlers,
   each of which must restate the guard, the bump, and the payload.
3. **The guard and the bump are separated by the payload**, so reading the flow
   means reading every handler's first filter and last action and reassembling
   the order in your head.

Exhibit B, from `docs/scenario-system.md` and `webmods/gauntlet/`: the
"act-gating pattern", where every terminal handler must be guarded on a sentinel
counter value (`gate < 8`) so a post-victory death does not overwrite a Victory
with a Defeat. That is manual invariant maintenance across handlers that are
hundreds of lines apart. It is also exactly the class of bug a type system or a
structured control-flow construct would make unrepresentable.

### C2. What it would look like in Lua

```lua
scenario.opening = coroutine.wrap(function()
  wait(2);  say("Capt. Halloran", "Shakedown's your own now - fresh hull, cold guns.")
  wait(3);  say("You",            "Copy, Halloran. Board's green, lines are cold.")
  wait(3);  say("Capt. Halloran", "Belt's quiet today. Good day to learn her helm.")
  wait(3);  say("You",            "Understood. Where do you want me?")
  wait(3);  say("Capt. Halloran", "Salvage beacon's lit dead ahead.")
end)
```

Five lines instead of one hundred and fifteen. That is a 20x reduction and it is
not a rhetorical trick - the whole `open_step` variable disappears, because a
coroutine's resume point IS the program counter.

Note carefully what did the work there. It was **not** Lua's type system, its
closures, its tables, its metatables or its stdlib. It was **one coroutine and
one `wait`**. The entire win in the motivating example comes from a sequencing
primitive.

### C3. What Lua costs here

**Determinism (the expensive one).** `CONVENTIONS.md` Nova 4 requires seeded
`bevy_rand` because thread RNG "silently voids every probe assertion built on a
seeded layout", and the project has been burned by exactly that. Today the rule
layer is *provably* deterministic: every filter is a pure function of
(variables, payload), every action is a write, and the only randomness is
`ScatterObjectsConfig` drawing from `StdRng::seed_from_u64(self.seed)`
(`actions/spawn.rs:330`) with an authored seed. A Lua VM brings in at least
three silent nondeterminism sources: `pairs()` iteration order over a table is
explicitly unspecified and in practice varies with hash seed; `math.random` is a
second, unseeded PRNG stream; and GC timing changes allocation addresses, which
in a stackless VM can feed back into iteration order. None of these are visible
in a diff. All of them void the probe roster in
`crates/nova_probe_cli/tests/catalog_drift.rs`.

This is not fatal - you can lock it down (fixed table iteration, replace
`math.random` with a bound to the seeded `bevy_rand` stream, forbid
`os.time`/`os.clock`). But it is a permanent, ongoing sandboxing burden, and the
failure mode is a silently-weaker probe run, which is the worst failure mode
this project has.

**Save/load.** Currently free: `NovaEventWorld` is a `HashMap<String,
VariableLiteral>` plus a few `Vec`s of serde-derived configs, so a save is a
serialise. There is **no save system today** (no grep hits), so this is a future
cost, but it is a large one: a suspended Lua coroutine is not serialisable in
any Lua implementation without a bespoke continuation-capture scheme. The
coroutine that makes C2 beautiful is the exact thing that makes mid-scenario
save impossible. Worth deciding NOW whether mid-scenario save is ever wanted.

**Hot reload.** Currently absent (no `watch_for_changes` anywhere). RON would be
trivial to hot-reload because handler state lives entirely in `NovaEventWorld`.
Lua with live coroutines would not be.

**WASM.** The game ships a Trunk/wasm path and `nova_mod_format`'s whole design
is bent around it - `BundleManifest`'s doc says the manifest, not directory
enumeration, "is what makes bundles wasm-safe (`load_folder` is broken on the
web target)". The prior spike (`tasks/20260708-161726/SPIKE.md`) already picked
`piccolo` over `mlua` for exactly this reason, and that call still looks right:
pure Rust, no vendored C, and fuel-based stepping that bounds CPU per frame.
The cost is that piccolo remains WIP with an incomplete stdlib and a stackless
`Sequence` callback API that needs real binding glue.

**Security.** This is the one that has changed since the spike. `nova_mod_format`
now describes a live PORTAL: `PortalCatalog` / `PortalEntry` / `PortalFile` with
sha256 per file, `MAX_CATALOG_BYTES`, `MAX_CATALOG_ENTRIES`, and a
`check_size()` guard. Mods are **downloaded and executed on player machines,
including in a browser tab**. Today the worst a hostile mod can do is spawn
absurd content, and `content lint` catches most of that statically. With a script
VM, "arbitrary code from a stranger, in the player's browser" becomes the threat
model. Piccolo's fuel accounting handles CPU exhaustion; it does not by itself
handle a mod that quietly reads and exfiltrates whatever you bound into its
globals. Every binding becomes a security review.

**Per-call VM crossing.** `EventFilter::filter` takes `&self` and `&W`
(`engine.rs:79`) and filters are held as `Arc<dyn EventFilter<W>> : Send + Sync`.
A Lua VM needs `&mut` for its own stack and GC arena, so a `LuaFilter` must wrap
its interpreter in a `Mutex` or `RefCell` - and `Sync` forces the `Mutex`.
Multiply that by 40 filter evaluations per frame in `shakedown_run` and you are
locking a mutex forty times a frame to ask "is `open_step == 3`". The
lock is not the real cost; the real cost is that the trait signature
**silently pushes you toward a coarser seam** (one Lua call per handler, or per
frame) than the one the engine is built around.

### C4. The middle option - and it is already built

**`crates/nova_authoring` IS the compiler.** This is the most important finding
in the review, and I do not think it is being credited.

`crates/nova_authoring/src/scenario_helpers.rs` is a 209-line constructor
catalog: `number(2.0)`, `variable("gate")`, `number_equals("act", 1.0)`,
`entity_pair(a, b)`, `story_message(who, what)`, `start_timer(key, secs)`.
`crates/nova_authoring/src/base_content/scenarios/nova_protocol/pacing.rs` is a
second, higher layer that builds the *idioms*: `mark_clock`, `clock_past`,
`gated_once`, `open_outro`, `outro_beats`. `gated_once` (`pacing.rs:113`) is
literally the C2 `wait(); say()` construct, expressed as a combinator that emits
the RON.

The measured result: **4,057 non-comment Rust lines generate 8,068 lines of
scenario RON**, and the Rust reads as prose while the RON does not. `content --
gen` serialises; `content_ron_parity` pins builders == RON.

That is the exact architecture I would recommend if it did not exist: **a
declarative interchange format that a lint can walk, plus a real programming
language at authoring time that emits it.** The owner built it. It works.

The **gap** is that it is only available to first-party content. `webmods/` -
8,907 lines across two mods - is hand-written RON, because the front-end is
in-tree Rust that a modder cannot reach.

### C5. My pick

**Keep RON as the runtime contract. Do not embed a VM. Close the gap by
shipping the authoring front-end, and add ONE missing primitive to the runtime.**

Three moves, in order:

**1. Add a sequencing primitive to the vocabulary.** This is the single highest
value change in this whole review, and it is a day of work, not a quarter.
Something like:

```ron
Sequence((
    id: "opening",
    steps: [
        (after: 2.0, actions: [ StoryMessage((speaker: "Capt. Halloran", text: "...")) ]),
        (after: 3.0, actions: [ StoryMessage((speaker: "You",            text: "...")) ]),
        (after: 3.0, actions: [ StoryMessage((speaker: "Capt. Halloran", text: "...")) ]),
    ],
)),
```

One action, one variable behind the scenes (or a step index on a component),
driven by the clock the engine already keeps. It deletes roughly **19 of
`shakedown_run`'s 42 handlers, 15 of `ledger_ch3`'s 27, and 13 of `lifeline`'s
27** - which is also the `OnUpdate` load from section B, so the per-frame cost
falls out with the ceremony. It is serialisable, hot-reloadable, lintable, and
deterministic, and it captures 90% of what the coroutine in C2 was buying.

Check the prior spike's own trigger condition before doing this: it said phase 2
starts "when we would otherwise be tempted to keep growing `variables.rs`". Note
what this proposal is - it grows the ACTION vocabulary with a control-flow
construct, and it deliberately does NOT grow the expression language. See E for
why the expression language is not the thing that needs growing.

**2. Ship `scenario_helpers` + `pacing` as a public authoring crate, and write
`/create/` docs for it.** Modders who want real code get real code - in Rust,
today, with type checking and rustc's error messages, and no VM in the shipped
binary. `content -- gen` already does the lowering. This is a documentation and
packaging task, not an engineering one, and it converts `webmods/`'s 8,907
hand-written lines into something maintainable.

**3. If a scripting front-end is still wanted after 1 and 2, put Lua at
AUTHORING time only.** A `mod.lua` that runs in the `content` CLI, emits
`*.content.ron`, and never ships. Cost: zero determinism risk, zero WASM risk,
zero security risk, zero per-frame cost, and `content lint` still gets the full
program to walk. The lint even improves, because you can lint the emitted RON
and report the finding against the Lua source line.

I would not do runtime Lua at all unless a concrete requirement appears that
data cannot express - a mod that needs its own pathfinding, its own economy
simulation, its own UI. None of the shipped content is close to that line.

### C6. What a naive Lua port would LOSE

This is the part that should decide it.

**`content lint` is whole-program static analysis, and it only works because the
vocabulary is closed and the content is data.** `crates/nova_scenario/src/lint/scenario.rs`
(1,452 lines) walks every handler of every scenario and proves things no runtime
check can:

- a `NextScenario` naming a scenario no bundle provides is an ERROR
  (`scenario.rs`, `dangling_next_scenario_is_an_error`)
- a filter targeting an id nothing spawns is an ERROR
  (`unspawnable_filter_id_is_an_error_but_scatter_prefix_satisfies`)
- a `SetAllegiance` or `ForceTorpedoLaunch` on a nonexistent ship is an ERROR
- a menu backdrop with no `SetCamera` is an ERROR (so it degrades to "not in the
  rotation" instead of "menu with no picture")
- an `ObjectiveComplete` with no matching `Objective`, or a variable read that
  nothing sets, is a WARN
- a write to a watched variable (`scenario_elapsed`, `player_speed`) is an ERROR
- duplicate spawn ids, absurd scatter counts, out-of-range story dwell, pacing
  and beat-sheet checks

Plus the balance audit graded against each bundle's `balance_acks.ron`, with a
stale ack an ERROR, and the flight-rig input-overlap check.

**Every single one of those becomes undecidable the moment a handler's target id
can be `"gate_" .. i`.** Not harder - undecidable. You would be trading a static
gate that runs in CI (`content_lint_gate`, `balance_audit_gate`) for runtime
warnings that only fire if a player reaches that branch.

The second casualty is `/create/`. `web/src/create/reference.md` is an
exhaustive, A-to-Z, construct-by-construct catalog with field tables, defaults,
units and copyable snippets, across 4,476 lines of docs. It is exhaustive
BECAUSE the vocabulary is closed and finite. A Lua API's surface is its bindings
plus the language plus the stdlib subset you allow, and the equivalent document
does not exist for any Lua-modded game I know of - they all have a wiki of
examples instead. `CONVENTIONS.md` Documentation 4 says `/create/` "is the
authored contract and must be exact". A script API cannot be exact in that sense.

Third: the closed vocabulary is what makes `content_ron_parity` possible, what
makes headless dispatch tests trivial (`filters.rs:256`, `dispatch_app()` on
`MinimalPlugins`), and what lets the probe roster name every assertion.

---

## D. Extensibility - is this a dead end?

**No. It is a good foundation, and the seam is unusually clean.**

The seam is `EventFilter<W>` / `EventAction<W>` (`engine.rs:68` and `:79`).
Handlers hold `Arc<dyn ...>` trait objects and neither the dispatcher
(`engine.rs:429`) nor the index (`engine.rs:371`) knows or cares what is behind
them. A `LuaFilter` / `LuaAction` implementing those two traits would dispatch
identically to `ExpressionFilterConfig`. Nothing in `loader/`, `world.rs` or the
sync systems would change.

Four things WOULD have to change:

1. **`&self` on both traits.** A Lua VM needs `&mut` for its stack and GC arena.
   Since `Arc<dyn EventFilter<W>> : Send + Sync`, that means `Mutex`, not
   `RefCell`. Either accept the lock, or change the trait signatures to `&mut
   self` and hold handlers as `Arc<Mutex<...>>` (which costs the cheap-clone
   snapshot property `EventHandlerIndex` was built for). Neither is hard;
   both are decisions the current code has not had to make.

2. **`NovaEventWorld` needs a stable, scriptable surface.** Right now actions
   reach into private fields of `world.rs:66` directly. A script binding needs a
   deliberate, versioned API - and that API becomes the security boundary from
   C3. This is the real work.

3. **Two enum variants for the RON.** `EventFilterConfig::Script(...)` and
   `EventActionConfig::Script(...)` (`filters.rs:27`, `actions/mod.rs:45`). Both
   are one arm plus one match arm - the modules' own doc comments promise
   exactly this ("a new filter kind is one enum arm plus one match arm and
   nothing else moves"). This means Lua could be added **incrementally, beside**
   the declarative form, not as a replacement, which preserves everything in C6
   for the content that does not use it.

4. **`content lint` needs an escape hatch.** A scenario containing a `Script`
   filter cannot be fully proven. It should DOWNGRADE loudly (a WARN naming
   which checks were skipped), never silently pass. Get that right on day one or
   the lint's authority is gone.

The one architectural thing that would need thought is `is_settling`
(`engine.rs:48`, `:457`). A script that queues world work mid-execution has to
be suspendable at that point, or the "the world is not yet live" invariant
breaks. In piccolo's stackless model that is actually natural; in mlua it is
not. That is one more independent vote for the spike's original piccolo call.

---

## E. Minimality

Counts are over the 22 shipped `*.content.ron` files (`assets/base/`,
`assets/mods/example/`, `webmods/`). "Elsewhere" means examples, tests, or the
bench.

### Dead: zero authored uses anywhere

| construct | file:line | notes |
| --- | --- | --- |
| `ScenarioObjectKind::Anchor` | `actions/spawn.rs:115` | 0 uses. Carries `objects/anchor.rs` (181 lines), `ANCHOR_TYPE_NAME`, and a `/create/objects/#anchor` section. `docs/scenario-system.md` justifies it as the deterministic-gravity-well alternative to a carved rock - a case that never arrived. |
| `EventActionConfig::Screenshot` | `actions/mod.rs:88` | 0 uses, anywhere. `nova_autopilot` has its own capture path (`autopilot/src/screenshot.rs`, `capture.rs`), which is what the `screenshots/` examples actually use. Redundant. |
| `EntityFilterConfig::type_name` | `filters.rs:65` | 0 authored uses. Only `examples/systems/system_scenario_grammar.rs:347` and the bench. |
| `EntityFilterConfig::other_type_name` | `filters.rs:78` | 0 authored uses, 0 elsewhere. |
| `ConditionalFilterConfig::Not` | `filters.rs:169` | 0 uses. |
| `ConditionalFilterConfig::And` | `filters.rs:173` | 0 uses. (Two filters on a handler already AND.) |
| `VariableTermNode::Multiply` | `variables.rs:106` | 0 authored uses (bench only). |
| `VariableTermNode::Divide` | `variables.rs:108` | 0 authored uses. |
| `VariableFactorNode::Parens` | `variables.rs:53` | 0 authored uses. |
| `VariableLiteral::String` as an authored literal | `variables.rs:40` | 0 uses. So is string concat via `Add`. |
| `VariableLiteral::Boolean` as an authored literal | `variables.rs:44` | 0 authored uses (tests only). So are boolean `Add`=OR and `Multiply`=AND. |
| `VariableFactorNode::Query` inline in an expression | `variables.rs:59` | 0 uses. Queries reach content ONLY through `watches`. |
| `HudReadoutFormatConfig::Number` | `actions/mission.rs:93` | 0 uses (and it is the `#[default]`). |
| `HudReadoutFormatConfig::Integer` | `actions/mission.rs:95` | 0 uses. Only `Time` is used (2x). |
| `EventConfig::OnOrbitStart` | `events.rs:42` | 0 authored handlers. |
| `EventConfig::OnTravelLockEnd` | `events.rs:53` | 0 authored handlers. |
| `EventConfig::OnCombatLockEnd` | `events.rs:57` | 0 authored handlers. |
| `ModEntry::hidden` | `nova_mod_format/src/lib.rs:138` | Its own doc says "No shipped mod uses it right now". |
| `NovaEventWorld::world_to_state_system` | `world.rs` | Empty body. Its comment says it is "Kept ... so the plumbing exists if a future action needs live world state pulled in". That is the textbook definition of speculative machinery, which `AGENTS.md` forbids. |

### Barely used: one file each

| construct | uses | files |
| --- | --- | --- |
| `EventConfig::OnExit` | 1 | 1 |
| `EventConfig::OnOrbitStable` / `OnOrbitUnstable` / `OnOrbitEnd` | 1 each | 1 |
| `EntityProperty::Speed` (the only entity query) | 1 watch | `ledger_ch3` |
| `SetSpeedCap` | 1 | `shakedown_run` |
| `DebugMessage` | 2 | 2 |
| `HudReadout` | 2 | 2 |
| `SetControllerVerb` | 3 | `shakedown_run` |
| `TimerCancel` | 3 | 2 |
| `ConditionalFilterConfig::Or` | 4 | 2 (`lifeline`, `final_tally`) |
| `ForceTorpedoLaunch` | 5 | 2 (menu backdrops only) |
| `HintEmphasisSet` / `Clear` | 6 / 5 | tutorial only |
| `DespawnScenarioObject` | 7 | 2 |
| `CreateScenarioArea` | 7 | - |

### Healthy: earning their place

`VariableSet` 346, `SpawnScenarioObject` 223, `StoryMessage` 105, `Entity`
filter 112, `Expression` filter 415, `ObjectiveComplete` 63, `Objective` 48,
`ObjectiveMarkerAttach`/`Detach` 40/41, `NextScenario` 40, `Outcome` 40,
`TimerStart` 35, `ScatterObjects` 18, `SetAllegiance` 11. Object kinds:
`Asteroid` 83, `Spaceship` 62, `Light` 57, `Beacon` 32, `SalvageCrate` 7.
Conditions: `Equal` 289, `GreaterThan` 89, `LessThan` 37. `ScatterRegion`:
`Box` 9, `Ring` 9.

### What the dead list MEANS

Read it as a shape, not a chore list. **Everything unused clusters in the
expression language and the filter combinators. Nothing unused clusters in the
action vocabulary.**

Of the 16 expression-grammar nodes `/create/reference.md` advertises, shipped
content uses exactly seven: `Number` literals, `Name`, `Term`, `Factor`, `Add`,
`Subtract`, and the three comparisons. No multiplication. No division. No
parentheses. No strings. No booleans. No inline queries. No `Not`, no `And`.

**That is a counting machine, not a language.** The mini-language was built for
generality that authors have never once reached for in 17,759 lines of content -
and it is simultaneously missing the ONE thing every scenario has to hand-roll,
which is sequencing. That is the whole Lua argument in one paragraph, and it
argues against Lua: authors are not blocked on expressiveness, so a more
expressive language solves a problem they do not have, while costing everything
in C6.

### Recommended deletions

Zero risk, pure subtraction of maintenance and documentation cost:

1. `ScenarioObjectKind::Anchor` + `objects/anchor.rs` + `ANCHOR_TYPE_NAME` + its
   `/create/objects/` section. If deterministic wells are wanted later, an
   `Asteroid` with an authored `mass` and a fixed `seed` already gets there.
2. `EventActionConfig::Screenshot` + `ScreenshotActionConfig`. `nova_autopilot`
   owns capture.
3. `EntityFilterConfig::type_name` and `other_type_name`. This one also removes
   two `serde_json` map lookups and two string compares from the hottest filter
   in the engine (`filters.rs:81`), which is a measurable win for `OnEnter`
   storms as well as a documentation cut.
4. `ConditionalFilterConfig::Not` and `And` (keep `Or` - it has 4 real uses and
   there is no other way to express it). That collapses `Conditional` to a
   single-purpose `Or` filter, which is honest.
5. `VariableTermNode` entirely (`Multiply`, `Divide`), `VariableFactorNode::Parens`,
   and `VariableLiteral::String` / `Boolean`. This flattens the grammar from
   five levels to three and cuts `/create/expressions.md` roughly in half.
   Caution: `Boolean` is used by the ENGINE in tests and is the natural type for
   a latch, so check whether flattening forces content to use `0.0`/`1.0`
   numbers where it currently could use a bool - it already does, in every
   shipped file, but confirm before deleting the variant.
6. `HudReadoutFormatConfig::Number` and `Integer`. Note `Number` is `#[default]`,
   so removing it means picking a new default or making the field required.
7. `NovaEventWorld::world_to_state_system`'s empty body - make it a no-op
   default on the trait rather than a kept-for-later stub in the impl.

Hold, do not delete:

- The zero-use orbit/lock events (`OnOrbitStart`, `OnTravelLockEnd`,
  `OnCombatLockEnd`). They complete lifecycle PAIRS whose other halves are used,
  and `docs/scenario-system.md` documents them as one-shot edges a scenario
  composes with a timer. An asymmetric lifecycle (a start with no end) is worse
  than an unused variant.
- `ModEntry::hidden`. Its semantics are pinned by tests and it is one bool.

---

## Summary

The model is an event-condition-action rule engine with a closed vocabulary and
a staged world seam, dispatched from an event-driven queue with a by-name index.
It is the right model. The Wesnoth lineage brought the good parts (closed tag
set, declarative filters, data a lint can walk) and left the bad parts (loops,
recursion, variable substitution) behind. The one real-time cost is the
`OnUpdate` pulse, it is confined, it has been benchmarked, and it is a symptom of
a missing sequencing primitive rather than of the model.

Lua at runtime would buy syntax and cost `content lint`, the exhaustive
`/create/` contract, provable determinism, and future save/load. The authoring
front-end that gives you the syntax without the costs already exists in
`nova_authoring` and just is not shipped to modders.

Add `Sequence`. Ship the front-end. Delete the seven dead constructs. Revisit
Lua when a mod needs to do something data genuinely cannot express - which,
across 17,759 lines of shipped content, has not happened once.
