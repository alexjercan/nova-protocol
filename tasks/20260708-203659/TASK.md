# Playable capital-combat vertical-slice scenario

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.6.0,example,scenario

Spike: tasks/20260708-203517/SPIKE.md

North-star demo for the "shippable game" identity: one real capital-combat fight
that dogfoods every juice + combat system end to end, rather than the isolated
test-range examples v0.4.0 built. Your ship vs an enemy that launches torpedoes
you must screen with the PDC, with sections coming apart under fire.

Goal: a single playable scenario that exercises the whole loop - handling, PDC
screening, torpedo offense/defense, section destruction, audio, hit FX, and HUD -
under a win/lose frame.

Depends on: the v0.6.0 objectives/win-lose work (20260525-133026/133027) for the
frame, and ideally the v0.5.0 feel systems (handling, audio 162011, hit FX 162013,
smarter AI 162012) landing first so there is juice to show. Lands in v0.6.0 once
objectives exist. Build it on the RON scenario format (20260525-133029) if that
has landed, so the flagship scenario also dogfoods the authoring format.

