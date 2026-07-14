# Spike: multi-file scenario bundles - folder + manifest, id namespacing/overlay, directory loader

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-110502/SPIKE.md

Goal (step 4, its OWN spike): a scenario as a folder + manifest ("bundle of files")
- separate files for sections, ships, objectives, events - loaded via a directory
`AssetLoader` (or a manifest that `load_context.load`s the parts). The real
uncertainty is id namespacing/overlay (scenario-local vs catalog, collisions, load
order), so this is a spike, not a plan yet. Gated on the prototype/catalog model
(steps 1-3) being proven first - do not attempt before them.
