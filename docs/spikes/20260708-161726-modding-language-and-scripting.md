# Spike: Modding authoring - declarative markup, embedded Lua, or both?

- DATE: 20260708-161726
- STATUS: RECOMMENDED
- TAGS: spike, modding, scenario, roadmap

## Question

The user has wanted, for a long time, "a markup language for the modding part of
the game", and possibly a scripting language (Lua, an mlua/rlua-style binding, or
a pure-Rust Lua such as piccolo). How should nova-protocol expose scenario/mod
authoring: a declarative data format, an embedded scripting VM, or both - and in
what order? A good answer names a concrete phase-1 direction that is shippable in
0.5.0, and a defensible position on the scripting VM (which one, and when).

This spike also sets the wider 0.5.0+ roadmap: what to pull into the (nearly done)
0.4.0 sprint, and what new backlog items to seed. Those are captured in "Next
steps"; the body focuses on the modding-language decision, which is the hard part.

## Context

The scenario/mod engine already exists and is surprisingly complete - it is just
not serialized. Everything below is Rust today, built programmatically in
`crates/nova_assets/src/scenario.rs` and stored in a `GameScenarios` resource.

The data model (all in `crates/nova_scenario/src/`):

- `ScenarioConfig { id, name, description, cubemap, events: Vec<ScenarioEventConfig> }`
  (`loader.rs`).
- `ScenarioEventConfig { name: EventConfig, filters: Vec<EventFilterConfig>, actions: Vec<EventActionConfig> }`
  - `EventConfig`: `OnStart | OnUpdate | OnDestroyed | OnEnter | OnExit` (`events.rs`).
  - `EventActionConfig` (7 actions, `actions.rs`): `DebugMessage`, `VariableSet`,
    `Objective`, `ObjectiveComplete`, `SpawnScenarioObject`, `CreateScenarioArea`,
    `NextScenario`.
  - `EventFilterConfig` (`filters.rs`): `Entity(id/type_name/other_*)`,
    `Conditional(Not/And/Or)`, `Expression(...)`.
- A hand-rolled expression/condition mini-language (`variables.rs`): a recursive
  AST of `VariableFactorNode`/`TermNode`/`ExpressionNode`/`ConditionNode` over
  `VariableLiteral(String|Number|Boolean)`, with `evaluate(&NovaEventWorld)`.
  Used for `VariableSet` actions and `Expression` filters (e.g. increment
  `asteroids_destroyed`, gate on `asteroids_destroyed > 4`).
- Handlers are spawned as entities carrying `EventHandler<NovaEventWorld>` (the
  generic event framework from `bevy_common_systems`); actions mutate a staging
  `NovaEventWorld` (`world.rs`) which is flushed to the Bevy world in a second
  phase (`state_to_world_system`).

Two facts decide the shape of the answer:

1. **The config model already covers ~90% of scenario authoring** (spawn objects,
   trigger areas, objectives, scenario transitions, variables, boolean/arith
   conditions). It derives `Reflect` but **not** `serde` (grep: only
   `nova_events`/`nova_gameplay` pull serde today, for event *payloads*).
   `docs/scenario-system.md:146` already flags this: "a real modding pipeline
   would deserialize these configs", and there is a literal
   `// This should be loaded from a JSON file` note in `sections.rs`.
2. **There is already a bespoke expression language** (`variables.rs`). It is the
   canary: the moment modders want more than arithmetic + comparisons (functions,
   locals, loops, richer state), that AST stops scaling and a real VM earns its
   keep. Until then it does not.

Constraints that bias the choice:

- The game ships **native and wasm** (Trunk build). There is already one
  wasm-blocked feature (particles/hanabi, task 162908). Anything added should not
  make the wasm target worse.
- Mods are, by definition, **untrusted third-party input**. Whatever runs them
  wants sandboxing (no filesystem/network by default; ideally CPU/RAM bounds).
- The project's style is "correct and maintainable over fast" (`AGENTS.md`), and
  it already leans on `bevy_common_systems` for generic reuse.

## Options considered

### Modding-format axis (the "markup language")

- **A. RON data files + `AssetLoader` (serialize the existing model).** Add
  `serde` derives to the config enums, write a Bevy `AssetLoader` for
  `*.scenario.ron`, and move the built-in scenarios out of `scenario.rs` into
  asset files. RON is the Bevy-ecosystem-native format, maps cleanly onto Rust
  enums/structs (which the model already is), supports comments, and needs no
  hand-written parser. Pros: cheapest path, reuses everything, safe (pure data),
  trivially wasm-safe, unblocks the objectives/config tasks. Cons: RON's
  enum/struct syntax is a bit noisy for non-Rust modders; still limited to the
  fixed action/filter set; the expression AST is awkward to author by hand.
- **B. A custom human-friendly markup (bespoke DSL).** Design a purpose-built
  surface syntax (`on destroyed(player) { next_scenario("lose") }`-ish) with our
  own parser. Pros: nicest authoring experience; can hide the AST behind readable
  syntax. Cons: a whole parser + error reporting + tooling to build and maintain
  forever; easy to under-design; no ecosystem. High effort for a solo project,
  and it competes with just embedding a real language (see scripting axis).
- **C. KDL / TOML / JSON.** Off-the-shelf document formats. KDL is a genuinely
  nice nested-node document language; TOML is awkward for deeply nested,
  enum-heavy trees; JSON has no comments and is painful to hand-author. All
  require a mapping layer to the config enums that RON gives for free. Pros:
  familiar. Cons: extra dep + mapping work for no win over RON (KDL), or a worse
  fit than RON (TOML/JSON).
- **D. Do nothing (keep Rust-defined scenarios).** Always a candidate. Costs all
  moddability and keeps content changes gated behind a recompile; contradicts the
  stated long-term goal. Only acceptable as "not yet".

### Scripting axis (the optional "Lua or Rust-Lua")

- **E. `mlua` (or rlua/hlua) - C Lua via FFI.** The mature, ergonomic binding;
  `bevy_mod_scripting_lua` (v0.19, Jan 2026) is built on mlua and gives a
  ready-ish Bevy<->Lua bridge. Pros: real language, huge ecosystem, best-in-class
  ergonomics for binding Rust functions. Cons: links a C Lua/LuaJIT - heavier and
  fiddlier on wasm (vendored lua54 works but adds weight); sandboxing untrusted
  scripts (CPU/RAM/instruction limits) is manual and error-prone; couples the
  update loop to a C VM.
- **F. `piccolo` - pure-Rust, stackless Lua.** Pros: pure Rust (no C dep) so the
  **wasm target stays clean**, which matters here; **fuel-based execution** bounds
  CPU and RAM per step, i.e. real sandboxing of untrusted mods almost for free;
  cycle-collecting GC (gc-arena); used in production (Fish Folk for game scripting,
  Ruffle for its ActionScript VM). Cons: **WIP** - incomplete stdlib, sandboxing
  still maturing; the stackless/`Sequence` callback API is more work to bind than
  mlua's; no Bevy integration crate, so we write the glue.
- **G. `bevy_mod_scripting` off the shelf.** Least glue for Lua-in-Bevy. Cons:
  heavy, tracks Bevy releases closely (the game is already doing 0.17->0.19
  migrations by hand), "significant API changes anticipated", and it assumes its
  own world-access model rather than this project's `NovaEventWorld` staging.
- **H. Extend the existing expression AST instead of a VM.** Grow `variables.rs`
  with functions/more operators. Pros: no new dep. Cons: reinventing a language
  one feature at a time; this is exactly the trap a real VM avoids.

## Recommendation

**Stage it. Ship the declarative format first (Option A, RON), and treat scripting
as a separate, later, prototype-gated phase (Option F, piccolo) - not part of the
same release.**

### Phase 1 (target 0.5.0): declarative RON scenario format

Do Option A. Concretely: add `serde` derive to the `ScenarioConfig` /
`EventConfig` / `EventActionConfig` / `EventFilterConfig` / `variables.rs` enums,
write a `*.scenario.ron` `AssetLoader`, and port `asteroid_field` / `asteroid_next`
out of `scenario.rs` into `assets/scenarios/*.ron` so the built-ins dogfood the
format. This is the "markup language for modding" the user wants, and it is mostly
mechanical because the model already exists and is already `Reflect`. It is safe
(pure data, no code execution), wasm-trivial, and it directly delivers /
supersedes the existing objectives/config tasks:

- 133029 (scenario language/config format) becomes "the RON format + loader".
- 133028 (scenario config resource) becomes "load `GameScenarios` from assets".
- 133026 / 133027 (hardcoded objectives + HUD, win/lose) build on top of it.

It beat the bespoke-DSL (B) and KDL/TOML/JSON (C) because RON maps onto the
existing enums for free - a custom parser or a mapping layer is pure added cost
when the data model is already Rust enums. It beat "do nothing" (D) because
moddability is the stated goal and the code already has the stubs pointing here.

### Phase 2 (a later sprint, 0.6.0+, spike-gated): embedded scripting

Only when the declarative form provably runs out of road - i.e. modders need
custom actions/conditions the fixed enum set and the arithmetic AST cannot
express. That threshold is visible: it is when we would otherwise be tempted to
keep growing `variables.rs` (Option H) or keep adding `EventActionConfig`
variants for one-off behaviours.

When that happens, recommend **piccolo over mlua for this game specifically**,
for two reasons that are decisive *here* even though mlua is the more mature
library in general:

1. **Wasm.** Piccolo is pure Rust, so it does not complicate the existing
   Trunk/wasm build the way a vendored C Lua would. The game already carries one
   wasm-blocked feature; adding a second is a bad trade.
2. **Sandboxing untrusted mods.** Piccolo's fuel-based stepping bounds CPU and
   memory per frame, which is exactly the property you want when running
   arbitrary community scripts inside the game loop. Getting the same guarantee
   out of mlua is manual and fragile.

The accepted cost is that piccolo is WIP and the stackless API needs more binding
glue. Because of that immaturity, phase 2 must start as a **prototype/spike
task**, not a commit-to-it task: build a throwaway integration that runs one
scenario hook (say, a custom `OnUpdate` condition) through piccolo end to end,
measure the binding ergonomics and wasm build impact, and only then decide. If
piccolo blocks (missing stdlib we need, sandboxing gaps), the documented fallback
is mlua with a vendored `lua54` feature and a manual instruction-count limiter -
accepting the heavier wasm story. `bevy_mod_scripting` (G) is explicitly *not*
recommended: its Bevy-version coupling and its own world-access model fight this
project's hand-rolled `NovaEventWorld` more than they help.

### Why not do scripting now

The declarative format covers the near-term authoring need, is low-risk, and is
shippable soon. Scripting is a large, higher-risk integration whose value only
materializes once the declarative ceiling is hit. Doing them together would let
the risky half hold the easy, valuable half hostage. Keep them in separate
sprints.

## Open questions

- **RON vs a friendlier surface later.** RON is right for phase 1, but if
  community modders find its enum/struct syntax too noisy, a thin friendlier
  surface (KDL, or a small custom front-end that lowers to the config model) could
  be added later without redoing the loader. Resolve by dogfooding the ported
  built-in scenarios and seeing how they read.
- **Where the expression AST goes.** If phase 2 lands piccolo, does the
  `variables.rs` AST get retired in favour of Lua expressions, or kept for the
  simple/safe cases? Decide during the phase-2 prototype.
- **Modding surface for *sections*, not just scenarios.** `sections.rs` has the
  same "load from JSON" stub. Phase 1 could extend to section blueprints too;
  scope this when planning 133029.
- **piccolo maturity at the time phase 2 starts.** It is WIP; re-evaluate its
  stdlib/sandboxing state at that point rather than trusting this spike's snapshot.

## Next steps

Direction-level tasks (for `/plan` to break into steps). Modding tasks are
retagged/annotated rather than duplicated where they already exist.

### 0.4.0 pull-ins (retag v0.5.0 -> v0.4.0)

0.4.0 ("sections polish + testability", `docs/2026-07-07-v0.4.0-plan.md`) is
essentially shipped. Pull in only the section-visual-polish items that *finish*
that theme; leave the feature epics in 0.5.0.

- 133022 (torpedo-fired HUD indicator) -> v0.4.0, p55 - finishes torpedo UX;
  there is already a TODO in `hud/torpedo_target.rs`.
- 133023 (blast-radius visual) -> v0.4.0, p50 - can be a shader (not necessarily
  particle-blocked); completes detonation feedback.
- 133011 (status info on `ScenarioLoaded`) -> v0.4.0, p40 - tiny; supports the
  testability theme.
- 133024 (torpedo bay particles) stays v0.5.0 on purpose - coupled to the
  wasm-blocked particle system (162908).

### Modding tasks reframed around this spike (stay v0.5.0)

- 133029 scenario language/config format -> phase 1: RON `AssetLoader` + serde on
  the config model; cite this spike.
- 133028 scenario config resource -> load `GameScenarios` from the RON assets;
  cite this spike.

### New backlog tasks seeded (0.5.0+)

- New (v0.6.0, spike): embedded scripting VM prototype (piccolo), phase 2 - gated
  on the phase-1 format existing and the declarative ceiling being hit.
- New (v0.5.0, audio): audio/SFX system (thrust, weapon fire, explosions,
  impacts) - no audio exists today.
- New (v0.5.0, ai): smarter enemy AI (target selection, evasion, patrol) - TODOs
  in `input/player.rs` / `input/ai.rs`; today it only "shoots nearest".
- New (v0.5.0, polish): hit feedback / juice (camera shake, hit flash, impact FX).
- New (v0.6.0, editor): ship-editor polish + save/load ship blueprints
  (rotation, copy/paste, templates) - dovetails with the phase-1 asset format.
- New (v0.6.0, weapons): weapon & damage-type variety (alt-fire, AP/EMP) -
  extends existing 133004 (variable damage) and 133025 (ammo).
</content>
</invoke>
