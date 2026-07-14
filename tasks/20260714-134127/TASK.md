# Mod loading + load-order overlay + a demo mod (override a section, add a scenario)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.6.0,modding,scenario,spike

Spike: tasks/20260714-113418/SPIKE.md

Goal: the payoff - a mod is another bundle merged on top of the base. A wasm-safe
top-level `mods.ron` lists enabled mod-bundle manifests; each loads after the base
and merges by kind with LOAD-ORDER overlay (later id wins = mod overrides base;
intra-bundle duplicate id = hard error). Native may optionally enumerate a `mods/`
dir, but `mods.ron` stays the wasm-safe source of truth. Ship a DEMO mod that
overrides one base section and adds one scenario, with a test proving the base+mod
merge + overlay end-to-end. Gated on the base-as-bundle (20260714-134123). `spike`
until planned.
