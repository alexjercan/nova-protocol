# Review: Bundle manifest + loader + merge-by-kind router into id-keyed registries

- TASK: 20260714-134119
- BRANCH: modding/folder-bundle

## Round 1

- VERDICT: APPROVE

Reviewed the folder-bundle diff (BundleManifest + BundleAsset + BundleAssetLoader
in nova_modding, the `assets/base/` packaging with git-renamed content files, and
the `register_bundles` router in nova_assets replacing the six `Handle<ContentAsset>`
with one `Handle<BundleAsset>`). Verified independently: `demo_scenario` gates on the
recursive dependency load state and passes; `cargo test --workspace --no-run` green;
nova_modding + nova_assets tests pass; both windowed examples (`12_menu_newgame`,
`09_editor`) reached Playing / exit 0 under `DISPLAY=:0 BCS_AUTOPILOT=1 --features debug`;
content parity green. Behavior is identical to the pre-bundle path (same content
registered, now via the base bundle). An out-of-context `/code-review` pass concurred
(APPROVE) and surfaced the findings below.

- [x] R1.1 (MINOR) crates/nova_assets/src/lib.rs (register_bundles) - `GameSections`
  was accumulated with `sections.push(cfg.clone())` (first-wins, appends duplicates),
  whereas scenarios overlay by id (`insert`, last-wins). A later bundle would not
  overlay an earlier section by id; `get_section`'s first-match would then ignore the
  mod's override. Hazard for the mods task (20260714-134127). Make the router overlay
  sections by id, last-wins, in place (preserve palette order).
  - Response: Fixed. Extracted the routing into a pure `merge_content_item` helper that
    overlays sections by id in place (linear replace, order preserved) matching the
    scenario map's insert-overlay. Pinned with two unit tests: `later_section_overlays
    _earlier_by_id_in_place` (replace not append, order kept, last value wins) and
    `later_scenario_overlays_earlier_by_id`. nova_assets test count 16 -> 18.

- [x] R1.2 (NIT) crates/nova_modding/src/lib.rs:16 - module doc referenced the old
  router name `register_content`; the router is `register_bundles`.
  - Response: Fixed - doc now reads `register_bundles`.

- [x] R1.3 (NIT) crates/nova_modding/src/lib.rs:225 - `AssetPath::from(rel.to_string())`
  looked like an unnecessary allocation; suggested `rel.as_str()`.
  - Response: Investigated, NOT applied - the suggestion does not compile. An
    `AssetPath::from(&str)` borrows `manifest.content`, which does not outlive the
    resolved path (`error[E0597]: manifest.content does not live long enough`). The
    owned `to_string()` is load-bearing, not a smell. Added a comment saying so.

Findings R1.1-R1.3 addressed in commit da39bb9. Re-verified: nova_assets 18 tests pass,
`cargo test --workspace --no-run` green, fmt clean. Cycle complete.
