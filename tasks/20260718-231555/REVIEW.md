# Review: scenario-authoring vocabulary + promote Gauntlet to the worked example

- TASK: 20260718-231555
- BRANCH: docs/scenario-patterns-gauntlet

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

Round-1 findings from a fresh reviewer with no sight of the implementing
session. It verified every quoted pattern excerpt VERBATIM against
`webmods/gauntlet/gauntlet.content.ron` (gate-counter: `gate` seeded to 1.0 at
lines 658-661, per-gate `Equal(Name("gate"), Number(N.0))` filters, bump to N+1,
range 1..=7 terminal 8.0; act-gating: FINISH sets `gate=8.0` then
`Outcome(Victory)` at 854-865, Defeat guarded `LessThan(Name("gate"), 8.0)` at
871-891), confirmed the covered-by-wiki ticks are backed by real wiki text,
confirmed `AsteroidConfig` has the audio/gravity/invulnerable fields where the
doc now claims (asteroid.rs:37,43,54,60), and confirmed the guide->file->rig
cross-links resolve both ways. `cd web && npm run ci` GREEN.

- [x] R1.1 (MINOR) webmods/gauntlet/README.md:7 - README said "v1.1.0" while the
  bundle ships 1.2.0, in a paragraph this branch already rewrites.
  - Response: fixed - now "(currently v1.2.0; grown from a thin four-gate slalom
    in v1.0.0)". Also repointed the same paragraph's stale
    "published ... by `nova_portal_gen`" to `scripts/gen-portal.py` (the
    generator is the Python script since task 20260718-152247; a drift that
    task's sweep missed, fixed here since the line was being edited anyway).

No BLOCKER/MAJOR findings. Pattern excerpts match the content file verbatim,
covered-by-wiki ticks are real, cross-links resolve, npm run ci is green.
