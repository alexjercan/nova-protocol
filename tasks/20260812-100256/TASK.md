# Spike: art research round 2 - planet/asteroid textures + scene props

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.10.0,spike,art,content

Goal: continue the 2026-08-12 art research with acquisition and evaluation.
Focus: nicer planet and asteroid textures and nicer-looking objects around
the scene. Skyboxes are covered (space-3d output is good, keep it).

Closed as complete. `SPIKE.md` records rounds 1-3 and the ordered escalation
plan, `ASSETS.md` records inspected candidates and licenses, and
`DESIGN-round3.md` resolves the planet, triplanar asteroid, carving, and scene
prop directions. The `compare_asteroids` and `compare_planets` examples retain
the in-engine evidence. Follow-up implementation must be scheduled from the
round-3 escalation plan rather than reopening this research task.

Context:
- Current environment art: ONE asteroid texture (assets/base/textures/
  asteroid.png) on a procedural asteroid mesh; no planets, stations, or
  debris props; beacon/salvage render as primitives.
- SPIKE.md in this task folder holds the research round 1 findings (sources,
  licenses verified 2026-08-12, rejected list, escalation options).
- Relevance filter: this is an IN SPACE game - Kenney terrain/interior
  content is mostly not usable; craft/turrets/props are. Quaternius packs
  look really good (palette-atlas caveat in SPIKE.md).
- License policy: CC0 preferred, CC-BY 4.0 workable (credits/CREDITS.md),
  no NC/ND/SA/store licenses. Verify on source pages, record verified date.

Scope:
- Asteroid textures: find/evaluate better CC0 rock/asteroid textures
  (tileable, works on the procedural mesh); shortlist + acquire what is
  scriptable, list manual downloads otherwise.
- Planets: pick the concrete route (Screaming Brain Studios sphere-wrap
  textures vs Quaternius low-poly planets vs own Blender bake) and evaluate
  real candidates against the game's flat-shaded look; note that planets
  need a new scenario object type (code, separate escalation).
- Scene dressing: what else makes space feel alive (debris, derelicts,
  stations, distant traffic) - map wanted props to verified-license sources
  (Kenney space kit unused models, Fertile Soil pieces, Quaternius).

DoD:
- SPIKE.md updated with round 2: candidates evaluated (not just listed),
  license + verified date each, downloads acquired or listed for manual
  download, and a concrete recommendation per category.
- Escalation plan: ordered follow-up tasks (e.g. planet scenario object,
  asteroid texture swap, props import).
