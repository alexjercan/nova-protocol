# Decision: category run policy is two booleans in a code table, and one capture window

- DATE: 20260804-093855
- STATUS: ACCEPTED
- TASK: 20260804-093855
- TAGS: decision, tooling, examples, testing

## Context

Probe's run policy was two unrelated mechanisms: a `category == "perf"` test
that chose the fps capture window, and a hand-listed `fps_exempt` array in
`[package.metadata.nova_probe]` that let a single example opt out of the
frame-time pass. Neither said what a CATEGORY promises, so `screenshots/`
(judged by PNGs and human eyes) and `sections/` (a correctness curriculum)
were indistinguishable to probe. The roster spike (`20260804-003244`) settled
five categories; this task gives each an explicit contract and resolves
probe's behavior from it.

## Decision

Four decisions, one contract. The category is the unit: it says what its
examples prove, and probe resolves what to run from that.

### `CategoryPolicy` carries two booleans, not three

**Decision.** `CategoryPolicy { probed: bool, frame_time: bool }`.

The NOTES draft proposed `{ correctness, frame_time, in_all }`. Against the
five settled rows those three collapse to two:

| Category | correctness | frame_time | in_all |
|-|-|-|-|
| `sections/` | yes | no | yes |
| `systems/` | yes | no | yes |
| `stress/` | yes | yes | yes |
| `ui/` | yes | no | yes |
| `screenshots/` | no | no | no |

`correctness` and `in_all` are the same column. There is no row - and no
requirement in this sprint - for a category that runs correctness but is
excluded from `--all`, or that is in `--all` but runs no correctness pass.
Per the concept budget, a field with no caller and no invariant is deferred:
one boolean, `probed`, expresses both, and `frame_time` is the only genuine
second axis.

**Consequence, and the reason this is load-bearing.** With `probed` gating
BOTH `--all` and bare category expansion, `probe run screenshots` must ERROR
("category `screenshots/` is not a probe target") rather than expand to an
empty run. That is the honest reading of "screenshots/ leaves probe's scope",
and it is a user-visible CLI behavior that a three-field design could have
fudged by leaving `screenshots` expandable-but-inert.

If a later category ever needs "runs under an explicit spec but not under
`--all`", split `probed` then, with that caller in hand.

### The policy table lives in `catalog.rs`, not in `Cargo.toml` metadata

**Decision.** Carried forward from NOTES unchanged.

A second `[package.metadata.nova_probe]` parser is precisely the thing this
task exists to delete. The category strings become an API either way; putting
the table in code means a new category is a compile-time edit next to
`CatalogExample`, and `every_category_has_a_probe_policy` turns a missing row
into a red test rather than a silent default.

### `gameplay` and `perf` get transitional policy rows

**Decision.** Ship rows for both, marked `# remove with <task-id>`.

This task moves no files, so both directories exist on disk when it lands.
Without rows they would hit the unknown-category default and trip
`every_category_has_a_probe_policy`. The alternative - doing the directory
moves here - would collide with `093910` / `093934` / `094006`, which own the
content and must edit `tests/examples_smoke.rs` atomically with each move.

Accepted cost: `probe run gameplay --fps` loses its frame-time pass during
transit. See TASK.md "Transitional behavior, in the open".

### ...and it is a `const` slice, not a bare `match`

**Decision.** `CATEGORY_POLICIES: &[(&str, CategoryPolicy)]`, with
`category_policy()` a lookup over it.

Refines the NOTES decision above, whose point stands: the table is CODE, next
to `CatalogExample`, not a second `[package.metadata.nova_probe]` parser. A
`match` cannot be enumerated, and `every_category_has_a_probe_policy` has to
ask "is there a row for this category?" - which a `match` can only answer by
comparing against its own fallback, i.e. by asserting a category is not
accidentally identical to the default. A slice makes the gate say what it
means.

### One fps capture window, not a per-category default

**Decision.** `resolve_fps_window()` loses its `category` parameter and
always resolves the capture crate's 180/900 baseline window (operator
`NOVA_PERF_*` still override). The short 60/240 non-`perf/` window
(`NON_PERF_WARMUP`/`NON_PERF_FRAMES`) is deleted.

**Correction to Step 2**, which said `resolve_fps_window`'s `if category ==
"perf"` becomes `category_policy(category).frame_time`. Written literally that
branch is UNREACHABLE: `fps_window_and_deadline_env` is only called inside the
fps pass, and the fps pass now only runs when `frame_time` is true - so the
"else" arm serves exactly the categories that no longer capture at all. The
step would have shipped a dead branch plus a test pinning dead behavior.

Keeping one window is also the better end state on its own terms: the
categories that capture exist to be compared against a baseline, and a
different window would make their numbers incomparable with the sweep's. The
sweep path is untouched - it never used this function.

## Alternatives considered

- **A third `CategoryPolicy` field carrying the exclusion reason string.**
  Rejected: it reopens the two-vs-three-field question for text that is
  derivable from the category name. Both reasons ("carries no frame-time
  pass", "is not a probe target") are formatted at their consumer.
- **Doing the directory moves here**, so no transitional rows would be
  needed. Rejected: it collides with `20260804-093910` / `093934` / `094006`,
  which own the content and must edit `tests/examples_smoke.rs` atomically
  with each move; doing it here would land a red tree.
- **Keeping `screenshots` expandable-but-inert** (`probe run screenshots`
  succeeds with nothing to run). Rejected: a silent no-op reads as a pass.
- **Keeping the short 60/240 non-`perf/` capture window** as Step 2 wrote it.
  Rejected as unreachable; see above.

## Consequences

- `probe run screenshots` is now an ERROR, and `--all` records the category
  as excluded rather than running its members. User-visible CLI change.
- `--fps` skips the frame-time pass for every category except `stress/` (and
  `perf/` while it transits) - including `sections/` and `ui/`, which used to
  get the short window. Each skipped run records WHY, in `probe-run.json`
  (`fps_skipped`) and in the report's Performance section.
- The `checks.json` / `probe-run.json` field `fps_exempt` is renamed
  `fps_skipped`, same `Option<String>` shape, no compatibility shim: a
  pre-rename manifest reads back with `fps_skipped: None`.
- Adding a category is now a compile-time edit in
  `nova_probe::CATEGORY_POLICIES`; `every_category_has_a_probe_policy` fails a
  category that skips it.
- The `Cargo.toml` `fps_exempt` key survives this task as an inert orphan,
  marked as such, and is deleted by `20260804-093910` / `094006`.

## The per-EXAMPLE axis has no live member (review round 1)

`NOT_PROBED` has exactly one production entry, `render_scale_shot`, and it
lives in `screenshots/` - which the category gate now excludes wholesale,
BEFORE the per-example check runs. So under `--all` and category expansion the
per-EXAMPLE axis is currently unreachable, and the unit tests had to retarget
`EXCLUDED` to `playable` to keep the two axes separable. DECISION's
"`NOT_PROBED` stays - it is per-EXAMPLE, an orthogonal axis" is true in shape
but no longer true of any real example.

**Left standing anyway.** The entry is still load-bearing on a path the
category gate does not cover: an explicit `probe run render_scale_shot`
resolves, and `sweep.rs:48-51` prints its reason as a note. Deleting it would
let that command start a real-GPU pixel capture under Xvfb with no warning at
all. Whether the axis survives belongs to `20260804-093910`, which owns
`screenshots/`; if that task settles the directory such that no example ever
needs a per-example opt-out, `NOT_PROBED` and its branch go with it.

## `excluded` entries name their axis by shape (review round 1)

`AllManifest::excluded` now carries both axes, but its serialized key is
`example`. Rather than a second breaking schema change (the `fps_exempt ->
fps_skipped` rename is already one), a category entry is recorded WITH the
contract's trailing slash - `screenshots/`, never `screenshots`. That is how
`Cargo.toml`, `development.md` and every reason string already write a
category, it cannot collide with an example name, and it makes the value
self-describing in `index.json`, the HTML list and the terminal line without
touching the shape. Rejected: renaming the key to `name` (schema churn beyond
this task), and adding an explicit axis field (a third concept for something
the existing naming convention already encodes).
