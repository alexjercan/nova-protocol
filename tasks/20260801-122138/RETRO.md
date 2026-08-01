# Retro: Fix shakedown an_early_derelict_kill_skips_to_the_fight failing on master

- TASK: 20260801-122138
- BRANCH: master
- REVIEW ROUNDS: 1

## What went well

The failing guard named the right boundary: the shortcut was not really at the
rehearsal. Reading the full walk beside the helper made the fix one line.
Focused and crate-level proofs both stayed cheap.

## What went wrong

The helper looked like a harmless fast-forward, but it no longer mirrored the
script's real opening path after beat 1 moved behind the conversation
hand-off. The failed decision was trusting the shortcut's name instead of
checking its upstream setup against the full walk first.

## What to improve next time

For scenario shortcut helpers, compare the shortcut against the full
end-to-end walk before diagnosing the handler being tested. Every upstream
hand-off that production requires belongs in the shortcut too.

## Action items

- Bumped `production-faithful-rigs`: shortcut fixtures must replay production
  hand-offs, not just fire later events.
