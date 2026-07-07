# Retro: spaceship brain cleanup (task 20260525-132947)

## What was asked
A "clean up refactor pass" on the spaceship brain (the AI controller).

## What happened
No specific defect - the AI worked. Did a disciplined, behavior-preserving readability
pass: named the steering magic numbers (0.2/2.0/20.0/1.0/0.95), extracted the duplicated
chase/brake steering into ai_desired_direction(), and fixed a non-ASCII apostrophe.

## Lessons
- A vague "clean it up" task is a trap for speculative churn, especially when you can't
  runtime-verify. The safe interpretation is *provably equivalent* readability changes:
  same literal values behind names, dedup that matches in every reachable state.
- Before deduping two near-identical blocks, prove the difference is dead. Here one copy
  had a zero-direction fallback the other lacked; it was unreachable (too_fast implies
  non-zero speed), so unifying them was safe. Assuming they were identical would have
  been wrong; checking made it safe.
