# Split the sound bank: UI sounds stay in assets/, world sounds move behind the base mod boundary

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: spike, v0.7.0, audio, modding, refactor

## Goal

Make the sound ownership boundary structural: a small `UiSfx` bank (menu_select,
ui_toggle, objective_new, objective_complete) loaded from root `assets/sounds/`
(move those 4 wavs BACK out of `assets/base/sounds/` and OUT of base
`resources`), and a transitional `WorldSfx` bank for the remaining 12 world
sounds (still `base/sounds/` paths) that later per-family tasks shrink to
nothing. Repoint nova_menu + hud/objective_feedback to `UiSfx`. After this task,
a bank key means engine chrome; an `AssetRef` content field means mod content.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md (ownership table + architecture; this is
  step 1, the foundation the other five family tasks depend on).
- gen-placeholder-sounds.py writes all wavs to one dir today - it must split its
  output too.
- Stepless direction-level task: run /plan before /work.

## Plan (2026-07-17)

Sweep result (untruncated, planning pass): exactly 7 .rs files touch
NovaSfx/SoundBank - audio.rs (61), hud/objective_feedback.rs (13),
nova_menu/lib.rs (8), nova_scenario/objects/salvage.rs (6), nova_assets/lib.rs
(5), turret_section.rs (2), nova_gameplay/lib.rs prelude (1). No examples.

### Steps

- [x] Split the enum (audio.rs): `UiSfx` { MenuSelect, UiToggle, ObjectiveNew,
      ObjectiveComplete } + `WorldSfx` { ThrusterLoop, TurretFire,
      TorpedoLaunch, Explosion, Impact, LockOn, LockOff, SafetyOn, RadarDeny,
      SalvagePickup, DryFire, RadarRetarget }; NOVA_SFX_FILES splits into
      UI_SFX_FILES (4) + WORLD_SFX_FILES (12); update the prelude export
      (nova_gameplay/lib.rs:65) and grep docs/comments for the dead NovaSfx name.
- [x] register_sounds (nova_assets) inserts BOTH banks: `SoundBank<UiSfx>` via
      `SoundBank::load` (the bcs root `sounds/<name>.wav` convention - restored,
      it IS the engine-chrome convention) and `SoundBank<WorldSfx>` via
      `load_paths` from `base/sounds/<name>.wav`.
- [x] Move the 4 UI wavs back: `git mv assets/base/sounds/{menu_select,
      ui_toggle,objective_new,objective_complete}.wav assets/sounds/`; remove
      the 4 from base.bundle.ron `resources`; split the README (new
      assets/sounds/README.md for UI cues, base README keeps the 12).
- [x] Repoint consumers: audio.rs world cues + salvage.rs -> WorldSfx bank;
      objective_feedback.rs + nova_menu -> UiSfx bank. Volume constants and all
      throttle/attenuation logic unchanged - this task is purely structural.
- [x] gen-placeholder-sounds.py writes 4 to assets/sounds/, 12 to
      assets/base/sounds/.
- [x] Tests: update every rig inserting SoundBank<NovaSfx> (audio.rs rigs;
      objective_feedback.rs:288; nova_menu:4047; salvage.rs:424) to the correct
      new bank; split every_nova_sfx_key_has_a_file into per-enum guards. Run
      affected test targets + workspace all-targets check, reading cargo OUTPUT
      not pipe exit (LESSONS: piped-cargo-masks-exit-code,
      crate-solo-tests-miss-unified-features).
- [x] Docs incl. the 002228 leftovers (LESSONS: keep-docs-in-sync-with-code):
      CHANGELOG player-facing lines (mods can ship + reference sounds; UI
      sounds are engine chrome), wiki modding guide if it lists authorable
      fields, architecture.md assets para, mod-binary-resources.md, CREDITS.md,
      spike 20260717-101524 Fix record entry.
- [x] Verify: fmt; content_ron_parity + content_lint_gate (base resources
      shrank by 4); full check surface green.
