# Review 07: Ship integrity, skin, camera, and audio

- Baseline: `b7a56f0586`
- Lanes: craft, performance, correctness, contracts
- Verdict: minor test finding

## Finding

### MINOR - `crates/nova_ship/src/ship_audio/cues.rs:825-837` - The unauthored lock-warning test never exercises its named branch

`a_ship_that_authors_no_warn_lock_is_locked_in_silence` puts `ShipFeedbackSounds::default()` on a child of the player root. Production lookup requires `ShipFeedbackSounds` and `PlayerSpaceshipMarker` on the same entity. The system therefore returns because it finds no player sounds row, not because it finds a row whose `warn_lock` is absent.

The silent result is correct, and positive lock-warning behavior has other coverage. This fixture still fails to prove the specific authored-or-silent branch named by the test.

Why not higher: production behavior was not shown wrong; this is a false-positive regression proof for one edge.

## Adjudication

No runtime finding survived for section destruction and severing, docking lifecycle, shell/skin derivation, decoration determinism, camera authority and handback, or ship audio routing and latching.

Dropped unmeasured allocation and world-scan performance candidates. Existing code documents bounded workloads and prior measured reductions, and the review established no current frame cost.

## Coverage

Across the pair, fully read the complete `nova_ship` remainder: base/hull/docking/shared sections, integrity, shell/skin/patch and render helpers, section animation, camera, ship audio, crate wiring, and all direct tests. Together with reviews 05-06, every Rust file in `nova_ship` has a full static pass.

Not checked: no Cargo command, content lint, game, probe, wasm build, workspace test, or Clippy run. Audio and camera tuning were not measured or judged as balance. Cross-crate gameplay, scenario, and HUD consumers were checked only where needed to establish direct contracts.
