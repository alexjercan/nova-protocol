# Retro: ledger_ch5 - torpedo-ship reward raid finale

- TASK: 20260723-182855
- BRANCH: feature/ledger-ch5-raid
- REVIEW ROUNDS: 1 (APPROVE, out-of-context; one NIT fixed post-approval)

See TASK.md Outcome for what/why; this is process only.

## What went well

- The content LINT was the highest-leverage tool in the whole task. Authoring a
  ~2900-line data file, the errors that matter are not RON syntax (the parser
  catches those) but game-geometry and balance invariants a human cannot eyeball:
  it caught floating turrets (mount -Y must sit against an occupied cell),
  enemies spawned inside their own threat envelope ("spawned-dead" = under fire
  before first input), and an input binding that double-drove the flight RCS.
  Running lint FIRST, before writing the rig, turned three latent bugs into three
  quick fixes. Lesson worth keeping: for a new scenario, lint is the fast oracle
  - run it the moment the file parses, iterate to clean, THEN write the rig.
- Splicing the big ship / small ship section blocks from the SHIPPED Auditor
  (cargoB) and broadside (racer) layouts, via a placeholder pass, instead of
  hand-transcribing ~2000 lines of cube entries. Section ids are ship-local so
  one block is reused across all six small ships safely, and the exact
  cube/prototype ids are guaranteed to match a loading ship. Zero transcription
  bugs.
- Verifying the ONE genuine runtime risk myself rather than hoping: the base is
  an AI ship with no thrusters, which lint/parse cannot exercise. I read
  `on_thruster_input` and the AI vector math (try_normalize / normalize_or_zero)
  and confirmed a thrusterless AI ship just holds station and fires its turrets -
  no panic. That is exactly the "verify a load-bearing claim against the
  production path" the review discipline asks for, done before the reviewer.
- Caught the cross-file fixture pin: the ch4 change (add a NextScenario to the
  sell win) broke the ch4 rig's "the sell win does not chain" assertion, a pin
  far from the diff. Updated it to the real new contract (not weakened) and fixed
  the stale ch4 header comments in the same pass.

## What went wrong

- I mis-stated the ch5 rig test count as "10" in TASK.md and REVIEW.md; it is 9
  (the "10 passed" I remembered was the ch4 rig, which cargo ran first in the
  combined invocation). Root cause: reading a combined two-binary test summary
  and attributing the counts to the wrong file from memory. Fixed both. Cheap,
  but it is the same class as the AGENTS.md "re-read the artifact, do not trust
  the success report" rule - I trusted a remembered number instead of grepping
  `#[test]`.
- One NIT from review: the rig imported an `outcome_message` helper it never
  used (dead_code warning). I copied the ch4 harness wholesale, including a
  helper ch5's assertions did not initially need. Fixed by actually asserting the
  Victory/Defeat messages (which also strengthened the tests) rather than
  deleting the helper.

## What to improve next time

- When quoting a per-file test count, grep `#[test]` in that file (or read the
  single-binary result), never attribute a number from a combined run's memory.
- When copying a sibling test harness, prune the helpers the new file will not
  use in the same pass, or plan to use them - an unused copied helper is a
  guaranteed dead_code warning.

## Action items

- [x] Fixed the DoD lint command to the `--target` form (same
  `inherited-cli-string-drifts` lesson as the sibling overspeed task - the
  ledger already had it logged this cycle).
- [x] Lessons ledger: added `lint-is-the-fast-oracle-for-new-scenarios`.
