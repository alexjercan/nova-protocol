# Decision: Refactor nova_* crates for structure and clarity

- DATE: 20260807-134851
- STATUS: ACCEPTED
- TASK: 20260806-121625
- TAGS: refactor, benchmark, conventions, ci, nova_gameplay, nova_probe, nova_ui, nova_assets, nova_editor

## Context

~155k LOC across 16 crates. The cost is **orientation**: half the workspace is
`nova_gameplay`, 43% of that crate is `hud/`, and `hud/` contains a 14.3k-line
terminal runtime that is not a HUD. `AGENTS.md`, the map that would orient a
reader, is measurably wrong on three counts.

The understanding phase ran three parallel workstreams over seven audits and a
six-reviewer code review, all against `4a8b55aa`:

| Output | Holds |
| --- | --- |
| `notes/16-findings-master.md` | 86 deduplicated findings, every `file:line` re-opened against the tree, ranked by expected harm |
| `notes/17-lanes.md` + `plan/lane00-11.md` | 12 lanes with dependencies, verification and per-lane implementation outlines |
| `CONVENTIONS.md` | 12 house-style rules, all ruled, each with a measured violation count |
| `benchmark/` | the navigability harness: 30 tier 1 questions, 3 tier 2 design tasks, a tier 3 mod brief, Docker isolation per persona |

Three premises the phase **tested and rejected**, which changed the scope:

1. "Useless comments all over the code" - measured 83% why-comments; a strict
   purge yields ~440 lines of 155,587. The deletion target was redirected to
   stale narrative, duplicated implementations and dead surface.
2. "The never-compiled wasm paths have rotted" - all 14 crates type-check
   clean on `wasm32-unknown-unknown`.
3. "The simulation core needs attention" - flight, physics, integrity and
   gravity were audited deeply and came back clean. What is defective is the
   layer above.

The review added an axis nobody planned for: **`nova_probe` is the CI gate and
it is blind in four ways, three failing OPEN.** A green sweep after a large
refactor currently means less than it appears to.

## Decision

**One tatr task, all 86 findings, split into 12 lanes.** Not a parent epic.

Lanes are ordered by one question, which matters more than any severity
ranking: **does the lane move a file, rename a symbol, or edit a doc?** If yes
it is BLOCKS BASELINE and must be explicitly ordered relative to the benchmark
baseline. If no it is NEUTRAL and can land in parallel.

| Lane | Baseline | Depends on |
| --- | --- | --- |
| L0 Fix the map, close the CI gaps | **BLOCKS** (before) | - |
| L1 Unblind the probe gate | NEUTRAL | - |
| L2 Build and baseline the benchmark | is the gate | L0 |
| L3 Untrusted input, data loss, persistence | NEUTRAL | L1 |
| L4 Reconciler discipline and terminal input | NEUTRAL | L1 |
| L5 Delete the dead and lying surface | **BLOCKS** (after) | L2 |
| L6 nova_editor | NEUTRAL | L1 |
| L7 `nova_ui::screen` extraction | **BLOCKS** (after) | L2 |
| L8 nova_probe restructure | **BLOCKS** (after) | L1, L2 |
| L9 nova_gameplay four-way split | **BLOCKS** (after) | L2, L4, L5, L8 |
| L10 nova_assets / nova_scenario cleanup | **BLOCKS** (after) | L2, L3 |
| L11 Perf and small correctness | NEUTRAL | L1 |

Two consequences of that line are easy to get backwards and are the reason it
is written down:

1. **L0 lands BEFORE the baseline.** A baseline taken against a tree CI does
   not check, with docs that lie, produces an unattributable delta.
2. **L5 lands AFTER it.** Deletion count is success criterion #2; lines deleted
   before the baseline never enter the ledger.

Five rulings shape the whole task:

1. **L1 goes first among the code work.** It is the one lane whose absence
   invalidates every other lane's verification. It needs its own
   fixture-driven harness, because the thing being fixed is the thing that
   normally does the verifying.
2. **Two benchmark runs, owner-driven.** The L2 baseline and one final run.
   Not per seam. **The owner starts and runs both** - no lane and no agent runs
   the benchmark; a lane that needs a number stops and prompts.
3. **Re-key once, before the final run.** The epic moves the things
   `keys/tier1.json` cites. Question text frozen; only `expect` and `citation`
   change; a question whose answer no longer exists is a finding, not a re-key.
4. **CONVENTIONS.md is not a lane.** Each of its 12 rules is placed by the same
   baseline question and then by which lane already reads the affected code.
   Rule 10 (68 new `SystemSet`s) goes **inside L9, per seam** - decisions made
   before the split are all re-made after it.
5. **Clusters over findings.** 14 groups of findings are cheaper together than
   apart, and naming them is the highest-value thing `notes/17-lanes.md` does.
   The largest: F06+F07 are one failure mode in two halves; F38 is one
   extraction that kills a byte-identical duplicate and both copies of the
   workspace's only per-tick complexity bug; F01+F03 share one root cause.

Settled since, all owner rulings dated 2026-08-07:

| Item | Ruling |
| --- | --- |
| Benchmark location | **`<root>/benchmark/`**, moved in L0. `sandbox.sh:38-42` gets a named exclusion list covering `tasks/` **and** `benchmark/` |
| F61 - exact float `Equal` in the scenario DSL | **epsilon compare** inside `Equal`. Not a second `ApproxEqual` node |
| F47 - `NovaGameplayPlugin::render` | **make it real.** Gate hanabi, skybox, post and the HUD on it |
| F66 - a no-lock torpedo can never detonate | **intended - it is a misfire.** Behavior unchanged, one comment so it is not re-reported |
| F24 - which clock the AI chain runs on | **`update_fire_cadence` alone moves to `FixedUpdate`**, the rest of the chain stays in `Update`. Twelve of the thirteen systems read an eased `Transform`/`GlobalTransform`, which in `FixedUpdate` holds the previous frame's pose. Consequence: `SpaceshipInputSystems` now needs a `FixedUpdate` gate configuration in both the pause and scenario-live gates, each with its own probe test |
| F67 - the main drive's thrust unit | **stays a per-tick impulse.** Behavior unchanged; the reason is written where a reader meets the unit. Consequence: the migration (`apply_force_at_point` plus a 64x content rescale) becomes mandatory the day anything configures `Time<Fixed>` |
| F82 - four over-declared `&mut` system params | **dropped as falsified.** One path does not exist; the other three are `#[cfg(test)]` rig helpers, the shape the finding itself cleared. Consequence: no edit, and the fifth time this epic's RE-MEASURE rule has paid |

## Alternatives considered

**Splitting into multiple tatr tasks.** Rejected by the owner. The lanes share
one baseline and one findings list; splitting them buys tracking granularity
and costs the ordering fact the whole plan hangs on.

**A `clippy::pedantic` CI job.** Rejected: 66% of its output here is
`needless_pass_by_value` and `redundant_pub_crate`, both wrong for a Bevy
codebase. Plain clippy is already at 0 warnings, so `-D warnings` is free.

**A `cast_*` cleanup pass.** Rejected: sampled from two directions and measured
clean - every float-to-int cast read was clamped within 2 lines.

**Deriving convention rules without violation counts.** Rejected. Against a
codebase this clean a rule with no measured count produces only false
positives. Every accepted rule carries one.

**Re-keying the benchmark per seam.** Rejected by the owner in favor of one
pass before the final run. Accepted cost: a seam that is not paying for itself
is not visible until the epic is over.

**A workspace-wide prelude pass** for CONVENTIONS rules 3 and 4. Rejected:
deciding what goes in a module's prelude *is* deciding what crosses its
boundary, which is the audit the structural lanes already do. Each lane pays
for its own crates; L5 takes the 19 in crates no structural lane opens.

**Leaving the benchmark in the task folder.** Rejected: safe today because
`tasks/` is already excluded from every image, but the tool outlives the epic.

## Consequences

- **CI gains three jobs and a flag**: `-D warnings` (free today, 0 warnings
  measured), a default-features build, and a wasm `cargo check`. Two have
  ordering constraints inside L0 - F79 before the default-features job, the
  `report.rs` cfg gate before the wasm job.
- **`nova_probe` becomes two crates**, `nova_probe` and `nova_probe_cli`, split
  at the process boundary that already exists and that no module name states.
  **This renames the gate's own invocation**: `cargo run -p nova_probe -- run
  --all` becomes `-p nova_probe_cli`, so `ci.yaml`, `AGENTS.md` and every doc
  quoting it move in the same commit. The only rename in the epic with a
  non-Rust consumer.
- **The benchmark moves to the repo root**, which supersedes success criterion
  1's wording ("a `benchmark/` suite in this task folder"). The criterion is
  unchanged; only the path is. **It is not done until `sandbox.sh build tree`
  is inspected and `TREE.txt` contains no `benchmark/` path** - a wrong
  exclusion ships the answer key inside `blind`'s image and fails silently.
- **`CONVENTIONS.md` at the repo root carries a `## Not yet true` section.**
  Four rules are normative and violated by the tree until late in the epic (80
  preludes, 36 imports, 84 system sets, 28 module docs). Without that section
  it is `AGENTS.md`'s failure mode again, and every agent working L1-L11 will
  "helpfully" fix preludes inside unrelated diffs. **Deleting it is the epic's
  last commit.**
- **`nova_editor` enters scope.** Five defects in 2,378 LOC - the worst defect
  density in the workspace - and it was not on the original list at all.
- **Deletion count reports two numbers, not one.** CONVENTIONS rule 1 *adds* 28
  module docs and nets against criterion #2.
- **F57's fix regenerates `assets/base/**/*.content.ron`** via the builders plus
  `content -- gen`, never a hand-edit. It gets its own commit so the generated
  churn does not hide a real diff.
- **Tests are not a lane.** Owner's explicit instruction. The per-lane
  verification recorded in `plan/` is the evidence each lane needs to land, not
  a coverage push.
- **F84** (`proc-macro-error2` future-incompatibility) needs its own tatr task.
  Transitive dependency, breaks on a rustc bump, `-D warnings` does not cover
  it.

## Still open

- **The L7 escape hatch.** F17 and F28 are player-visible bugs waiting on owner
  review time only because their fix happens to be an extraction. If
  ratification drags, land the unit conversion and the shrink clamp in place
  during L4's window. Decidable once the ratification timeline is known.
- **The benchmark harness has never carried a real run.** Ratification, a
  review of the harness code, and a smoke run all precede the baseline.
  `plan/lane02.md` lists four review targets; the sharpest is that the
  persona-filter rule is implemented twice, in `make-papers.py:151` and
  `grade.sh:59-66`, and nothing fails loudly if they drift.
