# Retro: Scenario clock primitive

- TASK: 20260717-112647
- BRANCH: work/scenario-timer (landed a4c6390d)
- REVIEW ROUNDS: 1 (APPROVE; 3 MINOR + 3 NIT, all fixed)

## What went well

- Restating the design goal ("compose with the existing gate vocabulary")
  instead of implementing the feature as named ("add a timer") collapsed
  the solution to a reserved variable: zero schema churn, one system, and
  every existing pattern (acts, flags, rearm counters) works on it. The
  spike's open question about timer semantics dissolved rather than got
  answered.
- Planning the failure paths as first-class steps (lint ERROR on writes,
  read exemption, fail-closed unseeded gates) meant the reviewer's probes
  - including the undocumented snapshot pattern - all landed on already
  -sound behavior; only docs needed patching.
- The reviewer re-derived chain+run_if semantics from vendored bevy
  source and confirmed the double-freeze under pause (virtual time AND
  the gate); the design's one load-bearing engine assumption held.

## What went wrong

- R1.1: the clock defeated the change-only variable log guard (a
  per-frame diff that the always-changing key trivially triggers). Root
  cause: I enumerated the CONSUMERS of variables I could remember
  (filters, actions, lint) but not every reader of the variables MAP -
  the logging diff is a consumer too. The gate-producer-and-its-consumers
  lesson applies to data structures, not just entities: a value that now
  changes every frame changes the cost model of everything that diffs it.
- R1.2: I mirrored the load-bearing registration inline in two test rigs
  when the repo already has precedent for a shared registration helper;
  the reviewer had to point at configure_scenario_gating.

## What to improve next time

- When introducing a value with a NEW change cadence (per-frame vs
  per-event), grep every reader of the containing structure and re-ask
  each one's cost/meaning under the new cadence.

## Action items

- [x] LESSONS.md: new lesson new-cadence-reaudits-readers (x1);
  production-faithful-rigs bumped (registration mirrored in rigs instead
  of shared).
