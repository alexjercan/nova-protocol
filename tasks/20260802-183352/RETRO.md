# Retro: Add a runnable nova_autopilot example with a headless integration test

- TASK: 20260802-183352
- BRANCH: feat/autopilot-driven-example
- REVIEW ROUNDS: 1

## What went well

The plan's "Discovered facts" section had already checked the two things that
would otherwise have cost a cycle each: that a crate-level `examples/` needs no
`[[example]]` block (`autoexamples = false` is a root-manifest key) and that the
`--examples` DoD command was GREEN on base, so the RED proof had to be the new
test. Work started with no discovery detour.

Falsifying the in-example guard (short-circuit the input closure, watch the run
exit 101) took one ~90s Xvfb run and is what turns "the example passes" into
"the example could fail". Cheap, and it is the evidence the review leaned on.

Breadth: two new files plus a CI step, matching the plan exactly. No split was
missed. Churn: zero review rework - the only diff-shaping surprise was found by
the DoD command itself, not by the reviewer. Context: no pressure observed, no
checkpoint or handoff.

## What went wrong

The no-coupling DoD grep failed on the example's own doc comment, which named
`nova_probe` in prose about who reuses the shape. The proof greps the whole
directory - comments included - and the plan wrote that DoD before the example
existed, so nothing flagged that prose could trip it. Diagnosis was immediate
(the grep printed the offending line); the fix was a reword plus a comment
saying why the crate path is spelled out longhand.

`DECISION.md` came out of the plan phase in a free-form `# Decisions - <id>`
shape that `tatr check` rejects (it wants `# Decision: <title>` plus DATE /
STATUS / TASK / TAGS and four fixed sections). It went unnoticed until the
first `check` in the work phase, because plan never ran one.

## What to improve next time

A proof-by-grep over a source directory catches comments. When a DoD encodes
one, either scope it to code (`--type rust` with a `^use` anchor) or expect the
implementation to spell forbidden names longhand - and say which in the plan.

## Action items

- None. Both nits (R1.1 test timeout, shared with `tests/examples_smoke.rs`;
  R1.2 crate-doc pointer) are recorded in REVIEW.md and belong to existing
  surfaces - the doc pointer to `20260802-183355`, the timeout to whichever
  task revisits the smoke harness.
