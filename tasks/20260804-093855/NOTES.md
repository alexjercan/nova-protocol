# Notes: Example categories: write the contract and resolve probe run policy from it

Goal in one line: make the category a DECLARED contract with a run policy
attached, so probe stops asking "is this string `perf`?" and stops consulting a
hand-maintained exemption list.

## What changes

Before: a category is whatever path segment an example happens to sit under.
Probe derives behavior from it in exactly one branch (`category == "perf"`
picks the long fps window), and the "does this run fps at all" question is
answered by an unrelated mechanism - a hand-listed `fps_exempt = ["broadside"]`
in `Cargo.toml`. Nothing states what a category means, so nothing can be wrong.

After: five categories, each with a written contract and a declared run policy.
`probe run <category>` and `probe run --all` select passes from the policy.
`screenshots/` leaves probe's scope entirely. `fps_exempt` is deleted as an
input - `stress/` runs fps, nothing else does.

| Category | Proves | Correctness pass | Frame-time pass | In `--all` |
|-|-|-|-|-|
| `sections/` | one section family, deeply | yes | no | yes |
| `systems/` | cross-cutting systems on code-built fixtures | yes | no | yes |
| `stress/` | scale + a frame-time window | yes | yes | yes |
| `ui/` | UI surfaces driven by pointer input | yes | no | yes |
| `screenshots/` | image production only | no | no | no |

## Surfaces

| File | Why |
|-|-|
| `Cargo.toml` | The catalog is the single source of truth (`autoexamples = false`). The contract lands as the catalog's section comments; `[package.metadata.nova_probe] fps_exempt` (:34-35) goes away. |
| `crates/nova_probe/src/catalog.rs` | Owns `CatalogExample { name, path, category }`, `categories()`, and `parse_fps_exempt`/`load_fps_exempt` (:127-176). The policy table lands here; the two `fps_exempt` parsers and their four unit tests are deleted. |
| `crates/nova_probe/src/bin/probe/native/env.rs` | `example_fps_policy` (:33) and `resolve_fps_window` (:65, `if category == "perf"`) - both become policy lookups. |
| `crates/nova_probe/src/bin/probe/native/spec.rs` | `NOT_PROBED` (:8) and `resolve_spec`. `screenshots/` exclusion from `--all` belongs here, as a policy consequence rather than seven more `NOT_PROBED` rows. |
| `crates/nova_probe/src/bin/probe/native/run.rs` | Threads `fps_exempt: Option<String>` through `passes_total` (:374), `armed_fps` (:341) and the manifest. Source changes; shape can stay. |
| `crates/nova_probe/src/run_report/manifest.rs` | `RunManifest.fps_exempt: Option<String>` (:34) is a SERIALIZED checks.json field, also rendered by `html.rs:181-193`. Not just a Cargo.toml line - see open questions. |
| `tests/examples_smoke.rs` | `SECTIONS:32`, `GAMEPLAY:43`, `UI:47`, `SCREENSHOTS:51`, `NOT_SMOKED:78`, the four `*_reach_playing_without_panic` tests, and `catalog_matches_disk:109`. The drift gate; every later task's rename is atomic with an edit here. |
| `web/src/wiki/dev/*.md` (10 files) | Name `gameplay/`, `examples/perf`, `perf_baseline` or `broadside`. Sweep per `keeping-docs-in-sync.md`. |

## Data and interfaces

New, in `nova_probe::catalog`:

```rust
/// What probe does with a category. Declared once; every pass decision
/// resolves from it.
pub struct CategoryPolicy {
    pub correctness: bool,
    pub frame_time: bool,
    pub in_all: bool,
}

pub fn category_policy(category: &str) -> CategoryPolicy;
```

Changed:

```rust
// env.rs - was (String, Option<String>) from the exempt list
pub(crate) fn example_fps_policy(root: &Path, example: &str)
    -> (String, Option<String>);   // reason now comes from the policy

// env.rs - was `if category == "perf"`
fn resolve_fps_window(category: &str) -> (u32, u32);
```

Deleted: `parse_fps_exempt`, `load_fps_exempt` and their re-exports in
`lib.rs:147`.

## Sketches

Illustrative only.

```diff
-fn resolve_fps_window(category: &str) -> (u32, u32) {
-    let (default_warmup, default_frames) = if category == "perf" {
+fn resolve_fps_window(category: &str) -> (u32, u32) {
+    let (default_warmup, default_frames) = if category_policy(category).frame_time {
```

```diff
 const SECTIONS: &[&str] = &[...];
-const GAMEPLAY: &[&str] = &["scenario", "playable", "broadside", "lifeline"];
+const SYSTEMS: &[&str] = &["scenario_grammar", "player_path", "outcomes"];
+const STRESS: &[&str] = &["scene_baseline", "many_bodies", ...];
```

## Shape

```
Cargo.toml [[example]] catalog          contract prose
   |  path = examples/<cat>/<f>.rs         (catalog comments + dev wiki)
   v
parse_example_catalog -> CatalogExample{name, path, category}
   |                                  |
   |                                  +--> category_policy(category)  <-- NEW
   |                                          |  correctness / frame_time / in_all
   v                                          v
tests/examples_smoke.rs               spec.rs  (--all, category expand)
  catalog_matches_disk                env.rs   (fps window, exempt reason)
  (disk == catalog == smoke lists)    run.rs   (passes_total, armed_fps)
```

## Consequences and open questions

- This is the sequencing spine: every other task in the sprint renames a
  directory, and each such rename must edit `tests/examples_smoke.rs` in the
  SAME commit or a bare `cargo test` goes red on `catalog_matches_disk`. That
  is a per-commit constraint, not a per-task one.
- The category strings become an API. A sixth category later means adding a
  policy row, not just a directory - deliberate, and the point of the change.
- RESOLVED (owner principle, 2026-08-04): do not add tests for our tests -
  examples ARE tests. `catalog_examples_satisfy_their_category_contract` is
  dropped entirely, not weakened. It would have inspected example source to
  judge whether a run asserts enough, which is exactly the thing the principle
  rules out. The contract is prose (catalog comments + dev wiki), enforced by
  review. `category_run_policy_selects_passes_per_category` stays - it tests
  `nova_probe`, production code. `catalog_matches_disk` stays - it is a
  build-integrity gate (with `autoexamples = false` an uncataloged example does
  not compile), and it pins names and paths, never behavior.
- RESOLVED (owner, 2026-08-04): `RunManifest.fps_exempt` is renamed
  `fps_skipped` and stays an `Option<String>` reason. It already WAS a reason
  string - only the source changes, from the exempt list to the category
  policy. No schema shape change, no compatibility shim. `fps_exempt` then
  splits cleanly by owner: the Cargo.toml key goes to whichever of 093910 /
  094006 lands second, the parsers and the checks.json field are this task's.
- OPEN: `screenshots/` leaving `--all` means the smoke tests become their only
  automated exercise. `screenshots_reach_playing_without_panic` must survive
  the reduction in `20260804-093910`, or those eight runs go unexercised.
- Chosen assumption, flag if wrong: the policy table is a `match` in
  `catalog.rs`, not new `Cargo.toml` metadata. A second metadata parser is the
  thing this task exists to delete.
