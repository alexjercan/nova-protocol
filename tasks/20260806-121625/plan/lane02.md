# L2 - Build and baseline the benchmark

**This lane is not code.** It is the gate that makes L5-L10 provable rather
than churn. It has no findings.

**Depends on:** L0 only, so the delta is attributable to structure rather than
to a stale `AGENTS.md`. **Not** blocked by L1 - the probe gate and the
benchmark are independent, and L1 should run in parallel with the slowest part
of this lane, which is owner review time.

## State - 2026-08-07

The harness is **built and unit-verified. It has never been run.** No agent has
answered a paper; no baseline exists. What remains is owner ratification,
review, a placement decision, and the run itself.

| Piece | State |
| --- | --- |
| 30 tier 1 questions + answer key, citations re-verified against `89c049fd` | drafted, **awaiting ratification** |
| 3 tier 2 prompts + rubric + ground-truth surface lists | drafted, **awaiting ratification** |
| grading scale, both tiers: one number `0.00`-`1.00` | **owner ruling 2026-08-07**, replaces right/partial/wrong |
| tier 3 mod brief + pass criteria | drafted, **awaiting ratification** |
| Docker isolation, one image per persona | built, all six images verified |
| paper generation, run, grade, aggregate, report | written, exercised on synthetic data |
| end-to-end agent run | **not done** |

## Artifacts

```
tasks/20260806-121625/benchmark/
  README.md              the protocol
  make-papers.py         keys + task bodies -> papers/<paper>-<persona>.md
  sandbox.sh             build | list | inspect | clean the persona images
  run.sh                 one paper, one persona, one container
  grade.sh               grader agent; key mounted, no source tree
  aggregate.py           result tree -> aggregate.json + aggregate.csv
  report.py              aggregate.json -> self-contained report.html
  docker/                base image, persona image, entrypoint
  keys/                  tier1.json, tier2.md, tier3.md - never mounted to a respondent
  papers/                generated; papers/src/ holds the tier 2 and 3 task bodies
  results/<run>/         one directory per persona per paper
```

`questions.md` / `answers.md` / `baseline/results.md` from the first draft of
this plan do not exist and will not. Questions and answers live together in
`keys/tier1.json` so they cannot drift; papers are generated from it; results
are structured data plus a generated HTML report.

## The personas

| Persona | Starts from | Image |
| --- | --- | --- |
| `blind` | the source tree with **every `.md` deleted** | yes |
| `docs` | `AGENTS.md`, `README.md`, `CONVENTIONS.md`, the wiki. No source | yes |
| `tree` | one `TREE.txt`, names only | yes |
| `rustdoc` | `cargo doc`, `[source]` pages stripped | yes |
| `modder` | 4 wiki modding pages + `webmods/` | yes |
| `owner` | prior knowledge + the repo, timed | no - the control |

**Deltas are the signal, not absolute scores.** A question every persona fails
measures the question, not the tree.

## The one rule the protocol must enforce

**No reads outside the sandbox.** A persona that opens the task folder is
reading the answer key.

This was going to be a `/tmp` copy plus a post-hoc transcript audit - a soft
guardrail that made contamination *detectable* rather than impossible. It is
now enforced by the image: the repository is never mounted, and a persona
cannot read what is not in its container. `blind` does not have the `.md` files
deleted by instruction; they are absent. `rustdoc` cannot reach the source
because rustdoc's `src/` tree is stripped before the image is built.

The audit script is gone. There is nothing left to grep for.

**Residual:** the network stays up, because the agent talks to the API over it.
Small - the repo is a private remote and the container holds no key for it -
and `aggregate.py` flags any transcript showing a clone, fetch or web tool.

## Sequence

1. ~~Question set drafted, answer key written against the current tree.~~ Done.
2. **Owner ratification.** The gate: a question the owner disagrees with is a
   question that would have measured the wrong thing. Owner reviews
   `keys/tier1.json`, `keys/tier2.md`, `keys/tier3.md`.
3. **Review the harness code** (below).
4. **Placement decision** (below) - it changes the sandbox exclusion list, so
   it lands before the baseline or not at all.
5. **Smoke run:** one persona, one paper, end to end. Nothing has run yet.
6. Baseline: `./run.sh baseline all tier1`, then each tier 2 paper, then tier 3.
7. `./grade.sh baseline all`, `./aggregate.py baseline`, `./report.py baseline`.
8. Owner runs the `owner` persona by hand: the fixed 8-question tier 1 subset
   plus one tier 2 task, timed.

## Placement - RULED 2026-08-07: move to `<root>/benchmark/`

The benchmark is a **rerunnable tool**, not a task record - it outlives this
epic even though this epic only runs it twice. Burying it in a dated task
folder is wrong for something with that lifetime.

Moving it to the repo root has one hard consequence that must not be missed:

> **`tasks/` is excluded from every image. `benchmark/` at the root would not
> be.** The answer key would ship inside `blind`'s image and every path in it
> would appear in `TREE.txt`.

| Option | Verdict |
| --- | --- |
| Move to `<root>/benchmark/`, add it to the sandbox exclusion list | **RULED 2026-08-07.** One named exclusion list in `sandbox.sh` covering `tasks/` and `benchmark/`, applied to both the file copy and `TREE.txt` |
| Leave in `tasks/20260806-121625/benchmark/` | rejected: safe today, wrong lifetime. The epic closes; the tool should not close with it |
| Separate repository | rejected: isolation for free, but the papers cite `file:line` into this tree and would rot silently |

**The move lands in L0**, before the baseline, and it is not done until
`sandbox.sh build tree` is inspected and `TREE.txt` contains no `benchmark/`
path. `repo_files()` is the single chokepoint (`sandbox.sh:38-42`) - both the
tar copy and `TREE.txt` go through it, so one exclusion list covers both. **A
wrong exclusion ships `keys/tier1.json` inside `blind`'s image**, which does
not fail loudly; it just answers most of tier 1 for free.

`results/` is gitignored whole - **owner ruling 2026-08-07**, revising the
earlier plan to commit the three rollups. Nothing a run produces is stored. A
run is reproducible from the harness plus the keys, and the baseline-vs-after
comparison is made locally: `./report.py after baseline` renders both side by
side. The consequence to accept: the baseline number lives only on the owner's
disk between the two runs.

## Open - review

The harness was written in one pass and has never carried a real run. Worth a
reviewer's eye before the baseline, because a bug here silently corrupts the
number every later lane is measured against:

- `aggregate.py` tool-call counting against a **real** stream-json transcript.
  It has only been exercised on synthetic data.
- Whether a persona can be handed a paper for a question it was not asked
  (`make-papers.py --check` guards the generation, not the wiring).
- `grade.sh` filters the key per persona; verify the grader never marks a
  persona down for a question it never saw.
- The rustdoc payload: confirm `[source]` really is unreachable from every page
  after `src/` is stripped, rather than merely delinked.

## Prerequisites

- **Docker.** Verified on 29.6.2.
- **Auth: no API key needed.** A `claude auth login` subscription works - the
  OAuth token file is copied into the container. Verified end to end
  2026-08-07. Sharp edge: the container refreshes the token and throws the
  refreshed copy away, and Anthropic rotates refresh tokens, so the host copy
  can go stale. Use Claude on the host once before a batch.
- **L0 landed**, including `CONVENTIONS.md` at the repo root. It is still only
  a draft in the task folder, so the `docs` image currently builds without it
  and prints `MISSING`. A baseline taken like that under-measures `docs`.

## Why the ordering matters more than the content

Everything downstream of here is measured against this number:

| Lands before the baseline | Lands after |
| --- | --- |
| L0 (docs and CI must be honest first) | L5 - **deletion count is success criterion #2**; lines deleted before the baseline never enter the ledger |
| | L7, L8, L9, L10 - every structural move |

Behavior-only lanes (L1, L3, L4, L6, L11) are unconstrained and can run in
parallel with owner review.

## Verified by

Owner ratification of the question set, plus a green smoke run. The transcript
audit that used to verify this lane is obsolete - isolation is now a property
of the image, not of the transcript.

## Two runs only - owner ruling 2026-08-07

**The benchmark runs twice: the baseline here, and once at the end of the
epic.** Not per seam. Re-keying happens once, immediately before the final run.

**The owner starts and runs both.** No lane runs it, and no agent runs it.
A lane that reaches the point of needing a benchmark **stops and prompts the
owner** - it does not proceed on an assumed number.

### Re-keying, once, before the final run

Every `expect` and `citation` in `keys/tier1.json` points into the pre-epic
tree, and the epic exists to move exactly those things (`t1-001` expects
`crates/nova_gameplay/src/hud/nova_os/`; L9 moves it). So the final run needs
an updated key. Two rules:

1. **The question text is frozen.** Only `expect` and `citation` change, and
   only to the new location of the same thing. A question that has to be
   reworded to stay answerable was measuring the old structure.
2. **A question whose answer no longer exists is a finding, not a re-key.**
   Record it rather than retargeting the question at its nearest survivor.

Bump `_keyed_at` and re-open each touched citation against the tree. The
citations are what make the drafter's bias auditable, and the drafter also
designed the refactor.

**The cost of one final run, accepted:** a seam that is not paying for itself
is not visible until the epic is over. `probe run --all` still runs per seam,
so correctness is covered continuously - it is only the navigability number
that arrives late.
