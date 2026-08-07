# Handoff: absorb the code review into the notes

You are picking up the third of three parallel understanding workstreams for
tatr task `20260806-121625` in `/home/alex/personal/nova-protocol`. The other
two are `question-set-prompt.md` (benchmark) and `conventions-prompt.md`
(Rust style). They do not block you and you do not block them.

**Your job is documentation, not code.** Make no source edits, open no branch,
fix no bug. The output is a notes tree that a planning agent can turn into a
task without re-reading the codebase.

## What already happened

A six-agent code review plus a clippy audit ran on 2026-08-07 against `master`
@ `4a8b55aa`. The raw findings are already written up:

| File | Covers |
| --- | --- |
| `notes/09-clippy-and-lints.md` | clippy at three configurations, measured CI blind spots |
| `notes/10-review-hud-nova-os.md` | `nova_gameplay/src/hud/**`, `src/input/**` |
| `notes/11-review-assets-scenario.md` | nova_assets, nova_scenario, nova_modding, nova_mod_format |
| `notes/12-review-ui-layer.md` | nova_ui, nova_menu, nova_editor, nova_os |
| `notes/13-review-cross-cutting.md` | workspace-wide pattern sweep with counts |
| `notes/14-review-flight-sections.md` | nova_gameplay flight, physics, camera, sections, mesh, audio |

| `notes/15-review-probe.md` | nova_probe, nova_autopilot, nova_events, examples |

**`notes/15` is the one to read most carefully.** `nova_probe` is the CI gate,
and the review found four ways the gate is already blind - three of which fail
OPEN (a run can verdict OK when it should FAIL). Those fixes gate the entire
epic's verification story, so they belong at or near the front of lane one.

`notes/00-08` are the pre-review understanding notes. Several already carry
corrections applied during the review; `05` and `08` were amended in place.

## Read first, in this order

1. `NOTES.md` - the problem statement, success criteria, constraints, ranked
   ideas. This is the file the planning phase reads.
2. `notes/00-index.md` - the map.
3. `notes/01-decisions.md` - every owner ruling, with verbatim quotes. **Treat
   these as settled.** If a review finding appears to contradict one, that is
   a question for the owner, not a licence to overrule it.
4. `notes/02` through `notes/08` - the pre-review picture.
5. `notes/09` through `notes/14` (and `15` if present) - the review.

## Rule 1: amend, never silently replace

The most valuable content in this tree is the record of claims that were
**measured and rejected**. A fresh agent that reads only conclusions will
re-derive the wrong version. Four of these exist so far:

| Claim | What measurement showed |
| --- | --- |
| "useless comments all over the code" | 83% why-comments, 0 commented-out code, 3 TODOs. Premise rejected; deletion redirected to three other targets |
| "nova_events is unused inside nova_gameplay, so the coupling doctrine is not real" | Wrong. nova_events is the scenario/modding vocabulary and is correctly used. AGENTS.md's wording is what misled the audit |
| "the never-compiled wasm paths have probably rotted" | All 14 crates type-check clean on wasm32. Prediction wrong |
| "seven `unreachable!()` are a refactor hazard" | Four are in `#[cfg(test)]`. Only `mesh/slice.rs:67` is production |

When the review contradicts an earlier note:

- Keep the original claim visible.
- Mark it: `**Corrected 2026-08-07**` or `**WITHDRAWN**`, with the evidence and
  the file:line that settled it.
- Never delete the original sentence and write the new one in its place.

`notes/05` and `notes/08` already show the format. Follow it.

## Rule 2: every finding carries a verified citation

Do not copy a `file:line` from a review note into a new note without opening
the file and confirming the line still says what the note claims. One audit
agent in this task already reported a path that did not exist
(`src/bin/probe/run_report/` instead of `src/run_report/`). Assume at least one
more such error is present in the corpus and that finding it is part of your job.

If a citation does not check out, say so explicitly rather than quietly
dropping the finding - a withdrawn finding is information.

## What to produce

### 1. Amend `notes/00-08` wherever the review changes them

Known cases, non-exhaustive - find the rest yourself:

- `02-workspace-map.md` - the review found 633 crate-local `pub` items
  (nova_gameplay 358, ~55% of its public surface) and zero genuinely dead
  items. That bears on the prelude and visibility discussion.
- `03-nova-gameplay.md` - the seam analysis predates six defect reports in the
  same code. Note which seams now carry known bugs.
- `06-ui-layer.md` - said three duplicated scroll clamps. There are two
  `max_*_scroll_y`, they agree with each other, and **both** carry a
  physical-vs-logical pixel unit bug. The `nova_editor` third site is unclamped
  and is a different defect. Also: the proposed `nova_ui::screen` extraction now
  fixes four defects, not just duplication.
- `07-comments-and-docs.md` - the cross-cutting sweep independently corroborated
  the "hygiene is better than it looks" conclusion from a different direction.
  Worth one cross-reference.
- `08-tests-ci-risk.md` - the risk register was written before the review.
  Re-rank it against what was actually found. `nova_editor` (5 bugs in 2,378
  LOC, 13 tests) was not on the risk register at all and should be.

### 2. Write `notes/16-findings-master.md`

The single ranked list a planner works from. One row per finding, deduplicated
across the six reports - several were found independently by two agents
(the terminal completion-ghost bug, the `unwrap`/panic sites in
`nova_editor/src/placement.rs`), and those deserve one row, not two.

Suggested columns: id, `file:line`, one-line defect, severity, confidence,
estimated blast radius (files touched), whether it is independent of the
structural refactor, and which lane it belongs to.

Rank by **expected harm**, not by severity label alone. A certain-confidence
data-loss bug outranks a certain-confidence cosmetic one; a speculative crash
does not outrank a confirmed silent-corruption path.

### 3. Write `notes/17-lanes.md`

The owner has decided: **one tatr task, all findings, split into lanes.**
That decision is made - do not re-open it or propose splitting the task.

Your job is to define the lanes so the work is landable in sequence. For each
lane give: name, what is in it, what it depends on, what verifies it, and a
rough size. Constraints that are already known:

- **`nova_probe` is itself the CI gate** (`cargo run -p nova_probe -- run --all`
  blocks CI). Its lane goes first and needs its own verification, because every
  other lane is verified by a gate that probe provides.
- **The AGENTS.md fix and `-D warnings` land before the benchmark baseline**,
  or the baseline measures a tree CI does not check and docs that lie.
- **Anything that moves a file, renames a symbol, or edits a doc blocks the
  benchmark baseline.** Behavior-only bug fixes do not. Mark each lane with
  which side of that line it falls on - this is the scheduling fact the plan
  depends on most.
- Several findings cluster naturally and are cheaper together than apart. Two
  are already identified: the atomic-write helper touches the same four files
  as the `Storage`-trait extraction, and the flight engine-spool extraction
  fixes a 16-line duplicate and the workspace's only real per-tick complexity
  bug in one edit. **Look for more of these.** Naming them is the highest-value
  thing this document does.

### 4. Update `notes/00-index.md`

Add every new file with a one-line hook. Keep the one-paragraph summary at the
top accurate.

### 5. Update `NOTES.md`

The ranked `## Ideas` section was written before the review. Re-rank it with
the review as evidence, and add any idea the review created that was not there
before. Two candidates:

- Converting 36 `#[allow(clippy::type_complexity)]` to
  `#[expect(..., reason = "...")]`, which makes suppression rot self-reporting
  via `unfulfilled_lint_expectations` at zero analysis cost. The codebase
  already uses `#[expect]` with a reason in 4 places, so this enforces an
  existing local convention.
- Deleting the two features the review proved are dead: the entire `Tween`
  subsystem (421 lines, 11 tests, **zero consumers workspace-wide**) and
  `StatusBarStore` (declared, `init_resource`d, never read or written).

Do not touch `## Problem Statement` or `## Constraints` - those are owner-confirmed.

## Cross-cutting patterns worth a paragraph each

The review found four patterns that recur across crates. Each is a
CONVENTIONS.md rule candidate with a real violation count, and each should be
handed to that workstream as well as recorded here:

1. **Stale `Local<T>`.** Four independent instances (`hud/nova_os/shell.rs:363`,
   `audio/cues.rs:99`, `audio/cues.rs:147` unpruned, plus the one already in
   the owner's memory as `mode-keyed-reconciler-just-spawned-override`).
   `Local<T>` is per-system and process-lifetime; any use tracking entity state
   is a latent bug the moment that entity respawns. The tree already contains
   both correct fixes: an `Added<Marker>` override (`shell.rs:288,320`) and an
   explicit prune (`mixing.rs:195`).
2. **Unguarded per-frame writes through `DerefMut`.** Writing `node.width` or
   `color.0` unconditionally marks the component changed regardless of value
   equality, forcing a UI relayout. At least five sites. `keybind_dock.rs` is
   the reference implementation and carries the explanatory comment.
3. **Code that lies about its guard.** `torpedo_section/bay.rs:112` (a
   `Without<>` filter that excludes nothing), `objectives.rs:123` (a system
   that can never run), `nova_ui/src/widget/panel.rs:112` (a `skin` parameter
   discarded as `_skin`), `nova_ui/src/status_bar.rs:238` (an entity that is
   never rendered). This is the owner's "dead and lying surface" deletion
   target, now with concrete instances.
4. **Unvalidated authored values reaching arithmetic.** `fire_rate` into
   `Duration::from_secs_f32` (panics), `ScatterObjectsConfig::count` into an
   uncapped spawn loop, unbounded recursion in two DSL decoders and the
   dependency walker. Mod content is untrusted input; a panic reachable from it
   is a defect, not an upheld invariant.

## Constraints

- ASCII punctuation only. No em dashes, smart quotes, typographic ellipses or
  unicode arrows. Applies to every file you write.
- Keep each notes file under ~500 lines. Split rather than let one grow.
- Do not run `cargo test` or `cargo clippy` - CI owns both. If you need a cargo
  command, it goes through `nix develop --command cargo`, and respect the
  `jobs = 4` cap in `.cargo/config.toml`.
- Do not create a tests task. The owner said: "tests as you said should be a
  separate task, I will see to it do not create it."
- Do not stage or commit. The owner commits task records.
- Never hand-edit `assets/base/**/*.content.ron`.

## Done when

- Every note in `00-08` that the review contradicts carries a dated, cited
  amendment, with the original claim still visible.
- `notes/16-findings-master.md` holds one deduplicated, ranked row per finding,
  every citation re-verified against the tree.
- `notes/17-lanes.md` defines the lanes with dependencies, verification, and
  the blocks-baseline / does-not-block-baseline marking.
- `notes/00-index.md` and `NOTES.md`'s `## Ideas` are current.
- Your final message states: how many findings survived verification, how many
  were withdrawn and why, and the single finding you would fix first if you
  could only fix one.
