# Implement scenario with hardcoded objectives and win/lose

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: wontdo,objectives

Foundation for the objective system. Legacy #72.

CLOSED (wontdo, 20260714): the objective foundation shipped in v0.5.0 - the
scenario engine has `Objective`/`ObjectiveComplete` actions, objective markers,
and `NextScenario` transitions, and the Shakedown Run drives a full beat chain.
An explicit win/lose *frame* is the one piece not yet built; it is folded into the
playable capital-combat vertical-slice task (20260708-203659), which owns the
win/lose framing on top of the RON scenario format. This legacy foundation task is
superseded.
