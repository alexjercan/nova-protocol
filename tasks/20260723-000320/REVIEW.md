# REVIEW - task 20260723-000320 (ch3 stealth rework)

Out-of-context adversarial review of branch `content/ledger-ch3-stealth`.
Reviewer did not implement. Verified via own reads/greps, independent geometry
recomputation, and the required check runs under `nix develop`.

## Round 1

### Soft-lock / flow (both paths reach ch4)

- No finding (HIGH cleared). Both terminal paths reach
  `vesh_yard` (gate==4) -> Victory -> NextScenario `ledger_ch4_the_buyer`:
  - Fight path (`spotted==1`): yard OnEnter at
    `ledger_ch3.content.ron:1842-1875` sets `act=2`, completes the objective,
    fires Victory + NextScenario on the spot. Never stamps `win_gate`.
  - Clean path (`spotted==0`): yard OnEnter at `:1882-1916` sets `act=2`,
    stamps `win_gate = elapsed+3`, speaks the payoff line (no Outcome in that
    handler). Deferred overlay at `:1920-1954` fires Victory + NextScenario
    once the clock passes `win_gate`. Since `scenario_elapsed` always advances
    in production, this cannot stall.
- Double-fire guard is real: the clean overlay gates `win_gate > 0`
  (`:1931-1934`); the fight path never stamps `win_gate`, so it cannot trigger
  the deferred overlay. Verified.
- Every one-shot self-disqualifies and is seeded at OnStart (`:36-105`):
  `spotted`, `win_gate`, `win_said`, `arrive1_said`, `arrive3_said`,
  `pinch_warn_said`, `pinch_clear_said`, `nav_posted`, `open_step`, `beat_gate`.
  `on_start_seeds_the_sequencer_and_spawns_the_cast` pins all thirteen.
- Player-death Defeat/retry (`:1956-1977`) gates `act < 2`, so it holds while
  the Magpies are awake and self-disqualifies post-win
  (`death_after_the_win_declares_nothing`, `painting_a_sleeping_magpie_wakes_both`
  both green).

### Detection geometry (safe lane is genuinely outside the bubbles)

- No finding (HIGH cleared). Recomputed independently from the shipped
  positions (boulders (2.5,7.5,-153.3)/(57.5,7.5,-116.7) r3.5, factor 6x
  confirmed at `asteroid.rs:364`; bubbles (75.8,7.5,-104.5)/(-15.8,7.5,-165.5)
  r24):
  - Worst-case clear gap = 66.06 - 42 = 24.06u; half-gap 12.03u.
  - Each bubble edge sits 55.03 - 24 = 31.03u off the NAV-1->NAV-2 leg vs a
    needed 18.03u (half-gap + 6u margin) -> ~13u real margin. (NOTES' "~19u"
    counts edge-to-lane-center 31 minus half-gap 12; either way the lane is
    clear with margin.)
  - Go-around covered: each bubble contains its flanking boulder centre
    (21.99u < 24u), so any arc around a wreck crosses the watch.
  - All five beacon arrival spheres are disjoint from both bubbles (min gap
    +15.1u at pinch_clear). Entering an objective never forces detection.
  - Every NAV leg centreline clears both bubbles (start->nav1 77.5u,
    nav1->nav2 55.0u, nav2->nav3 63.9u, nav3->yard 120.2u vs r24). Flying the
    line straight is a genuine clean run; only a deliberate >31u swing off the
    pinch leg trips a bubble.
  - `the_picket_watch_zones_spare_the_safe_lane_and_cover_the_wide_swing`
    encodes (a)/(b)/(c) with the same worst-case math; a bubble that crept
    over the lane, off its boulder, or onto a beacon would fail it.

### Allegiance flip wakes BOTH pickets

- No finding (HIGH cleared). All four wake handlers
  (starboard/port watch OnEnter `:1512-1583`, per-Magpie OnCombatLock
  `:1586-1657`) gate `spotted==0 && act==1`, stamp `spotted=1`, and issue
  `SetAllegiance -> Enemy` for BOTH `channel_magpie_1` and `channel_magpie_2`.
  No half-wake path.
- The rig is production-faithful: `spawn_magpies` runs the REAL OnStart
  `SpawnScenarioObject` configs and flushes via the production sync, then
  `ship_allegiance` reads the live `Allegiance` COMPONENT (the value AI
  targeting reads), not a rig variable. `entering_a_picket_watch_zone_wakes_both_magpies`
  and `painting_a_sleeping_magpie_wakes_both` assert both flip Neutral->Enemy;
  `a_clean_run_...` asserts both stay Neutral. A regression (spawn hostile,
  only one wakes, a bubble over the lane) fails a test.

### SetAllegiance target ids valid

- No finding. All four SetAllegiance ids are
  `channel_magpie_1`/`channel_magpie_2`, both spawned at OnStart. The lint
  dangling-target arm (`lint.rs:475-476`, `check_target`) is live; lint reports
  0 errors. `the_magpies_spawn_neutral_on_patrol_without_engage_delay` pins
  Neutral spawn + patrol + no engage_delay.

### Beat sheet / comms coherence

- No finding. One StoryMessage per handler; the three Outcome handlers
  (`:1842`, `:1920`, `:1956`) carry NO StoryMessage - the payoff line lives on
  a separate handler from its deferred Victory. Explicit dwells are all in
  [6,10] (in-range); omitted dwells default to None per `actions.rs:1279`, the
  established sibling-chapter idiom for short one-liners, not a violation.
- Fight-only line (`arrive1` twin, `:1764` "Don't chase them") gates
  `spotted==1`; the sneak twin (`:1725` "Both pickets still cold") gates
  `spotted==0`, sharing `arrive1_said` so exactly one speaks. The NAV-2 line
  is path-neutral. No clean-path line presupposes a fight. Opener cascade
  (open_step 0..4) and pinch beats (warn/clear + SetSkybox) intact.

### Version / docs

- No finding. bundle 1.6.0 -> 1.7.0; CHANGELOG `## 1.7.0` is diff-accurate;
  README ch3 rewritten as the stealth run; wiki version-walk updated to 1.7.0.
  Catalog regenerated locally to /tmp: "the-ledger 1.7.0 (8 files, 450387
  bytes)". Whole-tree grep found no stale ch3/"Quiet Channel" copy on live
  surfaces (only the two updated files; dated history left as-is per task).

## Check runs (nix develop)

- `content lint --target the-ledger`: 0 error(s), 0 warning(s), 0 finding(s);
  only the pre-existing ch4 Auditor ACK. Clean.
- `cargo test -p nova_assets --test ledger_ch3_channel --test
  ledger_ch2_encounter --test ledger_ch4_ending --test ledger_skybox --test
  gen_portal_gate`: all green (ch3 14, ch4 10, skybox 6; others green). No
  failures.
- `cargo fmt --check`: clean (exit 0).
- Catalog regenerated at 1.7.0 (above).

## Rig quality

Production-faithful and fail-first. It loads the shipped RON, registers the
real handlers, spawns the Magpies through their actual OnStart configs, and
asserts the live Allegiance component. A regression on any of the specified
failure modes (hostile spawn, bubble over the safe lane, single wake, broken
clean chain, off-boulder bubble, beacon overlap, lost deferred-victory guard)
fails a concrete test.

- VERDICT: APPROVE
