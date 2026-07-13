# Retro: Audio/SFX system

- TASK: 20260708-162011
- BRANCH: feature/audio-sfx-system (squash-merged as 6c2006b)
- REVIEW ROUNDS: 2 (round 1 APPROVE with 2 MINOR + 1 NIT; round 2 APPROVE after
  addressing the two MINORs)

See `tasks/20260708-162011/TASK.md` for what shipped and
`tasks/20260708-162011/NOTES.md` for the design. This retro is about how the
working went.

## What went well

- **Reuse-first paid off hugely.** Before writing anything, I checked what
  bevy-common-systems already had and found a complete audio layer
  (`SfxPlugin`, `SoundBank`, `SfxMasterVolume`) already at Nova's pinned rev.
  The task collapsed from "build an audio system" to "write the event->sound
  map" - ~300 lines instead of a subsystem. The prior spike's instinct to lean
  on bcs was correct and reading the bcs source first is what surfaced it.
- **Checked for decoupled seams before editing systems.** The plan said to wire
  turret/torpedo fire inside `shoot_spawn_projectile`. Grepping first showed both
  projectiles already carry a distinct spawn marker, so `On<Add, Marker>`
  observers gave the same cue with zero edits to the weapon systems. Smaller,
  decoupled diff for the same behaviour.
- **Pure helpers carried the only real logic.** `throttle` and `engine_volume`
  are free functions, so the parts a headless run can't verify (an audio device)
  are exactly the parts that are unit-tested. One of those tests immediately
  caught a real bug (see below).
- **Applied the last two retros' lessons proactively.** Used
  `cargo test --workspace` and built examples cold before timing the run, so
  neither bit this time. The compounding is visibly working.

## What went wrong

- **Root cause: planned against an assumed API shape.** The plan had the
  `SoundBank` riding the gated `bevy_asset_loader` `GameAssets` collection. When
  I got there, the bcs `SoundBank` exposes no public "build from existing
  handles" constructor - only `SoundBank::load(&assets, ...)`. The plan needed a
  mid-implementation correction (use `SoundBank::load` in `register_sounds`
  instead). It seemed right at planning time because the gated collection is how
  every other Nova asset loads; I just never checked the registry's constructor
  surface. Lesson: when a plan step hinges on a library type's constructor/API
  shape, confirm that shape while planning, not while implementing.
- **The plan edit stranded itself in the main checkout.** I wrote the Steps into
  `TASK.md` on `master` during `/plan`, then sprouted the worktree - which cuts
  from committed `HEAD`, so the uncommitted plan stayed behind and the fresh
  worktree had the step-less task. Had to copy the file into the worktree and
  `git restore` master. Root cause: in a single plan->work session the plan
  output is uncommitted when the sprout happens. Lesson: commit the plan (or
  write it directly in the worktree) before sprouting.
- **Two feel findings escaped implementation** (R1.1 explosion stacking, R1.2
  engine hum pinning to max). Both are the same class: I reasoned about single
  events and didn't think through the aggregate case (a whole ship dying marks
  every section at once; a ship maneuvers on several thrusters at once). The
  turret/impact throttles show I *had* the "bursts happen" instinct - I just
  didn't apply it to explosion or to the thruster sum. Enumerate the many-at-once
  case for every per-event cue, not just the obviously spammy ones.

## What to improve next time

- Confirm a library type's constructor/API surface during planning when a step
  depends on it.
- Commit (or worktree-author) a plan before sprouting, so the Steps travel with
  the branch.
- For any per-event effect, ask "what happens when N of these fire in one
  frame?" before calling it done.

## Action items

- [x] Promoted the standout build lesson to `docs/development.md`: reuse the main
  checkout's warm `target/` from a sprout worktree via `CARGO_TARGET_DIR`, which
  turned a cold multi-minute Bevy build into ~27s. This is high-value for every
  future Nova worktree and was not written down anywhere.
- [ ] Optional follow-up (not filed as a task; NIT R1.3): guard `register_sounds`
  with `resource_exists` if scene reloads (re-entering Processing) become common,
  to avoid re-loading the SoundBank handles each time.
- [ ] Standing lesson still pending promotion (from the two prior retros +
  applied cleanly here): `cargo test --workspace`, and build-cold-then-time-run
  for headless examples. Noted in `docs/development.md` alongside the target-dir
  tip now, so it is no longer only in retros.
