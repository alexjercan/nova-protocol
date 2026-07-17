# Review: Balance audit rig

- TASK: 20260717-112656
- BRANCH: work/balance-audit

## Round 1

- VERDICT: REQUEST_CHANGES

- [x] R1.1 (MAJOR) crates/nova_assets/src/balance.rs:190 - A torpedo-only hostile
  evades BOTH finding rules. `findings()` skips any hostile with
  `stats.dps <= 0.0` before either rule runs, and dps counts turrets only
  (tubes are deliberately not dps-folded). So a tube-only ambusher spawned by
  OnStart at point-blank grades CLEAN - yet the engine makes it a live opening
  threat: the AI launch envelope is `[3 x blast_radius, AI_TORPEDO_MAX_RANGE]`
  = [90u, 1000u] for the base bay (ai.rs:1553,1560,1596-1604, blast_radius
  30.0, blast_damage 100.0 in sections.rs:327-328), and the bay's first-launch
  cooldown starts elapsed (ai.rs:1577-1586) - the torpedo comes as soon as the
  envelope opens. The base catalog ships `torpedo_section`, and ch4 already
  fields it on a hostile (the Auditor was caught only because it ALSO carries
  turrets, so the turret envelope fired the WARN; a mixed-armament ship's tube
  threat is likewise unpriced beyond its turret range). This is exactly the
  regression class the ERROR gate exists to catch, silently waved through for
  a whole shipped weapon class. Suggested change: treat `torpedo_tubes > 0` as
  armed (`if hostile.stats.dps <= 0.0 && hostile.stats.torpedo_tubes == 0 {
  continue; }`), give tubes an envelope (mirror `AI_TORPEDO_MAX_RANGE` the way
  `EFFECTIVE_RANGE_MARGIN` mirrors `AI_FIRE_RANGE_FACTOR`, and take the max of
  turret and tube envelopes for the inside-reach test), and add a unit test
  pinning a tube-only OnStart ambusher as ERROR.
  - Response: fixed - ShipStats::threat_envelope() = max(turret reach,
    tubes -> TORPEDO_ENVELOPE 1000u); both rules now gate on it; permanent
    tube-only fail-first test (a_tube_only_onstart_ambusher_is_spawned_dead).
    broadside_gunship (tubes, 1214u) still grades clean; ch4's auditor now
    warns via the 1000u envelope.

- [ ] R1.2 (MINOR) crates/nova_assets/src/balance.rs:77 - "Sustained turret
  dps" overclaims: `fire_rate x bullet_damage` is BURST (within-magazine) dps.
  Every shipped catalog turret now carries a finite magazine with
  `only_when_empty` reload (task 20260717-085640): better_turret 500 rds at
  100 rds/s = 5.0s magazine + 3.0s reload -> long-run sustained 500x4.0/8.0s =
  250 dps, not 400 (62.5%); light_turret 150 rds at 25 rds/s = 6.0s + 2.5s ->
  150x3.825/8.5s = 67.5 dps, not 95.6 (70.6%). The numbers as USED are still
  right for every shipped scenario - all reported TTKs (0.9s-5.2s) land inside
  the hostile's first magazine, and burst dps is the conservative
  (danger-flagging) direction - but the label is wrong in balance.rs:8, the
  ShipStats doc, NOTES.md ("sustained turret dps", "TTK 1.2s sustained"), the
  follow-up task, and the wiki ("combined sustained dps"). Suggested change:
  rename to burst dps (or "magazine dps") and add one doc line stating the
  magazine/reload duty cycle is ignored and why that is conservative for TTK.
  - Response: fixed - dps is documented and reported as BURST dps with the
    ~62% sustained note; TTK meaning unchanged (all shipped TTKs sit inside
    one magazine, as your derivation showed).

- [x] R1.3 (MINOR) crates/nova_assets/src/balance.rs:47 - The catalog docstring
  claims "The SAME id space the runtime merge uses", but the join is an
  approximation of `register_bundles` (nova_assets/src/lib.rs:524) in three
  ways: (a) transitive dependencies are not layered - the runtime merges the
  whole enabled set in TOPOLOGICAL order (deps before dependents,
  lib.rs:581-609), so a dep-of-a-dep's override is live at runtime but
  invisible to the audit, which layers only DIRECT `meta.dependencies`
  (balance.rs:359-365); (b) when two sibling deps collide on an id, the audit
  resolves in DECLARED order while the runtime topo sort tiebreaks in
  catalog-then-download order; (c) an intra-bundle duplicate id is
  first-wins-plus-conflict at runtime (`merge_bundles`, lib.rs:829-857) but
  silently last-wins in `SectionCatalog::resolve`'s HashMap insert. All three
  are moot on today's tree (every shipped bundle declares only `["base"]`,
  content_lint flags dupes), so nothing is mis-priced NOW, but a modded chain
  would be silently mis-priced - the exact failure the docstring says the
  overlay exists to prevent. Suggested change: build the per-bundle catalog
  through `nova_mod_format::deps::topological_order` over the bundle's
  transitive dep closure (or directly reuse `merge_bundles`), or weaken the
  docstring to state the direct-deps-only approximation and its limits.
  - Response: fixed - the docstring now states the exact divergence cases
    (transitive deps, sibling collisions, intra-bundle first-wins) and that
    the join matches the runtime for every shipped bundle today.

- [x] R1.4 (MINOR) crates/nova_assets/src/balance.rs:305 - The hostile
  predicate `ship.allegiance.is_none()` misses an authored `Some(Enemy)`.
  `Allegiance::Enemy` is an authorable variant (relations.rs:26-30) and at
  runtime an AI ship with `Some(Enemy)` is exactly as hostile as the `None`
  default (spaceship.rs:131-137); the audit would skip it entirely - no dps,
  no findings. No shipped content authors it (grep: only `Some(Neutral)`, 5
  sites), so nothing is missed today, but a mod scenario using the explicit
  form evades the gate. Suggested change: skip only the genuinely non-hostile
  forms, e.g. `!matches!(ship.allegiance, Some(Allegiance::Neutral) |
  Some(Allegiance::Player))`, plus a unit test for an explicit
  `Some(Enemy)` hostile.
  - Response: fixed - the hostile predicate is now not-Neutral-not-Player,
    so an authored Some(Enemy) is audited.

- [ ] R1.5 (MINOR) crates/nova_assets/src/balance.rs:150 - The scattered-cover
  tier miscounts and its doc claim is false on shipped data.
  `cover.scattered += scatter.count` counts EVERY ScatterObjects action
  regardless of template kind or hardness, and the field doc says "all shipped
  scatters are destructible chaff" - but gauntlet.content.ron:329 scatters 22
  rocks with `invulnerable: true` into a player scenario (the report line
  `[gauntlet] gauntlet_run: ... cover 9 hard / 0 soft / 22 scattered` files 22
  HARD anchors under the chaff-implying tier). A `Spaceship` template is also
  legal (ScatterObjectsConfig.template is any ScenarioObjectConfig,
  actions.rs:2254-2259) and would be doubly wrong: counted as cover and
  invisible as a hostile - no shipped content does this (verified all 9
  scatter sites: 7 Asteroid, 1 SalvageCrate in a playerless menu scene, 0
  Spaceship). Suggested change: bucket scatters by template kind and
  `invulnerable` (hard vs soft), ignore or separately report non-cover
  templates, and either hostile-audit a Spaceship template or emit a WARN
  that scattered ships are unpriced.
  - Response: fixed - scattered cover splits hard/soft by template hardness;
    the gauntlet's 22 invulnerable belt-wall rocks report as scattered hard.

- [x] R1.6 (MINOR) crates/nova_assets/src/balance.rs:98 - `ship_stats` reads
  `section.source` but ignores `section.modifications`, and
  `SectionModification::SetHealth` (modification.rs:41) overrides a section's
  starting health "regardless of the prototype's authored value" - an
  authorable rebalancing surface the hp sum (and thus TTK) silently misses.
  Shipped content only uses `DisableVerb` (shakedown_run.content.ron:54-58),
  so every reported number is currently exact. Suggested change: apply
  SetHealth overrides in the hp sum (last one wins, matching the observer
  apply), or document the exclusion in the ShipStats.hp doc.
  - Response: fixed - SetHealth modifications override the prototype health
    in hp sums (last modification wins, matching the runtime observers).

### Verification record

Re-derivations against engine source (not the notes):

- Effective range formula: ai.rs:1496-1497 `effective_range = config.muzzle_speed
  * config.projectile_lifetime * AI_FIRE_RANGE_FACTOR` with
  `AI_FIRE_RANGE_FACTOR: f32 = 0.9` (ai.rs:1030). balance.rs:45 uses
  `EFFECTIVE_RANGE_MARGIN: f32 = 0.9` x muzzle_speed x projectile_lifetime -
  the SAME formula. (The engine measures from the muzzle transform, the audit
  from ship spawn positions; the offset is ~1u against 270-450u envelopes,
  negligible.) better_turret: 0.9 x 100 x 5.0 = 450u; light_turret:
  0.9 x 60 x 5.0 = 270u - both match the report.
- Sustained-dps honesty call (R1.2): sections.rs authors better_turret
  fire_rate 100, bullet_damage 4.0, ammo_capacity Some(500), reload 3.0s
  only_when_empty (sections.rs:201-227) -> burst 400 dps, true long-run
  sustained 250 dps; light_turret 25 x 3.825 = 95.6 burst vs 67.5 sustained
  (sections.rs:279-300). Every shipped TTK is inside the first magazine
  (max TTK 5.2s vs 6.0s light magazine; 0.9s vs 5.0s better magazine), so the
  computed TTKs are correct as printed; only the "sustained" label overclaims.
- Player pool / TTK meaning: a ship root dies when the last section dies
  (integrity/glue.rs:117, test at glue.rs:705), so summed section HP IS the
  death threshold and TTK = hp/dps is a lower bound (overkill waste, misses,
  reloads all push real death later). Mission-kill (dead thruster/turret)
  comes earlier than pool-empty; the module names its metric "summed section
  health" honestly (balance.rs:73-75), no doc overclaims death semantics.
- ERROR-rule semantics after the restructure (the shared `continue`): the rule
  is "OnStart AND armed AND distance < own max_effective_range" - identical to
  the task's original predicate; the hoisted guards only skip cases neither
  rule fires on. Pinned by unit tests:
  `an_armed_onstart_hostile_inside_its_range_is_spawned_dead` (175u inside
  450u -> exactly 1 Error; same hostile at 600u -> clean) and
  `triggered_close_spawns_warn_and_unarmed_ships_pass` /
  `a_triggered_mook_outside_its_own_reach_is_clean` for the WARN side. The
  WARN rule's 400u-blanket -> own-envelope change is deliberate and
  documented (NOTES.md). No test pins the tube-only case - see R1.1.
- Overlay join (R1.3): runtime = `register_bundles` topo-sorts enabled
  bundles (deps before dependents, catalog-order tiebreak, lib.rs:594-618)
  and `merge_bundles` overlays cross-bundle last-wins / intra-bundle
  first-wins-with-conflict (lib.rs:807-857). Audit = base -> direct declared
  deps in declared order -> own, HashMap last-wins throughout
  (balance.rs:56-64, 354-367). Same result on the shipped tree (all bundles
  declare only ["base"]: gauntlet.bundle.ron, the-ledger.bundle.ron,
  assets/mods/example); diverges on transitive deps, sibling-dep collisions,
  and intra-bundle dupes.
- Coverage: Inline section sources ARE covered (balance.rs:99-102). The
  player-finder scans ALL events' actions, not just OnStart
  (balance.rs:283-293) - a non-OnStart player spawn is found (last-wins if a
  scenario ever spawns two; none shipped does). EventActionConfig has no
  nested/composite action variant (actions.rs:31-58), so
  SpawnScenarioObject + ScatterObjects is the complete spawn surface.
  Shipped allegiance grep: 5 x `Some(Neutral)`, zero `Some(Enemy)`.
- ch3 NOTES numbers recomputed from ledger_ch3.content.ron: player (0,0,40);
  magpie_1 (120,20,-220) -> sqrt(120^2+20^2+260^2) = 287.0u; magpie_2
  (-90,-30,-240) -> sqrt(90^2+30^2+280^2) = 295.6u -> "287/296u vs 270u
  reach", both outside -> clean, as claimed. ch4 auditor: player (0,0,40),
  auditor (0,30,-260) on both branches -> sqrt(30^2+300^2) = 301.5u inside
  450u, 400 dps (1 better_turret) + 1 tube, player 500hp -> TTK 1.25s, 1
  invulnerable rock - the follow-up task 20260717-143806's numbers all match
  the report. broadside_gunship: 800 dps (2 better turrets), 700hp player ->
  TTK 0.875 -> "0.9s", matches. Filing (not fixing) ch4 follows the task's own
  plan step ("fix here only if one-line-content-level; otherwise file
  follow-up tasks with the numbers") - the right call; a spawn push/cover add
  is a playtest-gated content decision.
- Docs honesty: CI runs `cargo test --workspace --features debug`
  (.github/workflows/ci.yaml "Tests" step), which includes
  nova_assets/tests/balance_audit_gate.rs - "CI fails the build" is accurate.

Verbatim results (run from the worktree):

```
test balance::tests::catalog_overlay_is_last_wins ... ok
test balance::tests::an_armed_onstart_hostile_inside_its_range_is_spawned_dead ... ok
test balance::tests::a_triggered_mook_outside_its_own_reach_is_clean ... ok
test balance::tests::scenarios_without_a_player_are_skipped ... ok
test balance::tests::ship_stats_sum_the_resolved_sections ... ok
test balance::tests::triggered_close_spawns_warn_and_unarmed_ships_pass ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.00s

test shipped_content_carries_no_balance_errors ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo run -p nova_assets --bin balance_audit` (exit 0), key lines:

```
[base] broadside_gunship: player 700hp 400dps | cover 5 hard / 0 soft / 24 scattered
  OnStart: 1 hostile(s), 800 dps, 2 tube(s), closest 1214u, TTK vs player 0.9s
[gauntlet] gauntlet_run: player 500hp 400dps | cover 9 hard / 0 soft / 22 scattered
[the-ledger] ledger_ch3_quiet_channel: player 500hp 400dps | cover 0 hard / 0 soft / 26 scattered
  OnEnter(nav_2): 2 hostile(s), 191 dps, 0 tube(s), closest 287u, TTK vs player 2.6s
[the-ledger] ledger_ch4_the_buyer: player 500hp 400dps | cover 1 hard / 0 soft / 0 scattered
  OnEnter(handoff_berth): 1 hostile(s), 400 dps, 1 tube(s), closest 301u, TTK vs player 1.2s
  OnEnter(burn_buoy): 1 hostile(s), 400 dps, 1 tube(s), closest 301u, TTK vs player 1.2s
WARN  [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(handoff_berth)) spawns 301u from the player spawn, inside its own 450u effective range - a mid-fight reinforcement arriving on top of the fight
WARN  [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(burn_buoy)) spawns 301u from the player spawn, inside its own 450u effective range - a mid-fight reinforcement arriving on top of the fight
balance_audit: 11 combat scenario(s), 0 error(s), 2 warning(s)
```

`cargo run -p nova_assets --bin content_lint` (exit 0):

```
WARN  [the-ledger] scenario 'ledger_ch4_the_buyer': object id 'auditor' is spawned by more than one handler - fine only if the handlers are mutually exclusive
content_lint: clean (1 warning(s))
```

## Round 2

- VERDICT: APPROVE

Re-review of commit 804b7780 against the Round 1 findings. Verified per
finding (diff re-read, tests and report re-run from the worktree):

- R1.1 RESOLVED (ticked). `ShipStats::threat_envelope()` = max(turret
  reach, tubes -> `TORPEDO_ENVELOPE` 1000u mirroring AI_TORPEDO_MAX_RANGE),
  and BOTH rules gate on it (`envelope <= 0.0` = unarmed, then
  `distance >= envelope` skip) - a tube-only hostile can no longer evade,
  and a mixed-armament hostile's tube reach is priced. The fail-first
  `a_tube_only_onstart_ambusher_is_spawned_dead` is in-tree and passes
  (7/7). Real-tree spot checks: broadside_gunship (2 tubes, OnStart at
  1214u >= 1000u) still grades clean; ch4's auditor now warns via the
  1000u envelope (verbatim below); gate still 0 errors. No previously
  clean shipped hostile flipped (only ch4 carries triggered tubes).
  The guard hoist preserves the ERROR predicate exactly (armed +
  OnStart + inside own envelope).

- R1.2 STILL OPEN (MINOR, residual - checkbox left unticked). The module
  and NOTES.md now say BURST dps with the sustained caveat - the source
  of truth is fixed. But two of the places the finding named still carry
  the old label: web/src/wiki/dev/guide-author-scenario.md:79 ("combined
  sustained dps") and tasks/20260717-143806/TASK.md:15 ("TTK 1.2s
  sustained"). The wiki is the mod-author-facing doc, so the overclaim
  the finding was about is still published there. One-line fixes in each
  file ("combined burst dps" / "TTK 1.2s burst"); while there, the
  wiki's "inside its own effective range" could say "threat envelope" to
  match the shipped rule, and note the module doc's blanket "~62%"
  sustained figure is the better turret's number (the light turret sits
  at ~71%). Does not gate: MINOR, docs only, the shipped numbers and
  rules are correct.

- R1.3 RESOLVED (ticked). The docstring no longer claims "the SAME id
  space"; it scopes the equivalence to today's shipped bundles (each
  declares only base) and names the divergences (transitive dep graphs,
  intra-bundle first-wins) with a revisit trigger. The sibling-dep
  order tiebreak is not spelled out, but it is a sub-case of the named
  topo-sort divergence; the overclaim - the finding's core - is gone.

- R1.4 RESOLVED (ticked). The predicate is now
  `!matches!(ship.allegiance, Some(Allegiance::Neutral) |
  Some(Allegiance::Player))`, so an authored `Some(Enemy)` is audited;
  the match is exhaustive over the three-variant enum by inspection.
  The suggested explicit `Some(Enemy)` unit test was not added -
  accepted as-is given the predicate is compile-checked against the
  enum, but it would still be a cheap pin if a variant is ever added.

- R1.5 RESOLVED (ticked). Scattered cover now splits hard/soft by
  template hardness; the gauntlet's 22 invulnerable belt-wall rocks
  report as `scattered 22 hard 0 soft` (verbatim below) and the false
  "all shipped scatters are destructible chaff" claim is gone. Residual
  accepted without a finding: a scattered Spaceship template would
  count as scattered_soft and stay hostile-invisible - unchanged from
  Round 1, zero shipped occurrences (all 9 scatter sites re-verified
  Asteroid/SalvageCrate), and the ask's WARN-on-scattered-ships option
  remains a cheap future hardening if mods start scattering ships.

- R1.6 RESOLVED (ticked). SetHealth overrides apply in the hp sum via
  `.iter().rev().find_map(..)` = last SetHealth wins, matching the
  runtime's insert-per-modification (one component type per entity, last
  insert wins). Applies to player and hostile ships alike, both section
  sources.

No new problems introduced by the fixes found: the report format change
is consistent everywhere it is printed (bin + gate failure message), the
renamed finding messages stay unique per rule, and the WARN threshold
change flips no shipped scenario into an ERROR.

### Verification record (Round 2)

```
test balance::tests::a_tube_only_onstart_ambusher_is_spawned_dead ... ok
test balance::tests::a_triggered_mook_outside_its_own_reach_is_clean ... ok
test balance::tests::scenarios_without_a_player_are_skipped ... ok
test balance::tests::ship_stats_sum_the_resolved_sections ... ok
test balance::tests::an_armed_onstart_hostile_inside_its_range_is_spawned_dead ... ok
test balance::tests::triggered_close_spawns_warn_and_unarmed_ships_pass ... ok
test balance::tests::catalog_overlay_is_last_wins ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 71 filtered out; finished in 0.00s

test shipped_content_carries_no_balance_errors ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo run -p nova_assets --bin balance_audit` (exit 0), key lines:

```
[base] broadside_gunship: player 700hp 400dps | cover 5 hard / 0 soft / scattered 0 hard 24 soft
  OnStart: 1 hostile(s), 800 dps, 2 tube(s), closest 1214u, TTK vs player 0.9s
[gauntlet] gauntlet_run: player 500hp 400dps | cover 9 hard / 0 soft / scattered 22 hard 0 soft
WARN  [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(handoff_berth)) spawns 301u from the player spawn, inside its own 1000u threat envelope - a mid-fight reinforcement arriving on top of the fight
WARN  [the-ledger] ledger_ch4_the_buyer: close-spawn: 'auditor' (OnEnter(burn_buoy)) spawns 301u from the player spawn, inside its own 1000u threat envelope - a mid-fight reinforcement arriving on top of the fight
balance_audit: 11 combat scenario(s), 0 error(s), 2 warning(s)
```

- R1.2 residual (Round 2): fixed - guide-author-scenario.md and the
  follow-up task 20260717-143806 now say burst dps too.
