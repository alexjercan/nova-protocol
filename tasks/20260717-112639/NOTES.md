# Broadside act-split + cover hardening - design record

Task 20260717-112639, spike tasks/20260717-111808/SPIKE.md (F4/F7).
Sibling of the ledger_ch2 rework (20260717-112630); the engine mechanic
both lean on is the AI line-of-fire gate (20260717-112622).

## What shipped

- The chapter is now two BUILDER-GENERATED scenarios
  (crates/nova_assets/src/scenario/broadside.rs is the single source; the
  committed RON is written by `cargo run -p nova_assets --bin gen_content`
  and byte-pinned by content_ron_parity):
  - `broadside` (picker entry, id unchanged so shakedown's chain-in is
    untouched): approach + corvette ambush. Both corvettes down = the
    chapter CHECKPOINT: Victory beat + lingering NextScenario into part
    two. Defeat gates tightened from act < 3 to act < 2; retry = itself.
  - `broadside_gunship` (hidden): player, hauler, the same chaff scatter
    (same seed) and boulders, and the gunship spawned at OnStart - its
    ~720u burn toward the fight is the act's pacing, tubes open on
    approach (unchanged setpiece). Victory ends the base story (nothing
    queued); death retries THIS part.
- Hard cover: five invulnerable boulders shared by both parts (nominal
  r3.5-5 = 12-30u bodies at the 3.5x-6x geometric factor): three anchor
  the corvette fight north of the hauler (z -520..-575), two sit on the
  gunship lane (z -700..-750). All outside the destructible scatter box
  (z >= -430), so chaff can never merge with an anchor. The corridor,
  overlap and station clearances are computed pins in
  broadside_assault.rs (hard_cover_anchors_both_threat_lanes), not
  eyeballed numbers.
- Consumers updated: broadside_assault.rs (checkpoint walk, part-B walks,
  per-part retries, hidden pin, bundle pin, geometry pins),
  examples/19_broadside.rs (new stage: ride Continue through the
  checkpoint into part two; runway 40s -> 50s for the third scenario
  load), base.bundle.ron (+ gunship file), scenarios.md wiki, CHANGELOG.
- Fixed in passing: player_ship()'s doc comment still claimed infinite
  ammo from a superseded playtest verdict; the code moved to finite
  auto-reload ammo in 20260717-085640. Comment now matches code.

## Decisions

- Part one keeps the corvette fight EXACTLY as shipped (light turrets,
  550u spawns, patrol + leash 420, bracketing flanks): the spike rated it
  "hard but playable", and this task's mandate was checkpoint + cover,
  not re-geometry. The bracketing-vs-single-lane question for broadside
  belongs to the balance audit rig (20260717-112656) with data.
- The gunship keeps its spawn distance, loadout, and both torpedo tubes:
  the PDC-screen setpiece is the chapter's identity; the fairness lever
  is that failing it no longer costs the ambush replay.
- Boulder destroy_sound: None (nothing can destroy them); impact_sound
  kept so rounds expending on cover stay audible.

## Verification

- gen_content run TWICE, git status identical (generator stable).
- content_ron_parity (parity + bundle uniformity): 2 passed.
- broadside_assault: 10 passed (includes the new checkpoint, per-part
  retry, hidden, and geometry tests).
- ledger_ch2_encounter: 12 passed (unaffected, re-run after merge).
- content_lint: clean (pre-existing ch4 warning only).
- cargo check --workspace --all-targets green (includes the example).
- Full suite intentionally left to CI per standing instruction.
