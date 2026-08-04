# Decision: where the shared example fixture builders live, and how stress/ takes its count knob

- DATE: 20260804-234504
- STATUS: ACCEPTED
- TASK: 20260804-094006
- TAGS: examples, testing, perf

## Context

`stress/` is the third caller of the same two fixture shapes that
`examples/sections/` and `examples/systems/` already build inline: a
`SpaceshipConfig` assembled out of `GameSections` lookups, and an asteroid
`ScenarioObjectConfig` with a health knob. `torpedo_section.rs:226-230` carries
an explicit owner note deferring the extraction to this task, so that three
shapes are visible before a signature is fixed.

Four load-bearing choices had to be made before the Steps are executable.

## Decision

**D1. The builders live in a new `crates/nova_probe/src/fixtures.rs`,
exported from `nova_probe`'s lib.**

Callers reach them as `nova_probe::fixtures::{ship, asteroid, spawn_on_start}`.

**D2. The count knob is the `NOVA_STRESS_COUNT` env var only. No clap flag,
and no count parameter on `fixtures::ship`.**

Each stress example reads it once in `main` against a named `const
DEFAULT_COUNT` whose doc comment records the llvmpipe measurement that picked
the value. `ship` takes a `&[SectionSpec]`; the caller decides how long that
slice is, so "N sections" needs no knob inside the builder.

**D3. "Entity counts return to baseline after teardown" is an in-example
assertion that panics, not a new `nova_probe::invariants` check.**

`InvariantsPlugin` today offers `strict` and `monotonic(keys)` over scenario
variables only. The claim is about ECS entity counts, which is a different
subject.

**D4. `scene_baseline` is a pure move: every env knob keeps its `NOVA_PERF_*`
name and every default keeps its value.**

Only the file path, the example name, the clap `command(name)`, the module doc
and the panic message change.

## Alternatives considered

**D1 - `examples/support/fixtures.rs`, reached by `#[path]` from each caller.**
Rejected. `tests/examples_smoke.rs::catalog_matches_disk` scans every `.rs`
directly under an `examples/*/` directory and demands a catalog block for each,
and `every_category_has_a_probe_policy` demands a policy row per catalog
category. A `support/` category would have to dodge both by nesting the file a
level deeper (`examples/support/fixtures/ship.rs`), purely so the scan misses
it. That is a trick that reads as an accident to the next person, and it
recompiles the module once per example on top.

**D1 - a new `crates/nova_fixtures` crate.** Rejected as a concept without a
requirement: `nova_probe` already depends on `nova-protocol` and
`nova_scenario` (so it can name every type the builders return) and is already
an unconditional `[dev-dependencies]` entry of the root package (so every
example already links it). A new crate buys a boundary nothing is pushing on.

**D2 - a `--count` clap flag alongside the env var, mirroring
`perf_baseline`'s `--scenario`.** Rejected, YAGNI. `perf_baseline` has both
because a shell script sweeps scene x renderer x preset through one binary;
probe drives `stress/` through the environment (`env.rs` builds the whole
`NOVA_PERF_*` set), and no caller in this task passes argv.

**D3 - extend `InvariantsPlugin` with an entity-count check.** Deferred. One
caller is not an abstraction, and the three stress runs each count a different
marker component. If a fourth subject appears, the shape will be visible then -
the same reasoning that governed D1's own timing.

**Splitting this task.** Considered and rejected. The extraction in step 2 is
only justified by three visible callers, which means `many_sections` and
`many_projectiles` cannot be deferred without either leaving a one-caller
"abstraction" or re-doing the signature later. The Steps are ordered into six
groups that each stand alone as a commit, which is where the reviewability
comes from instead.

## Consequences

- `nova_probe` grows a gameplay-fixture module next to its harness modules. It
  is the dev-tools crate, so this is in charge, but it is a widened remit worth
  naming: if a second unrelated fixture family lands there, that is the signal
  to cut `nova_fixtures` after all.
- The four other inline `SpaceshipConfig` builders in `examples/sections/` and
  `examples/systems/` stay inline. That is deliberate: this task retargets the
  three callers whose shapes designed the signature, and a blanket sweep would
  make the diff unreviewable against a signature nobody has used yet.
- `NOVA_STRESS_COUNT` is global to a run, so `probe run stress` gives all three
  scale sweeps the same N. Each example's `DEFAULT_COUNT` differs, so the
  unset - and therefore CI - case still sizes each swarm independently.
- The release-over-release frame-time number survives the rename: D4 means an
  old sweep script only changes `--example perf_baseline` to
  `--example scene_baseline`.
