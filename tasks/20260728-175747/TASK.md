# Contextual HUD: show-by-relevance + grow-in-use + On/Cinematic

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.9.0,ui,hud,gameplay

## Flow State

- FLOW STEP: DONE
- PLAN STATUS: APPROVED

## Story

Per the accepted ruleset (SPIKE.md D3/D5, demo 2 `hud_rework_poc.html`),
make importance-driven visibility automatic on top of the restyled HUD
(20260728-175742): idle cruise is near-empty, elements appear while their
situation is live, the element in direct use grows and settles back, and
the `~` triple All/Minimal/None collapses to On/Cinematic. Much of the
show/hide already exists per-element (the HUD map confirms mode chip,
reticle, inset, dwell ring etc. are state-driven); this task fills the
gaps, adds the emphasis layer, and rewires the level control.

## Steps (re-planned 2026-07-28 from demo 2's reflect() ruleset + HUD map)

- [x] Encode the ruleset (demo 2 reflect(), mapped to real state sources):
      - idle cruise: velocity shader + speed chip + dim dock + status bar
        only. Gap to close: ammo gauges currently show whenever a weapon
        has `SectionAmmo` - gate them on weapons-hot OR any-group-low.
      - AP burn (`Autopilot` engaged): mode chip + destination marker +
        readout (already conditional); NEW: speed chip emphasis while
        burning; dock GOTO chip hot / STOP available.
      - combat lock (`CombatLock` Some): reticle + DST/CLS readout +
        target inset (already conditional); dock RADAR hot; goto cue only
        while locked and no maneuver engaged (current behavior, keep).
      - weapons hot (`WeaponsHot`): ammo groups shown; lock readout
        emphasized; reticle pulses while firing.
      - low ammo: near-empty groups warn-pulse (state from 175742) and
        force the ammo row visible.
      - objective posted (`GameObjectives` diff): chip pops (~1.2s
        emphasis) then settles to the slow breath; reconcile with the
        existing objective_reveal card so the pop rides the card's tuck,
        not a second animation.
      - comms (`StoryFeed` push): card emphasis on arrival (~0.9s), ~5s
        dwell, fade (comms_panel already fades; align dwell + cap).
- [x] Grow-in-use emphasis: one shared mechanism (component holding target
      scale + optional settle timer, one system driving Transform scale on
      UI nodes) with demo 2's values: speed 1.14, lock readout 1.12, dock
      hot chip 1.08, objective pop 1.16/1.2s, comms 0.9s. Continuous while
      the driving state holds; timer for one-shots.
- [x] `~` control: replace `HudVisibility { All, Minimal, None }` with
      `{ On, Cinematic }` (mod.rs): On = full contextual HUD, Cinematic =
      clean screen (old None). Keep the HudTier machinery (Status tier,
      HudNovaOsExempt and the every-frame indicator enforcement still
      carry the None-clearing + NOVA OS exemptions). Grep EVERY
      HudVisibility consumer and migrate each (new-entry-into-state rule);
      keyboard Backquote + gamepad Select cycle stays.
- [x] Migrate the mod.rs level tests (enumerated in the map:
      `backquote_cycles_all_minimal_none_all`,
      `tiers_hide_and_restore_across_levels`,
      `status_tier_shows_through_minimal_and_hides_at_none`, the NOVA OS
      exemption family) to the two-level model; rename per behavior.
- [x] Settings CONTROLS reference: the `HUD detail ~` row text updates to
      On/Cinematic wording (build_settings_body keybind rows).
- [x] Docs sweep (keep-docs-in-sync): wiki hud.md (levels section rewritten
      to On/Cinematic + contextual rules), getting-started / tutorial.html
      if they name the three levels, CHANGELOG [Unreleased].

## Definition of Done (re-planned 2026-07-28)

1. test: App-driven tests per rule - situation on -> element shown/grown,
   situation off -> reverted; one-shots settle on virtual-time advance
   (objective pop, comms dwell). At minimum: ammo gating, speed-chip
   emphasis on burn, dock hot states, objective pop-settle, comms dwell.
2. test: `backquote_cycles_on_cinematic` replaces the three-level cycle
   test; Cinematic clears the HUD (old None pins reused); and
   (cmd: `grep -rn "Minimal" crates/nova_gameplay/src/hud` prints 0 hits,
   counts recorded here).
3. cmd: `cargo run -p nova_probe -- run <playable example>` passes with the
   contextual HUD active (before/after report attached per the probe
   skill).
4. manual: owner playtest verdict - idle cruise is near-empty, the right
   things surface at the right time, Cinematic gives a clean screen.

## Notes

- Layered strictly on 20260728-175742 (restyle lands first; this task adds
  behaviour only).
- State sources confirmed by the HUD map (2026-07-28): `Autopilot`,
  `CombatLock`, `WeaponsHot`, `RadarState`, `SectionAmmo`/`SectionReload`,
  `GameObjectives`, `StoryFeed` - all already read by HUD systems; no new
  gameplay state is needed.
- Allegiance triangles (always-on over every ship) are NOT in the accepted
  ruleset either way - leave them as-is and put the question to the owner
  at the playtest gate (candidate: dim or hide in idle cruise). Recorded
  so it is a conscious open point, not scope creep.
- Emphasis must not fight the screen-indicator projection systems - the
  scale system runs on the chip nodes, not the projected anchors; check
  ordering against `apply_hud_visibility` (PostUpdate) when wiring.
- Depends on: 20260728-175742.

## Implementation notes (2026-07-29)

Full design record: `NOTES.md`. Deviations from the plan, recorded here so
review reads them as decisions, not misses:

- The CONTROLS reference had NO `HUD detail ~` row to reword - the HUD level
  was never in `keybind_reference()`. Added a SYSTEM row
  `HUD (On / Cinematic)` / `` ` `` / Select instead.
- The comms DWELL was not retimed to the demo's 5 s (the panel's authored
  per-line dwell drives nova_assets' scenario pacing); only the ~0.9 s arrival
  emphasis was adopted. See NOTES.md.
- `HudVisibility::shows()` dropped its `HudTier` parameter: at two levels every
  tier answers the same. `HudTier` stays as the HUD-managed marker + vocabulary.
- DoD 2 grep counts (2026-07-29):
  `grep -rn "Minimal" crates/nova_gameplay/src/hud` = 60 hits, of which 59 are
  `MinimalPlugins` (bevy test rigs) and 1 is the deliberate historical sentence
  in `HudVisibility`'s doc explaining the recut. Zero HUD-level `Minimal`.
  Repo-wide `HudVisibility::{All,Minimal,None}` outside `tasks/` = 0.
- DoD 3: `cargo run -p nova_probe -- run playable` -> OK,
  `probe-runs/ff59c72c/playable/report.html` (fps SKIPPED - no baseline).
- Not visually verified: the ammo gauges APPEARING on weapons-hot - no
  harnessed example fires the player's guns (all example ships are
  `infinite_ammo: true`). The closed state WAS eyeballed on a finite-ammo ship
  (shakedown_run). Covered by App-driven tests + the owner playtest.
- Open for the playtest gate: allegiance triangles (left as-is per the plan),
  and whether the lock readout inheriting the reticle's firing pulse reads as
  too much motion.
