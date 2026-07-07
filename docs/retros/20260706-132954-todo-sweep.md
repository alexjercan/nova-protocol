# Retro: TODO sweep (task 20260525-132954)

## What was asked
Sweep all TODO comments; resolve or convert into tatr tickets.

## What happened
All 22 TODO/FIXME comments were design decisions, enhancements, or known limits - none
trivially fixable. Converted each into a tracked ticket (5 new v0.4.0 tickets + mapping
the rest to existing tasks) and back-annotated the code as `// TODO(<id>): ...`.

## Lessons
- "Fix TODOs" usually means "make them tracked", not "implement them now". The honest
  move is ticket + back-reference, not sneaking 5 features into a cleanup task.
- tatr ids are second-granularity: creating tickets in a tight shell loop collides into
  one id. Space them (sleep) or create sequentially with a pause.
- Back-annotating `// TODO(<id>):` is cheap and high-value: it turns a floating comment
  into a two-way link between code and backlog.
