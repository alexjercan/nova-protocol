# ch5 raid tuning: shrink planetoid gravity/SOI, base holds station, fewer turrets, torpedo on R, unhide (content)

- STATUS: OPEN
- PRIORITY: 60
- TAGS: v0.8.0, content, scenario, playtest

## Story

Umbrella 20260723-200636. Playtest tuning of `ledger_ch5_the_raid.content.ron`
(the reward raid from task 20260723-182855). Pure content. See GOAL.md for the
engine root-cause analysis (gravity SOI = 8*radius; only piloted ships feel
gravity; armed AI Engages by chasing).

## Steps

- [ ] Shrink + reposition the three planetoids so NO SOI (`8 * radius`) reaches
      the base spawn (0,15,-520) and the field is calmer:
      - planetoid_1: radius 22->14, gravity 6->3, pos (-170,-40,-150)->(-160,-40,-140) (SOI 112; ~416u from base).
      - planetoid_2: radius 18->12, gravity 5->3, pos (150,55,-330)->(150,50,-300) (SOI 96; ~269u from base).
      - planetoid_3: radius 26->14, gravity 7->3, pos (30,-85,-470)->(-110,-70,-360) (SOI 112; ~212u from base - the one that was dragging it).
      (Compute the base-to-planetoid distances and confirm each > that
      planetoid's SOI before trusting the fix.)
- [ ] Base holds station: keep it thrusterless AI + the controller core (do NOT
      add thrusters - an armed AI with thrusters chases the player). It now sits
      outside all wells, so it station-keeps. No RON change beyond the turret
      trim; the fix is the gravity repositioning above.
- [ ] Reduce the base turrets 4 -> 2: keep turret_zp (0,1,2) and turret_zm
      (0,1,-2) (they still sit base-against occupied spine cubes); remove
      turret_xp and turret_xm. Leave the arm hull cubes for the silhouette.
- [ ] Rebind torpedoes to the R key: change the two torpedo cubes'
      input_mapping from `Mouse(Right)` to `Keyboard(KeyR)`; keep
      `Gamepad(RightTrigger2)`. Update the OnStart briefing text ("torpedo tubes
      on the right mouse" -> "the R key"). Confirm no flight-rig key conflict
      (lint WARN) - R is not a reserved flight key.
- [ ] Set the scenario `hidden: true` -> `hidden: false` so it launches from the
      Scenarios picker for testing. (Temporary test state; flag re-hiding before
      release in the final report.)
- [ ] Update the ch5 rig `crates/nova_assets/tests/ledger_ch5_raid.rs`:
      - the torpedo-binding test asserts the tubes bind the R key
        (`Keyboard(KeyCode::KeyR)`), not just contains_key;
      - a base test asserts exactly 2 turret sections and no thruster section;
      - assert the scenario is not hidden.
- [ ] Docs: bump bundle `meta.version` 1.10.0 -> 1.11.0; CHANGELOG 1.11.0 entry
      (gravity/turret/torpedo-key tuning); update the README + `docs/news-*.md`
      torpedo-control mention ("right mouse" -> "R key"); mod-guide version walk
      1.10.0 -> 1.11.0.
- [ ] Content lint (`--target webmods/the-ledger`) clean; `webmods_validation`
      loads ch5; ch5 rig + ch4 rig green; `cargo fmt --check -p nova_assets`.

## Definition of Done

- cmd: `cargo run -p nova_assets --bin content -- lint --target webmods/the-ledger`
  is clean (no key-conflict WARN, turret mounts still valid); `cargo test -p
  nova_assets --test webmods_validation` loads ch5.
- test: `cargo test -p nova_assets --test ledger_ch5_raid` green with the updated
  torpedo-key / turret-count / not-hidden assertions.
- cmd: `cargo fmt --check -p nova_assets` clean.
- manual: playtest ch5 - calmer gravity for the small ships, the base holds its
  position, torpedoes on R, 2 turrets on the base, launches from the picker.

## Notes

- Do NOT run the full local suite / clippy (memory: skip-local-tests-and-clippy).
- `Keyboard(KeyR)` RON: `BindingInput::Keyboard(KeyCode)`; KeyCode serializes by
  variant name, so `Keyboard(KeyR)`. If the lint/parse rejects that token, check
  the exact KeyCode RON repr and fix.
- Bump to 1.11.0 (1.10.0 was landed but each portal version is immutable; a tune
  is a new version).
