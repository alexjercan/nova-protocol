# Design-promised probe markers + monotonics for sections/broadside (show the feature working on the timeline)

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.8.0,testing,examples

## Goal

Depth pass on the probe timeline for the examples whose in-example
assertions already encode OUTCOMES: add design-promised `probe_marker`
beats and `.monotonic([...])` variables so reports SHOW the feature
working (the turret fired and a gate took hits; the broadside acts
progressed), not just the process surviving. Bounded by the standing
rule: only variables the scenario DESIGN promises one-way, only beats
the script actually stages - overreach makes flaky invariants (the
goldens lesson).

Candidates (judged per example at implementation, not exhaustively
committed here):

- sections/: the outcome flags the autopilot assertions already track
  (turret: fired + gate_damaged; torpedo: launch + hit; hull: destroy +
  survive; com_range: mass-properties checkpoints) become markers at the
  assertion sites; counters that only grow (rounds fired, gates hit)
  become monotonics IF the script design promises them one-way.
- gameplay/broadside: stage markers for the scripted arc (defeat ->
  Retry -> acts -> Victory) - the self-ending script's stages are
  design-promised by construction.
- ui/: state-transition timeline already covers the flows generically;
  add markers only where a staged AutopilotPlugin timeline has named
  beats worth showing.

## Steps

- [ ] Per-example pass over sections/ + broadside (+ ui where a staged
      beat exists): markers at assertion/stage sites, monotonics only
      where design-promised; each addition justified in a line here.
- [ ] Validate: `probe run --all` (or the touched categories) - every
      new marker appears on its timeline, no new invariant violations;
      read the reports.
- [ ] Probe skill: refresh the wired-examples statement with the depth
      coverage.
- [ ] Verify: fmt; touched examples compile; CHANGELOG Unreleased line.

## Notes

- Spike: tasks/20260719-205543/SPIKE.md (T3). User adjudication
  2026-07-19: in-sprint at p52 (not backlog). Depends on T2 (fleet
  wiring) landing first.
