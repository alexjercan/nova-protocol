# fix(assets): shakedown early-derelict-kill test fails on master

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: bug, backlog
- KIND: TASK
- FLOW STEP: DROPPED
- PLAN STATUS: DRAFT

`scenario::shakedown::tests::an_early_derelict_kill_skips_to_the_fight` fails on
master. Surfaced by a full-workspace run made for the RAM investigation
(20260731-210651), not caused by any config change there: 1466 passed / 1 failed
across 64 binaries.

Panic at `crates/nova_assets/src/scenario/shakedown.rs:2522`, message
"delivery guard: the rehearsal was mid-lesson".

Repro: `nix develop --command cargo test --lib -p nova_assets an_early_derelict_kill_skips_to_the_fight`


## Dropped

- REASON: duplicate of above; test passes.
