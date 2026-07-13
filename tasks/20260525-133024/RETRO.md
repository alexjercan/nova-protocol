# Retro: Torpedo bay launch particles

- TASK: 20260525-133024
- BRANCH: feature/torpedo-launch-particles (squash-merged as a464768)
- REVIEW ROUNDS: 1 (APPROVE, three NITs, all addressed)

Short retro for a smooth, single-round cycle. What shipped is in the task's
Resolution and `tasks/20260525-133024/NOTES.md`.

## What went well

- **Found the sibling pattern before writing anything.** The task said "shooting
  particles"; the win was recognizing the turret already had exactly this
  (`insert_turret_barrel_muzzle_effect` + `on_projectile_marker_effect`) and
  mirroring its shape - spawn-on-command effect parented to the firing point,
  triggered from the projectile `Add` observer via `reset()`. The whole feature
  became "port the turret muzzle effect to the torpedo bay," which is why review
  found zero correctness issues. This is the fourth consecutive repo cycle where
  reusing a previously-reviewed shape produced a clean-or-near-clean round; the
  pattern is now the default move.
- **The frame assumption was checked, not assumed.** The one genuinely
  bug-prone spot was passing a world-space `normal` into an effect parented to a
  rotated spawner. Rather than trust it, I confirmed neither turret, blast, nor
  the new effect sets `SimulationSpace`, so all use hanabi's default (Global),
  which is precisely why a world-space direction is correct. The
  shooter-frame-lead retro's lesson ("when new data flows into old math,
  re-derive the assumptions") generalized cleanly to "when copying an effect,
  re-check its coordinate frame."
- **The range runbook paid off verbatim.** Build the example binary once, run it
  directly with `BEVY_ASSET_ROOT=$PWD DISPLAY=:99 BCS_AUTOPILOT=1` - reached
  Playing, fired repeatedly, cycle complete no panic, first try. No `cargo run`
  recompile trap, no asset-root hang. The com-range/component-lock retros'
  runbook is now just how you smoke-test a section.

## What went wrong

- **Missed a non-defaulted construction site on the first build.** Adding
  `launch_effect` to `TorpedoSectionConfig` compiled `nova_gameplay` fine but
  broke `nova_assets`, which builds the config with explicit fields (no
  `..default()`). Root cause: checked only the crate I was editing, not all
  construction sites of the struct I widened. Cheap to catch (the example build
  failed loudly) but avoidable: a `grep` for the struct's other literal
  constructions before building would have caught it in the plan, not the build.

## What to improve next time

- When adding a field to a config/struct that is constructed with explicit
  literals anywhere, `grep` for all `StructName {` construction sites (not just
  the `Default` impl) before compiling - widening a struct is a repo-wide edit,
  not a local one.

## Action items

- None. Visual tuning of the burst (color/size/spread at gameplay distance) is
  the one unverifiable-by-headless property and is flagged as a playtest item in
  the task and the doc, consistent with how prior visual tasks handled feel.
