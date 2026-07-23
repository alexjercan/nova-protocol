# Notes - ch3 stealth rework: neutral-until-provoked channel Magpies

Task 20260723-000320. Branch `content/ledger-ch3-stealth`. Data-only RON +
tests + docs; the engine primitive (SetAllegiance, 20260723-000253) landed
before this and is unchanged here.

## What changed (from the final diff)

- `webmods/the-ledger/ledger_ch3.content.ron`:
  - The two channel Magpies moved OUT of the NAV-2 OnEnter ambush into
    OnStart: they spawn at chapter start with `allegiance: Some(Neutral)`,
    a two-point `patrol:` loop each, and NO `engage_delay` (that was the
    hostile-arrival grace; a Neutral ship needs none). The player sees the
    pickets patrolling ahead from the first frame and plans the sneak - no
    jump-scare spawn.
  - Two `CreateScenarioArea` detection bubbles ("picket_watch_starboard",
    "picket_watch_port"), one per patrol lane.
  - Four wake handlers, all gated `spotted == 0` AND `act == 1`: OnEnter on
    each watch zone, OnCombatLock on each Magpie. Same actions in each:
    `spotted = 1`, SetAllegiance -> Enemy for BOTH Magpies, one Vesh line
    (proximity and paint each have their own text). Setting spotted
    disqualifies the other three (self-disqualifying one-shot).
  - Yard split: the shipped OnEnter victory handler now gates
    `spotted == 1` (fight path, unchanged flow); a new clean-path OnEnter
    (`spotted == 0`) stamps `act = 2`, completes the objective, stamps
    `win_gate = elapsed + 3` and speaks the payoff line ("Nothing on their
    scopes..."); a deferred OnUpdate (`act == 2 && win_said == 0 &&
    win_gate > 0 && elapsed > win_gate`, the ch2b idiom) fires the Victory +
    NextScenario ch4. `win_gate > 0` keeps the fight path (which never
    stamps it) from double-firing. Victory -> ledger_ch4_the_buyer on BOTH
    paths; the player-death Defeat/retry handler is untouched.
  - OnStart seeds `spotted`, `win_gate`, `win_said` (fail-closed contract).
  - Comms coherence: the briefing line now calls out the two cold pickets
    ("stay off their scopes, keep your lock stowed"); the pinch warning
    sells the gap as the blind spot; NAV-2's "Contacts... run or fight"
    became a path-neutral "Drop two. Halfway..."; the post-NAV-2 breather
    split into spotted-gated twins (sneak: "both pickets still cold";
    fight: the old "don't chase them" line) sharing the `arrive1_said`
    one-shot so exactly one speaks. Opener cascade and pinch beats
    (including the SetSkybox accent) untouched.
- `crates/nova_assets/tests/ledger_ch3_channel.rs`: see below.
- `the-ledger.bundle.ron` 1.6.0 -> 1.7.0; mod CHANGELOG `## 1.7.0`; README
  ch3 entry rewritten as the stealth run; wiki `guide-make-a-mod.md`
  version walk extended to 1.7.0.

## Spawn-timing decision

Spawned at OnStart, not NAV-2 (the task offered both). Reasons: stealth
needs the threat visible before the choice - the player must see the
patrol lanes from the corridor to pick the pinch on purpose; and the
detection geometry flanks the NAV-1 -> NAV-2 leg, which the player reaches
BEFORE the old NAV-2 trigger, so a NAV-2 spawn would have empty detection
bubbles on the very leg they guard. Geometry allows it: both patrol lanes
sit ~55u off the corridor, and the start leg (0,0,40) -> NAV-1 passes no
closer than ~54u to any patrol endpoint.

## Bubble geometry (computed, not guessed)

All numbers derive from the shipped pinch: boulders at (2.5, 7.5, -153.3)
and (57.5, 7.5, -116.7), nominal radius 3.5, worst-case body factor 6x
(ASTEROID_GEOMETRIC_FACTOR_MAX) => 21u bodies; centres 66.07u apart; gap
centre C = (30, 7.5, -135) sits on the NAV-1 (0,0,-90) -> NAV-2
(60,15,-180) leg; pinch perpendicular u = (0.8325, 0, 0.5540) (unit,
u . leg_dir ~ 0).

- Bubble centres: C +/- 55u => starboard (75.8, 7.5, -104.5), port
  (-15.8, 7.5, -165.5). Radius 24.
- Safe lane: worst-case clear half-gap = (66.07 - 42)/2 = 12.03u around
  the leg. Bubble edge sits 55 - 24 = 31u off the leg (measured against
  the segment in the rig: ~54.9 centre distance) => ~19u margin; the rig
  pins edge clearance >= half_gap + 6u margin.
- Wide swing covered: each boulder's worst-case far edge reaches
  33.03 + 21 = 54.03u from C; the bubble centred at 55u with radius 24
  spans 31..79u, and the rig pins dist(bubble centre, its boulder centre)
  < radius (22.0 < 24) - an arc around either wreck crosses the watch.
- Objectives never trip detection: every beacon arrival sphere is disjoint
  from every bubble (min beacon-centre distance 61.1u vs worst sum
  24 + 25 = 49; rig pins all five beacons vs both bubbles).
- Patrol endpoints: bubble centre +/- 40u along the leg direction
  (0.5494, 0.1374, -0.8242): starboard (53.8, 2.0, -71.5) <-> (97.8, 13.0,
  -137.5); port (6.2, 13.0, -198.5) <-> (-37.8, 2.0, -132.5). Ships spawn
  at their first waypoint.
- Known honor-system edge (documented, accepted): the bubbles are spheres
  in the pinch plane, so a large vertical detour over the boulders clears
  both pinch and watch - exactly as the shipped pinch hazard itself was
  already skippable in y. Same contract as before: the beats reward flying
  the line.

## Rig (`ledger_ch3_channel.rs`, 12 -> 14 tests)

- Structural: OnStart also seeds spotted/win_gate/win_said and spawns both
  Magpies + both watch zones; new pin `the_magpies_spawn_neutral_on_patrol_
  without_engage_delay` (allegiance Some(Neutral), >= 2 patrol points,
  engage_delay None).
- Geometry: `the_picket_watch_zones_spare_the_safe_lane_and_cover_the_wide_
  swing` - computed pin per bubble: (a) edge clearance off the NAV-1->NAV-2
  leg >= worst-case half-gap + margin, (b) bubble contains its boulder
  centre (go-around covered), (c) no beacon arrival sphere overlaps a
  bubble.
- Behavior: Magpies are spawned through their REAL OnStart
  SpawnScenarioObject configs (the same EventAction the loader runs) and
  flushed by the production `state_to_world` sync GameEventsPlugin already
  registers - assertions read the live `Allegiance` COMPONENT off the
  spawned entities, the value AI targeting reads. Pinned: watch-zone entry
  flips BOTH to Enemy + stamps spotted (second zone disqualified);
  OnCombatLock on a Magpie does the same and the existing Defeat/retry
  holds after the wake; clean run walks the corridor to YARD with the
  payoff line on the arrival handler (and no Outcome beside it), Victory
  deferred a beat, chaining ch4, with both Magpies STILL Neutral; provoked
  run wins on the spot into ch4.
- The rig needed no new flush plumbing: GameEventsPlugin<NovaEventWorld>
  already schedules `state_to_world_system` (verified in
  bevy-common-systems modding/events.rs), which is the same flush the
  engine's `set_allegiance_flips_the_scoped_ship` test drives manually.

## Verification

- `content lint --target the-ledger`: 0 errors / 0 warnings, 1 pre-existing
  ch4 Auditor ack (expected). The four SetAllegiance ids resolve against
  the OnStart spawns (the new dangling-target lint arm passes).
- `cargo test -p nova_assets --test ledger_ch3_channel --test
  ledger_ch2_encounter --test ledger_ch4_ending --test ledger_skybox
  --test gen_portal_gate`: 14 + 12 + 10 + 6 + 4 = 46 green.
- `cargo fmt --check`: clean.
- Catalog regenerated locally (NOT published):
  `gen-portal.py --source webmods --shipped assets/mods.catalog.ron --out
  ~/.claude/jobs/d2748cf1/tmp/ledger-catalog-17` => "the-ledger 1.7.0
  (8 files, 450387 bytes)".
- Full cargo test suite intentionally skipped per standing instruction
  (CI runs it); only the listed targets were run locally.
- Whole-tree grep for stale ch3/"Quiet Channel" copy: live surfaces
  (README, wiki) updated; dated history (root CHANGELOG, news, tasks/)
  left as-is per the task.

## Owner Finish checks (manual, before publish)

1. Replay the sneak: thread the pinch, no paint - pickets stay cold,
   payoff line at the yard, Victory a beat later, ch4 chains.
2. Replay a provoked run: cross a watch flank (or paint a picket) - both
   go hot, the fight/defeat/retry flow reads right, yard still wins.
3. Publish 1.7.0 to the portal.

## Reflection

- Deriving the bubble/patrol numbers from the measured pinch geometry
  (perpendicular + leg direction) made the rig pins fall out naturally;
  author-against-measured-values continues to pay off.
- The one design fork considered: a single shared "hot zone" over the leg
  vs per-flank bubbles. A shared zone covering the leg cannot exclude the
  safe lane (the lane runs through the middle of it), so per-flank bubbles
  were the only geometry that makes the pinch the blind spot. No engine
  tweak was needed; no fork escalated.
- Went smoothly end to end; the only friction was rustfmt on the new test
  asserts (fixed by running `cargo fmt` instead of hand-wrapping).
