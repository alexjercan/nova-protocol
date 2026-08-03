# Notes: Rebuild the example fleet per category contract

## What changes

Before: 5 category dirs (`sections/`, `gameplay/`, `ui/`, `screenshots/`,
`perf/`), 22 cataloged examples, and one implicit policy - `perf` gets the long
frame-time window, everything else gets the short one, and `broadside` is
hand-listed in `fps_exempt` because a narrative one-shot cannot fill a window.
`probe run --all` therefore also runs the screenshot producers, which prove
nothing and only exist to write PNGs, and `render_scale_shot` needs a
hand-written `NOT_PROBED` entry to stay out. Category membership is a directory
convention with no stated meaning.

After: each category has a written contract and probe derives its run policy
from the category, not from per-example exception lists. The examples are
rebuilt against their contract on the predicate driver from `20260802-120025`:
`sections/` runs get multiple scenes/rounds and predicate-gated assertions,
`gameplay/` runs get more beats and a declared loop point that fills the fps
window, `ui/` runs assert the live UI tree, `screenshots/` runs shrink to
"enter scene, wait for predicate, shoot, exit".

Player-visible behavior is unchanged; what changes is what a green
`probe run <category>` means.

## Surfaces

| File | Why |
| --- | --- |
| `Cargo.toml` (root) | The catalog and `[package.metadata.nova_probe]`. Gains the per-category policy table; `fps_exempt` folds into it. |
| `crates/nova_probe/src/catalog.rs` | Parses the catalog + `fps_exempt`. Gains category-policy parsing; still fail-closed on the catalog, fail-open on config. |
| `crates/nova_probe/src/bin/probe/native/env.rs` | `example_fps_policy` / `resolve_fps_window` - the `perf`-vs-rest split lives here. |
| `crates/nova_probe/src/bin/probe/native/spec.rs` | `--all` / category expansion and the `NOT_PROBED` list; `screenshots/` leaves via policy instead of a hand list. |
| `crates/nova_probe/src/bin/probe/native/run.rs` | Chooses which passes run (`passes_total`, clean/trace/fps). Policy decides, not `fps_exempt.is_some()`. |
| `examples/sections/*.rs` (7) | Rebuilt: multi-scene, multi-round, predicate assertions. |
| `examples/gameplay/*.rs` (4) | Rebuilt: longer paths, loop point, markers/invariants per beat. |
| `examples/ui/*.rs` (5) | Rebuilt: live-tree assertions on panes/widgets/reconcilers. |
| `examples/screenshots/*.rs` (8) | Reduced to producers; probe enrollment and assertions removed. |
| `tests/` (root, `catalog_matches_disk`) | Drift test; extended to enforce the contract. |
| `docs/` dev wiki example page | Where the contract is written down for humans. |

## Data and interfaces

```toml
# root Cargo.toml - one table per category, replacing fps_exempt
[package.metadata.nova_probe.categories]
sections    = { correctness = true,  fps = false, in_all = true }
gameplay    = { correctness = true,  fps = true,  in_all = true }
ui          = { correctness = true,  fps = false, in_all = true }
perf        = { correctness = false, fps = true,  in_all = true }
screenshots = { correctness = false, fps = false, in_all = false }
```

```rust
// nova_probe::catalog
pub struct CategoryPolicy { pub correctness: bool, pub fps: bool, pub in_all: bool }
pub fn parse_category_policies(manifest: &str) -> HashMap<String, CategoryPolicy>;
pub fn load_category_policies(root: &Path) -> HashMap<String, CategoryPolicy>;
// default for an undeclared category: correctness only (today's behavior)
```

Example-side, each rebuilt run declares its beats through the predicate driver
(see `20260802-120025` NOTES) and, for `gameplay/`, a loop point instead of the
current `loop_while_pending` + `AutopilotLoop` reader + `capture_reloading`
poll.

## Sketches

Illustrative only.

```diff
-let (category, fps_exempt) = example_fps_policy(&root, &opts.example);
+let policy = category_policy(&root, &opts.example);   // correctness / fps / in_all
```

```diff
 // examples/sections/hull_section.rs - round 2 in a second layout
+.step("reload as 3-hull spine").on_enter(load_layout(Layout::Spine3)).until(scenario_loaded()).add()
+.step("round 2: sustained fire").each(hold_fire).until(section_destroyed("hull_2")).add()
+.step("assert integrity after 2 rounds").on_enter(assert_integrity_ledger).until(frames(1)).add()
```

## Shape

```
root Cargo.toml
  [[example]] catalog  --------> nova_probe::catalog ------> spec resolution
  [.nova_probe.categories] ----> CategoryPolicy       \        (--all honors in_all)
                                                       \
                                                        -> run passes
                                                           clean | trace | fps

examples/
  sections/    correctness, no fps   |  multi-scene, multi-round, predicate asserts
  gameplay/    correctness + fps     |  full player path, loop point
  ui/          correctness, no fps   |  live UI-tree asserts
  perf/        fps                   |  baseline scenes
  screenshots/ (not probed)          |  enter -> wait -> shoot -> exit
                     |
                     +--> scripts/gen-web-screenshots.py (packaging, python only)
```

## Consequences and open questions

- Cost: this is the largest task of the sprint - 22 examples, ~7k lines, all
  of which must be RUN under Xvfb (`cargo check` misses duplicate-component
  panics and UI ghosting). Expect it to split into per-category commits.
- Removing screenshot examples from `--all` reduces what CI exercises. Their
  correctness value today is "it did not crash"; that is real but small, and
  the packaging script surfaces a broken producer as a missing PNG.
- `render_scale_shot`'s `NOT_PROBED` entry can go away only if the category
  policy covers it - it is a `screenshots/` member, so it does.
- Open: whether `sections/` runs reload scenes in-process (a second
  `LoadScenario` trigger) or restart the app per round. In-process is the point
  of the loop point but exercises teardown paths that have crashed before
  (20260720-014142).
- Open: how `ui/` asserts "nothing ghosts" generically - the known pattern is
  an `Added<Marker>` override with matching siblings on a live tree; whether
  that generalizes to a reusable assertion or stays per-example is unknown.
- Resolved 2026-08-04: `nova_os_rtt_poc` is retired (the RTT pipeline shipped)
  and its coverage returns as an RTT element test in the `ui/` fleet. The three
  `*_poc.html` files are NOT prototypes - webpack copies them into the site and
  `web/tests/theme.test.ts` reads one as the token source for both
  `nova_ui/src/theme.rs` and `web/src/style.css` - so they move to `web/design/`
  in `20260804-003301`.
- Resolved 2026-08-04: the fleet roster (keep / retire / rewrite / add, and
  which new test-only scenarios to build) is decided by the spike
  `20260804-003244` before this task rewrites anything.
- Resolved 2026-08-04: mainline story scenarios only need to prove the game
  plays normally and yield frame data; a real win/lose outcome is the job of
  purpose-built test scenarios.
- Deferred: screenshot capture/packaging stays python (`20260802-120045`
  WONTDO). This task only guarantees the producers exist and are consistent.
