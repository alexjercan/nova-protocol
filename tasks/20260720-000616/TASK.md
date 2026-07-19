# Measurement passes always split (--fps dedicated, capture-only) + loop_while_pending scene repetition with reload lines

- STATUS: OPEN
- PRIORITY: 61
- TAGS: v0.8.0,tooling,performance,testing

## Goal

Measurement passes always split + scene looping (spike
tasks/20260719-235305/SPIKE.md, D2 + adjudications 1-3). Depends on S1
(completion protocol).

- `--fps` becomes a DEDICATED capture-only pass, always (like --profile):
  the clean pass never arms NOVA_PERF, the fps pass never arms the
  recorder/invariants. Rationale is honesty, not tidiness: the recorder
  flushes JSONL per entry on the frame path, so fps-on-clean-pass numbers
  were contaminated (spike R4). Manifest records the pass; the fps check
  reads frametime.csv exactly as today.
- `loop_while_pending` (surface TBD): opt-in per example - when the
  script/timeline completes and a registered collector (the capture) is
  still pending, restart the scene (re-trigger LoadScenario, reset script
  state) and keep sampling until the window closes. Enrollment starts
  with gameplay/scenario + playable; broadside decided in-cycle (a full
  campaign slice may not loop cleanly).
- Reload intervals are MARKED: the capture excludes those frames from
  the scene stats (comparability - reload COUNT is host-speed-dependent)
  and tallies them into their own report line per looped row
  ("3 reloads: mean 210ms, max 320ms"). Scene-loading cost becomes a
  readable number, not a smear in the tail (user adjudication 2).

## Steps

- [ ] probe: fps as its own pass (env assembly, pass record, run-N log
      naming, USAGE/docs); clean pass drops NOVA_PERF arming.
- [ ] looping surface (bcs or nova_debug harness extension + capture
      reload-interval resource); scenario + playable enrolled.
- [ ] capture: reload exclusion + tally; report renders the reload line.
- [ ] Tests: env-assembly pins (fps pass vs clean pass), reload
      exclusion math (pure), report line.
- [ ] E2E: `probe run gameplay --fps` with ZERO env knobs -> three FULL
      windows; looped rows carry reload lines; baseline comparison
      between two looped runs stays like-for-like.
- [ ] Docs: skill (--fps semantics + reload line), wiki capture section,
      CHANGELOG.

## Notes

- Spike: tasks/20260719-235305/SPIKE.md. After this, 20260719-233732
  (partial-emit) shrinks to the deadline net + diagnostics.
