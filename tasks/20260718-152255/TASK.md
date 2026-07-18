# Spike: can nova_meta_gen become a Python build-time hook, or must it stay Rust (Bevy .meta coupling)? Decide and port-or-document

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.8.0,tooling,spike,web

## Story

As the project owner consolidating tooling, I want a grounded decision on
whether the `.meta` sidecar generator can follow the portal generator to
Python, so that the tooling map has one answer instead of an open question -
and so we do not trade a working Rust tool for a Python script that silently
drifts from Bevy.

The user floated converting the `.meta` sidecar generator to a Python
build-time script (a Trunk post_build hook) like the portal port. But
`nova_meta_gen` is NOT engine-free the way the portal is: it boots a headless
(GPU-free) Bevy app and uses Bevy's `AssetServer` to produce the exact default
`.meta` content for each asset type, so `AssetMetaCheck::Always` works on the
web. A Python rewrite would have to hardcode Bevy's default meta format and
would silently drift when Bevy changes it. So this is a spike: decide
port-vs-keep, don't blindly port.

## Steps

- [ ] Characterize what the current tool actually emits: dump the `.meta`
      content it writes per asset extension (png, glb, ogg, wav, ...) and
      confirm how much is Bevy-version-specific vs static boilerplate. Check
      it against at least the last two Bevy versions' defaults (0.18 vs 0.19)
      to see whether the format actually moved.
- [ ] Judge drift risk: if the meta is stable, documented boilerplate, a
      Python script that writes it is safe and cheaper; if it is derived from
      Bevy internal defaults, keeping the Rust tool (which asks Bevy) is the
      correct call. Weigh that v0.7.0 made `.meta` correctness load-bearing
      for EVERY mod cubemap (the WebGL2 crash class), so silent drift now
      breaks mods, not just the base build.
- [ ] If PORT: write `scripts/gen-meta.py`, wire it as the Trunk post_build
      hook in place of the Rust bin, add a parity check vs the Rust output
      over `assets/`, and note the Bevy-version assumption it bakes in.
- [ ] If KEEP: record the decision and why in this task + the dev wiki
      (development.md tools list gets a "stays Rust because it asks Bevy"
      line), and update the tooling inventory (20260718-152304) so the plan
      reflects reality.
- [ ] Write the SPIKE.md in this task folder either way: evidence dump,
      decision, consequences.

## Definition of Done

- SPIKE.md exists with the per-extension meta dump and the drift assessment.
- Exactly one of: `scripts/gen-meta.py` wired into Trunk with a parity check,
  OR a recorded keep-Rust decision reflected in the dev wiki and the tooling
  inventory.
- Either way, the README/wiki tools sections name the current tool and its
  invocation accurately.

## Notes

- Tool: `crates/nova_meta_gen/{src/lib.rs,src/main.rs}`; invoked as a Trunk
  post_build hook (`TRUNK_STAGING_DIR/assets`).
- Background: `docs/design/wasm-asset-meta-always.md` - source material for
  the dev-wiki write-up (see 20260718-152214's sequencing note before the
  ephemeral-docs wipe folds it away).
