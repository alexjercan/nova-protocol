# Minimal faction/relation model (hostile/neutral/own)

- STATUS: OPEN
- PRIORITY: 62
- TAGS: v0.4.0,factions,ai,hud

Spike: docs/spikes/20260708-203517-roadmap-reprioritization-and-juice.md

Enabling task: both the smarter-AI work (20260708-162012) and reticle-by-relation
colouring in the weapons HUD (flagged as an open question in
docs/spikes/20260708-165647-weapons-hud.md) need a notion the game does not have
yet - who is hostile to whom. There is no faction/relation concept today.

Goal: a minimal, lightweight relation model - hostile / neutral / your-own -
sufficient for v0.5.0, not a full faction/reputation system.

Direction (for /plan to break into steps):
- A small component/relation expressing an entity's allegiance (e.g. player,
  enemy, neutral) and a helper to resolve the relation between any two entities.
- Consumers: AI target selection filters to hostiles; HUD reticles/pips colour by
  relation (hostile vs your-own-torpedo vs neutral).
- Keep it deliberately small; a fuller faction system (alliances, reputation) is
  its own future spike if the game ever needs it.

