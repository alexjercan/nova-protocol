# Playable capital-combat vertical-slice scenario

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,example,scenario

Spike: tasks/20260708-203517/SPIKE.md

North-star demo for the "shippable game" identity: one real capital-combat fight
that dogfoods every juice + combat system end to end, rather than the isolated
test-range examples v0.4.0 built. Your ship vs an enemy that launches torpedoes
you must screen with the PDC, with sections coming apart under fire.

Goal: a single playable scenario that exercises the whole loop - handling, PDC
screening, torpedo offense/defense, section destruction, audio, hit FX, and HUD -
under a win/lose frame.

Depends on: the v0.5.0 feel systems (handling, audio 162011, hit FX 162013,
smarter AI 162012) - all shipped. Build it on the RON scenario format
(20260525-133029) so the flagship scenario also dogfoods the authoring format.

This task now OWNS the win/lose frame. The legacy objectives tasks (133026/133027)
were closed 20260714: the objective foundation and HUD conveyance shipped in
v0.5.0, and an explicit win/lose state was never built - it belongs here, on top
of the RON format, as part of the vertical slice rather than as a standalone
legacy task.

