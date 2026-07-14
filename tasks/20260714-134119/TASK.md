# Bundle manifest + loader + merge-by-kind router into id-keyed registries

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.6.0, modding, scenario, spike

Spike: tasks/20260714-113418/SPIKE.md

Goal: the core bundle mechanism (wasm-safe). A bundle is a directory with a
`bundle.ron` manifest listing its content files (relative paths); a bundle loader
`load_context.load`s each -> its typed asset by extension (`*.sections.ron` /
`*.ship.ron` / `*.scenario.ron`). A `merge_bundle` step routes each loaded asset into
its id-keyed registry by kind (sections -> `GameSections`, ships -> `GameShips`,
scenarios -> `GameScenarios`); adding a kind = one new arm. Manifest, NOT
directory enumeration - `load_folder` is broken on wasm (see the spike). Gated on the
ship kind (20260714-134115) existing so all three kinds route. `spike` until planned.

## Re-based v2 (20260714, spike tasks/20260714-150410)

GATED ON the content model (20260714-150508), which already gives one `ContentLoader` +
`register_content` for single files. This task adds the FOLDER packaging on top: a
`bundle.ron` manifest listing content files (relative paths), a bundle loader that
`load_context.load`s each `Content` file and flattens the items, then merges by kind via
the existing `register_content` router with load-order overlay. The merge-by-KIND is
already the content router (data flag, not extension); this task is really "manifest +
directory-of-content-files + overlay", not per-extension routing. Old extension framing
superseded.
