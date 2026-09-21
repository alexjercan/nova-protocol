# Review 06: Ship weapon sections

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: no findings

## Adjudication

No BLOCKER, MAJOR, or MINOR finding survived.

The pair checked and cleared:

- turret, torpedo, and railgun ammo spend and shot commit ordering;
- reload ordering after all section fire systems;
- torpedo shot-down, detonation, and deferred despawn races;
- safety, stow, door, charge, and arming gates;
- unlimited and limited ammo behavior;
- per-shot render-asset reuse and thruster material pooling;
- raw physics pose use during fixed-step projectile spawn;
- authored weapon and section ID consumers;
- zero, NaN, and degenerate authored-value guards.

A possible repeated `error!` in the thruster impulse hot path was not raised because no reachable production state or log-spam reproduction was established.

## Coverage

Across the pair, every file for ammo, controller, thruster, turret, torpedo, and railgun sections was read in full, including inline tests and `railgun_section/tests.rs`. `sections/mod.rs` and catalog IDs were also checked for registration, ordering, and content consumers.

One reviewer sampled several long implementation and test regions rather than reading them fully; the other reviewer reported full line-by-line coverage of the same files. The pair therefore provides full static coverage for the batch.

Not checked: no Cargo command, content lint, game, probe, wasm build, workspace test, or Clippy run. Shipped balance values were not judged. Docking, hull, integrity, shell/skin, shared animation, camera, and ship audio remain in review 07. Cross-crate scenario and HUD behavior was not reviewed beyond direct contracts.
