# Replace cube ships with parts ships (no compat, fix what breaks)

- STATUS: OPEN
- PRIORITY: 70
- TAGS: v0.10.0,ship,content

Goal: replace the cube-section ship models (racer, cargob) with the tuned
parts models. PURE REPLACEMENT per owner direction 2026-08-12: no
backward-compatibility effort - swap the models, fix the errors that happen
or break the game. Balance breakage is accepted (few mods, little gameplay
content); re-tuning is a later pass.

Depends: 20260812-130953 (graph adjacency gate - parts do not weld on the
distance==1.0 glue).

Inputs (ship-parts branch, task 20260812-100246 rounds 1-4):
- scripts/cut-obj-into-parts.py (solid plane-aware caps) + tuned recipes in
  scripts/part-recipes/.
- art/part-candidates/ racer 7 + cargob 7 parts.

Scope:
- Regenerate final part glbs -> assets/base/gltf/parts/ (they now ship).
- Credits: CREDITS.md entries + credits/licenses/ CC0 texts for whatever
  ships (Fertile Soil only if block pieces ship; Kenney already covered).
- Content builders (never generated RON by hand): part section prototypes
  (render_mesh per part, primitive colliders, simple HP values), redefine
  the racer/cargob ship prototypes as parts; content -- gen; content lint.
- DELETE the cube libraries (assets/base/gltf/cargob/, racer/) and cube
  section prototypes in the same change; fix all fallout (scenarios,
  examples, editor palette, lints, webmods example) as it surfaces.
- Re-run the probe sweep; refresh screenshot captures (visuals change);
  update wiki sections pages per the docs routing map.

DoD:
- A scenario plays end to end with parts ships (player-path harness).
- probe run --all green; zero cube glbs or cube prototypes left.
- Balance drift noted in NOTES.md, not fixed here.
