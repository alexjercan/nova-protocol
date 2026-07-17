# Emit mod scaffold: bundle.ron + content.ron with Hull/Thruster/Controller classification + assembly

- STATUS: OPEN
- PRIORITY: 0
- TAGS: spike,backlog,tooling,modding

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

## Notes

- Spike: tasks/20260717-220919/SPIKE.md (section format + classification)
- Depends on tatr 20260717-221101 (the cutter)
