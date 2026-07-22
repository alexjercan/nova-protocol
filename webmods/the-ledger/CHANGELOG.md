# Changelog

All notable changes to The Ledger campaign mod. Versions are the `meta.version`
in `the-ledger.bundle.ron`; the portal keeps every published version.

## 1.6.0

- Pacing pass across the chapters: each opens with a clock-paced briefing
  conversation, the first objective lazy-posts only once the briefing hands
  off (no objective shares a frame with a conversation), and comms beats
  land a beat apart (announce -> arrive -> confirm -> breathe) with `dwell`
  holds so lines are readable. Chapter two and the burn ending defer their
  Victory overlay a beat behind the win comms line.
- Chapter three deepened: a clock-paced opening act, per-gate breathers along
  the corridor, and a NEW debris-pinch hazard between NAV-1 and NAV-2 - two
  invulnerable boulders tighten the lane to a threadable gap, a piloting test
  that makes the debris load-bearing (the NAV-2 Magpie ambush still stands,
  and both stay optional to a careful pilot).
- Chapter four endings now diverge, not just in flavor: SELL docks the box
  and the sale calls the Auditor down (now telegraphed - a warning line and
  an 8s engage grace so the gunship reads before it is lethal) for a payday
  at a price; BURN torches the box so nothing is left to collect and the
  Auditor never comes - no fight, but no payout either. Each path reaches its
  own terminal Victory. Dead burn-path Auditor handler and its stale lint ack
  removed.
- Per-chapter skybox identity: chapters pick a deliberate starting cubemap,
  and `SetSkybox` accents mark key beats - the chapter-one crate reveal, the
  chapter-three pinch, and the chapter-four sell path. Chapter two and the
  burn path keep their sky unchanged by design.

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
