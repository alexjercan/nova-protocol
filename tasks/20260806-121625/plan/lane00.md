# L0 - Fix the map, close the CI gaps

**Baseline: BLOCKS - lands BEFORE it.** Edits docs and CI config, so it changes
what the benchmark measures. Findings: **F79, F80**.

**Depends on:** nothing. This is the first commit of the epic.

## Order inside the lane

Two hard sequences, everything else free:

1. F79 (`#[cfg(feature = "debug")]` the 11 dead example items) **before** the
   default-features CI job - the job fails on those 11 otherwise.
2. The `report.rs` wasm gate **before** the wasm CI job - the gate clears all
   7 warnings the job would otherwise report.

## Docs

### `AGENTS.md` - three corrections

| Site | Change |
| --- | --- |
| the `nova_modding` row | wrong on 3 of 4 items. Bundle merge, portal client and downloads all live in `nova_assets`. Rewrite the row to name what `nova_modding` actually owns |
| `AGENTS.md:102` | "Cross-subsystem communication through `nova_events`, not direct coupling" reads as a general architecture mandate. It is the scenario/modding vocabulary (`nova_events/src/lib.rs:1-9`). Reword to say so - it has already misled one audit into flagging 46 healthy files |
| the crate table | no signal that `nova_gameplay` is half the workspace. Add the LOC share |

### `CONVENTIONS.md` at the repo root - NEW

A **rewrite** of `../CONVENTIONS.md` (648 lines, the evidence record), not a
copy. Target 120-150 lines, one `##` per rule, modelled on
`~/personal/scufris/CONVENTIONS.md`.

```
NEW  CONVENTIONS.md

## <rule, as an imperative heading>     x12
    one real in-repo snippet
    one or two sentences of rationale

## Tool traps
    wildcard_imports, redundant_pub_crate, needless_pass_by_value,
    pedantic/nursery - why they are wrong for a Bevy codebase

## Not yet true            <- MUST NOT be dropped
    rule 3  - 80 open sites  - closed by L5, L7, L8, L9, L10
    rule 4  - 36 open sites  - with rule 3
    rule 10 - 84 open sites  - L9, per seam
    rule 1  - 28 open sites  - L5
```

Everything dropped (violation counts, counter-example file lists, the
`RULED 2026-08-07` annotations, the lane table) stays in `../CONVENTIONS.md`.

**Deleting `## Not yet true` is the epic's last commit.** Its emptiness is the
proof the conventions are real. Without it, every agent working L1-L11 will
"helpfully" fix preludes inside unrelated diffs.

`AGENTS.md`'s `## Code rules` section shrinks to a pointer at this file.

## CI - `.github/workflows/ci.yaml`

```yaml
# CHANGE  ci.yaml:70 - the clippy step
- run: cargo clippy --workspace --all-targets -- -D warnings
#                                                 ^^^^^^^^^^ added
#   FREE TODAY: the tree produces 0 warnings at this configuration, measured.

# NEW job - default features
- run: cargo check --workspace --all-targets
#   Requires F79 to have landed.

# NEW job - wasm
- run: cargo check --workspace --target wasm32-unknown-unknown
#   Requires the report.rs gate below.
```

## Source

### The wasm gate

```rust
// CHANGE  crates/nova_probe/src/lib.rs  (around :82-109, where the siblings are)
#[cfg(not(target_arch = "wasm32"))]
pub mod report;
//  ^ report.rs is the only host-side module NOT behind this cfg. Adding it
//    clears all 7 wasm warnings; no code inside report.rs changes.
```

### F79 - 11 dead default-feature items

Eight example files, 11 items, each existing **only** to serve debug-feature
code. `#[cfg(feature = "debug")]` on each, no body changes:

| File | Items |
| --- | --- |
| `examples/sections/hull_section.rs` | `:535`, `:547`, `:563` |
| `examples/sections/torpedo_section.rs` | `:69`, `:349` |
| `examples/sections/controller_section.rs` | `:64` |
| `examples/screenshots/screenshot_combat.rs` | `:128`, `:134` |
| `examples/screenshots/screenshot_sections.rs` | `:199` |
| `examples/systems/player_path.rs` | `:55` |
| `examples/sections/many_sections.rs` | `:37` |

**Write down what you learn about `--features debug` while doing this.** F52
(in L5) is the same investigation from the other end - which crates force the
feature on and what is orphaned without it. Done twice otherwise.

### F80 - `#[allow]` -> `#[expect]`, 38 sites

```rust
// CHANGE  38 sites workspace-wide
- #[allow(clippy::type_complexity)]
+ #[expect(clippy::type_complexity, reason = "<the actual reason>")]
```

**All 38 are currently dead.** `Cargo.toml:314-316` sets
`type_complexity = "allow"` workspace-wide and all 17 manifests carry
`[lints] workspace = true`, so they suppress a lint that cannot fire. The
conversion still works: `#[expect]` overrides the workspace `allow` at the
site, proven by the 4 existing `#[expect(clippy::type_complexity, ...)]` sites
coexisting with it at 0 warnings. Model sites: `hints.rs:200`,
`keybind_dock.rs:569,737,790`.

Two are already known stale and should simply be deleted rather than converted:
`ammo_readout.rs:325`, `ammo_readout.rs:510`.

## Move the benchmark to the repo root - RULED 2026-08-07

```
MOVE    tasks/20260806-121625/benchmark/  ->  <root>/benchmark/
CHANGE  benchmark/sandbox.sh:38-42  repo_files() - one named exclusion list
        covering `tasks/` AND `benchmark/`, instead of the hardcoded `^tasks/`
NEW     .gitignore  benchmark/results/  (keep aggregate.json, aggregate.csv,
        report.html; the transcripts are large)
```

**Lands here because it must precede the baseline** - it changes what
`TREE.txt` contains, and `tree` is a persona. **Not done until
`./sandbox.sh build tree` is inspected and no `benchmark/` path appears.** A
wrong exclusion ships `keys/tier1.json` inside `blind`'s image, which fails
silently: it just answers most of tier 1 for free. `repo_files()` is the single
chokepoint - the tar copy and `TREE.txt` both go through it.

## Why these five items are one lane

Individually each is a one-line change. Together they turn the two things a
large refactor silently produces into CI-reported failures:

| Produced by a refactor | Caught by |
| --- | --- |
| unused imports, dead code | `-D warnings` |
| stale suppressions | `unfulfilled_lint_expectations`, once F80 lands |

`-D warnings` without F80 leaves the second class unaudited. F80 without
`-D warnings` leaves it reported but not blocking.

## Verified by

CI itself - the new jobs are the verification. Plus a re-read of `AGENTS.md`
against `../notes/02-workspace-map.md`.
