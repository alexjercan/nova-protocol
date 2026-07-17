# Scenario balance audit rig: computed exposure/TTK metrics over shipped scenario RON

- STATUS: OPEN
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
