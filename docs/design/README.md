# Design records

Durable design and implementation-reflection records for changes that are not
owned by a single tatr task, or whose reasoning outlives the task that produced
them. Each file captures the AGENTS.md record for a change - what changed and
why (alternatives, tradeoffs), the difficulties hit and how they were diagnosed,
and what to do differently next time.

Use this folder when the record is cross-cutting or the work landed outside a
task's folder. A record tied to one task still lives in that task's folder as
`tasks/<id>/NOTES.md`; durable player/developer reference (how a system works,
not why it was built) belongs in the wiki under `web/src/wiki/dev/`.

Name files `topic-in-kebab-case.md`. Add an index line here when adding a file.

## Index

- [craft-ships-into-base.md](craft-ships-into-base.md) - moving the racer /
  cargob example mods into the base campaign, built from Rust content builders.
- [craft-ships-prototypes-and-mods.md](craft-ships-prototypes-and-mods.md) -
  racer / cargob as reusable base section prototypes; re-skinning menus + webmods.
- [mod-binary-resources.md](mod-binary-resources.md) - mods shipping their own
  binary resources via mod-relative asset refs.
- [mod-skybox-meta-always.md](mod-skybox-meta-always.md) - mod-shipped skybox
  cubemaps under `AssetMetaCheck::Always` (task 20260717-111558).
- [scenario-linger.md](scenario-linger.md) - scenario transitions when `linger`
  is false.
- [section-collider-config.md](section-collider-config.md) - authorable section
  collider shape and size (task 20260718-102022).
- [section-render-mesh-transform.md](section-render-mesh-transform.md) -
  `render_mesh_transform` for all section kinds (task 20260718-121205).
- [turret-render-mesh-transform.md](turret-render-mesh-transform.md) - per-joint
  render-mesh transform for turret sections (task 20260718-113307).
- [wasm-asset-meta-always.md](wasm-asset-meta-always.md) - why
  `AssetMetaCheck::Always` plus the `nova_meta_gen` `.meta` sidecar generator
  exist (the web trunk-serve 200-not-404 trap).
