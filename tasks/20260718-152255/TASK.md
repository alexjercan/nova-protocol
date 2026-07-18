# Spike: can nova_meta_gen become a Python build-time hook, or must it stay Rust (Bevy .meta coupling)? Decide and port-or-document

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.8.0,tooling,spike,web

## Goal

The user floated converting the `.meta` sidecar generator to a Python build-time
script (a Trunk post_build hook) like the portal port. But `nova_meta_gen` is
NOT engine-free the way the portal is: it boots a headless (GPU-free) Bevy app
and uses Bevy's `AssetServer` to produce the exact default `.meta` content for
each asset type, so `AssetMetaCheck::Always` works on the web. A Python rewrite
would have to hardcode Bevy's default meta format and would silently drift when
Bevy changes it. So this is a spike: decide port-vs-keep, don't blindly port.

## Steps

- Characterize what the current tool actually emits: dump the `.meta` content it
  writes per asset extension (png, glb, ogg, ...) and confirm how much is
  Bevy-version-specific vs static boilerplate.
- Judge drift risk: if the meta is stable, documented boilerplate, a Python
  script that writes it is safe and cheaper; if it is derived from Bevy internal
  defaults, keeping the Rust tool (which asks Bevy) is the correct call.
- If PORT: write `scripts/gen-meta.py`, wire it as the Trunk post_build hook in
  place of the Rust bin, add a parity check vs the Rust output over `assets/`,
  and note the Bevy-version assumption it bakes in.
- If KEEP: record the decision and why in the task NOTES / a wiki dev page, and
  drop this from the release scope so the plan reflects reality.

## Notes

- Tool: `crates/nova_meta_gen/{src/lib.rs,src/main.rs}`; invoked as a Trunk
  post_build hook (`TRUNK_STAGING_DIR/assets`).
- Background: `docs/design/wasm-asset-meta-always.md` (itself a docs-junk
  file the cleanup task 20260718-152329 should relocate).

