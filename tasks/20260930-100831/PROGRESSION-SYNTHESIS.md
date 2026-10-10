# Open-world progression and Plan D: comparative research synthesis

Research only on master `467ae965b`, 2026-10-10. No implementation, runtime
play, build, benchmark, commit or owner selection. Sources:
[RESEARCH.md](RESEARCH.md), [PROGRESSION-BUILDER.md](PROGRESSION-BUILDER.md),
[PROGRESSION-WORLD.md](PROGRESSION-WORLD.md); an independent adversarial source
review of both papers. External-game claims are illustrative, not Nova proof;
community-only or inaccessible claims do not determine the recommendation.

## Claim and evidence

The shipped New Game has no open-world objectives and starts with a line
warship, credits and ammo (`crates/nova_authoring/src/base_content/scenarios/open_world.rs:1-9,72-101`). Mining, trade and repair work, but no in-world
section swap, production recipe or player-built station is present
(`crates/nova_gameplay/src/inventory.rs:478-534,793-825`,
`crates/nova_scenario/src/mining.rs:151-157`). Native saves already persist
visited sectors, changed asteroids, ships and canisters, including across
restart (`crates/nova_world/src/streaming.rs:954-959`,
`crates/nova_world/src/frozen.rs:386-400`,
`crates/nova_world_base/src/save/mod.rs:96-114`). Do not describe the open
world as resetting all player action or as having no persistence. The absence
of objectives is a source fact; whether players feel no purpose is an
unmeasured experience hypothesis.

## Three competing spines

| Choice | First thing the player accomplishes | First thing built/improved | Main cost and risk |
|---|---|---|---|
| A. Charter frontier | Reach a surveyed advancement band, then complete authored goals | Persistent chart/charter rank; later restoration or refit | Goals need saved state, target validity and proof of actual player actions; reaching a band alone can become a travel chore. |
| B. Salvage and refit | Discover a wreck, choose one placement to replace | Own and change a section on the ship | Runtime section replacement, saved-state rebasing and compatibility checks; resembles deleted ship-tuning work and offers no destination goal by itself. |
| C. Regional trade and infrastructure | Find and exploit a viable regional route | Cargo depot, then relay or station | Price/arbitrage and finite-stock balancing, permanent body IDs, storage and docking; requires an observed viable trade route. |

**Recommendation for research priority, not implementation:** A as the
motivational spine, with a small recovery or improvement sink from B after
its runtime/save contract is designed. C can add geographically different
reasons to travel; treat player-built stations as a later, separate feature.
An alternative is B first if the owner explicitly wants ship customization
back in scope. A static depot is not equivalent to a productive station:
off-window sectors do not simulate, save state has no world clock, and
station traffic and service ownership are absent
(`crates/nova_world/src/frozen.rs:266-275`,
`crates/nova_world_base/src/save/mod.rs:96-114`). Do not promise that
stations are easy.

## Evidence corrections that change the sequence

- `MinedOre` is owed by the rock, not player inventory
  (`crates/nova_scenario/src/mining.rs:204-213`). Trade, take, pickup,
  repair and cell crossing do not already emit charter completion facts;
  some existing dock/lock/destroy events carry only an id and type. A
  charter's completion owner must validate the action and target state
  directly, including generated objects. The lowest-risk candidate is a
  precisely defined advancement-band Reach predicate over seed and pose.
- A missing `FrozenShipState.sections` entry means destroyed, not fresh.
  Section thaw reapplies old health/ammo (`crates/nova_scenario/src/objects/spaceship.rs:578-581`,
  `crates/nova_ship/src/sections/frozen.rs:280-299`). Both section swap and
  patch offer need a live spawn/update seam and new compatible saved state;
  neither is a simple metadata change. A weaker start also needs a
  capacity-safe starting kit and new section bindings; current warship
  stock cannot simply move to a small cutter.
- Plan D is an owner-favored *research direction*, not an approved naming
  or overlay contract. `ItemType` is closed, price and mass are coded, and
  all seven current content kinds allow cross-pack last-wins
  (`crates/nova_gameplay/src/inventory.rs:39-149`,
  `crates/nova_assets/src/merge.rs:858-941`). An open ID breaks shipped
  bare-key stock RON in base builders and six nonempty Ledger examples;
  empty Gauntlet maps do not need a key rewrite. Native world saves are
  currently unshipped, but new item IDs still need an explicit save-format
  bump, last-good-save refusal, and all-saved-item-ID validation after
  parsing. Digest checking alone does not reject a malformed state with a
  string-backed unknown item (`crates/nova_world_base/src/save/mod.rs:297-323`).
- Before Plan D's collision rule, settle cycles, duplicate definitions
  inside one pack and which unrelated pack(s) to refuse. Quarantining the
  later pack makes enabled content depend on input order again. The
  world-generation section snapshot already rejects unrelated section
  duplicates but can panic, while the generic merge remains last-wins;
  new progression content kinds need a deliberate fail-closed policy.

## Conditional order and closest proof (all proposed)

1. **Measure one recorded native New Game journey**, with explicit seed,
   player goals and direct observations of mining yield, pickup, docked sale,
   travel and saving. Report elapsed observations descriptively, not FPS,
   economy or usability conclusions without matched evidence. This may
   change the ranking; previous bench runs did not establish a complete
   mine-to-sale path.
2. **Choose the motivational spine and ship-change boundary.** If A wins,
   define the exact Reach predicate, reward, player feedback and save owner;
   prove completion and fresh-process Load without any new item reference.
   Expand only when trade/pickup/neutralize facts have authoritative hooks.
3. **If ship improvement is in scope**, first design a credit-only
   restoration or narrowly authored offer with a section respawn and thaw
   rebase contract; assert live component plus fresh-process save state,
   failure atomicity and no stranded ship. Refit/stripping and a weaker
   starting hull are distinct owner decisions.
4. **Plan D near the end of the v0.16 sequence**, before any *new RON
   progression field* names items: typed `ItemDesignId`, catalog, pre-merge
   conflict refusal, RON migration, all consumer rewrites, strict save ID
   validation. Preserve modded canister->inventory->market quantities and
   explicit catalog mismatch refusal. Item identity does not itself grant
   ship refit, production or stations.
5. **Later:** item-cost recipes as their own content kind, surveyed regional
   demand, depot identity/storage, and finally station time/docking/traffic.
   Each requires an owner and proof beyond generic item RON support.

## Owner decisions still open

1. Purpose: goal/charter frontier, ship restoration/refit, or geographic
   trade as the first progression slice? Recommendation: charter frontier,
   conditional on measured New Game play.
2. Does restricted authored patch purchase or section swap reopen the
   ship-tuning/section-editing work previously cut from v0.16? Restoration
   of a destroyed section can be a separate recovery policy.
3. Retain the warship start, or explicitly rebalance a weaker start with
   compatible cargo/credits/weapon bindings?
4. When to admit stations: depot storage only, service ship, static station
   spike, or a player-buildable station? These have different save/docking
   and simulation costs.
5. Plan D failure semantics: namespace convention, mutual dependencies,
   duplicate-in-pack, unrelated collision handling without order dependence,
   old save refusal, and exact mod/catalog diagnostic. The directional
   preference for D does not answer these.

No tasks, implementation interfaces or content IDs are approved by this
synthesis. Each feature slice requires its own code-backed gate.
