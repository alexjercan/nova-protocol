# Relocate nova_meta_gen to tools/ as a workspace-member build tool (out of crates/)

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.8.0,tooling,refactor,web,spike

## Story

As the project owner, I want the web-only `.meta` sidecar generator out of my
game's `crates/` list and framed as build tooling, so the native game's crate
graph is not cluttered by a tool it never uses - without breaking the tool's
pin to the game's exact Bevy (version AND features).

Decided by spike 20260718-152255 (round 2, Option A): the tool must stay Rust
and ask Bevy (round 1 - the metas carry version-specific loader type-names +
non-defaulted settings, so a hardcode drifts), and it must stay a WORKSPACE
MEMBER (feature unification is what auto-pins the `wav` feature via
nova_modding -> nova_gameplay; leaving the workspace silently drops `.wav`
sidecars). So the move is: relocate the crate OUT of `crates/` into a top-level
`tools/` dir, keep it a workspace member, and optionally exclude it from bare
builds via `default-members`.

## Notes

- Spike: tasks/20260718-152255/SPIKE.md (round 2 = the location decision).
- Leaf tool: nothing depends on `nova_meta_gen`; Trunk invokes it BY PACKAGE
  NAME (`cargo run -p nova_meta_gen`, Trunk.toml post_build), so the hook is
  unchanged by a directory move - only path-shaped references move.
- Web-only: native's real-filesystem 404 lets Bevy fall back to defaults; only
  the web SPA-fallback-200-HTML trap needs pre-written sidecars.
- Coordinates with the tooling-inventory umbrella 20260718-152304 (keep its
  catalog in sync: meta-gen moves to tools/).
