# Review: Split the sound bank - UiSfx in assets/, WorldSfx behind the base mod

- TASK: 20260717-101615
- BRANCH: task-20260717-101615-sound-bank-split

Reviewed the committed diff (e03c4130) with fresh eyes plus an independent
out-of-context pass (subagent over `git diff master...HEAD`), per the
shared-session rule. Load-bearing claims re-verified independently:

- bcs `SoundBank::load` is literally `assets.load(format!("sounds/{name}.wav"))`
  (bevy-common-systems src/audio/registry.rs:66) - exactly where the 4 UI wavs
  now live; `load_world_sfx_bank` is the single site of the
  `base/sounds/<name>.wav` convention, shared by production and every rig.
- `SoundBank<UiSfx>` / `SoundBank<WorldSfx>` are distinct resource types, so
  both banks coexist; the clean `--workspace --all-targets --features debug`
  check proves no consumer was left on the dead `SoundBank<NovaSfx>` type.
- Zero sound refs exist in webmods/, assets/mods/, examples/ - no mod
  referenced the moved UI files, so removing them from base `resources` breaks
  nothing (and `content_lint_gate` passing confirms tree coherence).
- Placeholder regen is byte-stable into the split layout (ran the script; only
  the 4 staged renames in git status, zero modifications).

Independent pass verified: exact key partition (UiSfx 4 + WorldSfx 12 = the
old 16, each exactly once); every consumer on the right bank (nova_menu +
objective_feedback -> UiSfx; audio.rs cues + salvage -> WorldSfx); the
every-key guard split covers all 16 variants; all rigs mirror production
loading; no stale NovaSfx/path claims in code, READMEs, CHANGELOG, wiki or
design docs; WorldSfx's doc is honest about being transitional ("do not add
new keys"); no cyclic deps (nova_assets -> nova_gameplay already existed).
Suites: nova_gameplay lib 534, nova_menu 61, nova_scenario (serde) 89, content
gates 4 - all green (independently re-run by the reviewer: 777 total).

## Round 1

- VERDICT: APPROVE

No findings. A purely structural split executed completely: partition exact,
paths consistent, tests preserved and extended (guards went 1 -> 2, none
weakened or deleted), docs swept including the 20260717-002228 leftovers
(CHANGELOG modding entry, wiki turret `fire_sound` field). The diff delivers
the task Goal as specified by spike 20260717-101524.
