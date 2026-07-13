# Review: Minimal faction/relation model (hostile/neutral/own)

- TASK: 20260708-203708
- BRANCH: feature/faction-relations

## Round 1

- VERDICT: APPROVE

Reviewed `git diff master...feature/faction-relations` (10 files, +426/-16)
against TASK.md; ran the full nova_gameplay suite on the branch: 193/193
green. The diff delivers the Goal exactly as scoped: a pure `relation()`
resolver over optional `Allegiance`, allegiance-by-requirement on the two
ship markers (verified: the pre-existing targeting tests that spawn bare
markers pass unchanged, which proves the require path works), copy-at-spawn
on both projectile paths with an honest owner-death rationale, and the two
consumer swaps (targeting hostility, reticle tint) each with an invariant
test (neutral never auto-acquired; three-way tint). The relation matrix
test covers all 10 meaningful cells including the deliberate
`Neutral vs Neutral = Neutral` decision, which is documented in the
resolver's doc comment. TASK.md's step edits honestly record the plan
change (scenario wiring superseded by requires).

Non-blocking findings:

- [x] R1.1 (MINOR) docs/ - the relation model is a new gameplay concept
  other tasks will build on (AI target selection 20260709-225727, future
  HUD pips), but no docs/ page records the model's semantics (Own/Hostile/
  Neutral matrix, the requires wiring, copy-at-spawn). Repo convention
  (AGENTS.md, thrust-balancing precedent) keeps such decisions in docs/.
  Suggested: a short docs/retros/20260709-faction-relations.md covering the
  matrix, where allegiance comes from, and the two consumers.
  - Response: fixed in 2cea663 - added docs/retros/20260709-faction-relations.md
    (matrix, allegiance sources, consumers, alternatives considered).
- [x] R1.2 (NIT) crates/nova_gameplay/src/sections/turret_section.rs:1710 -
  `spawned_projectile_allegiance` queries `(Entity, Option<&Allegiance>)`
  and then discards the entity with `let _ = entity;`. Query only
  `Option<&Allegiance>` like the torpedo test does.
  - Response: fixed in 2cea663 - query narrowed, `let _` gone.
