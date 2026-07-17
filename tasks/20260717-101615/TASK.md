# Split the sound bank: UI sounds stay in assets/, world sounds move behind the base mod boundary

- STATUS: OPEN
- PRIORITY: 34
- TAGS: spike,v0.7.0,audio,modding,refactor


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
