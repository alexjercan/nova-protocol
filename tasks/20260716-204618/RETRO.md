# Retro: content_lint --target

- TASK: 20260716-204618
- BRANCH: feature/lint-target (landed, see squash commit)
- REVIEW ROUNDS: 1

## What went well

- The walk/lint split fell out cleanly because the original core was
  already pure - target mode is ~60 lines and the known-set rules are
  literally the same function for both modes.
- The external-mod test fixture discriminates three properties in one
  assertion set (base visibility, error detection, id attribution).

## What went wrong

- Nothing significant; second string-continuation whitespace artifact
  in as many days (format!/eprintln with a trailing backslash keeps the
  next line's indentation) - worth remembering, not worth a ledger line
  until it recurs again.

## What to improve next time

- Rust multi-line string literals in user-facing messages: prefer
  concat!/single-line over backslash continuations.
