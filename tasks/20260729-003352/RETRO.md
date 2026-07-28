# Retro: nova_probe commit-keyed probe-runs folders and baseline discovery

- TASK: 20260729-003352
- BRANCH: tooling/probe-commit-keyed-runs
- REVIEW ROUNDS: 1

## What went well

- The test-first compile failed on the intended missing helpers, so the first
  red state matched the planned behavior instead of an unrelated build issue.
- Treating a single example as a one-item aggregate simplified the output model
  and made the docs easier to state: one storage base, one commit root, one
  aggregate index shape.
- The explicit compatibility test caught the important migration line: explicit
  old roots still work, but automatic discovery does not select ad hoc folders.

## What went wrong

- R1.1 and R1.2 happened because the first docs sweep fixed the high-level run
  layout but missed the CLI synopsis and the report-command snippet. Root cause:
  I searched for old path examples before checking every live command synopsis.
- R1.3 happened because a compatibility helper was left as a production wrapper
  after its last production caller moved to `baseline_for`. Root cause: I did
  not run `cargo check -p nova_probe` before handing the branch to review.

## What to improve next time

- For CLI behavior changes, sweep both prose examples and synopsis lines before
  review; users often copy synopsis lines first.
- When a helper becomes test-only during a refactor, either delete the wrapper
  or gate it immediately, then run `cargo check -p <crate>` in addition to the
  test command.
- If an existing aggregate path can represent one item, start from that
  primitive before preserving a special single-item path.

## Action items

- [x] Fixed the CLI usage and dev-wiki report command during review.
- [x] Removed the test-only baseline wrapper from production code.
