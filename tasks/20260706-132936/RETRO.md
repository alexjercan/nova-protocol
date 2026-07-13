# Retro: finalize nova_gameplay crate boundary (task 20260525-132936)

## What was asked
Audit nova_gameplay and confirm it is the umbrella for gameplay-specific plugins,
moving misplaced modules in or out.

## What happened
Audited every module under crates/nova_gameplay/src. The boundary was already clean:
all spaceship/section/weapon/input/camera code is correctly placed, and nothing
gameplay-specific is stranded elsewhere. Outcome was documentation, not code: wrote a
three-tier crate boundary policy into docs/architecture.md and filed follow-up task
20260706-151804 for the generic-leaning promotion candidates.

## What went well
- Delegating the module-by-module read to an Explore subagent kept the audit thorough
  without flooding the main context.
- Recognizing that "move out to bevy_common_systems" is now a cross-repo change (the
  crate was externalized in v0.3.0) avoided a premature, out-of-scope extraction.

## What to do differently
- For an "audit" task with a likely-clean result, decide the deliverable up front:
  a written policy + a tracked follow-up, not a forced code change to justify the
  cycle. That framing would have saved a couple of exploratory steps.

## Lessons for future tasks
- The crate boundary policy now lives in docs/architecture.md - consult it before
  deciding where new helpers belong. Promotion to bevy_common_systems is deliberate
  and cross-repo, never automatic.
- Several v0.3.1 "crates/refactor" tasks are audits whose honest answer may be "already
  correct"; closing them with a documented rationale is a valid completion.
