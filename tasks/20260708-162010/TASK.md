# Embed a sandboxed scripting VM for scenario logic (piccolo prototype)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.5.0,modding,spike

Spike: docs/spikes/20260708-161726-modding-language-and-scripting.md

Phase 2 of the modding-language direction, and itself a prototype/spike task (do
NOT commit to it before the prototype). Gated on phase 1 (RON scenario format,
133029) existing and the declarative form provably running out of road - i.e.
modders needing custom actions/conditions the fixed `EventActionConfig` set and
the `variables.rs` arithmetic AST cannot express.

Recommendation from the spike: prototype with `piccolo` (pure-Rust, stackless
Lua) rather than mlua, for two reasons decisive for THIS game: (1) pure Rust keeps
the wasm/Trunk build clean (the game already has a wasm-blocked feature), and
(2) fuel-based stepping bounds CPU/RAM per frame - real sandboxing for untrusted
community mods. Accept that piccolo is WIP and its stackless API needs more
binding glue. Build a throwaway end-to-end integration (one scenario hook, e.g. a
custom `OnUpdate` condition, driven through piccolo), measure binding ergonomics
and wasm build impact, THEN decide. Documented fallback if piccolo blocks: mlua
with vendored `lua54` + a manual instruction-count limiter, accepting the heavier
wasm story. `bevy_mod_scripting` is explicitly not recommended (Bevy-version
coupling + its own world-access model fights `NovaEventWorld`).
</content>
