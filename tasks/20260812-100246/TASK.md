# Spike: ship building from parts instead of cubes (design + prototype)

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: v0.10.0,spike,art,editor

Goal: design and prototype the transition from cube-section ships ("blocks")
to piece-based ships ("parts"), like the Fertile Soil Spaceship Blocks
Collection. Users should eventually build ships in the editor from parts
(fuselage, wings, cockpits, thrusters, weapons) instead of 1-unit cubes.
Better silhouettes, better colliders.

Closed after the spike verdict and prototype were consumed. v0.10.0 shipped the
authoritative link-point graph (`20260812-130953`) and semantic Racer/Cargo
migration (`20260812-131842`). Remaining editor snapping is narrowed into
`20260812-131005`; this spike no longer owns implementation.

Context:
- Today: ships = root + section children on a 1-unit grid; visuals come from
  cube .glb libraries cut by scripts/cut-obj-into-hulls.py (a quickstart
  prototype, by design cut-only). Colliders: SectionCollider primitives
  (crates/nova_ship/src/sections/base_section.rs).
- Reference pack: ~/Downloads/spaceship_blocks_collection.zip (CC0, verified
  on itch page 2026-08-12; record license at import - the zip has no license
  file). 95 OBJ+MTL pieces, TRUE flat Kd colours, no textures. Naming:
  Spacestation_<Category>_<Part>_<Variant> (Structure fuselage/wings/
  cockpits/runway/habitat, Propulsion thrusters/hyperdrive, Weapon modular
  gun parts, Miscellaneous). Pieces are multi-object OBJs with half-unit
  friendly bboxes (e.g. cockpit 2.2x1.2x1.0, wing 1.7x0.3x3.0).
- Research spike record: tasks/20260812-100256/SPIKE.md (art research 2026-08-12);
  helper scripts/inspect-obj-pack.py scores packs.

Scope:
- Design doc: part data model (mount points/attachment rules vs free
  placement), collider strategy (convex hull per part vs primitive fits),
  section-behaviour mapping (a part IS a section? parts group into
  sections?), editor UX sketch, migration path for existing cube ships +
  content RON, WASM/download size impact.
- Better slicing research: the current cutter is a proof of concept. Explore
  "one dedicated script per ship model" that generates NICE parts (wings,
  nose, engines) from existing .obj (Kenney, later Quaternius) the way the
  blocks collection ships pieces - mesh generation via Python from .obj
  input, following the repo stdlib-only asset-script convention.
- Prototype: runnable script(s) proving part extraction/generation; optional
  minimal in-game/editor wiring if cheap.

DoD:
- SPIKE.md in this task folder: design, tradeoffs, migration, verdict.
- A runnable prototype script with example output opened and inspected
  (exit status alone is not proof).
- Escalation plan: ordered follow-up tasks if the verdict is "go".
