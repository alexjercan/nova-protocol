# DECISION: how to give the `.ttc` font a `.meta` on the web

- STATUS: ACCEPTED

## Context

The web NOVA OS text bug is caused by the custom-loaded `.ttc` font having no
`.meta` sidecar in the web build, because `nova_meta_gen` does not know the
`NovaOsTtcFontLoader` (which claims the `ttc` extension). The runtime resolves
a loader named in a `.meta` by its registered type path, so any fix's meta MUST
name the exact loader type the game registers
(`nova_gameplay::hud::nova_os::NovaOsTtcFontLoader`).

## Options

- **A. Register the real `NovaOsTtcFontLoader` in `nova_meta_gen`** so it
  auto-generates the default `...ttc.meta` (naming the real loader, default
  settings), exactly as it does for every other asset. `nova_gameplay` is
  already in meta_gen's build graph (meta_gen -> nova_modding -> nova_gameplay),
  so this adds no meaningful compile cost; it needs only `pub` on the loader
  type and a direct `nova_gameplay` dep line. Fits meta_gen's stated design:
  generate default sidecars from registered loaders; hand-authored metas are
  reserved for NON-default settings (the cubemap `array_layout` metas).

- **B. Commit a hand-authored `assets/fonts/....ttc.meta`** naming the loader
  type. Minimal, mirrors the cubemap-meta precedent, but duplicates the loader
  type-path as a bare string that can silently rot on a rename/move, and it is
  the exact thing meta_gen exists to AVOID hand-authoring. The font needs only
  DEFAULT settings, so it does not qualify for the hand-authored exception.

## Decision: Option A (ACCEPTED, pending plan-gate confirm)

Register `NovaOsTtcFontLoader` in `nova_meta_gen::build_app`. The loader becomes
`pub`; meta_gen gains a direct `nova_gameplay` dependency (already compiled
transitively) and registers the loader alongside Image/Shader/Audio/Gltf/RON.
The `.ttc` then gets an auto-generated, always-correct default meta - no
duplicated type-path string, no hand-authored exception, and any future custom
loader added to the game is a one-line addition here.
