# Retro: Example categories: write the contract and resolve probe run policy from it

- TASK: 20260804-093855
- BRANCH: refactor/example-category-policy
- REVIEW ROUNDS: 2

## What went well

**The scope boundary was written down before implementation, and it held.**
The planning correction in TASK.md ("this task ships the CONTRACT and the
POLICY TABLE only") named the three sibling tasks that own the directory moves
and predicted the exact failure mode of ignoring it: `catalog_matches_disk`
goes red if the smoke consts move without their directory. Nothing in either
review round touched that boundary. Transitional `gameplay`/`perf` rows with
`# remove with <task-id>` gave the follow-ups a grep target instead of a
memory.

**Deleting the second config parser was the right call, and DECISION carried
why.** The two-vs-three-field question (`{probed, frame_time}` vs
`{correctness, frame_time, in_all}`) was settled against the five real rows
rather than against imagined ones, and the consequence that fell out of it -
`probe run screenshots` must ERROR rather than expand to nothing - was
recorded as load-bearing before it was implemented. Round 2 re-derived that
branch and agreed.

**The transitional behavior change was flagged in the open, not buried.**
`--fps` losing its frame-time pass for `sections/`, `ui/` and `gameplay/`
during transit is a live CLI difference; it was called out in TASK.md, in
DECISION Consequences, and in CHANGELOG rather than left for a user to
discover.

## What went wrong

**A DoD `cmd:` proof was written narrower than the claim it was standing in
for, and the gap was invisible.** The proof
`! rg -n 'fps_exempt' crates/nova_probe web/src/wiki` was intended to mean
"the exempt-list mechanism is gone from the code and the docs". It actually
meant "gone from two paths". Two things hid the difference: the path list
omitted `.claude/`, and ripgrep skips hidden directories by default, so even a
whole-repo `rg fps_exempt .` would have stayed silent. `.claude/skills/probe/SKILL.md`
is named by root `AGENTS.md:94` as THE probe manual, and it still instructed
readers to use the deleted `fps_exempt` key and the deleted 60/240 window.
That was round 1's only MAJOR.

Why it seemed sound at plan time: the doc-surface analysis in TASK.md's
planning corrections was unusually careful - it caught that `rg 'gameplay/'`
was inflating the count via `crates/nova_gameplay/`, and correctly narrowed
the sweep to one wiki page. The care went into *which* pages under
`web/src/wiki`, and the question "which doc surfaces exist outside the wiki"
was never asked. The routing map in `keeping-docs-in-sync.md` reinforced this:
its `nova_probe` row names `dev/automation-harness.md` and CHANGELOG, and does
not list the skill file at all.

**The same defect existed in a higher-authority surface and survived round
1.** Round 1 found the stale category list in `SKILL.md:25`; round 2 found the
identical text in `cli.rs:12-13`, which is what `probe --help` actually
prints. The round-1 fix corrected the doc *about* the tool without checking
whether the tool said the same thing about itself.

**A test was weakened in the same edit that introduced the branch it was
pinning.** `resolve_all_and_explicit_excluded` went from an exact
`assert_eq!(resolved.excluded, ...)` to `.contains(...)` while the same diff
added the per-category dedupe guard (`if !excluded.contains(&entry)`). The
exact assertion was the only thing that would have caught a broken dedupe. It
looks like collateral from retargeting the fixture (`EXCLUDED` moved to
`playable` because `screenshots` became unprobed wholesale), not a decision -
`examples` was *strengthened* in the same edit, from `len() == 5` to an exact
vector. The new test's own message claimed "recorded once, by category" while
asserting only "at least once".

## What to improve next time

**A `! rg` proof needs its search paths derived from the claim, not from the
files being edited.** If the sentence is "gone from the code and the docs",
enumerate the doc surfaces first - including `AGENTS.md` pointers and hidden
directories - and write the proof to cover them. Concretely: prefer
`rg --hidden -g '!.git' -g '!tasks' <pattern> .` over a hand-listed path set,
so a new surface fails the proof instead of hiding behind it.

**When a change invalidates prose, sweep the tool's own strings alongside the
docs.** `--help` text, usage constants and error messages are documentation
with a shorter feedback loop and higher authority than any Markdown file, and
they live in `.rs` files where a doc sweep does not look.

**A test edit that changes an assertion's STRENGTH deserves the same scrutiny
as a code change.** Retargeting a fixture is mechanical; downgrading
`assert_eq!` to `contains` is not, and the two arrived in one hunk. Worth
re-reading assertions touched by a fixture change specifically for what they
stopped proving.

## Diagnose

**Breadth.** The diff is large (21 files, ~1100 insertions) but not
overgrown. The size is inherent: renaming `fps_exempt -> fps_skipped` touched
six files and 26 sites because the field is serialized into `checks.json` and
rendered in three places, and the plan predicted exactly this ("`fps_skipped`
touches six files, not two" - a correction made *against* the pre-plan draft's
estimate). The genuinely separable work - the three directory moves - was
already split out into `093910`/`093934`/`094006` before implementation
started. No missed split.

**Churn.** Round 1's MAJOR would not have been prevented by the from-scratch
challenge or the cold-reader test; both interrogate the design, and the design
was sound. It would have been prevented by a doc-surface question the plan
does ask but scoped too narrowly - the plan enumerated pages under
`web/src/wiki` and treated that as the doc surface. The plan-time question
worth adding is "which files outside the wiki describe this behavior", with
`AGENTS.md` pointers and `.claude/` as the standing answer for this repo. The
R1.5 test weakening is a work-time miss rather than a plan-time one; the plan
correctly named the fixtures as needing new members.

**Context.** No context pressure observed. No checkpoint, no compaction
warning, no handoff, and no delegation beyond the two review subagents the
review skill requires. Both review rounds ran out-of-context by default, which
is what surfaced both the MAJOR and R2.1 - neither was visible from the
implementation context that wrote the proof.

## Knowledge

Three existing lessons bumped, one added. All writes succeeded;
`knowledge check` clean.

| Lesson | Why |
|-|-|
| `verification/a-search-tool-excludes-by-default` (bumped) | The ripgrep hidden-directory miss behind round 1's MAJOR. |
| `verification/absence-needs-an-enumerated-scope` (bumped) | The proof's path list was derived from the files being edited, not from the claim. |
| `testing/a-presence-assertion-does-not-bound-the-output` (bumped) | The `assert_eq!` -> `.contains(...)` downgrade, and the `.find(...)` whose message claimed once-ness it did not assert. |
| `docs/in-code-help-text-is-a-doc-surface` (NEW) | Round 2's R2.1: the fix corrected the doc *about* the tool while the tool's own `--help` kept contradicting it. No existing lesson covered usage constants and error strings as doc surfaces. |

**The search-tool lesson is a REPEAT, and that is the sharpest signal here.**
Its previous occurrence is `20260802-183406` - same repository, two days
earlier, same mechanism, and even the same unsearched directory (`.claude/`).
A lesson that has now fired twice in one week in one repo is not being carried
into the next task's planning. That argues for a mechanical guard rather than
another remembered lesson; see the action items.

## Action items

- Make the repeat structural instead of remembered: any `! rg` proof in a DoD
  for this repo should be written `rg --hidden -g '!.git' -g '!tasks'` by
  default. Worth adding to the repository `AGENTS.md` checks line so it is
  read at plan time rather than recalled - the two occurrences two days apart
  show recall is not working.
- `R2.3` stays open as a NIT in REVIEW.md: `spec_help` lists an unprobed
  category as runnable in the very error that rejects it. Owner is
  `20260804-093910`, which settles `screenshots/` and may remove the question.
- The per-EXAMPLE `NOT_PROBED` axis now has no live member under
  `--all`/category expansion; recorded in DECISION.md with the reason it is
  left standing (an explicit `probe run render_scale_shot` still needs its
  warning). `20260804-093910` decides its fate.

## Landing message

```
refactor(probe): resolve example run policy from a category contract

Give every example category an explicit contract and a two-boolean policy
table in code (nova_probe::CATEGORY_POLICIES), replacing the
perf-vs-everything frame-time split and the hand-listed fps_exempt array.

probed gates both --all and bare category expansion: --all now skips an
unprobed category and records the absence by category, and a bare
`probe run screenshots` errors instead of expanding to a no-op that would
read as a pass. frame_time carries the --fps pass, so only stress/ (and
perf/ until it is absorbed) makes a frame-time claim; every other category
records WHY its frame-time section is empty.

The contract's prose half lands in its two homes - per-block comments in the
root Cargo.toml and a five-row table in the dev wiki - plus probe's own
--help and skill manual, which documented the deleted mechanism.
every_category_has_a_probe_policy fails any category without a row, so the
unknown-category default can never quietly apply.

Ships the contract and the table only: no example moves and no directory is
renamed, so gameplay/ and perf/ carry transitional rows until
20260804-093910/093934/094006 land.

BREAKING: the checks.json manifest field fps_exempt is renamed fps_skipped,
with no compatibility shim.
```
