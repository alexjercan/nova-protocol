# Decision: StoryMessage carries an optional icon asset reference

- DATE: 20260725-170900
- STATUS: ACCEPTED
- TASK: 20260721-211526
- TAGS: decision, hud, ui, scenario, modding

## Context

The current story path is `StoryMessageActionConfig { speaker, text, dwell }`
in `nova_scenario`, synced into `nova_gameplay::hud::comms_panel::StoryFeed`.
The task asks for speaker icons, and the owner clarified at the plan gate that
an icon should be authorable like other mod assets: `self://` for the speaking
mod's own resources, `dep://<id>/` for dependencies, and `dep://base/` for base
game assets. Existing scenario fields such as `thumbnail: Option<AssetRef<Image>>`
already prove the shape and strict RON syntax.

## Decision

Add `icon: Option<AssetRef<Image>>` to `StoryMessageActionConfig` and mirror it
through `StoryLine`. Authors may omit the field or write `icon: None` for the
fallback, or write `icon: Some("self://...")` / `icon: Some("dep://...")` for a
real image. The comms HUD resolves authored refs with `AssetServer` and renders
a deterministic fallback when no icon is supplied.

## Alternatives considered

- **Renderer-only speaker mapping** - avoids a schema change, but blocks mod
  authors from choosing custom speaker art and duplicates speaker knowledge in
  HUD code.
- **Required `icon: AssetRef<Image>`** - forces every line to carry art and
  breaks older authored StoryMessage RON. Optional keeps back-compat and lets
  short/status lines use the fallback.
- **Separate cast registry keyed by speaker** - avoids repeating an icon on
  each line, but it creates a new authoring surface before the comms-stack UX is
  proven. A later cast registry can still build on this field.

## Consequences

This task becomes a small schema change: serde, content generation, docs, and
resource-ref validation all need coverage. In return, base content and mods use
the same asset-reference pipeline as thumbnails, skyboxes, section meshes, and
sounds, and the HUD does not need a hardcoded speaker-to-icon table.
