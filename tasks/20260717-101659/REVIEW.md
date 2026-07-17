# Review: Salvage pickup sound + WorldSfx bank deletion

- TASK: 20260717-101659
- BRANCH: task-20260717-101659-salvage-pickup-sound

Reviewed the committed diff (388b5dc6 + c4d7a2db) fresh + independent
out-of-context pass. Verified (mine + reviewer's independently):

- Deletion completeness: WorldSfx enum, WORLD_SFX_FILES, load_world_sfx_bank,
  the guard test, the prelude exports and the world half of register_sounds
  are all gone; the only remaining mentions are past-tense records. No code
  reads a SoundBank<WorldSfx> anywhere.
- Cue correctness: crate identification handles both collision orderings; the
  DingedCrates dedup still books EVERY pickup (authored or not), so silence
  never corrupts the dedup; player gate intact.
- Silent-regression sweep: base scrapyard (1) + shakedown (3) crates author
  self:// refs via regenerated content, and the-ledger ch1's 4 crates author
  dep://base refs - the implementer's own webmod sweep (the 101641 lesson)
  caught the ledger gap in-cycle, before review. salvage_pickup.wav remains in
  base resources (content references it now).
- Tests: authored-ding delivery guard + NEW unauthored-silent + multi-collider
  dedup + non-player gate all meaningful; nothing weakened.
- Suites: nova_gameplay lib 546, nova_scenario (serde) 92, parity + lint +
  webmods_validation green, workspace all-targets clean.

## Round 1

- VERDICT: APPROVE

No findings. The spike's end state is fully realized: every world sound is an
authorable AssetRef on its owning content config; only the UiSfx bank remains.
