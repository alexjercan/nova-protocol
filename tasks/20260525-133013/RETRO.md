# Retro: Add spawn-less visual mode for low-end machines

- TASK: 20260525-133013
- BRANCH: spawnless-visual-mode (landed on master as 08479947)
- REVIEW ROUNDS: 2

See TASK.md for what/why and REVIEW.md for the findings; this is process only.

## What went well

- The out-of-context review agent earned its keep. Implementer and reviewer
  were the same session, so I ran an independent review sub-agent on the diff
  with no prior context. It surfaced the exact MAJOR (R1.1) I had just found in
  my own verification pass, and independently re-derived every load-bearing
  claim (PostUpdate chain ordering, the `Option`-defaulting gate logic, scatter
  math). Two blind paths converging on the same defect is far stronger evidence
  than one.
- Honored the baseline dependency gap instead of faking it. The task says "tuned
  against the frame-time baseline", but that baseline task (20260716-123551) is
  OPEN with no published numbers. Rather than invent baseline-tuned constants, I
  fixed the *shape* (what each tier skips) and labelled the exact fractions
  "provisional pending the baseline" in code and CHANGELOG. Respects
  `advertised-but-unwired` and `verbosity-invites-fabrication`.
- Followed the existing pattern rather than a parallel one: `GraphicsBudget`
  derived at the single `apply_graphics_quality` seam, read at spawn sites,
  mirrors how the preset already drives `JuiceSettings` - no new policy scattered
  across sites.

## What went wrong

- R1.1: I gated the three `insert_*` particle-spawn observers but missed the two
  paired per-shot reset observers (`on_torpedo_launch_effect`,
  `on_projectile_marker_effect`). Those look up the spawned effect entity and
  `EffectSpawner::reset()` it every shot; with the spawn gated on Low the lookup
  fails and hits an `error!` branch - so a valid Low config error-spammed on
  every launch/shot, in the very mode meant for low-end machines.
- Root cause: the code-mapping sub-agent's sweep was scoped to *spawn* sites
  (`ParticleEffect::new` / `EffectAsset` inserts) and confidently reported "three
  observers, all gated". That was true and complete for spawns - but a particle
  system is a producer/consumer pair, and the *consumer* side (systems that later
  look up and drive the spawned entity) was never in the sweep. I gated exactly
  what the map surfaced. My own follow-up grep for `effect_spawner.reset()` is
  what caught the consumers - the mapping agent's framing had quietly narrowed
  the question.

## What to improve next time

- When a flag skips *producing* an entity/asset, sweep for its *consumers*
  before calling the gate done: grep for every lookup of the same marker /
  component and confirm each tolerates the producer having been skipped (early
  return, not an error path). A gate on the producer is only half a gate.
- Treat a sub-agent's "find all X" as answering exactly the X you framed. A
  spawn-site sweep is not a particle-system sweep; when the thing has two sides,
  ask for both explicitly.

## Action items

- [x] Added `gate-producer-and-its-consumers` to docs/LESSONS.md.
- No follow-up code task: the provisional tuning constants are intentionally
  deferred to the baseline task (20260716-123551), which already carries that
  scope; nothing new to file.
