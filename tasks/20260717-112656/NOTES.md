# Balance audit rig - design record

Task 20260717-112656, spike tasks/20260717-111808/SPIKE.md
(Recommendation item 5). The last task of the difficulty-rework family.

## What shipped

- `nova_assets::balance`: SectionCatalog (last-wins overlay base ->
  declared deps -> own, the runtime merge's order - the gauntlet's dep
  once rebalanced a base hull by override, so the join goes through the
  overlay, never base alone); ship_stats (hp sum, BURST turret dps =
  fire_rate x bullet_damage (first-magazine rate; reload cycles put true
  sustained at ~62% for the catalog turrets, but every shipped TTK lands
  inside one magazine - review R1.2 renamed this honestly), max effective range = 0.9 x muzzle_speed x
  lifetime mirroring AI_FIRE_RANGE_FACTOR, torpedo tubes counted not
  dps-folded); audit_scenario (per-spawn-group hostiles with distances
  from the player spawn, TTK vs the player pool, cover tiers; None
  without a Player spawn so menu scenes skip); findings.
- Post-review (R1.1 MAJOR): the envelope is now threat_envelope() =
  max(turret reach, tubes -> 1000u AI launch envelope) - a tube-only
  ambusher can no longer evade the rules (its bay's first-launch cooldown
  starts elapsed, so it is a live opening threat); permanent tube-only
  fail-first test added. The ch4 auditor now grades via its 1000u tube
  envelope. Hostile predicate covers authored Some(Enemy); SetHealth
  modifications apply to hp sums; scattered cover splits hard/soft by
  template (the gauntlet's 22 invulnerable belt-wall rocks now report as
  hard); the overlay docstring states exactly where the static join and
  the runtime merge diverge (transitive deps, intra-bundle dupes).
- Two graded findings, both scaled by the hostile's OWN threat envelope:
  ERROR spawned-dead (OnStart, inside own effective range of the player
  spawn) and WARN close-spawn (same predicate, triggered handler).
  The first draft used a 400u blanket for the WARN; the first real run
  flagged shakedown's light-turret scavenger at 395u - a mook 125u
  OUTSIDE its own reach, an approach not an ambush - which drove the
  self-scaling refinement (measure-before-writing-the-number, lived).
- `lint_walk::audit_bundles()`: the existing walk exposed with full
  section configs; `balance::audit_content_tree()` joins per bundle.
- `balance_audit` bin (report + exit code) and `balance_audit_gate` CI
  test (zero ERRORs repo-wide). The ERROR rule's fail-first is a
  permanent unit test: a synthetic pre-rework-ledger_ch2-shaped scenario
  MUST grade spawned-dead, and the same hostile moved out grades clean.

## First-run results (the spike's unmeasured-content question, answered)

11 combat scenarios audited. Zero errors - the ch2/broadside reworks
hold. Headline numbers now on record: broadside_gunship 800dps capital,
TTK 0.9s burst vs the 700hp chapter-two ship (the setpiece, checkpointed);
ledger_ch2 waves TTK 2.6s / 1.0s at 622u/922u; ledger_ch3's magpies
enter at 287/296u but with 270u light-turret reach (clean by the
envelope rule - marginal, worth a playtest eye); ledger_ch4's AUDITOR
spawns at 301u inside its 450u envelope with a torpedo tube on BOTH
ending branches, vs a 500hp player with 1 hard rock - the one WARN, and
the first genuinely unknown finding. Filed with the numbers as task
20260717-143806 (drama-vs-fairness playtest decision, does not gate).

## Decisions

- WARN does not gate CI: a triggered hot spawn can be intended drama
  (ch4's finale entrance may be exactly that); the ERROR (opening
  UNDER fire) has no legitimate reading.
- Torpedo tubes reported, not folded into dps: blast + guidance is not
  sustained fire; folding it would fake precision.
- Kinetic-only damage model (resistance 1.0 everywhere in the shipped
  table, all catalog turrets kinetic); revisit if typed enemy loadouts
  ship.
- The per-scenario invariant PINS stay in the encounter tests; the rig
  is the repo-wide floor, not a replacement.

## Verification

- balance unit tests 6/6 (overlay, stats math, spawned-dead fail-first +
  delivery guard, triggered WARN + unarmed clean, mook-outside-reach
  clean, no-player skip); balance_audit_gate 1/1 over the real tree;
  content_lint still clean; bin exits 0 with 0 errors / 2 warnings.
- cargo check --workspace --all-targets green; fmt last. Full suite on
  CI per standing instruction.
