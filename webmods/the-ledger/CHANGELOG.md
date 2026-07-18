# Changelog

All notable changes to The Ledger campaign mod. Versions are the `meta.version`
in `the-ledger.bundle.ron`; the portal keeps every published version.

## 1.5.0

- Re-skin onto the base-game racer/cargo prototypes now that ships are reusable
  prototypes shared by mods and menus.
- Promote `craft_cargoa` to a base prototype; use it for neutral ships.

## 1.4.0 - The beat-sheet pass

- Pacing rework across all chapters: announce, breathe, arrive.
- Auditor bay side-mounted; content lint catches section overlaps.
- Auditor gains a light gun; balance-audit acknowledgments added to tooling.
- Salvage pickup is now crate content (the `WorldSfx` bank is gone); the
  thruster's engine hum is a per-handle section sound.

## 1.3.0 - Chapter 2 difficulty rework

- Chapter two split into two fair acts across two scenario files (claim
  jumpers, then the heavies) so a death retries the current act, not the whole
  chapter. Breaking wave one is a checkpoint.

## 1.2.0

- Reference base art via `self://` + `dep://base` after base art moved under
  `assets/base/`; declare `base` as a dependency.
- Per-target impact and destroy sounds for sections, asteroids, and torpedoes.

## 1.1.0

- Fix chapter one: unbury the quota crates from the wreck asteroids.

## 1.0.0

- First release: the four-chapter alt storyline campaign on the mod portal.
  Five scenario files (chapter two plays in two acts), chained with
  `NextScenario`; chapter one is the picker entry, later chapters hidden and
  reached by playing. Hand-written RON on base-game assets and section
  prototypes, using only shipped scenario vocabulary.
