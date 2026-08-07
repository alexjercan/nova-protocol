# Handoff - draft the Tier 1 benchmark question set

## Your job

Draft the full ~30-question Tier 1 set in
`tasks/20260806-121625/benchmark/tier1.json`, replacing the 6 samples already
there. Then draft `benchmark/tier2.md`, `benchmark/modder.md` and
`benchmark/build-sandboxes.sh`.

The owner ratifies everything before any run. Draft, do not run.

## Read first

| File | Why |
| --- | --- |
| `tasks/20260806-121625/NOTES.md` | the problem, the confirmed decisions, the ranked plan |
| `tasks/20260806-121625/benchmark/README.md` | the protocol - personas, tiers, measurement, sandboxes |
| `tasks/20260806-121625/benchmark/tier1.json` | 6 samples. Match this format exactly |
| `tasks/20260806-121625/notes/*.md` | all audit evidence, with file:line citations |
| `AGENTS.md` | repo conventions. Note: its crate table is stale, see `notes/02` |

## What the benchmark measures

Whether the refactor made the tree navigable, or only moved code. Same questions
run before and after, across six personas. The primary metric is **tool calls to
a correct answer**, not correctness - an agent can be right both times while the
cost drops from 12 calls to 2.

A question earns its place only if it discriminates. Ask yourself: what does a
wrong answer at baseline tell us, and what would change it? If the answer is
"nothing in this epic", drop the question.

## Rules for each question

1. **Every `expect` carries a `file:line` citation** verified against the
   current tree. Do not cite from `notes/*.md` alone - open the file and confirm
   the path and line still hold. The notes are from 2026-08-07 at HEAD
   `4a8b55aa`.
2. **`why_this_question` states what a baseline failure would prove.** If you
   cannot write that sentence, the question is filler.
3. **`notes` defines partial credit.** "Right crate, wrong module" is `partial`;
   be explicit about which answers are `wrong` rather than `partial`.
4. **Answerable by every persona in its list.** A `tree` persona sees filenames
   only - a question needing a line number is unfair to it and should exclude it
   via the `personas` field.
5. **No question whose answer is only in `tasks/`.** That folder is excluded
   from every sandbox and will describe the refactor after the fact.

## Coverage targets

Roughly 30 questions, distributed so no single seam dominates:

| Area | Count | Focus |
| --- | --- | --- |
| NOVA OS / `hud/` seam | 5 | the 14.3k runtime living under a folder named `hud`; its three-crate ownership smear |
| `nova_probe` | 5 | the child/parent process boundary; where collect vs evaluate vs report live |
| `nova_assets` / `nova_scenario` | 5 | the authoring toolchain split across two crates; base content living in an asset crate |
| workspace / crate-level | 5 | "which crate owns X" - targets the stale crate table and the merge candidates |
| UI layer | 3 | is `nova_ui` the shared layer or bypassed; where duplication lives |
| composition / features | 3 | plugin assembly order, what `debug` actually gates, what `render: bool` claims |
| **controls** | 4 | boundaries that are currently **correct** and must stay correct |

The controls are not optional. Without questions the refactor should *not*
change, a flat score is unreadable - you cannot distinguish "no improvement"
from "broke something that worked". `t1-005` (`nova_ui/src/theme.rs`) is the
model.

## Question shapes that work

- "Which module owns X?" - the base case.
- "Where would you add Y?" - tests the model, not just recall.
- "What does the folder `Z` contain?" - aimed squarely at names that lie.
- "Which process writes file W?" - two-part; the second half exposes hidden
  architecture. See `t1-004`.
- "Name every file that decides X." - catches agents that stop at the first
  answer. See `t1-006`.

Avoid: questions answerable by a single unambiguous grep for a unique string
(they measure grep, not structure), and questions with several defensible
answers (they measure the grader).

## Tier 2 - `tier2.md`

Three owner-selected tasks. Write the prompt for each plus the shared rubric
(the rubric is already specified in `benchmark/README.md` - reproduce it, do not
redesign it).

1. Add a new ship section type
2. Add a NOVA OS app
3. Add a new scenario action + event

Each prompt must ask for a `NOTES.md` - what the agent understands and what it
would change. **No code.** For each task, also write the ground-truth surface
list (every file a correct answer must name) so the reviewer agent can score
completeness. `AGENTS.md`'s Documentation table names the doc surfaces a change
must touch; use it.

## Modder task - `modder.md`

Brief for building a mod from `web/src/wiki/modding.md`,
`web/src/wiki/dev/guide-make-a-mod.md`, `web/src/wiki/dev/modding-ron.md`,
`web/src/wiki/dev/mod-portal.md` and the two worked examples in `webmods/`
(`gauntlet`, `the-ledger`).

Objective verdict, no grader: the mod is copied back and must pass
`cargo run -p nova_assets --bin content -- lint` and then load in a scenario.
Specify a mod small enough to be built from the wiki alone but large enough to
exercise more than one RON construct. Record any wiki gap the agent hits - those
are real doc bugs worth fixing regardless of this epic.

## `build-sandboxes.sh`

Builds `/tmp/nova-bench-<persona>-<run>/` for `docs`, `tree`, `rustdoc`,
`modder`. Per `benchmark/README.md`:

- `docs`: `AGENTS.md`, `README.md`, `CONVENTIONS.md`, `web/src/wiki/**`.
  **Exclude `tasks/`.**
- `tree`: one `TREE.txt`. Ask the owner whether to include per-directory line
  counts - this is an open question in the last handoff and is not yet settled.
- `rustdoc`: output of `nix develop --command cargo doc --workspace --no-deps`.
- `modder`: the 4 wiki files plus `webmods/` (skip `thumbnails/*.png`).

Also write the transcript audit as a script or a documented grep: sandbox
personas must not touch `nova-protocol/crates`, `/src/`, or `.rs`; `blind` must
not read `.md`. A hit invalidates the run.

## Constraints

- NixOS: every cargo command runs as `nix develop --command cargo ...`.
- Do not run `cargo test` or `cargo clippy` locally. CI owns both.
- ASCII punctuation only. No em dashes, smart quotes or typographic arrows.
- Do not create tatr tasks. Do not start a branch or worktree.
- Do not run the benchmark. Baseline waits on the AGENTS.md fix and the CI gaps
  (`-D warnings`, default-features job, wasm job) - see `NOTES.md` idea 1.

## Done when

`tier1.json` (~30 questions, every citation verified against the tree),
`tier2.md`, `modder.md` and `build-sandboxes.sh` exist, and you have reported to
the owner: the coverage distribution you actually hit, any question you were
unsure about, and any citation in `notes/*.md` that no longer matches the tree.
