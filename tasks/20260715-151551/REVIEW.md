# Review: Unship screenshot-reel - embed the reel scenario in the example

- TASK: 20260715-151551
- BRANCH: refactor/unship-reel

## Round 1

- VERDICT: APPROVE (two MINORs + two NITs, all addressed before landing)

Out-of-context review pass (fresh-context agent over the full 10-file diff vs
master). Verified: the example's `ron::de::from_str` with default options is
deserialization-identical to `ContentAssetLoader`'s `from_bytes` (same Content
type, one shared ron 0.12.2 in the lock); direct `LoadScenario` at
`OnEnter(Loaded)` matches the established `08_scenario.rs` pattern and the
observer registers at plugin build; the three synthetic-rig tests each fail
with their mechanism deleted (mutation analysis: dropping the ModCatalog
filter, the seed strip branch, or adding hidden-filtering to register_bundles
each breaks its test - the merge test is non-vacuous because ONLY
"hidden-fixture" is enabled, so demo content can enter GameScenarios only
through the hidden entry); no 3-entry assumptions remain anywhere; Trunk
copy-dirs only assets/ + credits/, so examples/data ships nowhere; deps are in
the right section with a coherent lockfile. Test re-runs: nova_modding 3,
demo_scenario 11, nova_menu 13; fmt + check --workspace --all-targets clean.

- [x] R1.1 (MINOR) examples/13_screenshot_reel.rs:127-153 - the smoke-probe
  half still described the deleted mod pipeline ("mod-driven load", "enable ->
  re-merge -> load chain", panic "(mod enable or registration failed)") - the
  panic text would mislead whoever debugs a failing smoke run.
  - Response: fixed on-branch - docs and panic message now describe the
    embedded path ("embedded scenario load failed"). Verified by reviewer.
- [x] R1.2 (MINOR) crates/nova_modding/src/lib.rs:312-315 - `ModEntry::hidden`
  rustdoc still cited the unshipped reel and the deleted example mechanism; the
  docs sweep covered markdown but missed rustdoc.
  - Response: fixed on-branch - the doc is now generic ("enableable by id from
    code; no shipped mod uses it right now; pinned by nova_assets'
    synthetic-catalog tests"). Verified by reviewer.
- [x] R1.3 (NIT) crates/nova_debug/src/harness.rs:277 - `ScreenshotReelPlugin`
  rustdoc cited "the `screenshot-reel` mod".
  - Response: fixed on-branch ("the embedded reel scenario in
    `13_screenshot_reel`"). Verified by reviewer.
- [x] R1.4 (NIT) examples/13_screenshot_reel.rs `parse_reel_scenario` -
  silently dropped non-Scenario items / extra scenarios, contradicting the
  "fails loud" doc (the mod path would have routed a Section into
  GameSections).
  - Response: fixed on-branch - the parser now panics on any non-Scenario item
    and asserts exactly one scenario. Both example flavors re-checked clean
    (plain + --features debug, zero new warnings). Verified by reviewer.
