# The Ledger

The alt storyline campaign, published on the mod portal (task
20260716-123535, spike tasks/20260716-183104/SPIKE.md): a salvage crew in
over its head, told across four chained chapters (five scenario files -
chapter two plays in two acts, each its own retry point). Install it from the
in-game portal (Mods > Explore), enable it, and start "The Ledger 1: Dead
Weight" from the Scenarios picker - chapters two through four are hidden
and reached by playing.

You fly the salvage tug Kestrel for Mesa Verde Reclamation. A routine
wreck-strip turns up a sealed military black box nobody logged, and the
belt starts paying attention.

1. **Dead Weight** - strip the Ceres Matron for the quota; the fourth ping
   is not on the manifest.
2. **Claim Jumpers / The Heavies** - the Magpies come to take it, two
   waves in two acts; keep them off the Dray Mule. Breaking wave one is a
   checkpoint: dying to the heavies retries the heavies.
3. **The Quiet Channel** - run dark to Broker Vesh's yard, threading the
   NAV drops in order: a real stealth run. Two Magpie pickets patrol the
   flanks, cold and neutral until provoked - stray into their watch, paint
   one, or burn too hot (over 8 u/s is noise they hear) and both go hot.
   Overspeed warns first: the second time you push it, they wake. The debris
   pinch between the first two drops is the blind spot: thread the wrecks
   slow and slip past unseen for Vesh's quiet-scopes payoff, or wake them and
   fight it out. The drops are the job either way.
4. **The Buyer** - sell the box or burn it, decided at two beacons, and the
   endings diverge. Sell it at Vesh's berth and the sale calls the Auditor
   down: break the gunship for the payday. Burn it at the buoy and there is
   nothing left to collect - the Auditor never comes, no fight, but no
   payout either. Clear, and broke.

Authoring notes: hand-written RON on base-game assets and section
prototypes only (the gauntlet path); every fight, gate, pickup and branch
uses shipped scenario vocabulary - act-gated handlers, expression-guarded
OnEnter sequencing, per-id OnDestroyed counting, StoryMessage comms beats
(with `dwell` holds), Outcome + NextScenario chaining. Pacing is authored
too: each chapter opens with a clock-paced briefing conversation, objectives
lazy-post only once the briefing hands off, and `beat_gate` clock stamps
space the comms beats a beat apart (announce -> arrive -> confirm ->
breathe). Each chapter picks a deliberate starting cubemap and uses
`SetSkybox` to accent key beats (the chapter-one reveal, the chapter-three
pinch, the chapter-four sell path). The story deliberately carries no state
across chapters (scenario variables clear at teardown); the two-ending
branch lives entirely inside chapter four.
