# Retro: NOVA OS map view label table + map goto <label>

- TASK: 20260728-160001
- BRANCH: feat/map-view-labels
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what/why and REVIEW.md for the findings; this is process only.

## What went well

- Mirrored the just-landed sibling (`ship view`, 20260728-125510): reused
  `SectionCode`/`assign_section_codes`/`terminal_ship_rows` as the template, so
  the map side landed in one review round with a familiar shape a cold reader
  already knows.
- Found the two load-bearing cross-app hazards during UNDERSTANDING, before
  writing code: the single `pending_invocation` slot (needs a name-gated
  peek-then-take) and `set_arg_completions` being replace-all (needs a merge so
  ship and map do not clobber each other's Tab set). Surfacing them up front
  meant they were plan steps, not review findings.
- Confirmed the real fork (label mechanism: minted component vs ephemeral; and
  scope: include the `goto` verb) with the user via AskUserQuestion before
  planning, so the architecture was decided, not guessed.

## What went wrong

- The Bash shell cwd resets to the MAIN checkout on every call, so bare
  `grep`/`rg` read the unmodified main tree, not the sprout worktree. Caught
  only because a grep line number disagreed with a Read of the worktree file
  (grep said a test was at L1185; Read showed app code there). Root cause:
  assumed shell cwd persisted into the worktree after the first `cd`. Cost: a
  couple of confused greps and a re-grep with the absolute path.
- One compile error: `Query::get` returns `Result`, not `Option`, so
  `.map(..).unwrap_or_else(||..)` failed on closure arity. Trivial (`.ok()`),
  but a reminder the ECS getter is a Result.

## What to improve next time

- In a sprout worktree, prefix every shell command with `cd <worktree> &&` or
  pass absolute worktree paths to grep/rg - never trust the cwd to have stayed
  in the worktree. A grep that reads the wrong tree can falsely "verify" stale
  code.

## Action items

- [x] Ledger: added `worktree-cwd-resets-verify-absolute-path`.
- No follow-up code tasks: the goal (labels + table + goto) shipped whole.
