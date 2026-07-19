# Design-promised probe markers + monotonics for sections/broadside (show the feature working on the timeline)

- STATUS: CLOSED
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

- [x] Per-example pass over sections/ + broadside (+ ui where a staged
      beat exists): markers at assertion/stage sites, monotonics only
      where design-promised; each addition justified in a line here.
      ADDITIONS (all at existing assertion/stage sites, once-guarded):
      - turret_section: "outcome: turret fired" / "outcome: gate damaged"
        on first observation of the range's own outcome flags.
      - torpedo_section: the full chain - fired / armed / detonated /
        gate damaged - one marker each on first observation.
      - hull_section: "outcome: partial hit exact" (health_after on the
        record) and "outcome: section destroyed, ship survives".
      - controller_section: "outcome: attitude tracks" (error_rad).
      - thruster_section: "outcome: burn accelerates, plume follows"
        (speed_before/after).
      - com_range: "outcome: com + camera track section loss"
        (com_drift, camera_drift).
      - broadside: "stage N" per script transition (note + t) - buffered
        in `advance` (state is out of the world there), flushed at the
        closure's single insert point.
      MONOTONICS: none added, deliberately. broadside's `act` RESETS on
      the Retry reload (not design-promised one-way); the section ranges
      have no growing scenario variables; scenario/playable keep theirs
      from the original wiring. The design-promised bar filtered every
      candidate out - markers are the honest depth here.
      UI: no markers - menu_newgame/hud_range flows are state-transition
      shaped, and the generic timeline already records every transition;
      a duplicate marker would say nothing the timeline does not.
      torpedo_guidance: no outcome flags exist (it asserts at
      scenario-load level) - nothing design-promised to mark.
- [x] Validate: `probe run --all` (or the touched categories) - every
      new marker appears on its timeline, no new invariant violations;
      read the reports.
- [x] Probe skill: refresh the wired-examples statement with the depth
      coverage.
- [x] Verify: fmt; touched examples compile; CHANGELOG Unreleased line.

## Notes

- Spike: tasks/20260719-205543/SPIKE.md (T3). User adjudication
  2026-07-19: in-sprint at p52 (not backlog). Depends on T2 (fleet
  wiring) landing first.

## Close-out (2026-07-19, branch feature/probe-depth-markers, stacked on T2)

Validated live: `probe run sections,broadside` (also the first real MIXED
category+name spec) - aggregate OK, 8/8 rows measured 5/6, 0 invariant
violations, and every marker verified ON its timeline: the torpedo chain
in order (fired -> armed -> detonated -> gate damaged), turret's two
outcomes, hull's two stages, attitude/burn/com with their values, and all
11 broadside stage transitions. Skill depth table + CHANGELOG updated.

Stacked-flow note: implemented while T2's exit gate ran (user-directed);
the one cross-branch fix (perf_baseline exit ownership) merged forward
cleanly - T3 never touched that file.
