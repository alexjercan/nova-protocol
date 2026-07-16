# The Ledger

The alt storyline campaign, published on the mod portal (task
20260716-123535, spike tasks/20260716-183104/SPIKE.md): a salvage crew in
over its head, told across four chained scenarios. Install it from the
in-game portal (Mods > Explore), enable it, and start "The Ledger 1: Dead
Weight" from the Scenarios picker - chapters two through four are hidden
and reached by playing.

You fly the salvage tug Kestrel for Mesa Verde Reclamation. A routine
wreck-strip turns up a sealed military black box nobody logged, and the
belt starts paying attention.

1. **Dead Weight** - strip the Ceres Matron for the quota; the fourth ping
   is not on the manifest.
2. **Claim Jumpers** - the Magpies come to take it; keep them off the
   Dray Mule.
3. **The Quiet Channel** - run the box through the debris channel to
   Broker Vesh's yard.
4. **The Buyer** - sell it or burn it, then survive the Auditor. Two
   endings, decided at two beacons.

Authoring notes: hand-written RON on base-game assets and section
prototypes only (the gauntlet path); every fight, gate, pickup and branch
uses shipped scenario vocabulary - act-gated handlers, expression-guarded
OnEnter sequencing, per-id OnDestroyed counting, StoryMessage comms beats,
Outcome + NextScenario chaining. The story deliberately carries no state
across chapters (scenario variables clear at teardown); the two-ending
branch lives entirely inside chapter four.
