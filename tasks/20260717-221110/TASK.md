# Demo scenario spawning reconstructed Kenney ship from cut sections + content lint acceptance

- STATUS: OPEN
- PRIORITY: 0
- TAGS: spike,backlog,tooling,modding

## Goal

Wire a demo scenario or example that spawns the reconstructed Kenney ship from
the cut sections at their grid positions, proving the pieces snap back into the
original `craft_cargoB` silhouette. Resolve the forward-axis / nose-vs-tail
orientation empirically here. Acceptance:
`cargo run -p nova_assets --bin content -- lint --target <mod-path>` passes and
the reconstructed ship renders correctly.

## Steps

- [ ] Register the cut mod in `assets/mods.catalog.ron` (or a dedicated demo catalog entry).
- [ ] Resolve forward-axis / nose-vs-tail: place one cut piece and confirm Kenney +z maps to the game's expected forward.
- [ ] Add a demo: an example (`examples/NN_kenney_reconstruct.rs`) or scenario that spawns the ship from the assembly emitted by task 221106.
- [ ] Run `cargo run -p nova_assets --bin content -- lint --target assets/mods/<id>` and make it pass.
- [ ] Verify the reconstructed ship renders as `craft_cargoB` (screenshot or example run).

## Notes

- Spike: tasks/20260717-220919/SPIKE.md (open question: axis orientation)
- Depends on tatr 20260717-221106 (the mod scaffold + assembly)
