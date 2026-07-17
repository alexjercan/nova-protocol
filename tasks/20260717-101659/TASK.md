# Salvage crate pickup sound as content on SalvageCrateConfig

- STATUS: OPEN
- PRIORITY: 24
- TAGS: spike,v0.7.0,audio,modding,feature


## Goal

`pickup_sound: Option<AssetRef<AudioSource>>` on `SalvageCrateConfig`; the
pickup cue site (nova_scenario objects/salvage.rs) already holds the crate
entity and plays the bank key today. gen_content/base content authors the
default; deleting this last `WorldSfx` key deletes the transitional WorldSfx
bank entirely - only `UiSfx` remains, completing the spike's end state.

## Notes

- Spike: tasks/20260717-101524/SPIKE.md. Depends on 20260717-101615 (bank
  split); as the WorldSfx-bank deleter it lands last of the family.
- Spike open question: user flagged salvage ownership as debatable; recommended
  mod-side (crates are scenario content). Reverting to a UiSfx key later is a
  one-file change.
- Stepless direction-level task: run /plan before /work.
