# Fix the screenshot_combat torpedo-tracking flake

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

`screenshot_combat`'s autopilot step `track the torpedoes in` intermittently
stalls at its 12s deadline, so the run error-exits. Seen once in three runs
of task 20260805-185103's step 5 (a `probe run screenshots` sweep);
`process_exit`, `run_completed` and `log_clean` all caught it.

Not a probe-wiring regression: the example completes with the evidence
plugins inert AND armed, and the passing probe runs clear that step in 4.2s
against the 12s deadline. A 3x margin argues a MISSED INTERCEPT (the salvo
never closes to `TORPEDO_FUZE_RANGE + 8.0`), not slow frames - so widening
the deadline would hide it, not fix it.

Requires: reproduce (sweep the category in a loop), then find why the
torpedoes sometimes fail to close. Suspects: the preceding `blow a section
off the raider` step changing the target, the commit-salvo trigger dropping
on a frame where the target is already gone, or launch-frame nondeterminism.

Repro:
```
nix develop --command cargo run -q -p nova_probe -- run screenshots
```
