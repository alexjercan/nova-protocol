# Retro: modding error handling & logging (task 20260525-132942)

## What was asked
Make modding logic fail loudly with clear messages instead of silently.

## What happened
Audited nova_scenario's evaluation path. It was cleaner than expected: variable
expressions already return descriptive errors and callers log them; no unwraps/panics.
The real silent failure was `remove_objective` no-oping when the id didn't exist (a
scenario typo completing a non-existent objective vanished). Made objective ops loud
(warn on missing/duplicate id) and added debug tracing to the actions that ran silently
(NextScenario, SpawnScenarioObject, CreateScenarioArea).

## Lessons
- "Fail loudly" does not mean log everything. Filter non-matches (`return false`) are
  legitimate control flow; logging them would drown the signal. The failures worth
  shouting about are *semantic mistakes in the mod* (id typos), not normal branches.
- Modding is still typed Rust enums, so there is no "unknown action name" runtime path
  to guard yet. When a file/data scenario format lands, its parser will need its own
  loud error handling - noted in the task.
