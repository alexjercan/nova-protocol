# Scenario balance audit rig: computed exposure/TTK metrics over shipped scenario RON

- STATUS: CLOSED
- PRIORITY: 49
- TAGS: spike,v0.7.0,tooling,balance,testing

Goal: balance is currently judged by feel, and the developer's skill is far
above the average player's, so "feels hard to me" means "impossible for
them". Build a rig that computes fairness metrics from shipped scenario RON
so balance becomes reviewable and regression-guarded (repo lesson
authored-vs-derived-values: encode invariants as computed assertions over
shipped content).

Metrics per scenario (from the spike's analysis):
- combined enemy dps at each spawn trigger, and simultaneous-shooter count
  over the script's act structure;
- spawn distance vs each weapon's effective range (0.9 x muzzle_speed x
  projectile_lifetime) - flag spawns inside kill range;
- cover count and hardness (invulnerable vs destructible HP vs attacker
  dps) between spawn bearings and the fight space;
- TTK vs the scenario's player ship (per-section HP against aligned dps).

Direction notes:
- Shape: an example/bin like content_lint (compare
  crates/nova_assets/src/bin) walking base + webmods scenarios; assertions
  for the reworked scenarios' invariants (e.g. "wave 2 never spawns inside
  400u"), report mode for the rest.
- Run it over asteroid_field, ledger_ch3/ch4 and future content - nobody
  has balance-checked those; the user never got past ch2.

Spike: tasks/20260717-111808/SPIKE.md (Recommendation item 5)

Verified at plan time: shipped weapon/hull numbers live in
SectionConfig { base.health, kind: Turret(fire_rate, bullet_damage,
muzzle_speed, projectile_lifetime) } (base_section.rs:30-94); the
content-tree walk exists in nova_assets::lint_walk (bundle manifests +
parsed Content, deps resolved base-first) and needs a small public
surface for full section configs; section id resolution is a LAST-WINS
overlay across base -> declared deps -> own content (the
mod-dependency-overrides-are-load-bearing lesson - the audit must join
stats through the same overlay). The per-scenario invariant PINS for the
reworked encounters already live in ledger_ch2_encounter.rs /
broadside_assault.rs - the rig is the repo-wide generalization, not a
duplicate.

## Steps

- [x] nova_assets::balance module: last-wins SectionCatalog overlay
  (base -> deps -> own); audit_scenario(scenario, catalog) ->
  Option<ScenarioAudit> (None without a Player spawn: menu scenes skip);
  per spawn-group metrics (hostile count, per-hostile dps = sum
  fire_rate x bullet_damage, effective range = 0.9 x muzzle_speed x
  projectile_lifetime, distance from the player spawn, TTK vs the player
  ship's summed section HP); cover counts (fixed invulnerable vs
  destructible + scatter fields); findings(): ERROR spawned-dead (an
  armed hostile spawned by OnStart inside its own effective range of the
  player spawn), WARN close-spawn (a TRIGGERED armed hostile spawned
  within 400u of the player spawn - the static proxy for "wave 2 never
  spawns inside 400u").
- [x] lint_walk: expose the walked bundles (id, dependencies, full
  section configs, scenarios) for the audit without duplicating the
  reader.
- [x] balance_audit bin (sibling of content_lint): walk, audit, print
  the per-scenario metric table + findings; exit non-zero on ERROR.
- [x] balance_audit_gate integration test (sibling of
  content_lint_gate): the same walk must produce zero ERROR findings -
  the repo-wide fairness regression gate. Unit tests: overlay last-wins
  join, dps/range/TTK math, and a synthetic spawned-dead scenario that
  MUST produce the ERROR (the rule's fail-first, permanently in-tree).
- [x] Run the report over the whole tree; record asteroid_field and
  ledger ch3/ch4 numbers in NOTES.md (the spike's unmeasured-content
  open question). Findings in shipped content: fix here only if
  one-line-content-level; otherwise file follow-up tasks with the
  numbers (cross-cycle-warning-with-numbers).
- [x] Docs: dev wiki guide-author-scenario.md or development.md tooling
  note (how to run the audit, what the findings mean); CHANGELOG
  (Internals & Tooling); NOTES.md design record.
- [x] Verify: cargo test -p nova_assets --test balance_audit_gate (+ the
  unit tests); content_lint still clean; cargo check --workspace
  --all-targets; fmt last. Full suite on CI.

## Close-out record

All seven steps landed; design, the first-run numbers and the rule
refinement story are in NOTES.md. Headlines: nova_assets::balance +
balance_audit bin + balance_audit_gate CI test; zero errors repo-wide
(the reworks hold); the one WARN is a real discovery - ledger ch4's
Auditor spawns hot on both ending branches - filed with numbers as
20260717-143806. The WARN rule was refined against its own first run
(shakedown's 395u mook exposed the 400u blanket as blunt; the
self-scaling own-envelope predicate replaced it).

Verification: balance tests 6/6, gate 1/1 over the real tree,
content_lint clean, workspace --all-targets green, fmt last. Full suite
on CI per standing instruction.
