# Off-screen target/threat edge indicators (HUD)

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.5.0, hud, spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 3. When a locked target or an incoming threat (e.g. a hostile torpedo) is off
the screen, show a direction arrow clamped to the screen edge pointing at it, so the
player knows where to turn. Consumer of the screen-projected-indicator widget
(20260708-165700), which should own the off-screen edge-clamping logic.

"Threat" needs a notion of what is dangerous to the player (incoming torpedoes,
hostiles); scope that during planning - a first cut can just point at the current
lock and at committed enemy torpedoes.
</content>
