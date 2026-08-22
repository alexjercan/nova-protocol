# Navigability benchmark - protocol

Measures whether the refactor made the codebase easier to navigate, or only
moved code. Run against the current tree (**baseline**) and again after the
epic (**after**). Baseline is taken *after* the AGENTS.md correction, so the
measured delta is attributable to structure rather than to a stale table.

Owner ratifies the question set and the answer key before the baseline runs.

## Personas

Each persona isolates one information channel. The deltas between them matter
more than any single score.

| Persona | Gets | Isolates |
| --- | --- | --- |
| `blind` | the full source tree with **every `.md` deleted** | code + tree alone. The number the epic must move. |
| `docs` | `AGENTS.md`, `README.md`, the wiki. No source. | how much prose is carrying |
| `tree` | one `TREE.txt`, names only | can the folder structure alone answer it |
| `rustdoc` | `cargo doc --workspace --no-deps`, `[source]` pages stripped | is the public API self-documenting |
| `modder` | 4 wiki modding pages, `webmods/`, `assets/base/` | external contract regression guard |
| `owner` | prior knowledge + the repo | control/ceiling; validates the questions |

Key deltas:

- `docs - blind`: how much the prose compensates for structure. Should
  **shrink** if the refactor works - the tree starts answering what prose did.
- `tree`: the literal test of "tell what each module does from the folder
  structure". Expect a low baseline; that is the point.
- `owner - docs`: what is in the owner's head and written down nowhere.
- A refactor that raises `docs` but not `blind`/`tree` is the shuffling-code
  failure this benchmark exists to catch.

`modder` is a pass/fail regression guard, not a delta. The modding surface is
the wiki plus the RON format, neither of which this epic changes; its job is to
prove the epic did not break the external contract.

## Isolation

Every persona runs in its own Docker image holding **only** that persona's
channel. The repository is never mounted. There is no honor system, no "please
do not open that file", and no transcript audit to catch a violation after the
fact - a persona cannot read what is not in its container.

| Was | Is |
| --- | --- |
| a `/tmp` sandbox next to the repo | an image with no path to the repo |
| `blind` asked not to read `.md` | the `.md` files are deleted from the image |
| `rustdoc` shipping every `[source]` page | `src/` stripped, so it cannot collapse into `blind` |
| grep the transcript afterwards and hope | nothing to grep for |

The container is `--rm`, so anything an agent writes outside `/out` dies with
it. One run cannot contaminate the next, and there is no cleanup step.

**The one hole:** the network stays up, because the agent talks to the API over
it. It is a small hole - the repo is a private GitHub remote and the container
holds no SSH key or token for it - and `aggregate.py` flags any transcript
showing a fetch, clone or web tool. A flagged run needs a human glance before
it is trusted.

`tasks/` is in no image. After the refactor it holds records naming exactly
what moved, which would answer most of tier 1 outright.

`TREE.txt` is what a fresh clone shows: `git ls-files --cached --others
--exclude-standard`, minus `tasks/`. Files only, **names only** - no line counts
(owner ruling 2026-08-07). Counts would be a second information channel and
would flatter the baseline, shrinking the delta this persona exists to measure.
Build output, `.direnv` and editor droppings are absent because git ignores
them, not because a prune list is maintained by hand.

## Running

```sh
./make-papers.py                    # generate papers/ from keys/tier1.json
./sandbox.sh build                  # build one image per persona
./run.sh baseline all tier1         # every persona that is asked tier 1
./run.sh baseline all tier2a        # ... and each design task
./run.sh baseline modder tier3
./grade.sh baseline all             # grader agent, one container per result
./aggregate.py baseline             # -> results/baseline/aggregate.{json,csv}
./report.py baseline                # -> results/baseline/report.html
```

After the epic, the same with `after` as the run id, then:

```sh
./report.py after baseline          # every headline number gains a delta column
```

`owner` has no image. `./run.sh baseline owner tier1` sets the result directory
up and points at the paper; the answers are filled in by hand into the same
`answers.json` an agent would write, so everything downstream treats them
identically. The row carries no transcript, so the report marks its tool-call
counts as self-reported.

The agent is pinned, not left to the CLI's default: `run.sh` and `grade.sh`
default `NOVA_BENCH_MODEL` to `claude-opus-5` and `NOVA_BENCH_EFFORT` to
`medium` (`claude --model` / `--effort`), and record both in `run.json`.
Recording it matters: a benchmark of navigability is only comparable across
runs that used the same agent. Override either env var to run something else.

### Authentication

No API key required. A `claude auth login` subscription works: the OAuth token
in `~/.claude/.credentials.json` is mounted at `/run/secrets/credentials.json`
and copied into the container's `$HOME` by the entrypoint. Verified end to end
on 2026-08-07. `ANTHROPIC_API_KEY` is used instead when it is set.

Copied rather than mounted in place, because Claude Code refreshes the token
and a read-only bind mount would fail mid-run.

**One sharp edge.** The refresh happens inside the container and is thrown away
with it. If the access token was already near expiry, the container refreshes
it, and Anthropic rotates refresh tokens - the host copy can end up stale and
you have to `claude auth login` again. Cheap insurance: use Claude on the host
once before a batch, so the token the containers copy is fresh.

Containers run as `--user $(id -u):$(id -g)`, so results are owned by whoever
launched the run rather than by root.

## Papers

Papers are **generated**, never hand-edited. `make-papers.py` builds
`papers/<paper>-<persona>.md` from `keys/tier1.json` plus the hand-written task
bodies in `papers/src/`.

Two reasons. Tier 1 has one source of truth: the questions live next to their
answers, and drift between a question sheet and the key would silently change
what is being measured. And a persona is never shown a question it is not
asked - seeing the question is itself a hint that the thing exists.

Each paper opens with what that persona's sandbox holds. Discovering the shape
of the harness must not cost tool calls; tool calls are the metric, and they
have to measure navigating the codebase.

| Paper | Personas | Questions |
| --- | --- | --- |
| `tier1` | blind 26, docs 23, tree 17, rustdoc 24 | locate |
| `tier1` | owner 5 | the fixed control subset |
| `tier2a/b/c` | blind, docs, tree, rustdoc, owner | design |
| `tier3` | modder | build a mod |

## Tier 1 - locate

Short questions, one answer each. Recorded per question:

| Field | Values |
| --- | --- |
| `score` | any number `0.00` to `1.00`, two decimals. `1.00` matches `expect` at the stated `grain`; `0.00` is a wrong crate/module/file. In between, the key's `notes` fix the values for the cases they call out, and a multi-part question the notes do not pin scores the fraction of parts answered |
| `gave_up` | an honest refusal. Scores `0.00`, but counted apart from a confident wrong answer - same points, different finding |
| `tool_calls` | counted from the transcript, **not** self-reported. The primary metric - continuous, and the closest proxy to "every change starts with a grep" |
| `detours` | files opened that were not on the path. Measures how misleading current names are |
| `confidence` | self-report before checking. Colour, not a metric |

Score is the mean of the per-question scores, each persona against the
questions it was asked. `tool_calls` is expected to move most: an agent can
answer correctly before and after while the cost drops from 12 calls to 2.

There are no grade names anywhere in the scale - no right / partial / wrong.
The key states a number for every case it fixes, and where it fixes one that
number wins over any fraction the grader would otherwise compute. Everything
else is the fraction of required parts answered, so a two-of-three answer reads
as `0.67` instead of collapsing into the same bucket as a one-of-three answer.
See `papers/grade-tier1.md`.

Answers are capped at 40 words and graded on what they locate, never on length.
Every paper carries the same style block; the graders are told in as many words
not to reward volume.

The agent's self-reported count is kept beside the transcript count. A wide gap
is a finding about the agent, not about the codebase.

## Tier 2 - design

Three cross-cutting change requests, one paper each. The agent produces a
`NOTES.md` stating what it understands and what it would change - no code. A
grader agent scores it against the rubric, in a container holding the answer
key and no source tree, so it grades the answer rather than re-deriving it.

Tasks (owner-selected; each crosses seams the epic cuts):

1. **Add a new ship section type.** Spans `nova_ship` (sections + input
   intent), a `nova_hud` readout, the `nova_authoring` content builders, and
   the generated RON.
2. **Add a NOVA OS app.** Three crates by design: logic in `nova_os`, UI in
   `nova_os_ui`, settings in `nova_menu`.
3. **Add a new scenario action + event.** Spans `nova_scenario` actions, the
   `nova_events` vocabulary, and the gameplay emitter. Tests whether the
   events-vs-observers distinction is legible.

Rubric, `0.00` to `1.00` each on the same continuous scale as tier 1; the
grader must justify every score with a citation:

| Dimension | Asks |
| --- | --- |
| Ownership | Does it name the right crates/modules to touch? |
| Completeness | Does it miss a required surface (prelude export, RON regeneration, wiki, CHANGELOG)? |
| No phantom structure | Does it invent modules that do not exist, or trust a misleading name? |
| Cost of arrival | Tool calls to a correct model |

"No phantom structure" is the dimension that catches names lying about their
contents - the baseline's `hud/` held a terminal runtime, and `nova_modding`
held neither bundle merge nor the portal client.

The headline number is the mean of the four. Ground truth and the full anchor
table: `keys/tier2.md`.

## Tier 3 - modder

Build a mod from the wiki and the two worked examples. Objective verdict, no
grader: copy `results/<run>/modder/tier3/salvage-run/` back into the repo, run
`cargo run -p nova_authoring --bin content -- lint`, then load it in a scenario.

Pass = lints and loads. Record it in `verdict.json` beside the result;
`aggregate.py` picks it up. Any lint failure is a concrete wiki bug worth fixing
regardless of this epic. Full procedure: `keys/tier3.md`.

## Answer key

Owner ratifies. Every expected answer carries a `file:line` citation from the
2026-08-07 audits so each entry is checkable against the tree independently of
whoever drafted it - the drafter also designed the refactor, and the citations
are what make that bias auditable.

## Files

| Path | Holds |
| --- | --- |
| `make-papers.py` | generates every paper. Run it after editing the key |
| `sandbox.sh` | builds the persona images. `build` / `list` / `inspect` / `clean` |
| `run.sh` | one paper, one persona, one container |
| `grade.sh` | grader agent, key mounted, no source tree |
| `aggregate.py` | result tree -> `aggregate.json` + `aggregate.csv` |
| `report.py` | `aggregate.json` -> self-contained `report.html` |
| `keys/tier1.json` | questions, expected answers, citations, grain, coverage map |
| `keys/tier2.md` | the rubric and the ground-truth surface lists |
| `keys/tier3.md` | the mod brief rationale and the pass criteria |
| `papers/src/*.md` | hand-written tier 2 and tier 3 task bodies |
| `results/<run>/` | every run artifact. **Gitignored whole** - nothing a run produces is stored. Compare two runs locally with `./report.py after baseline` |
| `papers/*.md` | generated. Do not edit |
| `papers/grade-tier*.md` | the grader's own papers |
| `docker/` | base runtime image, persona image, container entrypoint |
| `results/<run>/<persona>/<paper>/` | transcript, answers, grades, one run each |
