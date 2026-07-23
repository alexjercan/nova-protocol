# Changelog

All notable changes to The Ledger campaign mod. Versions are the `meta.version`
in `the-ledger.bundle.ron`; the portal keeps every published version.

## 1.11.0

- The Raid (chapter five) tuning from the first playtest: the planetoids' pull
  is much gentler now, so the small ships handle the field instead of getting
  dragged around, and the Magpie base holds its post - it gained station-keeping
  RCS thrusters and a tight tether so it no longer drifts off in the gravity. The
  base also carries a lighter turret load. Torpedoes now fire on the R key
  (freeing the mouse). (The chapter is also temporarily visible in the Scenarios
  picker for testing; it will go back to fight-only before release.)

## 1.10.0

- A reward finale: THE RAID (chapter five). Win the chapter-four fight - break
  the Auditor gunship on the SELL path - and the campaign no longer just ends.
  Your payday buys a gunship, and for the first time in the whole story you fly
  a capital ship with torpedoes (guns on the left mouse, torpedo tubes on the
  right). With two of Vesh's wing flying escort, you raid the Magpies' forward
  base among the asteroids and planetoids: a real multi-section station and four
  fighters to break. Crack the base and clear its defenders to close the account
  for good. The BURN (no-fight) ending is unchanged - it stays terminal, so the
  raid is the reward for choosing to fight.

## 1.9.0

- Chapter three overspeed now gives you a real reaction window before it wakes
  the pickets. The first breach over 8 u/s still warns instantly (harmless).
  But the SECOND strike no longer catches you the instant you cross the line: a
  fresh breach after slowing starts a 3.5-second countdown, Vesh shouts the
  last-chance warning, and the Magpies only wake if you HOLD the burn past the
  window. Ease back under the rearm band in time and the countdown cancels - the
  run stays dark and a later breach simply starts a fresh window. The trip is
  now a sustained mistake, not a twitch of the throttle.

## 1.8.0

- Chapter three stealth gains a fifth way to blow the run: burning too hot.
  Running over 8 u/s through the channel is noise the pickets hear. It is
  warn-then-trip - the first time you push past 8, Vesh calls it and tells you
  to throttle down (the pickets stay asleep); if you slow back under 7 and then
  gun it again, that fresh breach wakes both Magpies, same as tripping a watch
  zone or painting one. A single continuous burn only ever earns the one
  warning, so an accidental nudge over the line will not end the run. The
  channel now rewards a genuinely slow, dark approach, not just a wide berth
  around the watch bubbles.

## 1.7.0

- Chapter three is now a real stealth run. The two channel Magpies no longer
  spawn hostile on the NAV-2 trigger: they are present from the first frame
  as NEUTRAL pickets patrolling the open flanks of the debris pinch, and
  they only wake - both flipping hostile at once (`SetAllegiance`) - if the
  player provokes them by entering one of the two picket-watch detection
  zones (radius 24, centred 55u off the lane on each flank) or by
  red-locking either picket. The pinch gap is the one lane the watch does
  not cover: thread it slow and you slip past; swing wide around the wrecks
  and you are seen. Reaching the yard undetected earns Vesh's "nothing on
  their scopes" payoff line with the Victory a beat behind it and the
  pickets still asleep; waking them is the same fight (and the same win at
  the yard) as before. Comms retuned to read true on both paths: the
  briefing calls out the pickets, the pinch warning sells the blind spot,
  and the mid-run breather reports whether the flanks are still cold.

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
