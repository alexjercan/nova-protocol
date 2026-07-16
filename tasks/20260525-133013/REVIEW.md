# Review: Add spawn-less visual mode for low-end machines

- TASK: 20260525-133013
- BRANCH: spawnless-visual-mode

## Round 1

- VERDICT: REQUEST_CHANGES

Reviewed the full diff against master, TASK.md, the bevy_common_systems v0.19.0
event-chain source, and the surrounding particle/reset observers. Load-bearing
claims independently re-verified (shared implementer/reviewer session): the
PostUpdate chain ordering, the `Option`-defaulting gate logic, the scatter
thinning math, and the provisional-tuning honesty all hold. One MAJOR defect,
introduced by the change.

- [ ] R1.1 (MAJOR) crates/nova_gameplay/src/sections/torpedo_section/render.rs:473
  (`on_torpedo_launch_effect`) and crates/nova_gameplay/src/sections/turret_section.rs:1180
  (`on_projectile_marker_effect`) - the two ungated per-shot reset observers. They
  look up the pre-spawned child effect entity and call `EffectSpawner::reset()`.
  On Low, the gates in `insert_torpedo_spawner_effect` and
  `insert_turret_barrel_muzzle_effect` return early *before inserting that child*,
  so the lookup fails and hits the `error!` path (render.rs:510,
  turret_section.rs:1218) on EVERY torpedo launch and turret shot. No panic (both
  `let Some(..) else { return }`), but a valid Low-tier config emits an error-level
  log line per shot - misleading, and needless logging cost in the very mode meant
  for low-end machines. The paired reset observers were never taught that "no
  effect entity" is the intended Low state. Missed because no test exercises the
  Low-tier launch/muzzle path (only the scatter path is tested). Fix: gate the two
  reset observers on the same `GraphicsBudget.particles` and early-return before
  the lookup, matching the three spawn-site gates.
  - Response: Fixed in e36615e0. Added `budget: Option<Res<GraphicsBudget>>` to both
    observers with an early `return` right after the entry `trace!`, before the
    `q_projectile`/`q_effect` lookups - so on Low nothing is looked up and no
    `error!` fires. High/Medium and the settings-less (absent budget) path are
    unchanged. Compiles warning-clean under `--features debug --all-targets`.

### What checks out (verified, not assumed)

- Ordering/staleness: the v0.19.0 chain is `world_to_state_system -> queue_system
  -> state_to_world_system`, `.chain()` in PostUpdate. `apply_graphics_quality`
  writes the budget in Update, so PostUpdate reads a same-frame-fresh value.
  `world_to_state_system` copies it via `get_resource().copied().unwrap_or_default()`
  - no panic when absent.
- Gate logic: `!budget.as_deref().map_or(true, |b| b.particles)` is correct in all
  three cases (absent -> full quality; particles=true -> spawn; particles=false ->
  skip).
- Spec: particles gated off on Low, scatter thinned on Low/Medium, tiers observably
  distinct, no new UI/persistence, folded into the single `apply_graphics_quality`
  seam. Nothing the spec asked for is missing (R1.1 is a side-effect quality bug).
- Tests: meaningful - `scatter_action_thins_the_field_on_low_graphics` drives the
  real world_to_state/action/state_to_world path and would fail if the gate were
  removed (10 vs 20); the pure-policy and min-1/empty edges are covered; the apply
  seam write is verified.
- Design: mirrors the existing `JuiceSettings` pattern; exactly the four
  `ParticleEffect::new` spawn sites (across three `insert_*` observers) exist and
  all are gated; thinning by dropping the tail of the deterministic seeded sequence
  keeps positions stable.
- Honesty: baseline task 20260716-123551 is OPEN with no published numbers; code
  and CHANGELOG consistently say "provisional pending the frame-time baseline",
  never claiming baseline-tuned.

## Round 2

- VERDICT: APPROVE

- [x] R1.1 - resolved in e36615e0. Verified the diff: both `on_torpedo_launch_effect`
  (render.rs) and `on_projectile_marker_effect` (turret_section.rs) now take
  `budget: Option<Res<GraphicsBudget>>` and early-return on
  `!budget.as_deref().map_or(true, |b| b.particles)` immediately after the entry
  `trace!`, ahead of every lookup. On Low the observers no-op silently (no `error!`);
  High/Medium and absent-budget paths are byte-for-byte unchanged. `cargo check
  -p nova_gameplay --features debug --all-targets` is warning-clean.

No new findings. The change delivers the Goal, tiers stay observably distinct, and
the provisional-tuning framing is honest.
