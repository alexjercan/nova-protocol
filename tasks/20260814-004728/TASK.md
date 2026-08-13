# Menu backdrop battle scenes and cleanup engine work

- STATUS: OPEN
- PRIORITY: 65
- TAGS: v0.11.0,scenarios,menu

## Goal

Add three combat-flavored menu backdrop scenes and the engine work they
expose: debris cleanup, deterministic asteroid geometry, and AI obstacle
avoidance for passive flight.

## Scope

- Scene 1 "torpedo gauntlet": solo racer in an asteroid ring; off-screen
  torpedo battery fires at it; the racer's PDC turrets shoot torpedoes down.
- Scene 2 "asteroid weave": AI ship flies patrol waypoints through a dense
  asteroid field and visibly avoids rocks.
- Scene 3 "duel cycle": two ships fight, one wins, an unstoppable heavy
  torpedo destroys the winner; after a beat two new ships fly in and the
  cycle repeats forever.
- Engine: asteroid mesh fragments get a despawn lifetime (today they
  persist until scenario teardown).
- Engine: authorable asteroid silhouette seed; scatter derives per-rock
  seeds from the scatter seed so field geometry is stable across runs.
- Engine: passive-flight obstacle avoidance (detour steering around
  bodies with a BodyRadius) so patrol routes survive rock placement.
- Content: heavy torpedo section prototype for the scene-3 finisher.
- Dev knob: force a specific menu backdrop for capture/verification.

## Definition of done

- Three new scenarios registered, generated, bundled; content lint clean.
- New engine behavior covered by lib tests in the owning crates.
- Backdrops verified running (screenshots), not only compiled.
- Wiki pages updated per docs routing map; CHANGELOG entries added.
