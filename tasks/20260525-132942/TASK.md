# Improve error handling and logging in modding logic

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.3.1, bug

Fail loudly with clear messages instead of silently. Legacy #115.

## Steps

- [x] Audit the modding evaluation path (filters, variables, actions, world) for silent
      failures and missing logging.
- [x] Make objective operations loud (completing/removing a non-existent objective).
- [x] Add debug tracing to the actions that executed silently.
- [x] Verify build --all-targets, clippy, fmt.

## Resolution (CLOSED)

Audited nova_scenario's mod-evaluation code. The evaluation layer was already mostly
sound: variable expressions return descriptive VariableError (undefined variable, type
mismatch, division by zero), and the VariableSet action and expression filter already
log those; the filter early-returns are legitimate non-matches, not errors, so they
should stay quiet. No unwrap/expect/panic anywhere in the crate.

The genuine silent failure was in objective handling (world.rs):
- remove_objective just did `retain(id != ...)`, so completing an objective whose id
  does not exist (a scenario typo, or a Complete without a matching Objective action)
  vanished with no signal. Now it warns with a clear message when nothing was removed,
  and debug-logs the completion when it succeeds.
- push_objective now warns on a duplicate id and debug-logs additions.

Also added debug tracing to the actions that previously ran silently so a scenario's
execution can be followed in the logs: NextScenario (which scenario is queued + linger),
SpawnScenarioObject (which object id), and CreateScenarioArea (id + radius).

Scope note: modding configs are currently typed Rust enums (no file parsing yet), so
there is no "unknown event/action name" runtime failure to guard; the meaningful silent
failures are semantic (objective id mismatches), which is what this addresses. When a
data/file scenario format lands, its parser will need its own loud error handling.

Verified: build --all-targets, clippy, fmt green.

Self-reflection: the crate was cleaner than expected, so the win was targeted (loud
objective mismatch + action tracing) rather than a broad rewrite. Resisting the urge to
add logging to the legitimate filter non-match paths kept the logs signal, not noise.
