# Emit mod scaffold: bundle.ron + content.ron with Hull/Thruster/Controller classification + assembly

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: spike, backlog, tooling, modding

## Goal

Extend the cutter to emit the standard mod scaffold beside the `.glb` pieces:
`<id>.content.ron` (a `[Content]` list of `Section((base, kind))` items) and
`<id>.bundle.ron` manifest listing every glb in `resources` + `meta`. Classify
each cell into a section kind via heuristic (`--classify`): rear `-z` cells that
contain `metalRed` faces -> `Thruster`; the most-central occupied cell ->
`Controller`; everything else -> `Hull`; each with sensible base stats
(mass/health) and its `render_mesh: Some("self://gltf/<piece>.glb#Scene0")`.
Also emit the reconstruction assembly: the list of `SpaceshipSectionConfig`
(`position: Vec3(i,j,k)`, `source: Prototype("<id>")`) for the demo task.

## Steps

- [ ] Classify each non-empty cell: rear (`min k`) cells containing `metalRed` faces -> `Thruster`; most-central occupied cell -> `Controller`; rest -> `Hull`.
- [ ] Section stats per kind (mass/health, plus Thruster `magnitude`, Controller `frequency`/`damping_ratio`/`max_torque`).
- [ ] Emit `<id>.content.ron`: a `[Content]` list of `Section((base:(id,name,description,mass,health), kind:...))` with `render_mesh: Some("self://gltf/<piece>.glb#Scene0")`.
- [ ] Emit `<id>.bundle.ron`: manifest with `content`, `resources` (every glb), `meta` (name/description/author/version).
- [ ] Emit the reconstruction assembly (`SpaceshipSectionConfig` list: `position: Vec3(i,j,k)`, `source: Prototype("<id>")`) as a RON snippet for the demo.
- [ ] Write the mod under `assets/mods/<id>/`; `--classify` toggles kind heuristic.

## Notes

- Spike: tasks/20260717-220919/SPIKE.md (section format + classification)
- Depends on tatr 20260717-221101 (the cutter)

## Descoped 2026-07-17

Closed without implementing. Direction changed: the `cut-obj-into-hulls.py`
script is kept narrowly scoped to CUTTING the obj into `.glb` cube meshes only.
Section classification (Hull/Thruster/Controller), the mod scaffold
(bundle.ron/content.ron), and reconstructing the ship are NOT done by the Python
tool - they will be handled in-game later. Re-plan fresh tasks when that work is
picked up; this task's steps no longer reflect the intended approach.
