# Retro: Weapon-section one-shot sounds

- TASK: 20260717-101624
- BRANCH: task-20260717-101624-weapon-sounds (squash-landed 26d7264c)
- REVIEW ROUNDS: 2 (APPROVE at round 2)

Short retro for a clean cycle. See TASK.md/REVIEW.md for detail.

## What went well

- mirror-sibling-resolve-site applied at PLAN time found the torpedo template
  (`TorpedoSectionSpawnerEffect` on the spawner + the projectile back-ref)
  before any code was written - the launch sound landed exactly on the
  established seam, no redesign round needed.
- The authored-or-silent flip's silent-regression risk was swept BEFORE review
  (prototypes in mods/webmods/examples, .rs config literals, test-vs-prod
  boundaries), so the independent reviewer confirmed rather than discovered.

## What went wrong

- The same failure shape as last cycle in miniature: rewriting cue BEHAVIOR
  (dropping the bank fallback) left three rustdoc sites still describing the
  old behavior - caught by the independent reviewer (R1.1, one round). Root
  cause: the mechanical enum rename swept identifiers but not PROSE; a
  behavior flip needs a prose grep for the old model's words ("falls back",
  "bank default"), not just the old symbols.

## What to improve next time

- After changing a behavior, grep for the old behavior's VOCABULARY in
  comments/docs (not just renamed identifiers) before calling it done.

## Action items

- None new; the keep-docs-in-sync lesson (x4) already covers doc staleness -
  this is its rustdoc-prose variant, noted here without a separate slug.
