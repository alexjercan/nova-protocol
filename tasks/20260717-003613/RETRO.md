# Retro: Diegetic HP v1 - per-section mesh damage tint/glow + retire the generic bar

- TASK: 20260717-003613
- BRANCH: spike/diegetic-hp (landed 4e3373f8); warning follow-up on
  fix/damage-tint-warning
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES -> R2 APPROVE)

What/why/evidence live in TASK.md and NOTES.md; this is process only.

## What went well

- Reading the shipped content data (`base.content.ron`) before writing code
  caught that the ship is all gltf `WorldAssetRoot` meshes, not cuboids - the
  spike's "v1 = cuboid colour swap" would have tinted nothing. The mechanism was
  redesigned (per-section gltf material cloning) before a wrong line was written.
- Dropping the hunt for `WorldAssetRoot`'s internals in favour of an
  observable-state hook (`Added<MeshMaterial3d>`) unblocked the capture system
  and made it robust to async gltf loading - a better design than the
  scene-ready hook the plan flagged as verify-first.
- The R1 self-review independently re-verified the one risky interaction (do
  fired projectiles/muzzle meshes get wrongly tinted?) instead of trusting the
  diff, and confirmed they resolve to no section. That is the shared-session
  blind-spot mitigation working.

## What went wrong

- Landed a compiler warning (unused `#[must_use]` `Assets::insert` Result in a
  test). Root cause: I verified with `cargo test ... | grep -E 'error|test
  result'`, which discards warning lines - so the check looked green. It
  surfaced only via the editor's LSP diagnostic AFTER the squash-land, forcing
  this follow-up sprout. This is `warnings-clean-before-land` (now x2) - a lesson
  that already existed on the ledger and that I did not apply, because I did not
  read `LESSONS.md` at the start of the cycle as flow prescribes.
- The plan carried the spike's cuboid assumption as fact; it was only corrected
  at work-time by reading the data. It worked out, but per
  `verify-first-plan-steps` (now x8) the verification belonged in the plan, and
  the plan should have grepped `base.content.ron` for the render path.

## What to improve next time

- Read `LESSONS.md` at the START of the cycle (flow step 3.1). The exact
  trap I hit was already written down.
- In the verify step, never filter build/test output down to `error`/`test
  result`. Run a warnings-surfaced build (`cargo build`/`clippy`) and read the
  warnings before landing.
- When a task's mechanism depends on shipped content (render path, asset kinds),
  grep the `.content.ron` during planning, not implementation.

## Action items

- [x] Fixed the landed warning on fix/damage-tint-warning.
- [x] Bumped `warnings-clean-before-land` (x2) and `verify-first-plan-steps`
  (x8) in LESSONS.md with this cycle's variants.
- [ ] Follow-up (not a code task): on-ship tint legibility playtest on the real
  gltf ship - tracked in TASK.md/NOTES.md, the spike's open camera question.
