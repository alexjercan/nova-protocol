# Salvage crate pickup sound as content on SalvageCrateConfig

- STATUS: IN_PROGRESS
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

## Plan (2026-07-17, grounded)

Verified: SalvageCrateConfig (salvage.rs:44, size + area_radius) + bundle fn
:51; the cue (:148-176) has the crate entity in hand; 2 authoring sites in
nova_assets scenario builders + salvage.rs's own test rigs.

### Steps

- [x] `pickup_sound: Option<AssetRef<AudioSource>>` on SalvageCrateConfig;
      bundle snapshots `SalvageCratePickupSound`; cue resolves it
      (authored-or-silent, crate entity already in hand).
- [x] DELETE the WorldSfx bank end to end: enum, WORLD_SFX_FILES,
      load_world_sfx_bank, the world half of register_sounds, guard test,
      prelude exports, every import. Repo-wide grep for WorldSfx afterwards
      must be zero (code+docs).
- [x] Builders author self://sounds/salvage_pickup.wav on both crate sites;
      regen; parity + lint green; sweep webmods for salvage crates.
- [x] Tests: pickup tests author the sound; authored-plays/unauthored-silent
      pair with delivery guard.
- [x] Docs: sounds README (delete the bank section - everything authored now;
      intro reword), wiki salvage/scenario page if it lists crate fields,
      CHANGELOG, mod-binary-resources.md (end state reached), spike fix record
      + Next-steps closure, audio.rs module header. Prose-grep "WorldSfx" and
      "bank" across assets/ web/ docs/ crates/.
- [x] Verify: fmt; workspace all-targets; nova_gameplay lib + nova_scenario
      (serde) + gates.
