# Review: Ledger chapter 2 encounter rework

- TASK: 20260717-112630
- BRANCH: work/ledger-ch2-rework

## Round 1

- VERDICT: APPROVE

- [ ] R1.1 (MINOR) webmods/the-ledger/ledger_ch2b.content.ron:336 - Kettle Black's
  leash sphere excludes the player spawn by ~15u. The leash anchors on the patrol
  centroid (crates/nova_scenario/src/objects/spaceship.rs:323-330); magpie_4's
  anchor is the midpoint of (-620,30,-790)/(-140,10,-220) = (-380,20,-505), which
  is 664.7u from the player spawn (0,0,40) with leash 650. `leash_exceeded`
  (crates/nova_gameplay/src/input/ai.rs:663-670) breaks combat strictly beyond the
  radius and re-engages only inside 0.8x (520u), so a heavy chasing a player who
  holds at spawn disengages ~15u short and yo-yos on the 520/650 hysteresis band.
  In practice the impact is small (the light turret's 270u effective range still
  reaches the spawn from the boundary, and the under-fire override keeps it
  engaged once the player shoots back), but NOTES.md claims "full aggression in
  the arena" and one heavy's aggression sphere misses one point of the arena.
  Crowbar is fine (anchor (-305,-25,-440), spawn at 569.3u < 650); both wave-one
  leashes cover the spawn (386.4 and 440.3 vs 550). Suggested change: bump
  magpie_4's leash to Some(700.0), or pull its patrol endpoint arena-ward (e.g.
  (-120.0, 10.0, -180.0) puts the anchor 642.6u from spawn); optionally add a
  test pin that every hostile's patrol-midpoint leash sphere contains the player
  spawn.
  - Response: fixed - magpie_4's leash bumped to 700 (with a comment
    naming the 665u anchor-to-spawn distance), and the test now pins
    leash-covers-spawn for every leashed hostile in both waves
    (assert_leash_covers_spawn).

- [ ] R1.2 (NIT) crates/nova_assets/tests/ledger_ch2_encounter.rs:160 -
  `corridor_cover` leaks a String per matching rock per call
  (`Box::leak(id.to_string().into_boxed_str())`) just to return
  `Vec<&'static str>`. Harmless in a test (bounded, process-lifetime), but the
  leak only exists to dodge a lifetime annotation. Suggested change:
  `fn corridor_cover<'a>(event: &'a ScenarioEventConfig) -> Vec<&'a str>` and
  return the borrowed ids directly.
  - Response: fixed - corridor_cover returns borrowed &str now, no leak.

### Verification record

Independent re-derivation (python over the raw RON values, not the test's
helpers). Player spawn (0,0,40), Mule (-40,85,-110) in both parts.

- Spawn ranges from player spawn: magpie_1 622.0u, magpie_2 690.6u (pin >= 500);
  magpie_3 921.7u, magpie_4 1036.4u (pin >= 800). NOTES claims ~600/690 and
  ~920/1030 - honest.
- Bearing spreads from the player spawn: wave one 6.94 deg, wave two 4.78 deg
  (pin <= 35). Single lane per wave confirmed; the two lanes are opposite
  (east vs south-west).
- Corridor cover (point-to-segment Mule -> threat centroid, half-width 120u,
  t in (0.05,0.95)): part one centroid (385,5,-490), corridor 575.7u long;
  in-corridor invulnerable rocks: cover_east_1 (43.3u, t=0.43), cover_east_2
  (50.5u, t=0.67), cover_center (106.3u, t=0.32) -> 3 >= 2. Part two centroid
  (-570,-5,-755), corridor 839.7u: cover_west_1 (61.9u, t=0.27), cover_west_2
  (64.1u, t=0.46) -> 2 >= 2 (exactly at the pin; that is the tripwire working
  as intended). The wave-two lane is genuinely covered by the western rocks;
  from the PLAYER's line to each threat centroid the nearest worst-case bodies
  actually intersect or graze the sightline (cover_east_2 18.7u off the part-one
  lane with a 21u max body; cover_west_2 33.6u off the part-two lane).
- 6x worst-case body overlaps: tightest rock-vs-rock gap is cover_center vs
  chaff_3 at +52.0u (all pairs positive); tightest station clearance is chaff_2
  vs Mule at 132.6u (pin >= 20u) - all clear with wide margins.
- Mule vs hostile->player fire lane extended 500u past the player: magpie_1
  140.5u, magpie_2 152.4u, magpie_3 104.9u, magpie_4 99.3u (pin >= 60u).
  Regression check: the pre-rework Mule station (0,-5,-60) computes to 55.5u
  worst case -> the pin FAILS it; the NOTES-described first-layout station
  (-40,-5,-110) computes to 51.7u vs magpie_3, matching NOTES' claimed 52u
  fail-first catch exactly.
- Reachability: player at speed_cap 25 reaches cover_center in 13.3s /
  cover_west_1 in 15.2s; at AI_MAX_CHASE_SPEED 20 (ai.rs:1012, AI_ORBIT_SPEED 8
  at :1011) wave one needs ~17.6s to close to its 270u effective range
  (light turret: muzzle 60 x life 5 x 0.9), the heavies ~23.6s to their 450u
  (better: 100 x 5 x 0.9). Cover is reachable before either approach closes.
- Leash sanity: anchors verified as patrol midpoints (spaceship.rs:330);
  cover_center sits 171/232u (wave one) and 368/463u (wave two) inside the
  550/650 leashes - full aggression across the rock field, with the one
  spawn-edge wrinkle filed as R1.1.
- Act machines, walked handler by handler: ch2a - two OnDestroyed kill counters
  gated act == 1; win OnUpdate gated act == 1 && kills > 1 (the > N-1 form),
  sets act=2, Victory, NextScenario(ledger_ch2b_the_heavies, linger); both
  Defeat handlers gated act < 2 and requeue ledger_ch2_claim_jumpers. ch2b -
  identical shape over magpie_3/4, Victory chains ledger_ch3_quiet_channel,
  Defeats requeue ledger_ch2b_the_heavies. No handler in either file references
  the other part's entities (ch2a never mentions magpie_3/4, ch2b never
  mentions magpie_1/2). Same-frame semantics match the pre-existing single-file
  pattern (old ch2 on master uses the same act-gated counters / OnUpdate win /
  act < N defeats), so nothing new to flag there.
- Chain ids: ledger_ch1.content.ron:432 chains into ledger_ch2_claim_jumpers
  (id kept by the act-1 file); ch3's id is ledger_ch3_quiet_channel
  (ledger_ch3.content.ron:11). content_lint validates dangling NextScenario
  targets, backstopping the bundle test's string-contains pins.
- Content refs: all six section prototypes used (basic_controller_section,
  reinforced_hull_section, light_hull_section, basic_thruster_section,
  light_turret_section, better_turret_section) exist in
  assets/base/sections/base.content.ron; asteroid texture
  dep://base/textures/asteroid.png matches sibling chapters; hidden: true on
  both parts (ledger_ch2.content.ron:24, ledger_ch2b.content.ron:20); bundle
  lists ch2b and bumps 1.0.0 -> 1.1.0. Repo-wide grep for ledger_ch2 ids finds
  only this mod, this test, and task/spike docs - no stale wiki, portal or
  example references (web/src/wiki has no ledger-campaign page).
- Test-quality probes: mentally reverting each regression trips a distinct pin
  (better turrets on wave one -> prototype_count assert; 175u spawns -> the
  >= 500/800 range asserts; cover removed -> the >= 4 invulnerable and >= 2
  corridor asserts; bracketing spawns -> the 35-degree spread assert; Mule on
  the axis -> the 60u lane assert, verified failing on both old stations).
  hostiles() correctly excludes the Mule (it authors allegiance Some(Neutral),
  the filter requires allegiance.is_none() + AI controller). The OnStart
  structural pin checks seeded keys (not values), matching the
  gauntlet_course.rs house pattern. Box::leak filed as R1.2.
- Docs honesty: "five invulnerable boulders" - counted 5 (+3 destructible
  chaff); "exactly one big gun" in wave two - Crowbar better_turret_section,
  Kettle Black light_turret_section; "85u above the fight plane" - Mule y=85
  vs fight elements y in [-40,30], lane distances 99-152u; README/CHANGELOG
  act-split and checkpoint claims match the handlers. AI smarts, weapon stats
  and player damage untouched (diff touches only scenario RON, bundle, docs,
  and the new test); the LOS fire-gate commit 2d006707 the cover design leans
  on is an ancestor of this branch.

Verbatim test results (run from the worktree):

```
running 12 tests
test the_bundle_ships_both_parts_and_the_bump ... ok
test the_heavies_spawn_farther_with_exactly_one_better_gun ... ok
test wave_one_spawns_far_light_and_on_one_bearing ... ok
test the_mule_sits_off_both_threat_axes ... ok
test on_start_seeds_the_act_machine ... ok
test worst_case_rock_bodies_overlap_nothing ... ok
test invulnerable_cover_sits_in_both_threat_corridors ... ok
test heavies_kills_clear_the_lane_to_chapter_three ... ok
test wave_one_kills_checkpoint_into_the_heavies ... ok
test heavies_deaths_retry_the_heavies_only ... ok
test wave_one_deaths_retry_wave_one ... ok
test deaths_after_the_win_declare_nothing ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

```
running 1 test
test every_webmods_bundle_loads_recursively ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

```
WARN  [the-ledger] scenario 'ledger_ch4_the_buyer': object id 'auditor' is spawned by more than one handler - fine only if the handlers are mutually exclusive
content_lint: clean (1 warning(s))
```

(The single content_lint warning is the pre-existing ch4 'auditor' note,
untouched by this change.)
