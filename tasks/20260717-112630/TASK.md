# Rework ledger_ch2 encounter design: loadouts, spawn ranges, real cover, aggro stagger, act-split retry

- STATUS: CLOSED
- PRIORITY: 53
- TAGS: spike,v0.7.0,scenario,content,balance

Goal: ledger_ch2_claim_jumpers currently opens with two better_turret
magpies (800 dps combined, perfect lead) at ~175u in open void, chains two
reinforced replacements at ~130u the frame the kill counter flips, and the
neutral Dray Mule sits on the crossfire axis so dodged bursts kill the
escort. Rework the encounter so it still demands skill but is fair, WITHOUT
touching AI smarts, weapon stats, or player damage taken.

Direction notes (all levers verified available in shipped RON):
- Loadout discipline: light_turret_section on act-1 magpies (broadside's
  corvettes set this canon); better_turret on at most one act-2 ship.
- Spawn geometry: push spawns to 500-800u so an approach phase exists; wave
  2 never spawns inside 400u of the player.
- Real cover: an invulnerable rock field (AsteroidConfig invulnerable: true,
  asteroid.rs:45) between the spawn bearings and the Dray Mule; destructible
  rocks stay as chaff only.
- Aggro stagger: patrols + leashes (AIControllerConfig patrol/leash) instead
  of four ships converging on a 250u orbit at once.
- Escort geometry: move the Dray Mule off the crossfire axis; consider
  objective text that hints at drawing fire away (enemies never target
  neutrals, relations.rs:50 - the hauler only dies to strays).
- Act-split retry: each act its own hidden scenario chained via
  NextScenario, so defeat retries the current act, not the chapter.

Spike: tasks/20260717-111808/SPIKE.md (findings F1/F2/F3/F4/F6/F7)

Verified at plan time: the LOS fire gate landed (2d006707), so invulnerable
cover now relieves pressure, not just absorbs rounds. Asteroid bodies run
3.5x-6x their nominal `radius` (ASTEROID_GEOMETRIC_FACTOR_MIN/MAX,
asteroid.rs:309) - cover sizing and spacing must use the 6x worst case.
Patrol/leash RON syntax proven in shakedown_run.content.ron:1101-1118.
Cross-file refs to `ledger_ch2_claim_jumpers`: ledger_ch1.content.ron:432
(chain in) and the bundle list - the act-1 file KEEPS the id so ch1 is
untouched. Geometry-pin test pattern to mirror:
crates/nova_assets/tests/gauntlet_course.rs (include_str the shipped RON,
drive real handlers, computed invariants over shipped positions).

## Steps

- [x] Rework webmods/the-ledger/ledger_ch2.content.ron into ACT 1 ONLY
  ("Claim Jumpers"): magpie_1/2 swap better_turret_section ->
  light_turret_section; both spawn ~600u out in ONE quadrant (same-bearing
  attack, no bracketing crossfire - spike F3) with patrols converging on
  the pickup; an invulnerable cover field (4-6 rocks, nominal radius 3-5,
  texture dep://base/textures/asteroid.png, spaced for the 6x factor so no
  body overlaps the Mule station, player spawn, or another rock) between
  the threat bearing and the Dray Mule, plus 2-3 destructible chaff rocks
  (health 100, invulnerable: false); Dray Mule moved off the threat axis
  into the rocks' lee; an Okono story line teaching the draw-fire-away
  play; win when kills > 1 -> Victory outcome ("wave one broken", the
  checkpoint beat) + NextScenario(ledger_ch2b_the_heavies, linger: true);
  both Defeat handlers retry ledger_ch2_claim_jumpers only.
- [x] New webmods/the-ledger/ledger_ch2b.content.ron ("The Heavies",
  hidden): same arena (player ship, Dray Mule, rock field), magpie_3/4 as
  reinforced-hull heavies spawning ~950u out on a DIFFERENT single
  bearing; exactly ONE better_turret between them (Crowbar better, Kettle
  Black light); win kills > 1 -> Victory + NextScenario(
  ledger_ch3_quiet_channel, linger: true); Defeat handlers retry
  ledger_ch2b only. Act numbering stays coherent within each file.
- [x] Wire and sweep: add ledger_ch2b.content.ron to
  the-ledger.bundle.ron content list, bump the mod version to 1.1.0;
  sweep webmods/the-ledger/README.md chapter descriptions; CHANGELOG entry
  (Scenarios & Objectives); grep the player wiki for ledger chapter
  descriptions and update if present.
- [x] Test crates/nova_assets/tests/ledger_ch2_encounter.rs mirroring
  gauntlet_course.rs: (a) behavior walk of BOTH parts - OnStart vars,
  per-magpie kill counting, act transition, Victory chains (ch2a -> ch2b
  -> ch3), Defeat handlers retrying their OWN part; (b) computed geometry
  pins over the shipped RON: hostile spawns >= 500u (part A) / >= 800u
  (part B) from player spawn, hostile bearings within a ~35 degree spread
  per part (single-bearing pin), >= 4 invulnerable rocks inside the threat
  corridor between the Mule and the spawn bearing, worst-case 6x bodies
  overlap nothing (Mule/player/each other), zero better_turret in part A
  hostiles and exactly one in part B; (c) turret-tier counts derived from
  the RON, not hardcoded ship-by-ship.
- [x] Verify: cargo run -p nova_assets --bin content_lint; cargo test -p
  nova_assets --test ledger_ch2_encounter; --test webmods_validation;
  cargo fmt. Full suite stays on CI.
- [x] tasks/<id>/NOTES.md design record: bearing/cover geometry rationale
  with the numbers, the checkpoint-beat decision (mid-chapter Victory
  overlay as the act boundary), infinite_ammo left as-is on purpose
  (mod-wide ammo consistency routed to the audit-rig task).

## Close-out record

All six steps landed. What changed, the numbers, the alternatives and the
fail-first evidence are in NOTES.md (kept to one file, not repeated here).
Highlights: chapter two now plays as two hidden scenarios with the act
boundary as checkpoint; wave one is light-turret/single-lane/600u with an
invulnerable corridor field; wave two is the reinforced pair at 950u on
the opposite lane with exactly one better turret; the Mule stations 85u
above the fight plane after the new geometry pin failed the first layout
(a genuine catch - see NOTES.md "The Mule and the stray-fire model").

Verification: content_lint clean; ledger_ch2_encounter 12/12;
webmods_validation green; cargo fmt. Full suite on CI per standing
instruction.

Reflection: the computed-pin-first approach (write the invariant test,
then iterate the layout against it) caught in seconds what eyeballing
missed, and forced the stray-fire model to be stated precisely (overshoot
past the player). Worth repeating for every encounter layout task.
