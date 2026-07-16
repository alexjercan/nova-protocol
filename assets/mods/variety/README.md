# Variety Pack mod

The worked example for a mod that ships and uses its OWN binary art (task
20260716-123544). Unlike every other bundle - whose asset refs point at BASE
files under `assets/` - this bundle carries its own skybox and asteroid texture
and references them with mod-relative `self://` paths.

Layout:

```
variety/
  variety.bundle.ron      # manifest: content + resources + meta
  variety.content.ron     # a scenario using self:// asset refs
  textures/
    nebula.png            # placeholder skybox (stacked 1x6 cube faces)
    nebula.png.meta       # RowCount cube reinterpret sidecar (rides along)
    rock.png              # placeholder asteroid texture
```

How mod-owned art works:

- `resources` in the manifest lists the binary files the bundle ships
  (bundle-dir-relative). Sidecar `.meta` files ship automatically and are not
  listed.
- Content references a resource with `self://<path>` (e.g.
  `cubemap: "self://textures/nebula.png"`). At merge time `self://` is rewritten
  to the mod's own folder: `mods/variety/...` when shipped, `mods://variety/...`
  when installed from the portal - native and web alike.
- A `self://` ref that names no declared resource is rejected by the portal
  generator, the static `content_lint`, and the in-game content gate.

The textures here are PLACEHOLDERS (debug-colored). Real art replaces them under
task 20260716-205214, keeping the same paths. See
`docs/design/mod-binary-resources.md` and the "Shipping your own art" section of
`web/src/wiki/dev/guide-make-a-mod.md`.
