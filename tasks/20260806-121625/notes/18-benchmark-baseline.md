# Benchmark baseline - reading

The Lane02 baseline run of `benchmark/`, taken against HEAD `89c049fd` after L0
landed. Protocol is `benchmark/README.md`; this file is the reading of the
numbers, not a restatement of the method. The after-run gets `19-benchmark-after.md`
and carries the deltas.

Status: complete. Agent personas, `owner` tier 1 and tier2a, `modder` tier 3.
Tier 2 was graded three times; the numbers here are the third pass.

Numbers below are **post-regrade**. The first grading pass had a scoring bug
(see "Channel scope" below) and its tier 2 numbers must not be quoted.

Tier 2 is quoted from the third grading pass, after `H10`/`H11` removed the
fabricated Cost of arrival. It is a mean of three dimensions, not four.

## Headline

| Persona | tier 1 score | tier 1 calls | tier 2 mean |
| --- | --- | --- | --- |
| `blind` | 0.97 | 40 | 0.91 |
| `rustdoc` | 0.94 | 52 | 0.88 |
| `tree` | 0.83 | 3 | 0.74 |
| `docs` | 0.72 | 25 | 0.73 |
| `owner` | 0.75 | not recorded | 0.38 (tier2a only) |

`modder` tier 3: **PASS**. Lints clean, loads, reaches the win outcome.

Cost of the run: $27.14, 59 minutes of agent time, 17 containers. Network hits: 0.

## Tier 1 is at ceiling and cannot measure the epic

`blind` scored 0.97 - 28 of 30 full, 0 wrong, 0 gave-up - at 40 tool calls for
30 questions. Both axes are saturated. The README calls this "the number the
epic must move"; it has 0.03 of headroom, well inside grader variance.

Treat tier 1 as a **regression guard**, not a progress metric. The headline for
the after-run has to be tier 2.

Question-set sizes differ per persona, so the raw cross-persona gaps are not
like-for-like. Re-scoring `blind` against each other persona's subset:

| Subset | persona | `blind` on the same questions |
| --- | --- | --- |
| docs, 27 q | 0.716 | 0.963 |
| tree, 19 q | 0.829 | 0.947 |
| rustdoc, 27 q | 0.941 | 0.963 |
| owner, 8 q | 0.750 | 0.906 |

The gaps are real, not an artefact of which questions each was asked.

## The control is not the ceiling

`owner` scored 0.75 against `blind`'s 0.906 on the same 8 questions. The subset
is the hard end of the paper - `blind` drops from 0.97 to 0.906 on it - but the
owner is still below every agent persona except `tree`.

The number that matters is `owner - docs = 0.03`. That delta is defined as
"what is in the owner's head and written down nowhere", and it is approximately
zero. Read with `docs` being the weakest channel, the finding is not that the
prose is good; it is that the owner is navigating from the same incomplete
model the prose encodes. Several owner answers were explicitly unchecked
("I just vibed it"), which is the honest form of exactly that.

The owner answers are transcribed verbatim into `answers.json` beside the
`answers.md` they were written in; `tool_calls`, `detours` and `confidence`
were never recorded and are null rather than estimated.

### Tier 2a: owner 0.38, against `blind` 0.93

Ownership 0.17, Completeness 0.27, No phantom structure 0.70 - the lowest cell
in the whole run, by a distance. **Read it carefully, because two things are
tangled in it.**

The brief asked for "every part of the codebase that has to change... Name
files by path". The owner wrote an implementation sketch - how the feature would
be built - rather than a surface inventory. Eleven Required surfaces are absent,
including the entire content pipeline (`nova_assets/src/sections.rs`, the RON
regeneration), the ship lint, the editor palette, the input path and the HUD.
One crate named where the key requires five.

So the score conflates *did not answer the question asked* with *did not know*.
One sample cannot separate them.

What is separable, and is the real signal: **No phantom structure scored 0.70 -
the highest of the three - and the one hard structural claim was exactly right.**
"the SpaceshipSectionPlugin at line 133" matches the key's `sections/mod.rs:133-149`
from memory, and everything speculative was self-flagged as speculative. The
owner's model of the code is accurate where it exists and narrow in extent.

That is consistent with the tier 1 reading, and with `owner - docs = 0.03`: the
owner is navigating the same partial model the prose encodes. It also means the
`owner` persona is **not a ceiling on either tier**, which is what `README.md`
casts it as. Worth restating before the after-run.

## Channel scope - a scoring bug found and fixed

The tier 2 grader was deducting Completeness from `blind` and `rustdoc` for not
naming `CHANGELOG.md` and `web/src/wiki/**`. Those files are **not in those
containers**: `sandbox.sh:65` deletes every `.md` for `blind`, and `rustdoc`
holds the rendered crate API only. The rubric was measuring the sandbox.

It was 100% concentrated in those two personas - 9 doc-surface entries for
`blind`, 7 for `rustdoc`, 0 for `tree`, 1 for `docs`.

Fixed by a `## Channel scope` table in `keys/tier2.md`: an out-of-channel
Required surface leaves the Completeness denominator and reports under
`out_of_channel` rather than `missed_required`. This is the tier 2 form of what
`grade.sh` already did for tier 1, where the key is filtered to the questions
the persona was actually asked. Touched `keys/tier2.md`, `papers/grade-tier2.md`,
`grade.sh`, `aggregate.py`, `report.py`.

Effect on the two affected personas:

| Cell | before | after |
| --- | --- | --- |
| blind/tier2a | 0.78 | 0.93 |
| blind/tier2b | 0.83 | 0.95 |
| blind/tier2c | 0.68 | 0.84 |
| rustdoc/tier2a | 0.85 | 0.94 |
| rustdoc/tier2b | 0.81 | 0.95 |
| rustdoc/tier2c | 0.68 | 0.76 |

Before the fix, all four personas sat in 0.763-0.787 - indistinguishable. After
it and `H10`, the source-channel personas lead (`blind` 0.91, `rustdoc` 0.88)
and the two derived channels trail (`tree` 0.74, `docs` 0.73). The channel bug
and the fabricated Cost of arrival were together flattening the only ranking
tier 2 exists to produce.

## The grader noise floor, and where it lives

`tree` and `docs` were graded three times on the same notes - the channel rule
is a no-op for them and the key did not change for them, so all three passes are
the same grader on the same input. Comparing the three dimensions that survived
`H10`:

| Dimension | mean spread | max spread |
| --- | --- | --- |
| Completeness | 0.047 | 0.08 |
| Ownership | 0.110 | 0.27 |
| No phantom structure | 0.133 | 0.25 |

Mean spread across all 18 dimension-cells is 0.097; 3 of 18 exceed 0.20.

**The noise is not uniform, and that is the useful part.** Completeness is
stable because it counts against a Required list - an objective anchor.
Ownership and No-phantom-structure are judgement calls with only an anchor
table, and they swing by up to 0.27 on identical input. Worst case:
docs/tier2b Ownership read 0.80, 0.58, 0.85 across the three passes.

Two consequences for the after-run:

1. **Completeness is the dimension to read a delta from.** It is the only one
   whose movement can be attributed to the codebase rather than the grader.
2. **`H1` stands, and it is now targeted**: grade k=3 and take the mean, at
   minimum for Ownership and No-phantom-structure. A single-pass delta under
   ~0.25 on either of those is unreadable. Grading is a small container against
   the key with no source tree, so three passes cost a fraction of one persona
   run.

## Structural findings for the epic

These survived the regrade. Each is channel-independent - personas with
completely different information make the same error - so they are structure,
not information access.

| Id | Finding | Evidence |
| --- | --- | --- |
| B1 | The `OnDocked` emitter seam. Gameplay detects and emits, scenario only reacts. Every persona instead places the emitter in `nova_scenario/src/loader/trackers.rs` | 4/4 on tier2c. After the channel fix it is the **only** thing `blind` and `rustdoc` miss on that task |
| B2 | The NOVA OS registration chain is invisible **from the derived channels only**. `hud/nova_os/mod.rs:121` (`NovaOsCommandRegistry` init) and `nova_menu/src/lib.rs` missed by `tree` and `docs`; `blind` and `rustdoc` both found them once the channel fix stopped drowning the signal. Weaker than first read - it is a prose/tree gap, not a structural one | tier2b |
| B3 | `nova_events/src/engine.rs` is not reachable from either the tree or the prose | missed by tree and docs, tier2c |
| B4 | The prose asserts structure that does not exist. `docs` invented `crates/nova_os/src/apps/cargo*.rs`, "mirroring the existing map/ship app modules" - they live in `nova_gameplay/src/hud/` | only phantom path in the run |
| B5 | Cross-crate duplication with no shared owner is invisible to structure. `nova_menu` list+details written 3x (`t1-023`: blind 0.25, tree 0.00); max-scroll computed in 3 crates (`t1-024`: docs 0.00, rustdoc 0.50) | tier 1 |
| B6 | The public API is not cheaper to navigate than the source. `rustdoc` needs 52 calls to `blind`'s 40 and scores lower (0.94 vs 0.97) | tier 1 |

**B1 is the sharpest after-run target and the only finding missed by every
channel.** It survived all three grading passes at 4/4, and after the channel
fix it is the only thing `blind` and `rustdoc` miss on tier2c. B2 demoted: it is
real but it is a `tree`/`docs` gap, not something the source personas trip on.

B6 is the one unsaturated cost axis in the suite. If the refactor makes the
public API self-documenting, `rustdoc` tool calls should drop below `blind`'s.

## What the prose channel is doing

`docs` is the worst channel on tier 1 by a wide margin - 0.72 against `blind`'s
0.963 on the same questions - and it is the only persona that invented structure
(B4). On tier 2 it sits at 0.73, level with `tree` (0.74) and well behind the
two source channels (`blind` 0.91, `rustdoc` 0.88).

The README's stated failure mode is "a refactor that raises `docs` but not
`blind`/`tree`". That is close to unreachable from here. The live risk is the
opposite: prose that describes a codebase that does not exist.

## Harness recommendations

| Id | Item | Status |
| --- | --- | --- |
| H1 | Grade k=3 and average, at minimum for Ownership and No-phantom-structure (spread up to 0.27 on identical input). Completeness is stable at 0.08 and can be read single-pass | **required before the after-run** |
| H2 | `aggregate.py:287` / `report.py:300` say `docs - blind` "should SHRINK". The gap is -0.25; shrinking it means `docs` getting worse. As written a prose regression prints as a pass | open, owner declined for now |
| H3 | `tree` tool_calls of 3 is an artefact - one file in the sandbox, so the cost axis is structurally pinned. Reporting it as a number reads as "cheapest persona" | open, owner declined for now |
| H4 | The tier 3 brief says "Everything you need is in this directory". `sandbox.sh:133-135` stages 4 wiki pages; the modder needed `guide-author-scenario.md` and `guide-author-section.md`, which exist in the repo but are outside that set. Either stage them or reword the brief | open |
| H5 | `aggregate.py` records `model: "default"`. Transcripts record `claude-opus-5` on all 17 rows. Read it from the transcript so the after-run is provably comparable | open, owner accepts the risk |
| H6 | `.gitignore:258` ignores `benchmark/results/` whole, so the baseline exists only on the owner's disk. `README.md` documents this as intended, but the L0 step text claims `aggregate.json`, `aggregate.csv` and `report.html` stay tracked - they do not. `./report.py after baseline` needs the baseline tree present; losing it means the epic has no before | open, decide before the after-run. TASK.md's step corrected to void the commit half |
| H7 | The grader dropped `rustdoc`'s `t1-018` and `aggregate.py` derived `asked` from the grades, averaging 27 answers over 26 | **fixed.** `asked` comes from the key, `ungraded` is per row, aggregate prints a loud line. Regraded: `t1-018` = 1.00, rustdoc 0.95 over 26 -> **0.94 over 27** |
| H8 | `rustdoc`'s `[source]` hrefs survived the deletion of the pages they point at, and `../src/nova_mod_format/lib.rs.html#139` is a file:line answer at tier 1 grain | **fixed.** Baseline never touched them (0 tool calls reference the pattern) so it stands; `stage_rustdoc` now rewrites them away |
| H9 | The persona filter deciding what a paper shows, what the grader may mark, and what a score is a mean over was implemented twice with nothing catching drift | **fixed.** One implementation in `benchmark/persona_filter.py` |
| H10 | **Cost of arrival was never computable.** It is a ratio against the owner's tool-call count; the owner works in an editor and has no transcript, so the denominator does not exist for any task. 11 of 12 graders said so in their own citations and scored it anyway - some defaulting to the 0.67 anchor, some judging the respondent's count absolutely. A quarter of every tier 2 headline was unmeasured | **fixed and regraded.** Null when unanchored; headline is a 3-dimension mean, `max` follows |
| H11 | The grader was fed the **self-reported** tool count, not the transcript count, for that same dimension. `blind/tier2a` self-reported 14 against 28 actual - a 2x error on the one input the dimension consumes. `README.md` says in as many words not to trust the self-report | **fixed and regraded.** `grade.sh` counts via `aggregate.parse_transcript`, so there is one counter |

## Corrections to earlier readings of this run

Recorded so the after-run does not inherit them:

- "`tree` beat `blind` on tier2a, contradicting the low-tree-baseline premise"
  was an artefact of the channel bug. Post-fix it is tree 0.90 vs blind 0.87,
  and `blind` leads overall. `tree` at 0.83/0.78 is still higher than the README
  predicted, but it is not an outperformer.
- "Doc surfaces missed by 4/4 personas" was wrong; it was 7 of 12 tier 2 runs
  and entirely confined to `blind` and `rustdoc`.
- "`GAPS.md` found five dead wiki links" was the modder's claim taken at face
  value. All five pages exist in the repo. They are absent from the modder
  image only. There are no wiki bugs in this run, and `keys/tier3.md`'s
  "every lint failure is a wiki bug" rule never fired - the lint passed.
- The tier 2 means quoted after the second grading pass (`blind` 0.86,
  `rustdoc` 0.81, `tree` 0.78, `docs` 0.70) still contained the fabricated Cost
  of arrival. The third pass replaces them with `blind` 0.91, `rustdoc` 0.88,
  `tree` 0.74, `docs` 0.73 over three dimensions. Direction unchanged, spread
  wider.
- "B1 and B2 are the sharpest after-run targets" - B2 is demoted. `blind` and
  `rustdoc` both found the NOVA OS registration chain in the third pass; only
  `tree` and `docs` miss it. B1 is the one finding every channel misses.
- The noise floor was first reported as 0.065 mean absolute delta from two
  passes. Three passes put it at 0.097 mean spread, and locate it: Completeness
  0.047, Ownership 0.110, No phantom structure 0.133.
