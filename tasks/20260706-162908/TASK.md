# Re-enable particle effects on wasm

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.5.0,chore,wasm

From the TODO sweep (task 20260525-132954). Particle effects are disabled on wasm
because hanabi was not working there. Re-enable once a wasm-compatible path exists.

Source FIXMEs:
- crates/nova_gameplay/src/plugin.rs (HanabiPlugin add is cfg'd off for wasm)
- crates/nova_gameplay/src/sections/torpedo_section.rs
- crates/nova_gameplay/src/sections/turret_section.rs (x2)
