# ch5 gravity r2: strip base thrusters, move base + raiders clear of tiny wells, shrink planetoid wells (content)

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.8.0, content, scenario, playtest

## Story

Umbrella 20260723-223947. Second gravity pass on `ledger_ch5_the_raid.content.ron`.
Reverts the RCS/leash base (task 20260723-200643) and keeps all combatants OUT of
gravity, with only tiny wells left as approach scenery. Pure content. See GOAL.md.

## Steps

- [ ] Strip the base's RCS: remove the two `basic_thruster_section` sections
      (rcs_xp, rcs_xm) and set `controller: AI(())` (drop the leash - moot on a
      thrusterless ship, which cannot move/chase). Keep the 2 turrets and the
      controller core. Update the base comment (thrusterless, out of gravity,
      holds station because it cannot move).
- [ ] Move the base further out, clear of all wells: `magpie_base` position
      (0,15,-520) -> (0,15,-580).
- [ ] Move the four raiders near the new base and leash them TIGHT (320 -> 200)
      so they stay in the base fight and never reach a planetoid:
      raider_1 (-70,10,-520), raider_2 (75,-5,-540), raider_3 (-40,-30,-630),
      raider_4 (45,35,-640). (All > 450u from the player spawn (0,0,90) - no
      spawned-dead.)
- [ ] Shrink the planetoid wells and relocate them to the EARLY approach
      corridor (z >= -230), clear of the base and outside the raiders' 200u leash
      reach (raiders bounded to ~z <= -380):
      - planetoid_1: radius 14->8, gravity 3->1, pos -> (-120,-30,-150) (SOI 64).
      - planetoid_2: radius 12->9, gravity 3->1, pos -> (130,40,-190) (SOI 72).
      - planetoid_3: radius 16->8, gravity 3->1, pos -> (-40,-60,-230) (SOI 64).
      Verify: base(0,15,-580) is > each SOI from every planetoid; the nearest a
      raider (leash 200 around base) can get to any planetoid SOI is > 0 (a real
      gap). Compute before trusting.
- [ ] Extend the wingmen patrol endpoints toward the new base (z ~ -520) so they
      still fly in with the player.
- [ ] Update the ch5 rig `crates/nova_assets/tests/ledger_ch5_raid.rs`: rewrite
      `the_base_holds_station_with_rcs_and_a_tight_leash` into a thrusterless
      version - assert the base has 2 turret sections, 0 thruster sections, and
      `controller: AI` with `leash == None` (holds station because it cannot
      move). Keep the R-key / not-hidden / other tests.
- [ ] Docs: bump bundle `meta.version` 1.11.0 -> 1.12.0; CHANGELOG 1.12.0 entry
      (revert RCS, tiny wells, keep AI clear; the 1.11.0 entry stays as dated
      history); mod-guide version walk 1.11.0 -> 1.12.0. README/news describe the
      raid generally (no thruster/gravity specifics) - no change beyond version.
- [ ] Lint (`--target webmods/the-ledger`) clean; `webmods_validation` loads ch5;
      ch5 rig + ch4 rig green; `cargo fmt --check -p nova_assets`.

## Definition of Done

- cmd: `cargo run -p nova_assets --bin content -- lint --target webmods/the-ledger`
  clean; `cargo test -p nova_assets --test webmods_validation` loads ch5.
- test: `cargo test -p nova_assets --test ledger_ch5_raid` green with the
  thrusterless-base assertion (0 thrusters, 2 turrets, no leash).
- cmd: `cargo fmt --check -p nova_assets` clean.
- manual: playtest ch5 - the base holds; no AI ship falls into a well; wells are
  gentle approach scenery; torpedoes on R; launches from the picker.

## Notes

- Do NOT run the full local suite / clippy (memory: skip-local-tests-and-clippy).
- Removing the thrusters ALSO resolves the R1.1 MINOR from the prior round (a
  thrusterless base cannot chase even under the recently-damaged leash override).
- Grep the rig for the old version string on the bump: `grep -rn '"1.11.0"'
  crates/` (the version-pin has bitten twice - lesson bundle-version-string-pin).
